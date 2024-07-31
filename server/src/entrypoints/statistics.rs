use std::sync::Arc;

use race_of_sloths_server::db::DB;
use rocket::{serde::json::Json, State};
use shared::telegram::TelegramSubscriber;

use super::types::Statistics;

#[utoipa::path(context_path = "/info", responses(
    (status = 200, description = "Get application statistics", body = Statistics)
))]
#[get("/")]
async fn get_statistics(
    telegram: &State<Arc<TelegramSubscriber>>,
    db: &State<DB>,
) -> Option<Json<Statistics>> {
    let statistics = db.statistics().await;
    let Ok(statistics) = statistics else {
        race_of_sloths_server::error(
            telegram,
            &format!("Failed to fetch statistscs: {}", statistics.err().unwrap()),
        );
        return None;
    };
    Some(Json(statistics.into()))
}

pub fn stage() -> rocket::fairing::AdHoc {
    rocket::fairing::AdHoc::on_ignite("Installing entrypoints", |rocket| async {
        rocket.mount("/info", rocket::routes![get_statistics,])
    })
}
