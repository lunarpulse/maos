#!/usr/bin/env bash
set -euo pipefail

export MAOS_SUPERVISION_FAST=1
# Use dev binary (faster than --release build) with a timeout.
# Shutdown hang is a pre-existing tokio-runtime issue (smoke-spirit-5 has the same
# behaviour); we capture output before the timeout and grep-assert.
output=$(MAOS_ONE_SHOT=smoke-supervision-5 timeout 15 ./target/debug/maos-bin 2>&1 || true)
echo "$output"

echo "$output" | grep -q '"step": 1, "surface": "crash_detector"' || { echo "FAIL: step 1 crash_detector missing"; exit 1; }
echo "$output" | grep -q '"step": 2, "surface": "progress_watchdog"' || { echo "FAIL: step 2 progress_watchdog missing"; exit 1; }
echo "$output" | grep -q '"step": 3, "surface": "silent_failure_detector"' || { echo "FAIL: step 3 silent_failure_detector missing"; exit 1; }
echo "$output" | grep -q '"step": 4, "surface": "cold_restart"' || { echo "FAIL: step 4 cold_restart missing"; exit 1; }

# Magnitude assertions (Story 5.1 deferred §1 closure)
echo "$output" | grep -qE '"halt_receipts_produced": [1-9]' || { echo "FAIL: no halt receipts produced"; exit 1; }
echo "$output" | grep -qE '"task_stalled_emitted": [1-9]' || { echo "FAIL: no task stalled emitted"; exit 1; }
echo "$output" | grep -qE '"silent_failure_suspect_emitted": [1-9]' || { echo "FAIL: no silent failure suspect emitted"; exit 1; }
echo "$output" | grep -qE '"in_flight_recovered": [1-9]' || { echo "FAIL: no in-flight recovered"; exit 1; }

echo "smoke-supervision-5 OK"
