use tracing::{debug, info, instrument};

use crate::messages::MsgCategory;

use shared::github::User;

use super::*;

#[derive(Clone, Debug)]
pub struct BotPaused {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub comment_id: Option<u64>,
}

impl BotPaused {
    #[instrument(skip(self, pr, context, check_info, sender), fields(pr = pr.full_id))]
    pub async fn execute(
        &self,
        pr: &PrMetadata,
        context: Context,
        check_info: PRInfo,
        sender: &User,
    ) -> anyhow::Result<EventResult> {
        if check_info.paused {
            info!(
                "Tried to pause a PR from paused repo: {}. Skipping",
                pr.full_id
            );
            context
                .reply_with_error(
                    pr,
                    self.comment_id,
                    MsgCategory::ErrorPausePausedMessage,
                    vec![],
                )
                .await?;
            return Ok(EventResult::RepliedWithError);
        }

        if !sender.is_maintainer() {
            info!(
                "Tried to pause a PR from not maintainer: {}. Skipping",
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
            return Ok(EventResult::RepliedWithError);
        }

        debug!("Pausing the repository in the PR: {}", pr.full_id);
        context.near.send_pause(&pr.owner, &pr.repo).await?;
        context
            .reply(pr, self.comment_id, MsgCategory::PauseMessage, vec![])
            .await?;
        Ok(EventResult::success(false))
    }

    pub fn construct(comment: &CommentRepr) -> Command {
        Command::Pause(BotPaused {
            timestamp: comment.timestamp,
            comment_id: comment.comment_id,
        })
    }
}

#[derive(Clone, Debug)]
pub struct BotUnpaused {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub comment_id: Option<u64>,
}

impl BotUnpaused {
    #[instrument(skip(self, pr, context, info, sender), fields(pr = pr.full_id))]
    pub async fn execute(
        &self,
        pr: &PrMetadata,
        context: Context,
        info: PRInfo,
        sender: &User,
    ) -> anyhow::Result<EventResult> {
        if !sender.is_maintainer() {
            info!(
                "Tried to unpause a PR from not maintainer: {}. Skipping",
                pr.full_id
            );
            return Ok(EventResult::Skipped);
        }

        if info.paused {
            context.near.send_unpause(&pr.owner, &pr.repo).await?;
            debug!("Unpaused PR {}", pr.full_id);
            context
                .reply(pr, self.comment_id, MsgCategory::UnpauseMessage, vec![])
                .await?;
            Ok(EventResult::success(false))
        } else {
            context
                .reply(
                    pr,
                    self.comment_id,
                    MsgCategory::ErrorUnpauseUnpausedMessage,
                    vec![],
                )
                .await?;
            Ok(EventResult::RepliedWithError)
        }
    }

    pub fn construct(comment: &CommentRepr) -> Command {
        Command::Unpause(BotUnpaused {
            timestamp: comment.timestamp,
            comment_id: comment.comment_id,
        })
    }
}
