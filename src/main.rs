use askama::Template;
use axum::extract::{Extension, Form};
use axum::response::Json;
use axum::routing::{get, post};
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::convert::From;
use std::sync::Arc;
use time::macros::format_description;
use tower_cookies::{Cookie, CookieManagerLayer, Cookies};
use tower_http::trace::TraceLayer;

mod auth;
mod db;
mod error;
mod models;
mod serve;

use auth::Token;
use error::Error;

static USER: Lazy<String> =
    Lazy::new(|| std::env::var("LOOM_USER").expect("LOOM_USER must be set"));

static SECRET: Lazy<String> =
    Lazy::new(|| std::env::var("LOOM_SECRET").expect("LOOM_SECRET must be set"));

struct State {
    db: db::Database,
}

#[derive(Template)]
#[template(path = "index.html")]
struct HtmlTemplate {
    token: Option<Token>,
}

async fn index(cookies: Cookies) -> Result<HtmlTemplate, Error> {
    let token = cookies
        .get("token")
        .map(|cookie| cookie.value().try_into())
        .transpose()?;

    Ok(HtmlTemplate { token })
}

#[derive(Deserialize, Debug)]
struct AuthorizePayload {
    user: String,
    secret: String,
}

async fn login(
    Form(payload): Form<AuthorizePayload>,
    cookies: Cookies,
) -> Result<HtmlTemplate, Error> {
    if payload.user.is_empty() || payload.secret.is_empty() {
        return Err(Error::MissingCredentials);
    }

    if payload.user != USER.as_str() || payload.secret != SECRET.as_str() {
        return Err(Error::WrongCredentials);
    }

    let token = tokio::task::spawn_blocking(move || Ok::<Token, Error>(Token::new(&payload.user)?))
        .await??;

    let mut cookie = Cookie::new("token", token.as_str().to_string());
    cookie.set_same_site(Some(cookie::SameSite::Strict));
    cookies.add(cookie);
    Ok(HtmlTemplate { token: Some(token) })
}

async fn get_current(
    cookies: Cookies,
    Extension(state): Extension<Arc<State>>,
) -> Result<Json<models::Current>, Error> {
    let _ = Token::try_from(cookies)?;
    Ok(Json(state.db.current().await?))
}

async fn post_current(
    cookies: Cookies,
    Extension(state): Extension<Arc<State>>,
    Json(payload): Json<models::Current>,
) -> Result<(), Error> {
    let _ = Token::try_from(cookies)?;
    let format = format_description!("[year]-[month]-[day]");
    let date = time::OffsetDateTime::now_utc().format(&format)?;
    state.db.upsert(date, payload.weight).await
}

async fn get_series(
    cookies: Cookies,
    Extension(state): Extension<Arc<State>>,
) -> Result<Json<models::RawAndAveragedSeries>, Error> {
    let _ = Token::try_from(cookies)?;

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
        .route("/login", post(login))
        .route("/api/current", get(get_current).post(post_current))
        .route("/api/series", get(get_series))
        .route("/static/*path", get(serve::static_data))
        .layer(Extension(state))
        .layer(CookieManagerLayer::new())
        .layer(TraceLayer::new_for_http());

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 3000));

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
