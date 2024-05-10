use tracing::{debug, info, instrument};

use super::*;

#[derive(Clone, Debug)]
pub struct BotPaused {
    pub pr_metadata: PrMetadata,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub comment_id: u64,
}

#[async_trait::async_trait]
impl Execute for BotPaused {
    async fn execute(&self, context: Context) -> anyhow::Result<()> {
        let info = context.check_info(&self.pr_metadata).await?;
        if info.allowed_repo {
            debug!("Paused PR {}", self.pr_metadata.full_id);
            context
                .near
                .send_pause(&self.pr_metadata.owner, &self.pr_metadata.repo)
                .await?;
        }

        context
            .github
            .like_comment(
                &self.pr_metadata.owner,
                &self.pr_metadata.repo,
                self.comment_id,
            )
            .await
    }
}

impl ParseCommand for BotPaused {
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
        let command = format!("@{} pause", bot_name);

        if body.contains(&command) {
            return Some(Command::Pause(BotPaused {
                pr_metadata: pr_metadata.clone(),
                timestamp: notification.updated_at,
                comment_id: comment.id.0,
            }));
        }

        None
    }
}

#[derive(Clone, Debug)]
pub struct BotUnpaused {
    pub pr_metadata: PrMetadata,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub comment_id: u64,
}

#[async_trait::async_trait]
impl Execute for BotUnpaused {
    #[instrument(skip(self, context), fields(pr = self.pr_metadata.full_id))]
    async fn execute(&self, context: Context) -> anyhow::Result<()> {
        let info = context.check_info(&self.pr_metadata).await?;
        if !info.allowed_org {
            info!(
                "Tried to unpause a PR from not allowed org: {}. Skipping",
                self.pr_metadata.full_id
            );
            return Ok(());
        }

        if !info.allowed_repo {
            context
                .near
                .send_unpause(&self.pr_metadata.owner, &self.pr_metadata.repo)
                .await?;
            debug!("Unpaused PR {}", self.pr_metadata.full_id);
        }

        context
            .github
            .like_comment(
                &self.pr_metadata.owner,
                &self.pr_metadata.repo,
                self.comment_id,
            )
            .await
    }
}

impl ParseCommand for BotUnpaused {
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
        let command = format!("@{} unpause", bot_name);

        if body.contains(&command) {
            return Some(Command::Unpause(BotUnpaused {
                pr_metadata: pr_metadata.clone(),
                timestamp: notification.updated_at,
                comment_id: comment.id.0,
            }));
        }

        None
    }
}
