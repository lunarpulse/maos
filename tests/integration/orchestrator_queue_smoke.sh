#!/usr/bin/env bash
# Story 3.4 AC2 — orchestrator-queue smoke test.
#
# Verifies that `maosctl orchestrator queue` exits 0 three times and
# writes exactly three rows to the approval_decision_log with
# capability='orchestrator.queue'.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"

echo "::group::Build maosctl + maos-bin (release, locked)"
cargo build -p maos-bin -p maos-cli --release --locked --quiet
echo "::endgroup::"

MAOSCTL="${REPO_ROOT}/target/release/maosctl"

DB="$(mktemp --suffix=.sqlite)"
rm -f "$DB"
cleanup() { rm -f "$DB"; }
trap cleanup EXIT

echo "::group::Run maosctl orchestrator queue (×3)"

for i in 1 2 3; do
    set +e
    MAOS_AUDIT_DB="$DB" "$MAOSCTL" orchestrator queue \
        --spirit hello-spirit \
        "test instruction ${i}"
    RC=$?
    set -e

    if [ "$RC" -ne 0 ]; then
        echo "FAIL: maosctl orchestrator queue (${i}) exited with code ${RC}" >&2
        exit 1
    fi
    echo "orchestrator queue ${i}: exit code 0"
done

if ! command -v sqlite3 &>/dev/null; then
    echo "SKIP: sqlite3 not found — cannot verify approval_decision_log rows" >&2
    exit 0
fi

COUNT="$(sqlite3 "$DB" \
    "SELECT COUNT(*) FROM approval_decision_log WHERE capability='orchestrator.queue';")"

if [ "$COUNT" -ne 3 ]; then
    echo "FAIL: expected 3 rows in approval_decision_log with capability='orchestrator.queue', got ${COUNT}" >&2
    exit 1
fi

echo "approval_decision_log: ${COUNT} row(s) with capability='orchestrator.queue'"

rm -f "$DB"
trap - EXIT
echo "::endgroup::"

echo "orchestrator_queue_smoke: PASS"
