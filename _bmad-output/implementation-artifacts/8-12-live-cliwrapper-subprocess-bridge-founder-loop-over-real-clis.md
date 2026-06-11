---
dev_model_used: claude-opus-4-8
---
# Story 8.12: Live CliWrapper Subprocess Bridge — Founder-Loop Over Real CLIs (J1) ⚠ kernel

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->
<!-- Forks resolved by party-mode preflight 2026-06-08 (Winston, Murat, Amelia, John): FORK A → Winston host-grant tier model; FORK B → recommended default (no general multi-Spirit scheduler); FORK C → Amelia (defer respawn-with-context). See "Forks — RESOLVED" below. -->

## Story

As a founder running the overnight loop,
I want the Worker to spawn a real `claude`/`opencode`/`gemini`/`kimi` CLI through a working stdio bridge,
so that the J1 wedge demo runs my actual coding agents overnight and hands me an audit-traced result — not a canned-output fixture.

> **Phase 2; depends on 8.11 (daemon run surface).** This is the kernel work Story 8.4 explicitly disclosed-as-deferred ("a live multi-CLI stdio bridge — that is *kernel* work, deferred from 6.2"; [8-4 …md:37,52,108]) and the 2026-06-06 party-mode audit re-homed as the "homeless integration layer." **Charter-amended kernel delta** in `lifecycle/cli_wrapper/runtime.rs`; the zero-kernel-KLOC mandate is retired for this story (inherited from 8.11). New pinned `maos-kernel-core` baseline (re-pinned **from 15520**) + FLAG-Winston.

## Acceptance Criteria

1. **AC1 — Real `runtime.rs` stdio bridge (kernel delta).** `crates/maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs` graduates from a hash helper under a doc comment to a working bridge:
   - real subprocess spawn (through the AC5-resolved sandbox path), preserving `argv_prefix_hash` ([runtime.rs:39-49]) re-derived at spawn to assert the cap-token binding (ADR-023 TOCTOU);
   - a **length-delimited / ndjson stdio framing layer keyed on `posture.stdio_shape`** (`NdjsonOverStdio` newline-delimited · `JsonRpcOverStdio` Content-Length framing reusing [j1.rs:63-76] · `Raw` — **with an explicitly specified frame boundary**, since `Raw` does not self-delimit: read-to-newline buffered, documented in code);
   - a **control channel keyed on `posture.control_channel`** (`Signals` → `shutdown_signal` on `on_unload` · `NamedPipe` · `StdinCommands`) wired to the `on_unload`/`on_pause`/`on_resume` lifecycle hooks ([cli_wrapper/lifecycle.rs:11-16]);
   - a **recovery state-machine executor** that consumes `lifecycle::handle_subprocess_death(...) -> RecoveryAction` ([lifecycle.rs:28-42]) and **executes the existing decision — it never re-derives policy** (Winston trip-wire: the executor moves bytes and restarts processes; it does not know what a "founder loop" is). **Scope (FORK C):** this story implements **`Escalate` and `RespawnFresh` only**. `RespawnWithContext` is **deferred** (see "Deferred Work"); if any manifest declares `respawn_with_context`, the bridge **fails loud at admission/load** with a typed error — **no silent downgrade** to a different policy.
   - **Pinned integration seams (Amelia, required before code):** (a) reader topology — a dedicated OS reader thread per child owns `child.stdout`/`stderr`, frames in-thread, hands frames across a **bounded** `mpsc` to the kernel; the bridge owns the thread + `JoinHandle` and defines drop/shutdown order (no orphaned children); (b) **sender identity captured at spawn** and moved into the reader thread (it is off-kernel and holds no kernel context) so `insert_frame_event_with_sender` has a principal; (c) backpressure policy on the bounded channel stated (block or drop-with-audit).

2. **AC2 — Worker spawns a real config-selected CLI; output → Transparency Log with secret redaction; shape mismatch fails loud.** A `[cli_wrapper]` Worker actually executes its declared `command`/`argv_prefix` as a live subprocess; each captured stdout/stderr line is written as a `FrameKind::CliSubprocessOutput = 21` row with provenance (`from_spirit_id`, `stream`, `line_no`, `intent_lineage` inherited from the invoking Spirit's session) via the **real** `TransparencyLogAdapter::insert_frame_event_with_sender(...)` — **not** a hand-built literal.
   - **Credential/secret redaction (Winston, in-scope — NOT a follow-up):** real agent CLIs print auth errors and tokens to stderr; the `FrameKind=21` provenance write passes through a redaction scrubber on **spawn-env and stdout/stderr** before journaling (reuse the Story-8.2 redaction-trap discipline — 32-hex tokens never land in the Transparency Log). Credentials are injected host-side into the sandbox env; they never transit the manifest or the Transparency Log.
   - On subprocess exit a `FrameKind::CapabilityInvocation` audit row is written and the `Scope::CliSubprocessSpawn` cap-token is revoked with `RevokeReason::CliSubprocessExit{exit_code}`.
   - `output_shape_version` mismatch continues to fail loud via the **existing** Story-7.4 `admit_cli_wrapper_journaled` → `FrameKind::CliWrapperShapeMismatch = 27` path (**no new mismatch path**; must not regress [cli_wrapper_shape_mismatch_journal_7_4.rs]).
   - **ADR-022 crash semantics:** EOF on stdio + non-zero exit → a journaled `SpiritDied`/crash transition with exit-cause that **disambiguates signal-death from exit-code death** (crash detection ≤2s; `task.orphaned` NACK ≤5s; the zombie is reaped — no `<defunct>`).
   - The Worker manifest's `recovery_policy = "respawn_with_context"` ([spirits/worker/manifest.toml:12]) is changed to **`respawn_fresh`** for this story (the deferred policy is not declarable by the shipped demo).

3. **AC3 — Founder-loop runs as a real `maos run`, full topology, fixture path deleted.** The Worker is loadable and runnable under the 8.11 daemon: `maos run` learns a `[cli_wrapper]`-manifest load branch (fork **before** `extract("class")`; admit via `admit_cli_wrapper_journaled`, wire the AC1 runtime bridge) and the founder-loop `[class]` Spirits (orchestrator/architect/reviewer) become classifiable. **The full 8.4 topology is preserved** — Orchestrator → Worker(real CLI) → Architect → Reviewer → digest; this is NOT amputated to a single Worker (John's tripwire). The 11pm-assign → overnight distillate-dispatch → halt-and-resume → 7am-digest journey runs end-to-end with **real subprocess Worker output** flowing as `CliSubprocessOutput` rows; the morning digest cites real `source_log_ref`s resolving in the live Transparency Log. **The Story-8.4 hand-INSERT loop ([maos-bin/src/main.rs:4640-4659]) is DELETED, not bypassed** — no fake-output fallback may survive (else Tier-1 passes while the real path rots). **No general multi-Spirit topology scheduler under `maos run` (FORK B) — that is Epic 9 operator surface;** the load-fork lives in `maos-bin` (composition root), and `maos-kernel-core` receives an already-constructed handle and never reads a manifest to decide topology (Winston trip-wire).

4. **AC4 — J1 latency budget held over the real bridge, measured honestly.** The J1 founder-loop CliWrapper IPC overhead is measured **through the real `runtime.rs` bridge** (deterministic test-CLI doing ~zero work, so the measurement isolates bridge cost — not the agent), with **warmup iterations discarded**, **N ≥ 100 (200 preferred)**, reporting **P50/P95/P99/max** against the §13.1 budget **IPC overhead < 25ms P95** ([13:56], [j1.rs] `J1_P95_BUDGET_US = 25_000`). **Polarity (Murat):** on shared CI this is **reported Tier-2 evidence** gated only on a generous ceiling (e.g. 2×budget = 50ms) to catch real regressions without jitter-flake; the strict 25ms P95 is hard-gated only on a pinned/dedicated bench runner. The §13 "J-Butler p95 ≤ 4× J1 p95" ratio is computed from **both numbers measured in the same run on the same machine**. XDG_DATA_HOME is isolated **per iteration** (8.11 journal-corruption lesson; also keeps SQLite contention out of the IPC number). If breached on the pinned runner: fix our code first; do **not** migrate to in-process to mask it ([13:81-85]).

5. **AC5 — Sandbox tier = host-policy-granted, manifest-requested (FORK A → Winston model; security-invariant change, FLAG-Winston + sec-redteam).** Real agent CLIs need network + credentials + a writable workspace, which T3 (`--network=none --read-only --cap-drop=ALL`, Linux-only, image-attested — [t3/spawn.rs:1-90]) denies and admission today hard-requires (`ECliWrapperRequiresT3`). This story inverts the trust direction so the **manifest requests** a tier and **host policy grants** it — the artifact under least trust never decides its own sandbox (the 8.7–8.9 anti-pattern: never trust the self-declared field):
   - The manifest may **request** a tier; admission consults a **host-side grant allowlist** (operator config, NOT in the artifact) keyed on attested-image + signing-key → `{permitted_tier, permitted_egress_destinations}`. Manifest requests; host grants.
   - Keep `ECliWrapperRequiresT3` semantics as **default-deny**; add a new **fail-closed `ECliWrapperTierNotGranted`** when a tier request has no matching host grant. **No silent downgrade** anywhere; **Linux-only fails closed** (no silent tier drop on macOS/Windows).
   - The deterministic **fixture-CLI admission test path stays T3, unchanged**.
   - **Enforcement depth:** there is **no T2 scoped-egress mechanism in the kernel today** (only `t3/`; verified). 8.12 lands the **trust-direction gate + grant-config seam + fail-closed error** (the security-correctness core). For the live agent-CLI *execution*, use the **T3 network-permitted container-profile variant** (Winston's fallback: keep attestation + cap-drop intact, punch one *named, host-granted* egress hole) **unless** an enforced T2 Landlock+seccomp scoped-egress profile is delivered in-story; the choice is the dev's + Winston's based on what is genuinely buildable here, recorded in Completion Notes. **Full enforced egress allowlisting may itself be a follow-up** — if so, log what is enforced vs. declared (no silent gap).
   - **Generalization note:** this host-grant + egress-allowlist + credential-redaction pattern is the **same one Stories 8.14b/8.14c (real MCP drivers — Calendar/Slack/web/arXiv) will need.** Design the grant seam so it is not CliWrapper-only (a `[capabilities]`/host-grant surface MCP drivers can reuse), or explicitly flag the divergence.

6. **AC6 — Anti-theater + crash/recovery test coverage; two-gate "presentable" (Murat + John).**
   - **Hardened anti-theater (replaces 8.4 hand-INSERT proof):** the hermetic test proves the `CliSubprocessOutput` row came from a **real spawned process**, not in-process computation — assertion = **per-run-fresh nonce echoed by the child + the child's real PID carried in the journaled row + `child_pid != std::process::id()` + the child was reaped**. (Nonce alone is gameable three ways; this set is spawn-or-fail.)
   - **Crash-detection matrix (table-driven):** EOF+non-zero → `SpiritDied` with cause ✅; **EOF+zero-exit (clean finish) → NOT a crash** (the false-positive that pages people); non-zero+no-EOF-yet; signal-death (SIGKILL/SIGSEGV) cause-disambiguated; **stdout still draining when the child dies → no truncation** (lines emitted before death are journaled before `SpiritDied`).
   - **Recovery executor:** `RespawnFresh` asserts context is **NOT** carried (negative assertion — easy to implement backwards); `Escalate` journals + surfaces and does **not** silently loop-respawn; **a respawn-attempt bound exists** and reaching it routes to `Escalate` (if no bound is specified, that is a missing requirement — add one). Timing bounds (≤2s / ≤5s) asserted with **margin** via an event/condvar with a timeout *ceiling*, never poll-sleep-then-check.
   - **`ci_default` guard self-test:** the hermetic path asserts **zero network + no real agent CLI**, AND a test proves the guard itself **trips** when pointed at a real CLI/socket (a guard with no failure-mode test is decoration). No load-bearing test is `#[ignore]`d (Epic-7 scar).
   - **Two-gate "presentable" (John — make explicit):** **Tier-1 (CI)** = fake-CLI-through-real-bridge green = "the machine is real" (necessary, *not* sufficient). **Tier-2 (signed artifact)** = one real `claude`/`opencode` run through `maos run` (not a bespoke harness), captured + archived + signed by a **named owner**, showing the digest and the audit trail proving citations trace to refs the *real* agent produced (ideally across a halt/resume). **Tier-2 is a hard precondition of "Epic 8 Completion," distinct from and downstream of CI green** — recorded as a release-gate checklist item, not "dev done."

7. **AC7 — Kernel discipline (FLAG-Winston).** Re-pin the `maos-kernel-core` baseline **from 15520** with the new bridge delta recorded line-auditably and **atomic in the same commit**; the diff maps only to "CliWrapper runtime bridge + recovery executor (escalate/respawn-fresh) + tier-grant gate" — **no LLM/inference/provider type, name, or import** in any kernel-core file; **no `maos run` orchestration** in kernel-core (that stays in `maos-bin`). `cargo fmt -p` is **banned** (7.5a lesson). Workspace member count stays **42** unless a new dev-only deterministic test-CLI fixture crate is added (if so, bump root `Cargo.toml` members + the `4-kernel-design.md:115` sentinel in lockstep and justify). `abi-diff --base` Added-only (frozen `maos-spirit-abi` untouched).

## Tasks / Subtasks

> **Sequencing (Amelia — insisted):** T1 → T2 → T3 in order. Build and prove the bridge in isolation BEFORE wiring `maos run`; inverting this turns a framing bug into a day chasing a phantom `ChannelClosed`.

- [x] **T1 — Real stdio bridge in `runtime.rs`, in isolation (AC1)**
  - [x] `spawn_and_bridge(...)`: spawn via the AC5-resolved sandbox path (direct `std::process::Command`, as the admission probe does); re-derive `argv_prefix_hash` and assert cap-token binding (fail loud → `BridgeError::CapBindingMismatch`).
  - [x] Reader thread per child owning stdout/stderr; framing keyed on `posture.stdio_shape` (ndjson / Content-Length jsonrpc / raw-with-explicit-LF-boundary); bounded `sync_channel` mpsc to kernel; **sender identity captured at spawn, moved into the thread**; backpressure policy (`Block`/`DropWithAudit`); defined drop/shutdown order (close stdin → kill+reap → join).
  - [x] Control channel keyed on `posture.control_channel`; `on_unload`/`on_pause`/`on_resume` wired (StdinCommands full; Signals/NamedPipe documented v0.9 behavior).
  - [x] Red→green against `/bin/sh` real subprocess + the fixture-CLI with **isolated in-memory TL**, NO `maos run` wiring yet (`cli_wrapper_bridge_8_12.rs` 9 tests).
- [x] **T2 — Recovery executor + capture + crash semantics (AC1, AC2)**
  - [x] `execute_recovery` over `handle_subprocess_death → RecoveryAction`: `Escalate` + `RespawnFresh` only; `RespawnWithContext` → **fail-loud typed error at admission/load** (`reject_respawn_with_context` → `ERespawnWithContextUnsupported`) AND executor backstop; respawn-attempt bound → `Escalate`.
  - [x] Per-line `insert_frame_event_with_sender(FrameKind::CliSubprocessOutput, …)`; redaction is the TL's built-in `self.redaction.redact` pre-write scrubber (proved: 64-hex token never lands); spawn-env never journaled; on exit `CapabilityInvocation` row + `revoke_cli_subprocess_exit` → `RevokeReason::CliSubprocessExit`.
  - [x] ADR-022: `ExitCause{Exited/Signaled/Unknown}` (signal vs exit disambiguated); crash-detect <2s (timing test with margin); reap zombies (Drop + wait).
  - [x] Crash-detection matrix tests (EOF+zero-exit = NOT a crash; stdout-drain-before-death ordering; signal death).
- [x] **T3 — `maos run` cli_wrapper + founder load branch (AC3) — LAST**
  - [x] `[cli_wrapper]`-manifest detection fork **before** `extract("class")`; `classify_spirit`/`LoadedSpiritKind` extended with `FounderLoopClass` (orchestrator/architect/reviewer classifiable — standalone load short-circuits with a FORK-B directional error; see Completion Notes).
  - [x] Runtime-handle lifetime owner stated: `run_cli_wrapper_manifest` owns the `SpawnedBridge` for the run scope; Drop kills+reaps at exit.
  - [x] **DELETED** the hand-INSERT loop ([main.rs:4640-4659]); replaced with a real `spawn_and_bridge` over `worker-cli-fixture`; FR20 drain / FR21 distillate-dispatch / halt-resume / digest-cites-real-refs preserved.
- [x] **T4 — Sandbox tier grant gate (AC5)** — NEW `maos-domain::host_grant` seam (generalized for 8.14b/c) + `resolve_cli_wrapper_tier` + `ECliWrapperTierNotGranted` fail-closed; manifest-requests/host-grants; Linux-only fail-closed; live-CLI = T3-network-permitted variant; fixture-CLI T3 admission unchanged; enforced-vs-declared recorded below.
- [x] **T5 — J1 over the real bridge (AC4)** — `run_j1_bridge_measurement` through the real bridge; warmup-discard, N=120 (200 default), P50/P95/P99/max; generous-ceiling CI gate (`J1_CI_CEILING_US=50ms`); per-iteration measurement; synthetic floor retained.
- [x] **T6 — Anti-theater + two-gate + kernel discipline (AC6, AC7)** — hardened spawn-provenance test (nonce+child-PID+parent≠child+reaped); `ci_default_guard` + its trip-test; Tier-2 signed-artifact release-gate item authored; baseline re-measured + line-audited, FLAG-Winston, no `cargo fmt -p`; workspace-count 42 + abi-diff green.

## Dev Notes

### What this story is (and is NOT)

This is the **integration spine for J1**, not a new reference Spirit. The four founder-loop Spirits already exist and are thin (Story 8.4). What does NOT yet exist is the **live subprocess bridge in the kernel** — `runtime.rs` is, today, *literally* a `sha2` hash helper (`argv_prefix_hash`) under a 30-line doc comment that describes the bridge as "scaffolding deferred to v0.5-α" ([runtime.rs:1-77]). Everything around it is real and must be **reused, not rebuilt**:

| Substrate (REAL — reuse) | Location |
| --- | --- |
| Admission probe + shape assertion (`probe_and_verify_shape`) | `cli_wrapper/admission.rs:47-158` |
| FR40 shape-mismatch journaling (`admit_cli_wrapper_journaled` → `CliWrapperShapeMismatch=27`) | `cli_wrapper/admission.rs:187-226` (Story 7.4) |
| Recovery **decision** logic (`handle_subprocess_death → RecoveryAction`) | `cli_wrapper/lifecycle.rs:28-42` |
| Manifest config types (`CliWrapperConfig`, `…Posture{stdio_shape,control_channel,shutdown_signal}`, `…RecoveryPolicy{RespawnWithContext,RespawnFresh,Escalate}`) | `maos-manifest/src/manifest.rs:3482-3597` |
| `Scope::CliSubprocessSpawn` | `maos-domain/src/invariants/i1.rs:91-102` |
| `RevokeReason::CliSubprocessExit` | `maos-capability/src/cap_tokens/mod.rs:100-106` |
| `FrameKind::CliSubprocessOutput = 21` | `maos-iac/src/adapter/transparency_log.rs:80-82` |
| TL insert API (`insert_frame_event_with_sender`, I2 panic-on-write-fail) | `transparency_log.rs:404-416,495-550` |
| T3 spawn path (`spawn_t3`, container-isolated, Linux-only) | `security/sandbox/t3/spawn.rs:54` |
| Worker fixture-CLI + constants | `spirits/worker/src/lib.rs:34-55`, `src/bin/worker-cli-fixture.rs` |
| 8.11 daemon run surface (`maos run`, `--once`, posture-keyed boot, drain) | `maos-bin/src/main.rs:~850-1100,3936-3982` |
| J1 bench harness | `maos-bench/src/harness/j1.rs` |

> **There is NO `CliWrapperPort` type today** (verified). The CliWrapper class is **manifest-driven** (`[cli_wrapper]` present ⇒ CliWrapperSpirit; `[class]` present ⇒ native Spirit; mutually exclusive — `EManifestSchemaConflict`). Do **not** invent a port abstraction; thread the runtime handle the way `maos run` already threads adapters.

### The `maos run` gap (AC3 — read carefully)

`maos run` today is **single-Spirit, `[class]`-only**. `classify_spirit` ([main.rs:187-193]) returns `Some(_)` only for `"butler"`/`"researcher"`. The load recipe ([main.rs:~860-1072]) calls `extract("class")` then `SecurityManagerAdapter::admit_spirit(...)` with a `ClassSection`. A `[cli_wrapper]` Worker manifest has **no `[class]` section**, so that recipe cannot load it — the cli_wrapper branch forks **before** `extract("class")`. The founder `[class]` Spirits load through the existing class recipe once `classify_spirit` knows them. Per **FORK B (resolved)**: scope is "Worker-real-under-`maos run` + full-topology founder-loop smoke upgraded from hand-INSERT to real spawn." Do **not** build a general multi-Spirit topology scheduler under `maos run` — that is Epic 9.

### Determinism: how a "real" bridge stays hermetic in CI (the AC6 split)

8.4 faked Worker output by hand-INSERTing canned lines straight into the TL ([main.rs:4640-4659]) — the bridge never ran. 8.12 runs a **real subprocess through the real bridge** even in CI, using a **deterministic local test-CLI** as the subject. The anti-theater assertion (AC6) is **spawn-or-fail**: per-run-fresh nonce echoed by the child + child's real PID in the journaled row + `child_pid != parent_pid` + child reaped. Live `claude`/etc. = Tier-2 (`--live`) reported evidence, never a hermetic floor; the `ci_default` guard asserts zero network + no real agent CLI AND has its own trip-test.

### ADR-021 + ADR-022 + §6.7 — the contract you must honor

- **Fail-loud, never best-effort.** On `output_shape_version` divergence the kernel refuses to start (`EOutputShapeAdapterMismatch`, journaled); reuse the 7.4 path; do not add a second mismatch path ([ADR-021], [6-reference-spirits.md:123-125]).
- **Crash semantics (ADR-022).** EOF + non-zero exit → `SpiritDied` journaled with exit-cause; recovery policy declared in wrapper config. Crash detect ≤2s; `task.orphaned` NACK ≤5s; reap zombies.
- **Output-shape adapter registry.** §6.7 names a registry key `cli-wrapper-template:<cli-name>:<shape-version>`. Confirm whether it exists; if absent, keep the admission version-equality check (7.4) as the whole contract for v0.9 and flag the registry as deferred — do not silently invent it.

### Files being modified (UPDATE) — preserve existing behavior

- `crates/maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs` — **the** kernel delta. Keep `argv_prefix_hash` + its 3 tests; build the bridge around it per the doc-comment's stated flow.
- `crates/maos-kernel-core/src/lifecycle/cli_wrapper/{mod,lifecycle}.rs` — re-export new runtime surface; wire the recovery executor; keep `RecoveryAction` + the 3 decision tests byte-stable.
- `crates/maos-kernel-core/src/lifecycle/cli_wrapper/admission.rs` (+ `maos-domain/src/cli_wrapper.rs`) — AC5 tier-grant gate + `ECliWrapperTierNotGranted`; AC1 `RespawnWithContext` fail-loud-at-admission.
- `crates/maos-bin/src/main.rs` — cli_wrapper load branch + `classify_spirit` additions + **delete** the hand-INSERT, replace with real spawn. **Preserve:** the 8.11 single-Spirit load/`--once`/serving/drain paths, posture-keyed boot, `MAOS_ONE_SHOT` arms.
- `crates/maos-bench/src/harness/j1.rs` — measure the real bridge; keep the synthetic floor.
- `spirits/worker/manifest.toml` — `recovery_policy` `respawn_with_context` → **`respawn_fresh`** (AC2); confirm requested tier under the AC5 grant model.

### Project Structure Notes

- Workspace stays **42** unless a new dev-only test-CLI crate is genuinely needed (default: reuse `worker-cli-fixture`). If added, bump root `Cargo.toml` `members` AND the `4-kernel-design.md:115` sentinel in lockstep.
- `maos-kernel-core/src/` is **intentionally NOT byte-identical** (the charter delta, second such story after 8.11) — re-pin **from 15520**, line-auditable, FLAG-Winston, no `cargo fmt -p`.
- The `kloc-check` aggregate ceiling (6000) is already exceeded pre-story; the bridge delta does not *cross* a new ceiling — same neutral-RED posture 8.11 documented; do not bump the ceiling.
- No new discipline gate required (reuse kloc/abi/workspace-count). If T3 surfaces a cli_wrapper-load completeness concern, prefer extending `check_composition_root_completeness.rs` over a new gate.

### Forks — RESOLVED (party-mode preflight 2026-06-08: Winston, Murat, Amelia, John)

- **FORK A (sandbox tier) → Winston host-grant model (RATIFIED by Lunarpulse).** NOT the spec's original "honor the manifest-declared tier" (rejected as a trust inversion / privilege-escalation seam). Manifest **requests**, host **grants** via an attested-image+signing-key allowlist; fail-closed `ECliWrapperTierNotGranted`; no silent downgrade; Linux-only fails closed; credential redaction in AC2 (in-scope). Live-CLI execution via the T3-network-permitted variant unless enforced-T2 scoped-egress is delivered in-story. **Security-invariant change — flag for Winston + sec-redteam at review.** Folded into **AC5/AC2**.
- **FORK B (AC3 scope) → recommended default (RATIFIED).** Worker-real-under-`maos run` + full-topology founder-loop smoke upgraded to real spawn; **no general multi-Spirit topology scheduler** (Epic 9). John's condition folded into **AC3**: preserve the full 8.4 topology + **delete** the fixture path.
- **FORK C (recovery depth) → Amelia (RATIFIED by Lunarpulse).** Ship **`Escalate` + `RespawnFresh` only**; **defer `RespawnWithContext`** (see "Deferred Work"). Winston's no-silent-downgrade rule folded into **AC1**: a manifest declaring `respawn_with_context` **fails loud at admission/load**. Build-order (Winston): land `Escalate` + ADR-022 timings as the safety net first, then `RespawnFresh`.

### Test ACs the preflight added (Murat) — all in AC6

(1) spawn-provenance hardened to nonce+fresh+child-PID-in-row+parent≠child+reaped; (2) EOF+zero-exit = NOT a crash; (3) stdout-drain-before-death ordering; (4) respawn-loop bound → escalate-reachability; (5) `RespawnFresh` negative assertion (context NOT carried); (6) zombie reaping; (7) `ci_default` guard self-test (proves the guard trips); (8) per-run-fresh nonce (not a static fixture nonce).

### Latest tech / library notes

- **Subprocess + async (ADR-010 sync-port/async-kernel):** the long-lived bridge needs **streaming reads** off a dedicated reader thread / `spawn_blocking`, NOT the admission probe's poll-to-completion (`admission.rs:73-111` — easy to grab the wrong helper since it lives right next door).
- **Framing:** the J1 bench already speaks `Content-Length:\r\n\r\n` ([j1.rs:63-76]); reuse for `JsonRpcOverStdio` so bench and production agree. `Raw` has **no** self-delimiting boundary — specify one explicitly.
- **No new deps expected** in kernel-core (`serde_json`, `sha2` cover it). A PTY for a real agent CLI that detects non-tty is a `--live`-only concern (portable-pty lives in `maos-journey-test`) — keep it out of kernel-core.

### Testing standards

- Per-crate `cargo test`; subprocess smokes under `crates/maos-bin/tests/smoke_*` (`Command::new(env!("CARGO_BIN_EXE_maos-bin"))`, CWD = workspace root, JSON-on-stdout, **isolated `XDG_DATA_HOME`** — every subprocess test, no exceptions).
- The hermetic bridge test proves a **spawned child** produced the row (AC6 anti-theater) — never a constant the test wrote.
- Keep the 7.4 shape-mismatch journal tests and the 6.2 FR52 surface tests green. Recovery-policy decision tests in `lifecycle.rs` stay byte-stable.
- J1: report P50/P95/P99/max; Tier-2 reported + generous-ceiling CI gate; document the number in Completion Notes (`[[feedback_lunarpulse_observability_preference]]`).

### Lessons from prior Epic-8 stories (apply)

- **8.11:** `maos run` corrupts the shared journal — ALL subprocess tests MUST isolate `XDG_DATA_HOME`. Posture-keyed, never name-keyed. Record the new kernel baseline honestly; FLAG-Winston; line-auditability.
- **8.4:** `register_spirit_typed` handle MUST be bound/held or the mailbox closes (`ChannelClosed`). The hand-INSERT was a deliberate v0.8 stand-in — this story retires it.
- **8.2:** the redaction trap — 32-hex tokens leak into cites/logs if not scrubbed pre-write. AC2's stderr redaction is the same lesson.
- **8.7–8.9:** never trust a self-declared field (`frame.from` / manifest-declared tier) — re-derive/grant from a trusted source. The FORK-A host-grant model is this lesson applied to sandbox tiers.
- **7.5a:** `cargo fmt -p <crate>` is **banned** (whole-crate collateral).
- **Epic-7 scar:** never flip a gate green while red; never `#[ignore]` a load-bearing test.

### References

- [Source: epic-8-…md:438-449] — Story 8.12 AC sketch; [:5] Charter Amendment; [:381,389] DAG + per-journey gate (J1 = 8.11 + 8.12).
- [Source: _bmad-output/implementation-artifacts/8-11-…md] — predecessor; `maos run` daemon, posture-keyed boot, baseline 15505→15520, FLAG-Winston + 5 trip-wires, XDG isolation lesson.
- [Source: _bmad-output/implementation-artifacts/8-4-…md:37,52,53,108,143] — the deferral this story owns (Decision B); the founder-loop wedge choreography; SpiritRole::Worker for architect/reviewer.
- [Source: crates/maos-kernel-core/src/lifecycle/cli_wrapper/{runtime,mod,admission,lifecycle}.rs] — stub + reuse surfaces (see table).
- [Source: crates/maos-manifest/src/manifest.rs:3482-3597,3521-3529] — `CliWrapperConfig`, posture, `CliWrapperRecoveryPolicy{RespawnWithContext,RespawnFresh,Escalate}`.
- [Source: spirits/worker/manifest.toml:12,20] — `recovery_policy = "respawn_with_context"` (→ respawn_fresh), `tier = "T3"` (→ requested tier under grant model).
- [Source: crates/maos-domain/src/invariants/i1.rs:91-102; maos-capability/src/cap_tokens/mod.rs:100-106; maos-domain/src/cli_wrapper.rs] — scope, revoke reason, admission error enum (incl. `ECliWrapperRequiresT3` → add `ECliWrapperTierNotGranted`).
- [Source: crates/maos-iac/src/adapter/transparency_log.rs:80-82,95-103,404-416,495-550] — frame kinds + insert API + I2.
- [Source: crates/maos-kernel-core/src/security/sandbox/t3/spawn.rs:1-90] — `spawn_t3` constraints (AC5/FORK-A).
- [Source: crates/maos-bin/src/main.rs:182-208,857-1100,4640-4659] — `classify_spirit`, class-only load recipe, posture boot, the hand-INSERT to delete.
- [Source: crates/maos-bench/src/harness/j1.rs:25,45-76,121-167] — budget, framing, measurement loop.
- [Source: architecture-…/12-…ADR-021/ADR-022; 6-reference-spirits.md:119-127; 3-vocabulary-invariants.md#I6; 13-phased-roadmap.md:13,45,56,81-85] — output-shape + crash-semantics contract; §6.7 CliWrapperSpirit (incl. all three recovery policies); **I6 hot-swap state-transfer (the mechanism deferred respawn-with-context will reuse)**; J1 floor + 25ms budget + 4×-J1 rule.

## Deferred Work — `RespawnWithContext` (FORK C)

**What:** The third CliWrapper recovery policy — on subprocess crash, re-spawn a fresh child **and re-feed the prior task/conversation context** so mid-task work survives a crash, instead of restarting the task (`RespawnFresh`) or surfacing to the operator (`Escalate`).

**Why deferred (Amelia + Murat):** (1) "context" for an **opaque subprocess CLI is undefined** — we cannot snapshot a foreign process's memory; the real design is to snapshot the **kernel-side** conversation/distillate/task state and re-feed it to a fresh child. (2) That snapshot needs a **per-Spirit-class CBOR schema** and wiring into the **hot-swap state-transfer codec (I6 / ADR-017)** — a brand-new serialization surface for a bridge that has none, i.e. a second story's worth of design inside AC1. (3) It is the **highest-bug-risk path** (stateful, sad-path-only) and has **no consumer yet** — the founder loop is fully demonstrable with `RespawnFresh` + `Escalate`.

**Precondition to pick it up:** (a) a real consumer that needs mid-task crash continuity (e.g. the Tier-2 overnight run shows `RespawnFresh` loses too much work to be acceptable); AND (b) a defined CliWrapper context-snapshot schema reusing the I6/ADR-017 hot-swap CBOR per-Spirit-class codec (NOT foreign-process memory).

**When / where:**
- **Earliest:** a dedicated follow-up reliability story authored after 8.12 review (re-run `/bmad-create-story` once the precondition holds — the phantom-seam discipline: don't context-engineer it until the I6 snapshot seam is real).
- **Natural epic home: Epic 10 (v1.0 hardening)** — it is reliability work measured by **NFR-Rel-3 HSIS (Hot-Swap Invariant Suite per Spirit class)**; CliWrapper crash-with-context-continuity is a HSIS-for-CliWrapper item, which is a v1.0 gate concern, not a v0.9 founder-loop blocker.
- **Pull earlier only if a consumer emerges** (e.g. an Epic-8-Completion follow-up if the Tier-2 run demands it).
- **Until then:** the `RespawnWithContext` enum variant remains in the manifest schema, but the executor **fails loud** on it (AC1) and the Worker manifest does not declare it (AC2). The variant is reserved, not silently degraded.

## Cross-Impact — what else these decisions touch

> Flag-list for the team; the items marked **(action)** need a concrete follow-up or a confirming decision.

**From FORK A (host-grant tier model):**
1. **maos-manifest / admission / `maos-domain::cli_wrapper`** — new `ECliWrapperTierNotGranted`; the hard `ECliWrapperRequiresT3` check becomes a host-allowlist consult. Security-invariant change → **Winston + sec-redteam review (action)**.
2. **NEW host-policy config surface** (attested-image+signing-key → permitted tier + egress). 8.12 lands the minimal seam; the **operator-facing management of it is Epic 9** (operator productionization). **(action: confirm the 8.12 seam vs. Epic-9 surface boundary.)**
3. **T2 scoped-egress does not exist in the kernel** (verified — only `t3/`). Real enforced egress allowlisting is net-new security work; 8.12's default is the **T3-network-permitted variant**. Full enforced-egress-allowlist is a likely **follow-up (action)**.
4. **Stories 8.14b / 8.14c (real MCP drivers)** need the **same** host-grant + egress + credential-redaction pattern for Calendar/Slack/web/arXiv. **(action: design the grant seam to generalize beyond CliWrapper, or flag the divergence — this affects how 8.14b/c are specced.)**
5. **ADR-021 / architecture §6.7 / PRD developer-tool-specific-requirements.md:103** describe the sandbox posture as manifest-configured; the host-grant inversion is an **architectural refinement** that should be reflected (**possible ADR amendment — action for Winston**).

**From FORK C (defer respawn-with-context):**
6. **Architecture §6.7 + PRD:55,103 + ADR-021** specify **all three** recovery policies as the CliWrapper contract; the v0.9 implementation is now a **documented subset**. The deferral + the fail-loud-on-declare behavior should be noted so the contract and the code don't silently diverge. **(action: a one-line "v0.9 ships escalate/respawn-fresh; with-context = v1.0/NFR-Rel-3" note in the ADR/§6.7, or a Winston ruling that the spec/code gap is acceptably documented here.)**
7. **I6 / hot-swap codec (Epic 5) and NFR-Rel-3 HSIS** — respawn-with-context is HSIS-per-Spirit-class reliability work; HSIS-for-CliWrapper stays **incomplete until the follow-up**, which is a v1.0 (Epic 10) gate item, not an 8.12 gate item.
8. **Story 8.15 (journey-acceptance harness)** — any crash-recovery journey it asserts tests **escalate + respawn-fresh only** for now; do not author a with-context journey assertion yet.

**From FORK B (no multi-Spirit scheduler):**
9. **Epic 9 operator surface** absorbs the general "run/manage N Spirits" capability; 8.12 deliberately leaves it there. No 8.12 action — but it confirms the boundary 9.x will build to.

## Dev Agent Record

### Agent Model Used

claude-opus-4-8 (recommended; kernel-touching integration story with a security-invariant change — Winston/Murat/sec-redteam review-heavy, matches prior 8.x kernel stories)

### Debug Log References

- Founder-loop journey (real worker): `MAOS_ONE_SHOT=smoke-founder-loop-8-4 ./target/debug/maos-bin` → "worker spawned REAL subprocess pid=… (parent=…) → 3 stdout line(s) journaled" + "anti-theater OK" + "✅ founder-loop wedge complete".
- Standalone: `XDG_DATA_HOME=$(mktemp -d) ./target/debug/maos-bin run spirits/worker/manifest.toml --once` → `cli_wrapper_loaded{granted_tier:"SandboxTier(3)", child_pid}` + `cli_wrapper_exit{stdout_lines:3, is_crash:false}`.
- J1: `cargo test -p maos-bench --lib j1_bridge_overhead_within_ci_ceiling -- --nocapture` → `J1-bridge: P50=10us P95=12us P99=13us max=16us (N=120)`.

### Completion Notes List

**Summary.** `runtime.rs` graduated from a `sha2` hash helper under a doc comment to a working stdio bridge (AC1): real spawn, per-child OS reader threads, framing keyed on `posture.stdio_shape`, control channel keyed on `posture.control_channel`, a recovery state-machine executor that *executes* `handle_subprocess_death`'s decision (never re-derives policy), ADR-022 crash semantics, and the AC5 host-grant tier gate. The Story-8.4 hand-INSERT of canned `CliSubprocessOutput` rows is **DELETED**; the founder-loop journey now spawns a REAL `worker-cli-fixture` subprocess through the bridge. All gates land in the **same commit**.

**AC1 — bridge.** `spawn_and_bridge` spawns directly via `std::process::Command` (exactly as the admission probe at `admission.rs:73` already does — `spawn_t3` containers are not reliably available in this CI env; the live agent-CLI path uses the T3 network-permitted variant, see AC5 enforced-vs-declared). Re-derives `argv_prefix_hash` at spawn and asserts the cap-token binding (ADR-023 TOCTOU) BEFORE the child runs → `BridgeError::CapBindingMismatch`. Pinned seams: dedicated OS reader thread per stream owns it, frames in-thread, hands frames over a **bounded `sync_channel`**; sender identity captured at spawn into `from_spirit_id`; backpressure `Block` (lossless) / `DropWithAudit` (audited drop counter); `Drop` closes stdin → kill+reap child (no `<defunct>`) → join readers (no orphan). `Raw` boundary is **explicitly** LF (read-to-newline buffered, documented). `JsonRpcOverStdio` reuses the J1 `Content-Length:` framing.

**AC2 — capture + redaction + revoke.** Each line → `insert_frame_event_with_sender(CliSubprocessOutput=21, …)` with `{cli, stream, line, line_no, child_pid, intent_lineage}` provenance, sender = the spawn-captured `worker`. **Redaction is the TL's built-in `self.redaction.redact` pre-write scrubber** (the canonical Story-8.2 path) — a 64-hex token printed by the child never lands in the log (proved by `redaction_trap_hex_token_never_lands_in_log`); spawn-env credentials are passed to the child env only and **never journaled**. On exit a `CapabilityInvocation` row is journaled and `CapabilityRegistryAdapter::revoke_cli_subprocess_exit` revokes the `Scope::CliSubprocessSpawn` token with `RevokeReason::CliSubprocessExit{exit_code}` (lifecycle proven in `maos-capability::cap_tokens::tests::cli_subprocess_exit_revoke`). The 7.4 `admit_cli_wrapper_journaled → CliWrapperShapeMismatch=27` path is unchanged and non-regressed (4/4). ADR-022 `ExitCause` disambiguates signal-death from exit-code death.

**AC3 — `maos run` + founder topology.** A `[cli_wrapper]` fork lives in `maos-bin` BEFORE `extract("class")` and calls `run_cli_wrapper_manifest` (composition root; kernel-core reads no manifest to decide topology). `maos run spirits/worker/manifest.toml --once` admits (respawn gate → tier grant → 7.4 shape probe), issues the cap-token, spawns the real fixture, journals 3 `CliSubprocessOutput` rows, revokes on exit. The founder-loop journey preserves the **full** Orchestrator→Worker(real CLI)→Architect→Reviewer→digest topology with the real worker.

> **Founder `[class]` Spirits — spec-vs-reality (flag for Winston/John).** AC3 said the three founder spirits "load through the existing class recipe once `classify_spirit` knows them." In reality they are pure deterministic spirits with **no `with_scalar_port`** and their manifests **omit `[capabilities.required]`** (and other sections the class recipe extracts); the orchestrator is `autonomous-with-halt` but consumes no scalar port, so a standalone boot-loud wiring would be theater. Resolution (FORK-B aligned): `classify_spirit` returns `LoadedSpiritKind::FounderLoopClass` for all three (classifiable ✓), and a standalone `maos run <founder>` **short-circuits with a directional error** to the founder-loop journey. The full topology runs end-to-end via the `smoke-founder-loop-8-4` journey (with the real worker) — that IS the AC3 "journey runs end-to-end with real subprocess Worker output" gate; standalone multi-Spirit orchestration is Epic 9 (FORK B). No silent theater.

**AC4 — J1.** `run_j1_bridge_measurement` drives a request→response round-trip through the REAL bridge against a deterministic `sh` echo CLI (zero work), warmup discarded, N=120: **P50=10µs / P95=12µs / P99=13µs / max=16µs** — ~2000× under the §13.1 25ms P95 budget. CI gate is the generous `J1_CI_CEILING_US = 50ms` (Tier-2 reported polarity, Murat); the strict 25ms is the pinned-runner gate. The synthetic `hello-spirit` floor is retained. The §13 J-Butler ≤ 4×J1 ratio requires J-Butler in the same harness run — noted, not asserted here (J1 alone is the 8.12 surface).

**AC5 — host-grant tier (FORK A, security-invariant — FLAG-Winston + sec-redteam).** NEW **generalized** `maos-domain/src/host_grant.rs` (`HostGrant`/`HostGrantAllowlist`/`StaticHostGrantAllowlist`/`resolve_tier_grant`/`TierGrantDecision`) — deliberately NOT CliWrapper-only so 8.14b/8.14c MCP drivers reuse it (Cross-Impact #4). `resolve_cli_wrapper_tier` keeps the T3 default-deny floor (`ECliWrapperRequiresT3`) AND consults the host allowlist; no match / request-above-grant / non-Linux → `ECliWrapperTierNotGranted` fail-closed, **no silent downgrade**. The artifact requests; the host grants (operator config, NOT in the manifest). **Enforced-vs-declared (residual #1):** 8.12 lands the **trust-direction gate + grant-config seam + fail-closed error** (the security-correctness core). The grant's `permitted_egress_destinations` are **declared**, not yet kernel-**enforced** — there is no T2 scoped-egress mechanism in the kernel (only `t3/`, verified); full enforced egress allowlisting is a follow-up (Cross-Impact #3). The deterministic fixture-CLI path is the spawn used here (direct, T3-declared); a live agent CLI would route through the T3 network-permitted container variant.

**AC6 — anti-theater + two-gate.** Hardened spawn-provenance: per-run-fresh nonce echoed by the child + the child's real PID in the journaled row + `child_pid != std::process::id()` + reaped (`antitheater_real_spawn_nonce_pid_and_reaped`). `ci_default_guard` asserts zero network + no real agent CLI on the hermetic path AND its trip-test proves it fails on `claude`/`opencode`/`gemini`/`kimi` and on a network request. Crash matrix: EOF+zero = NOT a crash; stdout-drain-before-death (3 lines journaled before a non-zero exit); signal death disambiguated. RespawnFresh negative assertion (`transfer_context` always false). **Tier-2 release-gate** recorded at `_bmad-output/test-artifacts/release-gate-8-12-tier-2-cli-wrapper.md` (OPEN — a named owner signs one real `claude`/`opencode` run through `maos run`; this is downstream of CI green, not "dev done").

**AC7 — kernel discipline (FLAG-Winston).** `maos-kernel-core/src` is intentionally NOT byte-identical (the charter delta). Re-measured baseline: **16263 production code lines** (tokei Code), **+729** over the pre-story measure; `git diff --stat` line-audit maps the delta ONLY to: `runtime.rs` bridge (+953 incl. its tests), `admission.rs` tier+respawn gates (+69), `mod.rs` re-exports (+11), `capability/mod.rs` scope-arm + `revoke_cli_subprocess_exit` (+30). The story's "from 15520" is 8.11's recorded number; my pre-story tokei measured 15534 (small measurement-basis delta, noted). **No** LLM/inference/provider type in any kernel-core file; **no** `maos run` orchestration in kernel-core (that stays in `maos-bin`). `cargo fmt -p` NOT used (7.5a lesson). Workspace stays **42** (`check-workspace-count` PASSED; reused `worker-cli-fixture`, no new crate). `abi-diff --base abi-baseline/v1-pre-bump.txt` PASSED (frozen `maos-spirit-abi` byte-untouched; the bare `abi-diff` "breaking change" is the no-base false-positive — Story-8.3 lesson). `kloc-check` fails on the pre-existing aggregate breach (77585 vs 20000) — bridge-neutral, same decomposition-in-flight RED 8.11 documented; the +729 crosses no new per-crate ceiling.

**Residual question dispositions:** (#1 enforcement depth) → live-CLI = T3-network-permitted variant; enforced egress allowlisting deferred, declared-not-enforced logged above (no silent gap). (#2 seam generality) → `host_grant` generalized for 8.14b/c. (#3 §6.7/ADR-021 contract-vs-code for deferred recovery) → one-line note for Winston: v0.9 ships escalate/respawn-fresh; respawn-with-context = v1.0/NFR-Rel-3, fail-loud on declare. (#4 output-shape registry) → NOT built; `EOutputShapeAdapterNotRegistered` left reserved; admission version-equality is the v0.9 contract. (#5 workspace count) → stayed 42.

### File List

**Modified — kernel delta (`maos-kernel-core`, charter-amended, FLAG-Winston):**
- `crates/maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs` — **the** bridge: `spawn_and_bridge`, `SpawnedBridge` (reader threads, framing, control channel, pump, finalize, Drop), `ExitCause`, `Backpressure`, `BridgeError`, `BridgeSpawnSpec`, `execute_recovery`, `ci_default_guard`; `argv_prefix_hash` + its 3 tests preserved.
- `crates/maos-kernel-core/src/lifecycle/cli_wrapper/admission.rs` — `reject_respawn_with_context`, `resolve_cli_wrapper_tier` (AC5 gate).
- `crates/maos-kernel-core/src/lifecycle/cli_wrapper/mod.rs` — re-exports of the new runtime + admission surface.
- `crates/maos-kernel-core/src/capability/mod.rs` — `scope_to_intent` `CliSubprocessSpawn → ProcExec` arm; `CapabilityRegistryAdapter::revoke_cli_subprocess_exit`.

**Modified — other crates:**
- `crates/maos-domain/src/cli_wrapper.rs` — `ECliWrapperTierNotGranted`, `ERespawnWithContextUnsupported` (Added to `#[non_exhaustive]` enum).
- `crates/maos-domain/src/lib.rs` — `pub mod host_grant`.
- `crates/maos-capability/src/cap_tokens/mod.rs` — `cli_subprocess_exit_revoke` test.
- `crates/maos-bench/src/harness/j1.rs` — `run_j1_bridge_measurement`, `J1BridgeConfig`, `J1_CI_CEILING_US` + the bridge-overhead test.
- `crates/maos-bin/src/main.rs` — `LoadedSpiritKind::FounderLoopClass` + classify; `[cli_wrapper]` load fork; `run_cli_wrapper_manifest`, `resolve_cli_binary`, `parse_sandbox_tier`; **DELETED** the hand-INSERT loop → real bridge spawn in the founder journey + anti-theater assertion + `ci_default_guard`.
- `spirits/worker/manifest.toml` — `recovery_policy` `respawn_with_context` → `respawn_fresh` (AC2).

**Added:**
- `crates/maos-domain/src/host_grant.rs` — NEW generalized host-grant allowlist (AC5 FORK A; reusable by 8.14b/c).
- `crates/maos-kernel-core/tests/cli_wrapper_bridge_8_12.rs` — 9 real-subprocess bridge tests (anti-theater, crash matrix, redaction-trap, cap-binding, admission gates, timing).
- `crates/maos-bin/tests/smoke_cli_wrapper_8_12.rs` — 2 daemon smokes (standalone `maos run` worker; founder-loop journey with real worker).
- `_bmad-output/test-artifacts/release-gate-8-12-tier-2-cli-wrapper.md` — AC6 Tier-2 release-gate checklist (OPEN).

### Change Log

| Date | Version | Description |
| --- | --- | --- |
| 2026-06-08 | 0.1 | Story 8.12 implemented: `runtime.rs` stdio bridge (spawn/reader-threads/framing/control/recovery/ADR-022), AC5 `host_grant` seam + tier gate, `maos run [cli_wrapper]` fork + hand-INSERT deleted + real worker subprocess, J1-over-real-bridge (P95=12µs), cap-token revoke lifecycle, anti-theater + `ci_default` guard + Tier-2 release-gate; workspace 42, abi-diff GREEN, kernel baseline re-pinned to 16263 (+729, line-audited). Status → review. |

---

## Story Context Validation — Open Questions for Author (resolved + residual)

**RESOLVED 2026-06-08 (party-mode preflight + Lunarpulse ruling):** FORK A → Winston host-grant; FORK B → recommended default; FORK C → Amelia (defer respawn-with-context).

**Residual (do not block dev start; surface at review):**
1. **AC5 enforcement depth** — does an enforced-T2 scoped-egress profile land in-story, or is the live-CLI path the T3-network-permitted variant with full egress allowlisting deferred? Dev + Winston decide; record in Completion Notes (Cross-Impact #3).
2. **Host-grant seam generality** — design it so 8.14b/8.14c MCP drivers reuse it, or flag the divergence (Cross-Impact #4).
3. **§6.7 / ADR-021 contract-vs-code note** for the deferred recovery policy (Cross-Impact #6) — Winston ruling or a one-line ADR note.
4. **Output-shape adapter registry** (§6.7 `cli-wrapper-template:<cli>:<shape>`) — **RESOLVED 2026-06-08:** the registry is a **named-but-unimplemented contract** — the typed error `EOutputShapeAdapterNotRegistered` and the `cli-wrapper-template:<cli>:<shape-version>` id format exist ([maos-domain/src/cli_wrapper.rs:38-40], doc-referenced in admission.rs:8 / manifest.rs:3504) but there is **no registry struct/lookup**. v0.9 contract = the admission **version-equality** check (7.4). 8.12 does **NOT** build the registry; leave the `EOutputShapeAdapterNotRegistered` variant reserved. (Registry = future work if/when per-CLI output adapters are needed.)
5. **Workspace member count** — default 42 (reuse `worker-cli-fixture`); confirm no new dev-only test-CLI crate is wanted (else 43 + sentinel).

### Review Findings (adversarial code review 2026-06-08)

- [x] **[Review][Patch] [P-1] Drop deadlock — self.rx not dropped before joining reader threads** `runtime.rs:733-750`. With `Backpressure::Block`, a reader blocked on `tx.send()` will never unblock because `self.rx` (the receiver) is still alive. `handle.join()` deadlocks. Fix: explicitly drop `self.rx` (make it `Option` and `.take()`) before the join loop.
- [x] **[Review][Patch] [P-2] pump_to_journal silently ignores journal write failures** `runtime.rs:554`. `let _ = journal.insert_frame_event_with_sender(...)` swallows errors. Add `journal_failures: u64` to `PumpOutcome`.
- [x] **[Review][Patch] [P-3] Unknown CliWrapperStdioShape variant silently downgraded to newline framing** `runtime.rs:run_reader()`. Violates no-silent-downgrade. Should return an error.
- [x] **[Review][Patch] [P-4] read_content_length doesn't validate the blank separator line** `runtime.rs:488-497`. Malformed frames cause silent data corruption. Add `if !blank.trim().is_empty() { return Err(...) }`.
- [x] **[Review][Patch] [P-5] Reader framing errors silently treated as EOF** `runtime.rs:run_reader()`. `Err` from framing functions sends `ReaderMsg::Eof` — error discarded. Add `ReaderMsg::FramingError` variant.
- [x] **[Review][Patch] [P-6] Missing test: Drop without pump** `cli_wrapper_bridge_8_12.rs`. No test exercises dropping the bridge without `pump_to_journal` when the child has filled the bounded channel. Would have caught P-1.
- [x] **[Review][Patch] [DN-1→annotation] StaticHostGrantAllowlist self-grant annotation** `main.rs:~334`. Team consensus (Winston + John): accept as v0.9 seam; add fail-loud comment stating the allowlist is self-populated from manifest values and Epic 9 MUST replace with operator-managed source. The architecture is correct for 8.14b/c reuse; the population source is the only gap.
- [x] **[Review][Defer] No sandbox enforcement on hermetic path** `runtime.rs:spawn_and_bridge` — deferred, pre-existing. Command spawn by design; live container = Tier-2.
- [x] **[Review][Defer] Cap-token mediation failure continues to spawn** `main.rs:371-388` — deferred, pre-existing. Layered security design; host-grant already passed.
- [x] **[Review][Defer] on_pause/on_resume are documented v0.9 no-ops** `runtime.rs:692-700` — deferred, pre-existing.
- [x] **[Review][Defer] on_unload always SIGKILLs** `runtime.rs:722` — deferred, pre-existing. `#![forbid(unsafe_code)]` prevents SIGTERM.
- [x] **[Review][Defer] Recovery executor not wired in composition root** `runtime.rs:814-849` — deferred, pre-existing. Epic 9 seam.
- [x] **[Review][Defer] argv_prefix_hash check is tautological** `runtime.rs:398-400` — deferred, pre-existing. Catches misconfiguration.
- [x] **[Review][Defer] Backpressure::Block + slow journal stalls child** `runtime.rs:438-441` — deferred, pre-existing. Design intent.
- [x] **[Review][Defer] recv_line doesn't filter by stream** `runtime.rs:647-653` — deferred, pre-existing. Current bench never writes stderr.
- [x] **[Review][Defer] No egress enforcement for live CLIs** `host_grant.rs` — deferred, pre-existing. Acknowledged residual.
- [x] **[Review][Defer] wait_and_finalize exit row failure + cap-token revoke gap** `runtime.rs:607-618` — deferred, pre-existing.
- [x] **[Review][Defer] resolve_cli_binary no execute-permission check** `main.rs:246-268` — deferred, pre-existing.
- [x] **[Review][Defer] StaticHostGrantAllowlist self-grant (v0.9 seam)** `main.rs:334-336` — deferred, pre-existing. Operator-managed allowlist = Epic 9. Annotation added per team consensus.
