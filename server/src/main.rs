use std::{collections::HashMap, path::PathBuf, str::FromStr};

use futures::future::join_all;
use near_workspaces::types::SecretKey;
use race_of_sloths_bot::{
    api::{
        github::{GithubClient, PrMetadata},
        near::NearClient,
    },
    events::{actions::Action, commands::Command, Context, Event},
    messages::MessageLoader,
};
use serde::Deserialize;
use tokio::signal;
use tracing::{debug, error, info, instrument, trace};
use tracing_subscriber::{layer::SubscriberExt, EnvFilter};

#[derive(Deserialize)]
struct Env {
    github_token: String,
    contract: String,
    secret_key: String,
    is_mainnet: bool,
    message_file: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    let subscriber = tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer());
    tracing::subscriber::set_global_default(subscriber)?;

    let env = envy::from_env::<Env>()?;

    let github_api = GithubClient::new(env.github_token).await?;
    let messages = MessageLoader::load_from_file(&env.message_file, &github_api.user_handle)?;
    let near_api = NearClient::new(
        env.contract,
        SecretKey::from_str(&env.secret_key)?,
        env.is_mainnet,
    )
    .await?;
    let context = Context {
        github: github_api.into(),
        near: near_api.into(),
        messages: messages.into(),
    };

    tokio::select! {
        _ = run(context) => {
            error!("Main loop exited unexpectedly.")
        }
        _ = signal::ctrl_c() => {
            info!("Received SIGINT. Exiting.");
        }
    }

    Ok(())
}

async fn run(context: Context) {
    let minute = tokio::time::Duration::from_secs(60);
    let mut interval: tokio::time::Interval = tokio::time::interval(minute);
    let mut merge_time = std::time::SystemTime::now();
    let merge_interval = 60 * minute;

    loop {
        let current_time = std::time::SystemTime::now();
        (_, _, merge_time) = tokio::join!(
            interval.tick(),
            event_task(context.clone()),
            merge_and_execute_task(context.clone(), current_time, merge_time, merge_interval)
        )
    }
}

async fn event_task(context: Context) {
    let events = match context.github.get_events().await {
        Ok(events) => events,
        Err(e) => {
            error!("Failed to get events: {}", e);
            return;
        }
    };

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

    let events = match merge_events(&context).await {
        Ok(events) => events,
        Err(e) => {
            error!("Failed to get merge events: {}", e);
            return merge_time;
        }
    };

    execute(context.clone(), events).await;

    // It matters to first execute the merge events and then finalize
    // as the merge event is a requirement for the finalize event
    let event = match finalized_events(&context).await {
        Ok(events) => events,
        Err(e) => {
            error!("Failed to get finalize events: {}", e);
            return merge_time;
        }
    };
    execute(context.clone(), event).await;

    current_time + merge_interval
}

// Runs events from the same PR
#[instrument(skip(context, events))]
async fn execute(context: Context, events: Vec<Event>) {
    if events.is_empty() {
        return;
    }

    debug!("Executing {} events", events.len());
    let mut should_update = false;
    for event in &events {
        match event.execute(context.clone()).await {
            Ok(res) => {
                should_update |= res;
            }
            Err(e) => {
                error!("Failed to execute event for {}: {e}", event.pr().full_id);
            }
        }
    }
    let pr = events[0].pr();

    if !should_update {
        debug!(
            "No events that require updating status comment for {}",
            pr.full_id
        );
        return;
    }

    debug!(
        "Finished executing events. Updating status comment for {}",
        pr.full_id
    );
    let info = match context.check_info(pr).await {
        Ok(info) => info,
        Err(e) => {
            error!("Failed to get PR info for {}: {e}", pr.full_id);
            return;
        }
    };

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
        let pr = match pr {
            Ok(pr) => pr,
            Err(e) => {
                error!("Failed to get PR: {e}");
                continue;
            }
        };

        let pr_metadata = match PrMetadata::try_from(pr) {
            Ok(pr) => pr,
            Err(e) => {
                error!("Failed to convert PR: {e}");
                continue;
            }
        };

        if pr_metadata.merged.is_none() {
            trace!(
                "PR {} is not merged. Checking for stale",
                pr_metadata.full_id
            );
            if check_for_stale_pr(&pr_metadata) {
                info!("PR {} is stale. Creating an event", pr_metadata.full_id);
                results.push(Event::Action(Action::stale(pr_metadata)));
            }
            continue;
        }
        trace!("PR {} is merged. Creating an event", pr_metadata.full_id);
        if let Some(merged) = Action::merge(pr_metadata) {
            results.push(Event::Action(merged));
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
        .map(|pr| Event::Action(Action::finalize(pr.into())))
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
