#!/usr/bin/env bash
# Story 1b.5b — AC4 integration smoke for `maosctl audit query`.
#
# Runs the canonical FR4 verification path end-to-end:
#   1. Build `maos-bin` and `maosctl` (release, locked).
#   2. Point both at a fresh on-disk SQLite via `MAOS_AUDIT_DB`.
#   3. Run `MAOS_ONE_SHOT=hello-spirit ./target/release/maos-bin` — this
#      seeds the Transparency Log with one `inference.call` row (plus the
#      capability-issue / capability-invocation rows the registry emits).
#   4. Pipe `maosctl audit query --spirit hello-spirit --format ndjson`
#      through `jq -e` and assert all six FR4 mandatory keys are present
#      on the first line.
#
# Exit 0 on success, exit 1 on missing field (jq fails non-zero on either
# missing key or false value). Wall-clock elapsed is printed for the
# operator NFR-Onb-2 (≤5-minute evaluator path).
#
# Wired into CI by `.github/workflows/discipline.yml::audit-query-fr4-smoke`.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
START_NS=$(date +%s%N)

# Honor NO_COLOR if the operator set it; never inject color in CI.
export NO_COLOR="${NO_COLOR:-1}"

echo "::group::Build maos-bin + maosctl (release, locked)"
cargo build -p maos-bin --release --locked --quiet
cargo build -p maos-cli --release --locked --quiet
echo "::endgroup::"

# Use a tempfile so concurrent runs (CI matrix) don't collide.
# mktemp creates the file atomically (no TOCTOU race vs mktemp -u).
DB="$(mktemp --suffix=.sqlite)"
rm -f "$DB"
export MAOS_AUDIT_DB="$DB"

# Some CI runners chmod $HOME oddly — force XDG_DATA_HOME to a writable
# location so `default_transparency_log_path()` resolves predictably.
export XDG_DATA_HOME="${XDG_DATA_HOME:-$(mktemp -d)}"

cleanup() {
  rm -f "$DB"
}
trap cleanup EXIT

echo "::group::Seed Transparency Log via one-shot hello-spirit"
MAOS_ONE_SHOT=hello-spirit "${REPO_ROOT}/target/release/maos-bin" >/dev/null
echo "::endgroup::"

echo "::group::maosctl audit query — FR4 NDJSON schema check"
FIRST_LINE="$("${REPO_ROOT}/target/release/maosctl" audit query \
  --spirit hello-spirit \
  --format ndjson | head -1)"

if [ -z "$FIRST_LINE" ]; then
  echo "FR4 smoke FAILED: maosctl audit query produced no rows" >&2
  exit 1
fi

echo "$FIRST_LINE" | jq -e \
  '.call_id != null
   and (.capability_token | type == "string") and (.capability_token | length == 64)
   and (.spirit_pid | type == "number")
   and (.boot_nonce | type == "number")
   and (.call_type | type == "string")
   and (.timestamp_ns | type == "number")' >/dev/null
echo "::endgroup::"

END_NS=$(date +%s%N)
ELAPSED_MS=$(( (END_NS - START_NS) / 1000000 ))
echo "audit_query_fr4_smoke: PASS (wall-clock=${ELAPSED_MS}ms)"
