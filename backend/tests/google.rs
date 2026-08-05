//! Google OAuth route behavior. A full code-exchange round trip needs real
//! Google credentials (a browser + a configured OAuth client), which CI and
//! dev stacks don't have, so the externally-observable contract tested here
//! is: (1) with `GOOGLE_CLIENT_*` unset, both routes fail closed (503, no
//! panic, no redirect to Google); (2) with Google configured, a callback
//! whose `state` never started a flow is rejected before any Google call.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::*;
use sqlx::PgPool;
use std::sync::Arc;

fn get(uri: &str) -> Request<Body> {
    Request::builder().method("GET").uri(uri).body(Body::empty()).unwrap()
}

/// With no Google client configured, starting the flow is refused with a
/// clear 503 — the frontend hides the button when the backend says so.
/// Explicitly clears the config regardless of ambient env so the test is
/// deterministic on machines where Google IS configured.
#[sqlx::test]
async fn google_login_fails_closed_when_not_configured(pool: PgPool) {
    let mut state = test_state(pool).await;
    let mut cfg = (*state.config).clone();
    cfg.google_client_id = None;
    cfg.google_client_secret = None;
    cfg.google_redirect_uri = None;
    state.config = Arc::new(cfg);
    let app = me_doc_backend::build_app(state);

    let (status, _, body) = send(&app, get("/auth/google/login")).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body}");
    assert_eq!(body["message"], "Google sign-in is not configured");
}

/// A callback that never started a flow (no valid `state`) must be rejected
/// before any Google call — the Redis verifier lookup fails first. This uses
/// a Google-configured state so it exercises the CSRF guard, not the
/// "not configured" shortcut.
#[sqlx::test]
async fn google_callback_rejects_unknown_state(pool: PgPool) {
    let mut state = test_state(pool).await;
    // Simulate an operator who configured Google.
    let mut cfg = (*state.config).clone();
    cfg.google_client_id = Some("test-client-id".into());
    cfg.google_client_secret = Some("test-client-secret".into());
    cfg.google_redirect_uri = Some("http://localhost:8080/auth/google/callback".into());
    state.config = Arc::new(cfg);
    let app = me_doc_backend::build_app(state);

    // Forged state: never issued by /auth/google/login, so no verifier in Redis.
    let (status, _, body) = send(&app, get("/auth/google/callback?code=x&state=forged")).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{body}");
    assert_eq!(body["message"], "invalid or expired session");
}
