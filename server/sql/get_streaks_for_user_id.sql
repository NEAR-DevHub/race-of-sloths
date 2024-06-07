SELECT
    streak.id as streak_id,
    name,
    period as streak_type,
    amount,
    best,
    latest_time_string
FROM
    streak_user_data
    JOIN streak ON streak.id = streak_user_data.streak_id
WHERE
    user_id = $1
