#!/usr/bin/env bash
# onb_nfr2_timing.sh — NFR-Onb-2 gate: 5-minute evaluator path.
#
# Simulates a fresh clone: clean build artifacts, build release binary,
# run hello-Spirit one-shot, measure elapsed wall-clock ≤ 300 s,
# validate JSON output shape.
#
# CI runs this on ubuntu-latest. Live path requires MAOS_ANTHROPIC_API_KEY;
# without it the mock fallback path runs in <1s.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$REPO_ROOT"

echo "=== NFR-Onb-2: 5-minute evaluator path ==="

# Start timing before cargo clean to measure the full clone-to-response path (AC2)
time_start=$(date +%s)

# Step 1: Simulate fresh clone — clean build artifacts
echo "--- Cleaning build artifacts (simulating fresh clone) ---"
cargo clean

# Step 2: Build maos-bin in release mode
# Use `-p maos-bin` instead of `--bin maos-bin` because the workspace
# manifest declares `default-members = []` — bare `--bin` resolution
# from the workspace root would panic with "manifest is virtual,
# workspace has no members" (exit 101).
echo "--- Building maos-bin (release, locked) ---"
cargo build -p maos-bin --release --locked

# Step 3: One-shot execution
echo "--- Running hello-Spirit one-shot ---"
output=$(MAOS_ONE_SHOT=hello-spirit NO_COLOR=1 ./target/release/maos 2>/dev/null)
time_end=$(date +%s)
elapsed=$((time_end - time_start))

echo "Elapsed wall-clock: ${elapsed}s (limit: 300s)"

# Step 4: Validate JSON output
echo "--- Validating JSON output ---"
if ! echo "$output" | python3 -c "
import json, sys
data = json.load(sys.stdin)
assert 'introduction' in data, 'missing introduction'
assert 'capability_scope' in data, 'missing capability_scope'
assert 'halt_tags' in data, 'missing halt_tags'
assert 'transparency_log' in data, 'missing transparency_log'
print('JSON keys validated OK')
"; then
    echo "ERROR: JSON output missing required keys"
    echo "Output was: $output"
    exit 1
fi

# Step 5: Binary size gate — stripped maos-bin ≤10MB (AC4)
echo "--- Checking binary size ---"
strip target/release/maos
bin_size=$(stat -c%s target/release/maos)
max_size=10485760  # 10 MiB
echo "maos-bin stripped size: ${bin_size} bytes (limit: ${max_size})"
if [ "$bin_size" -gt "$max_size" ]; then
    echo "ERROR: AC4 binary size violation: ${bin_size} bytes > ${max_size} bytes limit"
    exit 1
fi

# Step 6: Assert elapsed ≤ 300s
if [ "$elapsed" -gt 300 ]; then
    echo "ERROR: NFR-Onb-2 violation: elapsed ${elapsed}s > 300s limit"
    exit 1
fi

# Step 7: Assert no ANSI escape codes
if echo "$output" | grep -q $'\x1b\['; then
    echo "ERROR: NFR-Ops-5 violation: JSON output contains ANSI escape codes"
    exit 1
fi

echo "=== NFR-Onb-2 PASSED: evaluator path completed in ${elapsed}s ==="
exit 0
