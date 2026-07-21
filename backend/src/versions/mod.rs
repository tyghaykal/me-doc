//! Page version history and rollback. Versions are recorded by the collab
//! module on last-client-disconnect (one row per editing session); this module
//! only lists them and restores a chosen one back into `page_content`.

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::{error::AuthError, extractor::AuthenticatedUser};
use crate::collab;
use crate::pages::require_membership;
use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/pages/:id/versions", get(list_versions))
        .route("/pages/:id/versions/:version_id/restore", post(restore_version))
}

async fn page_workspace(db: &PgPool, id: Uuid) -> Result<Uuid, AuthError> {
    let row: Option<(Uuid,)> = sqlx::query_as("select workspace_id from pages where id = $1")
        .bind(id)
        .fetch_optional(db)
        .await?;
    Ok(row.ok_or(AuthError::NotFound)?.0)
}

#[derive(Debug, Serialize)]
pub struct Version {
    pub id: Uuid,
    pub size: i32,
    pub created_at: DateTime<Utc>,
}

async fn list_versions(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<Version>>, AuthError> {
    let workspace_id = page_workspace(&state.db, id).await?;
    require_membership(&state.db, workspace_id, user.user_id).await?;

    // octet_length keeps this cheap — no decoding the Yjs doc per row for a list view.
    let rows: Vec<(Uuid, i32, DateTime<Utc>)> = sqlx::query_as(
        "select id, octet_length(yjs_state), created_at from page_versions
         where page_id = $1 order by created_at desc",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(
        rows.into_iter()
            .map(|(id, size, created_at)| Version { id, size, created_at })
            .collect(),
    ))
}

async fn restore_version(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path((id, version_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, AuthError> {
    let workspace_id = page_workspace(&state.db, id).await?;
    require_membership(&state.db, workspace_id, user.user_id).await?;

    let version: Option<(Vec<u8>,)> =
        sqlx::query_as("select yjs_state from page_versions where id = $1 and page_id = $2")
            .bind(version_id)
            .bind(id)
            .fetch_optional(&state.db)
            .await?;
    let (bytes,) = version.ok_or(AuthError::NotFound)?;

    // 1. Write restored bytes to page_content first.
    sqlx::query(
        "insert into page_content (page_id, yjs_state, updated_at)
         values ($1, $2, now())
         on conflict (page_id) do update set yjs_state = excluded.yjs_state, updated_at = now()",
    )
    .bind(id)
    .bind(&bytes)
    .execute(&state.db)
    .await?;

    // Touch pages.updated_at so the topbar "Edited …" reflects the restore.
    let _ = sqlx::query("update pages set updated_at = now() where id = $1")
        .bind(id)
        .execute(&state.db)
        .await;

    // 2. Drop any live collab room so reconnecting clients rebuild from DB.
    //    The superseded room is deactivated so its flusher/disconnect cannot
    //    overwrite the restored bytes with the pre-restore in-memory doc.
    collab::invalidate_room(&state.docs, id);

    Ok(Json(serde_json::json!({ "message": "version restored" })))
}
