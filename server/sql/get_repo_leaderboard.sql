WITH top_contributors AS (
    SELECT
        pr.repo_id,
        u.name AS contributor_name,
        SUM(pr.score) AS total_score,
        ROW_NUMBER() OVER (
            PARTITION BY pr.repo_id
            ORDER BY
                SUM(pr.score) DESC
        ) AS rank
    FROM
        pull_requests pr
        JOIN users u ON pr.author_id = u.id
    GROUP BY
        pr.repo_id,
        u.id
)
SELECT
    o.name AS organization,
    r.name AS name,
    COALESCE(COUNT(pr.id), 0) AS total_prs,
    COALESCE(SUM(pr.score), 0) AS total_score,
    tc.contributor_name AS top_contributor,
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
GROUP BY
    o.name,
    r.name,
    tc.contributor_name,
    r.primary_language,
    r.open_issues,
    r.stars,
    r.forks
ORDER BY
    total_prs DESC,
    total_score DESC
LIMIT
    $1 OFFSET $2;
