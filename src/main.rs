mod cache;
mod errors;
mod models;
mod routes;

use crate::routes::{app_routes, AppContext};
use axum::{
    body::Body,
    extract::Request,
    middleware::{self, Next},
    response::{IntoResponse, Response},
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[macro_use(info, warn, debug, error)]
extern crate tracing;

#[cfg(debug_assertions)]
const LOG_LEVEL: &str = "info,warn,error,moderation_service=debug,axum=debug";

// Less verbose logging in production
#[cfg(not(debug_assertions))]
const LOG_LEVEL: &str = "info,warn,error,moderation_service=debug";

lazy_static::lazy_static! {
    static ref API_KEY: String = std::env::var("API_KEY").unwrap();
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| LOG_LEVEL.into()),
        ))
        .with(
            tracing_subscriber::fmt::layer()
                .with_line_number(true)
                .with_target(true)
                .with_file(true),
        )
        .init();

    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(10)
        .connect(&db_url)
        .await
        .expect("failed to connect database");

    let cache = cache::ModerationCache::new();

    // Save bad words, regex rules and settings to cache on startup
    let bad_words: Vec<models::BadWordRow> = sqlx::query_as("SELECT * FROM bad_words ORDER BY id")
        .fetch_all(&pool)
        .await
        .expect("bad_words load failed");

    cache
        .load_bad_words(
            bad_words
                .into_iter()
                .map(|r| (r.word, r.moderation_action.to_string()))
                .collect(),
        )
        .await;

    let regex_rules: Vec<models::RegexRuleRow> =
        sqlx::query_as("SELECT * FROM regex_rules ORDER BY id")
            .fetch_all(&pool)
            .await
            .expect("regex_rules load failed");

    // Compile the regex rules for faster matches and make sure they are valid regexes
    let compiled = regex_rules
        .into_iter()
        .map(|r| {
            let regex: regex::Regex = regex::Regex::new(&r.pattern).expect("invalid regex in DB");
            (
                r.id,
                regex,
                r.description.unwrap_or_else(|| "Regex kuralı".into()),
                r.moderation_action.to_string(),
            )
        })
        .collect();

    cache.load_regex_rules(compiled).await;

    let settings: Vec<models::SettingRow> = sqlx::query_as("SELECT * FROM settings ORDER BY key")
        .fetch_all(&pool)
        .await
        .expect("settings load failed");

    // Load the settings to cache for future use
    cache
        .load_settings(settings.into_iter().map(|r| (r.key, r.value)).collect())
        .await;

    let ctx = AppContext { pool, cache };

    let port = std::env::var("PORT").unwrap_or_else(|_| {
        debug!("PORT not set, using default port 5000");
        "5000".to_string()
    });

    let app = app_routes()
        .with_state(ctx)
        .layer(middleware::from_fn(check_auth));

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await;

    match listener {
        Ok(listener) => {
            info!("Server starting on: {}", listener.local_addr().unwrap());
            axum::serve(listener, app.into_make_service())
                .await
                .unwrap();
        }
        Err(e) => {
            panic!("Failed to bind to port: {e}");
        }
    }
}

async fn check_auth(req: Request<Body>, next: Next) -> Response {
    //TODO: Add a limit for the unauthorized requests
    let headers = req.headers();
    let get_bearer_token = headers.get("Authorization");
    if let Some(bearer_token) = get_bearer_token {
        let token = match bearer_token.to_str() {
            Ok(s) => s,
            Err(e) => {
                warn!("Invalid Authorization header: {}", e);
                return errors::Error::Unauthorized.into_response();
            }
        };
        let token = match token.split(' ').nth(1) {
            Some(t) => t,
            None => {
                warn!("Malformed Authorization header: {}", token);
                return errors::Error::Unauthorized.into_response();
            }
        };

        if token == API_KEY.as_str() {
            return next.run(req).await;
        }
    }

    errors::Error::Unauthorized.into_response()
}
