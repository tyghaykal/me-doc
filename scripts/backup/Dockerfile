# Scheduled `pg_dump | gzip` → S3 for the `db-backup` compose service.
# Same base image as the `postgres` service so pg_dump's version always
# matches the server it dumps (pg_dump refuses to dump a newer server).
# Build context is ./scripts, so backup-db.sh stays next to the other
# operator scripts rather than being duplicated into this directory.
FROM postgres:16-alpine

# dcron: alpine ships no cron daemon. tzdata: so a schedule written for a
# non-UTC TZ fires when the operator expects. curl: just to fetch mc below.
RUN apk add --no-cache bash curl dcron tzdata \
 && curl -fsSL https://dl.min.io/client/mc/release/linux-amd64/mc -o /usr/local/bin/mc \
 && chmod +x /usr/local/bin/mc

COPY backup-db.sh restore-db.sh backup/entrypoint.sh /usr/local/bin/
RUN chmod +x /usr/local/bin/backup-db.sh /usr/local/bin/restore-db.sh /usr/local/bin/entrypoint.sh

# Drops the base image's `postgres` CMD, which would otherwise be passed to
# the entrypoint as a stray argument.
CMD []
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
