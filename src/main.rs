use askama::Template;
use axum::extract::Extension;
use axum::response::Json;
use axum::routing::get;
use std::sync::Arc;
use time::macros::format_description;
use tower_http::trace::TraceLayer;

mod db;
mod error;
mod models;
mod serve;

use error::Error;

struct State {
    db: db::Database,
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
        .route("/static/*path", get(serve::static_data))
        .layer(Extension(state))
        .layer(TraceLayer::new_for_http());

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 3000));

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
