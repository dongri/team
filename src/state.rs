use std::sync::Arc;

use axum::extract::FromRef;
use axum_extra::extract::cookie::Key;
use handlebars::Handlebars;
use sha2::{Digest, Sha512};

use crate::config::Config;
use crate::db::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Arc<Config>,
    pub templates: Arc<Handlebars<'static>>,
    pub cookie_key: Key,
    pub http: reqwest::Client,
}

impl AppState {
    pub fn new(pool: PgPool, config: Config, templates: Handlebars<'static>) -> AppState {
        // Derive a stable 64-byte signing key from TEAM_SECRET_COOKIE so the
        // configured secret can be any length.
        let digest = Sha512::digest(config.team_secret_cookie.as_bytes());
        let cookie_key = Key::from(&digest);
        AppState {
            pool,
            config: Arc::new(config),
            templates: Arc::new(templates),
            cookie_key,
            http: reqwest::Client::new(),
        }
    }
}

// Required by SignedCookieJar to locate the signing key.
impl FromRef<AppState> for Key {
    fn from_ref(state: &AppState) -> Key {
        state.cookie_key.clone()
    }
}
