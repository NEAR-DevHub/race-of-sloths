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

use self::types::{RepoRecord, StreakRecord, UserContributionRecord, UserPeriodRecord, UserRecord};

impl DB {
    pub async fn upsert_user(&self, user: &str) -> anyhow::Result<i32> {
        let rec = sqlx::query!(
            r#"
        INSERT INTO users (name)
        VALUES ($1)
        ON CONFLICT (name) DO UPDATE
        SET name = EXCLUDED.name
        RETURNING id
        "#,
            user
        )
        .fetch_one(&self.0)
        .await?;

        Ok(rec.id)
    }

    pub async fn upsert_organization(&self, name: &str) -> anyhow::Result<i32> {
        let rec = sqlx::query!(
            r#"
        INSERT INTO organizations (name)
        VALUES ($1)
        ON CONFLICT (name) DO UPDATE
        SET name = EXCLUDED.name
        RETURNING id
        "#,
            name
        )
        .fetch_one(&self.0)
        .await?;

        Ok(rec.id)
    }

    pub async fn upsert_repo(&self, organization_id: i32, name: &str) -> anyhow::Result<i32> {
        let rec = sqlx::query!(
            r#"
        INSERT INTO repos (organization_id, name)
        VALUES ($1, $2)
        ON CONFLICT (organization_id, name) DO UPDATE
        SET name = EXCLUDED.name
        RETURNING id
        "#,
            organization_id,
            name
        )
        .fetch_one(&self.0)
        .await?;

        Ok(rec.id)
    }

    pub async fn upsert_pull_request(
        &self,
        repo_id: i32,
        number: i32,
        author_id: i32,
        created_at: chrono::NaiveDateTime,
        merged_at: Option<chrono::NaiveDateTime>,
        score: Option<u32>,
        executed: bool,
    ) -> anyhow::Result<i32> {
        let rec = sqlx::query!(
            r#"
        INSERT INTO pull_requests (repo_id, number, author_id, created_at, merged_at, executed, score)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT (repo_id, number) DO UPDATE
        SET 
            merged_at = EXCLUDED.merged_at,
            executed = EXCLUDED.executed
        RETURNING id
        "#,
            repo_id,
            number,
            author_id,
            Some(created_at),
            merged_at,
            executed,
            score.map(|s| s as i32),
        )
        .fetch_one(&self.0)
        .await?;

        Ok(rec.id)
    }

    pub async fn upsert_user_period_data(
        &self,
        period: TimePeriodString,
        data: &UserPeriodData,
        user_id: i32,
    ) -> anyhow::Result<()> {
        sqlx::query!(
        r#"
        INSERT INTO user_period_data (user_id, period_type, total_score, executed_prs, largest_score, prs_opened, prs_merged)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT (user_id, period_type) DO UPDATE
        SET total_score = EXCLUDED.total_score,
            executed_prs = EXCLUDED.executed_prs,
            largest_score = EXCLUDED.largest_score,
            prs_opened = EXCLUDED.prs_opened,
            prs_merged = EXCLUDED.prs_merged
        "#,
        user_id, period, data.total_score as i32, data.executed_prs as i32, data.largest_score as i32, data.prs_opened as i32, data.prs_merged as i32
    )
    .execute(&self.0)
    .await?;
        Ok(())
    }

    pub async fn upsert_streak_user_data(
        &self,
        data: &StreakUserData,
        streak_id: i32,
        user_id: i32,
    ) -> anyhow::Result<()> {
        sqlx::query!(
            r#"
        INSERT INTO streak_user_data (user_id, streak_id, amount, best, latest_time_string)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (user_id, streak_id) DO UPDATE
        SET amount = EXCLUDED.amount,
            best = EXCLUDED.best,
            latest_time_string = EXCLUDED.latest_time_string
        "#,
            user_id,
            streak_id,
            data.amount as i32,
            data.best as i32,
            data.latest_time_string
        )
        .execute(&self.0)
        .await?;
        Ok(())
    }

    pub async fn get_user(
        &self,
        name: &str,
        place_strings: &[String],
    ) -> anyhow::Result<Option<UserRecord>> {
        let user_rec: i32 = match sqlx::query!("SELECT id, name FROM users WHERE name = $1", name)
            .fetch_optional(&self.0)
            .await?
        {
            Some(rec) => rec.id,
            None => return Ok(None),
        };

        let period_data_recs: Vec<UserPeriodRecord> = sqlx::query_as!(
            UserPeriodRecord,
            r#"
                SELECT period_type, total_score, executed_prs, largest_score, prs_opened, prs_merged
                FROM user_period_data 
                WHERE user_id = $1
                "#,
            user_rec,
        )
        .fetch_all(&self.0)
        .await?;

        let streak_recs: Vec<StreakRecord> = sqlx::query_as!(
            StreakRecord,
            r#"
                SELECT streak_id, amount, best, latest_time_string
                FROM streak_user_data
                WHERE user_id = $1
                "#,
            user_rec
        )
        .fetch_all(&self.0)
        .await?;

        let mut leaderboard_places = Vec::with_capacity(place_strings.len());
        for place in place_strings {
            let record = self.get_leaderboard_place(place, user_rec).await?;
            leaderboard_places.push((place.clone(), record.unwrap_or_default() as u32));
        }

        let user = UserRecord {
            name: name.to_string(),
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
            .fetch_one(&self.0)
            .await?;

        Ok(rec.place)
    }

    pub async fn get_repo_leaderboard(
        &self,
        page: i64,
        limit: i64,
    ) -> anyhow::Result<(Vec<RepoRecord>, u64)> {
        let offset = page * limit;
        // COALESCE is used to return 0 if there are no PRs for a repo
        // But sqlx still thinks that it's NONE
        let records = sqlx::query_file_as_unchecked!(
            RepoRecord,
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
            WHERE users.name = $1
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
        SELECT users.name, SUM(pr.score) as total_score
        FROM organizations o
        JOIN repos r ON r.organization_id = o.id
        JOIN pull_requests pr ON pr.repo_id = r.id
        JOIN users ON pr.author_id = users.id
        WHERE pr.created_at >= (now() - INTERVAL '1 MONTH')
        AND r.name = $1
        AND o.name = $2
        GROUP BY users.name
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
            .map(|r| (r.name, r.total_score.unwrap_or_default()))
            .collect())
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
