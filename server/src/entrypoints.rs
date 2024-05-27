use std::ops::Add;

use base64::Engine;
use race_of_sloths_server::{
    db::{
        types::{LeaderboardRecord, UserRecord},
        DB,
    },
    svg::generate_badge,
};
use rocket::{
    fairing::AdHoc,
    http::{ContentType, Header, Status},
    response::{self, Responder},
    serde::json::Json,
    Request, Response,
};

pub struct Badge {
    svg: Option<String>,
    status: Status,
}

impl Badge {
    pub fn new(svg: String) -> Self {
        Self {
            svg: Some(svg),
            status: Status::Ok,
        }
    }

    pub fn with_status(status: Status) -> Self {
        Self { status, svg: None }
    }
}

impl<'r> Responder<'r, 'static> for Badge {
    fn respond_to(self, _req: &'r Request<'_>) -> response::Result<'static> {
        let expiration = chrono::Utc::now().add(chrono::Duration::minutes(1));

        match self.svg {
            Some(svg) => Response::build()
                .header(Header::new("Cache-Control", "no-cache"))
                .header(Header::new("Pragma", "no-cache"))
                .header(Header::new("Expires", expiration.to_rfc2822()))
                .header(ContentType::SVG)
                .sized_body(svg.len(), std::io::Cursor::new(svg))
                .ok(),
            None => Err(self.status),
        }
    }
}

#[get("/badges/<username>")]
async fn get_svg<'a>(username: &str, db: &DB) -> Badge {
    let user = match db.get_user(username).await {
        Ok(Some(value)) => value,
        _ => return Badge::with_status(Status::NotFound),
    };
    let place = match db.get_leaderboard_place("all-time", &user.name).await {
        Ok(Some(value)) => value,
        _ => return Badge::with_status(Status::NotFound),
    };

    let request = match reqwest::get(format!("https://github.com/{}.png", user.name)).await {
        Ok(value) => value.bytes().await,
        Err(e) => {
            rocket::error!("Failed to fetch image for {username}: {e}");
            return Badge::with_status(Status::InternalServerError);
        }
    };

    let image_base64 = match request {
        Ok(value) => base64::engine::general_purpose::STANDARD.encode(value),
        Err(e) => {
            rocket::error!("Failed to fetch bytes from avatar of {username}: {e}");
            return Badge::with_status(Status::InternalServerError);
        }
    };

    let svg = match generate_badge(user, place as u64, &image_base64) {
        Ok(Some(value)) => value,
        _ => return Badge::with_status(Status::InternalServerError),
    };

    Badge::new(svg)
}

#[get("/users/<username>")]
async fn get_user(username: &str, db: &DB) -> Option<Json<UserRecord>> {
    let user = match db.get_user(username).await {
        Err(e) => {
            rocket::error!("Failed to get user: {username}: {e}");
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
            rocket::error!("Failed to get leaderboard: {period}: {e}");
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
