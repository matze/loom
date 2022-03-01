use askama::Template;
use axum::body::{self, Empty, Full};
use axum::extract::Path;
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response, Json};
use axum::routing::get;
use axum_extra::routing::{RouterExt, TypedPath};
use include_dir::{include_dir, Dir};
use serde::{Serialize, Deserialize};
use tower_http::trace::TraceLayer;
use tracing::info;

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

#[derive(Serialize, Deserialize, Debug)]
struct CurrentPayload {
    point: f64,
}

#[derive(TypedPath)]
#[typed_path("/api/current")]
struct CurrentPath;

async fn get_current(_: CurrentPath) -> Json<CurrentPayload> {
    Json(CurrentPayload { point: 64.1 })
}

async fn post_current(_: CurrentPath, Json(payload): Json<CurrentPayload>) {
    info!("{:?}", payload);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let app = axum::Router::new()
        .typed_get(index)
        .typed_get(get_current)
        .typed_post(post_current)
        .route("/static/*path", get(static_path))
        .layer(TraceLayer::new_for_http());

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 3000));

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
