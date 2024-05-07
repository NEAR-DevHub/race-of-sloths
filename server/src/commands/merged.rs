use super::*;

#[async_trait::async_trait]
impl Execute for api::github::PullRequestMerged {
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
}
