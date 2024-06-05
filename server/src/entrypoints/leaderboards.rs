use race_of_sloths_server::db::{
    types::{LeaderboardRecord, RepoRecord},
    DB,
};
use rocket::{serde::json::Json, State};

#[get("/users/<period>?<page>&<limit>")]
async fn get_leaderboard(
    period: &str,
    db: &State<DB>,
    page: Option<u32>,
    limit: Option<u32>,
) -> Option<Json<Vec<LeaderboardRecord>>> {
    let page = page.unwrap_or(0);
    let limit = limit.unwrap_or(50);
    let users = match db.get_leaderboard(period, page as i64, limit as i64).await {
        Err(e) => {
            rocket::error!("Failed to get leaderboard: {period}: {e}");
            return None;
        }
        Ok(value) => value,
    };
    Some(Json(users))
}

#[get("/repos?<page>&<limit>")]
async fn get_repos(
    page: Option<u32>,
    limit: Option<u32>,
    db: &State<DB>,
) -> Option<Json<Vec<RepoRecord>>> {
    let page = page.unwrap_or(0);
    let limit = limit.unwrap_or(50);
    let repos = match db.get_repo_leaderboard(page as i64, limit as i64).await {
        Err(e) => {
            rocket::error!("Failed to get repos leaderboard: {e}");
            return None;
        }
        Ok(value) => value,
    };
    Some(Json(repos))
}

#[get("/contributors-of-the-month?<repo>&<org>")]
async fn get_contributor_of_the_month(
    repo: &str,
    org: &str,
    db: &State<DB>,
) -> Option<Json<Vec<(String, i64)>>> {
    let user = match db.get_contributors_of_the_month(repo, org).await {
        Err(e) => {
            rocket::error!("Failed to get contributor of the month: {e}");
            return None;
        }
        Ok(value) => value,
    };
    Some(Json(user))
}

pub fn stage() -> rocket::fairing::AdHoc {
    rocket::fairing::AdHoc::on_ignite("Installing entrypoints", |rocket| async {
        rocket.mount(
            "/api/leaderboard",
            rocket::routes![get_repos, get_leaderboard, get_contributor_of_the_month],
        )
    })
}
