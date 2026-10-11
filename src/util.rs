use argon2::password_hash::rand_core::{OsRng, RngCore};
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use chrono::{Duration, NaiveDateTime};
use sha2::{Digest, Sha256};

/// Salt of the pre-renewal password scheme (SHA-256 + static salt).
/// Kept only to verify accounts that have not signed in since the
/// migration to Argon2; such hashes are upgraded on successful login.
const LEGACY_SALT: &str = "6jpmgwMiTzFtFoF";

fn legacy_password_hash(plain: &str) -> String {
    hex::encode(Sha256::digest(
        format!("{}{}", plain, LEGACY_SALT).as_bytes(),
    ))
}

fn hash_password_sync(plain: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(plain.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("password hashing failed: {}", e))?;
    Ok(hash.to_string())
}

fn verify_password_sync(plain: &str, stored: &str) -> bool {
    if stored.starts_with("$argon2") {
        PasswordHash::new(stored)
            .map(|parsed| {
                Argon2::default()
                    .verify_password(plain.as_bytes(), &parsed)
                    .is_ok()
            })
            .unwrap_or(false)
    } else {
        legacy_password_hash(plain) == stored
    }
}

/// Hash a password with Argon2id (runs on the blocking thread pool).
pub async fn hash_password(plain: String) -> anyhow::Result<String> {
    tokio::task::spawn_blocking(move || hash_password_sync(&plain)).await?
}

/// Verify a password against an Argon2 or legacy hash (blocking pool).
pub async fn verify_password(plain: String, stored: String) -> bool {
    tokio::task::spawn_blocking(move || verify_password_sync(&plain, &stored))
        .await
        .unwrap_or(false)
}

/// True when the stored hash still uses the legacy scheme and should be
/// re-hashed with Argon2 after a successful verification.
pub fn password_needs_rehash(stored: &str) -> bool {
    !stored.starts_with("$argon2")
}

/// 256-bit random token, hex-encoded (used for OAuth CSRF state).
pub fn random_token() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

pub fn username_hash(username: &str) -> String {
    hex::encode(Sha256::digest(username.as_bytes()))
}

/// Minimum password length for sign-up and password changes.
pub const MIN_PASSWORD_LEN: usize = 8;
pub const MAX_PASSWORD_LEN: usize = 1024;

pub fn password_acceptable(plain: &str) -> bool {
    let n = plain.chars().count();
    (MIN_PASSWORD_LEN..=MAX_PASSWORD_LEN).contains(&n)
}

pub const MAX_USERNAME_LEN: usize = 39;

/// First path segments owned by the router. A username equal to one of these
/// would shadow (or be shadowed by) an application route.
const RESERVED_USERNAMES: &[&str] = &[
    "account",
    "auth",
    "codemirror",
    "css",
    "draft",
    "favicons",
    "gist",
    "image",
    "img",
    "js",
    "nippo",
    "notification_count",
    "notifications",
    "pinned",
    "post",
    "search",
    "signin",
    "signout",
    "signup",
    "stocked",
    "tag",
    "tweet",
    "webfonts",
];

/// Usernames are `[A-Za-z0-9_.-]`, 1–39 chars, not starting with `.`/`-`,
/// and not a reserved route name. They appear in URLs (`/{username}`) and
/// Slack mentions, so anything else is rejected.
pub fn username_valid(username: &str) -> bool {
    let n = username.len();
    if n == 0 || n > MAX_USERNAME_LEN {
        return false;
    }
    if !username
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b == b'.')
    {
        return false;
    }
    if username.starts_with('.') || username.starts_with('-') {
        return false;
    }
    !RESERVED_USERNAMES.contains(&username.to_ascii_lowercase().as_str())
}

/// Turn an arbitrary string (e.g. an email local part) into a valid
/// username by replacing disallowed characters.
pub fn sanitize_username(raw: &str) -> String {
    let mut s: String = raw
        .chars()
        .filter(|c| c.is_ascii())
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .take(MAX_USERNAME_LEN)
        .collect();
    while s.starts_with('.') || s.starts_with('-') {
        s.remove(0);
    }
    if s.is_empty() || !username_valid(&s) {
        s = format!("user_{}", s);
        s.truncate(MAX_USERNAME_LEN);
    }
    s
}

/// Icon URLs end up in `<img src>`. Allow an in-app path or an http(s) URL.
pub fn icon_url_acceptable(url: &str) -> bool {
    if url.is_empty() {
        return true;
    }
    if url.len() > 2048 || url.chars().any(|c| c.is_control()) {
        return false;
    }
    if url.starts_with('/') {
        return !url.starts_with("//") && !url.starts_with("/\\");
    }
    let lower = url.to_ascii_lowercase();
    lower.starts_with("https://") || lower.starts_with("http://")
}

/// Image formats accepted by the upload endpoints, detected from the file
/// signature rather than the client-supplied name or content type.
pub fn sniff_image_extension(data: &[u8]) -> Option<&'static str> {
    if data.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        Some("png")
    } else if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("jpg")
    } else if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        Some("gif")
    } else if data.len() >= 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WEBP" {
        Some("webp")
    } else {
        None
    }
}

/// Client address for logging and rate limiting. `X-Forwarded-For` is only
/// honored when the app is configured to sit behind a reverse proxy;
/// otherwise the header is attacker-controlled and the peer address is used.
pub fn client_ip(
    headers: &axum::http::HeaderMap,
    peer: Option<std::net::SocketAddr>,
    trust_proxy: bool,
) -> String {
    if trust_proxy {
        if let Some(ip) = headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(',').next())
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        {
            return ip.to_string();
        }
    }
    peer.map(|p| p.ip().to_string())
        .unwrap_or_else(|| "-".to_string())
}

pub fn uppercase_first_letter(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

pub fn jst_time_formatter(time: NaiveDateTime) -> String {
    (time + Duration::hours(9))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

pub fn split_menu(menu: &str) -> Vec<String> {
    menu.split(',').map(|s| s.to_string()).collect()
}

pub const PAGINATES_PER: i32 = 10;

/// Normalized pagination values shared by all list pages.
pub struct Page {
    pub current: i32,
    pub offset: i32,
    pub limit: i32,
}

impl Page {
    pub fn new(page: Option<i32>) -> Page {
        let current = page.unwrap_or(1).max(1);
        Page {
            current,
            offset: (current - 1) * PAGINATES_PER,
            limit: PAGINATES_PER,
        }
    }

    pub fn total_pages(&self, count: i32) -> i32 {
        count / PAGINATES_PER + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn argon2_hash_roundtrip() {
        let hash = hash_password_sync("secret123").unwrap();
        assert!(hash.starts_with("$argon2"));
        assert!(verify_password_sync("secret123", &hash));
        assert!(!verify_password_sync("wrong", &hash));
        assert!(!password_needs_rehash(&hash));
    }

    #[test]
    fn legacy_hash_verifies_and_needs_rehash() {
        let stored = legacy_password_hash("secret123");
        assert!(verify_password_sync("secret123", &stored));
        assert!(!verify_password_sync("wrong", &stored));
        assert!(password_needs_rehash(&stored));
    }

    #[test]
    fn random_token_is_unique() {
        assert_ne!(random_token(), random_token());
        assert_eq!(random_token().len(), 64);
    }

    #[test]
    fn username_rules() {
        assert!(username_valid("alice"));
        assert!(username_valid("alice.b-c_1"));
        assert!(!username_valid(""));
        assert!(!username_valid("a/b"));
        assert!(!username_valid("a b"));
        assert!(!username_valid(".alice"));
        assert!(!username_valid("post"));
        assert!(!username_valid("Signin"));
        assert!(!username_valid(&"a".repeat(MAX_USERNAME_LEN + 1)));
        assert_eq!(sanitize_username("alice+tag"), "alice_tag");
        assert_eq!(sanitize_username("post"), "user_post");
        assert!(username_valid(&sanitize_username("")));
        assert!(username_valid(&sanitize_username("太郎@x")));
    }

    #[test]
    fn icon_url_rules() {
        assert!(icon_url_acceptable(""));
        assert!(icon_url_acceptable("/img/profile/icons/a.png"));
        assert!(icon_url_acceptable("https://example.com/a.png"));
        assert!(!icon_url_acceptable("//evil.com/a.png"));
        assert!(!icon_url_acceptable("javascript:alert(1)"));
        assert!(!icon_url_acceptable("data:image/png;base64,AAAA"));
    }

    #[test]
    fn image_sniffing() {
        assert_eq!(
            sniff_image_extension(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0]),
            Some("png")
        );
        assert_eq!(
            sniff_image_extension(&[0xFF, 0xD8, 0xFF, 0xE0]),
            Some("jpg")
        );
        assert_eq!(sniff_image_extension(b"GIF89a...."), Some("gif"));
        assert_eq!(sniff_image_extension(b"RIFF\0\0\0\0WEBPVP8 "), Some("webp"));
        assert_eq!(sniff_image_extension(b"<html>"), None);
        assert_eq!(sniff_image_extension(b""), None);
    }

    #[test]
    fn client_ip_trusts_proxy_only_when_configured() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("x-forwarded-for", "203.0.113.5, 10.0.0.1".parse().unwrap());
        let peer: std::net::SocketAddr = "10.0.0.9:1234".parse().unwrap();
        assert_eq!(client_ip(&headers, Some(peer), true), "203.0.113.5");
        assert_eq!(client_ip(&headers, Some(peer), false), "10.0.0.9");
        assert_eq!(client_ip(&headers, None, false), "-");
    }
}
