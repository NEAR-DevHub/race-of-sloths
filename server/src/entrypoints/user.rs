use base64::Engine;
use race_of_sloths_server::{
    db::{types::UserRecord, DB},
    svg::generate_svg_badge,
};
use rocket::{
    http::{ContentType, Header, Status},
    response::{self, Responder},
    serde::json::Json,
    Request, Response, State,
};

use super::types::{PaginatedResponse, UserContributionResponse, UserProfile};

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
        let expiration = chrono::Utc::now();
        //.add(chrono::Duration::minutes(1));

        match self.svg {
            Some(png) => Response::build()
                .header(Header::new("Cache-Control", "no-cache"))
                .header(Header::new("Pragma", "no-cache"))
                .header(Header::new("Expires", expiration.to_rfc2822()))
                .header(ContentType::SVG)
                .sized_body(png.len(), std::io::Cursor::new(png))
                .ok(),
            None => Err(self.status),
        }
    }
}

#[get("/<username>/badge")]
async fn get_svg<'a>(
    username: &str,
    db: &State<DB>,
    font: &State<usvg::fontdb::Database>,
) -> Badge {
    let user = match db.get_user(username).await {
        Ok(Some(value)) => value,
        Ok(None) => {
            rocket::info!("User {username} not found, fallback to default");
            UserRecord::newcommer(username.to_string())
        }
        Err(e) => {
            rocket::error!("Failed to get user {username}: {e}");
            return Badge::with_status(Status::InternalServerError);
        }
    };
    let place = match db.get_leaderboard_place("all-time", &user.name).await {
        Ok(Some(value)) => value.to_string(),
        _ => "N/A".to_owned(),
    };

    let request = match reqwest::get(format!("https://github.com/{}.png", user.name)).await {
        Ok(value) => value.bytes().await,
        Err(e) => {
            rocket::error!("Failed to fetch image for {username}: {e}");
            return Badge::with_status(Status::NotFound);
        }
    };

    let image_base64 = match request {
        Ok(value) => base64::engine::general_purpose::STANDARD.encode(value),
        Err(e) => {
            rocket::error!("Failed to fetch bytes from avatar of {username}: {e}");
            return Badge::with_status(Status::InternalServerError);
        }
    };

    match generate_svg_badge(user, &place, &image_base64, font) {
        Ok(value) => Badge::new(value),
        _ => {
            rocket::error!("Failed to generate badge for {username}");
            Badge::with_status(Status::InternalServerError)
        }
    }
}

#[get("/<username>")]
async fn get_user(username: &str, db: &State<DB>) -> Option<Json<UserProfile>> {
    let user = match db.get_user(username).await {
        Err(e) => {
            rocket::error!("Failed to get user: {username}: {e}");
            return None;
        }
        Ok(value) => value?,
    };

    Some(Json(user.into()))
}

#[get("/<username>/contributions?<page>&<limit>")]
async fn get_user_contributions(
    username: &str,
    page: Option<u64>,
    limit: Option<u64>,
    db: &State<DB>,
) -> Option<Json<PaginatedResponse<UserContributionResponse>>> {
    let page = page.unwrap_or(0);
    let limit = limit.unwrap_or(50);
    let (repos, total) = match db
        .get_user_contributions(username, page as i64, limit as i64)
        .await
    {
        Err(e) => {
            rocket::error!("Failed to get user contributions: {username}: {e}");
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
        let mut font = usvg::fontdb::Database::new();
        font.load_font_file("./public/Inter-VariableFont_slnt,wght.ttf")
            .expect("Failed to load font");

        rocket.manage(font).mount(
            "/api/users/",
            rocket::routes![get_user, get_user_contributions, get_svg],
        )
    })
}
