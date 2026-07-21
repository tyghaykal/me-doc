pub mod error;
pub mod extractor;
pub mod jwt;
pub mod otp;
pub mod password;
pub mod tokens;
pub mod util;

use axum::{extract::State, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower_cookies::{cookie::time::Duration as CookieDuration, cookie::SameSite, Cookie, Cookies};
use uuid::Uuid;

use crate::{workspaces, AppState};
use error::AuthError;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/refresh", post(refresh))
        .route("/logout", post(logout))
}

/// Credential/OTP-guessable endpoints — kept on a separate, much stricter
/// rate-limit bucket in main.rs than the rest of the API (see the comment
/// there). `refresh`/`logout` don't belong here: both require an
/// already-valid session (cookie/token), so they aren't brute-forceable the
/// same way.
pub fn sensitive_router() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route("/register/verify", post(register_verify))
        .route("/login", post(login))
        .route("/login/verify", post(login_verify))
}

#[derive(Deserialize)]
struct RegisterRequest {
    email: String,
    password: String,
}

#[derive(Deserialize)]
struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Deserialize)]
struct OtpVerifyRequest {
    email: String,
    code: String,
}

#[derive(Serialize)]
struct UserResponse {
    id: Uuid,
    email: String,
}

#[derive(Serialize)]
struct AuthResponse {
    access_token: String,
    user: UserResponse,
    workspace: Option<workspaces::Workspace>,
}

fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> Result<Json<Value>, AuthError> {
    if body.password.len() < 8 {
        return Err(AuthError::Validation(
            "password must be at least 8 characters".into(),
        ));
    }
    let email = normalize_email(&body.email);
    if !email.contains('@') || email.len() < 3 {
        return Err(AuthError::Validation("invalid email address".into()));
    }

    let existing: Option<(Uuid,)> = sqlx::query_as("select id from users where email = $1")
        .bind(&email)
        .fetch_optional(&state.db)
        .await?;
    if existing.is_some() {
        return Err(AuthError::EmailTaken);
    }

    let hash = password::hash_password(&body.password)?;

    let (user_id,): (Uuid,) =
        sqlx::query_as("insert into users (email, password_hash) values ($1, $2) returning id")
            .bind(&email)
            .bind(&hash)
            .fetch_one(&state.db)
            .await?;

    // Resolve any pages shared with this email before they had an account —
    // see sharing::share_with_user's pending_email path.
    sqlx::query(
        "update permissions set principal_id = $1, pending_email = null
         where principal_type = 'user' and principal_id is null and pending_email = $2",
    )
    .bind(user_id)
    .bind(&email)
    .execute(&state.db)
    .await?;

    let code = otp::issue_otp(&state.redis, "register", &email, state.config.otp_ttl_seconds).await?;
    state.email.send_otp(&email, "register", &code).await?;

    Ok(Json(json!({ "message": "verification code sent" })))
}

async fn register_verify(
    State(state): State<AppState>,
    cookies: Cookies,
    Json(body): Json<OtpVerifyRequest>,
) -> Result<Json<AuthResponse>, AuthError> {
    let email = normalize_email(&body.email);
    otp::verify_otp(&state.redis, "register", &email, &body.code).await?;

    let (user_id,): (Uuid,) =
        sqlx::query_as("update users set email_verified_at = now() where email = $1 returning id")
            .bind(&email)
            .fetch_one(&state.db)
            .await?;

    let workspace = workspaces::create_default_workspace(&state.db, user_id).await?;
    let access_token = issue_session(&state, &cookies, user_id).await?;

    Ok(Json(AuthResponse {
        access_token,
        user: UserResponse { id: user_id, email },
        workspace: Some(workspace),
    }))
}

async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<Value>, AuthError> {
    let email = normalize_email(&body.email);

    let row: Option<(Uuid, String, Option<chrono::DateTime<chrono::Utc>>)> = sqlx::query_as(
        "select id, password_hash, email_verified_at from users where email = $1",
    )
    .bind(&email)
    .fetch_optional(&state.db)
    .await?;

    let Some((_id, hash, verified_at)) = row else {
        return Err(AuthError::InvalidCredentials);
    };
    if !password::verify_password(&body.password, &hash) {
        return Err(AuthError::InvalidCredentials);
    }
    if verified_at.is_none() {
        return Err(AuthError::EmailNotVerified);
    }

    let code = otp::issue_otp(&state.redis, "login", &email, state.config.otp_ttl_seconds).await?;
    state.email.send_otp(&email, "login", &code).await?;

    Ok(Json(json!({ "message": "verification code sent" })))
}

async fn login_verify(
    State(state): State<AppState>,
    cookies: Cookies,
    Json(body): Json<OtpVerifyRequest>,
) -> Result<Json<AuthResponse>, AuthError> {
    let email = normalize_email(&body.email);
    otp::verify_otp(&state.redis, "login", &email, &body.code).await?;

    let (user_id,): (Uuid,) = sqlx::query_as("select id from users where email = $1")
        .bind(&email)
        .fetch_one(&state.db)
        .await?;

    let workspace = fetch_first_workspace(&state.db, user_id).await?;
    let access_token = issue_session(&state, &cookies, user_id).await?;

    Ok(Json(AuthResponse {
        access_token,
        user: UserResponse { id: user_id, email },
        workspace,
    }))
}

async fn refresh(State(state): State<AppState>, cookies: Cookies) -> Result<Json<AuthResponse>, AuthError> {
    let token = cookies
        .get("refresh_token")
        .map(|c| c.value().to_string())
        .ok_or(AuthError::InvalidToken)?;

    let user_id = tokens::consume_refresh_token(&state.db, &token).await?;
    let access_token = issue_session(&state, &cookies, user_id).await?;

    let (email,): (String,) = sqlx::query_as("select email from users where id = $1")
        .bind(user_id)
        .fetch_one(&state.db)
        .await?;
    let workspace = fetch_first_workspace(&state.db, user_id).await?;

    Ok(Json(AuthResponse {
        access_token,
        user: UserResponse { id: user_id, email },
        workspace,
    }))
}

async fn logout(State(state): State<AppState>, cookies: Cookies) -> Result<Json<Value>, AuthError> {
    if let Some(cookie) = cookies.get("refresh_token") {
        tokens::revoke_refresh_token(&state.db, cookie.value()).await?;
    }
    let mut removal = Cookie::new("refresh_token", "");
    removal.set_path("/");
    cookies.remove(removal);

    Ok(Json(json!({ "message": "logged out" })))
}

async fn fetch_first_workspace(
    db: &PgPool,
    user_id: Uuid,
) -> Result<Option<workspaces::Workspace>, AuthError> {
    let row: Option<(Uuid, String, String)> = sqlx::query_as(
        "select w.id, w.name, w.slug from workspaces w
         join workspace_members m on m.workspace_id = w.id
         where m.user_id = $1
         order by w.created_at asc
         limit 1",
    )
    .bind(user_id)
    .fetch_optional(db)
    .await?;

    Ok(row.map(|(id, name, slug)| workspaces::Workspace { id, name, slug }))
}

/// Issues an access token and sets a fresh httpOnly refresh-token cookie, returning the access token.
async fn issue_session(state: &AppState, cookies: &Cookies, user_id: Uuid) -> Result<String, AuthError> {
    let access_token = jwt::create_access_token(
        &state.config.jwt_access_secret,
        user_id,
        state.config.jwt_access_ttl_seconds,
    )?;

    let refresh = tokens::issue_refresh_token(
        &state.db,
        user_id,
        state.config.jwt_refresh_ttl_seconds,
        None,
        None,
    )
    .await?;

    let mut cookie = Cookie::new("refresh_token", refresh.token);
    cookie.set_http_only(true);
    cookie.set_same_site(SameSite::Lax);
    cookie.set_path("/");
    cookie.set_max_age(CookieDuration::seconds(state.config.jwt_refresh_ttl_seconds));
    cookies.add(cookie);

    Ok(access_token)
}
