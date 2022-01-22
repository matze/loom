use crate::Error;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::{ConnectOptions, FromRow};
use std::env;
use std::str::FromStr;

#[derive(FromRow, Serialize, Deserialize)]
pub struct Day {
    id: i64,
    pub date: chrono::NaiveDate,
    pub begin_eat: Option<chrono::NaiveTime>,
    pub end_eat: Option<chrono::NaiveTime>,
}

#[derive(Debug)]
pub struct Database {
    pool: SqlitePool,
}

fn naive_now_date_and_time() -> (chrono::NaiveDate, chrono::NaiveTime) {
    let now = chrono::Utc::now();
    (now.date().naive_utc(), now.time())
}

impl Database {
    pub async fn new() -> Result<Self, Error> {
        let url = env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite::memory:".to_string());

        let options = SqliteConnectOptions::from_str(&url)?
            .disable_statement_logging()
            .to_owned();

        let pool = SqlitePoolOptions::new().connect_with(options).await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS days (
            id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
            date datetime UNIQUE NOT NULL,
            begin_eat datetime,
            end_eat datetime
        );",
        )
        .execute(&pool)
        .await?;

        Ok(Self { pool })
    }

    pub async fn update_begin(&self) -> Result<(), Error> {
        let (today, time) = naive_now_date_and_time();

        sqlx::query!(
            "INSERT INTO days(date, begin_eat)
            VALUES(?, ?)
            ON CONFLICT(date) DO UPDATE SET begin_eat=excluded.begin_eat;",
            today,
            time
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_end(&self) -> Result<(), Error> {
        let (today, time) = naive_now_date_and_time();

        sqlx::query!(
            "INSERT INTO days(date, end_eat)
            VALUES(?, ?)
            ON CONFLICT(date) DO UPDATE SET end_eat=excluded.end_eat;",
            today,
            time
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
