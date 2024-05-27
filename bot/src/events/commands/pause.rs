use tracing::{debug, info, instrument};

use crate::messages::MsgCategory;

use shared::github::User;

use super::*;

#[derive(Clone, Debug)]
pub struct BotPaused {
    pub pr_metadata: PrMetadata,
    pub sender: User,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub comment_id: u64,
}

impl BotPaused {
    #[instrument(skip(self, context, _check_info), fields(pr = self.pr_metadata.full_id))]
    pub async fn execute(&self, context: Context, _check_info: PRInfo) -> anyhow::Result<bool> {
        if !self.sender.is_maintainer() {
            info!(
                "Tried to pause a PR from not maintainer: {}. Skipping",
                self.pr_metadata.full_id
            );
            context
                .reply_with_error(
                    &self.pr_metadata,
                    Some(self.comment_id),
                    MsgCategory::ErrorRightsViolationMessage,
                    vec![],
                )
                .await?;
            return Ok(false);
        }

        debug!(
            "Pausing the repository in the PR: {}",
            self.pr_metadata.full_id
        );
        context
            .near
            .send_pause(&self.pr_metadata.owner, &self.pr_metadata.repo)
            .await?;
        context
            .reply(
                &self.pr_metadata,
                Some(self.comment_id),
                MsgCategory::PauseMessage,
                vec![],
            )
            .await?;
        Ok(true)
    }

    pub fn construct(pr_metadata: &PrMetadata, comment: &Comment) -> Command {
        Command::Pause(BotPaused {
            pr_metadata: pr_metadata.clone(),
            sender: User {
                login: comment.user.login.clone(),
                contributor_type: comment.author_association.clone(),
            },
            timestamp: comment.created_at,
            comment_id: comment.id.0,
        })
    }
}

#[derive(Clone, Debug)]
pub struct BotUnpaused {
    pub pr_metadata: PrMetadata,
    pub sender: User,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub comment_id: u64,
}

impl BotUnpaused {
    #[instrument(skip(self, context, info), fields(pr = self.pr_metadata.full_id))]
    pub async fn execute(&self, context: Context, info: PRInfo) -> anyhow::Result<bool> {
        if !self.sender.is_maintainer() {
            info!(
                "Tried to unpause a PR from not maintainer: {}. Skipping",
                self.pr_metadata.full_id
            );
            return Ok(false);
        }

        if !info.allowed_repo {
            context
                .near
                .send_unpause(&self.pr_metadata.owner, &self.pr_metadata.repo)
                .await?;
            debug!("Unpaused PR {}", self.pr_metadata.full_id);
            context
                .reply(
                    &self.pr_metadata,
                    Some(self.comment_id),
                    MsgCategory::UnpauseMessage,
                    vec![],
                )
                .await?;
            Ok(false)
        } else {
            context
                .reply(
                    &self.pr_metadata,
                    Some(self.comment_id),
                    MsgCategory::UnpauseUnpausedMessage,
                    vec![],
                )
                .await?;
            Ok(false)
        }
    }

    pub fn construct(pr_metadata: &PrMetadata, comment: &Comment) -> Command {
        Command::Unpause(BotUnpaused {
            pr_metadata: pr_metadata.clone(),
            sender: User {
                login: comment.user.login.clone(),
                contributor_type: comment.author_association.clone(),
            },
            timestamp: comment.created_at,
            comment_id: comment.id.0,
        })
    }
}
