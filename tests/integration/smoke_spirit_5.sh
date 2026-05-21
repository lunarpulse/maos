#!/usr/bin/env bash
# Story 5.1 Task 8 — smoke-spirit-5 integration test.
#
# Verifies that `MAOS_ONE_SHOT=smoke-spirit-5` walks the supervised
# lifecycle end-to-end (load/start/pause/resume/unload) and prints
# all 11 hook lines (5 fired, 6 deferred).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"

echo "::group::Build maos-bin (release, locked)"
cargo build -p maos-bin --release --locked --quiet
echo "::endgroup::"

echo "::group::Run MAOS_ONE_SHOT=smoke-spirit-5"

set +e
OUTPUT="$(MAOS_ONE_SHOT=smoke-spirit-5 \
    MAOS_IDLE_FAST=1 \
    MAOS_AUDIT_DB="$(mktemp -u --suffix=.sqlite)" \
    "${REPO_ROOT}/target/release/maos-bin" 2>/dev/null)"
RC=$?
set -e

if [ "$RC" -ne 0 ]; then
    echo "FAIL: smoke-spirit-5 exited with code ${RC}" >&2
    exit 1
fi

echo "smoke-spirit-5: exit code 0"

# Verify all 11 hook lines are present
EXPECTED_HOOKS=(
    on_load
    on_start
    on_pause
    on_resume
    on_unload
    on_frame
    on_idle
    on_telemetry_event
    on_schedule
    on_swap_in
    on_consolidate
)

for hook in "${EXPECTED_HOOKS[@]}"; do
    if ! echo "$OUTPUT" | grep -q "\"hook\": \"$hook\""; then
        echo "FAIL: missing hook $hook" >&2
        exit 1
    fi
done

HOOK_COUNT=$(echo "$OUTPUT" | grep -c '"hook"')
if [ "$HOOK_COUNT" -ne 11 ]; then
    echo "FAIL: expected 11 hooks, got ${HOOK_COUNT}" >&2
    exit 1
fi

echo "All 11 hooks present (5 fired, 6 deferred)"
echo "::endgroup::"

echo "smoke_spirit_5: PASS"
