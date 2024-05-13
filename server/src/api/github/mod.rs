use octocrab::models::{activity::Notification, pulls::PullRequest, NotificationId};
use tracing::{instrument, warn};

use crate::commands::{Command, ParseCommand};

mod types;
pub use types::*;

#[derive(Clone)]
pub struct GithubClient {
    octocrab: octocrab::Octocrab,
    pub user_handle: String,
}

impl GithubClient {
    pub async fn new(github_token: String) -> anyhow::Result<Self> {
        let octocrab = octocrab::Octocrab::builder()
            .personal_token(github_token)
            .build()?;
        let user_handle = octocrab.current().user().await?.login;

        Ok(Self {
            octocrab,
            user_handle,
        })
    }

    #[instrument(skip(self))]
    pub async fn get_events(&self) -> anyhow::Result<Vec<Command>> {
        let page = self
            .octocrab
            .activity()
            .notifications()
            .list()
            .all(false)
            .participating(true)
            .per_page(50)
            .page(0)
            .send()
            .await?;

        let events = self.octocrab.all_pages(page).await?;
        let interested_events = events.into_iter().filter(|notification| {
            notification.subject.r#type == "PullRequest"
                && (notification.reason == "mention" || notification.reason == "state_change")
        });

        let mut results = Vec::new();

        for event in interested_events {
            if event.reason != "mention" {
                // We are only interested in mentions
                continue;
            }

            let pr = self.get_pull_request_from_notification(&event).await;
            if pr.is_err() {
                warn!("Failed to get PR: {:?}", pr.err());
                continue;
            }
            let pr = pr.unwrap();
            let pr_metadata = types::PrMetadata::try_from(pr);
            if pr_metadata.is_err() {
                warn!("Failed to convert PR: {:?}", pr_metadata.err());
                continue;
            }
            let pr_metadata = pr_metadata.unwrap();

            let comments = self
                .octocrab
                .issues(&pr_metadata.owner, &pr_metadata.repo)
                .list_comments(pr_metadata.number)
                .per_page(100)
                .send()
                .await;

            if comments.is_err() {
                warn!("Failed to get comments: {:?}", comments.err());
                continue;
            }
            let comments = comments.unwrap();

            // TODO: think if we can avoid this and just load from the last page
            let comments = self.octocrab.all_pages(comments).await;
            if comments.is_err() {
                warn!("Failed to get all comments: {:?}", comments.err());
                continue;
            }
            let comments = comments.unwrap();

            for comment in comments.into_iter().rev() {
                if comment.user.login == self.user_handle {
                    // We have replied to last ask
                    break;
                }

                let event = Command::parse_command(&self.user_handle, &pr_metadata, &comment);
                if event.is_none() {
                    continue;
                }
                results.push(event.unwrap());
            }
            if let Err(_) = self.mark_notification_as_read(event.id).await {
                warn!("Failed to mark notification as read");
            }
        }

        Ok(results)
    }

    #[instrument(skip(self), fields(notification = notification.id.0))]
    pub async fn get_pull_request_from_notification(
        &self,
        notification: &Notification,
    ) -> anyhow::Result<PullRequest> {
        assert_eq!(notification.subject.r#type, "PullRequest");

        let pull_request = self
            .octocrab
            .get(
                notification
                    .subject
                    .url
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("No PR url"))?,
                None::<&()>,
            )
            .await?;

        Ok(pull_request)
    }

    #[instrument(skip(self))]
    pub async fn get_pull_request(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
    ) -> anyhow::Result<PullRequest> {
        let pull_request = self.octocrab.pulls(owner, repo).get(number).await?;

        Ok(pull_request)
    }

    #[instrument(skip(self, text))]
    pub async fn reply(&self, owner: &str, repo: &str, id: u64, text: &str) -> anyhow::Result<()> {
        self.octocrab
            .issues(owner, repo)
            .create_comment(id, text)
            .await?;

        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn like_comment(
        &self,
        owner: &str,
        repo: &str,
        comment_id: u64,
    ) -> anyhow::Result<()> {
        self.octocrab
            .issues(owner, repo)
            .create_comment_reaction(
                comment_id,
                octocrab::models::reactions::ReactionContent::PlusOne,
            )
            .await?;

        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn like_pr(&self, owner: &str, repo: &str, pr_number: u64) -> anyhow::Result<()> {
        self.octocrab
            .issues(owner, repo)
            .create_reaction(
                pr_number,
                octocrab::models::reactions::ReactionContent::PlusOne,
            )
            .await?;

        Ok(())
    }

    pub async fn mark_notification_as_read(
        &self,
        id: impl Into<NotificationId>,
    ) -> anyhow::Result<()> {
        self.octocrab
            .activity()
            .notifications()
            .mark_as_read(id.into())
            .await?;
        Ok(())
    }

    pub async fn edit_comment(
        &self,
        owner: &str,
        repo: &str,
        comment_id: u64,
        text: &str,
    ) -> anyhow::Result<()> {
        self.octocrab
            .issues(owner, repo)
            .update_comment(comment_id, text)
            .await?;
        Ok(())
    }

    pub async fn get_comment(
        &self,
        owner: &str,
        repo: &str,
        comment_id: u64,
    ) -> anyhow::Result<octocrab::models::issues::Comment> {
        let comment = self
            .octocrab
            .issues(owner, repo)
            .get_comment(comment_id)
            .await?;
        Ok(comment)
    }
}
