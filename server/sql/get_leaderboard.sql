SELECT
    users.login,
    users.full_name,
    period_type,
    total_score,
    executed_prs,
    largest_score,
    prs_opened,
    prs_merged,
    prs_scored,
    weekly_streak.best as weekly_streak_best,
    weekly_streak.amount as weekly_streak_amount,
    weekly_streak.latest_time_string as weekly_streak_latest_time_string,
    monthly_streak.best as monthly_streak_best,
    monthly_streak.amount as monthly_streak_amount,
    monthly_streak.latest_time_string as monthly_streak_latest_time_string,
    user_period_data.total_rating as total_rating,
    RANK() OVER (
        ORDER BY
            total_rating DESC
    ) as place,
    users.permanent_bonus as permanent_bonus
FROM
    user_period_data
    JOIN users ON users.id = user_period_data.user_id
    JOIN streak_user_data AS weekly_streak ON weekly_streak.user_id = users.id
    AND weekly_streak.streak_id = 0
    JOIN streak_user_data AS monthly_streak ON monthly_streak.user_id = users.id
    AND monthly_streak.streak_id = 1
WHERE
    period_type = $1
    and total_rating > 0
ORDER BY
    place,
    total_rating DESC
LIMIT
    $2 OFFSET $3
