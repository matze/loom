use askama::Template;
use axum::body::{self, Empty, Full};
use axum::extract::{Extension, Path};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::get;
use axum_extra::routing::{RouterExt, TypedPath};
use include_dir::{include_dir, Dir};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::{ConnectOptions, FromRow};
use std::str::FromStr;
use std::sync::Arc;
use time::macros::format_description;
use tower_http::trace::TraceLayer;

struct State {
    pool: SqlitePool,
}

#[derive(thiserror::Error, Debug)]
enum Error {
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

#[derive(FromRow, Serialize, Deserialize, Debug)]
struct CurrentPayload {
    weight: f64,
}

#[derive(TypedPath)]
#[typed_path("/api/current")]
struct CurrentPath;

async fn get_current(
    _: CurrentPath,
    Extension(state): Extension<Arc<State>>,
) -> Result<Json<CurrentPayload>, Error> {
    let result =
        sqlx::query_as::<_, CurrentPayload>("SELECT weight, MAX(date) FROM weights LIMIT 1")
            .fetch_one(&state.pool)
            .await?;

    Ok(Json(result))
}

async fn post_current(
    _: CurrentPath,
    Extension(state): Extension<Arc<State>>,
    Json(payload): Json<CurrentPayload>,
) -> Result<(), Error> {
    let format = format_description!("[year]-[month]-[day]");
    let date = time::OffsetDateTime::now_utc().format(&format)?;

    sqlx::query("INSERT INTO weights (date, weight) VALUES (?, ?) ON CONFLICT(date) DO UPDATE SET weight=excluded.weight")
        .bind(date)
        .bind(payload.weight)
        .execute(&state.pool).await?;

    Ok(())
}

#[derive(TypedPath)]
#[typed_path("/api/series")]
struct SeriesPath;

#[derive(FromRow, Debug)]
struct SeriesRow {
    date: String,
    weight: f64,
}

#[derive(Serialize)]
struct Series {
    pub dates: Vec<String>,
    pub weights: Vec<f64>,
}

#[derive(Serialize)]
struct SeriesResponse {
    pub raw: Series,
    pub average: Series,
}

async fn get_series(
    _: SeriesPath,
    Extension(state): Extension<Arc<State>>,
) -> Result<Json<SeriesResponse>, Error> {
    let (dates, weights) =
        sqlx::query_as::<_, SeriesRow>("SELECT date, weight FROM weights ORDER BY date")
            .fetch_all(&state.pool)
            .await?
            .into_iter()
            .map(|row| (row.date, row.weight))
            .unzip();

    let raw = Series { dates, weights };

    let weights = raw
        .weights
        .windows(7)
        .map(|w| w.iter().sum::<f64>() / 7.0)
        .collect();

    let average = Series {
        dates: raw.dates[6..].to_vec(),
        weights,
    };

    Ok(Json(SeriesResponse { raw, average }))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let db_options = SqliteConnectOptions::from_str(&"state.db")?
        .create_if_missing(true)
        .disable_statement_logging()
        .to_owned();

    let pool = SqlitePoolOptions::new().connect_with(db_options).await?;

    let state = Arc::new(State { pool });

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
