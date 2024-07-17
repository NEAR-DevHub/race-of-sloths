use std::{
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

use octocrab::{models::pulls::PullRequest, Octocrab};
use rocket::fairing::AdHoc;
use rocket_db_pools::Database;
use shared::telegram::TelegramSubscriber;
use tracing::instrument;

use crate::db::DB;

struct RepoMetadata {
    stars: u32,
    forks: u32,
    open_issues: u32,
    primary_language: Option<String>,
}

pub struct GithubClient {
    pub octocrab: Octocrab,
}

impl GithubClient {
    pub fn new(github_token: String) -> anyhow::Result<Self> {
        let octocrab = octocrab::Octocrab::builder()
            .personal_token(github_token)
            .build()?;
        Ok(Self { octocrab })
    }

    #[instrument(skip(self))]
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

    pub async fn pull_requests_for_period(
        &self,
        org: &str,
        repo: &str,
        since: chrono::NaiveDate,
    ) -> anyhow::Result<Vec<PullRequest>> {
        let mut page = self
            .octocrab
            .pulls(org, repo)
            .list()
            .state(octocrab::params::State::All)
            .sort(octocrab::params::pulls::Sort::Updated)
            .direction(octocrab::params::Direction::Descending)
            .per_page(100)
            .send()
            .await?;
        let mut ret = page.take_items();
        while ret.last().map_or(false, |pr| {
            pr.updated_at
                .or(pr.created_at)
                .unwrap_or_default()
                .date_naive()
                >= since
        }) {
            let next_page = self.octocrab.get_page(&page.next).await?;
            if let Some(mut next_page) = next_page {
                ret.append(&mut next_page.take_items());
                page = next_page;
            } else {
                break;
            }
        }

        let ret = ret.into_iter().filter(|pr| {
            pr.updated_at
                .or(pr.created_at)
                .unwrap_or_default()
                .date_naive()
                >= since
        });

        Ok(ret.collect())
    }

    #[instrument(skip(self))]
    pub async fn get_user(&self, username: &str) -> anyhow::Result<octocrab::models::UserProfile> {
        Ok(self.octocrab.users(username).profile().await?)
    }
}

#[instrument(skip(telegram, github, db))]
async fn fetch_repos_metadata(
    telegram: &Arc<TelegramSubscriber>,
    github: &GithubClient,
    db: &DB,
) -> anyhow::Result<()> {
    let repos = db.get_repos().await?;
    for repo in repos {
        let metadata = match github.repo_metadata(&repo.organization, &repo.repo).await {
            Ok(metadata) => metadata,
            Err(e) => {
                crate::error(
                    telegram,
                    &format!(
                        "Failed to fetch repo metadata for {}/{}: {:#?}",
                        repo.organization, repo.repo, e
                    ),
                );
                continue;
            }
        };
        if let Err(e) = db
            .update_repo_metadata(
                repo.repo_id,
                metadata.stars,
                metadata.forks,
                metadata.open_issues,
                metadata.primary_language,
            )
            .await
        {
            crate::error(
                telegram,
                &format!(
                    "Failed to update repo metadata for {}/{}: {:#?}",
                    &repo.organization, &repo.repo, e
                ),
            );
        }
    }
    Ok(())
}

#[instrument(skip(telegram, github, db))]
async fn fetch_missing_user_organization_metadata(
    telegram: &Arc<TelegramSubscriber>,
    github: &GithubClient,
    db: &DB,
) -> anyhow::Result<()> {
    let users = db.get_users().await?;
    for user in users {
        if user.full_name.is_some() {
            // TODO: add user entry to sync cache
            continue;
        }

        let profile = match github.get_user(&user.login).await {
            Ok(profile) => profile,
            Err(e) => {
                crate::error(
                    telegram,
                    &format!("Failed to fetch user profile for {}: {:#?}", user.login, e),
                );
                continue;
            }
        };
        if let Some(full_name) = &profile.name {
            db.update_user_full_name(&user.login, full_name).await?;
        }
    }

    let orgs = db.get_organizations().await?;
    for org in orgs {
        if org.full_name.is_some() {
            continue;
        }

        let profile = match github.get_user(&org.login).await {
            Ok(profile) => profile,
            Err(e) => {
                crate::error(
                    telegram,
                    &format!(
                        "Failed to fetch organization profile for {}: {:#?}",
                        org.login, e
                    ),
                );
                continue;
            }
        };
        if let Some(full_name) = &profile.name {
            db.update_organization_full_name(&org.login, full_name)
                .await?;
        }
    }
    Ok(())
}

pub fn stage(
    github_client: GithubClient,
    sleep_duration: Duration,
    atomic_bool: Arc<AtomicBool>,
) -> AdHoc {
    AdHoc::on_ignite("GitHub metadata pull", move |rocket| async move {
        rocket
            .manage(Arc::new(github_client))
            .attach(AdHoc::on_liftoff(
                "Loads github repository metadata every X minutes",
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

                                // Execute a query of some kind
                                if let Err(e) =
                                    fetch_repos_metadata(&telegram, &github_client, &db).await
                                {
                                    rocket::error!(
                                        "Failed to fetch and store github data: {:#?}",
                                        e
                                    );
                                }

                                if let Err(e) = fetch_missing_user_organization_metadata(
                                    &telegram,
                                    &github_client,
                                    &db,
                                )
                                .await
                                {
                                    rocket::error!(
                                        "Failed to fetch and store github data: {:#?}",
                                        e
                                    );
                                }
                            }
                        });
                    })
                },
            ))
    })
}
