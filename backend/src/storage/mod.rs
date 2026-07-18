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

pub async fn presign_upload_url(
    client: &Client,
    bucket: &str,
    key: &str,
    content_type: &str,
) -> anyhow::Result<String> {
    let presign = PresigningConfig::expires_in(Duration::from_secs(15 * 60))?;

    let req = client
        .put_object()
        .bucket(bucket)
        .key(key)
        .content_type(content_type)
        .presigned(presign)
        .await?;

    Ok(req.uri().to_string())
}
