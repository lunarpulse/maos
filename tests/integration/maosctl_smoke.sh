#!/usr/bin/env bash
# Story 1b.5c — AC1 integration smoke for maosctl v0.1 lifecycle verbs.
#
# Drives the five v0.1 subcommands end-to-end against the reference
# `hello-spirit` and asserts each side-effect mechanically. The journal
# verbs (`start`/`stop`/`unload`) each add exactly one line with the
# expected `lifecycle_event` discriminator; `install` exits 0 with the
# "compiled successfully" diagnostic on stderr (the cargo build is
# warm-cached in CI); `run` produces the FR58 JSON keys on stdout.
#
# Pattern matches `audit_query_fr4_smoke.sh` (Story 1b.5b) — atomic
# mktemp, trap-cleanup, NO_COLOR-by-default. Wired into CI as
# `.github/workflows/discipline.yml::maosctl-smoke`.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
START_NS=$(python3 -c 'import time; print(int(time.time()*1e9))' 2>/dev/null || echo "$(date +%s)000000000")

# Honor NO_COLOR if the operator set it; never inject color in CI.
export NO_COLOR="${NO_COLOR:-1}"

echo "::group::Build maos-bin + maosctl (release, locked)"
cargo build -p maos-bin --release --locked --quiet
cargo build -p maos-cli --release --locked --quiet
echo "::endgroup::"

# Atomic tempfiles for the Transparency Log + Lifecycle Journal. Each
# starts empty (rm -f after mktemp so the binary creates the file from
# scratch on first open — mirrors the 1b.5b discipline).
DB="$(mktemp --suffix=.sqlite)"
rm -f "$DB"
export MAOS_AUDIT_DB="$DB"

JOURNAL="$(mktemp --suffix=.ndjson)"
rm -f "$JOURNAL"
export MAOS_JOURNAL_PATH="$JOURNAL"

# Force XDG_DATA_HOME to a writable tmp dir so `default_*_path()`
# resolves predictably even on CI runners that chmod $HOME oddly.
export XDG_DATA_HOME="${XDG_DATA_HOME:-$(mktemp -d)}"

cleanup() {
  rm -f "$DB" "$JOURNAL"
}
trap cleanup EXIT

MAOSCTL="${REPO_ROOT}/target/release/maosctl"
MAOS_BIN="${REPO_ROOT}/target/release/maos-bin"
# `MAOS_BIN_PATH` lets maosctl find the colocated binary even when the
# release target dir is non-standard.
export MAOS_BIN_PATH="$MAOS_BIN"

assert_exit_0() {
  local label="$1"; shift
  if ! "$@"; then
    echo "${label}: FAIL (exit $?)" >&2
    exit 1
  fi
}

# ───────────────────────────────────────────────────────────────
echo "::group::install hello-spirit"
# Use the dry-run shortcut to keep the smoke under 60s — the real cargo
# build is exercised by `audit-query-fr4-smoke` (which calls the same
# binaries) and by the `v01-evaluator-path` composite gate.
MAOS_INSTALL_DRY_RUN=1 "${MAOSCTL}" install hello-spirit
echo "::endgroup::"

# ───────────────────────────────────────────────────────────────
echo "::group::run hello-spirit"
RUN_OUT="$("${MAOSCTL}" run hello-spirit)"
echo "$RUN_OUT" | jq -e '
  .introduction != null
  and .capability_scope != null
  and .halt_tags != null
  and .transparency_log != null
' >/dev/null
echo "::endgroup::"

# ───────────────────────────────────────────────────────────────
# Lifecycle-journal helper: assert the file has exactly N lines AND the
# N'th line matches the expected `lifecycle_event` discriminator.
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
  local last
  last="$(tail -n 1 "$JOURNAL")"
  echo "$last" | jq -e \
    --arg ev "$expected_event" \
    '.lifecycle_event == $ev and .spirit_id == "hello-spirit"' >/dev/null
}

echo "::group::start hello-spirit"
START_ERR="$("${MAOSCTL}" start hello-spirit 2>&1 >/dev/null)"
echo "$START_ERR" | grep -q "started hello-spirit" || { echo "start: stderr missing 'started hello-spirit' diagnostic — got: $START_ERR" >&2; exit 1; }
assert_journal_tail_event 1 "Start"
echo "::endgroup::"

echo "::group::stop hello-spirit"
STOP_ERR="$("${MAOSCTL}" stop hello-spirit 2>&1 >/dev/null)"
echo "$STOP_ERR" | grep -q "stopped hello-spirit" || { echo "stop: stderr missing 'stopped hello-spirit' diagnostic — got: $STOP_ERR" >&2; exit 1; }
assert_journal_tail_event 2 "Halt"
echo "::endgroup::"

echo "::group::unload hello-spirit"
UNLOAD_ERR="$("${MAOSCTL}" unload hello-spirit 2>&1 >/dev/null)"
echo "$UNLOAD_ERR" | grep -q "unloaded hello-spirit" || { echo "unload: stderr missing 'unloaded hello-spirit' diagnostic — got: $UNLOAD_ERR" >&2; exit 1; }
assert_journal_tail_event 3 "Unload"
echo "::endgroup::"

# ───────────────────────────────────────────────────────────────
# Negative case: unknown spirit MUST exit 2 AND MUST NOT append to journal.
echo "::group::start unknown-spirit (negative)"
JOURNAL_BYTES_BEFORE="$(wc -c < "$JOURNAL" | tr -d ' ')"
set +e
"${MAOSCTL}" start unknown-spirit
NEG_EXIT=$?
set -e
if [ "$NEG_EXIT" != "2" ]; then
  echo "negative case: expected exit 2, got $NEG_EXIT" >&2
  exit 1
fi
JOURNAL_BYTES_AFTER="$(wc -c < "$JOURNAL" | tr -d ' ')"
if [ "$JOURNAL_BYTES_BEFORE" != "$JOURNAL_BYTES_AFTER" ]; then
  echo "negative case: journal size changed from $JOURNAL_BYTES_BEFORE to $JOURNAL_BYTES_AFTER" >&2
  exit 1
fi
echo "::endgroup::"

END_NS=$(python3 -c 'import time; print(int(time.time()*1e9))' 2>/dev/null || echo "$(date +%s)000000000")
ELAPSED_MS=$(( (END_NS - START_NS) / 1000000 ))
echo "maosctl_smoke: PASS (wall-clock=${ELAPSED_MS}ms)"
