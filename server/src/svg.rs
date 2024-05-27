use crate::db::types::UserRecord;

pub fn generate_badge(
    user_record: UserRecord,
    leaderboard_place: u64,
    image_base64: &str,
) -> anyhow::Result<Option<String>> {
    let total_period = user_record
        .period_data
        .iter()
        .find(|p| p.period_type == "all-time");
    let week_streak = user_record.streaks.iter().find(|s| s.streak_id == 0);
    let month_streak = user_record.streaks.iter().find(|s| s.streak_id == 1);

    if total_period.is_none() || week_streak.is_none() || month_streak.is_none() {
        return Ok(None);
    }
    let (total_period, week_streak, month_streak) = (
        total_period.unwrap(),
        week_streak.unwrap(),
        month_streak.unwrap(),
    );

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

    Ok(Some(
        svg_icon.replace("{place}", &leaderboard_place.to_string()),
    ))
}
