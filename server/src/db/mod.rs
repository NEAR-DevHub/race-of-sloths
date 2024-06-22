use rocket::{
    fairing::{self, AdHoc},
    Build, Rocket,
};
use rocket_db_pools::Database;
use shared::{StreakUserData, TimePeriodString, UserPeriodData};
use sqlx::PgPool;

#[derive(Database, Clone, Debug)]
#[database("race-of-sloths")]
pub struct DB(PgPool);

pub mod types;

use types::LeaderboardRecord;

use self::types::{
    RepoLeaderboardRecord, RepoRecord, StreakRecord, User, UserCachedMetadata,
    UserContributionRecord, UserPeriodRecord, UserRecord,
};

impl DB {
    pub async fn upsert_user(&self, user: &str, percent: u32) -> anyhow::Result<i32> {
        // First try to update the user
        let rec = sqlx::query!(
            r#"
            UPDATE users
            SET permanent_bonus = $2
            WHERE login = $1
            RETURNING id
            "#,
            user,
            percent as i32
        )
        .fetch_optional(&self.0)
        .await?;

        // If the update did not find a matching row, insert the user
        if let Some(record) = rec {
            Ok(record.id)
        } else {
            let rec = sqlx::query!(
                r#"
                INSERT INTO users (login, permanent_bonus)
                VALUES ($1, $2)
                ON CONFLICT (login) DO NOTHING
                RETURNING id
                "#,
                user,
                percent as i32
            )
            .fetch_one(&self.0)
            .await?;

            Ok(rec.id)
        }
    }

    pub async fn update_user_full_name(&self, user: &str, full_name: &str) -> anyhow::Result<()> {
        sqlx::query!(
            r#"
            UPDATE users
            SET full_name = $2
            WHERE login = $1
            "#,
            user,
            full_name
        )
        .execute(&self.0)
        .await?;
        Ok(())
    }

    pub async fn update_organization_full_name(
        &self,
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
        .execute(&self.0)
        .await?;
        Ok(())
    }

    pub async fn upsert_organization(&self, name: &str) -> anyhow::Result<i32> {
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
        .fetch_optional(&self.0)
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
            .fetch_one(&self.0)
            .await?;

            Ok(rec.id)
        }
    }

    pub async fn upsert_repo(&self, organization_id: i32, name: &str) -> anyhow::Result<i32> {
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
        .fetch_optional(&self.0)
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
            .fetch_one(&self.0)
            .await?;

            Ok(rec.id)
        }
    }

    pub async fn upsert_pull_request(
        &self,
        repo_id: i32,
        number: i32,
        author_id: i32,
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
            SET merged_at = $3, executed = $4, score = $5, rating = $6, permanent_bonus = $7, streak_bonus = $8
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
        )
        .fetch_optional(&self.0)
        .await?;

        // If the update did not find a matching row, insert the pull request
        if let Some(record) = rec {
            Ok(record.id)
        } else {
            let rec = sqlx::query!(
                r#"
                INSERT INTO pull_requests (repo_id, number, author_id, created_at, merged_at, executed, score, rating, permanent_bonus, streak_bonus)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                ON CONFLICT (repo_id, number) DO NOTHING
                RETURNING id
                "#,
                repo_id,
                number,
                author_id,
                Some(created_at),
                merged_at,
                executed,
                score.map(|s| s as i32),
                rating as i32,
                permanent_bonus as i32,
                streak_bonus as i32,
            )
            .fetch_one(&self.0)
            .await?;

            Ok(rec.id)
        }
    }

    pub async fn upsert_user_period_data(
        &self,
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
        .fetch_optional(&self.0)
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
            .execute(&self.0)
            .await?;
        }
        Ok(())
    }

    pub async fn upsert_streak_user_data(
        &self,
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
        .fetch_optional(&self.0)
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
            .execute(&self.0)
            .await?;
        }
        Ok(())
    }

    pub async fn update_repo_metadata(
        &self,
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
        .execute(&self.0)
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

        let user = UserRecord {
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
        let records = sqlx::query_file_as!(
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
        let records = sqlx::query_file_as_unchecked!(
            RepoLeaderboardRecord,
            "./sql/get_repo_leaderboard.sql",
            limit,
            offset
        )
        .fetch_all(&self.0)
        .await?;

        // TODO: Replace this with a single query
        let total_count = sqlx::query!(
            r#"SELECT COUNT(DISTINCT(r.organization_id, r.id)) as id
            FROM repos r
            "#,
        )
        .fetch_one(&self.0)
        .await?;

        Ok((records, total_count.id.unwrap_or_default() as u64))
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
        WHERE pr.created_at >= (now() - INTERVAL '1 MONTH')
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

    pub async fn get_repos(&self) -> anyhow::Result<Vec<RepoRecord>> {
        let rec = sqlx::query_file_as!(RepoRecord, "./sql/get_repos.sql")
            .fetch_all(&self.0)
            .await?;

        Ok(rec)
    }

    pub async fn get_users(&self) -> anyhow::Result<Vec<User>> {
        let rec = sqlx::query_as!(
            User,
            r#"
            SELECT login, full_name
            FROM users"#
        )
        .fetch_all(&self.0)
        .await?;

        Ok(rec)
    }

    pub async fn get_organizations(&self) -> anyhow::Result<Vec<User>> {
        let rec = sqlx::query_as!(
            User,
            r#"
            SELECT login, full_name
            FROM organizations"#
        )
        .fetch_all(&self.0)
        .await?;

        Ok(rec)
    }

    pub async fn clear_prs(&self) -> anyhow::Result<()> {
        sqlx::query!("DELETE FROM pull_requests")
            .execute(&self.0)
            .await?;
        Ok(())
    }

    pub async fn get_user_id(&self, name: &str) -> anyhow::Result<i32> {
        let rec = sqlx::query!(
            r#"
            SELECT id
            FROM users
            WHERE login = $1
            "#,
            name
        )
        .fetch_one(&self.0)
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
        username: &str,
        image_base64: &str,
    ) -> anyhow::Result<()> {
        // First try to update the user
        let rec = sqlx::query!(
            r#"
                UPDATE user_cached_metadata
                SET image_base64 = $2, load_time = now()
                WHERE user_id = (SELECT id FROM users WHERE login = $1)
                RETURNING user_id
                "#,
            username,
            image_base64
        )
        .fetch_optional(&self.0)
        .await?;

        // If the update did not find a matching row, insert the user
        if rec.is_none() {
            sqlx::query!(
                r#"
                INSERT INTO user_cached_metadata (user_id, image_base64, load_time)
                VALUES ((SELECT id FROM users WHERE login = $1), $2, now())
                ON CONFLICT (user_id) DO NOTHING
                "#,
                username,
                image_base64
            )
            .execute(&self.0)
            .await?;
        }

        Ok(())
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
