use serde::{Deserialize, Serialize};
use shared::{GithubHandle, TimePeriod, TimePeriodString};
#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct LeaderboardRecord {
    pub login: String,
    pub full_name: Option<String>,
    pub total_score: i32,
    pub total_rating: i32,
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
    pub place: i64,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize, Default)]
pub struct UserPeriodRecord {
    pub period_type: TimePeriodString,
    pub total_score: i32,
    pub executed_prs: i32,
    pub largest_score: i32,
    pub prs_opened: i32,
    pub prs_merged: i32,
    pub total_rating: i64,
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
pub struct User {
    pub login: String,
    pub full_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRecord {
    pub id: i32,
    pub login: String,
    pub first_contribution: chrono::NaiveDateTime,
    pub name: Option<String>,
    pub lifetime_percent: i32,
    pub period_data: Vec<UserPeriodRecord>,
    pub streaks: Vec<StreakRecord>,
    pub leaderboard_places: Vec<(String, u32)>,
}

impl UserRecord {
    pub fn get_total_period(&self) -> Option<&UserPeriodRecord> {
        self.period_data
            .iter()
            .find(|p| p.period_type == "all-time")
    }

    pub fn get_current_month(&self) -> Option<&UserPeriodRecord> {
        let timestamp = chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default();
        let period = TimePeriod::Month.time_string(timestamp as u64);
        self.period_data.iter().find(|p| p.period_type == period)
    }
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct RepoLeaderboardRecord {
    pub organization: String,
    pub organization_full_name: Option<String>,
    pub name: String,
    pub total_prs: Option<i64>,
    pub total_score: Option<i64>,
    pub total_rating: Option<i64>,
    pub contributor_login: Option<GithubHandle>,
    pub contributor_full_name: Option<String>,
    pub stars: Option<i32>,
    pub open_issues: Option<i32>,
    pub primary_language: Option<String>,
    pub forks: Option<i32>,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct UserContributionRecord {
    pub organization_login: String,
    pub organization_full_name: Option<String>,
    pub repo: String,
    pub number: i32,
    pub score: Option<i32>,
    pub rating: i32,
    pub percentage_multiplier: i32,
    pub streak_bonus_rating: i32,
    pub executed: bool,
    pub created_at: chrono::NaiveDateTime,
    pub merged_at: Option<chrono::NaiveDateTime>,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct RepoRecord {
    pub organization: String,
    pub repo: String,
    pub organization_full_name: Option<String>,
    pub repo_id: i32,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct UserCachedMetadata {
    pub image_base64: String,
    pub load_time: chrono::NaiveDateTime,
}
