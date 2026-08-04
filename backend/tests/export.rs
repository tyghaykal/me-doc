//! Integration tests for `GET /pages/:id/export` (src/export/mod.rs) at the
//! HTTP layer: permission gating, format selection, and response headers.
//! The Yjs -> Markdown walk itself is unit-tested inside the module.

mod common;

use axum::body::Body;
use axum::http::header::{AUTHORIZATION, CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::http::{Request, StatusCode};
use common::*;
use http_body_util::BodyExt;
use me_doc_backend::AppState;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use yrs::{
    Doc, ReadTxn, StateVector, Transact, Xml, XmlElementPrelim, XmlFragment, XmlTextPrelim,
};

/// A page with a heading, a paragraph and a list, encoded the way the collab
/// server persists it.
fn sample_yjs_state() -> Vec<u8> {
    let doc = Doc::new();
    let frag = doc.get_or_insert_xml_fragment("default");
    {
        let mut txn = doc.transact_mut();

        let h = frag.push_back(&mut txn, XmlElementPrelim::empty("heading"));
        h.insert_attribute(&mut txn, "level", "1");
        h.push_back(&mut txn, XmlTextPrelim::new("Quarterly Report"));

        let p = frag.push_back(&mut txn, XmlElementPrelim::empty("paragraph"));
        p.push_back(&mut txn, XmlTextPrelim::new("Revenue grew this quarter."));

        let list = frag.push_back(&mut txn, XmlElementPrelim::empty("bulletList"));
        let li = list.push_back(&mut txn, XmlElementPrelim::empty("listItem"));
        let lip = li.push_back(&mut txn, XmlElementPrelim::empty("paragraph"));
        lip.push_back(&mut txn, XmlTextPrelim::new("north region up 12%"));
    }
    let update = doc
        .transact()
        .encode_state_as_update_v1(&StateVector::default());
    update
}

async fn set_content(pool: &PgPool, page_id: Uuid, state: &[u8]) {
    sqlx::query("insert into page_content (page_id, yjs_state) values ($1, $2)")
        .bind(page_id)
        .bind(state)
        .execute(pool)
        .await
        .unwrap();
}

fn token_for(state: &AppState, user_id: Uuid) -> String {
    me_doc_backend::auth::jwt::create_access_token(&state.config.jwt_access_secret, user_id, 600)
        .unwrap()
}

/// Export responses are binary and header-bearing, so `common::send` (which
/// only surfaces JSON + the refresh cookie) isn't enough here.
async fn export(
    app: &axum::Router,
    page_id: Uuid,
    format: Option<&str>,
    token: &str,
) -> (StatusCode, Option<String>, Option<String>, Vec<u8>) {
    let uri = match format {
        Some(f) => format!("/pages/{page_id}/export?format={f}"),
        None => format!("/pages/{page_id}/export"),
    };
    let req = Request::builder()
        .method("GET")
        .uri(uri)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    let (parts, body) = res.into_parts();
    let header = |name: axum::http::HeaderName| {
        parts
            .headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string)
    };
    let content_type = header(CONTENT_TYPE);
    let disposition = header(CONTENT_DISPOSITION);
    let bytes = body.collect().await.unwrap().to_bytes().to_vec();
    (parts.status, content_type, disposition, bytes)
}

/// Editor on a page with real content, plus its slug.
async fn editor_page(pool: &PgPool, state: &AppState) -> (Uuid, String, String) {
    let user = insert_user(pool, "editor@example.com").await;
    let ws = insert_workspace(pool, user, "ws").await;
    let page = insert_page(pool, ws, None, user, "quarterly-report").await;
    grant_page(pool, page, user, "editor").await;
    set_content(pool, page, &sample_yjs_state()).await;
    (page, "quarterly-report".to_string(), token_for(state, user))
}

#[sqlx::test]
async fn editor_exports_markdown(pool: PgPool) {
    let state = test_state(pool.clone()).await;
    let (page, slug, token) = editor_page(&pool, &state).await;
    let app = me_doc_backend::build_app(state);

    let (status, ct, disposition, bytes) = export(&app, page, Some("md"), &token).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(ct.as_deref(), Some("text/markdown; charset=utf-8"));
    assert_eq!(
        disposition.as_deref(),
        Some(format!("attachment; filename=\"{slug}.md\"").as_str())
    );
    let md = String::from_utf8(bytes).unwrap();
    assert!(md.contains("# Quarterly Report"), "heading missing: {md:?}");
    assert!(md.contains("- north region up 12%"), "list missing: {md:?}");
}

/// No `?format=` at all falls back to Markdown.
#[sqlx::test]
async fn default_format_is_markdown(pool: PgPool) {
    let state = test_state(pool.clone()).await;
    let (page, slug, token) = editor_page(&pool, &state).await;
    let app = me_doc_backend::build_app(state);

    let (status, ct, disposition, bytes) = export(&app, page, None, &token).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(ct.as_deref(), Some("text/markdown; charset=utf-8"));
    assert_eq!(
        disposition.as_deref(),
        Some(format!("attachment; filename=\"{slug}.md\"").as_str())
    );
    assert!(!bytes.is_empty());
}

#[sqlx::test]
async fn editor_exports_docx(pool: PgPool) {
    let state = test_state(pool.clone()).await;
    let (page, slug, token) = editor_page(&pool, &state).await;
    let app = me_doc_backend::build_app(state);

    let (status, ct, disposition, bytes) = export(&app, page, Some("docx"), &token).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        ct.as_deref(),
        Some("application/vnd.openxmlformats-officedocument.wordprocessingml.document")
    );
    assert_eq!(
        disposition.as_deref(),
        Some(format!("attachment; filename=\"{slug}.docx\"").as_str())
    );
    // A .docx is a zip; anything else means we shipped a broken download.
    assert_eq!(&bytes[..2], b"PK", "docx must be a zip archive");
}

#[sqlx::test]
async fn editor_exports_pdf(pool: PgPool) {
    let state = test_state(pool.clone()).await;
    let (page, slug, token) = editor_page(&pool, &state).await;
    let app = me_doc_backend::build_app(state);

    let (status, ct, disposition, bytes) = export(&app, page, Some("pdf"), &token).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(ct.as_deref(), Some("application/pdf"));
    assert_eq!(
        disposition.as_deref(),
        Some(format!("attachment; filename=\"{slug}.pdf\"").as_str())
    );
    assert_eq!(&bytes[..4], b"%PDF", "pdf must carry the PDF magic bytes");
}

/// The one explicitly flagged permission edge case: read access is not export
/// access. A Viewer on the very same page is refused.
#[sqlx::test]
async fn viewer_cannot_export(pool: PgPool) {
    let state = test_state(pool.clone()).await;
    let (page, _, _) = editor_page(&pool, &state).await;

    let viewer = insert_user(&pool, "viewer@example.com").await;
    grant_page(&pool, page, viewer, "viewer").await;
    let viewer_token = token_for(&state, viewer);
    let app = me_doc_backend::build_app(state);

    for format in ["md", "docx", "pdf"] {
        let (status, _, _, _) = export(&app, page, Some(format), &viewer_token).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "viewer exported {format}");
    }
}

#[sqlx::test]
async fn unsupported_format_is_rejected(pool: PgPool) {
    let state = test_state(pool.clone()).await;
    let (page, _, token) = editor_page(&pool, &state).await;
    let app = me_doc_backend::build_app(state);

    let (status, _, _, _) = export(&app, page, Some("epub"), &token).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn missing_page_is_not_found(pool: PgPool) {
    let state = test_state(pool.clone()).await;
    let user = insert_user(&pool, "nobody@example.com").await;
    let token = token_for(&state, user);
    let app = me_doc_backend::build_app(state);

    let (status, _, _, _) = export(&app, Uuid::new_v4(), Some("md"), &token).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// A page that was never opened in the editor has no `page_content` row at all
/// — that must still export (empty), not 500 on the left join's NULL.
#[sqlx::test]
async fn page_without_content_exports_empty(pool: PgPool) {
    let state = test_state(pool.clone()).await;
    let user = insert_user(&pool, "editor@example.com").await;
    let ws = insert_workspace(&pool, user, "ws").await;
    let page = insert_page(&pool, ws, None, user, "blank").await;
    grant_page(&pool, page, user, "editor").await;
    let token = token_for(&state, user);
    let app = me_doc_backend::build_app(state);

    let (status, _, disposition, bytes) = export(&app, page, Some("md"), &token).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        disposition.as_deref(),
        Some("attachment; filename=\"blank.md\"")
    );
    assert!(bytes.is_empty());
}
