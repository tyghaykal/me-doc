//! Library surface for the app: holds every module + `AppState`. `main.rs` is
//! a thin binary that wires these into the actual `axum::serve` call; `tests/`
//! (a separate crate) imports this same lib to reach internal logic like
//! `sharing::resolve_role`.
pub mod auth;
pub mod collab;
pub mod comments;
pub mod config;
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

use redis::Client as RedisClient;
use sqlx::PgPool;

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
