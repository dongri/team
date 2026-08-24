use serde::Deserialize;
use serde_json::{json, Value};

use crate::models;
use crate::state::AppState;

/// A single unified Slack notification. Every event in the app (post,
/// nippo, comment, gist, tweet, tag update) is delivered in this shape.
pub struct SlackEvent {
    /// Event emoji, e.g. ":memo:"
    pub emoji: &'static str,
    /// Event label, e.g. "New Post"
    pub label: String,
    /// Linked title. Empty for title-less content (tweets).
    pub title: String,
    /// Body preview (markdown / diff / comment / code).
    pub body: String,
    /// Render the body as a code block.
    pub code: bool,
    /// In-app path, e.g. "/post/show/3".
    pub path: String,
    /// Usernames to cc.
    pub mentions: Vec<String>,
}

fn escape_mrkdwn(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let cut: String = s.chars().take(max).collect();
        format!("{}…", cut)
    }
}

/// Post a unified Block Kit message to the Slack incoming webhook.
/// Fire-and-forget: failures are logged, never propagated.
pub async fn notify_slack(state: &AppState, user_id: i32, event: SlackEvent) {
    let webhook_url = state.config.team_slack_webhook_url.clone();
    if webhook_url.is_empty() {
        tracing::debug!("TEAM_SLACK_WEBHOOK_URL not set: skipping notification");
        return;
    }

    let user = match models::user::get_by_id(&state.pool, user_id).await {
        Ok(user) => user,
        Err(e) => {
            tracing::error!("notify_slack: failed to load user: {:?}", e);
            return;
        }
    };

    let link = format!("{}{}", state.config.team_domain, event.path);
    let header = format!("{} *{}* by @{}", event.emoji, event.label, user.username);
    let title_line = if event.title.is_empty() {
        format!("<{}|View on Team>", link)
    } else {
        format!("*<{}|{}>*", link, escape_mrkdwn(&event.title))
    };

    let mut blocks: Vec<Value> = vec![json!({
        "type": "section",
        "text": { "type": "mrkdwn", "text": format!("{}\n{}", header, title_line) },
    })];

    if !event.body.trim().is_empty() {
        let preview = escape_mrkdwn(&truncate_chars(event.body.trim(), 500));
        let preview = if event.code {
            format!("```{}```", preview)
        } else {
            preview
        };
        blocks.push(json!({
            "type": "section",
            "text": { "type": "mrkdwn", "text": preview },
        }));
    }

    if !event.mentions.is_empty() {
        let cc = event
            .mentions
            .iter()
            .map(|m| format!("@{}", m))
            .collect::<Vec<_>>()
            .join(" ");
        blocks.push(json!({
            "type": "context",
            "elements": [{ "type": "mrkdwn", "text": format!("cc {}", cc) }],
        }));
    }

    let payload = json!({
        "username": "Team",
        "icon_emoji": ":beers:",
        // Fallback text for push notifications and clients without Block Kit
        "text": format!("{} {} by @{}", event.label, event.title, user.username),
        "blocks": blocks,
    });

    let client = state.http.clone();
    tokio::spawn(async move {
        if let Err(e) = client.post(&webhook_url).json(&payload).send().await {
            tracing::error!("slack webhook failed: {:?}", e);
        }
    });
}

const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://www.googleapis.com/oauth2/v3/token";
const GOOGLE_TOKENINFO_URL: &str = "https://www.googleapis.com/oauth2/v1/tokeninfo";

/// Build the Google OAuth authorization URL. `oauth_state` is a per-request
/// CSRF token that the callback verifies against the signed cookie.
pub fn google_authorize_url(state: &AppState, oauth_state: &str) -> String {
    let mut url = url::Url::parse(GOOGLE_AUTH_URL).expect("static url");
    url.query_pairs_mut()
        .append_pair("client_id", &state.config.team_google_client_id)
        .append_pair("redirect_uri", &state.config.team_google_redirect_url)
        .append_pair("response_type", "code")
        .append_pair("scope", "https://www.googleapis.com/auth/userinfo.email")
        .append_pair("state", oauth_state);
    url.to_string()
}

/// Exchange an authorization code for the user's verified email address.
pub async fn google_email_for_code(state: &AppState, code: &str) -> anyhow::Result<String> {
    #[derive(Deserialize)]
    struct Token {
        access_token: String,
    }
    #[derive(Deserialize)]
    struct Info {
        email: String,
    }

    let token: Token = state
        .http
        .post(GOOGLE_TOKEN_URL)
        .form(&[
            ("code", code),
            ("client_id", state.config.team_google_client_id.as_str()),
            (
                "client_secret",
                state.config.team_google_client_secret.as_str(),
            ),
            (
                "redirect_uri",
                state.config.team_google_redirect_url.as_str(),
            ),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let info: Info = state
        .http
        .get(GOOGLE_TOKENINFO_URL)
        .query(&[("access_token", token.access_token.as_str())])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    Ok(info.email)
}
