use near_sdk::serde::{Deserialize, Serialize};

use crate::StreakId;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
pub enum Event {
    StreakIncreased {
        streak_id: StreakId,
        new_streak: u32,
        largest_streak: u32,
        name: String,
    },
    NewSloth {
        github_handle: String,
    },
}
