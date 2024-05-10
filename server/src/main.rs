use std::{collections::HashMap, str::FromStr, sync::Arc};

use near_workspaces::types::SecretKey;
use serde::Deserialize;
use slothrace::{
    api::{
        github::{GithubClient, PrMetadata},
        near::NearClient,
    },
    commands::{
        merged::PullRequestMerged, stale::PullRequestStale, Command, Context, ContextStruct, Event,
        Execute,
    },
};
use tokio::sync::{
    mpsc::{self, UnboundedReceiver},
    Mutex,
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

    let github_api = GithubClient::new(env.github_token).await?;
    let (tx, rx) = mpsc::unbounded_channel::<Vec<Event>>();
    let rx: Arc<Mutex<UnboundedReceiver<Vec<Event>>>> = Arc::new(Mutex::new(rx));
    let context: Arc<ContextStruct> = Arc::new(ContextStruct {
        github: github_api,
        near: NearClient::new(
            env.contract,
            SecretKey::from_str(&env.secret_key)?,
            env.is_mainnet,
        )
        .await?,
    });

    // Spawn worker tasks
    for _ in 0..4 {
        let context = context.clone();
        let rx = Arc::clone(&rx);
        tokio::spawn(async move {
            loop {
                let span = tracing::span!(tracing::Level::DEBUG, "WORKER");
                let _guard = span.enter();

                let events: Option<Vec<Event>> = {
                    let mut rx = rx.lock().await;
                    rx.recv().await
                };
                if let Some(events) = events {
                    execute(context.clone(), events).await;
                } else {
                    break;
                }
            }
        });
    }

    let minute = tokio::time::Duration::from_secs(60);
    let mut interval: tokio::time::Interval = tokio::time::interval(minute);
    let mut merge_time = std::time::SystemTime::now();
    let merge_interval = 60 * minute;

    let mut finalize_time = std::time::SystemTime::now();
    let finalize_interval = minute * 60 * 24;

    loop {
        interval.tick().await;

        let events = context.github.get_events().await;
        if let Err(e) = events {
            error!("Failed to get events: {}", e);
            continue;
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

        for (key, mut events) in events_per_pr.into_iter() {
            events.sort_by_key(|event| *event.timestamp());
            let events = events
                .into_iter()
                .map(|event| Event::Command(event))
                .collect();

            if let Err(e) = tx.send(events) {
                error!("Failed to send events from {}: {}", key, e);
            }
        }
        let current_time = std::time::SystemTime::now();
        if current_time > merge_time {
            let events = merge_task(&context).await?;
            if let Err(e) = tx.send(events) {
                error!("Failed to send merge events: {}", e);
            }
            merge_time = current_time + merge_interval;
        }

        if current_time > finalize_time {
            let res = context.near.finalize_prs().await;
            if let Err(e) = res {
                error!("Failed to finalize PRs: {}", e);
            }
            finalize_time = current_time + finalize_interval;
        }
    }

    Ok(())
}

// Runs events from the same PR
#[instrument(skip(context, events))]
async fn execute(context: Context, events: Vec<Event>) {
    debug!("Executing {} events", events.len());
    for event in events {
        if let Err(e) = event.execute(context.clone()).await {
            error!("Failed to execute event for {}: {e}", event.pr().full_id);
        }
    }
}

#[instrument(skip(context))]
async fn merge_task(context: &Context) -> anyhow::Result<Vec<Event>> {
    let prs = context.near.unmerged_prs_all().await.unwrap();
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
        let event = Event::Merged(PullRequestMerged { pr_metadata });
        results.push(event);
    }
    info!("Finished merge task with {} events", results.len());
    Ok(results)
}

fn check_for_stale_pr(pr: &PrMetadata) -> bool {
    if pr.merged.is_some() {
        return false;
    }

    let now = chrono::Utc::now();
    let stale = now - pr.updated_at;
    if stale.num_days() > 14 {
        true
    } else {
        false
    }
}
