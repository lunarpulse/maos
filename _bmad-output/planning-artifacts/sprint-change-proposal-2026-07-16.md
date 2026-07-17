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

### 2.1 Epic impact — **no epic changes; this is BRIDGE work**

| Epic | Impact | Why |
|---|---|---|
| **Epic 13** (Reza Cortex v2.2, `backlog`, 6 stories) | **None** | Remit = tenant wall + cross-team sharing + FR37 + Reza scene. J1 is not Reza scope. Epic 13 completes as planned; it stays the critical path. |
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

**Two new standalone bridge stories** (Option A, operator-ratified), sequenced **after Epic 13 closes**, parallelizable with Epic 14:

- `j1-crosshost-1-loopback-developer-remote-delegation`
- `j1-crosshost-2-cross-host-signed-run`

No existing story changes scope. `j1-tier2-live-agent-signed-bridge` stays `done`.

### 2.5 Artifact conflicts

| Artifact | Conflict | Action |
|---|---|---|
| **PRD** | **[!]** The v1.0 milestone (`project-scoping-phased-development.md:180`) implies J1 cross-host shipped. It did not. | Add a **`[DELTA-2026-07-16]`** honesty note mirroring the J3 `[DELTA-2026-07-06]` shape. **Zero new FRs** (FR23a/FR23b already scope the A2A peer mesh, `user-journeys.md:355`). **MVP unaffected** — the v0.8 wedge demo is presentable today. |
| **Architecture** | None requiring a new ADR | ADR-014's four-protocol ceiling is honored (A2A is an existing protocol; **no fifth**). ADR-012 (typed-intent consent) + ADR-013 (two-level `task.assign`) are reused. One interaction to record: the founder-loop run path gains a transport-selected delegation route **through the existing `A2ARouter` port**. |
| **UI/UX** | N/A | No UX artifacts in this project. |
| **Epic 13 / Epic 14 docs** | **[!] NEW FINDING — stale baseline pins** | Epic 13 pins **23081**; Epic 14 pins **23141**. The actual pin is **23202** (post-J1-bridge, `xtask/kernel-core-baseline.toml`). Both need a repin note at their next preflight. *(Surfaced by this analysis; not caused by it.)* |
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
Add two standalone BRIDGE stories on the established precedent; no epic edits; no new epic.
- **Effort: Medium.** ~10/12 components already exist and are individually proven; the work is composition at the composition root + topology + a two-host audit artifact.
- **Risk: Low–Medium.** ZERO-kernel-Δ is settled. Residual risk is concentrated in rung 2 (mTLS/TOFU provisioning, two-host audit reconciliation) — which is precisely why Option A splits it out.
- **Rationale:** the bridge precedent (8.16 · j1-tier2) fits exactly — Epic-8/J1 debt, epic-external, dedicated branch, run between epics. Epic 13 (tenant-wall, critical path) is untouched; Epic 14's remit is not violated.

**Option 2 — Rollback. NOT VIABLE / N/A.** Nothing to revert. T6's local leg is correct, signed, and stays.

**Option 3 — PRD MVP Review. NOT VIABLE as a path.** MVP is unaffected (zero new FRs; the v0.8 wedge demo is presentable today). The PRD needs only an **honesty delta-note**, which is absorbed as a documentation edit inside Option 1 — not a replan.

**Why two rungs, not one** (operator-ratified Option A): rung 1 proves the *wire* with no network and admits a hermetic CI gate; rung 2 adds network/mTLS/two-host audit. Collapsing them bundles wiring risk + network risk + audit risk into one review — the exact shape that let **four fixture-masked gaps** survive undetected into T6 (admission · userns · stdin-EOF · `CODEX_API_KEY`). The through-line from that bridge stands: **a live-agent gate is only proven by a live agent.**

**Timeline impact:** none to Epic 13. The bridge runs after Epic 13 closes, parallelizable with Epic 14.

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

**`j1-crosshost-2-cross-host-signed-run`** — *v1.0 rung: two real hosts, mTLS, signed — the operator's model*

| Field | Value |
|---|---|
| Scope | Swap the loopback router for `TcpA2ATransport` + mTLS/TOFU across **two real hosts**; the remote daemon runs its **own** local `codex` with its **own** host-side credential; **two-host signed bundle** reconciling both Transparency Logs; human-signed release-gate artifact. |
| ACs | 6 |
| Model | frontier + **full §A6** (cross-host security + audit surface, non-degradable) |
| Kernel-Δ risk | **ZERO @23202** (transport + sealed-export are out-of-kernel) |
| Depends | **`j1-crosshost-1`** |

**AC sketch (finalize at preflight):**
1. Two real `maos` daemons on **two hosts**: the laptop Orchestrator's `task.assign` crosses `TcpA2ATransport` under **mTLS** to a **TOFU-pinned** peer; the **remote** daemon runs its **own local `codex`** through the T6-proven bridge and completes. This realizes `user-journeys.md:122` — *"Different host, different CLI, different provider, same protocol."*
2. **Credential isolation (negative):** the remote host injects its **own** `CODEX_API_KEY` **host-side**; the `task.assign` frame carries **no credential**. Proven by a negative over **both** TLs **and** the captured wire: `LIKE '%sk-%'` = **0**.
3. **TOFU:** a pin-mismatch peer is **REFUSED** and logged (the ratified v0.8 A2A floor: pin-mismatch 100/100 detected/rejected/logged); the run **fails closed**.
4. **Two-host signed bundle:** ONE verifiable artifact reconciling **BOTH** Transparency Logs (local Orchestrator + remote Worker), citing the **remote** worker's completion TL ref; `verify-bundle` **OK** against the operator pubkey; signed by the named human (Lunarpulse). **A single-host log presented as the whole run is a FAIL** (the explicit anti-over-claim control; cf. the deferred `FOLLOWUP-J1-RESUME-SEAM`).
5. Gate: the live cross-host leg is a **human-signed release-gate artifact, NEVER in CI**; rung 1's hermetic Tier-1 gate remains the CI blocker. Egress recorded **`declared-not-enforced`** + `FOLLOWUP-EPIC14-V2.0-PACKET-EGRESS-ENFORCEMENT` — never "enforced." The operator audit/signing key **never enters any sandbox** and stays distinct from the per-host LLM keys.
6. **ZERO kernel-Δ @23202**; FLAG-Winston only if two-host audit reconciliation surfaces a kernel seam (verify at preflight; expected NONE — `sealed-export` lives in `maos-cli`/`maos-audit`).

**Sequencing:** `j1-crosshost-1` → `j1-crosshost-2`. Both depend on the closed `j1-tier2-live-agent-signed-bridge`. The bridge runs **after Epic 13 closes**, on a dedicated branch, parallelizable with Epic 14.

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

- **Applied on approval (this proposal):** sprint-status bridge entries (4.4) · PRD delta-note (4.3) · release-gate pointer (4.5) · stale-baseline notes flagged for the epic preflights (4.6).
- **Next (create-story / preflight, when Epic 13 closes):** context-engineer `j1-crosshost-1` first, then `j1-crosshost-2`; each ≤6 ACs; both **frontier + full §A6** (delegation/consent + cross-host security/audit are non-degradable review surfaces per the E12-B6 reflex discipline).
- **Dependency gates:** `j1-crosshost-1` requires `j1-tier2-live-agent-signed-bridge` (**met** — done 2026-07-16). `j1-crosshost-2` requires `j1-crosshost-1`. The bridge itself waits on Epic 13's close (priority: tenant wall).
- **Not in scope:** no new protocol (ADR-014 ceiling); no change to the T6-proven local worker bridge (reused as-is on each host); no enforced egress (Epic-14 v2.0); no cohort/N-host mesh (that is J3/Reza) — this is a **bilateral** Orchestrator↔remote-Worker delegation.
- **Success criteria:**
  1. The Orchestrator delegates a real story-granularity `task.assign` to a `developer-remote` worker over A2A — **loopback (rung 1)**, then **cross-host mTLS on two real hosts (rung 2)**.
  2. The remote worker runs its **own local `codex`** and completes, kernel-mediated + host-granted.
  3. **No credential crosses the wire**; redaction = 0 over both TLs.
  4. A **two-host signed bundle** reconciles both Transparency Logs and `verify-bundle` = OK, human-signed.
  5. **ZERO kernel-Δ** — `check-kernel-baseline` PASS @23202.
  6. The PRD no longer implies J1 cross-host shipped.

---

*Course-correction workflow (`bmad-correct-course`) — checklist sections 1–6 worked; change scope **Moderate**; routed to PO/DEV. Input brief: `analysis-j1-cross-host-developer-remote-2026-07-16.md`. Key correction recorded in §2.2: the brief's Epic-14 placement recommendation was **refuted** by the epic doc's own remit; the BRIDGE precedent (8.16 · j1-tier2) is the correct home.*
