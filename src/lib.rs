//! Letheans website: Axum server, Google Sheets whitelist, X OAuth 2.0.

pub mod api;
pub mod auth;
pub mod config;
pub mod deck;
pub mod http_util;
pub mod pages;
pub mod ratelimit;
pub mod router;
pub mod sheets;
pub mod state;
pub mod util;
pub mod verify;
pub mod wl;
pub mod xlink;

pub use router::router;
pub use state::AppState;
