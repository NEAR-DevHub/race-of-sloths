use std::{
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

use anyhow::Context;
use chrono::DateTime;
use rocket::fairing::AdHoc;
use rocket_db_pools::Database;
use shared::{near::NearClient, telegram::TelegramSubscriber, TimePeriod};
use sqlx::{Postgres, Transaction};

use crate::db::DB;

async fn fetch_and_store_users(
    near_client: &NearClient,
    tx: &mut Transaction<'static, Postgres>,
) -> anyhow::Result<()> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let periods = [TimePeriod::Month, TimePeriod::Quarter, TimePeriod::AllTime]
        .into_iter()
        .map(|e| e.time_string(timestamp as u64))
        .collect();
    let users = near_client
        .users(periods)
        .await
        .context("Failed to fetch users")?;

    for user in users {
        let user_id = DB::upsert_user(tx, user.id, &user.name, user.percentage_bonus)
            .await
            .with_context(|| format!("Failed to upsert user with id: {}", user.id))?;
        for (period, data) in user.period_data {
            DB::upsert_user_period_data(tx, period, &data, user_id)
                .await
                .with_context(|| {
                    format!("Failed to upsert period data for user id: {}", user_id)
                })?;
        }
        for (streak_id, streak_data) in user.streaks {
            DB::upsert_streak_user_data(tx, &streak_data, streak_id as i32, user_id)
                .await
                .with_context(|| {
                    format!(
                        "Failed to upsert streak data for user id: {} and streak id: {}",
                        user_id, streak_id
                    )
                })?;
        }
    }

    Ok(())
}

async fn fetch_and_store_prs(
    telegram: &Arc<TelegramSubscriber>,
    near_client: &NearClient,
    tx: &mut Transaction<'static, Postgres>,
) -> anyhow::Result<()> {
    let prs = near_client
        .prs()
        .await
        .context("Failed to fetch PRs from near_client")?;

    DB::remove_non_existent_prs(tx, &prs).await?;

    for (pr, executed) in prs {
        let Some((_, repo_id)) = DB::get_organization_repo_id(tx, &pr.organization, &pr.repo)
            .await
            .context("Failed on getting org/repo")?
        else {
            crate::error(
                telegram,
                &format!(
                    "Pull request in repo({}) or organization({}) that does not exist, skipping.",
                    pr.repo, pr.organization
                ),
            );
            continue;
        };

        let author_id = DB::get_user_id(tx, &pr.author)
            .await
            .context("Failed on getting user id")?;

        DB::upsert_pull_request(
            tx,
            repo_id,
            pr.number as i32,
            author_id,
            DateTime::from_timestamp_nanos(pr.included_at as i64).naive_utc(),
            DateTime::from_timestamp_nanos(pr.created_at.unwrap_or_default() as i64).naive_utc(),
            pr.merged_at
                .map(|t| DateTime::from_timestamp_nanos(t as i64).naive_utc()),
            pr.score(),
            pr.rating(),
            pr.percentage_multiplier,
            pr.streak_bonus_rating,
            executed,
        )
        .await
        .context("Failed on upserting PR")?;
    }

    Ok(())
}

async fn fetch_and_store_repos(
    near_client: &NearClient,
    tx: &mut Transaction<'static, Postgres>,
) -> anyhow::Result<()> {
    let organizations = near_client.repos().await?;
    for org in organizations {
        let organization_id = DB::upsert_organization(tx, &org.organization)
            .await
            .context("Failed on upserting organization")?;
        for repo in org.repos {
            DB::upsert_repo(tx, organization_id, &repo)
                .await
                .context("Failed on upserting repo")?;
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
    let mut tx = db.begin().await?;
    fetch_and_store_users(near_client, &mut tx)
        .await
        .context("Failed to fetch and store users")?;
    tx.commit().await?;

    let mut tx = db.begin().await?;
    fetch_and_store_repos(near_client, &mut tx)
        .await
        .context("Failed to fetch and store repositories")?;
    tx.commit().await?;

    let mut tx = db.begin().await?;
    fetch_and_store_prs(telegram, near_client, &mut tx)
        .await
        .context("Failed to fetch and store pull requests")?;

    tx.commit().await?;
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
                        crate::error(&telegram, &format!("{e:#}"));
                    }
                }
            });
        })
    })
}
