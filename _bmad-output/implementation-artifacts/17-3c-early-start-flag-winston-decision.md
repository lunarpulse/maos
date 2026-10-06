# 17-3c early-start and FLAG-Winston decision record

**Date:** 2026-10-01  
**Status:** `review` — detached candidate implementation/amended acceptance complete; independent review pending. Initial 2026-10-01 preparation/backlog disposition is historical and superseded below.
**Scope:** architecture and executable preparation only. This record changes no runtime, kernel, baseline, or KLOC ledger.

## Decision

| ID | Decision | Status |
|---|---|---|
| **D-17-3c-ES-01** | Start **17-3c preparation** after the verified 17-3b contract/admission snapshot and 17-6's four-image T2 evidence. This is not an assertion that 17-3b is done: its outstanding kernel-verdict finding stays open. | **Approved** |
| **D-17-3c-ES-02** | The missing proof is owned by **17-3c AC2** because only its runner-output-to-kernel ingress can observe a framed spawned session at the real authorization decision. Frame preservation in 17-3b's `equiv_harness` is not that observation. | **Approved** |
| **D-17-3c-ES-03** | Move WASM conformance compilation/instantiation into the T2 runner before launch. The daemon may inspect artifact metadata and enforce the pre-launch size bound, but it MUST NOT instantiate guest code. | **Approved** |
| **D-17-3c-ES-04** | A FLAG-Winston is an architectural location charter, not a numerical grant. 17-3b remains frozen at kernel pin `25032`; any 17-3c kernel or KLOC grant requires an isolated measured prototype and explicit operator approval. | **Approved policy; explicit candidate numerical/surface grants subsequently approved — see current disposition** |
| **D-17-3c-ES-05** | 17-3d recall is excluded from this preparation and is not a prerequisite. The contained surface is launch, framed ingress/egress, identity/authority binding, conformance, and T2 evidence only. | **Approved** |

## Evidence and boundary

The preserved 17-3b patch snapshot implements the necessary contract: `maos:spirit@2.0.0`, lossless bridge fields, typed runner exits, form admission, and the `wasm_launch_not_built` refusal. It is not merged or complete. Its review finding remains unchecked because `gate_manifest` returns `WasmEngineOff` or `WasmLaunchNotBuilt` before a WASM process can emit into a kernel authorization decision. This record does not alter that finding, its status, or 17-3b acceptance.

17-6 is the launch prerequisite, not a substitute for 17-3c proof: `spawn_and_bridge` applies an admitted T2 `SandboxSpec` through Linux `spawn_sandboxed`; its CI evidence covers ubuntu-24.04/ubuntu-26.04 on x64 and arm64. It proves the sandbox route and refusal-state evidence, not a live WASM bus session.

Current source evidence establishes the architectural starting point:

- `crates/maos-host/src/lib.rs` defines the form-specific `SpiritHostPort` boundary while the kernel's `BridgeSpawnSpec` remains form-agnostic.
- `crates/maos-wasm-host/src/adapter.rs:56-103` accepts a file, applies a 64 MiB metadata cap, then calls `probe_component`; `conformance.rs:22-52` reads, compiles, and instantiates it in a daemon-owned thread. That is the unsafe execution boundary to move.
- `crates/maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs:248-280,477-524` already accepts an admitted `SandboxSpec`, pipes stdio, and routes T2 to `spawn_sandboxed`; it currently has no framed, identity-bearing session protocol.
- `crates/maos-kernel-core/src/security/sandbox/linux.rs:45-79,168-250` establishes the required pattern: all allocations and filesystem preparation occur parent-side; the `pre_exec` closure performs only prepared, non-allocating sandbox steps.
- The real existing consent decision is `CrossTeamCrossingAdapter::apply_crossing` → `CrossTeamConsentPort::is_granted` (`crates/maos-bin/src/cross_team_crossing.rs:333-565`), which refuses `ConsentDenied` at `:536-547`. In-band `IacFrame.consent_envelope` is parsed separately at `:93-130`; it is not evidence that the trusted authorization decision was reached.

## 17-3c AC ownership

### AC2 — framed spawned session, identity, authority, and verdict observation

17-3c AC2 MUST create one long-lived spawned-class session, allocated by the scheduler before launch. It owns all of the following as one causal chain:

1. Allocate the scheduler SCB and its `spirit_pid`; bind that identity and the admission-derived `SandboxSpec` to the runner endpoint before any guest frame is accepted. Guest bytes MUST NOT select either value.
2. Replace text-line bridge handling for this form with length-delimited ADR-032 frames through one shared codec. The bridge MUST attribute every accepted runner output to the already-bound `spirit_pid`, retain provenance from the refusal/launch transition, and deliver it to the selected real authorization decision.
3. Observe both the authorization input and its allow/refusal result from that live path, then journal the result under the bound identity. A frame that parses, round-trips, or is merely written to stdin has not reached this criterion.
4. Reuse `WorkerSupervision`/pidfd and emit a halt receipt for every terminal runner exit. The `2 IncompatibleWorld`, `3 InvalidComponent`, and `5 UnrepresentableFrame` exits remain typed pre-/launch refusals; none may fall through as an authorized frame.

### AC3 — conformance and safe launch

17-3c AC3 MUST make the T2 runner, not the daemon, compile and instantiate guest code. The daemon enforces the artifact regular-file/metadata policy before launch. Safe launch also requires a runner/guest resource boundary that bounds Wasmtime's allocation of a guest return value **before outbound serialization**; record the enforcement mechanism, configured limit, and hostile-guest observation in 17-3c. Bounding host-to-runner input or length prefixes does not establish this property. The existing 16 MiB outbound codec cap remains a separate serialized-frame bound, not the guest allocation bound.

The launch is safe only when the admission grant, resolved artifact read scope, frame bounds, and runner argv are all fixed before `spawn_and_bridge`; a conformance error is returned as a typed refusal with no bare fallback. The daemon-side probe and its timeout-thread pattern are removed rather than retained as a second execution path.

### Refusal-to-launch provenance

The cutover replaces only the final WASM `WasmLaunchNotBuilt` branch after all structural admission gates have succeeded. The evidence is ordered, not retrospective:

- **before cutover:** record a CI refusal-state snapshot in which the `wasm-host` build reaches `wasm_launch_not_built`;
- **after cutover:** record a downstream CI snapshot from the same valid manifest proving the admitted T2 runner session, scheduler-assigned identity, framed journal rows, and observed authorization result;
- retain both snapshots with their immutable revision/run identifiers. The launch snapshot does not retroactively turn the refusal snapshot into a launch, and the refusal snapshot does not substitute for the later verdict.

The before-cutover evidence is now [run `36860382516`](https://github.com/lunarpulse/maos/actions/runs/36860382516), **success**, at CI revision `4c1b561d9fcb4f973ec2194220397d79593e36af`. Its underlying patch snapshot is `c544b66ee206122a0de001f88c7be95a03017873`; only the isolated evidence workflow differs. Public annotations name both actual CLI refusals and the `25032` kernel baseline. The run/API/annotation provenance is retained in `17-3b-evidence/post-review-ci-*`; it does not prove a launched session, guest allocation containment, or kernel-verdict equality.

Default-build `wasm_engine_off`, malformed manifest/component refusals, and runner exits 2/3/5 remain refusals. No test may relabel a refusal as launch merely because the process binary started.

## Required executable verdict gate

The later implementation MUST add one integration gate that feeds native and WASM frames through the same real kernel authorization decision. `CrossTeamCrossingAdapter::apply_crossing` → `CrossTeamConsentPort::is_granted` is an existing consent check to investigate, not established evidence of kernel authorization. Before selecting the observation point, preflight MUST trace the production call chain to the kernel enforcement decision, name its trusted state source, and identify the equivalent compiled mutation there. Observing only this adapter's predicted or preflight result does not satisfy the 17-3b finding.

The gate's trusted authorization state, not `consent_envelope` or `intent` supplied by the frame, determines the expected result:

| Form | Trusted state | Frame claim | Expected verdict |
|---|---|---|---|
| Native | allow | clean | allow |
| Native | allow | forged | allow |
| Native | deny | clean | deny |
| Native | deny | forged | deny |
| WASM | allow | clean | allow |
| WASM | allow | forged | allow |
| WASM | deny | clean | deny |
| WASM | deny | forged | deny |

A passing gate MUST also prove all of the following:

- **Reachability:** it records eight invocations and eight terminal authorization verdicts at the selected decision symbol. Any runner refusal, parse error, `NotCrossing`, skipped case, or missing observation fails the gate; it cannot become a green non-applicable result.
- **Nonvacuity:** both allow and deny occur at least once for each form, and clean/forged pairs are compared only with the same trusted state.
- **Cross-form equality:** native and WASM verdicts MUST agree for each equivalent trusted-state/claim case, not merely within each form's clean/forged pair. The deny-side forgery MUST claim authority that would grant the request if trusted; only admissible frames reaching the authorization decision count.
- **Mutation proof:** in a disposable compiled snapshot, mutate the selected real authorization branch to trust forged authority or bypass one trusted deny. Rebuild and run the gate; the relevant deny cases MUST turn red. A mock, test double, parser mutation, assertion removal, or source-text grep is not a substitute. An adapter-only bypass is sufficient only if preflight establishes that it changes the actual kernel-enforced outcome; a compile failure is not the required RED.
- **Staged-work preservation:** create the snapshot from `HEAD` plus `git diff --binary HEAD`, apply it only in a temporary detached worktree, compile and mutate there, and delete only that temporary worktree. The working tree/index is never reset, stashed, or modified by the proof.

The original four-image requirement is superseded by the user's explicit acceptance of completed **Ubuntu25.10 x86_64** testing as sufficient. Additional images/platform-specific repetitions are not required or claimed verified. No recall assertion, socketpair, `sendto`, `recvfrom`, inherited descriptor, or WIT `@2.1.0` work is added; those remain 17-3d.

## FLAG-Winston policy

| Area | Historical evidence at initial preparation | Architectural authorization | Initial numerical grant / implementation state (superseded below) |
|---|---|---|---|
| `crates/maos-bin/src/` (new spawned-session integration beside existing admission/composition-root paths) | `admission.rs:669-728` creates the scheduler pid then derives an admission `SandboxSpec`; `main.rs` owns launch composition. | AC2 may design the session, identity binding, provenance, and runtime wiring here. | Pending implementation measurement and normal crate-ceiling check. |
| `crates/maos-wasm-host/src/adapter.rs`, `conformance.rs`, runner/frame codec | Adapter currently probes/instantiates in the daemon; runner owns the component execution shape. | AC3 may move conformance to the runner and define bounded control/framed transport. | Pending implementation measurement; no numerical allowance is inferred. |
| `crates/maos-host/src/lib.rs` | `SpiritHostPort` already resolves a form to a runner plan while preserving a form-agnostic kernel bridge. | Reuse it unless the prototype demonstrates a missing seam. | No change authorized merely by this record. |
| `crates/maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs` | Existing `BridgeSpawnSpec`, pipe setup, and T2 routing are the only current launch primitive. | Candidate only for the minimal framed bridge seam if the isolated prototype proves user-space composition cannot preserve the AC2 invariant. | **Pending** FLAG-Winston grant; no edit or baseline re-pin authorized now. |
| `crates/maos-kernel-core/src/security/sandbox/linux.rs` and `security/mod.rs` | Parent-side preparation/no-allocation `pre_exec` rule; existing `classify_exit`/`emit_sandbox_block` primitive. | Candidate only for a proven required T2 syscall/audit integration, with leave-one-out evidence. | **Pending** FLAG-Winston grant; no edit or baseline re-pin authorized now. |

`xtask/kernel-core-baseline.toml` remains **`src_lines = 25032`**, the 17-3b zero-delta pin. A future 17-3c prototype may request a new grant only after it records: exact changed symbols/files, exact physical source delta and KLOC measurements, an isolated before/after runnable proof, four-image T2 outcome, mutation-red evidence, dependency-closure result, and the operator's explicit approval. The operator then decides whether to grant and re-pin in the same implementation change. This record grants none of those items and permits no baseline raise.

## Preparation gates and disposition

| Gate | Approval now | What remains pending |
|---|---|---|
| Contract/admission input | 17-3b's verified contract and refusal snapshot may be used to prepare 17-3c. | 17-3b remains `in-progress`; its open finding is neither transferred nor waived. |
| T2 route | 17-6's four-image evidence is a prerequisite satisfied for preparation. | 17-3c must prove the actual runner session on the same four images. |
| Launch cutover | Define the before/after refusal-to-launch provenance now. | Implement and observe both states with the same valid manifest. |
| Authorization verdict | Define the eight-case, nonreach-failing, compiled-bypass gate now. | Build AC2 ingress and run the gate against the real decision. Only then may 17-3b's finding close. |
| Conformance safety | Define daemon metadata-only and runner-only instantiation now. | Implement AC3 and prove no daemon guest instantiation remains. |
| Kernel / KLOC | Define named candidate locations and a grant protocol now. | Isolated measured prototype plus explicit operator approval; no numerical grant exists. |
| Recall | Explicitly excluded. | 17-3d after 17-3c; it is not a gate here. |

This resolves the scheduling cycle: 17-3c preparation depends on the verified contract and T2/refusal evidence, while 17-3b's status and its kernel-verdict finding depend on a later 17-3c runtime proof. It is therefore contract-ready for preparation, not ready-for-development or done.

## Isolated executable preflight (2026-10-01)

The detached prototype used the verified patch snapshot `c544b66ee206122a0de001f88c7be95a03017873`. Only its disposable source retains changes; no production cutover or numerical grant was applied. Retained patch, measurements, commands, and execution logs are in `17-3c-evidence/`.

### Selected kernel oracle and production constraint

The actual capability-issuance decision is `CapabilityRegistryAdapter::issue_with_mediation` (`capability/mod.rs:171-202`) → `PolicyTable::evaluate` (`capability/cap_policy/mod.rs:114-226`). The former derives the policy intent from the requested `Scope`; guest `IacFrame.intent` and `consent_envelope` are not trusted policy state. `PolicyDecision::Deny` returns `CapError::PolicyDenied` at `capability/mod.rs:191-193`; an unknown trusted pid fails closed. Production composition must reuse `worker_spawn::issue_enterprise_governed_capability` (`maos-bin/src/worker_spawn.rs:313-369`), which preserves SSO/PDP before kernel mediation. Adding a second direct kernel mint would bypass that governance.

This selects a real authorization oracle, not an existing frame dispatcher: no production guest-output-to-authorization ingress exists yet. AC2 must still specify the authorized operation represented by each request and bind its input to the scheduler session. `CrossTeamCrossingAdapter` alone cannot establish this kernel outcome.

### Observed executable results

| Probe | Observation | Retained evidence |
|---|---|---|
| Raw framed native bridge | Non-UTF8 CBOR, empty/back-to-back frames, 70,000-byte payload, diagnostics, shape/closed-input errors, and malformed framing passed. The combined kernel suite passed **4/4**, including actual T2 runner startup. | `integrated-green-before.log` |
| Actual T2 runner, no `memfd_create` | Compilation/instantiation refused with `cannot create a memfd`; runner exit **3**, test exit **101**. | `t2-memfd-omission-red.log` |
| T2-only `memfd_create` allowance | The same real runner/component exited **0**. | `t2-memfd-allow-green.log` |
| Native T0 and WASM T2 framed bridge → kernel oracle | Both real processes emitted four frames and exited **0**. Eight clean/forged × trusted allow/deny observations matched across forms. | `integrated-green-before.log`, `integrated-restored-green.log` |
| Compiled kernel deny bypass | Replacing the actual `PolicyDecision::Deny` return with an empty arm compiled; native deny case 2 incorrectly allowed and failed the verdict assertion, exit **101**. Restoring the branch restored all eight GREEN verdicts. | `integrated-kernel-bypass-red.log`, `integrated-restored-green.log` |
| Dependency closure | `maos-kernel-core` and `maos-domain` passed, zero forbidden dependencies. | `dependency-closure.log` |

The trusted fixture admits pid `7` and leaves pid `8` unknown; all requests use the same `FsRead("/tmp")` scope. This avoids an invalid subtree-denial assumption: current policy scope matching compares enum discriminants, not FsRead subtree values. `FsRead` is test-seeded, not a production manifest capability grant. The gate manually supplies trusted pid/class context after decoding real process output; it does **not** prove scheduler identity binding, enterprise composition, production bus delivery, or policy enforcement during guest execution. These are explicit AC2 gaps, not passed acceptance criteria.

### Measured minimum seam, not a full implementation grant

| Counter | Original | Prototype | Delta |
|---|---:|---:|---:|
| Kernel `src/` physical Rust lines | 25,032 | 25,127 | **+95** |
| `tokei 14.0.0` Rust code, tests/benches excluded | 19,427 | 19,495 | **+68** |
| Kernel `src/` Rust files | 97 | 97 | 0 |

The source delta is confined to `lifecycle/cli_wrapper/runtime.rs` (**+100/-8**, net **+92** physical lines) and `security/sandbox/linux.rs` (**+3**): raw frame writer/receiver, graceful input EOF, Content-Length stdin retention under Signals, stdout binary framing with stderr diagnostics, surfaced framing errors, and the T2-only syscall allowance. The kernel remains form-agnostic and acquires no Wasmtime dependency. Test-only additions are excluded from these source measurements.

The table's raw tokei counter is not the `xtask kloc-check` counter, which also excludes `memory/spill_test_faults.rs` (`xtask/src/kloc_check.rs:312-316`). Original-tree governance passed with kernel **19,309**, aggregate **168,632**, zero over-budget crates, and `alarm: true`; that advisory alarm is not claimed cleared. Kernel baseline/content hashes, epic-close coherence, and dev-record completeness passed. These results are retained in `governance.log`; no complete-candidate prototype KLOC gate or four-image grant is claimed.

This is a measured **minimum feasibility seam**, not an estimate of complete AC2/AC3. The prototype still duplicates framing codecs, has no bounded kernel frame allocation, retains lossy text journaling, and lacks production SCB/session wiring, enterprise mediation, runner-only conformance cutover, halt receipts, and pre-serialization guest-allocation containment. Local execution reported no writable cgroup and used the existing setrlimit fallback. Only this Linux x64 host was exercised; the four CI images, arm64 behavior, hostile-runner SIGSYS proof, resource enforcement, and warning-free gate remain unproven. No baseline/KLOC re-pin or numerical grant is justified by these measurements alone.

### Preservation and disposition

The prototype edit initially touched the original `runtime.rs` by a relative-path mistake. That edit was reversed before the absolute-path prototype patch; the original blob was independently verified as `551af28ef9bfbaa4607194e332c0cc5af39fcafa`, identical to the verified snapshot. The restored prototype policy blob equals the original policy blob. Main HEAD remains `5075443c8a87523581cfcf6cb0b6af815f7ebd4f`, staged index tree remains `4fd937d15144c0a01a934d072b90a1d27215cbe2`, and the authoritative kernel baseline remains **25032**. `measurements-and-preservation.json` records this evidence.

Historical preflight disposition (2026-10-01): complete isolated AC2/AC3 implementation, safe conformance/resource proof and explicit measured grants were still required; the raw prototype did not close 17-3b or move 17-3c out of backlog. Superseded by the completed candidate and current handoff below.

## Current disposition and review handoff (2026-10-05)

- 17-3b and 17-3c are `review`, not `done`. Review the detached `/tmp/maos-17-3c-candidate-wecx8ef7`; original checkout retains the refusal runtime/kernel25032 with tracking documents updated only.
- Implementation, eight-case admitted-session kernel/enterprise proof and compiled mutation RED/GREEN are complete. The non-waived 17-3b finding awaits independent verification, not new ingress implementation. See candidate specification and `17-3c-evidence/candidate-grants-and-verification.json`.
- Explicit candidate grants approved: physical kernel25149/98 files, exact KLOC kernel19412/bin24213/IAC7088/frame-codec218, separate surface deltas; aggregate169654 below unchanged170884 hardfail. No original baseline/ledger re-pin.
- Approved CPU10%/768 MiB process/360 s time-and-compile/64 MiB guest/fuel10M actually passes Ready, mailbox delivery, attributed audit and clean receipt on Ubuntu25.10 x86_64. Same-manifest original feature-on refusal/candidate launch and actual SQLite are in the candidate evidence bundle.
- User accepts completed Ubuntu25.10 x86_64 testing as sufficient. Ubuntu24/26, ARM and extra platform-specific syscall controls are not remaining acceptance gates; unexecuted checks are not labeled passed. Historical 17-6 CI evidence is unchanged.
- Next: independent candidate review, including security and non-author verification. No original closure, runtime cutover, commit, push/publication or deployment; 17-3d stays backlog and outside scope.

