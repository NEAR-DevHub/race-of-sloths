use rocket::{
    fairing::{self, AdHoc},
    Build, Rocket,
};
use rocket_db_pools::Database;
use shared::{PRv2, StreakUserData, TimePeriod, TimePeriodString, UserPeriodData};
use sqlx::{PgPool, Postgres, Transaction};

#[derive(Database, Clone, Debug)]
#[database("race-of-sloths")]
pub struct DB(PgPool);

pub mod types;

use types::{LeaderboardRecord, Statistics};

use self::types::{
    RepoLeaderboardRecord, RepoRecord, StreakRecord, User, UserCachedMetadata,
    UserContributionRecord, UserPeriodRecord, UserRecord,
};

impl DB {
    pub async fn upsert_user(
        tx: &mut Transaction<'static, Postgres>,
        user_id: u32,
        user: &str,
        percent: u32,
    ) -> anyhow::Result<i32> {
        // First try to update the user
        let rec = sqlx::query!(
            r#"
            UPDATE users
            SET permanent_bonus = $2
            WHERE id = $1
            RETURNING id
            "#,
            user_id as i32,
            percent as i32
        )
        .fetch_optional(tx.as_mut())
        .await?;

        // If the update did not find a matching row, insert the user
        if let Some(record) = rec {
            Ok(record.id)
        } else {
            let rec = sqlx::query!(
                r#"
                INSERT INTO users (id, login, permanent_bonus)
                VALUES ($1, $2, $3)
                ON CONFLICT (id) DO NOTHING
                RETURNING id
                "#,
                user_id as i32,
                user,
                percent as i32
            )
            .fetch_one(tx.as_mut())
            .await?;

            Ok(rec.id)
        }
    }

    pub async fn update_user_full_name(
        tx: &mut Transaction<'static, Postgres>,
        user: &str,
        full_name: &str,
    ) -> anyhow::Result<()> {
        sqlx::query!(
            r#"
            UPDATE users
            SET full_name = $2
            WHERE login = $1
            "#,
            user,
            full_name
        )
        .execute(tx.as_mut())
        .await?;
        Ok(())
    }

    pub async fn update_organization_full_name(
        tx: &mut Transaction<'static, Postgres>,
        organization: &str,
        full_name: &str,
    ) -> anyhow::Result<()> {
        sqlx::query!(
            r#"
            UPDATE organizations
            SET full_name = $2
            WHERE login = $1
            "#,
            organization,
            full_name
        )
        .execute(tx.as_mut())
        .await?;
        Ok(())
    }

    pub async fn get_organization_repo_id(
        tx: &mut Transaction<'static, Postgres>,
        organization: &str,
        repo: &str,
    ) -> anyhow::Result<Option<(i32, i32)>> {
        Ok(sqlx::query!(
            r#"
                    SELECT org.id as org_id, r.id as repo_id
                    FROM organizations org
                    JOIN repos r ON r.organization_id = org.id
                    WHERE org.login = $1 AND r.name = $2
                    "#,
            organization,
            repo
        )
        .fetch_optional(tx.as_mut())
        .await
        .map(|rec| rec.map(|r| (r.org_id, r.repo_id)))?)
    }

    pub async fn upsert_organization(
        tx: &mut Transaction<'static, Postgres>,
        name: &str,
    ) -> anyhow::Result<i32> {
        // First try to update the organization
        let rec = sqlx::query!(
            r#"
            UPDATE organizations
            SET login = $1
            WHERE login = $1
            RETURNING id
            "#,
            name
        )
        .fetch_optional(tx.as_mut())
        .await?;

        // If the update did not find a matching row, insert the organization
        if let Some(record) = rec {
            Ok(record.id)
        } else {
            let rec = sqlx::query!(
                r#"
                INSERT INTO organizations (login)
                VALUES ($1)
                ON CONFLICT (login) DO NOTHING
                RETURNING id
                "#,
                name
            )
            .fetch_one(tx.as_mut())
            .await?;

            Ok(rec.id)
        }
    }

    pub async fn upsert_repo(
        tx: &mut Transaction<'static, Postgres>,
        organization_id: i32,
        name: &str,
    ) -> anyhow::Result<i32> {
        // First try to update the repo
        let rec = sqlx::query!(
            r#"
            UPDATE repos
            SET name = $2
            WHERE organization_id = $1 AND name = $2
            RETURNING id
            "#,
            organization_id,
            name
        )
        .fetch_optional(tx.as_mut())
        .await?;

        // If the update did not find a matching row, insert the repo
        if let Some(record) = rec {
            Ok(record.id)
        } else {
            let rec = sqlx::query!(
                r#"
                INSERT INTO repos (organization_id, name)
                VALUES ($1, $2)
                ON CONFLICT (organization_id, name) DO NOTHING
                RETURNING id
                "#,
                organization_id,
                name
            )
            .fetch_one(tx.as_mut())
            .await?;

            Ok(rec.id)
        }
    }

    pub async fn remove_non_existent_prs(
        tx: &mut Transaction<'static, Postgres>,
        prs: &[(PRv2, bool)],
    ) -> anyhow::Result<()> {
        let pr_keys: Vec<(String, String, i32)> = prs
            .iter()
            .map(|(pr, _)| (pr.organization.clone(), pr.repo.clone(), pr.number as i32))
            .collect();

        sqlx::query!(
            r#"
            DELETE FROM pull_requests
            WHERE (repo_id, number) NOT IN (
                SELECT r.id, p.number
                FROM unnest($1::text[], $2::text[], $3::int[]) AS p(org, repo, number)
                JOIN organizations o ON o.login = p.org
                JOIN repos r ON r.organization_id = o.id AND r.name = p.repo
            )
            "#,
            &pr_keys
                .iter()
                .map(|(org, _, _)| org.clone())
                .collect::<Vec<_>>(),
            &pr_keys
                .iter()
                .map(|(_, repo, _)| repo.clone())
                .collect::<Vec<_>>(),
            &pr_keys
                .iter()
                .map(|(_, _, number)| *number as i32)
                .collect::<Vec<_>>(),
        )
        .execute(tx.as_mut())
        .await?;

        Ok(())
    }

    pub async fn upsert_pull_request(
        tx: &mut Transaction<'static, Postgres>,
        repo_id: i32,
        number: i32,
        author_id: i32,
        included_at: chrono::NaiveDateTime,
        created_at: chrono::NaiveDateTime,
        merged_at: Option<chrono::NaiveDateTime>,
        score: Option<u32>,
        rating: u32,
        permanent_bonus: u32,
        streak_bonus: u32,
        executed: bool,
    ) -> anyhow::Result<i32> {
        // First try to update the pull request
        let rec = sqlx::query!(
            r#"
            UPDATE pull_requests
            SET merged_at = $3, executed = $4, score = $5, rating = $6, permanent_bonus = $7, streak_bonus = $8, created_at = $9, included_at = $10
            WHERE repo_id = $1 AND number = $2
            RETURNING id
            "#,
            repo_id,
            number,
            merged_at,
            executed,
            score.map(|s| s as i32),
            rating as i32,
            permanent_bonus as i32,
            streak_bonus as i32,
            created_at,
            included_at
        )
        .fetch_optional(tx.as_mut())
        .await?;

        // If the update did not find a matching row, insert the pull request
        if let Some(record) = rec {
            Ok(record.id)
        } else {
            let rec = sqlx::query!(
                r#"
                INSERT INTO pull_requests (repo_id, number, author_id, included_at, created_at, merged_at, executed, score, rating, permanent_bonus, streak_bonus)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                ON CONFLICT (repo_id, number) DO NOTHING
                RETURNING id
                "#,
                repo_id,
                number,
                author_id,
                included_at,
                created_at,
                merged_at,
                executed,
                score.map(|s| s as i32),
                rating as i32,
                permanent_bonus as i32,
                streak_bonus as i32,
            )
            .fetch_one(tx.as_mut())
            .await?;

            Ok(rec.id)
        }
    }

    pub async fn upsert_user_period_data(
        tx: &mut Transaction<'static, Postgres>,
        period: TimePeriodString,
        data: &UserPeriodData,
        user_id: i32,
    ) -> anyhow::Result<()> {
        // First try to update the user period data
        let rec = sqlx::query!(
            r#"
            UPDATE user_period_data
            SET total_score = $3, executed_prs = $4, largest_score = $5, prs_opened = $6, prs_merged = $7, total_rating = $8, largest_rating_per_pr = $9
            WHERE user_id = $1 AND period_type = $2
            RETURNING user_id
            "#,
            user_id,
            period,
            data.total_score as i32,
            data.executed_prs as i32,
            data.largest_score as i32,
            data.prs_opened as i32,
            data.prs_merged as i32,
            data.total_rating as i32,
            data.largest_rating_per_pr as i32
        )
        .fetch_optional(tx.as_mut())
        .await?;

        // If the update did not find a matching row, insert the user period data
        if rec.is_none() {
            sqlx::query!(
                r#"
                INSERT INTO user_period_data (user_id, period_type, total_score, executed_prs, largest_score, prs_opened, prs_merged, total_rating, largest_rating_per_pr)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                ON CONFLICT (user_id, period_type) DO NOTHING
                "#,
                user_id,
                period,
                data.total_score as i32,
                data.executed_prs as i32,
                data.largest_score as i32,
                data.prs_opened as i32,
                data.prs_merged as i32,
                data.total_rating as i32,
                data.largest_rating_per_pr as i32
            )
            .execute(tx.as_mut())
            .await?;
        }
        Ok(())
    }

    pub async fn upsert_streak_user_data(
        tx: &mut Transaction<'static, Postgres>,
        data: &StreakUserData,
        streak_id: i32,
        user_id: i32,
    ) -> anyhow::Result<()> {
        // First try to update the streak user data
        let rec = sqlx::query!(
            r#"
            UPDATE streak_user_data
            SET amount = $3, best = $4, latest_time_string = $5
            WHERE user_id = $1 AND streak_id = $2
            RETURNING user_id
            "#,
            user_id,
            streak_id,
            data.amount as i32,
            data.best as i32,
            data.latest_time_string
        )
        .fetch_optional(tx.as_mut())
        .await?;

        // If the update did not find a matching row, insert the streak user data
        if rec.is_none() {
            sqlx::query!(
                r#"
                INSERT INTO streak_user_data (user_id, streak_id, amount, best, latest_time_string)
                VALUES ($1, $2, $3, $4, $5)
                ON CONFLICT (user_id, streak_id) DO NOTHING
                "#,
                user_id,
                streak_id,
                data.amount as i32,
                data.best as i32,
                data.latest_time_string
            )
            .execute(tx.as_mut())
            .await?;
        }
        Ok(())
    }

    pub async fn update_repo_metadata(
        tx: &mut Transaction<'static, Postgres>,
        repo_id: i32,
        stars: u32,
        forks: u32,
        open_issues: u32,
        primary_language: Option<String>,
    ) -> anyhow::Result<()> {
        sqlx::query!(
            r#"
            UPDATE repos
            SET stars = $1, forks = $2, open_issues = $3, primary_language = $4
            WHERE id = $5"#,
            stars as i32,
            forks as i32,
            open_issues as i32,
            primary_language,
            repo_id
        )
        .execute(tx.as_mut())
        .await?;
        Ok(())
    }

    pub async fn get_user(
        &self,
        login: &str,
        place_strings: &[String],
    ) -> anyhow::Result<Option<UserRecord>> {
        let (user_rec, full_name, percent) = match sqlx::query!(
            "SELECT id, full_name, permanent_bonus FROM users
            WHERE login = $1",
            login
        )
        .fetch_optional(&self.0)
        .await?
        {
            Some(rec) => (rec.id, rec.full_name, rec.permanent_bonus),
            None => return Ok(None),
        };

        let period_data_recs: Vec<UserPeriodRecord> = sqlx::query_as!(
            UserPeriodRecord,
            r#"
                SELECT period_type, total_score, executed_prs, largest_score, prs_opened, prs_merged, total_rating
                FROM user_period_data
                WHERE user_id = $1
                "#,
            user_rec,
        )
        .fetch_all(&self.0)
        .await?;

        let streak_recs: Vec<StreakRecord> =
            sqlx::query_file_as!(StreakRecord, "./sql/get_streaks_for_user_id.sql", user_rec)
                .fetch_all(&self.0)
                .await?;

        let mut leaderboard_places = Vec::with_capacity(place_strings.len());
        for place in place_strings {
            let record = self.get_leaderboard_place(place, user_rec).await?;
            if record.is_none() {
                continue;
            }
            leaderboard_places.push((place.clone(), record.unwrap() as u32));
        }

        let first_contribution = sqlx::query!(
            r#"
            SELECT included_at
            FROM pull_requests
            WHERE author_id = $1
            ORDER BY included_at ASC
            LIMIT 1
            "#,
            user_rec
        )
        .fetch_optional(&self.0)
        .await?;

        let user = UserRecord {
            id: user_rec,
            first_contribution: first_contribution
                .map(|x| x.included_at)
                .unwrap_or_else(|| chrono::Utc::now().naive_utc()),
            login: login.to_string(),
            name: full_name,
            lifetime_percent: percent,
            period_data: period_data_recs,
            streaks: streak_recs,
            leaderboard_places,
        };

        Ok(Some(user))
    }

    pub async fn get_leaderboard(
        &self,
        period: &str,
        streak_id: i32,
        page: i64,
        limit: i64,
    ) -> anyhow::Result<(Vec<LeaderboardRecord>, i64)> {
        // Unchecked as rank() doesn't return NULL, but sqlx thinks it does
        let records = sqlx::query_file_as_unchecked!(
            LeaderboardRecord,
            "sql/get_leaderboard.sql",
            period,
            streak_id,
            limit,
            page * limit
        )
        .fetch_all(&self.0)
        .await?;

        // TODO: Replace this with a single query
        let total_count = sqlx::query!(
            r#"SELECT COUNT(DISTINCT(user_id)) as id
            FROM user_period_data
            WHERE period_type = $1
            "#,
            period
        )
        .fetch_one(&self.0)
        .await?;

        Ok((records, total_count.id.unwrap_or_default()))
    }

    pub async fn get_leaderboard_place(
        &self,
        period: &str,
        user_id: i32,
    ) -> anyhow::Result<Option<i64>> {
        let rec = sqlx::query_file!("./sql/get_leaderboard_place.sql", period, user_id)
            .fetch_optional(&self.0)
            .await?;

        Ok(rec.and_then(|rec| rec.place))
    }

    pub async fn get_repo_leaderboard(
        &self,
        page: i64,
        limit: i64,
    ) -> anyhow::Result<(Vec<RepoLeaderboardRecord>, u64)> {
        let offset = page * limit;
        // COALESCE is used to return 0 if there are no PRs for a repo
        // But sqlx still thinks that it's NONE
        let current_month = chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default();
        let current_month = TimePeriod::Month
            .start_period(current_month as u64)
            .unwrap_or_default();
        let records = sqlx::query_file_as!(
            RepoLeaderboardRecord,
            "./sql/get_repo_leaderboard.sql",
            limit,
            offset,
            current_month.naive_utc()
        )
        .fetch_all(&self.0)
        .await?;

        let total_count = sqlx::query!(
            r#"SELECT COUNT(DISTINCT(r.organization_id, r.id)) as id
            FROM repos r
            "#,
        )
        .fetch_one(&self.0)
        .await?;

        Ok((records, total_count.id.unwrap_or_default() as u64))
    }

    pub async fn get_contribution(
        &self,
        org: &str,
        repo: &str,
        number: i32,
    ) -> anyhow::Result<Option<UserContributionRecord>> {
        Ok(sqlx::query_file_as!(
            UserContributionRecord,
            "./sql/get_contribution.sql",
            org,
            repo,
            number
        )
        .fetch_optional(&self.0)
        .await?)
    }

    pub async fn get_user_contributions(
        &self,
        user: &str,
        page: i64,
        limit: i64,
    ) -> anyhow::Result<(Vec<UserContributionRecord>, u64)> {
        let offset = page * limit;
        let records = sqlx::query_file_as!(
            UserContributionRecord,
            "./sql/get_user_contributions.sql",
            user,
            limit,
            offset
        )
        .fetch_all(&self.0)
        .await?;

        let total = sqlx::query!(
            r#"SELECT COUNT(DISTINCT(pr.id)) as id
            FROM pull_requests pr
            JOIN users ON pr.author_id = users.id
            WHERE users.login = $1
            "#,
            user
        )
        .fetch_one(&self.0)
        .await?;
        Ok((records, total.id.unwrap_or_default() as u64))
    }

    pub async fn get_contributors_of_the_month(
        &self,
        repo: &str,
        org: &str,
    ) -> anyhow::Result<Vec<(String, i64)>> {
        let rec = sqlx::query!(
            r#"
        SELECT users.login, SUM(pr.score) as total_score
        FROM organizations o
        JOIN repos r ON r.organization_id = o.id
        JOIN pull_requests pr ON pr.repo_id = r.id
        JOIN users ON pr.author_id = users.id
        WHERE pr.included_at >= (now() - INTERVAL '1 MONTH')
        AND r.name = $1
        AND o.login = $2
        GROUP BY users.login
        ORDER BY COUNT(pr.id) DESC
        LIMIT 3
        "#,
            repo,
            org
        )
        .fetch_all(&self.0)
        .await?;

        Ok(rec
            .into_iter()
            .map(|r| (r.login, r.total_score.unwrap_or_default()))
            .collect())
    }

    pub async fn get_repos_for_update(
        tx: &mut Transaction<'static, Postgres>,
    ) -> anyhow::Result<Vec<RepoRecord>> {
        let rec = sqlx::query_file_as!(RepoRecord, "./sql/get_repos.sql")
            .fetch_all(tx.as_mut())
            .await?;

        Ok(rec)
    }

    pub async fn get_users_for_update(
        tx: &mut Transaction<'static, Postgres>,
    ) -> anyhow::Result<Vec<User>> {
        let rec = sqlx::query_as!(
            User,
            r#"
            SELECT login, full_name
            FROM users
            FOR UPDATE"#
        )
        .fetch_all(tx.as_mut())
        .await?;

        Ok(rec)
    }

    pub async fn get_organizations_for_update(
        tx: &mut Transaction<'static, Postgres>,
    ) -> anyhow::Result<Vec<User>> {
        let rec = sqlx::query_as!(
            User,
            r#"
            SELECT login, full_name
            FROM organizations
            FOR UPDATE"#
        )
        .fetch_all(tx.as_mut())
        .await?;

        Ok(rec)
    }

    pub async fn get_projects(&self) -> anyhow::Result<Vec<(String, String)>> {
        let rec = sqlx::query!(
            r#"
            SELECT o.login, r.name
            FROM repos r
            JOIN organizations o ON r.organization_id = o.id
            "#,
        )
        .fetch_all(&self.0)
        .await?;

        Ok(rec.into_iter().map(|r| (r.login, r.name)).collect())
    }

    pub async fn is_pr_available(
        &self,
        org: &str,
        repo: &str,
        number: i32,
    ) -> anyhow::Result<bool> {
        let rec = sqlx::query!(
            r#"
            SELECT pull_requests.id
            FROM pull_requests
            JOIN repos r ON repo_id = r.id
            JOIN organizations o ON r.organization_id = o.id
            WHERE o.login = $1 AND r.name = $2 AND number = $3
            "#,
            org,
            repo,
            number
        )
        .fetch_optional(&self.0)
        .await?;

        Ok(rec.is_some())
    }

    pub async fn clear_prs(tx: &mut Transaction<'static, Postgres>) -> anyhow::Result<()> {
        sqlx::query!("DELETE FROM pull_requests")
            .execute(tx.as_mut())
            .await?;
        Ok(())
    }

    pub async fn get_user_id(
        tx: &mut Transaction<'static, Postgres>,
        name: &str,
    ) -> anyhow::Result<i32> {
        let rec = sqlx::query!(
            r#"
            SELECT id
            FROM users
            WHERE login = $1
            "#,
            name
        )
        .fetch_one(tx.as_mut())
        .await?;

        Ok(rec.id)
    }

    pub async fn get_user_cached_metadata(
        &self,
        username: &str,
    ) -> anyhow::Result<Option<UserCachedMetadata>> {
        Ok(sqlx::query_as!(
            UserCachedMetadata,
            r#"
                    SELECT image_base64, load_time
                    FROM user_cached_metadata
                    JOIN users u ON user_id = u.id
                    WHERE u.login = $1
                    "#,
            username
        )
        .fetch_optional(&self.0)
        .await?)
    }

    pub async fn upsert_user_cached_metadata(
        &self,
        user_id: i32,
        image_base64: &str,
    ) -> anyhow::Result<()> {
        // First try to update the user
        let rec = sqlx::query!(
            r#"
                UPDATE user_cached_metadata
                SET image_base64 = $2, load_time = now()
                WHERE user_id = $1
                "#,
            user_id,
            image_base64
        )
        .fetch_optional(&self.0)
        .await?;

        // If the update did not find a matching row, insert the user
        if rec.is_none() {
            sqlx::query!(
                r#"
                INSERT INTO user_cached_metadata (user_id, image_base64, load_time)
                VALUES ($1, $2, now())
                ON CONFLICT (user_id) DO NOTHING
                "#,
                user_id,
                image_base64
            )
            .execute(&self.0)
            .await?;
        }

        Ok(())
    }

    pub async fn statistics(&self) -> anyhow::Result<Statistics> {
        let rec = sqlx::query_file_as!(Statistics, "./sql/get_statistics.sql")
            .fetch_one(&self.0)
            .await?;

        Ok(rec)
    }
}

async fn run_migrations(rocket: Rocket<Build>) -> fairing::Result {
    match DB::fetch(&rocket) {
        Some(db) => match sqlx::migrate!("./migrations").run(&**db).await {
            Ok(_) => Ok(rocket),
            Err(e) => {
                rocket::error!("Failed to initialize SQLx database: {}", e);
                Err(rocket)
            }
        },
        None => Err(rocket),
    }
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("SQLx Stage", |rocket| async {
        rocket
            .attach(DB::init())
            .attach(AdHoc::try_on_ignite("SQLx Migrations", run_migrations))
    })
}
