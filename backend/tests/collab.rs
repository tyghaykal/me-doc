//! The Yjs collaboration socket (`GET /ws/pages/:id`). Needs a real bound port —
//! `oneshot` can't perform an Upgrade handshake — so every test here goes
//! through `spawn_real_server`.
//!
//! Frames are checked at the byte level rather than decoded into yrs types: a
//! y-sync v1 message starts with a varint message type then a varint sub-type,
//! so `[0, 0]` is Sync/SyncStep1 and `[0, 2]` is Sync/Update. That's enough to
//! tell the handshake apart from a relayed peer edit.

mod common;

use std::time::Duration;

use common::*;
use futures_util::{SinkExt, Stream, StreamExt};
use me_doc_backend::auth::jwt;
use sqlx::PgPool;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::{Error as WsError, Message};
use uuid::Uuid;
use yrs::sync::{Message as SyncProto, SyncMessage};
use yrs::updates::encoder::Encode;
use yrs::{Doc, ReadTxn, StateVector, Text, Transact};

/// An owner (Editor by workspace membership) and a page in their workspace.
async fn fixture(pool: &PgPool) -> (Uuid, Uuid) {
    let owner = insert_user(pool, "owner@example.com").await;
    let ws = insert_workspace(pool, owner, "ws").await;
    add_member(pool, ws, owner, "owner").await;
    let page = insert_page(pool, ws, None, owner, "page").await;
    (owner, page)
}

/// Next frame, or a panic — a hung socket must fail the test in seconds rather
/// than block the whole suite.
async fn next_binary<S>(ws: &mut S) -> Vec<u8>
where
    S: Stream<Item = Result<Message, WsError>> + Unpin,
{
    let frame = tokio::time::timeout(Duration::from_secs(5), ws.next())
        .await
        .expect("timed out waiting for a websocket frame")
        .expect("socket closed before a frame arrived")
        .expect("websocket error");
    match frame {
        Message::Binary(b) => b,
        other => panic!("expected a binary y-sync frame, got {other:?}"),
    }
}

/// A real, decodable Yjs v1 update wrapped in a y-sync Update message. It has to
/// actually decode: the server hands inbound frames to yrs's own protocol
/// handler, which drops (and on error, disconnects) anything malformed.
fn yjs_update() -> Vec<u8> {
    let doc = Doc::new();
    let text = doc.get_or_insert_text("content");
    {
        let mut txn = doc.transact_mut();
        text.push(&mut txn, "hello");
    }
    let update = doc
        .transact()
        .encode_state_as_update_v1(&StateVector::default());
    SyncProto::Sync(SyncMessage::Update(update)).encode_v1()
}

#[sqlx::test]
async fn valid_jwt_connects_and_gets_sync_step1(pool: PgPool) {
    let (owner, page) = fixture(&pool).await;
    let state = test_state(pool).await;
    let tok = jwt::create_access_token(&state.config.jwt_access_secret, owner, 3600).unwrap();
    let (addr, _server) = spawn_real_server(state).await;

    let (mut ws, res) = connect_async(format!("ws://{addr}/ws/pages/{page}?token={tok}"))
        .await
        .expect("a valid access token must upgrade");
    assert_eq!(res.status().as_u16(), 101);

    let frame = next_binary(&mut ws).await;
    assert_eq!(&frame[..2], &[0, 0], "server must open with Sync/SyncStep1");
}

#[sqlx::test]
async fn public_link_token_connects(pool: PgPool) {
    let (_owner, page) = fixture(&pool).await;
    grant_page_link(&pool, page, "linktok", "viewer").await;
    let state = test_state(pool).await;
    let (addr, _server) = spawn_real_server(state).await;

    let (mut ws, _) = connect_async(format!("ws://{addr}/ws/pages/{page}?link=linktok"))
        .await
        .expect("a valid public link must upgrade without any JWT");

    let frame = next_binary(&mut ws).await;
    assert_eq!(&frame[..2], &[0, 0]);
}

#[sqlx::test]
async fn bad_or_missing_credentials_are_rejected(pool: PgPool) {
    let (_owner, page) = fixture(&pool).await;
    let state = test_state(pool).await;
    let (addr, _server) = spawn_real_server(state).await;

    for query in ["", "?token=not-a-jwt", "?link=not-a-real-token"] {
        assert!(
            connect_async(format!("ws://{addr}/ws/pages/{page}{query}"))
                .await
                .is_err(),
            "upgrade must fail for query {query:?}"
        );
    }
}

#[sqlx::test]
async fn an_edit_from_one_client_reaches_the_other(pool: PgPool) {
    let (owner, page) = fixture(&pool).await;
    let state = test_state(pool).await;
    let tok = jwt::create_access_token(&state.config.jwt_access_secret, owner, 3600).unwrap();
    let (addr, _server) = spawn_real_server(state).await;
    let url = format!("ws://{addr}/ws/pages/{page}?token={tok}");

    // Reading each client's handshake frame also proves it has subscribed to
    // the room's broadcast channel (the server subscribes before sending it).
    let (mut listener, _) = connect_async(&url).await.unwrap();
    next_binary(&mut listener).await;
    let (mut editor, _) = connect_async(&url).await.unwrap();
    next_binary(&mut editor).await;

    editor.send(Message::Binary(yjs_update())).await.unwrap();

    let relayed = next_binary(&mut listener).await;
    assert_eq!(&relayed[..2], &[0, 2], "peer must see the edit as Sync/Update");
}
