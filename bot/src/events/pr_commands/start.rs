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
    #[instrument(skip(self, pr, context, info, sender), fields(pr = pr.repo_info.full_id))]
    pub async fn execute(
        &self,
        pr: &PrMetadata,
        context: Context,
        info: &mut PRInfo,
        sender: &User,
    ) -> anyhow::Result<EventResult> {
        if info.exist {
            debug!(
                "Sloth is already included in {}. Skipping",
                pr.repo_info.full_id,
            );
            return Ok(EventResult::Skipped);
        }

        match (pr.merged, pr.closed) {
            (Some(merged_at), _) if (chrono::Utc::now() - merged_at) < Duration::days(1) => {}
            (_, false) => {}
            _ => {
                debug!("PR {} is already merged. Skipping", pr.repo_info.full_id,);
                context
                    .reply_with_error(
                        &pr.repo_info,
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
                pr.repo_info.full_id
            );
            context
                .reply_with_error(
                    &pr.repo_info,
                    self.user_comment_id,
                    MsgCategory::ErrorRightsViolationMessage,
                    vec![],
                )
                .await?;
            return Ok(EventResult::RepliedWithError);
        }

        if sender.login != pr.author.login {
            let user_info = context.near.user_info(&pr.author.login, vec![]).await?;
            if user_info.is_none() {
                debug!(
                    "Author of PR {} is not registered in Race-of-Sloths. Inviting instead of including. Skipping",
                    pr.repo_info.full_id
                );
                let invite_txt = context
                    .messages
                    .invite_message(&pr.author.login, &sender.login)?;
                context
                    .reply_with_text(&pr.repo_info, self.user_comment_id, &invite_txt)
                    .await?;
                return Ok(EventResult::Success {
                    should_update: false,
                });
            }
        }

        debug!("Starting PR {}", pr.repo_info.full_id);
        context.near.send_start(pr, sender.is_maintainer()).await?;
        info.exist = true;
        info.excluded = false;

        if let Some(comment_id) = self.user_comment_id {
            context
                .github
                .like_comment(&pr.repo_info.owner, &pr.repo_info.repo, comment_id)
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
