use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
    NearSchema,
};

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, Serialize, Deserialize, NearSchema)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub enum VersionedRepository {
    V1(Repository),
}

impl VersionedRepository {
    pub fn is_paused(&self) -> bool {
        match self {
            VersionedRepository::V1(data) => data.paused,
        }
    }
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, Serialize, Deserialize, NearSchema)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub struct Repository {
    pub paused: bool,
}
