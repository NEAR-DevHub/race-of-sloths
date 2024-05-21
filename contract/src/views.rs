use near_sdk::near_bindgen;
use shared_types::{PRInfo, User};

use super::*;

#[near_bindgen]
impl Contract {
    pub fn check_info(&self, organization: String, repo: String, issue_id: u64) -> PRInfo {
        let pr_id = format!("{}/{}/{}", organization, repo, issue_id);
        let executed_pr = self.executed_prs.get(&pr_id);
        let pr = self.prs.get(&pr_id).or(executed_pr);
        let pr: Option<PR> = pr.cloned().map(|pr| pr.into());
        let organization = self.organizations.get(&organization);
        PRInfo {
            allowed_org: organization.is_some(),
            allowed_repo: organization
                .map(|org| org.is_allowed(&repo))
                .unwrap_or_default(),
            exist: pr.is_some(),
            merged: pr
                .as_ref()
                .map(|pr| pr.merged_at.is_some())
                .unwrap_or_default(),
            executed: executed_pr.is_some(),
            excluded: self.excluded_prs.contains(&pr_id),
            votes: pr.as_ref().map(|pr| pr.score.clone()).unwrap_or_default(),
            comment_id: pr.map(|pr| pr.comment_id).unwrap_or_default(),
        }
    }

    pub fn prs(&self, page: u64, limit: u64) -> Vec<PR> {
        self.prs
            .values()
            .skip((page * limit) as usize)
            .take(limit as usize)
            .cloned()
            .map(Into::into)
            .collect()
    }

    pub fn unmerged_prs(&self, page: u64, limit: u64) -> Vec<PR> {
        self.prs
            .values()
            .filter(|pr| !pr.is_merged())
            .skip((page * limit) as usize)
            .take(limit as usize)
            .cloned()
            .map(Into::into)
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
            .map(Into::into)
            .collect()
    }

    pub fn user_streaks(&self, user: &String) -> Vec<(StreakId, StreakUserData)> {
        self.streaks
            .into_iter()
            .filter(|s| s.is_active())
            .filter_map(|streak| {
                self.user_streaks
                    .get(&(user.to_string(), streak.id()))
                    .map(|data| (streak.id(), data.clone().into()))
            })
            .collect()
    }

    pub fn period_data(&self, user: &String, period_string: &String) -> Option<UserPeriodData> {
        self.sloths_per_period
            .get(&(user.to_string(), period_string.to_string()))
            .cloned()
            .map(Into::into)
    }

    pub fn user(&self, user: &String, period_string: Option<String>) -> Option<User> {
        let period = period_string
            .unwrap_or_else(|| TimePeriod::AllTime.time_string(env::block_timestamp()));
        self.accounts.get(user).map(|_| User {
            name: user.to_string(),
            period_data: self.period_data(user, &period).unwrap_or_default(),
            streaks: self.user_streaks(user),
        })
    }

    pub fn users(&self, limit: u64, page: u64, period_string: Option<String>) -> Vec<User> {
        let period = period_string
            .unwrap_or_else(|| TimePeriod::AllTime.time_string(env::block_timestamp()));
        self.accounts
            .iter()
            .skip((page * limit) as usize)
            .filter_map(|(user, _data)| self.user(user, Some(period.clone())))
            .collect()
    }
}
