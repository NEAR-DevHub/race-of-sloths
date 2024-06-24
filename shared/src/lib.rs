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

pub type UserId = u32;

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

#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, NearSchema)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub struct LifetimeBonusStorage {
    pub streak_id: StreakId,
    pub percent: u32,
    // It's actually a dirty hack to avoid creating more sophisticated data structures
    // The lifetime bonus can be received on pr cration, but we congratulate user only in the end of the PR
    // So we need to know if there is a new bonus to congratulate user
    // TODO: refactor this
    pub new: bool,
}

#[derive(
    Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, NearSchema, Default,
)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub struct AccountWithPermanentPercentageBonus {
    pub account_id: Option<AccountId>,
    pub github_handle: GithubHandle,
    pub permanent_percentage_bonus: Vec<LifetimeBonusStorage>,
    pub flat_bonus: Vec<FlatBonusStorage>,
}

impl AccountWithPermanentPercentageBonus {
    pub fn new(github_handle: GithubHandle) -> Self {
        Self {
            github_handle,
            permanent_percentage_bonus: Vec::new(),
            flat_bonus: Vec::new(),
            account_id: None,
        }
    }

    pub fn add_streak_percent(&mut self, streak_id: StreakId, streak_percent_bonus: u32) -> bool {
        if let Some(bonus) = self
            .permanent_percentage_bonus
            .iter_mut()
            .find(|bonus| bonus.streak_id == streak_id)
        {
            let old = bonus.percent;
            bonus.percent = streak_percent_bonus.max(old);
            bonus.new = bonus.new || bonus.percent > old;
            bonus.new
        } else {
            self.permanent_percentage_bonus.push(LifetimeBonusStorage {
                streak_id,
                percent: streak_percent_bonus,
                new: true,
            });
            true
        }
    }

    pub fn add_flat_bonus(&mut self, streak_id: StreakId, reward: u32, streak_min: u32) -> bool {
        if self
            .flat_bonus
            .iter()
            .any(|bonus| bonus.streak_id == streak_id && bonus.streak_min == streak_min)
        {
            return false;
        }

        self.flat_bonus.push(FlatBonusStorage {
            streak_id,
            reward,
            streak_min,
        });
        true
    }

    pub fn lifetime_percentage_bonus(&self) -> u32 {
        self.permanent_percentage_bonus
            .iter()
            .map(|bonus| bonus.percent)
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

    // Clear new flags and return the sum of all new bonuses
    // TODO: refactor this. See the comment in the struct
    pub fn clear_new_flags(&mut self) -> u32 {
        let mut new_bonus = 0;
        for bonus in self.permanent_percentage_bonus.iter_mut() {
            if bonus.new {
                new_bonus += bonus.percent;
            }
            bonus.new = false;
        }
        new_bonus
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

    pub fn pr_executed_with_score(&mut self, score: u32) {
        let mut data: UserPeriodData = self.clone().into();
        data.executed_prs += 1;
        data.total_score += score;
        if score > data.largest_score {
            data.largest_score = score;
        }

        *self = VersionedUserPeriodData::V1(data);
    }

    pub fn pr_final_rating(&mut self, rating: u32) {
        let mut data: UserPeriodData = self.clone().into();
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
    pub id: UserId,
    pub name: GithubHandle,
    pub percentage_bonus: u32,
    pub period_data: Vec<(TimePeriodString, UserPeriodData)>,
    pub streaks: Vec<(StreakId, StreakUserData)>,
}
