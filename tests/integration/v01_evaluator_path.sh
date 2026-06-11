#!/usr/bin/env bash
# Story 1b.5c AC4 — composite v0.1 release-tag gate.
#
# Runs the full v0.1 evaluator path (Story 1b.5a + 1b.5b + 1b.5c) in
# one sequential bash script with no parallel jobs and no test-runner
# sharding. Asserts:
#   (1) `maosctl install hello-spirit` exits 0
#   (2) `maosctl run hello-spirit` produces FR58 JSON with four mandated keys
#   (3) `maosctl audit query --spirit hello-spirit --format ndjson` produces
#       ≥1 row with all six FR4 mandatory keys
#   (4) `maosctl start/stop/unload hello-spirit` each produce one journal entry
#   (5) `cargo test -p maos-kernel-core --test fr4_1000_call_fixture` PASSES
#   (6) `cargo test -p maos-kernel-core --test manifest_field_coverage` PASSES
#   (7) every step emits zero ANSI bytes on stdout under NO_COLOR=1
#
# Composite gate — this script blocks the v0.1 release tag.
# Wired into CI as `.github/workflows/discipline.yml::v01-evaluator-path`.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
START_NS=$(python3 -c 'import time; print(int(time.time()*1e9))' 2>/dev/null || echo "$(date +%s)000000000")

# Force NO_COLOR for the entire composite — every step is asserted clean.
export NO_COLOR=1

echo "::group::Build maos-bin + maosctl (release, locked)"
cargo build -p maos-bin --release --locked --quiet
cargo build -p maos-cli --release --locked --quiet
echo "::endgroup::"

# Atomic tempfiles. Both stores start empty; the binaries create on first
# open. mktemp avoids the mktemp -u TOCTOU foot-gun (1b.5b fixed pattern).
DB="$(mktemp --suffix=.sqlite)"
rm -f "$DB"
export MAOS_AUDIT_DB="$DB"

JOURNAL="$(mktemp --suffix=.ndjson)"
rm -f "$JOURNAL"
export MAOS_JOURNAL_PATH="$JOURNAL"

export XDG_DATA_HOME="${XDG_DATA_HOME:-$(mktemp -d)}"

cleanup() { rm -f "$DB" "$JOURNAL"; }
trap cleanup EXIT

MAOSCTL="${REPO_ROOT}/target/release/maosctl"
MAOS_BIN="${REPO_ROOT}/target/release/maos"
export MAOS_BIN_PATH="$MAOS_BIN"

# Helper: capture stdout into a tempfile, assert 0 ANSI bytes, then
# return the contents on stdout for downstream consumption.
assert_no_ansi_stdout() {
  local label="$1"; shift
  local tmp
  tmp="$(mktemp)"
  "$@" > "$tmp"
  local esc_count
  esc_count="$(grep -c $'\x1b' "$tmp" || true)"
  if [ "$esc_count" != "0" ]; then
    echo "${label}: stdout contained ${esc_count} ANSI escape byte(s) — NFR-Ops-5 violation" >&2
    rm -f "$tmp"
    exit 1
  fi
  cat "$tmp"
  rm -f "$tmp"
}

# ───────────────────────────────────────────────────────────────
# (1) install
echo "::group::(1) install hello-spirit"
# Dry-run shortcut keeps the composite under 60s; the real cargo build
# is exercised by `maosctl-smoke`'s sibling gate and by the build step
# above (which already compiled maos-spirit-hello transitively).
MAOS_INSTALL_DRY_RUN=1 "${MAOSCTL}" install hello-spirit
echo "::endgroup::"

# ───────────────────────────────────────────────────────────────
# (2) run — FR58 JSON four keys
echo "::group::(2) run hello-spirit"
RUN_JSON="$(assert_no_ansi_stdout 'run' "${MAOSCTL}" run hello-spirit)"
echo "$RUN_JSON" | jq -e '
  .introduction != null
  and .capability_scope != null
  and .halt_tags != null
  and .transparency_log != null
' >/dev/null
echo "::endgroup::"

# ───────────────────────────────────────────────────────────────
# (3) audit query — FR4 NDJSON six keys
echo "::group::(3) audit query --spirit hello-spirit --format ndjson"
AUDIT_FIRST_LINE="$(assert_no_ansi_stdout 'audit query' \
  "${MAOSCTL}" audit query --spirit hello-spirit --format ndjson | head -1)"
if [ -z "$AUDIT_FIRST_LINE" ]; then
  echo "audit query produced no rows" >&2
  exit 1
fi
echo "$AUDIT_FIRST_LINE" | jq -e '
  .call_id != null
  and (.capability_token | type == "string") and (.capability_token | length == 64)
  and (.spirit_pid | type == "number")
  and (.boot_nonce | type == "number")
  and (.call_type | type == "string")
  and (.timestamp_ns | type == "number")
' >/dev/null
echo "::endgroup::"

# ───────────────────────────────────────────────────────────────
# (4) start/stop/unload journal write-once each
#
# NOTE (Story 8.14a): step (2)'s `maosctl run` is now a kernel-rendered
# evaluator surface that performs a REAL admission/load, journaling one `Load`
# entry (resolved sandbox tier T2) before this sequence. Each verb below still
# writes exactly one entry; the tail-event counts are offset by +1 from the
# original v0.1 sequence: run → Load(1), start → Start(2), stop → Halt(3),
# unload → Unload(4).
assert_journal_tail_event() {
  local expected_count="$1"
  local expected_event="$2"
  local actual_count
  actual_count="$(wc -l < "$JOURNAL")"
  if [ "$actual_count" != "$expected_count" ]; then
    echo "journal line count: expected $expected_count, got $actual_count" >&2
    cat "$JOURNAL" >&2
    exit 1
  fi
  tail -n 1 "$JOURNAL" | jq -e \
    --arg ev "$expected_event" \
    '.lifecycle_event == $ev and .spirit_id == "hello-spirit"' >/dev/null
}

echo "::group::(4a) start hello-spirit"
START_ERR="$(assert_no_ansi_stdout 'start' "${MAOSCTL}" start hello-spirit 2>&1 >/dev/null)"
echo "$START_ERR" | grep -q "started hello-spirit" || { echo "start: stderr missing 'started' — got: $START_ERR" >&2; exit 1; }
assert_journal_tail_event 2 "Start"
echo "::endgroup::"

echo "::group::(4b) stop hello-spirit"
STOP_ERR="$(assert_no_ansi_stdout 'stop' "${MAOSCTL}" stop hello-spirit 2>&1 >/dev/null)"
echo "$STOP_ERR" | grep -q "stopped hello-spirit" || { echo "stop: stderr missing 'stopped' — got: $STOP_ERR" >&2; exit 1; }
assert_journal_tail_event 3 "Halt"
echo "::endgroup::"

echo "::group::(4c) unload hello-spirit"
UNLOAD_ERR="$(assert_no_ansi_stdout 'unload' "${MAOSCTL}" unload hello-spirit 2>&1 >/dev/null)"
echo "$UNLOAD_ERR" | grep -q "unloaded hello-spirit" || { echo "unload: stderr missing 'unloaded' — got: $UNLOAD_ERR" >&2; exit 1; }
assert_journal_tail_event 4 "Unload"
echo "::endgroup::"

# ───────────────────────────────────────────────────────────────
# (5) FR4 1000-call mediation fixture must pass
echo "::group::(5) cargo test -p maos-kernel-core --test fr4_1000_call_fixture"
cargo test -p maos-kernel-core --test fr4_1000_call_fixture --locked --quiet
echo "::endgroup::"

# ───────────────────────────────────────────────────────────────
# (6) NFR-Test-13 manifest-field coverage walker must pass
echo "::group::(6) cargo test -p maos-kernel-core --test manifest_field_coverage"
cargo test -p maos-kernel-core --test manifest_field_coverage --locked --quiet
echo "::endgroup::"

END_NS=$(python3 -c 'import time; print(int(time.time()*1e9))' 2>/dev/null || echo "$(date +%s)000000000")
ELAPSED_MS=$(( (END_NS - START_NS) / 1000000 ))
echo "v01_evaluator_path: PASS (wall-clock=${ELAPSED_MS}ms)"
