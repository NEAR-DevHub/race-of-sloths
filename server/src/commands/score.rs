use tracing::{debug, instrument};

use self::api::github::User;

use super::*;

#[derive(Debug, Clone)]
pub struct BotScored {
    pub sender: User,
    pub pr_metadata: PrMetadata,
    pub score: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub comment_id: u64,
    pub notification_id: u64,
}

impl BotScored {
    pub fn new(
        sender: User,
        pr_metadata: PrMetadata,
        score: String,
        timestamp: chrono::DateTime<chrono::Utc>,
        comment_id: u64,
        notification_id: u64,
    ) -> Self {
        Self {
            sender,
            pr_metadata,
            score,
            timestamp,
            comment_id,
            notification_id,
        }
    }

    pub fn is_valid_score(&self) -> bool {
        if let Ok(number) = self.score.parse::<u8>() {
            (1..=5).contains(&number)
        } else {
            false
        }
    }

    pub fn is_accepted(&self) -> bool {
        self.pr_metadata.author.is_participant()
            && self.sender.is_maintainer()
            && self.is_valid_score()
            && self.pr_metadata.author.login != self.sender.login
    }
}

#[async_trait::async_trait]
impl Execute for BotScored {
    #[instrument(skip(self, context), fields(pr = self.pr_metadata.full_id, score = self.score))]
    async fn execute(&self, context: Context) -> anyhow::Result<()> {
        let info = context.check_info(&self.pr_metadata).await?;
        if !info.allowed_repo || !info.exist || info.executed {
            debug!(
                "PR {} is not started or not allowed or already executed. Skipping.",
                self.pr_metadata.full_id,
            );
            return context
                .github
                .mark_notification_as_read(self.notification_id)
                .await;
        }

        debug!("Scoring PR {}", self.pr_metadata.full_id);

        let score = self.score.parse::<u8>()?;
        if score < 1 || score > 10 {
            context
                .reply_with_error(
                    &self.pr_metadata.owner,
                    &self.pr_metadata.repo,
                    self.pr_metadata.number,
                    "Score should be between 1 and 10",
                )
                .await?;
            return context
                .github
                .mark_notification_as_read(self.notification_id)
                .await;
        }

        if self.pr_metadata.author.login == self.sender.login || !self.sender.is_maintainer() {
            context
                .reply_with_error(
                    &self.pr_metadata.owner,
                    &self.pr_metadata.repo,
                    self.pr_metadata.number,
                    "Only maintainers can score PRs, and you can't score your own PRs.",
                )
                .await?;
            return context
                .github
                .mark_notification_as_read(self.notification_id)
                .await;
        }

        context
            .near
            .send_scored(&self.pr_metadata, &self.sender.login, score as u64)
            .await?;

        context
            .github
            .reply(
                &self.pr_metadata.owner,
                &self.pr_metadata.repo,
                self.pr_metadata.number,
                "Thanks for submitting your score for the Sloth race.",
            )
            .await?;
        context
            .github
            .like_comment(
                &self.pr_metadata.owner,
                &self.pr_metadata.repo,
                self.comment_id,
            )
            .await?;
        context
            .github
            .mark_notification_as_read(self.notification_id)
            .await
    }
}

impl ParseCommand for BotScored {
    fn parse_command(
        bot_name: &str,
        notification: &Notification,
        pr_metadata: &PrMetadata,
        comment: &Comment,
    ) -> Option<Command> {
        let body = comment
            .body
            .as_ref()
            .or(comment.body_html.as_ref())
            .or(comment.body_text.as_ref())?;

        let phrase = format!("@{} score", bot_name);
        if let Some(result) = body.find(&phrase) {
            Some(Command::Score(BotScored::new(
                User {
                    login: comment.user.login.clone(),
                    contributor_type: comment.author_association.clone(),
                },
                pr_metadata.clone(),
                body[result + phrase.len()..].trim().to_string(),
                notification.updated_at,
                comment.id.0,
                notification.id.0,
            )))
        } else {
            None
        }
    }
}
