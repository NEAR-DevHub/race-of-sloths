use tracing::instrument;

use crate::{
    api::{github::PrMetadata, near::PRInfo},
    commands::Context,
};

#[derive(Debug, Clone)]
pub struct PullRequestMerge {
    pub pr_metadata: PrMetadata,
}

impl PullRequestMerge {
    pub fn new(pr_metadata: PrMetadata) -> Option<Self> {
        if pr_metadata.merged.is_some() {
            Some(Self { pr_metadata })
        } else {
            None
        }
    }
}

impl PullRequestMerge {
    #[instrument(skip(self, context, info), fields(pr = self.pr_metadata.full_id))]
    pub async fn execute(&self, context: Context, info: PRInfo) -> anyhow::Result<()> {
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
        Ok(())
    }
}
