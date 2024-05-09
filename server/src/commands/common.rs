use crate::api::{github::PrMetadata, near::PRInfo};

use super::ContextStruct;

impl ContextStruct {
    pub(super) async fn check_info(&self, pr_metadata: &PrMetadata) -> anyhow::Result<PRInfo> {
        self.near
            .check_info(&pr_metadata.owner, &pr_metadata.repo, pr_metadata.number)
            .await
    }

    pub(super) async fn reply_with_error(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
        error: &str,
    ) -> anyhow::Result<()> {
        self.github
            .reply(
                owner,
                repo,
                number,
                &format!("Hey, I'm sorry, but I can't process that: {}", error),
            )
            .await
    }
}
