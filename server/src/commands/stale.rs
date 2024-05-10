use tracing::{debug, instrument};

use super::*;

pub struct PullRequestStale {
    pub pr_metadata: PrMetadata,
}

#[async_trait::async_trait]
impl Execute for PullRequestStale {
    #[instrument(skip(self, context), fields(pr = self.pr_metadata.full_id))]
    async fn execute(&self, context: Context) -> anyhow::Result<()> {
        debug!("Staling PR {}", self.pr_metadata.full_id);
        context.near.send_stale(&self.pr_metadata).await?;

        let check_info = context.check_info(&self.pr_metadata).await?;
        if check_info.allowed_repo {
            context.github.reply(&self.pr_metadata.owner, &self.pr_metadata.repo, self.pr_metadata.number, 
                "The PR has been inactive for two weeks. We are staling it now. If you want to continue, please restart the bot with `include` command"
            ).await?;
        }
        Ok(())
    }
}
