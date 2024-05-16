use tracing::{instrument, warn};

use crate::{
    api::{github::PrMetadata, near::PRInfo},
    commands::Context,
    consts::STALE_MESSAGES,
};

#[derive(Debug, Clone)]
pub struct PullRequestStale {
    pub pr_metadata: PrMetadata,
}

impl PullRequestStale {
    #[instrument(skip(self, context, check_info), fields(pr = self.pr_metadata.full_id))]
    pub async fn execute(&self, context: Context, check_info: PRInfo) -> anyhow::Result<()> {
        if check_info.merged {
            warn!(
                "PR {} is already merged. Skipping",
                self.pr_metadata.full_id
            );
            return Ok(());
        }

        context.near.send_stale(&self.pr_metadata).await?;

        if check_info.allowed_repo {
            context
                .reply(&self.pr_metadata, None, &STALE_MESSAGES)
                .await?;
        }
        Ok(())
    }
}
