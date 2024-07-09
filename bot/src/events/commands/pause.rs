use tracing::{debug, info, instrument};

use crate::messages::MsgCategory;

use shared::github::User;

use super::*;

#[derive(Clone, Debug)]
pub struct BotPaused {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub comment_id: u64,
}

impl BotPaused {
    #[instrument(skip(self, pr, context, check_info, sender), fields(pr = pr.full_id))]
    pub async fn execute(
        &self,
        pr: &PrMetadata,
        context: Context,
        check_info: PRInfo,
        sender: &User,
    ) -> anyhow::Result<bool> {
        if !check_info.allowed_repo {
            info!(
                "Tried to pause a PR from paused repo: {}. Skipping",
                pr.full_id
            );
            context
                .reply_with_error(
                    pr,
                    Some(self.comment_id),
                    MsgCategory::ErrorPausePausedMessage,
                    vec![],
                )
                .await?;
            return Ok(false);
        }

        if !sender.is_maintainer() {
            info!(
                "Tried to pause a PR from not maintainer: {}. Skipping",
                pr.full_id
            );
            context
                .reply_with_error(
                    pr,
                    Some(self.comment_id),
                    MsgCategory::ErrorRightsViolationMessage,
                    vec![],
                )
                .await?;
            return Ok(false);
        }

        debug!("Pausing the repository in the PR: {}", pr.full_id);
        context.near.send_pause(&pr.owner, &pr.repo).await?;
        context
            .reply(pr, Some(self.comment_id), MsgCategory::PauseMessage, vec![])
            .await?;
        Ok(true)
    }

    pub fn construct(comment: &Comment) -> Command {
        Command::Pause(BotPaused {
            timestamp: comment.updated_at.unwrap_or(comment.created_at),
            comment_id: comment.id.0,
        })
    }
}

#[derive(Clone, Debug)]
pub struct BotUnpaused {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub comment_id: u64,
}

impl BotUnpaused {
    #[instrument(skip(self, pr, context, info, sender), fields(pr = pr.full_id))]
    pub async fn execute(
        &self,
        pr: &PrMetadata,
        context: Context,
        info: PRInfo,
        sender: &User,
    ) -> anyhow::Result<bool> {
        if !sender.is_maintainer() {
            info!(
                "Tried to unpause a PR from not maintainer: {}. Skipping",
                pr.full_id
            );
            return Ok(false);
        }

        if !info.allowed_repo {
            context.near.send_unpause(&pr.owner, &pr.repo).await?;
            debug!("Unpaused PR {}", pr.full_id);
            context
                .reply(
                    pr,
                    Some(self.comment_id),
                    MsgCategory::UnpauseMessage,
                    vec![],
                )
                .await?;
            Ok(false)
        } else {
            context
                .reply(
                    pr,
                    Some(self.comment_id),
                    MsgCategory::ErrorUnpauseUnpausedMessage,
                    vec![],
                )
                .await?;
            Ok(false)
        }
    }

    pub fn construct(comment: &Comment) -> Command {
        Command::Unpause(BotUnpaused {
            timestamp: comment.updated_at.unwrap_or(comment.created_at),
            comment_id: comment.id.0,
        })
    }
}
