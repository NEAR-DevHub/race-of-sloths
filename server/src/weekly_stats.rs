use std::{
    collections::HashMap,
    sync::{atomic::AtomicBool, Arc},
    thread::sleep,
    time::Duration,
};

use octocrab::models::pulls::PullRequest;
use rocket::{fairing::AdHoc, futures::future::join_all};
use rocket_db_pools::Database;
use serde::{Deserialize, Serialize};
use shared::telegram::TelegramSubscriber;
use tracing::Level;

use crate::{db::DB, github_pull::GithubClient};

#[derive(Serialize, Deserialize)]
pub struct Prs {
    pub org_repo: String,
    pub url: String,
    pub user_login: String,
}

async fn calculate_weekly_pr_stats(
    db: &DB,
    github: &GithubClient,
    telegram: &TelegramSubscriber,
) -> anyhow::Result<()> {
    rocket::tokio::time::sleep(Duration::from_secs(10)).await;
    let projects = db.get_projects().await?;
    let file = std::fs::File::create("prs.csv")?;
    let mut writer = csv::Writer::from_writer(file);
    for (org, repo) in projects {
        let period = (chrono::Utc::now() - chrono::Duration::days(7)).date_naive();
        let prs = github.pull_requests_for_period(&org, &repo, period).await?;

        for pr in prs {
            if pr.closed_at.is_some()
                || db
                    .is_pr_available(&org, &repo, pr.number as i32)
                    .await
                    .unwrap_or_default()
            {
                continue;
            }
            writer
                .serialize(Prs {
                    org_repo: format!("{org}/{repo}"),
                    url: format!("https://github.com/{org}/{repo}/pull/{}", pr.number),
                    user_login: pr.user.unwrap().login,
                })
                .unwrap();
        }
    }

    panic!("Done");
    Ok(())
}

pub fn stage(sleep_duration: Duration, atomic_bool: Arc<AtomicBool>) -> AdHoc {
    AdHoc::on_ignite("Weekly stats", move |rocket| async move {
        rocket.attach(AdHoc::on_liftoff(
            "Analyzes weekly statistics",
            move |rocket| {
                Box::pin(async move {
                    // Get an actual DB connection
                    let db = DB::fetch(rocket)
                        .expect("Failed to get DB connection")
                        .clone();
                    let github_client: Arc<GithubClient> = rocket
                        .state()
                        .cloned()
                        .expect("Failed to get github client");
                    let telegram: Arc<TelegramSubscriber> = rocket
                        .state()
                        .cloned()
                        .expect("failed to get telegram client");
                    rocket::tokio::spawn(async move {
                        let mut interval: rocket::tokio::time::Interval =
                            rocket::tokio::time::interval(sleep_duration);
                        while atomic_bool.load(std::sync::atomic::Ordering::Relaxed) {
                            interval.tick().await;
                            if let Err(e) =
                                calculate_weekly_pr_stats(&db, &github_client, &telegram).await
                            {
                                telegram.send_to_telegram(
                                    &format!("Failed to calculate weekly stats: {:#?}", e),
                                    &Level::ERROR,
                                );

                                tracing::error!("Failed to calculate weekly stats: {:#?}", e);
                            }
                        }
                    });
                })
            },
        ))
    })
}
