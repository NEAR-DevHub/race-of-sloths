use std::{
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

use chrono::NaiveDate;
use itertools::Itertools;
use rocket::fairing::AdHoc;
use rocket_db_pools::Database;
use shared::{telegram::TelegramSubscriber, GithubHandle};
use tracing::Level;

use crate::{db::DB, github_pull::GithubClient};

async fn calculate_pr_stats(
    db: &DB,
    github: &GithubClient,
    telegram: &TelegramSubscriber,
    period_string: &str,
    start_period: NaiveDate,
) -> anyhow::Result<()> {
    let projects = db.get_projects().await?;
    let mut project_stats = Vec::with_capacity(projects.len());
    let mut user_stats = std::collections::HashMap::<GithubHandle, (u32, u32)>::new();
    for (org, repo) in projects {
        let prs = github
            .pull_requests_for_period(&org, &repo, start_period)
            .await?;
        let mut sloths_prs = 0;
        let total_prs = prs.len();

        for pr in prs {
            let user = pr.user.map(|u| u.login).unwrap_or_default();
            let (total_prs, user_sloths_pr) = user_stats.entry(user).or_default();
            *total_prs += 1;

            if let Ok(true) = db.is_pr_available(&org, &repo, pr.number as i32).await {
                sloths_prs += 1;
                *user_sloths_pr += 1;
            }
        }

        project_stats.push((org, repo, total_prs, sloths_prs));
    }

    project_stats.sort_by(|(_, _, prs1, with_sloth1), (_, _, prs2, with_sloth2)| {
        let diff1 = *prs1 - *with_sloth1;
        let diff2 = *prs2 - *with_sloth2;
        diff2.cmp(&diff1)
    });

    let mut user_stats: Vec<(String, (u32, u32))> = user_stats.into_iter().collect();
    user_stats.sort_by(|(_, (prs1, with_sloth1)), (_, (prs2, with_sloth2))| {
        let diff1 = *prs1 - *with_sloth1;
        let diff2 = *prs2 - *with_sloth2;
        diff2.cmp(&diff1)
    });

    let message = [format!("{period_string} PR stats:")].into_iter().chain(project_stats.into_iter().take(10).map(|(org, repo, prs, prs_with_sloth)| format!(
        "- [{org}/{repo}](https://github.com/{org}/{repo}) - {prs} PRs, {prs_with_sloth} PRs with sloth, {} Difference\n",
        prs - prs_with_sloth
    ))).join("\n");

    let user_message = [String::from("User;Total PRs;Sloth PRs;Difference")]
        .into_iter()
        .chain(
            user_stats
                .into_iter()
                .map(|(user, (prs, prs_with_sloths))| {
                    format!("{user};{prs};{prs_with_sloths};{}", prs - prs_with_sloths)
                }),
        )
        .join("\n");

    telegram.send_to_telegram(&message, &Level::INFO);
    telegram.send_csv_file_to_telegram(user_message.into_bytes(), "user-stats.csv".to_string());

    Ok(())
}

pub fn stage(sleep_duration: Duration, atomic_bool: Arc<AtomicBool>) -> AdHoc {
    AdHoc::on_ignite("Weekly/Monthly stats", move |rocket| async move {
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
                        let mut count = 0;
                        while atomic_bool.load(std::sync::atomic::Ordering::Relaxed) {
                            interval.tick().await;

                            let (period, start_period) = if count % 4 == 0 {
                                ("Monthly", (chrono::Utc::now() - chrono::Duration::days(30)))
                            } else {
                                ("Weekly", (chrono::Utc::now() - chrono::Duration::days(7)))
                            };

                            if let Err(e) = calculate_pr_stats(
                                &db,
                                &github_client,
                                &telegram,
                                period,
                                start_period.date_naive(),
                            )
                            .await
                            {
                                telegram.send_to_telegram(
                                    &format!("Failed to calculate weekly stats: {:#?}", e),
                                    &Level::ERROR,
                                );

                                tracing::error!("Failed to calculate weekly stats: {:#?}", e);
                            }
                            count += 1;
                        }
                    });
                })
            },
        ))
    })
}
