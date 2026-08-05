//! Server-side Google OAuth 2.0 authorization-code flow.
//!
//! No Google Sign-In JS SDK: the frontend redirects to `/auth/google/login`,
//! which bounces to Google, and Google redirects back to `/auth/google/callback`.
//! The callback exchanges the code, upserts the user by `google_sub`, issues
//! the same refresh-token session the OTP flow uses, and redirects the browser
//! to the frontend's `/oauth/google/callback` page with the cookie set.
//!
//! The flow is GET-based (OAuth redirects can't carry POST bodies), so these
//! routes live in `auth::router()` — the non-sensitive router — not
//! `sensitive_router()`, whose POST-only strict rate-limit bucket exists to
//! slow credential/OTP guessing. A state nonce + PKCE verifier (stored in
//! Redis, TTL-bounded) keeps the callback CSRF-safe instead.

use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect},
};
use rand::RngCore;
use redis::AsyncCommands;
use serde::Deserialize;
use tower_cookies::Cookies;
use uuid::Uuid;

use super::error::AuthError;
use super::util::sha256_b64;
use crate::{auth, workspaces, AppState};

const STATE_TTL_SECONDS: i64 = 600;

fn state_key(state: &str) -> String {
    format!("oauth:state:{state}")
}

/// Random URL-safe token used as the OAuth `state` (CSRF protection).
fn generate_state() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, bytes)
}

/// Persists the PKCE code verifier keyed by the state token (hashed, so a
/// Redis dump leaks nothing usable).
async fn store_verifier(redis: &redis::Client, state: &str, verifier: &str) -> Result<(), AuthError> {
    let mut conn = redis.get_multiplexed_async_connection().await?;
    let () = conn
        .set_ex(state_key(&sha256_b64(state)), verifier, STATE_TTL_SECONDS as u64)
        .await?;
    Ok(())
}

/// Recovers and consumes the PKCE verifier for a callback's state.
async fn take_verifier(redis: &redis::Client, state: &str) -> Result<String, AuthError> {
    let mut conn = redis.get_multiplexed_async_connection().await?;
    let verifier: Option<String> = conn.get_del(state_key(&sha256_b64(state))).await?;
    verifier.ok_or(AuthError::InvalidToken)
}

fn google_config<'a>(
    state: &'a AppState,
) -> Result<(&'a str, &'a str, &'a str), AuthError> {
    let (Some(client_id), Some(client_secret), Some(redirect_uri)) = (
        state.config.google_client_id.as_deref(),
        state.config.google_client_secret.as_deref(),
        state.config.google_redirect_uri.as_deref(),
    ) else {
        tracing::warn!("google oauth requested but GOOGLE_CLIENT_ID/SECRET/REDIRECT_URI not configured");
        return Err(AuthError::OAuthNotConfigured);
    };
    Ok((client_id, client_secret, redirect_uri))
}

fn authorize_url(client_id: &str, redirect_uri: &str, state: &str, verifier: &str) -> String {
    let challenge = crate::auth::util::pkce_challenge(verifier);
    let mut url = reqwest::Url::parse("https://accounts.google.com/o/oauth2/v2/auth").unwrap();
    {
        let mut q = url.query_pairs_mut();
        q.append_pair("client_id", client_id);
        q.append_pair("redirect_uri", redirect_uri);
        q.append_pair("response_type", "code");
        q.append_pair("scope", "openid email profile");
        q.append_pair("state", state);
        q.append_pair("code_challenge", &challenge);
        q.append_pair("code_challenge_method", "S256");
        q.append_pair("access_type", "online");
    }
    url.to_string()
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    code: String,
    state: String,
}

/// GET /auth/google/login — starts the flow by redirecting the browser to Google.
pub async fn login(State(state): State<AppState>) -> Result<Redirect, AuthError> {
    let (client_id, _, redirect_uri) = google_config(&state)?;

    let state_token = generate_state();
    let verifier = generate_state();
    store_verifier(&state.redis, &state_token, &verifier).await?;

    Ok(Redirect::temporary(&authorize_url(
        client_id,
        redirect_uri,
        &state_token,
        &verifier,
    )))
}

/// GET /auth/google/callback — Google redirects the browser here after the user
/// consents. Exchanges the code, upserts the user, issues a session, and sends
/// the browser to the frontend's callback page.
pub async fn callback(
    State(state): State<AppState>,
    cookies: Cookies,
    Query(q): Query<CallbackQuery>,
) -> Result<impl IntoResponse, AuthError> {
    let (_, client_secret, redirect_uri) = google_config(&state)?;

    let verifier = take_verifier(&state.redis, &q.state).await?;
    let userinfo = exchange_code(&state, client_secret, redirect_uri, &q.code, &verifier).await?;

    let email = userinfo.email.ok_or(AuthError::Validation(
        "Google account has no email address".into(),
    ))?;
    let email = email.trim().to_lowercase();

    let (user_id, first_google_login, is_new_user) =
        upsert_google_user(&state, &userinfo.sub, &email, userinfo.name.as_deref()).await?;

    // Welcome email only on the FIRST Google login — a returning Google user
    // (google_sub already set before this callback) shouldn't be re-greeted on
    // every sign-in. Failures are logged but not fatal: the auth session must
    // succeed even if the welcome email bounces.
    if first_google_login {
        if let Err(err) = state.email.send_welcome(&email, is_new_user).await {
            tracing::warn!(?err, email = %email, "failed to send welcome email");
        }
    }

    // Reconcile pages shared to this email before the account existed.
    sqlx::query(
        "update permissions set principal_id = $1, pending_email = null
         where principal_type = 'user' and principal_id is null and pending_email = $2",
    )
    .bind(user_id)
    .bind(&email)
    .execute(&state.db)
    .await?;

    // Reuse an existing workspace when the account already has one (e.g. a
    // Google-linked account signing in again), else create the default.
    if auth::fetch_first_workspace(&state.db, user_id)
        .await?
        .is_none()
    {
        workspaces::create_default_workspace(&state.db, user_id).await?;
    }

    let _ = auth::issue_session(&state, &cookies, user_id).await?;

    // Point the browser at the frontend callback page; the refresh_token cookie
    // rides along automatically (same-site). That page calls /auth/refresh with
    // the cookie to grab the access token and finish.
    //
    // NOTE: this must NOT live under /login (e.g. /login/google): Nuxt 4 treats
    // pages/login/google.vue as a child of pages/login.vue, which has no
    // <NuxtPage> outlet, so the route silently renders login.vue instead.
    // /oauth/google/callback is a top-level route that renders standalone.
    let origin = state.config.frontend_origin.trim_end_matches('/');
    let redirect = format!("{origin}/oauth/google/callback");
    Ok(Redirect::temporary(&redirect))
}

struct GoogleUserInfo {
    sub: String,
    email: Option<String>,
    name: Option<String>,
}

/// Exchanges the authorization code for tokens and fetches the user's profile.
async fn exchange_code(
    state: &AppState,
    client_secret: &str,
    redirect_uri: &str,
    code: &str,
    verifier: &str,
) -> Result<GoogleUserInfo, AuthError> {
    let http = reqwest::Client::new();

    let token_resp = http
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("code", code),
            ("client_id", state.config.google_client_id.as_deref().unwrap_or_default()),
            ("client_secret", client_secret),
            ("redirect_uri", redirect_uri),
            ("grant_type", "authorization_code"),
            ("code_verifier", verifier),
        ])
        .send()
        .await
        .map_err(AuthError::Google)?;

    if !token_resp.status().is_success() {
        let status = token_resp.status();
        let text = token_resp.text().await.unwrap_or_default();
        tracing::warn!(status = %status, body = %text, "google token exchange failed");
        return Err(AuthError::GoogleExchangeFailed(status.as_u16()));
    }

    let tokens: serde_json::Value = token_resp.json().await.map_err(AuthError::Google)?;
    let access_token = tokens["access_token"].as_str().ok_or_else(|| {
        AuthError::Validation("google token exchange returned no access token".into())
    })?;

    let info = http
        .get("https://www.googleapis.com/oauth2/v3/userinfo")
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(AuthError::Google)?;
    if !info.status().is_success() {
        return Err(AuthError::GoogleExchangeFailed(info.status().as_u16()));
    }
    let v: serde_json::Value = info.json().await.map_err(AuthError::Google)?;
    let sub = v["sub"]
        .as_str()
        .ok_or_else(|| AuthError::Validation("google userinfo missing sub".into()))?;

    Ok(GoogleUserInfo {
        sub: sub.to_string(),
        email: v["email"].as_str().map(|s| s.to_string()),
        name: v["name"].as_str().map(|s| s.to_string()),
    })
}

/// Inserts the Google user if `google_sub` is new, or re-links an existing
/// account (matched by verified email) to Google.
///
/// Returns `(user_id, first_google_login, is_new_user)`:
/// - `first_google_login` is true only when the user did NOT already have
///   `google_sub` before this callback — the single moment the welcome email
///   should fire. A returning Google user (google_sub already set) gets false.
/// - `is_new_user` is true only when a brand-new account row was created —
///   drives the "Welcome to" vs "Welcome back" copy.
async fn upsert_google_user(
    state: &AppState,
    sub: &str,
    email: &str,
    name: Option<&str>,
) -> Result<(Uuid, bool, bool), AuthError> {
    // Match by google_sub first (stable, the user's identity on Google).
    if let Some((id,)) = sqlx::query_as::<_, (Uuid,)>("select id from users where google_sub = $1")
        .bind(sub)
        .fetch_optional(&state.db)
        .await?
    {
        if let Some(name) = name {
            sqlx::query("update users set display_name = $2 where id = $1")
                .bind(id)
                .bind(name)
                .execute(&state.db)
                .await?;
        }
        // Already had Google linked — this is a repeat login, not a first one.
        return Ok((id, false, false));
    }

    // Existing verified account with this email — link it to Google.
    if let Some((id,)) = sqlx::query_as::<_, (Uuid,)>(
        "select id from users where email = $1 and email_verified_at is not null",
    )
    .bind(email)
    .fetch_optional(&state.db)
    .await?
    {
        sqlx::query(
            "update users set google_sub = $2, display_name = coalesce(display_name, $3) where id = $1",
        )
        .bind(id)
        .bind(sub)
        .bind(name)
        .execute(&state.db)
        .await?;
        // First time this account used Google.
        return Ok((id, true, false));
    }

    // Brand-new account (or an unverified account with this email — adopt it:
    // Google proving ownership is as good as the email OTP).
    let (id,): (Uuid,) = sqlx::query_as(
        "insert into users (email, google_sub, email_verified_at, display_name, password_hash)
         values ($1, $2, now(), $3, null)
         on conflict (email) do update
           set google_sub = excluded.google_sub,
               email_verified_at = now(),
               display_name = coalesce(users.display_name, excluded.display_name)
         returning id",
    )
    .bind(email)
    .bind(sub)
    .bind(name)
    .fetch_one(&state.db)
    .await?;

    // Brand-new Google account.
    Ok((id, true, true))
}
