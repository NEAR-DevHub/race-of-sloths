use serde::{Deserialize, Serialize};
use shared::TimePeriodString;

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct LeaderboardRecord {
    pub name: String,
    pub total_score: i32,
    pub period_type: TimePeriodString,
    pub executed_prs: i32,
    pub largest_score: i32,
    pub prs_opened: i32,
    pub prs_merged: i32,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize, Default)]
pub struct UserPeriodRecord {
    pub period_type: TimePeriodString,
    pub total_score: i32,
    pub executed_prs: i32,
    pub largest_score: i32,
    pub prs_opened: i32,
    pub prs_merged: i32,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize, Default)]
pub struct StreakRecord {
    pub streak_id: i32,
    pub amount: i32,
    pub best: i32,
    pub latest_time_string: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRecord {
    pub name: String,
    pub period_data: Vec<UserPeriodRecord>,
    pub streaks: Vec<StreakRecord>,
}

impl UserRecord {
    pub fn newcommer(name: String) -> Self {
        Self {
            name,
            period_data: vec![],
            streaks: vec![],
        }
    }
}
