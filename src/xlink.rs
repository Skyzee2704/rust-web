//! Parses an X post link like https://x.com/<user>/status/<id>.

use std::sync::LazyLock;

use regex::Regex;
use url::Url;

static HOST: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^(www\.|mobile\.)?(x|twitter)\.com$").unwrap());
static STATUS_PATH: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^/(?:i/web|i|[A-Za-z0-9_]{1,15})/status/(\d{5,25})/?").unwrap());

#[derive(Debug, PartialEq, Eq)]
pub struct XStatus {
    pub user: String,
    pub id: String,
}

pub fn parse_x_status_url(raw: Option<&str>) -> Option<XStatus> {
    let raw = raw?;
    if raw.is_empty() {
        return None;
    }
    let u = Url::parse(raw.trim()).ok()?;
    if u.scheme() != "https" && u.scheme() != "http" {
        return None;
    }
    let host = u.host_str()?;
    if !HOST.is_match(host) {
        return None;
    }
    let path = u.path();
    let caps = STATUS_PATH.captures(path)?;
    let id = caps.get(1)?.as_str().to_string();
    let user = path.split('/').nth(1)?.to_ascii_lowercase();
    Some(XStatus { user, id })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_x_and_twitter_post_links() {
        assert_eq!(
            parse_x_status_url(Some("https://x.com/Lethe_Fan/status/1234567890123")),
            Some(XStatus {
                user: "lethe_fan".into(),
                id: "1234567890123".into()
            })
        );
        assert_eq!(
            parse_x_status_url(Some("https://twitter.com/a/status/1234567890123?s=20"))
                .map(|s| s.id)
                .as_deref(),
            Some("1234567890123")
        );
        assert_eq!(
            parse_x_status_url(Some("https://x.com/i/status/1234567890123"))
                .map(|s| s.user)
                .as_deref(),
            Some("i")
        );
    }

    #[test]
    fn rejects_other_links() {
        assert!(parse_x_status_url(Some("")).is_none());
        assert!(parse_x_status_url(None).is_none());
        assert!(parse_x_status_url(Some("https://x.com/LetheansNFT")).is_none());
        assert!(parse_x_status_url(Some("https://evil.com/a/status/1234567890123")).is_none());
        assert!(parse_x_status_url(Some("javascript:alert(1)")).is_none());
    }
}
