use std::collections::HashMap;

use axum::{
    async_trait,
    extract::{FromRequestParts, Path, Query, State},
    http::request::Parts,
    routing::{delete, post},
    Json, Router,
};
use rand::RngCore;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::{error::AuthError, extractor::AuthenticatedUser};
use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/pages/:id/share", post(share_with_user))
        .route("/pages/:id/share/link", post(share_link))
        .route("/permissions/:id", delete(delete_permission))
}

#[derive(Deserialize)]
struct ShareRequest {
    email: String,
    role: String,
}

#[derive(Deserialize)]
struct ShareLinkRequest {
    role: String,
}

/// Validates a client-supplied role string, rejecting anything but viewer/editor.
fn validate_role(role: &str) -> Result<Role, AuthError> {
    Role::from_db(role).ok_or_else(|| AuthError::Validation("role must be 'viewer' or 'editor'".into()))
}

fn generate_link_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, bytes)
}

async fn share_with_user(
    State(state): State<AppState>,
    perm: PagePermission,
    Json(body): Json<ShareRequest>,
) -> Result<Json<Value>, AuthError> {
    let role = validate_role(&body.role)?;
    if perm.role != Role::Editor {
        return Err(AuthError::Forbidden);
    }

    let email = body.email.trim().to_lowercase();
    let target: Option<(Uuid,)> = sqlx::query_as("select id from users where email = $1")
        .bind(&email)
        .fetch_optional(&state.db)
        .await?;
    let (target_id,) = target.ok_or_else(|| AuthError::Validation("no user with that email".into()))?;

    let grant: (Uuid, String) = sqlx::query_as(
        "insert into permissions (subject_type, subject_id, principal_type, principal_id, role)
         values ('page', $1, 'user', $2, $3)
         returning id, role",
    )
    .bind(perm.page_id)
    .bind(target_id)
    .bind(role_to_db(role))
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "id": grant.0, "role": grant.1 })))
}

async fn share_link(
    State(state): State<AppState>,
    perm: PagePermission,
    Json(body): Json<ShareLinkRequest>,
) -> Result<Json<Value>, AuthError> {
    let role = validate_role(&body.role)?;
    if perm.role != Role::Editor {
        return Err(AuthError::Forbidden);
    }

    let token = generate_link_token();
    sqlx::query(
        "insert into permissions (subject_type, subject_id, principal_type, principal_id, link_token, role)
         values ('page', $1, 'link', null, $2, $3)",
    )
    .bind(perm.page_id)
    .bind(&token)
    .bind(role_to_db(role))
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "link_token": token, "role": role_to_db(role) })))
}

async fn delete_permission(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, AuthError> {
    let subject: Option<(Uuid,)> =
        sqlx::query_as("select subject_id from permissions where id = $1")
            .bind(id)
            .fetch_optional(&state.db)
            .await?;
    let (subject_id,) = subject.ok_or(AuthError::NotFound)?;

    // MVP grants are page-level only, so treat subject_id as a page id and
    // reuse resolve_role to require the requester have edit rights on it.
    let role = resolve_role(&state.db, subject_id, Some(user.user_id), None).await?;
    if role != Role::Editor {
        return Err(AuthError::Forbidden);
    }

    sqlx::query("delete from permissions where id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "message": "permission removed" })))
}

fn role_to_db(role: Role) -> &'static str {
    match role {
        Role::Viewer => "viewer",
        Role::Editor => "editor",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Viewer,
    Editor,
}

impl Role {
    fn from_db(s: &str) -> Option<Role> {
        match s {
            "viewer" => Some(Role::Viewer),
            "editor" => Some(Role::Editor),
            _ => None,
        }
    }
}

/// Maps a `workspace_members.role` to an effective sharing role: any real
/// member (owner/admin/member) can edit; a guest is read-only.
fn membership_role(db_role: &str) -> Role {
    match db_role {
        "guest" => Role::Viewer,
        _ => Role::Editor,
    }
}

pub struct PagePermission {
    pub page_id: Uuid,
    pub role: Role,
}

#[derive(Deserialize)]
struct LinkQuery {
    link: Option<String>,
}

#[async_trait]
impl FromRequestParts<AppState> for PagePermission {
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let params = Path::<HashMap<String, String>>::from_request_parts(parts, state)
            .await
            .map_err(|_| AuthError::NotFound)?
            .0;
        let raw = params
            .get("page_id")
            .or_else(|| params.get("id"))
            .ok_or(AuthError::NotFound)?;
        let page_id = Uuid::parse_str(raw).map_err(|_| AuthError::NotFound)?;

        // Principal: an authenticated user (Bearer token) takes precedence; if
        // there's no user, fall back to a `?link=<token>` public-link grant.
        let user_id: Option<Uuid> = match AuthenticatedUser::from_request_parts(parts, state).await {
            Ok(u) => Some(u.user_id),
            Err(_) => None,
        };
        let link_token: Option<String> = if user_id.is_none() {
            Query::<LinkQuery>::from_request_parts(parts, state)
                .await
                .ok()
                .and_then(|q| q.0.link)
        } else {
            None
        };
        if user_id.is_none() && link_token.is_none() {
            return Err(AuthError::Forbidden);
        }

        let role = resolve_role(&state.db, page_id, user_id, link_token.as_deref()).await?;
        Ok(PagePermission { page_id, role })
    }
}

pub async fn resolve_role(
    db: &PgPool,
    page_id: Uuid,
    user_id: Option<Uuid>,
    link_token: Option<&str>,
) -> Result<Role, AuthError> {
    // 1. Page-level grant, walking parent_page_id up to the root. Closest
    //    ancestor wins, so a page-level override beats an inherited one.
    let page_role: Option<String> = sqlx::query_scalar(
        r#"
        with recursive page_chain as (
            select id, parent_page_id, workspace_id, 0 as depth
            from pages where id = $1
            union all
            select p.id, p.parent_page_id, p.workspace_id, pc.depth + 1
            from pages p join page_chain pc on p.id = pc.parent_page_id
        )
        select perm.role
        from page_chain pc
        join permissions perm
          on perm.subject_type = 'page' and perm.subject_id = pc.id
        where (perm.principal_id = $2 or perm.link_token = $3)
          and (perm.expires_at is null or perm.expires_at > now())
        order by pc.depth asc
        limit 1
        "#,
    )
    .bind(page_id)
    .bind(user_id)
    .bind(link_token)
    .fetch_optional(db)
    .await?;
    if let Some(r) = page_role.as_deref().and_then(Role::from_db) {
        return Ok(r);
    }

    let workspace_id: Option<Uuid> =
        sqlx::query_scalar("select workspace_id from pages where id = $1")
            .bind(page_id)
            .fetch_optional(db)
            .await?;
    let workspace_id = workspace_id.ok_or(AuthError::NotFound)?;

    // 2. Workspace-level grant.
    let ws_role: Option<String> = sqlx::query_scalar(
        r#"
        select role from permissions
        where subject_type = 'workspace' and subject_id = $1
          and (principal_id = $2 or link_token = $3)
          and (expires_at is null or expires_at > now())
        limit 1
        "#,
    )
    .bind(workspace_id)
    .bind(user_id)
    .bind(link_token)
    .fetch_optional(db)
    .await?;
    if let Some(r) = ws_role.as_deref().and_then(Role::from_db) {
        return Ok(r);
    }

    // 3. Workspace membership (users only — a link grant never implies membership).
    if let Some(uid) = user_id {
        let member_role: Option<String> = sqlx::query_scalar(
            "select role from workspace_members where workspace_id = $1 and user_id = $2",
        )
        .bind(workspace_id)
        .bind(uid)
        .fetch_optional(db)
        .await?;
        if let Some(m) = member_role {
            return Ok(membership_role(&m));
        }
    }

    Err(AuthError::Forbidden)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_from_db() {
        assert_eq!(Role::from_db("viewer"), Some(Role::Viewer));
        assert_eq!(Role::from_db("editor"), Some(Role::Editor));
        assert_eq!(Role::from_db("owner"), None);
    }

    #[test]
    fn membership_mapping() {
        assert_eq!(membership_role("owner"), Role::Editor);
        assert_eq!(membership_role("admin"), Role::Editor);
        assert_eq!(membership_role("member"), Role::Editor);
        assert_eq!(membership_role("guest"), Role::Viewer);
    }
}
