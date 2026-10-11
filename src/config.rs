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
    pub team_signup_enabled: bool,         // TEAM_SIGNUP_ENABLED
    pub team_signup_code: String,          // TEAM_SIGNUP_CODE
    pub team_trust_proxy: bool,            // TEAM_TRUST_PROXY
}

fn env_or(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

fn env_bool(key: &str, default: bool) -> bool {
    match env::var(key) {
        Ok(v) => matches!(
            v.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        ),
        Err(_) => default,
    }
}

/// Minimum length of TEAM_SECRET_COOKIE. The value is hashed to a 64-byte
/// signing key, so anything shorter than this is trivially brute-forceable.
pub const MIN_SECRET_COOKIE_LEN: usize = 32;

/// Placeholder shipped in `.env.example`; refused so a copied example file
/// can never be deployed as-is.
const EXAMPLE_SECRET_COOKIE: &str = "change-me-to-a-long-random-string";

impl Config {
    /// Google sign-in is offered only when the OAuth settings are complete.
    pub fn google_configured(&self) -> bool {
        !self.team_google_client_id.is_empty()
            && !self.team_google_client_secret.is_empty()
            && !self.team_google_redirect_url.is_empty()
    }

    /// Session cookies carry the `Secure` flag when the app is served over
    /// HTTPS (judged from TEAM_DOMAIN).
    pub fn secure_cookies(&self) -> bool {
        self.team_domain.starts_with("https://")
    }

    /// `host[:port]` of TEAM_DOMAIN, used to validate request origins.
    pub fn domain_authority(&self) -> Option<String> {
        let url = url::Url::parse(&self.team_domain).ok()?;
        let host = url.host_str()?;
        Some(match url.port() {
            Some(port) => format!("{}:{}", host, port),
            None => host.to_string(),
        })
    }
}

impl Config {
    pub fn from_env() -> anyhow::Result<Config> {
        let config = Config {
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
            team_secret_cookie: env_or("TEAM_SECRET_COOKIE", ""),
            team_signup_enabled: env_bool("TEAM_SIGNUP_ENABLED", true),
            team_signup_code: env_or("TEAM_SIGNUP_CODE", ""),
            team_trust_proxy: env_bool("TEAM_TRUST_PROXY", false),
        };
        config.validate()?;
        Ok(config)
    }

    /// Refuse to start with a missing, placeholder or short cookie secret:
    /// anyone who knows the key can forge a login cookie for any user.
    fn validate(&self) -> anyhow::Result<()> {
        let secret = self.team_secret_cookie.trim();
        anyhow::ensure!(
            !secret.is_empty() && secret != EXAMPLE_SECRET_COOKIE,
            "TEAM_SECRET_COOKIE is not set. Generate one with `openssl rand -hex 32` and put it in .env"
        );
        anyhow::ensure!(
            secret.len() >= MIN_SECRET_COOKIE_LEN,
            "TEAM_SECRET_COOKIE must be at least {} characters (got {})",
            MIN_SECRET_COOKIE_LEN,
            secret.len()
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Config {
        Config {
            port: "3000".into(),
            team_database_url: String::new(),
            team_domain: "https://team.example.com".into(),
            team_slack_webhook_url: String::new(),
            team_google_client_id: String::new(),
            team_google_client_secret: String::new(),
            team_google_redirect_url: String::new(),
            team_google_allow_domain: String::new(),
            team_menu: String::new(),
            team_theme: String::new(),
            team_secret_cookie: "x".repeat(MIN_SECRET_COOKIE_LEN),
            team_signup_enabled: true,
            team_signup_code: String::new(),
            team_trust_proxy: false,
        }
    }

    #[test]
    fn rejects_missing_or_weak_secret() {
        let mut c = base();
        assert!(c.validate().is_ok());
        c.team_secret_cookie = String::new();
        assert!(c.validate().is_err());
        c.team_secret_cookie = EXAMPLE_SECRET_COOKIE.into();
        assert!(c.validate().is_err());
        c.team_secret_cookie = "short".into();
        assert!(c.validate().is_err());
    }

    #[test]
    fn domain_authority_and_secure() {
        let mut c = base();
        assert_eq!(c.domain_authority().as_deref(), Some("team.example.com"));
        assert!(c.secure_cookies());
        c.team_domain = "http://localhost:3000".into();
        assert_eq!(c.domain_authority().as_deref(), Some("localhost:3000"));
        assert!(!c.secure_cookies());
        c.team_domain = String::new();
        assert_eq!(c.domain_authority(), None);
    }
}
