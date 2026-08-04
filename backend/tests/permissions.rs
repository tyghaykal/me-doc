//! Integration tests for `sharing::resolve_role` permission resolution.
//! Each `#[sqlx::test]` gets its own freshly-migrated database (migrations in
//! ./migrations are applied automatically); rows are inserted directly.

mod common;

use common::*;
use me_doc_backend::sharing::{has_workspace_access, resolve_role, Role};
use sqlx::PgPool;

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

/// `has_workspace_access` (used to gate attachment/avatar reads, see
/// finding #3 of the security audit) must accept a workspace member...
#[sqlx::test]
async fn workspace_access_allows_member(pool: PgPool) {
    let owner = insert_user(&pool, "owner@example.com").await;
    let ws = insert_workspace(&pool, owner, "ws").await;
    add_member(&pool, ws, owner, "owner").await;

    assert!(has_workspace_access(&pool, ws, Some(owner), None).await.unwrap());
}

/// ...and a non-member who was individually granted access to a page inside
/// that workspace (an external share recipient)...
#[sqlx::test]
async fn workspace_access_allows_non_member_with_page_grant(pool: PgPool) {
    let owner = insert_user(&pool, "owner@example.com").await;
    let outsider = insert_user(&pool, "outsider@example.com").await;
    let ws = insert_workspace(&pool, owner, "ws").await;
    let page = insert_page(&pool, ws, None, owner, "page").await;
    grant_page(&pool, page, outsider, "viewer").await;

    assert!(has_workspace_access(&pool, ws, Some(outsider), None).await.unwrap());
}

/// ...and an anonymous caller presenting a valid public-link token for a page
/// in that workspace...
#[sqlx::test]
async fn workspace_access_allows_valid_link_token(pool: PgPool) {
    let owner = insert_user(&pool, "owner@example.com").await;
    let ws = insert_workspace(&pool, owner, "ws").await;
    let page = insert_page(&pool, ws, None, owner, "page").await;
    grant_page_link(&pool, page, "tok123", "viewer").await;

    assert!(has_workspace_access(&pool, ws, None, Some("tok123")).await.unwrap());
}

/// ...but must reject an unrelated user and an anonymous caller with no
/// grant at all — this is the actual fix for the bucket having gone from
/// "anonymous public read" to "checked before every download".
#[sqlx::test]
async fn workspace_access_denies_unrelated_and_anonymous(pool: PgPool) {
    let owner = insert_user(&pool, "owner@example.com").await;
    let stranger = insert_user(&pool, "stranger@example.com").await;
    let ws = insert_workspace(&pool, owner, "ws").await;

    assert!(!has_workspace_access(&pool, ws, Some(stranger), None).await.unwrap());
    assert!(!has_workspace_access(&pool, ws, None, None).await.unwrap());
    assert!(!has_workspace_access(&pool, ws, None, Some("not-a-real-token"))
        .await
        .unwrap());
}
