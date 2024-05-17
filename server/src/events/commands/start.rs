use tracing::{debug, instrument};

use crate::messages::MsgCategory;

use self::api::github::User;

use super::*;

#[derive(Debug, Clone)]
pub struct BotIncluded {
    pub sender: User,
    pub pr_metadata: PrMetadata,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub comment_id: Option<u64>,
}

impl BotIncluded {
    pub fn new(
        sender: User,
        pr_metadata: PrMetadata,
        timestamp: chrono::DateTime<chrono::Utc>,
        comment_id: Option<u64>,
    ) -> Self {
        Self {
            sender,
            pr_metadata,
            timestamp,
            comment_id,
        }
    }
}

impl BotIncluded {
    #[instrument(skip(self, context, info), fields(pr = self.pr_metadata.full_id))]
    pub async fn execute(&self, context: Context, info: PRInfo) -> anyhow::Result<bool> {
        if info.exist {
            debug!(
                "Sloth is already included in {}. Skipping",
                self.pr_metadata.full_id,
            );
            return Ok(false);
        }

        if self.pr_metadata.merged.is_some() {
            debug!(
                "PR {} is already merged. Skipping",
                self.pr_metadata.full_id,
            );
            context
                .reply_with_error(
                    &self.pr_metadata,
                    self.comment_id,
                    MsgCategory::ErrorLateIncludeMessage,
                    vec![],
                )
                .await?;
            return Ok(false);
        }

        debug!("Starting PR {}", self.pr_metadata.full_id);

        // TODO: other types of events
        let comment = context
            .reply(
                &self.pr_metadata,
                self.comment_id,
                MsgCategory::IncludeBasicMessage,
                vec![(
                    "pr-author-username".to_string(),
                    self.pr_metadata.author.login.clone(),
                )],
            )
            .await?;

        context
            .near
            .send_start(&self.pr_metadata, self.sender.is_maintainer(), comment.id.0)
            .await?;
        // We already put the status message in the reply, so we don't need to send it again
        Ok(false)
    }

    pub fn construct(pr_metadata: &PrMetadata, comment: &Comment) -> Command {
        Command::Include(BotIncluded::new(
            User::new(
                comment.user.login.clone(),
                comment.author_association.clone(),
            ),
            pr_metadata.clone(),
            comment.created_at,
            Some(comment.id.0),
        ))
    }

    pub fn parse_body(bot_name: &str, pr_metadata: &PrMetadata) -> Option<Command> {
        let body = pr_metadata.body.as_str();
        let bot_name = format!("@{}", bot_name);
        if !body.contains(&bot_name) {
            return None;
        }

        Some(Command::Include(Self {
            sender: pr_metadata.author.clone(),
            pr_metadata: pr_metadata.clone(),
            timestamp: pr_metadata.started,
            comment_id: None,
        }))
    }
}
