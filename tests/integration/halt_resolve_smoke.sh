#!/usr/bin/env bash
# Story 3.3 AC7 — halt-resolve end-to-end smoke test.
#
#   1. Build maosctl + maos-bin (release, locked).
#   2. Seed a fresh Transparency Log via the one-shot hello-spirit path so the
#      FR4 read surface has rows to project (mirrors audit_query_fr4_smoke.sh).
#   3. `maosctl audit query --spirit hello-spirit --format ndjson` exits 0 and
#      yields rows — the pre-halt baseline for the read path.
#   4. `maosctl halt resolve` exits 0.
#   5. The same audit query, scoped to the seeded incarnation with `--boot`,
#      still exits 0 and still yields rows: resolving a halt does not disturb
#      the pre-existing FR4 read path.
#      (`--boot` is required because `AcceptedHalt` emits a `task.orphaned`
#      frame under the kernel's own boot_nonce, and FR4 projection refuses any
#      row without a capability_token by design — see maos-audit
#      `to_fr4_ndjson`. Scoping to the seeded boot keeps the assertion about
#      the read path rather than about FR4's mediation gate.)
#   6. Exactly one approval_decision_log row with capability='halt.resolve'.
#
# Step 6 is NEVER skipped: if the `sqlite3` CLI is absent we fall back to
# python3's stdlib sqlite3 module (the reader audit_spine_smoke.sh already
# relies on). If neither reader exists the script FAILS — exiting green without
# the row proof would defeat the purpose of the test.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"

# Honor NO_COLOR if the operator set it; never inject color in CI.
export NO_COLOR="${NO_COLOR:-1}"

echo "::group::Build maosctl + maos-bin (release, locked)"
cargo build -p maos-bin -p maos-cli --release --locked --quiet
echo "::endgroup::"

MAOSCTL="${REPO_ROOT}/target/release/maosctl"
MAOS_BIN="${REPO_ROOT}/target/release/maos"

DB="$(mktemp --suffix=.sqlite)"
rm -f "$DB"
SCRATCH="$(mktemp -d)"
cleanup() { rm -f "$DB"; rm -rf "$SCRATCH"; }
trap cleanup EXIT

# Hermetic: never read or write the operator's real log/journal.
export MAOS_AUDIT_DB="$DB"
export MAOS_JOURNAL_PATH="${SCRATCH}/journal.ndjson"
export XDG_DATA_HOME="${SCRATCH}/xdg"
export MAOS_BIN_PATH="$MAOS_BIN"
mkdir -p "$XDG_DATA_HOME"

echo "::group::Seed Transparency Log via one-shot hello-spirit"
MAOS_ONE_SHOT=hello-spirit "$MAOS_BIN" >/dev/null
echo "::endgroup::"

echo "::group::maosctl audit query — pre-halt baseline"

set +e
BASELINE="$("$MAOSCTL" audit query --spirit hello-spirit --format ndjson)"
BASELINE_RC=$?
set -e

if [ "$BASELINE_RC" -ne 0 ]; then
    echo "FAIL: baseline maosctl audit query exited with code ${BASELINE_RC}" >&2
    exit 1
fi

if [ -z "$BASELINE" ]; then
    echo "FAIL: baseline maosctl audit query produced no rows" >&2
    exit 1
fi

SEED_BOOT="$(printf '%s\n' "$BASELINE" | head -1 |
    sed -n 's/.*"boot_nonce":\([0-9]*\).*/\1/p')"

if [ -z "$SEED_BOOT" ]; then
    echo "FAIL: could not read boot_nonce out of the FR4 NDJSON projection" >&2
    exit 1
fi

echo "audit query baseline: exit 0, seeded boot_nonce=${SEED_BOOT}"
echo "::endgroup::"

echo "::group::Run maosctl halt resolve"

set +e
"$MAOSCTL" halt resolve halt-001 \
    --spirit hello-spirit \
    --kind accepted-halt
RC=$?
set -e

if [ "$RC" -ne 0 ]; then
    echo "FAIL: maosctl halt resolve exited with code ${RC}" >&2
    exit 1
fi

echo "halt resolve: exit code 0"
echo "::endgroup::"

echo "::group::maosctl audit query — post-halt regression (seeded boot only)"

set +e
QUERY_OUT="$("$MAOSCTL" audit query --spirit hello-spirit --boot "$SEED_BOOT" --format ndjson)"
QUERY_RC=$?
set -e

if [ "$QUERY_RC" -ne 0 ]; then
    echo "FAIL: maosctl audit query exited with code ${QUERY_RC} after a halt resolution" >&2
    exit 1
fi

if [ -z "$QUERY_OUT" ]; then
    echo "FAIL: maosctl audit query produced no rows" >&2
    exit 1
fi

echo "audit query post-halt: exit 0, $(printf '%s\n' "$QUERY_OUT" | wc -l) row(s)"
echo "::endgroup::"

echo "::group::Verify approval_decision_log row"

SQL="SELECT COUNT(*) FROM approval_decision_log WHERE capability='halt.resolve';"

if command -v sqlite3 &>/dev/null; then
    READER="sqlite3"
    COUNT="$(sqlite3 "$DB" "$SQL")"
elif command -v python3 &>/dev/null && python3 -c "import sqlite3" 2>/dev/null; then
    READER="python3-sqlite3"
    COUNT="$(SQL="$SQL" python3 -c '
import os, sqlite3, sys
conn = sqlite3.connect(sys.argv[1])
print(conn.execute(os.environ["SQL"]).fetchone()[0])
' "$DB")"
else
    echo "FAIL: no SQLite reader available (need sqlite3, or python3 with the" >&2
    echo "      sqlite3 stdlib module) — cannot prove the approval_decision_log row" >&2
    exit 1
fi

if [ "$COUNT" -ne 1 ]; then
    echo "FAIL: expected 1 row in approval_decision_log with capability='halt.resolve', got ${COUNT}" >&2
    exit 1
fi

echo "approval_decision_log (${READER}): ${COUNT} row(s) with capability='halt.resolve'"
echo "::endgroup::"

echo "halt_resolve_smoke: PASS"
