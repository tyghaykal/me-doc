use axum::{async_trait, extract::FromRequestParts, http::request::Parts};
use uuid::Uuid;

use super::{error::AuthError, jwt};
use crate::AppState;

pub struct AuthenticatedUser {
    pub user_id: Uuid,
}

#[async_trait]
impl FromRequestParts<AppState> for AuthenticatedUser {
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .ok_or(AuthError::InvalidToken)?;

        let claims = jwt::verify_access_token(&state.config.jwt_access_secret, token)?;
        Ok(AuthenticatedUser {
            user_id: claims.sub,
        })
    }
}
