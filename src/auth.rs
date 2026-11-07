//! X OAuth 2.0 (PKCE) and an HMAC-signed session cookie.
//!
//! Callback path matches Auth.js: `{AUTH_URL}/api/auth/callback/twitter`.
//! Scopes: `users.read tweet.read offline.access`. Session lifetime: 7 days.

use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::http_util::{read_cookie, safe_redirect, same_origin};
use crate::state::AppState;
use crate::util::now_secs;

type HmacSha256 = Hmac<Sha256>;

const SESSION_COOKIE: &str = "letheans_session";
const OAUTH_COOKIE: &str = "letheans_oauth";
const SESSION_TTL: u64 = 60 * 60 * 24 * 7;
const OAUTH_TTL: u64 = 60 * 10;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionUser {
    #[serde(rename = "xId")]
    pub x_id: String,
    pub username: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub image: Option<String>,
    pub exp: u64,
}

#[derive(Serialize, Deserialize)]
struct OauthBlob {
    state: String,
    verifier: String,
    redirect: String,
    exp: u64,
}

pub fn oauth_cookie_value(headers: &HeaderMap) -> Option<String> {
    read_cookie(headers, OAUTH_COOKIE)
}

pub fn session_user(state: &AppState, headers: &HeaderMap) -> Option<SessionUser> {
    let raw = read_cookie(headers, SESSION_COOKIE)?;
    let bytes = open(&state.auth_secret, &raw)?;
    let user: SessionUser = serde_json::from_slice(&bytes).ok()?;
    if user.exp <= now_secs() || user.x_id.is_empty() || user.username.is_empty() {
        return None;
    }
    Some(user)
}

pub async fn begin_signin(
    state: &AppState,
    redirect_to: Option<&str>,
    require_origin: bool,
    headers: &HeaderMap,
) -> Response {
    if require_origin && !same_origin(headers) {
        return (StatusCode::FORBIDDEN, "Cross-origin request blocked.").into_response();
    }
    if !state.auth_configured {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "X login is not configured.",
        )
            .into_response();
    }
    let redirect = safe_redirect(redirect_to, "/apply");
    let verifier = random_b64(32);
    let challenge = pkce_challenge(&verifier);
    let oauth_state = random_hex(16);
    let blob = OauthBlob {
        state: oauth_state.clone(),
        verifier,
        redirect,
        exp: now_secs() + OAUTH_TTL,
    };
    let sealed = match serde_json::to_vec(&blob)
        .ok()
        .map(|b| seal(&state.auth_secret, &b))
    {
        Some(s) => s,
        None => return (StatusCode::INTERNAL_SERVER_ERROR, "auth error").into_response(),
    };
    let redirect_uri = redirect_uri(state);
    let url = format!(
        "https://twitter.com/i/oauth2/authorize?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&code_challenge={}&code_challenge_method=S256",
        urlencoding::encode(&state.x_client_id),
        urlencoding::encode(&redirect_uri),
        urlencoding::encode("users.read tweet.read offline.access"),
        urlencoding::encode(&oauth_state),
        urlencoding::encode(&challenge),
    );
    let mut res = Redirect::to(&url).into_response();
    append_cookie(
        &mut res,
        &cookie_str(
            OAUTH_COOKIE,
            &sealed,
            OAUTH_TTL as i64,
            cookie_secure(state),
        ),
    );
    res
}

/// Completes the OAuth callback. `oauth_cookie` is the raw `letheans_oauth` value.
pub async fn complete_callback(
    state: &AppState,
    oauth_cookie: Option<&str>,
    code: Option<&str>,
    state_q: Option<&str>,
    error: Option<&str>,
) -> Response {
    let secure = cookie_secure(state);
    let fail = |err: &str| {
        let mut res = Redirect::to(&format!("/apply?error={err}")).into_response();
        append_cookie(&mut res, &clear_cookie(OAUTH_COOKIE, secure));
        res
    };
    if let Some(err) = error.filter(|s| !s.is_empty()) {
        let code = if err == "access_denied" {
            "access_denied"
        } else {
            "oauth"
        };
        return fail(code);
    }
    let (Some(code), Some(state_q), Some(raw)) = (code, state_q, oauth_cookie) else {
        return fail("oauth");
    };
    let Some(bytes) = open(&state.auth_secret, raw) else {
        return fail("state");
    };
    let Ok(blob) = serde_json::from_slice::<OauthBlob>(&bytes) else {
        return fail("state");
    };
    if blob.exp <= now_secs() || !ct_eq(&blob.state, state_q) {
        return fail("state");
    }
    let redirect_uri = redirect_uri(state);
    let token = match exchange_code(state, code, &redirect_uri, &blob.verifier).await {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!("x token exchange failed: {e}");
            return fail("oauth");
        }
    };
    let profile = match fetch_me(state, &token).await {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("x user lookup failed: {e}");
            return fail("oauth");
        }
    };
    let session = SessionUser {
        x_id: profile.id,
        username: profile.username,
        name: profile.name,
        image: profile.image,
        exp: now_secs() + SESSION_TTL,
    };
    let Ok(payload) = serde_json::to_vec(&session) else {
        return fail("oauth");
    };
    let sealed = seal(&state.auth_secret, &payload);
    let dest = safe_redirect(Some(&blob.redirect), "/apply");
    let mut res = Redirect::to(&dest).into_response();
    append_cookie(
        &mut res,
        &cookie_str(SESSION_COOKIE, &sealed, SESSION_TTL as i64, secure),
    );
    append_cookie(&mut res, &clear_cookie(OAUTH_COOKIE, secure));
    res
}

pub fn sign_out(state: &AppState, redirect_to: Option<&str>) -> Response {
    let dest = safe_redirect(redirect_to, "/");
    let mut res = Redirect::to(&dest).into_response();
    append_cookie(
        &mut res,
        &clear_cookie(SESSION_COOKIE, cookie_secure(state)),
    );
    res
}

pub fn session_json(user: Option<&SessionUser>) -> serde_json::Value {
    match user {
        Some(u) => serde_json::json!({
            "user": {
                "xId": u.x_id,
                "username": u.username,
                "name": u.name,
                "image": u.image,
            }
        }),
        None => serde_json::Value::Null,
    }
}

fn redirect_uri(state: &AppState) -> String {
    format!(
        "{}/api/auth/callback/twitter",
        state.auth_url.trim_end_matches('/')
    )
}

fn cookie_secure(state: &AppState) -> bool {
    state.auth_url.starts_with("https://") || state.site_url.starts_with("https://")
}

async fn exchange_code(
    state: &AppState,
    code: &str,
    redirect_uri: &str,
    verifier: &str,
) -> Result<String, String> {
    let basic = base64::engine::general_purpose::STANDARD
        .encode(format!("{}:{}", state.x_client_id, state.x_client_secret));
    let res = state
        .http
        .post("https://twitter.com/i/oauth2/token")
        .header(header::AUTHORIZATION, format!("Basic {basic}"))
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("code_verifier", verifier),
        ])
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = res.status();
    let body: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!("HTTP {status}"));
    }
    body.get("access_token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "missing access_token".into())
}

struct XProfile {
    id: String,
    username: String,
    name: Option<String>,
    image: Option<String>,
}

async fn fetch_me(state: &AppState, access: &str) -> Result<XProfile, String> {
    let res = state
        .http
        .get("https://api.twitter.com/2/users/me")
        .query(&[("user.fields", "profile_image_url")])
        .bearer_auth(access)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = res.status();
    let body: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!("HTTP {status}"));
    }
    let data = body.get("data").cloned().unwrap_or(serde_json::Value::Null);
    let id = data
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let username = data
        .get("username")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if id.is_empty() || username.is_empty() {
        return Err("profile missing id".into());
    }
    Ok(XProfile {
        id,
        username,
        name: data
            .get("name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        image: data
            .get("profile_image_url")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
    })
}

fn seal(secret: &str, payload: &[u8]) -> String {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("hmac accepts any key length");
    mac.update(payload);
    let sig = mac.finalize().into_bytes();
    format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(payload),
        URL_SAFE_NO_PAD.encode(sig)
    )
}

fn open(secret: &str, token: &str) -> Option<Vec<u8>> {
    let (p, s) = token.split_once('.')?;
    let payload = URL_SAFE_NO_PAD.decode(p).ok()?;
    let sig = URL_SAFE_NO_PAD.decode(s).ok()?;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).ok()?;
    mac.update(&payload);
    mac.verify_slice(&sig).ok()?;
    Some(payload)
}

fn pkce_challenge(verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(hasher.finalize())
}

fn random_b64(n: usize) -> String {
    let mut buf = vec![0u8; n];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut buf);
    URL_SAFE_NO_PAD.encode(buf)
}

fn random_hex(n: usize) -> String {
    let mut buf = vec![0u8; n];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut buf);
    hex::encode(buf)
}

fn ct_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut d = 0u8;
    for (x, y) in a.iter().zip(b) {
        d |= x ^ y;
    }
    d == 0
}

pub fn cookie_str(name: &str, value: &str, max_age: i64, secure: bool) -> String {
    let secure = if secure { "; Secure" } else { "" };
    format!("{name}={value}; HttpOnly; Path=/; SameSite=Lax; Max-Age={max_age}{secure}")
}

fn clear_cookie(name: &str, secure: bool) -> String {
    cookie_str(name, "", 0, secure)
}

fn append_cookie(res: &mut Response, cookie: &str) {
    if let Ok(v) = HeaderValue::from_str(cookie) {
        res.headers_mut().append(header::SET_COOKIE, v);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seal_roundtrip() {
        let payload = br#"{"xId":"1","username":"a","exp":9999999999}"#;
        let token = seal("secret", payload);
        assert_eq!(open("secret", &token).as_deref(), Some(payload.as_slice()));
        assert!(open("other", &token).is_none());
    }
}
