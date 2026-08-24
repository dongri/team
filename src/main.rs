mod auth;
mod config;
mod db;
mod error;
mod handlers;
mod models;
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

    let config = config::Config::from_env();
    if config.team_secret_cookie == config::DEFAULT_SECRET_COOKIE {
        tracing::warn!(
            "TEAM_SECRET_COOKIE is the built-in default — set a long random value in production"
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
