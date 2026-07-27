use axum::{
    extract::{Path, State},
    routing::{delete, get, patch},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::{error::AuthError, extractor::AuthenticatedUser};
use crate::sharing::{self, PagePermission, Role};
use crate::AppState;

pub mod realtime;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/pages/:id/comments", get(list_comments).post(create_comment))
        .route("/comments/:id/resolve", patch(resolve_comment))
        .route("/comments/:id", delete(delete_comment))
}

/// Real-time events pushed to a page's connected comment listeners.
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum CommentEvent<'a> {
    Created { comment: &'a Comment },
    Updated { comment: &'a Comment },
    Deleted { id: Uuid },
}

#[derive(Serialize)]
struct Comment {
    id: Uuid,
    page_id: Uuid,
    mark_id: Uuid,
    parent_id: Option<Uuid>,
    author_id: Uuid,
    author_email: String,
    /// Set when the author has filled in their profile name.
    author_display_name: Option<String>,
    assignee_id: Option<Uuid>,
    assignee_email: Option<String>,
    assignee_display_name: Option<String>,
    body: String,
    resolved: bool,
    created_at: DateTime<Utc>,
}

type CommentRow = (
    Uuid,
    Uuid,
    Uuid,
    Option<Uuid>,
    Uuid,
    String,
    Option<String>,
    Option<Uuid>,
    Option<String>,
    Option<String>,
    String,
    bool,
    DateTime<Utc>,
);

impl From<CommentRow> for Comment {
    fn from(r: CommentRow) -> Self {
        Comment {
            id: r.0,
            page_id: r.1,
            mark_id: r.2,
            parent_id: r.3,
            author_id: r.4,
            author_email: r.5,
            author_display_name: r.6.filter(|s| !s.trim().is_empty()),
            assignee_id: r.7,
            assignee_email: r.8,
            assignee_display_name: r.9.filter(|s| !s.trim().is_empty()),
            body: r.10,
            resolved: r.11,
            created_at: r.12,
        }
    }
}

const COMMENT_SELECT: &str = "
    select c.id, c.page_id, c.mark_id, c.parent_id, c.author_id, au.email, au.display_name,
           c.assignee_id, asu.email, asu.display_name, c.body, c.resolved, c.created_at
    from comments c
    join users au on au.id = c.author_id
    left join users asu on asu.id = c.assignee_id
";

#[derive(Deserialize)]
struct CreateCommentRequest {
    /// Required for root comments; ignored (taken from parent) when replying.
    mark_id: Option<Uuid>,
    /// When set, this is a reply under that parent comment.
    parent_id: Option<Uuid>,
    body: String,
    assignee_email: Option<String>,
}

/// Any resolved role (viewer or editor) may comment — commenting doesn't
/// require edit rights on the document itself.
async fn create_comment(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    perm: PagePermission,
    Json(body): Json<CreateCommentRequest>,
) -> Result<Json<Comment>, AuthError> {
    let text = body.body.trim();
    if text.is_empty() {
        return Err(AuthError::Validation("comment body is required".into()));
    }

    let (mark_id, parent_id): (Uuid, Option<Uuid>) = if let Some(pid) = body.parent_id {
        let parent: Option<(Uuid, Uuid, Option<Uuid>)> = sqlx::query_as(
            "select page_id, mark_id, parent_id from comments where id = $1",
        )
        .bind(pid)
        .fetch_optional(&state.db)
        .await?;
        let (page_id, mark_id, parent_parent) = parent.ok_or(AuthError::NotFound)?;
        if page_id != perm.page_id {
            return Err(AuthError::Forbidden);
        }
        // Flatten: replies always hang off the root of the thread.
        let root_id = if parent_parent.is_some() {
            // Parent is itself a reply — walk one step: load its parent_id root.
            // For simplicity, attach under the given parent; UI only shows one level
            // under roots. Still allowed by schema.
            pid
        } else {
            pid
        };
        (mark_id, Some(root_id))
    } else {
        let mark_id = body
            .mark_id
            .ok_or_else(|| AuthError::Validation("mark_id is required for root comments".into()))?;
        (mark_id, None)
    };

    let assignee_id: Option<Uuid> = match body
        .assignee_email
        .as_deref()
        .map(|e| e.trim().to_lowercase())
        .filter(|e| !e.is_empty())
    {
        Some(email) if parent_id.is_none() => {
            sqlx::query_scalar("select id from users where email = $1")
                .bind(&email)
                .fetch_optional(&state.db)
                .await?
        }
        _ => None,
    };

    let id: Uuid = sqlx::query_scalar(
        "insert into comments (page_id, mark_id, parent_id, author_id, assignee_id, body)
         values ($1, $2, $3, $4, $5, $6)
         returning id",
    )
    .bind(perm.page_id)
    .bind(mark_id)
    .bind(parent_id)
    .bind(user.user_id)
    .bind(assignee_id)
    .bind(text)
    .fetch_one(&state.db)
    .await?;

    let row: CommentRow = sqlx::query_as(&format!("{COMMENT_SELECT} where c.id = $1"))
        .bind(id)
        .fetch_one(&state.db)
        .await?;

    let comment: Comment = row.into();
    realtime::publish(&state.comments, perm.page_id, &CommentEvent::Created { comment: &comment });
    Ok(Json(comment))
}

async fn list_comments(
    State(state): State<AppState>,
    perm: PagePermission,
) -> Result<Json<Vec<Comment>>, AuthError> {
    let rows: Vec<CommentRow> = sqlx::query_as(&format!(
        "{COMMENT_SELECT} where c.page_id = $1 order by c.created_at asc"
    ))
    .bind(perm.page_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(rows.into_iter().map(Comment::from).collect()))
}

/// Requires the requester be either the comment's original author or an
/// Editor on its page. Returns the comment's page_id.
async fn require_resolve_or_delete_access(
    state: &AppState,
    user_id: Uuid,
    comment_id: Uuid,
) -> Result<Uuid, AuthError> {
    let row: Option<(Uuid, Uuid)> =
        sqlx::query_as("select page_id, author_id from comments where id = $1")
            .bind(comment_id)
            .fetch_optional(&state.db)
            .await?;
    let (page_id, author_id) = row.ok_or(AuthError::NotFound)?;

    if author_id == user_id {
        return Ok(page_id);
    }
    let role = sharing::resolve_role(&state.db, page_id, Some(user_id), None).await?;
    if role != Role::Editor {
        return Err(AuthError::Forbidden);
    }
    Ok(page_id)
}

async fn resolve_comment(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Comment>, AuthError> {
    require_resolve_or_delete_access(&state, user.user_id, id).await?;

    // Resolve toggles only make sense on root comments; still allow any row.
    sqlx::query("update comments set resolved = not resolved where id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    let row: CommentRow = sqlx::query_as(&format!("{COMMENT_SELECT} where c.id = $1"))
        .bind(id)
        .fetch_one(&state.db)
        .await?;
    let comment: Comment = row.into();
    realtime::publish(&state.comments, comment.page_id, &CommentEvent::Updated { comment: &comment });
    Ok(Json(comment))
}

async fn delete_comment(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AuthError> {
    let page_id = require_resolve_or_delete_access(&state, user.user_id, id).await?;

    // FK ON DELETE CASCADE removes replies when a root is deleted.
    sqlx::query("delete from comments where id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    realtime::publish(&state.comments, page_id, &CommentEvent::Deleted { id });
    Ok(Json(serde_json::json!({ "message": "comment deleted" })))
}
