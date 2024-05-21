use futures::future::join_all;
use octocrab::models::{
    activity::Notification, issues::Comment, pulls::PullRequest, CommentId, NotificationId,
};
use tracing::{error, info, instrument};

use crate::events::commands::Command;

mod types;
pub use types::*;

#[derive(Clone, Debug)]
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

        let fetch_pr_futures = events.into_iter().map(|event| async move {
            if event.subject.r#type != "PullRequest"
                || (event.reason != "mention" && event.reason != "state_change")
            {
                info!(
                    "Skipping event: {} with reason {}",
                    event.subject.r#type, event.reason
                );
                if let Err(_) = self.mark_notification_as_read(event.id).await {
                    error!(
                        "Failed to mark notification as read for event: {:?}",
                        event.id
                    );
                }
                return None;
            }

            let pr = self.get_pull_request_from_notification(&event).await;
            if let Err(e) = pr {
                error!("Failed to get PR: {:?}", e);
                return None;
            }
            let pr = pr.unwrap();

            let pr_metadata = types::PrMetadata::try_from(pr);
            if let Err(e) = pr_metadata {
                error!("Failed to convert PR: {:?}", e);
                return None;
            }
            let pr_metadata = pr_metadata.unwrap();

            let comments = self
                .octocrab
                .issues(&pr_metadata.owner, &pr_metadata.repo)
                .list_comments(pr_metadata.number)
                .per_page(100)
                .send()
                .await;

            if let Err(e) = comments {
                error!("Failed to get comments: {:?}", e);
                return None;
            }
            let comments = comments.unwrap();

            let comments = self.octocrab.all_pages(comments).await;
            if let Err(e) = comments {
                error!("Failed to get all comments: {:?}", e);
                return None;
            }
            let comments = comments.unwrap();

            let mut results = Vec::new();
            let mut found_us = false;
            for comment in comments.into_iter().rev() {
                if comment.user.login == self.user_handle {
                    found_us = true;
                    break;
                }

                let event = Command::parse_command(&self.user_handle, &pr_metadata, &comment);
                if let Some(event) = event {
                    results.push(event);
                }
            }

            // We haven
            if results.is_empty() && !found_us {
                if let Some(event) = Command::parse_body(&self.user_handle, &pr_metadata) {
                    results.push(event);
                }
            }

            if let Err(e) = self.mark_notification_as_read(event.id).await {
                error!("Failed to mark notification as read: {:?}", e);
            }

            Some(results)
        });

        let results = join_all(fetch_pr_futures)
            .await
            .into_iter()
            .flatten()
            .flatten()
            .collect();
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
    pub async fn reply(
        &self,
        owner: &str,
        repo: &str,
        id: u64,
        text: &str,
    ) -> anyhow::Result<Comment> {
        Ok(self
            .octocrab
            .issues(owner, repo)
            .create_comment(id, text)
            .await?)
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

    #[instrument(skip(self, text))]
    pub async fn edit_comment(
        &self,
        owner: &str,
        repo: &str,
        comment_id: u64,
        text: &str,
    ) -> anyhow::Result<()> {
        self.octocrab
            .issues(owner, repo)
            .update_comment(CommentId(comment_id), text)
            .await?;
        Ok(())
    }
}
