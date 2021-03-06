use axum::body;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Task failed to execute: {0}")]
    TaskJoin(#[from] tokio::task::JoinError),
    #[error("Time format problem: {0}")]
    TimeFormatting(#[from] time::error::Format),
    #[error("Database problem: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Wrong credentials")]
    WrongCredentials,
    #[error("JWT creation problem: {0}")]
    TokenGeneration(#[from] jsonwebtoken::errors::Error),
    #[error("Invalid token")]
    InvalidToken,
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let status = match self {
            Error::WrongCredentials | Error::InvalidToken => StatusCode::UNAUTHORIZED,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        Response::builder()
            .status(status)
            .body(body::boxed(body::Full::from(format!("Error: {}", self))))
            .unwrap()
    }
}
