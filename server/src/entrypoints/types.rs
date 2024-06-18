use std::collections::HashMap;

use chrono::NaiveDateTime;
use race_of_sloths_server::db::types::{
    LeaderboardRecord, RepoLeaderboardRecord, UserContributionRecord, UserRecord,
};
use serde::{Deserialize, Serialize};
use shared::TimePeriod;
use utoipa::ToSchema;

#[derive(Clone, Debug, Serialize, Deserialize, Default, ToSchema)]
#[aliases(PaginatedLeaderboardResponse = PaginatedResponse<LeaderboardResponse>, PaginatedRepoResponse = PaginatedResponse<RepoResponse>, PaginatedUserContributionResponse = PaginatedResponse<UserContributionResponse>)]
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

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct GithubMeta {
    login: String,
    name: Option<String>,
    image: String,
}

impl GithubMeta {
    pub fn new(login: String, name: Option<String>) -> Self {
        let image = format!("https://github.com/{}.png", login);
        Self { login, name, image }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct RepoResponse {
    pub name: String,
    pub organization: GithubMeta,
    pub repo_language: Option<String>,
    pub stars: u32,
    pub forks: u32,
    pub open_issues: u32,
    pub contributor_of_the_month: Option<GithubMeta>,
    pub contributions_with_sloth: u32,
    pub total_score: u32,
}

impl From<RepoLeaderboardRecord> for RepoResponse {
    fn from(record: RepoLeaderboardRecord) -> Self {
        Self {
            name: record.name,
            organization: GithubMeta::new(record.organization, record.organization_full_name),
            repo_language: record.primary_language,
            stars: record.stars as u32,
            forks: record.forks as u32,
            open_issues: record.open_issues as u32,
            contributor_of_the_month: record
                .contributor_login
                .map(|login| GithubMeta::new(login, record.contributor_full_name)),
            contributions_with_sloth: record.total_prs as u32,
            total_score: record.total_score as u32,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
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
            user: GithubMeta::new(record.login, record.full_name),
            rating: record.total_rating as u32,
            contributions: record.prs_opened as u32,
            streak: Streak::new(
                record.streak_name,
                record.streak_amount as u32,
                record.streak_best as u32,
                record.streak_latest_time_string,
                record.streak_type,
            ),
            merged_prs: record.prs_merged as u32,
            score: record.total_score as u32,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default, ToSchema)]
pub struct Streak {
    name: String,
    streak_type: String,
    current: u32,
    longest: u32,
}

impl Streak {
    pub fn new(
        name: String,
        current: u32,
        longest: u32,
        time_period_string: String,
        streak_type: String,
    ) -> Self {
        if let Some(time_period) = TimePeriod::from_streak_type(&streak_type) {
            let current_time = chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default();
            let previous_period = time_period
                .previous_period(current_time as u64)
                .unwrap_or_default();
            let current_time_string = time_period.time_string(current_time as u64);
            let previous_period_string = time_period.time_string(previous_period);
            if current_time_string == time_period_string
                || previous_period_string == time_period_string
            {
                return Self {
                    name,
                    streak_type,
                    current,
                    longest,
                };
            };
        }

        Self {
            name,
            streak_type,
            current: 0,
            longest,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct UserProfile {
    pub user: GithubMeta,
    pub rating: u32,
    pub contributions: u32,
    pub leaderboard_places: HashMap<String, u32>,
    pub streaks: HashMap<String, Streak>,
}

impl From<UserRecord> for UserProfile {
    fn from(record: UserRecord) -> Self {
        let (contributions, rating) = record
            .period_data
            .iter()
            .find(|x| x.period_type == TimePeriod::AllTime.time_string(0))
            .map(|x| (x.prs_opened, x.total_rating))
            .unwrap_or_default();
        Self {
            user: GithubMeta::new(record.login, record.name),
            rating: rating as u32,
            contributions: contributions as u32,
            streaks: record
                .streaks
                .into_iter()
                .map(|streak| {
                    (
                        streak.name.clone(),
                        Streak::new(
                            streak.name,
                            streak.amount as u32,
                            streak.best as u32,
                            streak.latest_time_string,
                            streak.streak_type,
                        ),
                    )
                })
                .collect(),
            leaderboard_places: record.leaderboard_places.into_iter().collect(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct UserContributionResponse {
    pub pull_request_link: String,
    pub repository: String,
    pub organization: GithubMeta,
    pub status: String,
    pub score: Option<i32>,
    pub total_rating: i32,
    pub percentage_multiplier: i32,
    pub streak_bonus_rating: i32,
    pub created_at: NaiveDateTime,
    pub merged_at: Option<NaiveDateTime>,
}

impl From<UserContributionRecord> for UserContributionResponse {
    fn from(record: UserContributionRecord) -> Self {
        let pull_request_link = format!(
            "https://github.com/{}/{}/pull/{}",
            record.organization_login, record.repo, record.number
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
            organization: GithubMeta::new(record.organization_login, record.organization_full_name),
            status: status.to_string(),
            score: record.score,
            created_at: record.created_at,
            merged_at: record.merged_at,
            total_rating: record.rating,
            percentage_multiplier: record.percentage_multiplier,
            streak_bonus_rating: record.streak_bonus_rating,
        }
    }
}
