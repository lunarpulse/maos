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

# Story 16-1 / D-16-1-Q — an EMPTY scratch HOME, deliberately not
# `maos init`-ed: step (4) below asserts the "no operator door configured"
# refusal, and inheriting the runner's HOME would make it depend on whether
# the CI decoy had seeded a real `control.json` there. `HOME` and not
# `MAOS_HOME`, which would override the `MAOS_JOURNAL_PATH` this gate reads.
export HOME="$(mktemp -d)"

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
# (4) Story 16-1 / AC7 — the lifecycle verbs are DOOR verbs and write NOTHING
# offline.
#
# This step used to assert that `start`/`stop`/`unload` each appended exactly
# one Lifecycle Journal row. Measured: none of them reached the scheduler, and
# `stop` wrote a `Halt` row while the Spirit kept running. They are now
# authenticated door verbs (D-16-1-A); with no daemon and no `control.json`
# they are a typed configuration refusal, so what this step proves is the
# INVERSE — the journal does not move, and the refusal output stays ANSI-free
# (the v0.1 accessibility contract this gate exists for).
#
# Step (2)'s `maosctl run` is deliberately unchanged: it is the FR58 evaluator
# turn (D-16-1-M), not an operator verb.
JOURNAL_BYTES_BEFORE="$(wc -c < "$JOURNAL" | tr -d ' ')"

assert_refused_ansi_free() {
  local verb="$1" spirit="$2" want="$3"
  # Story 16-2 — capture the exit code DIRECTLY. The previous shape ran the
  # command inside a `set +e` command substitution whose function always
  # returned 0, so every refusal read as exit 0 and step (4a) failed even
  # against a correct binary (measured red at `bb9f0657` in a clean
  # worktree). The ANSI-free contract is checked on BOTH streams here.
  local out err got
  out="$(mktemp)"
  err="$(mktemp)"
  set +e
  "${MAOSCTL}" "$verb" "$spirit" > "$out" 2> "$err"
  got=$?
  set -e
  if [ "$got" != "$want" ]; then
    echo "$verb $spirit: expected exit $want, got $got — $(cat "$err")" >&2
    rm -f "$out" "$err"
    exit 1
  fi
  if grep -q $'\x1b' "$out" || grep -q $'\x1b' "$err"; then
    echo "$verb $spirit: refusal carried ANSI bytes" >&2
    rm -f "$out" "$err"
    exit 1
  fi
  local after
  after="$(wc -c < "$JOURNAL" | tr -d ' ')"
  if [ "$JOURNAL_BYTES_BEFORE" != "$after" ]; then
    echo "$verb $spirit: journal moved $JOURNAL_BYTES_BEFORE -> $after; a refused verb writes nothing" >&2
    rm -f "$out" "$err"
    exit 1
  fi
  cat "$err"
  rm -f "$out" "$err"
}

echo "::group::(4a) start hello-spirit with no door (78, journal unchanged)"
assert_refused_ansi_free start hello-spirit 78 >/dev/null
echo "::endgroup::"

echo "::group::(4b) stop hello-spirit refused client-side (2)"
STOP_ERR="$(assert_refused_ansi_free stop hello-spirit 2)"
echo "$STOP_ERR" | grep -q "no kernel transition is named stop" || {
  echo "stop: the refusal must name why — got: $STOP_ERR" >&2; exit 1; }
echo "::endgroup::"

echo "::group::(4c) unload hello-spirit with no door (78, journal unchanged)"
assert_refused_ansi_free unload hello-spirit 78 >/dev/null
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
