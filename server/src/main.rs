use std::{collections::HashMap, sync::Arc};

use serde::Deserialize;
use slothrace::{
    api::{
        github::{Event, GithubClient},
        near::NearClient,
    },
    commands::{Context, ContextStruct, Execute},
};
use tokio::sync::{
    mpsc::{self, UnboundedReceiver},
    Mutex,
};

#[derive(Deserialize)]
struct Env {
    github_token: String,
    contract: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();

    let env = envy::from_env::<Env>()?;

    let github_api = GithubClient::new(env.github_token)?;
    let (tx, rx) = mpsc::unbounded_channel::<Vec<Event>>();
    let rx: Arc<Mutex<UnboundedReceiver<Vec<Event>>>> = Arc::new(Mutex::new(rx));
    let context: Arc<ContextStruct> = Arc::new(ContextStruct {
        github: github_api,
        near: NearClient {},
    });

    // Spawn worker tasks
    for _ in 0..4 {
        let context = context.clone();
        let rx = Arc::clone(&rx);
        tokio::spawn(async move {
            loop {
                let events = {
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

    // TODO: load from database last time we checked
    let mut updated_from = chrono::Utc::now() - chrono::Duration::days(1);

    loop {
        interval.tick().await;
        let (events, max_time) = context.github.get_events(updated_from).await?;
        log::info!("Got {} events", events.len());

        let events_per_pr = events.into_iter().fold(
            std::collections::HashMap::new(),
            |mut map: HashMap<String, Vec<Event>>, event| {
                let pr = event.pr();
                map.entry(pr.full_id.clone()).or_default().push(event);
                map
            },
        );

        for (key, events) in events_per_pr.into_iter() {
            if let Err(e) = tx.send(events) {
                log::error!("Failed to send events from {}: {}", key, e);
            }
        }
        updated_from = max_time;
    }

    Ok(())
}

// Runs events from the same PR
async fn execute(context: Context, mut events: Vec<Event>) {
    events.sort_by_key(|event| *event.timestamp());
    for event in events {
        if let Err(e) = event.execute(context.clone()).await {
            log::error!("Failed to execute event for {}: {}", event.pr().full_id, e);
        }
    }
}
