//! Integration tests for the `/auth/me` profile endpoints and avatar
//! presign/download. Redis and Mailpit are shared across the suite, so each
//! test uses a unique email (see the note in `auth.rs`).

mod common;

use axum::body::Body;
use axum::http::header::{AUTHORIZATION, CONTENT_TYPE};
use axum::http::{Request, StatusCode};
use common::*;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

const PASSWORD: &str = "correct-horse-battery";

fn uniq(tag: &str) -> String {
    format!("{tag}-{}@example.com", Uuid::new_v4().simple())
}

fn get_authed(uri: &str, token: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(uri)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap()
}

fn json_authed(method: &str, uri: &str, token: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

/// `GET /auth/me` resolves the bearer token to the row that registered it; a
/// brand-new account has no display name or avatar yet.
#[sqlx::test]
async fn get_me_returns_the_authenticated_user(pool: PgPool) {
    let state = common::test_state(pool).await;
    let app = me_doc_backend::build_app(state);
    let email = uniq("me");

    let client = register_and_login(&app, &email, PASSWORD).await;

    let (status, _, body) = send(&app, get_authed("/auth/me", &client.access_token)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["email"], email.as_str());
    assert!(body["id"].is_string());
    assert!(body["display_name"].is_null());
    assert!(body["avatar_key"].is_null());
}

/// Without a bearer token the profile endpoint is unauthorized.
#[sqlx::test]
async fn get_me_requires_a_bearer_token(pool: PgPool) {
    let state = common::test_state(pool).await;
    let app = me_doc_backend::build_app(state);

    let req = Request::builder()
        .method("GET")
        .uri("/auth/me")
        .body(Body::empty())
        .unwrap();
    let (status, _, _) = send(&app, req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

/// `PATCH /auth/me` echoes the updated row back and the change is durable — a
/// later GET sees the same display name.
#[sqlx::test]
async fn patch_me_updates_display_name_and_round_trips(pool: PgPool) {
    let state = common::test_state(pool).await;
    let app = me_doc_backend::build_app(state);

    let client = register_and_login(&app, &uniq("patch"), PASSWORD).await;

    let (status, _, body) = send(
        &app,
        json_authed(
            "PATCH",
            "/auth/me",
            &client.access_token,
            json!({ "display_name": "Ada Lovelace" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["display_name"], "Ada Lovelace");

    let (status, _, body) = send(&app, get_authed("/auth/me", &client.access_token)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["display_name"], "Ada Lovelace");

    // `coalesce($2, display_name)` means an omitted field leaves it alone.
    let (status, _, body) = send(
        &app,
        json_authed("PATCH", "/auth/me", &client.access_token, json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["display_name"], "Ada Lovelace");
}

/// Changing the password requires proving the current one.
#[sqlx::test]
async fn change_password_rejects_a_wrong_current_password(pool: PgPool) {
    let state = common::test_state(pool).await;
    let app = me_doc_backend::build_app(state);

    let client = register_and_login(&app, &uniq("badpw"), PASSWORD).await;

    let (status, _, body) = send(
        &app,
        json_authed(
            "POST",
            "/auth/me/password",
            &client.access_token,
            json!({ "current_password": "not-the-password", "new_password": "a-brand-new-one" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["message"], "invalid email or password");
}

/// A successful change actually re-hashes: the new password then works as the
/// "current" one, the old password no longer authenticates anywhere, and short
/// passwords are still refused.
#[sqlx::test]
async fn change_password_replaces_the_stored_hash(pool: PgPool) {
    let state = common::test_state(pool).await;
    let app = me_doc_backend::build_app(state);
    let email = uniq("pwchange");
    let new_password = "an-entirely-different-secret";

    let client = register_and_login(&app, &email, PASSWORD).await;

    let (status, _, body) = send(
        &app,
        json_authed(
            "POST",
            "/auth/me/password",
            &client.access_token,
            json!({ "current_password": PASSWORD, "new_password": new_password }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["message"], "password changed");

    // The new hash verifies...
    let (status, _, body) = send(
        &app,
        json_authed(
            "POST",
            "/auth/me/password",
            &client.access_token,
            json!({ "current_password": new_password, "new_password": "yet-another-secret" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    // ...and the original password is dead at the login endpoint too. The
    // credential check runs before any OTP is issued, so this doesn't collide
    // with the login cooldown left behind by `register_and_login`.
    let (status, _, body) = send(
        &app,
        post_json("/auth/login", json!({ "email": email, "password": PASSWORD })),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["message"], "invalid email or password");
}

/// The length floor applied at registration is enforced on change too.
#[sqlx::test]
async fn change_password_rejects_a_too_short_new_password(pool: PgPool) {
    let state = common::test_state(pool).await;
    let app = me_doc_backend::build_app(state);

    let client = register_and_login(&app, &uniq("shortpw"), PASSWORD).await;

    let (status, _, body) = send(
        &app,
        json_authed(
            "POST",
            "/auth/me/password",
            &client.access_token,
            json!({ "current_password": PASSWORD, "new_password": "short" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "password must be at least 8 characters");
}

/// Presigning an avatar returns an upload URL plus the key it was signed for,
/// and records that key on the user so `GET /auth/me` reports it.
#[sqlx::test]
async fn avatar_presign_returns_a_url_and_records_the_key(pool: PgPool) {
    let state = common::test_state(pool).await;
    let app = me_doc_backend::build_app(state);

    let client = register_and_login(&app, &uniq("avatar"), PASSWORD).await;

    let (status, _, body) = send(
        &app,
        json_authed(
            "POST",
            "/auth/me/avatar/presign",
            &client.access_token,
            json!({ "filename": "me.png", "content_type": "image/png", "size": 2048 }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let key = body["s3_key"].as_str().unwrap();
    assert!(key.starts_with("avatars/"), "unexpected key {key}");
    assert!(key.ends_with("me.png"), "unexpected key {key}");
    let url = body["upload_url"].as_str().unwrap();
    assert!(url.starts_with("http"), "unexpected url {url}");
    assert!(url.contains("X-Amz-Signature"), "presigned url must be signed: {url}");

    let (status, _, body) = send(&app, get_authed("/auth/me", &client.access_token)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["avatar_key"], key);
}

/// Non-image uploads and oversized files are refused before anything is signed.
#[sqlx::test]
async fn avatar_presign_validates_type_and_size(pool: PgPool) {
    let state = common::test_state(pool).await;
    let app = me_doc_backend::build_app(state);

    let client = register_and_login(&app, &uniq("avatarbad"), PASSWORD).await;

    let (status, _, body) = send(
        &app,
        json_authed(
            "POST",
            "/auth/me/avatar/presign",
            &client.access_token,
            json!({ "filename": "x.svg", "content_type": "image/svg+xml", "size": 100 }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "unsupported file type");

    let (status, _, body) = send(
        &app,
        json_authed(
            "POST",
            "/auth/me/avatar/presign",
            &client.access_token,
            json!({ "filename": "x.png", "content_type": "image/png", "size": 6 * 1024 * 1024 }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "file too large");
}

/// `GET /auth/avatars/download` authenticates off the `refresh_token` cookie
/// (the URL is persisted in presence payloads and can't carry a short-lived
/// access token). No cookie, or a bogus one, is unauthorized; a key outside
/// the `avatars/` prefix is a 404 so the endpoint can't be used to read
/// arbitrary bucket objects.
#[sqlx::test]
async fn avatar_download_requires_a_valid_session_cookie(pool: PgPool) {
    let state = common::test_state(pool).await;
    let app = me_doc_backend::build_app(state);

    let client = register_and_login(&app, &uniq("avatardl"), PASSWORD).await;

    let anonymous = Request::builder()
        .method("GET")
        .uri("/auth/avatars/download?key=avatars/whatever.png")
        .body(Body::empty())
        .unwrap();
    let (status, _, body) = send(&app, anonymous).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["message"], "invalid or expired session");

    let forged = Request::builder()
        .method("GET")
        .uri("/auth/avatars/download?key=avatars/whatever.png")
        .header("cookie", "refresh_token=not-a-real-token")
        .body(Body::empty())
        .unwrap();
    let (status, _, _) = send(&app, forged).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let escaping = Request::builder()
        .method("GET")
        .uri("/auth/avatars/download?key=attachments/someone-elses.pdf")
        .header("cookie", client.refresh_cookie.clone())
        .body(Body::empty())
        .unwrap();
    let (status, _, _) = send(&app, escaping).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// Avatars are deliberately visible to *any* signed-in user, not just
/// workspace peers: they show up in presence and comment threads across shared
/// pages. A stranger with no workspace relationship to the owner therefore
/// gets the redirect — this pins that documented decision so tightening it
/// later is a conscious change, not an accident.
#[sqlx::test]
async fn avatar_download_is_open_to_any_signed_in_user(pool: PgPool) {
    let state = common::test_state(pool).await;
    let app = me_doc_backend::build_app(state);

    let owner = register_and_login(&app, &uniq("avatarowner"), PASSWORD).await;
    let (status, _, body) = send(
        &app,
        json_authed(
            "POST",
            "/auth/me/avatar/presign",
            &owner.access_token,
            json!({ "filename": "me.png", "content_type": "image/png", "size": 2048 }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let key = body["s3_key"].as_str().unwrap().to_string();

    // No shared workspace, no shared page, no membership row.
    let stranger = register_and_login(&app, &uniq("avatarstranger"), PASSWORD).await;

    let req = Request::builder()
        .method("GET")
        .uri(format!("/auth/avatars/download?key={key}"))
        .header("cookie", stranger.refresh_cookie.clone())
        .body(Body::empty())
        .unwrap();
    let (status, _, _) = send(&app, req).await;
    assert_eq!(status, StatusCode::TEMPORARY_REDIRECT);
}
