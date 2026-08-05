use std::env;

#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub redis_url: String,
    pub backend_port: u16,
    pub s3_endpoint: String,
    /// Browser-reachable equivalent of `s3_endpoint`, used only when signing
    /// presigned URLs (the browser can't resolve the internal Docker hostname).
    pub s3_public_endpoint: String,
    pub s3_region: String,
    pub s3_access_key: String,
    pub s3_secret_key: String,
    pub s3_bucket: String,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_from: String,
    pub jwt_access_secret: String,
    pub jwt_refresh_secret: String,
    pub jwt_access_ttl_seconds: i64,
    pub jwt_refresh_ttl_seconds: i64,
    pub otp_ttl_seconds: i64,
    pub frontend_origin: String,
    /// Google OAuth client credentials — `None` (unset in `.env`) disables the
    /// Google sign-in button/redirects entirely.
    pub google_client_id: Option<String>,
    pub google_client_secret: Option<String>,
    pub google_redirect_uri: Option<String>,
    pub converter_url: String,
    /// White-label display name, used in emails and (via the frontend's own
    /// env var) page titles. Defaults to "MeDoc" — never hardcode the name
    /// elsewhere in the backend.
    pub product_name: String,
    /// Off by default: enabling this sends every exported diagram's full
    /// source text to the public mermaid.ink service to render a PNG. An
    /// operator must opt in explicitly — it's not implied by just running
    /// the app.
    pub export_diagram_render_enabled: bool,
    /// Raw 32 bytes (decoded from base64 `AI_ENCRYPTION_KEY`) used to seal each
    /// user's BYOK provider API key at rest. Rotating it makes every stored key
    /// undecryptable — users have to re-enter theirs.
    pub ai_encryption_key: Vec<u8>,
}

/// Reads a required secret from the environment, rejecting both "unset" and
/// "set to an empty string" (docker-compose substitutes the latter for an
/// undefined `.env` variable rather than leaving it unset).
fn non_empty_env(key: &str) -> anyhow::Result<String> {
    match env::var(key) {
        Ok(v) if !v.is_empty() => Ok(v),
        _ => anyhow::bail!("{key} must be set to a non-empty value"),
    }
}

/// Reads an optional value — `None` when unset or blank (docker-compose
/// substitutes an empty string for undefined `.env` vars).
fn optional_env(key: &str) -> Option<String> {
    match env::var(key) {
        Ok(v) if !v.is_empty() => Some(v),
        _ => None,
    }
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            database_url: env::var("DATABASE_URL")?,
            redis_url: env::var("REDIS_URL")?,
            backend_port: env::var("BACKEND_PORT")
                .unwrap_or_else(|_| "8080".into())
                .parse()?,
            s3_endpoint: env::var("S3_ENDPOINT").unwrap_or_default(),
            s3_public_endpoint: env::var("S3_PUBLIC_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:9010".into()),
            s3_region: env::var("S3_REGION").unwrap_or_else(|_| "us-east-1".into()),
            s3_access_key: env::var("S3_ACCESS_KEY").unwrap_or_default(),
            s3_secret_key: env::var("S3_SECRET_KEY").unwrap_or_default(),
            s3_bucket: env::var("S3_BUCKET").unwrap_or_else(|_| "medoc".into()),
            smtp_host: env::var("SMTP_HOST").unwrap_or_default(),
            smtp_port: env::var("SMTP_PORT")
                .unwrap_or_else(|_| "1025".into())
                .parse()?,
            smtp_from: env::var("SMTP_FROM").unwrap_or_else(|_| "no-reply@me-doc.local".into()),
            jwt_access_secret: non_empty_env("JWT_ACCESS_SECRET")?,
            jwt_refresh_secret: non_empty_env("JWT_REFRESH_SECRET")?,
            jwt_access_ttl_seconds: env::var("JWT_ACCESS_TTL_SECONDS")
                .unwrap_or_else(|_| "900".into())
                .parse()?,
            jwt_refresh_ttl_seconds: env::var("JWT_REFRESH_TTL_SECONDS")
                .unwrap_or_else(|_| "1209600".into())
                .parse()?,
            otp_ttl_seconds: env::var("OTP_TTL_SECONDS")
                .unwrap_or_else(|_| "600".into())
                .parse()?,
            frontend_origin: env::var("FRONTEND_ORIGIN")
                .unwrap_or_else(|_| "http://localhost:3000".into()),
            google_client_id: optional_env("GOOGLE_CLIENT_ID"),
            google_client_secret: optional_env("GOOGLE_CLIENT_SECRET"),
            google_redirect_uri: optional_env("GOOGLE_REDIRECT_URI"),
            converter_url: env::var("CONVERTER_URL")
                .unwrap_or_else(|_| "http://converter:8000".into()),
            product_name: env::var("PRODUCT_NAME").unwrap_or_else(|_| "MeDoc".into()),
            export_diagram_render_enabled: env::var("EXPORT_DIAGRAM_RENDER_ENABLED")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),
            ai_encryption_key: ai_encryption_key()?,
        })
    }
}

fn ai_encryption_key() -> anyhow::Result<Vec<u8>> {
    let raw = non_empty_env("AI_ENCRYPTION_KEY")?;
    let key = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, raw.trim())
        .map_err(|_| anyhow::anyhow!("AI_ENCRYPTION_KEY must be valid base64"))?;
    if key.len() != 32 {
        anyhow::bail!(
            "AI_ENCRYPTION_KEY must decode to exactly 32 bytes, got {}",
            key.len()
        );
    }
    Ok(key)
}
