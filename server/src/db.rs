use rocket::{
    fairing::{self, AdHoc},
    Build, Rocket,
};
use rocket_db_pools::Database;
use shared::{StreakId, StreakUserData, TimePeriodString, User, UserPeriodData};
use sqlx::PgPool;
use tracing::instrument;

#[derive(Database, Clone, Debug)]
#[database("race-of-sloths")]
pub struct DB(PgPool);

impl DB {
    #[instrument(skip(self))]
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

    #[instrument(skip(self))]
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

    #[instrument(skip(self))]
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

    #[instrument(skip(self))]
    pub async fn get_user(
        &self,
        name: &str,
        time_string: Option<&str>,
    ) -> anyhow::Result<Option<User>> {
        let user_rec: i32 = match sqlx::query!("SELECT id, name FROM users WHERE name = $1", name)
            .fetch_optional(&self.0)
            .await?
        {
            Some(rec) => rec.id,
            None => return Ok(None),
        };
        let time_string = time_string
            .map(|s| s.to_string())
            .unwrap_or_else(|| "all-time".to_string());

        let period_data_recs = sqlx::query!(
            r#"
                SELECT period_type, total_score, executed_prs, largest_score, prs_opened, prs_merged
                FROM user_period_data 
                WHERE user_id = $1 AND period_type = $2
                "#,
            user_rec,
            time_string
        )
        .fetch_all(&self.0)
        .await?;

        let streak_recs = sqlx::query!(
            r#"
                SELECT streak_id as "streak_id: i32", amount, best, latest_time_string
                FROM streak_user_data
                WHERE user_id = $1
                "#,
            user_rec
        )
        .fetch_all(&self.0)
        .await?;

        let user = User {
            name: name.to_string(),
            period_data: period_data_recs
                .into_iter()
                .map(|rec| {
                    (
                        rec.period_type,
                        UserPeriodData {
                            total_score: rec.total_score as u32,
                            executed_prs: rec.executed_prs as u32,
                            largest_score: rec.largest_score as u32,
                            prs_opened: rec.prs_opened as u32,
                            prs_merged: rec.prs_merged as u32,
                        },
                    )
                })
                .collect(),
            streaks: streak_recs
                .into_iter()
                .map(|rec| {
                    (
                        rec.streak_id as StreakId,
                        StreakUserData {
                            amount: rec.amount as u32,
                            best: rec.best as u32,
                            latest_time_string: rec.latest_time_string,
                        },
                    )
                })
                .collect(),
        };

        Ok(Some(user))
    }
}

async fn run_migrations(rocket: Rocket<Build>) -> fairing::Result {
    match DB::fetch(&rocket) {
        Some(db) => match sqlx::migrate!("./migrations").run(&**db).await {
            Ok(_) => Ok(rocket),
            Err(e) => {
                tracing::error!("Failed to initialize SQLx database: {}", e);
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
