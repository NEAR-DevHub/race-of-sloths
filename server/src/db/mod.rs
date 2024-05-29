use rocket::{
    fairing::{self, AdHoc},
    Build, Rocket,
};
use rocket_db_pools::Database;
use shared::{StreakUserData, TimePeriodString, User, UserPeriodData};
use sqlx::PgPool;

#[derive(Database, Clone, Debug)]
#[database("race-of-sloths")]
pub struct DB(PgPool);

pub mod types;

use types::LeaderboardRecord;

use self::types::{StreakRecord, UserPeriodRecord, UserRecord};

impl DB {
    pub async fn upsert_user(&self, user: &User) -> anyhow::Result<i32> {
        let rec = sqlx::query!(
            r#"
        INSERT INTO users (name)
        VALUES ($1)
        ON CONFLICT (name) DO UPDATE
        SET name = EXCLUDED.name
        RETURNING id
        "#,
            user.name
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

    pub async fn get_user(&self, name: &str) -> anyhow::Result<Option<UserRecord>> {
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

        let user = UserRecord {
            name: name.to_string(),
            period_data: period_data_recs,
            streaks: streak_recs,
        };

        Ok(Some(user))
    }

    pub async fn get_leaderboard(
        &self,
        period: &str,
        page: i64,
        limit: i64,
    ) -> anyhow::Result<Vec<LeaderboardRecord>> {
        Ok(sqlx::query_as!(LeaderboardRecord,r#"
                                SELECT users.name, period_type, total_score, executed_prs, largest_score, prs_opened, prs_merged
                                FROM user_period_data 
                                JOIN users ON users.id = user_period_data.user_id
                                WHERE period_type = $1
                                ORDER BY total_score DESC
                                LIMIT $2 OFFSET $3
                                "#,period,limit,page*limit).fetch_all(&self.0,).await? )
    }

    pub async fn get_leaderboard_place(
        &self,
        period: &str,
        name: &str,
    ) -> anyhow::Result<Option<i64>> {
        let rec = sqlx::query!(
            r#"
        SELECT rownum as place
        FROM (SELECT user_id, RANK() OVER (ORDER BY total_score DESC) as rownum
              FROM user_period_data
              WHERE period_type = $1) as ranked
        JOIN users ON users.id = ranked.user_id
        WHERE users.name = $2
        "#,
            period,
            name
        )
        .fetch_one(&self.0)
        .await?;

        Ok(rec.place)
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
