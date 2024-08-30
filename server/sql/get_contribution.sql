SELECT
    o.login as organization_login,
    o.full_name as organization_full_name,
    r.name as repo,
    pr.number as number,
    pr.included_at as included_at,
    pr.merged_at as merged_at,
    pr.score as score,
    pr.executed as executed,
    pr.permanent_bonus as percentage_multiplier,
    pr.streak_bonus as streak_bonus_rating,
    pr.rating as rating
FROM
    users
    JOIN pull_requests pr ON pr.author_id = users.id
    JOIN repos r ON pr.repo_id = r.id
    JOIN organizations o ON r.organization_id = o.id
WHERE
    o.login = $1
    AND r.name = $2
    AND pr.number = $3
GROUP BY
    o.login,
    o.full_name,
    r.name,
    pr.number,
    pr.included_at,
    pr.merged_at,
    pr.score,
    pr.executed,
    pr.permanent_bonus,
    pr.streak_bonus,
    pr.rating
