use tracing::{instrument, warn};

use shared::{github::PrMetadata, PRInfo};

use crate::{events::Context, messages::MsgCategory};

#[derive(Debug, Clone)]
pub struct PullRequestFinalize {
    pub pr_metadata: PrMetadata,
}

impl PullRequestFinalize {
    #[instrument(skip(self, context, info), fields(pr = self.pr_metadata.full_id))]
    pub async fn execute(&self, context: Context, info: PRInfo) -> anyhow::Result<bool> {
        if info.executed {
            warn!(
                "PR {} is already finalized. Skipping",
                self.pr_metadata.full_id
            );
            return Ok(false);
        }

        context
            .near
            .send_finalize(&self.pr_metadata.full_id)
            .await?;

        if info.allowed_repo {
            context
                .reply(
                    &self.pr_metadata,
                    None,
                    MsgCategory::FinalMessage,
                    vec![
                        (
                            "pr_author_username".to_string(),
                            self.pr_metadata.author.login.clone(),
                        ),
                        ("score".to_string(), info.average_score().to_string()),
                    ],
                )
                .await?;
        }
        Ok(true)
    }
}
