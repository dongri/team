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
}
