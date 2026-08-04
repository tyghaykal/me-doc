use axum::{
    extract::{Query, State},
    response::Redirect,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::auth::error::AuthError;
use crate::auth::extractor::AuthenticatedUser;
use crate::auth::password;
use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/me", get(get_me).patch(update_me))
        .route("/me/password", post(change_password))
        .route("/me/avatar/presign", post(presign_avatar))
        .route("/avatars/download", get(download_avatar))
}

#[derive(Serialize)]
pub struct MeResponse {
    id: Uuid,
    email: String,
    display_name: Option<String>,
    avatar_key: Option<String>,
}

async fn get_me(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<MeResponse>, AuthError> {
    let row: Option<(Uuid, String, Option<String>, Option<String>)> = sqlx::query_as(
        "select id, email, display_name, avatar_key from users where id = $1",
    )
    .bind(user.user_id)
    .fetch_optional(&state.db)
    .await?;

    let (id, email, display_name, avatar_key) = row.ok_or(AuthError::NotFound)?;
    Ok(Json(MeResponse {
        id,
        email,
        display_name,
        avatar_key,
    }))
}

#[derive(Deserialize)]
struct UpdateMeRequest {
    display_name: Option<String>,
}

async fn update_me(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<UpdateMeRequest>,
) -> Result<Json<MeResponse>, AuthError> {
    let row: (Uuid, String, Option<String>, Option<String>) = sqlx::query_as(
        "update users set display_name = coalesce($2, display_name)
         where id = $1
         returning id, email, display_name, avatar_key",
    )
    .bind(user.user_id)
    .bind(body.display_name)
    .fetch_one(&state.db)
    .await?;

    let (id, email, display_name, avatar_key) = row;
    Ok(Json(MeResponse {
        id,
        email,
        display_name,
        avatar_key,
    }))
}

#[derive(Deserialize)]
struct ChangePasswordRequest {
    current_password: String,
    new_password: String,
}

async fn change_password(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<ChangePasswordRequest>,
) -> Result<Json<serde_json::Value>, AuthError> {
    let hash: Option<(String,)> =
        sqlx::query_as("select password_hash from users where id = $1")
            .bind(user.user_id)
            .fetch_optional(&state.db)
            .await?;
    let (hash,) = hash.ok_or(AuthError::NotFound)?;

    if !password::verify_password(&body.current_password, &hash) {
        return Err(AuthError::InvalidCredentials);
    }
    if body.new_password.len() < 8 {
        return Err(AuthError::Validation(
            "password must be at least 8 characters".into(),
        ));
    }

    let new_hash = password::hash_password(&body.new_password)?;
    sqlx::query("update users set password_hash = $2 where id = $1")
        .bind(user.user_id)
        .bind(&new_hash)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "message": "password changed" })))
}

const MAX_AVATAR_BYTES: i64 = 5 * 1024 * 1024;

#[derive(Deserialize)]
struct AvatarDownloadQuery {
    key: String,
}

/// Avatars are low-sensitivity and meant to be visible to any collaborator
/// across workspaces (comments, presence, shared pages), so unlike
/// attachments this only requires the caller to be logged in — no
/// per-workspace check.
///
/// Auth comes from the `refresh_token` cookie (peeked, not consumed), same
/// reasoning as `pages::download_attachment`: this URL is persisted (e.g. in
/// `useCollab`'s presence payload, cached UI state) and can't carry a
/// short-lived access token.
async fn download_avatar(
    State(state): State<AppState>,
    cookies: tower_cookies::Cookies,
    Query(q): Query<AvatarDownloadQuery>,
) -> Result<Redirect, AuthError> {
    let Some(cookie) = cookies.get("refresh_token") else {
        return Err(AuthError::InvalidToken);
    };
    crate::auth::tokens::peek_refresh_token(&state.db, cookie.value()).await?;
    if !q.key.starts_with("avatars/") {
        return Err(AuthError::NotFound);
    }
    let url =
        crate::storage::presign_download_url(&state.s3_presign, &state.config.s3_bucket, &q.key)
            .await?;
    Ok(Redirect::temporary(&url))
}

#[derive(Deserialize)]
struct PresignAvatarRequest {
    filename: String,
    content_type: String,
    size: i64,
}

async fn presign_avatar(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<PresignAvatarRequest>,
) -> Result<Json<serde_json::Value>, AuthError> {
    if body.size <= 0 || body.size > MAX_AVATAR_BYTES {
        return Err(AuthError::Validation("file too large".into()));
    }
    if !crate::storage::is_allowed_image_type(&body.content_type) {
        return Err(AuthError::Validation("unsupported file type".into()));
    }
    let filename = crate::storage::sanitize_filename(&body.filename);
    let s3_key = format!("avatars/{}/{}-{}", user.user_id, Uuid::new_v4(), filename);

    let upload_url = crate::storage::presign_upload_url(
        &state.s3_presign,
        &state.config.s3_bucket,
        &s3_key,
        &body.content_type,
        body.size,
    )
    .await?;

    sqlx::query("update users set avatar_key = $2 where id = $1")
        .bind(user.user_id)
        .bind(&s3_key)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "upload_url": upload_url, "s3_key": s3_key })))
}
