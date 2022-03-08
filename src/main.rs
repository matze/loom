use askama::Template;
use axum::body::{self, Empty, Full};
use axum::extract::{Extension, Path};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::get;
use include_dir::{include_dir, Dir};
use std::sync::Arc;
use time::macros::format_description;
use tower_http::trace::TraceLayer;

mod db;
mod error;
mod models;

use error::Error;

struct State {
    db: db::Database,
}

static STATIC_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/static");

async fn static_path(Path(path): Path<String>) -> impl IntoResponse {
    let path = path.trim_start_matches('/');
    let mime_type = mime_guess::from_path(path).first_or_text_plain();

    match STATIC_DIR.get_file(path) {
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(body::boxed(Empty::new()))
            .unwrap(),
        Some(file) => Response::builder()
            .status(StatusCode::OK)
            .header(
                header::CONTENT_TYPE,
                HeaderValue::from_str(mime_type.as_ref()).unwrap(),
            )
            .body(body::boxed(Full::from(file.contents())))
            .unwrap(),
    }
}

#[derive(Template)]
#[template(path = "index.html")]
struct HtmlTemplate {}

async fn index() -> HtmlTemplate {
    HtmlTemplate {}
}

async fn get_current(
    Extension(state): Extension<Arc<State>>,
) -> Result<Json<models::Current>, Error> {
    Ok(Json(state.db.current().await?))
}

async fn post_current(
    Extension(state): Extension<Arc<State>>,
    Json(payload): Json<models::Current>,
) -> Result<(), Error> {
    let format = format_description!("[year]-[month]-[day]");
    let date = time::OffsetDateTime::now_utc().format(&format)?;
    state.db.upsert(date, payload.weight).await
}

async fn get_series(
    Extension(state): Extension<Arc<State>>,
) -> Result<Json<models::RawAndAveragedSeries>, Error> {
    let raw = state.db.raw_series().await?;

    let raw_and_averaged = tokio::task::spawn_blocking(move || {
        let average = models::AveragedSeries::from(&raw);
        models::RawAndAveragedSeries { raw, average }
    })
    .await?;

    Ok(Json(raw_and_averaged))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let db = db::Database::new().await?;
    let state = Arc::new(State { db });

    let app = axum::Router::new()
        .route("/", get(index))
        .route("/api/current", get(get_current).post(post_current))
        .route("/api/series", get(get_series))
        .route("/static/*path", get(static_path))
        .layer(Extension(state))
        .layer(TraceLayer::new_for_http());

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 3000));

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
