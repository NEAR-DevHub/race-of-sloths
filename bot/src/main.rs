use std::{collections::HashMap, path::PathBuf, sync::Arc};

use futures::future::join_all;
use race_of_sloths_bot::{
    api::{prometheus::PrometheusClient, GithubClient},
    events::{actions::Action, Context, Event, EventResult, EventType},
    messages::MessageLoader,
};
use rocket::routes;
use serde::Deserialize;
use tokio::signal;
use tracing::{debug, error, info, instrument, trace};
use tracing_subscriber::{layer::SubscriberExt, EnvFilter};

use shared::github::PrMetadata;
use shared::near::NearClient;
use shared::telegram;

#[derive(Deserialize)]
struct Env {
    read_github_tokens: String,
    github_token: String,
    contract: String,
    rpc_addr: Option<String>,
    secret_key: String,
    is_mainnet: bool,
    message_file: PathBuf,
    telegram_token: String,
    telegram_chat_id: String,
    desired_bot_name: Option<String>,
}

#[rocket::get("/metrics")]
pub async fn metrics(
    state: &rocket::State<Context>,
) -> Option<(
    rocket::http::ContentType,
    rocket::response::content::RawHtml<String>,
)> {
    let rate_linits = state.github.get_rate_limits().await.ok()?;
    state
        .prometheus
        .set_read_requests(rate_linits.resources.core.used as i64);
    let metrics = state.prometheus.encode().ok()?;
    Some((
        rocket::http::ContentType::new(
            "application/openmetrics-text",
            " version=1.0.0; charset=utf-8",
        ),
        rocket::response::content::RawHtml(metrics),
    ))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("Failed to install AWS LC provider");
    dotenv::dotenv().ok();
    let env = envy::from_env::<Env>()?;
    let telegram: telegram::TelegramSubscriber =
        telegram::TelegramSubscriber::new(env.telegram_token, env.telegram_chat_id).await;

    let subscriber = tracing_subscriber::registry()
        .with(telegram.clone())
        .with(EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer());
    tracing::subscriber::set_global_default(subscriber)?;

    let prometheus: Arc<PrometheusClient> = Default::default();
    let read_tokens = env
        .read_github_tokens
        .split(',')
        .map(|s| s.to_string())
        .collect();
    let github_api = GithubClient::new(env.github_token, read_tokens, prometheus.clone()).await?;
    let bot_name = env
        .desired_bot_name
        .unwrap_or_else(|| github_api.write_user_handle().to_string());
    let messages = MessageLoader::load_from_file(&env.message_file, &bot_name)?;
    let near_api =
        NearClient::new(env.contract, env.secret_key, env.is_mainnet, env.rpc_addr).await?;
    let context = Context {
        github: github_api.into(),
        bot_name,
        near: near_api.into(),
        messages: messages.into(),
        prometheus,
        telegram: telegram.into(),
    };

    tokio::select! {
        _ = run(context.clone()) => {
        }
        _ = signal::ctrl_c() => {
            tracing::warn!("Received SIGINT. Exiting.");
        }
        _ = rocket::build()
            .mount("/", routes![metrics])
            .manage(context)
            .launch() => {

            }
    }
    tracing::warn!("Exiting bot...");

    Ok(())
}

async fn run(context: Context) {
    tracing::warn!("Starting bot...");

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
        |mut map: HashMap<String, Vec<Event>>, event| {
            let repo_info = event.event.repo_info();
            map.entry(repo_info.full_id.clone())
                .or_default()
                .push(event);
            map
        },
    );

    let futures = events_per_pr.into_iter().map(|(key, events)| {
        debug!("Received {} events for PR {}", events.len(), key);
        execute_events_from_one_pr(context.clone(), events)
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

    let futures = events
        .into_iter()
        .map(|event| async { execute_events_from_one_pr(context.clone(), vec![event]).await });
    join_all(futures).await;

    // It matters to first execute the merge events and then finalize
    // as the merge event is a requirement for the finalize event
    let events = match finalized_events(&context).await {
        Ok(events) => events,
        Err(e) => {
            error!("Failed to get finalize events: {}", e);
            return merge_time;
        }
    };

    let futures = events
        .into_iter()
        .map(|event| async { execute_events_from_one_pr(context.clone(), vec![event]).await });
    join_all(futures).await;

    current_time + merge_interval
}

// Runs events from the same PR
#[instrument(skip(context, events))]
async fn execute_events_from_one_pr(context: Context, mut events: Vec<Event>) {
    // TODO: pretty sure that we can achive deduplication with keeping the last element more easily
    events.reverse();
    events.dedup_by(|a, b| {
        a.event.repo_info().full_id == b.event.repo_info().full_id && a.event.same_event(&b.event)
    });
    events.reverse();

    if events.is_empty() {
        return;
    }

    debug!("Executing {} events", events.len());
    let mut should_update = false;
    let event = &events[0];
    let repo_info = event.event.repo_info();

    if !events
        .iter()
        .all(|e| repo_info.full_id == e.event.repo_info().full_id)
    {
        error!("Constraint failed: all events should be for the same PR")
    }

    let mut check_info = match context.check_info(repo_info).await {
        Ok(info) => info,
        Err(e) => {
            error!("Failed to get PR info for {}: {e}", repo_info.full_id);
            return;
        }
    };

    for event in &events {
        match event.execute(context.clone(), &mut check_info).await {
            Ok(EventResult::Success { should_update: upd }) => {
                should_update |= upd;
            }
            Ok(_) => {}
            Err(e) => {
                error!("Failed to execute event for {}: {e}", repo_info.full_id);
            }
        }
    }

    if !should_update {
        debug!(
            "No events that require updating status comment for {}",
            repo_info.full_id
        );
        return;
    }

    debug!(
        "Finished executing events. Updating status comment for {}",
        repo_info.full_id
    );

    let Some(pr) = events.iter().find_map(|e| match &e.event {
        EventType::Action { pr, .. } | EventType::PRCommand { pr, .. } => Some(pr),
        _ => None,
    }) else {
        error!("Not found any PR in events, but requested update");
        return;
    };

    context
        .status_message(pr, event.comment.clone(), check_info, None)
        .await;
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

        let merged_by = pr.merged_by.clone();

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
                pr_metadata.repo_info.full_id
            );
            if check_for_stale_pr(&pr_metadata) {
                info!(
                    "PR {} is stale. Creating an event",
                    pr_metadata.repo_info.full_id
                );
                results.push(Event {
                    event: EventType::Action {
                        action: Action::stale(),
                        pr: pr_metadata,
                    },
                    comment: None,
                    event_time: chrono::Utc::now(),
                });
            }
            continue;
        }
        trace!(
            "PR {} is merged. Creating an event",
            pr_metadata.repo_info.full_id
        );
        let merged_by = merged_by
            .map(|e| e.login)
            .unwrap_or_else(|| pr_metadata.author.login.clone());

        let reviewers = context
            .github
            .get_positive_or_pending_review(
                &pr_metadata.repo_info.owner,
                &pr_metadata.repo_info.repo,
                pr_metadata.repo_info.number,
            )
            .await
            .unwrap_or_default();
        results.push(Event {
            event_time: pr_metadata.merged.unwrap(),
            comment: None,
            event: EventType::Action {
                action: Action::merge(merged_by, reviewers),
                pr: pr_metadata,
            },
        });
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
        .map(|pr| Event {
            event_time: pr
                .ready_to_move_timestamp()
                .map(|t| chrono::DateTime::from_timestamp_nanos(t as i64))
                .unwrap_or_else(chrono::Utc::now),
            comment: None,
            event: EventType::Action {
                action: Action::finalize(),
                pr: pr.into(),
            },
        })
        .collect())
}

fn check_for_stale_pr(pr: &PrMetadata) -> bool {
    if pr.merged.is_some() {
        return false;
    }

    let now = chrono::Utc::now();
    let stale = now - pr.updated_at;
    stale.num_days() > 14 || pr.closed
}
