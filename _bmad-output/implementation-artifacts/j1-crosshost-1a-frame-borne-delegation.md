---
baseline_commit: af788c3e
depends_on: j1-tier2-live-agent-signed-bridge (done 2026-07-16) — T6 signed the LOCAL leg
blocked_by: none — Epic 13 closed 2026-08-11
blocks: j1-crosshost-1b-consent-proofs-and-gate → j1-crosshost-2-cross-host-signed-run
split_from: j1-crosshost-1-loopback-developer-remote-delegation (SCP 2026-07-16 §4.1; split ratified by Lunarpulse 2026-08-14 at preflight)
kernel_grant: ZERO `maos-kernel-core/src` line Δ (pin **24472**, verified green) — BUT the public **API surface** grows by one method via the `pub use maos_iac::*` shim. See AC4.1; this needs an `abi-diff` ratification row, NOT a line-pin grant.
kloc_grant: **NONE REQUESTED — funded by reclaim, not by grant.** `xtask/src/example_spirit_regen.rs` is 133 tokei-code lines with zero callers; deleting it funds the gate skeleton outright. `maos-a2a-core` is at ZERO headroom and must not be touched. See Budget.
model: frontier-class {opus-4-8, gpt-5.5, glm-5.2, opus-5, equiv}
review: §A6 full-layer net (Blind + Edge + Acceptance + Test-Infra + runtime) — NON-DEGRADABLE (consent/A2A surface)
---

# j1-crosshost-1a — Frame-borne developer-remote delegation (mechanism)

Status: done — **review completed 2026-08-14**; one acceptance decision resolved, five patch
findings fixed, targeted tests and the Blocking gate green. The active customized workflow omitted
Test Infrastructure Auditor for this `anthropic/opus-5` dev pass by policy.

**Kernel-Δ: ZERO lines @ 24472 — but NOT zero ABI.** See AC4.1: `crates/maos-kernel-core/src/iac.rs:13`
is `pub use maos_iac::*;`, so the new `Mailbox` installer grows `maos-kernel-core`'s *public API*
while `src_lines` does not move. That axis is unmentioned in the ratified card and is this story's
single largest risk.

<!-- Model/review: frontier-class dev allowlist (E11 retro A1 / E12-B3). §A6 full-layer net
     NON-DEGRADABLE per E12-B6 — consent/A2A is the row epic-10-process-agreements.md:15 marks
     mandatory. NOTE: five story-file gates skip this filename (digit-prefix scoping) — §A6 is
     cultural, not mechanical, here. See AC4.5. -->

> **Why this story exists.** The SCP of 2026-07-16 §4.1 described this rung as wiring: *"the
> **existing** `maos-iac` mailbox `host_id.is_some()` branch routes it."* A six-scout preflight
> measured every clause against `af788c3e` and disproved twelve premises ([Appendix A](#appendix-a--the-ratified-sketch-refuted-measured-at-af788c3e)).
> The diagnosis holds — the founder loop really does inject its worker task by env var and never
> touches A2A — but the mechanism is substantially net-new. This story builds the mechanism.
> **`j1-crosshost-1b` proves it refuses.**

---

## Corrected facts — read this block, then the ACs

1. `MAOS_WORKER_TASK` is read at **`main.rs:1088-1089`**; const+doc at **`:938-944`**. `:832-834`
   is host-grant **egress** parsing — do not touch it.
2. `with_a2a_router` **consumes `self`** (`mailbox.rs:188`) and the mailbox is `Arc`'d at
   `main.rs:2560` — a `&self` installer is required, and **it moves kernel-core's ABI surface**.
3. There is **no production inbound path**; `install_intake_sink` is a documented *test-only hook*
   (`a2a-core/router.rs:344`). A pump is net-new and must strip `host_id` or recurse forever.
4. The delegation path is **not on the IAC bus at all**. The production `maos run` path builds the
   mailbox (`:2560`), gives it a TL (`:3148`) and a tracker (`:3248`), and **never uses it** — no
   `register_spirit`, no `deliver`, in any non-`smoke_*` code. **And nothing anywhere in the repo has
   ever drained a handle and acted on a frame**: even `smoke_orchestrator_fanout_6_2` binds all four
   handles to `_` (`:8991-9001`). The consumer is net-new with no template — see the AC3 preamble.
5. `Orchestrator::assign_frame` hardcodes `host_id: None` (`lib.rs:217`, `:242`) and
   `consent_envelope: None` (`:250`). **`lineage` is already a caller-supplied 5th parameter**
   (`:210`) — do not "add" it.
6. The topology parser reads only `manifest`/`path` (`main.rs:663-685`). A **`host` key already
   exists** in `bilateral-2-host-mira-nash.toml:17,22` — do not invent `host_id` as a second spelling.
7. **`task.assign` is not a legal consent intent** (`.` rejected by `i8.rs:85-115`). This story must
   use **`development-task:write-workspace`** or the frame fails closed at the sender before it
   ever routes. (The room widened this from plain `development-task` — see AC1.4 for why.)
8. `worker_completion` is a **`println!`** (`main.rs:1284-1296`), not a TL row.
9. Baseline is **24472**, not 23202. `kloc-check` is **RED at HEAD** on `maos-kernel-core`
   (pre-existing, not yours).

---

## Story

**As** Lunarpulse, running the founder's loop,
**I want** the Orchestrator to delegate a story to a `developer-remote` worker by emitting a real
`task.assign` frame that is routed through the loopback A2A layer and actually reaches a worker that
runs — instead of the worker inheriting its task from an environment variable,
**so that** J1's `developer-remote` leg has a working wire on the same protocol the v1.0 cross-host
rung will use.

---

## Acceptance Criteria (5)

### AC1 — The delegation path becomes frame-borne, and the env shortcut is deleted, not bypassed

1. **Topology typing.** `topology_manifest_entries` (`main.rs:663-685`) returns a typed entry
   instead of `Vec<String>`, parsing the **existing `host` key** (already shipped in
   `bilateral-2-host-mira-nash.toml:17,22`). **Do not introduce `host_id` as a second spelling.**
2. **Unknown keys are REJECTED, and `priority_weight` is removed from all three topologies.**
   *(Consensus 2026-08-14 — this was an open choice and is now closed to option (a).)*
   The dead keys are `j1-founder-loop.toml:6,10,14,24`, `j4-mira-nash.toml:6,10`,
   `bilateral-2-host-mira-nash.toml:16,21`. **Why this is not cosmetic scope creep:** the parser has
   never read `priority_weight`, so ordering has always been file order — which means **the signed
   J1 wedge demo has documented scheduling behavior that never once happened**, and the comment at
   `j1-founder-loop.toml:16` explains those weights to the reader in good faith. Strict rejection is
   the control that would have caught a false statement inside a signed artifact. Either consume the
   weights or delete them; do not leave them.
3. **Remote emit API.** Add a **new** `Orchestrator::assign_frame_remote(…, to_host, from_host, intent)`
   rather than changing `assign_frame`'s signature. `assign_frame` has **12 call sites across 4
   files** (`spirits/orchestrator/src/lib.rs:266,284,364,377`;
   `spirits/orchestrator/tests/distillate_dispatch.rs:133,159,197,229,248,276,298`;
   `spirits/architect/tests/code_review_loop.rs:136,177,223`) which must stay untouched. The new fn
   sets `to[..].host_id`, `from.host_id`, and `consent_envelope` via
   `ConsentEnvelope::with_fine_grained_intent(granter, intent)` where **`granter == frame.from`**
   (granter-binding check, `router.rs:1169-1206`).
4. **The intent is `development-task:write-workspace`.** *(Consensus 2026-08-14 — the story
   originally said `development-task`; the room widened it and the reason matters.)* ADR-012's own
   worked example is `diagnosis-handoff:read-only-evidence` versus `code-mutation-directive` —
   **neither names a verb; both name what the receiver is authorized to DO.** That is the
   confused-deputy axis the ADR exists for. `task:assign` describes the envelope and
   `development-task` describes a job category; neither states the authority. What is actually being
   granted here is a T3 worker running `codex exec --sandbox workspace-write` — arbitrary code
   execution and filesystem mutation on the receiver. **The most powerful grant in the system must
   not be named after a job title.** The namespaced form says out loud what a host consents to, which
   is what a rung-2 operator will read when deciding whether to admit it from a remote peer.
   Canonical under `i8.rs:85-115` (namespace:verb, one colon, kebab segments), and **net-new
   vocabulary** — zero code hits today. `task.assign` must not appear as a consent intent anywhere;
   it contains a `.` and would fail closed at `prepare_outbound` (`router.rs:696-701`) before
   routing. Land two pin-tests so the trap stays pinned:
   `A2AIntent::new("development-task:write-workspace").is_canonical()` is true,
   `A2AIntent::new("task.assign").is_canonical()` is **false**.
5. **`intent_lineage` is already a parameter** (`lib.rs:210`) — the requirement is a **non-empty**
   lineage whose **every entry is canonical** (`IntentLineage` is `Vec<A2AIntent>`, `i13.rs:34`).
   Empty lineage on a Spirit-emitted cross-Spirit frame is rejected `EIntentLineageBroken`
   (architecture §7.3.2).
6. **DELETED, verified by a grep control over `crates/` and `spirits/`:** the read
   (`main.rs:1088-1089`), the const **and its doc comment** (`main.rs:938-944`), the
   `env_contract.rs:419-423` row, and the `MAOS_WORKER_TASK` mention in
   `spirits/topologies/j1-founder-loop.toml:19`. **Ordering:** delete the *read* before the registry
   row — `check-env-contract` reds on unregistered reads, never on orphan rows
   (`check_env_contract.rs:186`).
7. **Regression reconciliation.** Both J1 tests run `maos run … --once` with no env set and rely on
   the deleted default: `crates/maos-journey-test/tests/journey_j1.rs:41`,
   `crates/maos-bin/tests/smoke_cli_wrapper_8_12.rs:116`. `journey_j1.rs:73-76` asserts
   `loaded_ids == {orchestrator, architect, reviewer}` **exactly**. Note `journey_j1.rs` today
   asserts **nothing** about the worker — reconciling `:73-76` is not enough; **add a positive
   assertion that the delegated worker ran and completed**, or J1 still proves nothing about the leg
   this story exists to build.

### AC2 — The router is installable on an `Arc`'d mailbox, and its absence fails closed

1. **`maos-iac` gains `Mailbox::install_a2a_router(&self, router: Arc<dyn A2ARouter>) -> Result<(), ()>`**
   over a `OnceLock`, mirroring `install_consent_gate` (`mailbox.rs:214`) and
   `install_transparency_log` (`:220`) exactly. `with_a2a_router` (`:188`) is retained or removed;
   if retained its single test caller (`:848`) stays green.
   **Do NOT route around the ABI growth** *(consensus 2026-08-14)*. The tempting dodge — build the
   router first and pass it through a constructor so no method is added — costs the *same* ABI growth
   and **loses a security property**: `OnceLock` is set-once, so nothing can swap the router after
   boot. Set-once is the invariant, not the ergonomic. Ratify the growth (AC5.1); don't hide it.
2. **Install site is `main.rs:3148`**, beside the existing `mailbox.install_transparency_log` — not
   the construction at `:2560`. Both are inside `async fn main()` (`:2372`), the same scope as the
   topology loop, so the site is reachable.
3. **Peer configs and TOFU are construction-only and are unspecified in the ratified card.**
   `LoopbackA2ARouter::new(peer_configs: Vec<A2APeerConfig>, tofu: Arc<dyn TofuPinStore>)`
   (`crates/maos-a2a/src/adapter.rs:47`). There is no post-construction peer add; each `peer_id`
   must equal the `HostId` string; each side needs `pin_first_contact` before use. **Copy the
   working template at `spirits/mira/tests/halt_bilateral.rs:305-313`** — one router instance must
   carry configs for **both** endpoints. On loopback, intake resolves the peer by
   `frame.from.host_id`, falling back to `HostId("loopback")` when `None` (`router.rs:1087-1090`).
4. **Negative control — fail-closed, never silent local delivery.** With no router installed, a
   `host.is_some()` frame returns `IacBusError::CrossHostNotConfigured` (`mailbox.rs:497-518`). The
   test asserts the error **and** that the target's mailbox received nothing.
5. **Two hardenings, or the negative passes vacuously.** (a) Phase 2 delivers same-host recipients
   *before* the Phase-3 error (`mailbox.rs:453-518`) — use a cross-host-only recipient or assert the
   partial explicitly. (b) `cross_host_targets` is **recomputed post-gate** at `mailbox.rs:427-429`;
   an installed `ConsentGate` that rejects the remote recipient makes the target vanish and returns
   **`Ok`** with no error (`:418-420`, cf. `consent_rupture_adr_034.rs:292-300`). Assert no consent
   gate is installed on the path under test.

### AC3 — The routed frame reaches a worker that actually runs, and the run is journaled

> **⚠ READ FIRST — there is no consumer precedent in this repo.** *(Added 2026-08-14 after a
> dev-readiness pass; the ACs below previously assumed one existed.)*
>
> **Nothing in MAOS has ever drained a mailbox handle and acted on the frame.** On the production
> `maos run` path the mailbox is built (`main.rs:2560`), given a TL (`:3148`) and a tracker
> (`:3248`), and then **never used** — no `register_spirit`, no `deliver`. Every such call in
> `main.rs` is inside a `smoke_*` arm (`:8992-9001`, `:9042-9149`, `:11320-11322`, `:11358`).
>
> And the closest template does **not** close the loop: `smoke_orchestrator_fanout_6_2` registers
> four spirits and binds **every handle to `_`** —
> `let _orchestrator = adapter.register_spirit_typed(…)` (`:8991-9001`). It proves frames are
> *delivered and journaled*. It never proves one is *received and acted on*.
>
> **Do not go looking for a consumer to copy. There isn't one. AC3.2 is where you build it.**

1. **Pump.** `install_intake_sink` → strip `to[..].host_id` → `mailbox.deliver`. Without the strip
   the frame re-enters the cross-host branch forever. **Constraint:** `install_intake_sink` lives in
   `crates/maos-a2a-core/src/router.rs:344`, and **`maos-a2a-core` is at ZERO kloc headroom**
   (4654/4654) — rewording its doc comment is free, adding code lines there reds `kloc-check` on
   contact. **Put the pump in `maos-bin`.**
2. **NEW COMPONENT — the delegation consumer.** This is the piece the ratified card, the analysis
   brief, and this story's first draft all omitted. Build it in `maos-bin`:
   - It owns the recipient's `SpiritMailboxHandle` returned by `register_spirit` — **bound to a real
     name, not `_`**. A `_`-bound handle is the 6.2 smoke's mistake and it silently drops the frame.
   - It drains the handle, extracts `TaskAssignPayload.goal` from the delivered `IacFrame`, and hands
     that string to `run_cli_wrapper_manifest` as its new task parameter (AC3.5).
   - **Why a translator is required at all:** the Worker is a **subprocess, not a mailbox peer.**
     `"worker"` is a plain string in `BridgeSpawnSpec.from_spirit_id` (`main.rs:1178`); it has no
     mailbox, no handle, and cannot be a delivery target. "Deliver the frame to the worker" is a
     category error. The frame is delivered to an **in-process consumer** that then *spawns* the
     worker with the payload's goal.
3. **Registered identities — use these exact strings, and keep `peer_id == HostId`:**

   | Role | Value | Where |
   |---|---|---|
   | Delegation recipient (registered spirit) | `developer-remote` | `register_spirit` in the consumer |
   | Frame `from` spirit | `orchestrator` | matches `Orchestrator::new(&spirit_id)` at `main.rs:3954` |
   | Topology `host` on the worker entry | `developer-remote-host` | `spirits/topologies/j1-founder-loop.toml` |
   | Sender peer_id (selects accept_allowlist at intake) | `founder-loop-host` | `A2APeerConfig` |
   | Destination peer_id (selects send_allowlist) | `developer-remote-host` | `A2APeerConfig` |

   `register_spirit` is load-bearing: Phase 1 `continue`s for `host.is_some()`
   (`mailbox.rs:303-306`) and `:463` `.expect("validated in phase 1")` depends on it, so an
   unregistered recipient errors or panics on delivery.
4. **Recursion guard, asserted directly:** a routed frame produces **exactly one** local delivery and
   the pump does not re-emit.
5. **Task threading, and the ordering is DECIDED — not left to the implementer.**
   `run_cli_wrapper_manifest` (`main.rs:947-955`) is a **7-arg synchronous** fn called at `:3861`
   and `:4216`; `worker_task` feeds `task_args` at `:1090`. Deleting the env read leaves it
   undefined, so add the delegated task as a parameter and update both call sites.
   **Ordering (decided): emit → route → pump → drain, ALL completed synchronously BEFORE the worker
   admit.** The consumer holds the goal string; `run_cli_wrapper_manifest` is then called with it,
   unchanged in its synchronous shape. **Do not** restructure the topology loop to `await` mid-call —
   that ripples through `:3861` and `:4216` and through every `[cli_wrapper]` admission path, for no
   gain at the loopback rung where the whole exchange is in-process anyway.
6. **Nothing drives the Orchestrator today — wire the trigger explicitly.** `Orchestrator::new` at
   `main.rs:3954` is its only production touch, and `on_idle` is inert (`lib.rs:101-109`, zero
   callers). The composition root calls `assign_frame_remote` directly, once per `host`-bearing
   topology entry, at topology-load time. **This story does not build an Orchestrator dispatch loop**
   — one delegation, driven by the composition root, is the v0.8 rung. Say so in the code comment so
   the next reader does not mistake the absence of a loop for an oversight.
7. **Bridge reuse, unchanged.** Host-grant T3 exact-match (`host_grant.rs:68-72`) + adapter-aware
   probe fork (`main.rs:1103-1130`). **No change to `spawn_and_bridge`** (`runtime.rs:449`) — it is
   `maos-kernel-core` and the line budget forbids it.
8. **Hermetic leg:** `worker-cli-fixture`, exit 0, adapter-parsed completion, `CliSubprocessOutput`
   rows carrying a per-run nonce and the real child PID (the 8.12 Tier-1 shape,
   `cli_wrapper_bridge_8_12.rs:20-27`, `:53-98`).
9. **Live leg** (`MAOS_LIVE_AGENT=1`, local-only, never CI): real `codex` reaches host-grant
   admission, liveness probe, bridge spawn, captured output, and adapter parsing. A successful
   `exit 0` / `completed=true` proof requires an operator credential and is explicitly assigned to
   `j1-crosshost-2-cross-host-signed-run` through
   `_bmad-output/test-artifacts/runbook-j1-tier-2-signed-live-run.md` Phase 3; it is not an
   acceptance condition for this mechanism-only story.
10. **`worker_completion` is journaled by emitting the EXISTING `TaskComplete` frame — not a new
   audit kind.** *(Consensus 2026-08-14; this reverses the story's first answer.)* It is a `println!`
   today (`main.rs:1284-1296`), so the SCP's "TL-journaled" is an overclaim. The story originally
   said to mint kind 32 via the `record-capture` raw-INSERT precedent
   (`crates/maos-cli/src/subcommands.rs:2452`). **Do not.** Two reasons:
   - **Nothing would read kind 32.** A write-only row is another claim standing in for a control.
   - **The precedent already shipped that defect.** `maos-audit::kind_to_string` does not map kind
     **21** (`CliSubprocessOutput`), which is why **159 of the 247 entries** in the signed T6 bundle
     render as `unknown`. Minting 32 the same way ships the same bug twice.

   `FramePayload::TaskComplete` and `TaskCompletePayload` **already exist** — Story 3.1 froze them
   (`crates/maos-domain/src/frame.rs`). The completion *is* a frame; emit the frame. It gets a
   `kind_to_string` mapping for free because it is a real `FrameKind`, not a literal int.
11. **Fix `kind_to_string` for kind 21 in this story.** One match arm in
   `crates/maos-audit/src/lib.rs` (mapping region ~`:658-716`). Without it the completion is legible
   and every `CliSubprocessOutput` row it cites still renders `unknown` — a legible verdict citing
   illegible evidence.

### AC4 — The gate skeleton lands HERE, funded by reclaim, and its one leg is the proven-red

*(Consensus 2026-08-14. The split originally left the whole gate to `1b` and gave `1a` only an
in-crate test. The room rejected that: a test that isn't behind a gate is a suggestion — it rots,
someone "fixes" a red assertion by relaxing it, and the routing regression walks through. That is
`smoke_cli_wrapper_8_12` with extra steps. Equally, requesting an `xtask` grant on an estimate would
break the exact rule the split was created to honor (`kloc.toml:60-65`). Both horns dissolved on
measurement — see AC4.2.)*

1. **`1a` lands `check-j1-loopback-delegation` as a registered, BLOCKING skeleton** —
   `BindingClass::Blocking` with `dev_enforced_red_blocks(BindingClass::Blocking, true)`, enrolled in
   **all seven surfaces** (enumerated in `1b`'s AC2.7; the enrollment work is `1a`'s, the additional
   legs are `1b`'s). `1b` then adds legs to a gate that already blocks, instead of standing one up in
   a green field.
2. **Funded by reclaim, not by grant — no grant is requested.**
   `xtask/src/example_spirit_regen.rs` is **133 tokei-code lines**, declared `mod example_spirit_regen;`
   at `xtask/src/main.rs:108`, exposing exactly one public item (`pub fn run`, `:12`) with
   **zero callers repo-wide** — no dispatch arm, no CI job, no test. Five of its functions have been
   emitting dead-code warnings on every build. **Delete it.** 133 reclaimed against a ~52-69 code-line
   skeleton leaves real margin, so `xtask` never approaches its ceiling and the measured-not-estimated
   rule is never bent.
   **Verification method (Murat's condition):** remove the `mod` line **first** and build. If it
   compiles, the module was unreachable and the delete is proven safe by the compiler rather than by
   grep.
3. **The skeleton's single leg IS the proven-red.** Not a smoke test, not a compile check. A planted
   "route locally anyway" regression must RED the gate. **If the one leg is anything else, we have
   registered an empty box in seven places and called it enrollment** — the exact failure shape this
   story exists to stop.
4. **Second leg, four lines, non-negotiable (Vex):** a leg asserting that on the loopback profile
   `frame.from.host_id` is **unverified** — recorded as a *boundary*, not a failure. `handle_intake`
   does not bind wire identity the way `handle_intake_verified` does (`router.rs:1478-1479`). When
   rung 2 turns verification on, this leg flips from "documented gap" to "now enforced" and the
   change is **visible in a CI diff** instead of buried in a story nobody re-reads.
5. **This story does not fix the dead-code class.** `#![deny(dead_code)]` on `xtask` plus
   `-D warnings` in CI would red today across ~12 further sites in live gates
   (`check_abi_ratification.rs:22`, `check_pentest_gate.rs:24`, `check_red_team_gate.rs:59`,
   `check_skill_conformance.rs:35`, `check_third_party_trial.rs` ×3, `check_epic_6_bridge.rs` ×3,
   `check_fkcs.rs` ×2, `evidence_ledger.rs` ×2). That triage belongs to **14-6** under D11 — see
   AC5.6. `1a` deletes only its own module, because that is what funds its gate.

### AC5 — Budget honesty on both kernel axes, and the D11 evidence is filed by ID

1. **Two different kernel axes — the ratified card conflates them.**
   (a) **Line pin:** `maos-kernel-core/src` stays at **24472**; `check-kernel-baseline` green.
   (b) **API surface:** `crates/maos-kernel-core/src/iac.rs:13` is `pub use maos_iac::*;`, and
   `xtask/kernel-api-classes.toml:264-268` classifies `maos_kernel_core::iac::mailbox::Mailbox::*`
   individually. **Adding `install_a2a_router` therefore grows `maos-kernel-core`'s public API even
   though `src_lines` does not move.** Add the `kernel-api-classes.toml` row and run **`abi-diff`**
   and **`check-abi-ratification`** before claiming green. Neither gate is mentioned in the SCP. If
   either demands ratification, that is the FLAG-Winston trigger — **stop and raise it; do not re-pin.**
2. **The `maos-iac` edit is authorized and named.** It is outside the line pin but it is **not**
   "composition root only"; say so in the record rather than repeating the SCP framing.
3. **kloc discipline.** `xtask` **nets negative**: −133 reclaimed (AC4.2) against a ~52-69 code-line
   skeleton. `maos-iac` has 40 spare (6848/6888) and `maos-bin` 151 (16027/16178) — the pump and
   threading go in `maos-bin`. **`maos-a2a-core` is at 4654/4654, ZERO headroom.** `spirits/` and
   `xtask/tests/` cost zero kloc. **No grant is requested by this story.**
4. **Do not absorb the pre-existing red.** `kloc-check` is RED at HEAD on `maos-kernel-core`
   (18933 > 18248) with **zero** working-tree contribution from this story's crates — the Epic-5
   closure's unrebased ceiling, **not in scope**. `maos-domain` 8694 > 8644 is a 50-line breach
   inside the +134 uncommitted Story-3.3 halt lines. **Measure before and after; attribute honestly.**
5. **§A6 is not mechanically enforced on this filename.** Five story-file gates skip non-digit names:
   `check-dev-model-tier` (`:103`), `check-dev-model-used-populated` (`:136-138`),
   `check-bare-review-findings` (`:35-37`), `check-dev-record-completeness` (`:240-250`),
   `check-review-findings-resolved` (`:50-63`). Operator decision 2026-08-14: **keep the `j1-` prefix
   and state the gap** rather than rename. Record the model and the review artifact anyway; a green
   CI does not mean the net ran. (Also note CI runs only `-p worker` (`discipline.yml:815`) and
   `-p maos-journey-test` (`:1869`) on this path — **a test in any other crate is dead in CI**, so
   AC4.3's proven-red must be reachable from the gate or from one of those two targets.)
6. **File the dead-code finding against D11 by ID — this is mandatory, not courtesy.**
   *(Consensus 2026-08-14.)* `epic-14-preflight-decisions.md` binding rule 1: *"No residual may be
   closed by implication. A row closes only when a decision is recorded against its ID with named
   evidence. **Shipping adjacent work does not close a row.**"* A note in this story's change log is
   *precisely* the forbidden move. Three obligations, all in
   `_bmad-output/planning-artifacts/epics/epic-14-preflight-decisions.md`:
   - **Evidence, not closure.** D11 stays **open**, owners Winston + Murat, deadline unchanged
     (before `14-6` leaves `backlog`). Substance is settled at 14-6's preflight, not here.
   - **D11(b) is widened** from *"in-`src` `#[cfg(test)]` modules are KLOC-budget-charged but never
     CI-executed"* to *"budget-charged code with no execution path"*. `example_spirit_regen` is
     **not** a `#[cfg(test)]` module — it is ordinary production code — so it is not another instance
     of D11(b); it is proof D11(b) was **scoped around the instance instead of the category** and
     would never have caught this.
   - **D11's numbers are corrected.** The row says *"36 entries vs 66 `check_*.rs`"*; HEAD measures
     **67**, and this story takes it to **37 / 68**. C1's single-source rule applies to the decision
     queue itself — a mechanical deadline queried against numbers nobody updated is theatre.

   Also filed in the same packet, **not fixed by anyone here**: `discipline.yml:44` installs `clippy`
   as a toolchain component and **no workflow ever invokes `cargo clippy`** — an instrument that is
   not an instrument, same species as the dead module. If it belongs to Epic 0 rather than D11,
   14-6's owner re-homes it. That is a routing call, not this story's.

---

## Traps

1. **`cargo run -q -p xtask -- <cmd>`.** There is no `cargo xtask` alias.
2. **Do not delete `main.rs:832-834`** — host-grant egress parsing. The target is `:1088-1089`.
3. **`task.assign` as a consent intent is un-admittable.** Use `development-task:write-workspace`.
3b. **Do not mint a new audit kind for the completion.** `TaskComplete` already exists; a write-only
    kind 32 is another null control, and kind 21 already renders `unknown` in signed bundles.
3c. **Delete the dead module by removing its `mod` line FIRST and building.** Compiler-proven, not
    grep-proven — that was the condition on which the reclaim was accepted.
3d. **Do not close D11.** File evidence against the ID; the owners settle substance at 14-6.
4. **`with_a2a_router` cannot be called on an `Arc`** — and the `&self` installer grows kernel-core's
   **API surface** through the `pub use maos_iac::*` shim even at a flat line count.
5. **The intake sink is a documented test-only hook**, and its crate has **zero** kloc headroom.
   Put the pump in `maos-bin`.
6. **`consent_envelope: None` fails closed at the sender.** Every production IAC frame builder sets
   it to `None` (`maos-iac/src/adapter.rs:725,863,948,1004`). Yours must not.
7. **`lineage` is already an `assign_frame` parameter.** Add `assign_frame_remote`; do not change the
   existing signature — 12 call sites across 4 files depend on it.
8. **A `host` key already exists** in `bilateral-2-host-mira-nash.toml:17,22`. Do not add `host_id`.
9. **Strict unknown-key parsing reds all three topologies** (dead `priority_weight`), not just J1's.
10. **`run_cli_wrapper_manifest` is synchronous and 7-arg**, called inside the topology loop. It
    cannot await a pumped frame.
11. **The `maos run` path calls neither `register_spirit` nor `deliver`** — both are net-new.
11b. **There is NO frame-consumer precedent in the repo.** `smoke_orchestrator_fanout_6_2` is a
     *delivery* template only — it `_`-binds every handle. Do not spend a day hunting for a consumer
     to copy; AC3.2 is where you write the first one.
11c. **A `_`-bound `SpiritMailboxHandle` silently drops the frame.** Bind it to a real name.
11d. **The Worker is a subprocess, not a mailbox peer.** `"worker"` is a string in
     `BridgeSpawnSpec.from_spirit_id` (`main.rs:1178`). You cannot address a frame to it; the
     consumer receives the frame and *spawns* the worker with the payload's goal.
11e. **Ordering is decided, not open:** emit → route → pump → drain, all synchronous, all **before**
     the worker admit. Do not restructure the topology loop to `await` mid-call.
12. **`worker_completion` is not a TL row.** Build it (AC3.8) or the claim is false.
13. **`journey_j1.rs:73-76` will red**, and `journey_j1.rs` asserts nothing about the worker today.
14. **`maos-bin`'s `worker_cli` module is binary-private** (`main.rs:45`). Promote to `pub mod` in
    `crates/maos-bin/src/lib.rs` before an integration test can call it.
15. **`--features network` is not a CI/live split.** It is default-on; the split is the runtime
    `MAOS_LIVE_AGENT` check at `main.rs:1066`.
16. **The working tree is dirty** with Story-3.3 halt work. Establish a clean baseline measurement
    before attributing any kloc movement.
17. **`attested_image` is the pre-resolution command name** (`main.rs:994`); `config.command` is
    overwritten with the absolute path at `:1029-1030`.
18. **`LoopbackA2ARouter::route_outbound` hard-codes `boot_nonce = 0`**, and the Liveness probe arm
    (`main.rs:1108-1130`) skips `admit_cli_wrapper_journaled` — **no TL admission row on the
    real-CLI path**.
19. **`ConsentEnvelope::with_fine_grained_intent` never expires** — `timestamp_ns = 0`,
    `valid_until_ns = None` (`maos-domain/src/frame.rs:447-450`). State it or set a TTL.
20. **A test not run by CI is not a control.** CI runs `-p worker` and `-p maos-journey-test` here.

---

## Tasks

- [x] **T1 (AC1.1-1.2)** — Typed topology entry parsing the existing `host` key; decide and document
      the unknown-key policy across all three topology manifests.
- [x] **T2 (AC1.3-1.5)** — `Orchestrator::assign_frame_remote` (NEW fn, 12 existing call sites
      untouched): `to.host_id`, `from.host_id`, `consent_envelope` with granter == from; canonical
      non-empty lineage; both `development-task:write-workspace` / `task.assign` canonicality
      pin-tests.
- [x] **T3 (AC1.6)** — Delete `main.rs:938-944`, the read at `:1088-1089`, `env_contract.rs:419-423`,
      and `j1-founder-loop.toml:19`. Read-first, row-second. Grep control.
- [x] **T4 (AC1.7)** — Update `journey_j1.rs:41`, `:73-76` and `smoke_cli_wrapper_8_12.rs:116`; add a
      positive worker-ran/completed assertion to `journey_j1.rs`.
- [x] **T5 (AC2.1)** — `maos-iac`: `Mailbox::install_a2a_router(&self, …)` over a `OnceLock`,
      mirroring `mailbox.rs:214` / `:220`. **Read AC4.1 first.**
- [x] **T6 (AC2.2-2.3)** — Composition root at `main.rs:3148`: peer configs for **both** endpoints +
      `TofuPinStore` + `pin_first_contact` (template `halt_bilateral.rs:305-313`) + install.
- [x] **T7 (AC2.4-2.5)** — Fail-closed negative with both hardenings.
- [x] **T8 (AC3.1, 3.4)** — Pump in `maos-bin`: intake sink → strip `to[..].host_id` → `deliver`;
      recursion-guard test asserting **exactly one** local delivery and no re-emit.
- [x] **T8b (AC3.2-3.3)** — **NEW COMPONENT: the delegation consumer.** Owns the
      `developer-remote` `SpiritMailboxHandle` (real binding, never `_`), drains it, extracts
      `TaskAssignPayload.goal`. No precedent exists — this is first-of-kind. Register the identities
      from AC3.3's table and keep `peer_id == HostId`.
- [x] **T9 (AC3.5-3.6)** — Thread the goal into `run_cli_wrapper_manifest` (new param); update call
      sites `:3861` and `:4216`. Keep the fn **synchronous** and complete emit→route→pump→drain
      before the worker admit. Call `assign_frame_remote` from the composition root at topology-load
      time, once per `host`-bearing entry, with a comment saying the absence of a dispatch loop is
      deliberate at v0.8.
- [x] **T10 (AC3.6-3.9)** — Hermetic + live legs; emit the existing **`TaskComplete`** frame (NOT a
      new audit kind); fix `kind_to_string` for kind 21 in `crates/maos-audit/src/lib.rs`.
- [x] **T11 (AC4.2)** — Delete `xtask/src/example_spirit_regen.rs` + its `mod` line at
      `xtask/src/main.rs:108`. **Remove the `mod` line first and build** — compiler-proven, not
      grep-proven. Record the reclaimed code-line count.
- [x] **T12 (AC4.1, 4.3-4.4)** — Stand up `check-j1-loopback-delegation` as a BLOCKING skeleton,
      enrolled in all seven surfaces, with **one leg = the proven-red** ("route locally anyway" must
      RED) plus Vex's unverified-`from.host_id` boundary leg.
- [x] **T13 (AC5.1-5.4)** — Before/after `check-kernel-baseline`, **`abi-diff`**,
      **`check-abi-ratification`**, `kloc-check`; add the `kernel-api-classes.toml` row; attribute
      movement honestly; FLAG-Winston on any ratification demand.
- [x] **T14 (AC5.6)** — File the D11 amendment in
      `_bmad-output/planning-artifacts/epics/epic-14-preflight-decisions.md`: evidence not closure,
      widen D11(b) to "budget-charged code with no execution path", correct 36/66 → 37/68, add the
      clippy-never-invoked note. **Do not close the row.**
- [x] **T15** — On completion: `sprint-status.yaml` row → `done`; hand the remaining gate legs +
      consent negatives to `j1-crosshost-1b`.

### Review Findings

- [x] [Review][Decision] Move successful live Codex completion proof forward — resolved by Lunarpulse: AC3.9 now accepts the documented real-Codex admission/spawn/captured-output/adapter-parse evidence for Story 1a and assigns `exit 0` / `completed=true` proof to `j1-crosshost-2-cross-host-signed-run` through the Phase 3 runbook.
- [x] [Review][Patch] Generate delegation frame IDs uniquely across persistent runs [crates/maos-bin/src/main.rs:3831] — fixed: sequence occupies the low 8 bytes and the per-run `boot_nonce` the high 8; a same-sequence/two-run regression test inserts both IDs into one log.
- [x] [Review][Patch] Reject or remotely route host-bearing class topology entries instead of loading them locally [crates/maos-bin/src/main.rs:3869] — fixed: unsupported host-bearing non-`[cli_wrapper]` entries fail loud before local scheduler loading.
- [x] [Review][Patch] Enforce non-empty, canonical lineage in `assign_frame_remote` [spirits/orchestrator/src/lib.rs:306] — fixed: the API returns `RemoteFrameError` before recording a dispatch; empty and non-canonical cases are pinned.
- [x] [Review][Patch] Do not emit `TaskComplete` or reopen the safe point for a non-completed worker outcome [crates/maos-bin/src/main.rs:1285] — fixed: the worker returns a typed oracle outcome; only `Completed` reaches the parameter-free success journaler.
- [x] [Review][Patch] Make the blocking fail-closed gate inspect the production absent-router branch rather than any file-wide error-name occurrence [xtask/src/check_j1_loopback_delegation.rs:169] — fixed: the oracle scopes both absent-router and dotted-intent checks to production surfaces; test-only decoys stay green while production plantings red.

---

## Dev Notes

### Budget — read before writing code

kloc counts tokei **`code`** lines and **excludes** `tests/`, `benches/`, `examples/`, `spirits/`
(`xtask/src/kloc_check.rs:167-190`). Do not budget in raw line counts.

| Instrument | Ceiling / pin | Measured at HEAD | State |
|---|---|---|---|
| `check-kernel-baseline` (`maos-kernel-core/src`) | 24472 | 24472 | **GREEN — hold it** |
| `abi-diff` / `check-abi-ratification` | — | — | **Will see the new `Mailbox` method (AC4.1)** |
| kloc `maos-a2a-core` | 4654 | 4654 | **ZERO headroom — do not add code to `router.rs`** |
| kloc `maos-iac` | 6888 | 6848 | 40 spare — the installer is small; watch it |
| kloc `maos-bin` | 16178 | 16027 | 151 spare — pump + task threading live here |
| kloc `maos-a2a` | 1500 | 226 | ample |
| kloc `xtask` | 37287 | 37023 | **nets NEGATIVE** — −133 reclaimed vs ~52-69 skeleton; no grant |
| dead module (reclaim source) | — | 133 code lines, 0 callers | `xtask/src/example_spirit_regen.rs` — delete |
| kloc `maos-kernel-core` | 18248 | 18933 | **RED, pre-existing — not this story's** |
| kloc `maos-domain` | 8644 | 8694 | RED; inside the +134 uncommitted Story-3.3 lines |
| `spirits/`, `xtask/tests/` | — | — | **zero kloc cost** |

The pin and the ceiling are different mechanisms (`kloc.toml:54-56`): the **pin is anti-drift**; the
**ceilings are anti-growth**.

### Where the code goes

| Concern | File | Anchor |
|---|---|---|
| Topology typing + `host` key | `crates/maos-bin/src/main.rs` | `topology_manifest_entries` :663-685 |
| Env shortcut deletion | `crates/maos-bin/src/main.rs` | :1088-1089, :938-944 |
| Env registry row | `crates/maos-bin/src/env_contract.rs` | :419-423 |
| Remote frame emit | `spirits/orchestrator/src/lib.rs` | new fn beside `assign_frame` :204-252 |
| Router installer | `crates/maos-iac/src/adapter/mailbox.rs` | field :125, models :214 / :220 |
| Router install + peers + TOFU | `crates/maos-bin/src/main.rs` | **:3148** |
| Pump (here, not a2a-core) | `crates/maos-bin/src/main.rs` | — |
| **Delegation consumer (NEW, no precedent)** | `crates/maos-bin/` | owns the `developer-remote` handle; drains; extracts `goal` |
| Frame-emit trigger | `crates/maos-bin/src/main.rs` | composition root, topology-load, per `host`-bearing entry |
| Task threading | `crates/maos-bin/src/main.rs` | `run_cli_wrapper_manifest` :947-955; sites :3861, :4216 |
| Loopback router ctor | `crates/maos-a2a/src/adapter.rs` | :47 (peers + TofuPinStore) |
| Canonicality grammar | `crates/maos-domain/src/invariants/i8.rs` | :85-115 |
| Fail-closed error | `crates/maos-domain/src/iac_bus_types.rs` | `CrossHostNotConfigured` :25-31 |
| ABI classification row | `xtask/kernel-api-classes.toml` | :264-268 |

### Round-table consensus 2026-08-14

Seven decisions the room settled after this story was drafted. Each reversed or tightened something
the story already said — they are folded into the ACs above, and listed here so a reviewer can see
what changed and why.

| # | Decision | Reverses |
|---|---|---|
| 1 | Ratify the ABI growth; **`OnceLock` is a security property** (set-once router), not an ergonomic. Do not dodge via a constructor. | AC2.1 |
| 2 | Unknown keys **rejected**; `priority_weight` removed from all three topologies. The signed J1 demo documents scheduling that never happened. | AC1.2 (option (b) withdrawn) |
| 3 | Intent is **`development-task:write-workspace`** — ADR-012 names *effect authority*, not verbs or job categories. | AC1.4 |
| 4 | Emit the existing **`TaskComplete`** frame; do **not** mint audit kind 32. Fix `kind_to_string` for kind 21 in the same story. | AC3.8 |
| 5 | **The gate skeleton lands in `1a`**, not `1b`, with one leg = the proven-red. A test not behind a gate is a suggestion. | the original split seam |
| 6 | Funded by **deleting 133 dead `xtask` lines**, not by a grant on an estimate. No rule bent. | AC5.3 |
| 7 | Dead-code finding **filed against D11 by ID**; D11(b) widened to "budget-charged code with no execution path". Change-log-only was the forbidden option. | AC5.6 |

Open, deliberately: whether clippy-never-invoked belongs to D11 or Epic 0 — 14-6's owner routes it.

### What this story does NOT do

- **No allowlist admit/refuse negatives** (`-32001`, `-32009`) — `j1-crosshost-1b`. This story
  *defines and uses* the `development-task:write-workspace` vocabulary and stands the gate up;
  1b *proves the refusals* by adding legs.
- **No fix for the `xtask` dead-code class** — ~12 further sites; that triage is 14-6's under D11.
- **No `#![deny(dead_code)]` / `-D warnings`** — proposed as D11's mechanical form, not built here.
- **No new protocol** — ADR-014's four-protocol ceiling holds.
- **No change to `spawn_and_bridge`** — `maos-kernel-core`, reused as-is.
- **No mTLS, no cross-network TOFU, no second host** — `j1-crosshost-2`.
- **No enforced egress** — stays `declared-not-enforced` (`FOLLOWUP-EPIC14-V2.0-PACKET-EGRESS-ENFORCEMENT`).
- **No cohort / N-host mesh** — this is a **bilateral** Orchestrator↔Worker delegation.
- **No `task_id` correlation / idempotency / `indeterminate` state machine** — rung 2 owns those.
  Note `analysis-maos-coding-delegation-ux-2026-07-17.md:434-446` argues a stable `task_id` minted
  before admission is a *prerequisite* for rung 2's correlation AC; if neither 1a nor 1b lays it,
  rung 2's preflight must.
- **No fix for the pre-existing `maos-kernel-core` kloc red.**

### References

- [Source: `_bmad-output/planning-artifacts/sprint-change-proposal-2026-07-16.md#4.1`] — the ratified card this story corrects.
- [Source: `_bmad-output/planning-artifacts/analysis-j1-cross-host-developer-remote-2026-07-16.md#5.1`] — OQ-1 ZERO-Δ chain (line-pin axis only; misses the ABI axis).
- [Source: `_bmad-output/planning-artifacts/prd/user-journeys.md#122`] — *"Different host, different CLI, different provider, same protocol."*
- [Source: `_bmad-output/planning-artifacts/prd/user-journeys.md#326`] — v0.8 loopback-only → v1.0 cross-host.
- [Source: `_bmad-output/planning-artifacts/prd/functional-requirements.md#FR23a`] — loopback A2A + corpus floors.
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/7-inter-agent-communication.md#7.2`] — fail-closed-on-unclassified.
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/7-inter-agent-communication.md#7.3.2`] — `intent_lineage` / `EIntentLineageBroken`.
- [Source: `docs/adr/ADR-012-typed-intent-a2a-consent.md`] — `(peer-identity, intent-class)`.
- [Source: `_bmad-output/implementation-artifacts/spec-j1-tier-2-live-agent-demonstration.md#91`] — the HONEST SCOPE clause that created this bridge.
- [Source: `_bmad-output/planning-artifacts/analysis-maos-coding-delegation-ux-2026-07-17.md#199`] — `worker_completion` is not persisted.

---

## Appendix A — the ratified sketch, refuted (measured at `af788c3e`)

Shared with `j1-crosshost-1b`; rows D-2/D-3/D-10/D-11 are that story's to close.

| # | Sketch premise | Verdict | Evidence |
|---|---|---|---|
| D-1 | `MAOS_WORKER_TASK` at `main.rs:832-834` | **STALE** | Now `:1088-1089`; `:832-834` is `parse_host_grants_toml` egress parsing |
| D-1b | "no new public fn" at `main.rs:8130-8137` | **STALE** | Now `:10351`; `:8130-8137` is `fn extract_section` |
| D-2 | allowlist admits `task.assign` | **FALSE** | `i8.rs:98` admits only `[a-z0-9]`, `-`, one `:`. Fails closed at `router.rs:696-701`. Precedent trap: `cohort/manifest.rs:74-76` |
| D-3 | disallowed intent → smoke-8-8 shape | **FALSE** | Disallowed is `-32001`; smoke-8-8 asserts `-32009` and fails on `-32001` (`main.rs:11166-11169`); separation pinned by `fail_closed_8_8.rs:215` → **1b** |
| D-4 | install at composition root, no `maos-iac` change | **FALSE** | `with_a2a_router(mut self)` `:188` vs `&self` installers `:214`/`:220`; `Arc` at `main.rs:2560`. Only caller repo-wide is the test at `:848` |
| D-4b | ZERO kernel line-Δ implies ZERO kernel API Δ | **FALSE** | `kernel-core/src/iac.rs:13` `pub use maos_iac::*`; `kernel-api-classes.toml:264-268` |
| D-5 | mailbox branch routes it end-to-end | **PARTIAL** | Outbound branch real (`mailbox.rs:301-310`, `:453-458`, `:497-518`); **no inbound path** — `install_intake_sink` is a "test-only hook" (`a2a-core/router.rs:344`) |
| D-6 | "un-shortcut the two-level `task.assign`" | **MISLEADING** | No upper level exists; `run_cli_wrapper_manifest` has zero `Mailbox`/`deliver` refs; `assign_frame` has zero production callers; `on_idle` inert (`lib.rs:101-109`) |
| D-7 | Orchestrator can emit a remote frame | **FALSE** | `host_id: None` at `lib.rs:217`, `:242`; `consent_envelope: None` at `:250`. `lineage` already a param at `:210` |
| D-8 | topology worker "w/ `host_id`" | **NO SCHEMA** | Parser reads `manifest`/`path` only (`main.rs:663-685`); a **`host`** key already exists (`bilateral-2-host-mira-nash.toml:17,22`); `priority_weight` is dead config in all three topologies |
| D-9 | `worker_completion` TL-journaled | **FALSE** | `println!` at `main.rs:1284-1296`; corroborated by `analysis-maos-coding-delegation-ux-2026-07-17.md:199` |
| D-10 | hermetic fixture leg is BLOCKING/CI-safe | **NULL CONTROL** | `smoke_cli_wrapper_8_12.rs` has no CI invocation; CI runs `-p worker` (`:815`) and `-p maos-journey-test` (`:1869`) only → **1b** |
| D-11 | §A6 enforced | **NOT MECHANICAL** | Five story-file gates skip non-digit filenames |
| D-12 | ZERO-Δ @23202 | **STALE** | Pin is **24472**, verified green. `kloc-check` RED at HEAD on `maos-kernel-core`, pre-existing |

---

## Dev Agent Record

### Agent Model Used

`anthropic/opus-5` (frontier-class, E11-A1 allowlist) — Oh My Pi harness, 2026-08-14.
§A6 full-layer review net: **NOT YET RUN** — this record is the dev pass. The net is
NON-DEGRADABLE for this row (consent/A2A surface, `epic-10-process-agreements.md:15`),
and five story-file gates skip this filename (AC4.5), so a green CI does **not** mean
the net ran. It has not.

### Debug Log References

End-to-end run, hermetic fixture, isolated `XDG_DATA_HOME`
(`maos run spirits/topologies/j1-founder-loop.toml --once`), exit 0:

```
{"event":"delegation_routed","to_host":"developer-remote-host","recipient":"developer-remote",
 "intent":"development-task:write-workspace","goal":"founder-loop: execute the delegated assignment from founder-loop-host"}
{"event":"topology_worker_admit","manifest":".../worker/manifest.toml","topology":true,"frame_borne":true}
{"event":"cli_wrapper_loaded","spirit_id":"worker","granted_tier":"SandboxTier(3)","child_pid":504993,"live":false}
{"event":"cli_wrapper_exit","child_pid":504993,"stdout_lines":3,"exit_cause":"Exited { code: 0 }","is_crash":false}
{"event":"worker_completion","worker_cli":"worker-cli-fixture","completion":"completed","completed":true,
 "completion_tl_ref":"01a0005d36ef4d89d33bc184d7f8caee"}
{"event":"delegation_completed","result":"completed","orchestrator_frames_drained":1,"orchestrator_safe_point":true}
```

Transparency Log for that run — the goal is proven to have travelled the whole wire,
because the fixture echoes its routed argv and the echo is journaled:

| kind | from | to | payload |
|---|---|---|---|
| 0 `task.assign` | `orchestrator` | `developer-remote` | `{"TaskAssign":{"goal":"founder-loop: execute…"}}` |
| 21 `cli.subprocess.output` | `worker` | kernel | `"worker: received task assignment: founder-loop: execute the delegated assignment from founder-loop-host"` |
| 21 ×2 | `worker` | kernel | fixture work + `worker: task complete` |
| 1 `task.complete` | `developer-remote` | `orchestrator` | `{"TaskComplete":{"result":"completed"}}` |

Chain: Orchestrator → consent-bearing frame → loopback router → intake sink → pump
(`host_id` stripped) → `developer-remote` handle → `run_cli_wrapper_manifest` argv →
child stdout → TL row. **Nothing here inherits from an environment variable.**

**Live leg (AC3.9) — accepted for this mechanism-only story; successful completion moved
forward by review decision.** Real `codex` (`~/.local/bin/codex`) +
`MAOS_LIVE_AGENT=1` + `MAOS_HOST_GRANTS`, run from a git worktree. The frame-borne
path reached a real paid subprocess: host-grant exact match admitted,
**liveness-probe admission** (`codex --version`, not a bridge handshake), bridge spawn
with a real child pid, **358 `CliSubprocessOutput` rows journaled**, and the codex
adapter's `parse_completion` returned `not_completed:process_crash` from **captured
output**, not from the exit code. It stopped at `401 Unauthorized` on the model call:
`CODEX_API_KEY` is unset and `~/.codex/auth.json` is absent on this host. Nothing was
faked to make this leg look green. Successful `exit 0` / `completed=true` proof is
assigned to `j1-crosshost-2-cross-host-signed-run` through
`_bmad-output/test-artifacts/runbook-j1-tier-2-signed-live-run.md` Phase 3.

**Verification summary.** Everything this story owns is green:

| Suite | Result |
|---|---|
| `orchestrator` filtered `assign_frame_remote` contract tests | 4/4 |
| `maos-iac` `mailbox_a2a_router_installer_1a` (installer + fail-closed negative) | 3/3 |
| `maos-bin` `delegation_leg_1a` (wire, recursion, run-unique IDs, completion) | 5/5 |
| `maos-bin` `topology_delegation_1a` (parser, identities, unsupported remote-class refusal) | 14/14 |
| `xtask` `j1_crosshost_1a_proven_red` | 10/10 |
| `maos-journey-test` `journey_j1` | 3/3 |
| `maos-bin` `smoke_cli_wrapper_8_12` (untouched contract, still green) | 3/3 |
| `xtask` `example_spirit_regen_integration` (subcommand survives the module delete) | 4/4 |
| `maos-bin` `cohort_daemon_smoke_13_5c` (its src-file tripwire, enrolled) | 9/9 |

Gates: `check-j1-loopback-delegation`, `check-ship-gate-completeness`,
`check-coverage-matrix-completeness`, `coverage-matrix`, `check-kernel-baseline`,
`check-abi-ratification`, `check-dev-model-tier`, `check-dev-record-completeness`,
`check-bare-review-findings` — **all PASS**.

Review closure reran `check-j1-loopback-delegation` directly: **PASS**. The first post-patch run
caught a second capture-surface defect—the dotted-intent oracle matched the new rejection test—and
the final oracle now ignores test-only decoys while retaining a proven-red production planting.

**Two PRE-EXISTING flaky tests observed during `cargo test --workspace`, root-caused and
named** (a flaky test is a null control, so it is recorded rather than ignored). Two
separate whole-workspace runs each produced **one** failure, and they were **different
tests** — the first run's failure passed on the second and vice-versa:

1. `maos-telemetry::otel_gates::gate_otel_degradation_hot_path_completes` — asserts
   `drop_count() == 3` after `pause_consumer()`, but `pause_consumer`
   (`crates/maos-telemetry/src/otel_sink.rs:118-120`) only **sets a flag**; it does not
   wait for the consumer to park. Check-then-act: the consumer can drain one item after
   the flag is set, and the expected drop count shifts. 21/21 standalone, 12/12 under
   32-core artificial load. `otel_gates.rs` contains **zero** references to `Mailbox`,
   `A2ARouter`, `a2a_router` or `install_a2a` — the only `maos-iac` symbols this story
   changed.
2. `maos-kernel-core::on_revocation_three_actions::applier_executes_terminate_drain_and_quarantine_actions`
   — awaits two spawned deferred-action tasks with a **fixed** `for _ in 0..4 { yield_now() }`
   after advancing a paused clock (`:226-234`). Whether four yields suffice depends on the
   tokio interleaving. It flakes **2 of 5 runs in isolation with no load at all**, which is
   intrinsic nondeterminism, and `maos-kernel-core` has **zero** files modified by this
   story (`check-kernel-baseline` PASSED at 24472 = 24472).

Neither is a regression from this story; both belong to the retro's flaky-test surface.

### Completion Notes List

**What landed.** The delegation is frame-borne end to end. `Orchestrator::assign_frame_remote`
is a NEW fn (all 12 `assign_frame` call sites untouched) that sets both `host_id`s and a
granter-bound `ConsentEnvelope`; `Mailbox::install_a2a_router` is a `&self` installer over a
`OnceLock` (set-once — a second install is refused, proven); `maos_bin::delegation::DelegationLeg`
is the repo's **first frame consumer** — it owns the `developer-remote` handle under a real
binding, drains it, and hands `TaskAssignPayload.goal` to `run_cli_wrapper_manifest`; the
completion is journaled as the **existing** `FrameKind::TaskComplete`; `kind_to_string` now maps
kind 21 (plus the symmetric `kind_from_string` arm, because a one-way mapping leaves
`--kind cli.subprocess.output` unable to select the rows it renders).

**Six places the story's own premises were wrong, all corrected by measurement:**

1. **AC4.2's reclaim premise is right, its wording is not.** `example_spirit_regen` *does* have a
   clap subcommand and a dispatch arm — but the arm delegates to `templates_regen::run`, so the
   **module** had zero callers. Removing the `mod` line first and building compiled clean
   (`cargo check -p xtask`, exit 0): **Murat's compiler condition satisfied**, 133 tokei-code lines
   (+1 `mod` line) reclaimed. The four tests in `xtask/tests/example_spirit_regen_integration.rs`
   exercise the *subcommand* and stay green.
2. **AC5.1's ABI mechanism is refuted.** `abi-diff` and `check-abi-ratification` both scope to
   `crates/maos-spirit-abi` **only**; `check-service-boundary` walks kernel-core's own AST, where
   `iac.rs:13` is a `syn::Item::Use` that never expands. All three were run. **None can see the new
   `Mailbox` method**, so none demanded ratification → **no FLAG-Winston trigger**, and nothing was
   re-pinned. The `kernel-api-classes.toml` row was still added, labelled for what it is:
   documentation, exactly like the six pre-existing `Mailbox::*` rows this walker also never emits.
   Filed as **FLAG-E4**.
3. **AC1.7's predicted red did not happen, and the real gap was elsewhere.**
   `journey_j1.rs:73-76` never breaks — the Worker emits `cli_wrapper_loaded`, not `spirit_loaded`.
   The load-bearing half was the missing positive: `journey_j1` asserted **nothing** about the
   Worker, so the delegated leg could vanish with the test green. Five assertions added (intent,
   `frame_borne`, real child pid, adapter-parsed completion, and that the `TaskComplete` frame
   actually reached the Orchestrator, re-opening the FR20 safe point).
4. **AC5.3's budget arithmetic does not survive a correct implementation.** A faithful build put
   `maos-bin` at **16411 / 16178 (+233 OVER)** and `maos-iac` at **6889 / 6888 (+1 OVER)**. The story
   requests no grant and `kloc.toml:60-65` forbids one on an estimate, so this **decomposed**:
   in-`src` test modules moved to `tests/` (zero-cost, and CI-executable for the first time), and the
   peer/TOFU/router prologue moved to `maos_a2a::pairing` — pure A2A surface, in the crate that owns
   it, and the helper `1b` needs for its refusal legs. Final: `maos-bin` **16176/16178**,
   `maos-iac` **6851/6888**, `maos-a2a` **298/1500**. **No grant requested.** Filed as **D11-E3**,
   because it turns D11(b) from an accounting oddity into a constraint that decided two crates'
   architecture.
5. **`xtask` does NOT net negative.** AC4.2/AC5.3 predicted −133 against a ~52-69-line skeleton. A
   real 2-leg gate plus its oracle is ~205 code lines, so `xtask` nets **+71** (37023 → 37094),
   193 under its ceiling. Reported rather than reconciled: the estimate was wrong, the ceiling holds,
   and no grant is requested.
6. **`maos-bin` closes at 2 lines of headroom.** Uncomfortably tight and stated as a fact, not
   smoothed over. The reclaim path is demonstrated and repeatable (more in-`src` test modules remain).

**One regression this story would have shipped, found by testing for it and NOT papered over.**
AC3.6 (emit an Orchestrator `TaskAssign`) and AC3.10 (journal a `TaskComplete`) collide through
FR21's gate, whose predecessor test is a **60-second wall clock**, not causality. Two
`maos run … --once` invocations against the same `XDG_DATA_HOME` inside 60s: run 1 exits 0, run 2
exits 1 with `EOrchestratorDispatchRawOutput` — even though the dispatch carries
`prior_distillate_ref: None` and an empty `scope`, i.e. it references nothing, which is not the case
the published error contract describes. This story is the **first production emitter** of such a
frame, so it is where the false positive becomes reachable. It was **not relaxed** (window/role
tweaks), **not faked** (no synthetic `Distillate`), **not bypassed** (no `Mailbox::deliver` around a
kernel permission check), and **not repaired** (FR21 semantics belong to Story 6.2's owners). It is
fail-closed with a self-explaining error naming the window. Every in-repo caller uses a fresh temp
data home, so no test is affected. Filed as **FLAG-E5**; the real fix is rung 2's `task_id`
correlation, which this story's own "does NOT do" list already assigns to `j1-crosshost-2`.

**Gate (AC4).** `check-j1-loopback-delegation` is registered, `BindingClass::Blocking`, hermetic,
**PASS** at HEAD. Two legs: `frame-borne-route-intact` (the proven-red) and
`loopback-from-host-unverified` (Vex's boundary, published as `true` so rung 2 flipping it shows up
in a CI diff). Proven-red suite: `xtask/tests/j1_crosshost_1a_proven_red.rs`, **10/10**, including a
**green baseline** so no red is vacuous, five distinct "route locally anyway" plantings, and a
discrimination test proving the gate does not confuse the `task.assign` pin-test with a real dotted
consent intent. Enrolled in six surfaces; the seventh (`xtask/src/lib.rs`) is correctly **not**
needed because the proven-red suite drives the real binary against a tempdir fixture tree, per the
`story_10_5_proven_red` idiom. `env_contract.rs` needed no addition — this story **removes** a row
and adds no variable. Its CI job also invokes the behavioural delegation legs, which live in
`maos-bin`/`maos-iac` and are otherwise invisible to this workflow's `-p worker` /
`-p maos-journey-test` steps (AC5.5).

**Budget + attribution (AC5.4).** `check-kernel-baseline` **PASSED, 24472 = pinned 24472** — zero
kernel-core line Δ. `maos-a2a-core` **untouched** (4654/4654, zero headroom respected). The two
`kloc-check` breaches (`maos-domain` 8694>8644, `maos-kernel-core` 18933>18248) are **pre-existing
and not absorbed**. Three other gates fail identically at HEAD and in the working tree, measured in
a clean `git worktree` at HEAD: `check-service-boundary` **40 → 40**, `check-empty-kernel`
**6 → 6**, `check-env-contract` **2 → 2** (same two variables, `MAOS_OPERATOR_BEARER_TOKEN` /
`MAOS_OPERATOR_HTTP_BIND`; only line numbers shifted). Notably `check-env-contract` did **not** red
on the `MAOS_WORKER_TASK` deletion, because AC1.6's ordering was followed: read first, registry row
second.

**AC5.6 numbers, verified by measurement:** `EXPECTED_GATES` **36 → 37**; `xtask/src/check_*.rs`
**67 → 68**. D11's row said "36 vs 66"; HEAD measured 67. Both corrected in the packet.

**Handoff to `j1-crosshost-1b`.** The wire exists and blocks. `1b` adds ADR-012 refusal legs to a
gate that already blocks (`-32001` vs `-32009` separation), verifies the six landed enrollment
surfaces rather than repeating them, and closes `smoke_cli_wrapper_8_12`'s null control.
`maos_a2a::pairing::paired_loopback_router` and `DelegationLeg::install(mailbox, &intent)` both take
the intent as a parameter specifically so `1b` drives refusals through the **production** path
instead of a hand-built router.

### File List

**Added**

- `crates/maos-bin/src/delegation.rs` — the delegation leg: pump + the repo's first frame consumer
- `crates/maos-bin/src/topology.rs` — typed `[topology]` parser (in the lib so `tests/` can run it)
- `crates/maos-a2a/src/pairing.rs` — bilateral loopback pairing (send/accept asymmetry made explicit)
- `xtask/src/check_j1_loopback_delegation.rs` — the Blocking gate
- `xtask/tests/j1_crosshost_1a_proven_red.rs` — 10 proven-red vectors + green baseline
- `crates/maos-bin/tests/delegation_leg_1a.rs` — in-process emit→route→pump→drain legs
- `crates/maos-bin/tests/topology_delegation_1a.rs` — parser + identity + grep-control legs
- `crates/maos-iac/tests/mailbox_a2a_router_installer_1a.rs` — installer + fail-closed negative

**Modified**

- `crates/maos-bin/src/main.rs` — delegation-leg install at the composition root; frame-borne emit
  trigger in the topology loop; `run_cli_wrapper_manifest` takes `delegated_task: Option<&str>` and
  returns the completion label; `MAOS_WORKER_TASK` read + `DEFAULT_WORKER_TASK` const deleted;
  parser moved to the lib
- `crates/maos-bin/src/lib.rs` — `pub mod delegation; pub mod topology;`
- `crates/maos-bin/src/env_contract.rs` — `MAOS_WORKER_TASK` row **removed**
- `crates/maos-iac/src/adapter/mailbox.rs` — `a2a_router` → `OnceLock`; `install_a2a_router`
- `crates/maos-audit/src/lib.rs` — kind 21 mapped in `kind_to_string` **and** `kind_from_string`
- `spirits/orchestrator/src/lib.rs` — `assign_frame_remote`, `DELEGATION_CONSENT_INTENT`, pin-tests
- `spirits/topologies/j1-founder-loop.toml` — `host` on the worker entry; dead keys + env mention gone
- `spirits/topologies/j4-mira-nash.toml`, `spirits/topologies/bilateral-2-host-mira-nash.toml` —
  dead `priority_weight` removed
- `crates/maos-journey-test/tests/journey_j1.rs` — the positive frame-borne assertions (AC1.7)
- `crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs` — the two new `src` files enrolled in the
  exhaustive manifest-scope negative (its tripwire fired; enrolled properly, not count-bumped)
- `xtask/src/main.rs` — gate `mod` + clap variant + dispatch arm; `example_spirit_regen` mod removed
- `xtask/gate-registry.toml` — flat `gates` entry + `[[ship_gate]]` `blocking`/`blocking`
- `xtask/src/check_ship_gate_completeness.rs` — `EXPECTED_GATES` 36 → 37
- `xtask/kernel-api-classes.toml` — `Mailbox::install_a2a_router` row (documentation; see FLAG-E4)
- `.github/workflows/discipline.yml` — gate job (oracle + proven-red + behavioural legs),
  `v1-0-ship-gate` `needs:`, both v1.0 echo tables
- `tests/coverage-matrix.yaml` — `FR23a.gates` gains the gate (registry landed first, per
  `coverage_matrix.rs:140-143`)
- `_bmad-output/planning-artifacts/epics/epic-14-preflight-decisions.md` — D11-E1/E2 (preflight)
  plus **D11-E3**, **FLAG-E4**, **FLAG-E5** from implementation. D11 stays **OPEN**.
- `_bmad-output/implementation-artifacts/sprint-status.yaml` — status transitions

**Deleted**

- `xtask/src/example_spirit_regen.rs` — 133 tokei-code lines, zero module callers, compiler-proven

---

## Change Log

| Date | Change |
|---|---|
| 2026-08-14 | Created by splitting `j1-crosshost-1-loopback-developer-remote-delegation` (split ratified by Lunarpulse at preflight). Carries the mechanism: AC1-3 of the combined story plus a dedicated budget/ABI AC. Six-scout preflight @ `af788c3e` + adversarial validation disproved 12 premises of SCP §4.1 (Appendix A); baseline re-pinned **23202 → 24472**; the **ABI-surface axis** (`pub use maos_iac::*`) surfaced as the largest unflagged risk. |
| 2026-08-14 | **Dev-readiness pass — AC3 rewritten (9 items → 11).** A check of where a dev would stall found AC3 assumed a component that does not exist: **nothing in the repo has ever drained a mailbox handle and acted on a frame.** The production `maos run` path never calls `register_spirit` or `deliver` at all, and the closest template (`smoke_orchestrator_fanout_6_2`) `_`-binds all four handles — it proves delivery, never receipt. Added: an AC3 preamble stating there is no consumer precedent; **new AC3.2** naming the delegation consumer and why a translator is required (the Worker is a subprocess, not a mailbox peer); **new AC3.3** fixing the five identity strings with `peer_id == HostId`; **new AC3.6** wiring the emit trigger (the Orchestrator's `on_idle` is inert and nothing drives it) and stating that no dispatch loop is in scope at v0.8. AC3.5's sync/async ordering is now **decided** (emit→route→pump→drain, synchronous, before the worker admit) rather than delegated to the implementer — the same non-verifiable either/or shape already removed from AC3.7 and AC5.5. |
| 2026-08-14 | **Round-table consensus applied — 4 ACs → 5.** (1) ABI growth ratified, not dodged: `OnceLock` is a set-once *security* property. (2) `priority_weight` removed from all three topologies — the parser never read it, so the **signed** J1 demo documents scheduling that never happened. (3) Intent widened to **`development-task:write-workspace`**: ADR-012 names effect authority, and the grant is arbitrary code execution + filesystem mutation on the receiver. (4) Completion emits the **existing `TaskComplete` frame** instead of minting kind 32, plus a `kind_to_string` fix for kind 21 (159/247 entries in the signed T6 bundle render `unknown`). (5) **NEW AC4** — the gate skeleton lands here, not in 1b, with one leg = the proven-red; a test not behind a gate is a suggestion. (6) Funded by deleting `xtask/src/example_spirit_regen.rs` (133 code lines, zero callers, dead-code-warning since authored) — **no grant requested, measured-not-estimated rule intact**. (7) Dead-code finding **filed against D11 by ID** with D11(b) widened to "budget-charged code with no execution path" and its 36/66 corrected to 37/68; `epic-14-preflight-decisions.md` rule 1 makes the change-log-only option the forbidden one. Also recorded: `clippy` is installed at `discipline.yml:44` and never invoked. |
| 2026-08-14 | **Implemented — all 5 ACs, 16 tasks. Status → review.** The delegation is frame-borne end to end and PROVEN by journal, not by claim: `task.assign` (kind 0, `orchestrator`→`developer-remote`) → loopback router → intake sink → pump (`host_id` stripped) → the repo's **first** frame consumer → worker argv → the fixture's echo of the routed goal captured as a kind-21 row → `TaskComplete` (kind 1) back to the Orchestrator, FR20 safe point re-opened. Gate `check-j1-loopback-delegation` registered **Blocking** and PASSING, with a **10/10 proven-red suite over a green baseline** (five "route locally anyway" plantings + a pin-test discrimination). **Six story premises corrected by measurement** — AC4.2's "no dispatch arm" (there is one; it delegates elsewhere, so the *module* was dead: compiler-proven, 133 lines reclaimed), AC5.1's ABI mechanism (`abi-diff`/`check-abi-ratification`/`check-service-boundary` are all blind to kernel-core's re-exported surface → **no FLAG-Winston**, filed FLAG-E4), AC1.7's predicted red (never fires; the real gap was that `journey_j1` asserted nothing about the Worker), AC5.3's budget (a faithful build breached `maos-bin` by +233 and `maos-iac` by +1 → **decomposed, no grant**: in-`src` tests to `tests/` and the A2A prologue to `maos_a2a::pairing`; filed D11-E3), `xtask` "nets negative" (it nets **+71**, 193 under ceiling), and `maos-bin` closing at **2** lines of headroom. **One regression caught and refused rather than shipped:** AC3.6 + AC3.10 collide through FR21's 60s wall-clock predecessor test, so a second `maos run` on the same data home inside a minute fails closed — not relaxed, not faked with a synthetic distillate, not bypassed around the kernel gate, and not repaired (Story 6.2's owners); filed **FLAG-E5** with a reproduction. Kernel pin **24472 = 24472**, `maos-a2a-core` untouched at zero headroom, and the three other red gates measured identical at HEAD in a clean worktree (service-boundary 40→40, empty-kernel 6→6, env-contract 2→2). Live `codex` leg is **PARTIAL**: reached a real paid subprocess (liveness-probe admitted, 358 journaled rows, adapter-parsed completion) and stopped at `401 Unauthorized` — `exit 0` needs an operator key and was **not** faked. §A6 review net **not yet run**. |
| 2026-08-14 | **Code review closed — Status → done.** Parallel Blind, Edge, and Acceptance layers produced one decision and five patches; two additional reports were dismissed after call-site review. Lunarpulse moved successful live Codex completion proof to `j1-crosshost-2` via the Phase 3 runbook. Patches made frame IDs run-unique with `boot_nonce`, rejected unsupported host-bearing class entries, made `assign_frame_remote` reject empty/non-canonical lineage before state mutation, prevented non-completed workers from emitting `TaskComplete` or reopening FR20, and scoped the Blocking oracle to production branches despite test-only decoys. Verification: remote-frame tests 4/4, delegation 5/5, topology 14/14, proven-red 10/10, journey 3/3, direct Blocking gate PASS. |
