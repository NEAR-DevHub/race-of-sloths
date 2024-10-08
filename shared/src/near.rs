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

    #[instrument(skip(self, pr), fields(pr = pr.repo_info.full_id))]
    pub async fn send_start(
        &self,
        pr: &PrMetadata,
        is_maintainer: bool,
    ) -> anyhow::Result<Vec<Event>> {
        let args = json!({
            "organization": pr.repo_info.owner,
            "repo": pr.repo_info.repo,
            "pr_number": pr.repo_info.number,
            "user": pr.author.login,
            "created_at": pr.created.timestamp_nanos_opt().unwrap_or(0),
            "override_exclude": is_maintainer,
        });

        let result = self
            .contract
            .call("sloth_include")
            .args_json(args)
            .transact()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to call sloth_include: {:?}", e))?;

        process_execution_final_result(result)
    }

    #[instrument(skip(self), fields(pr = pr.repo_info.full_id, user, score))]
    pub async fn send_scored(
        &self,
        pr: &PrMetadata,
        user: &str,
        score: u64,
    ) -> anyhow::Result<Vec<Event>> {
        let args = json!({
            "pr_id": pr.repo_info.full_id,
            "user": user,
            "score": score,
        });

        let result = self
            .contract
            .call("sloth_scored")
            .args_json(args)
            .transact()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to call sloth_scored: {:?}", e))?;
        process_execution_final_result(result)
    }

    #[instrument(skip(self, pr), fields(pr = pr.repo_info.full_id))]
    pub async fn send_merge(&self, pr: &PrMetadata) -> anyhow::Result<Vec<Event>> {
        if pr.merged.is_none() {
            bail!("PR is not merged")
        }

        let args = json!({
            "pr_id": pr.repo_info.full_id,
            "merged_at": pr.merged.unwrap().timestamp_nanos_opt().unwrap_or(0),
        });

        let result = self
            .contract
            .call("sloth_merged")
            .args_json(args)
            .transact()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to call sloth_merged: {:?}", e))?;
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
            .transact()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to call sloth_paused: {:?}", e))?;
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
            .transact()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to call sloth_resumed: {:?}", e))?;
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
    pub async fn unmerged_prs(&self, page: u64, limit: u64) -> anyhow::Result<Vec<PRv2>> {
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
    pub async fn unmerged_prs_all(&self) -> anyhow::Result<Vec<PRv2>> {
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
    pub async fn unfinalized_prs(&self, page: u64, limit: u64) -> anyhow::Result<Vec<PRv2>> {
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
    pub async fn unfinalized_prs_all(&self) -> anyhow::Result<Vec<PRv2>> {
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

    #[instrument(skip(self, pr), fields(pr = pr.repo_info.full_id))]
    pub async fn send_stale(&self, pr: &PrMetadata) -> anyhow::Result<Vec<Event>> {
        let args = json!({
            "pr_id": pr.repo_info.full_id,
        });

        let result = self
            .contract
            .call("sloth_stale")
            .args_json(args)
            .transact()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to call sloth_stale: {:?}", e))?;
        process_execution_final_result(result)
    }

    #[instrument(skip(self, pr), fields(pr = pr.repo_info.full_id))]
    pub async fn send_exclude(&self, pr: &PrMetadata) -> anyhow::Result<Vec<Event>> {
        let args = json!({
            "pr_id": pr.repo_info.full_id,
        });

        let result = self
            .contract
            .call("sloth_exclude")
            .args_json(args)
            .transact()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to call sloth_exclude: {:?}", e))?;
        process_execution_final_result(result)
    }

    #[instrument(skip(self))]
    pub async fn send_finalize(
        &self,
        pr_id: &str,
        active_pr: Option<(bool, GithubHandle)>,
    ) -> anyhow::Result<Vec<Event>> {
        let result = self
            .contract
            .call("sloth_finalize")
            .args_json(json!({
                "pr_id": pr_id,
                "active_pr": active_pr
            }))
            .transact()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to call execute_prs: {:?}", e))?;
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
        const LIMIT: u64 = 250;
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
    pub async fn prs_paged(&self, page: u64, limit: u64) -> anyhow::Result<Vec<(PRv2, bool)>> {
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
    pub async fn prs(&self) -> anyhow::Result<Vec<(PRv2, bool)>> {
        let mut page = 0;
        const LIMIT: u64 = 250;
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
    pub async fn repos_paged(
        &self,
        page: usize,
        limit: usize,
    ) -> anyhow::Result<Vec<AllowedRepos>> {
        let res = self
            .contract
            .view("repos")
            .args_json(json!({
                "page": page,
                "limit": limit,
            }))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to call allowed_repos: {:?}", e))?;
        let res = res.json()?;
        Ok(res)
    }

    #[instrument(skip(self))]
    pub async fn repos(&self) -> anyhow::Result<Vec<AllowedRepos>> {
        let mut page = 0;
        const LIMIT: usize = 500;
        let mut res = vec![];
        loop {
            let prs = self.repos_paged(page, LIMIT).await?;
            if prs.is_empty() {
                break;
            }
            res.extend(prs);
            page += 1;
        }
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
