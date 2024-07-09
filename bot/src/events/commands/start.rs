use tracing::{debug, instrument};

use crate::messages::MsgCategory;

use shared::github::User;

use super::*;

#[derive(Debug, Clone)]
pub struct BotIncluded {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub user_comment_id: Option<u64>,
}

impl BotIncluded {
    pub fn new(timestamp: chrono::DateTime<chrono::Utc>, comment_id: Option<u64>) -> Self {
        Self {
            timestamp,
            user_comment_id: comment_id,
        }
    }
}

impl BotIncluded {
    #[instrument(skip(self, pr, context, info, sender), fields(pr = pr.full_id))]
    pub async fn execute(
        &self,
        pr: &PrMetadata,
        context: Context,
        info: PRInfo,
        sender: &User,
    ) -> anyhow::Result<bool> {
        if info.exist {
            debug!("Sloth is already included in {}. Skipping", pr.full_id,);
            return Ok(false);
        }

        if pr.merged.is_some() || pr.closed {
            debug!("PR {} is already merged. Skipping", pr.full_id,);
            context
                .reply_with_error(
                    pr,
                    self.user_comment_id,
                    MsgCategory::ErrorLateIncludeMessage,
                    vec![],
                )
                .await?;
            return Ok(false);
        }

        debug!("Starting PR {}", pr.full_id);
        context
            .near
            .send_start(pr, self.timestamp, sender.is_maintainer())
            .await?;

        if let Some(comment_id) = self.user_comment_id {
            context
                .github
                .like_comment(&pr.owner, &pr.repo, comment_id)
                .await?;
        }

        let check_info = PRInfo {
            exist: true,
            ..info
        };

        context.status_message(pr, None, check_info).await;

        Ok(false)
    }

    pub fn construct(comment: &CommentRepr) -> Command {
        Command::Include(BotIncluded::new(comment.timestamp, comment.comment_id))
    }

    pub fn parse_body(bot_name: &str, pr_metadata: &PrMetadata) -> Option<Command> {
        let body = pr_metadata.body.as_str();
        let bot_name = format!("@{}", bot_name);
        if !body.contains(&bot_name) {
            return None;
        }

        Some(Command::Include(Self {
            timestamp: pr_metadata.updated_at,
            user_comment_id: None,
        }))
    }
}
