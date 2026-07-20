use axum::{
    extract::{Path, State},
    routing::{delete, get},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::error::AuthError;
use crate::auth::extractor::AuthenticatedUser;
use crate::AppState;

#[derive(Debug, Serialize)]
pub struct Workspace {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Serialize)]
pub struct Member {
    pub user_id: Uuid,
    pub email: String,
    pub role: String,
}

#[derive(Deserialize)]
struct CreateWorkspaceRequest {
    name: String,
}

#[derive(Deserialize)]
struct AddMemberRequest {
    email: String,
    role: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_workspaces).post(create_workspace))
        .route("/:id", get(get_workspace))
        .route("/:id/members", get(list_members).post(add_member))
        .route("/:id/members/:user_id", delete(remove_member))
}

async fn list_workspaces(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<Vec<Workspace>>, AuthError> {
    let rows: Vec<(Uuid, String, String)> = sqlx::query_as(
        "select w.id, w.name, w.slug from workspaces w
         join workspace_members m on m.workspace_id = w.id
         where m.user_id = $1
         order by w.created_at asc",
    )
    .bind(user.user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(
        rows.into_iter()
            .map(|(id, name, slug)| Workspace { id, name, slug })
            .collect(),
    ))
}

async fn get_workspace(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Workspace>, AuthError> {
    let row: Option<(Uuid, String, String)> = sqlx::query_as(
        "select w.id, w.name, w.slug from workspaces w
         where w.id = $1
           and exists (select 1 from workspace_members where workspace_id = w.id and user_id = $2)",
    )
    .bind(id)
    .bind(user.user_id)
    .fetch_optional(&state.db)
    .await?;

    let (id, name, slug) = row.ok_or(AuthError::NotFound)?;
    Ok(Json(Workspace { id, name, slug }))
}

async fn insert_workspace(db: &PgPool, name: &str, owner_id: Uuid) -> Result<Workspace, AuthError> {
    let slug = format!("workspace-{}", &Uuid::new_v4().simple().to_string()[..8]);

    let (id,): (Uuid,) = sqlx::query_as(
        "insert into workspaces (name, slug, owner_id) values ($1, $2, $3) returning id",
    )
    .bind(name)
    .bind(&slug)
    .bind(owner_id)
    .fetch_one(db)
    .await?;

    sqlx::query("insert into workspace_members (workspace_id, user_id, role) values ($1, $2, 'owner')")
        .bind(id)
        .bind(owner_id)
        .execute(db)
        .await?;

    Ok(Workspace {
        id,
        name: name.to_string(),
        slug,
    })
}

/// Every new user gets a personal workspace they own, created alongside their account.
pub async fn create_default_workspace(db: &PgPool, owner_id: Uuid) -> Result<Workspace, AuthError> {
    insert_workspace(db, "My Workspace", owner_id).await
}

async fn member_role(
    db: &PgPool,
    workspace_id: Uuid,
    user_id: Uuid,
) -> Result<Option<String>, AuthError> {
    let role: Option<String> = sqlx::query_scalar(
        "select role from workspace_members where workspace_id = $1 and user_id = $2",
    )
    .bind(workspace_id)
    .bind(user_id)
    .fetch_optional(db)
    .await?;
    Ok(role)
}

async fn create_workspace(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<CreateWorkspaceRequest>,
) -> Result<Json<Workspace>, AuthError> {
    let name = body.name.trim();
    if name.is_empty() {
        return Err(AuthError::Validation("workspace name is required".into()));
    }
    let ws = insert_workspace(&state.db, name, user.user_id).await?;
    Ok(Json(ws))
}

async fn list_members(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<Member>>, AuthError> {
    if member_role(&state.db, id, user.user_id).await?.is_none() {
        return Err(AuthError::Forbidden);
    }

    let rows: Vec<(Uuid, String, String)> = sqlx::query_as(
        "select m.user_id, u.email, m.role from workspace_members m
         join users u on u.id = m.user_id
         where m.workspace_id = $1
         order by m.created_at asc",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(
        rows.into_iter()
            .map(|(user_id, email, role)| Member { user_id, email, role })
            .collect(),
    ))
}

async fn add_member(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
    Json(body): Json<AddMemberRequest>,
) -> Result<Json<Member>, AuthError> {
    let caller_role = member_role(&state.db, id, user.user_id).await?;
    if !matches!(caller_role.as_deref(), Some("owner") | Some("admin")) {
        return Err(AuthError::Forbidden);
    }

    if !matches!(body.role.as_str(), "admin" | "member" | "guest") {
        return Err(AuthError::Validation(
            "role must be 'admin', 'member', or 'guest'".into(),
        ));
    }

    let email = body.email.trim().to_lowercase();
    let target: Option<(Uuid,)> = sqlx::query_as("select id from users where email = $1")
        .bind(&email)
        .fetch_optional(&state.db)
        .await?;
    let (target_id,) = target.ok_or_else(|| AuthError::Validation("no user with that email".into()))?;

    if member_role(&state.db, id, target_id).await?.is_some() {
        return Err(AuthError::Validation("user is already a member".into()));
    }

    sqlx::query("insert into workspace_members (workspace_id, user_id, role) values ($1, $2, $3)")
        .bind(id)
        .bind(target_id)
        .bind(&body.role)
        .execute(&state.db)
        .await?;

    Ok(Json(Member {
        user_id: target_id,
        email,
        role: body.role,
    }))
}

async fn remove_member(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path((id, target_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, AuthError> {
    let caller_role = member_role(&state.db, id, user.user_id).await?;
    let is_admin = matches!(caller_role.as_deref(), Some("owner") | Some("admin"));
    let is_self = target_id == user.user_id;
    if !is_admin && !is_self {
        return Err(AuthError::Forbidden);
    }

    let owner_id: Option<(Uuid,)> = sqlx::query_as("select owner_id from workspaces where id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?;
    let (owner_id,) = owner_id.ok_or(AuthError::NotFound)?;
    if target_id == owner_id {
        return Err(AuthError::Validation("cannot remove the workspace owner".into()));
    }

    sqlx::query("delete from workspace_members where workspace_id = $1 and user_id = $2")
        .bind(id)
        .bind(target_id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({ "message": "member removed" })))
}
