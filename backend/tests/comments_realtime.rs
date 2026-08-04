//! The comment fan-out socket (`GET /ws/comments/:id`). Needs a real bound port
//! for the Upgrade handshake, but the REST call that triggers the broadcast can
//! still go through `oneshot` — both share the same `AppState`, and the hub the
//! events flow through is an `Arc` inside it.

mod common;

use std::time::Duration;

use axum::http::header::{AUTHORIZATION, CONTENT_TYPE};
use axum::http::{Request, StatusCode};
use common::*;
use futures_util::StreamExt;
use me_doc_backend::auth::jwt;
use me_doc_backend::build_app;
use serde_json::{json, Value};
use sqlx::PgPool;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

/// An owner (Editor by workspace membership) and a page in their workspace.
async fn fixture(pool: &PgPool) -> (Uuid, Uuid) {
    let owner = insert_user(pool, "owner@example.com").await;
    let ws = insert_workspace(pool, owner, "ws").await;
    add_member(pool, ws, owner, "owner").await;
    let page = insert_page(pool, ws, None, owner, "page").await;
    (owner, page)
}

#[sqlx::test]
async fn valid_token_connects_and_bad_one_does_not(pool: PgPool) {
    let (owner, page) = fixture(&pool).await;
    let state = test_state(pool).await;
    let tok = jwt::create_access_token(&state.config.jwt_access_secret, owner, 3600).unwrap();
    let (addr, _server) = spawn_real_server(state).await;

    let (_ws, res) = connect_async(format!("ws://{addr}/ws/comments/{page}?token={tok}"))
        .await
        .expect("a valid access token must upgrade");
    assert_eq!(res.status().as_u16(), 101);

    assert!(
        connect_async(format!("ws://{addr}/ws/comments/{page}"))
            .await
            .is_err(),
        "an unauthenticated upgrade must be rejected"
    );
}

#[sqlx::test]
async fn rest_create_is_pushed_to_a_connected_listener(pool: PgPool) {
    let (owner, page) = fixture(&pool).await;
    let state = test_state(pool).await;
    let tok = jwt::create_access_token(&state.config.jwt_access_secret, owner, 3600).unwrap();
    let app = build_app(state.clone());
    let (addr, _server) = spawn_real_server(state).await;

    let (mut ws, _) = connect_async(format!("ws://{addr}/ws/comments/{page}?token={tok}"))
        .await
        .unwrap();
    // The client's subscription is registered inside `on_upgrade`, i.e. after
    // the 101 we just saw — and `publish` silently drops events with no
    // receiver, so give that task a moment to land before publishing.
    tokio::time::sleep(Duration::from_millis(250)).await;

    let (status, _, body) = send(
        &app,
        Request::builder()
            .method("POST")
            .uri(format!("/pages/{page}/comments"))
            .header(AUTHORIZATION, format!("Bearer {tok}"))
            .header(CONTENT_TYPE, "application/json")
            .body(axum::body::Body::from(
                json!({ "mark_id": Uuid::new_v4(), "body": "ping" }).to_string(),
            ))
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create failed: {body}");

    let frame = tokio::time::timeout(Duration::from_secs(5), ws.next())
        .await
        .expect("no realtime event arrived within 5s")
        .expect("socket closed before the event arrived")
        .expect("websocket error");
    let Message::Text(text) = frame else {
        panic!("expected a text event frame, got {frame:?}")
    };

    let event: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(event["type"], "created");
    assert_eq!(event["comment"]["body"], "ping");
    assert_eq!(event["comment"]["id"], body["id"]);
    assert_eq!(event["comment"]["page_id"], page.to_string());
}
