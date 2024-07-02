use std::sync::Arc;

use race_of_sloths_server::{db::DB, github_pull::GithubClient};
use rocket::State;
use shared::telegram::TelegramSubscriber;

use crate::entrypoints::user::Badge;

#[utoipa::path(context_path = "/", responses(
    (status = 200, description = "Get dynamically generated user image", content_type = "image/svg+xml")
))]
#[get("/<username>?<type>", rank = 2)]
async fn get_badge<'a>(
    telegram: &State<Arc<TelegramSubscriber>>,
    username: &str,
    db: &State<DB>,
    font: &State<Arc<usvg::fontdb::Database>>,
    github_client: &State<Arc<GithubClient>>,
    r#type: Option<String>,
) -> Badge {
    super::user::get_badge(telegram, username, db, font, github_client, r#type).await
}

pub fn stage() -> rocket::fairing::AdHoc {
    rocket::fairing::AdHoc::on_ignite("Installing entrypoints", |rocket| async {
        rocket.mount("/", rocket::routes![get_badge,])
    })
}
