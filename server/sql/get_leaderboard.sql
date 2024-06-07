SELECT
    users.name,
    period_type,
    total_score,
    executed_prs,
    largest_score,
    prs_opened,
    prs_merged,
    best as streak_best,
    amount as streak_amount,
    latest_time_string as streak_latest_time_string
FROM
    user_period_data
    JOIN users ON users.id = user_period_data.user_id
    JOIN streak_user_data ON streak_user_data.user_id = users.id
WHERE
    period_type = $1
    and streak_user_data.streak_id = $2
ORDER BY
    total_score DESC
LIMIT
    $3 OFFSET $4
