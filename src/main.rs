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
use sqlx::FromRow;
use std::str::FromStr;
use std::sync::Arc;
use tower_http::trace::TraceLayer;
use tracing::info;

struct State {
    pool: SqlitePool,
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
) -> Json<CurrentPayload> {
    let result =
        sqlx::query_as::<_, CurrentPayload>("SELECT weight, MAX(date) FROM weights LIMIT 1")
            .fetch_one(&state.pool)
            .await
            .unwrap();

    Json(result)
}

async fn post_current(_: CurrentPath, Json(payload): Json<CurrentPayload>) {
    info!("{:?}", payload);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let db_options = SqliteConnectOptions::from_str(&"state.db")?
        .create_if_missing(true)
        .to_owned();

    let pool = SqlitePoolOptions::new().connect_with(db_options).await?;

    let state = Arc::new(State { pool });

    let app = axum::Router::new()
        .typed_get(index)
        .typed_get(get_current)
        .typed_post(post_current)
        .route("/static/*path", get(static_path))
        .layer(Extension(state))
        .layer(TraceLayer::new_for_http());

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 3000));

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
