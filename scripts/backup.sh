#!/usr/bin/env bash
# Online backup of the SQLite database. Safe while the app is running:
# `VACUUM INTO` reads a consistent snapshot through SQLite itself, unlike a
# raw file copy, which can catch the database (or its WAL file) mid-write.
#
# Usage:  scripts/backup.sh            (keeps the newest 30 backups)
#         KEEP=60 scripts/backup.sh
# Cron:   0 3 * * * /path/to/EnglishIELTSWebsite/scripts/backup.sh >> /path/to/backup.log 2>&1

set -euo pipefail
cd "$(dirname "$0")/.."

KEEP="${KEEP:-30}"
mkdir -p backups

# VACUUM INTO refuses to overwrite, so never reuse a name (two runs in one second).
name="ielts-$(date +%Y-%m-%d_%H%M%S).db"
while [[ -e "backups/$name" ]]; do
    sleep 1
    name="ielts-$(date +%Y-%m-%d_%H%M%S).db"
done

docker compose exec -T app sqlite3 /data/ielts.db "VACUUM INTO '/backups/$name'"

# Make sure the copy opens and is not corrupt.
check="$(docker compose exec -T app sqlite3 "/backups/$name" 'PRAGMA integrity_check;')"
if [[ "$check" != "ok" ]]; then
    echo "Backup $name failed integrity check: $check" >&2
    exit 1
fi
echo "Backup written: backups/$name"

# Delete all but the newest $KEEP backups.
ls -1t backups/ielts-*.db | tail -n +"$((KEEP + 1))" | xargs -r rm --
