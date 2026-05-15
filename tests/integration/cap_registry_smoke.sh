#!/usr/bin/env bash
set -euo pipefail

# Capability Registry smoke test — v0.1-β
# Verifies that maos-bin starts, capability registry initializes,
# and exits cleanly within the run-window timeout.

# Build first so the 5s run-window does NOT include compile time.
# On CI with a cold Swatinem cache, `cargo run` would otherwise eat
# the whole timeout in compilation and exit 124 before the binary
# ever launched.
echo "=== cap_registry_smoke: building maos-bin ==="
cargo build -p maos-bin --quiet

echo "=== cap_registry_smoke: starting maos-bin ==="

# maos-bin must start, initialize, accept SIGINT, and exit cleanly
# within 8 seconds. We send SIGINT after 3 seconds to trigger
# graceful shutdown.
timeout 8s bash -c '
    ./target/debug/maos-bin &
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
