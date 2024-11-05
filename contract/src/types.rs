use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
    NearSchema,
};

#[derive(
    Debug, Clone, Copy, BorshSerialize, BorshDeserialize, Serialize, Deserialize, NearSchema,
)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub enum VersionedRepository {
    V1(Repository),
    V2(RepositoryV2),
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    NearSchema,
)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub enum RepositoryStatus {
    Paused,
    Active,
    Blocked,
}

impl VersionedRepository {
    pub fn is_active(&self) -> bool {
        let v2: RepositoryV2 = self.into();
        v2.status == RepositoryStatus::Active
    }

    pub fn is_paused(&self) -> bool {
        let v2: RepositoryV2 = self.into();
        v2.status == RepositoryStatus::Paused
    }

    pub fn is_blocked(&self) -> bool {
        let v2: RepositoryV2 = self.into();
        v2.status == RepositoryStatus::Blocked
    }
}

impl From<&VersionedRepository> for RepositoryV2 {
    fn from(value: &VersionedRepository) -> Self {
        match value {
            VersionedRepository::V1(data) => RepositoryV2 {
                status: if data.paused {
                    RepositoryStatus::Paused
                } else {
                    RepositoryStatus::Active
                },
            },
            VersionedRepository::V2(data) => *data,
        }
    }
}

#[derive(
    Debug, Copy, Clone, BorshSerialize, BorshDeserialize, Serialize, Deserialize, NearSchema,
)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub struct Repository {
    pub paused: bool,
}

#[derive(
    Debug, Copy, Clone, BorshSerialize, BorshDeserialize, Serialize, Deserialize, NearSchema,
)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub struct RepositoryV2 {
    pub status: RepositoryStatus,
}
