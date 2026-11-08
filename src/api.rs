//! HTTP APIs: apply, wl-status, avatar, stats, and the X OAuth routes.

use std::collections::HashSet;

use axum::body::Body;
use axum::extract::{Query, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use http_body_util::BodyExt;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::{self, SessionUser};
use crate::config::{required_task_ids, task_by_id, TASKS};
use crate::http_util::{client_ip, same_origin, user_agent};
use crate::ratelimit::{limit_with_upstash, LimitResult};
use crate::sheets::{self, Application};
use crate::state::{AppState, TabCache};
use crate::util::{now_ms, salted_hash};
use crate::verify::verify_task;
use crate::wl::{self, ApplicantInfo, LookupQuery, WlStatus};
use crate::xlink::parse_x_status_url;

const BODY_LIMIT: usize = 256 * 1024;

pub async fn apply(State(state): State<AppState>, req: axum::extract::Request) -> Response {
    let (parts, body) = req.into_parts();
    let headers = &parts.headers;
    if !same_origin(headers) {
        return err(
            StatusCode::FORBIDDEN,
            "bad_origin",
            "Cross-origin request blocked.",
        );
    }
    let ip_hash = salted_hash(&client_ip(headers), &state.ip_salt);
    let rl = limit_apply(&state, &ip_hash).await;
    if !rl.ok {
        return err(
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limited",
            "Too many attempts. Take a breath and try again in a minute.",
        );
    }
    if state.demo_mode {
        return err(
            StatusCode::SERVICE_UNAVAILABLE,
            "demo_mode",
            "Demo mode: X login and Google Sheets env vars are not configured yet, so applications are not accepted.",
        );
    }
    let Some(session) = auth::session_user(&state, headers) else {
        return err(
            StatusCode::UNAUTHORIZED,
            "not_signed_in",
            "Connect your X account first.",
        );
    };

    let bytes = match read_limited(body, BODY_LIMIT).await {
        Ok(b) => b,
        Err(_) => return err(StatusCode::BAD_REQUEST, "bad_json", "Invalid request."),
    };
    let body: Value = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        Err(_) => return err(StatusCode::BAD_REQUEST, "bad_json", "Invalid request."),
    };
    let input = match parse_apply(&body) {
        Ok(v) => v,
        Err(message) => return err(StatusCode::BAD_REQUEST, "invalid", &message),
    };
    // Honeypot: pretend success to bots.
    if !input.website.is_empty() {
        return Json(json!({"ok": true})).into_response();
    }
    let Some(wallet) = crate::util::normalize_wallet(&input.wallet) else {
        return err(
            StatusCode::BAD_REQUEST,
            "bad_wallet",
            "That doesn't look like a valid EVM address (0x…).",
        );
    };
    let done: Vec<&str> = input
        .tasks
        .iter()
        .filter(|id| task_by_id(id).is_some())
        .map(|s| s.as_str())
        .collect();
    let required = required_task_ids();
    if required.iter().any(|id| !done.contains(id)) {
        return err(
            StatusCode::BAD_REQUEST,
            "tasks_incomplete",
            "Please complete all required quests.",
        );
    }
    let mut reply_url = String::new();
    if TASKS
        .iter()
        .any(|t| t.requires_link && done.contains(&t.id))
    {
        let Some(link) = parse_x_status_url(Some(&input.reply_url)) else {
            return err(
                StatusCode::BAD_REQUEST,
                "bad_reply_link",
                "Paste the link to your reply on X (x.com/yourname/status/…).",
            );
        };
        if link.user != "i" && link.user != session.username.to_ascii_lowercase() {
            return err(
                StatusCode::BAD_REQUEST,
                "reply_not_yours",
                &format!(
                    "That reply isn't from @{}. Paste the link to your own reply.",
                    session.username
                ),
            );
        }
        reply_url = input.reply_url.clone();
    }
    for id in &done {
        let Some(task) = task_by_id(id) else { continue };
        if !verify_task(
            &state.http,
            task,
            &session.x_id,
            state.x_verification,
            &state.x_bearer,
        )
        .await
        {
            return err(
                StatusCode::BAD_REQUEST,
                "task_unverified",
                &format!(
                    "We couldn't verify \"{}\". Finish it and try again.",
                    task.title
                ),
            );
        }
    }

    let Some(sheets_api) = state.sheets.as_ref() else {
        return err(
            StatusCode::BAD_GATEWAY,
            "sheet_error",
            "Our list keeper is napping. Please try again shortly.",
        );
    };
    let number = match write_application(
        &state, sheets_api, &session, &wallet, &done, &reply_url, &ip_hash, headers,
    )
    .await
    {
        Ok(n) => n,
        Err(response) => return response,
    };
    Json(json!({
        "ok": true,
        "wallet": wallet,
        "username": session.username,
        "number": number,
    }))
    .into_response()
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::result_large_err)]
async fn write_application(
    state: &AppState,
    sheets_api: &sheets::GoogleSheets,
    session: &SessionUser,
    wallet: &str,
    done: &[&str],
    reply_url: &str,
    ip_hash: &str,
    headers: &HeaderMap,
) -> Result<Option<i64>, Response> {
    let rows = sheets::ensure_header(sheets_api, &state.sheet_tab)
        .await
        .map_err(sheet_err)?;
    if let Some(dup) = sheets::find_duplicate(&rows, &session.x_id, wallet) {
        let message = match dup {
            sheets::Duplicate::X => "This X account has already applied.",
            sheets::Duplicate::Wallet => "This wallet has already been submitted.",
        };
        return Err(err(StatusCode::CONFLICT, "already_applied", message));
    }
    let number = sheets::append_application(
        sheets_api,
        &state.sheet_tab,
        &Application {
            timestamp: crate::util::iso_timestamp(),
            x_user_id: session.x_id.clone(),
            x_username: session.username.clone(),
            wallet: wallet.to_string(),
            tasks_completed: done.iter().map(|s| (*s).to_string()).collect(),
            reply_url: if reply_url.is_empty() {
                None
            } else {
                Some(reply_url.to_string())
            },
            ua_hash: salted_hash(&user_agent(headers), &state.ip_salt),
            ip_hash: ip_hash.to_string(),
        },
    )
    .await
    .map_err(sheet_err)?;
    Ok(number)
}

fn sheet_err(e: sheets::SheetsError) -> Response {
    tracing::error!("sheets: {e}");
    err(
        StatusCode::BAD_GATEWAY,
        "sheet_error",
        "Our list keeper is napping. Please try again shortly.",
    )
}

struct ApplyInput {
    wallet: String,
    tasks: Vec<String>,
    reply_url: String,
    website: String,
}

fn parse_apply(body: &Value) -> Result<ApplyInput, String> {
    let obj = body
        .as_object()
        .ok_or_else(|| "Invalid input.".to_string())?;
    let wallet = match obj.get("wallet") {
        None | Some(Value::Null) => return Err("Wallet is required".into()),
        Some(Value::String(s)) => {
            let t = s.trim();
            if t.is_empty() {
                return Err("Wallet is required".into());
            }
            if t.len() > 64 {
                return Err("Invalid input.".into());
            }
            t.to_string()
        }
        Some(_) => return Err("Invalid input.".into()),
    };
    let tasks_v = obj
        .get("tasks")
        .ok_or_else(|| "Invalid input.".to_string())?;
    let tasks_arr = tasks_v
        .as_array()
        .ok_or_else(|| "Invalid input.".to_string())?;
    if tasks_arr.len() > 50 {
        return Err("Invalid input.".into());
    }
    let mut tasks = Vec::with_capacity(tasks_arr.len());
    for item in tasks_arr {
        let Some(s) = item.as_str() else {
            return Err("Invalid input.".into());
        };
        if s.len() > 64 {
            return Err("Invalid input.".into());
        }
        tasks.push(s.to_string());
    }
    let reply_url = match obj.get("replyUrl") {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => {
            let t = s.trim();
            if t.len() > 300 {
                return Err("Invalid input.".into());
            }
            t.to_string()
        }
        Some(_) => return Err("Invalid input.".into()),
    };
    let website = match obj.get("website") {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => {
            if s.len() > 200 {
                return Err("Invalid input.".into());
            }
            s.clone()
        }
        Some(_) => return Err("Invalid input.".into()),
    };
    Ok(ApplyInput {
        wallet,
        tasks,
        reply_url,
        website,
    })
}

#[derive(Deserialize)]
pub struct StatusQuery {
    q: Option<String>,
    me: Option<String>,
}

pub async fn wl_status(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<StatusQuery>,
) -> Response {
    let ip_hash = salted_hash(&client_ip(&headers), &state.ip_salt);
    let rl = limit_lookup(&state, &ip_hash).await;
    if !rl.ok {
        return nostore(
            StatusCode::TOO_MANY_REQUESTS,
            json!({"ok": false, "code": "rate_limited", "message": "Too many lookups. Try again in a minute."}),
        );
    }
    if !state.sheets_configured {
        return nostore(
            StatusCode::SERVICE_UNAVAILABLE,
            json!({"ok": false, "code": "demo_mode", "message": "Status lookup isn't available yet."}),
        );
    }
    let Some(sheets_api) = state.sheets.as_ref() else {
        return nostore(
            StatusCode::BAD_GATEWAY,
            json!({"ok": false, "code": "sheet_error", "message": "Our list keeper is napping. Please try again shortly."}),
        );
    };
    let me = params.me.as_deref().is_some_and(|s| !s.is_empty());
    let result = if me {
        lookup_me(&state, sheets_api, &headers).await
    } else {
        lookup_public(&state, sheets_api, params.q.as_deref()).await
    };
    match result {
        Ok(body) => nostore(StatusCode::OK, body),
        Err(resp) => resp,
    }
}

#[allow(clippy::result_large_err)]
async fn lookup_me(
    state: &AppState,
    sheets_api: &sheets::GoogleSheets,
    headers: &HeaderMap,
) -> Result<Value, Response> {
    let Some(session) = auth::session_user(state, headers) else {
        return Ok(json!({"ok": true, "found": false}));
    };
    let rows = sheets::read_rows(sheets_api, &state.sheet_tab)
        .await
        .map_err(status_sheet_err)?;
    let Some(info) = wl::find_applicant_by_x_id(&rows, &session.x_id) else {
        return Ok(json!({"ok": true, "found": false}));
    };
    let (gtd, fcfs) = tabs(state, sheets_api).await.map_err(status_sheet_err)?;
    Ok(card_json(Some(&info), &info.wallet, &gtd, &fcfs))
}

#[allow(clippy::result_large_err)]
async fn lookup_public(
    state: &AppState,
    sheets_api: &sheets::GoogleSheets,
    raw: Option<&str>,
) -> Result<Value, Response> {
    let Some(q) = wl::parse_lookup_query(raw) else {
        return Err(nostore(
            StatusCode::BAD_REQUEST,
            json!({"ok": false, "code": "bad_query", "message": "Paste a wallet (0x…) or an X username."}),
        ));
    };
    let rows = sheets::read_rows(sheets_api, &state.sheet_tab)
        .await
        .map_err(status_sheet_err)?;
    let (gtd, fcfs) = tabs(state, sheets_api).await.map_err(status_sheet_err)?;
    if let Some(info) = wl::find_applicant(&rows, &q) {
        return Ok(card_json(Some(&info), &info.wallet, &gtd, &fcfs));
    }
    if let LookupQuery::Wallet(value) = &q {
        if gtd.contains(value) || fcfs.contains(value) {
            return Ok(card_json(None, value, &gtd, &fcfs));
        }
    }
    Ok(json!({"ok": true, "found": false}))
}

fn card_json(
    info: Option<&ApplicantInfo>,
    wallet: &str,
    gtd: &HashSet<String>,
    fcfs: &HashSet<String>,
) -> Value {
    let status: WlStatus = wl::wallet_status(wallet, gtd, fcfs);
    json!({
        "ok": true,
        "found": true,
        "number": info.map(|i| i.number),
        "username": info.map(|i| i.username.clone()),
        "wallet": wl::mask_wallet(wallet),
        "status": status.as_str(),
    })
}

async fn tabs(
    state: &AppState,
    sheets_api: &sheets::GoogleSheets,
) -> Result<(HashSet<String>, HashSet<String>), sheets::SheetsError> {
    let now = now_ms();
    if let Some(cache) = state
        .tab_cache
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
    {
        if now.saturating_sub(cache.at) < 10_000 {
            return Ok((cache.gtd.clone(), cache.fcfs.clone()));
        }
    }
    let gtd = sheets::read_wallet_tab(sheets_api, "GTD").await?;
    let fcfs = sheets::read_wallet_tab(sheets_api, "FCFS").await?;
    *state.tab_cache.lock().unwrap_or_else(|e| e.into_inner()) = Some(TabCache {
        at: now_ms(),
        gtd: gtd.clone(),
        fcfs: fcfs.clone(),
    });
    Ok((gtd, fcfs))
}

fn status_sheet_err(e: sheets::SheetsError) -> Response {
    tracing::error!("sheets: {e}");
    nostore(
        StatusCode::BAD_GATEWAY,
        json!({"ok": false, "code": "sheet_error", "message": "Our list keeper is napping. Please try again shortly."}),
    )
}

#[derive(Deserialize)]
pub struct AvatarQuery {
    u: Option<String>,
}

pub async fn avatar(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<AvatarQuery>,
) -> Response {
    let u = q.u.unwrap_or_default();
    let u = u.trim().trim_start_matches('@');
    if !username_ok(u) {
        return empty(StatusCode::BAD_REQUEST);
    }
    let ip_hash = salted_hash(&client_ip(&headers), &state.ip_salt);
    let rl = limit_lookup(&state, &ip_hash).await;
    if !rl.ok {
        return empty(StatusCode::TOO_MANY_REQUESTS);
    }
    let url = format!("https://unavatar.io/x/{}", urlencoding::encode(u));
    let res = match state
        .http
        .get(url)
        .timeout(std::time::Duration::from_secs(6))
        .header(header::ACCEPT, "image/*")
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => return empty(StatusCode::NOT_FOUND),
    };
    let ctype = res
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    if !res.status().is_success() || !is_image_type(&ctype) {
        return empty(StatusCode::NOT_FOUND);
    }
    let bytes = match res.bytes().await {
        Ok(b) => b,
        Err(_) => return empty(StatusCode::NOT_FOUND),
    };
    if bytes.len() > 2_000_000 {
        return empty(StatusCode::NOT_FOUND);
    }
    let mime = ctype.split(';').next().unwrap_or(&ctype).trim().to_string();
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime)
        .header(
            header::CACHE_CONTROL,
            "public, max-age=3600, s-maxage=86400, stale-while-revalidate=604800",
        )
        .body(Body::from(bytes))
        .unwrap_or_else(|_| empty(StatusCode::NOT_FOUND))
}

fn username_ok(u: &str) -> bool {
    static RE: std::sync::LazyLock<regex::Regex> =
        std::sync::LazyLock::new(|| regex::Regex::new(r"^[A-Za-z0-9_]{1,15}$").unwrap());
    RE.is_match(u)
}

fn is_image_type(ct: &str) -> bool {
    let ct = ct.split(';').next().unwrap_or(ct).trim();
    matches!(ct, "image/jpeg" | "image/png" | "image/webp" | "image/gif")
}

pub async fn stats(State(state): State<AppState>) -> Response {
    let headers = [(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, s-maxage=60, stale-while-revalidate=120"),
    )];
    if !state.sheets_configured {
        return (headers, Json(json!({"applicants": null, "demo": true}))).into_response();
    }
    let Some(sheets_api) = state.sheets.as_ref() else {
        return (headers, Json(json!({"applicants": null, "demo": true}))).into_response();
    };
    let now = now_ms();
    let stale = {
        let cache = state.stats_cache.lock().unwrap_or_else(|e| e.into_inner());
        now.saturating_sub(cache.at) > 60_000
    };
    if stale {
        match sheets::count_applicants(sheets_api, &state.sheet_tab).await {
            Ok(n) => {
                let mut cache = state.stats_cache.lock().unwrap_or_else(|e| e.into_inner());
                *cache = crate::state::StatsCache {
                    at: now_ms(),
                    count: Some(n),
                };
            }
            Err(e) => {
                tracing::error!("stats: {e}");
                let mut cache = state.stats_cache.lock().unwrap_or_else(|e| e.into_inner());
                cache.at = now_ms();
            }
        }
    }
    let count = state
        .stats_cache
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .count;
    (headers, Json(json!({"applicants": count}))).into_response()
}

pub async fn auth_session(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let user = auth::session_user(&state, &headers);
    (
        [(header::CACHE_CONTROL, HeaderValue::from_static("no-store"))],
        Json(auth::session_json(user.as_ref())),
    )
        .into_response()
}

#[derive(Deserialize, Default)]
pub struct RedirectForm {
    #[serde(rename = "redirectTo")]
    redirect_to: Option<String>,
    redirect: Option<String>,
}

pub async fn auth_signin(
    State(state): State<AppState>,
    headers: HeaderMap,
    req: axum::extract::Request,
) -> Response {
    let method = req.method().clone();
    let (parts, body) = req.into_parts();
    let query: RedirectForm =
        serde_urlencoded::from_str(parts.uri.query().unwrap_or("")).unwrap_or_default();
    let form = if method == axum::http::Method::POST {
        let bytes = read_limited(body, 16 * 1024).await.unwrap_or_default();
        serde_urlencoded::from_bytes::<RedirectForm>(&bytes).unwrap_or_default()
    } else {
        RedirectForm::default()
    };
    let dest = form
        .redirect_to
        .as_deref()
        .or(form.redirect.as_deref())
        .or(query.redirect_to.as_deref())
        .or(query.redirect.as_deref());
    let require_origin = method == axum::http::Method::POST;
    auth::begin_signin(&state, dest, require_origin, &headers).await
}

#[derive(Deserialize, Default)]
pub struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

pub async fn auth_callback(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<CallbackQuery>,
) -> Response {
    let cookie = auth::oauth_cookie_value(&headers);
    auth::complete_callback(
        &state,
        cookie.as_deref(),
        q.code.as_deref(),
        q.state.as_deref(),
        q.error.as_deref(),
    )
    .await
}

pub async fn auth_signout(State(state): State<AppState>, req: axum::extract::Request) -> Response {
    let (parts, body) = req.into_parts();
    let query: RedirectForm =
        serde_urlencoded::from_str(parts.uri.query().unwrap_or("")).unwrap_or_default();
    let bytes = read_limited(body, 16 * 1024).await.unwrap_or_default();
    let form = serde_urlencoded::from_bytes::<RedirectForm>(&bytes).unwrap_or_default();
    let dest = form
        .redirect_to
        .as_deref()
        .or(form.redirect.as_deref())
        .or(query.redirect_to.as_deref())
        .or(query.redirect.as_deref());
    auth::sign_out(&state, dest)
}

async fn limit_apply(state: &AppState, key: &str) -> LimitResult {
    if let Some((url, token)) = &state.upstash {
        limit_with_upstash(
            &state.http,
            url,
            token,
            "letheans:apply",
            key,
            5,
            60_000,
            &state.apply_limiter,
        )
        .await
    } else {
        state.apply_limiter.check(key)
    }
}

async fn limit_lookup(state: &AppState, key: &str) -> LimitResult {
    if let Some((url, token)) = &state.upstash {
        limit_with_upstash(
            &state.http,
            url,
            token,
            "letheans:status",
            key,
            20,
            60_000,
            &state.lookup_limiter,
        )
        .await
    } else {
        state.lookup_limiter.check(key)
    }
}

fn err(status: StatusCode, code: &str, message: &str) -> Response {
    (
        status,
        Json(json!({"ok": false, "code": code, "message": message})),
    )
        .into_response()
}

fn nostore(status: StatusCode, body: Value) -> Response {
    let mut res = (status, Json(body)).into_response();
    res.headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    res
}

async fn read_limited(body: Body, limit: usize) -> Result<axum::body::Bytes, ()> {
    let bytes = body.collect().await.map_err(|_| ())?.to_bytes();
    if bytes.len() > limit {
        return Err(());
    }
    Ok(bytes)
}

fn empty(status: StatusCode) -> Response {
    Response::builder()
        .status(status)
        .body(Body::empty())
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}
