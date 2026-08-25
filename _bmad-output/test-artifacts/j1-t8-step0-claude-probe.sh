#!/usr/bin/env bash
# j1-crosshost-2d T8 — Step 0: the cheapest possible probe of the REAL claude adapter.
#
# WHY THIS EXISTS. The two-host choreography (runbook-j1-t8-two-host-paid-run.md §3) was verified
# end to end on 2026-08-22 with a FAKE `claude` fixture: the crossing, host B's spawn, a shared
# frame_id in both Transparency Logs, both signatures, and a green `reconcile-hosts`. Exactly ONE
# link was never exercised with real money: whether the real `claude`, under the argv posture
# `manifest-claude.toml` declares, actually WRITES THE FILE.
#
# That is the whole question. Run it single-host for cents before spending on two-host setup.
#
# It also exercises RELEASE-HOLDS row 16 by hand: claude's oracle is NOT an effect oracle, so
# `completed=true` does not imply an effect. This script checks the effect SEPARATELY.
#
# Usage:
#   export ANTHROPIC_API_KEY=<metered key>       # NOT a subscription token
#   ./_bmad-output/test-artifacts/j1-t8-step0-claude-probe.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

# ── Preconditions ────────────────────────────────────────────────────────────
[ -n "${ANTHROPIC_API_KEY:-}" ] || { echo "ABORT: ANTHROPIC_API_KEY unset (metered key required)"; exit 1; }
command -v claude >/dev/null || { echo "ABORT: 'claude' not on PATH"; exit 1; }
[ -x ./target/release/maos ] || { echo "ABORT: run 'cargo build --release -p maos-bin -p maos-cli' first"; exit 1; }

echo "=== claude version ==="; claude --version || true

# An isolated home. `refuse_ambient_auth` REFUSES the run if a subscription credential is visible
# in the sandbox home, so this must not inherit your real ~/.claude.
PROBE="$(mktemp -d)"; WORK="$PROBE/work"; mkdir -p "$WORK"
trap 'echo; echo "probe dir kept for inspection: $PROBE"' EXIT

if [ -e "$HOME/.claude/.credentials.json" ]; then
  echo
  echo "NOTE: $HOME/.claude/.credentials.json exists. This probe uses an ISOLATED HOME"
  echo "      ($PROBE) so it will not be inherited. The real two-host run must do the same."
fi

# ── The topology. j1-founder-loop.toml is pinned by two Blocking controls and MUST NOT be edited,
#    so we copy its shape and point the worker at the CLAUDE manifest. ────────
cat > "$PROBE/claude-probe.toml" <<EOF
[topology]
name = "claude-probe"

[[topology.spirits]]
manifest = "$ROOT/spirits/orchestrator/manifest.toml"

[[topology.spirits]]
manifest = "$ROOT/spirits/architect/manifest.toml"

[[topology.spirits]]
manifest = "$ROOT/spirits/reviewer/manifest.toml"

[[topology.spirits]]
manifest = "$ROOT/spirits/worker/manifest-claude.toml"
# `host` is LOAD-BEARING and omitting it silently breaks the probe. Without it no delegation
# frame is created, and the worker is spawned with the argv_prefix and NO TASK ARGUMENT — the
# fake-fixture run that caught this showed claude receiving only `--settings '<json>'` as its
# last argv element. With it, the loopback rehearsal fires and the worker receives
# argv_prefix ++ [task], where task is MAOS_DELEGATED_GOAL verbatim. Verified 2026-08-22.
host = "developer-remote-host"
EOF

# The host grant. `signing_key_id` MUST equal `[author].name` in manifest-claude.toml ("Anthropic").
cat > "$PROBE/grants.toml" <<'EOF'
[[grant]]
attested_image = "claude"
signing_key_id = "Anthropic"
permitted_tier = "T3"
permitted_egress_destinations = ["api.anthropic.com"]
EOF

GOAL='write a two-line haiku about a café into the file ./probe.txt in the current directory'

echo
echo "=== running the REAL claude worker (this spends money) ==="
echo "    goal : $GOAL"
echo "    cwd  : $WORK"
echo

set +e
( cd "$WORK" && \
  HOME="$PROBE" MAOS_HOME="$PROBE" XDG_DATA_HOME="$PROBE" \
  MAOS_HOST_GRANTS="$PROBE/grants.toml" \
  MAOS_LIVE_AGENT=1 \
  MAOS_DELEGATED_GOAL="$GOAL" \
  MAOS_OLLAMA_URL=skip \
  ANTHROPIC_API_KEY="$ANTHROPIC_API_KEY" \
  "$ROOT/target/release/maos" run "$PROBE/claude-probe.toml" --once ) \
  > "$PROBE/run.log" 2>&1
RC=$?
set -e

echo "=== verdict ==="
echo "  maos exit code : $RC"
grep -o '{"event":"worker_completion".*}' "$PROBE/run.log" | head -1 | sed 's/^/  /' || echo "  (no worker_completion event — read $PROBE/run.log)"
grep -o '{"event":"host_grant_disposition".*}' "$PROBE/run.log" | head -1 | cut -c1-200 | sed 's/^/  /' || true

echo
echo "=== EFFECT CHECK — the part claude's oracle does NOT do (RELEASE-HOLDS row 16) ==="
FILES="$(ls -A "$WORK" 2>/dev/null || true)"
if [ -z "$FILES" ]; then
  echo "  files written: NONE"
  echo
  echo "  >>> STOP. Even if worker_completion said completed=true, there is NO EFFECT."
  echo "  >>> claude's verdict proves only that no tool permission was DENIED."
  echo "  >>> Do NOT proceed to the two-host run; do NOT sign. Read the run log:"
  echo "  >>>   $PROBE/run.log"
  exit 2
fi
echo "  files written: $FILES"
for f in "$WORK"/*; do
  echo "  --- $(basename "$f") ---"; sed 's/^/    /' "$f"
done

echo
echo "=== permission posture ==="
if grep -q '"permission_denials":\[\]' "$PROBE/run.log"; then
  echo "  permission_denials: EMPTY (no tool call was refused)"
else
  grep -o '"permission_denials":\[[^]]*\]' "$PROBE/run.log" | head -1 | sed 's/^/  /' || \
    echo "  (not found in log — inspect manually)"
fi

echo
if [ "$RC" -eq 0 ]; then
  echo "PROBE PASSED — the real claude adapter produced a real effect."
  echo "Proceed to runbook-j1-t8-two-host-paid-run.md chapter 2 (keys, certs, cohort manifest)."
else
  echo "PROBE FAILED (exit $RC) — fix this before spending on two-host setup."
  echo "Log: $PROBE/run.log"
fi
