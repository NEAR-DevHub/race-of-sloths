use near_sdk::serde::{Deserialize, Serialize};

use crate::StreakId;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
pub enum Event {
    NewSloth {
        github_handle: String,
    },
    StreakFlatRewarded {
        streak_id: StreakId,
        streak_number: u32,
        bonus_rating: u32,
    },
    StreakLifetimeRewarded {
        streak_id: StreakId,
        lifetime_percent: u32,
        total_lifetime_percent: u32,
    },
}
