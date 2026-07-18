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
            jwt_access_secret: env::var("JWT_ACCESS_SECRET").unwrap_or_else(|_| "dev-secret".into()),
            jwt_refresh_secret: env::var("JWT_REFRESH_SECRET").unwrap_or_else(|_| "dev-secret".into()),
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
        })
    }
}
