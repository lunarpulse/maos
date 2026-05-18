#!/usr/bin/env bash
# Story 3.3 D2 — halt-resolve smoke test.
#
# Verifies that `maosctl halt resolve` exits 0 and writes exactly one
# row to the approval_decision_log with capability='halt.resolve'.

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

echo "::group::Run maosctl halt resolve"

set +e
MAOS_AUDIT_DB="$DB" "$MAOSCTL" halt resolve halt-001 \
    --spirit hello-spirit \
    --kind accepted-halt
RC=$?
set -e

if [ "$RC" -ne 0 ]; then
    echo "FAIL: maosctl halt resolve exited with code ${RC}" >&2
    exit 1
fi

echo "halt resolve: exit code 0"

if ! command -v sqlite3 &>/dev/null; then
    echo "SKIP: sqlite3 not found — cannot verify approval_decision_log row" >&2
    exit 0
fi

COUNT="$(sqlite3 "$DB" \
    "SELECT COUNT(*) FROM approval_decision_log WHERE capability='halt.resolve';")"

if [ "$COUNT" -ne 1 ]; then
    echo "FAIL: expected 1 row in approval_decision_log with capability='halt.resolve', got ${COUNT}" >&2
    exit 1
fi

echo "approval_decision_log: ${COUNT} row(s) with capability='halt.resolve'"

rm -f "$DB"
trap - EXIT
echo "::endgroup::"

echo "halt_resolve_smoke: PASS"
