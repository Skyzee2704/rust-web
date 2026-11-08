//! Google Sheets access and the pure row helpers the routes depend on.

use std::collections::HashSet;
use std::sync::Mutex;

use async_trait::async_trait;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::Serialize;
use serde_json::Value;

use crate::util::{normalize_private_key, now_secs};

pub const HEADERS: [&str; 8] = [
    "timestamp",
    "x_user_id",
    "x_username",
    "wallet",
    "tasks_completed",
    "reply_url",
    "ua_hash",
    "ip_hash",
];

#[derive(Clone, Debug)]
pub struct Application {
    pub timestamp: String,
    pub x_user_id: String,
    pub x_username: String,
    pub wallet: String,
    pub tasks_completed: Vec<String>,
    pub reply_url: Option<String>,
    pub ua_hash: String,
    pub ip_hash: String,
}

pub fn to_row(a: &Application) -> Vec<String> {
    vec![
        a.timestamp.clone(),
        a.x_user_id.clone(),
        a.x_username.clone(),
        a.wallet.clone(),
        a.tasks_completed.join(","),
        a.reply_url.clone().unwrap_or_default(),
        a.ua_hash.clone(),
        a.ip_hash.clone(),
    ]
}

/// Returns which identity is already present in existing rows (header excluded).
pub fn find_duplicate(rows: &[Vec<String>], x_user_id: &str, wallet: &str) -> Option<Duplicate> {
    let w = wallet.to_ascii_lowercase();
    for r in rows {
        if r.get(1).is_some_and(|id| !id.is_empty() && id == x_user_id) {
            return Some(Duplicate::X);
        }
        if r.get(3)
            .is_some_and(|cell| !cell.is_empty() && cell.eq_ignore_ascii_case(&w))
        {
            return Some(Duplicate::Wallet);
        }
    }
    None
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Duplicate {
    X,
    Wallet,
}

pub fn sheet_range(tab: &str, cells: &str) -> String {
    format!("'{}'!{cells}", tab.replace('\'', "''"))
}

/// "Applications!A3:H3" -> 2 (row 1 is the header).
pub fn row_number_from_range(updated_range: Option<&str>) -> Option<i64> {
    let updated_range = updated_range?;
    static RE: std::sync::LazyLock<regex::Regex> =
        std::sync::LazyLock::new(|| regex::Regex::new(r"!?[A-Z]+(\d+)(?::[A-Z]+\d+)?$").unwrap());
    let caps = RE.captures(updated_range)?;
    let n = caps.get(1)?.as_str().parse::<i64>().ok()? - 1;
    if n >= 1 {
        Some(n)
    } else {
        None
    }
}

#[derive(Debug, Clone)]
pub struct SheetsError(pub String);

impl std::fmt::Display for SheetsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for SheetsError {}

impl SheetsError {
    pub fn is_bad_range(&self) -> bool {
        self.0.contains("Unable to parse range")
    }
}

#[async_trait]
pub trait SheetsApi: Send + Sync {
    async fn get_values(&self, range: &str) -> Result<Vec<Vec<String>>, SheetsError>;
    async fn update_values(&self, range: &str, values: Vec<Vec<String>>)
        -> Result<(), SheetsError>;
    async fn append_values(
        &self,
        range: &str,
        values: Vec<Vec<String>>,
    ) -> Result<Option<String>, SheetsError>;
    async fn add_sheet(&self, title: &str) -> Result<(), SheetsError>;
}

pub async fn read_rows(api: &dyn SheetsApi, tab: &str) -> Result<Vec<Vec<String>>, SheetsError> {
    api.get_values(&sheet_range(tab, "A:H")).await
}

/// Creates the tab if missing and writes the header row if empty.
/// Returns data rows (header excluded).
pub async fn ensure_header(
    api: &dyn SheetsApi,
    tab: &str,
) -> Result<Vec<Vec<String>>, SheetsError> {
    let rows = match read_rows(api, tab).await {
        Ok(rows) => rows,
        Err(e) if e.is_bad_range() => {
            api.add_sheet(tab).await?;
            Vec::new()
        }
        Err(e) => return Err(e),
    };
    if rows.is_empty() {
        let header: Vec<String> = HEADERS.iter().map(|s| (*s).to_string()).collect();
        api.update_values(&sheet_range(tab, "A1"), vec![header])
            .await?;
        return Ok(Vec::new());
    }
    Ok(rows.into_iter().skip(1).collect())
}

/// Appends the application and returns its 1-based registration number (sheet row minus the header).
pub async fn append_application(
    api: &dyn SheetsApi,
    tab: &str,
    app: &Application,
) -> Result<Option<i64>, SheetsError> {
    let updated = api
        .append_values(&sheet_range(tab, "A:H"), vec![to_row(app)])
        .await?;
    Ok(row_number_from_range(updated.as_deref()))
}

/// Wallets listed from A1 downward on a tab (e.g. "GTD", "FCFS"). A missing tab counts as empty.
pub async fn read_wallet_tab(
    api: &dyn SheetsApi,
    tab: &str,
) -> Result<HashSet<String>, SheetsError> {
    let range = sheet_range(tab, "A:A");
    match api.get_values(&range).await {
        Ok(rows) => {
            let mut out = HashSet::new();
            for r in rows {
                let v = r
                    .first()
                    .map(|s| s.trim().to_ascii_lowercase())
                    .unwrap_or_default();
                if regex_wallet(&v) {
                    out.insert(v);
                }
            }
            Ok(out)
        }
        Err(e) if e.is_bad_range() => Ok(HashSet::new()),
        Err(e) => Err(e),
    }
}

fn regex_wallet(v: &str) -> bool {
    static RE: std::sync::LazyLock<regex::Regex> =
        std::sync::LazyLock::new(|| regex::Regex::new(r"^0x[0-9a-f]{40}$").unwrap());
    RE.is_match(v)
}

pub async fn count_applicants(api: &dyn SheetsApi, tab: &str) -> Result<i64, SheetsError> {
    let rows = read_rows(api, tab).await?;
    Ok((rows.len() as i64 - 1).max(0))
}

pub struct GoogleSheets {
    http: reqwest::Client,
    email: String,
    sheet_id: String,
    key: Result<EncodingKey, String>,
    token: Mutex<Option<CachedToken>>,
}

struct CachedToken {
    access: String,
    exp: u64,
}

impl GoogleSheets {
    pub fn from_env(http: reqwest::Client) -> Option<Self> {
        let email = crate::util::env_trim("GOOGLE_SERVICE_ACCOUNT_EMAIL");
        let key_raw = std::env::var("GOOGLE_PRIVATE_KEY").unwrap_or_default();
        let sheet_id = crate::util::env_trim("GOOGLE_SHEET_ID");
        if email.is_empty() || key_raw.trim().is_empty() || sheet_id.is_empty() {
            return None;
        }
        let pem = normalize_private_key(&key_raw);
        let key = EncodingKey::from_rsa_pem(pem.as_bytes()).map_err(|e| e.to_string());
        Some(Self {
            http,
            email,
            sheet_id,
            key,
            token: Mutex::new(None),
        })
    }

    async fn bearer(&self) -> Result<String, SheetsError> {
        let key = self
            .key
            .as_ref()
            .map_err(|e| SheetsError(format!("invalid service account key: {e}")))?;
        let now = now_secs();
        if let Some(tok) = self
            .token
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
        {
            if tok.exp > now + 30 {
                return Ok(tok.access.clone());
            }
        }
        let claims = JwtClaims {
            iss: self.email.clone(),
            scope: "https://www.googleapis.com/auth/spreadsheets",
            aud: "https://oauth2.googleapis.com/token",
            iat: now,
            exp: now + 3600,
        };
        let assertion = encode(&Header::new(Algorithm::RS256), &claims, key)
            .map_err(|e| SheetsError(format!("jwt: {e}")))?;
        let res = self
            .http
            .post("https://oauth2.googleapis.com/token")
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
                ("assertion", assertion.as_str()),
            ])
            .send()
            .await
            .map_err(|e| SheetsError(e.to_string()))?;
        let status = res.status();
        let body: Value = res.json().await.map_err(|e| SheetsError(e.to_string()))?;
        if !status.is_success() {
            return Err(SheetsError(format!("token HTTP {status}")));
        }
        let access = body
            .get("access_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| SheetsError("token response missing access_token".into()))?
            .to_string();
        let expires_in = body
            .get("expires_in")
            .and_then(|v| v.as_u64())
            .unwrap_or(3600);
        let exp = now + expires_in.saturating_sub(60);
        *self.token.lock().unwrap_or_else(|e| e.into_inner()) = Some(CachedToken {
            access: access.clone(),
            exp,
        });
        Ok(access)
    }

    fn values_url(&self, range: &str, suffix: &str) -> String {
        format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}{suffix}",
            self.sheet_id,
            urlencoding::encode(range)
        )
    }

    async fn send(&self, req: reqwest::RequestBuilder) -> Result<Value, SheetsError> {
        let token = self.bearer().await?;
        let res = req
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| SheetsError(e.to_string()))?;
        let status = res.status();
        let text = res.text().await.map_err(|e| SheetsError(e.to_string()))?;
        if !status.is_success() {
            return Err(SheetsError(format!("HTTP {status}: {text}")));
        }
        if text.trim().is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_str(&text).map_err(|e| SheetsError(e.to_string()))
    }
}

#[derive(Serialize)]
struct JwtClaims {
    iss: String,
    scope: &'static str,
    aud: &'static str,
    iat: u64,
    exp: u64,
}

fn cells_of(value: &Value) -> Vec<Vec<String>> {
    let Some(rows) = value.get("values").and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    rows.iter()
        .map(|row| {
            row.as_array()
                .map(|cells| cells.iter().map(cell_string).collect())
                .unwrap_or_default()
        })
        .collect()
}

fn cell_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        other => other.to_string(),
    }
}

#[async_trait]
impl SheetsApi for GoogleSheets {
    async fn get_values(&self, range: &str) -> Result<Vec<Vec<String>>, SheetsError> {
        let req = self.http.get(self.values_url(range, ""));
        let body = self.send(req).await?;
        Ok(cells_of(&body))
    }

    async fn update_values(
        &self,
        range: &str,
        values: Vec<Vec<String>>,
    ) -> Result<(), SheetsError> {
        let req = self
            .http
            .put(self.values_url(range, "?valueInputOption=RAW"))
            .json(&serde_json::json!({ "values": values }));
        self.send(req).await?;
        Ok(())
    }

    async fn append_values(
        &self,
        range: &str,
        values: Vec<Vec<String>>,
    ) -> Result<Option<String>, SheetsError> {
        let req = self
            .http
            .post(self.values_url(
                range,
                ":append?valueInputOption=RAW&insertDataOption=INSERT_ROWS",
            ))
            .json(&serde_json::json!({ "values": values }));
        let body = self.send(req).await?;
        Ok(body
            .get("updates")
            .and_then(|u| u.get("updatedRange"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()))
    }

    async fn add_sheet(&self, title: &str) -> Result<(), SheetsError> {
        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}:batchUpdate",
            self.sheet_id
        );
        let req = self.http.post(url).json(&serde_json::json!({
            "requests": [{ "addSheet": { "properties": { "title": title } } }]
        }));
        self.send(req).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Mock {
        values: Mutex<Result<Vec<Vec<String>>, SheetsError>>,
        updates: Mutex<Vec<(String, Vec<Vec<String>>)>>,
        appends: Mutex<Vec<(String, Vec<Vec<String>>)>>,
    }

    impl Mock {
        fn ok(values: Vec<Vec<String>>) -> Self {
            Self {
                values: Mutex::new(Ok(values)),
                updates: Mutex::new(Vec::new()),
                appends: Mutex::new(Vec::new()),
            }
        }
        fn err(msg: &str) -> Self {
            Self {
                values: Mutex::new(Err(SheetsError(msg.into()))),
                updates: Mutex::new(Vec::new()),
                appends: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl SheetsApi for Mock {
        async fn get_values(&self, _range: &str) -> Result<Vec<Vec<String>>, SheetsError> {
            self.values.lock().unwrap().clone()
        }
        async fn update_values(
            &self,
            range: &str,
            values: Vec<Vec<String>>,
        ) -> Result<(), SheetsError> {
            self.updates
                .lock()
                .unwrap()
                .push((range.to_string(), values));
            Ok(())
        }
        async fn append_values(
            &self,
            range: &str,
            values: Vec<Vec<String>>,
        ) -> Result<Option<String>, SheetsError> {
            self.appends
                .lock()
                .unwrap()
                .push((range.to_string(), values));
            Ok(Some("Applications!A3:H3".into()))
        }
        async fn add_sheet(&self, _title: &str) -> Result<(), SheetsError> {
            Ok(())
        }
    }

    fn sample() -> Application {
        Application {
            timestamp: "2026-01-01T00:00:00.000Z".into(),
            x_user_id: "123".into(),
            x_username: "lethe".into(),
            wallet: "0x52908400098527886E0F7030069857D2E4169EE7".into(),
            tasks_completed: vec!["follow-x".into(), "like-rt-post".into()],
            reply_url: None,
            ua_hash: "ua".into(),
            ip_hash: "ip".into(),
        }
    }

    #[test]
    fn maps_application_to_columns_in_header_order() {
        let row = to_row(&sample());
        assert_eq!(row.len(), HEADERS.len());
        assert_eq!(
            row,
            vec![
                "2026-01-01T00:00:00.000Z",
                "123",
                "lethe",
                "0x52908400098527886E0F7030069857D2E4169EE7",
                "follow-x,like-rt-post",
                "",
                "ua",
                "ip",
            ]
        );
    }

    #[tokio::test]
    async fn appends_a_row() {
        let s = Mock::ok(vec![]);
        let app = sample();
        append_application(&s, "Applications", &app).await.unwrap();
        let calls = s.appends.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "'Applications'!A:H");
        assert_eq!(calls[0].1, vec![to_row(&app)]);
    }

    #[tokio::test]
    async fn writes_header_when_empty() {
        let s = Mock::ok(vec![]);
        assert!(ensure_header(&s, "Applications").await.unwrap().is_empty());
        assert_eq!(s.updates.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn returns_data_rows_without_header() {
        let header: Vec<String> = HEADERS.iter().map(|s| (*s).to_string()).collect();
        let s = Mock::ok(vec![header, to_row(&sample())]);
        assert_eq!(
            ensure_header(&s, "Applications").await.unwrap(),
            vec![to_row(&sample())]
        );
        assert!(s.updates.lock().unwrap().is_empty());
    }

    #[test]
    fn dedupes_x_and_wallet() {
        let rows = vec![to_row(&sample())];
        assert_eq!(
            find_duplicate(&rows, "123", "0x0000000000000000000000000000000000000001"),
            Some(Duplicate::X)
        );
        assert_eq!(
            find_duplicate(&rows, "999", &sample().wallet.to_ascii_lowercase()),
            Some(Duplicate::Wallet)
        );
        assert_eq!(
            find_duplicate(&rows, "999", "0x0000000000000000000000000000000000000001"),
            None
        );
    }

    #[test]
    fn derives_registration_number_from_appended_range() {
        assert_eq!(row_number_from_range(Some("Applications!A3:H3")), Some(2));
        assert_eq!(
            row_number_from_range(Some("'Applications'!A12:H12")),
            Some(11)
        );
        assert_eq!(row_number_from_range(None), None);
    }

    #[tokio::test]
    async fn reads_wallets_and_treats_missing_tab_as_empty() {
        let w1 = "0x52908400098527886E0F7030069857D2E4169EE7";
        let w2 = "0x1111111111111111111111111111111111111111";
        let ok = Mock::ok(vec![
            vec![w1.into()],
            vec!["not a wallet".into()],
            vec![],
            vec![format!(" {w2} ")],
        ]);
        let mut set: Vec<_> = read_wallet_tab(&ok, "GTD")
            .await
            .unwrap()
            .into_iter()
            .collect();
        set.sort();
        let mut expect = vec![w1.to_ascii_lowercase(), w2.to_ascii_lowercase()];
        expect.sort();
        assert_eq!(set, expect);

        let missing = Mock::err("Unable to parse range: 'GTD'!A:A");
        assert_eq!(read_wallet_tab(&missing, "GTD").await.unwrap().len(), 0);
    }
}
