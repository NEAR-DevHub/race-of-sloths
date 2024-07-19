use std::str::FromStr;

use anyhow::bail;
use near_workspaces::{result::ExecutionFinalResult, types::SecretKey, Contract};
use serde_json::json;
use tracing::instrument;

use super::github::PrMetadata;

use crate::*;

#[derive(Clone, Debug)]
pub struct NearClient {
    contract: Contract,
}

impl NearClient {
    pub async fn new(
        contract: String,
        sk: String,
        mainnet: bool,
        rpc_addr: Option<String>,
    ) -> anyhow::Result<Self> {
        let sk = SecretKey::from_str(&sk)?;
        if mainnet {
            let mut mainnet = near_workspaces::mainnet();
            if let Some(rpc_addr) = rpc_addr {
                mainnet = mainnet.rpc_addr(&rpc_addr);
            }
            let contract = Contract::from_secret_key(contract.parse()?, sk, &mainnet.await?);
            return Ok(Self { contract });
        }
        let mut testnet = near_workspaces::testnet();
        if let Some(rpc_addr) = rpc_addr {
            testnet = testnet.rpc_addr(&rpc_addr);
        }
        let contract = Contract::from_secret_key(contract.parse()?, sk, &testnet.await?);
        Ok(Self { contract })
    }

    #[instrument(skip(self, pr), fields(pr = pr.full_id))]
    pub async fn send_start(
        &self,
        pr: &PrMetadata,
        timestamp: chrono::DateTime<chrono::Utc>,
        is_maintainer: bool,
    ) -> anyhow::Result<Vec<Event>> {
        let args = json!({
            "organization": pr.owner,
            "repo": pr.repo,
            "pr_number": pr.number,
            "user": pr.author.login,
            "started_at": timestamp.timestamp_nanos_opt().unwrap_or(0),
            "override_exclude": is_maintainer,
        });

        let result = self
            .contract
            .call("sloth_include")
            .args_json(args)
            .transact_async()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to call sloth_include: {:?}", e))?
            .await?;

        process_execution_final_result(result)
    }

    #[instrument(skip(self), fields(pr = pr.full_id, user, score))]
    pub async fn send_scored(
        &self,
        pr: &PrMetadata,
        user: &str,
        score: u64,
    ) -> anyhow::Result<Vec<Event>> {
        let args = json!({
            "pr_id": pr.full_id,
            "user": user,
            "score": score,
        });

        let result = self
            .contract
            .call("sloth_scored")
            .args_json(args)
            .transact_async()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to call sloth_scored: {:?}", e))?
            .await?;
        process_execution_final_result(result)
    }

    #[instrument(skip(self, pr), fields(pr = pr.full_id))]
    pub async fn send_merge(&self, pr: &PrMetadata) -> anyhow::Result<Vec<Event>> {
        if pr.merged.is_none() {
            bail!("PR is not merged")
        }

        let args = json!({
            "pr_id": pr.full_id,
            "merged_at": pr.merged.unwrap().timestamp_nanos_opt().unwrap_or(0),
        });

        let result = self
            .contract
            .call("sloth_merged")
            .args_json(args)
            .transact_async()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to call sloth_merged: {:?}", e))?
            .await?;
        process_execution_final_result(result)
    }

    #[instrument(skip(self))]
    pub async fn send_pause(&self, organization: &str, repo: &str) -> anyhow::Result<Vec<Event>> {
        let result = self
            .contract
            .call("pause_repo")
            .args_json(json!({
                "organization": organization,
                "repo": repo,}))
            .transact_async()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to call sloth_paused: {:?}", e))?
            .await?;
        process_execution_final_result(result)
    }

    #[instrument(skip(self))]
    pub async fn send_unpause(&self, organization: &str, repo: &str) -> anyhow::Result<Vec<Event>> {
        let result = self
            .contract
            .call("unpause_repo")
            .args_json(json!({
                "organization": organization,
                "repo": repo,}))
            .transact_async()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to call sloth_resumed: {:?}", e))?
            .await?;
        process_execution_final_result(result)
    }

    #[instrument(skip(self))]
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
            .finality(near_workspaces::types::Finality::Optimistic)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to call is_organization_allowed: {:?}", e))?;
        let res: PRInfo = res.json()?;
        Ok(res)
    }

    #[instrument(skip(self))]
    pub async fn unmerged_prs(&self, page: u64, limit: u64) -> anyhow::Result<Vec<PRWithRating>> {
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

    #[instrument(skip(self))]
    pub async fn unmerged_prs_all(&self) -> anyhow::Result<Vec<PRWithRating>> {
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

    #[instrument(skip(self))]
    pub async fn unfinalized_prs(
        &self,
        page: u64,
        limit: u64,
    ) -> anyhow::Result<Vec<PRWithRating>> {
        let args = json!({
            "page": page,
            "limit": limit,
        });

        let res = self
            .contract
            .view("unfinalized_prs")
            .args_json(args)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to call unfinalized_prs: {:?}", e))?;
        let res = res.json()?;

        Ok(res)
    }

    #[instrument(skip(self))]
    pub async fn unfinalized_prs_all(&self) -> anyhow::Result<Vec<PRWithRating>> {
        let mut page = 0;
        const LIMIT: u64 = 100;
        let mut res = vec![];
        loop {
            let prs = self.unfinalized_prs(page, LIMIT).await?;
            if prs.is_empty() {
                break;
            }
            res.extend(prs);
            page += 1;
        }
        Ok(res)
    }

    #[instrument(skip(self, pr), fields(pr = pr.full_id))]
    pub async fn send_stale(&self, pr: &PrMetadata) -> anyhow::Result<Vec<Event>> {
        let args = json!({
            "pr_id": pr.full_id,
        });

        let result = self
            .contract
            .call("sloth_stale")
            .args_json(args)
            .transact_async()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to call sloth_stale: {:?}", e))?
            .await?;
        process_execution_final_result(result)
    }

    #[instrument(skip(self, pr), fields(pr = pr.full_id))]
    pub async fn send_exclude(&self, pr: &PrMetadata) -> anyhow::Result<Vec<Event>> {
        let args = json!({
            "pr_id": pr.full_id,
        });

        let result = self
            .contract
            .call("sloth_exclude")
            .args_json(args)
            .transact_async()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to call sloth_exclude: {:?}", e))?
            .await?;
        process_execution_final_result(result)
    }

    #[instrument(skip(self))]
    pub async fn send_finalize(&self, pr_id: &str) -> anyhow::Result<Vec<Event>> {
        let result = self
            .contract
            .call("sloth_finalize")
            .args_json(json!({
                "pr_id": pr_id,
            }))
            .transact_async()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to call execute_prs: {:?}", e))?
            .await?;
        process_execution_final_result(result)
    }

    #[instrument(skip(self))]
    pub async fn user_info(
        &self,
        user: &str,
        periods: Vec<TimePeriodString>,
    ) -> anyhow::Result<Option<User>> {
        let res = self
            .contract
            .view("user")
            .args_json(json!({
                "user": user,
                "periods": periods
            }))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to call user_info: {:?}", e))?;
        let res: Option<User> = res.json()?;
        Ok(res)
    }

    pub async fn users_paged(
        &self,
        page: u64,
        limit: u64,
        periods: Vec<TimePeriodString>,
    ) -> anyhow::Result<Vec<User>> {
        let res = self
            .contract
            .view("users")
            .args_json(json!({
                "page": page,
                "limit": limit,
                "periods": periods,
            }))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to call users: {:?}", e))?;
        let res = res.json()?;
        Ok(res)
    }

    #[instrument(skip(self))]
    pub async fn users(&self, periods: Vec<TimePeriodString>) -> anyhow::Result<Vec<User>> {
        let mut page = 0;
        const LIMIT: u64 = 100;
        let mut res = vec![];
        loop {
            let users = self.users_paged(page, LIMIT, periods.clone()).await?;
            if users.is_empty() {
                break;
            }
            res.extend(users);
            page += 1;
        }
        Ok(res)
    }

    #[instrument(skip(self))]
    pub async fn prs_paged(
        &self,
        page: u64,
        limit: u64,
    ) -> anyhow::Result<Vec<(PRWithRating, bool)>> {
        let res = self
            .contract
            .view("prs")
            .args_json(json!({
                "page": page,
                "limit": limit,
            }))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to call prs: {:?}", e))?;
        let res = res.json()?;
        Ok(res)
    }

    #[instrument(skip(self))]
    pub async fn prs(&self) -> anyhow::Result<Vec<(PRWithRating, bool)>> {
        let mut page = 0;
        const LIMIT: u64 = 100;
        let mut res = vec![];
        loop {
            let prs = self.prs_paged(page, LIMIT).await?;
            if prs.is_empty() {
                break;
            }
            res.extend(prs);
            page += 1;
        }
        Ok(res)
    }

    #[instrument(skip(self))]
    pub async fn repos(&self) -> anyhow::Result<Vec<AllowedRepos>> {
        let res = self
            .contract
            .view("repos")
            .await
            .map_err(|e| anyhow::anyhow!("Failed to call allowed_repos: {:?}", e))?;
        let res = res.json()?;
        Ok(res)
    }
}

fn process_execution_final_result(result: ExecutionFinalResult) -> anyhow::Result<Vec<Event>> {
    if !result.is_success() && !result.is_failure() {
        tracing::error!(
            "debugging: Result is not success and not failure. {:?}",
            result
        );
        bail!("Execution is not final: {:?}", result);
    }

    if !result.is_success() {
        bail!("Execution failure: {:?}", result);
    }

    let events = result
        .logs()
        .into_iter()
        .flat_map(|l| serde_json::from_str::<Event>(l).ok())
        .collect();
    Ok(events)
}
