use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
    NearSchema,
};

mod pr;
mod streak;
mod timeperiod;

#[cfg(feature = "github")]
pub mod github;
#[cfg(feature = "client")]
pub mod near;

pub use pr::*;
pub use streak::*;
pub use timeperiod::*;

pub type GithubHandle = String;

#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, NearSchema)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub enum VersionedUserPeriodData {
    V1(UserPeriodData),
}

impl VersionedUserPeriodData {
    pub fn pr_opened(&mut self) {
        match self {
            VersionedUserPeriodData::V1(data) => data.prs_opened += 1,
        }
    }

    pub fn pr_merged(&mut self) {
        match self {
            VersionedUserPeriodData::V1(data) => data.prs_merged += 1,
        }
    }

    pub fn pr_executed(&mut self, score: u32) {
        match self {
            VersionedUserPeriodData::V1(data) => {
                data.executed_prs += 1;
                data.total_score += score;
                if score > data.largest_score {
                    data.largest_score = score;
                }
            }
        }
    }

    pub fn pr_closed(&mut self) {
        match self {
            VersionedUserPeriodData::V1(data) => data.prs_opened -= 1,
        }
    }
}

impl From<VersionedUserPeriodData> for UserPeriodData {
    fn from(message: VersionedUserPeriodData) -> Self {
        match message {
            VersionedUserPeriodData::V1(x) => x,
        }
    }
}

#[derive(
    Debug,
    Clone,
    BorshDeserialize,
    BorshSerialize,
    Serialize,
    Deserialize,
    NearSchema,
    PartialEq,
    Default,
)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub struct UserPeriodData {
    pub total_score: u32,
    pub executed_prs: u32,
    pub largest_score: u32,
    pub prs_opened: u32,
    pub prs_merged: u32,
}

#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, NearSchema)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub struct User {
    pub name: GithubHandle,
    pub period_data: Vec<(TimePeriodString, UserPeriodData)>,
    pub streaks: Vec<(StreakId, StreakUserData)>,
}
