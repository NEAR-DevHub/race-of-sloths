#[macro_use]
extern crate rocket;

mod entrypoints;

use std::sync::Arc;
use std::time::Duration;

use chrono::DateTime;
use rocket_db_pools::Database;
use shared::near::NearClient;
use shared::TimePeriod;

use race_of_sloths_server::db::{self, DB};

#[derive(Debug, serde::Deserialize)]
pub struct Env {
    contract: String,
    secret_key: String,
    is_mainnet: bool,
    sleep_duration_in_minutes: Option<u32>,
}

#[launch]
async fn rocket() -> _ {
    dotenv::dotenv().ok();

    let env = envy::from_env::<Env>().expect("Failed to load environment variables");
    let sleep_duration =
        Duration::from_secs(env.sleep_duration_in_minutes.unwrap_or(10) as u64 * 60);
    let atomic_bool = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let atomic_bool_clone = atomic_bool.clone();
    let prometheus = rocket_prometheus::PrometheusMetrics::new();

    rocket::build()
        .attach(db::stage())
        .attach(rocket::fairing::AdHoc::on_liftoff(
            "Load users from Near every X minutes",
            move |rocket| {
                Box::pin(async move {
                    // Get an actual DB connection
                    let db = DB::fetch(rocket)
                        .expect("Failed to get DB connection")
                        .clone();

                    rocket::tokio::spawn(async move {
                        let mut interval = rocket::tokio::time::interval(sleep_duration);
                        let near_client = NearClient::new(
                            env.contract.clone(),
                            env.secret_key.clone(),
                            env.is_mainnet,
                        )
                        .await
                        .expect("Failed to create Near client");
                        while atomic_bool.load(std::sync::atomic::Ordering::Relaxed) {
                            interval.tick().await;

                            // Execute a query of some kind
                            if let Err(e) = fetch_and_store_all_data(&near_client, &db).await {
                                rocket::error!("Failed to fetch and store data: {:#?}", e);
                            }
                        }
                    });
                })
            },
        ))
        .attach(rocket::fairing::AdHoc::on_shutdown(
            "Stop loading users from Near",
            |_| {
                Box::pin(async move {
                    atomic_bool_clone.store(false, std::sync::atomic::Ordering::Relaxed);
                })
            },
        ))
        .attach(prometheus.clone())
        .attach(entrypoints::stage())
        .mount("/metrics", prometheus)
}

async fn fetch_and_store_users(near_client: &NearClient, db: &DB) -> anyhow::Result<()> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let periods = [TimePeriod::Month, TimePeriod::Quarter, TimePeriod::AllTime]
        .into_iter()
        .map(|e| e.time_string(timestamp as u64))
        .collect();
    let users = near_client.users(periods).await?;
    for user in users {
        let user_id = db.upsert_user(&user.name).await?;
        for (period, data) in user.period_data {
            db.upsert_user_period_data(period, &data, user_id).await?;
        }
        for (streak_id, streak_data) in user.streaks {
            db.upsert_streak_user_data(&streak_data, streak_id as i32, user_id)
                .await?;
        }
    }

    Ok(())
}

async fn fetch_and_store_prs(near_client: &NearClient, db: &DB) -> anyhow::Result<()> {
    let prs = near_client.prs().await?;
    for (pr, executed) in prs {
        let organization_id = db.upsert_organization(&pr.organization).await?;
        let repo_id = db.upsert_repo(organization_id, &pr.repo).await?;
        let author_id = db.upsert_user(&pr.author).await?;
        let _ = db
            .upsert_pull_request(
                repo_id,
                pr.number as i32,
                author_id,
                DateTime::from_timestamp_nanos(pr.created_at as i64).naive_utc(),
                pr.merged_at
                    .map(|t| DateTime::from_timestamp_nanos(t as i64).naive_utc()),
                pr.score(),
                executed,
            )
            .await?;
    }
    Ok(())
}

async fn fetch_and_store_all_data(near_client: &NearClient, db: &DB) -> anyhow::Result<()> {
    fetch_and_store_users(near_client, db).await?;
    fetch_and_store_prs(near_client, db).await?;
    Ok(())
}
