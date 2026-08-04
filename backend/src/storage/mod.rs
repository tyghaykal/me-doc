use std::time::Duration;

use aws_sdk_s3::config::{Credentials, Region};
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::Client;

use crate::config::Config;

pub fn build_client(config: &Config) -> Client {
    build_client_for_endpoint(config, &config.s3_endpoint)
}

/// A second client, identical except for its endpoint, used only for
/// presigning: SigV4 signs the Host header, so a presigned URL must be built
/// against whichever host will actually receive the request (the browser),
/// not the internal Docker-network endpoint the server itself uses.
pub fn build_presign_client(config: &Config) -> Client {
    build_client_for_endpoint(config, &config.s3_public_endpoint)
}

fn build_client_for_endpoint(config: &Config, endpoint: &str) -> Client {
    let creds = Credentials::new(
        &config.s3_access_key,
        &config.s3_secret_key,
        None,
        None,
        "me-doc-static",
    );

    let s3_config = aws_sdk_s3::Config::builder()
        .behavior_version(aws_config::BehaviorVersion::latest())
        .region(Region::new(config.s3_region.clone()))
        .endpoint_url(endpoint)
        .credentials_provider(creds)
        .force_path_style(true)
        .build();

    Client::from_conf(s3_config)
}

/// Presigns a PUT for exactly `content_length` bytes — signing the length
/// forces the upload to match it, so callers must reject oversized requests
/// (checked against a per-endpoint cap) before calling this, not just note
/// the intended size.
pub async fn presign_upload_url(
    client: &Client,
    bucket: &str,
    key: &str,
    content_type: &str,
    content_length: i64,
) -> anyhow::Result<String> {
    let presign = PresigningConfig::expires_in(Duration::from_secs(15 * 60))?;

    let req = client
        .put_object()
        .bucket(bucket)
        .key(key)
        .content_type(content_type)
        .content_length(content_length)
        .presigned(presign)
        .await?;

    Ok(req.uri().to_string())
}

/// Presigns a short-lived GET — the bucket itself is private, so reads go
/// through an authenticated backend endpoint that checks the caller's
/// permission before redirecting here, instead of a permanent public URL.
pub async fn presign_download_url(client: &Client, bucket: &str, key: &str) -> anyhow::Result<String> {
    let presign = PresigningConfig::expires_in(Duration::from_secs(5 * 60))?;
    let req = client
        .get_object()
        .bucket(bucket)
        .key(key)
        .presigned(presign)
        .await?;
    Ok(req.uri().to_string())
}

/// Reduces a client-supplied filename to a safe basename before it's used in
/// an S3 key or a converter tempfile suffix — strips any path components (so
/// `../../etc/passwd` or `a/b` can't influence where a key/tempfile lands),
/// then keeps only a conservative character set. Never empty.
pub fn sanitize_filename(name: &str) -> String {
    let base = name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(name)
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>();
    let base = base.trim_matches('.');
    let base: String = base.chars().take(200).collect();
    if base.is_empty() {
        "upload".to_string()
    } else {
        base
    }
}

/// Content-types accepted by image-only upload endpoints (avatar, editor
/// image paste/drop) — deliberately excludes `image/svg+xml` (can carry
/// script) and anything not an image at all.
pub fn is_allowed_image_type(content_type: &str) -> bool {
    matches!(
        content_type,
        "image/png" | "image/jpeg" | "image/gif" | "image/webp"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_strips_path_traversal() {
        assert_eq!(sanitize_filename("../../etc/passwd"), "passwd");
        assert_eq!(sanitize_filename("a/b/c.png"), "c.png");
        assert_eq!(sanitize_filename("..\\..\\windows"), "windows");
    }

    #[test]
    fn sanitize_replaces_unsafe_chars() {
        assert_eq!(sanitize_filename("my file (1).png"), "my_file__1_.png");
    }

    #[test]
    fn sanitize_never_empty() {
        assert_eq!(sanitize_filename(""), "upload");
        assert_eq!(sanitize_filename("..."), "upload");
    }
}
