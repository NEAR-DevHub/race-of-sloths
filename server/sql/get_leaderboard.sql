SELECT
    users.login,
    users.full_name,
    period_type,
    total_score,
    executed_prs,
    largest_score,
    prs_opened,
    prs_merged,
    best as streak_best,
    amount as streak_amount,
    user_period_data.total_rating as total_rating,
    period as streak_type,
    streak.name as streak_name,
    latest_time_string as streak_latest_time_string
FROM
    user_period_data
    JOIN users ON users.id = user_period_data.user_id
    JOIN streak_user_data ON streak_user_data.user_id = users.id
    JOIN streak ON streak.id = streak_user_data.streak_id
WHERE
    period_type = $1
    and streak_user_data.streak_id = $2
ORDER BY
    total_rating DESC
LIMIT
    $3 OFFSET $4
