#!/usr/bin/env bash
set -euo pipefail

# Story 5.4 — smoke-upgrade-revoke-5 integration companion script.
# Runs the smoke arm with timeout and asserts exit-code 0 + 4 JSON lines.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

cd "${PROJECT_ROOT}"

echo "=== smoke-upgrade-revoke-5 integration test ==="

OUTPUT=$(timeout 20 bash -c 'MAOS_ONE_SHOT=smoke-upgrade-revoke-5 cargo run -p maos-bin 2>&1' || true)

# Assert 4 JSON lines
JSON_COUNT=$(echo "${OUTPUT}" | grep -c '^\s*{"step"' || true)
if [ "${JSON_COUNT}" -ne 4 ]; then
    echo "FAIL: expected 4 JSON lines, got ${JSON_COUNT}"
    echo "--- OUTPUT ---"
    echo "${OUTPUT}"
    exit 1
fi

# Assert step 1: hot-swap
echo "${OUTPUT}" | grep -q '"step":1.*"policy":"hot-swap"' || {
    echo "FAIL: step 1 (hot-swap) JSON line missing"
    exit 1
}

# Assert step 2: cold-swap
echo "${OUTPUT}" | grep -q '"step":2.*"policy":"cold-swap"' || {
    echo "FAIL: step 2 (cold-swap) JSON line missing"
    exit 1
}

# Assert step 3: revocation
echo "${OUTPUT}" | grep -q '"step":3.*"surface":"revocation_applier"' || {
    echo "FAIL: step 3 (revocation) JSON line missing"
    exit 1
}

# Assert step 4: capability denial
echo "${OUTPUT}" | grep -q '"step":4.*"surface":"capability_registry"' || {
    echo "FAIL: step 4 (capability) JSON line missing"
    exit 1
}

# Assert completion message
echo "${OUTPUT}" | grep -q 'smoke-upgrade-revoke-5 complete' || {
    echo "FAIL: completion message missing"
    exit 1
}

echo "PASS: smoke-upgrade-revoke-5 — 4 JSON lines + completion verified"
