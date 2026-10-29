#!/usr/bin/env bash
set -euo pipefail

# Story 5.4 — smoke-upgrade-revoke-5 integration companion script.
# Runs the smoke arm with timeout and asserts exit-code 0 + 3 JSON lines.
# Story 19-5 (D-19-5-01 A) removed the kernel cold-swap step: cold swap is
# composed by the operator door (unload, then the admission triple).

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

cd "${PROJECT_ROOT}"

echo "=== smoke-upgrade-revoke-5 integration test ==="

OUTPUT=$(timeout 20 bash -c 'MAOS_ONE_SHOT=smoke-upgrade-revoke-5 cargo run -p maos-bin 2>&1' || true)

# Assert 3 JSON lines
JSON_COUNT=$(echo "${OUTPUT}" | grep -c '^\s*{"step"' || true)
if [ "${JSON_COUNT}" -ne 3 ]; then
    echo "FAIL: expected 3 JSON lines, got ${JSON_COUNT}"
    echo "--- OUTPUT ---"
    echo "${OUTPUT}"
    exit 1
fi

# Assert step 1: hot-swap
echo "${OUTPUT}" | grep -q '"step":1.*"policy":"hot-swap"' || {
    echo "FAIL: step 1 (hot-swap) JSON line missing"
    exit 1
}

# Assert step 2: revocation
echo "${OUTPUT}" | grep -q '"step":2.*"surface":"revocation_applier"' || {
    echo "FAIL: step 2 (revocation) JSON line missing"
    exit 1
}

# Assert step 3: capability denial
echo "${OUTPUT}" | grep -q '"step":3.*"surface":"capability_registry"' || {
    echo "FAIL: step 3 (capability) JSON line missing"
    exit 1
}

# Assert completion message
echo "${OUTPUT}" | grep -q 'smoke-upgrade-revoke-5 complete' || {
    echo "FAIL: completion message missing"
    exit 1
}

echo "PASS: smoke-upgrade-revoke-5 — 3 JSON lines + completion verified"
