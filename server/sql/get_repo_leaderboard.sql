WITH top_contributors AS (
    SELECT
        pr.repo_id,
        u.login AS contributor_login,
        u.full_name AS contributor_full_name,
        ROW_NUMBER() OVER (
            PARTITION BY pr.repo_id
            ORDER BY
                SUM(pr.rating) DESC
        ) AS rank
    FROM
        pull_requests pr
        JOIN users u ON pr.author_id = u.id
    WHERE
        pr.included_at >= $3
    GROUP BY
        pr.repo_id,
        u.id
)
SELECT
    o.login AS organization,
    o.full_name AS organization_full_name,
    r.name AS name,
    COALESCE(COUNT(pr.id), 0) AS total_prs,
    COALESCE(SUM(pr.score), 0) AS total_score,
    COALESCE(SUM(pr.rating), 0) AS total_rating,
    MAX(tc.contributor_full_name) AS contributor_full_name,
    MAX(tc.contributor_login) AS contributor_login,
    r.primary_language,
    r.open_issues,
    r.stars,
    r.forks
FROM
    repos r
    JOIN organizations o ON r.organization_id = o.id
    LEFT JOIN pull_requests pr ON pr.repo_id = r.id
    LEFT JOIN top_contributors tc ON tc.repo_id = r.id
    AND tc.rank = 1
WHERE
    r.paused = false
GROUP BY
    o.login,
    o.full_name,
    r.name,
    r.primary_language,
    r.open_issues,
    r.stars,
    r.forks
ORDER BY
    total_prs DESC,
    open_issues DESC,
    organization ASC,
    name ASC
LIMIT
    $1 OFFSET $2;
