//! Integration tests for `sharing::resolve_role` permission resolution.
//! Each `#[sqlx::test]` gets its own freshly-migrated database (migrations in
//! ./migrations are applied automatically); rows are inserted directly.

use me_doc_backend::sharing::{resolve_role, Role};
use sqlx::PgPool;
use uuid::Uuid;

async fn insert_user(pool: &PgPool, email: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("insert into users (id, email, password_hash) values ($1, $2, 'x')")
        .bind(id)
        .bind(email)
        .execute(pool)
        .await
        .unwrap();
    id
}

async fn insert_workspace(pool: &PgPool, owner_id: Uuid, slug: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("insert into workspaces (id, name, slug, owner_id) values ($1, $2, $3, $4)")
        .bind(id)
        .bind(slug)
        .bind(slug)
        .bind(owner_id)
        .execute(pool)
        .await
        .unwrap();
    id
}

async fn insert_page(pool: &PgPool, workspace_id: Uuid, parent: Option<Uuid>, created_by: Uuid, slug: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "insert into pages (id, workspace_id, parent_page_id, slug, created_by) values ($1, $2, $3, $4, $5)",
    )
    .bind(id)
    .bind(workspace_id)
    .bind(parent)
    .bind(slug)
    .bind(created_by)
    .execute(pool)
    .await
    .unwrap();
    id
}

async fn grant_page(pool: &PgPool, page_id: Uuid, principal_id: Uuid, role: &str) {
    sqlx::query(
        "insert into permissions (subject_type, subject_id, principal_type, principal_id, role)
         values ('page', $1, 'user', $2, $3)",
    )
    .bind(page_id)
    .bind(principal_id)
    .bind(role)
    .execute(pool)
    .await
    .unwrap();
}

async fn add_member(pool: &PgPool, workspace_id: Uuid, user_id: Uuid, role: &str) {
    sqlx::query("insert into workspace_members (workspace_id, user_id, role) values ($1, $2, $3)")
        .bind(workspace_id)
        .bind(user_id)
        .bind(role)
        .execute(pool)
        .await
        .unwrap();
}

/// A page-level grant resolves to that role even when the user is not a member
/// of the workspace at all.
#[sqlx::test]
async fn page_grant_resolves_without_membership(pool: PgPool) {
    let user = insert_user(&pool, "u@example.com").await;
    let ws = insert_workspace(&pool, user, "ws").await;
    let page = insert_page(&pool, ws, None, user, "page").await;
    grant_page(&pool, page, user, "editor").await;

    let role = resolve_role(&pool, page, Some(user), None).await.unwrap();
    assert_eq!(role, Role::Editor);
}

/// A page with no direct grant inherits the closest ancestor's grant via the
/// parent_page_id chain.
#[sqlx::test]
async fn child_page_inherits_ancestor_grant(pool: PgPool) {
    let user = insert_user(&pool, "u@example.com").await;
    let ws = insert_workspace(&pool, user, "ws").await;
    let parent = insert_page(&pool, ws, None, user, "parent").await;
    let child = insert_page(&pool, ws, Some(parent), user, "child").await;
    grant_page(&pool, parent, user, "viewer").await;

    let role = resolve_role(&pool, child, Some(user), None).await.unwrap();
    assert_eq!(role, Role::Viewer);
}

/// With no permissions rows, resolution falls back to workspace membership:
/// an 'owner' maps to Editor, a 'guest' maps to Viewer.
#[sqlx::test]
async fn membership_fallback_maps_roles(pool: PgPool) {
    let owner = insert_user(&pool, "owner@example.com").await;
    let guest = insert_user(&pool, "guest@example.com").await;
    let ws = insert_workspace(&pool, owner, "ws").await;
    let page = insert_page(&pool, ws, None, owner, "page").await;
    add_member(&pool, ws, owner, "owner").await;
    add_member(&pool, ws, guest, "guest").await;

    let owner_role = resolve_role(&pool, page, Some(owner), None).await.unwrap();
    assert_eq!(owner_role, Role::Editor);

    let guest_role = resolve_role(&pool, page, Some(guest), None).await.unwrap();
    assert_eq!(guest_role, Role::Viewer);
}
