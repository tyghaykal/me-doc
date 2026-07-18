#!/bin/sh
# Creates the S3 bucket in MinIO, then removes the container it ran in.
# Safe to re-run: bucket-already-exists is tolerated.
set -eu
cd "$(dirname "$0")/.."
docker compose run --rm minio-createbuckets
