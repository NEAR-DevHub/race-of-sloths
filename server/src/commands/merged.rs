use super::*;

#[derive(Debug, Clone)]
pub struct PullRequestMerged {
    pub pr_metadata: PrMetadata,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[async_trait::async_trait]
impl Execute for PullRequestMerged {
    async fn execute(&self, context: Context) -> anyhow::Result<()> {
        let info = context.check_info(&self.pr_metadata).await?;
        if !info.allowed_repo || !info.exist {
            return Ok(());
        }

        context.near.send_merge(&self.pr_metadata).await?;

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
