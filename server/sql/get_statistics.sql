WITH sloth_stats AS (
    SELECT
        (
            SELECT
                COUNT(*)
            FROM
                users
        ) AS number_of_sloths,
        (
            SELECT
                COUNT(*)
            FROM
                organizations
        ) AS number_of_orgs,
        (
            SELECT
                COUNT(*)
            FROM
                repos
        ) AS number_of_repos,
        (
            SELECT
                COUNT(*)
            FROM
                pull_requests
        ) AS number_of_contributions,
        (
            SELECT
                SUM(total_rating)
            FROM
                user_period_data upd
            WHERE
                period_type = 'all-time'
        ) AS total_rating
),
highest_rating AS (
    SELECT
        upd.total_rating AS highest_sloth_rating,
        u.login,
        u.full_name
    FROM
        user_period_data upd
        JOIN users u ON u.id = upd.user_id
    WHERE
        period_type = 'all-time'
    ORDER BY
        total_rating DESC
    LIMIT
        1
), fastest_merge AS (
    SELECT
        r.name,
        o.login,
        o.full_name,
        pr.created_at,
        pr.merged_at,
        pr.number
    FROM
        pull_requests pr
        JOIN repos r ON pr.repo_id = r.id
        JOIN organizations o ON o.id = r.organization_id
    ORDER BY
        pr.merged_at - pr.created_at ASC
    LIMIT
        1
), hall_of_fame AS (
    SELECT
        STRING_AGG(u.login, ',') AS fame_logins
    FROM
        users AS u
    WHERE
        u.permanent_bonus > 0
)
SELECT
    ss.number_of_sloths,
    ss.number_of_orgs,
    ss.number_of_repos,
    ss.number_of_contributions,
    ss.total_rating,
    hr.highest_sloth_rating,
    hr.login AS highest_sloth_login,
    hr.full_name AS highest_sloth_full_name,
    fm.name AS fastest_repo_name,
    fm.login AS fastest_org_login,
    fm.full_name AS fastest_org_full_name,
    fm.created_at as fastest_included,
    fm.merged_at as fastest_merged,
    fm.number AS fastest_pr_number,
    hof.fame_logins as hall_of_fame
FROM
    sloth_stats ss,
    highest_rating hr,
    fastest_merge fm,
    hall_of_fame hof;
