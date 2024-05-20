use near_sdk::near_bindgen;
use shared_types::PRInfo;

use super::*;

#[near_bindgen]
impl Contract {
    pub fn check_info(&self, organization: String, repo: String, issue_id: u64) -> PRInfo {
        let pr_id = format!("{}/{}/{}", organization, repo, issue_id);
        let executed_pr = self.executed_prs.get(&pr_id);
        let pr = self.prs.get(&pr_id).or(executed_pr);
        let organization = self.organizations.get(&organization);
        PRInfo {
            allowed_org: organization.is_some(),
            allowed_repo: organization
                .map(|org| org.is_allowed(&repo))
                .unwrap_or_default(),
            exist: pr.is_some(),
            merged: pr.map(|pr| pr.merged_at.is_some()).unwrap_or_default(),
            scored: pr.map(|pr| pr.score().is_some()).unwrap_or_default(),
            executed: executed_pr.is_some(),
            excluded: self.excluded_prs.contains(&pr_id),
            votes: pr.map(|pr| pr.score.clone()).unwrap_or_default(),
            comment_id: pr.map(|pr| pr.comment_id).unwrap_or_default(),
        }
    }

    pub fn unmerged_prs(&self, page: u64, limit: u64) -> Vec<PR> {
        self.prs
            .values()
            .filter(|pr| pr.merged_at.is_none())
            .skip((page * limit) as usize)
            .take(limit as usize)
            .cloned()
            .collect()
    }

    pub fn unfinalized_prs(&self, page: u64, limit: u64) -> Vec<PR> {
        let timestamp = env::block_timestamp();
        self.prs
            .values()
            .filter(|pr| pr.is_ready_to_move(timestamp))
            .skip((page * limit) as usize)
            .take(limit as usize)
            .cloned()
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
}
