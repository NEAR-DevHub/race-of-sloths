use tracing::instrument;

use shared::{github::PrMetadata, PRInfo};

use crate::{events::Context, messages::MsgCategory};

#[derive(Debug, Clone)]
pub struct PullRequestMerge {}

impl PullRequestMerge {
    #[instrument(skip(self, pr, context, info), fields(pr = pr.full_id))]
    pub async fn execute(
        &self,
        pr: &PrMetadata,
        context: Context,
        info: PRInfo,
    ) -> anyhow::Result<bool> {
        if info.merged {
            return Ok(false);
        }

        context.near.send_merge(pr).await?;

        if !info.allowed_org {
            return Ok(false);
        }

        if info.votes.is_empty() {
            context
                .reply(pr, None, MsgCategory::MergeWithoutScoreMessage, vec![])
                .await?;
        }
        Ok(true)
    }
}
