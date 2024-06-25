use race_of_sloths_server::db::DB;
use rocket::{serde::json::Json, State};
use shared::TimePeriod;

use super::types::{LeaderboardResponse, PaginatedResponse, RepoResponse};

#[utoipa::path(context_path = "/leaderboard", responses(
    (status = 200, description = "Get user leaderboard", body = PaginatedLeaderboardResponse)
))]
#[get("/users/<period>?<page>&<limit>&<streak_id>")]
async fn get_leaderboard(
    db: &State<DB>,
    period: Option<String>,
    page: Option<u64>,
    limit: Option<u64>,
    streak_id: Option<i32>,
) -> Option<Json<PaginatedResponse<LeaderboardResponse>>> {
    let period = period.unwrap_or(TimePeriod::AllTime.time_string(0));
    let streak_id = streak_id.unwrap_or(0);
    let page = page.unwrap_or(0);
    let limit = limit.unwrap_or(50);
    let (records, total) = match db
        .get_leaderboard(&period, streak_id, page as i64, limit as i64)
        .await
    {
        Err(e) => {
            rocket::error!("Failed to get leaderboard: {period}: {e}");
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
    (status = 200, description = "Get repo leaderboard", body = PaginatedRepoResponse)
))]
#[get("/repos?<page>&<limit>")]
async fn get_repos(
    page: Option<u64>,
    limit: Option<u64>,
    db: &State<DB>,
) -> Option<Json<PaginatedResponse<RepoResponse>>> {
    let page = page.unwrap_or(0);
    let limit = limit.unwrap_or(50);
    let (repos, total) = match db.get_repo_leaderboard(page as i64, limit as i64).await {
        Err(e) => {
            rocket::error!("Failed to get repos leaderboard: {e}");
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

pub fn stage() -> rocket::fairing::AdHoc {
    rocket::fairing::AdHoc::on_ignite("Installing entrypoints", |rocket| async {
        rocket.mount("/leaderboard", rocket::routes![get_repos, get_leaderboard,])
    })
}
