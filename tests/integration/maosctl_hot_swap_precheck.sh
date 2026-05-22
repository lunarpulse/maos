#!/usr/bin/env bash
# Smoke test: maosctl spirit hot-swap-precheck subcommand parsing (AC7).
# Verifies clap parsing and exit-code semantics.
set -euo pipefail

cd "$(dirname "$0")/../.."

# Build the CLI and binary first.
cargo build -p maos-cli -p maos-bin --quiet

CLI="./target/debug/maosctl"

echo "=== smoke: hot-swap-precheck clap parsing ==="

# 1. Missing --from should fail (clap error).
if $CLI spirit hot-swap-precheck butler --to /dev/null 2>/dev/null; then
    echo "FAIL: missing --from should error"
    exit 1
fi
echo "PASS: missing --from errors"

# 2. Missing --to should fail (clap error).
if $CLI spirit hot-swap-precheck butler --from 0.3.1 2>/dev/null; then
    echo "FAIL: missing --to should error"
    exit 1
fi
echo "PASS: missing --to errors"

# 3. Non-existent manifest path should return exit 1.
if $CLI spirit hot-swap-precheck butler --from 0.3.1 --to /nonexistent/manifest.toml 2>/dev/null; then
    echo "FAIL: non-existent manifest should error"
    exit 1
fi
echo "PASS: non-existent manifest errors"

# 4. Valid invocation (one-shot stub) should succeed.
$CLI spirit hot-swap-precheck butler --from 0.3.1 --to spirits/hello-spirit/manifest.toml
echo "PASS: valid invocation succeeds"

echo "=== all hot-swap-precheck smoke tests passed ==="
