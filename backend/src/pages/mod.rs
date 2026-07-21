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
    pub icon: Option<String>,
    /// The requester's resolved sharing role ("viewer"/"editor") — only
    /// populated by `get_page`, which is the only handler that knows it
    /// (others gate on workspace membership, not the page-sharing grant).
    pub role: Option<String>,
    /// Whether this page has non-archived direct children. Set by list endpoints
    /// that support expand-without-loading; omitted/null elsewhere.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_children: Option<bool>,
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
    Option<String>,
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
            icon: r.10,
            role: None,
            has_children: None,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PageListResponse {
    pub items: Vec<Page>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ListPagesQuery {
    /// When set, list direct children of this page. When absent, list roots
    /// (`parent_page_id is null`) only — never the full flat dump.
    parent_id: Option<Uuid>,
    /// Opaque cursor from a previous response (`order_index:id`).
    cursor: Option<String>,
    /// Page size (default 30, max 100).
    limit: Option<i64>,
}

fn encode_cursor(order_index: i32, id: Uuid) -> String {
    format!("{order_index}:{id}")
}

fn decode_cursor(cursor: &str) -> Option<(i32, Uuid)> {
    let (oi, id) = cursor.split_once(':')?;
    Some((oi.parse().ok()?, id.parse().ok()?))
}

const PAGE_COLUMNS: &str = "id, workspace_id, parent_page_id, title, slug, order_index, archived_at, created_by, created_at, updated_at, icon";

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
        .route("/me/shared-pages", get(list_shared_pages))
        .route("/me/favorite-pages", get(list_favorite_pages))
        .route(
            "/pages/:id/favorite",
            post(favorite_page).delete(unfavorite_page),
        )
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
    #[serde(default, deserialize_with = "double_option")]
    icon: Option<Option<String>>,
}

/// Distinguishes "field absent" (None) from "field present and null" (Some(None)).
fn double_option<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
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
    Query(query): Query<ListPagesQuery>,
) -> Result<Json<PageListResponse>, AuthError> {
    require_membership(&state.db, workspace_id, user.user_id).await?;

    let limit = query.limit.unwrap_or(30).clamp(1, 100);
    // Fetch one extra row to know whether a next page exists.
    let fetch = limit + 1;

    let cursor_pair: Option<(i32, Uuid)> = match query.cursor.as_deref() {
        None => None,
        Some(c) => Some(
            decode_cursor(c).ok_or_else(|| AuthError::Validation("invalid cursor".into()))?,
        ),
    };

    // sqlx needs typed binds; branch on parent + cursor rather than dynamic SQL soup.
    type ListRow = (
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
        Option<String>,
        bool,
    );

    let rows: Vec<ListRow> = match (query.parent_id, cursor_pair) {
        (None, None) => {
            sqlx::query_as(
                "select p.id, p.workspace_id, p.parent_page_id, p.title, p.slug, p.order_index,
                        p.archived_at, p.created_by, p.created_at, p.updated_at, p.icon,
                        exists(
                          select 1 from pages c
                          where c.parent_page_id = p.id and c.archived_at is null
                        ) as has_children
                 from pages p
                 where p.workspace_id = $1
                   and p.parent_page_id is null
                   and p.archived_at is null
                 order by p.order_index asc, p.id asc
                 limit $2",
            )
            .bind(workspace_id)
            .bind(fetch)
            .fetch_all(&state.db)
            .await?
        }
        (None, Some((oi, id))) => {
            sqlx::query_as(
                "select p.id, p.workspace_id, p.parent_page_id, p.title, p.slug, p.order_index,
                        p.archived_at, p.created_by, p.created_at, p.updated_at, p.icon,
                        exists(
                          select 1 from pages c
                          where c.parent_page_id = p.id and c.archived_at is null
                        ) as has_children
                 from pages p
                 where p.workspace_id = $1
                   and p.parent_page_id is null
                   and p.archived_at is null
                   and (p.order_index, p.id) > ($2, $3)
                 order by p.order_index asc, p.id asc
                 limit $4",
            )
            .bind(workspace_id)
            .bind(oi)
            .bind(id)
            .bind(fetch)
            .fetch_all(&state.db)
            .await?
        }
        (Some(parent_id), None) => {
            sqlx::query_as(
                "select p.id, p.workspace_id, p.parent_page_id, p.title, p.slug, p.order_index,
                        p.archived_at, p.created_by, p.created_at, p.updated_at, p.icon,
                        exists(
                          select 1 from pages c
                          where c.parent_page_id = p.id and c.archived_at is null
                        ) as has_children
                 from pages p
                 where p.workspace_id = $1
                   and p.parent_page_id = $2
                   and p.archived_at is null
                 order by p.order_index asc, p.id asc
                 limit $3",
            )
            .bind(workspace_id)
            .bind(parent_id)
            .bind(fetch)
            .fetch_all(&state.db)
            .await?
        }
        (Some(parent_id), Some((oi, id))) => {
            sqlx::query_as(
                "select p.id, p.workspace_id, p.parent_page_id, p.title, p.slug, p.order_index,
                        p.archived_at, p.created_by, p.created_at, p.updated_at, p.icon,
                        exists(
                          select 1 from pages c
                          where c.parent_page_id = p.id and c.archived_at is null
                        ) as has_children
                 from pages p
                 where p.workspace_id = $1
                   and p.parent_page_id = $2
                   and p.archived_at is null
                   and (p.order_index, p.id) > ($3, $4)
                 order by p.order_index asc, p.id asc
                 limit $5",
            )
            .bind(workspace_id)
            .bind(parent_id)
            .bind(oi)
            .bind(id)
            .bind(fetch)
            .fetch_all(&state.db)
            .await?
        }
    };

    let mut items: Vec<Page> = rows
        .into_iter()
        .map(|r| {
            let mut p = Page::from((
                r.0, r.1, r.2, r.3, r.4, r.5, r.6, r.7, r.8, r.9, r.10,
            ));
            p.has_children = Some(r.11);
            p
        })
        .collect();

    let next_cursor = if items.len() as i64 > limit {
        items.pop();
        items.last().map(|p| encode_cursor(p.order_index, p.id))
    } else {
        None
    };

    Ok(Json(PageListResponse { items, next_cursor }))
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
    let (set_icon, icon_value) = match body.icon {
        Some(v) => (true, v),
        None => (false, None),
    };

    let row: PageRow = sqlx::query_as(&format!(
        "update pages set
            title = coalesce($2, title),
            order_index = coalesce($3, order_index),
            parent_page_id = case when $4 then $5 else parent_page_id end,
            icon = case when $6 then $7 else icon end,
            updated_at = now()
         where id = $1
         returning {PAGE_COLUMNS}"
    ))
    .bind(id)
    .bind(body.title)
    .bind(body.order_index)
    .bind(set_parent)
    .bind(parent_value)
    .bind(set_icon)
    .bind(icon_value)
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
    let row: PageRow = sqlx::query_as(&format!(
        "select {PAGE_COLUMNS} from pages where id = $1 and archived_at is null"
    ))
    .bind(perm.page_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AuthError::NotFound)?;

    let mut page: Page = row.into();
    page.role = Some(match perm.role {
        Role::Viewer => "viewer",
        Role::Editor => "editor",
    }.to_string());

    Ok(Json(page))
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

async fn list_shared_pages(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<Vec<Page>>, AuthError> {
    let rows: Vec<PageRow> = sqlx::query_as(&format!(
        "select {} from pages p
         join permissions perm on perm.subject_type = 'page' and perm.subject_id = p.id
         where perm.principal_type = 'user' and perm.principal_id = $1
           and (perm.expires_at is null or perm.expires_at > now())
           and p.archived_at is null
         order by p.updated_at desc",
        PAGE_COLUMNS
            .split(", ")
            .map(|c| format!("p.{c}"))
            .collect::<Vec<_>>()
            .join(", ")
    ))
    .bind(user.user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(rows.into_iter().map(Page::from).collect()))
}

async fn list_favorite_pages(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<Vec<Page>>, AuthError> {
    let rows: Vec<PageRow> = sqlx::query_as(&format!(
        "select {} from pages p
         join page_favorites f on f.page_id = p.id
         where f.user_id = $1 and p.archived_at is null
         order by f.created_at desc",
        PAGE_COLUMNS
            .split(", ")
            .map(|c| format!("p.{c}"))
            .collect::<Vec<_>>()
            .join(", ")
    ))
    .bind(user.user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(rows.into_iter().map(Page::from).collect()))
}

async fn favorite_page(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AuthError> {
    sqlx::query(
        "insert into page_favorites (user_id, page_id) values ($1, $2)
         on conflict (user_id, page_id) do nothing",
    )
    .bind(user.user_id)
    .bind(id)
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({ "message": "favorited" })))
}

async fn unfavorite_page(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AuthError> {
    sqlx::query("delete from page_favorites where user_id = $1 and page_id = $2")
        .bind(user.user_id)
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({ "message": "unfavorited" })))
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

    let q = query.q.trim();
    if q.is_empty() {
        return Ok(Json(vec![]));
    }

    let like = format!("%{}%", q.replace('%', "\\%").replace('_', "\\_"));
    let cols = PAGE_COLUMNS
        .split(", ")
        .map(|c| format!("p.{c}"))
        .collect::<Vec<_>>()
        .join(", ");

    // Postgres rejects SELECT DISTINCT ... ORDER BY p.updated_at when updated_at
    // isn't in the DISTINCT select list — use a subquery + DISTINCT ON instead.
    // Cap at 50 — palette UX, not a full export.
    let rows: Vec<PageRow> = sqlx::query_as(&format!(
        "select {cols} from (
           select distinct on (p.id) {cols}
           from pages p
           left join page_content pc on pc.page_id = p.id
           left join users creator on creator.id = p.created_by
           where p.workspace_id = $1
             and p.archived_at is null
             and (
               p.title ilike $2 escape '\\'
               or coalesce(pc.plain_text, '') ilike $2 escape '\\'
               or (
                 pc.search_vector is not null
                 and length(trim($3)) > 0
                 and pc.search_vector @@ plainto_tsquery('english', $3)
               )
               or creator.email ilike $2 escape '\\'
               or coalesce(creator.display_name, '') ilike $2 escape '\\'
               or exists (
                 select 1 from permissions perm
                 join users u on u.id = perm.principal_id
                 where perm.subject_type = 'page' and perm.subject_id = p.id
                   and perm.principal_type = 'user'
                   and (perm.expires_at is null or perm.expires_at > now())
                   and (
                     u.email ilike $2 escape '\\'
                     or coalesce(u.display_name, '') ilike $2 escape '\\'
                   )
               )
               or exists (
                 select 1 from comments c
                 join users u on u.id = c.author_id
                 where c.page_id = p.id
                   and (
                     u.email ilike $2 escape '\\'
                     or coalesce(u.display_name, '') ilike $2 escape '\\'
                   )
               )
             )
           order by p.id, p.updated_at desc
         ) p
         order by p.updated_at desc
         limit 50"
    ))
    .bind(workspace_id)
    .bind(&like)
    .bind(q)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(rows.into_iter().map(Page::from).collect()))
}
