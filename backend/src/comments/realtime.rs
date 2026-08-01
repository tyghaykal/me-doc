//! Real-time comment fan-out. A page's comment mutations (create/reply/resolve/
//! delete) are pushed to every connected viewer so open sidebars stay current
//! without polling. Unlike `collab`, there's no CRDT here — comments are plain
//! DB rows, so this is just a per-page `broadcast` of JSON event strings.
//!
//! ponytail: single-instance in-process fan-out (one `backend` service today).
//! For horizontal scaling, back this with Redis pub/sub (client already in
//! `AppState`) so events cross instances.

use std::sync::Arc;

use axum::extract::ws::{Message as WsMessage, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, State};
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::auth::{error::AuthError, jwt};
use crate::{sharing, AppState};

/// Low volume: comment events are rare compared to keystrokes.
const CHANNEL_CAPACITY: usize = 64;

/// page_id -> broadcaster of JSON event strings. Created lazily on first
/// subscriber, evicted when the last one leaves.
pub type CommentHub = Arc<DashMap<Uuid, broadcast::Sender<String>>>;

pub fn new_hub() -> CommentHub {
    Arc::new(DashMap::new())
}

pub fn router() -> Router<AppState> {
    Router::new().route("/ws/comments/:id", get(ws_handler))
}

/// Publish a comment event to a page's subscribers. Serializes `event` and
/// drops it silently if nobody is listening (no channel / no receivers).
pub fn publish<T: serde::Serialize>(hub: &CommentHub, page_id: Uuid, event: &T) {
    if let Some(tx) = hub.get(&page_id) {
        if let Ok(json) = serde_json::to_string(event) {
            let _ = tx.send(json);
        }
    }
}

#[derive(Deserialize)]
struct TokenQuery {
    token: Option<String>,
    link: Option<String>,
}

async fn ws_handler(
    State(state): State<AppState>,
    Path(page_id): Path<Uuid>,
    Query(q): Query<TokenQuery>,
    ws: WebSocketUpgrade,
) -> Result<Response, AuthError> {
    // Token rides the query string (browsers can't set WS handshake headers) —
    // an anonymous public-link guest sends none at all, so fall back to the
    // `?link=` grant exactly like the REST `PagePermission` extractor does.
    let user_id = match q.token.as_deref() {
        Some(token) => Some(jwt::verify_access_token(&state.config.jwt_access_secret, token)?.sub),
        None => None,
    };
    if user_id.is_none() && q.link.is_none() {
        return Err(AuthError::InvalidToken);
    }

    // Any resolved role (viewer or editor) may read comments — same gate as the
    // REST `list_comments`. Reject the upgrade if there's no access at all.
    sharing::resolve_role(&state.db, page_id, user_id, q.link.as_deref()).await?;

    let tx = state
        .comments
        .entry(page_id)
        .or_insert_with(|| broadcast::channel(CHANNEL_CAPACITY).0)
        .clone();

    Ok(ws.on_upgrade(move |socket| async move {
        stream_events(socket, tx.subscribe()).await;
        // Last listener out: drop the channel so idle pages don't leak entries.
        state
            .comments
            .remove_if(&page_id, |_, tx| tx.receiver_count() == 0);
    }))
}

/// Forwards broadcast events to this client until the socket closes. Read-only:
/// inbound frames are ignored except Close (the client only listens).
async fn stream_events(socket: WebSocket, mut rx: broadcast::Receiver<String>) {
    let (mut sink, mut stream) = socket.split();

    loop {
        tokio::select! {
            msg = rx.recv() => match msg {
                Ok(json) => {
                    if sink.send(WsMessage::Text(json)).await.is_err() {
                        break;
                    }
                }
                // Lagged: client fell behind the buffer. It will refetch on the
                // next event; keep the connection alive rather than dropping it.
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            },
            frame = stream.next() => match frame {
                Some(Ok(WsMessage::Close(_))) | None => break,
                Some(Ok(_)) => continue,
                Some(Err(_)) => break,
            },
        }
    }
}
