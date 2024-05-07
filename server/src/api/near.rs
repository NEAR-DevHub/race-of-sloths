use anyhow::bail;
use near_workspaces::{network::Testnet, types::SecretKey, Contract, Worker};
use serde_json::json;

use super::github::PrMetadata;

#[derive(Clone)]
pub struct NearClient {
    worker: Worker<Testnet>,
    contract: Contract,
}

impl NearClient {
    pub async fn new(contract: String, sk: SecretKey, mainnet: bool) -> anyhow::Result<Self> {
        if mainnet {
            bail!("Mainnet is not supported yet")
        }
        let testnet = near_workspaces::testnet().await?;
        let contract = Contract::from_secret_key(contract.parse()?, sk, &testnet);
        Ok(Self {
            worker: testnet,
            contract,
        })
    }

    pub async fn send_start(&self, pr: PrMetadata) -> anyhow::Result<()> {
        let args = json!({
            "organization": pr.owner,
            "repo": pr.repo,
            "pr_number": pr.number,
            "user": pr.author.login,
            "created_at": pr.started.timestamp_nanos_opt().unwrap_or(0),
        });

        let tx = self
            .contract
            .call("sloth_called")
            .args_json(args)
            .transact_async()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to call sloth_called: {:?}", e))?;
        let _ = tx.await?;
        Ok(())
    }

    pub async fn send_scored(&self, pr: PrMetadata, score: u64) -> anyhow::Result<()> {
        let args = json!({
            "pr_id": format!("{}/{}/{}", pr.owner, pr.repo, pr.number),
            "score": score,
        });

        let tx = self
            .contract
            .call("sloth_scored")
            .args_json(args)
            .transact_async()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to call sloth_scored: {:?}", e))?;
        let _ = tx.await?;
        Ok(())
    }

    pub async fn send_merge(&self, pr: PrMetadata) -> anyhow::Result<()> {
        if pr.merged.is_none() {
            bail!("PR is not merged")
        }

        let args = json!({
            "pr_id": format!("{}/{}/{}", pr.owner, pr.repo, pr.number),
            "merged_at": pr.merged.unwrap().timestamp_nanos_opt().unwrap_or(0),
        });

        let tx = self
            .contract
            .call("sloth_merged")
            .args_json(args)
            .transact_async()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to call sloth_merged: {:?}", e))?;
        let _ = tx.await?;
        Ok(())
    }
}
