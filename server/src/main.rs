#[macro_use]
extern crate rocket;

use race_of_sloths_server::generate_svg;
use rocket::http::ContentType;
use rocket::response::content::RawHtml;

#[get("/<username>")]
fn get_svg(username: &str) -> (ContentType, RawHtml<String>) {
    // Dummy data for illustration; replace with actual logic
    let streak_count = 5;
    let total_points = 120;

    let svg_content = generate_svg(username, streak_count, total_points);
    (ContentType::SVG, RawHtml(svg_content))
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![get_svg])
}
