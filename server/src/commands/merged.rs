use tracing::{debug, error, instrument};

use super::*;

#[derive(Debug, Clone)]
pub struct PullRequestMerged {
    pub pr_metadata: PrMetadata,
}

#[async_trait::async_trait]
impl Execute for PullRequestMerged {
    #[instrument(skip(self, context), fields(pr = self.pr_metadata.full_id))]
    async fn execute(&self, context: Context) -> anyhow::Result<()> {
        let info = context.check_info(&self.pr_metadata).await?;
        if !info.allowed_repo || !info.exist || self.pr_metadata.merged.is_none() {
            error!(
                "PR {} is not started or not allowed. Skipping",
                self.pr_metadata.full_id,
            );
            return Ok(());
        }

        debug!("Merging PR {}", self.pr_metadata.full_id);
        context.near.send_merge(&self.pr_metadata).await?;

        if !info.scored {
            context
                .github
                .reply(
                    &self.pr_metadata.owner,
                    &self.pr_metadata.repo,
                    self.pr_metadata.number,
                    "The PR has been merged, but it was not scored. The score process will be closed after 24 hours automatically.",
                )
                .await?;
        }

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
