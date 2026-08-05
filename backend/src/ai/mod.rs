//! BYOK AI actions. Each user configures their own OpenAI-compatible endpoint,
//! key and model; the key is sealed with AES-256-GCM under `AI_ENCRYPTION_KEY`
//! and never leaves the backend — the frontend only ever learns whether one is
//! set.

use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{AeadCore, Aes256Gcm, Key, Nonce};
use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::auth::error::AuthError;
use crate::auth::extractor::AuthenticatedUser;
use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/ai/settings", get(get_settings).put(put_settings))
        .route("/ai/complete", post(complete))
}

fn cipher(state: &AppState) -> Aes256Gcm {
    // Config validated the length at boot, so this can't fail here.
    Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&state.config.ai_encryption_key))
}

#[derive(Serialize)]
struct SettingsResponse {
    api_url: String,
    model: String,
    has_key: bool,
}

async fn get_settings(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<SettingsResponse>, AuthError> {
    let row: Option<(String, String)> =
        sqlx::query_as("select api_url, model from user_ai_settings where user_id = $1")
            .bind(user.user_id)
            .fetch_optional(&state.db)
            .await?;

    Ok(Json(match row {
        Some((api_url, model)) => SettingsResponse {
            api_url,
            model,
            has_key: true,
        },
        None => SettingsResponse {
            api_url: String::new(),
            model: String::new(),
            has_key: false,
        },
    }))
}

#[derive(Deserialize)]
struct SettingsRequest {
    api_url: String,
    /// Absent (or blank) means "keep the key already stored" — the frontend
    /// never receives the current key, so it can't echo it back on save.
    api_key: Option<String>,
    model: String,
}

async fn put_settings(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<SettingsRequest>,
) -> Result<Json<SettingsResponse>, AuthError> {
    let api_url = body.api_url.trim().trim_end_matches('/').to_string();
    let model = body.model.trim().to_string();
    if api_url.is_empty() {
        return Err(AuthError::Validation("API URL is required".into()));
    }
    if !api_url.starts_with("http://") && !api_url.starts_with("https://") {
        return Err(AuthError::Validation(
            "API URL must start with http:// or https://".into(),
        ));
    }
    if model.is_empty() {
        return Err(AuthError::Validation("model is required".into()));
    }

    let new_key = body.api_key.filter(|k| !k.trim().is_empty());

    match new_key {
        Some(key) => {
            let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
            let sealed = cipher(&state)
                .encrypt(&nonce, key.trim().as_bytes())
                .map_err(|_| AuthError::Internal(anyhow::anyhow!("failed to encrypt api key")))?;
            sqlx::query(
                "insert into user_ai_settings (user_id, api_url, api_key_encrypted, api_key_nonce, model)
                 values ($1, $2, $3, $4, $5)
                 on conflict (user_id) do update set
                   api_url = excluded.api_url,
                   api_key_encrypted = excluded.api_key_encrypted,
                   api_key_nonce = excluded.api_key_nonce,
                   model = excluded.model,
                   updated_at = now()",
            )
            .bind(user.user_id)
            .bind(&api_url)
            .bind(&sealed)
            .bind(nonce.as_slice())
            .bind(&model)
            .execute(&state.db)
            .await?;
        }
        None => {
            let updated = sqlx::query(
                "update user_ai_settings set api_url = $2, model = $3, updated_at = now()
                 where user_id = $1",
            )
            .bind(user.user_id)
            .bind(&api_url)
            .bind(&model)
            .execute(&state.db)
            .await?;
            if updated.rows_affected() == 0 {
                return Err(AuthError::Validation("an API key is required".into()));
            }
        }
    }

    Ok(Json(SettingsResponse {
        api_url,
        model,
        has_key: true,
    }))
}

/// Frontends distinguish this exact message from a generic failure to show a
/// "configure your API key" prompt instead of an error.
const NOT_CONFIGURED: &str = "no AI settings configured";

fn system_prompt(instruction: &str) -> Option<&'static str> {
    Some(match instruction {
        "rephrase" => "Rephrase the following text, preserving its meaning and approximate length. Reply with only the rewritten text, no commentary.",
        "fix_grammar" => "Fix grammar and spelling in the following text. Reply with only the corrected text, no commentary.",
        "reformat" => "Reformat the following text for clarity (paragraphs/line breaks as needed) without changing its meaning or wording. Reply with only the reformatted text, no commentary.",
        "proofread" => "Proofread the following text and correct any errors — grammar, spelling, punctuation, clarity. Reply with only the corrected text, no commentary.",
        "explain" => "Explain the following text clearly and concisely for someone unfamiliar with it. Reply with only the explanation, no commentary.",
        // Free-form request ("chat"): the body.text is the user's own prompt,
        // optionally with their selected text appended by the frontend.
        "chat" => "You are a helpful writing assistant inside a Markdown document editor. Follow the user's request directly. Use Markdown formatting (headings, lists, code blocks) when it improves readability. Reply with the content only, no meta-commentary.",
        _ => return None,
    })
}

#[derive(Deserialize)]
struct CompleteRequest {
    instruction: String,
    text: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
    /// Present on most OpenAI-compatible providers; optional so a provider
    /// that omits it doesn't break the response.
    usage: Option<ChatUsage>,
}

#[derive(Deserialize)]
struct ChatUsage {
    prompt_tokens: i64,
    completion_tokens: i64,
    total_tokens: i64,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Deserialize)]
struct ChatMessage {
    content: String,
}

async fn complete(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<CompleteRequest>,
) -> Result<Json<serde_json::Value>, AuthError> {
    let prompt = system_prompt(&body.instruction)
        .ok_or_else(|| AuthError::Validation("unknown instruction".into()))?;
    if body.text.trim().is_empty() {
        return Err(AuthError::Validation("nothing to send".into()));
    }

    let row: Option<(String, Vec<u8>, Vec<u8>, String)> = sqlx::query_as(
        "select api_url, api_key_encrypted, api_key_nonce, model from user_ai_settings where user_id = $1",
    )
    .bind(user.user_id)
    .fetch_optional(&state.db)
    .await?;
    let (api_url, sealed, nonce, model) =
        row.ok_or_else(|| AuthError::Validation(NOT_CONFIGURED.into()))?;

    let key = cipher(&state)
        .decrypt(Nonce::from_slice(&nonce), sealed.as_slice())
        .map_err(|_| AuthError::Internal(anyhow::anyhow!("failed to decrypt api key")))?;
    let key = String::from_utf8(key).map_err(|e| AuthError::Internal(e.into()))?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| AuthError::Internal(e.into()))?;

    let resp = client
        .post(format!("{api_url}/chat/completions"))
        .bearer_auth(key)
        .json(&serde_json::json!({
            "model": model,
            "messages": [
                { "role": "system", "content": prompt },
                { "role": "user", "content": body.text },
            ],
        }))
        .send()
        .await
        .map_err(|_| AuthError::Validation("could not reach your AI provider".into()))?;

    if !resp.status().is_success() {
        let status = resp.status();
        return Err(AuthError::Validation(format!(
            "your AI provider rejected the request ({status})"
        )));
    }

    let parsed: ChatResponse = resp
        .json()
        .await
        .map_err(|_| AuthError::Validation("unexpected response from your AI provider".into()))?;
    let result = parsed
        .choices
        .into_iter()
        .next()
        .ok_or_else(|| AuthError::Validation("your AI provider returned no completion".into()))?
        .message
        .content;

    Ok(Json(serde_json::json!({
        "result": result.trim(),
        "usage": parsed.usage.map(|u| serde_json::json!({
            "prompt": u.prompt_tokens,
            "completion": u.completion_tokens,
            "total": u.total_tokens,
        })),
    })))
}
