#!/usr/bin/env bash
set -euo pipefail

Sandbox_smoke_test() {
    local test_name="$1"
    shift
    local output
    output=$("$@" 2>&1) || {
        echo "FAIL ${test_name}: command exited non-zero"
        echo "${output}"
        return 1
    }
    if [ -z "${output}" ]; then
        echo "FAIL ${test_name}: unexpected empty output"
        return 1
    fi
    echo "PASS ${test_name}"
}

echo "=== sandbox_smoke: running admission + enforcement tests ==="

Sandbox_smoke_test "sandbox_admission" \
    cargo test -p maos-kernel-core --test sandbox_admission -- --quiet 2>&1

if [ "$(uname -s)" = "Linux" ]; then
    Sandbox_smoke_test "sandbox_enforcement_linux" \
        cargo test -p maos-kernel-core --test sandbox_enforcement_linux -- --quiet 2>&1
    Sandbox_smoke_test "resource_caps_linux" \
        cargo test -p maos-kernel-core --test resource_caps_linux -- --quiet 2>&1
else
    echo "SKIP sandbox_enforcement_linux: not running on Linux"
    echo "SKIP resource_caps_linux: not running on Linux"
fi

echo "=== sandbox_smoke: ALL PASSED ==="
