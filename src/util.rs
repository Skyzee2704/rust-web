//! Hashing, EIP-55 wallets, env-key normalization, and small formatters.

use chrono::Datelike;
use sha2::{Digest, Sha256};
use tiny_keccak::{Hasher, Keccak};

pub fn salted_hash(value: &str, salt: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(format!("{salt}:{value}").as_bytes());
    let dig = hasher.finalize();
    hex::encode(dig)[..32].to_string()
}

/// Private keys pasted into env often have literal "\n" — normalize them.
pub fn normalize_private_key(raw: &str) -> String {
    let mut k = raw.trim().to_string();
    if k.len() >= 2 && k.starts_with('"') && k.ends_with('"') {
        k = k[1..k.len() - 1].to_string();
    }
    k.replace("\\n", "\n")
}

/// Returns the checksummed (EIP-55) address, or `None` if it isn't 20 bytes of hex.
pub fn normalize_wallet(input: &str) -> Option<String> {
    let v = input.trim();
    if !is_hex_address(v) {
        return None;
    }
    Some(to_checksum_address(&v.to_ascii_lowercase()))
}

fn is_hex_address(v: &str) -> bool {
    let bytes = v.as_bytes();
    if bytes.len() != 42 || &bytes[..2] != b"0x" {
        return false;
    }
    bytes[2..].iter().all(|b| b.is_ascii_hexdigit())
}

fn to_checksum_address(lower_with_0x: &str) -> String {
    let hexaddr = &lower_with_0x[2..];
    let mut hasher = Keccak::v256();
    hasher.update(hexaddr.as_bytes());
    let mut out = [0u8; 32];
    hasher.finalize(&mut out);
    let hash = hex::encode(out);
    let mut result = String::from("0x");
    for (i, c) in hexaddr.chars().enumerate() {
        let nibble = u8::from_str_radix(&hash[i..i + 1], 16).unwrap_or(0);
        if nibble >= 8 {
            result.push(c.to_ascii_uppercase());
        } else {
            result.push(c);
        }
    }
    result
}

/// en-US grouping, e.g. 2222 → "2,222".
pub fn format_int(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out.chars().rev().collect()
}

pub fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn env_nonempty(key: &str) -> bool {
    std::env::var(key)
        .ok()
        .is_some_and(|v| !v.trim().is_empty())
}

pub fn env_trim(key: &str) -> String {
    std::env::var(key).unwrap_or_default().trim().to_string()
}

/// Milliseconds as a CSS duration, e.g. 170 → "0.17s".
pub fn css_delay(ms: i32) -> String {
    let whole = ms / 1000;
    let frac = ms % 1000;
    if frac == 0 {
        format!("{whole}s")
    } else {
        let mut f = format!("{frac:03}");
        while f.ends_with('0') {
            f.pop();
        }
        format!("{whole}.{f}s")
    }
}

pub fn iso_timestamp() -> String {
    chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string()
}

#[derive(Clone, Debug)]
pub struct MintWhen {
    pub announced: bool,
    pub iso: String,
    /// Stats tile, e.g. "Dec 21, 2026" or "TBA".
    pub label: String,
    /// Ticket departure line (UTC, " GMT" stripped) or "TBA".
    pub departure: String,
    pub target_ms: i64,
}

pub fn mint_when(raw: &str) -> MintWhen {
    let parsed = parse_mint(raw);
    match parsed {
        Some(dt) => {
            let utc = dt.with_timezone(&chrono::Utc);
            let label = format!("{} {}, {}", utc.format("%b"), utc.day(), utc.format("%Y"));
            let departure = utc.format("%a, %d %b %Y %H:%M:%S").to_string();
            MintWhen {
                announced: true,
                iso: utc.to_rfc3339().replace("+00:00", "Z"),
                label,
                departure,
                target_ms: utc.timestamp_millis(),
            }
        }
        None => MintWhen {
            announced: false,
            iso: String::new(),
            label: "TBA".into(),
            departure: "TBA".into(),
            target_ms: 0,
        },
    }
}

fn parse_mint(raw: &str) -> Option<chrono::DateTime<chrono::FixedOffset>> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(raw) {
        return Some(dt);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_sha256_hex_prefix() {
        assert_eq!(
            salted_hash("value", "salt"),
            "dba76fca79c1eadce36084f8b227f5d4"
        );
        assert_eq!(salted_hash("value", "salt").len(), 32);
    }

    #[test]
    fn private_key_newlines_and_quotes() {
        assert_eq!(normalize_private_key("\"a\\nb\\n\""), "a\nb\n");
    }

    #[test]
    fn checksums_known_addresses() {
        assert_eq!(
            normalize_wallet("0x52908400098527886e0f7030069857d2e4169ee7").as_deref(),
            Some("0x52908400098527886E0F7030069857D2E4169EE7")
        );
        assert_eq!(
            normalize_wallet("  0xde709f2102306220921060314715629080e2fb77 ").as_deref(),
            Some("0xde709f2102306220921060314715629080e2fb77")
        );
    }

    #[test]
    fn rejects_bad_wallets() {
        for v in [
            "",
            "0x123",
            "hello",
            "0xZZ08400098527886e0f7030069857d2e4169ee7",
            "52908400098527886e0f7030069857d2e4169ee7",
        ] {
            assert!(normalize_wallet(v).is_none(), "{v}");
        }
    }

    #[test]
    fn groups_thousands() {
        assert_eq!(format_int(2222), "2,222");
        assert_eq!(format_int(222), "222");
        assert_eq!(format_int(1_000_000), "1,000,000");
    }

    #[test]
    fn empty_mint_is_tba() {
        let m = mint_when("");
        assert!(!m.announced);
        assert_eq!(m.label, "TBA");
        assert_eq!(m.departure, "TBA");
    }

    #[test]
    fn iso_mint_formats_utc() {
        let m = mint_when("2026-12-21T14:00:00Z");
        assert!(m.announced);
        assert_eq!(m.label, "Dec 21, 2026");
        assert_eq!(m.departure, "Mon, 21 Dec 2026 14:00:00");
    }
}
