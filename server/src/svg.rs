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
            .clone()
            .unwrap_or_else(|| format!("@{}", user_record.login)),
    );
    let svg_icon = svg_icon.replace(
        "{total-contributions}",
        &total_period.prs_opened.to_string(),
    );
    let svg_icon = process_rank(svg_icon, &user_record);

    let svg_icon = svg_icon.replace(
        "{total-rating}",
        &total_period.total_rating.to_formatted_string(&Locale::en),
    );
    let svg_icon = svg_icon.replace("{streak}", &streak.amount.to_string());
    let svg_icon = svg_icon.replace("{streak-type}", &streak.streak_type);
    let svg_icon = svg_icon.replace("{place}", &place);
    let svg_icon = svg_icon.replace("{place-type}", &place_type);
    let svg_icon = svg_icon.replace("{image}", &user_metadata.image_base64);

    postprocess_svg(svg_icon, fontdb)
}

pub fn generate_svg_share_badge(
    user_record: UserRecord,
    fontdb: Arc<fontdb::Database>,
) -> anyhow::Result<String> {
    let all_time = TimePeriod::AllTime.time_string(0);
    let total_period = user_record
        .period_data
        .iter()
        .find(|p| p.period_type == all_time)
        .cloned()
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
    let svg_icon = process_rank(svg_icon, &user_record);

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
        .iter()
        .find(|p| p.period_type == all_time)
        .cloned()
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

    let svg_icon = process_rank(svg_icon, &user_record)
        .replace(
            "{total-rating}",
            &total_period.total_rating.to_formatted_string(&Locale::en),
        )
        .replace("{max-week-streak}", &week_streak.to_string())
        .replace("{max-month-streak}", &month_streak.to_string())
        .replace("{place}", &place)
        .replace("{image}", &user_metadata.image_base64)
        .replace("{place-type}", &place_type)
        .replace(
            "{total-contributions}",
            &total_period.prs_opened.to_string(),
        )
        .replace("{github-handle}", &github_handle)
        .replace(
            "{name}",
            &user_record
                .name
                .clone()
                .unwrap_or_else(|| github_handle.clone()),
        );
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

fn process_rank(svg_icon: String, user_record: &UserRecord) -> String {
    let (rank, rank_svg, title) = rank_data(user_record);
    svg_icon
        .replace("{rank}", &rank)
        .replace("{rank-svg}", &rank_svg)
        .replace("{rank-title}", &title)
}

fn rank_data(user_record: &UserRecord) -> (String, String, String) {
    let (rank, rank_svg_file, title) = match user_record.lifetime_percent {
        a if a >= 25 => ("Rust".to_string(), "rust.svg", "Rank"),
        a if a >= 20 => ("Platinum".to_string(), "platinum.svg", "Rank"),
        a if a >= 15 => ("Gold".to_string(), "gold.svg", "Rank"),
        a if a >= 10 => ("Silver".to_string(), "silver.svg", "Rank"),
        a if a >= 5 => ("Bronze".to_string(), "bronze.svg", "Rank"),
        _ => {
            let age = user_record.first_contribution;
            let current_time = chrono::Utc::now().naive_utc();
            let days = (current_time - age).num_days() + 1;
            let day_suffix = if days > 1 { "days" } else { "day" };
            (format!("{days} {day_suffix}"), "unranked.svg", "Sloth age")
        }
    };
    (
        rank,
        std::fs::read_to_string(format!("./public/{}", rank_svg_file)).unwrap_or_default(),
        title.to_string(),
    )
}
