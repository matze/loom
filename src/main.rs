use askama::Template;
use axum::body::{self, Empty, Full};
use axum::extract::{Extension, Path};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::get;
use axum_extra::routing::{RouterExt, TypedPath};
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

#[derive(TypedPath)]
#[typed_path("/")]
struct Index;

async fn index(_: Index) -> HtmlTemplate {
    HtmlTemplate {}
}

#[derive(TypedPath)]
#[typed_path("/api/current")]
struct CurrentPath;

async fn get_current(
    _: CurrentPath,
    Extension(state): Extension<Arc<State>>,
) -> Result<Json<models::Current>, Error> {
    Ok(Json(state.db.current().await?))
}

async fn post_current(
    _: CurrentPath,
    Extension(state): Extension<Arc<State>>,
    Json(payload): Json<models::Current>,
) -> Result<(), Error> {
    let format = format_description!("[year]-[month]-[day]");
    let date = time::OffsetDateTime::now_utc().format(&format)?;
    state.db.upsert(date, payload.weight).await
}

#[derive(TypedPath)]
#[typed_path("/api/series")]
struct SeriesPath;

async fn get_series(
    _: SeriesPath,
    Extension(state): Extension<Arc<State>>,
) -> Result<Json<models::RawAndAveragedSeries>, Error> {
    let raw = state.db.raw_series().await?;

    // TODO: run this in a separate thread
    let average = models::AveragedSeries::from(&raw);

    Ok(Json(models::RawAndAveragedSeries { raw, average }))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let db = db::Database::new().await?;
    let state = Arc::new(State { db });

    let app = axum::Router::new()
        .typed_get(index)
        .typed_get(get_current)
        .typed_post(post_current)
        .typed_get(get_series)
        .route("/static/*path", get(static_path))
        .layer(Extension(state))
        .layer(TraceLayer::new_for_http());

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 3000));

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
