//! HTTP-level integration tests for `pages::router()` — CRUD, trash/restore,
//! duplicate, the raw-Yjs content round trip, favorites and search scoping.

mod common;

use axum::body::Body;
use axum::http::header::{AUTHORIZATION, CONTENT_TYPE};
use axum::http::{Request, StatusCode};
use axum::Router;
use common::*;
use http_body_util::BodyExt;
use me_doc_backend::build_app;
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
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

fn json_body(v: Value) -> Body {
    Body::from(v.to_string())
}

/// `common::send` parses the body as JSON; page content is raw Yjs bytes.
async fn send_bytes(app: &Router, req: Request<Body>) -> (StatusCode, Vec<u8>) {
    let res = app.clone().oneshot(req).await.unwrap();
    let (parts, body) = res.into_parts();
    let bytes = body.collect().await.unwrap().to_bytes().to_vec();
    (parts.status, bytes)
}

struct Ctx {
    app: Router,
    pool: PgPool,
    token: String,
    user: Uuid,
    ws: Uuid,
}

async fn setup(pool: PgPool, email: &str) -> Ctx {
    let app = build_app(test_state(pool.clone()).await);
    let client = register_and_login(&app, email, PASSWORD).await;
    let user: Uuid = sqlx::query_scalar("select id from users where email = $1")
        .bind(email)
        .fetch_one(&pool)
        .await
        .unwrap();
    let ws = insert_workspace(&pool, user, "ws-primary").await;
    add_member(&pool, ws, user, "owner").await;

    Ctx { app, pool, token: client.access_token, user, ws }
}

impl Ctx {
    async fn create_page(&self, title: &str) -> Uuid {
        let (status, _, body) = send(
            &self.app,
            authed(
                "POST",
                &format!("/workspaces/{}/pages", self.ws),
                &self.token,
                json_body(json!({ "title": title })),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "create page failed: {body}");
        body["id"].as_str().unwrap().parse().unwrap()
    }
}

#[sqlx::test]
async fn create_read_update_delete_page(pool: PgPool) {
    let ctx = setup(pool, &unique_email("pages-crud")).await;
    let page = ctx.create_page("First page").await;

    let (status, _, body) = send(
        &ctx.app,
        authed("GET", &format!("/pages/{page}"), &ctx.token, Body::empty()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["title"], "First page");
    assert_eq!(body["kind"], "document");
    // Workspace membership resolves to an editor role.
    assert_eq!(body["role"], "editor");

    let (status, _, body) = send(
        &ctx.app,
        authed(
            "PATCH",
            &format!("/pages/{page}"),
            &ctx.token,
            json_body(json!({ "title": "Renamed", "icon": "*" })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["title"], "Renamed");
    assert_eq!(body["icon"], "*");

    let (status, _, _) = send(
        &ctx.app,
        authed("DELETE", &format!("/pages/{page}"), &ctx.token, Body::empty()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // delete is a soft archive, and `get_page` filters archived rows out.
    let (status, _, _) = send(
        &ctx.app,
        authed("GET", &format!("/pages/{page}"), &ctx.token, Body::empty()),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn trash_then_restore_page(pool: PgPool) {
    let ctx = setup(pool, &unique_email("pages-trash")).await;
    let page = ctx.create_page("Doomed").await;

    send(
        &ctx.app,
        authed("DELETE", &format!("/pages/{page}"), &ctx.token, Body::empty()),
    )
    .await;

    let (status, _, trash) = send(
        &ctx.app,
        authed(
            "GET",
            &format!("/workspaces/{}/pages/trash", ctx.ws),
            &ctx.token,
            Body::empty(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let ids: Vec<&str> = trash
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, vec![page.to_string()]);

    let (status, _, body) = send(
        &ctx.app,
        authed(
            "PATCH",
            &format!("/pages/{page}/restore"),
            &ctx.token,
            Body::empty(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(body["archived_at"].is_null());

    let (status, _, _) = send(
        &ctx.app,
        authed("GET", &format!("/pages/{page}"), &ctx.token, Body::empty()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (_, _, trash) = send(
        &ctx.app,
        authed(
            "GET",
            &format!("/workspaces/{}/pages/trash", ctx.ws),
            &ctx.token,
            Body::empty(),
        ),
    )
    .await;
    assert!(trash.as_array().unwrap().is_empty());
}

#[sqlx::test]
async fn duplicate_page_copies_title_and_content(pool: PgPool) {
    let ctx = setup(pool, &unique_email("pages-duplicate")).await;
    let page = ctx.create_page("Original").await;

    let content = b"duplicate-me".to_vec();
    let (status, _) = send_bytes(
        &ctx.app,
        authed(
            "PUT",
            &format!("/pages/{page}/content"),
            &ctx.token,
            Body::from(content.clone()),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _, body) = send(
        &ctx.app,
        authed(
            "POST",
            &format!("/pages/{page}/duplicate"),
            &ctx.token,
            Body::empty(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["title"], "Original (copy)");
    let copy = body["id"].as_str().unwrap();
    assert_ne!(copy, page.to_string());

    let (status, copied) = send_bytes(
        &ctx.app,
        authed("GET", &format!("/pages/{copy}/content"), &ctx.token, Body::empty()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(copied, content);
}

#[sqlx::test]
async fn page_content_round_trips_raw_bytes(pool: PgPool) {
    let ctx = setup(pool, &unique_email("pages-content")).await;
    let page = ctx.create_page("Binary").await;

    // Not a decodable Yjs update — the handler derives plain_text best-effort
    // and must still persist the bytes verbatim.
    let content: Vec<u8> = (0u8..=255).collect();

    let (status, _) = send_bytes(
        &ctx.app,
        authed(
            "PUT",
            &format!("/pages/{page}/content"),
            &ctx.token,
            Body::from(content.clone()),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, fetched) = send_bytes(
        &ctx.app,
        authed("GET", &format!("/pages/{page}/content"), &ctx.token, Body::empty()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(fetched, content);
}

#[sqlx::test]
async fn favorite_and_unfavorite_page(pool: PgPool) {
    let ctx = setup(pool, &unique_email("pages-favorite")).await;
    let page = ctx.create_page("Favorite me").await;

    let (status, _, _) = send(
        &ctx.app,
        authed(
            "POST",
            &format!("/pages/{page}/favorite"),
            &ctx.token,
            Body::empty(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _, favorites) = send(
        &ctx.app,
        authed("GET", "/me/favorite-pages", &ctx.token, Body::empty()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let ids: Vec<&str> = favorites
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, vec![page.to_string()]);

    let (status, _, _) = send(
        &ctx.app,
        authed(
            "DELETE",
            &format!("/pages/{page}/favorite"),
            &ctx.token,
            Body::empty(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (_, _, favorites) = send(
        &ctx.app,
        authed("GET", "/me/favorite-pages", &ctx.token, Body::empty()),
    )
    .await;
    assert!(favorites.as_array().unwrap().is_empty());
}

#[sqlx::test]
async fn search_is_scoped_to_workspace(pool: PgPool) {
    let ctx = setup(pool, &unique_email("pages-search")).await;
    let here = ctx.create_page("Quarterly zebra report").await;

    // Second workspace the same user belongs to — its pages must never leak
    // into the first workspace's search results.
    let other_ws = insert_workspace(&ctx.pool, ctx.user, "ws-other").await;
    add_member(&ctx.pool, other_ws, ctx.user, "owner").await;
    let there = insert_page(&ctx.pool, other_ws, None, ctx.user, "other-zebra").await;
    sqlx::query("update pages set title = 'Other zebra notes' where id = $1")
        .bind(there)
        .execute(&ctx.pool)
        .await
        .unwrap();

    let (status, _, results) = send(
        &ctx.app,
        authed(
            "GET",
            &format!("/workspaces/{}/search?q=zebra", ctx.ws),
            &ctx.token,
            Body::empty(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let ids: Vec<&str> = results
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, vec![here.to_string()]);

    // ...and the other workspace only sees its own.
    let (_, _, results) = send(
        &ctx.app,
        authed(
            "GET",
            &format!("/workspaces/{other_ws}/search?q=zebra"),
            &ctx.token,
            Body::empty(),
        ),
    )
    .await;
    let ids: Vec<&str> = results
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, vec![there.to_string()]);
}
