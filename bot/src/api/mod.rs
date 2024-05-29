use futures::future::join_all;
use octocrab::models::{
    activity::Notification, issues::Comment, pulls::PullRequest, CommentId, NotificationId,
};
use tracing::{error, info, instrument};

use crate::events::{actions::Action, commands::Command, Event, EventType};

pub use shared::github::*;

pub mod prometheus;

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
    pub async fn get_events(&self) -> anyhow::Result<Vec<Event>> {
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
                if let Err(e) = self.mark_notification_as_read(event.id).await {
                    error!(
                        "Failed to mark notification as read for event: {:?}: {e:?}",
                        event.id
                    );
                }
                return None;
            }

            let pr = match self.get_pull_request_from_notification(&event).await {
                Ok(pr) => pr,
                Err(e) => {
                    error!("Failed to get PR: {:?}", e);
                    return None;
                }
            };

            let pr_metadata = match PrMetadata::try_from(pr) {
                Ok(pr) => pr,
                Err(e) => {
                    error!("Failed to convert PR: {:?}", e);
                    return None;
                }
            };

            let comments = self
                .octocrab
                .issues(&pr_metadata.owner, &pr_metadata.repo)
                .list_comments(pr_metadata.number)
                .per_page(100)
                .send()
                .await;

            let comments = match comments {
                Ok(comments) => comments,
                Err(e) => {
                    error!("Failed to get comments: {:?}", e);
                    return None;
                }
            };

            let comments = match self.octocrab.all_pages(comments).await {
                Ok(comments) => comments,
                Err(e) => {
                    error!("Failed to get all comments: {:?}", e);
                    return None;
                }
            };
            let comment_id = comments
                .iter()
                .find(|c| c.user.login == self.user_handle)
                .map(|c| c.id);

            let mut results = Vec::new();
            let mut found_us = false;

            for comment in comments.into_iter().rev() {
                if comment.user.login == self.user_handle {
                    found_us = true;
                    break;
                }

                if let Some(command) =
                    Command::parse_command(&self.user_handle, &pr_metadata, &comment)
                {
                    results.push(Event {
                        event: EventType::Command {
                            command,
                            notification_id: event.id,
                            sender: User::new(comment.user.login, comment.author_association),
                        },
                        pr: pr_metadata.clone(),
                        comment_id,
                    });
                }
            }

            // We haven
            if results.is_empty() && !found_us {
                if let Some(command) = Command::parse_body(&self.user_handle, &pr_metadata) {
                    results.push(Event {
                        event: EventType::Command {
                            command,
                            notification_id: event.id,
                            sender: pr_metadata.author.clone(),
                        },
                        pr: pr_metadata.clone(),
                        comment_id,
                    });
                }
            }

            if results.is_empty() {
                info!("No commands found in PR: {}", pr_metadata.number);
                if let Err(e) = self.mark_notification_as_read(event.id).await {
                    error!("Failed to mark notification as read: {:?}", e);
                }
            }

            // To keep the chronological order, we reverse the results
            results.reverse();

            if pr_metadata.merged.is_some() {
                results.push(Event {
                    event: EventType::Action(Action::merge()),
                    pr: pr_metadata.clone(),
                    comment_id,
                });
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

    #[instrument(skip(self, comment_id))]
    pub async fn delete_comment(
        &self,
        owner: &str,
        repo: &str,
        comment_id: CommentId,
    ) -> anyhow::Result<()> {
        self.octocrab
            .issues(owner, repo)
            .delete_comment(comment_id)
            .await?;
        Ok(())
    }

    #[instrument(skip(self,))]
    pub async fn get_comment_id(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> anyhow::Result<Option<CommentId>> {
        let mut page = self
            .octocrab
            .issues(owner, repo)
            .list_comments(pr_number)
            .per_page(100)
            .send()
            .await?;

        loop {
            let items = page.take_items();
            for comment in items {
                if comment.user.login == self.user_handle {
                    return Ok(Some(comment.id));
                }
            }

            if let Some(next) = self.octocrab.get_page(&page.next).await? {
                page = next;
            } else {
                return Ok(None);
            }
        }
    }
}
