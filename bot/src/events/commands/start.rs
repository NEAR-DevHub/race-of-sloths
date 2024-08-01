use chrono::Duration;
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
        info: &mut PRInfo,
        sender: &User,
    ) -> anyhow::Result<EventResult> {
        if info.exist {
            debug!("Sloth is already included in {}. Skipping", pr.full_id,);
            // Let's update the status message just in case
            return Ok(EventResult::success(true));
        }

        match (pr.merged, pr.closed) {
            (Some(merged_at), _) if (chrono::Utc::now() - merged_at) < Duration::days(1) => {}
            (_, false) => {}
            _ => {
                debug!("PR {} is already merged. Skipping", pr.full_id,);
                context
                    .reply_with_error(
                        pr,
                        self.user_comment_id,
                        MsgCategory::ErrorLateIncludeMessage,
                        vec![],
                    )
                    .await?;
                return Ok(EventResult::RepliedWithError);
            }
        };

        if info.excluded && !sender.is_maintainer() {
            debug!(
                "Tried to include an excluded PR from not maintainer: {}. Skipping",
                pr.full_id
            );
            context
                .reply_with_error(
                    pr,
                    self.user_comment_id,
                    MsgCategory::ErrorRightsViolationMessage,
                    vec![],
                )
                .await?;
            return Ok(EventResult::RepliedWithError);
        }

        debug!("Starting PR {}", pr.full_id);
        context.near.send_start(pr, sender.is_maintainer()).await?;
        info.exist = true;
        info.excluded = false;

        if let Some(comment_id) = self.user_comment_id {
            context
                .github
                .like_comment(&pr.owner, &pr.repo, comment_id)
                .await?;
        }

        context.status_message(pr, None, info.clone(), None).await;

        Ok(EventResult::success(false))
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
