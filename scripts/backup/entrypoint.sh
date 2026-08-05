#!/bin/sh
# Renders the crontab from $BACKUP_CRON_SCHEDULE, then hands off to crond.
set -eu

# Alpine's dcron hands jobs a near-empty environment, so the compose
# `environment:` block would be invisible to backup-db.sh. Snapshot it here
# and source it from the cron line below. Must stay /bin/sh: ash's `export -p`
# emits sourceable `export VAR='value'` lines, bash's emits `declare -x`.
export -p > /etc/backup.env

# Job output goes to pid 1's stdout (crond itself, after the exec) so failures
# show up in `docker compose logs db-backup` instead of a mail spool nobody reads.
echo "${BACKUP_CRON_SCHEDULE:-0 3 * * *} . /etc/backup.env; backup-db.sh >> /proc/1/fd/1 2>&1" > /etc/crontabs/root

# Deliberately not `exec`ed: dcron calls setpgid() on itself at startup, which
# the kernel refuses for a session leader — i.e. for pid 1. Leaving this shell
# as pid 1 and crond as its child sidesteps that (and keeps /proc/1/fd/1
# pointing at the container's stdout for the cron line above).
crond -f -l 2
