mod auth;
mod config;
mod db;
mod error;
mod handlers;
mod models;
mod ratelimit;
mod services;
mod state;
mod util;
mod views;

use handlebars::{DirectorySourceOptions, Handlebars};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env if present; real environment variables take precedence.
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("team=debug")),
        )
        .init();

    // Fails fast on a missing / placeholder / short cookie secret.
    let config = config::Config::from_env()?;

    if config.team_signup_enabled && config.team_signup_code.is_empty() {
        tracing::warn!(
            "Sign-up is open to anyone (TEAM_SIGNUP_ENABLED=true, no TEAM_SIGNUP_CODE). \
             Set TEAM_SIGNUP_CODE or TEAM_SIGNUP_ENABLED=false once your team has accounts."
        );
    }
    if config.google_configured() && config.team_google_allow_domain.is_empty() {
        tracing::warn!(
            "Google sign-in accepts any Google account (TEAM_GOOGLE_ALLOW_DOMAIN is empty)"
        );
    }
    if config.team_domain.is_empty() {
        tracing::warn!(
            "TEAM_DOMAIN is empty: generated links and origin checks fall back to the Host header"
        );
    }

    let mut templates = Handlebars::new();
    #[cfg(debug_assertions)]
    templates.set_dev_mode(true);
    templates.register_templates_directory("./templates", DirectorySourceOptions::default())?;

    let pool = db::create_pool(&config.team_database_url)?;

    let listen = format!("0.0.0.0:{}", config.port);
    let state = state::AppState::new(pool, config, templates);
    let app = handlers::router::create_router(state);

    tracing::info!("Listening on {}", listen);
    let listener = tokio::net::TcpListener::bind(&listen).await?;
    // ConnectInfo provides the peer address for access logging.
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await?;
    Ok(())
}
