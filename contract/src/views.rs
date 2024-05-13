use near_sdk::{
    near_bindgen,
    serde::{Deserialize, Serialize},
    NearSchema,
};

use super::*;

#[derive(Serialize, Deserialize, NearSchema)]
#[serde(crate = "near_sdk::serde")]
pub struct PRInfo {
    allowed_org: bool,
    allowed_repo: bool,
    exist: bool,
    merged: bool,
    scored: bool,
    executed: bool,
    excluded: bool,
}

#[derive(Debug, Serialize, Deserialize, NearSchema)]
#[serde(crate = "near_sdk::serde")]
pub struct UserWithMonthScore {
    user: UserData,
    score: u32,
    month: MonthYearString,
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
        let organization = self.organizations.get(&organization);
        PRInfo {
            allowed_org: organization.is_some(),
            allowed_repo: organization
                .map(|org| org.is_allowed(&repo))
                .unwrap_or_default(),
            exist: pr.is_some() || executed_pr.is_some(),
            merged: executed_pr.is_some()
                || pr.map(|pr| pr.merged_at.is_some()).unwrap_or_default(),
            scored: executed_pr.is_some() || pr.map(|pr| pr.score().is_some()).unwrap_or_default(),
            executed: executed_pr.is_some(),
            excluded: self.excluded_prs.contains(&pr_id),
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

    pub fn users(
        &self,
        limit: u64,
        page: u64,
        year_month_string: Option<String>,
    ) -> Vec<UserWithMonthScore> {
        let month =
            year_month_string.unwrap_or_else(|| timestamp_to_month_string(env::block_timestamp()));

        self.sloths
            .values()
            .skip((page * limit) as usize)
            .take(limit as usize)
            .map(|user| UserWithMonthScore {
                user: user.clone(),
                score: self
                    .sloths_per_month
                    .get(&(user.handle.clone(), month.clone()))
                    .copied()
                    .unwrap_or(0),
                month: month.clone(),
            })
            .collect()
    }

    pub fn should_finalize(&self) -> bool {
        let time: u64 = env::block_timestamp();
        self.prs.iter().any(|(_, pr)| pr.is_ready_to_move(time))
    }
}
