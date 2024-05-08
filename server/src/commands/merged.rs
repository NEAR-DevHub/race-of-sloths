use self::api::github::PrMetadata;

use super::*;

#[async_trait::async_trait]
impl BotCommand for api::github::PullRequestMerged {
    type Command = api::github::PullRequestMerged;

    async fn execute(&self, context: Context) -> anyhow::Result<()> {
        let message = format!(
            "Hey hey.\nCongratulations @{}! Your PR has been merged!",
            self.pr_metadata.author.login
        );
        context
            .github
            .reply(
                &self.pr_metadata.owner,
                &self.pr_metadata.repo,
                self.pr_metadata.number,
                &message,
            )
            .await
    }

    fn parse_comment(
        _bot_name: &str,
        _notification: &Notification,
        _pr_metadata: &PrMetadata,
        _comment: &Comment,
    ) -> Option<Self::Command> {
        // Pull request merged is a special case, because it doesn't require a comment
        None
    }
}
