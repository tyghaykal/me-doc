//! Integration tests for `/workspaces` — creation, the membership-scoped
//! listing, and who may change the member roster. Redis and Mailpit are shared
//! across the suite, so each test uses a unique email (see the note in
//! `auth.rs`).

mod common;

use axum::body::Body;
use axum::http::header::{AUTHORIZATION, CONTENT_TYPE};
use axum::http::{Request, StatusCode};
use common::*;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

const PASSWORD: &str = "correct-horse-battery";

fn uniq(tag: &str) -> String {
    format!("{tag}-{}@example.com", Uuid::new_v4().simple())
}

fn authed(method: &str, uri: &str, token: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap()
}

fn json_authed(method: &str, uri: &str, token: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

/// Creating a workspace returns it and makes the creator a member, so it shows
/// up alongside the personal workspace that registration provisions.
#[sqlx::test]
async fn create_workspace_adds_it_to_the_callers_list(pool: PgPool) {
    let state = common::test_state(pool).await;
    let app = me_doc_backend::build_app(state);

    let client = register_and_login(&app, &uniq("wscreate"), PASSWORD).await;

    let (status, _, body) = send(
        &app,
        json_authed("POST", "/workspaces", &client.access_token, json!({ "name": "Team Docs" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["name"], "Team Docs");
    let created = body["id"].as_str().unwrap().to_string();
    assert!(body["slug"].as_str().unwrap().starts_with("workspace-"));

    let (status, _, body) = send(&app, authed("GET", "/workspaces", &client.access_token)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let names: Vec<&str> = body
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, vec!["My Workspace", "Team Docs"]);

    let (status, _, body) = send(
        &app,
        authed("GET", &format!("/workspaces/{created}"), &client.access_token),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["id"], created.as_str());
}

/// A blank name is refused rather than creating an unnamed workspace.
#[sqlx::test]
async fn create_workspace_requires_a_name(pool: PgPool) {
    let state = common::test_state(pool).await;
    let app = me_doc_backend::build_app(state);

    let client = register_and_login(&app, &uniq("wsblank"), PASSWORD).await;

    let (status, _, body) = send(
        &app,
        json_authed("POST", "/workspaces", &client.access_token, json!({ "name": "   " })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "workspace name is required");
}

/// The listing joins through `workspace_members`, so someone else's workspace
/// is invisible even though both rows live in the same table — and fetching it
/// by id is a 404, not a leak.
#[sqlx::test]
async fn list_workspaces_excludes_ones_the_caller_does_not_belong_to(pool: PgPool) {
    let state = common::test_state(pool.clone()).await;
    let app = me_doc_backend::build_app(state);

    let client = register_and_login(&app, &uniq("wsscope"), PASSWORD).await;

    let outsider = insert_user(&pool, &uniq("outsider")).await;
    let foreign_ws = insert_workspace(&pool, outsider, "outsiders-space").await;
    add_member(&pool, foreign_ws, outsider, "owner").await;

    let (status, _, body) = send(&app, authed("GET", "/workspaces", &client.access_token)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let ids: Vec<&str> = body
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids.len(), 1, "only the personal workspace: {body}");
    assert!(!ids.contains(&foreign_ws.to_string().as_str()));

    let (status, _, _) = send(
        &app,
        authed("GET", &format!("/workspaces/{foreign_ws}"), &client.access_token),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// An owner can add a member by email and then remove them again.
#[sqlx::test]
async fn owner_can_add_and_remove_a_member(pool: PgPool) {
    let state = common::test_state(pool.clone()).await;
    let app = me_doc_backend::build_app(state);

    let owner = register_and_login(&app, &uniq("wsowner"), PASSWORD).await;
    let (_, _, list) = send(&app, authed("GET", "/workspaces", &owner.access_token)).await;
    let ws = list[0]["id"].as_str().unwrap().to_string();

    let invitee_email = uniq("invitee");
    let invitee = insert_user(&pool, &invitee_email).await;

    let (status, _, body) = send(
        &app,
        json_authed(
            "POST",
            &format!("/workspaces/{ws}/members"),
            &owner.access_token,
            json!({ "email": invitee_email, "role": "member" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["user_id"], invitee.to_string());
    assert_eq!(body["role"], "member");

    let (status, _, body) = send(
        &app,
        authed("GET", &format!("/workspaces/{ws}/members"), &owner.access_token),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body.as_array().unwrap().len(), 2);

    let (status, _, body) = send(
        &app,
        authed(
            "DELETE",
            &format!("/workspaces/{ws}/members/{invitee}"),
            &owner.access_token,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["message"], "member removed");

    let (status, _, body) = send(
        &app,
        authed("GET", &format!("/workspaces/{ws}/members"), &owner.access_token),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body.as_array().unwrap().len(), 1);
}

/// Adding is validated on both the role name and the target's existence, and
/// the same person can't be added twice.
#[sqlx::test]
async fn add_member_validates_role_and_target(pool: PgPool) {
    let state = common::test_state(pool.clone()).await;
    let app = me_doc_backend::build_app(state);

    let owner = register_and_login(&app, &uniq("wsvalidate"), PASSWORD).await;
    let (_, _, list) = send(&app, authed("GET", "/workspaces", &owner.access_token)).await;
    let ws = list[0]["id"].as_str().unwrap().to_string();

    let invitee_email = uniq("invitee");
    insert_user(&pool, &invitee_email).await;

    let (status, _, body) = send(
        &app,
        json_authed(
            "POST",
            &format!("/workspaces/{ws}/members"),
            &owner.access_token,
            json!({ "email": invitee_email, "role": "superuser" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "role must be 'admin', 'member', or 'guest'");

    let (status, _, body) = send(
        &app,
        json_authed(
            "POST",
            &format!("/workspaces/{ws}/members"),
            &owner.access_token,
            json!({ "email": "nobody-here@example.com", "role": "member" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "no user with that email");

    for expected in [StatusCode::OK, StatusCode::BAD_REQUEST] {
        let (status, _, body) = send(
            &app,
            json_authed(
                "POST",
                &format!("/workspaces/{ws}/members"),
                &owner.access_token,
                json!({ "email": invitee_email, "role": "member" }),
            ),
        )
        .await;
        assert_eq!(status, expected, "{body}");
    }
}

/// The workspace owner can't be removed, even by themselves — that would leave
/// the workspace orphaned.
#[sqlx::test]
async fn the_owner_cannot_be_removed(pool: PgPool) {
    let state = common::test_state(pool).await;
    let app = me_doc_backend::build_app(state);

    let owner = register_and_login(&app, &uniq("wsownerkeep"), PASSWORD).await;
    let (_, _, list) = send(&app, authed("GET", "/workspaces", &owner.access_token)).await;
    let ws = list[0]["id"].as_str().unwrap().to_string();
    let (_, _, me) = send(&app, {
        Request::builder()
            .method("GET")
            .uri("/auth/me")
            .header(AUTHORIZATION, format!("Bearer {}", owner.access_token))
            .body(Body::empty())
            .unwrap()
    })
    .await;
    let owner_id = me["id"].as_str().unwrap();

    let (status, _, body) = send(
        &app,
        authed(
            "DELETE",
            &format!("/workspaces/{ws}/members/{owner_id}"),
            &owner.access_token,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "cannot remove the workspace owner");
}

/// A plain 'member' is not an admin: they can neither invite nor evict anyone
/// else, though `remove_member`'s `is_self` branch still lets them leave.
#[sqlx::test]
async fn a_plain_member_cannot_change_the_roster(pool: PgPool) {
    let state = common::test_state(pool.clone()).await;
    let app = me_doc_backend::build_app(state);

    let owner_email = uniq("wsrosterowner");
    let member_email = uniq("wsrostermember");
    let owner = register_and_login(&app, &owner_email, PASSWORD).await;
    let member = register_and_login(&app, &member_email, PASSWORD).await;

    let (_, _, list) = send(&app, authed("GET", "/workspaces", &owner.access_token)).await;
    let ws = list[0]["id"].as_str().unwrap().to_string();

    let (status, _, body) = send(
        &app,
        json_authed(
            "POST",
            &format!("/workspaces/{ws}/members"),
            &owner.access_token,
            json!({ "email": member_email, "role": "member" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let member_id = body["user_id"].as_str().unwrap().to_string();

    let bystander_email = uniq("wsbystander");
    let bystander = insert_user(&pool, &bystander_email).await;
    add_member(&pool, ws.parse::<Uuid>().unwrap(), bystander, "member").await;

    let (status, _, body) = send(
        &app,
        json_authed(
            "POST",
            &format!("/workspaces/{ws}/members"),
            &member.access_token,
            json!({ "email": bystander_email, "role": "member" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");

    let (status, _, body) = send(
        &app,
        authed(
            "DELETE",
            &format!("/workspaces/{ws}/members/{bystander}"),
            &member.access_token,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");

    // Leaving on their own account is allowed.
    let (status, _, body) = send(
        &app,
        authed(
            "DELETE",
            &format!("/workspaces/{ws}/members/{member_id}"),
            &member.access_token,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

/// A complete outsider can't even read the member list of a workspace they
/// have no row in.
#[sqlx::test]
async fn non_members_cannot_list_members(pool: PgPool) {
    let state = common::test_state(pool.clone()).await;
    let app = me_doc_backend::build_app(state);

    let outsider = register_and_login(&app, &uniq("wsoutsider"), PASSWORD).await;

    let other = insert_user(&pool, &uniq("wsother")).await;
    let ws = insert_workspace(&pool, other, "private-space").await;
    add_member(&pool, ws, other, "owner").await;

    let (status, _, _) = send(
        &app,
        authed("GET", &format!("/workspaces/{ws}/members"), &outsider.access_token),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}
