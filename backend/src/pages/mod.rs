use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    http::header,
    response::IntoResponse,
    routing::{get, patch, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::{error::AuthError, extractor::AuthenticatedUser};
use crate::sharing::{PagePermission, Role};
use crate::AppState;

#[derive(Debug, Serialize)]
pub struct Page {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub parent_page_id: Option<Uuid>,
    pub title: String,
    pub slug: String,
    pub order_index: i32,
    pub archived_at: Option<DateTime<Utc>>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

type PageRow = (
    Uuid,
    Uuid,
    Option<Uuid>,
    String,
    String,
    i32,
    Option<DateTime<Utc>>,
    Uuid,
    DateTime<Utc>,
    DateTime<Utc>,
);

impl From<PageRow> for Page {
    fn from(r: PageRow) -> Self {
        Page {
            id: r.0,
            workspace_id: r.1,
            parent_page_id: r.2,
            title: r.3,
            slug: r.4,
            order_index: r.5,
            archived_at: r.6,
            created_by: r.7,
            created_at: r.8,
            updated_at: r.9,
        }
    }
}

const PAGE_COLUMNS: &str = "id, workspace_id, parent_page_id, title, slug, order_index, archived_at, created_by, created_at, updated_at";

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/workspaces/:workspace_id/pages",
            get(list_pages).post(create_page),
        )
        .route("/workspaces/:workspace_id/pages/trash", get(list_trash))
        .route("/pages/:id", get(get_page).patch(update_page).delete(delete_page))
        .route("/pages/:id/restore", patch(restore_page))
        .route("/pages/:id/duplicate", post(duplicate_page))
        .route(
            "/pages/:id/content",
            get(get_page_content).put(put_page_content),
        )
        .route("/attachments/presign", post(presign_attachment))
        .route("/workspaces/:workspace_id/search", get(search_pages))
}

async fn page_workspace(db: &PgPool, id: Uuid) -> Result<Uuid, AuthError> {
    let row: Option<(Uuid,)> = sqlx::query_as("select workspace_id from pages where id = $1")
        .bind(id)
        .fetch_optional(db)
        .await?;
    Ok(row.ok_or(AuthError::NotFound)?.0)
}

#[derive(Deserialize)]
struct CreatePageRequest {
    title: Option<String>,
    parent_page_id: Option<Uuid>,
}

#[derive(Deserialize)]
struct UpdatePageRequest {
    title: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    parent_page_id: Option<Option<Uuid>>,
    order_index: Option<i32>,
}

/// Distinguishes "field absent" (None) from "field present and null" (Some(None)).
fn double_option<'de, D>(deserializer: D) -> Result<Option<Option<Uuid>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Some(Option::deserialize(deserializer)?))
}

pub(crate) async fn require_membership(db: &PgPool, workspace_id: Uuid, user_id: Uuid) -> Result<(), AuthError> {
    let member: Option<(Uuid,)> = sqlx::query_as(
        "select user_id from workspace_members where workspace_id = $1 and user_id = $2",
    )
    .bind(workspace_id)
    .bind(user_id)
    .fetch_optional(db)
    .await?;

    if member.is_none() {
        return Err(AuthError::Forbidden);
    }
    Ok(())
}

fn slugify(title: &str) -> String {
    let base: String = title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let base = base.trim_matches('-');
    let base = if base.is_empty() { "page" } else { base };
    format!("{}-{}", base, &Uuid::new_v4().simple().to_string()[..8])
}

async fn create_page(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(workspace_id): Path<Uuid>,
    Json(body): Json<CreatePageRequest>,
) -> Result<Json<Page>, AuthError> {
    require_membership(&state.db, workspace_id, user.user_id).await?;

    let title = body.title.unwrap_or_else(|| "Untitled".to_string());
    let slug = slugify(&title);

    let row: PageRow = sqlx::query_as(&format!(
        "insert into pages (workspace_id, parent_page_id, title, slug, created_by)
         values ($1, $2, $3, $4, $5)
         returning {PAGE_COLUMNS}"
    ))
    .bind(workspace_id)
    .bind(body.parent_page_id)
    .bind(&title)
    .bind(&slug)
    .bind(user.user_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(row.into()))
}

async fn list_pages(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<Vec<Page>>, AuthError> {
    require_membership(&state.db, workspace_id, user.user_id).await?;

    let rows: Vec<PageRow> = sqlx::query_as(&format!(
        "select {PAGE_COLUMNS} from pages
         where workspace_id = $1 and archived_at is null
         order by order_index asc"
    ))
    .bind(workspace_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(rows.into_iter().map(Page::from).collect()))
}

async fn update_page(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdatePageRequest>,
) -> Result<Json<Page>, AuthError> {
    let workspace_id: Option<(Uuid,)> =
        sqlx::query_as("select workspace_id from pages where id = $1")
            .bind(id)
            .fetch_optional(&state.db)
            .await?;
    let (workspace_id,) = workspace_id.ok_or(AuthError::NotFound)?;
    require_membership(&state.db, workspace_id, user.user_id).await?;

    let (set_parent, parent_value) = match body.parent_page_id {
        Some(v) => (true, v),
        None => (false, None),
    };

    let row: PageRow = sqlx::query_as(&format!(
        "update pages set
            title = coalesce($2, title),
            order_index = coalesce($3, order_index),
            parent_page_id = case when $4 then $5 else parent_page_id end,
            updated_at = now()
         where id = $1
         returning {PAGE_COLUMNS}"
    ))
    .bind(id)
    .bind(body.title)
    .bind(body.order_index)
    .bind(set_parent)
    .bind(parent_value)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(row.into()))
}

async fn delete_page(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AuthError> {
    let workspace_id: Option<(Uuid,)> =
        sqlx::query_as("select workspace_id from pages where id = $1")
            .bind(id)
            .fetch_optional(&state.db)
            .await?;
    let (workspace_id,) = workspace_id.ok_or(AuthError::NotFound)?;
    require_membership(&state.db, workspace_id, user.user_id).await?;

    sqlx::query("update pages set archived_at = now(), updated_at = now() where id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({ "message": "page archived" })))
}

async fn duplicate_page(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Page>, AuthError> {
    let workspace_id = page_workspace(&state.db, id).await?;
    require_membership(&state.db, workspace_id, user.user_id).await?;

    let source: Option<(Option<Uuid>, String)> =
        sqlx::query_as("select parent_page_id, title from pages where id = $1")
            .bind(id)
            .fetch_optional(&state.db)
            .await?;
    let (parent_page_id, title) = source.ok_or(AuthError::NotFound)?;

    let new_title = format!("{title} (copy)");
    let slug = slugify(&new_title);

    let row: PageRow = sqlx::query_as(&format!(
        "insert into pages (workspace_id, parent_page_id, title, slug, created_by)
         values ($1, $2, $3, $4, $5)
         returning {PAGE_COLUMNS}"
    ))
    .bind(workspace_id)
    .bind(parent_page_id)
    .bind(&new_title)
    .bind(&slug)
    .bind(user.user_id)
    .fetch_one(&state.db)
    .await?;

    let new_id = row.0;
    sqlx::query(
        "insert into page_content (page_id, yjs_state, plain_text)
         select $2, yjs_state, plain_text from page_content where page_id = $1",
    )
    .bind(id)
    .bind(new_id)
    .execute(&state.db)
    .await?;

    Ok(Json(row.into()))
}

async fn get_page(
    State(state): State<AppState>,
    perm: PagePermission,
) -> Result<Json<Page>, AuthError> {
    let row: PageRow = sqlx::query_as(&format!("select {PAGE_COLUMNS} from pages where id = $1"))
        .bind(perm.page_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AuthError::NotFound)?;

    Ok(Json(row.into()))
}

async fn get_page_content(
    State(state): State<AppState>,
    perm: PagePermission,
) -> Result<impl IntoResponse, AuthError> {
    let row: Option<(Vec<u8>,)> =
        sqlx::query_as("select yjs_state from page_content where page_id = $1")
            .bind(perm.page_id)
            .fetch_optional(&state.db)
            .await?;

    let bytes = row.map(|r| r.0).unwrap_or_default();
    Ok((
        [(header::CONTENT_TYPE, "application/octet-stream")],
        bytes,
    ))
}

async fn put_page_content(
    State(state): State<AppState>,
    perm: PagePermission,
    body: Bytes,
) -> Result<Json<serde_json::Value>, AuthError> {
    if perm.role != Role::Editor {
        return Err(AuthError::Forbidden);
    }

    sqlx::query(
        "insert into page_content (page_id, yjs_state, updated_at)
         values ($1, $2, now())
         on conflict (page_id) do update set yjs_state = excluded.yjs_state, updated_at = now()",
    )
    .bind(perm.page_id)
    .bind(body.as_ref())
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({ "message": "content saved" })))
}

async fn list_trash(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<Vec<Page>>, AuthError> {
    require_membership(&state.db, workspace_id, user.user_id).await?;

    let rows: Vec<PageRow> = sqlx::query_as(&format!(
        "select {PAGE_COLUMNS} from pages
         where workspace_id = $1 and archived_at is not null
         order by order_index asc"
    ))
    .bind(workspace_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(rows.into_iter().map(Page::from).collect()))
}

async fn restore_page(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Page>, AuthError> {
    let workspace_id = page_workspace(&state.db, id).await?;
    require_membership(&state.db, workspace_id, user.user_id).await?;

    let row: PageRow = sqlx::query_as(&format!(
        "update pages set archived_at = null, updated_at = now()
         where id = $1
         returning {PAGE_COLUMNS}"
    ))
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(row.into()))
}

#[derive(Deserialize)]
struct PresignRequest {
    workspace_id: Uuid,
    filename: String,
    content_type: String,
}

async fn presign_attachment(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<PresignRequest>,
) -> Result<Json<serde_json::Value>, AuthError> {
    require_membership(&state.db, body.workspace_id, user.user_id).await?;

    let s3_key = format!("{}/{}-{}", body.workspace_id, Uuid::new_v4(), body.filename);

    let upload_url = crate::storage::presign_upload_url(
        &state.s3_presign,
        &state.config.s3_bucket,
        &s3_key,
        &body.content_type,
    )
    .await?;

    // ponytail: size=0 placeholder — real size unknown until upload completes; patch on confirm if needed.
    sqlx::query(
        "insert into attachments (workspace_id, page_id, s3_key, filename, mime_type, size, uploaded_by)
         values ($1, null, $2, $3, $4, 0, $5)",
    )
    .bind(body.workspace_id)
    .bind(&s3_key)
    .bind(&body.filename)
    .bind(&body.content_type)
    .bind(user.user_id)
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({ "upload_url": upload_url, "s3_key": s3_key })))
}

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
}

async fn search_pages(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(workspace_id): Path<Uuid>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<Vec<Page>>, AuthError> {
    require_membership(&state.db, workspace_id, user.user_id).await?;

    let rows: Vec<PageRow> = sqlx::query_as(&format!(
        "select {} from pages p
         join page_content pc on pc.page_id = p.id
         where p.workspace_id = $1
           and p.archived_at is null
           and pc.search_vector @@ plainto_tsquery('english', $2)
         order by ts_rank(pc.search_vector, plainto_tsquery('english', $2)) desc",
        PAGE_COLUMNS
            .split(", ")
            .map(|c| format!("p.{c}"))
            .collect::<Vec<_>>()
            .join(", ")
    ))
    .bind(workspace_id)
    .bind(&query.q)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(rows.into_iter().map(Page::from).collect()))
}
