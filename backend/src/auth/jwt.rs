use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::error::AuthError;

#[derive(Debug, Serialize, Deserialize)]
pub struct AccessClaims {
    pub sub: Uuid,
    pub exp: i64,
    pub iat: i64,
}

pub fn create_access_token(secret: &str, user_id: Uuid, ttl_seconds: i64) -> anyhow::Result<String> {
    let now = Utc::now().timestamp();
    let claims = AccessClaims {
        sub: user_id,
        exp: now + ttl_seconds,
        iat: now,
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?;
    Ok(token)
}

pub fn verify_access_token(secret: &str, token: &str) -> Result<AccessClaims, AuthError> {
    let data = decode::<AccessClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| AuthError::InvalidToken)?;
    Ok(data.claims)
}
