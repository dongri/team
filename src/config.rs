use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub port: String,                      // PORT
    pub team_database_url: String,         // TEAM_DATABASE_URL
    pub team_domain: String,               // TEAM_DOMAIN
    pub team_slack_webhook_url: String,    // TEAM_SLACK_WEBHOOK_URL
    pub team_google_client_id: String,     // TEAM_GOOGLE_CLIENT_ID
    pub team_google_client_secret: String, // TEAM_GOOGLE_CLIENT_SECRET
    pub team_google_redirect_url: String,  // TEAM_GOOGLE_REDIRECT_URL
    pub team_google_allow_domain: String,  // TEAM_GOOGLE_ALLOW_DOMAIN
    pub team_menu: String,                 // TEAM_MENU
    pub team_theme: String,                // TEAM_THEME
    pub team_secret_cookie: String,        // TEAM_SECRET_COOKIE
}

fn env_or(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

pub const DEFAULT_SECRET_COOKIE: &str = "FLEo9NZJDhZbBaT";

impl Config {
    /// Google sign-in is offered only when the OAuth settings are complete.
    pub fn google_configured(&self) -> bool {
        !self.team_google_client_id.is_empty()
            && !self.team_google_client_secret.is_empty()
            && !self.team_google_redirect_url.is_empty()
    }
}

impl Config {
    pub fn from_env() -> Config {
        Config {
            port: env_or("PORT", "3000"),
            team_database_url: env_or("TEAM_DATABASE_URL", "postgres://root:@postgres:5432/team"),
            team_domain: env_or("TEAM_DOMAIN", ""),
            team_slack_webhook_url: env_or("TEAM_SLACK_WEBHOOK_URL", ""),
            team_google_client_id: env_or("TEAM_GOOGLE_CLIENT_ID", ""),
            team_google_client_secret: env_or("TEAM_GOOGLE_CLIENT_SECRET", ""),
            team_google_redirect_url: env_or("TEAM_GOOGLE_REDIRECT_URL", ""),
            team_google_allow_domain: env_or("TEAM_GOOGLE_ALLOW_DOMAIN", ""),
            team_menu: env_or("TEAM_MENU", "post,nippo,gist,tweet,tag"),
            team_theme: env_or("TEAM_THEME", "light"),
            team_secret_cookie: env_or("TEAM_SECRET_COOKIE", DEFAULT_SECRET_COOKIE),
        }
    }
}
