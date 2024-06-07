use std::{
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

use octocrab::Octocrab;
use rocket::fairing::AdHoc;
use rocket_db_pools::Database;

use crate::db::DB;

struct RepoMetadata {
    stars: u32,
    forks: u32,
    open_issues: u32,
    primary_language: Option<String>,
}

pub struct GithubClient {
    octocrab: Octocrab,
}

impl GithubClient {
    pub fn new(github_token: String) -> anyhow::Result<Self> {
        let octocrab = octocrab::Octocrab::builder()
            .personal_token(github_token)
            .build()?;
        Ok(Self { octocrab })
    }

    async fn repo_metadata(&self, org: &str, repo: &str) -> anyhow::Result<RepoMetadata> {
        let repo = self.octocrab.repos(org, repo).get().await?;
        Ok(RepoMetadata {
            stars: repo.stargazers_count.unwrap_or_default(),
            forks: repo.forks_count.unwrap_or_default(),
            open_issues: repo.open_issues_count.unwrap_or_default(),
            primary_language: repo
                .language
                .and_then(|l| l.as_str().map(ToString::to_string)),
        })
    }
}

async fn fetch_repos_metadata(github: &GithubClient, db: &DB) -> anyhow::Result<()> {
    let repos = db.get_repos().await?;
    for repo in repos {
        let metadata = github.repo_metadata(&repo.organization, &repo.repo).await?;
        db.update_repo_metadata(
            repo.repo_id,
            metadata.stars,
            metadata.forks,
            metadata.open_issues,
            metadata.primary_language,
        )
        .await?;
    }
    Ok(())
}

pub fn stage(
    github_client: GithubClient,
    sleep_duration: Duration,
    atomic_bool: Arc<AtomicBool>,
) -> AdHoc {
    rocket::fairing::AdHoc::on_liftoff(
        "Loads github repository metadata every X minutes",
        move |rocket| {
            Box::pin(async move {
                // Get an actual DB connection
                let db = DB::fetch(rocket)
                    .expect("Failed to get DB connection")
                    .clone();

                rocket::tokio::spawn(async move {
                    let mut interval = rocket::tokio::time::interval(sleep_duration);
                    let github_client = github_client;
                    while atomic_bool.load(std::sync::atomic::Ordering::Relaxed) {
                        interval.tick().await;

                        // Execute a query of some kind
                        if let Err(e) = fetch_repos_metadata(&github_client, &db).await {
                            rocket::error!("Failed to fetch and store github data: {:#?}", e);
                        }
                    }
                });
            })
        },
    )
}
