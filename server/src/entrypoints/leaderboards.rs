use std::sync::Arc;

use race_of_sloths_server::{db::DB, types::HallOfFameResponse};
use rocket::{serde::json::Json, State};
use shared::{telegram, TimePeriod};

use super::types::{LeaderboardResponse, PaginatedResponse, RepoResponse};

#[utoipa::path(context_path = "/leaderboard", responses(
    (status = 200, description = "Get user leaderboard", body = PaginatedLeaderboardResponse)
))]
#[get("/users/<period>?<page>&<limit>")]
async fn get_leaderboard(
    db: &State<DB>,
    telegram: &State<Arc<telegram::TelegramSubscriber>>,
    period: Option<String>,
    page: Option<u64>,
    limit: Option<u64>,
) -> Option<Json<PaginatedResponse<LeaderboardResponse>>> {
    let period = period.unwrap_or(TimePeriod::AllTime.time_string(0));
    let page = page.unwrap_or(0);
    let limit = limit.unwrap_or(50);
    let (records, total) = match db.get_leaderboard(&period, page as i64, limit as i64).await {
        Err(e) => {
            race_of_sloths_server::error(
                telegram,
                &format!("Failed to get leaderboard: {period}: {e}"),
            );
            return None;
        }
        Ok(value) => value,
    };
    Some(Json(PaginatedResponse::new(
        records.into_iter().map(Into::into).collect(),
        page + 1,
        limit,
        total as u64,
    )))
}
#[utoipa::path(context_path = "/leaderboard", responses(
    (status = 200, description = "Get hall of fame records", body = PaginatedHallOfFameResponse)
))]
#[get("/users/hall_of_fame?<period>&<page>&<limit>")]
async fn get_hall_of_fame(
    db: &State<DB>,
    telegram: &State<Arc<telegram::TelegramSubscriber>>,
    period: Option<String>,
    page: Option<u64>,
    limit: Option<u64>,
) -> Option<Json<PaginatedResponse<HallOfFameResponse>>> {
    let page = page.unwrap_or(0);
    let limit = limit.unwrap_or(50);
    let period = period.unwrap_or(TimePeriod::AllTime.time_string(0));
    let (records, total) = match db
        .get_hall_of_fame(&period, page as i64, limit as i64)
        .await
    {
        Err(e) => {
            race_of_sloths_server::error(
                telegram,
                &format!("Failed to get leaderboard: {period}: {e}"),
            );
            return None;
        }
        Ok(value) => value,
    };
    Some(Json(PaginatedResponse::new(
        records.into_iter().map(Into::into).collect(),
        page + 1,
        limit,
        total,
    )))
}

#[utoipa::path(context_path = "/repos", responses(
    (status = 200, description = "Get repo leaderboard", body = PaginatedRepoResponse)
))]
#[get("/repos?<page>&<limit>")]
async fn get_repos(
    telegram: &State<Arc<telegram::TelegramSubscriber>>,
    page: Option<u64>,
    limit: Option<u64>,
    db: &State<DB>,
) -> Option<Json<PaginatedResponse<RepoResponse>>> {
    let page = page.unwrap_or(0);
    let limit = limit.unwrap_or(50);
    let (repos, total) = match db.get_repo_leaderboard(page as i64, limit as i64).await {
        Err(e) => {
            race_of_sloths_server::error(
                telegram,
                &format!("Failed to get repos leaderboard: {e}"),
            );
            return None;
        }
        Ok(value) => value,
    };
    Some(Json(PaginatedResponse::new(
        repos.into_iter().map(Into::into).collect(),
        page + 1,
        limit,
        total,
    )))
}

#[utoipa::path(context_path = "/potential_repos", responses(
    (status = 200, description = "Get paused repos", body = PaginatedRepoResponse)
))]
#[get("/potential_repos")]
async fn get_potential_repos(
    telegram: &State<Arc<telegram::TelegramSubscriber>>,
    db: &State<DB>,
) -> Option<Json<Vec<String>>> {
    let repos = match db.get_potential_repos().await {
        Err(e) => {
            race_of_sloths_server::error(telegram, &format!("Failed to get potential repos: {e}"));
            return None;
        }
        Ok(value) => value,
    };
    Some(Json::from(repos))
}

pub fn stage() -> rocket::fairing::AdHoc {
    rocket::fairing::AdHoc::on_ignite("Installing entrypoints", |rocket| async {
        rocket.mount(
            "/leaderboard",
            rocket::routes![
                get_repos,
                get_leaderboard,
                get_potential_repos,
                get_hall_of_fame
            ],
        )
    })
}
