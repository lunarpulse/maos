#!/usr/bin/env bash
# Story 5.1 Task 0 — Epic 4 retro §A1 closure smoke test.
#
# Verifies that `MAOS_ONE_SHOT=smoke-epic-4` walks the kernel-side
# Epic 4 dataflow end-to-end and exits 0 with the expected 6 stdout
# lines confirming each surface.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"

echo "::group::Build maos-bin (release, locked)"
cargo build -p maos-bin --release --locked --quiet
echo "::endgroup::"

echo "::group::Run MAOS_ONE_SHOT=smoke-epic-4"

set +e
OUTPUT="$(MAOS_ONE_SHOT=smoke-epic-4 \
    MAOS_AUDIT_DB="$(mktemp -u --suffix=.sqlite)" \
    "${REPO_ROOT}/target/release/maos" 2>/dev/null)"
RC=$?
set -e

if [ "$RC" -ne 0 ]; then
    echo "FAIL: smoke-epic-4 exited with code ${RC}" >&2
    exit 1
fi

echo "smoke-epic-4: exit code 0"

# Verify the 6 expected surfaces are present in stdout
echo "$OUTPUT" | grep -q '"surface": "scalar_write_halt_fire"' || {
    echo "FAIL: missing scalar_write_halt_fire surface" >&2
    exit 1
}
echo "$OUTPUT" | grep -q '"surface": "halt_resolve_provided_context"' || {
    echo "FAIL: missing halt_resolve_provided_context surface" >&2
    exit 1
}
echo "$OUTPUT" | grep -q '"surface": "self_telemetry"' || {
    echo "FAIL: missing self_telemetry surface" >&2
    exit 1
}
echo "$OUTPUT" | grep -q '"surface": "distillate_write_empty_lineage"' || {
    echo "FAIL: missing distillate_write_empty_lineage surface" >&2
    exit 1
}
echo "$OUTPUT" | grep -q '"surface": "distillate_write_proper_lineage"' || {
    echo "FAIL: missing distillate_write_proper_lineage surface" >&2
    exit 1
}
echo "$OUTPUT" | grep -q '"surface": "log_recall"' || {
    echo "FAIL: missing log_recall surface" >&2
    exit 1
}

echo "All 6 surfaces present in output"
echo "::endgroup::"

echo "smoke_epic_4: PASS"
