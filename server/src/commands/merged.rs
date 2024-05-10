use tracing::{debug, instrument};

use super::*;

#[derive(Debug, Clone)]
pub struct PullRequestMerged {
    pub pr_metadata: PrMetadata,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[async_trait::async_trait]
impl Execute for PullRequestMerged {
    #[instrument(skip(self, context), fields(pr = self.pr_metadata.full_id))]
    async fn execute(&self, context: Context) -> anyhow::Result<()> {
        let info = context.check_info(&self.pr_metadata).await?;
        if !info.allowed_repo || !info.exist {
            debug!(
                "PR {} is not started or not allowed. Skipping",
                self.pr_metadata.full_id,
            );
            return Ok(());
        }

        context.near.send_merge(&self.pr_metadata).await?;

        context
            .github
            .like_pr(
                &self.pr_metadata.owner,
                &self.pr_metadata.repo,
                self.pr_metadata.number,
            )
            .await
    }
}
