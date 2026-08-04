//! REST CRUD for page comments (`comments::router`). Access is gated by the
//! `PagePermission` extractor, so tokens are minted directly rather than going
//! through the register/login round trip — that flow has its own test file.

mod common;

use axum::body::Body;
use axum::http::header::{AUTHORIZATION, CONTENT_TYPE};
use axum::http::{Request, StatusCode};
use common::*;
use me_doc_backend::auth::jwt;
use me_doc_backend::{build_app, AppState};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

fn token(state: &AppState, user: Uuid) -> String {
    jwt::create_access_token(&state.config.jwt_access_secret, user, 3600).unwrap()
}

fn authed(method: &str, uri: &str, token: &str, body: Option<Value>) -> Request<Body> {
    let req = Request::builder()
        .method(method)
        .uri(uri)
        .header(AUTHORIZATION, format!("Bearer {token}"));
    match body {
        Some(v) => req
            .header(CONTENT_TYPE, "application/json")
            .body(Body::from(v.to_string()))
            .unwrap(),
        None => req.body(Body::empty()).unwrap(),
    }
}

/// An owner (Editor by workspace membership) and a page in their workspace.
async fn fixture(pool: &PgPool) -> (Uuid, Uuid) {
    let owner = insert_user(pool, "owner@example.com").await;
    let ws = insert_workspace(pool, owner, "ws").await;
    add_member(pool, ws, owner, "owner").await;
    let page = insert_page(pool, ws, None, owner, "page").await;
    (owner, page)
}

#[sqlx::test]
async fn create_list_resolve_delete(pool: PgPool) {
    let (owner, page) = fixture(&pool).await;
    let state = test_state(pool).await;
    let app = build_app(state.clone());
    let tok = token(&state, owner);

    let (status, _, body) = send(
        &app,
        authed(
            "POST",
            &format!("/pages/{page}/comments"),
            &tok,
            Some(json!({ "mark_id": Uuid::new_v4(), "body": "  needs a citation  " })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create failed: {body}");
    assert_eq!(body["body"], "needs a citation", "body must be trimmed");
    assert_eq!(body["resolved"], false);
    assert_eq!(body["author_email"], "owner@example.com");
    let id = body["id"].as_str().unwrap().to_string();

    let (status, _, body) = send(
        &app,
        authed("GET", &format!("/pages/{page}/comments"), &tok, None),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 1);

    let (status, _, body) = send(
        &app,
        authed("PATCH", &format!("/comments/{id}/resolve"), &tok, None),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "resolve failed: {body}");
    assert_eq!(body["resolved"], true);

    let (status, _, body) = send(&app, authed("DELETE", &format!("/comments/{id}"), &tok, None)).await;
    assert_eq!(status, StatusCode::OK, "delete failed: {body}");

    let (_, _, body) = send(
        &app,
        authed("GET", &format!("/pages/{page}/comments"), &tok, None),
    )
    .await;
    assert!(body.as_array().unwrap().is_empty(), "deleted comment still listed");
}

/// Any resolved role may comment, but resolving someone else's comment needs
/// Editor — a Viewer gets 403 on the owner's comment and 200 on their own.
#[sqlx::test]
async fn viewer_may_comment_but_not_resolve_anothers(pool: PgPool) {
    let (owner, page) = fixture(&pool).await;
    let viewer = insert_user(&pool, "viewer@example.com").await;
    grant_page(&pool, page, viewer, "viewer").await;

    let state = test_state(pool).await;
    let app = build_app(state.clone());
    let owner_tok = token(&state, owner);
    let viewer_tok = token(&state, viewer);
    let uri = format!("/pages/{page}/comments");

    let (_, _, owners) = send(
        &app,
        authed("POST", &uri, &owner_tok, Some(json!({ "mark_id": Uuid::new_v4(), "body": "mine" }))),
    )
    .await;
    let owners_id = owners["id"].as_str().unwrap();

    let (status, _, viewers) = send(
        &app,
        authed("POST", &uri, &viewer_tok, Some(json!({ "mark_id": Uuid::new_v4(), "body": "theirs" }))),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "a viewer must still be able to comment: {viewers}");

    let (status, _, _) = send(
        &app,
        authed("PATCH", &format!("/comments/{owners_id}/resolve"), &viewer_tok, None),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, _, _) = send(
        &app,
        authed(
            "PATCH",
            &format!("/comments/{}/resolve", viewers["id"].as_str().unwrap()),
            &viewer_tok,
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "authors may resolve their own comment");
}

#[sqlx::test]
async fn no_access_to_page_means_no_comments(pool: PgPool) {
    let (_owner, page) = fixture(&pool).await;
    let stranger = insert_user(&pool, "stranger@example.com").await;

    let state = test_state(pool).await;
    let app = build_app(state.clone());
    let tok = token(&state, stranger);
    let uri = format!("/pages/{page}/comments");

    let (status, _, _) = send(
        &app,
        authed("POST", &uri, &tok, Some(json!({ "mark_id": Uuid::new_v4(), "body": "hello" }))),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, _, _) = send(&app, authed("GET", &uri, &tok, None)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}
