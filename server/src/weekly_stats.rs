use std::{
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

use rocket::{fairing::AdHoc, futures::future::join_all};
use rocket_db_pools::Database;
use shared::telegram::TelegramSubscriber;
use tracing::Level;

use crate::{db::DB, github_pull::GithubClient};

async fn calculate_weekly_pr_stats(
    db: &DB,
    github: &GithubClient,
    telegram: &TelegramSubscriber,
) -> anyhow::Result<()> {
    let projects = db.get_projects().await?;
    let mut project_stats = Vec::with_capacity(projects.len());
    for (org, repo) in projects {
        let period = (chrono::Utc::now() - chrono::Duration::days(7)).date_naive();
        let prs = github.pull_requests_for_period(&org, &repo, period).await?;
        let result = join_all(
            prs.iter()
                .map(|pr| db.is_pr_available(&org, &repo, pr.number as i32)),
        )
        .await
        .into_iter()
        .filter(|r| matches!(r, Ok(true)))
        .count();

        project_stats.push((org, repo, prs.len(), result));
    }

    project_stats.sort_by(|(_, _, prs1, with_sloth1), (_, _, prs2, with_sloth2)| {
        let diff1 = *prs1 - *with_sloth1;
        let diff2 = *prs2 - *with_sloth2;
        diff2.cmp(&diff1)
    });

    let mut message = String::from("Weekly PR stats:\n");
    for (org, repo, prs, prs_with_sloth) in project_stats.into_iter().take(10) {
        message.push_str(&format!(
            "- [{org}/{repo}](https://github.com/{org}/{repo}) - {prs} PRs, {prs_with_sloth} PRs with sloth, {} Difference\n",
            prs - prs_with_sloth
        ));
    }

    telegram.send_to_telegram(&message, &Level::INFO);

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
