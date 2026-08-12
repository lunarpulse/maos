# Sprint Change Proposal — J1 cross-host `developer-remote` leg closed as a post-Epic-13 bridge

**Date:** 2026-07-16
**Author:** Paige (Technical Writer) running the course-correction workflow · with `{user_name}` Lunarpulse
**Change scope classification:** **Moderate** (backlog reorganization — two standalone BRIDGE stories on the 8.16 / j1-tier2 precedent + a PRD honesty delta-note + stale-baseline repins). **Not Major:** no PRD replan, **no new epic**, no rollback, zero new FRs, ZERO kernel-Δ.
**Mode:** Batch (analysis was completed in `analysis-j1-cross-host-developer-remote-2026-07-16.md`; this proposal presents the full set at once).
**Input brief:** `_bmad-output/planning-artifacts/analysis-j1-cross-host-developer-remote-2026-07-16.md`
**Operator decisions ratified at Step 1:** Batch mode; **Option A — two rungs** (loopback → cross-host).

---

## Section 1 — Issue Summary

**Trigger story:** `j1-tier2-live-agent-signed-bridge` (**done** 2026-07-16 — T6 signed + verified, Myoungki Jung/Lunarpulse). Its signed run closed J1's **local** leg; the release gate's own SCOPE clause (`release-gate-8-12-tier-2-cli-wrapper.md:53-60`) names the unclosed remainder as "a separate A2A-peer-mesh story."

**Issue type: scope discovery during planning** (not a technical limitation, not a failed approach, not a stakeholder pivot).

**Core problem.** J1's user journey has **two** worker legs (`user-journeys.md:116`: `workers: developer-local, developer-remote (laptop in office)`). T6 proved the local one. The **`developer-remote`** leg — *"the kernel's A2A adapter ferries the same-shape `task.assign` frame across mTLS … Different host, different CLI, different provider, same protocol"* (`:122`) — **was never wired into the founder-loop `maos run` path at any tier**, and **no epic or story schedules it**.

**Evidence (grounded; code + PRD, no speculation):**
- **Single-host, A2A-free J1 path:** `spirits/topologies/j1-founder-loop.toml` has four members, no `peer`/`a2a`/`remote` directive. The worker is spawned **in-process** (`crates/maos-bin/src/main.rs:917`, `BridgeSpawnSpec`). Every A2A entrypoint in `main.rs` (`run_cohort_a2a_daemon` ~7931; `smoke-a2a-*` 8165–8812) is a **separate `MAOS_ONE_SHOT` mode** the founder-loop path never dispatches into.
- **Two-level `task.assign` shortcut:** `main.rs:832-834` — `MAOS_WORKER_TASK` env → `worker_cli.argv(task)` → argv. The PRD requires Human→Orchestrator→Worker (`:151`, ADR-013); the upper level is bypassed. A *remote* worker cannot inherit an env var from a local parent process, so this shortcut is untenable for the remote leg.
- **A2A is BUILT, not missing:** crates `maos-a2a` / `-core` / `-tcp`; proven-in-isolation demos `smoke-a2a-loopback-6-3` (8349), `smoke-a2a-tcp-8-6` (8165, real mTLS handshake + wire), `smoke-a2a-consent-vocab-8-7` (8617), `smoke-a2a-fail-closed-8-8` (8812), `smoke-orchestrator-fanout-6-2` (7649); plus the shipped `cohort-a2a-daemon`.
- **Component readiness ~10/12** (brief §3.4). Only two genuine build items: founder-loop run-path wiring + two-host audit stitching.
- **PRD phase scoping:** v0.8 = J1 wedge with **A2A loopback-only** (`project-scoping-phased-development.md:144-152`); v1.0 = **"Cross-host A2A peer mesh — lifted from loopback-only"** (`:180`).

**Discovery context.** The gap surfaced when the operator asked whether the T6 demo depicts J1 as written ("laptop delegates to a remote maos that uses its local codex"). Code verification showed it depicts the **local composition only**. This mirrors the delta the PRD already recorded for J3 (`user-journeys.md:205`: *"the v1.0 tag became dishonest when … Epics 1–11 never picked it up"*) — **J1 carries the identical hole with no delta-note yet.**

---

## Section 2 — Impact Analysis

### 2.1 Epic impact — **no J1 scope is absorbed into an epic; independent Epic-13 hardening affects schedule**

| Epic | Impact | Why |
|---|---|---|
| **Epic 13** (Reza Cortex v2.2, `in-progress`, 8 stories after 2026-07-17 hardening) | **No J1 scope** | J1 remains bridge work. Separate evidence review added 13.5b/13.5c before Reza's closer, so “after Epic 13” now means after the hardened eight-story critical path. |
| **Epic 14** (v2.2 Hardening + Closers, `backlog`, 9 stories) | **None — and it must NOT absorb this** | See 2.2. |
| **Epics 1–12** | **None** | Done/merged. J1's lineage (Epic 8 story 8.12; A2A mesh Epic 6; task-assignment Epic 3) is historical. |
| **New epic** | **Not needed** | The BRIDGE precedent covers it (2.3). |

### 2.2 Why Epic 14 is the WRONG home (this corrects the input brief)

The brief's §6/OQ-4 floated Epic 14 as "the honest home." **A full read of the epic doc refutes that on four independent grounds** — recorded here because the analysis must be falsifiable:

1. **"Zero new PRD FRs"** is stated flatly in Epic 14's Objective (line 14). A cross-host delegation path is new functional surface.
2. **The three permitted modes** — *"scale-out of proven substrates + deferred-surface completion + honest constitutional accounting"* — exclude it. J1 cross-host is not an N-scale-out, and it is not on the enumerated deferred list (canary rollback, native mobile push, KMS backends, multi-provider, distro installers).
3. **The sweep list is CLOSED and enumerated** (14.4 operational · 14.5 backends/providers · 14.6 constitutional ceiling). The epic guards its own boundary by precedent — line 142: *"**Exception:** `loom-threat-model.md` is **NOT** here — it is an **Epic-13 prerequisite**."* Adjacent-seeming work gets routed to its rightful home, not absorbed.
4. **Kernel-Δ budget is ZERO across all 9**, justified per-story by out-of-kernel placement; there is no delta to spend and no seam contemplated beyond 14.6/14.7 leaf-crate placement.

**Verification:** a case-insensitive sweep of both epic docs for `J1|cross-host|developer-remote|task\.assign|maos run|founder-loop|delegat*` returns **zero matches** (control `J3|journey` returns 11 / 3 → grep proven live). A2A appears twice, both as substrate-reuse or load-generator, never as a delegation feature.

### 2.3 The correct home — the established BRIDGE pattern

`sprint-status.yaml:214-215` records the precedent verbatim:

> `# === PRE-EPIC-13 BRIDGE (8.16 precedent): close the J1 Tier-2 live-agent gate OPEN since 8.12. NOT Reza scope — Epic-8 debt run before Epic 13 opens, on a dedicated branch`
> `j1-tier2-live-agent-signed-bridge: done`

MAOS has a **two-instance precedent** for epic-external bridges that close journey/gate debt on a dedicated branch, tracked in `sprint-status` between epics: **Story 8.16** (pre-Epic-9 readiness bridge) and **`j1-tier2-live-agent-signed-bridge`** (pre-Epic-13, closed 2026-07-16). **The J1 cross-host leg is the direct continuation of the very bridge that just closed** — same Epic-8/J1 debt lineage, same shape, same rationale. This avoids a new epic (which would classify the change **Major**) and avoids amending Epic 14's remit.

### 2.4 Story impact

**Two standalone bridge stories** remain the planned path, sequenced after the hardened Epic 13 and parallelizable with Epic 14:

- `j1-crosshost-1-loopback-developer-remote-delegation`
- `j1-crosshost-2-cross-host-signed-run`

Rung 2 is hardened: it requires a heterogeneous real adapter/provider, duplicate-safe assignment lifecycle, explicit halt/indeterminate semantics, structural credential absence, and evidence-state truth. If the existing non-Codex adapter is not viable live at preflight, insert a focused adapter-enablement story rather than downgrade to Codex↔Codex while claiming provider independence. `j1-tier2-live-agent-signed-bridge` stays `done`.

### 2.5 Artifact conflicts

| Artifact | Conflict | Action |
|---|---|---|
| **PRD** | **[!]** The v1.0 milestone (`project-scoping-phased-development.md:180`) implies J1 cross-host shipped. It did not. | Add a **`[DELTA-2026-07-16]`** honesty note mirroring the J3 `[DELTA-2026-07-06]` shape. **Zero new FRs** (FR23a/FR23b already scope the A2A peer mesh, `user-journeys.md:355`). **MVP unaffected** — the v0.8 wedge demo is presentable today. |
| **Architecture** | None requiring a new ADR | ADR-014's four-protocol ceiling is honored (A2A is an existing protocol; **no fifth**). ADR-012 (typed-intent consent) + ADR-013 (two-level `task.assign`) are reused. One interaction to record: the founder-loop run path gains a transport-selected delegation route **through the existing `A2ARouter` port**. |
| **CLI/operator UX** | **[!]** | Rung 2 must distinguish `completed`, `halted`, and `indeterminate`, name the responsible host/authority, and prevent unsafe auto-retry after lost completion. |
| **Epic 13 / Epic 14 docs** | **[!]** | Epic 13 was re-pinned to **23202** at the 13.1 preflight and independently hardened to 8 stories. Epic 14 still carries **23141** and must repin at its next preflight. |
| **CI/CD** | **[!]** | The live/paid cross-host run **never enters CI** (inherits the T6 rule). A hermetic Tier-1 loopback leg with the fixture worker **can** gate (blocking). |
| **Release gate** | **[!]** | `release-gate-8-12-tier-2-cli-wrapper.md`'s SCOPE clause (53-60) already names this remainder — add a forward pointer to the new bridge so the gate doc stays the single truth. |
| **Docs/testing** | **[!]** | A two-host runbook (mirroring `runbook-j1-tier-2-signed-live-run.md`) + the two-host signed-bundle artifact. |

### 2.6 Technical impact

- **ZERO kernel-Δ — OQ-1 RESOLVED, proven by precedent** (brief §5.1). The baseline counts **only** `maos-kernel-core/src` (`kernel-core-baseline.toml:1`). The IAC mailbox that routes remote frames lives in **`maos-iac/src/adapter/mailbox.rs`** — it **already** holds `Option<Arc<dyn A2ARouter>>` and **already** routes `host_id.is_some()` frames (`maos-domain/src/ports/a2a.rs:4-9`). `maos-kernel-core` has **no a2a dependency**; a grep for `A2ARouter`/`route_outbound`/`CrossHostNotConfigured` in `maos-kernel-core/src` returns **nothing**. The router is installed at the **composition root** (`main.rs:8130-8137`, verbatim: *"maos-kernel-core receives NO new public fn (this lives entirely in the composition root)"*). The **cohort/J4 A2A daemon already ships this ZERO-Δ.**
- **Not a blocker:** `maos-kernel-core/src/isolation/runner.rs:143`'s `CrossHostUnsupported` is the sandbox-isolation corpus (Sec-14b, v0.3-β harness) — a separate concern, not on the `A2ARouter` path.
- **Carried-forward risks** (brief §5.2): two-host audit stitching (must not present one host's log as the whole run); no credential on the wire (each host injects its own host-side); ADR-012 allowlist authoring; mTLS/TOFU provisioning; egress stays `declared-not-enforced`.

---

## Section 3 — Recommended Approach

**Option 1 — Direct Adjustment. SELECTED.**
Add two standalone BRIDGE stories on the established precedent; no **J1-driven** epic edits and no new epic. Epic 13's separate 2026-07-17 evidence hardening changes only the bridge's earliest start.
- **Effort: Medium–High.** The transport pieces exist, but provider heterogeneity, correlation/idempotency, partition/halt semantics, structural secret absence, and a two-host signed artifact must compose through the real path.
- **Risk: Medium.** ZERO-kernel-Δ remains the expected transport posture, not a settled lifecycle result. Rung 2 must preflight the assignment/reconciliation seams and insert adapter enablement if no real non-Codex worker is viable.
- **Rationale:** the bridge precedent (8.16 · j1-tier2) still fits — Epic-8/J1 debt, epic-external, dedicated branch. The stricter contract prevents transport success from standing in for heterogeneous, duplicate-safe remote delegation.

**Option 2 — Rollback. NOT VIABLE / N/A.** Nothing to revert. T6's local leg is correct, signed, and stays.

**Option 3 — PRD MVP Review. NOT VIABLE as a path.** MVP is unaffected (zero new FRs; the v0.8 wedge demo is presentable today). The PRD needs only an **honesty delta-note**, which is absorbed as a documentation edit inside Option 1 — not a replan.

**Why two rungs, not one** (operator-ratified Option A): rung 1 proves the *wire* with no network and admits a hermetic CI gate; rung 2 adds network/mTLS/two-host audit. Collapsing them bundles wiring risk + network risk + audit risk into one review — the exact shape that let **four fixture-masked gaps** survive undetected into T6 (admission · userns · stdin-EOF · `CODEX_API_KEY`). The through-line from that bridge stands: **a live-agent gate is only proven by a live agent.**

**Timeline impact:** no J1 work enters Epic 13, but the bridge still waits for Epic 13's close; the independently hardened eight-story Epic therefore lengthens its earliest start. A conditional adapter-enablement story may add another rung before the signed run.

---

## Section 4 — Detailed Change Proposals

### 4.1 — New bridge story 1

**`j1-crosshost-1-loopback-developer-remote-delegation`** — *v0.8 rung: wire the founder-loop delegation over loopback A2A*

| Field | Value |
|---|---|
| Scope | Un-shortcut the two-level `task.assign`; give the founder-loop a `developer-remote` worker with a `host_id`; install `LoopbackA2ARouter` at the composition root; the existing `maos-iac` mailbox routes it; the routed worker runs real `codex` through the T6-proven bridge; new `check-j1-loopback-delegation` hermetic gate. |
| ACs | 6 |
| Model | frontier + **full §A6** (delegation/consent surface, non-degradable) |
| Kernel-Δ risk | **ZERO @23202** (routing seam in `maos-iac`; router install at composition root) |
| Depends | `j1-tier2-live-agent-signed-bridge` (done) |

**AC sketch (finalize at preflight):**
1. The founder-loop topology gains a **`developer-remote`** worker carrying a **`host_id`**; the Orchestrator emits a real **`task.assign` frame**. The `MAOS_WORKER_TASK` env shortcut is **DELETED from the delegation path** (`main.rs:832-834`), not merely bypassed — a grep-anchored control proves no env-inherited task reaches the routed worker.
2. The composition root installs a `LoopbackA2ARouter` as `Arc<dyn A2ARouter>`; the **existing** `maos-iac` mailbox `host_id.is_some()` branch routes the frame. **Negative control:** with **no** router installed the frame fails **`CrossHostNotConfigured`** — fail-closed, never a silent local-delivery fallback.
3. The loopback-routed worker runs **real `codex`** through the T6-proven bridge (host-grant T3 + adapter-aware liveness probe) and completes: exit 0, adapter-parsed completion, `CliSubprocessOutput` + `worker_completion` journaled to the TL.
4. **ADR-012:** the receiver's `accept_allowlist` admits `task.assign` / `development-task` (`user-journeys.md:122`); a **disallowed intent is REFUSED** (fail-closed negative, reusing the `smoke-a2a-fail-closed-8-8` shape).
5. Gate (**`check-j1-loopback-delegation`**): hermetic Tier-1 leg with `worker-cli-fixture` = **blocking, CI-safe**; **proven-red** — a planted "route locally anyway" regression must RED the gate; anti-canned (per-run nonce + real child PID, the 8.12 Tier-1 shape). The live-`codex` leg is `MAOS_LIVE_AGENT=1` **local-only, never CI**.
6. **ZERO kernel-Δ @23202** — `maos-kernel-core` receives no new public fn (`main.rs:8130-8137`); FLAG-Winston only if the Orchestrator's frame-emit surfaces a kernel API need (verify at preflight; expected NONE).

### 4.2 — New bridge story 2

**`j1-crosshost-2-cross-host-signed-run`** — *v1.0 rung: heterogeneous two-host delegation, mTLS, retry/halt truth, signed*

| Field | Value |
|---|---|
| Scope | Replace loopback with `TcpA2ATransport` + mTLS/TOFU across **two real hosts**; the remote daemon uses a **different real `WorkerCli` adapter/provider** from the T6 local Codex baseline; assignment/completion/halt state is correlated and duplicate-safe; credentials remain host-local; one human-signed bundle reconciles both Transparency Logs. |
| ACs | 6 |
| Model | frontier + **full §A6** (cross-host security, state-transition, and audit surfaces; non-degradable) |
| Kernel-Δ risk | **ZERO expected @23202**, but only after preflight proves assignment lifecycle + audit reconciliation remain in `maos-iac`/`maos-bin`/`maos-audit` |
| Depends | **`j1-crosshost-1`**; a viable real non-Codex adapter. `WorkerCli` currently has `CodexCli` and `ClaudeCli`; `FixtureCli` never satisfies provider heterogeneity. |

**AC sketch (finalize at preflight):**
1. **Heterogeneous real worker:** two real `maos` daemons on two hosts; the laptop Orchestrator's `task.assign` crosses `TcpA2ATransport` under mTLS to a TOFU-pinned peer, where a real non-Codex `WorkerCli` adapter/provider (currently `ClaudeCli` is the second implemented adapter) completes through the same protocol. Remote Codex or `FixtureCli` proves transport only and **does not** satisfy *"Different host, different CLI, different provider, same protocol."* If no live non-Codex adapter is viable at preflight, insert an enablement story and leave this story blocked—never downgrade the claim.
2. **Assignment lifecycle + duplicate safety:** one `task_id`/correlation ID is identical across Orchestrator, wire, remote worker, both TLs, and completion. Fault-inject disconnects before execution, during execution, and after completion-before-ACK; reconnect or duplicate delivery never executes the coding task twice and returns either the existing idempotent result or an explicit duplicate refusal.
3. **Halt and uncertainty semantics:** local halt, remote halt, network partition, and lost completion resolve to `completed`, `halted`, or `indeterminate` with journaled reason and safe next action. `indeterminate` is never displayed as completion and is never auto-retried until two-host audit reconciliation establishes whether the remote mutation occurred.
4. **Structural credential isolation:** `TaskAssign` and completion wire schemas contain no credential-bearing field. Each host injects its own provider credential host-side; no credential, auth file, bearer token, or serialized secret crosses A2A. Both TLs and the captured wire run provider-aware secret detection as secondary evidence; `LIKE '%sk-%' = 0` alone is insufficient.
5. **mTLS/TOFU + two-host signed truth:** pin mismatch is refused and journaled. One human-signed bundle reconciles both TLs under the same correlation ID and cites the remote completion; `verify-bundle` is OK. A single-host log, mismatched IDs, or missing remote artifact is a FAIL. The operator signing key never enters either sandbox.
6. **Evidence-state gate + boundary:** rung 1 remains `PROVEN_BLOCKING` in CI; the real paid run must be `PROVEN_LIVE_SIGNED`. An unperformed run is `ABSENT`; a lost/unreconciled completion is `INDETERMINATE`; neither can close J1. Egress stays `declared-not-enforced`. Verify ZERO kernel-Δ @23202; any required kernel seam triggers FLAG-Winston rather than an implicit re-pin.

**Sequencing:** `j1-crosshost-1` → optional real-adapter enablement only if preflight cannot run the existing non-Codex adapter → `j1-crosshost-2`. The bridge still starts after the now-hardened eight-story Epic 13 closes and remains parallelizable with Epic 14. The added adapter/lifecycle requirements raise rung-2 risk from Low–Medium to **Medium**; they do not change the protocol ceiling or Epic placement.

### 4.3 — PRD edit (honesty delta-note; zero new FRs)

**File:** `_bmad-output/planning-artifacts/prd/project-scoping-phased-development.md`, the **v1.0** phase (line ~180).

OLD (context):
```
- **Cross-host A2A peer mesh** with full mTLS + TOFU + ADR-012 typed-intent consent (lifted from loopback-only)
```

NEW:
```
- **Cross-host A2A peer mesh** with full mTLS + TOFU + ADR-012 typed-intent consent (lifted from loopback-only)
  **[DELTA-2026-07-16]** For **J1** this clause was **NOT delivered at v1.0**. The A2A
  mechanism shipped and is proven in isolation (smoke-6-3 loopback / smoke-8-6 cross-host
  mTLS / 8-7 consent / 8-8 fail-closed; cohort A2A daemon), but **no epic wired it into the
  founder-loop `maos run` path at any tier** — the T6 signed run (2026-07-16) closed J1's
  **v0.8 local composition only** (`release-gate-8-12-tier-2-cli-wrapper.md:53-60`). J1's
  cross-host `developer-remote` leg is retagged to the **post-Epic-13 J1 cross-host bridge**
  (`j1-crosshost-1` → `j1-crosshost-2`). Same correction shape as the J3
  `[DELTA-2026-07-06]` note. **Zero new FRs** — FR23a/FR23b already scope the peer mesh.
```

**Rationale:** the PRD currently implies J1 cross-host shipped at v1.0. It did not. This is the J1 analogue of the J3 delta the PRD already carries — an honesty annotation, not a replan.

### 4.4 — sprint-status.yaml edits

Insert **after** `epic-13-retrospective: optional` (mirroring the `j1-tier2` bridge placement between epics):

```yaml
  # === POST-EPIC-13 BRIDGE (8.16 / j1-tier2 precedent): close J1's cross-host developer-remote
  # leg — the v1.0 rung the PRD scoped (:180) that no epic wired into the founder-loop path.
  # NOT Epic-13 (Reza) scope; NOT Epic-14 (its remit is closed-enumerated sweep + zero new FRs).
  # Direct continuation of j1-tier2-live-agent-signed-bridge. Dedicated branch. ZERO-Δ @23202.
  j1-crosshost-1-loopback-developer-remote-delegation: backlog  # v0.8 rung — un-shortcut two-level task.assign (DELETE MAOS_WORKER_TASK from the delegation path); developer-remote worker w/ host_id; install LoopbackA2ARouter at composition root; EXISTING maos-iac mailbox host_id.is_some() branch routes it (negative: no router → CrossHostNotConfigured, fail-closed); routed worker runs REAL codex via T6-proven bridge; ADR-012 accept_allowlist admits task.assign/development-task + disallowed-intent REFUSED (smoke-8-8 shape); gate check-j1-loopback-delegation hermetic Tier-1 fixture leg BLOCKING + proven-red (planted "route locally anyway" must RED) + anti-canned nonce/PID; live codex leg MAOS_LIVE_AGENT local-only never CI. 6 ACs. frontier + full §A6 (delegation/consent). ZERO-Δ @23202 (routing seam in maos-iac; router install = composition root; main.rs:8130-8137 "maos-kernel-core receives NO new public fn"). Depends: j1-tier2-live-agent-signed-bridge (done).
  j1-crosshost-2-cross-host-signed-run: backlog  # v1.0 rung — TWO REAL HOSTS: laptop Orchestrator task.assign → TcpA2ATransport mTLS → TOFU-pinned remote maos daemon → its OWN local codex via T6-proven bridge (user-journeys.md:122 "different host, different CLI, different provider, same protocol"). Credential isolation negative: remote injects its OWN CODEX_API_KEY host-side, frame carries NO credential, LIKE '%sk-%' = 0 over BOTH TLs + wire. TOFU pin-mismatch REFUSED 100/100 + fail-closed. TWO-HOST SIGNED BUNDLE reconciling BOTH TLs citing remote worker completion ref; verify-bundle OK; human signer (Lunarpulse); single-host-log-as-whole-run = FAIL (anti-over-claim, cf. FOLLOWUP-J1-RESUME-SEAM). Live leg = release-gate artifact NEVER in CI; egress declared-not-enforced + FOLLOWUP-EPIC14-V2.0-PACKET-EGRESS-ENFORCEMENT; audit key never in sandbox. 6 ACs. frontier + full §A6 (cross-host security+audit). ZERO-Δ @23202. Depends: j1-crosshost-1.
```

Also update line 1:
```yaml
last_updated: '2026-07-16+epic-13-planning+j1-tier2-bridge-merged-T6-gate-CLOSED+j1-crosshost-bridge-scheduled'
```

### 4.5 — Release-gate forward pointer

**File:** `_bmad-output/test-artifacts/release-gate-8-12-tier-2-cli-wrapper.md`, SCOPE clause (53-60). Append one sentence so the gate doc stays the single truth:

> The cross-host leg is now scheduled as the post-Epic-13 J1 cross-host bridge
> (`j1-crosshost-1` → `j1-crosshost-2`, Sprint Change Proposal 2026-07-16).

### 4.6 — Stale-baseline repin notes (discovered, not caused, by this analysis)

Epic 13 pins **23081**; Epic 14 pins **23141**; the actual pin is **23202** (`xtask/kernel-core-baseline.toml`, post-J1-bridge +55 stdin-EOF fix, FLAG-Winston Lunarpulse 2026-07-15). **Action:** each epic repins to 23202 at its next preflight (Epic 14 already documents the pattern: *"the historical 23081 note predates the 12.5 re-pin"*). No story content changes; ZERO-Δ claims remain valid — only the pin number is stale.

---

## Section 5 — Implementation Handoff

**Scope: Moderate → Product Owner / Developer.**

- **Applied:** sprint-status bridge entries · PRD honesty delta · release-gate pointer · baseline repin note · 2026-07-17 evidence hardening for heterogeneous provider, task lifecycle, halt/uncertainty, structural credential isolation, and evidence states.
- **Next (create-story / preflight, when hardened Epic 13 closes):** context-engineer `j1-crosshost-1`, then verify a live non-Codex adapter and insert enablement only if required, then `j1-crosshost-2`; each story remains ≤6 ACs and frontier + full §A6.
- **Dependency gates:** `j1-crosshost-1` requires `j1-tier2-live-agent-signed-bridge` (**met**). `j1-crosshost-2` requires rung 1 plus a viable real non-Codex adapter. The bridge waits on the hardened Epic 13 close.
- **Not in scope:** no new protocol (ADR-014 ceiling); no change to the T6-proven local worker bridge (reused as-is on each host); no enforced egress (Epic-14 v2.0); no cohort/N-host mesh (that is J3/Reza) — this is a **bilateral** Orchestrator↔remote-Worker delegation.
- **Success criteria:**
  1. The Orchestrator delegates one correlated story-granularity `task.assign` over loopback, then mTLS/TOFU across two real hosts.
  2. The remote host uses a different real `WorkerCli` adapter/provider from the local Codex baseline; fixture or Codex↔Codex cannot close the provider-independence claim.
  3. Duplicate delivery, completion-ACK loss, reconnect, and halt never cause a second execution or a false completion; unresolved work is explicitly `INDETERMINATE`.
  4. Wire schemas carry no credential field; each host injects its own secret locally, and provider-aware scans over both TLs + wire find no credential material.
  5. One human-signed bundle reconciles both TLs, the shared correlation ID, and the remote completion; only `PROVEN_LIVE_SIGNED` closes rung 2.
  6. `check-kernel-baseline` passes @23202 or a named FLAG-Winston seam reopens preflight; the PRD never implies the cross-host value shipped before this evidence exists.

---

*Course-correction workflow (`bmad-correct-course`) — checklist sections 1–6 worked; change scope **Moderate**; routed to PO/DEV. Input brief: `analysis-j1-cross-host-developer-remote-2026-07-16.md`. Key correction recorded in §2.2: the brief's Epic-14 placement recommendation was **refuted** by the epic doc's own remit; the BRIDGE precedent (8.16 · j1-tier2) is the correct home.*
