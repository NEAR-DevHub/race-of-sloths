use std::sync::Arc;

use num_format::{Locale, ToFormattedString};
use rocket::FromFormField;
use shared::{telegram::TelegramSubscriber, TimePeriod};
use usvg::{fontdb, Options, Tree, WriteOptions};

use crate::db::types::{UserCachedMetadata, UserRecord};

#[derive(Debug, Clone, Copy, FromFormField)]
pub enum Mode {
    Dark,
    Light,
}

pub fn generate_svg_badge(
    telegram: &Arc<TelegramSubscriber>,
    fontdb: Arc<fontdb::Database>,
    user_record: UserRecord,
    mode: Mode,
    user_metadata: Option<UserCachedMetadata>,
) -> anyhow::Result<String> {
    let all_time = TimePeriod::AllTime.time_string(0);
    let total_period = user_record.get_total_period().cloned().unwrap_or_default();
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

    let svg_icon = match (&user_metadata, mode) {
        (Some(_), Mode::Light) => std::fs::read_to_string("./public/badge_bot_template_white.svg")?,
        (Some(_), Mode::Dark) => std::fs::read_to_string("./public/badge_bot_template_dark.svg")?,
        (None, Mode::Light) => std::fs::read_to_string("./public/badge_share_template_white.svg")?,
        (None, Mode::Dark) => std::fs::read_to_string("./public/badge_share_template_dark.svg")?,
    };

    let svg_icon = if let Some(user_metadata) = user_metadata {
        let sloth_id = if user_record.id == i32::MAX {
            "Newcomer".to_string()
        } else {
            format!("Sloth#{:04}", user_record.id)
        };
        image_processing(
            telegram,
            svg_icon,
            &user_metadata.image_base64,
            &user_record.login,
        )
        .replace("{login}", &user_record.login)
        .replace("{sloth-id}", &sloth_id)
    } else {
        svg_icon
    };

    let svg_icon = svg_icon
        .replace(
            "{total-contributions}",
            &total_period.prs_opened.to_string(),
        )
        .replace("{max-week-streak}", &week_streak.to_string())
        .replace("{max-month-streak}", &month_streak.to_string())
        .replace("{place}", &place)
        .replace("{place-type}", &place_type)
        .replace(
            "{total-rating}",
            &total_period.total_rating.to_formatted_string(&Locale::en),
        );

    let svg_icon = process_rank(svg_icon, &user_record);

    postprocess_svg(svg_icon, fontdb)
}

pub fn generate_png_meta_badge(
    telegram: &Arc<TelegramSubscriber>,
    user_record: UserRecord,
    user_metadata: UserCachedMetadata,
    fontdb: Arc<fontdb::Database>,
) -> anyhow::Result<Vec<u8>> {
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

    let svg_icon = image_processing(
        telegram,
        svg_icon,
        &user_metadata.image_base64,
        &user_record.login,
    );

    let svg_icon = process_rank(svg_icon, &user_record)
        .replace(
            "{total-rating}",
            &total_period.total_rating.to_formatted_string(&Locale::en),
        )
        .replace("{max-week-streak}", &week_streak.to_string())
        .replace("{max-month-streak}", &month_streak.to_string())
        .replace("{place}", &place)
        .replace("{place-type}", &place_type)
        .replace(
            "{total-contributions}",
            &total_period.prs_opened.to_formatted_string(&Locale::en),
        )
        .replace("{github-handle}", &github_handle)
        .replace(
            "{name}",
            &user_record
                .name
                .clone()
                .unwrap_or_else(|| github_handle.clone()),
        );
    postprocess_svg_to_png(svg_icon, fontdb)
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

fn postprocess_svg_to_png(svg: String, fontdb: Arc<fontdb::Database>) -> anyhow::Result<Vec<u8>> {
    let tree = Tree::from_str(
        &svg,
        &Options {
            fontdb,
            ..Default::default()
        },
    )?;

    let pixmap_size = tree.size().to_int_size();
    let mut pixmap = tiny_skia::Pixmap::new(pixmap_size.width(), pixmap_size.height())
        .ok_or_else(|| anyhow::anyhow!("Failed to create pixmap"))?;
    resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());
    Ok(pixmap.encode_png()?)
}

fn process_rank(svg_icon: String, user_record: &UserRecord) -> String {
    let (rank, rank_svg, title) = rank_data(user_record);
    svg_icon
        .replace("{rank}", &rank)
        .replace("{rank-svg}", &rank_svg)
        .replace("{rank-title}", &title)
}

// TODO: Might be better to use a library for this or extract that when we load the image
fn determine_image_type(image_base64: &str) -> Option<&'static str> {
    // Base64 encoded magic numbers
    const PNG_MAGIC: &str = "iVBORw0KGgo";
    const JPEG_MAGIC: &str = "/9j/";

    if image_base64.starts_with(PNG_MAGIC) {
        Some("image/png")
    } else if image_base64.starts_with(JPEG_MAGIC) {
        Some("image/jpeg")
    } else {
        None
    }
}

fn image_processing(
    telegram: &Arc<TelegramSubscriber>,
    svg_icon: String,
    icon_base64: &str,
    user: &str,
) -> String {
    let image_type = if let Some(image) = determine_image_type(icon_base64) {
        image
    } else {
        crate::error(
            telegram,
            &format!(
                "Failed to determine image type for {} avatar. Defaulting to PNG",
                user
            ),
        );
        "image/png"
    };

    svg_icon
        .replace("{image}", icon_base64)
        .replace("{image-type}", image_type)
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
