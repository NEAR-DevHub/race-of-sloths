use svg::node::element::{Rectangle, Text};
use svg::Document;

pub mod db;

pub fn generate_svg(contributor_name: &str, streak_count: u32, total_points: u32) -> String {
    let document = Document::new()
        .set("viewBox", (0, 0, 200, 100))
        .add(
            Rectangle::new()
                .set("x", 0)
                .set("y", 0)
                .set("width", 200)
                .set("height", 100)
                .set("fill", "white")
                .set("stroke", "black")
                .set("stroke-width", 2),
        )
        .add(
            Text::new(format!("Name: {}", contributor_name))
                .set("x", 10)
                .set("y", 30)
                .set("font-family", "Arial")
                .set("font-size", 20)
                .set("fill", "black"),
        )
        .add(
            Text::new(format!("Streak: {}", streak_count))
                .set("x", 10)
                .set("y", 60)
                .set("font-family", "Arial")
                .set("font-size", 20)
                .set("fill", "black"),
        )
        .add(
            Text::new(format!("Points: {}", total_points))
                .set("x", 10)
                .set("y", 90)
                .set("font-family", "Arial")
                .set("font-size", 20)
                .set("fill", "black"),
        );

    document.to_string()
}
