//! HTTP-level integration tests for `sharing::router()` — sharing by email
//! (registered and not-yet-registered recipients), public share links, and the
//! permission-management gate.

mod common;

use std::time::Duration;

use axum::body::Body;
use axum::http::header::{AUTHORIZATION, CONTENT_TYPE};
use axum::http::{Request, StatusCode};
use axum::Router;
use common::*;
use me_doc_backend::build_app;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

const PASSWORD: &str = "Password123!";

/// Redis (OTP cooldown) and Mailpit are shared across runs and are not
/// reset by `#[sqlx::test]`, so every account needs a fresh address.
fn unique_email(prefix: &str) -> String {
    format!("{prefix}-{}@example.com", Uuid::new_v4().simple())
}

fn authed(method: &str, uri: &str, token: &str, body: Body) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(CONTENT_TYPE, "application/json")
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .body(body)
        .unwrap()
}

fn anon(method: &str, uri: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

fn json_body(v: Value) -> Body {
    Body::from(v.to_string())
}

/// Blocks until the share-notification email has actually landed in Mailpit,
/// so a later `mailpit_clear` can't leave it in flight and have it beat the
/// registration OTP to the top of the mailbox.
async fn wait_for_mail(to: &str) {
    for _ in 0..25 {
        let found: Value = reqwest::Client::new()
            .get("http://mailpit:8025/api/v1/search")
            .query(&[("query", format!("to:{to}"))])
            .send()
            .await
            .expect("mailpit must be reachable at mailpit:8025")
            .json()
            .await
            .unwrap();
        if found["messages"][0]["ID"].is_string() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    panic!("no share notification for {to} arrived in Mailpit within 5s");
}

struct Owner {
    app: Router,
    pool: PgPool,
    token: String,
    ws: Uuid,
    page: Uuid,
}

/// A registered owner with a workspace and one page in it.
async fn setup_owner(pool: PgPool, email: &str) -> Owner {
    let app = build_app(test_state(pool.clone()).await);
    let client = register_and_login(&app, email, PASSWORD).await;
    let user: Uuid = sqlx::query_scalar("select id from users where email = $1")
        .bind(email)
        .fetch_one(&pool)
        .await
        .unwrap();
    let ws = insert_workspace(&pool, user, "ws-owner").await;
    add_member(&pool, ws, user, "owner").await;
    let page = insert_page(&pool, ws, None, user, "shared-page").await;

    Owner { app, pool, token: client.access_token, ws, page }
}

impl Owner {
    /// Returns the created grant's id.
    async fn share(&self, email: &str, role: &str) -> (StatusCode, Value) {
        let (status, _, body) = send(
            &self.app,
            authed(
                "POST",
                &format!("/pages/{}/share", self.page),
                &self.token,
                json_body(json!({ "email": email, "role": role })),
            ),
        )
        .await;
        (status, body)
    }
}

#[sqlx::test]
async fn share_with_existing_user_grants_access(pool: PgPool) {
    let owner = setup_owner(pool, &unique_email("share-owner")).await;
    let guest_email = &unique_email("share-guest");
    let guest = register_and_login(&owner.app, guest_email, PASSWORD).await;

    // Before the share, the guest is a stranger to the page.
    let (status, _, _) = send(
        &owner.app,
        authed(
            "GET",
            &format!("/pages/{}", owner.page),
            &guest.access_token,
            Body::empty(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, body) = owner.share(guest_email, "viewer").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["role"], "viewer");
    assert_eq!(body["invited"], false);

    let (status, _, body) = send(
        &owner.app,
        authed(
            "GET",
            &format!("/pages/{}", owner.page),
            &guest.access_token,
            Body::empty(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["role"], "viewer");
}

#[sqlx::test]
async fn pending_share_resolves_when_the_invitee_registers(pool: PgPool) {
    let owner = setup_owner(pool, &unique_email("pending-owner")).await;
    let invitee_email = &unique_email("pending-invitee");

    let (status, body) = owner.share(invitee_email, "editor").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["invited"], true, "no account exists for this email yet");

    // The grant is parked on pending_email with no principal yet.
    let pending: (Option<Uuid>, Option<String>) =
        sqlx::query_as("select principal_id, pending_email from permissions where id = $1")
            .bind(Uuid::parse_str(body["id"].as_str().unwrap()).unwrap())
            .fetch_one(&owner.pool)
            .await
            .unwrap();
    assert_eq!(pending, (None, Some(invitee_email.to_string())));

    wait_for_mail(invitee_email).await;
    mailpit_clear(invitee_email).await;

    let invitee = register_and_login(&owner.app, invitee_email, PASSWORD).await;

    let (status, _, body) = send(
        &owner.app,
        authed(
            "GET",
            &format!("/pages/{}", owner.page),
            &invitee.access_token,
            Body::empty(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "pending grant did not resolve: {body}");
    assert_eq!(body["role"], "editor");
}

#[sqlx::test]
async fn share_link_grants_the_configured_role_anonymously(pool: PgPool) {
    let owner = setup_owner(pool, &unique_email("link-owner")).await;

    let (status, _, body) = send(
        &owner.app,
        authed(
            "POST",
            &format!("/pages/{}/share/link", owner.page),
            &owner.token,
            json_body(json!({ "role": "viewer" })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let token = body["link_token"].as_str().unwrap().to_string();
    assert_eq!(body["role"], "viewer");

    // No Authorization header at all — the link is the only credential.
    let (status, _, body) = send(
        &owner.app,
        anon("GET", &format!("/pages/{}?link={token}", owner.page)),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["role"], "viewer");

    // A viewer link must not confer write access.
    let (status, _, _) = send(
        &owner.app,
        Request::builder()
            .method("PATCH")
            .uri(format!("/pages/{}?link={token}", owner.page))
            .header(CONTENT_TYPE, "application/json")
            .body(json_body(json!({ "title": "hijacked" })))
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, _, _) = send(
        &owner.app,
        anon("GET", &format!("/pages/{}?link=not-a-real-token", owner.page)),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[sqlx::test]
async fn viewer_cannot_edit_or_delete_permissions(pool: PgPool) {
    let owner = setup_owner(pool, &unique_email("perm-owner")).await;
    let viewer_email = &unique_email("perm-viewer");
    let other_email = &unique_email("perm-other");
    let viewer = register_and_login(&owner.app, viewer_email, PASSWORD).await;
    register_and_login(&owner.app, other_email, PASSWORD).await;

    owner.share(viewer_email, "viewer").await;
    let (_, other_grant) = owner.share(other_email, "viewer").await;
    let other_id = other_grant["id"].as_str().unwrap();

    // Managing grants requires an Editor role on the page (`resolve_role`),
    // which a page-level viewer never has.
    let (status, _, _) = send(
        &owner.app,
        authed(
            "PATCH",
            &format!("/permissions/{other_id}"),
            &viewer.access_token,
            json_body(json!({ "role": "viewer" })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, _, _) = send(
        &owner.app,
        authed(
            "DELETE",
            &format!("/permissions/{other_id}"),
            &viewer.access_token,
            Body::empty(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Listing the page's grants is editor-only too.
    let (status, _, _) = send(
        &owner.app,
        authed(
            "GET",
            &format!("/pages/{}/permissions", owner.page),
            &viewer.access_token,
            Body::empty(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // The owner, who is an editor, can still do it.
    let (status, _, _) = send(
        &owner.app,
        authed(
            "DELETE",
            &format!("/permissions/{other_id}"),
            &owner.token,
            Body::empty(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

#[sqlx::test]
async fn viewer_cannot_escalate_own_role(pool: PgPool) {
    let owner = setup_owner(pool, &unique_email("escalate-owner")).await;
    let viewer_email = &unique_email("escalate-viewer");
    let viewer = register_and_login(&owner.app, viewer_email, PASSWORD).await;

    let (_, grant) = owner.share(viewer_email, "viewer").await;
    let grant_id = grant["id"].as_str().unwrap();

    let (status, _, _) = send(
        &owner.app,
        authed(
            "PATCH",
            &format!("/permissions/{grant_id}"),
            &viewer.access_token,
            json_body(json!({ "role": "editor" })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // ...and re-sharing the page with themselves as editor is refused too.
    let (status, _, _) = send(
        &owner.app,
        authed(
            "POST",
            &format!("/pages/{}/share", owner.page),
            &viewer.access_token,
            json_body(json!({ "email": viewer_email, "role": "editor" })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let role: String = sqlx::query_scalar("select role from permissions where id = $1")
        .bind(Uuid::parse_str(grant_id).unwrap())
        .fetch_one(&owner.pool)
        .await
        .unwrap();
    assert_eq!(role, "viewer");

    let (status, _, body) = send(
        &owner.app,
        authed(
            "GET",
            &format!("/pages/{}", owner.page),
            &viewer.access_token,
            Body::empty(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["role"], "viewer");
    // `ws` is only used to anchor the page; assert it stayed put.
    assert_eq!(body["workspace_id"], owner.ws.to_string());
}
