use std::sync::Arc;

use num_format::{Locale, ToFormattedString};
use rocket::{tokio::fs::read_to_string, FromFormField};
use shared::{telegram::TelegramSubscriber, TimePeriod};
use usvg::{fontdb, Options, Tree, WriteOptions};

use crate::{
    db::types::{StreakRecord, UserCachedMetadata, UserContributionRecord, UserRecord},
    types::UserContributionResponse,
};

#[derive(Debug, Clone, Copy, FromFormField)]
pub enum Mode {
    Dark,
    Light,
}

pub async fn generate_svg_badge(
    telegram: &Arc<TelegramSubscriber>,
    fontdb: Arc<fontdb::Database>,
    user_record: UserRecord,
    mode: Mode,
    user_metadata: Option<UserCachedMetadata>,
    contribution: Option<UserContributionRecord>,
) -> anyhow::Result<String> {
    let all_time = TimePeriod::AllTime.time_string(0);
    let total_period = user_record.get_total_period().cloned().unwrap_or_default();
    let week_streak = user_record
        .streaks
        .iter()
        .find(|e| e.streak_type == "Weekly")
        .cloned()
        .unwrap_or_default();
    let month_streak = user_record
        .streaks
        .iter()
        .find(|e| e.streak_type == "Monthly")
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

    let svg_icon = match (&contribution, &user_metadata) {
        (None, Some(_)) => {
            read_svg_with_mode("./public/templates/badge_bot_template", mode).await?
        }
        (Some(_), Some(_)) => {
            read_svg_with_mode("./public/templates/badge_bot_with_pr_info", mode).await?
        }
        (None, None) => read_svg_with_mode("./public/templates/badge_share_template", mode).await?,

        (Some(_), None) => return Err(anyhow::anyhow!("PR info provided without user metadata")),
    };

    let svg_icon = if let Some(user_metadata) = user_metadata {
        process_user_metadata(telegram, svg_icon, &user_record, user_metadata)
    } else {
        svg_icon
    };

    let svg_icon = if let Some(contribution) = contribution {
        let svg_icon = process_challenges(svg_icon, &week_streak, &month_streak, mode).await?;
        process_contribution(svg_icon, contribution).await?
    } else {
        svg_icon
    };

    let svg_icon = svg_icon
        .replace(
            "{total-contributions}",
            &total_period.prs_opened.to_string(),
        )
        .replace("{week-streak}", &week_streak.amount.to_string())
        .replace("{max-week-streak}", &week_streak.best.to_string())
        .replace("{month-streak}", &month_streak.amount.to_string())
        .replace("{max-month-streak}", &month_streak.best.to_string())
        .replace("{place}", &place)
        .replace("{place-type}", &place_type)
        .replace(
            "{total-rating}",
            &total_period.total_rating.to_formatted_string(&Locale::en),
        );

    let svg_icon = process_rank(svg_icon, &user_record).await;

    postprocess_svg(svg_icon, fontdb)
}

pub async fn read_svg_with_mode(string: &str, mode: Mode) -> Result<String, std::io::Error> {
    let file_suffix = match mode {
        Mode::Dark => "dark",
        Mode::Light => "white",
    };
    read_to_string(format!("{}_{}.svg", string, file_suffix)).await
}

pub async fn generate_png_meta_badge(
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
        .unwrap_or_default();
    let month_streak = user_record
        .streaks
        .iter()
        .find(|e| e.streak_type == "Monthly")
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

    let svg_icon = std::fs::read_to_string("./public/templates/badge_meta_template.svg")?;
    let github_handle = format!("@{}", user_record.login);

    let svg_icon = image_processing(
        telegram,
        svg_icon,
        &user_metadata.image_base64,
        &user_record.login,
    );

    let svg_icon = process_rank(svg_icon, &user_record)
        .await
        .replace(
            "{total-rating}",
            &total_period.total_rating.to_formatted_string(&Locale::en),
        )
        .replace("{week-streak}", &week_streak.amount.to_string())
        .replace("{max-week-streak}", &week_streak.best.to_string())
        .replace("{month-streak}", &month_streak.amount.to_string())
        .replace("{max-month-streak}", &month_streak.best.to_string())
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

fn process_user_metadata(
    telegram: &Arc<TelegramSubscriber>,
    svg_icon: String,
    user_record: &UserRecord,
    user_metadata: UserCachedMetadata,
) -> String {
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
}

async fn process_challenges(
    svg_icon: String,
    week_streak: &StreakRecord,
    month_streak: &StreakRecord,
    mode: Mode,
) -> anyhow::Result<String> {
    let week_streak = crate::types::Streak::new(
        week_streak.name.clone(),
        week_streak.amount as u32,
        week_streak.best as u32,
        week_streak.latest_time_string.clone(),
        week_streak.streak_type.clone(),
    );
    let month_streak = crate::types::Streak::new(
        month_streak.name.clone(),
        month_streak.amount as u32,
        month_streak.best as u32,
        month_streak.latest_time_string.clone(),
        month_streak.streak_type.clone(),
    );

    let (challenge_svg, challenge_subtitle) = match (week_streak.achived, month_streak.achived) {
        (true, true) => (
            read_to_string("public/streaks/streak_done.svg").await?,
            "Great job! Relax and take your time.. Or keep running!",
        ),
        (true, false) => (
            read_svg_with_mode("public/streaks/streak_part_done", mode).await?,
            "Weekly completed! Monthly is still missing, just do it!",
        ),
        (false, true) => (
            read_svg_with_mode("public/streaks/streak_part_done", mode).await?,
            "Monthly completed! Weekly is still missing, just do it",
        ),
        (false, false) => (
            read_to_string("public/streaks/streak_not_done.svg").await?,
            "Not completed yet… You can do it!",
        ),
    };

    Ok(svg_icon
        .replace("{challenge-title}", "Your streaks status:")
        .replace("{challenge-text}", challenge_subtitle)
        .replace("{challenge-svg}", &challenge_svg))
}

async fn process_contribution(
    svg_icon: String,
    contribution: UserContributionRecord,
) -> anyhow::Result<String> {
    let contribution = UserContributionResponse::from(contribution);

    let svg = match contribution.executed {
        true => "public/pr_state/finalized.svg",
        false => "public/pr_state/in-progress.svg",
    };
    let svg = read_to_string(svg).await?;

    let contribution_text = match (contribution.total_rating, contribution.executed) {
        (rating, true) => {
            format!("This is the way, sloth! You've got {rating} points!")
        }
        (rating, false) if rating > 0 => {
            format!("Keep it up, sloth! You've got {rating} points!")
        }
        _ => "Stay calm and keep pushing, sloth!".to_string(),
    };

    Ok(svg_icon
        .replace(
            "{pr-status-title}",
            &format!("Your PR status: {}", contribution.status),
        )
        .replace("{pr-status-text}", &contribution_text)
        .replace("{pr-status-svg}", &svg))
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

async fn process_rank(svg_icon: String, user_record: &UserRecord) -> String {
    let (rank, rank_svg, title) = rank_data(user_record).await;
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

async fn rank_data(user_record: &UserRecord) -> (String, String, String) {
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
        read_to_string(format!("./public/ranks/{}", rank_svg_file))
            .await
            .unwrap_or_default(),
        title.to_string(),
    )
}
