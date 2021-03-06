use askama::Template;
use axum::extract::{Extension, Form};
use axum::response::{IntoResponse, Json, Redirect};
use axum::routing::{get, post};
use axum_extra::extract::cookie::{Cookie, CookieJar};
use clap::{Parser, Subcommand};
use serde::Deserialize;
use std::convert::From;
use std::sync::Arc;
use time::macros::format_description;
use tower_http::trace::TraceLayer;
use tracing::{error, info};

mod auth;
mod db;
mod error;
mod models;
mod serve;

use auth::Token;
use error::Error;

#[derive(Parser)]
struct Opt {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Hash user password and add to database
    InsertHash {
        #[clap(long)]
        user: String,

        #[clap(long)]
        password: String,
    },

    /// Run the server
    Run {},
}

struct State {
    db: db::Database,
}

#[derive(Template)]
#[template(path = "index.html")]
struct HtmlTemplate {
    logged_in: bool,
}

async fn index(cookies: CookieJar) -> Result<HtmlTemplate, Error> {
    let logged_in = cookies.get("token").map_or(false, |cookie| {
        let value: Result<Token, Error> = cookie.value().try_into();
        match value {
            Ok(_) => true,
            Err(err) => {
                error!(error = ?err);
                false
            }
        }
    });

    Ok(HtmlTemplate { logged_in })
}

#[derive(Deserialize, Debug)]
struct AuthorizePayload {
    user: String,
    secret: String,
}

async fn login(
    Form(payload): Form<AuthorizePayload>,
    cookies: CookieJar,
    Extension(state): Extension<Arc<State>>,
) -> impl IntoResponse {
    let hash = match state.db.hash(&payload.user).await {
        Err(err) => {
            if matches!(err, error::Error::Database(_)) {
                return Ok((cookies, Redirect::to("/")));
            }
            Err(err)
        }
        Ok(hash) => Ok(hash),
    }?;

    let cookies = if auth::verify_secret(&hash, &payload.secret) {
        let token = tokio::task::spawn_blocking(move || Token::new(&payload.user)).await??;
        let mut cookie = Cookie::new("token", token.as_str().to_string());
        cookie.set_same_site(Some(cookie::SameSite::Strict));
        cookies.add(cookie)
    } else {
        cookies
    };

    Ok::<_, Error>((cookies, Redirect::to("/")))
}

async fn get_current(
    cookies: CookieJar,
    Extension(state): Extension<Arc<State>>,
) -> Result<Json<models::Current>, Error> {
    let _ = Token::try_from(cookies)?;
    Ok(Json(state.db.current().await?))
}

async fn post_current(
    cookies: CookieJar,
    Extension(state): Extension<Arc<State>>,
    Json(payload): Json<models::Current>,
) -> Result<(), Error> {
    let _ = Token::try_from(cookies)?;
    let format = format_description!("[year]-[month]-[day]");
    let date = time::OffsetDateTime::now_utc().format(&format)?;
    state.db.upsert(date, payload.weight).await
}

async fn get_series(
    cookies: CookieJar,
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

async fn run(db: db::Database) -> Result<(), Box<dyn std::error::Error>> {
    let state = Arc::new(State { db });

    let app = axum::Router::new()
        .route("/", get(index))
        .route("/login", post(login))
        .route("/api/current", get(get_current).post(post_current))
        .route("/api/series", get(get_series))
        .route("/static/*path", get(serve::static_data))
        .layer(Extension(state))
        .layer(TraceLayer::new_for_http());

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 8989));

    info!("listening on {addr:?}");

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c()
                .await
                .expect("failed to listen to ctrl-c");
        })
        .await?;

    Ok(())
}

async fn insert_hash(
    db: db::Database,
    user: &str,
    password: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let hash = auth::hash_secret(password);
    Ok(db.insert_hash(user, &hash).await?)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let opt = Opt::parse();
    let db = db::Database::new().await?;

    match &opt.command {
        Commands::InsertHash { user, password } => insert_hash(db, user, password).await,
        Commands::Run {} => run(db).await,
    }
}
