use anyhow::bail;
use near_workspaces::{types::SecretKey, Contract};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{debug, instrument};

use super::github::PrMetadata;

#[derive(Clone)]
pub struct NearClient {
    contract: Contract,
}

impl NearClient {
    pub async fn new(contract: String, sk: SecretKey, mainnet: bool) -> anyhow::Result<Self> {
        if mainnet {
            let mainnet = near_workspaces::mainnet().await?;
            let contract = Contract::from_secret_key(contract.parse()?, sk, &mainnet);
            return Ok(Self { contract });
        }
        let testnet = near_workspaces::testnet().await?;
        let contract = Contract::from_secret_key(contract.parse()?, sk, &testnet);
        Ok(Self { contract })
    }

    #[instrument(skip(self, pr), fields(pr = pr.full_id))]
    pub async fn send_start(&self, pr: &PrMetadata, is_maintainer: bool) -> anyhow::Result<()> {
        let args = json!({
            "organization": pr.owner,
            "repo": pr.repo,
            "pr_number": pr.number,
            "user": pr.author.login,
            "started_at": pr.started.timestamp_nanos_opt().unwrap_or(0),
            "override_exclude": is_maintainer,
        });

        self.contract
            .call("sloth_include")
            .args_json(args)
            .transact_async()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to call sloth_include: {:?}", e))?;
        Ok(())
    }

    #[instrument(skip(self), fields(pr = pr.full_id, user, score))]
    pub async fn send_scored(&self, pr: &PrMetadata, user: &str, score: u64) -> anyhow::Result<()> {
        let args = json!({
            "pr_id": pr.full_id,
            "user": user,
            "score": score,
        });

        self.contract
            .call("sloth_scored")
            .args_json(args)
            .transact_async()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to call sloth_scored: {:?}", e))?;
        Ok(())
    }

    #[instrument(skip(self, pr), fields(pr = pr.full_id))]
    pub async fn send_merge(&self, pr: &PrMetadata) -> anyhow::Result<()> {
        if pr.merged.is_none() {
            bail!("PR is not merged")
        }

        let args = json!({
            "pr_id": pr.full_id,
            "merged_at": pr.merged.unwrap().timestamp_nanos_opt().unwrap_or(0),
        });

        let tx = self
            .contract
            .call("sloth_merged")
            .args_json(args)
            .transact_async()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to call sloth_merged: {:?}", e))?;
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn send_pause(&self, organization: &str, repo: &str) -> anyhow::Result<()> {
        self.contract
            .call("exclude_repo")
            .args_json(json!({
                "organization": organization,
                "repo": repo,}))
            .transact_async()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to call sloth_paused: {:?}", e))?;
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn send_unpause(&self, organization: &str, repo: &str) -> anyhow::Result<()> {
        self.contract
            .call("include_repo")
            .args_json(json!({
                "organization": organization,
                "repo": repo,}))
            .transact_async()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to call sloth_resumed: {:?}", e))?;
        Ok(())
    }

    pub async fn check_info(
        &self,
        organization: &str,
        repo: &str,
        issue_id: u64,
    ) -> anyhow::Result<PRInfo> {
        let args = json!({
            "organization": organization,
            "repo": repo,
            "issue_id": issue_id,
        });

        let res = self
            .contract
            .view("check_info")
            .args_json(args)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to call is_organization_allowed: {:?}", e))?;
        let res: PRInfo = res.json()?;
        Ok(res)
    }

    pub async fn unmerged_prs(&self, page: u64, limit: u64) -> anyhow::Result<Vec<PRData>> {
        let args = json!({
            "page": page,
            "limit": limit,
        });

        let res = self
            .contract
            .view("unmerged_prs")
            .args_json(args)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to call unmerged_prs: {:?}", e))?;
        let res = res.json()?;

        Ok(res)
    }

    pub async fn unmerged_prs_all(&self) -> anyhow::Result<Vec<PRData>> {
        let mut page = 0;
        const LIMIT: u64 = 100;
        let mut res = vec![];
        loop {
            let prs = self.unmerged_prs(page, LIMIT).await?;
            if prs.is_empty() {
                break;
            }
            res.extend(prs);
            page += 1;
        }
        Ok(res)
    }

    #[instrument(skip(self, pr), fields(pr = pr.full_id))]
    pub async fn send_stale(&self, pr: &PrMetadata) -> anyhow::Result<()> {
        let args = json!({
            "pr_id": pr.full_id,
        });

        self.contract
            .call("sloth_stale")
            .args_json(args)
            .transact_async()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to call sloth_stale: {:?}", e))?;
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn finalize_prs(&self) -> anyhow::Result<()> {
        let result: bool = self.contract.view("should_finalize").await?.json()?;

        if !result {
            debug!("No PRs to finalize");
            // Nothing to finalize
            return Ok(());
        }
        self.contract
            .call("sloth_finalize")
            .transact_async()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to call execute_prs: {:?}", e))?;
        Ok(())
    }

    #[instrument(skip(self, pr), fields(pr = pr.full_id))]
    pub async fn send_exclude(&self, pr: &PrMetadata) -> anyhow::Result<()> {
        let args = json!({
            "pr_id": pr.full_id,
        });

        self.contract
            .call("sloth_exclude")
            .args_json(args)
            .transact_async()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to call sloth_exclude: {:?}", e))?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
pub struct PRInfo {
    pub allowed_org: bool,
    pub allowed_repo: bool,
    pub exist: bool,
    pub merged: bool,
    pub scored: bool,
    pub executed: bool,
    pub excluded: bool,
}

#[derive(Serialize, Deserialize)]
pub struct PRData {
    pub organization: String,
    pub repo: String,
    pub number: u64,
}
