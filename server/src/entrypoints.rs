use race_of_sloths_server::{
    db::{
        types::{LeaderboardRecord, UserRecord},
        DB,
    },
    svg::generate_badge,
};
use rocket::{fairing::AdHoc, http::ContentType, response::content::RawHtml, serde::json::Json};
use tracing::instrument;

#[get("/badges/<username>")]
#[instrument]
async fn get_svg(username: &str, db: &DB) -> Option<(ContentType, RawHtml<String>)> {
    let user = match db.get_user(username).await {
        Err(e) => {
            error!("Failed to get user: {username}: {e}");
            return None;
        }
        Ok(value) => value?,
    };
    let place = match db.get_leaderboard_place("all-time", &user.name).await {
        Err(e) => {
            error!("Failed to get leaderboard place: {username}: {e}");
            return None;
        }
        Ok(value) => value?,
    };
    let svg = generate_badge(user, place as u64)?;

    Some((ContentType::SVG, RawHtml(svg)))
}

#[get("/users/<username>")]
async fn get_user(username: &str, db: &DB) -> Option<Json<UserRecord>> {
    let user = match db.get_user(username).await {
        Err(e) => {
            error!("Failed to get user: {username}: {e}");
            return None;
        }
        Ok(value) => value?,
    };
    Some(Json(user))
}

#[get("/leaderboard/<period>?<page>&<limit>")]
async fn get_leaderboard(
    period: &str,
    db: &DB,
    page: Option<u32>,
    limit: Option<u32>,
) -> Option<Json<Vec<LeaderboardRecord>>> {
    let page = page.unwrap_or(0);
    let limit = limit.unwrap_or(50);
    let users = match db.get_leaderboard(period, page as i64, limit as i64).await {
        Err(e) => {
            error!("Failed to get leaderboard: {period}: {e}");
            return None;
        }
        Ok(value) => value,
    };
    Some(Json(users))
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("Installing entrypoints", |rocket| async {
        rocket.mount("/api", routes![get_svg, get_user, get_leaderboard])
    })
}
