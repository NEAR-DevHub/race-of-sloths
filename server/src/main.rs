use std::{collections::HashMap, str::FromStr, sync::Arc};

use futures::future::join_all;
use near_workspaces::types::SecretKey;
use serde::Deserialize;
use slothrace::{
    api::{
        github::{GithubClient, PrMetadata},
        near::NearClient,
    },
    commands::{
        actions::PullRequestFinalize, actions::PullRequestMerge, actions::PullRequestStale,
        Command, Context, Event,
    },
};
use tracing::{debug, error, info, instrument, trace, warn};

#[derive(Deserialize)]
struct Env {
    github_token: String,
    contract: String,
    secret_key: String,
    is_mainnet: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();

    let env = envy::from_env::<Env>()?;

    let github_api = Arc::new(GithubClient::new(env.github_token).await?);
    let near_api = Arc::new(
        NearClient::new(
            env.contract,
            SecretKey::from_str(&env.secret_key)?,
            env.is_mainnet,
        )
        .await?,
    );
    let context = Context {
        github: github_api,
        near: near_api,
    };

    let minute = tokio::time::Duration::from_secs(60);
    let mut interval: tokio::time::Interval = tokio::time::interval(minute);
    let mut merge_time = std::time::SystemTime::now();
    let merge_interval = 60 * minute;

    loop {
        let current_time = std::time::SystemTime::now();
        (_, _, merge_time) = tokio::join!(
            interval.tick(),
            event_task(context.clone()),
            merge_and_execute_task(context.clone(), current_time, merge_time, merge_interval),
        );
    }

    Ok(())
}

async fn event_task(context: Context) {
    let events = context.github.get_events().await;
    if let Err(e) = events {
        error!("Failed to get events: {}", e);
        return;
    }
    let events = events.unwrap();

    info!("Received {} events.", events.len());

    let events_per_pr = events.into_iter().fold(
        std::collections::HashMap::new(),
        |mut map: HashMap<String, Vec<Command>>, event| {
            let pr = event.pr();
            map.entry(pr.full_id.clone()).or_default().push(event);
            map
        },
    );

    let futures = events_per_pr.into_iter().map(|(key, events)| {
        debug!("Received {} events for PR {}", events.len(), key);
        let events: Vec<_> = events.into_iter().map(Event::Command).collect();
        execute(context.clone(), events)
    });

    join_all(futures).await;
}

async fn merge_and_execute_task(
    context: Context,
    current_time: std::time::SystemTime,
    merge_time: std::time::SystemTime,
    merge_interval: std::time::Duration,
) -> std::time::SystemTime {
    if current_time < merge_time {
        return merge_time;
    }

    let events = merge_events(&context).await;
    if let Err(e) = events {
        error!("Failed to get merge events: {}", e);
        return merge_time;
    }
    execute(context.clone(), events.unwrap()).await;

    // It matters to first execute the merge events and then finalize
    // as the merge event is a requirement for the finalize event
    let event = finalized_events(&context).await;
    if let Err(e) = event {
        error!("Failed to get execute events: {}", e);
        return merge_time;
    }
    execute(context.clone(), event.unwrap()).await;

    return current_time + merge_interval;
}

// Runs events from the same PR
#[instrument(skip(context, events))]
async fn execute(context: Context, events: Vec<Event>) {
    if events.is_empty() {
        return;
    }

    debug!("Executing {} events", events.len());
    for event in &events {
        if let Err(e) = event.execute(context.clone()).await {
            error!("Failed to execute event for {}: {e}", event.pr().full_id);
        }
    }

    debug!("Finished executing events");
    debug!("Updating status comment");
    let pr = events[0].pr();
    let info = context.check_info(&pr).await;
    if let Err(e) = info {
        error!("Failed to get PR info for {}: {e}", pr.full_id);
        return;
    }
    let info = info.unwrap();

    if let Err(e) = context
        .github
        .edit_comment(&pr.owner, &pr.repo, info.comment_id, &info.status_message())
        .await
    {
        error!("Failed to update status comment for {}: {e}", pr.full_id);
    }
}

#[instrument(skip(context))]
async fn merge_events(context: &Context) -> anyhow::Result<Vec<Event>> {
    let prs = context.near.unmerged_prs_all().await?;
    info!("Received {} PRs for merge request check", prs.len());
    let mut results = vec![];

    for pr in prs {
        let pr = context
            .github
            .get_pull_request(&pr.organization, &pr.repo, pr.number)
            .await;
        if let Err(e) = pr {
            warn!("Failed to get PR: {e}");
            continue;
        }
        let pr = pr.unwrap();

        let pr_metadata = PrMetadata::try_from(pr);

        if let Err(e) = pr_metadata {
            warn!("Failed to convert PR: {e}");
            continue;
        }

        let pr_metadata = pr_metadata.unwrap();
        if pr_metadata.merged.is_none() {
            trace!(
                "PR {} is not merged. Checking for stale",
                pr_metadata.full_id
            );
            if check_for_stale_pr(&pr_metadata) {
                info!("PR {} is stale. Creating an event", pr_metadata.full_id);
                results.push(Event::Stale(PullRequestStale { pr_metadata }));
            }
            continue;
        }
        trace!("PR {} is merged. Creating an event", pr_metadata.full_id);
        let merged = PullRequestMerge::new(pr_metadata);
        if let Some(merged) = merged {
            results.push(Event::Merged(merged));
        }
    }
    info!("Finished merge task with {} events", results.len());
    Ok(results)
}

#[instrument(skip(context))]
async fn finalized_events(context: &Context) -> anyhow::Result<Vec<Event>> {
    let prs = context.near.unfinalized_prs_all().await?;
    info!("Received {} PRs for merge request check", prs.len());

    Ok(prs
        .into_iter()
        .map(|pr| Event::Finalize(PullRequestFinalize::new(pr)))
        .collect())
}

fn check_for_stale_pr(pr: &PrMetadata) -> bool {
    if pr.merged.is_some() {
        return false;
    }

    let now = chrono::Utc::now();
    let stale = now - pr.updated_at;
    stale.num_days() > 14
}
