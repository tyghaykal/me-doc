//! Realtime collaborative editing over WebSockets, wire-compatible with JS Yjs
//! WebSocket providers (`y-websocket`). One [`DocRoom`] is created lazily per
//! actively-edited page and evicted once its last client disconnects.
//!
//! The y-sync wire protocol itself (sync-step-1/2, updates, awareness) is NOT
//! hand-rolled: it's driven by yrs's own `sync::DefaultProtocol::handle_message`.
//! This module only adds the tokio broadcast fan-out that relays each client's
//! changes to the others, plus persistence/eviction. (The `yrs-axum` crate would
//! normally provide this glue, but its only releases pin yrs 0.18 + axum 0.8,
//! incompatible with this app's yrs 0.27 + axum 0.7.)

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
// `active` is an AtomicUsize (0/1) rather than AtomicBool so it pairs with the
// existing client-counter style loads without pulling in another type.
use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{Message as WsMessage, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, State};
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use dashmap::mapref::entry::Entry;
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use sqlx::PgPool;
use tokio::sync::{broadcast, Mutex, RwLock};
use uuid::Uuid;
use yrs::sync::{Awareness, DefaultProtocol, Message, Protocol, SyncMessage};
use yrs::updates::decoder::Decode;
use yrs::updates::encoder::Encode;
use yrs::{Doc, ReadTxn, StateVector, Transact, Update};

use crate::auth::{error::AuthError, jwt};
use crate::sharing;
use crate::AppState;

/// Registry of active documents, keyed by page id. Becomes an `AppState` field.
pub type DocRegistry = Arc<DashMap<Uuid, Arc<DocRoom>>>;

pub fn new_registry() -> DocRegistry {
    Arc::new(DashMap::new())
}

/// Fan-out buffer per room. If a client lags more than this many messages behind
/// it gets dropped (its provider reconnects and re-syncs from scratch).
const BROADCAST_CAPACITY: usize = 256;

/// An actively-edited page: the shared awareness/doc, a broadcast channel that
/// relays every local change to all connected clients, a live-client counter
/// (for eviction), a hash of the last-persisted state (so the periodic flusher
/// only writes on change), and the observer subscriptions that must stay alive.
pub struct DocRoom {
    awareness: Arc<RwLock<Awareness>>,
    sender: broadcast::Sender<Vec<u8>>,
    clients: AtomicUsize,
    last_hash: AtomicU64,
    /// Cleared when the room is superseded (version restore) or fully evicted.
    /// Flush/disconnect must not write a dead room's bytes back over a restore.
    active: AtomicUsize, // 1 = live, 0 = superseded/evicted
    _doc_sub: yrs::Subscription,
    _awareness_sub: yrs::Subscription,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/ws/pages/:id", get(ws_handler))
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
    // Browsers can't set custom headers on the WS handshake, so the access token
    // arrives as a query param instead of Authorization — and a genuinely
    // anonymous guest (public link, never logged in) sends no token at all.
    // Same precedence as the REST `PagePermission` extractor: verify a bearer
    // token if present, otherwise fall back to the `?link=` grant. Without
    // this fallback, an anonymous editor-link guest could never open this
    // socket at all — their edits would only ever reach the DB through the
    // periodic REST content PUT, which this room's own flusher/final-persist
    // would then silently clobber since it never saw those changes.
    let user_id = match q.token.as_deref() {
        Some(token) => Some(jwt::verify_access_token(&state.config.jwt_access_secret, token)?.sub),
        None => None,
    };
    if user_id.is_none() && q.link.is_none() {
        return Err(AuthError::InvalidToken);
    }

    // Same resolution as the REST endpoints (workspace membership, a direct
    // page/workspace grant, or a `?link=` token) — any resolved role can join
    // the room, but only Editor may actually mutate the doc (enforced below).
    let role = sharing::resolve_role(&state.db, page_id, user_id, q.link.as_deref()).await?;

    let room = join_room(&state, page_id).await?;

    Ok(ws.on_upgrade(move |socket| async move {
        handle_socket(&room, socket, role).await;

        // Last client out: final persist, then evict so idle pages don't leak.
        // Skip entirely when this room was superseded (e.g. version restore
        // swapped it out) — writing its in-memory doc would clobber the restore.
        if room.clients.fetch_sub(1, Ordering::SeqCst) == 1 && room.is_active() {
            {
                let a = room.awareness.read().await;
                if let Err(e) = persist_snapshot(&state.db, page_id, a.doc()).await {
                    tracing::error!(?e, "collab final persist failed");
                }
                // Version history: one snapshot per finished editing session.
                if let Err(e) = persist_version(&state.db, page_id, a.doc()).await {
                    tracing::error!(?e, "collab version snapshot failed");
                }
            }
            room.deactivate();
            // remove_if guards against a fresh client that rejoined this room
            // between our decrement and here (its increment holds the shard
            // lock, so its count is already visible).
            state
                .docs
                .remove_if(&page_id, |_, r| r.clients.load(Ordering::SeqCst) == 0);
        }
    }))
}

/// Relays one WebSocket connection into the room until it closes. Inbound frames
/// are fed to yrs's protocol handler (which mutates the shared doc, in turn
/// triggering the doc observer that broadcasts to peers); outbound broadcast
/// messages are forwarded to this client.
async fn handle_socket(room: &Arc<DocRoom>, socket: WebSocket, role: sharing::Role) {
    let (ws_sink, mut ws_stream) = socket.split();
    let sink = Arc::new(Mutex::new(ws_sink));

    // Subscribe before reading any snapshot so no broadcast update can slip
    // through the gap between capturing state and starting to listen.
    let mut rx = room.sender.subscribe();

    // Initial handshake: send our state vector so the client replies with the
    // updates we're missing (y-websocket's standard sync-step-1 on connect),
    // plus a snapshot of already-connected peers' awareness state (cursors,
    // names) — otherwise a late joiner only sees peers who move *after* they
    // connect.
    let (step1, awareness_snapshot) = {
        let a = room.awareness.read().await;
        let sv = a.doc().transact().state_vector();
        let step1 = Message::Sync(SyncMessage::SyncStep1(sv)).encode_v1();
        let snapshot = a
            .update()
            .ok()
            .filter(|u| !u.clients.is_empty())
            .map(|u| Message::Awareness(u).encode_v1());
        (step1, snapshot)
    };
    if sink
        .lock()
        .await
        .send(WsMessage::Binary(step1))
        .await
        .is_err()
    {
        return;
    }
    if let Some(snapshot) = awareness_snapshot {
        if sink
            .lock()
            .await
            .send(WsMessage::Binary(snapshot))
            .await
            .is_err()
        {
            return;
        }
    }

    // Writer: forward broadcast messages (peers' changes) to this client.
    let writer = {
        let sink = sink.clone();
        tokio::spawn(async move {
            while let Ok(msg) = rx.recv().await {
                if sink.lock().await.send(WsMessage::Binary(msg)).await.is_err() {
                    break;
                }
            }
        })
    };

    // Reader: apply this client's inbound protocol messages.
    while let Some(Ok(frame)) = ws_stream.next().await {
        let data = match frame {
            WsMessage::Binary(b) => b,
            WsMessage::Close(_) => break,
            _ => continue, // text/ping/pong — axum auto-handles ping
        };
        let msg = match Message::decode_v1(&data) {
            Ok(m) => m,
            Err(_) => continue,
        };

        // Viewers may sync (read) and broadcast awareness (cursor presence),
        // but never mutate the doc — a compromised/hand-rolled client could
        // otherwise smuggle edits through SyncStep2 or a raw Update message.
        let is_mutation = matches!(
            msg,
            Message::Sync(SyncMessage::Update(_)) | Message::Sync(SyncMessage::SyncStep2(_))
        );
        if role != sharing::Role::Editor && is_mutation {
            continue;
        }

        let reply = {
            let mut a = room.awareness.write().await;
            DefaultProtocol.handle_message(&mut a, msg)
        };
        match reply {
            Ok(Some(reply)) => {
                let _ = sink
                    .lock()
                    .await
                    .send(WsMessage::Binary(reply.encode_v1()))
                    .await;
            }
            Ok(None) => {}
            Err(_) => break,
        }
    }

    writer.abort();
}

/// Looks up (or lazily builds) the room for `page_id` and registers one client.
async fn join_room(state: &AppState, page_id: Uuid) -> Result<Arc<DocRoom>, AuthError> {
    // Holding the DashMap read guard across the increment serializes with the
    // disconnect path's `remove_if`, so we never hand back a room mid-eviction.
    if let Some(r) = state.docs.get(&page_id) {
        r.clients.fetch_add(1, Ordering::SeqCst);
        return Ok(r.clone());
    }

    // Build outside the map lock (it's async: DB load).
    let room = Arc::new(build_room(&state.db, page_id).await?);
    match state.docs.entry(page_id) {
        // Lost the create race — reuse the winner, drop our freshly-built room.
        // ponytail: the loser's room is built then dropped; harmless wasted work
        // on a rare concurrent-first-open race.
        Entry::Occupied(e) => {
            let existing = e.get().clone();
            existing.clients.fetch_add(1, Ordering::SeqCst);
            Ok(existing)
        }
        Entry::Vacant(e) => {
            room.clients.fetch_add(1, Ordering::SeqCst);
            e.insert(room.clone());
            tokio::spawn(flusher(state.db.clone(), page_id, room.clone()));
            Ok(room)
        }
    }
}

async fn build_room(db: &PgPool, page_id: Uuid) -> Result<DocRoom, AuthError> {
    let row: Option<(Vec<u8>,)> =
        sqlx::query_as("select yjs_state from page_content where page_id = $1")
            .bind(page_id)
            .fetch_optional(db)
            .await?;
    let state_bytes = row.map(|r| r.0).unwrap_or_default();

    let doc = Doc::new();
    if !state_bytes.is_empty() {
        let update =
            Update::decode_v1(&state_bytes).map_err(|e| AuthError::Internal(anyhow::anyhow!(e)))?;
        doc.transact_mut()
            .apply_update(update)
            .map_err(|e| AuthError::Internal(anyhow::anyhow!(e)))?;
    }
    let init_hash = hash_bytes(&doc.transact().encode_state_as_update_v1(&StateVector::default()));

    let awareness = Arc::new(RwLock::new(Awareness::new(doc)));
    let (sender, _) = broadcast::channel::<Vec<u8>>(BROADCAST_CAPACITY);

    // Observers fan every local change out to all subscribed clients.
    let (doc_sub, awareness_sub) = {
        let mut guard = awareness.write().await;

        let tx = sender.clone();
        let doc_sub = guard
            .doc()
            .observe_update_v1(move |_txn, e| {
                let msg = Message::Sync(SyncMessage::Update(e.update.clone())).encode_v1();
                let _ = tx.send(msg);
            })
            .map_err(|e| AuthError::Internal(anyhow::anyhow!(e)))?;

        let tx = sender.clone();
        let awareness_sub = guard.on_update(move |aware, e, _origin| {
            let mut changed = Vec::with_capacity(e.added().len() + e.updated().len() + e.removed().len());
            changed.extend_from_slice(e.added());
            changed.extend_from_slice(e.updated());
            changed.extend_from_slice(e.removed());
            if let Ok(update) = aware.update_with_clients(changed) {
                let _ = tx.send(Message::Awareness(update).encode_v1());
            }
        });

        (doc_sub, awareness_sub)
    };

    Ok(DocRoom {
        awareness,
        sender,
        clients: AtomicUsize::new(0),
        last_hash: AtomicU64::new(init_hash),
        active: AtomicUsize::new(1),
        _doc_sub: doc_sub,
        _awareness_sub: awareness_sub,
    })
}

impl DocRoom {
    fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst) != 0
    }

    fn deactivate(&self) {
        self.active.store(0, Ordering::SeqCst);
    }
}

/// Drop the live collab room for `page_id` (if any) so the next websocket join
/// rebuilds from `page_content`. Used by version restore after writing the
/// restored bytes to the DB — without this, the in-memory room keeps serving
/// pre-restore content and its flusher/disconnect path can overwrite the restore.
pub fn invalidate_room(docs: &DocRegistry, page_id: Uuid) {
    if let Some((_, room)) = docs.remove(&page_id) {
        room.deactivate();
    }
}

/// Per-room background task: every 5s, persist the doc if it changed since the
/// last write. Stops once the room has no clients (the disconnect path does the
/// final persist and eviction).
async fn flusher(db: PgPool, page_id: Uuid, room: Arc<DocRoom>) {
    let mut ticker = tokio::time::interval(Duration::from_secs(5));
    ticker.tick().await; // consume the immediate first tick
    loop {
        ticker.tick().await;

        // Superseded rooms (version restore) must not write stale bytes back.
        if !room.is_active() {
            break;
        }

        // Encode under the lock, but persist without holding it (avoids blocking
        // edits during the DB round-trip).
        let bytes = {
            let a = room.awareness.read().await;
            let snapshot = a
                .doc()
                .transact()
                .encode_state_as_update_v1(&StateVector::default());
            snapshot
        };
        let h = hash_bytes(&bytes);
        if room.last_hash.load(Ordering::SeqCst) != h {
            match persist_bytes(&db, page_id, &bytes).await {
                Ok(()) => {
                    room.last_hash.store(h, Ordering::SeqCst);
                }
                Err(e) => tracing::error!(?e, "collab periodic persist failed"),
            }
        }

        // ponytail: break-on-empty leaks no task, but a room reused in the 0->1
        // window right at a tick loses its periodic flusher; its final
        // on-disconnect persist still runs, so no data loss — just no midsession
        // autosave for that reused room.
        if room.clients.load(Ordering::SeqCst) == 0 {
            break;
        }
    }
}

/// Encodes the full Yjs state and writes it to `page_content.yjs_state`.
/// Phase 8's version-history feature reuses this (adding a `page_versions`
/// insert) rather than reimplementing the encode path.
pub async fn persist_snapshot(db: &PgPool, page_id: Uuid, doc: &Doc) -> Result<(), sqlx::Error> {
    let bytes = doc
        .transact()
        .encode_state_as_update_v1(&StateVector::default());
    persist_bytes(db, page_id, &bytes).await
}

/// Records a point-in-time snapshot into `page_versions` for the version-history
/// feature. Called on last-client-disconnect, so it's one row per editing
/// session rather than one per 5s flush. Skips writing when the state is
/// identical to the latest version (open-without-edit, reconnect churn).
pub async fn persist_version(db: &PgPool, page_id: Uuid, doc: &Doc) -> Result<(), sqlx::Error> {
    let bytes = doc
        .transact()
        .encode_state_as_update_v1(&StateVector::default());
    let last: Option<(Vec<u8>,)> = sqlx::query_as(
        "select yjs_state from page_versions where page_id = $1 order by created_at desc limit 1",
    )
    .bind(page_id)
    .fetch_optional(db)
    .await?;
    if last.map(|r| r.0).as_deref() == Some(bytes.as_slice()) {
        return Ok(());
    }
    sqlx::query("insert into page_versions (page_id, yjs_state) values ($1, $2)")
        .bind(page_id)
        .bind(&bytes)
        .execute(db)
        .await?;
    Ok(())
}

async fn persist_bytes(db: &PgPool, page_id: Uuid, bytes: &[u8]) -> Result<(), sqlx::Error> {
    sqlx::query(
        "insert into page_content (page_id, yjs_state, updated_at)
         values ($1, $2, now())
         on conflict (page_id) do update set yjs_state = excluded.yjs_state, updated_at = now()",
    )
    .bind(page_id)
    .bind(bytes)
    .execute(db)
    .await?;
    Ok(())
}

fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut h = DefaultHasher::new();
    bytes.hash(&mut h);
    h.finish()
}
