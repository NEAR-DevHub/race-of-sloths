use std::sync::Arc;

use base64::Engine;
use http_body_util::BodyExt;
use race_of_sloths_server::{
    db::{types::UserCachedMetadata, DB},
    github_pull::GithubClient,
    svg::{generate_svg_bot_badge, generate_svg_meta_badge, generate_svg_share_badge},
};
use rocket::{
    http::{ContentType, Header, Status},
    response::{self, Responder},
    serde::json::Json,
    Request, Response, State,
};
use shared::TimePeriod;

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

/// Fetches the user metadata lazily, either from the cache or from the web.
async fn fetch_user_metadata_lazily(
    db: &DB,
    client: &State<Arc<GithubClient>>,
    username: &str,
) -> anyhow::Result<UserCachedMetadata> {
    // Check if the metadata is already cached in the database
    if let Some(cached_metadata) = db.get_user_cached_metadata(username).await? {
        if chrono::Utc::now().naive_utc() - cached_metadata.load_time < chrono::Duration::days(1) {
            return Ok(cached_metadata);
        }
    }

    let user = client.get_user(username).await?;
    let user_image_url = user.avatar_url;

    let res = client.octocrab._get(user_image_url.as_str()).await?;

    let image = res.into_body().collect().await?.to_bytes();

    let image_base64 = base64::engine::general_purpose::STANDARD.encode(image);

    db.upsert_user_cached_metadata(username, &image_base64)
        .await?;

    Ok(UserCachedMetadata {
        image_base64,
        load_time: chrono::Utc::now().naive_utc(),
    })
}

#[utoipa::path(context_path = "/api/users", responses(
    (status = 200, description = "Get dynamically generated user image", content_type = "image/svg+xml")
))]
#[get("/<username>/badge?<type>")]
pub async fn get_badge<'a>(
    username: &str,
    db: &State<DB>,
    font: &State<Arc<usvg::fontdb::Database>>,
    github_client: &State<Arc<GithubClient>>,
    r#type: Option<String>,
) -> Badge {
    let badge_type = r#type.unwrap_or_else(|| "share".to_string());

    let timestamp = chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default() as u64;
    let user: race_of_sloths_server::db::types::UserRecord = match db
        .get_user(
            username,
            &[
                TimePeriod::AllTime.time_string(timestamp),
                TimePeriod::Month.time_string(timestamp),
            ],
        )
        .await
    {
        Ok(Some(value)) => value,
        Ok(None) => {
            rocket::info!("User {username} not found, returning 404");
            return Badge::with_status(Status::NotFound);
        }
        Err(e) => {
            rocket::error!("Failed to get user {username}: {e}");
            return Badge::with_status(Status::InternalServerError);
        }
    };

    let result = match badge_type.as_str() {
        "bot" => {
            let metadata = match fetch_user_metadata_lazily(db, github_client, username).await {
                Ok(metadata) => metadata,
                Err(e) => {
                    rocket::error!("Failed to fetch user metadata: {e}");
                    return Badge::with_status(Status::InternalServerError);
                }
            };
            generate_svg_bot_badge(user, metadata, font.inner().clone())
        }
        "meta" => {
            let metadata = match fetch_user_metadata_lazily(db, github_client, username).await {
                Ok(metadata) => metadata,
                Err(e) => {
                    rocket::error!("Failed to fetch user metadata: {e}");
                    return Badge::with_status(Status::InternalServerError);
                }
            };
            generate_svg_meta_badge(user, metadata, font.inner().clone())
        }
        "share" => generate_svg_share_badge(user, font.inner().clone()),
        _ => {
            rocket::info!("Unknown badge type {badge_type}, returning 404");
            return Badge::with_status(Status::NotFound);
        }
    };

    match result {
        Ok(value) => Badge::new(value),
        _ => {
            rocket::error!("Failed to generate badge for {username}");
            Badge::with_status(Status::InternalServerError)
        }
    }
}

#[utoipa::path(context_path = "/api/users",
    responses(
        (status = 200, description = "Get user profile info", body = UserProfile)
    )
)]
#[get("/<username>")]
async fn get_user(username: &str, db: &State<DB>) -> Option<Json<UserProfile>> {
    let time = chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default();
    let leaderboards = [
        TimePeriod::AllTime.time_string(time as u64),
        TimePeriod::Month.time_string(time as u64),
    ];

    let user = match db.get_user(username, &leaderboards).await {
        Err(e) => {
            rocket::error!("Failed to get user: {username}: {e}");
            return None;
        }
        Ok(value) => value?,
    };

    Some(Json(user.into()))
}

#[utoipa::path(context_path = "/api/users", responses(
    (status = 200, description = "Get user contributions", body = PaginatedUserContributionResponse)
))]
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
            .expect("Failed to load Inter font");

        rocket.manage(Arc::new(font)).mount(
            "/api/users/",
            rocket::routes![get_user, get_user_contributions, get_badge],
        )
    })
}
