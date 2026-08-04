//! Integration tests for `POST /pages/import` (src/convert.rs).
//!
//! These are *not* mocked: the request really is proxied to the live
//! `converter` service (MarkItDown) at `converter:8000`, so a pass here means
//! the whole import path works end to end.

mod common;

use axum::body::Body;
use axum::http::header::{AUTHORIZATION, CONTENT_TYPE};
use axum::http::{Request, StatusCode};
use common::*;
use http_body_util::BodyExt;
use sqlx::PgPool;
use tower::ServiceExt;

const BOUNDARY: &str = "----medoctestboundary";

/// One `file` part, shaped the way a browser's `FormData` upload arrives.
fn multipart_body(filename: &str, part_content_type: &str, data: &[u8]) -> Vec<u8> {
    let mut body = Vec::with_capacity(data.len() + 256);
    body.extend_from_slice(
        format!(
            "--{BOUNDARY}\r\n\
             Content-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\n\
             Content-Type: {part_content_type}\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(data);
    body.extend_from_slice(format!("\r\n--{BOUNDARY}--\r\n").as_bytes());
    body
}

fn import_request(token: Option<&str>, filename: &str, ct: &str, data: &[u8]) -> Request<Body> {
    let mut req = Request::builder()
        .method("POST")
        .uri("/pages/import")
        .header(CONTENT_TYPE, format!("multipart/form-data; boundary={BOUNDARY}"));
    if let Some(t) = token {
        req = req.header(AUTHORIZATION, format!("Bearer {t}"));
    }
    req.body(Body::from(multipart_body(filename, ct, data)))
        .unwrap()
}

/// A valid access token for a directly-inserted user — the import endpoint only
/// checks the JWT, so the full register/login round trip would be dead weight.
async fn authed_user(pool: &PgPool, state: &me_doc_backend::AppState, email: &str) -> String {
    let id = insert_user(pool, email).await;
    me_doc_backend::auth::jwt::create_access_token(&state.config.jwt_access_secret, id, 600).unwrap()
}

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR")))
        .unwrap_or_else(|e| panic!("fixture {name} must exist: {e}"))
}

async fn import(app: &axum::Router, req: Request<Body>) -> (StatusCode, serde_json::Value) {
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null),
    )
}

#[sqlx::test]
async fn imports_a_real_docx(pool: PgPool) {
    let state = test_state(pool.clone()).await;
    let token = authed_user(&pool, &state, "docx@example.com").await;
    let app = me_doc_backend::build_app(state);

    let (status, body) = import(
        &app,
        import_request(
            Some(&token),
            "sample.docx",
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            &fixture("sample.docx"),
        ),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "docx import failed: {body}");
    let md = body["markdown"].as_str().expect("response must carry markdown");
    assert!(!md.trim().is_empty(), "converted markdown was empty");
}

#[sqlx::test]
async fn imports_a_real_markdown_file(pool: PgPool) {
    let state = test_state(pool.clone()).await;
    let token = authed_user(&pool, &state, "md@example.com").await;
    let app = me_doc_backend::build_app(state);

    let (status, body) = import(
        &app,
        import_request(Some(&token), "sample.md", "text/markdown", &fixture("sample.md")),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "md import failed: {body}");
    let md = body["markdown"].as_str().expect("response must carry markdown");
    assert!(
        md.contains("Sample Document"),
        "markdown should round-trip the fixture's heading: {md:?}"
    );
}

/// An oversized upload is rejected by the backend itself — the converter is
/// never dialed. Note it's axum's `DefaultBodyLimit` (2 MiB) that actually
/// trips first, not `convert::MAX_BYTES`; both surface as a 400.
#[sqlx::test]
async fn rejects_oversized_upload(pool: PgPool) {
    let state = test_state(pool.clone()).await;
    let token = authed_user(&pool, &state, "big@example.com").await;
    let app = me_doc_backend::build_app(state);

    let oversized = vec![b'a'; 20 * 1024 * 1024 + 1];
    let (status, body) = import(
        &app,
        import_request(Some(&token), "huge.md", "text/markdown", &oversized),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST, "oversized upload was accepted: {body}");
}

/// An extension the converter's allow-list doesn't cover: MarkItDown can't make
/// sense of the bytes, and that surfaces as a 400 rather than a 500.
#[sqlx::test]
async fn rejects_unsupported_file_type(pool: PgPool) {
    let state = test_state(pool.clone()).await;
    let token = authed_user(&pool, &state, "exe@example.com").await;
    let app = me_doc_backend::build_app(state);

    let (status, body) = import(
        &app,
        import_request(
            Some(&token),
            "payload.exe",
            "application/octet-stream",
            &[0x4d, 0x5a, 0x90, 0x00, 0xff, 0xfe, 0x00, 0x01, 0xde, 0xad, 0xbe, 0xef],
        ),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST, "unsupported file was accepted: {body}");
}

#[sqlx::test]
async fn rejects_unauthenticated_import(pool: PgPool) {
    let state = test_state(pool).await;
    let app = me_doc_backend::build_app(state);

    let (status, _) = import(
        &app,
        import_request(None, "sample.md", "text/markdown", &fixture("sample.md")),
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
