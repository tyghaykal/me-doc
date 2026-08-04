//! Integration tests for the register/login OTP flow and refresh-token
//! lifecycle. Each `#[sqlx::test]` gets its own freshly-migrated database, but
//! Redis and Mailpit are *shared* across the whole suite — so every test mints
//! a unique email (`uniq`) to keep OTP keys and inbox searches from colliding.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::*;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

const PASSWORD: &str = "correct-horse-battery";

fn uniq(tag: &str) -> String {
    format!("{tag}-{}@example.com", Uuid::new_v4().simple())
}

/// `/auth/refresh` and `/auth/logout` take no body and authenticate purely off
/// the `refresh_token` cookie.
fn post_with_cookie(uri: &str, cookie: &str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("cookie", cookie)
        .body(Body::empty())
        .unwrap()
}

/// Any code that isn't the real one, without relying on a hardcoded guess
/// happening to differ from a randomly generated OTP.
fn wrong_code(real: &str) -> &'static str {
    if real == "000000" {
        "111111"
    } else {
        "000000"
    }
}

/// The whole register -> OTP -> verify -> login -> OTP -> verify chain driven
/// through raw HTTP, asserting the intermediate responses rather than just the
/// final token: register/login only acknowledge that a code was sent, and both
/// verify steps mint a session.
#[sqlx::test]
async fn register_login_flow_returns_tokens_at_each_verify(pool: PgPool) {
    let state = common::test_state(pool).await;
    let app = me_doc_backend::build_app(state);
    let email = uniq("flow");

    let (status, cookie, body) = send(
        &app,
        post_json("/auth/register", json!({ "email": email, "password": PASSWORD })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["message"], "verification code sent");
    assert!(cookie.is_none(), "register must not open a session");

    let code = mailpit_latest_code(&email).await;
    mailpit_clear(&email).await;

    let (status, cookie, body) = send(
        &app,
        post_json("/auth/register/verify", json!({ "email": email, "code": code })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(!body["access_token"].as_str().unwrap().is_empty());
    assert_eq!(body["user"]["email"], email.as_str());
    // Verifying registration also provisions the personal workspace.
    assert!(body["workspace"]["id"].is_string());
    assert!(cookie.is_some(), "register verify must set a refresh cookie");

    let (status, _, body) = send(
        &app,
        post_json("/auth/login", json!({ "email": email, "password": PASSWORD })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let code = mailpit_latest_code(&email).await;
    mailpit_clear(&email).await;

    let (status, cookie, body) = send(
        &app,
        post_json("/auth/login/verify", json!({ "email": email, "code": code })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(!body["access_token"].as_str().unwrap().is_empty());
    assert!(cookie.is_some());
}

/// The shared helper's happy path, which every other test binary leans on.
#[sqlx::test]
async fn register_and_login_helper_yields_a_usable_session(pool: PgPool) {
    let state = common::test_state(pool).await;
    let app = me_doc_backend::build_app(state);

    let client = register_and_login(&app, &uniq("helper"), PASSWORD).await;

    assert!(!client.access_token.is_empty());
    assert!(client.refresh_cookie.starts_with("refresh_token="));
}

/// A verify with the wrong code is rejected, and the real code still works
/// afterwards — a bad guess must not consume the pending OTP.
#[sqlx::test]
async fn wrong_otp_is_rejected_without_consuming_the_code(pool: PgPool) {
    let state = common::test_state(pool).await;
    let app = me_doc_backend::build_app(state);
    let email = uniq("badotp");

    let (status, _, body) = send(
        &app,
        post_json("/auth/register", json!({ "email": email, "password": PASSWORD })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let code = mailpit_latest_code(&email).await;
    mailpit_clear(&email).await;

    let (status, _, body) = send(
        &app,
        post_json(
            "/auth/register/verify",
            json!({ "email": email, "code": wrong_code(&code) }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["message"], "invalid or expired code");

    let (status, _, body) = send(
        &app,
        post_json("/auth/register/verify", json!({ "email": email, "code": code })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

/// `otp::issue_otp` parks a 60s cooldown key per (purpose, email), so asking
/// for a second login code straight away is refused with 429.
///
/// The same guard on `register` can't be reached through the HTTP surface: a
/// second `/auth/register` for the same address short-circuits on `EmailTaken`
/// before `issue_otp` runs, so login is the only observable cooldown path.
#[sqlx::test]
async fn second_otp_request_hits_the_cooldown(pool: PgPool) {
    let state = common::test_state(pool).await;
    let app = me_doc_backend::build_app(state);
    let email = uniq("cooldown");

    // Leaves a fresh login cooldown behind.
    register_and_login(&app, &email, PASSWORD).await;

    let (status, _, body) = send(
        &app,
        post_json("/auth/login", json!({ "email": email, "password": PASSWORD })),
    )
    .await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS, "{body}");
    assert_eq!(body["message"], "please wait before requesting another code");
}

/// Refresh rotates: it returns a new access token, hands back a *different*
/// refresh cookie, and the presented cookie is burned in the process.
#[sqlx::test]
async fn refresh_rotates_the_access_and_refresh_tokens(pool: PgPool) {
    let state = common::test_state(pool).await;
    let app = me_doc_backend::build_app(state);
    let email = uniq("refresh");

    let client = register_and_login(&app, &email, PASSWORD).await;

    let (status, cookie, body) =
        send(&app, post_with_cookie("/auth/refresh", &client.refresh_cookie)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["user"]["email"], email.as_str());

    let rotated = cookie.expect("refresh must set a new refresh_token cookie");
    assert_ne!(rotated, client.refresh_cookie);

    let (status, _, _) =
        send(&app, post_with_cookie("/auth/refresh", &client.refresh_cookie)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "the old cookie must be single-use");
}

/// Refreshing without a cookie at all is unauthorized rather than a 500.
#[sqlx::test]
async fn refresh_without_a_cookie_is_unauthorized(pool: PgPool) {
    let state = common::test_state(pool).await;
    let app = me_doc_backend::build_app(state);

    let req = Request::builder()
        .method("POST")
        .uri("/auth/refresh")
        .body(Body::empty())
        .unwrap();
    let (status, _, _) = send(&app, req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

/// Logout revokes the refresh token server-side, so the cookie a client may
/// still be holding is worthless.
#[sqlx::test]
async fn logout_invalidates_the_refresh_token(pool: PgPool) {
    let state = common::test_state(pool).await;
    let app = me_doc_backend::build_app(state);

    let client = register_and_login(&app, &uniq("logout"), PASSWORD).await;

    let (status, _, body) =
        send(&app, post_with_cookie("/auth/logout", &client.refresh_cookie)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["message"], "logged out");

    let (status, _, _) =
        send(&app, post_with_cookie("/auth/refresh", &client.refresh_cookie)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

/// Registering an address that already has a row is a 409, whether or not that
/// account ever finished verification.
#[sqlx::test]
async fn duplicate_email_registration_is_rejected(pool: PgPool) {
    let state = common::test_state(pool).await;
    let app = me_doc_backend::build_app(state);
    let email = uniq("dupe");

    let (status, _, body) = send(
        &app,
        post_json("/auth/register", json!({ "email": email, "password": PASSWORD })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (status, _, body) = send(
        &app,
        post_json("/auth/register", json!({ "email": email, "password": PASSWORD })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["message"], "email already registered");
}

/// Logging in before the emailed code has been verified is refused — the
/// password check passes but `email_verified_at` is still null.
#[sqlx::test]
async fn login_before_verification_is_forbidden(pool: PgPool) {
    let state = common::test_state(pool).await;
    let app = me_doc_backend::build_app(state);
    let email = uniq("unverified");

    let (status, _, body) = send(
        &app,
        post_json("/auth/register", json!({ "email": email, "password": PASSWORD })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (status, _, body) = send(
        &app,
        post_json("/auth/login", json!({ "email": email, "password": PASSWORD })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["message"], "email not verified");
}
