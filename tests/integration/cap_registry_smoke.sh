#!/usr/bin/env bash
set -euo pipefail

# Capability Registry smoke test — v0.1-β
# Verifies that maos-bin starts, capability registry initializes,
# and exits cleanly within the timeout.

echo "=== cap_registry_smoke: starting maos-bin ==="

# maos-bin must start and exit (via SIGINT) within 5 seconds.
# We send it SIGINT after 3 seconds to trigger graceful shutdown.
timeout 5s bash -c '
    cargo run -p maos-bin --quiet &
    MAOS_PID=$!
    sleep 3
    kill -INT $MAOS_PID 2>/dev/null || true
    wait $MAOS_PID
    exit $?
'

EXIT_CODE=$?
if [ $EXIT_CODE -ne 0 ]; then
    echo "=== cap_registry_smoke: FAIL (exit code $EXIT_CODE) ==="
    exit 1
fi

echo "=== cap_registry_smoke: PASS ==="
