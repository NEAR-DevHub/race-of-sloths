use crate::messages::MsgCategory;

use shared::github::User;

use super::*;

#[derive(Debug, Clone)]
pub struct UnknownCommand {
    pub user: User,
    pub command: String,
    pub args: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub comment_id: Option<u64>,
}

impl UnknownCommand {
    pub fn new(
        user: User,
        command: String,
        args: String,
        comment_id: Option<u64>,
        timestamp: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        Self {
            user,
            command,
            args,
            timestamp,
            comment_id,
        }
    }

    #[instrument(skip(self, pr, context, check_info, sender), fields(pr = pr.full_id))]
    pub async fn execute(
        &self,
        pr: &PrMetadata,
        context: Context,
        check_info: PRInfo,
        sender: &User,
    ) -> anyhow::Result<EventResult> {
        if !check_info.exist {
            // It's first call for this PR, so we will just include it
            let event = BotIncluded::new(self.timestamp, self.comment_id);
            return event.execute(pr, context, check_info, sender).await;
        }

        context
            .reply_with_error(
                pr,
                self.comment_id,
                MsgCategory::ErrorUnknownCommandMessage,
                vec![],
            )
            .await?;
        Ok(EventResult::RepliedWithError)
    }

    pub fn construct(comment: &CommentRepr, command: String, args: String) -> Command {
        Command::Unknown(Self::new(
            comment.user.clone(),
            command,
            args,
            comment.comment_id,
            comment.timestamp,
        ))
    }
}
