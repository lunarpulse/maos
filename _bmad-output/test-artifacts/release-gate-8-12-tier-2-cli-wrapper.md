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

## Tier-2 (signed artifact) — OPEN (Epic 8 Completion gate)

- [ ] **One real `claude` (or `opencode`) run through `maos run`** — NOT a
      bespoke harness — captured, archived, and **signed by a named owner**.
- [ ] The capture shows the morning **digest** and the **audit trail** proving
      citations trace to refs the *real* agent produced.
- [ ] Ideally the run spans a **halt/resume** (overnight founder loop).
- [ ] The live-CLI execution uses the **T3 network-permitted container variant**
      (Winston fallback) under a **host-granted** tier + egress allowlist;
      record **enforced-vs-declared** egress (full enforced egress allowlisting
      may be a follow-up — Cross-Impact #3).
- [ ] Credentials injected host-side into the sandbox env; **verify** no token
      lands in the Transparency Log (the AC2 redaction-trap, on the live wire).

**Named owner:** _<assign at Epic 8 Completion>_
**Signed artifact path:** _<archive location>_
**Date:** _<YYYY-MM-DD>_
