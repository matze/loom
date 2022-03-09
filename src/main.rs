use askama::Template;
use axum::extract::{Extension, FromRequest, RequestParts, TypedHeader};
use axum::headers::authorization::Bearer;
use axum::headers::Authorization;
use axum::http::Request;
use axum::response::Json;
use axum::routing::{get, post};
use jsonwebtoken as jwt;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::convert::From;
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

struct Keys {
    encoding: jwt::EncodingKey,
    decoding: jwt::DecodingKey,
}

impl From<&[u8]> for Keys {
    fn from(secret: &[u8]) -> Self {
        Self {
            encoding: jwt::EncodingKey::from_secret(secret),
            decoding: jwt::DecodingKey::from_secret(secret),
        }
    }
}

static KEYS: Lazy<Keys> = Lazy::new(|| {
    let secret = std::env::var("LOOM_JWT_SECRET").expect("LOOM_JWT_SECRET must be set");
    secret.as_bytes().into()
});

static USER: Lazy<String> =
    Lazy::new(|| std::env::var("LOOM_USER").expect("LOOM_USER must be set"));

static SECRET: Lazy<String> =
    Lazy::new(|| std::env::var("LOOM_SECRET").expect("LOOM_SECRET must be set"));

#[derive(Template)]
#[template(path = "index.html")]
struct HtmlTemplate {
    token: Option<String>,
}

async fn index(request: Request<axum::body::Body>) -> Result<HtmlTemplate, Error> {
    let mut parts = RequestParts::new(request);
    let result: Result<Token, Error> = FromRequest::from_request(&mut parts).await;
    let token = result.ok().map(|token| token.0);

    Ok(HtmlTemplate { token })
}

#[derive(Deserialize)]
struct AuthorizePayload {
    user: String,
    secret: String,
}

#[derive(Serialize, Deserialize)]
struct Claims {
    exp: usize,
    iss: String,
}

struct Token(String);

#[axum::async_trait]
impl<B> FromRequest<B> for Token
where
    B: Send,
{
    type Rejection = Error;

    async fn from_request(parts: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(bearer)) =
            TypedHeader::<Authorization<Bearer>>::from_request(parts)
                .await
                .map_err(|_| Error::InvalidToken)?;

        let _ = jwt::decode::<Claims>(bearer.token(), &KEYS.decoding, &jwt::Validation::default())
            .map_err(|_| Error::InvalidToken)?;

        Ok(Token(bearer.token().to_string()))
    }
}

async fn authorize(Json(payload): Json<AuthorizePayload>) -> Result<String, Error> {
    if payload.user.is_empty() || payload.secret.is_empty() {
        return Err(Error::MissingCredentials);
    }

    if payload.user != USER.as_str() || payload.secret != SECRET.as_str() {
        return Err(Error::WrongCredentials);
    }

    let claims = Claims {
        exp: 2000000000,
        iss: "foobar".to_string(),
    };

    let token = tokio::task::spawn_blocking(move || {
        Ok::<String, Error>(jwt::encode(
            &jwt::Header::default(),
            &claims,
            &KEYS.encoding,
        )?)
    })
    .await??;

    Ok(token)
}

async fn get_current(
    _: Token,
    Extension(state): Extension<Arc<State>>,
) -> Result<Json<models::Current>, Error> {
    Ok(Json(state.db.current().await?))
}

async fn post_current(
    _: Token,
    Extension(state): Extension<Arc<State>>,
    Json(payload): Json<models::Current>,
) -> Result<(), Error> {
    let format = format_description!("[year]-[month]-[day]");
    let date = time::OffsetDateTime::now_utc().format(&format)?;
    state.db.upsert(date, payload.weight).await
}

async fn get_series(
    _: Token,
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
        .route("/api/authorize", post(authorize))
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
