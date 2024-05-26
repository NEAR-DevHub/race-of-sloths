use race_of_sloths_server::{db::DB, generate_svg};
use rocket::{fairing::AdHoc, http::ContentType, response::content::RawHtml};
use tracing::instrument;

#[get("/badges/<username>")]
#[instrument]
async fn get_svg(username: &str, db: &DB) -> Option<(ContentType, RawHtml<String>)> {
    let user = match db.get_user(username, Some("all-time")).await {
        Err(e) => {
            error!("Failed to get user: {username}: {e}");
            return None;
        }
        Ok(value) => value?,
    };
    let period_data = user.period_data.get(0)?;
    let streak = user
        .streaks
        .iter()
        .max_by(|a, b| a.1.amount.cmp(&b.1.amount))?;
    let svg_content = generate_svg(&user.name, streak.1.amount, period_data.1.total_score);
    Some((ContentType::SVG, RawHtml(svg_content)))
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("Installing entrypoints", |rocket| async {
        rocket.mount("/api", routes![get_svg])
    })
}
