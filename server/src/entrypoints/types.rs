use std::collections::HashMap;

use chrono::NaiveDateTime;
use race_of_sloths_server::db::types::{
    LeaderboardRecord, RepoRecord, UserContributionRecord, UserRecord,
};
use serde::{Deserialize, Serialize};
use shared::{GithubHandle, TimePeriod};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct PaginatedResponse<T: Serialize> {
    pub records: Vec<T>,
    pub page: u64,
    pub total_pages: u64,
    pub limit: u64,
    pub total_records: u64,
}

impl<T: Serialize> PaginatedResponse<T> {
    pub fn new(records: Vec<T>, page: u64, limit: u64, total_records: u64) -> Self {
        let extra_page = if total_records % limit == 0 { 0 } else { 1 };
        let total_pages = (total_records / limit) + extra_page;
        Self {
            records,
            page,
            total_pages,
            limit,
            total_records,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GithubMeta {
    name: String,
    image: String,
}

impl GithubMeta {
    pub fn new(name: String) -> Self {
        let image = format!("https://github.com/{}.png", name);
        Self { name, image }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RepoResponse {
    pub name: String,
    pub organization: GithubMeta,
    pub project_language: String,
    pub project_language_color: String,
    pub contributor_of_the_month: GithubHandle,
    pub open_issues: u32,
    pub contributions_with_sloth: u32,
    pub total_score: u32,
}

impl From<RepoRecord> for RepoResponse {
    fn from(record: RepoRecord) -> Self {
        Self {
            name: record.name,
            organization: GithubMeta::new(record.organization),
            // TODO: fix these fields
            project_language: "RUST".to_string(),
            project_language_color: "#000000".to_string(),
            contributor_of_the_month: record.top_contributor,
            open_issues: 0,
            contributions_with_sloth: record.total_prs as u32,
            total_score: record.total_score as u32,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LeaderboardResponse {
    pub user: GithubMeta,
    pub rating: u32,
    pub contributions: u32,
    pub streak: Streak,
    pub merged_prs: u32,
    pub score: u32,
}

impl From<LeaderboardRecord> for LeaderboardResponse {
    fn from(record: LeaderboardRecord) -> Self {
        Self {
            user: GithubMeta::new(record.name),
            // TODO: fix ratings
            rating: 0,
            contributions: record.prs_opened as u32,
            streak: Streak::new(
                record.streak_amount as u32,
                record.streak_best as u32,
                &record.streak_latest_time_string,
            ),
            merged_prs: record.prs_merged as u32,
            score: record.total_score as u32,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Streak {
    current: u32,
    longest: u32,
}

impl Streak {
    pub fn new(current: u32, longest: u32, time_period_string: &str) -> Self {
        if let Some(time_period) = TimePeriod::from_time_period_string(time_period_string) {
            let current_time = chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default();
            let previous_period = time_period
                .previous_period(current_time as u64)
                .unwrap_or_default();
            let current_time_string = time_period.time_string(current_time as u64);
            let previous_period_string = time_period.time_string(previous_period);
            if current_time_string == time_period_string
                || previous_period_string == time_period_string
            {
                return Self { current, longest };
            };
        }

        Self {
            current: 0,
            longest,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserProfile {
    pub user: GithubMeta,
    pub rating: u32,
    pub contributions: u32,
    pub leaderboard_places: HashMap<String, u32>,
    pub streaks: HashMap<String, Streak>,
}

impl From<UserRecord> for UserProfile {
    fn from(record: UserRecord) -> Self {
        let contributions = record
            .period_data
            .iter()
            .find(|x| x.period_type == TimePeriod::AllTime.time_string(0))
            .map(|x| x.prs_opened)
            .unwrap_or(0) as u32;
        Self {
            user: GithubMeta::new(record.name),
            // TODO: fix ratings
            rating: 0,
            contributions,
            streaks: record
                .streaks
                .into_iter()
                .map(|streak| {
                    (
                        // TODO: We should write here a string name
                        streak.streak_id.to_string(),
                        Streak::new(
                            streak.amount as u32,
                            streak.best as u32,
                            &streak.latest_time_string,
                        ),
                    )
                })
                .collect(),
            leaderboard_places: record.leaderboard_places.into_iter().collect(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserContributionResponse {
    pub pull_request_link: String,
    pub repository: String,
    pub organization: GithubMeta,
    pub status: String,
    pub score: Option<i32>,
    pub created_at: NaiveDateTime,
    pub merged_at: Option<NaiveDateTime>,
}

impl From<UserContributionRecord> for UserContributionResponse {
    fn from(record: UserContributionRecord) -> Self {
        let pull_request_link = format!(
            "https://github.com/{}/{}/pull/{}",
            record.organization, record.repo, record.number
        );

        let status = if record.executed {
            "Finished"
        } else if record.score.is_none() {
            "Waiting for score"
        } else if record.merged_at.is_none() {
            "Waiting for merge"
        } else {
            "Waiting for execution"
        };

        Self {
            pull_request_link,
            repository: record.repo,
            organization: GithubMeta::new(record.organization),
            status: status.to_string(),
            score: record.score,
            created_at: record.created_at,
            merged_at: record.merged_at,
        }
    }
}
