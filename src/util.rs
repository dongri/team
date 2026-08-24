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
}
