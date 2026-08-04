//! Library surface for the app: holds every module + `AppState`. `main.rs` is
//! a thin binary that wires these into the actual `axum::serve` call; `tests/`
//! (a separate crate) imports this same lib to reach internal logic like
//! `sharing::resolve_role`.
pub mod auth;
pub mod collab;
pub mod comments;
pub mod config;
pub mod convert;
pub mod db;
pub mod email;
pub mod export;
pub mod health;
pub mod pages;
pub mod sharing;
pub mod storage;
pub mod users;
pub mod versions;
pub mod workspaces;

use std::sync::Arc;

use axum::{routing::get, Router};
use redis::Client as RedisClient;
use sqlx::PgPool;
use tower_cookies::CookieManagerLayer;
use tower_http::trace::TraceLayer;

use email::EmailClient;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub redis: RedisClient,
    pub config: Arc<config::Config>,
    pub email: Arc<EmailClient>,
    pub s3: Arc<aws_sdk_s3::Client>,
    pub s3_presign: Arc<aws_sdk_s3::Client>,
    pub docs: collab::DocRegistry,
    pub comments: comments::realtime::CommentHub,
}

/// The full route table with only the layers every caller wants. Rate limiting
/// and CORS are deliberately absent: `main.rs` wraps them around this, while
/// tests want neither (`tower_governor`'s IP extractor also has no peer address
/// to key on under `oneshot`).
pub fn build_app(state: AppState) -> Router {
    build_app_with_sensitive(auth::sensitive_router(), state)
}

/// `sensitive` is `auth::sensitive_router()`, optionally pre-layered — the
/// strict login/register rate-limit bucket has to be applied to those four
/// routes *before* they're merged with the rest of `/auth`, so it can't be
/// wrapped around the finished router the way CORS can.
pub fn build_app_with_sensitive(sensitive: Router<AppState>, state: AppState) -> Router {
    Router::new()
        .route("/health", get(health::health))
        .nest("/auth", sensitive.merge(auth::router()).merge(users::router()))
        .nest("/workspaces", workspaces::router())
        .merge(pages::router())
        .merge(convert::router())
        .merge(sharing::router())
        .merge(collab::router())
        .merge(export::router())
        .merge(versions::router())
        .merge(comments::router())
        .merge(comments::realtime::router())
        .layer(CookieManagerLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
