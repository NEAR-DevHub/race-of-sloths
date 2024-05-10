use std::collections::HashSet;

use chrono::{DateTime, Datelike};
use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
    AccountId, NearSchema, Timestamp,
};

pub type MonthYearString = String;

// We need to carefully think what we want to store in the contract storage
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, NearSchema)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub struct UserData {
    pub handle: String,
    pub total_prs_merged: u32,
    pub total_prs_opened: u32,
    pub total_score: u32,
    // Created for the future, but we would need to think more about it
    pub account_id: Option<AccountId>,
}

impl UserData {
    pub fn new(handle: String) -> Self {
        Self {
            handle,
            total_prs_merged: 0,
            total_score: 0,
            total_prs_opened: 0,
            account_id: None,
        }
    }

    pub fn add_score(&mut self, score: u32) {
        self.total_prs_merged += 1;
        self.total_score += score;
    }

    pub fn add_opened_pr(&mut self) {
        self.total_prs_opened += 1;
    }
}

pub fn timestamp_to_month_string(timestamp: u64) -> MonthYearString {
    let date = DateTime::from_timestamp_nanos(timestamp as i64);
    format!("{:02}{:04}", date.month(), date.year())
}

#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, NearSchema)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
enum PermissionModel {
    // Represents the list of repositories that are allowed
    Allowlist(HashSet<String>),
    // Represents `all, except` list of repositories
    Blocklist(HashSet<String>),
}

#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, NearSchema)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub struct Organization {
    pub name: String,
    all: PermissionModel,
}

impl Organization {
    pub fn new_all(name: String) -> Self {
        Self {
            name,
            all: PermissionModel::Blocklist(HashSet::new()),
        }
    }

    pub fn new_only(name: String, only: HashSet<String>) -> Self {
        Self {
            name,
            all: PermissionModel::Allowlist(only),
        }
    }

    pub fn exclude(&mut self, repo: &str) {
        match &mut self.all {
            PermissionModel::Allowlist(allowlist) => allowlist.remove(repo),
            PermissionModel::Blocklist(blocklist) => blocklist.insert(repo.to_string()),
        };
    }

    pub fn include(&mut self, repo: &str) {
        match &mut self.all {
            PermissionModel::Allowlist(allowlist) => allowlist.insert(repo.to_string()),
            PermissionModel::Blocklist(blocklist) => blocklist.remove(repo),
        };
    }

    pub fn is_allowed(&self, repo: &str) -> bool {
        match &self.all {
            PermissionModel::Allowlist(allowlist) => allowlist.contains(repo),
            PermissionModel::Blocklist(blocklist) => !blocklist.contains(repo),
        }
    }
}

#[derive(
    Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, NearSchema, PartialEq,
)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub struct Score {
    pub user: String,
    pub score: u32,
}

#[derive(
    Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, NearSchema, PartialEq,
)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub struct PR {
    pub organization: String,
    pub repo: String,
    pub number: u64,
    pub author: String,
    score: Vec<Score>,
    pub created_at: Timestamp,
    pub merged_at: Option<Timestamp>,
}

impl PR {
    pub fn new(
        organization: String,
        repo: String,
        number: u64,
        author: String,
        created_at: Timestamp,
    ) -> Self {
        Self {
            organization,
            repo,
            number,
            author,
            created_at,

            score: vec![],
            merged_at: None,
        }
    }

    pub fn add_score(&mut self, user: String, score: u32) {
        if let Some(user) = self.score.iter_mut().find(|s| s.user == user) {
            user.score = score;
        } else {
            self.score.push(Score { user, score });
        }
    }

    pub fn add_merge_info(&mut self, merged_at: Timestamp) {
        self.merged_at = Some(merged_at);
    }

    pub fn is_ready_to_move(&self, timestamp: Timestamp) -> bool {
        const SCORE_TIMEOUT_IN_SECONDS: Timestamp = 24 * 60 * 60;
        const SCORE_TIMEOUT_IN_NANOSECONDS: Timestamp = SCORE_TIMEOUT_IN_SECONDS * 1_000_000_000;

        self.merged_at.is_some()
            && (timestamp - self.merged_at.unwrap()) > SCORE_TIMEOUT_IN_NANOSECONDS
    }

    pub fn score(&self) -> Option<u32> {
        self.score
            .iter()
            .map(|s| s.score)
            .sum::<u32>()
            .checked_div(self.score.len() as u32)
    }

    pub fn full_id(&self) -> String {
        format!("{}/{}/{}", self.organization, self.repo, self.number)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_to_code() {
        let timestamp = 1625097600000000000; // 2021-07-01
        let code = timestamp_to_month_string(timestamp);
        assert_eq!(code, "072021");
    }
}
