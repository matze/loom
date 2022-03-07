use axum::body;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

#[derive(thiserror::Error, Debug)]
pub(crate) enum Error {
    #[error("Task failed to execute: {0}")]
    JoinError(#[from] tokio::task::JoinError),
    #[error("Time format problem: {0}")]
    TimeError(#[from] time::error::Format),
    #[error("Database problem: {0}")]
    SqlError(#[from] sqlx::Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(body::boxed(body::Full::from(format!("Error: {}", self))))
            .unwrap()
    }
}
