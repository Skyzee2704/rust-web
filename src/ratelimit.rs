//! Fixed-window in-memory limiter, plus an optional Upstash sliding window.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use serde_json::Value;

use crate::util::now_ms;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LimitResult {
    pub ok: bool,
    pub remaining: u32,
}

struct Hit {
    count: u32,
    reset: u64,
}

/// Simple fixed-window in-memory limiter (per server instance).
pub struct MemoryRateLimiter {
    limit: u32,
    window_ms: u64,
    hits: Mutex<HashMap<String, Hit>>,
    now: Box<dyn Fn() -> u64 + Send + Sync>,
}

impl MemoryRateLimiter {
    pub fn new(limit: u32, window_ms: u64) -> Self {
        Self::with_clock(limit, window_ms, now_ms)
    }

    pub fn with_clock(
        limit: u32,
        window_ms: u64,
        now: impl Fn() -> u64 + Send + Sync + 'static,
    ) -> Self {
        Self {
            limit,
            window_ms,
            hits: Mutex::new(HashMap::new()),
            now: Box::new(now),
        }
    }

    pub fn check(&self, key: &str) -> LimitResult {
        let t = (self.now)();
        let mut hits = self.hits.lock().unwrap_or_else(|e| e.into_inner());
        let fresh = !matches!(hits.get(key), Some(e) if e.reset > t);
        if fresh {
            hits.insert(
                key.to_string(),
                Hit {
                    count: 1,
                    reset: t.saturating_add(self.window_ms),
                },
            );
            if hits.len() > 5000 {
                hits.retain(|_, v| v.reset > t);
            }
            return LimitResult {
                ok: true,
                remaining: self.limit.saturating_sub(1),
            };
        }
        let e = hits.get_mut(key).expect("just checked");
        e.count = e.count.saturating_add(1);
        LimitResult {
            ok: e.count <= self.limit,
            remaining: self.limit.saturating_sub(e.count),
        }
    }
}

/// Sliding window via Upstash Redis REST. Falls back to `memory` on any error.
#[allow(clippy::too_many_arguments)]
pub async fn limit_with_upstash(
    http: &reqwest::Client,
    url: &str,
    token: &str,
    prefix: &str,
    key: &str,
    limit: u32,
    window_ms: u64,
    memory: &MemoryRateLimiter,
) -> LimitResult {
    match upstash_sliding(http, url, token, prefix, key, limit, window_ms).await {
        Ok(r) => r,
        Err(e) => {
            tracing::debug!("upstash rate limit fell back to memory: {e}");
            memory.check(key)
        }
    }
}

async fn upstash_sliding(
    http: &reqwest::Client,
    url: &str,
    token: &str,
    prefix: &str,
    key: &str,
    limit: u32,
    window_ms: u64,
) -> Result<LimitResult, String> {
    let now = now_ms();
    let member = format!("{now}-{}", rand_suffix());
    let redis_key = format!("{prefix}:{key}");
    let window_start = now.saturating_sub(window_ms);
    let body = serde_json::json!([
        ["ZREMRANGEBYSCORE", redis_key, "-inf", window_start],
        ["ZADD", redis_key, now, member],
        ["ZCARD", redis_key],
        ["PEXPIRE", redis_key, window_ms],
    ]);
    let endpoint = format!("{}/pipeline", url.trim_end_matches('/'));
    let res = http
        .post(endpoint)
        .bearer_auth(token)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err(format!("upstash {}", res.status()));
    }
    let payload: Value = res.json().await.map_err(|e| e.to_string())?;
    let count = payload
        .get(2)
        .and_then(|v| v.get("result"))
        .and_then(|v| v.as_i64())
        .ok_or_else(|| "upstash pipeline shape".to_string())?;
    let count = u32::try_from(count.max(0)).unwrap_or(u32::MAX);
    Ok(LimitResult {
        ok: count <= limit,
        remaining: limit.saturating_sub(count),
    })
}

fn rand_suffix() -> u64 {
    static N: AtomicU64 = AtomicU64::new(1);
    N.fetch_add(1, Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU64;
    use std::sync::Arc;

    #[test]
    fn blocks_after_limit_and_resets_after_window() {
        let t = Arc::new(AtomicU64::new(0));
        let clock = t.clone();
        let rl = MemoryRateLimiter::with_clock(2, 1000, move || clock.load(Ordering::SeqCst));
        assert!(rl.check("a").ok);
        assert!(rl.check("a").ok);
        assert!(!rl.check("a").ok);
        assert!(rl.check("b").ok);
        t.store(1001, Ordering::SeqCst);
        assert!(rl.check("a").ok);
    }
}
