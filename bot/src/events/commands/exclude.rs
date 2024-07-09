use tracing::{debug, instrument};

use crate::messages::MsgCategory;

use shared::github::User;

use super::*;

#[derive(Debug, Clone)]
pub struct BotExcluded {
    pub author: User,
    pub comment_id: Option<u64>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl BotExcluded {
    #[instrument(skip(self, pr, context, _check_info), fields(pr = pr.full_id))]
    pub async fn execute(
        &self,
        pr: &PrMetadata,
        context: Context,
        _check_info: PRInfo,
    ) -> anyhow::Result<bool> {
        if !self.author.is_maintainer() {
            info!(
                "Tried to exclude a PR from not maintainer: {}. Skipping",
                pr.full_id
            );
            context
                .reply_with_error(
                    pr,
                    self.comment_id,
                    MsgCategory::ErrorRightsViolationMessage,
                    vec![],
                )
                .await?;
            return Ok(false);
        }

        debug!("Excluding PR {}", pr.full_id);

        context.near.send_exclude(pr).await?;
        context
            .reply(pr, self.comment_id, MsgCategory::ExcludeMessages, vec![])
            .await?;
        Ok(true)
    }

    pub fn construct(comment: &CommentRepr) -> Command {
        Command::Excluded(BotExcluded {
            author: comment.user.clone(),
            comment_id: comment.comment_id,
            timestamp: comment.timestamp,
        })
    }
}
