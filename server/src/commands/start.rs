use super::*;

#[async_trait::async_trait]
impl Execute for api::github::BotStarted {
    async fn execute(&self, context: Context) -> anyhow::Result<()> {
        let message = if self.is_accepted() {
            format!(
            "Hey hey. You called me, @{}? :).\nKeep up the good work @{}!\nDear maintainer please score this PR when the time comes to merge it with given syntax `@akorchyn score 1/2/3/4/5`. Please note that I will ignore incorrectly provided messages to not spam!",
            self.sender,
            self.pr_metadata.author.login,
        )
        } else {
            format!(
                "Hey hey. You called me, @{}? :).\nI'm sorry but maintainers and members can't get rewarded for work in their own projects.!",
                self.sender
            )
        };
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
