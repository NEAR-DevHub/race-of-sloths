use tracing::{instrument, warn};

use shared::{github::PrMetadata, PRInfo};

use crate::{events::Context, messages::MsgCategory};

#[derive(Debug, Clone)]
pub struct PullRequestFinalize {}

impl PullRequestFinalize {
    #[instrument(skip(self, pr, context, info), fields(pr = pr.full_id))]
    pub async fn execute(
        &self,
        pr: &PrMetadata,
        context: Context,
        info: PRInfo,
    ) -> anyhow::Result<bool> {
        if info.executed {
            warn!("PR {} is already finalized. Skipping", pr.full_id);
            return Ok(false);
        }

        context.near.send_finalize(&pr.full_id).await?;

        if info.allowed_repo {
            context
                .reply(
                    pr,
                    None,
                    MsgCategory::FinalMessage,
                    vec![
                        ("pr_author_username".to_string(), pr.author.login.clone()),
                        ("score".to_string(), info.average_score().to_string()),
                    ],
                )
                .await?;
        }
        Ok(true)
    }
}
