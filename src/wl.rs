//! Pure helpers for the whitelist status lookup (no I/O).

use std::collections::HashSet;
use std::sync::LazyLock;

use regex::Regex;
use url::Url;

static HOST: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^(www\.|mobile\.)?(x|twitter)\.com$").unwrap());
static USER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[A-Za-z0-9_]{1,15}$").unwrap());
static WALLET: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^0x[0-9a-fA-F]{40}$").unwrap());

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WlStatus {
    Pending,
    Gtd,
    Fcfs,
}

impl WlStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            WlStatus::Pending => "pending",
            WlStatus::Gtd => "GTD",
            WlStatus::Fcfs => "FCFS",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LookupQuery {
    Wallet(String),
    Username(String),
}

/// Accepts a 0x wallet, "@name", "name" or an x.com/twitter.com profile link.
pub fn parse_lookup_query(raw: Option<&str>) -> Option<LookupQuery> {
    let mut v = raw.unwrap_or("").trim().to_string();
    if v.is_empty() || v.len() > 200 {
        return None;
    }
    if WALLET.is_match(&v) {
        return Some(LookupQuery::Wallet(v.to_ascii_lowercase()));
    }
    let is_url = (v.len() >= 7 && v[..7].eq_ignore_ascii_case("http://"))
        || (v.len() >= 8 && v[..8].eq_ignore_ascii_case("https://"));
    if is_url {
        let u = Url::parse(&v).ok()?;
        let host = u.host_str().unwrap_or("");
        if !HOST.is_match(host) {
            return None;
        }
        v = u.path().split('/').nth(1).unwrap_or("").to_string();
    }
    if let Some(rest) = v.strip_prefix('@') {
        v = rest.to_string();
    }
    if !USER.is_match(&v) {
        return None;
    }
    Some(LookupQuery::Username(v.to_ascii_lowercase()))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApplicantInfo {
    pub number: usize,
    pub username: String,
    pub wallet: String,
}

/// Finds an applicant in the full Applications rows (header at index 0).
/// The registration number is the row index, so it matches the number shown right after submit.
pub fn find_applicant(rows: &[Vec<String>], q: &LookupQuery) -> Option<ApplicantInfo> {
    for (i, r) in rows.iter().enumerate().skip(1) {
        let hit = match q {
            LookupQuery::Wallet(value) => r
                .get(3)
                .map(|w| w.eq_ignore_ascii_case(value))
                .unwrap_or(false),
            LookupQuery::Username(value) => r
                .get(2)
                .map(|u| u.eq_ignore_ascii_case(value))
                .unwrap_or(false),
        };
        if hit {
            return Some(ApplicantInfo {
                number: i,
                username: r.get(2).cloned().unwrap_or_default(),
                wallet: r.get(3).cloned().unwrap_or_default(),
            });
        }
    }
    None
}

pub fn find_applicant_by_x_id(rows: &[Vec<String>], x_id: &str) -> Option<ApplicantInfo> {
    for (i, r) in rows.iter().enumerate().skip(1) {
        if r.get(1).is_some_and(|id| !id.is_empty() && id == x_id) {
            return Some(ApplicantInfo {
                number: i,
                username: r.get(2).cloned().unwrap_or_default(),
                wallet: r.get(3).cloned().unwrap_or_default(),
            });
        }
    }
    None
}

pub fn wallet_status(wallet: &str, gtd: &HashSet<String>, fcfs: &HashSet<String>) -> WlStatus {
    let w = wallet.to_ascii_lowercase();
    if gtd.contains(&w) {
        WlStatus::Gtd
    } else if fcfs.contains(&w) {
        WlStatus::Fcfs
    } else {
        WlStatus::Pending
    }
}

pub fn mask_wallet(w: &str) -> String {
    let chars: Vec<char> = w.chars().collect();
    if chars.len() > 12 {
        let head: String = chars.iter().take(6).collect();
        let tail: String = chars.iter().skip(chars.len() - 4).collect();
        format!("{head}…{tail}")
    } else {
        w.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const W1: &str = "0x52908400098527886E0F7030069857D2E4169EE7";
    const W2: &str = "0x1111111111111111111111111111111111111111";

    fn rows() -> Vec<Vec<String>> {
        vec![
            vec![
                "timestamp".into(),
                "x_user_id".into(),
                "x_username".into(),
                "wallet".into(),
            ],
            vec!["t".into(), "100".into(), "SkyzeeReal".into(), W1.into()],
            vec!["t".into(), "200".into(), "Other_User".into(), W2.into()],
        ]
    }

    #[test]
    fn parses_wallets_handles_and_profile_links() {
        assert_eq!(
            parse_lookup_query(Some(&format!(" {W1} "))),
            Some(LookupQuery::Wallet(W1.to_ascii_lowercase()))
        );
        assert_eq!(
            parse_lookup_query(Some("@SkyzeeReal")),
            Some(LookupQuery::Username("skyzeereal".into()))
        );
        assert_eq!(
            parse_lookup_query(Some("SkyzeeReal")),
            Some(LookupQuery::Username("skyzeereal".into()))
        );
        assert_eq!(
            parse_lookup_query(Some("https://x.com/SkyzeeReal?s=20")),
            Some(LookupQuery::Username("skyzeereal".into()))
        );
    }

    #[test]
    fn rejects_junk_queries() {
        assert!(parse_lookup_query(Some("")).is_none());
        assert!(parse_lookup_query(None).is_none());
        assert!(parse_lookup_query(Some(&format!("0x{}", "a".repeat(39)))).is_none());
        assert!(parse_lookup_query(Some("two words")).is_none());
        assert!(parse_lookup_query(Some("https://evil.com/abc")).is_none());
        assert!(parse_lookup_query(Some(&"a".repeat(16))).is_none());
    }

    #[test]
    fn finds_by_username_case_insensitive() {
        assert_eq!(
            find_applicant(&rows(), &LookupQuery::Username("skyzeereal".into())),
            Some(ApplicantInfo {
                number: 1,
                username: "SkyzeeReal".into(),
                wallet: W1.into(),
            })
        );
        assert_eq!(
            find_applicant(&rows(), &LookupQuery::Wallet(W2.to_ascii_lowercase()))
                .map(|a| a.number),
            Some(2)
        );
        assert!(find_applicant(&rows(), &LookupQuery::Username("nobody".into())).is_none());
    }

    #[test]
    fn finds_by_x_id() {
        assert_eq!(
            find_applicant_by_x_id(&rows(), "200").map(|a| a.username),
            Some("Other_User".into())
        );
        assert!(find_applicant_by_x_id(&rows(), "999").is_none());
    }

    #[test]
    fn gtd_wins_over_fcfs_else_pending() {
        let gtd = HashSet::from([W1.to_ascii_lowercase()]);
        let fcfs = HashSet::from([W1.to_ascii_lowercase(), W2.to_ascii_lowercase()]);
        assert_eq!(wallet_status(W1, &gtd, &fcfs), WlStatus::Gtd);
        assert_eq!(wallet_status(W2, &gtd, &fcfs), WlStatus::Fcfs);
        assert_eq!(
            wallet_status(&format!("0x{}", "3".repeat(40)), &gtd, &fcfs),
            WlStatus::Pending
        );
    }

    #[test]
    fn masks_wallets() {
        assert_eq!(mask_wallet(W1), "0x5290…9EE7");
    }
}
