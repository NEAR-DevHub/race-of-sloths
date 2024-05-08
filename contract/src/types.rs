use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Datelike};
use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
    AccountId, NearSchema, Timestamp,
};

type MonthYearCode = [u8; 6];

// We need to carefully think what we want to store in the contract storage
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, NearSchema)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub struct UserData {
    pub handle: String,
    pub score: HashMap<MonthYearCode, u32>,
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
            score: HashMap::new(),
            total_prs_merged: 0,
            total_score: 0,
            total_prs_opened: 0,
            account_id: None,
        }
    }

    pub fn add_score(&mut self, score: u32, merged_at: Timestamp) {
        let month_year_code = timestamp_to_code(merged_at);

        self.score.insert(month_year_code, score);
        self.total_prs_merged += 1;
        self.total_score += score;
    }

    pub fn add_opened_pr(&mut self) {
        self.total_prs_opened += 1;
    }
}

fn timestamp_to_code(timestamp: u64) -> MonthYearCode {
    let date = DateTime::from_timestamp_nanos(timestamp as i64);

    let month = date.month();
    let year = date.year();
    let code = format!("{:02}{:04}", month, year);
    let code = code.as_bytes();
    let mut month_year_code = [0u8; 6];
    month_year_code.copy_from_slice(code);
    month_year_code
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

    pub fn is_ready_to_move(&self) -> bool {
        !self.score.is_empty() && self.merged_at.is_some()
    }

    pub fn score(&self) -> Option<u32> {
        self.score
            .iter()
            .map(|s| s.score)
            .sum::<u32>()
            .checked_div(self.score.len() as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_to_code() {
        let timestamp = 1625097600000000000; // 2021-07-01
        let code = timestamp_to_code(timestamp);
        assert_eq!(code, [b'0', b'7', b'2', b'0', b'2', b'1']);
    }
}
