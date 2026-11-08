//! Routes, static files, and the security headers from the Next.js config.

use std::convert::Infallible;
use std::path::PathBuf;

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get};
use axum::Router;
use include_dir::{include_dir, Dir};
use tower_http::services::ServeDir;

/// Compiled into the Vercel function. The function filesystem does not contain
/// the repo, so `/logo.png` and the rest are served from here when `public/`
/// is not on disk. Locally, `public/` is served from disk first.
static EMBEDDED_ASSETS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/public");

use crate::api;
use crate::pages;
use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    let state_404 = state.clone();
    let not_found = tower::service_fn(move |_req: Request| {
        let state = state_404.clone();
        async move { Ok::<_, Infallible>(pages::not_found(&state)) }
    });
    let routes = Router::new()
        .route("/", get(pages::home))
        .route("/apply", get(pages::apply_page))
        .route("/status", get(pages::status_page))
        .route("/api/apply", axum::routing::post(api::apply))
        .route("/api/wl-status", get(api::wl_status))
        .route("/api/avatar", get(api::avatar))
        .route("/api/stats", get(api::stats))
        .route("/api/auth/session", get(api::auth_session))
        .route(
            "/api/auth/signin/twitter",
            get(api::auth_signin).post(api::auth_signin),
        )
        .route("/api/auth/callback/twitter", get(api::auth_callback))
        .route(
            "/api/auth/signout",
            get(api::auth_signout).post(api::auth_signout),
        );
    let routes = if static_dir().is_dir() {
        routes.fallback_service(ServeDir::new(static_dir()).not_found_service(not_found))
    } else {
        routes.fallback(any(serve_embedded))
    };
    routes
        .layer(middleware::from_fn(security_headers))
        .with_state(state)
}

pub fn static_dir() -> PathBuf {
    if let Ok(p) = std::env::var("STATIC_DIR") {
        return PathBuf::from(p);
    }
    let compiled = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("public");
    if compiled.exists() {
        compiled
    } else {
        PathBuf::from("public")
    }
}

fn content_type(path: &str) -> &'static str {
    match path.rsplit('.').next() {
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("svg") => "image/svg+xml",
        Some("woff2") => "font/woff2",
        Some("html") => "text/html; charset=utf-8",
        _ => "application/octet-stream",
    }
}

fn safe_asset_path(path: &str) -> Option<&str> {
    let path = path.trim_start_matches('/');
    if path.is_empty()
        || path.contains('\\')
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return None;
    }
    Some(path)
}

fn asset_response(path: &str, bytes: Vec<u8>) -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type(path))
        .header(header::CACHE_CONTROL, "public, max-age=86400")
        .body(Body::from(bytes))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

/// Disk when `public/` is present (local and Docker). Embedded bytes on Vercel,
/// where the compile-time path is not mounted in the function.
async fn serve_embedded(State(state): State<AppState>, req: Request) -> Response {
    let Some(path) = safe_asset_path(req.uri().path()) else {
        return pages::not_found(&state);
    };
    let disk = static_dir().join(path);
    if disk.is_file() {
        if let Ok(bytes) = tokio::fs::read(&disk).await {
            return asset_response(path, bytes);
        }
    }
    if let Some(file) = EMBEDDED_ASSETS.get_file(path) {
        return asset_response(path, file.contents().to_vec());
    }
    pages::not_found(&state)
}

async fn security_headers(req: Request<Body>, next: Next) -> Response {
    let mut res = next.run(req).await;
    let h = res.headers_mut();
    h.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    h.insert(
        "referrer-policy",
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    h.insert("x-frame-options", HeaderValue::from_static("DENY"));
    res
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{header, Method, Request, StatusCode};
    use http_body_util::BodyExt;
    use serde_json::Value;
    use tower::ServiceExt;

    async fn text(res: Response) -> String {
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    fn contains_text(html: &str, text: &str) -> bool {
        if html.contains(text) {
            return true;
        }
        // Askama escapes `&` as `&#38;` and `'` as `&#39;` (or the hex forms).
        let mut forms = Vec::new();
        for amp in ["&amp;", "&#38;", "&#x26;"] {
            for apos in ["&#x27;", "&#39;", "&#x27"] {
                forms.push(text.replace('&', amp).replace('\'', apos));
            }
        }
        forms.iter().any(|v| html.contains(v))
    }

    #[tokio::test]
    async fn pages_render_original_copy() {
        let app = router(AppState::demo());
        let home = app
            .clone()
            .oneshot(Request::get("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(home.status(), StatusCode::OK);
        assert_eq!(home.headers().get("x-frame-options").unwrap(), "DENY");
        let home_html = text(home).await;
        for s in [
            "Letheans — Drink. Forget. Reborn.",
            "Skip to content",
            "souls on the shore",
            "a river of forgetting",
            "Join the whitelist",
            "Check WL Status",
            "@LetheansNFT",
            "Drink from the river",
            "Forget everything",
            "Reborn in Elysium",
            "2222 souls",
            "A second life",
            "New faces, no names",
            "The lore",
            "Born from the river of forgetting.",
            "The River Lethe",
            "Not every soul may drink",
            "What rises from the water",
            "The First",
            "A new face",
            "Meet the legends",
            "Four woke up",
            "early.",
            "drag or flick",
            "Still under the water",
            "2218",
            "Wake one up",
            "How to join",
            "Four steps before you drink.",
            "Connect X",
            "Finish the quests",
            "Drop a wallet",
            "Wait for the judgment",
            "Apply for WL",
            "The rebirth",
            "The river opens in",
            "Rebirth ticket",
            "No. 0001",
            "One Lethean per ticket",
            "WL price",
            "GTD → FCFS → Public",
            "Departure (UTC)",
            "To be announced",
            "Get on the list",
            "Questions from the riverbank.",
            "What are Letheans?",
            "How do I avoid scams?",
            "Forget the noise.",
            "Keep this.",
            "Whitelist is open while the petals fall.",
            "Apply for whitelist",
            "nothing here is financial advice",
            "drink responsibly",
            "2,222",
        ] {
            assert!(contains_text(&home_html, s), "home missing: {s}");
        }

        let apply = app
            .clone()
            .oneshot(Request::get("/apply").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(apply.status(), StatusCode::OK);
        let apply_html = text(apply).await;
        for s in [
            "Apply for whitelist · Letheans",
            "Get on the",
            "list.",
            "Connect X, finish the quests, drop your wallet.",
            "How it works",
            "required quests",
            "Demo mode: env not configured.",
            "Connect your X account",
            "Connect X (not configured)",
            "Follow @LetheansNFT on X",
            "Like & repost the WL post",
            "Reply & tag 3 friends",
            "Link to your reply",
            "Your EVM wallet",
            "Submit application",
            "Use a wallet you control. No exchange addresses.",
        ] {
            assert!(contains_text(&apply_html, s), "apply missing: {s}");
        }

        let status = app
            .clone()
            .oneshot(Request::get("/status").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(status.status(), StatusCode::OK);
        let status_html = text(status).await;
        for s in [
            "Check WL status · Letheans",
            "Check your",
            "status.",
            "Paste your wallet or X username. Your card shows Pending review until the judgment, then GTD or FCFS.",
            "Wallet or X username",
            "0x… or @yourname",
        ] {
            assert!(contains_text(&status_html, s), "status missing: {s}");
        }
    }

    #[test]
    fn embedded_assets_include_the_public_files() {
        assert!(EMBEDDED_ASSETS.get_file("logo.png").is_some());
        assert!(EMBEDDED_ASSETS.get_file("css/app.css").is_some());
        assert!(EMBEDDED_ASSETS.get_file("js/card.js").is_some());
        assert!(EMBEDDED_ASSETS.get_file("nft/base.png").is_some());
        assert!(EMBEDDED_ASSETS.get_file("fonts/inter.woff2").is_some());
    }

    #[tokio::test]
    async fn static_assets_and_missing_page() {
        let app = router(AppState::demo());
        for path in [
            "/logo.png",
            "/logo-arch.png",
            "/icon.png",
            "/apple-icon.png",
            "/nft/sample-1.webp",
            "/nft/base.png",
            "/letheans-knot.svg",
            "/css/app.css",
            "/js/app.js",
            "/js/card.js",
        ] {
            let res = app
                .clone()
                .oneshot(Request::get(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(res.status(), StatusCode::OK, "{path}");
        }
        let missing = app
            .oneshot(Request::get("/no-such-page").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(missing.status(), StatusCode::NOT_FOUND);
        let html = text(missing).await;
        assert!(contains_text(&html, "This page could not be found."));
    }

    #[tokio::test]
    async fn demo_mode_api_matches_original() {
        let app = router(AppState::demo());
        let blocked = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/apply")
                    .header(header::HOST, "localhost:3000")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(blocked.status(), StatusCode::FORBIDDEN);
        let v: Value = serde_json::from_str(&text(blocked).await).unwrap();
        assert_eq!(v["code"], "bad_origin");

        let demo = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/apply")
                    .header(header::HOST, "localhost:3000")
                    .header(header::ORIGIN, "http://localhost:3000")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"wallet":"0x52908400098527886E0F7030069857D2E4169EE7","tasks":["follow-x"]}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(demo.status(), StatusCode::SERVICE_UNAVAILABLE);
        let v: Value = serde_json::from_str(&text(demo).await).unwrap();
        assert_eq!(v["code"], "demo_mode");
        assert!(v["message"].as_str().unwrap().contains("Demo mode"));

        let wl = app
            .clone()
            .oneshot(
                Request::get("/api/wl-status?q=@LetheansNFT")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(wl.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(wl.headers().get(header::CACHE_CONTROL).unwrap(), "no-store");
        let v: Value = serde_json::from_str(&text(wl).await).unwrap();
        assert_eq!(v["message"], "Status lookup isn't available yet.");

        let me = app
            .clone()
            .oneshot(
                Request::get("/api/wl-status?me=1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(me.status(), StatusCode::SERVICE_UNAVAILABLE);

        let stats = app
            .clone()
            .oneshot(Request::get("/api/stats").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(stats.status(), StatusCode::OK);
        let v: Value = serde_json::from_str(&text(stats).await).unwrap();
        assert!(v["applicants"].is_null());
        assert_eq!(v["demo"], true);

        let avatar = app
            .clone()
            .oneshot(
                Request::get("/api/avatar?u=bad!")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(avatar.status(), StatusCode::BAD_REQUEST);

        let session = app
            .oneshot(
                Request::get("/api/auth/session")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(session.status(), StatusCode::OK);
        assert_eq!(text(session).await, "null");
    }

    #[tokio::test]
    async fn apply_rate_limit_counts_even_in_demo() {
        let app = router(AppState::demo());
        let mut saw_429 = false;
        for _ in 0..6 {
            let res = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(Method::POST)
                        .uri("/api/apply")
                        .header(header::HOST, "localhost:3000")
                        .header(header::ORIGIN, "http://localhost:3000")
                        .header("x-forwarded-for", "203.0.113.9")
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Body::from("{}"))
                        .unwrap(),
                )
                .await
                .unwrap();
            if res.status() == StatusCode::TOO_MANY_REQUESTS {
                saw_429 = true;
                let v: Value = serde_json::from_str(&text(res).await).unwrap();
                assert_eq!(v["code"], "rate_limited");
            }
        }
        assert!(saw_429);
    }
}
