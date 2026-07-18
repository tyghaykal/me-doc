use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("invalid email or password")]
    InvalidCredentials,
    #[error("email already registered")]
    EmailTaken,
    #[error("email not verified")]
    EmailNotVerified,
    #[error("invalid or expired code")]
    InvalidOtp,
    #[error("too many attempts, request a new code")]
    OtpLocked,
    #[error("please wait before requesting another code")]
    OtpCooldown,
    #[error("invalid or expired session")]
    InvalidToken,
    #[error("not found")]
    NotFound,
    #[error("access denied")]
    Forbidden,
    #[error("{0}")]
    Validation(String),
    #[error("internal error")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for AuthError {
    fn into_response(self) -> axum::response::Response {
        let status = match &self {
            AuthError::InvalidCredentials
            | AuthError::InvalidOtp
            | AuthError::InvalidToken => StatusCode::UNAUTHORIZED,
            AuthError::EmailTaken => StatusCode::CONFLICT,
            AuthError::EmailNotVerified | AuthError::Forbidden => StatusCode::FORBIDDEN,
            AuthError::NotFound => StatusCode::NOT_FOUND,
            AuthError::OtpLocked | AuthError::OtpCooldown => StatusCode::TOO_MANY_REQUESTS,
            AuthError::Validation(_) => StatusCode::BAD_REQUEST,
            AuthError::Internal(err) => {
                tracing::error!(?err, "internal auth error");
                StatusCode::INTERNAL_SERVER_ERROR
            }
        };
        let message = self.to_string();
        (status, Json(json!({ "message": message }))).into_response()
    }
}

impl From<sqlx::Error> for AuthError {
    fn from(err: sqlx::Error) -> Self {
        AuthError::Internal(err.into())
    }
}

impl From<redis::RedisError> for AuthError {
    fn from(err: redis::RedisError) -> Self {
        AuthError::Internal(err.into())
    }
}
