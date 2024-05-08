use near_sdk::{
    near_bindgen,
    serde::{Deserialize, Serialize},
    NearSchema,
};

use super::*;

#[derive(Serialize, Deserialize, NearSchema)]
#[serde(crate = "near_sdk::serde")]
pub struct PRInfo {
    allowed: bool,
    exist: bool,
    merged: bool,
    scored: bool,
    executed: bool,
}

#[derive(Serialize, Deserialize, NearSchema)]
#[serde(crate = "near_sdk::serde")]
pub struct PRData {
    pub organization: String,
    pub repo: String,
    pub number: u64,
}

#[near_bindgen]
impl Contract {
    pub fn check_info(&self, organization: String, repo: String, issue_id: u64) -> PRInfo {
        let pr_id = format!("{}/{}/{}", organization, repo, issue_id);
        let pr = self.prs.get(&pr_id);
        let executed_pr = self.executed_prs.get(&pr_id);
        PRInfo {
            allowed: self
                .organizations
                .get(&organization)
                .map(|org| org.is_allowed(&repo))
                .unwrap_or_default(),
            exist: pr.is_some() || executed_pr.is_some(),
            merged: executed_pr.is_some()
                || pr.map(|pr| pr.merged_at.is_some()).unwrap_or_default(),
            scored: executed_pr.is_some() || pr.map(|pr| pr.score().is_some()).unwrap_or_default(),
            executed: executed_pr.is_some(),
        }
    }

    pub fn unmerged_prs(&self, page: u64, limit: u64) -> Vec<PRData> {
        self.prs
            .values()
            .filter(|pr| pr.merged_at.is_none())
            .skip((page * limit) as usize)
            .take(limit as usize)
            .cloned()
            .map(|pr| PRData {
                organization: pr.organization,
                repo: pr.repo,
                number: pr.number,
            })
            .collect()
    }
}
