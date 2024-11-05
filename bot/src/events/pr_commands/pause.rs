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
    #[instrument(skip(self, pr, context, check_info, sender), fields(pr = pr.repo_info.full_id))]
    pub async fn execute(
        &self,
        pr: &PrMetadata,
        context: Context,
        check_info: &mut PRInfo,
        sender: &User,
    ) -> anyhow::Result<EventResult> {
        if check_info.paused_repo {
            info!(
                "Tried to pause a PR from paused repo: {}. Skipping",
                pr.repo_info.full_id
            );
            context
                .reply_with_error(
                    &pr.repo_info,
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
                pr.repo_info.full_id
            );
            context
                .reply_with_error(
                    &pr.repo_info,
                    self.comment_id,
                    MsgCategory::ErrorRightsViolationMessage,
                    vec![],
                )
                .await?;
            return Ok(EventResult::RepliedWithError);
        }

        debug!("Pausing the repository in the PR: {}", pr.repo_info.full_id);
        context
            .near
            .send_pause(&pr.repo_info.owner, &pr.repo_info.repo)
            .await?;
        check_info.paused_repo = true;
        context
            .reply(
                &pr.repo_info,
                self.comment_id,
                MsgCategory::PauseMessage,
                vec![],
            )
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
    pub from_issue: bool,
}

impl BotUnpaused {
    #[instrument(skip(self, repo_info, context, info, sender), fields(pr = repo_info.full_id))]
    pub async fn execute(
        &self,
        repo_info: &RepoInfo,
        context: Context,
        info: &mut PRInfo,
        sender: &User,
    ) -> anyhow::Result<EventResult> {
        if !sender.is_maintainer() {
            info!(
                "Tried to unpause a PR from not maintainer: {}. Skipping",
                repo_info.full_id
            );
            return Ok(EventResult::Skipped);
        }

        if info.paused_repo {
            context
                .near
                .send_unpause(&repo_info.owner, &repo_info.repo)
                .await?;
            info.paused_repo = false;
            debug!("Unpaused PR {}", repo_info.full_id);
            let msg = if self.from_issue {
                MsgCategory::UnpauseIssueMessage
            } else {
                MsgCategory::UnpauseMessage
            };
            context
                .reply(repo_info, self.comment_id, msg, vec![])
                .await?;
            Ok(EventResult::success(false))
        } else {
            context
                .reply(
                    repo_info,
                    self.comment_id,
                    MsgCategory::ErrorUnpauseUnpausedMessage,
                    vec![],
                )
                .await?;
            Ok(EventResult::RepliedWithError)
        }
    }

    pub fn construct(comment: &CommentRepr, from_issue: bool) -> Command {
        Command::Unpause(BotUnpaused {
            timestamp: comment.timestamp,
            comment_id: comment.comment_id,
            from_issue,
        })
    }
}
