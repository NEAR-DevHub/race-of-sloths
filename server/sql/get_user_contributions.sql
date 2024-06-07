SELECT
    users.name as name,
    o.name as organization,
    r.name as repo,
    pr.number as number,
    pr.created_at as created_at,
    pr.merged_at as merged_at,
    pr.score as score,
    pr.executed as executed
FROM
    users
    JOIN pull_requests pr ON pr.author_id = users.id
    JOIN repos r ON pr.repo_id = r.id
    JOIN organizations o ON r.organization_id = o.id
WHERE
    users.name = $1
GROUP BY
    users.name,
    o.name,
    r.name,
    pr.number,
    pr.created_at,
    pr.merged_at,
    pr.score,
    pr.executed
ORDER BY
    pr.created_at DESC
LIMIT
    $2 OFFSET $3
