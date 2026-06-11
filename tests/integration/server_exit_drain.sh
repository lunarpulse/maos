#!/usr/bin/env bash
# Story 2.5 AC1 (A7 / D11) — verifies the cap-audit drain on graceful
# server exit: (a) one-shot path generates cap-audit rows and drains them
# deterministically to SQLite; (b) server path injects a synthetic
# cap-audit row, verifies it survives SIGTERM-driven drain, and confirms
# the drain block is reachable without crash or data corruption.
#
# The drain block is identical in both the one-shot and server arms of
# `crates/maos-bin/src/main.rs`, so the one-shot row-persistence test
# validates the drain logic itself, and the server SIGTERM test validates
# the reachability of the drain block in the long-running path AND that
# rows persisted before shutdown survive the drain.
#
# Wired into CI as `.github/workflows/discipline.yml::server-exit-drain`.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"

echo "::group::Build maos-bin (release, locked)"
cargo build -p maos-bin --release --locked --quiet
echo "::endgroup::"

MAOS_BIN="${REPO_ROOT}/target/release/maos"

# ── (A) One-shot drain: generate cap-audit rows, verify persistence ──
echo "::group::(A) One-shot drain: hello-spirit -> verify cap-audit rows persist"

# Story 8.14a migrated the binary's audit-DB resolution from the legacy
# `MAOS_AUDIT_DB` override (dropped from the env-var contract, see
# crates/maos-bin/src/env_contract.rs) to `${MAOS_HOME}/audit/transparency.sqlite`.
# Point the smoke at a throwaway MAOS_HOME and read the TL from the resolved path.
HOME_A="$(mktemp -d)"
DB_A="${HOME_A}/audit/transparency.sqlite"
cleanup_a() { rm -rf "$HOME_A"; }
trap cleanup_a EXIT

set +e
MAOS_HOME="$HOME_A" MAOS_ONE_SHOT="hello-spirit" "$MAOS_BIN" </dev/null >/dev/null 2>/dev/null
ONE_SHOT_RC=$?
set -e

if [ "$ONE_SHOT_RC" -eq 0 ]; then
    ROW_COUNT_A="$(sqlite3 "$DB_A" "SELECT COUNT(*) FROM transparency_log WHERE kind = 7;")"
    if [ "$ROW_COUNT_A" -lt 1 ]; then
        echo "FAIL: one-shot drain — expected >=1 CapabilityInvocation row, got ${ROW_COUNT_A}" >&2
        exit 1
    fi

    INFERENCE_COUNT="$(sqlite3 "$DB_A" "SELECT COUNT(*) FROM transparency_log WHERE kind = 9;")"
    if [ "$INFERENCE_COUNT" -lt 1 ]; then
        echo "FAIL: one-shot drain — expected >=1 InferenceCall row, got ${INFERENCE_COUNT}" >&2
        exit 1
    fi

    echo "one-shot drain: OK (${ROW_COUNT_A} cap-audit, ${INFERENCE_COUNT} inference rows persisted)"
else
    if [ -n "${ANTHROPIC_API_KEY:-}" ]; then
        echo "FAIL: one-shot drain — hello-spirit failed with exit code ${ONE_SHOT_RC} despite ANTHROPIC_API_KEY being set" >&2
        exit 1
    fi
    # Distinguish structural failures (segfault, missing binary) from
    # expected "no API key" exits. Exit codes 1-2 are typical Rust error
    # paths; higher codes signal crashes or signal deaths.
    if [ "$ONE_SHOT_RC" -gt 128 ]; then
        echo "FAIL: one-shot drain — exited with signal code ${ONE_SHOT_RC} (likely crash; not an API-key issue)" >&2
        exit 1
    fi
    echo "one-shot drain: SKIPPED (ANTHROPIC_API_KEY unset; exit code ${ONE_SHOT_RC} consistent with unconfigured provider)"
fi

rm -rf "$HOME_A"
trap - EXIT
echo "::endgroup::"

# ── (B) Server SIGTERM: inject synthetic row, verify drain + row persistence ──
echo "::group::(B) Server SIGTERM: verify drain block reachable + synthetic row survives"

# Story 8.14a repurposed the no-arg / Spirit-less invocation into the
# kernel-rendered shell — that IS the long-running serving surface now. It opens
# the Transparency Log and stays alive until SIGTERM, draining on graceful exit.
# A FIFO fed by a background `sleep` holds the shell's stdin open so it does not
# EOF and exit before we inject the synthetic row and signal it. Audit DB
# resolves under MAOS_HOME.
HOME_B="$(mktemp -d)"
DB_B="${HOME_B}/audit/transparency.sqlite"
STDERR_LOG="$(mktemp --suffix=.log)"
STDIN_FIFO="$(mktemp -u)"
mkfifo "$STDIN_FIFO"
sleep 120 >"$STDIN_FIFO" &
STDIN_HOLDER=$!
cleanup_b() {
    kill "$STDIN_HOLDER" 2>/dev/null || true
    rm -rf "$HOME_B" "$STDERR_LOG" "$STDIN_FIFO"
}
trap cleanup_b EXIT

MAOS_HOME="$HOME_B" NO_COLOR=1 "$MAOS_BIN" <"$STDIN_FIFO" >/dev/null 2>"$STDERR_LOG" &
SERVER_PID=$!

SERVER_READY=0
for _ in $(seq 1 20); do
    sleep 0.5
    if ! kill -0 "$SERVER_PID" 2>/dev/null; then
        echo "FAIL: server exited prematurely (pid $SERVER_PID)" >&2
        if [ -f "$STDERR_LOG" ]; then
            cat "$STDERR_LOG" >&2
        fi
        exit 1
    fi
    if grep -q "Transparency Log opened" "$STDERR_LOG" 2>/dev/null; then
        SERVER_READY=1
        break
    fi
done

if [ "$SERVER_READY" -ne 1 ]; then
    echo "FAIL: server did not reach ready state within 10s" >&2
    kill -9 "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true
    exit 1
fi

sleep 1

# Inject a synthetic CapabilityInvocation row directly into the DB while
# the server is running. This simulates a cap-audit event that was flushed
# to SQLite before SIGTERM. The drain block (which awaits the MPSC channel
# writer) is validated by Part A; Part B additionally verifies that rows
# already in the DB survive a graceful SIGTERM shutdown.
# The transparency_log schema gained redaction + IAC-frame columns since this
# smoke was written (Story 8.2 renamed `payload` → `payload_redacted` and the
# frame columns spirit_pid/boot_nonce/intent/origin are NOT NULL). Supply every
# NOT NULL column without a default so the synthetic insert matches the kernel's
# current on-disk schema.
sqlite3 "$DB_B" "INSERT INTO transparency_log (frame_id, timestamp_ns, spirit_pid, boot_nonce, kind, intent, payload_redacted, origin) VALUES ('synthetic-drain-test', strftime('%s','now') * 1000000000, 0, 0, 7, 'synthetic', x'00', 0);" \
    || { echo "FAIL: could not insert synthetic row into audit DB" >&2; kill -9 "$SERVER_PID" 2>/dev/null || true; wait "$SERVER_PID" 2>/dev/null || true; exit 1; }

PRE_SIGTERM_ROWS="$(sqlite3 "$DB_B" "SELECT COUNT(*) FROM transparency_log WHERE kind = 7;")"

kill -TERM "$SERVER_PID" 2>/dev/null || true

for _ in $(seq 1 20); do
    if ! kill -0 "$SERVER_PID" 2>/dev/null; then
        break
    fi
    sleep 0.5
done

if kill -0 "$SERVER_PID" 2>/dev/null; then
    echo "FAIL: server did not exit within 10s after SIGTERM" >&2
    kill -9 "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true
    exit 1
fi

wait "$SERVER_PID" 2>/dev/null || true

POST_SIGTERM_ROWS="$(sqlite3 "$DB_B" "SELECT COUNT(*) FROM transparency_log WHERE kind = 7;" 2>/dev/null)" || true

if ! sqlite3 "$DB_B" ".tables" >/dev/null 2>&1; then
    echo "FAIL: server SIGTERM — audit DB unreadable after shutdown" >&2
    exit 1
fi

if [ "${POST_SIGTERM_ROWS:-0}" -lt "$PRE_SIGTERM_ROWS" ]; then
    echo "FAIL: server SIGTERM — ${PRE_SIGTERM_ROWS} row(s) before SIGTERM but only ${POST_SIGTERM_ROWS:-0} after drain" >&2
    exit 1
fi

echo "server SIGTERM: OK (${POST_SIGTERM_ROWS} cap-audit row(s) persisted after drain)"
rm -f "$DB_B" "$STDERR_LOG"
trap - EXIT
echo "::endgroup::"

echo "server_exit_drain: PASS"
