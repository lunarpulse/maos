# Release Gate — Story 8.12 Tier-2: Live Agent-CLI Through `maos run` (J1)

> **AC6 two-gate "presentable."** Tier-1 (CI, hermetic) proves *the machine is
> real* — necessary, **not** sufficient. Tier-2 is a **hard precondition of
> "Epic 8 Completion,"** distinct from and **downstream of CI green**. This is a
> release-gate checklist item, **NOT** "dev done." Story 8.12 dev-complete =
> Tier-1 green; this gate stays OPEN until a named owner signs the Tier-2 run.

## Tier-1 (CI) — CLOSED by Story 8.12 dev (the machine is real)

- [x] `worker-cli-fixture` spawned as a REAL subprocess through the live
      `runtime.rs` bridge (not the deleted 8.4 hand-INSERT).
      Proof: `crates/maos-kernel-core/tests/cli_wrapper_bridge_8_12.rs`,
      `crates/maos-bin/tests/smoke_cli_wrapper_8_12.rs`.
- [x] Anti-theater: per-run nonce echoed by the child + real child PID in the
      journaled row + `child_pid != parent` + child reaped.
- [x] `ci_default` hermetic guard asserts zero network + no real agent CLI, AND
      its trip-test proves it fails on a real CLI / network request.
- [x] J1 IPC overhead measured through the real bridge (deterministic echo CLI):
      P50≈10µs / P95≈12µs / P99≈13µs / max≈16µs (N=120) — far under the §13.1
      25ms P95 budget and the 50ms generous-CI ceiling.

## Tier-2 (signed artifact) — CLOSED 2026-07-16 (signed by Myoungki Jung / Lunarpulse)

- [x] **One real `codex` run through `maos run`** — NOT a bespoke harness —
      captured, archived, and **signed by a named owner**. (Worker = `codex`, the
      ratified first live worker per the J1 preflight; `claude`/`opencode` stay as
      adapters proving the seam.) Observed: host-grant (T3) + liveness-probe
      admission → `codex exec` ran the real c2 task (`create ./main.rs` +
      `./NOTES.md`) in `$DEMO` → `exit 0`, adapter-parsed completion →
      `worker_completion completed=true completion_tl_ref=019f67ef…`.
- [x] The capture's `audit_refs` cite the **worker-produced TL ref** (`019f67ef…`);
      122 `CliSubprocessOutput` rows + `host_grant_disposition` + `worker_completion`
      journaled. **DELTA (honest):** the full halt/resume **referential-identity**
      digest oracle is DEFERRED — `FOLLOWUP-J1-RESUME-SEAM` (Story 9.6 never built
      the persist→resume→cite seam); the digest chain is present, the overnight
      referential-identity proof is not this run.
- [~] Halt/resume span: **continuous service + safe shutdown proven** (entered the
      serving loop; SIGINT drained clean); the full overnight halt/resume
      digest-citation is the deferred seam above.
- [x] Live-CLI under **host-granted T3** + egress allowlist. Egress recorded
      **`declared-not-enforced`** + follow-up `FOLLOWUP-EPIC14-V2.0-PACKET-EGRESS-ENFORCEMENT`
      (enforced egress = Epic-14 v2.0, Cross-Impact #3). NOT "enforced."
- [x] Credential injected host-side via env (`CODEX_API_KEY`, inherited by the
      child; MAOS never holds the value), **no ambient `~/.codex/auth.json`**.
      Redaction VERIFIED on the live wire: `SELECT count(*) … LIKE '%sk-proj-%'`
      over the TL = **0** — no token landed in the Transparency Log.

**Signed bundle:** `j1-tier2-evidence/j1-tier2-bundle.json` (capture doc archived alongside as `j1-tier2-evidence/j1-tier2-capture.json`) — sealed-export `--range 1d`, **247 entries**
(incl. `run.capture d301a233…` + worker completion `019f67ef…`), pubkey `61f4f495…`.
`verify-bundle` = **OK (247 entries, seq 1784161270937636202)** against the operator pubkey.

**SCOPE (honest — no over-claim):** this closes the **local leg** of J1 —
Orchestrator + Architect + Reviewer (class Spirits) + a real agent-CLI Worker,
all on **one host**, delegating through the kernel bridge with full audit + signing.
It does **NOT** exercise J1's **cross-host** "developer-remote" leg (laptop →
remote maos over A2A/mTLS), which the PRD scopes to **v1.0 cross-host** (v0.8
loopback-only, `user-journeys.md:326`) and which is a separate A2A-peer-mesh story,
not this Epic-8-debt bridge. The worker task was operator-set (`MAOS_WORKER_TASK`)
routed to the Worker, not Orchestrator-decomposed.

**Named owner:** Myoungki Jung (Lunarpulse) — named human signer
**Signed artifact path:** `_bmad-output/test-artifacts/j1-tier2-evidence/j1-tier2-bundle.json` (pubkey/FPR `61f4f495dba703e74aff7d42b4286a1a914a89b592a98bf76ed3656c81107766`; re-verify: `maosctl audit verify-bundle <path> --pubkey <FPR>`)
**Date:** 2026-07-16
