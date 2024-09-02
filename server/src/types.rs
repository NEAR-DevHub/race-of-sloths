use std::collections::HashMap;

use crate::db::types::{
    LeaderboardRecord, RepoLeaderboardRecord, UserContributionRecord, UserRecord,
};
use chrono::NaiveDateTime;
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
    pub total_rating: u32,
}

impl From<RepoLeaderboardRecord> for RepoResponse {
    fn from(record: RepoLeaderboardRecord) -> Self {
        Self {
            name: record.name,
            organization: GithubMeta::new(record.organization, record.organization_full_name),
            repo_language: record.primary_language,
            stars: record.stars.unwrap_or_default() as u32,
            forks: record.forks.unwrap_or_default() as u32,
            open_issues: record.open_issues.unwrap_or_default() as u32,
            contributor_of_the_month: record
                .contributor_login
                .map(|login| GithubMeta::new(login, record.contributor_full_name)),
            contributions_with_sloth: record.total_prs.unwrap_or_default() as u32,
            total_score: record.total_score.unwrap_or_default() as u32,
            total_rating: record.total_rating.unwrap_or_default() as u32,
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
    pub place: u32,
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
            place: record.place as u32,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default, ToSchema)]
pub struct Streak {
    pub name: String,
    pub streak_type: String,
    pub current: u32,
    pub longest: u32,
    pub achived: bool,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default, ToSchema)]
pub struct Period {
    pub contributions: u32,
    pub rating: u32,
}

impl Period {
    pub fn new(contributions: u32, rating: u32) -> Self {
        Self {
            contributions,
            rating,
        }
    }
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
            let current_time_string = time_period.time_string(current_time as u64);

            let previous_period = time_period
                .previous_period(current_time as u64)
                .unwrap_or_default();
            let previous_period_string = time_period.time_string(previous_period);

            let mut result = Self {
                name,
                streak_type,
                current,
                longest,
                achived: time_period_string == current_time_string,
                start_time: time_period
                    .start_period(current_time as u64)
                    .unwrap_or_default(),
                end_time: time_period
                    .end_period(current_time as u64)
                    .unwrap_or_default(),
            };

            if current_time_string != time_period_string
                && previous_period_string != time_period_string
            {
                result.current = 0;
            }

            result
        } else {
            // TODO: probably we need to return error here for user request
            Self::default()
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct UserProfile {
    pub user_id: u32,
    pub user: GithubMeta,
    pub monthly: Period,
    pub global: Period,
    pub lifetime_bonus: u32,
    pub leaderboard_places: HashMap<String, u32>,
    pub streaks: Vec<Streak>,
    pub first_contribution: NaiveDateTime,
}

impl From<UserRecord> for UserProfile {
    fn from(record: UserRecord) -> Self {
        let total_period = record.get_total_period().cloned().unwrap_or_default();
        let monthly_period = record.get_current_month().cloned().unwrap_or_default();

        Self {
            user_id: record.id as u32,
            user: GithubMeta::new(record.login, record.name),
            monthly: Period::new(
                monthly_period.prs_opened as u32,
                monthly_period.total_rating as u32,
            ),
            global: Period::new(
                total_period.prs_opened as u32,
                total_period.total_rating as u32,
            ),
            lifetime_bonus: record.lifetime_percent as u32,
            streaks: record
                .streaks
                .into_iter()
                .map(|streak| {
                    Streak::new(
                        streak.name,
                        streak.amount as u32,
                        streak.best as u32,
                        streak.latest_time_string,
                        streak.streak_type,
                    )
                })
                .collect(),
            leaderboard_places: record.leaderboard_places.into_iter().collect(),
            first_contribution: record.first_contribution,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct UserContributionResponse {
    pub pull_request_link: String,
    pub repository: String,
    pub organization: GithubMeta,
    pub executed: bool,
    pub pr_number: i32,
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
            created_at: record.included_at,
            merged_at: record.merged_at,
            total_rating: record.rating,
            percentage_multiplier: record.percentage_multiplier,
            streak_bonus_rating: record.streak_bonus_rating,
            pr_number: record.number,
            executed: record.executed,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct Statistics {
    pub number_of_sloths: u32,
    pub number_of_repos: u32,
    pub number_of_orgs: u32,
    pub number_of_contributions: u32,
    pub total_rating: u32,
    pub highest_sloth_rating: (GithubMeta, u32),
    pub shortest_merge_time: (String, String),
    pub number_of_famed_sloths: u32,
    pub hall_of_fame: Vec<GithubMeta>,
}

impl From<crate::db::types::Statistics> for Statistics {
    fn from(value: crate::db::types::Statistics) -> Self {
        let duration = value
            .fastest_merged
            .map(|x| x - value.fastest_included.unwrap_or_default())
            .unwrap_or_default();
        let hall_of_fame = if let Some(hall_of_fame) = value.hall_of_fame {
            hall_of_fame
                .split(',')
                .map(|x| GithubMeta::new(x.to_string(), None))
                .collect()
        } else {
            vec![]
        };

        Self {
            number_of_sloths: value.number_of_sloths.unwrap_or_default() as u32,
            number_of_repos: value.number_of_repos.unwrap_or_default() as u32,
            number_of_orgs: value.number_of_orgs.unwrap_or_default() as u32,
            number_of_contributions: value.number_of_contributions.unwrap_or_default() as u32,
            total_rating: value.total_rating.unwrap_or_default() as u32,
            highest_sloth_rating: (
                GithubMeta::new(value.highest_sloth_login, value.highest_sloth_full_name),
                value.highest_sloth_rating.unwrap_or_default() as u32,
            ),
            shortest_merge_time: (
                format!(
                    "https://github.com/{}/{}/pull/{}",
                    value.fastest_org_login,
                    value.fastest_repo_name,
                    value.fastest_pr_number.unwrap_or_default()
                ),
                duration.to_string(),
            ),
            number_of_famed_sloths: hall_of_fame.len() as u32,
            hall_of_fame,
        }
    }
}
