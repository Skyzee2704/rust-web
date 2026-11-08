use std::net::SocketAddr;

use letheans::state::AppState;
use letheans::{router, sheets};

fn load_env() {
    // .env first, then .env.local overrides (same order as Next.js).
    let _ = dotenvy::from_filename(".env");
    let _ = dotenvy::from_filename_override(".env.local");
}

#[tokio::main]
async fn main() {
    load_env();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    if std::env::args().any(|a| a == "--sheet-check") {
        let code = sheet_check().await;
        std::process::exit(code);
    }

    let state = AppState::from_env();
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let mode = if state.demo_mode { "demo mode" } else { "live" };
    tracing::info!("Letheans listening on http://{addr} ({mode})");
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .unwrap_or_else(|e| {
            eprintln!("failed to bind {addr}: {e}");
            std::process::exit(1);
        });
    axum::serve(listener, router(state))
        .await
        .unwrap_or_else(|e| {
            eprintln!("server error: {e}");
            std::process::exit(1);
        });
}

async fn sheet_check() -> i32 {
    let state = AppState::from_env();
    if !state.sheets_configured {
        eprintln!(
            "ERROR: GOOGLE_SERVICE_ACCOUNT_EMAIL / GOOGLE_PRIVATE_KEY / GOOGLE_SHEET_ID not set."
        );
        return 1;
    }
    let Some(sheets_api) = state.sheets.as_ref() else {
        eprintln!("ERROR: cannot access sheet. Did you share the sheet with the service account email as Editor?");
        return 1;
    };
    match sheets::ensure_header(sheets_api, &state.sheet_tab).await {
        Ok(rows) => {
            println!(
                "OK: sheet reachable, tab \"{}\" ready, {} application(s).",
                state.sheet_tab,
                rows.len()
            );
            0
        }
        Err(e) => {
            tracing::error!("sheet check: {e}");
            eprintln!(
                "ERROR: cannot access sheet. Did you share the sheet with the service account email as Editor?"
            );
            1
        }
    }
}
