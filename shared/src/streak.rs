use super::*;

#[derive(
    BorshDeserialize, BorshSerialize, Serialize, Deserialize, NearSchema, Debug, Clone, Copy,
)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub enum StreakReward {
    FlatReward(u32),
    PermanentPercentageBonus(u32),
}

#[derive(
    BorshDeserialize, BorshSerialize, Serialize, Deserialize, NearSchema, Debug, Clone, Copy,
)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub enum StreakType {
    PRsOpened(u32),
    PRsMerged(u32),
    TotalScore(u32),
    LargestScore(u32),
    AverageScore(u32),
}

impl StreakType {
    pub const fn from_prs_opened(value: u32) -> Self {
        Self::PRsOpened(value)
    }

    pub const fn from_prs_merged(value: u32) -> Self {
        Self::PRsMerged(value)
    }

    pub const fn is_streak_achieved(&self, user_period_data: &UserPeriodDataV2) -> bool {
        match self {
            Self::PRsOpened(value) => user_period_data.prs_opened >= *value,
            Self::PRsMerged(value) => user_period_data.prs_merged >= *value,
            Self::TotalScore(score) => user_period_data.total_score >= *score,
            Self::LargestScore(score) => user_period_data.largest_score >= *score,
            Self::AverageScore(score) => {
                user_period_data.total_score / user_period_data.executed_prs >= *score
            }
        }
    }
}

pub type StreakId = u32;

#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, NearSchema)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub enum VersionedStreak {
    V1(Streak),
}

impl VersionedStreak {
    pub const fn is_active(&self) -> bool {
        match self {
            Self::V1(streak) => streak.is_active,
        }
    }

    pub const fn id(&self) -> StreakId {
        match self {
            Self::V1(streak) => streak.id,
        }
    }
}

impl From<VersionedStreak> for Streak {
    fn from(message: VersionedStreak) -> Self {
        match message {
            VersionedStreak::V1(x) => x,
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, NearSchema, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub struct Streak {
    pub id: StreakId,
    pub name: String,
    pub time_period: TimePeriod,
    pub streak_criterias: Vec<StreakType>,
    pub streak_rewards: Vec<StreakReward>,
    pub is_active: bool,
}

impl Streak {
    pub fn new(
        streak_id: u32,
        name: String,
        time_period: TimePeriod,
        streak_criterias: Vec<StreakType>,
        streak_rewards: Vec<StreakReward>,
    ) -> Self {
        assert!(
            !streak_criterias.is_empty(),
            "Streak criteria should not be empty"
        );
        assert_ne!(
            time_period,
            TimePeriod::AllTime,
            "All time is not allowed for streaks"
        );

        Self {
            id: streak_id,
            name,
            time_period,
            streak_criterias,
            is_active: true,
            streak_rewards,
        }
    }

    pub fn is_streak_achieved(&self, user_period_data: &VersionedUserPeriodData) -> bool {
        self.streak_criterias
            .iter()
            .all(|criteria| criteria.is_streak_achieved(&user_period_data.clone().into()))
    }

    pub fn get_streak_reward(&self, streak: u32) -> Option<StreakReward> {
        let streak = if streak >= self.streak_rewards.len() as u32 {
            self.streak_rewards.len() as u32 - 1
        } else if streak == 0 {
            return None;
        } else {
            streak - 1
        };

        Some(self.streak_rewards[streak as usize])
    }
}

#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, NearSchema)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub enum VersionedStreakUserData {
    V1(StreakUserData),
}

impl From<VersionedStreakUserData> for StreakUserData {
    fn from(message: VersionedStreakUserData) -> Self {
        match message {
            VersionedStreakUserData::V1(x) => x,
        }
    }
}

#[derive(
    BorshDeserialize, BorshSerialize, Serialize, Deserialize, NearSchema, Debug, Clone, Default,
)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub struct StreakUserData {
    pub amount: u32,
    pub best: u32,
    pub latest_time_string: TimePeriodString,
}
