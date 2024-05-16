use tracing::instrument;

use crate::{
    api::{github::PrMetadata, near::PRInfo},
    commands::Context,
    consts::MERGE_MESSAGES,
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
                .reply(&self.pr_metadata, None, &MERGE_MESSAGES)
                .await?;
        }
        Ok(())
    }
}
