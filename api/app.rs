//! Vercel Function entrypoint. `cargo run` still uses `src/main.rs`.
//!
//! The official Rust runtime builds every `api/*.rs` binary listed in
//! `Cargo.toml`. `vercel.json` rewrites each request to `/api/app`, and the
//! runtime passes the original path through, matching the Axum example at
//! https://github.com/vercel/examples/tree/main/rust/axum.

use tower::ServiceBuilder;
use vercel_runtime::axum::VercelLayer;
use vercel_runtime::Error;

#[tokio::main]
async fn main() -> Result<(), Error> {
    letheans::load_dotenv();
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .try_init();

    let state = letheans::AppState::from_env();
    let app = ServiceBuilder::new()
        .layer(VercelLayer::new())
        .service(letheans::router(state));
    vercel_runtime::run(app).await
}
