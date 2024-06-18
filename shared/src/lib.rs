use std::ops::Add;

use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
    AccountId, NearSchema,
};

mod event;
mod pr;
mod streak;
mod timeperiod;

#[cfg(feature = "github")]
pub mod github;

#[cfg(feature = "client")]
pub mod near;

pub use event::*;
pub use pr::*;
pub use streak::*;
pub use timeperiod::*;

pub type GithubHandle = String;

#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, NearSchema)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub enum VersionedAccount {
    V1(AccountWithPermanentPercentageBonus),
}

impl From<VersionedAccount> for AccountWithPermanentPercentageBonus {
    fn from(message: VersionedAccount) -> Self {
        match message {
            VersionedAccount::V1(x) => x,
        }
    }
}

#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, NearSchema)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub struct FlatBonusStorage {
    pub streak_id: StreakId,
    pub reward: u32,
    pub streak_min: u32,
}

#[derive(
    Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, NearSchema, Default,
)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub struct AccountWithPermanentPercentageBonus {
    pub account_id: Option<AccountId>,
    permanent_percentage_bonus: Vec<(StreakId, u32)>,
    flat_bonus: Vec<FlatBonusStorage>,
}

impl AccountWithPermanentPercentageBonus {
    pub fn add_streak_percent(&mut self, streak_id: StreakId, streak_percent_bonus: u32) -> bool {
        if let Some((_, percent)) = self
            .permanent_percentage_bonus
            .iter_mut()
            .find(|(id, _)| *id == streak_id)
        {
            let old = *percent;
            *percent = streak_percent_bonus.max(old);
            *percent > old
        } else {
            self.permanent_percentage_bonus
                .push((streak_id, streak_percent_bonus));
            true
        }
    }

    pub fn add_flat_bonus(&mut self, streak_id: StreakId, reward: u32, streak_min: u32) {
        self.flat_bonus.push(FlatBonusStorage {
            streak_id,
            reward,
            streak_min,
        });
    }

    pub fn lifetime_percentage_bonus(&self) -> u32 {
        self.permanent_percentage_bonus
            .iter()
            .map(|(_, bonus)| *bonus)
            .reduce(Add::add)
            .unwrap_or_default()
    }

    // Use bonus reward if result >= streak_min and remove it from the list
    pub fn use_flat_bonus(&mut self, streak_id: StreakId, result: u32) -> u32 {
        if let Some((index, _)) = self
            .flat_bonus
            .iter()
            .enumerate()
            .find(|(_, bonus)| bonus.streak_id == streak_id && result >= bonus.streak_min)
        {
            self.flat_bonus.swap_remove(index).reward
        } else {
            0
        }
    }
}

#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, NearSchema)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub enum VersionedUserPeriodData {
    V1(UserPeriodData),
}

impl VersionedUserPeriodData {
    pub fn pr_opened(&mut self) {
        let mut data: UserPeriodData = self.clone().into();
        data.prs_opened += 1;
        *self = VersionedUserPeriodData::V1(data);
    }

    pub fn pr_merged(&mut self) {
        let mut data: UserPeriodData = self.clone().into();
        data.prs_merged += 1;
        *self = VersionedUserPeriodData::V1(data);
    }

    pub fn pr_executed(&mut self, score: u32, rating: u32) {
        let mut data: UserPeriodData = self.clone().into();
        data.executed_prs += 1;
        data.total_score += score;
        if score > data.largest_score {
            data.largest_score = score;
        }
        data.total_rating += rating;
        if rating > data.largest_rating_per_pr {
            data.largest_rating_per_pr = rating;
        }
        *self = VersionedUserPeriodData::V1(data);
    }

    pub fn pr_closed(&mut self) {
        let mut data: UserPeriodData = self.clone().into();
        data.prs_opened -= 1;
        *self = VersionedUserPeriodData::V1(data);
    }

    pub fn pr_rating_bonus(&mut self, old_pr_rating: u32, new_pr_rating: u32) {
        let mut data: UserPeriodData = self.clone().into();
        data.total_rating -= old_pr_rating;
        data.total_rating += new_pr_rating;
        if new_pr_rating > data.largest_rating_per_pr {
            data.largest_rating_per_pr = new_pr_rating;
        }
        *self = VersionedUserPeriodData::V1(data);
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
    pub total_rating: u32,
    pub largest_rating_per_pr: u32,
}

#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, NearSchema)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub struct User {
    pub name: GithubHandle,
    pub percentage_bonus: u32,
    pub period_data: Vec<(TimePeriodString, UserPeriodData)>,
    pub streaks: Vec<(StreakId, StreakUserData)>,
}
