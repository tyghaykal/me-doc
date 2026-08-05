#!/bin/bash
# Dumps Postgres, gzips it, uploads it to the backup bucket, prunes old dumps.
# Runs on $BACKUP_CRON_SCHEDULE inside the `db-backup` service, and is safe to
# run by hand: `docker compose exec db-backup backup-db.sh`.
set -euo pipefail

dump="/tmp/${POSTGRES_DB}-$(date -u +%Y%m%dT%H%M%SZ).sql.gz"

# pipefail (above) is load-bearing here: without it a failed pg_dump still
# exits 0 as long as gzip succeeds, leaving a truncated .gz that looks like a
# healthy backup until the day someone tries to restore from it.
pg_dump "$DATABASE_URL" | gzip > "$dump"

mc alias set local "$S3_ENDPOINT" "$S3_ACCESS_KEY" "$S3_SECRET_KEY"
mc cp "$dump" "local/$S3_BUCKET_BACKUP/backups/"
rm -f "$dump"

# Unset/empty means keep everything — retention has to be opted into, so a
# misconfigured var can never silently delete the only copies that exist.
if [ -n "${BACKUP_RETENTION_DAYS:-}" ]; then
  mc rm --recursive --force --older-than "${BACKUP_RETENTION_DAYS}d" "local/$S3_BUCKET_BACKUP/backups/"
fi
