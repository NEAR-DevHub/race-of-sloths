use std::collections::HashSet;

use chrono::{DateTime, Datelike};
use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
    NearSchema,
};
use shared_types::MonthYearString;

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
