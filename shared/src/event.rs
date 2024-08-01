use near_sdk::serde::{Deserialize, Serialize};

use crate::StreakId;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
pub enum Event {
    NewSloth {
        user_id: u32,
        github_handle: String,
    },
    StreakFlatRewarded {
        streak_id: StreakId,
        streak_number: u32,
        bonus_rating: u32,
    },
    StreakLifetimeRewarded {
        reward: u32,
    },
    ExecutedWithRating {
        rating: u32,
        applied_multiplier: u32,
        pr_number_this_week: u32,
    },
    Autoscored {
        score: u32,
    },
}
