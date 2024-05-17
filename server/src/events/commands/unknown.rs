use tracing::debug;

use crate::messages::MsgCategory;

use self::api::github::User;

use super::*;

#[derive(Debug, Clone)]
pub struct UnknownCommand {
    pub pr_metadata: PrMetadata,
    pub user: User,
    pub command: String,
    pub args: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub comment_id: u64,
}

impl UnknownCommand {
    pub fn new(
        pr_metadata: PrMetadata,
        user: User,
        command: String,
        args: String,
        comment_id: u64,
        timestamp: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        Self {
            pr_metadata,
            user,
            command,
            args,
            timestamp,
            comment_id,
        }
    }

    #[instrument(skip(self, context, check_info), fields(pr = self.pr_metadata.full_id))]
    pub async fn execute(&self, context: Context, check_info: PRInfo) -> anyhow::Result<bool> {
        let pr = self.pr_metadata.clone();
        if !check_info.exist {
            // It's first call for this PR, so we will just include it
            let event = BotIncluded::new(
                self.user.clone(),
                pr.clone(),
                self.timestamp.clone(),
                Some(self.comment_id),
            );
            return event.execute(context, check_info).await;
        }

        context
            .reply_with_error(
                &self.pr_metadata,
                MsgCategory::ErrorUnknownCommandMessage,
                vec![],
            )
            .await?;
        Ok(false)
    }

    pub fn construct(
        pr_metadata: &PrMetadata,
        comment: &Comment,
        command: String,
        args: String,
    ) -> Command {
        debug!(
            "Constructing unknown command: {command} with {args} for {}",
            pr_metadata.full_id
        );
        Command::Unknown(Self::new(
            pr_metadata.clone(),
            User::new(
                comment.user.login.clone(),
                comment.author_association.clone(),
            ),
            command,
            args,
            comment.id.0,
            comment.created_at.clone(),
        ))
    }
}
