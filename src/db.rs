use crate::error::Error;
use crate::models::{Current, RawSeries};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::{ConnectOptions, FromRow};
use std::str::FromStr;

pub(crate) struct Database {
    pool: SqlitePool,
}

#[derive(FromRow, Debug)]
struct SeriesRow {
    date: String,
    weight: f64,
}

impl Database {
    pub async fn new() -> Result<Self, Error> {
        let db_options = SqliteConnectOptions::from_str("state.db")?
            .create_if_missing(true)
            .disable_statement_logging()
            .to_owned();

        let pool = SqlitePoolOptions::new().connect_with(db_options).await?;

        Ok(Self { pool })
    }

    pub async fn current(&self) -> Result<Current, Error> {
        Ok(
            sqlx::query_as::<_, Current>("SELECT weight, MAX(date) FROM weights LIMIT 1")
                .fetch_one(&self.pool)
                .await?,
        )
    }

    pub async fn upsert(&self, date: String, weight: f64) -> Result<(), Error> {
        sqlx::query("INSERT INTO weights (date, weight) VALUES (?, ?) ON CONFLICT(date) DO UPDATE SET weight=excluded.weight")
            .bind(date)
            .bind(weight)
            .execute(&self.pool).await?;

        Ok(())
    }

    pub async fn raw_series(&self) -> Result<RawSeries, Error> {
        let (dates, weights) =
            sqlx::query_as::<_, SeriesRow>("SELECT date, weight FROM weights ORDER BY date")
                .fetch_all(&self.pool)
                .await?
                .into_iter()
                .map(|row| (row.date, row.weight))
                .unzip();

        Ok(RawSeries { dates, weights })
    }
}
