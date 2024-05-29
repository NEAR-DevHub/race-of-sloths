use crate::messages::MsgCategory;

use shared::github::User;

use super::*;

#[derive(Debug, Clone)]
pub struct UnknownCommand {
    pub user: User,
    pub command: String,
    pub args: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub comment_id: u64,
}

impl UnknownCommand {
    pub fn new(
        user: User,
        command: String,
        args: String,
        comment_id: u64,
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
    ) -> anyhow::Result<bool> {
        if !check_info.exist {
            // It's first call for this PR, so we will just include it
            let event = BotIncluded::new(self.timestamp, Some(self.comment_id));
            return event.execute(pr, context, check_info, sender).await;
        }

        context
            .reply_with_error(
                pr,
                Some(self.comment_id),
                MsgCategory::ErrorUnknownCommandMessage,
                vec![],
            )
            .await?;
        Ok(false)
    }

    pub fn construct(comment: &Comment, command: String, args: String) -> Command {
        Command::Unknown(Self::new(
            User::new(
                comment.user.login.clone(),
                comment.author_association.clone(),
            ),
            command,
            args,
            comment.id.0,
            comment.created_at,
        ))
    }
}
