use crate::db::types::UserRecord;

pub fn generate_badge(user_record: UserRecord, leaderboard_place: u64) -> Option<String> {
    let total_period = user_record
        .period_data
        .iter()
        .find(|p| p.period_type == "all-time")?;
    let week_streak = user_record.streaks.iter().find(|s| s.streak_id == 0)?;
    let month_streak = user_record.streaks.iter().find(|s| s.streak_id == 1)?;

    let svg_icon = include_str!("./public/badge_template.svg").to_owned();
    let svg_icon = svg_icon.replace("{name}", &user_record.name);
    let svg_icon = svg_icon.replace(
        "{total-contributions}",
        &total_period.prs_opened.to_string(),
    );
    let svg_icon = svg_icon.replace("{total-score}", &total_period.total_score.to_string());
    let svg_icon = svg_icon.replace("{week-streak}", &week_streak.amount.to_string());
    let svg_icon = svg_icon.replace("{month-streak}", &month_streak.amount.to_string());

    Some(svg_icon.replace("{place}", &leaderboard_place.to_string()))
}
