use axum::body;
use axum::extract::{Extension, Path};
use axum::headers::{HeaderMap, HeaderValue};
use axum::http::header::CONTENT_TYPE;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{AddExtensionLayer, Router};
use include_dir::{include_dir, Dir};
use std::sync::Arc;
use tracing::{instrument, warn};

mod db;

static DIST_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../app/dist");

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Database problem")]
    Sql(#[from] sqlx::Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(body::boxed(body::Full::from(format!("Error: {}", self))))
            .unwrap()
    }
}

#[derive(Debug)]
struct State {
    db: db::Database,
}

fn insert_header_from_extension(map: &mut HeaderMap, ext: &str) {
    match ext {
        "wasm" => {
            map.insert(CONTENT_TYPE, HeaderValue::from_static("application/wasm"));
        }
        "js" => {
            map.insert(
                CONTENT_TYPE,
                HeaderValue::from_static("application/javascript"),
            );
        }
        "html" => {
            map.insert(
                CONTENT_TYPE,
                HeaderValue::from_static("text/html; charset=utf-8"),
            );
        }
        _ => {}
    }
}

#[instrument]
async fn get_static(Path(path): Path<String>) -> (StatusCode, HeaderMap, Vec<u8>) {
    let mut headers = HeaderMap::new();

    match DIST_DIR.get_file(&path) {
        Some(file) => {
            file.path()
                .extension()
                .map(|e| e.to_str())
                .flatten()
                .map(|e| insert_header_from_extension(&mut headers, e));

            (StatusCode::OK, headers, file.contents().to_vec())
        }
        None => {
            warn!("file not found");
            (StatusCode::NOT_FOUND, headers, vec![])
        }
    }
}

#[instrument]
async fn post_begin(Extension(state): Extension<Arc<State>>) -> Result<(), Error> {
    state.db.update_begin().await?;
    Ok(())
}

#[instrument]
async fn post_end(Extension(state): Extension<Arc<State>>) -> Result<(), Error> {
    state.db.update_end().await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv()?;
    tracing_subscriber::fmt::init();

    let state = State {
        db: db::Database::new().await?,
    };

    let app = Router::new()
        .route(
            "/",
            get(|| async { get_static(Path("index.html".into())).await }),
        )
        .route("/:key", get(get_static))
        .route("/api/begin", post(post_begin))
        .route("/api/end", post(post_end))
        .layer(AddExtensionLayer::new(Arc::new(state)));

    axum::Server::bind(&"0.0.0.0:3000".parse()?)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
