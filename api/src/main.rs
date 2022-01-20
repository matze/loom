use anyhow::Result;
use axum::extract::Path;
use axum::headers::{HeaderMap, HeaderValue};
use axum::http::header::CONTENT_TYPE;
use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use include_dir::{include_dir, Dir};
use tracing::{instrument, warn};

static DIST_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../app/dist");

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

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route(
            "/",
            get(|| async { get_static(Path("index.html".into())).await }),
        )
        .route("/:key", get(get_static));

    axum::Server::bind(&"0.0.0.0:3000".parse()?)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
