use super::*;

#[async_trait::async_trait]
impl Execute for api::github::BotScored {
    async fn execute(&self, context: Context) -> anyhow::Result<()> {
        let message = if self.is_valid_score() {
            format!(
                "Hey hey.\nThank you for scoring @{}'s PR with {}!",
                self.pr_metadata.author.login, self.score
            )
        } else {
            "Hey hey :).\nAre you sure that you are the one who is able to score? :)".to_string()
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
