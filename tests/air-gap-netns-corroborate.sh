#!/usr/bin/env bash
# Story 9.4 R-AG3 — netns corroborating harness (CORROBORATING, NOT merge-blocking).
#
# Boots the air-gap maos binary under `unshare -n` (network namespace isolation),
# counts SYN attempts (should be zero), and runs a negative-control canary that
# DOES try to connect — proving the counter works.
#
# Requirements:
#   - Linux with `unshare(1)` (util-linux)
#   - Root or CAP_SYS_ADMIN (for CLONE_NEWNET)
#   - `ss` or `strace` for SYN counting
#
# This test is environment-fragile and MUST NOT be a merge-blocking gate.
# It is a corroborating signal only.

set -euo pipefail

BINARY="${1:-target/debug/maos}"
STRACE_LOG=$(mktemp /tmp/maos-netns-strace.XXXXXX)

cleanup() {
    rm -f "$STRACE_LOG"
}
trap cleanup EXIT

echo "=== R-AG3 netns corroborate ==="

# ── Step 1: verify unshare is available ─────────────────────────────────────
if ! command -v unshare &>/dev/null; then
    echo "SKIP: unshare not found (requires util-linux)"
    exit 0
fi

if ! command -v strace &>/dev/null; then
    echo "SKIP: strace not found"
    exit 0
fi

# ── Step 2: check permissions ───────────────────────────────────────────────
if ! unshare -n true 2>/dev/null; then
    echo "SKIP: unshare -n requires root or CAP_SYS_ADMIN"
    exit 0
fi

# ── Step 3: binary exists? ──────────────────────────────────────────────────
if [ ! -f "$BINARY" ]; then
    echo "SKIP: binary not found at $BINARY (build with --features air-gap first)"
    exit 0
fi

# ── Step 4: run air-gap binary under network namespace isolation ────────────
echo "Running air-gap binary under unshare -n..."
# strace the binary, looking for connect() / socket() syscalls
strace -e trace=network -o "$STRACE_LOG" \
    unshare -n "$BINARY" --version 2>/dev/null || true

SYN_COUNT=$(grep -cE '^(connect|socket)\(' "$STRACE_LOG" 2>/dev/null || echo 0)

echo "Network syscalls from air-gap binary: $SYN_COUNT"

if [ "$SYN_COUNT" -gt 0 ]; then
    echo "FAIL: air-gap binary attempted $SYN_COUNT network syscall(s)"
    echo "Syscalls found:"
    grep -E '^(connect|socket)\(' "$STRACE_LOG" || true
    exit 1
fi

echo "PASS: zero network syscalls from air-gap binary"

# ── Step 5: negative-control canary (proves counter works) ──────────────────
echo ""
echo "Running negative-control canary (curl under unshare -n)..."
CANARY_LOG=$(mktemp /tmp/maos-canary-strace.XXXXXX)
strace -e trace=network -o "$CANARY_LOG" \
    unshare -n curl -s --max-time 1 http://127.0.0.1:1 2>/dev/null || true

CANARY_SYN=$(grep -cE '^(connect|socket)\(' "$CANARY_LOG" 2>/dev/null || echo 0)
rm -f "$CANARY_LOG"

echo "Network syscalls from canary: $CANARY_SYN"

if [ "$CANARY_SYN" -eq 0 ]; then
    echo "WARN: canary produced zero network syscalls — strace may not be working"
    echo "CORROBORATING result: INCONCLUSIVE"
    exit 0
fi

echo "PASS: canary correctly detected $CANARY_SYN network syscall(s)"
echo ""
echo "=== R-AG3 netns corroborate: ALL PASSED ==="
