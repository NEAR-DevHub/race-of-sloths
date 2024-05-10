use tracing::{debug, instrument};

use super::*;

fn msg(user: &str) -> String {
    format!("This pull request is a part of Sloth race now. Dear maintainer, please use `@{user} score [1-10]` to rate it, or `@{user} pause` to stop the sloth for the repository.")
}

#[derive(Debug, Clone)]
pub struct BotIncluded {
    pub sender: String,
    pub pr_metadata: PrMetadata,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub comment_id: u64,
    pub notification_id: u64,
}

impl BotIncluded {
    pub fn new(
        sender: String,
        pr_metadata: PrMetadata,
        timestamp: chrono::DateTime<chrono::Utc>,
        comment_id: u64,
        notification_id: u64,
    ) -> Self {
        Self {
            sender,
            pr_metadata,
            timestamp,
            comment_id,
            notification_id,
        }
    }

    pub fn is_accepted(&self) -> bool {
        self.pr_metadata.author.is_participant()
    }
}

#[async_trait::async_trait]
impl Execute for BotIncluded {
    #[instrument(skip(self, context), fields(pr = self.pr_metadata.full_id))]
    async fn execute(&self, context: Context) -> anyhow::Result<()> {
        let info = context.check_info(&self.pr_metadata).await?;
        if info.exist || !info.allowed_repo {
            debug!(
                "PR {} already exists or not allowed. Skipping",
                self.pr_metadata.full_id,
            );
            return context
                .github
                .mark_notification_as_read(self.notification_id)
                .await;
        }

        debug!("Starting PR {}", self.pr_metadata.full_id);

        context.near.send_start(&self.pr_metadata).await?;

        context
            .github
            .subscribe_to_repo(&self.pr_metadata.owner, &self.pr_metadata.repo)
            .await?;
        context
            .github
            .reply(
                &self.pr_metadata.owner,
                &self.pr_metadata.repo,
                self.pr_metadata.number,
                &msg(&context.github.user_handle),
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

impl ParseCommand for BotIncluded {
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

        if body.contains(format!("@{} include", bot_name).as_str()) {
            Some(Command::Include(BotIncluded::new(
                comment.user.login.clone(),
                pr_metadata.clone(),
                notification.updated_at,
                comment.id.0,
                notification.id.0,
            )))
        } else {
            None
        }
    }
}
