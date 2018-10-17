use envy;

lazy_static!{
pub static ref CONFIG: Config = {
    envy::from_env::<Config>().unwrap()
};
}

fn default_port() -> String {
    String::from("3000")
}

fn default_empty_string() -> String {
    String::from("")
}

fn default_database_url() -> String {
    String::from("postgres://root:@postgres:5432/team")
}

fn default_menu() -> String {
    String::from("post,nippo,tag,gist,tweet")
}

fn default_theme() -> String {
    String::from("light")
}

fn default_secret_cookie() -> String {
    String::from("FLEo9NZJDhZbBaT")
}

#[derive(Deserialize, Debug)]
pub struct Config {
    #[serde(default="default_port")]
    pub port: String, // PORT
    #[serde(default="default_database_url")]
    pub team_database_url: String, // TEAM_DATABASE_URL
    #[serde(default="default_empty_string")]
    pub team_domain: String, // TEAM_DOMAIN
    #[serde(default="default_empty_string")]
    pub team_slack: String, // TEAM_SLACK
    #[serde(default="default_empty_string")]
    pub team_webhook_url: String, // TEAM_WEBHOOK_URL
    #[serde(default="default_empty_string")]
    pub team_google_client_id: String, // TEAM_GOOGLE_CLIENT_ID
    #[serde(default="default_empty_string")]
    pub team_google_client_secret: String, // TEAM_GOOGLE_CLIENT_SECRET
    #[serde(default="default_empty_string")]
    pub team_google_redirect_url: String, // TEAM_GOOGLE_REDIRECT_URL
    #[serde(default="default_empty_string")]
    pub team_google_allow_domain: String, // TEAM_GOOGLE_ALLOW_DOMAIN
    #[serde(default="default_menu")]
    pub team_menu: String, // TEAM_MENU
    #[serde(default="default_theme")]
    pub team_theme: String, // TEAM_THEME
    #[serde(default="default_secret_cookie")]
    pub team_secret_cookie: String // TEAM_SECRET_COOKIE
}
