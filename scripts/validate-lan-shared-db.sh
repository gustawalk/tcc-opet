#!/usr/bin/env bash
# Validate the OpetS LAN shared database on a real network share.
#
# Usage:
#   ./scripts/validate-lan-shared-db.sh <share-mount-point>
#
# Requirements:
#   - a network share mounted (SMB/Samba, NFS, SSHFS...) and writable by the
#     current user. For SMB/Samba on Linux the mount MUST be usable by SQLite:
#       * SMB1 + unix extensions  -> SQLite works (see check below)
#       * SMB2/3 (default)        -> SQLite FAILS with "database is locked";
#         this script detects it before running the heavy matrix.
#       * Windows/macOS clients use native byte-range locks and are expected to
#         work, but that path cannot be validated from this Linux script.
#   - the workspace compiled. This runs the ignored stress suite via cargo:
#       cargo test --lib -- --ignored storage_concurrency --nocapture
#
# What it validates:
#   1. The share is SQLite-compatible (single-connection CREATE/INSERT/READ).
#   2. The write-storm matrix (2..64 concurrent writers, 8 readers).
#   3. The degraded path (commit backlog exceeding a short busy timeout).
#   4. Failure drill: kill a writer with SIGKILL mid-storm and confirm the
#      database stays intact and consistent afterwards.
#
# Exit codes: 0 all checks passed; 1 any check failed; 2 wrong arguments.

set -u

run() {
  local before
  before=$(date +%s)
  "$@" > /tmp/opets-lan-validation.log 2>&1
  local rc=$?
  local after
  after=$(date +%s)
  echo "  rc=$rc (${after}s) $*"
  return $rc
}

[ "$#" -ne 1 ] && {
  echo "usage: $0 <share-mount-point>" >&2
  exit 2
}
share="$1"
[ -d "$share" ] || { echo "not a directory: $share" >&2; exit 2; }
[ -w "$share" ] || { echo "share is not writable: $share" >&2; exit 2; }

cd "$(dirname "$0")/../src-tauri" || exit 2
echo "== OpetS LAN shared-db validation on: $share =="

# 1) SQLite compatibility probe (cheap, decisive).
echo "== [1/4] SQLite compatibility probe =="
probe="$share/opets-sqltest-$$.db"
if run sqlite3 "$probe" "CREATE TABLE t(a); INSERT INTO t VALUES(1);" && \
   [ "$(run sqlite3 "$probe" "SELECT COUNT(*) FROM t;")" = "1" ]; then
  rm -f "$probe" "$probe"-wal "$probe"-shm
  echo "   share is SQLite-compatible (proceed)"
else
  rm -f "$probe" "$probe"-wal "$probe"-shm
  echo "   FAILED: SQLite cannot write on this share. Over SMB/Samba this is the"
  echo "   known SMB2/3 byte-range-lock limitation. Use SMB1+unix extensions on"
  echo "   Linux, or a Windows/macOS client, or migrate to the embedded-server"
  echo "   approach. See docs/specs/database-compartilhada-lan/implementation.md."
  exit 1
fi

# 2) Write-storm matrix.
echo "== [2/4] write-storm matrix (2..64 writers x 2400 commits, 8 readers) =="
run env STRESS_DB_PATH="$share/opets-stress-matrix.db" \
  cargo test --lib -- --ignored \
  storage_concurrency_stress::write_storm_scale_and_integrity --nocapture \
  || { echo "   matrix FAILED" >&2; exit 1; }
rm -f "$share/opets-stress-matrix.db" "$share/opets-stress-matrix.db-wal" "$share/opets-stress-matrix.db-shm"

# 3) Degraded path.
echo "== [3/4] degraded path (commit backlog over short busy timeout) =="
run env STRESS_DB_PATH="$share/opets-stress-degraded.db" \
  cargo test --lib -- --ignored \
  storage_concurrency_stress::commit_backlog_exceeding_busy_timeout_fails_cleanly_without_corruption \
  --nocapture \
  || { echo "   degraded path FAILED" >&2; exit 1; }
rm -f "$share/opets-stress-degraded.db" "$share/opets-stress-degraded.db-wal" "$share/opets-stress-degraded.db-shm"

# 4) Failure drill: SIGKILL a writer mid-storm, then integrity-check.
echo "== [4/4] failure drill: kill a writer mid-storm =="
drilldb="$share/opets-drill.db"
rm -f "$drilldb" "$drilldb"-wal "$drilldb"-shm
STRESS_DB_PATH="$drilldb" cargo test --lib -- --ignored \
  storage_concurrency_stress::write_storm_scale_and_integrity --nocapture \
  > /tmp/opets-lan-drill.log 2>&1 &
cargo_pid=$!
# Wait for the storm to build (matrix reruns sequentially; kill during a later stage).
sleep 25
writers=$(pgrep -f "tcc_opet.*write_storm_scale_and_integrity|tcc_opet" | grep -v "^$cargo_pid$" | head -3)
for w in $writers; do kill -9 "$w" 2>/dev/null; done
wait "$cargo_pid" 2>/dev/null
rc=$?
echo "   storm exited rc=$rc after killers"
sqlite3 "$drilldb" "PRAGMA integrity_check;"
drill_rc=$?
rm -f "$drilldb" "$drilldb"-wal "$drilldb"-shm
if [ "$drill_rc" -eq 0 ] && grep -q "integrity_result=ok" /tmp/opets-lan-drill.log; then
  echo "   integrity_check: ok"
  echo "ALL CHECKS PASSED"
  exit 0
fi
echo "   integrity_check FAILED after kill drill" >&2
exit 1