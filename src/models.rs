use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::convert::From;

#[derive(FromRow, Serialize, Deserialize, Debug)]
pub struct Current {
    pub weight: f64,
}

#[derive(Serialize)]
pub struct RawSeries {
    pub dates: Vec<String>,
    pub weights: Vec<f64>,
}

#[derive(Serialize)]
pub struct AveragedSeries {
    pub dates: Vec<String>,
    pub weights: Vec<f64>,
}

impl From<&RawSeries> for AveragedSeries {
    fn from(raw: &RawSeries) -> Self {
        let weights = raw
            .weights
            .windows(7)
            .map(|w| w.iter().sum::<f64>() / 7.0)
            .collect();

        Self {
            dates: raw.dates[6..].to_vec(),
            weights,
        }
    }
}

#[derive(Serialize)]
pub struct RawAndAveragedSeries {
    pub raw: RawSeries,
    pub average: AveragedSeries,
}
