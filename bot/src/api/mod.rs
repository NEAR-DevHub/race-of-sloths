use std::sync::Arc;

use futures::future::join_all;
use octocrab::models::{
    activity::Notification,
    issues::Comment,
    pulls::{PullRequest, Review, ReviewState},
    AuthorAssociation, CommentId, NotificationId, RateLimit,
};
use shared::GithubHandle;
use tracing::{error, info, instrument};

use crate::events::{actions::Action, commands::Command, Event, EventType};

pub use shared::github::*;

pub mod prometheus;

#[derive(Clone)]
pub struct GithubClient {
    octocrab: octocrab::Octocrab,
    prometheus: Arc<prometheus::PrometheusClient>,
    pub user_handle: String,
}

#[derive(Debug, Clone)]
pub struct CommentRepr {
    pub id: u64,
    pub user: User,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub text: String,
    pub comment_id: Option<u64>,
}

impl From<Comment> for CommentRepr {
    fn from(comment: Comment) -> Self {
        Self {
            id: comment.id.0,
            user: User::new(comment.user.login, comment.author_association),
            timestamp: comment.updated_at.unwrap_or(comment.created_at),
            comment_id: Some(comment.id.0),
            text: comment
                .body
                .or(comment.body_html)
                .or(comment.body_text)
                .unwrap_or_default(),
        }
    }
}

impl TryFrom<Review> for CommentRepr {
    type Error = ();
    fn try_from(review: Review) -> Result<Self, ()> {
        let user = review.user.ok_or(())?;
        Ok(Self {
            id: review.id.0,
            user: User::new(user.login, AuthorAssociation::Contributor),
            timestamp: review.submitted_at.unwrap_or_else(chrono::Utc::now),
            comment_id: None,
            text: review
                .body
                .or(review.body_html)
                .or(review.body_text)
                .unwrap_or_default(),
        })
    }
}

impl GithubClient {
    pub async fn new(
        github_token: String,
        prometheus: Arc<prometheus::PrometheusClient>,
    ) -> anyhow::Result<Self> {
        let octocrab = octocrab::Octocrab::builder()
            .personal_token(github_token)
            .build()?;
        let user_handle = octocrab.current().user().await?.login;

        Ok(Self {
            octocrab,
            user_handle,
            prometheus,
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

            let merged_by = pr.merged_by.clone();
            let pr_metadata = match PrMetadata::try_from(pr) {
                Ok(pr) => pr,
                Err(e) => {
                    error!("Failed to convert PR: {:?}", e);
                    return None;
                }
            };

            if pr_metadata.merged.is_none() && pr_metadata.closed {
                info!("PR is closed: {}", pr_metadata.number);
                if let Err(e) = self.mark_notification_as_read(event.id).await {
                    error!(
                        "Failed to mark notification as read for event: {:?}: {e:?}",
                        event.id
                    );
                }

                return Some(vec![Event {
                    event: EventType::Action(Action::stale()),
                    pr: pr_metadata.clone(),
                    comment: None,
                    event_time: pr_metadata.updated_at,
                }]);
            }

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
            let first_bot_comment = comments
                .iter()
                .find(|c| c.user.login == self.user_handle)
                .cloned()
                .map(Into::into);

            let reviews = self
                .octocrab
                .pulls(&pr_metadata.owner, &pr_metadata.repo)
                .list_reviews(pr_metadata.number)
                .per_page(100)
                .send()
                .await;
            let reviews = match reviews {
                Ok(reviews) => reviews,
                Err(e) => {
                    error!("Failed to get reviews: {:?}", e);
                    return None;
                }
            };

            let mut comments = comments
                .into_iter()
                .map(CommentRepr::from)
                .chain(reviews.into_iter().flat_map(CommentRepr::try_from))
                .collect::<Vec<_>>();
            comments.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

            let mut results = Vec::new();

            for comment in comments.into_iter().rev() {
                // We have processed older messages
                if comment.user.login == self.user_handle {
                    break;
                }

                if let Some(command) =
                    Command::parse_command(&self.user_handle, &pr_metadata, &comment)
                {
                    results.push(Event {
                        event: EventType::Command {
                            command,
                            notification_id: event.id,
                            sender: comment.user.clone(),
                        },
                        pr: pr_metadata.clone(),
                        comment: first_bot_comment.clone(),
                        event_time: comment.timestamp,
                    });
                }
            }

            if first_bot_comment.is_none() {
                if let Some(command) = Command::parse_body(&self.user_handle, &pr_metadata) {
                    results.push(Event {
                        event: EventType::Command {
                            command,
                            notification_id: event.id,
                            sender: pr_metadata.author.clone(),
                        },
                        pr: pr_metadata.clone(),
                        comment: first_bot_comment.clone(),
                        event_time: pr_metadata.started,
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
                let reviewers = self
                    .get_positive_or_pending_review(
                        &pr_metadata.owner,
                        &pr_metadata.repo,
                        pr_metadata.number,
                    )
                    .await
                    .unwrap_or_default();
                let merged_by = merged_by
                    .map(|e| e.login)
                    .unwrap_or_else(|| pr_metadata.author.login.clone());

                results.push(Event {
                    event: EventType::Action(Action::merge(merged_by, reviewers)),
                    pr: pr_metadata.clone(),
                    comment: first_bot_comment,
                    event_time: pr_metadata.merged.unwrap(),
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

    pub async fn get_positive_or_pending_review(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
    ) -> anyhow::Result<Vec<GithubHandle>> {
        Ok(self
            .octocrab
            .pulls(owner, repo)
            .list_reviews(number)
            .per_page(10)
            .send()
            .await?
            .take_items()
            .into_iter()
            .flat_map(|e| match e.state {
                Some(ReviewState::Pending) | Some(ReviewState::Approved) => e.user.map(|u| u.login),
                _ => None,
            })
            .collect())
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
        self.prometheus.add_write_request();
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
        self.prometheus.add_write_request();
        self.octocrab
            .issues(owner, repo)
            .create_comment_reaction(
                comment_id,
                octocrab::models::reactions::ReactionContent::PlusOne,
            )
            .await?;

        Ok(())
    }

    pub async fn mark_notification_as_read(
        &self,
        id: impl Into<NotificationId>,
    ) -> anyhow::Result<()> {
        self.prometheus.add_write_request();
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
        self.prometheus.add_write_request();

        self.octocrab
            .issues(owner, repo)
            .update_comment(CommentId(comment_id), text)
            .await?;
        Ok(())
    }

    #[instrument(skip(self,))]
    pub async fn get_bot_comment(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> anyhow::Result<Option<CommentRepr>> {
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
                    return Ok(Some(comment.into()));
                }
            }

            if let Some(next) = self.octocrab.get_page(&page.next).await? {
                page = next;
            } else {
                return Ok(None);
            }
        }
    }

    pub async fn get_rate_limits(&self) -> anyhow::Result<RateLimit> {
        Ok(self.octocrab.ratelimit().get().await?)
    }

    /// Active PR is the PR where there are >2 messages from other users (exculding us and the author)
    pub async fn is_active_pr(
        &self,
        owner: &str,
        repo: &str,
        author: &str,
        number: u64,
    ) -> anyhow::Result<bool> {
        let comments = self
            .octocrab
            .issues(owner, repo)
            .list_comments(number)
            .per_page(100)
            .send()
            .await?;

        let comments = self.octocrab.all_pages(comments).await?;

        let active = comments
            .iter()
            .filter(|c| c.user.login != self.user_handle && c.user.login != author)
            .count();

        Ok(active > 2)
    }
}
