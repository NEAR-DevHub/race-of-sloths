use std::sync::Arc;

use anyhow::bail;
use near_api::{signer::Signer, types::Data, Contract, NetworkConfig};
use near_primitives::{
    types::BlockReference,
    views::{FinalExecutionOutcomeView, FinalExecutionStatus},
};
use serde_json::json;
use tracing::instrument;

use super::github::PrMetadata;

use crate::*;

#[derive(Clone)]
pub struct NearClient {
    network: NetworkConfig,
    signer: Arc<Signer>,
    contract: Contract,
}

impl NearClient {
    pub async fn new(
        contract: String,
        sk: String,
        mainnet: bool,
        rpc_addr: Option<String>,
    ) -> anyhow::Result<Self> {
        let signer = Signer::new(Signer::secret_key(sk.parse()?))?;
        let mut network = if mainnet {
            NetworkConfig::mainnet()
        } else {
            NetworkConfig::testnet()
        };

        if let Some(rpc_addr) = rpc_addr {
            network.rpc_url = rpc_addr.parse()?;
        }

        let contract = near_api::Contract(contract.parse()?);
        Ok(Self {
            network,
            signer,
            contract,
        })
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
            .call_function("sloth_include", args)?
            .transaction()
            .with_signer(self.contract.0.clone(), self.signer.clone())
            .send_to(&self.network)
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
            .call_function("sloth_scored", args)?
            .transaction()
            .with_signer(self.contract.0.clone(), self.signer.clone())
            .send_to(&self.network)
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
            .call_function("sloth_merged", args)?
            .transaction()
            .with_signer(self.contract.0.clone(), self.signer.clone())
            .send_to(&self.network)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to call sloth_merged: {:?}", e))?;

        process_execution_final_result(result)
    }

    #[instrument(skip(self))]
    pub async fn send_pause(&self, organization: &str, repo: &str) -> anyhow::Result<Vec<Event>> {
        let result = self
            .contract
            .call_function(
                "pause_repo",
                json!({
                    "organization": organization,
                    "repo": repo,
                }),
            )?
            .transaction()
            .with_signer(self.contract.0.clone(), self.signer.clone())
            .send_to(&self.network)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to call sloth_paused: {:?}", e))?;
        process_execution_final_result(result)
    }

    #[instrument(skip(self))]
    pub async fn send_unpause(&self, organization: &str, repo: &str) -> anyhow::Result<Vec<Event>> {
        let result = self
            .contract
            .call_function(
                "unpause_repo",
                json!({
                    "organization": organization,
                    "repo": repo,
                }),
            )?
            .transaction()
            .with_signer(self.contract.0.clone(), self.signer.clone())
            .send_to(&self.network)
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

        let res: Data<PRInfo> = self
            .contract
            .call_function("check_info", args)?
            .read_only()
            .at(BlockReference::latest())
            .fetch_from(&self.network)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to call check_info: {:?}", e))?;
        Ok(res.data)
    }

    #[instrument(skip(self))]
    pub async fn unmerged_prs(&self, page: u64, limit: u64) -> anyhow::Result<Vec<PRv2>> {
        let args = json!({
            "page": page,
            "limit": limit,
        });

        let res: Data<Vec<PRv2>> = self
            .contract
            .call_function("unmerged_prs", args)?
            .read_only()
            .fetch_from(&self.network)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to call unmerged_prs: {:?}", e))?;
        Ok(res.data)
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

        let res: Data<Vec<PRv2>> = self
            .contract
            .call_function("unfinalized_prs", args)?
            .read_only()
            .fetch_from(&self.network)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to call unfinalized_prs: {:?}", e))?;
        Ok(res.data)
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
            .call_function("sloth_stale", args)?
            .transaction()
            .with_signer(self.contract.0.clone(), self.signer.clone())
            .send_to(&self.network)
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
            .call_function("sloth_exclude", args)?
            .transaction()
            .with_signer(self.contract.0.clone(), self.signer.clone())
            .send_to(&self.network)
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
            .call_function(
                "sloth_finalize",
                json!({
                    "pr_id": pr_id,
                    "active_pr": active_pr
                }),
            )?
            .transaction()
            .with_signer(self.contract.0.clone(), self.signer.clone())
            .send_to(&self.network)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to call sloth_finalize: {:?}", e))?;
        process_execution_final_result(result)
    }

    #[instrument(skip(self))]
    pub async fn user_info(
        &self,
        user: &str,
        periods: Vec<TimePeriodString>,
    ) -> anyhow::Result<Option<User>> {
        let res: Data<Option<User>> = self
            .contract
            .call_function(
                "user",
                json!({
                    "user": user,
                    "periods": periods
                }),
            )?
            .read_only()
            .fetch_from(&self.network)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to call user_info: {:?}", e))?;
        Ok(res.data)
    }

    pub async fn users_paged(
        &self,
        page: u64,
        limit: u64,
        periods: Vec<TimePeriodString>,
    ) -> anyhow::Result<Vec<User>> {
        let res: Data<Vec<User>> = self
            .contract
            .call_function(
                "users",
                json!({
                    "page": page,
                    "limit": limit,
                    "periods": periods,
                }),
            )?
            .read_only()
            .fetch_from(&self.network)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to call users: {:?}", e))?;
        Ok(res.data)
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
        let res: Data<Vec<(PRv2, bool)>> = self
            .contract
            .call_function(
                "prs",
                json!({
                    "page": page,
                    "limit": limit,
                }),
            )?
            .read_only()
            .fetch_from(&self.network)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to call prs: {:?}", e))?;
        Ok(res.data)
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
            .call_function(
                "repos",
                json!({
                    "page": page,
                    "limit": limit,
                }),
            )?
            .read_only()
            .fetch_from(&self.network)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to call allowed_repos: {:?}", e))?;
        Ok(res.data)
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

fn process_execution_final_result(result: FinalExecutionOutcomeView) -> anyhow::Result<Vec<Event>> {
    if !matches!(result.status, FinalExecutionStatus::SuccessValue(_)) {
        bail!("Execution failure: {:?}", result);
    }

    let events = result
        .transaction_outcome
        .outcome
        .logs
        .into_iter()
        .chain(
            result
                .receipts_outcome
                .into_iter()
                .flat_map(|o| o.outcome.logs),
        )
        .flat_map(|l| serde_json::from_str::<Event>(&l).ok())
        .collect();

    Ok(events)
}
