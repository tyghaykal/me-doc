#!/bin/bash
# Lists dumps in the backup bucket and restores the one the operator picks.
# Interactive only, run by hand: `docker compose exec db-backup restore-db.sh`
set -euo pipefail

mc alias set local "$S3_ENDPOINT" "$S3_ACCESS_KEY" "$S3_SECRET_KEY" >/dev/null

mapfile -t dumps < <(mc ls "local/$S3_BUCKET_BACKUP/backups/" | awk '{print $NF}' | sort -r)

if [ "${#dumps[@]}" -eq 0 ]; then
  echo "No backups found in $S3_BUCKET_BACKUP/backups/" >&2
  exit 1
fi

echo "Available backups (newest first):"
for i in "${!dumps[@]}"; do
  printf '%3d) %s\n' "$((i + 1))" "${dumps[$i]}"
done

read -rp "Restore which one? [1-${#dumps[@]}]: " choice
if ! [[ "$choice" =~ ^[0-9]+$ ]] || [ "$choice" -lt 1 ] || [ "$choice" -gt "${#dumps[@]}" ]; then
  echo "Invalid selection." >&2
  exit 1
fi
dump="${dumps[$((choice - 1))]}"

read -rp "This overwrites the current $POSTGRES_DB database with $dump. Continue? [y/N]: " confirm
case "$confirm" in
  y|Y) ;;
  *) echo "Aborted."; exit 1 ;;
esac

tmp="/tmp/$dump"
mc cp "local/$S3_BUCKET_BACKUP/backups/$dump" "$tmp"
gunzip -c "$tmp" | psql "$DATABASE_URL"
rm -f "$tmp"

echo "Restored $dump."
