use tracing::instrument;

use shared::{github::PrMetadata, PRInfo};

use crate::{events::Context, messages::MsgCategory};

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
    pub async fn execute(&self, context: Context, info: PRInfo) -> anyhow::Result<bool> {
        context.near.send_merge(&self.pr_metadata).await?;

        if !info.allowed_org {
            return Ok(false);
        }

        if info.votes.is_empty() {
            context
                .reply(
                    &self.pr_metadata,
                    None,
                    MsgCategory::MergeWithoutScoreMessage,
                    vec![],
                )
                .await?;
        }
        Ok(true)
    }
}
