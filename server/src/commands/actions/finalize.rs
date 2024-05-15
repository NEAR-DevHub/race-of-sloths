use tracing::{instrument, warn};

use crate::{
    api::{
        github::PrMetadata,
        near::{PRInfo, PR},
    },
    commands::Context,
};

#[derive(Debug, Clone)]
pub struct PullRequestFinalize {
    pub pr_metadata: PrMetadata,
}

impl PullRequestFinalize {
    pub fn new(pr_metadata: PR) -> Self {
        Self {
            pr_metadata: pr_metadata.into(),
        }
    }

    #[instrument(skip(self, context, info), fields(pr = self.pr_metadata.full_id))]
    pub async fn execute(&self, context: Context, info: PRInfo) -> anyhow::Result<()> {
        if info.executed {
            warn!(
                "PR {} is already finalized. Skipping",
                self.pr_metadata.full_id
            );
            return Ok(());
        }

        context
            .near
            .send_finalize(&self.pr_metadata.full_id)
            .await?;

        context
                .github
                .reply(
                    &self.pr_metadata.owner,
                    &self.pr_metadata.repo,
                    self.pr_metadata.number,
                    "The PR has been finalized. Thank you for your contribution! The scoring process is closed now."
                )
                .await?;
        Ok(())
    }
}
