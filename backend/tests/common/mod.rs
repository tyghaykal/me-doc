//! Shared helpers for the integration tests. Each test binary includes this
//! module and uses a different subset of it.
#![allow(dead_code)]

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::header::{CONTENT_TYPE, SET_COOKIE};
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use me_doc_backend::{collab, comments, config::Config, email::EmailClient, storage, AppState};
use redis::Client as RedisClient;
use serde_json::{json, Value};
use sqlx::PgPool;
use tokio::task::JoinHandle;
use tower::ServiceExt;
use uuid::Uuid;

const MAILPIT_API: &str = "http://mailpit:8025/api/v1";

// ---------------------------------------------------------------------------
// Direct row insertion
// ---------------------------------------------------------------------------

pub async fn insert_user(pool: &PgPool, email: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("insert into users (id, email, password_hash) values ($1, $2, 'x')")
        .bind(id)
        .bind(email)
        .execute(pool)
        .await
        .unwrap();
    id
}

pub async fn insert_workspace(pool: &PgPool, owner_id: Uuid, slug: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("insert into workspaces (id, name, slug, owner_id) values ($1, $2, $3, $4)")
        .bind(id)
        .bind(slug)
        .bind(slug)
        .bind(owner_id)
        .execute(pool)
        .await
        .unwrap();
    id
}

pub async fn insert_page(
    pool: &PgPool,
    workspace_id: Uuid,
    parent: Option<Uuid>,
    created_by: Uuid,
    slug: &str,
) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "insert into pages (id, workspace_id, parent_page_id, slug, created_by) values ($1, $2, $3, $4, $5)",
    )
    .bind(id)
    .bind(workspace_id)
    .bind(parent)
    .bind(slug)
    .bind(created_by)
    .execute(pool)
    .await
    .unwrap();
    id
}

pub async fn grant_page(pool: &PgPool, page_id: Uuid, principal_id: Uuid, role: &str) {
    sqlx::query(
        "insert into permissions (subject_type, subject_id, principal_type, principal_id, role)
         values ('page', $1, 'user', $2, $3)",
    )
    .bind(page_id)
    .bind(principal_id)
    .bind(role)
    .execute(pool)
    .await
    .unwrap();
}

pub async fn add_member(pool: &PgPool, workspace_id: Uuid, user_id: Uuid, role: &str) {
    sqlx::query("insert into workspace_members (workspace_id, user_id, role) values ($1, $2, $3)")
        .bind(workspace_id)
        .bind(user_id)
        .bind(role)
        .execute(pool)
        .await
        .unwrap();
}

pub async fn grant_page_link(pool: &PgPool, page_id: Uuid, link_token: &str, role: &str) {
    sqlx::query(
        "insert into permissions (subject_type, subject_id, principal_type, link_token, role)
         values ('page', $1, 'link', $2, $3)",
    )
    .bind(page_id)
    .bind(link_token)
    .bind(role)
    .execute(pool)
    .await
    .unwrap();
}

// ---------------------------------------------------------------------------
// App wiring
// ---------------------------------------------------------------------------

/// Real `AppState` against the live compose stack (Redis/MinIO/Mailpit reached
/// by service name), but with the ephemeral per-test pool `#[sqlx::test]`
/// hands us in place of `db::create_pool`.
pub async fn test_state(pool: PgPool) -> AppState {
    let config = Config::from_env().expect("test env must carry the backend's env vars");
    let redis = RedisClient::open(config.redis_url.clone()).unwrap();
    let email = EmailClient::new(
        &config.smtp_host,
        config.smtp_port,
        &config.smtp_from,
        &config.product_name,
        &config.frontend_origin,
    )
    .unwrap();
    let s3 = storage::build_client(&config);
    let s3_presign = storage::build_presign_client(&config);

    AppState {
        db: pool,
        redis,
        config: Arc::new(config),
        email: Arc::new(email),
        s3: Arc::new(s3),
        s3_presign: Arc::new(s3_presign),
        docs: collab::new_registry(),
        comments: comments::realtime::new_hub(),
    }
}

/// A real bound port, for the two websocket endpoints — `oneshot` can't
/// perform an Upgrade handshake.
pub async fn spawn_real_server(state: AppState) -> (SocketAddr, JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        axum::serve(listener, me_doc_backend::build_app(state))
            .await
            .unwrap();
    });
    (addr, handle)
}

// ---------------------------------------------------------------------------
// HTTP
// ---------------------------------------------------------------------------

pub fn post_json(uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

/// Runs a request and returns the status, the `refresh_token` cookie if the
/// response set one, and the body parsed as JSON (`Value::Null` if it isn't).
pub async fn send(
    app: &axum::Router,
    req: Request<Body>,
) -> (StatusCode, Option<String>, Value) {
    let res = app.clone().oneshot(req).await.unwrap();
    let (parts, body) = res.into_parts();
    let cookie = parts
        .headers
        .get_all(SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find(|v| v.starts_with("refresh_token="))
        .map(|v| v.split(';').next().unwrap().to_string());
    let bytes = body.collect().await.unwrap().to_bytes();
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (parts.status, cookie, json)
}

pub struct AuthedClient {
    pub access_token: String,
    pub refresh_cookie: String,
}

/// Full register -> OTP -> verify -> login -> OTP -> verify round trip. The
/// OTP only exists in plaintext inside the email (Redis stores a hash), so
/// this really does go through Mailpit.
pub async fn register_and_login(app: &axum::Router, email: &str, password: &str) -> AuthedClient {
    let (status, _, body) = send(
        app,
        post_json("/auth/register", json!({ "email": email, "password": password })),
    )
    .await;
    assert!(status.is_success(), "register failed: {status} {body}");

    let code = mailpit_latest_code(email).await;
    // Otherwise the login poll below can race and read this same message back.
    mailpit_clear(email).await;

    let (status, _, body) = send(
        app,
        post_json("/auth/register/verify", json!({ "email": email, "code": code })),
    )
    .await;
    assert!(status.is_success(), "register verify failed: {status} {body}");

    let (status, _, body) = send(
        app,
        post_json("/auth/login", json!({ "email": email, "password": password })),
    )
    .await;
    assert!(status.is_success(), "login failed: {status} {body}");

    let code = mailpit_latest_code(email).await;
    mailpit_clear(email).await;

    let (status, cookie, body) = send(
        app,
        post_json("/auth/login/verify", json!({ "email": email, "code": code })),
    )
    .await;
    assert!(status.is_success(), "login verify failed: {status} {body}");

    AuthedClient {
        access_token: body["access_token"].as_str().unwrap().to_string(),
        refresh_cookie: cookie.expect("login verify must set a refresh_token cookie"),
    }
}

// ---------------------------------------------------------------------------
// Mailpit
// ---------------------------------------------------------------------------

async fn mailpit_search(to: &str) -> Value {
    reqwest::Client::new()
        .get(format!("{MAILPIT_API}/search"))
        .query(&[("query", format!("to:{to}"))])
        .send()
        .await
        .expect("mailpit must be reachable at mailpit:8025")
        .json()
        .await
        .unwrap()
}

/// Newest message sent to `to`, as its 6-digit OTP. SMTP delivery isn't
/// synchronous with Mailpit's search index, so this polls.
pub async fn mailpit_latest_code(to: &str) -> String {
    for _ in 0..25 {
        let messages = mailpit_search(to).await;
        if let Some(id) = messages["messages"][0]["ID"].as_str() {
            let msg: Value = reqwest::get(format!("{MAILPIT_API}/message/{id}"))
                .await
                .unwrap()
                .json()
                .await
                .unwrap();
            if let Some(code) = msg["Text"].as_str().and_then(six_digits) {
                return code;
            }
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    panic!("no OTP email for {to} arrived in Mailpit within 5s");
}

/// Drops every message addressed to `to`, so a later poll can't read a stale
/// (already-consumed) code.
pub async fn mailpit_clear(to: &str) {
    let found = mailpit_search(to).await;
    let ids: Vec<&str> = found["messages"]
        .as_array()
        .map(|m| m.iter().filter_map(|m| m["ID"].as_str()).collect())
        .unwrap_or_default();
    if ids.is_empty() {
        return;
    }
    reqwest::Client::new()
        .delete(format!("{MAILPIT_API}/messages"))
        .json(&json!({ "IDs": ids }))
        .send()
        .await
        .unwrap();
}

/// First run of exactly six digits — the OTP, as `\b\d{6}\b` would find it.
fn six_digits(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if !bytes[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i - start == 6 {
            return Some(text[start..i].to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    #[test]
    fn six_digits_skips_shorter_and_longer_runs() {
        assert_eq!(super::six_digits("year 2026, code 481902 ok"), Some("481902".into()));
        assert_eq!(super::six_digits("1234567 is too long"), None);
    }
}
