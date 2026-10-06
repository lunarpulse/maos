---
story_key: 17-3c-wasm-spirit-on-the-bus-under-t2
status: review
updated: 2026-10-05
dev_model_used: openai-codex/gpt-6.1-sol
baseline_commit: 5075443c8a87523581cfcf6cb0b6af815f7ebd4f
candidate_snapshot_commit: 72077a4e991561efd189779598adc1cbd34814c8
review_root: /tmp/maos-17-3c-candidate-wecx8ef7
---

# 17-3c — A WASM Spirit runs on the bus, contained by T2

Status: done

## Current scope and closure

Done 2026-10-06. The §A6 bmad-code-review is complete: every finding was patched, decided or deferred with an owner.
The detached candidate's source and evidence are now integrated into the original checkout, and the
launch, kernel re-pin and approved grants land with this story's commit. The approved intent remains in
`spec-17-3c-isolated-runtime-candidate.md`, whose frozen block is unchanged.

Dependencies:
- **17-3b:** `done`. Its final kernel-verdict finding was independently verified by this review,
  using the eight-case gate and a fresh compiled deny-bypass mutation (RED, then GREEN on restore).
- **17-6 T2 route:** completed.
- **Out of scope:** 17-3d recall, socket/fd imports and WIT @2.1.0.

User-approved platform amendment: “I mean ubuntu 25.10 x86_64 is sufficient testing
that. record and move on.” The completed Ubuntu25.10 x86_64 proof is sufficient;
no Ubuntu24/26 or ARM execution or extra image-specific syscall repetition is
required. Unexecuted controls are not claimed passed. Other stories' requirements
and the broader repository CI matrix remain separate.

As an operator, I can admit a WASM-component Spirit, run it in its admitted T2
runner, exchange ADR-032 frames through the scheduler mailbox under its trusted
identity, and observe governed decisions and durable terminal/audit receipts.

## Acceptance criteria and evidence

All evidence paths in this section resolve **inside the candidate checkout**, not
the original checkout's earlier raw-preflight evidence directory.

1. **Actual TS CLI launch and lifecycle:** release `maos run … --once` admits the
   first-party TS component through the scheduler, reaches Ready, handles its
   mailbox turn and attributes audit to the bound Spirit identity. Final actual
   Ubuntu25.10 x86_64 run: PID1/child5709, one turn and delivered frame, Standard /
   SpiritAuto authorization, exit0 non-crash receipt, unload/revocation and owned
   cgroup cleanup; elapsed323.9006 s. Evidence:
   `17-3c-evidence/remote-100-20-final-isolated.json` and its SQLite archive.
2. **Trusted spawned-session authorization:** native and WASM manifest grant/deny
   × clean/forged cases traverse the same current kernel issuance policy through
   real configured permitting Cedar. Eight decisions are nonvacuous and equal
   across forms, without manual PID/FsRead policy seeding. Separate actual Cedar
   forbid and mixed-scope cases prove atomic refusal with no effect. Guest identity,
   intent and consent claims are untrusted; each delegated scope requires current
   kernel entitlement. The compiled deny bypass is RED709; restored GREEN711;
   final six process gates GREEN771. Gate:
   `crates/maos-bin/tests/spawned_scope_17_3c.rs`. Policy compares scope kinds;
   **no MCP server/tool selector-level authorization is claimed**.
3. **Safe runner and pre-serialization resource bounds:** conformance and all guest
   compilation/instantiation/export run in the contained runner, not a daemon probe
   thread. Shared framing is bounded before allocation; hostile amplifying/aliasing
   guest returns exhaust lifting fuel before serialization, with setter-removal
   RED/GREEN evidence in `17-3c-evidence/hostile-lifting-mutation.json`.
   Approved actual contract: CPU10%, process768 MiB, time/compile360 s,
   guest64 MiB, instruction fuel10M. Final proof reads child `cpu.max=10000 100000`
   and `memory.max=805306368`. VM/RSS peaks are605380/509420 KiB; these are
   whole-runner peaks, not guest-memory high-water counters. TS capacity inspection
   and successful instantiation are in `ts-resource-capacity.json` and final proof.
4. **Containment, diagnostic and terminal controls:** the actual debug-only runner
   SIGSYS control reaches Ready, dies on ptrace with SIGSYS31, persists primitive
   SandboxBlock and feeds the production escape consumer. Planned termination
   produces neither SandboxBlock nor an anomaly. Idle/repeated mailbox behavior
   does not falsely create an active-turn stall. Release fault injection is absent.
   Existing local memfd omission and SIGSYS evidence is retained; extra platform
   runs are excluded by the explicit amendment, not faked as passing.
   Default/unsupported loading paths remain typed refusals with no bare fallback.
5. **Mutation and genuine revision provenance:** compiled kernel-deny and hostile
   lifting mutations turn the corresponding consumer-visible checks RED; restored
   source is GREEN. Original feature-on refusal `wasm_launch_not_built` and actual
   candidate launch use the identical approved manifest/component, not merely a
   feature-off control. Evidence: `matched-refusal-launch-360.json` and
   `preserved-refusal-revision-360.json`. Final CLI SHA
   `71297d7d08eee1ef42a276b2be6193fda0c63e8adc3a8e5f7ffd2f6d7d1ddd88`, runner SHA
   `521869ef96d1bf983f50cdf3c1d383a0db0a5983db6666ec30bd200647d25629`.
6. **Explicit grants and governance:** candidate physical kernel25032→25149
   (+117), 98 files, hash
   `76dec9f7563007168134157c728ea2611855beea144012ebca9857c9e5014eeb`;
   exact zero-headroom KLOC kernel19412/bin24213/IAC7088/new frame-codec218.
   Aggregate169654 is below unchanged170884 hardfail; the existing158608 alarm
   remains visible. Surface classifications/ownership rehash/additive
   CgroupUnavailable were separately approved; no aliases. Host surface gate
   permits two Timeout removal entries without a re-pin. Numerical, empty-kernel,
   dependency/service/workspace57/stability/template gates and kernel262 units
   passed. `candidate-grants-and-verification.json` is the current summary.

## Completed candidate tasks

- [x] Extract std-only bounded framing leaf and migrate its consumers.
- [x] Implement form-neutral duplex bridge, bounded LF diagnostics and primitive SIGSYS audit.
- [x] Move conformance to the contained runner; remove obsolete daemon probe/timeout API.
- [x] Enforce whole-runner, guest, frame and pre-serialization lifting limits.
- [x] Bind scheduler-owned admission identity and active-turn supervision.
- [x] Implement governed sessions, scope atomicity, mailbox delivery and typed loading refusals.
- [x] Migrate explicit trusted sender attribution and all affected consumers.
- [x] Exercise real enterprise/kernel, hostile-resource, lifecycle and SIGSYS gates/mutations.
- [x] Exercise actual approved CPU10 TS CLI and matched original-refusal/candidate-launch pair.
- [x] Record explicit numerical/surface/resource/platform approvals and migrate candidate docs/CI.

### Review Findings

Chunk A (2026-10-05): `Cargo.toml`, `crates/maos-bin/**`, `crates/maos-kernel-core/**`,
`crates/maos-frame-codec/**`, diffed against candidate HEAD `72077a4e` plus the untracked
files. Paths are relative to the candidate checkout. Chunks B (wasm-host, host, spirit-hello,
bench, iac, scripts) and C (CI, xtask, docs, templates, `_bmad-output`) are reviewed separately.

Chunk A patches were applied in the candidate on 2026-10-05; nothing was staged or committed.
Re-verified locally (Debian 13 container on a CachyOS x86_64 host; the runtime AC1 proof below ran on the accepted Ubuntu 25.10 x86_64 host):
- `spawned_scope_17_3c`: 15 passed (final tree), including the chunk-A reworks, the B/C gates and `a_delivery_needs_its_iac_send_grant`.
- Kernel bridge tests: `frame_bridge_poll_17_3c` 4/4 (new), `bridge_t2_route_17_6` 5/5, `cli_wrapper_bridge_8_12` 9/9.
- Unit and admission suites: kernel-core lib 262; frame-codec 3 + `tests/bounds.rs` 7; maos-bin lib 21; `wasm_form_admission_17_3b` and `cookbook_manifests_17_3b` in both feature sets.
- Downstream crates: orchestrator and architect spirit tests; maos-wasm-host/iac/host suites.
- `cargo check --workspace --all-targets`, with and without `wasm-host`.
- Mutation: deleting the lineage restamp turns `forged_guest_provenance_is_restamped_by_the_kernel` RED; restoring it is GREEN.

**Governance (updated 2026-10-06; measured after `cargo fmt --all`, tokei 14.0.0):**
- **Re-pinned under the operator's KLOC sign-off (2026-10-06):** maos-kernel-core 19412 → 19529, maos-bin 24213 → 24544, maos-frame-codec 218 → 249 (`xtask/kloc.toml`). These are the chunk A + B/C patch values as formatted; the +9/+1 over the figures shown for approval is formatting of the same code.
- **Re-pinned under the operator's KLOC-delta approval (2026-10-06, option-1 `iac.send` mediation):** maos-bin 24544 → 24600 (+56), maos-iac 7088 → 7100 (+12). Aggregate 170433 stays under the 170884 hard-fail. `kloc-check` passes.
- **Kernel surface re-pinned under the operator's approval (2026-10-06):**
  - The `SpawnedBridge` signature hash was refreshed (private `stdout_eof`; public `kill`).
  - `maos_kernel_core::capability::is_mediated_scope` was added to `docs/ci-baselines/kernel-surface-v0.1-beta.json` and classified `universal-arithmetic` (a pure `Scope → bool` predicate) in `xtask/kernel-api-classes.toml`.
  - `check-service-boundary` passes.
- **Kernel baseline re-pinned under the operator's approval (2026-10-06):** `src_lines` 25149 → 25293 (still on line 505); 98 files; set hash `509d311362def7ddeb028afed49e6d59e67b9d20f74f209fc66ae9a45e8ee612`. The per-file digests come from `check-kernel-baseline --emit-pin`, with a HISTORY entry in `xtask/kernel-core-baseline.toml`. Both kernel pins (line count and surface) moved together, per the E16-A1 rule.
- **All governance gates pass in the candidate:** kloc, kernel-baseline, service-boundary, empty-kernel, host-surface, dependency-closure, workspace-count, abi-ratification, stability, templates, dev-model-tier, dev-model-used, dev-record, bare-review-findings.
- **xtask tests:** the full suite passes with `--no-fail-fast`. Two tests carry the pins as literals: the pin-reader test `kernel_pin_content_hash_16_0` (25032) and the ceiling test `recovery_lane_ceiling_rule` (19309).
  - They were already red in the unpatched candidate, because the earlier candidate grant had not moved them.
  - They now carry the approved 25293 / 19529, with the history comment, so they still flag any unapproved change.
- **Durable record:** `17-3c-evidence/review-2026-10-06/verification-summary.json` (working-tree content hash `a89ec315…`, 5253 files).

- [x] [Review][Patch] (from Decision D1) Rebuild guest-frame provenance in the kernel [`crates/maos-bin/src/spirit_session.rs:828-866`] (high). Done:
  - Lineage is taken from the turn's inbound frame.
  - `consent_envelope` is cleared on delivery; the guest's value stays only in the authorization `claims`.
  - The kernel stamps a Lamport `logical_clock`.
  - `host_id` destinations are refused with `cross_host_destination`.
  - Tests: the forged-envelope eight-case matrix and `forged_guest_provenance_is_restamped_by_the_kernel`.
  - **Amendment:** guests addressing themselves are NOT refused. ADR-018 / architecture §7.3.2 define same-Spirit frames as legitimate, so refusing them would break an existing contract and every echo/TS component. The deadlock is closed by bounded delivery instead, proven by the self-flood gate.
- [x] [Review][Patch] (from Decision D2) Enforce the frame-kind contract [`crates/maos-bin/src/spirit_session.rs:640-689`] (high). Done:
  - Kernel-to-Spirit kinds are refused with `kernel_reserved_kind`.
  - A guest `Retract` goes through `iac.retract()` under the trusted sender after an authority pre-check (`retract_*` denials).
  - Test: `kernel_only_cross_host_oversized_and_foreign_retract_frames_are_refused`.
- [x] [Review][Decision→Dismiss] `memfd_create` on the T2 allow-list is a contracted change, not drift. Epic 17 AC4 says "`memfd_create` joins the T2 allow-list with its leave-one-out proof", and R17-46 confirms it. Residual risk (memfd + `execve` running a memory-backed executable inside T2) is accepted by that contract. Narrowing it to the runner profile would be a new requirement for a future sandbox story. [`crates/maos-kernel-core/src/security/sandbox/linux.rs:327`]
- [x] [Review][Patch] Spirit test crates no longer compile — fixed: all callers migrated to `deliver_typed(frame, pid)`; orchestrator 4/4 and architect 1/1 pass. [`spirits/orchestrator/tests/distillate_dispatch.rs:134`] (high)
- [x] [Review][Patch] A guest can hang the session forever — fixed: `deliver_bounded` caps delivery at the turn deadline and checks `is_stopping`; test `guest_floods_end_in_typed_failures_not_a_wedged_session` (self-flood → `Deadline("delivery")`). [`crates/maos-bin/src/spirit_session.rs:510-537`] (high)
- [x] [Review][Patch→Dismiss] Authorization row is fire-and-forget — false positive on re-check. `insert_frame_event_with_sender` is the I2 write-or-panic path (`maos-iac/src/adapter/transparency_log.rs:669-673`): a failed write halts and cannot return without the row. `let _` only discards the typestate. The verdict row is still written before delivery. [`crates/maos-bin/src/spirit_session.rs:833-844`] (high → none)
- [x] [Review][Patch] Vacuous release-flag assertion — fixed: the release runner is given the real component. Control: Ready and exit 0. With the flag: exit 1, no stdout, and `unknown argument: --test-forbidden-syscall-after-ready`. [`crates/maos-bin/tests/spawned_scope_17_3c.rs`] (high)
- [x] [Review][Patch] Unbounded guest output — fixed with these limits:
  - `MAX_FRAMES_PER_TURN` (64) ends the session with a typed `Protocol` error.
  - `MAX_SCOPES_PER_FRAME` (16) produces a `scope_limit` denial.
  - Diagnostics are capped at 256 rows / 64 KiB, then counted in `SessionReport.dropped_diagnostics` with one marker row.
  - Claims are capped at 16 KiB, with a digest beyond that, so an oversized frame is still decided rather than aborting the session.
  - Tests: the flood and diagnostics gates. (high)
- [x] [Review][Patch] Runner exits 2/3/5 — fixed: before Ready they become typed `IncompatibleWorld` / `InvalidComponent` / `UnrepresentableFrame` refusals, and the binding treats them as voluntary, not a crash. After Ready they stay crashes. Test: `runner_refusal_exits_before_ready_are_typed_and_not_crashes`. (medium)
- [x] [Review][Patch] Weakened 17-3b refusal tests — fixed: named codes, HTTP 400 and the form-gate negative guards are restored.
  - Under `wasm-host`, `gate_manifest(..None)` is asserted `Ok`, and the door and hot-swap successor are driven to `spawned_surface_unsupported` with their surface names.
  - No topology-load harness exists, so topology is still covered only by the helper. (medium)
- [x] [Review][Patch] Mixed-scope atomicity test — fixed with three cases:
  - Cedar forbid: exactly one decision with a refusal, nothing minted.
  - Policy-mixed (MCP allow, then `FsWrite` deny): the minted token's `cap.revoke` row is asserted.
  - `SkillAuthorSelf`: `unmediated`, nothing minted. (medium)
- [x] [Review][Patch] Non-discriminating idle/stall test — fixed: the 32 s sleep is replaced by an idle period that outlasts the turn budget, plus the deterministic `a_new_turn_after_idle_restarts_the_progress_window`, which ages the stamp to 40 s and asserts `begin_turn` resets it. (medium)
- [x] [Review][Patch] Hostile-lifting cause not pinned — fixed: assertions now require Wasmtime's hostcall-fuel trap text. An under-budget control (one 4 MiB alias) clears the canonical call. (medium)
- [x] [Review][Patch] Delivered-frame pid unasserted — fixed: the eight-case gate requires a `TaskAssign` row at the admitted pid only when the scope was granted. (medium)
- [x] [Review][Patch] Unbounded termination — fixed: `poll_frame` closes after stdout EOF once a quiet window passes; the session allows a bounded grace, then a planned stop, then `SpawnedBridge::kill`. Kernel tests cover EOF-while-stderr-open, framing errors and kill. (medium)
- [x] [Review][Patch] `--once` zero turns — fixed: a once session accepts only its seed as its turn and errors with zero turns. Test: `a_once_session_that_never_completes_its_turn_is_not_success`. (medium)
- [x] [Review][Patch] Mid-turn clean exit drops the task — fixed: a turn-scoped binding that ends `UnloadClean` with an active record takes the FR50 disposition. (medium)
- [x] [Review][Patch] Stderr chunk redaction split — fixed: an over-long diagnostic line is truncated (with a marker) rather than split. The cut never lands inside a UTF-8 sequence or keeps a sub-token hex tail. Kernel test: a 64-hex token straddling the bound never reaches the TL. (medium)
- [x] [Review][Patch] Cgroup behaviour — fixed:
  - Placement is validated in the parent.
  - Without a CPU cap, a failure degrades to rlimits with a loud warning (memory caps call out RLIMIT_AS-only enforcement).
  - `rmdir` retries with bounded backoff.
  - `memory.events` `oom_kill` produces a resource-cap SandboxBlock.
  - Not unit-tested here; a cgroup-delegated host is needed. (medium)
- [x] [Review][Patch] Escape-consumer claim overstated — fixed: the module doc now states there is no daemon caller (deferred). The AC4 wording in this story is accurate only as "the consumer, when invoked". (medium)
- [x] [Review][Patch] SandboxBlock dropped — fixed: if the audit channel refuses it, the record is written to the journal as `cli.subprocess.sandbox_block`. Kernel test covers both the open and the closed channel. (low)
- [x] [Review][Patch] Encode failure kills the session — fixed: the frame is journaled `spirit.frame.undeliverable` and skipped. (low)
- [x] [Review][Patch] `revoke()` semantics — fixed: `Revoked`/`UnknownToken` count as success, and the primary error is preserved over a rollback failure. (low)
- [x] [Review][Patch] Unmediated verdict label — fixed: `is_mediated_scope` is checked before any mint, giving `kernel_verdict: "unmediated"` with no quota burned. (low)
- [x] [Review][Patch] Lock held across executor calls — fixed: `UnloadClean`, `begin_turn` and `complete_turn` copy out and release the lock before calling the executor. (low)
- [x] [Review][Patch] Test plumbing — fixed:
  - Nested fixture builds use `--locked`.
  - The idle test gives startup a 10 s budget.
  - Markers print measured values.
  - The `target_root()` concern was dismissed: every path is relative to the same `--target-dir`.
  - CI zero-test detection belongs to `discipline.yml` and is tracked with chunk C. (low)
- [x] [Review][Patch] Missing kernel/codec seam tests — fixed: `frame_bridge_poll_17_3c.rs`, the SandboxBlock delivery test, and `maos-frame-codec/tests/bounds.rs` (BodyBuffer cap, blank-line limit, lowercase header, truncation). (low)
- [x] [Review][Defer] Delegation and host-B frames still journal sender pid 0, including `SpiritAuto` frames from real Spirits [`crates/maos-bin/src/delegation.rs:273,384,484,553`] — deferred, pre-existing
- [x] [Review][Defer] `spawn_and_bridge` does not `env_clear()`, so the T2 runner inherits the daemon environment; the guest gets no WASI env [`crates/maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs:526`] — deferred, pre-existing
- [x] [Review][Defer] The escape-detector consumer has no production caller [`crates/maos-bin/src/escape_detector_consumer.rs:24`] — deferred, pre-existing
- [x] [Review][Defer] Under `wasm-host`, the in-process refusal is a per-caller `require_in_process` convention rather than enforced centrally by `gate_manifest` [`crates/maos-bin/src/admission.rs:386`] — deferred, pre-existing
- [x] [Review][Defer] MCP selector-level (server/tool) entitlement is not enforced; policy compares scope kinds only [`crates/maos-kernel-core/src/capability/mod.rs:57`] — deferred, pre-existing
- [x] [Review][Defer] The stdout NDJSON/Raw reader is unbounded (`read_until` into a `Vec` with no cap) [`crates/maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs:364`] — deferred, pre-existing
- [x] [Review][Defer] Issuance quota is charged before scope mapping and policy, with no refund on deny or rollback [`crates/maos-kernel-core/src/capability/mod.rs:184`] — deferred, pre-existing

#### Chunks B+C (bmad-code-review, 2026-10-05)

Diff `/tmp/review-17-3c-BC.diff`: 50 files covering wasm-host, host, spirit-hello, bench, iac, scripts, CI, xtask, docs, templates and tracking docs. Layers run: Blind Hunter, Edge Case Hunter, Acceptance Auditor and Test Infrastructure Auditor. The decisions below are resolved under the operator's rule (PRD/contract fidelity, correct drift, honor existing contracts, fill gaps per roadmap) unless marked open.

- [x] [Review][Decision→Patch] The blocking `check-dev-model-tier` gate rejected `dev_model_used: openai-codex/gpt-6.1-sol` because it was not in the frontier allowlist.
  - **Ratified by the operator on 2026-10-06:** "sol is frontier model as powerful as opus".
  - **Change:** the token `gpt-6.1` is added to `FRONTIER_FAMILIES` in the candidate's `xtask/src/check_dev_model_tier.rs`, with a dated rationale following the opus-5 and glm-5.3 precedent.
  - **Scope:** this is A1 allowlist maintenance, not a waiver; the §A6 review stays mandatory.
  - **Original checkout:** its xtask is unchanged until an approved cutover.
- [x] [Review][Patch] A failed turn can deliver earlier frames — fixed: every emitted frame is now lifted and encoded before any is written, and a failure writes nothing for that turn. Test: `a_turn_with_one_unliftable_frame_emits_nothing` with the committed WAT fixture `test-fixtures/partial-turn/`. Mutation: restoring the per-frame write loop turns it RED; restoring the fix is GREEN. [`crates/maos-wasm-host/src/runner.rs:397-413`] (high)
- [x] [Review][Patch] `unregister_spirit` race — fixed: the sender is cloned out of the DashMap before `send().await`, and a vanished entry returns `UnknownSpirit` instead of panicking. `crates/maos-iac/tests/mailbox_reload_pid_17_3c.rs` adds 4 tests, including a deterministic blocked-delivery/unregister race. Restoring the old `expect` turns the race test RED. (high)
- [x] [Review][Patch] CBOR asymmetry — fixed: `encode_cbor` validates its own output against the decode budget and returns a typed refusal (`CBOR encode bound: …`). Tests cover the exact boundaries: the largest string MAX−1024 and 16,383 units round-trip, one more is refused on encode, and nesting 65 is refused. The 4 MiB lineage test now decodes as well. Decode tests assert exact error strings. (medium)
- [x] [Review][Patch] Guest diagnostics docs — fixed: the template/example READMEs, `index.ts` and the cookbook (en+ko) describe the bounded, untrusted `spirit.diagnostic` channel. Story-process narrative is removed. `templates-regen --check` passes. (medium)
- [x] [Review][Patch] Fault-inject house rule — fixed: the `runner-fault-inject` feature gates the flag and the ptrace block, and `compile_error!` fires under `not(debug_assertions)`. A release build with the feature fails to compile (proved). The SIGSYS gate builds the debug runner with the feature. The release runner still rejects the flag (asserted). (medium)
- [x] [Review][Patch] Aggregate guest memory — fixed: `StoreLimits` `.memories(1)`. The TS component defines one memory, and its ignored test passes. (medium)
- [x] [Review][Patch] Hostile lifting at the approved contract — fixed: both shapes and the under-budget control run at 768 MiB, and all 3 pass with the hostcall-fuel cause asserted. (medium)
- [x] [Review][Patch] Published launch claims — fixed: the docs-site cookbook, manifest v5 and migrate v4→v5 pages (en + ko) now say `wasm_engine_off` (default) / governed T2 launch (`wasm-host`) / `spawned_surface_unsupported` (in-process surfaces). ADR-060 has a dated amendment note. (medium)
- [x] [Review][Patch] AC1 audit query exercised — done:
  - The prover now runs `maosctl audit query --spirit`.
  - The query against the accepted final DB is archived in `17-3c-evidence/review-2026-10-06/audit-query-final-isolated.{txt,json}`.
  - **It FAILS (exit 2, FR4 schema violation)**, which surfaced the FR4 decision below. (medium)
- [x] [Review][Patch] Durable evidence — done: fresh logs, gate outputs, mutation RED/GREEN results and the working-tree content hash (`485adfeb…`, 5248 files) plus rustc/cargo versions are under `17-3c-evidence/review-2026-10-06/` (`verification-summary.json`). The spec marks `artifact://` references as non-durable. (medium)
- [x] [Review][Patch] Deleted tests restored:
  - the J1 budget/config/error tests, plus a `send_frame` codec decode test;
  - the 255/256 preferred-length check;
  - the exact default-fuel argv;
  - the encode-cap message and kind checks;
  - the e2e stderr prefixes and empty stdout;
  - the `SpiritHostError::InvalidComponent` and e2e module docs. (medium)
- [x] [Review][Patch] Native-twin test — fixed: it reads Ready/frame/TurnComplete/EOF, and the crate passes 5/5. A CI step now runs the crate. (medium)
- [x] [Review][Patch] CI zero-test pass — fixed: the step uses `pipefail` and fails unless rc=0, `test result: ok. 14 passed;`, every marker count is exact, and no `^SKIP ` line appears. It emits `::error` for each problem and each panic, and the summary row is updated. The checker was exercised against pass / missing-marker / SKIP logs. (medium)
- [x] [Review][Patch] Proof-script gaps — fixed:
  - `check()` replaces `assert`.
  - The component argv is tied to the hashed artifact.
  - `Seccomp: 2` / `NoNewPrivs: 1` are required.
  - `VmHWM` and cgroup `memory.peak` are recorded.
  - `declared_*` values are read from the sources.
  - Cgroup removal is polled.
  
  Syntax-checked only: it needs a delegated-cgroup host. (low)
- [x] [Review][Patch] `run_17_3c_delegated.sh` — fixed: it refuses with no command before mutating anything, uses `mkdir -p`, and matches the exact cgroup paths. (low)
- [x] [Review][Patch] IAC attribution / unregister unit tests — added (see the race item). (low)
- [x] [Review][Patch] Tracking-doc leftovers — fixed in the spec (24.04 amendment marked superseded, Code Map, `artifact://` note) and in the epic-17 17-5 pin text. (low)
- [x] [Review][Decision→Patch] FR4 `--spirit` view rejected the guest's delivered frames, so AC1 failed. **Operator ruling 2026-10-06: option 1, build the roadmap `iac.send` mediation (folded into 17-3c).** Done:
  - **Manifest:** `[capabilities.required.iac] send = [..]` with the closed peer-class set {`broadcast`, `spirit:peer`}. It lives on unreleased schema 5 (no second bump), is dropped below schema 5, and is ratified in `xtask/abi-ratifications.toml` as `17-3c-iac-send-capability`. `[capabilities.required.provider]` becomes optional (R17-63: a WASM Spirit cannot use it). The TS template and example declare only `iac.send = ["spirit:peer"]`. The docs (manifest v5, cookbook, en+ko) and field coverage are updated.
  - **Bus:** `deliver_typed` and `retract` take the sender's token and write it on the Transparency Log row, on both the direct and DRR paths. The 32-byte column layout has one source, `capability_token_column`, shared with the cap-audit writer.
  - **Session:** every guest delivery or retract mints its own governed `IacSend{peer_class}` token (enterprise PDP, then kernel policy) before delivery. The verdict is recorded as the authorization record's `send` field.
    - An undeclared peer class is refused as `iac_peer_class_not_granted`.
    - No `iac.send` grant at all gets the kernel's own deny.
    - Retract authority is now checked before any mint.
  - **FR4 classifier:**
    - Kind 8 `sandbox.block` joins the never-a-call kinds (epic AC4: it shows in `maos audit query`).
    - The bridge's lossless SandboxBlock fallback now writes the same kind-8 row as the audit writer.
    - The two now token-bearing `Call` table entries are retired.
  - **Tests:**
    - The provenance gate asserts the delivery row's token and a green real `maosctl audit query --spirit`.
    - The SIGSYS gate asserts `sandbox.block` in the operator view.
    - New `a_delivery_needs_its_iac_send_grant`.
    - Mutation: delivering without the token turns the gate RED; restoring it is GREEN.
  - **Fresh AC1 runtime proof** on the accepted Ubuntu 25.10 x86_64 host:
    - The actual TS CLI ran at CPU 10% / 768 MiB in 323.9 s.
    - The child shows `Seccomp: 2` / `NoNewPrivs: 1`.
    - The prover's `maosctl audit query --spirit example-spirit-ts` exits 0 with the attributed rows.
    - Archived in `17-3c-evidence/review-2026-10-06/remote-ac1-proof/`.
- [x] [Review][Patch] Missing §A6 review-artifact marker (`check-dev-model-tier` second violation) — fixed by this bmad-code-review section.
- [x] [Review][Defer] `Halt::Voluntary` from `handle-frame` emits the same TurnComplete as a normal turn. A queued frame dispatched before EOF ends `UnexpectedEof`; the task is still dispositioned. ADR-032 has no halt control record. [`crates/maos-wasm-host/src/runner.rs:371-375`] — deferred, protocol extension
- [x] [Review][Defer] The runner's `COMPILE_TIMEOUT` (360 s) can never fire before the kernel startup deadline. A compile bomb surfaces as `Deadline("startup")` rather than exit 3. [`crates/maos-wasm-host/src/runner.rs:154`] — deferred, needs a startup/compile budget split
- [x] [Review][Defer] Exit-code taxonomy gaps:
  - environment failures map to exit 3;
  - hostcall-fuel and limiter traps map to exit 1;
  - inbound decode errors map to 1 rather than 5;
  - `fuel = 0` and unparsable fuel values are accepted. [`crates/maos-wasm-host/src/runner.rs`, `adapter.rs:95-101`] — deferred
- [x] [Review][Defer] Runner hardening nits:
  - no `trap_on_grow_failure`;
  - zero memory reservation makes every grow copy;
  - a FIFO open blocks before the watchdog;
  - `on-shutdown` is skipped when the Ready write fails. — deferred
- [x] [Review][Defer] Runner-binary tests for exits 4/5, Voluntary halt, stdout isolation, and directory/oversize components; equiv-harness diagnostics and stale twin detection. — deferred, test-debt
- [x] [Review][Defer] The `wasm-host-tests` matrix `needs: [example-spirit-ts-tests]` couples the existing T2 evidence to the npm job. Split the TS proof into its own job (requires a workflow run to validate). — deferred
- [x] [Review][Defer] Historical runtime evidence records no source tree hash or toolchain; it cannot be fixed retroactively. — deferred
- [x] [Review][Defer] The new I9 exemption lacks the register's ≥2 maintainer sign-offs; it is blocked on humans before integration. [`docs/invariants/i9-exemptions.md:576-583`] — deferred
- [x] [Review][Defer] The memfd leave-one-out was not re-run on the shipped runner configuration. Image-specific syscall leave-one-out is waived by the operator's platform amendment, and the retained evidence stands. — deferred

## Review outcome and transitions

The independent full-layer review is complete. It ran in four layers, plus security and non-author
runtime verification, against the candidate source and exercised evidence.
- **Findings:** all are resolved (see Review Findings). 17-3b's final finding is verified and checked off.
- **Verification:** on the identical integrated source, recorded under `17-3c-evidence/review-2026-10-06/`.
  - Gates: `spawned_scope_17_3c` 15/15, the 14 governance gates, the full xtask suite, fmt, and workspace check in both feature sets.
  - Fresh AC1 remote proof on the accepted Ubuntu25.10 x86_64 host.
  - Mutation pairs (RED, then GREEN): lineage restamp, partial turn, `iac.send` token and kernel deny-bypass.
- **Commits:** two local commits, 17-3b then 17-3c. No push, publication or deployment. 17-3d stays backlog.
- **Open human obligation:** the ≥2 maintainer sign-offs on the `FrameWriter` I9 exemption (deferred-work).

## Dev Agent Record

### Agent Model Used

openai-codex/gpt-6.1-sol — detached candidate implementation and tracking reconciliation.

### Debug Log References

Candidate `_bmad-output/implementation-artifacts/17-3c-evidence/`:

- `candidate-grants-and-verification.json` — current approvals, checks and review scope.
- `configured-enterprise-gates.json`, `scoped-verdict-mutation.json` — earlier actual governance/mutation records; final fresh RED709/GREEN711 and six-gate771 references are in the candidate specification/summary, not relabeled as these earlier runs.
- `hostile-lifting-mutation.json`, `lifecycle-sigsys-gates.json` — earlier resource and lifecycle controls, preserved at their exercised revisions.
- `kernel-surface-grant.json`, `formatted-kloc-measurement.json` — explicit surface/numerical grants.
- `remote-100-20-probes-and-grant.json`, `remote-capped-startup-before.json` — failed180 s probes, bounded measurement and superseding360 s approval.
- `remote-100-20-final-isolated.json`, `remote-100-20-final-isolated-runtime.tar.gz` — final exact isolated binaries and actual SQLite audit.
- `matched-refusal-launch-360.json`, `preserved-refusal-revision-360.json` — genuine original feature-on refusal versus candidate success.
- `ts-resource-capacity.json` — static capacity; not a runtime guest high-water measurement.

### Completion Notes List

- Implementation and runtime evidence were produced in the detached candidate and then integrated
  byte-identically into the original checkout (tree comparison against the verified candidate).
  Tracking documents were three-way merged.
- Six final real-process gates passed under the approved 360 s guard; kernel 262 unit
  cases and candidate governance checks passed. Actual TS CPU10 launch and same-input
  refusal/launch revision pair passed on the accepted Ubuntu25.10 x86_64 platform.
- Final artifacts use `/mnt/build/maos-17-3c-final.oitpeZ/release`, not shared-target
  cross-worktree cache provenance. Measurement-only probes are not acceptance runs.
- §A6 review (2026-10-06): 43 patches. Decisions: D1, D2 and the FR4 `iac.send` mediation were built; D3 was dismissed; `gpt-6.1` was ratified.
  The approved re-pins are kernel 25293, KLOC kernel 19529 / bin 24600 / frame-codec 249 / IAC 7100, and the surface.
  `spawned_scope_17_3c` 15/15. 17-3b closed on this review's verification.
- Integration verification of the commit tree found `check-manifest-schema-version` red. Its independent ratified
  post-v1 inventory lacked the new `capabilities.required.iac` section (FR4 `iac.send`, ratification
  `17-3c-iac-send-capability`). The section was added to `RATIFIED_POST_V1_SCHEMA_SECTIONS` (xtask +1, within headroom), and the gate is green.

### File List

Paths changed by the 17-3c commit relative to the 17-3b commit. The evidence directory
`17-3c-evidence/` is listed as a whole. Generated TS components and ignored test-build
Cargo.lock files are not source changes.

- `.github/workflows/discipline.yml`
- `Cargo.lock`
- `Cargo.toml`
- `STABILITY.md`
- `_bmad-output/implementation-artifacts/17-3c-early-start-flag-winston-decision.md`
- (new) `_bmad-output/implementation-artifacts/17-3c-evidence/` (candidate and review evidence, incl. `review-2026-10-06/`)
- (new) `_bmad-output/implementation-artifacts/17-3c-wasm-spirit-on-the-bus-under-t2.md`
- `_bmad-output/implementation-artifacts/deferred-work.md`
- (new) `_bmad-output/implementation-artifacts/epic-17-context.md`
- (new) `_bmad-output/implementation-artifacts/spec-17-3c-isolated-runtime-candidate.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md`
- `_bmad-output/planning-artifacts/epics/epic-17-workers-and-third-party-form-w2.md`
- `crates/maos-audit/src/fr4_classifier.rs`
- `crates/maos-bench/Cargo.toml`
- `crates/maos-bench/benches/iac_routing_budget.rs`
- `crates/maos-bench/benches/orchestrator_fanout_nfr_perf_8.rs`
- `crates/maos-bench/src/harness/j1.rs`
- `crates/maos-bin/Cargo.toml`
- `crates/maos-bin/src/admission.rs`
- `crates/maos-bin/src/delegation.rs`
- `crates/maos-bin/src/escape_detector_consumer.rs`
- `crates/maos-bin/src/lib.rs`
- `crates/maos-bin/src/main.rs`
- `crates/maos-bin/src/operator_door.rs`
- (new) `crates/maos-bin/src/spirit_session.rs`
- `crates/maos-bin/src/supervision.rs`
- `crates/maos-bin/src/worker_spawn.rs`
- `crates/maos-bin/tests/cookbook_manifests_17_3b.rs`
- (new) `crates/maos-bin/tests/spawned_scope_17_3c.rs`
- `crates/maos-bin/tests/wasm_form_admission_17_3b.rs`
- (new) `crates/maos-frame-codec/Cargo.toml`
- (new) `crates/maos-frame-codec/src/lib.rs`
- (new) `crates/maos-frame-codec/tests/bounds.rs`
- `crates/maos-host/src/lib.rs`
- (deleted) `crates/maos-host/tests/resolve_launch.rs`
- `crates/maos-iac/src/adapter.rs`
- `crates/maos-iac/src/adapter/drr_scheduler.rs`
- `crates/maos-iac/src/adapter/log_recall.rs`
- `crates/maos-iac/src/adapter/mailbox.rs`
- `crates/maos-iac/src/adapter/transparency_log.rs`
- (new) `crates/maos-iac/tests/iac_send_token_17_3c.rs`
- (new) `crates/maos-iac/tests/mailbox_reload_pid_17_3c.rs`
- `crates/maos-kernel-core/Cargo.toml`
- `crates/maos-kernel-core/src/capability/cap_audit/writer_task.rs`
- `crates/maos-kernel-core/src/capability/mod.rs`
- `crates/maos-kernel-core/src/inference/mod.rs`
- `crates/maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs`
- `crates/maos-kernel-core/src/scheduler/hook_dispatch.rs`
- `crates/maos-kernel-core/src/scheduler/scheduler_loop.rs`
- `crates/maos-kernel-core/src/security/sandbox/linux.rs`
- `crates/maos-kernel-core/src/security/sandbox/mod.rs`
- `crates/maos-kernel-core/tests/abi_stability_admit.rs`
- `crates/maos-kernel-core/tests/bridge_t2_route_17_6.rs`
- `crates/maos-kernel-core/tests/cli_wrapper_bridge_8_12.rs`
- `crates/maos-kernel-core/tests/drr_scheduler.rs`
- (new) `crates/maos-kernel-core/tests/fixtures/manifest/capabilities/edge-case/iac_send.toml`
- (new) `crates/maos-kernel-core/tests/fixtures/manifest/capabilities/malformed-rejected/iac_send.toml`
- (new) `crates/maos-kernel-core/tests/fixtures/manifest/capabilities/well-formed/iac_send.toml`
- (new) `crates/maos-kernel-core/tests/frame_bridge_poll_17_3c.rs`
- `crates/maos-kernel-core/tests/intent_lineage_corpus_v0.rs`
- `crates/maos-kernel-core/tests/manifest_field_coverage.rs`
- `crates/maos-kernel-core/tests/operator_posture_ceiling_16_6.rs`
- `crates/maos-kernel-core/tests/orchestrator_distillate_dispatch.rs`
- `crates/maos-kernel-core/tests/provider_switched_journal.rs`
- `crates/maos-kernel-core/tests/retract_corpus_v0.rs`
- `crates/maos-kernel-core/tests/sandbox_admission.rs`
- `crates/maos-manifest/src/lib.rs`
- `crates/maos-manifest/src/manifest.rs`
- (new) `crates/maos-manifest/tests/iac_send_capability_17_3c.rs`
- `crates/maos-spirit-hello/Cargo.toml`
- `crates/maos-spirit-hello/src/bin/hello_spirit_bench.rs`
- `crates/maos-spirit-hello/tests/frame_roundtrip.rs`
- `crates/maos-wasm-host/Cargo.toml`
- `crates/maos-wasm-host/guests/equiv-fixture/native-twin/Cargo.lock`
- `crates/maos-wasm-host/guests/equiv-fixture/native-twin/Cargo.toml`
- (new) `crates/maos-wasm-host/guests/equiv-fixture/native-twin/src/bin/hostile_twin.rs`
- `crates/maos-wasm-host/guests/equiv-fixture/native-twin/src/main.rs`
- `crates/maos-wasm-host/guests/equiv-fixture/native-twin/tests/modes.rs`
- (new) `crates/maos-wasm-host/guests/routed-relay/Cargo.lock`
- (new) `crates/maos-wasm-host/guests/routed-relay/Cargo.toml`
- (new) `crates/maos-wasm-host/guests/routed-relay/src/lib.rs`
- `crates/maos-wasm-host/src/adapter.rs`
- `crates/maos-wasm-host/src/codec.rs`
- `crates/maos-wasm-host/src/config.rs`
- (deleted) `crates/maos-wasm-host/src/conformance.rs`
- `crates/maos-wasm-host/src/host_state.rs`
- `crates/maos-wasm-host/src/lib.rs`
- `crates/maos-wasm-host/src/runner.rs`
- (new) `crates/maos-wasm-host/test-fixtures/partial-turn/partial_turn_component.wat`
- `crates/maos-wasm-host/tests/codec_integration.rs`
- `crates/maos-wasm-host/tests/e2e_roundtrip.rs`
- `crates/maos-wasm-host/tests/equiv_harness.rs`
- `crates/maos-wasm-host/tests/frame_bridge_roundtrip.rs`
- `crates/maos-wasm-host/tests/ts_component_17_3b.rs`
- `docs-site/docs/cookbook/wasm-component-spirit.md`
- `docs-site/docs/manifest/v5.md`
- `docs-site/docs/migrate/v4-to-v5.md`
- `docs-site/i18n/ko/docusaurus-plugin-content-docs/current/cookbook/wasm-component-spirit.md`
- `docs-site/i18n/ko/docusaurus-plugin-content-docs/current/manifest/v5.md`
- `docs-site/i18n/ko/docusaurus-plugin-content-docs/current/migrate/v4-to-v5.md`
- `docs/adr/ADR-060-spirit-forms-by-trust-tier.md`
- `docs/ci-baselines/kernel-surface-v0.1-beta.json`
- `docs/invariants/i9-exemptions.md`
- `examples/example-spirit-ts/README.md`
- `examples/example-spirit-ts/manifest.toml`
- `examples/example-spirit-ts/src/index.ts`
- (new) `scripts/prove_17_3c.py`
- (new) `scripts/run_17_3c_delegated.sh`
- `spirits/architect/tests/code_review_loop.rs`
- `spirits/orchestrator/tests/distillate_dispatch.rs`
- `templates/spirit-ts/README.md`
- `templates/spirit-ts/manifest.toml`
- `templates/spirit-ts/src/index.ts`
- `xtask/abi-ratifications.toml`
- `xtask/kernel-api-classes.toml`
- `xtask/kernel-core-baseline.toml`
- `xtask/kloc.toml`
- `xtask/src/check_dev_model_tier.rs`
- `xtask/src/check_manifest_schema_version.rs`
- `xtask/tests/kernel_pin_content_hash_16_0.rs`
- `xtask/tests/recovery_lane_ceiling_rule.rs`

### Change Log

- 2026-10-02 — Detached candidate implementation authorized against the preserved refusal snapshot; explicit measured numerical, surface and runtime grants recorded subsequently in the candidate evidence.
- 2026-10-05 — User accepts completed Ubuntu25.10 x86_64 testing as sufficient. Implementation/amended acceptance complete; independent review pending.
- 2026-10-05 — Canonical story, sprint rows, approved specification status, 17-3b finding, early-start record, parent epic and context reconciled to review. No original runtime cutover, staging, commit or publication.
- 2026-10-06 — §A6 bmad-code-review complete (43 patches; decisions D1, D2 and FR4 `iac.send` built; D3 dismissed; `gpt-6.1` ratified). Approved kernel/KLOC/surface re-pins, fresh AC1 remote proof and four mutation pairs, all RED then GREEN. 17-3b verified and closed. Candidate integrated into the original checkout; `review` → `done`. Local commit only, no push.
