use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// In-memory failure counter used to throttle credential guessing.
///
/// A key (client IP, username, ...) is blocked once it accumulates `max`
/// failures inside a `window`; the counter resets when the window expires
/// or the key succeeds. Single-process only, which is fine for this app.
pub struct FailureLimiter {
    inner: Mutex<HashMap<String, Entry>>,
    max: u32,
    window: Duration,
}

struct Entry {
    failures: u32,
    since: Instant,
}

impl FailureLimiter {
    pub fn new(max: u32, window: Duration) -> FailureLimiter {
        FailureLimiter {
            inner: Mutex::new(HashMap::new()),
            max,
            window,
        }
    }

    pub fn is_blocked(&self, key: &str) -> bool {
        let now = Instant::now();
        let map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        match map.get(key) {
            Some(e) if now.duration_since(e.since) < self.window => e.failures >= self.max,
            _ => false,
        }
    }

    pub fn record_failure(&self, key: &str) {
        let now = Instant::now();
        let mut map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        // Keep the table bounded: drop expired entries before growing it.
        if map.len() >= 10_000 {
            let window = self.window;
            map.retain(|_, e| now.duration_since(e.since) < window);
        }
        let entry = map.entry(key.to_string()).or_insert(Entry {
            failures: 0,
            since: now,
        });
        if now.duration_since(entry.since) >= self.window {
            entry.failures = 0;
            entry.since = now;
        }
        entry.failures += 1;
    }

    pub fn clear(&self, key: &str) {
        let mut map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        map.remove(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_after_max_failures_and_clears() {
        let limiter = FailureLimiter::new(3, Duration::from_secs(60));
        assert!(!limiter.is_blocked("a"));
        limiter.record_failure("a");
        limiter.record_failure("a");
        assert!(!limiter.is_blocked("a"));
        limiter.record_failure("a");
        assert!(limiter.is_blocked("a"));
        assert!(!limiter.is_blocked("b"));
        limiter.clear("a");
        assert!(!limiter.is_blocked("a"));
    }

    #[test]
    fn window_expiry_resets_counter() {
        let limiter = FailureLimiter::new(1, Duration::from_millis(0));
        limiter.record_failure("a");
        // A zero-length window expires immediately.
        assert!(!limiter.is_blocked("a"));
    }
}
