use tracing::{debug, instrument};

use crate::messages::MsgCategory;

use shared::github::User;

use super::*;

#[derive(Debug, Clone)]
pub struct BotIncluded {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub comment_id: Option<u64>,
}

impl BotIncluded {
    pub fn new(timestamp: chrono::DateTime<chrono::Utc>, comment_id: Option<u64>) -> Self {
        Self {
            timestamp,
            comment_id,
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

        if pr.merged.is_some() {
            debug!("PR {} is already merged. Skipping", pr.full_id,);
            context
                .reply_with_error(
                    &pr,
                    self.comment_id,
                    MsgCategory::ErrorLateIncludeMessage,
                    vec![],
                )
                .await?;
            return Ok(false);
        }

        debug!("Starting PR {}", pr.full_id);
        context.near.send_start(&pr, sender.is_maintainer()).await?;

        context
            .reply(
                &pr,
                self.comment_id,
                MsgCategory::IncludeBasicMessage,
                vec![("pr_author_username".to_string(), pr.author.login.clone())],
            )
            .await?;
        Ok(false)
    }

    pub fn construct(comment: &Comment) -> Command {
        Command::Include(BotIncluded::new(comment.created_at, Some(comment.id.0)))
    }

    pub fn parse_body(bot_name: &str, pr_metadata: &PrMetadata) -> Option<Command> {
        let body = pr_metadata.body.as_str();
        let bot_name = format!("@{}", bot_name);
        if !body.contains(&bot_name) {
            return None;
        }

        Some(Command::Include(Self {
            timestamp: pr_metadata.started,
            comment_id: None,
        }))
    }
}
