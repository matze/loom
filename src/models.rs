use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(FromRow, Serialize, Deserialize, Debug)]
pub struct Current {
    pub weight: f64,
}

#[derive(Serialize)]
pub struct Series {
    pub dates: Vec<String>,
    pub weights: Vec<f64>,
}

#[derive(Serialize)]
pub struct RawAndAveragedSeries {
    pub raw: Series,
    pub average: Series,
}
