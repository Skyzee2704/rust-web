//! Process-wide server state. Built from the environment, or `AppState::demo()` in tests.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use reqwest::Client;

use crate::config::SITE;
use crate::ratelimit::MemoryRateLimiter;
use crate::sheets::GoogleSheets;
use crate::util::{env_nonempty, env_trim};

pub struct StatsCache {
    pub at: u64,
    pub count: Option<i64>,
}

pub struct TabCache {
    pub at: u64,
    pub gtd: HashSet<String>,
    pub fcfs: HashSet<String>,
}

pub struct Inner {
    pub demo_mode: bool,
    pub auth_configured: bool,
    pub sheets_configured: bool,
    pub auth_secret: String,
    pub ip_salt: String,
    pub sheet_tab: String,
    pub site_url: String,
    pub auth_url: String,
    pub share_host: &'static str,
    pub x_client_id: String,
    pub x_client_secret: String,
    pub x_verification: bool,
    pub x_bearer: String,
    pub http: Client,
    pub sheets: Option<GoogleSheets>,
    pub apply_limiter: MemoryRateLimiter,
    pub lookup_limiter: MemoryRateLimiter,
    pub upstash: Option<(String, String)>,
    pub stats_cache: Mutex<StatsCache>,
    pub tab_cache: Mutex<Option<TabCache>>,
}

#[derive(Clone)]
pub struct AppState(pub Arc<Inner>);

impl std::ops::Deref for AppState {
    type Target = Inner;
    fn deref(&self) -> &Inner {
        &self.0
    }
}

/// `.env` first, then `.env.local` overrides (same order as Next.js).
pub fn load_dotenv() {
    let _ = dotenvy::from_filename(".env");
    let _ = dotenvy::from_filename_override(".env.local");
}

impl AppState {
    pub fn from_env() -> Self {
        let auth_configured = env_nonempty("X_CLIENT_ID")
            && env_nonempty("X_CLIENT_SECRET")
            && env_nonempty("AUTH_SECRET");
        let sheets_configured = env_nonempty("GOOGLE_SERVICE_ACCOUNT_EMAIL")
            && env_nonempty("GOOGLE_PRIVATE_KEY")
            && env_nonempty("GOOGLE_SHEET_ID");
        let upstash =
            if env_nonempty("UPSTASH_REDIS_REST_URL") && env_nonempty("UPSTASH_REDIS_REST_TOKEN") {
                Some((
                    env_trim("UPSTASH_REDIS_REST_URL"),
                    env_trim("UPSTASH_REDIS_REST_TOKEN"),
                ))
            } else {
                None
            };
        let x_verification = std::env::var("ENABLE_X_TASK_VERIFICATION").ok().as_deref()
            == Some("true")
            && env_nonempty("X_BEARER_TOKEN");
        let site_url = trim_slash(&env_trim("NEXT_PUBLIC_SITE_URL"));
        let site_url = if site_url.is_empty() {
            "http://localhost:3000".to_string()
        } else {
            site_url
        };
        let auth_url = trim_slash(&env_trim("AUTH_URL"));
        let auth_url = if auth_url.is_empty() {
            site_url.clone()
        } else {
            auth_url
        };
        let sheet_tab = env_trim("GOOGLE_SHEET_TAB");
        let sheet_tab = if sheet_tab.is_empty() {
            "Applications".to_string()
        } else {
            sheet_tab
        };
        let ip_salt = env_trim("IP_HASH_SALT");
        let ip_salt = if ip_salt.is_empty() {
            "letheans-dev-salt-change-me".to_string()
        } else {
            ip_salt
        };
        let auth_secret = env_trim("AUTH_SECRET");
        let auth_secret = if auth_secret.is_empty() {
            "demo-mode-insecure-secret-not-for-production".to_string()
        } else {
            auth_secret
        };
        let http = build_http();
        let sheets = if sheets_configured {
            GoogleSheets::from_env(http.clone())
        } else {
            None
        };
        Self(Arc::new(Inner {
            demo_mode: !(auth_configured && sheets_configured),
            auth_configured,
            sheets_configured,
            auth_secret,
            ip_salt,
            sheet_tab,
            site_url,
            auth_url,
            share_host: SITE.share_host,
            x_client_id: env_trim("X_CLIENT_ID"),
            x_client_secret: env_trim("X_CLIENT_SECRET"),
            x_verification,
            x_bearer: env_trim("X_BEARER_TOKEN"),
            http,
            sheets,
            apply_limiter: MemoryRateLimiter::new(5, 60_000),
            lookup_limiter: MemoryRateLimiter::new(20, 60_000),
            upstash,
            stats_cache: Mutex::new(StatsCache { at: 0, count: None }),
            tab_cache: Mutex::new(None),
        }))
    }

    /// No X login and no Sheets. Used by tests and any boot without env.
    pub fn demo() -> Self {
        Self(Arc::new(Inner {
            demo_mode: true,
            auth_configured: false,
            sheets_configured: false,
            auth_secret: "demo-mode-insecure-secret-not-for-production".into(),
            ip_salt: "letheans-dev-salt-change-me".into(),
            sheet_tab: "Applications".into(),
            site_url: "http://localhost:3000".into(),
            auth_url: "http://localhost:3000".into(),
            share_host: SITE.share_host,
            x_client_id: String::new(),
            x_client_secret: String::new(),
            x_verification: false,
            x_bearer: String::new(),
            http: build_http(),
            sheets: None,
            apply_limiter: MemoryRateLimiter::new(5, 60_000),
            lookup_limiter: MemoryRateLimiter::new(20, 60_000),
            upstash: None,
            stats_cache: Mutex::new(StatsCache { at: 0, count: None }),
            tab_cache: Mutex::new(None),
        }))
    }
}

fn build_http() -> Client {
    Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .user_agent("Letheans/0.1 (+https://letheans.xyz)")
        .build()
        .unwrap_or_else(|_| Client::new())
}

fn trim_slash(s: &str) -> String {
    s.trim().trim_end_matches('/').to_string()
}
