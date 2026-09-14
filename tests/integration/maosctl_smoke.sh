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

# ── Story 16-1 / D-16-1-Q + AC7 ──────────────────────────────────────────
# An EMPTY scratch HOME, deliberately NOT `maos init`-ed. The operator door's
# endpoint comes from `<home>/control.json`, so inheriting the runner's HOME
# would make this script's exit codes depend on whether someone had ever run
# `maos init` on the machine — and the CI decoy step now guarantees that HOME
# HAS a control.json with a held port.
#
# `HOME` and NOT `MAOS_HOME`: `MAOS_HOME` takes precedence over the
# `MAOS_AUDIT_DB` / `MAOS_JOURNAL_PATH` this script sets and asserts on.
export HOME="$(mktemp -d)"

cleanup() {
  rm -f "$DB" "$JOURNAL"
}
trap cleanup EXIT

MAOSCTL="${REPO_ROOT}/target/release/maosctl"
MAOS_BIN="${REPO_ROOT}/target/release/maos"
# `MAOS_BIN_PATH` lets maosctl find the colocated binary even when the
# release target dir is non-standard.
export MAOS_BIN_PATH="$MAOS_BIN"

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

# ── Story 16-1 / AC7 — the lifecycle verbs no longer journal offline ─────
#
# `start`/`unload` are DOOR verbs now (D-16-1-A). With no daemon and no
# `control.json` they are a typed configuration refusal, not a journal write:
# the measured HEAD behaviour was `stop hello-spirit` exiting 0 and appending
# a `Halt` row while nothing had stopped. So the assertions invert — what this
# script proves is that the journal does NOT move.
#
# `run hello-spirit` above is unchanged: it is the FR58 evaluator turn
# (D-16-1-M), not an operator verb, and four scripts plus two CI jobs depend
# on it working with no daemon.
JOURNAL_BYTES_BEFORE="$(wc -c < "$JOURNAL" | tr -d ' ')"

assert_exit_and_journal_unchanged() {
  local verb="$1" spirit="$2" want="$3"
  set +e
  "${MAOSCTL}" "$verb" "$spirit" >/dev/null 2>"${JOURNAL}.err"
  local got=$?
  set -e
  if [ "$got" != "$want" ]; then
    echo "$verb $spirit: expected exit $want, got $got — $(cat "${JOURNAL}.err")" >&2
    exit 1
  fi
  local after
  after="$(wc -c < "$JOURNAL" | tr -d ' ')"
  if [ "$JOURNAL_BYTES_BEFORE" != "$after" ]; then
    echo "$verb $spirit: journal moved from $JOURNAL_BYTES_BEFORE to $after — a refused verb must write nothing" >&2
    exit 1
  fi
}

echo "::group::start hello-spirit with no door (78, journal unchanged)"
assert_exit_and_journal_unchanged start hello-spirit 78
echo "::endgroup::"

echo "::group::unload hello-spirit with no door (78, journal unchanged)"
assert_exit_and_journal_unchanged unload hello-spirit 78
echo "::endgroup::"

# `stop` never reaches a door at all: no kernel transition is named stop
# (D-16-1-I), so it is a local usage refusal, exit 2, whatever the door's
# state.
echo "::group::stop hello-spirit (refused client-side, 2)"
assert_exit_and_journal_unchanged stop hello-spirit 2
STOP_ERR="$(cat "${JOURNAL}.err")"
echo "$STOP_ERR" | grep -q "no kernel transition is named stop" || {
  echo "stop: the refusal must name why — got: $STOP_ERR" >&2; exit 1; }
echo "$STOP_ERR" | grep -q "maosctl pause" || {
  echo "stop: the refusal must name the verb that replaces it — got: $STOP_ERR" >&2; exit 1; }
echo "::endgroup::"

# ───────────────────────────────────────────────────────────────
# Negative case: an unknown spirit must still write nothing. The NAME check
# moved into the daemon (D-16-1-K deleted maosctl's Transparency-Log
# preflight, which was what refused a live `butler`), so with no door
# configured the refusal is the same 78 — and the journal is what matters.
echo "::group::start unknown-spirit (negative)"
assert_exit_and_journal_unchanged start unknown-spirit 78
echo "::endgroup::"
rm -f "${JOURNAL}.err"

END_NS=$(python3 -c 'import time; print(int(time.time()*1e9))' 2>/dev/null || echo "$(date +%s)000000000")
ELAPSED_MS=$(( (END_NS - START_NS) / 1000000 ))
echo "maosctl_smoke: PASS (wall-clock=${ELAPSED_MS}ms)"
