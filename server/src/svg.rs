use std::sync::Arc;

use usvg::{fontdb, Options, Tree, WriteOptions};

use crate::db::types::UserRecord;

pub fn generate_svg_badge(
    user_record: UserRecord,
    leaderboard_place: &str,
    image_base64: &str,
    fontdb: Arc<fontdb::Database>,
) -> anyhow::Result<String> {
    let total_period = user_record
        .period_data
        .into_iter()
        .find(|p| p.period_type == "all-time")
        .unwrap_or_default();
    let week_streak = user_record
        .streaks
        .iter()
        .find(|s| s.streak_id == 0)
        .cloned()
        .unwrap_or_default();
    let month_streak = user_record
        .streaks
        .into_iter()
        .find(|s| s.streak_id == 1)
        .unwrap_or_default();

    let svg_icon = std::fs::read_to_string("./public/badge_template.svg")?;
    let svg_icon = svg_icon.replace("{name}", &user_record.name);
    let svg_icon = svg_icon.replace(
        "{total-contributions}",
        &total_period.prs_opened.to_string(),
    );
    let svg_icon = svg_icon.replace("{total-score}", &total_period.total_score.to_string());
    let svg_icon = svg_icon.replace("{week-streak}", &week_streak.amount.to_string());
    let svg_icon = svg_icon.replace("{month-streak}", &month_streak.amount.to_string());
    let svg_icon = svg_icon.replace("{image}", image_base64);
    let svg_icon = svg_icon.replace("{place}", leaderboard_place);

    let tree = Tree::from_str(
        &svg_icon,
        &Options {
            fontdb,
            ..Default::default()
        },
    )?;
    let write_options = WriteOptions {
        use_single_quote: true,
        preserve_text: false,
        ..Default::default()
    };

    Ok(tree.to_string(&write_options))
}
