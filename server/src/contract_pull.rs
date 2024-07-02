use std::{
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

use chrono::DateTime;
use rocket::fairing::AdHoc;
use rocket_db_pools::Database;
use shared::{near::NearClient, telegram::TelegramSubscriber, TimePeriod};

use crate::db::DB;

async fn fetch_and_store_users(
    telegram: &Arc<TelegramSubscriber>,
    near_client: &NearClient,
    db: &DB,
) -> anyhow::Result<()> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let periods = [TimePeriod::Month, TimePeriod::Quarter, TimePeriod::AllTime]
        .into_iter()
        .map(|e| e.time_string(timestamp as u64))
        .collect();
    let users = near_client.users(periods).await?;
    for user in users {
        let user_id = match db
            .upsert_user(user.id, &user.name, user.percentage_bonus)
            .await
        {
            Ok(id) => id,
            Err(e) => {
                crate::error(
                    telegram,
                    &format!("Failed to upsert user ({}): {:#?}", user.name, e),
                );
                continue;
            }
        };
        for (period, data) in user.period_data {
            if let Err(e) = db.upsert_user_period_data(period, &data, user_id).await {
                crate::error(
                    telegram,
                    &format!(
                        "Failed to upsert user ({}) period data: {:#?}",
                        user.name, e
                    ),
                );
            }
        }
        for (streak_id, streak_data) in user.streaks {
            if let Err(e) = db
                .upsert_streak_user_data(&streak_data, streak_id as i32, user_id)
                .await
            {
                crate::error(
                    telegram,
                    &format!(
                        "Failed to upsert user ({}) streak data: {:#?}",
                        user.name, e
                    ),
                );
            }
        }
    }

    Ok(())
}

async fn fetch_and_store_prs(
    telegram: &Arc<TelegramSubscriber>,
    near_client: &NearClient,
    db: &DB,
) -> anyhow::Result<()> {
    let prs = near_client.prs().await?;
    // TODO: more efficient way to handle exclude and outdated PRs
    db.clear_prs().await?;
    for (pr, executed) in prs {
        let organization_id = db.upsert_organization(&pr.organization).await?;
        let repo_id = db.upsert_repo(organization_id, &pr.repo).await?;
        let author_id = db.get_user_id(&pr.author).await?;
        if let Err(e) = db
            .upsert_pull_request(
                repo_id,
                pr.number as i32,
                author_id,
                DateTime::from_timestamp_nanos(pr.created_at as i64).naive_utc(),
                pr.merged_at
                    .map(|t| DateTime::from_timestamp_nanos(t as i64).naive_utc()),
                pr.score(),
                pr.rating(),
                pr.percentage_multiplier,
                pr.streak_bonus_rating,
                executed,
            )
            .await
        {
            crate::error(
                telegram,
                &format!(
                    "Failed to upsert PR ({}/{}/pull/{}): {:#?}",
                    pr.organization, pr.repo, pr.number, e
                ),
            );
        }
    }
    Ok(())
}

async fn fetch_and_store_repos(
    telegram: &Arc<TelegramSubscriber>,
    near_client: &NearClient,
    db: &DB,
) -> anyhow::Result<()> {
    let organizations = near_client.repos().await?;
    for org in organizations {
        let organization_id = match db.upsert_organization(&org.organization).await {
            Ok(id) => id,
            Err(e) => {
                crate::error(
                    telegram,
                    &format!(
                        "Failed to upsert organization ({}): {:#?}",
                        org.organization, e
                    ),
                );
                continue;
            }
        };
        for repo in org.repos {
            if let Err(e) = db.upsert_repo(organization_id, &repo).await {
                crate::error(
                    telegram,
                    &format!(
                        "Failed to upsert repo ({}/{}): {:#?}",
                        org.organization, repo, e
                    ),
                );
            }
        }
    }
    Ok(())
}

// TODO: more efficient way to fetch only updated data
async fn fetch_and_store_all_data(
    telegram: &Arc<TelegramSubscriber>,
    near_client: &NearClient,
    db: &DB,
) -> anyhow::Result<()> {
    fetch_and_store_users(telegram, near_client, db).await?;

    fetch_and_store_repos(telegram, near_client, db).await?;
    // It matters that we fetch users first, because we need to know their IDs
    fetch_and_store_prs(telegram, near_client, db).await?;
    Ok(())
}

pub fn stage(client: NearClient, sleep_duration: Duration, atomic_bool: Arc<AtomicBool>) -> AdHoc {
    rocket::fairing::AdHoc::on_liftoff("Load users from Near every X minutes", move |rocket| {
        Box::pin(async move {
            // Get an actual DB connection
            let db = DB::fetch(rocket)
                .expect("Failed to get DB connection")
                .clone();
            let telegram: Arc<TelegramSubscriber> = rocket
                .state()
                .cloned()
                .expect("Failed to get telegram client");

            rocket::tokio::spawn(async move {
                let mut interval = rocket::tokio::time::interval(sleep_duration);
                let near_client = client;
                while atomic_bool.load(std::sync::atomic::Ordering::Relaxed) {
                    interval.tick().await;

                    // Execute a query of some kind
                    if let Err(e) = fetch_and_store_all_data(&telegram, &near_client, &db).await {
                        crate::error(
                            &telegram,
                            &format!("Failed to fetch and store data: {:#?}", e),
                        );
                    }
                }
            });
        })
    })
}
