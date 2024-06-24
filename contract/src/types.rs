use std::collections::HashSet;

use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
    NearSchema,
};

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
pub enum VersionedOrganization {
    V1(Organization),
}

impl VersionedOrganization {
    pub fn is_allowed(&self, repo: &str) -> bool {
        match self {
            VersionedOrganization::V1(org) => org.is_allowed(repo),
        }
    }
}

impl From<VersionedOrganization> for Organization {
    fn from(message: VersionedOrganization) -> Self {
        match message {
            VersionedOrganization::V1(x) => x,
        }
    }
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

    pub fn repos(&self) -> Vec<String> {
        match &self.all {
            PermissionModel::Allowlist(allowlist) => allowlist.iter().cloned().collect(),
            PermissionModel::Blocklist(_blocklist) => vec![],
        }
    }
}
