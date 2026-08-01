//! Proxies a single uploaded file to the `converter` service (MarkItDown) and
//! hands back Markdown. Deliberately stateless — no page/workspace touched
//! here; the frontend turns the Markdown into a page via the same
//! `markdownToHtml` + `setPendingImport` path the existing `.md`/`.txt`
//! import already uses (see `PageTree.vue`).

use axum::extract::{Multipart, State};
use axum::routing::post;
use axum::{Json, Router};
use serde::Deserialize;

use crate::auth::error::AuthError;
use crate::auth::extractor::AuthenticatedUser;
use crate::AppState;

/// Mirrors nginx's `client_max_body_size 20m` — nginx is the real gate;
/// this is defense-in-depth for anything reaching the backend directly.
const MAX_BYTES: usize = 20 * 1024 * 1024;

pub fn router() -> Router<AppState> {
    // Lives under the existing `/pages` prefix so nginx's location regex
    // (`^/(...|pages|...)(/|$)`) proxies it with no nginx config change.
    Router::new().route("/pages/import", post(import_convert))
}

#[derive(Deserialize)]
struct ConverterResponse {
    markdown: String,
}

async fn import_convert(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, AuthError> {
    let mut filename = "upload".to_string();
    let mut content_type = "application/octet-stream".to_string();
    let mut bytes: Option<Vec<u8>> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AuthError::Validation(format!("invalid upload: {e}")))?
    {
        if field.name() != Some("file") {
            continue;
        }
        filename = field.file_name().unwrap_or("upload").to_string();
        content_type = field
            .content_type()
            .unwrap_or("application/octet-stream")
            .to_string();
        let data = field
            .bytes()
            .await
            .map_err(|e| AuthError::Validation(format!("invalid upload: {e}")))?;
        if data.len() > MAX_BYTES {
            return Err(AuthError::Validation("file too large".into()));
        }
        bytes = Some(data.to_vec());
    }

    let bytes = bytes.ok_or_else(|| AuthError::Validation("no file provided".into()))?;
    if bytes.is_empty() {
        return Err(AuthError::Validation("empty file".into()));
    }

    let part = reqwest::multipart::Part::bytes(bytes)
        .file_name(filename)
        .mime_str(&content_type)
        .map_err(|e| AuthError::Internal(e.into()))?;
    let form = reqwest::multipart::Form::new().part("file", part);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| AuthError::Internal(e.into()))?;

    let resp = client
        .post(format!("{}/convert", state.config.converter_url))
        .multipart(form)
        .send()
        .await
        .map_err(|_| AuthError::Validation("conversion service unavailable".into()))?;

    if !resp.status().is_success() {
        return Err(AuthError::Validation(
            "couldn't convert this file — format may be unsupported or the file is corrupt".into(),
        ));
    }

    let converted: ConverterResponse = resp
        .json()
        .await
        .map_err(|e| AuthError::Internal(e.into()))?;

    Ok(Json(serde_json::json!({ "markdown": converted.markdown })))
}
