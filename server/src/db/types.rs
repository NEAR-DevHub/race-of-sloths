use serde::{Deserialize, Serialize};
use shared::{GithubHandle, TimePeriodString};
#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct LeaderboardRecord {
    pub name: String,
    pub total_score: i32,
    pub period_type: TimePeriodString,
    pub executed_prs: i32,
    pub largest_score: i32,
    pub prs_opened: i32,
    pub prs_merged: i32,
    pub streak_best: i32,
    pub streak_amount: i32,
    pub streak_name: String,
    pub streak_type: String,
    pub streak_latest_time_string: String,
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
    pub name: String,
    pub streak_type: String,
    pub amount: i32,
    pub best: i32,
    pub latest_time_string: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRecord {
    pub name: String,
    pub period_data: Vec<UserPeriodRecord>,
    pub streaks: Vec<StreakRecord>,
    pub leaderboard_places: Vec<(String, u32)>,
}

impl UserRecord {
    pub fn newcommer(name: String) -> Self {
        Self {
            name,
            period_data: vec![],
            streaks: vec![],
            leaderboard_places: vec![],
        }
    }
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct RepoLeaderboardRecord {
    pub organization: String,
    pub name: String,
    pub total_prs: i64,
    pub total_score: i64,
    pub top_contributor: GithubHandle,
    pub stars: i32,
    pub open_issues: i32,
    pub primary_language: Option<String>,
    pub forks: i32,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct UserContributionRecord {
    pub name: String,
    pub organization: String,
    pub repo: String,
    pub number: i32,
    pub score: Option<i32>,
    pub executed: bool,
    pub created_at: chrono::NaiveDateTime,
    pub merged_at: Option<chrono::NaiveDateTime>,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct RepoRecord {
    pub organization: String,
    pub repo: String,
    pub repo_id: i32,
}
