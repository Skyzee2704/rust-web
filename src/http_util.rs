//! Shared request helpers: client IP, same-origin check, safe redirects.

use axum::http::HeaderMap;
use url::Url;

/// First hop of `x-forwarded-for`, otherwise `"unknown"` (matches the Next.js routes).
pub fn client_ip(headers: &HeaderMap) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("unknown")
        .to_string()
}

pub fn user_agent(headers: &HeaderMap) -> String {
    headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string()
}

pub fn read_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    let raw = headers.get(axum::http::header::COOKIE)?.to_str().ok()?;
    for part in raw.split(';') {
        let part = part.trim();
        if let Some((n, v)) = part.split_once('=') {
            if n.trim() == name {
                return Some(v.trim().to_string());
            }
        }
    }
    None
}

/// `Origin` host (with port) must equal `x-forwarded-host` or `Host`.
pub fn same_origin(headers: &HeaderMap) -> bool {
    let origin = match headers.get("origin").and_then(|v| v.to_str().ok()) {
        Some(o) => o,
        None => return false,
    };
    let host = headers
        .get("x-forwarded-host")
        .or_else(|| headers.get("host"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let Ok(url) = Url::parse(origin) else {
        return false;
    };
    host_with_port(&url) == host
}

fn host_with_port(url: &Url) -> String {
    match (url.host_str(), url.port()) {
        (Some(h), Some(p)) => format!("{h}:{p}"),
        (Some(h), None) => h.to_string(),
        _ => String::new(),
    }
}

/// Only same-site relative paths. Rejects protocol-relative and absolute URLs.
pub fn safe_redirect(raw: Option<&str>, fallback: &str) -> String {
    let Some(raw) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return fallback.to_string();
    };
    if raw.starts_with('/') && !raw.starts_with("//") && !raw.contains("://") && !raw.contains('\\')
    {
        raw.to_string()
    } else {
        fallback.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn origin_must_match_host() {
        let mut h = HeaderMap::new();
        h.insert("origin", HeaderValue::from_static("http://localhost:3000"));
        h.insert("host", HeaderValue::from_static("localhost:3000"));
        assert!(same_origin(&h));
        h.insert("origin", HeaderValue::from_static("http://evil.example"));
        assert!(!same_origin(&h));
        h.remove("origin");
        assert!(!same_origin(&h));
    }

    #[test]
    fn forwarded_host_wins() {
        let mut h = HeaderMap::new();
        h.insert("origin", HeaderValue::from_static("https://letheans.xyz"));
        h.insert("host", HeaderValue::from_static("internal:3000"));
        h.insert("x-forwarded-host", HeaderValue::from_static("letheans.xyz"));
        assert!(same_origin(&h));
    }

    #[test]
    fn redirects_stay_relative() {
        assert_eq!(safe_redirect(Some("/apply"), "/"), "/apply");
        assert_eq!(
            safe_redirect(Some("https://evil.example"), "/apply"),
            "/apply"
        );
        assert_eq!(safe_redirect(Some("//evil.example"), "/apply"), "/apply");
        assert_eq!(safe_redirect(None, "/apply"), "/apply");
    }
}
