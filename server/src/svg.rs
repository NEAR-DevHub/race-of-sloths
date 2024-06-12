use std::sync::Arc;

use shared::TimePeriod;
use usvg::{fontdb, Options, Tree, WriteOptions};

use crate::db::types::{UserCachedMetadata, UserRecord};

pub fn generate_svg_badge(
    user_record: UserRecord,
    user_metadata: UserCachedMetadata,
    fontdb: Arc<fontdb::Database>,
) -> anyhow::Result<String> {
    let all_time = TimePeriod::AllTime.time_string(0);
    let total_period = user_record
        .period_data
        .into_iter()
        .find(|p| p.period_type == all_time)
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
    let all_time_place = user_record
        .leaderboard_places
        .iter()
        .find(|(period, _)| period == &all_time)
        .map(|(_, place)| place.to_string())
        .unwrap_or_else(|| "N/A".to_string());

    let svg_icon = std::fs::read_to_string("./public/badge_template.svg")?;
    let svg_icon = svg_icon.replace("{name}", &user_metadata.full_name);
    let svg_icon = svg_icon.replace(
        "{total-contributions}",
        &total_period.prs_opened.to_string(),
    );
    let svg_icon = svg_icon.replace("{total-score}", &total_period.total_score.to_string());
    let svg_icon = svg_icon.replace("{week-streak}", &week_streak.amount.to_string());
    let svg_icon = svg_icon.replace("{month-streak}", &month_streak.amount.to_string());
    let svg_icon = svg_icon.replace("{image}", &user_metadata.image_base64);
    let svg_icon = svg_icon.replace("{place}", &all_time_place);

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
