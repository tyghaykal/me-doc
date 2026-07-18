use chrono::{Duration, Utc};
use rand::RngCore;
use sqlx::PgPool;
use uuid::Uuid;

use super::error::AuthError;
use super::util::sha256_b64;

pub struct IssuedRefreshToken {
    pub token: String,
    pub expires_at: chrono::DateTime<Utc>,
}

fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, bytes)
}

pub async fn issue_refresh_token(
    db: &PgPool,
    user_id: Uuid,
    ttl_seconds: i64,
    user_agent: Option<&str>,
    ip: Option<&str>,
) -> Result<IssuedRefreshToken, AuthError> {
    let token = generate_token();
    let hash = sha256_b64(&token);
    let expires_at = Utc::now() + Duration::seconds(ttl_seconds);

    sqlx::query(
        "insert into refresh_tokens (user_id, token_hash, user_agent, ip, expires_at) values ($1, $2, $3, $4, $5)",
    )
    .bind(user_id)
    .bind(&hash)
    .bind(user_agent)
    .bind(ip)
    .bind(expires_at)
    .execute(db)
    .await?;

    Ok(IssuedRefreshToken { token, expires_at })
}

/// Validates a refresh token and revokes it (rotation is the caller's job: issue a new one after this succeeds).
pub async fn consume_refresh_token(db: &PgPool, token: &str) -> Result<Uuid, AuthError> {
    let hash = sha256_b64(token);

    let row: Option<(Uuid, Uuid)> = sqlx::query_as(
        "select id, user_id from refresh_tokens
         where token_hash = $1 and revoked_at is null and expires_at > now()",
    )
    .bind(&hash)
    .fetch_optional(db)
    .await?;

    let Some((id, user_id)) = row else {
        return Err(AuthError::InvalidToken);
    };

    sqlx::query("update refresh_tokens set revoked_at = now() where id = $1")
        .bind(id)
        .execute(db)
        .await?;

    Ok(user_id)
}

pub async fn revoke_refresh_token(db: &PgPool, token: &str) -> Result<(), AuthError> {
    let hash = sha256_b64(token);
    sqlx::query("update refresh_tokens set revoked_at = now() where token_hash = $1 and revoked_at is null")
        .bind(&hash)
        .execute(db)
        .await?;
    Ok(())
}
