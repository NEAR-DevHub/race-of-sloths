SELECT
    u.id,
    u.login,
    u.full_name,
    u.permanent_bonus,
    upd.total_rating
FROM
    users AS u
    JOIN user_period_data upd ON upd.user_id = u.id
    AND upd.period_type = $1
WHERE
    u.permanent_bonus > 0
ORDER BY
    u.permanent_bonus desc,
    upd.total_rating desc
LIMIT
    $2 OFFSET $3
