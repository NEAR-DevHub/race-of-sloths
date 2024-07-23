use std::collections::HashMap;

use near_sdk::near_bindgen;
use shared::{PRInfo, PRWithRating, User, UserId, UserPeriodData};

use super::*;

#[near_bindgen]
impl Contract {
    pub fn check_info(&self, organization: String, repo: String, issue_id: u64) -> PRInfo {
        let pr_id = format!("{}/{}/{}", organization, repo, issue_id);
        let executed_pr = self.executed_prs.get(&pr_id);
        let pr = self.prs.get(&pr_id).or(executed_pr);
        let pr: Option<PRWithRating> = pr.cloned().map(|pr| pr.into());
        let repo_allowed = self.repos.get(&(organization, repo));

        PRInfo {
            allowed_repo: repo_allowed.is_some(),
            paused: repo_allowed
                .map(|repo| repo.is_paused())
                .unwrap_or_default(),
            exist: pr.is_some(),
            merged: pr
                .as_ref()
                .map(|pr| pr.merged_at.is_some())
                .unwrap_or_default(),
            executed: executed_pr.is_some(),
            excluded: self.excluded_prs.contains(&pr_id),
            votes: pr.as_ref().map(|pr| pr.score.clone()).unwrap_or_default(),
        }
    }

    /// Returns a list of PRs with the execution status
    pub fn prs(&self, limit: u64, page: u64) -> Vec<(PRWithRating, bool)> {
        self.prs
            .into_iter()
            .chain(self.executed_prs.iter())
            .skip((page * limit) as usize)
            .take(limit as usize)
            .map(|(id, pr)| (pr.clone().into(), !self.prs.contains_key(id)))
            .collect()
    }

    pub fn unmerged_prs(&self, page: u64, limit: u64) -> Vec<PRWithRating> {
        self.prs
            .values()
            .filter(|pr| !pr.is_merged())
            .skip((page * limit) as usize)
            .take(limit as usize)
            .cloned()
            .map(Into::into)
            .collect()
    }

    pub fn unfinalized_prs(&self, page: u64, limit: u64) -> Vec<PRWithRating> {
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

    pub fn user_streaks(&self, user_id: UserId) -> Vec<(StreakId, StreakUserData)> {
        self.streaks
            .into_iter()
            .filter(|s| s.is_active())
            .filter_map(|streak| {
                self.user_streaks
                    .get(&(user_id, streak.id()))
                    .map(|data| (streak.id(), data.clone().into()))
            })
            .collect()
    }

    pub fn period_data(&self, user_id: UserId, period_string: &String) -> Option<UserPeriodData> {
        self.sloths_per_period
            .get(&(user_id, period_string.to_string()))
            .cloned()
            .map(Into::into)
    }

    pub fn user(&self, user: &String, periods: Vec<TimePeriodString>) -> Option<User> {
        let user = *self.account_ids.get(user)?;
        self.user_by_id(user, periods)
    }

    pub fn user_by_id(&self, user_id: UserId, periods: Vec<TimePeriodString>) -> Option<User> {
        let u: AccountWithPermanentPercentageBonus = self.users.get(user_id)?.clone().into();
        let percentage_bonus = u.lifetime_percentage_bonus();

        Some(User {
            id: user_id,
            name: u.github_handle,
            percentage_bonus,
            period_data: periods
                .iter()
                .map(|period| {
                    (
                        period.clone(),
                        self.period_data(user_id, period).unwrap_or_default(),
                    )
                })
                .collect(),
            streaks: self.user_streaks(user_id),
        })
    }

    pub fn users(&self, limit: u64, page: u64, periods: Vec<TimePeriodString>) -> Vec<User> {
        (page * limit..(page + 1) * limit)
            .filter_map(|user_id| self.user_by_id(user_id as UserId, periods.clone()))
            .collect()
    }

    pub fn users_by_name(
        &self,
        users: Vec<GithubHandle>,
        periods: Vec<TimePeriodString>,
    ) -> Vec<User> {
        users
            .into_iter()
            .filter_map(|user| self.user(&user, periods.clone()))
            .collect()
    }

    // TODO: remove this method after we would have enough data in the PRs
    pub fn repos(&self) -> Vec<AllowedRepos> {
        let mut repos = HashMap::new();

        for ((org, repo), data) in self.repos.into_iter() {
            if data.is_paused() {
                continue;
            }

            repos
                .entry(org)
                .or_insert_with(|| AllowedRepos {
                    organization: org.clone(),
                    repos: vec![],
                })
                .repos
                .push(repo.clone());
        }

        repos.into_values().collect()
    }
}
