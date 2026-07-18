use axum::{extract::{Path, State}, routing::get, Json, Router};
use serde::Serialize;
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

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_workspaces))
        .route("/:id", get(get_workspace))
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

/// Every new user gets a personal workspace they own, created alongside their account.
pub async fn create_default_workspace(db: &PgPool, owner_id: Uuid) -> Result<Workspace, AuthError> {
    let name = "My Workspace".to_string();
    let slug = format!("workspace-{}", &owner_id.simple().to_string()[..8]);

    let (id,): (Uuid,) = sqlx::query_as(
        "insert into workspaces (name, slug, owner_id) values ($1, $2, $3) returning id",
    )
    .bind(&name)
    .bind(&slug)
    .bind(owner_id)
    .fetch_one(db)
    .await?;

    sqlx::query("insert into workspace_members (workspace_id, user_id, role) values ($1, $2, 'owner')")
        .bind(id)
        .bind(owner_id)
        .execute(db)
        .await?;

    Ok(Workspace { id, name, slug })
}
