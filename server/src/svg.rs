use std::sync::Arc;

use num_format::{Locale, ToFormattedString};
use shared::TimePeriod;
use usvg::{fontdb, Options, Tree, WriteOptions};

use crate::db::types::{UserCachedMetadata, UserRecord};

pub fn generate_svg_bot_badge(
    user_record: UserRecord,
    user_metadata: UserCachedMetadata,
    fontdb: Arc<fontdb::Database>,
) -> anyhow::Result<String> {
    let all_time = TimePeriod::AllTime.time_string(0);
    let total_period = user_record.get_total_period().cloned().unwrap_or_default();
    let streak = user_record
        .streaks
        .iter()
        .max_by(|a, b| a.amount.cmp(&b.amount))
        .cloned()
        .unwrap_or_default();

    let (place_type, place) = user_record
        .leaderboard_places
        .iter()
        .min_by(|(_, a), (_, b)| a.cmp(b))
        .map(|(a, place)| {
            (
                if a == &all_time {
                    "Global".to_string()
                } else {
                    "Monthly".to_string()
                },
                place.to_string(),
            )
        })
        .unwrap_or_else(|| ("Global".to_string(), "N/A".to_string()));

    let svg_icon = std::fs::read_to_string("./public/badge_bot_template.svg")?;
    let svg_icon = svg_icon.replace(
        "{name}",
        &user_record
            .name
            .unwrap_or_else(|| format!("@{}", user_record.login)),
    );
    let svg_icon = svg_icon.replace(
        "{total-contributions}",
        &total_period.prs_opened.to_string(),
    );
    let svg_icon = svg_icon.replace("{rank}", rank_string(user_record.lifetime_percent).unwrap());
    let svg_icon = svg_icon.replace(
        "{total-rating}",
        &total_period.total_rating.to_formatted_string(&Locale::en),
    );
    let svg_icon = svg_icon.replace("{streak}", &streak.amount.to_string());
    let svg_icon = svg_icon.replace("{streak-type}", &streak.streak_type);
    let svg_icon = svg_icon.replace("{place}", &place);
    let svg_icon = svg_icon.replace("{place-type}", &place_type);
    let svg_icon = svg_icon.replace("{image}", &user_metadata.image_base64);
    let svg_icon = svg_icon.replace("{rank-svg}", &rank_svg(user_record.lifetime_percent));

    postprocess_svg(svg_icon, fontdb)
}

pub fn generate_svg_share_badge(
    user_record: UserRecord,
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
        .find(|e| e.streak_type == "Weekly")
        .cloned()
        .unwrap_or_default()
        .best;
    let month_streak = user_record
        .streaks
        .iter()
        .find(|e| e.streak_type == "Monthly")
        .cloned()
        .unwrap_or_default()
        .best;

    let (place_type, place) = user_record
        .leaderboard_places
        .iter()
        .min_by(|(_, a), (_, b)| a.cmp(b))
        .map(|(a, place)| {
            (
                if a == &all_time {
                    "Global".to_string()
                } else {
                    "Monthly".to_string()
                },
                place.to_string(),
            )
        })
        .unwrap_or_else(|| ("Global".to_string(), "N/A".to_string()));

    let svg_icon = std::fs::read_to_string("./public/badge_share_template.svg")?;

    let svg_icon = svg_icon.replace(
        "{total-contributions}",
        &total_period.prs_opened.to_string(),
    );
    let svg_icon = svg_icon.replace("{rank}", rank_string(user_record.lifetime_percent).unwrap());
    let svg_icon = svg_icon.replace("{rank-svg}", &rank_svg(user_record.lifetime_percent));

    let svg_icon = svg_icon.replace(
        "{total-rating}",
        &total_period.total_rating.to_formatted_string(&Locale::en),
    );
    let svg_icon = svg_icon.replace("{max-week-streak}", &week_streak.to_string());
    let svg_icon = svg_icon.replace("{max-month-streak}", &month_streak.to_string());
    let svg_icon = svg_icon.replace("{place}", &place);
    let svg_icon = svg_icon.replace("{place-type}", &place_type);

    postprocess_svg(svg_icon, fontdb)
}

pub fn generate_svg_meta_badge(
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
        .find(|e| e.streak_type == "Weekly")
        .cloned()
        .unwrap_or_default()
        .best;
    let month_streak = user_record
        .streaks
        .iter()
        .find(|e| e.streak_type == "Monthly")
        .cloned()
        .unwrap_or_default()
        .best;

    let (place_type, place) = user_record
        .leaderboard_places
        .iter()
        .min_by(|(_, a), (_, b)| a.cmp(b))
        .map(|(a, place)| {
            (
                if a == &all_time {
                    "Global".to_string()
                } else {
                    "Monthly".to_string()
                },
                place.to_string(),
            )
        })
        .unwrap_or_else(|| ("Global".to_string(), "N/A".to_string()));

    let svg_icon = std::fs::read_to_string("./public/badge_meta_template.svg")?;
    let github_handle = format!("@{}", user_record.login);

    let svg_icon = svg_icon.replace(
        "{name}",
        &user_record.name.unwrap_or_else(|| github_handle.clone()),
    );
    let svg_icon = svg_icon.replace("{github-handle}", &github_handle);

    let svg_icon = svg_icon.replace(
        "{total-contributions}",
        &total_period.prs_opened.to_string(),
    );
    let svg_icon = svg_icon.replace("{rank}", rank_string(user_record.lifetime_percent).unwrap());
    let svg_icon = svg_icon.replace("{rank-svg}", &rank_svg(user_record.lifetime_percent));

    let svg_icon = svg_icon.replace(
        "{total-rating}",
        &total_period.total_rating.to_formatted_string(&Locale::en),
    );
    let svg_icon = svg_icon.replace("{max-week-streak}", &week_streak.to_string());
    let svg_icon = svg_icon.replace("{max-month-streak}", &month_streak.to_string());
    let svg_icon = svg_icon.replace("{place}", &place);
    let svg_icon = svg_icon.replace("{image}", &user_metadata.image_base64);
    let svg_icon = svg_icon.replace("{place-type}", &place_type);

    postprocess_svg(svg_icon, fontdb)
}

fn postprocess_svg(svg: String, fontdb: Arc<fontdb::Database>) -> anyhow::Result<String> {
    let tree = Tree::from_str(
        &svg,
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

fn rank_string(lifetime: i32) -> Option<&'static str> {
    Some(match lifetime {
        a if a >= 25 => "Rust",
        a if a >= 20 => "Platinum",
        a if a >= 15 => "Gold",
        a if a >= 10 => "Silver",
        a if a >= 5 => "Bronze",
        _ => "Unranked",
    })
}

fn rank_svg(lifetime: i32) -> String {
    let name = match lifetime {
        a if a >= 25 => "rust.svg",
        a if a >= 20 => "platinum.svg",
        a if a >= 15 => "gold.svg",
        a if a >= 10 => "silver.svg",
        a if a >= 5 => "bronze.svg",
        _ => "unranked.svg",
    };
    std::fs::read_to_string(format!("./public/{}", name)).unwrap_or_default()
}
