---
story_id: '17.2'
story_key: 17-2-worker-cgroups-applied
created: 2026-10-10
context_commit: b19556ab
baseline_commit: b19556abbf4937784b2c7662f3a00025289590a9
security_sensitive: true
---

# Story 17.2: Worker cgroups applied

Status: done

## Story

As an operator,
I want the resource caps in a Worker's manifest to reach its actual operating-system containment boundary, with a truthful lifecycle record when a supported native fallback is used,
so that CPU, memory, file-descriptor and process limits constrain the child rather than merely appear in configuration.

**Contract:** Epic 17 AC1–AC3, FR6, and preservation of the completed 17-1 T3 Worker and 17-3c T2 session boundaries. This is a FLAG-Winston, security-sensitive story, and the non-author §A6 review net is non-degradable. On 2026-10-11 the operator **ratified** the kernel location charter, the numerical rule and the CI-execution route (`17-2-flag-winston-decision.md`, D-17-2-FW-01…08):

- **Charter:** eight kernel source files. Five are edited; three are deletion-only, including the `journal/mod.rs` fix.
- **Ceilings:** no raise, with exact figures at landing.

`ready-for-dev` means the guide is prepared and its charter ruled. It does not mean the feature, its CI evidence or any figure above a current ceiling is approved.

## Current-tree corrections — read before implementing

| Epic premise | Current fact and implementation consequence |
|---|---|
| Add resource limits to the Worker | `worker_spawn.rs:877-885` constructs `SandboxSpec` with `resolved_caps: Default::default()`. The wrapper parser never reads `[resources]`. Fix this production custody first; changing the argv builder alone cannot satisfy AC1. |
| T3 already applies CPU and FD declarations correctly | `t3/argv.rs:48-68` passes a percentage as a number of CPU cores and uses `fd_max` as `--pids-limit`. It emits no `--ulimit=nofile`. These errors are directly inside FR6/AC1, not optional cleanup. |
| Own-cgroup layout is `<own>/maos.slice/spirit-<id>` | Completed 17-3c work uses the nearest ancestor with delegated controllers and creates `maos.slice/spirit-<sanitized-id>-<daemon-pid>-<sequence>`. Preserve ancestor discovery, collision resistance and pre-exec placement. |
| The epic's `mkdir/chown/echo $$` recipe is sufficient delegation | A populated domain cannot distribute CPU/memory controllers. Provision an empty delegated domain plus a separate populated process leaf. Enable controllers at the delegated domain and MAOS-owned intermediate node; never have production rewrite an ancestor's policy or move the daemon. |
| Undelegated spawn always falls back | Native CPU-capped spawn already refuses. Preserve that refusal; explicit `pids_max` must also refuse without its controller. The positive downgrade fixture requests memory/FD but no CPU/PID cap. Resource fallback never lowers the sandbox tier. |
| Add `LifecycleEvent::ResourceCeilingDowngraded { reason }` literally | The event enum is `Copy`, `repr(u8)`, with established discriminants 0–19. Append unit variant `ResourceCeilingDowngraded = 20`; serialize a typed reason payload through existing `LifecycleEntry.payload`, following `ProviderSwitchedPayload`. Do not break the enum's representation. |
| Schema 5 must land with 17-1 | Schema 5 already landed with 17-3b. Add the optional resources field without a schema-6 bump. `[cli_wrapper]` itself has no version triple; its versioning debt belongs to 20-5. |
| Wire `apply_resource_ceiling` | Delete it. It has no production caller and never places the process in its cgroup. The sprint row's old “wired” wording is wrong. |
| Kernel net change is an estimated −30…+10 | A formatted, compiled mechanism prototype measured **−157 physical kernel lines / −99 tokei code lines**. This includes deleting the entire 170-line obsolete module. It is not a final implementation ceiling or evidence for production journal/Worker plumbing. See §Measured prototype. |

[Sources: epic shard §17-2; `crates/maos-bin/src/worker_spawn.rs:432-473,877-909`; `crates/maos-kernel-core/src/security/sandbox/linux.rs:69-103,425-519`; `sandbox/mod.rs:105-119`; `t3/argv.rs:48-95`; `crates/maos-domain/src/invariants/i10.rs:35-128`.]

## Acceptance Criteria

### AC1 — Real delegated enforcement and truthful undelegated behavior

**Given** a real Linux cgroup-v2 domain delegated to the unprivileged test user, with CPU, memory and PID controllers enabled and the test process in a separate process leaf,
**when** the `cgroup-ceiling-delegated` discipline job launches three fixtures:

- the small conformant echo WASM component through real admission and the production spawned-class session, which is the only daemon production route to native T2;
- a bounded native ELF fixture through the existing T2 `spawn_and_bridge` route, for PID exhaustion;
- a T3 Worker through the real `maos run` Worker path,

**then**:

1. The native child's `/proc/<actual-os-pid>/cgroup` membership resolves to its owned MAOS leaf. Read that leaf's `cpu.max`, `memory.max` and `pids.max`; assert exact resolved values, not just file existence. For an example 10% / 64 MiB / 12 PID cap, the values are `10000 100000`, `67108864`, and `12` respectively. The process is placed there **before guest code runs**, and descendants inherit the containment.

   For the WASM session, `<actual-os-pid>` is the runner's `child_pid`, never the scheduler Spirit PID. A test reads it from the supervision `bindings()` view (`supervision.rs:1004-1008,1524-1525`; precedent `spawned_scope_17_3c.rs:1349-1375`), or correlates it with the CLI's `spirit_loaded.child_pid` (`scripts/prove_17_3c.py:76-112`). Inspect while the runner is alive, and finish the session only afterwards.
2. The T3 Worker's runtime inspect/configuration and actual child evidence show CPU, memory and PID limits derived from its manifest, including the new `pids_max`. Explicit FD limits are independent `nofile` limits. At 10% CPU, the runtime receives `--cpus=0.10`, never `--cpus=10`. Exercise `fd_max != pids_max` so the old alias cannot pass.

   An explicit `fd_max` above the invoking user's hard `RLIMIT_NOFILE` is a **launch refusal before guest execution**. A rootless runtime cannot raise the host hard limit ([Podman #5526](https://github.com/podman-container-tools/podman/issues/5526)), and native T2's `setrlimit` in `pre_exec` fails likewise (`linux.rs:149-158`). Such a request is never silently clamped, never reported as a downgrade, and never reported as applied. A bounded fixture proves the refusal and the release of owned resources.
3. Malformed Worker resources are refused before host probes, binding, credential minting or child spawn. Missing `[resources]` remains compatible. A stricter operator-policy resource cap wins through the existing resource-policy authority; no second policy registry or second full Spirit admission is introduced.
4. New jobs join `aggregate.needs` **and have an explicit success-required fan-in condition**. Today `aggregate` rejects failed/cancelled results but tolerates skipped ones (`discipline.yml:5042-5048,5207-5211`), so enrollment alone is not skip-proof. Add the narrow required-result checks for these jobs (T5 lists the enrollment sites), not a global gate rewrite.

   The jobs must not turn any of these into success: missing controllers, failed provisioning, refused T2 startup, an absent container runtime, zero selected tests, or an early return.

   Use the Epic-17 Ubuntu 24.04/26.04 policy: delegated and undelegated T2 execution on both x64 and ARM images, and rootless T3 proof on the two x64 images. Do not use `ubuntu-latest`.

**Given** a deliberately undelegated environment whose lack of usable delegation is asserted, constructed as a named system-manager service: `sudo systemd-run --unit=<named> --uid=<runner> --gid=<runner> -p Delegate=no --wait --pipe --collect …` with the caller environment transported. Inside it, the job asserts:

- a non-root UID;
- its exact `/proc/self/cgroup` membership and the cgroup-v2 mount;
- the unit's `Delegate=no`;
- that **no candidate ancestor usable by production discovery** (`linux.rs:425-507`) permits the owned-subtree and common-ancestor placement operations, probed as the test UID.

A probe that unexpectedly succeeds removes only its own uniquely named empty directory and fails provisioning. The job never alters ancestor controller policy to manufacture the failure.

**when** `cgroup-ceiling-undelegated` launches a schema-5 class Spirit through real admission and `run_admitted_session` — the small conformant echo component (`tests/fixtures/wasm/equiv-fixtures.provenance.toml:20-66`; guest `crates/maos-wasm-host/guests/echo-spirit`) on the real release runner, with `forms = ["wasm-component"]`, `[sandbox].tier = "T2"`, `memory_max_mb = 64`, `fd_max = 64`, and CPU/PID absent from both the manifest and the injected operator policy,

**then**:

- it retains Landlock/seccomp and applicable rlimits, and succeeds only if those mechanisms succeed;
- the live runner's `/proc/<child_pid>/limits` shows exactly those `RLIMIT_AS`/`RLIMIT_NOFILE` values, and the session reaches Ready and completes a turn;
- after the journal is reopened, it holds exactly one real lifecycle entry with `ResourceCeilingDowngraded`. That entry has the correct Spirit identity, an unchanged effective T2 tier, and a typed payload naming the missing cgroup mechanism and the surviving `RLIMIT_AS`/`RLIMIT_NOFILE` semantics;
- it does not claim an RSS cgroup cap or OOM visibility.

The 768-MiB TypeScript example is a separate capacity precedent, not this fixture: it requests CPU, and it needs Node/componentize-js (`17-3c-evidence/runtime-resource-probes.json:5-8`). A direct `spawn_sandboxed`/`spawn_and_bridge` helper alone is not the lifecycle proof. The archived 17-3c run already showed the release runner working at 64 MiB under `RLIMIT_AS` (`runner.rs:203-211` disables reservations; `17-3c-evidence/review-2026-10-06/logs/spawned_scope_17_3c.log:680-718`).

**And** an explicit CPU or PID cap without usable delegation refuses before guest execution; it is not converted into a successful downgrade. Failed spawn, normal delegated spawn and a no-requested-resource case must not produce a misleading successful-resource-downgrade row. A journal write failure is not silently swallowed. Merely printing stderr, writing a fixture-only event or emitting child-authored output does not satisfy this criterion.

[Sources: epic AC1; Linux CPU refusal `linux.rs:69-90`; T2 startup invariants from completed 17-6/17-3c; `i10.rs`; `WorkerSupervision`/spawn path; `.github/workflows/discipline.yml` existing `wasm-host-tests`, `epic-17-exit` and T3 substrate jobs.]

### AC2 — One native layout, distinct resource contracts and a complete cutover

**Given** resources from a manifest and the existing operator resource policy,
**when** caps are parsed, resolved and applied,
**then**:

1. `ResourceCaps`, `RawResourceCaps`, and `ResolvedCaps` gain `pids_max: Option<u32>`. Zero is rejected with the established `CapOutOfRange` error; negative, overflowing and non-integer TOML values fail parsing. Omission stays `None`. The same `strictest_opt` operation resolves all four resource dimensions independently.
2. Linux requires `pids` in discovery when a PID cap is requested, enables it below its owned `maos.slice`, and writes the final `pids.max`. Preserve existing CPU/memory math, rlimit application, async-signal-safe pre-exec placement, unique names and RAII cleanup. No persistent PID→cgroup registry is added.
3. T3 uses rootless runtime resource flags, not a parallel manually managed MAOS cgroup. Explicit `pids_max` drives `--pids-limit`; explicit `fd_max` drives `--ulimit=nofile=<n>:<n>`. Preserve absent-field backend defaults of one CPU core, 256 MiB and 64 PIDs. Do not install a universal 64-PID default in `resolve_caps`, and do not invent a new FD default.
4. Delete `scheduler/resource_ceiling.rs`, its `scheduler/mod.rs` declaration, `apply_resource_ceiling`, `ResourceCeilingHandle`, `IoError`, and obsolete tests/public-surface entries. Migrate every remaining caller; no compatibility alias or dead root-layout implementation remains. Native delegated cgroups and T3 runtime flags are the two Linux process-containment mechanisms.
5. Preserve kill/reap-before-cleanup and cgroup.kill + bounded EBUSY/rmdir cleanup. Verify no owned leaf remains after normal exit, setup failure, spawn failure and killed descendants on the capable runner. Do not use `remove_dir_all` on cgroupfs, remove shared ancestors, or make `maos purge` responsible for live resource handles. If cleanup exhausts its bound, report the real failure/residue rather than claiming guaranteed removal.
6. The additive lifecycle event preserves existing numeric discriminants, old journal replay and the established serde shape. Its reason belongs in a typed payload, not a free-form imitation of a lifecycle row in the Transparency Log.
7. Update schema-5 field coverage and ratification records, resource documentation and English/Korean published claims. Existing schema-4 manifests omitting the field still load; unknown fields remain rejected.

[Sources: epic AC2; `manifest.rs:103-211`; `scheduler/resource_ceiling.rs:1-170`; `sandbox/mod.rs:84-158`; `t3/argv.rs`; PRD FR6; architecture §4.1/§4.3.1; 16-4 purge disposition in epic AC2.]

### AC3 — A smoke test that proves containment, not directory creation

**Given** `crates/maos-kernel-core/tests/cgroup_ceiling_smoke.rs`,
**when** this story replaces the old fake-PID/root-directory exercise,
**then**:

1. The test has no `#[ignore]`, no `MAOS_CGROUP_TEST` early-return gate and no silent substrate skip. It executes the supported native path using a real child and actual membership, CPU/memory/PID values, limits that bite, and cleanup.
2. The `cgroup-ceiling-delegated` job explicitly runs the rewritten smoke target. The undelegated branch is independently exercised by its named job and verifies persisted lifecycle content through the production producer/reader.
3. Provision the same delegated-domain/process-leaf substrate around the mandatory whole-workspace test command; its existing T3 provisioning alone is insufficient. **Environment contract:** a transient system-manager service starts in a clean environment, and `--uid` creates no login session. The wrapper therefore transports, explicitly:
   - `PATH`, `HOME`, and the actual Cargo/rustup/target-dir settings;
   - `CI`/`GITHUB_ACTIONS` and `MAOS_SECRETS_BACKEND`;
   - the three T3 exports (`MAOS_T3_RUNTIME`, `MAOS_T3_IMAGE_LOCK_PATH`, `MAOS_T3_IMAGE_TRUST_ANCHOR_PUB_HEX`; `scripts/provision_t3_tests.py:95-104`);
   - `XDG_RUNTIME_DIR` and `DBUS_SESSION_BUS_ADDRESS` when present.

   Inside the service, *before* the suite runs, a canary proves that the same UID still reaches the user manager and the provisioned image store, and that a resource-limited rootless container actually runs in that context. Otherwise, T3 tests could take `tests/harness/t3_substrate.rs`'s local-skip branch and count as passed.

   Keep the job timeout and bounded unit shutdown; a terminated `systemd-run` client must not leave its service running. Isolate negative-delegation fixtures in their own `Delegate=no` service so the positive envelope cannot turn them into false positives. Do not exclude the smoke or its crate, restore an ignore, narrow the workspace command, add `--skip`, or count unexecuted assertions as passes. Local users running this strict smoke need the documented delegated harness. Never switch to rootful Podman or widen production runtime environment passthrough to make the envelope work.
4. Behavioral proven-red mutations demonstrate that losing `pids.max`, ignoring Worker `[resources]`, using CPU percent as cores, reusing `fd_max` for PID count, or suppressing the downgrade producer each fails its corresponding oracle. Keep regression tests for consumer-visible behavior; no source-text/copy-pinning tests.
5. The existing T2 Landlock/seccomp tests, T3 security/custody tests, Worker supervision and 17-3c spawned-session tests retain their guarantees. Preserve their actual supported platform behavior; do not use this story to claim macOS/Windows resource equivalence that has not been exercised.

[Sources: epic AC3; existing `cgroup_ceiling_smoke.rs:5-67`; Epic-17 rule 11/17-6 run-or-fail discipline; Epic-15 mandatory workspace job.]

### Delivery gate — measured authorization and complete re-pin

The source deletion still changes the frozen kernel surface. Kernel edits stay inside the FW-01 locations, and any other location returns to the operator as a charter request. Before landing:

1. Measure the complete implementation after formatting.
2. Keep every figure within the ratified numbers:
   - **Kernel (FW-02):** ≤ 25267 physical and ≤ 19553 tokei, re-pinned to the exact lower measurement.
   - **Manifest (FW-03):** no raise.
   - **Domain, bin, xtask and aggregate (FW-03):** exact figures at landing.
3. A figure above a current ceiling returns to the operator as a measured request **before** landing.

Then update the **seven-artifact kernel re-pin set** together. The list below is an inventory, not an order; execute it in the Grant-protocol order of `17-2-flag-winston-decision.md` (classes → surface → physical/hash → content-hash literals → KLOC → `RATIFIED_AT_EASING` → ratification):

1. `xtask/kernel-core-baseline.toml`: exact physical line count plus file-set/content hashes; remove the retired file. Keep the `src_lines` assignment on its existing line 505 unless every positional reader is deliberately migrated.
2. `xtask/kloc.toml`: authorized, exact tokei figures; no transfer of unused kernel allowance into another crate without authorization.
3. `docs/ci-baselines/kernel-surface-v0.1-beta.json`: retire removed public items and update affected signature hashes.
4. `xtask/kernel-api-classes.toml`: classify new public seams and update affected hashes.
5. `xtask/tests/recovery_lane_ceiling_rule.rs`: the independently pinned kernel-only scalar `RATIFIED_AT_EASING` (`:267-279`, currently `19_553`; the oracle checks `actual <= RATIFIED_AT_EASING`). Update it when the ratified kernel tokei bound changes; it is not a per-crate map.
6. `xtask/tests/kernel_pin_content_hash_16_0.rs`: independently pinned kernel set/count fixtures as required by the changed source set.
7. `xtask/abi-ratifications.toml`: additive resources/lifecycle/public-surface ratification record under the existing mechanism.

Re-derive the exact affected subset from the final diff; do not blindly rewrite unrelated historical fixtures. **Order matters:** classify new public symbols first, then regenerate the surface baseline and enumerate removals/additions, then update physical/content pins and measured budgets. Regenerating the baseline first can hide an unclassified new symbol from the current gate. Run the kernel baseline, service-boundary/public-surface, KLOC and relevant independent pin oracles. Neither a net deletion nor this story's status waives FLAG-Winston/charter authorization. [Sources: E16-A1/A7; 17-1 re-pin precedent; `xtask/kernel-core-baseline.toml:811-846`; `xtask/src/check_kernel_baseline.rs:3-45,131-157`.]

## Tasks / Subtasks

- [x] **T1 — Preserve the authority and compatibility contracts** (AC1, AC2)
  - [x] Add optional `pids_max` through parser/raw/resolved types and strictest-of resolution; migrate all struct literals, including inline unit tests and platform tests.
  - [x] Parse Worker resources before any probe/bind/mint side effect. Reuse the live operator resource-policy snapshot through a narrow resource-only seam; never call full admission twice.
  - [x] Replace Worker's all-default `SandboxSpec.resolved_caps` with resolved caps on every route converging on `run_cli_wrapper_manifest`.
  - [x] Add `("resources", "pids_max")` to `manifest_field_coverage.rs:55-57`, with well-formed, malformed-rejected and edge-case fixture files, plus behavioral precedence tests. Write the schema-5 ratification entry on the `17-3c-iac-send-capability` precedent (`xtask/abi-ratifications.toml:106-124`) and record its real reader contract: omission remains accepted, readers from 17-2 onward enforce the field, unknown keys stay errors, and **pre-17-2 readers reject `pids_max`** under `deny_unknown_fields` (`manifest.rs:162-168`). Do not copy the egress entry's "old readers ignore it" claim. Preserve the missing-table and v4 omission compatibility.
- [x] **T2 — Complete native PID containment and retire the false mechanism** (AC2, AC3)
  - [x] Add PID controller discovery/enable/write and CPU-or-PID refusal without delegation.
  - [x] Preserve native pre-exec placement and existing containment/cleanup ownership; expose only a transient resource-outcome seam to the caller.
  - [x] Delete the obsolete resource-ceiling module/export/API and replace its smoke callers; retire its false non-Linux no-op assertions.
  - [x] Exercise creation/limit-write/exec failures, simultaneous same-Spirit spawns and descendant cleanup on real cgroupfs; report bounded cleanup failure honestly.
- [x] **T3 — Make T3 resource flags truthful** (AC1, AC2)
  - [x] Convert CPU percentage to exact decimal cores; use independent PID and FD limits; preserve backend defaults for omission.
  - [x] Use the existing signed-image/rootless-runtime/proxy fixture and real Worker entry point. Inspect the running container before its existing auto-removal; no fake runtime is acceptance evidence.
  - [x] Verify task-limit exhaustion/FD refusal with bounded fixtures and preserve credential/proxy/image custody.
- [x] **T4 — Journal the actual native resource downgrade** (AC1, AC2)
  - [x] Append unit event discriminant 20 and a public `ResourceCeilingDowngradedPayload` in `i10.rs`, beside `ProviderSwitchedPayload` (`:35-70`), using its serde derives, documented fields and `::new` constructor convention (`check_pub_field_constructors.rs:156-191`). The payload names the missing mechanism, the actual spawn's resolved memory/FD caps and the surviving rlimit semantics. The producer serializes with `serde_json::to_vec`, and tests read the payload back with `serde_json::from_slice::<ResourceCeilingDowngradedPayload>`. No payload registry, `JournalEntry` kind, FrameKind, audit kind, WIT or Spirit-ABI change is needed: the readers are generic, and no exhaustive `LifecycleEvent` match exists (`i10.rs:153-196,295-302`; `log_composition.rs:216-260`). Control JSON must not gain a fake SCB state.
  - [x] Consume the parent-computed native downgrade at the production T2 spawned-session owner, after `LiveWorker` ownership and observer registration but before publishing successful startup. Thread the existing shared `Arc<JournalAdapter>` through every session constructor.
  - [x] Distinguish true memory-mechanism downgrade, no requested cgroup resource, strict-cap refusal and failed sandbox setup. Emit once per successful fallback spawn, not again from its pump/finalizer; a distinct restart spawn that falls back gets its own entry.
  - [x] Prove persisted read-back, identity/tier correctness, old-record replay, fast-child event ordering and append-failure propagation; a Worker-only producer is insufficient because production Workers are T3. Preserve raw last-event semantics rather than inventing a recovery-state index.
  - [x] Delete `drop(file);` in `JournalAdapter::append_transition` (`journal/mod.rs:188`; FW-01, deletion-only) so the `most_recent` insert runs under the writer guard. Add the barrier-synchronized concurrent same-Spirit append test, in which the live index equals the reopened index. Record the pre-fix run as a probabilistic stress control.
- [x] **T5 — Install honest CI enforcement** (AC1, AC3)
  - [x] Create `cgroup-ceiling-delegated` and `cgroup-ceiling-undelegated`, with explicit capable/negative preconditions, checked execution and non-vacuity evidence (executed-test counts, never zero).
  - [x] Use delegated-domain/process-leaf provisioning, not the stale epic command. Write a new 17-2 helper that validates its own unit and enables `+cpu +memory +pids`. The negative job uses the `Delegate=no` service and the effective-permission assertions from AC1. Keep privilege in job setup, not in the application under test.
  - [x] Enroll both jobs at these sites:
    - Add both jobs to `aggregate.needs`.
    - Extend the overall-status predicate (`discipline.yml:5042-5048`) and the failure step (`:5207-5211`) with `needs['cgroup-ceiling-delegated'].result != 'success' || needs['cgroup-ceiling-undelegated'].result != 'success'`.
    - Keep `aggregate`'s `if: always()` and keep the macOS applicable-but-unrun step last (`xtask/tests/macos_compile_guard_20_6.rs:259-293`).
    - Add the report/summary rows, so that a skipped required job never reads as success.
    - Add a behavioral workflow regression for skipped, failed, cancelled and zero-test results.
    
    These are aggregate-only test jobs (`xtask/tests/release_workflow_15_4.rs:344-364`): leave `EXPECTED_GATES`, the gate registry and the `v1-0-ship-gate` lists unchanged, and invent no xtask command.
  - [x] Apply the required Ubuntu image matrix; limit T3 portions to x64 where the existing substrate is defined.
  - [x] Run the workspace suite inside the delegated harness under AC3.3's environment contract and in-context rootless canary. Keep negative fixtures in their own `Delegate=no` service and retain bounded process cleanup and CI timeouts.
  - [x] Execute the behavioral proven-red controls and archive the command, runner and results for each claimed platform through the CI route that §Readiness re-validation records as authorized.
- [x] **T6 — Close contracts, documentation and review** (all ACs, delivery gate)
  - [x] Update the EN/KO manifest/cookbook/deployment resource claims, the retired implementation pointer, and any architecture statements contradicted by shipped behavior.
  - [x] Measure the final formatted kernel/per-crate/aggregate deltas against FW-02/FW-03 and return any figure above a current ceiling as a request before landing. Keep kernel edits inside the FW-01 locations, and re-pin the complete affected set in the decision record's order.
  - [x] Run focused native/T3/session/supervision regressions, relevant contract gates, macOS compile guard and the repository's required whole-workspace/CI lanes.
  - [x] Perform the non-author §A6 blind, edge-case, acceptance, test-infrastructure, security and runtime review. Fill the Dev Agent Record with actual commands/evidence; do not present 17-7's handoffs as completed work.

### Review Findings

Code review 2026-10-12, chunk 1 of 2: production source (kernel, domain, manifest, `maos-bin`) plus `discipline.yml`, `scripts/` and `xtask/`. The integration tests, fixtures and docs (chunk 2) are not yet reviewed. Review layers: Blind Hunter, Edge Case Hunter and Acceptance Auditor. The Acceptance Auditor found no AC/charter violation in this chunk. One finding was dismissed: the `DOCKER_HOST` passthrough, because production `t3/spawn.rs:47-61` clears the runtime-client environment the same way.

- [x] [Review][Patch] Document that `pids_max` is enforced only on Linux. Operator decision 2026-10-12: accept the Linux-only scope; no kernel charter extension. Native macOS (`macos.rs:22-24`) and Windows (`windows.rs:279-299`) sandboxes ignore an explicit `pids_max`, with no refusal or diagnostic. The EN/KO resource docs must say so rather than imply cross-platform enforcement. [docs-site/docs/cookbook/manifest-fields.md:89; docs-site/docs/manifest/v5.md:72; KO counterparts]
- [x] [Review][Patch] The `check_test_output` SKIP guard misses libtest-prefixed lines. Under `--nocapture --test-threads=1`, libtest flushes `test <name> ... ` before the test body runs, so an `eprintln!("SKIP …")` lands mid-line and `^\s*SKIP\b` never matches. The unit test passes only because its fixture puts SKIP at column 0. [scripts/run_cgroup_ceiling_17_2.py:29; scripts/tests/test_cgroup_ceiling_17_2.py:48]
- [x] [Review][Patch] The helper unconditionally runs `systemctl stop <unit>` in its `finally`/signal path, even when `systemd-run` refused because another live invocation owns that unit name. It then stops that invocation's tests. Use a per-invocation unit name, or prove ownership before stopping. [scripts/run_cgroup_ceiling_17_2.py:324,345]
- [x] [Review][Patch] The `--unit` regex accepts a `.service` suffix. Validation and stop then target `<name>.service.service`: membership validation fails closed, and the signal-path stop misses the real unit. Reject the suffix. [scripts/run_cgroup_ceiling_17_2.py:286,57]
- [x] [Review][Patch] The rustdoc for `pub fn run_cli_wrapper_manifest` (the 16-3 supervision/guard-`Drop` contract) now attaches to the private `resolve_worker_caps`, which was inserted between the doc block and the function. Move the helper above the doc block. [crates/maos-bin/src/worker_spawn.rs:382-409]
- [x] [Review][Patch] The `policy:` field insertion overwrote the first comment line, "Story 11.1a's Spirit Host Port is a `maos run` composition; the daemon", leaving a dangling fragment. Restore that line. [crates/maos-bin/src/main.rs:10179-10180]
- [x] [Review][Patch] The Worker resource error carries a doubled prefix: the user sees `maos run: resource caps: maos run: [resources] parse: …`. Drop one of the two prefixes. [crates/maos-bin/src/worker_spawn.rs:416-418,462]
- [x] [Review][Patch] The aggregate writes the two cgroup rows to `$GITHUB_STEP_SUMMARY` with no table header, so they render as literal pipe text. They also duplicate the PR-comment table rows. Add a header or drop the lines. [.github/workflows/discipline.yml:5137-5138]
- [x] [Review][Patch] A stray blank line separates the doc comment from `pub struct SandboxedChild` (clippy `empty_line_after_doc_comments`). Because the file is a kernel file, fixing it means re-pinning the physical count, file/set hashes and the `kernel_pin_content_hash_16_0.rs` literals; tokei is unchanged. [crates/maos-kernel-core/src/security/sandbox/mod.rs:89-90]

All nine patches were applied on 2026-10-12. Every change made exact lower re-pins only:

- **Kernel physical:** 25146 → 25145. The `src_lines` assignment stays on line 505, and the `sandbox/mod.rs` file hash and the set hash were re-pinned.
- **Kernel tokei:** unchanged at 19492.
- **`maos-bin` tokei:** 25397 → 25396.
- **Aggregate hard-fail:** 173502 → 173501.

Verification run locally:

- **Gates:** `check-kernel-baseline`, `check-service-boundary` and `kloc-check` pass.
- **xtask:** the full `cargo test -p xtask` passes.
- **Helper:** the unit tests pass 7/7. Against a real libtest log, the pre-patch helper accepted `test needs_substrate ... SKIP …` and the patched helper refuses it.
- **`maos-bin`:** `resource_policy_tests`, `two_host_delegation_2b::malformed_worker_resources_refuse_before_cli_probe_or_binding` and `cookbook_manifests_17_3b` pass.
- **Real binary:** `maos run` with a manifest declaring `memory_max_mb = 0` prints the single-prefix error `maos run: [resources] parse: resource cap out of range: memory_max_mb = 0`.
- **KO glossary lock:** the gate's algorithm, replayed in Python on the two changed page pairs, reports no violation. No Node runtime is available locally, so the npm gates themselves did not run.
- **Not run:** CI and hosted delegated runners.

**Chunk 2 of 2 (2026-10-12):** the integration tests, fixtures and docs (38 files, 4063 diff lines). Review layers: Blind Hunter, Edge Case Hunter and Acceptance Auditor. The Edge Case Hunter was asked for a second, exhaustive pass. The Acceptance Auditor's first result was empty, so it was asked to substantiate it. It returned a per-item coverage table with no AC violation; the topology parity gap below is the only AC2.7 issue found, and it came from triage.

Ten findings were dismissed:

- **RLIMIT_AS rather than `memory.max` producing the 96-MiB reservation failure:** sanctioned by Dev Notes, which require accepting only the mechanism actually observed.
- **The `assert_no_owned_leaf` early return:** each failure case asserts its stage-specific error first.
- **Unbounded `collect_until_exit` and nested `cargo build`:** bounded by the fixtures and by the unit's `RuntimeMaxSec`.
- **The `MAOS_17_2_CGROUP_MODE` panic:** this is the required run-or-fail behavior, and the harness is documented.
- **`pids_max` in the T0 examples (three findings):** the T0 examples already carried an inert resource block.
- **The architecture "persists" wording:** an append failure panics before Ready.
- **Skipping a container whose inspect fails during Drop:** its ownership is unknown, and production `--rm` plus the finite fixture remove it.

- [x] [Review][Patch] EN and KO topology diverge, and the EN text overstates. EN `:143` says "Worker manifest resource limits are applied…" without qualification, while KO `:144` scopes it to T3 Workers. KO `:148` replaced the CI-proof sentence with native x64/arm + T3-x64-only scope, but EN `:147` still reads "CI proof uses rootless podman on Ubuntu 24.04 and 26.04". Align both on one accurate statement. [docs-site/docs/deploy/topology.md:143,147; docs-site/i18n/ko/docusaurus-plugin-content-docs/current/deploy/topology.md:144,148]
- [x] [Review][Patch] `explicit_pids_cap_is_applied_or_spawn_is_refused` passed on any `SandboxSetup(_)`, so a regression that fails, for example, the `pids.max` write would pass this test. Applied: accept only `CgroupUnavailable`, or a `SandboxSetup` with the delegation-stage reasons `cgroup placement:` / `cgroup creation:`. A `cgroup limits:` failure happens after the owned leaf exists, so it is now a test failure. [crates/maos-kernel-core/tests/resource_caps_linux.rs:55]
- [x] [Review][Patch] `Root::drop` runs the runtime's `ps -a` and `inspect` without a deadline; only `stop` and `rm` go through `timeout 8s`. A hung runtime during panic teardown therefore blocks the test until the unit's `RuntimeMaxSec`. Applied: `runtime_command()` now wraps every runtime-client call in `timeout 8s`, and teardown reuses it. [crates/maos-bin/tests/worker_resource_limits_17_2.rs:101-115,141-172]
- [x] [Review][Patch] The smoke's compiled fixture binary leaked into the system temp directory on every run. `FIXTURE` is a `static LazyLock<TempPath>`, and statics are never dropped, so `TempPath::drop` never ran. Applied: the fixture is compiled to a per-process staging name under `CARGO_TARGET_TMPDIR`, then renamed to `cgroup-ceiling-fixture`; the inert guard is deleted. [crates/maos-kernel-core/tests/cgroup_ceiling_smoke.rs:24-47]
- [x] [Review][Defer] In delegated live-session tests, a failed assertion leaves the detached session, its T2 child and its cgroup leaf running until the session's own deadlines; the multi-thread runtime's shutdown waits on the blocking session. `stop_workers()` runs only after the assertions. The same pattern already exists in the 17-3c tests (`HEAD:spawned_scope_17_3c.rs:1374,1602`), so a stop-on-drop guard belongs across the whole suite. [crates/maos-bin/tests/spawned_scope_17_3c.rs:2757-2800] — deferred: suite-wide cleanup convention that predates 17-2; only failing runs are affected.

All four chunk-2 patches were applied on 2026-10-12. Verification run locally:

- **Format:** `cargo fmt --all -- --check` passes.
- **`resource_caps_linux`:** 1/1 passes on the refusal path. The local cgroup2 mount is read-only.
- **`cgroup_ceiling_smoke` fixture path:** the test publishes `target/tmp/cgroup-ceiling-fixture` with no staging residue and no new `/tmp` binary. It then fails as expected at the delegated spawn, because this container has no writable delegated cgroup.
- **`worker_resource_limits_17_2`:** compiles with `network,wasm-host`. The run used an isolated target directory, because another process was deleting `/mnt/build/cargo/debug/.fingerprint` during verification.
- **Topology:** the KO glossary-lock algorithm, replayed in Python on the page pair, reports no violation.
- **Not run:** delegated, T3 and CI.

## Dev Notes

### Architecture and ownership

The existing route is **manifest → resource policy resolution → admitted `SandboxSpec` → OS spawn → owned guard → composition-root lifecycle journal**. Ordinary class admission already returns a resolved `SandboxSpec`; `spirit_session.rs` transfers it into the T2 bridge. The Worker is the exception: it has a wrapper-specific admission path and rebuilds a default spec. Fix that exception without inventing another admission system.

`SecurityManagerAdapter::admit_spirit` already reads `PolicyTable.operator_policy.resource_cap_floor` and calls the pure `resolve_caps` (`security/mod.rs:428-435`). **Reuse the existing public authority access; a new kernel resolver API is unnecessary:** `SecurityManagerAdapter::policy()` (`security/mod.rs:236-239`) exposes the policy table, and `PolicyTable::inner().load_full()` exposes its current snapshot. Thread that existing authority to `run_cli_wrapper_manifest`, read the same resource-cap floor and call the existing pure resolver. Migrate the topology and standalone calls in `main.rs`, the Host-B call in `delegation.rs`, `HostBWorkerContext` and its constructors/tests together. No production configuration writer for the policy slot was found: demonstrate tighter injected/current policy, but do not claim a new operator CLI/configuration surface.

Keep OS handles and downgrade outcomes per-spawn and transient. The kernel does not acquire an inference policy, resource planner, persistent cgroup database or runtime implementation of application configuration. No new crate or dependency is required.

**Complete composition wiring:** the Host-B route is the cohort dispatch (`main.rs:7875`) → `run_cohort_a2a_daemon` (`:10200`) → `host_b_worker_context` (`:10126`, called at `:10324`) → `HostBWorkerContext` (`:10163`) → `handle_one_inbound` (`delegation.rs:688-701`). The separate transport builder does not construct this context. Carry an owned clone of the existing authority, clone it before the blocking `move` closure, and read its current snapshot for each launch; do not freeze boot-time caps or move away the root teardown's owner. In `tests/two_host_delegation_2b.rs::in_process_context`, lift the policy currently constructed inline for the capability registry and share that same authority with the context; a second default policy would invalidate precedence evidence. No new `RootSupervision`/`WorkerSupervisor` constructor dependency is needed.

There are two `SessionDeps` literals: `main.rs:5581` and `tests/spawned_scope_17_3c.rs::World::session` (`:237`); `governed_session` reuses the latter. Add a clone of their existing lifecycle journal while **retaining the existing Transparency Log field**, whose frame/authorization/finalization consumers still need it. In `spirit_session.rs`, capture necessary effective-tier/cap metadata before `admitted.sandbox` is partially moved (`:255`) and the sandbox is moved into `BridgeSpawnSpec` (`:270`). Reporting after guard/observer installation cannot borrow those moved values. Actual startup publication is `mark_ready`/`spirit_loaded` (`:380-387`), not the caller's session-complete marker.

### File implementation map: current / change / preserve

| File or group | Current state | Required change / preservation |
|---|---|---|
| `crates/maos-manifest/src/manifest.rs` | Optional CPU/memory/FD; zero checks; `strictest_opt`; strict raw serde | Add PID field/validation/resolution and literal migration. Preserve parser errors, independent dimensions and old omission behavior. |
| `crates/maos-bin/src/worker_spawn.rs` | Wrapper/on_crash/supervision parsing, probes, host grant, bind/mint, T3 launch; ignores resources | Parse and carry caps before side effects. Preserve `LiveWorker` binding-before-bridge drop order, stop-before-spawn checks, image/proxy lease, real identity and completion-from-persisted-output oracle. |
| `crates/maos-bin/src/supervision.rs` | Owns scheduler, Worker binding/task lifecycle, container reporting and phase guards | Narrow resource-policy/journal ownership wiring where needed. Preserve single SCB, planned-stop versus crash classification, token revocation and root shutdown. Do not redesign container termination: that is the accepted 17-7 handoff, which remains outside 17-2. |
| `crates/maos-bin/src/spirit_session.rs` | Real T2 WASM launch with admitted spec, session dispatch and finalization | Consume native downgrade through the existing lifecycle owner; preserve intent/consent/lineage/capability mediation and bridge custody. Add required port dependency at all session constructors. |
| `crates/maos-bin/src/{main.rs,delegation.rs}` | Main creates shared security/journal owners and calls Worker launch for topology/standalone; Host-B owns a remote Worker context | Thread existing policy authority to all three Worker routes and the shared lifecycle JournalAdapter to `SessionDeps`; migrate context constructors/tests. Preserve remote governance, blocking execution and single shared journal. `security/mod.rs` needs no new resolver API. |
| `crates/maos-kernel-core/src/security/sandbox/linux.rs` | Real T2 exec set, kill-first seccomp, CPU/memory cgroup + rlimits; fallback when no CPU cap | Add PID control and explicit resource outcome; preserve parent preparation and raw pre-exec `write(0)` before guest execution. No allocation/formatting/journaling in `pre_exec`. |
| `crates/maos-kernel-core/src/security/sandbox/mod.rs` | Native child guard and cgroup cleanup | Carry transient result, preserve kill/reap and bounded rmdir behavior. Prefer a Linux-only field/accessor to unnecessary platform constructor churn. |
| `crates/maos-kernel-core/src/security/sandbox/t3/argv.rs` | Verified image and isolation argv; wrong CPU unit, PID/FD alias | Correct only resource mapping; preserve `--network=none`, rootless policy, no-new-privileges, read-only root, proxy socket and names-only env. |
| `crates/maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs` | Plain/T2/container child variants, bounded stream framing, real T2 OOM read before cleanup | Expose one-shot native resource status. Preserve cap binding/env clearing, T3 route, stdin posture refusal and existing exit/audit behavior. |
| `crates/maos-kernel-core/src/scheduler/{resource_ceiling.rs,mod.rs}` | Dead root-layout API and declaration | Delete the module and export, not just its Linux branch; no shim. |
| `crates/maos-kernel-core/src/journal/mod.rs` | `append_transition` writes under the writer mutex, releases it (`drop(file);`, `:188`) and only then inserts into `most_recent` (`:189-195`). Concurrent same-Spirit appends can therefore leave the live index out of file order. | Delete `drop(file);` so the index insert runs under the writer guard (FW-01, deletion-only, −1). The lock order is writer → index, and readers take only the index. Preserve panic-on-append-failure, the raw last-event contract and the full scan. |
| `crates/maos-kernel-core/src/scheduler/scheduler_loop.rs:324-328` | Internal all-None `ResourceCaps` literal | Add PID omission mechanically; do not broaden this story into scheduler admission redesign. |
| `crates/maos-domain/src/invariants/i10.rs` | Copy enum 0–19 plus payload-bearing lifecycle entry | Add unit event 20 and typed payload; preserve historical replay and discriminants. |
| `crates/maos-kernel-core/tests/cgroup_ceiling_smoke.rs` | Ignored, env-gated, fake PID 99999, obsolete API | Rewrite around real native child membership/enforcement/cleanup and explicit CI substrate. |
| `crates/maos-kernel-core/tests/{resource_caps_linux.rs,sandbox_enforcement_windows.rs,manifest_field_coverage.rs}` and manifest inline tests | Existing caps literals and three-class field coverage | Migrate literals; retain genuine platform behavior, add resources field fixtures in the existing hierarchy. |
| `.github/workflows/discipline.yml`; `scripts/run_17_3c_delegated.sh` only if shared generalization is justified | T2 four-image job, x64 T3 provisioning, 17-3c named delegated-unit helper, workspace/aggregate jobs | Add cgroup jobs and workspace substrate coverage. The 17-3c helper accepts only its named unit and cpu/memory: do not call it under a new arbitrary unit or silently replace its contract. |
| `crates/maos-bin/tests/{spawned_scope_17_3c.rs,two_host_delegation_2b.rs}` | Sole test `SessionDeps` constructor (`World::session`, `:237`) with the real admission/journal/runner World; sole test `HostBWorkerContext` constructor (`in_process_context`, `:699-800`) | Add the shared lifecycle-journal clone and the lifted shared policy authority. Host the echo-component delegated-membership and undelegated-downgrade session oracles in the existing World; run them with `--features network,wasm-host` and assert that they actually execute. |
| New 17-2 delegation helper (`scripts/`), the native PID-exhaustion ELF fixture, and the T3 resource suite (`crates/maos-kernel-core/tests/t3_argv_custody_17_1.rs` or a sibling) | The 17-3c helper provides only cpu/memory for its own named unit. No native task-exhaustion fixture exists. The T3 argv test covers environment custody, not resources (`t3_argv_custody_17_1.rs:48-66`). | Helper: validate its named unit, assert `pids` is available, create the `daemon` leaf, enable `+cpu +memory +pids`. Fixture: a bounded native ELF that uses fallible thread creation or an allowed `clone`/`clone3`; never raw `SYS_fork` (absent from the T2 allowlist, `linux.rs:25-34,271-329`) and never exec of an ungranted second binary. T3 resource suite: CPU decimal cores, independent `--pids-limit`/`--ulimit=nofile`, omission defaults, and the above-hard-limit refusal. |
| `docs-site/docs/{manifest/v5.md,cookbook/manifest-fields.md,cookbook/cli-wrapper-spirit.md,deploy/topology.md}` and corresponding `i18n/ko/docusaurus-plugin-content-docs/current/` files | Published resource and Worker limitations | Explain optional PID cap, CPU units, FD independence, delegated prerequisites and actual downgrade/refusal behavior in both languages. Preserve acknowledged limitations not fixed here. Add the explicit retired-public-API migration note in `BREAKING.md`; do not hand-edit generated `STABILITY.md`. |
| `docs/red-team/README.md:78`; architecture `4-kernel-design.md` resource sections | Resource-exhaustion pointer and OS containment claims | Retire the old resource_ceiling pointer; document native versus container mechanisms accurately. |

`security/manifest.rs` is a re-export, not the parser to reimplement. `maos-manifest/src/lib.rs` already exports caps. Existing resource literals also occur in manifest strictest-of tests; do not stop at production compilation. Run language-server references for exported symbol changes. Creation queried `ResourceCaps` and `ResolvedCaps`; the retired `apply_resource_ceiling` API had only the smoke test's callers.

### Delegation and cleanup guardrails

- Provisioning precedent is the **system-manager** service the existing delegated jobs start from setup: `sudo systemd-run --unit=<named> --wait --pipe --collect --uid=<runner> --gid=<runner> --property=Delegate=yes …` (`.github/workflows/discipline.yml:1878-1882,3328-3334`), not `systemd-run --user`; do not claim the user-manager route was exercised merely because `scripts/run_17_3c_delegated.sh:14-20` accepts its path. `Delegate=yes` makes available controllers delegable, but descendants still need explicit `cgroup.subtree_control` enablement ([systemd delegation](https://systemd.io/CGROUP_DELEGATION/#delegation)). The 17-3c helper enables only `+cpu +memory` (`run_17_3c_delegated.sh:21-26`). A new 17-2 helper validates its own named unit, asserts `pids` is listed in the domain's `cgroup.controllers`, moves only the test process into a dedicated `daemon` leaf, and enables `+cpu +memory +pids`. Production discovery climbs to the provisioned domain.
- Discovery of a readable `cgroup.subtree_control` is not proof of write access. The creation/open/limit writes remain checked. Do not take the read-only local container mount as evidence that a hosted delegated job cannot work.
- PID control counts tasks/threads, not just process-group leaders. Use a bounded clone/fork workload, observe `EAGAIN`/`pids.events` on exhaustion and terminate all descendants. Never perform an unbounded fork bomb on the developer host.
- `RLIMIT_AS` remains applied even when `memory.max` is available. Allocation failure can precede a cgroup OOM kill. Tests must accept only the mechanism they actually observe; do not weaken the resource limit just to force an OOM verdict.
- Cleanup's bounded backoff is already present; a populated or inaccessible leaf can remain after exhaustion. Do not add recursive filesystem deletion or claim the common `maos.slice` parent must disappear after each child.

### Lifecycle outcome contract

The implementation must distinguish **applied**, **allowed resource-only fallback**, and **refused**. This need not become a large state-machine abstraction: a small parent-computed result on the guard, consumed once by the composition root, is sufficient.

A downgrade row names the actual missing mechanism, requested/resolved limits where needed to understand the event, and the fallback actually applied. No requested cgroup resource means no false resource-loss claim. Explicit CPU/PID requests do not receive an allowed fallback. Landlock/seccomp/exec failure never becomes a successful downgrade. The event producer belongs to the native T2 launch route, not solely to the T3 Worker path and not to `pump_to_journal`'s child-output stream.

Use the existing **`JournalAdapter`**, not a similarly named TL handle. `SessionDeps.journal` (`spirit_session.rs:54`) is a `TransparencyLogAdapter`; the shared lifecycle sink already exists in `AdmissionDeps.journal` (`admission.rs:607`, constructed as `shared_journal` in `main.rs:2741-2759`). Thread the existing **`Arc<JournalAdapter>`** into the session; do not open a second journal. `SessionDeps` crosses `spawn_blocking` (`spirit_session.rs:245`), so erasing it as bare `Arc<dyn SpiritSchedulerPort>` loses required thread-safety: an intentionally erased handle needs `+ Send + Sync`, not a public trait redesign. `JournalAdapter::journal_lifecycle` (`journal/mod.rs:280-282`) delegates to `append_transition` and retains the complete payload.

**Do not call `SpiritSchedulerAdapter::journal_lifecycle` as a shortcut:** its implementation (`scheduler/scheduler_loop.rs:733-759`) writes a reduced event to the Transparency Log and discards the payload. The identical method name does not imply the same sink. Consume the result only after the `LiveWorker` guard owns the child (`spirit_session.rs:283-284`) and before successful startup is published; preserve observer ordering. This is post-spawn reporting, not a newly promised suspended-child protocol.

Keep the enum stable and the payload serializable/typed. `JournalAdapter::append_transition` panics on write failure; its background drain and private `sync_flush`/Drop call `sync_data` but ignore that call's errors (`journal/mod.rs:144-157,177-195,256-277`). Verify the real persisted row by releasing all owners and reopening the journal; do not invent a public flush API or equate read-back with proven power-loss durability. Preserve the existing append-failure contract and child guard so an unjournaled child cannot be published as successful; redesigning general fsync error handling is not added scope.

**Linearize the live index with the file (FW-01).** This story creates the first concurrent same-Spirit producer pair:

- the session thread appends the downgrade;
- the pidfd observer's crash handler appends `Crash` (`supervision.rs:2487-2535`; `crash_detector.rs:180-195`).

Today `append_transition` releases the writer mutex before updating `most_recent` (`journal/mod.rs:188-195`). The file can persist `Crash → ResourceCeilingDowngraded` while the live `last_event` reports `Crash`, so the fast-child oracle below would intermittently lie. Delete `drop(file);` so that the index insert completes under the writer guard; there is no other change to the journal. Prove it with a barrier-synchronized multi-thread same-Spirit append test: the live `last_event` equals the reopened `last_event`, and the live index matches the final persisted row. The fixed test is deterministic. The pre-fix red is probabilistic, so report it as a stress control, not a guaranteed red.

The downgrade is **informational, not an SCB state transition**, like the existing `ProviderSwitched = 18` payload event that admission appends after `Load` (`i10.rs:113-114`; `security/mod.rs:446-499`). Preserve the port's documented raw “most recently journaled lifecycle event” contract (`ports/scheduler.rs:21-27`): live and reopened `last_event` include every lifecycle event, and `recover_in_flight_with_tasks` returns the full sequence (`journal/mod.rs:129-131,189-211,238-245`). `recover_in_flight` returns **every** indexed identity, including those whose last row is `Crash`/`Unload`; it is not a liveness predicate (`:206-212`). Do not silently filter this event or invent a last-state-transition index. Current state authority is the live SCB and binding phase, not journal replay: the daemon boots a fresh empty SCB map and never rehydrates from these APIs (`main.rs:2725-2778`; `scheduler_loop.rs:132-167`); the only production recovery call is the smoke arm's task-row count (`main.rs:6678-6702`). The independent observer may append `Crash` before the parent reports the downgrade; `CrashDetector` has already moved the SCB to `Unloaded`, torn down and removed it (`crash_detector.rs:95-130,180-195`), and a later informational append cannot reinstate it. Any future replay consumer must reduce state-changing events and treat informational ones as state-neutral. Do not move journaling ahead of observer registration as an implicit fix or claim a suspended-child guarantee.

### Tests and execution plan

Permanent tests should prove the following distinct behaviors; reuse existing suites where possible:

| Scenario | Observable oracle |
|---|---|
| PID parsing and omission | Zero/negative/overflow/wrong type refused; absent field preserves compatibility; positive value round-trips through real manifest path. |
| Resource precedence | Manifest tighter, operator tighter, sole present value and both absent; FD and PID unequal and independently resolved. |
| Real native delegated spawn | For the echo WASM session, the runner `child_pid`'s membership, exact `cpu.max`/`memory.max`/`pids.max`, and placement before guest code; same-id concurrent sessions receive different owned leaves. |
| Native PID exhaustion | The bounded native ELF fixture, through the T2 `spawn_and_bridge` route in the kernel smoke, holds its created tasks until inspection. Further task creation fails, and the owned leaf's `pids.events:max` increments. The fixture has a fixed upper bound and timeout and releases all its tasks; siblings and the host continue, and teardown leaves no owned populated leaf. A WASM guest cannot create native tasks (`runner.rs:206,237-240`), so this oracle is not a session test. |
| Native fallback and strict refusal | In the `Delegate=no` service, the 64-MiB/64-FD echo session runs with exact live rlimits and records one persisted downgrade. CPU or PID requests refuse before guest code runs. No success row is invented. |
| Real T3 Worker | Nondefault Worker `[resources]` survives parsing and admission into runtime inspect and process limits; FD and PID differ, CPU is percentage-derived. |
| No-resource Worker compatibility | Existing signed/proxy fixture without `[resources]` still uses documented T3 backend defaults. |
| Failure and cleanup | Partial native setup, exec failure and killed descendant trees release owned resources or report a real bounded-cleanup failure; no unrelated ancestor removal. |
| Lifecycle compatibility | Existing discriminants/old records preserved; correct identity/payload read back; write failure is not success. Cover fast-child `Crash → ResourceCeilingDowngraded`: crash handling still unloads/removes the live SCB and is not repeated; after releasing owners and reopening, the raw last event and full sequence match persisted file order. Do not claim boot-time state rehydration, which does not exist. |
| Production regressions | 17-1 egress/env/image/custody, 17-6 T2 SIGSYS/Landlock, 17-3c capability-restamped session, Worker unload/crash/hang behavior remain intact. |

Existing command surfaces to use after implementing the new behavior (a command here is a plan, not an executed result):

```sh
cargo test --locked -p maos-kernel-core --test cgroup_ceiling_smoke -- --nocapture
cargo test --locked -p maos-kernel-core --test resource_caps_linux -- --nocapture
cargo test --locked -p maos-kernel-core --test sandbox_t3_spawn_linux -- --nocapture
cargo test --locked -p maos-kernel-core --test cli_wrapper_t3_boundary_17_1 -- --nocapture
cargo test --locked -p maos-kernel-core --test manifest_field_coverage
cargo test --locked -p maos-bin --features network,wasm-host --test spawned_scope_17_3c -- --nocapture
cargo run --locked -p xtask -- check-kernel-baseline
cargo run --locked -p xtask -- check-service-boundary
cargo run --locked -p xtask -- kloc-check --json
```

Run substrate-dependent commands only in their declared capable job; the new undelegated job must exercise the genuine negative environment, not simply omit setup and assume its runner is undelegated. **Default maos-bin features do not compile the session changes:** the module and integration test require `network,wasm-host`; a test invocation without them can select zero tests. Assert actual test execution. Guard a Linux-only outcome accessor's session consumer with `target_os = "linux"`, not `unix`, because the feature-enabled session also compiles on macOS. Enumerate all-target callers and retain the existing macOS compile guard; Linux runtime evidence is not cross-platform evidence. The full repository acceptance lanes remain required at implementation close.

Existing incidental source scanners (`enterprise_daemon_seam_13_5a.rs:83-97` and `main.rs:14449-14458`) parse signatures at the first literal `)` or search a fixed 3,300-character window. Resource-parameter plumbing can trip them without any behavioral regression. If affected, delete those incidental assertions rather than re-pin a source window or contort production argument placement; retain or verify the real enterprise-governance behavior through its existing runtime tests. This is not permission to weaken behavioral or public-contract gates.

### Previous-story intelligence and git history

`17-1-worker-egress-allowlist-and-scoped-credential` is **done** in sprint tracking at creation, with its non-author review closed on 2026-10-10. Its earlier epic landing paragraph still says review; the tracker is authoritative. The actual current kernel pin is 25267, not the pre-review 25212 or pre-17-1 25293.

- Preserve the real T3 route, rootless runtime refusal, verified image, private proxy socket, scoped bearer, names-only container environment and cleared host child environment. Do not restore the old bare spawn or the deleted pretend quarantine/escape paths.
- Preserve the first Worker logical identity, real SCB/task binding, observer-before-pump ordering, and guard-first `LiveWorker` field order. Rebuilding admission to recover caps can break all four.
- The current 17-1 fixture/probe manifests omit resources. Optional resources must not make those valid manifests fail. Use a dedicated resource-bearing fixture or explicit additions to the intended acceptance manifest, not a broad rewrite of every Worker manifest.
- Existing capable-runner evidence is CI-owned. No real-container proof was claimed locally by this story-creation run.
- Documentation is bilingual; preserve the 17-1 honesty corrections and run existing Korean coverage/glossary checks when those pages change.

Last five commit titles inspected: `b19556ab` 17-1 implementation; `8f6869b8` 17-1 frontier re-verification; `f88a1968` 20-6 macOS compile-guard completion; `d371b5a4` 20-6 green evidence; `9e70ba1f` delegated TS proof annotation. The relevant implementation is `b19556ab`: kernel sandbox/bridge, Worker supervision, T3 provisioning/exit CI, manifest fixtures and EN/KO docs changed together. Do not reuse old epic line numbers as current implementation evidence.

### Measured prototype — E16-A7 creation precondition

Creation used an isolated source copy, never the production source tree. Evidence is retained in:

- `_bmad-output/implementation-artifacts/17-2-evidence/prototype.diff`
- `_bmad-output/implementation-artifacts/17-2-evidence/measurements.json`

The mechanism prototype was formatted and compiled through a throwaway executable. It exercised real T2 startup on this host: an explicit PID cap refused on the read-only cgroup mount; a memory-only T2 child exited 0 and returned one consumable downgrade reason; manifest PID zero rejection and independent FD/PID strictest-of behavior passed. The first compile exposed the scheduler's forgotten `ResourceCaps` literal; the corrected literal is included in the measurement. Existing compiler warnings remained.

| Instrument | Current source | Mechanism prototype | Delta |
|---|---:|---:|---:|
| Kernel physical `.rs` lines under `src` | 25267 | 25110 | -157 |
| Kernel tokei 14.0.0, canonical exclusions | 19553 | 19454 | -99 |
| Manifest tokei, canonical exclusions | 4456 | 4467 | +11 |

Kernel prototype changes: runtime accessor +9; scheduler module export −1; obsolete module −170; scheduler cap literal +1; Linux implementation −1; native guard +8; T3 argv −3 physical lines. Manifest adds 11 physical/code lines. The prototype's one-shot `Option<String>` is a sizing seam, not permission to omit the typed lifecycle payload/production consumer.

**Not measured as implemented:**

- production Worker resource-policy plumbing and persisted lifecycle emission;
- the journal-guard deletion (−1);
- all-target/test literal migration;
- delegated resource exhaustion and T3 inspect;
- CI jobs, docs and final surface pins.

17-7's container terminal-facts and startup work is out of scope entirely. A cleanup diagnostic may add kernel lines; existing public policy access avoids a resolver API. Plan from the deletion-inclusive measurement, then remeasure the real complete diff. No speculative final number is presented as observed.

Current exact budgets: kernel 19553; domain 9329; maos-bin 25247; manifest 4474; xtask 44370; aggregate hardfail 173383. Kernel, domain, bin and xtask are not a pool of interchangeable headroom. Manifest's measured 4456 leaves 18 code lines below its ceiling, and the minimal parser prototype uses 11. **`maos-domain` (9329) and `maos-bin` (25247) measured exactly at their ceilings at creation.** Their growth (event 20 and payload; Worker/session plumbing) will return as measured requests before landing; the 2026-10-08 domain bound of ≤ 9333 (`kloc.toml:411`) covers at most +4. The numerical rule is FW-02/FW-03 in `17-2-flag-winston-decision.md`: no raise, with exact figures at landing. Do not set budget pins before landing, and take no authority from the stale epic forecast.

### Latest technical information and library constraints

- Workspace: Rust 2021, MSRV 1.88, floating `stable` toolchain; creation host `rustc 1.98.1`. Keep `Cargo.lock`, existing libc/Landlock/seccompiler/Tokio/runtime integration and `tokei 14.0.0`; this story needs no dependency upgrade.
- [Linux cgroup-v2 documentation](https://www.kernel.org/doc/html/latest/admin-guide/cgroup-v2.html) specifies top-down controller enablement, the no-internal-process constraint, common-ancestor migration permissions, task-count PID limits, and empty-cgroup `rmdir`. Read the [upstream text source](https://www.kernel.org/doc/html/latest/_sources/admin-guide/cgroup-v2.rst.txt) if the HTML reader returns only metadata.
- [Podman run documentation](https://docs.podman.io/en/latest/markdown/podman-run.1.html) defines `--cpus` as cores, `--memory` as a size and `--pids-limit` independently of `--ulimit`. Rootless resource support requires cgroup v2/delegation; presence of the executable or generated argv is not enforcement proof. Preserve the repository's deliberate backend defaults rather than inheriting a changed runtime default.
- Upstream latest stable verified through the GitHub release API: [Podman 6.1.3](https://github.com/podman-container-tools/podman/releases/tag/v6.1.3), published 2026-09-29. Local executable is 5.4.2; the story does not silently upgrade it or assume runner versions match.
- [GHSA-2cvf-wqm6-wr9g / CVE-2026-94603](https://github.com/podman-container-tools/podman/security/advisories/GHSA-2cvf-wqm6-wr9g) concerns checkpoint images bypassing user-requested sandbox options; patched versions include 5.8.8 and 6.1.3. The advisory explicitly says rootless containers are unaffected because checkpoint/restore is unsupported there. Preserve 17-1's rootless gate; never switch the Worker to rootful Podman to make a cgroup test green. A future rootful path would need a separate security decision and patched runtime/checkpoint-image rejection.

### Scope boundaries and operator rulings

The three ACs are complete without the two 17-1 handoffs below. Neither is closed by this story's PID/CPU/FD work. **Operator ruling 2026-10-11 (D-17-2-FW-07):** both move to the new story **`17-7-t3-worker-retained-lifecycle-and-terminal-facts`**. That story is `backlog` and FLAG-Winston, ordered after 17-2, and cannot leave `backlog` without its own E16-A7 measured prototype.

1. **Authoritative T3 OOM/signal facts** (`deferred-work.md:1119`).
   - Today the journal records the runtime client's exit 137. That cannot show whether the container process was OOM-killed or externally SIGKILLed.
   - Conmon flattens signal death to 128+signal, and its OOM marker fires on `oom` or `oom_kill`.
   - The fix needs terminal facts shared with both classifiers *before* `task.orphaned`. An extra final log row is not enough.
2. **Very short-lived T3 Workers** (`deferred-work.md:1120`).
   - `podman run --rm` followed by `State.Pid` inspection can refuse a legitimate Worker that has already completed.
   - The retries sleep 2,250 ms in total (`t3/spawn.rs:229-258`), so this is not an exact runtime threshold.
   - A correct fix captures startup identity owned by the runtime (`--pidfile`) and changes `SandboxInspectReport`'s PID semantics once, in 17-7.

17-2 changes neither. Preserve the documented fail-closed behavior, and do not add a success fallback, an arbitrary sleep or `--pidfile` here. The mandatory T3 cap proof uses the existing fixture's three-second hold to inspect the real running container. It must not claim to repair the short-task limitation. Already-assigned 17-1 debts stay outside this story: host-side probes → 19-3, wrapper schema → 20-5, quarantine → 20-1.

### Preflight round-table — 2026-10-11 (operator directive: *resolve the issues raised in the story creation for the long-term correctness*)

Three read-only scouts re-read the story at `b19556ab`, which still matches the working tree's production source. The round-table filed the rows below; epic §R11 carries the rows that reshape other work.

| # | Finding | Disposition |
|---|---|---|
| **RT-1** | Three `spirit_session.rs` cites were off by one: the partial move is at `:255`, the `BridgeSpawnSpec` move at `:270`, and `spawn_blocking` at `:245`. | Corrected in place. |
| **RT-2** | `RATIFIED_AT_EASING` is a kernel-only scalar, `19_553` (`recovery_lane_ceiling_rule.rs:267-279`), not a map. | Corrected in the delivery gate. |
| **RT-3** | The delegation precedent is the system-manager `sudo systemd-run --unit … --uid --gid -p Delegate=yes` (`discipline.yml:1878-1882,3328-3334`), not `--user`. The 17-3c helper enables only `+cpu +memory`. | Corrected in the guardrails: the new helper asserts that `pids` is available and enables all three controllers. |
| **RT-4** | The live journal index is not linearized with the file (`journal/mod.rs:188-195`), and 17-2 adds the first concurrent same-Spirit producer pair. | **Ruled (FW-01): fixed at origin in 17-2** by deleting the `drop(file);` line (−1), with a stress test. |
| **RT-5** | No boot-time SCB rehydration exists. `recover_in_flight` returns every identity and is not a liveness predicate, and `ProviderSwitched` is already an informational row. | The lifecycle oracle was rewritten so it claims only live crash handling and file-order read-back. |
| **RT-6** | The story was marked `ready-for-dev` with no FLAG-Winston record. The 17-1 FW-05 precedent requires the charter first. | **Ruled:** `17-2-flag-winston-decision.md`, ratified 2026-10-11. |
| **RT-7** | Handoffs 1119/1120 share an evidence-lifetime obstacle but not one root cause. The cutover is an estimated +120…+360 kernel lines `[INFERENCE]`, against a measured −157. | **Ruled (FW-07):** new story 17-7. |
| **RT-8** | `[INFERENCE]` The T3 guard is armed before PID acquisition and stops/removes by *requested name* (`t3/spawn.rs:190-205`; `t3/child.rs:88-96`). A failed launch that collides on a name could therefore target a pre-existing container. | Goes into the 17-7 charter (CID-owned cleanup). 17-2 does not touch it. |
| **RT-9** | The OOM/short-task limitations appear in the cookbook (EN `:71`, KO `:72`), not in topology. | T6 rewrites only the resource paragraph (EN `:73`, KO `:74`). The 137 and short-task sentences stay until 17-7. |
| **RT-10** | These checks hold:<br>• there is no production writer for `resource_cap_floor`, so precedence is shown with injected policy only (`cap_policy/mod.rs:30,51`);<br>• all seven re-pin paths exist, and `:505` is `src_lines = 25267`;<br>• 17-1's landing (`b19556ab`) touched all seven, plus its decision record and ADR. | No change. |

### Readiness re-validation — 2026-10-11 (operator question: *is the story truly ready for development?*)

Three fresh non-author read-only scouts (CI executability, the native T2 fixture, contract consistency) re-read the story against `b19556ab`. They found **one BLOCKER**, thirteen SHOULD-FIX items and two NITs. Every SHOULD-FIX and NIT is applied in place above. The BLOCKER was ruled by the operator the same day.

| # | Finding | Disposition |
|---|---|---|
| **V-1 (BLOCKER)** | No 17-2 CI-execution authorization existed. `discipline.yml:3-7` triggers only on main pushes and PRs, and the 17-1/17-6 throwaway-branch grants were per-story (`17-6-…md:63,521-524`; `17-1-…md:345-352`). T5's hosted runs and proven-red mutations could not be executed. | **Ruled 2026-10-11 (Lunarpulse, D-17-2-FW-08):** `ci/17-2-<purpose>` throwaway branches are authorized, under the conditions in FW-08. |
| **V-2** | AC3.3's whole-workspace envelope had no environment contract. A transient system service starts with a clean environment, so the T3 tests could take their local-skip branch. | Environment contract and in-context rootless canary added to AC3.3/T5. |
| **V-3** | The undelegated environment had no construction and no assertion; the hosted runner's own delegation state is **unknown** (no recorded evidence). | A `Delegate=no` system service with effective-permission probes run as the test UID (AC1). |
| **V-4** | The aggregate fan-in sites were unnamed. | T5 now enumerates them; aggregate-only test jobs, with no `EXPECTED_GATES` change. |
| **V-5** | The T2 fixture was ambiguous between the production caller and the sandbox/bridge. | The echo WASM component through real admission and the session is the only production native-T2 route. `<actual-os-pid>` is the runner's `child_pid`. |
| **V-6** | The undelegated fixture and its memory budget were unspecified. The suspected incompatibility between Wasmtime and `RLIMIT_AS` is **not** real: `runner.rs:203-211` sets zero reservation, and 17-3c ran 64 MiB undelegated. | 64 MiB / 64 FD echo session with live `/proc/<pid>/limits`; the 768-MiB TS example is excluded. |
| **V-7** | The PID-exhaustion workload was unspecified. Raw `SYS_fork` is absent from the T2 allowlist, and a WASM guest cannot create native tasks. | A bounded native ELF fixture (threads or `clone`/`clone3`) through `spawn_and_bridge`, checked against `pids.events:max`. |
| **V-8** | An explicit `fd_max` above the user's hard `RLIMIT_NOFILE` fails rootless container start (Podman #5526), and no behavior was specified. | Launch refusal before guest code, never clamped or reported as a downgrade (AC1.2). |
| **V-9** | The payload type and its read-back were unnamed. | `ResourceCeilingDowngradedPayload` in `i10.rs`, on the `ProviderSwitchedPayload` convention, with a confirmed no-ripple inventory (T4). |
| **V-10** | The schema-5 ratification text would have implied old readers accept `pids_max`. | T1 records that pre-17-2 readers reject it, and adds the coverage fixtures. |
| **V-11** | The epic still presented the authoring-time design and its ≤+10 forecast as in force. The 17-1 landing note still routed OOM facts to 17-2, and 17-1 rows still said "grant PENDING". | Fenced as historical; owner set to 17-7; grant shown as ratified. |
| **V-12** | The file map lacked the session/delegation tests, the helper, the PID fixture and the T3 resource suite. | Rows added. |
| **V-13** | "Seven kernel locations" was wrong (eight files), the re-pin list read as an order, and "unaccepted handoff" was stale. | Corrected. |

**Verdict after V-1's ruling:** ready for development. The known stop points are by design:

- **Landing:** `maos-domain` and `maos-bin` are at zero headroom, so their measured requests (FW-03) return to the operator before landing.
- **Charter:** any kernel file outside FW-01 returns as a charter request.

### References and project-context discovery

No `project-context.md` matched the activation glob. BMad config identifies project `maos`, expert reader, English communication and output. Inputs used: selected Epic-17 shard and dependency context; PRD functional/non-functional resource requirements; architecture index and resource/security/ABI/deployment sections; complete previous Story 17-1 continuity; current code/tests/CI/docs; live upstream technical references. The coding-delegation UX analysis is not a new resource-control UI requirement: this story's operator surface is the existing manifest, CLI evidence and lifecycle journal, with bilingual documentation.

Primary local references:

- `_bmad-output/planning-artifacts/epics/epic-17-workers-and-third-party-form-w2.md` — §17-2, runner-image policy, completed dependencies and 17-1 landing notes.
- `_bmad-output/planning-artifacts/prd/functional-requirements.md:24-41` — FR5/FR6, lifecycle preservation.
- `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md` — §4.1 OS resource ceilings, §4.3.1 security/resource distinction.
- `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/5-spirit-abi.md` — resources and lifecycle contract.
- `_bmad-output/implementation-artifacts/17-1-worker-egress-allowlist-and-scoped-credential.md` — ratified Worker boundary, completed review and residual limitations.
- `_bmad-output/implementation-artifacts/epic-16-retro-2026-09-20.md:149-150` — E16-A7 measured-before-backlog precondition.
- `_bmad-output/implementation-artifacts/deferred-work.md:1115-1123` — separate assigned/proposed handoffs.
- `scripts/run_17_3c_delegated.sh` — bounded, named-unit delegation precedent, not a generic 17-2 helper.
- `xtask/src/{check_kernel_baseline.rs,kloc_check.rs}` — physical/hash versus tokei instruments; do not confuse their counts.

## Dev Agent Record

### Agent Model Used

Story context creation: `openai-codex/gpt-6-astra`. Preflight round-table and operator rulings (2026-10-11): `anthropic/claude-opus-5-5`. Implementation agent/model has not yet been recorded because implementation has not begun.

### Debug Log References

Creation evidence: `17-2-evidence/measurements.json` and `17-2-evidence/prototype.diff`. The prototype executable was a throwaway measurement tool; it is not a shipped feature or permanent regression suite.

- Implementation model (2026-10-11): `openai-codex/gpt-6.1-sol`.
- T1 red controls: PID consumer compile failed with missing `pids_max` after clearing stale `maos-manifest` build output (`artifact://14`); Worker early-refusal regression failed because host-grant validation won over malformed resources (`artifact://26`). An initial fixture-wrapper error and a vacuous substring assertion were corrected before these controls were accepted.
- T1 focused green: `cargo test --locked -p maos-manifest` — 194 passed across six suites; `cargo test --locked -p maos-kernel-core --test manifest_field_coverage -- --nocapture` — four passed (`artifact://29`).
- T1 Worker green: `cargo test --locked -p maos-bin --features network,wasm-host --test two_host_delegation_2b malformed_worker_resources_refuse_before_cli_probe_or_binding -- --exact --nocapture` and the live-policy unit regression — one executed test each, both passed (`artifact://36`).
- T1 runtime smoke: actual `target/debug/maos run <temporary-malformed-worker-manifest> --once`, isolated `MAOS_HOME`, environment secret backend, inference selector unset — exit 1 with `[resources] parse: resource cap out of range: memory_max_mb = 0`, before Worker startup/host-grant refusal. Temporary manifest removed.
- Execution substrate: local cgroup2 mount is read-only; `gh auth status` reports no login. SSH push and public GitHub Actions/check-annotation reads support the authorized disposable-branch route. No complete delegated acceptance pass, T3, full-workspace, final pin or final review evidence has been obtained.
- T2 red: the explicit PID regression failed on the old successful-but-uncontained spawn; the revised native path passes that regression (`artifact://43`, repeated after failure-path changes in `artifact://70`).
- T2 local runtime proof (`artifact://63`): real T2 child PID 781653 had membership `0::/`, `RLIMIT_AS=67108864`, `RLIMIT_NOFILE=64`, and the memory-only fallback reason consumed exactly once. Separate no-cap and FD-only T2 children returned no resource downgrade. The temporary smoke source was removed.
- T2 process-tree diagnostic (`artifact://68`): real T2 leader 785368 reported descendant 785371; `/proc` confirmed the parent relation. Only the owned descendant was explicitly terminated, and the temporary diagnostic source/executable was removed.
- Authorized native CI attempts: [38123445915](https://github.com/lunarpulse/maos/actions/runs/38123445915), [38123677660](https://github.com/lunarpulse/maos/actions/runs/38123677660), [38124842432](https://github.com/lunarpulse/maos/actions/runs/38124842432). First attempt failed before tests because systemd expanded shell dollars; `--expand-environment=no` corrected the service envelope. Subsequent oracle fixes use real `/proc` labels, the bridge's newline-stripped record contract, the actual Linux PATH_MAX creation failure, and deadline-held bounded tasks. No attempt is counted as a passing acceptance run or a requested behavioral mutation control.
- Native CI acceptance after authorized correction: [38139403269](https://github.com/lunarpulse/maos/actions/runs/38139403269), commit `661eab6d7fc5bd6a25fa95d32d8e9134e3adde00`. Ubuntu 24.04/26.04 x64/arm each executed four tests: four passed, zero failed, zero ignored. Real pre-exec membership, exact CPU/memory/PID/FD limits, bounded resource exhaustion/throttling, same-Spirit isolation, creation/limit-write/exec failures, normal cleanup and surviving-descendant cleanup passed. Scope is native T2 only, not workspace/T3/pins or final completion.
- **Workflow HALT cleared by operator guidance (2026-10-11):** after three failed delegated CI acceptance attempts, the operator selected “Continue with the known correction,” authorizing the borrowed absolute/relative `Path` comparison, complete four-image retry, and continuation through T3–T6/full gates unchanged. Retry passed as recorded above. All four complete run/job/annotation records and the complete passing 17-file executed diff are archived in `17-2-evidence/native-ci-attempts.json` and `17-2-evidence/native-ci-executed.diff`.
- T3 mapping red/green: `cargo test --locked -p maos-kernel-core --test t3_argv_custody_17_1 -- --nocapture` failed two meaningful regressions before implementation (`artifact://90`: 10% became 10 cores; FD31 became PID31), then passed all five tests (`artifact://93`). CPU conversion uses integer quotient/remainder; PID comes only from `pids_max`; an explicit FD cap produces equal soft/hard nofile limits and omission produces no nofile default.
- T3 real fixture smoke: `prlimit --nofile=64:64 -- /mnt/build/cargo/debug/worker-cli-fixture --maos-worker --maos-fixture-mode=resource-probe` exited successfully in 7.11s, read actual namespace links, opened 59 descriptors before `TooManyOpenFiles:24`, attempted exactly 64 uncontained threads, and emitted the existing completion marker. This proves fixture bounds/FD behavior, not container PID enforcement. New Worker integration target compiles.
- First signed rootless T3 attempt [38141366491](https://github.com/lunarpulse/maos/actions/runs/38141366491), commit `82040175680759b8bd6d1c96b011c06b6c244669`: both x64 images executed canary 1/1 and resource suite one passed/one failed/zero ignored. The above-hard-limit oracle found no nofile-specific error in root completion stderr; its full cause was truncated by the public annotation limit. Actual runtime-client stderr is persisted in `CliSubprocessOutput`, so the updated oracle reads that sink and retains specific refusal/no-guest assertions. Cause not yet established; retry commit `025fbf7ad99425756e65830cea2e0403548bdbed` is running. Evidence archived in `17-2-evidence/t3-ci-attempts.json`.
- Second T3 attempt [38142077624](https://github.com/lunarpulse/maos/actions/runs/38142077624): normal real Worker enforcement passed on both images; above-hard refusal happens during the existing pre-PID startup inspector (`PID not available after 10 attempts`), before bridge stderr persistence. Matched real OCI controls use the opaque verified-image builder and actual detected runtime under inherited hard nofile64: requested64 completes the finite real fixture; requested65 produces limit-specific denial before guest output. Production `maos run`65 also refuses with no guest output/no owned container. No out-of-charter startup-inspection change or 17-7 completion is claimed.
- Signed rootless T3 acceptance [38143184107](https://github.com/lunarpulse/maos/actions/runs/38143184107), commit `ac0a3aca46bf71d0aadb1a11fd6bcc70cce3f2ef`: both Ubuntu24.04/26.04 x64 images passed canary1/1 plus resource2/2, zero failures/ignored. Real running Worker CPU/memory/PID/FD limits, bounded exhaustion/throttling, namespaces, signed image/proxy custody, completed output and auto-removal passed. Above-hard real-runtime matched controls and production Worker refusal passed. The observer's public API quota failed after completion; the rendered public run summary independently exposes Success and both complete non-vacuity annotations, archived in `t3-ci-attempts.json`; full exact 24-file passing delta archived in `t3-ci-executed.diff`.
- T4 writer-ordering pre-fix stress control: `cargo test --locked -p maos-kernel-core --test journal_append_16_5 concurrent_same_spirit_live_index_matches_reopened_persisted_index -- --exact --nocapture` passed once (1passed,1filtered,0ignored,0.10s; `artifact://132`). This is a probabilistic control that did not expose the race on this schedule, not a proven red. Deleted only the chartered `drop(file);`, so the live-index insert remains under the writer mutex. Fixed journal suites subsequently passed 3tests/2suites (`artifact://136`).
- Typed lifecycle consumer red: `cargo test --locked -p maos-kernel-core --test journal_resource_downgrade_replay_17_2 legacy_lifecycle_and_typed_resource_downgrade_replay_from_journal -- --exact --nocapture` failed with missing public payload/event20 (E0432/E0599; `artifact://134`). Added the serde DTO/constructor and additive unit event20. The real untagged legacy/new typed persisted replay test passes with the concurrent-index suite (`artifact://136`).
- Isolated admitted-session control (`resource_downgrade_memory_only_is_persisted_for_each_fast_t2_restart`, real release runner, memory64/FD64) reached a completed turn and actual child PID2945, then failed on an unsupported `cli.subprocess.exit.child_pid` assumption (`Null`; `artifact://139`). This is an oracle defect, not a proven producer red; correlate the existing supervision binding with `SessionReport.child_pid` instead. Producer remains paused until a corrected meaningful red.
- Rejected `/dev/full` append-failure setup before another run: `JournalAdapter::open` reads to EOF, while a bounded `od --read-bytes=8 --format=u1 /dev/full` returned eight NUL bytes. An unbounded device replay risks exhaustion; replace with an isolated exact-test process and a finite regular journal plus process-local file-size fault. The earlier full focused control's result was not delivered across the user continuation; no result or OOM occurrence is claimed.
- Corrected producer behavioral red: isolated `resource_downgrade_memory_only_is_persisted_for_each_fast_t2_restart` now reaches its real lifecycle oracle after the admitted runner completes: persisted downgrade rows0 versus required1 (0passed/1failed/0ignored; `artifact://141`). Authorized the production T2 owner to consume and append the transient outcome after live ownership/observer registration and before Ready publication.
- T4 production composition is integrated: both `SessionDeps` constructors share the existing lifecycle `Arc<JournalAdapter>`. Linux resolved memory/FD caps and T2 tier are captured before moving the sandbox; the owner consumes the one-shot reason after `LiveWorker` ownership and real observer registration, serializes the typed payload, and appends before Ready publication. The existing supervisor test-seam pattern gained an allocation-free default-None after-watch callback and common-root construction to deterministically exercise actual pidfd SIGKILL → Crash/SCB removal → downgrade publication.
- Three complete session-suite verification attempts triggered the workflow retry HALT. Command: `env MAOS_17_2_CGROUP_MODE=undelegated cargo test --locked --offline -p maos-bin --features network,wasm-host --test spawned_scope_17_3c -- --nocapture`. Attempt1 failed compilation on test guard-lifetime E0597 (`artifact://143`), fixed with locals. Attempt2 ran all22: 18passed/4failed/0ignored (`artifact://144`); corrected reused seed identity (real IAC duplicate rows suppress redelivery), required empty class resource table, incidental stop-result pin and isolated file-size fault collateral.
- Attempt3 ran all22: 21passed/1failed/0ignored in13.23s (`artifact://147`). The sole failure is `memory_only_t2_live_child_has_exact_rlimits_and_mode_truthful_journal`: actual live AS64MiB/FD64, Ready/turn and typed-row assertions preceded planned stop; the test then rejects `SessionError::Child(Signaled { signal: 9 })`. Production `spirit_session.rs:508-509` retains nonzero terminal facts after a planned kill; AC83 does not promise a particular stop-return variant. Known correction, not applied after HALT: delete incidental planned-stop return-shape pins, retain bounded termination/planned binding/child reaping and reopened journal assertions, and do not change production teardown or relax caps.
- **HALT pending operator guidance:** three consecutive T4 complete-suite implementation-verification failures. All remaining workflow items are blocked; story/sprint remain `in-progress`, all task checkboxes remain unchecked. The eight complete bounded verification logs and attempt records are archived in `17-2-evidence/t4-session-attempts.json`. These local undelegated-mode results are not named `Delegate=no` CI acceptance; T5/T6, positive T2 session CI, full workspace, mutations, final budget grants/pins/docs and non-author review remain pending.
- FW-08 disposable CI branch cleanup after archiving complete native/T3 evidence: `git push origin --delete ci/17-2-native-proof` succeeded (`[deleted] ci/17-2-native-proof`). No merge or main push occurred. Future authorized CI verification requires a fresh `ci/17-2-*` branch.
- Operator explicitly selected “Continue with the oracle correction.” The T4 retry HALT is cleared. Removed only incidental stop-return pins; real planned-stop bindings, bounded termination, exact child reaping, live caps and reopened lifecycle assertions remain. Authorized attempt4 ran the complete real admitted-session suite: 22passed/0failed/0ignored in13.23s (`artifact://156`, archived in `17-2-evidence/t4-session-attempts.json`). Production terminal handling is unchanged. Local read-only cgroup2 is negative-mode smoke, not named `Delegate=no` or delegated acceptance.
- A fresh FW-08 disposable branch `ci/17-2-session-proof` now snapshots the exact scoped implementation delta at baseline `b19556abbf4937784b2c7662f3a00025289590a9`. Its branch-only acceptance workflow runs all22 session tests on four required images in both asserted system-manager `Delegate=yes` and `Delegate=no` units, with effective negative ancestor-permission probes and non-vacuity checks. YAML parsed and every generated run block passed `bash -n`; CI execution is pending.
- FW-08 session CI launched at commit `1f2249131a57e070289f49c84c13d84c63893711`: [38175786166](https://github.com/lunarpulse/maos/actions/runs/38175786166), four images × two modes. The complete executed diff is archived as `17-2-evidence/t4-ci-executed.diff`; SHA-256 `30ce84d064ac4cb1c52a5146d40ecf3022ba3b137adccfb646edc6482c23441b` matches the direct git diff stream. Observation is bounded and does not infer green from a running or unreadable run.
- Focused non-author T4 static review found an unbounded after-watch fixture barrier. Replaced it with bounded channel receipt; its sender drops before World teardown on unwinding. A temporary copy of the actual admitted-session test injected panic after real pidfd SIGKILL: old barrier needed a controlled emergency release after3s (expected failure,3.91s); fixed gate returned on Disconnected without emergency release (expected failure,1.13s), and actual childPID44732 was reaped (`/proc/44732/stat` absent). Removed the temporary target. The permanent full session suite then passed22/22, zero ignored,13.22s (`artifact://199`; `17-2-evidence/t4-after-watch-fault.json`). Running CI predates this test-only bounded-failure correction; its exact source is preserved separately.
- The producer review withdrew an initial mandatory-pidfd proposal after inspecting the preservation contract: existing per-child EMFILE/ENOMEM and after-pump observation remain supported, with truthful unavailable-observer diagnostics. Registration ordering is preserved; the resource payload makes no observation/OOM claim, and no new observer-success policy was added. These focused static reviews are not the final §A6 review.
- First actual session CI matrix [38175786166](https://github.com/lunarpulse/maos/actions/runs/38175786166) completed with failure: all8 named/probed jobs executed22 tests,21passed/1failed/0ignored. Positive jobs fail `admitted_t2_echo_enforces_exact_live_caps_in_delegated_mode`; negative jobs fail the isolated append-failure test. Full public job/annotation evidence is archived in `17-2-evidence/t4-ci-attempts.json`. Public summaries omit early panic details and authenticated logs are unavailable; no cause is inferred and no acceptance is claimed.
- Diagnostic matrix [38177533756](https://github.com/lunarpulse/maos/actions/runs/38177533756) launched at `de7a1ee6bc402b7e5858484a941040dae8688d29`. It retains all22 tests and exact caps, incorporates bounded watcher cleanup/formatting, reports actual isolated-child status and bounded failed-session return, and emits bounded panic excerpts. No production policy or cap was relaxed. Exact source: `17-2-evidence/t4-ci-diagnostic-executed.diff`, verified SHA-256 `3d1b9a940abfee5c46da37b2fde013575175b01a4e82d881d55c4e57207c9a58`. One complete CI verification failure is recorded; diagnostic result is pending.
- Second CI matrix completed before tests: all8 failed the compilation step. Local reproduction of the disposable checkout showed five E0252 duplicate imports caused by applying the already-integrated baseline-relative delta with three-way patching. Main-worktree source was not duplicated. Replaced all4 owned snapshot files with exact canonical source; its actual full local suite passed22/22,0ignored,46.75s (`artifact://221`). Full compiler/run evidence is appended to `t4-ci-attempts.json`; this attempt supplies no original runtime-panic diagnosis.
- Proved a separate append-fault harness defect using a temporary copy of the actual test and the actual Cargo debug-profile lock: with real fixtures prebuilt and the lock held, the old isolated child re-entered Cargo and was killed by the control's five-second deadline (status9,0passed/1failed/0ignored). Parent-only fixture building removes that unnecessary child dependency; the same held-lock scenario then passed1/1,0ignored in1.33s. The permanent child deadline remains60; no resource or journal assertion changed. Removed the temporary target. Complete current local session suite passed22/22,0ignored,13.13s (`artifact://238`; `t4-append-fixture-lock.json`). Attribution to the first CI negative failure remains `[INFERENCE]` until diagnostic CI.
- Corrected third matrix [38178799449](https://github.com/lunarpulse/maos/actions/runs/38178799449) launched at `930d181df8701850f18e2f0df18f714361e8d2c8`, retaining exact caps and all22 tests per mode. Its complete source is archived as `t4-ci-corrected-executed.diff`, SHA-256 `ce3d21143e177df4184db5964d4a5191d91ca169daa442b05c3c05edecf2cf71`. Canonical source also follows the newly enforced pre-2024 `Future` import convention; the language server offered no applicable import-shortening action. Two complete CI failures are recorded; third result remains pending, and the positive runtime failure still has no inferred cause.
- Third CI result [38178799449](https://github.com/lunarpulse/maos/actions/runs/38178799449): all4 named `Delegate=no` jobs passed22/22,zeroignored. All4 `Delegate=yes` jobs ran21passed/1failed,zeroignored; only the combined live-cap test fails. Actual panic on every positive image: the T2 runner did not remain live through Ready and one completed turn; bounded session cleanup `Ok(Ok(Err(Stopped)))`. The helper's ten-second deadline then explicitly stops the session, while the launch declares thirty seconds per startup/turn stage. Exact-cap completion beyond ten seconds remains unobserved; no later success or resource failure is inferred.
- **HALT pending operator guidance:** three full CI attempts without complete T4 acceptance. All9 remaining workflow items are blocked; story/sprint remain `in-progress` and all task checkboxes remain unchecked. `t4-ci-attempts.json` contains all3 actual runs, exact source references, complete public job/annotation outcomes, local copy-defect reproduction and bounded fault proofs. Proposed next action, not applied after HALT: align the bounded live-turn oracle with the existing launch-stage budget, retaining exact CPU10/memory64/PID12/FD64 and live-limit/cleanup/lifecycle assertions, then rerun the complete matrix. T5/T6 and all final landing gates remain pending.
- FW-08 cleanup completed after archiving all3 session attempts and their exact executed diffs: `git push origin --delete ci/17-2-session-proof` succeeded (`[deleted] ci/17-2-session-proof`). No merge or main push occurred. Any explicitly authorized retry will use a fresh disposable `ci/17-2-*` branch.
- Operator explicitly selected “Continue with bounded oracle fix.” The CI retry HALT is cleared. The three live-turn callers now supply their actual launch stage budget; the helper uses the existing 17-3c hostile-session envelope (4×stage budget+20s), rather than stopping at an incidental ten seconds. Exact caps, real live-limit/turn/lifecycle/cleanup assertions and production thirty-second stage deadlines remain unchanged. Full current local suite passed22/22,zeroignored,14.28s (`artifact://248`).
- Fresh authorized branch `ci/17-2-session-budget` launched [38181820306](https://github.com/lunarpulse/maos/actions/runs/38181820306) at `6cdaf8821b6d320d8a251061b7e76a3dd8158471`; all four images and both modes remain mandatory. Exact source: `t4-ci-budget-executed.diff`, SHA-256 `c8e66d5c844fc62af3ac9ff4a4d1f20415e5e15e11050d4e794aa8b9ac57e77a`. The archive records explicit authorization and local proof; full matrix outcome is pending and no positive completion is inferred.
- Fourth matrix result: all4 negative units and3/4 positive units passed22/22,zeroignored; Ubuntu24.04 x64 positive ran21passed/1failed. Actual terminal error is `Deadline("startup")`: the declared thirty-second production launch budget, not the old fixture cutoff. Complete outcomes and annotations are archived. Contention from simultaneous real runner compilation is `[INFERENCE]`, not an established cause.
- The next authorized experiment isolates libtest execution with `--test-threads=1`; all22 cases, actual caps and production launch budgets remain unchanged. Full canonical local serial suite passed22/22,zeroignored,42.10s (`artifact://263`). Fifth full matrix [38183350059](https://github.com/lunarpulse/maos/actions/runs/38183350059), commit `bc71d9402a908fdd1bb47819b6dfefc94de69165`, is running on the same disposable branch. Exact source `t4-ci-serial-executed.diff`, SHA-256 `be33e676b2468075ac801f811bc2d44690f3220a4708c8ab0196a84a805eda1b`. No complete positive acceptance is claimed.
- Fifth matrix [38183350059](https://github.com/lunarpulse/maos/actions/runs/38183350059) completed successfully: all8 required positive/negative image jobs passed22/22,zeroignored (176 actual session cases). Every job's public annotation reports `exit=0 suites=[('ok', '22', '0', '0')]`, and all jobs actually completed. Complete run/jobs/annotations are archived. Serial integration execution succeeds without changing resource caps or production budgets; it does not establish a general startup-performance guarantee.
- FW-08 cleanup: after archiving exact source and all5 outcomes, `git push origin --delete ci/17-2-session-budget` succeeded. No main push/merge occurred. Proceeding to T5 helper/workflow/mutations. Before helper implementation, six checked-execution boundary regressions failed because the required new helper is absent; failures cover zero/missing/extra suites, failed/ignored tests, nonzero command exit and explicit substrate skip.
- T5 valid behavioral RED (`artifact://277`): executing the actual aggregate status bash with the delegated required job skipped produces `overall=success`, instead of failure. The new regression observes this consumer-visible disposition; an earlier source-enrollment assertion and an authoring E0716 error are not claimed as behavioral RED. The helper's six checked-execution boundary regressions now pass. Its actual `check-log` CLI accepts the complete real22-test log and refuses the same log with command exit101 (exit1).
- Real new-helper lifetime run [38184923504](https://github.com/lunarpulse/maos/actions/runs/38184923504), `e132342a5f0e6a7f0fbef81133d4712bc0945eeb`, passed all4 required images. Each actually started a contained unprivileged command, then exercised wrapper SIGTERM, client SIGTERM, client SIGKILL and unit runtime expiry. All16 scenarios observed a nonzero helper exit, inactive service and reaped owned command. Exact source `t5-lifetime-executed.diff`, SHA-256 `fd834d013fae3ca5fa575535789dfb7f1ad4f894a1a034ef86617eab2c97e681`; complete proofs in `t5-validation.json`.
- Genuine green-before/red-after controls are running in [38184549819](https://github.com/lunarpulse/maos/actions/runs/38184549819), `921bb1e2f8c09cbecc5f2ba2cb8c36a1231162b1`: native missing PID write and suppressed session producer on all4 images; ignored Worker resources, percent-as-core CPU and FD/PID alias on both x64 images. Each must first pass its actual unmutated consumer oracle, then fail the selected assertion-bearing oracle after applying its sole exact production-source mutation. Exact source `t5-mutation-executed.diff`, SHA-256 `ed0873bd62687058bd7f42a5d888c86e68263c50bece802ba8270ccfc14edc91`. Prepared/application-checked patches are not runtime proven-red results; matrix outcome remains pending.
- All14 genuine mutation controls completed successfully in [38184549819](https://github.com/lunarpulse/maos/actions/runs/38184549819). Each passed its real unmutated consumer test, then ran its sole exact mutated source and failed the corresponding selected test with exit101,zeroignored and the expected actual assertion. The public API observer reached quota while collecting annotations after run success; complete primary public HTML recovered all14 proof notices and actual panic excerpts. Verified every notice's patch SHA-256 against the archived exact patch. `t5-validation.json` records complete primary source, parsed proofs and observer limitation.
- Permanent T5 integration is installed: both required four-image jobs, positive native4/session22 plus x64 Worker2/in-unit canary1, and the complete negative session22. The integration owner corrected the initially narrowed negative case, retained live-limit/strict-refusal/order coverage, fetched locked workspace dependencies for offline fixtures, reused feature-enabled bin artifacts, and removed incidental enrollment assertions from the behavioral regression. Actual aggregate status/failure-body regression passes1/1,zeroignored; helper boundary regressions pass6/6. Required-success fan-in/report rows, `always()`, last macOS disposition and unchanged ship-gate lists are preserved.
- Exact permanent-job/workspace run [38185566523](https://github.com/lunarpulse/maos/actions/runs/38185566523), `ecfa5aeb42e7f57eb1a51afa8e4100235f463c38`, is running from the actual parsed permanent job definitions. Its workspace invocation retains exactly `cargo test --workspace --locked --no-fail-fast`, preceded in the same delegated unit by the real rootless canary. Exact source `t5-ci-executed.diff`, SHA-256 `baab6ac84a4a37ec699630bd2ba3a715b73bd808c22c0b927f10e06c0ba164b2`. No completed job/workspace result is inferred. Public API quota failure is an observer limitation, not a CI failure; observation resumes at the actual reset.
- First permanent-job/workspace attempt completed with a real failure. All eight image jobs executed the complete session22 suite with zero ignored cases; all four positive jobs also passed native4. Both x64 Worker steps failed before a canary-success notice, and the workspace step failed before a canary-success notice. Therefore neither Worker2 nor whole-workspace execution is claimed for this attempt. Complete jobs and annotations are archived in `t5-validation.json`.
- Source inspection found two missing declared canary prerequisites: the positive x64 job had not built `worker-cli-fixture`; the workspace job had not fetched the locked feature-enabled dependency set used by its offline canary. These omissions are observed; attribution of the preceding failures is `[INFERENCE]` because private job logs were unavailable. Added the explicit prerequisites and bounded public annotations containing actual captured canary errors/harness stderr, without suppressing failures. Local checked-execution six-case suite and actual aggregate one-case regression pass (`artifact://333`).
- Full second permanent-job/workspace attempt [38187260379](https://github.com/lunarpulse/maos/actions/runs/38187260379), `de2422cbfa49079df06b1a05b0e814c26d8957d2`, is running. Exact source `t5-ci-prerequisites-executed.diff`, SHA-256 `5352aa757dbef8864c0444a804aa79ddc89d96ca5247dae57f9a57c727877820`. All original caps, production deadlines, complete test counts and the workspace command remain unchanged; no outcome is inferred.
- Quality verification found unformatted code on fourteen verified story-owned Rust paths. Formatted only those paths; `cargo fmt --all -- --check` now exits0. Formatted manifest compatibility suite passes194; four targeted kernel field-coverage/journal/argv suites pass12 (`artifact://351`). Existing release-workflow/macOS aggregate disposition regressions pass29 (`artifact://343`).
- Actual formatted `kloc-check --json` (`artifact://353`) reports kernel19495, manifest4466, domain9349, bin25397, xtask44370 and aggregate173504. Domain exceeds9329 by20; bin exceeds25247 by150; aggregate exceeds its strict hard-fail threshold173383. FW-03 requires an operator measured request before changing these three figures. The canonical strict aggregate convention requires measured+1, i.e.173505. No ceiling, baseline or independent pin has been raised; complete actual output is archived in `t5-validation.json`.
- Operator decision on 2026-10-12: **Authorize exact measured increases** — domain9349, bin25397 and `_aggregate_hardfail`173505. This is the measured173504+1 strict threshold, not speculative headroom. Kernel and manifest receive no raise; a later larger measurement requires a new request. Coordinated pins have not yet been changed.
- Second permanent-job attempt completed: all8 positive/negative image jobs succeeded. Every positive image passed native4 and session22; both positive x64 images passed the actual in-unit rootless canary1 and Worker2. Every negative image passed the complete session22. The workspace's actual in-unit rootless canary1 also passed; the original full workspace command then ran and returned101. Complete actual jobs/annotations are archived in `t5-ci-prerequisites-results.json`. Whole-workspace green remains unproven, and its specific failed tests are not inferred from the generic exit code.
- Existing `gh` tooling is unavailable (`command not found`), and private job logs cannot be read through the public API. Running the actual formatted full workspace locally with the existing binary/guest prerequisites to obtain specific consumer-contract failures before another CI attempt. The local host has no delegated cgroupfs/T3 acceptance substrate; its substrate failures will not be claimed as CI evidence or local passes.
- Actual local original workspace execution returned101 with19 failed targets; complete log is archived in `t5-workspace-local-diagnostic.log`. Most require the unavailable local delegated/T3 substrate or stale numerical pins. The uninstall failure actually reports `EndpointInUse` (retained-sidecar test); it is not attributed to CI or silently modified outside this story.
- Applied the authorized coordinated class/surface/baseline/digest/independent-literal/KLOC/RATIFIED/ABI pin cutover in charter order. Surface379→376: three retired resource-ceiling symbols, zero additions, two hashes changed; explicit native/bridge one-shot method witnesses remain. Initial measured kernel25149physical/19495tokei,95files; domain9349/bin25397 and strict aggregate173505 match the measured operator grant.
- All six non-author A6 layers completed against `t6-a6-reviewed.diff` (SHA-256`4c532a3df78d750afcb4789471e701980172aeb107504cf44fb1a2a21acecccb`); raw findings and deduplicated decisions are archived. Real uncapped/FD-only descendant containment regression, missing native above-hard FD acceptance and lost TypeScript report row were patched. Unexercised non-Linux PID equivalence was dismissed against the Linux ACs/eight-location charter and explicit EN/KO limitation; no parity is claimed.
- Actual failing-before descendant proof [38196319487](https://github.com/lunarpulse/maos/actions/runs/38196319487) passed its RED control on all4 required images: selected oracle0passed/1failed/0ignored, exit101, actual surviving-descendant assertion. Exact source `t6-a6-descendant-red-executed.diff`, SHA-256`4f3fcab0ff699b63f51abe811a592a47907af9c48aac43f36750a90ba74d96f9`; complete jobs/annotations retained.
- Restored best-effort owned native placement independently of configured cgroup ceilings, preserving silent/nonfatal no-resource refusal, strict CPU/PID refusal and truthful memory-only fallback. Native cleanup oracle now covers capped, uncapped and FD-only descendants without adding a fifth outer test. Native FD64/65 control re-enters the parent-built test executable below actual soft/hard64, never Cargo or the parent hard limit.
- Original runtime/security/acceptance reviewers found two defects in the added native FD oracle: unsupported Landlock file creation and nested libtest-summary forwarding. Actual T2 smoke proved absent-file PermissionDenied versus existing-file guest soft/hard64; unchanged actual helper rejected two genuine passing suite logs and accepted the sole actual27-case outer log. Parent now prepares/removes the marker and forwards only the named FD notice. No production MakeReg grant or checker weakening; all3 final narrow reviews report no remaining findings. Delegated64/65 runtime acceptance remains pending.
- Post-A6 actual measurement lowered the exact pins again: kernel25146physical/19492tokei,95files; manifest4466/domain9349/bin25397/xtask44370; aggregate173501, strict hardfail173502. This is below the authorized173505, not new headroom. Actual baseline/service-boundary/KLOC/unsafe checks pass; final fmt, public-field, deprecation and breaking-entry checks pass. The empty-kernel gate exposed an existing undocumented T3Launch exemption; its register now documents the existing one-shot value only, with no new annotation/type/runtime state, and the gate passes.
- Actual isolated corrected contracts pass87 xtask cases (seven suites) and3 a2a chaos cases,zeroignored; complete raw log is archived in `t6-corrected-contract-gates.log`. Current corrected native suite compiles (`--no-run`); this is not delegated runtime GREEN. A shared-target build-lock timeout is recorded as an environment limitation, not a passing check.
- EN/KO published resource guidance and architecture/migration pointers are updated (12 published documents,4 architecture documents, plus existing I9 register). Actual official SHA-verified Node20.20.2 (`df770b2a6f130ed8627c9782c988fda9669fa23898329a61a871e32f965e007d`) ran frozen npm-ci, KO100%(41/41), glossary, EN/KO build,90route,13cookbook,39troubleshooting and18ABI-anchor gates successfully. Actual HTTP served four EN/KO manifest/CLI surfaces200. Browser visual proof is unavailable: installed Chromium exits127 for missing libglib-2.0.so.0; no screenshot or new OS dependency is claimed.
- Actual docs-site CI [38196319460](https://github.com/lunarpulse/maos/actions/runs/38196319460) succeeded on the shipped published-document source. Every prior owned-branch run is inventoried with exact commit/command-diff references; four additional actual companion runs (38185566562/38185566461/38187260414/38187260402) have complete successful job dispositions archived, without inventing uncollected individual notices.
- Final exact-source CI commit`38754fd7200733c043aff03e0ba6dc63bace2445` pushed only to owned `ci/17-2-ci-enforcement`. Complete71-file executed source `t6-final-ci-executed.diff`, SHA-256`a87b66cbce32cee1c846102bdccb5adfdfe494f417780dfd1f68215cc315cd34`. Actual runs: acceptance[38200530152](https://github.com/lunarpulse/maos/actions/runs/38200530152), sole mutations[38200530174](https://github.com/lunarpulse/maos/actions/runs/38200530174), lifetime[38200530134](https://github.com/lunarpulse/maos/actions/runs/38200530134). The acceptance copies the five actual permanent jobs, including original full workspace command and applicable SDK macOS all-target/feature checks. Every sole patch actually applies. Bounded observation is running; no final result, task completion or review status is inferred.
- Final [38200530152](https://github.com/lunarpulse/maos/actions/runs/38200530152) completed with failure. All8 required image jobs succeeded: each positive native4/session22, each negative session22, and both x64 Worker2/canary1, zeroignored. Every positive image witnessed actual native64 guest soft/hard64 and native65 EPERM with no refused guest effects or owned residue; capped/uncapped/FD-only descendant cleanup passed. Applicable SDK macOS original workspace/all-targets and network+wasm-host checks both succeeded.
- The same run's actual in-unit rootless workspace canary passed; unchanged `cargo test --workspace --locked --no-fail-fast` returned101. The bounded public tail names exactly three failed targets: `maos-bin --test worker_supervision_16_3`, `maos-eval --test t3_network_escape_executor`, and `maos-kernel-core --test resource_caps_linux`. Individual failing assertions are not exposed by that tail; no cause is inferred. Complete jobs/steps/annotations are archived in `t6-final-ci-acceptance-results.json`.
- Current-source controls also completed: [38200530174](https://github.com/lunarpulse/maos/actions/runs/38200530174) has14 actual green-before/red-after notices, each baseline0/mutant101/zeroignored/correct assertion and exact sole-patch SHA; [38200530134](https://github.com/lunarpulse/maos/actions/runs/38200530134) has16 actual interruption/expiry scenarios with non-root UID, nonzero helper exit, inactive unit and reaped child. Complete primary HTML/notices and validated invariants are archived in `t6-final-ci-control-results.json` and `t6-final-runtime-summary.json`.
- **HALT pending operator guidance:** this is the third complete permanent-job/whole-workspace verification failure. No subsequent source fix or CI retry is authorized by this execution. Story/sprint stay `in-progress`, task checkboxes remain unchecked, and review readiness is not claimed. Proposed next action, not executed: capture the specific failed assertions for the three actual targets and apply only evidenced, charter-compliant corrections, preserving full workspace selection, caps, production deadlines and all existing guarantees.
- FW-08 cleanup completed only after full final outcomes and exact command/source archives: `git push origin --delete ci/17-2-ci-enforcement` succeeded (`[deleted] ci/17-2-ci-enforcement`); the owned local disposable worktree was removed without force. Owned marker/log-boundary/diagnostic/observer throwaways were removed after their actual proofs were archived. `t6-halt-cleanup.json` records the commands and results. No main push or merge occurred. Any operator-authorized continuation uses a fresh disposable branch.
- Operator selected **Continue focused diagnosis and fixes**, clearing the mandatory retry HALT. Fresh disposable `ci/17-2-workspace-diagnostic` captures actual assertions for the three named targets before production changes; full workspace selection, original caps/deadlines, exact numerical grant and kernel charter remain unchanged. No new source fix or passing workspace result is claimed.
- Exact diagnostic commit`5f4757bc73def045c2e97025cb17f61a67984cdf` contains only a branch-only bounded actual-failure reporter/workflow beyond the failed source. Complete73-file source is archived in `t5-workspace-diagnostic-executed.diff`, SHA-256`350353e46866dfde8719f3c4d81a66fdc8e1357d56010096e0925cd07648957c`; exact commands in `t5-workspace-diagnostic-source.json`. Bash syntax passes and the reporter extracts genuine failure sections from the previously captured local log, without claiming those local substrate failures as current CI attribution. Fresh branch push succeeded; actual run outcome remains pending.
- Actual diagnostic [38210028700](https://github.com/lunarpulse/maos/actions/runs/38210028700) and separately triggered unchanged docs-site [38210028619](https://github.com/lunarpulse/maos/actions/runs/38210028619) bind to exact commit`5f4757bc73def045c2e97025cb17f61a67984cdf`. Primary discovery and the finite source-bound observer are archived. Diagnostic production source remains identical to failed run38200530152; specific current assertions and final dispositions remain unobserved.
- Diagnostic [38210028700](https://github.com/lunarpulse/maos/actions/runs/38210028700) completed: original full workspace exit101, only two failed targets. Actual PID-cap panic compares absolute`0::/system.slice/...` with relative`system.slice/...`; both T3 executor cases panic because their declared static probe is missing. Worker supervision actually passes23/23,zeroignored on unchanged source; the earlier Worker failure's specific cause remains unknown. Complete current assertions/jobs/steps/notices are archived in `t5-workspace-diagnostic-results.json`.
- Applied only the two evidenced verification corrections: normalize the `/proc` membership separator via`0::/` in the existing exact owned-leaf assertion; add the existing locked/static probe build to the workspace's existing fixture prerequisites. No production `src` file, cap, deadline, runtime privilege, test selection, resource policy, unsafe, dependency or kernel location changed.
- Actual current formatting check and existing PID-cap test pass1/1 locally; this is not claimed as delegated membership evidence on the local unavailable substrate. The exact static build succeeds, ELF64 has no dynamic interpreter, and the real executable's safe usage-refusal smoke exits1 with its expected usage message. Optional `file` tooling was unavailable after the successful build; native ELF inspection replaced that observation, not the build or test. Full local outputs are archived.
- Fresh complete acceptance [38212495038](https://github.com/lunarpulse/maos/actions/runs/38212495038) and unchanged companion docs [38212495002](https://github.com/lunarpulse/maos/actions/runs/38212495002) bind to exact commit`2589e5627e74092c7b2aa27491a29cb17cf61ede` on owned`ci/17-2-workspace-fixed`. Complete73-file source `t5-workspace-fixed-executed.diff`, SHA-256`851571719f9b129a7c51d7f2e3ec286e2b562f340d1a54db47faecba45dd57fb`. Actual five permanent jobs retain full four-image positive/negative suites, original whole-workspace command/canary, and both SDK macOS checks. Results remain pending, not inferred.
- Diagnostic companion docs[38210028619](https://github.com/lunarpulse/maos/actions/runs/38210028619) and its actual job succeeded. After both diagnostic-run outcomes and complete command/source records were archived, remote`ci/17-2-workspace-diagnostic` deletion and unforced owned-worktree removal both succeeded. No main push/merge; exact lifecycle record is archived.
- Final formatted production measurement remains kernel25146physical/19492tokei/95files, manifest4466/domain9349/bin25397/xtask44370, aggregate173501 under strict173502. Actual kernel baseline/service-boundary/KLOC checks pass. An attempted`check-unsafe-budget` is not a repository command and was not counted as passed; actual`check-unsafe` and empty-kernel checks are running separately. Complete original output, including the unsupported-command failure, is retained.
- Correctly invoked final`check-unsafe` and recovered actual`check-empty-kernel` both pass with0 violations. Both unsupported command attempts remain archived as failures, not passes. Full original unwrapped quality/constitution outputs replace the earlier rendered snapshots; final actual empty-kernel output is archived separately. No code was changed to suppress a gate.
- Final accepted [38212495038](https://github.com/lunarpulse/maos/actions/runs/38212495038), exact`2589e5627e74092c7b2aa27491a29cb17cf61ede`, succeeds on all11 actual jobs. Original`cargo test --workspace --locked --no-fail-fast` exits0 after the real in-unit rootless canary; formerly named targets pass Worker23/T3-network2/native-PID1,zeroignored. Every required positive/negative image passes session22; every positive image passes native4 and actual FD64/65 proof; both x64 images pass Worker2/canary1. Actual SDK original full-workspace/all-target and network+wasm-host checks both succeed. Complete jobs/steps/annotations and numerical validation are archived. Existing whole-workspace ignored tests are retained; no global zero-ignored claim.
- Final companion docs[38212495002](https://github.com/lunarpulse/maos/actions/runs/38212495002) and its actual job succeed on the same source. Unchanged production/helper source retains the already exercised14 green-before/red-after mutations,16 interruption/expiry/reap scenarios and completed six non-author review layers. Only the two evidenced verification corrections followed those production proofs; no new production review finding is represented as solved without evidence.
- FW-08 closure succeeds after full evidence archival: final owned remote branch deleted, unmodified disposable worktree removed without force, and both completed observers/their temporary JSON outputs removed. Exact lifecycle record is archived. Neither temporary push workflow lands in the canonical tree; no main push/merge or source edit follows the accepted regression run.
- Enhanced installed dev-story DoD closure: all32 task/subtask boxes, all3 ACs and delivery gates verified;60 canonical new/modified/deleted source paths and every permanent evidence artifact are listed; protected story context/model/requirements remain unchanged, and baseline is preserved. Final accepted original full regression executes after all code changes; subsequent changes are tracking only. Story and sprint are set to`review`, not`done`; the final26-item checklist result is archived in`story-definition-of-done.json`. No active HALT, unresolved in-scope finding or incomplete deliverable remains.
- Final configured tracking commands`cargo run --locked -p xtask -- check-dev-record-completeness --json` and`cargo run --locked -p xtask -- check-epic-close-coherence --json` both pass with0 violations after review-status updates (167 done stories/21 epics checked; authoritative kernel25146). Their full actual output is retained in the enhanced DoD artifact. The remaining obsolete owned first-attempt observer/result were archived by provenance and removed; normal build caches and dependencies are preserved.

### Completion Notes List

- Ultimate context engine analysis completed - comprehensive developer guide created.
- Creation corrected stale root-layout wiring, delegation recipe, lifecycle variant shape, schema provenance and CPU/PID/FD custody premises.
- A real isolated T2 fallback/PID-refusal smoke and exact deletion-inclusive kernel measurements were obtained; delegated/T3 runtime proof remains implementation work on capable runners.
- Story validation must retain all three source ACs, the ratified FW-01…FW-08 rulings, previous-story preservation constraints and the 17-7 handoff boundary. No feature completion, CI green or cross-platform runtime result is claimed.
- Preflight round-table 2026-10-11: RT-1…RT-10 filed. The operator ratified the kernel charter (including the journal-index fix), *no raise; exact at landing*, and the new story 17-7 for the T3 lifecycle handoffs.
- Readiness re-validation 2026-10-11: V-1…V-13 filed. The one BLOCKER (CI-execution authorization) was ruled as FW-08; every other finding is applied in place.
- T1 implementation is integrated: optional independent PID caps, live shared operator-resource authority on topology/standalone/Host-B Worker routes, early malformed-resource refusal, schema-5 coverage and additive reader-contract ratification. Focused checks and CLI refusal smoke pass; all story tasks remain unchecked pending their full acceptance gates.
- T2 source cutover and strict native acceptance are integrated: native PID discovery/enable/write, strict CPU/PID refusal, per-successful-spawn memory-only outcome, real cgroup creation error propagation, bounded cleanup diagnostics, and obsolete module/export retirement. The real T2 bridge/std-only task/process suite passed all 16 assertions-bearing tests across the required four images without ignore/skip. Remaining story gates, including T3–T6 and whole-workspace contracts, are not complete.
- T3 source and real Worker acceptance are integrated: exact percentage-to-core conversion, independent PID/FD flags and preserved omitted-field backend defaults; real signed rootless Worker enforcement and above-hard FD refusal passed Ubuntu24.04/26.04 x64. Permanent delegated/undelegated fan-in, whole-workspace acceptance, mutations, final measurements/pins/docs and non-author review remain open.
- T4 source and complete session acceptance are integrated: all8 asserted delegated/undelegated jobs on Ubuntu24.04/26.04 x64/ARM passed22/22,zeroignored. Canonical full serial local suite also passes22/22. Typed persisted fallback/rlimits, restart/crash ordering, strict refusal and isolated append-failure cleanup are exercised. Exact caps and production deadlines remain unchanged. T5/T6 and final whole-workspace/landing/review gates remain open; story is not ready for review.
- T5 permanent required jobs and honest fan-in/report disposition are integrated; original14 mutation controls and16 helper lifetime scenarios are runtime-proven. Both complete prior image matrices are archived; the second actual workspace execution returned101. Final current-source whole-workspace/Linux/macOS acceptance is still gating completion.
- T6 coordinated exact pins, published EN/KO docs, architecture/migration closure and all six non-author reviews are integrated. Actual local corrected90 contract cases and constitutional/docs gates pass; real four-image failing-before descendant proof and source-level findings resolution are archived. Kernel stays within its eight-location/no-new-unsafe charter, and 17-7 handoffs remain untouched. All story checkboxes remain unchecked until the final runtime/DoD gates pass.
- Final exact-current-source native/session/Worker matrices, native above-hard FD/descendant controls, both SDK macOS compile checks,14 sole mutations and16 helper lifetime scenarios pass. Mandatory whole-workspace acceptance still fails three targets with exit101; the third-attempt retry HALT is active. Implementation is not complete or ready for review; operator guidance is required before further diagnosis/fixes/retries.
- The operator-authorized continuation is active; the prior retry HALT has been cleared. Fresh source-exact full-workspace diagnostics are running before cause-based corrections. Existing passing resource/security/custody guarantees, exact numerical authority and 17-7 boundary remain intact; no whole-workspace pass or review readiness is claimed.
- Focused diagnosis now proves the exact membership-path assertion defect and missing existing static-probe prerequisite; both are corrected without production behavior changes. Worker supervision is observed23/23 on unchanged source, not attributed to an invented cause. Fresh complete runtime/workspace/macOS acceptance is gating final completion; story remains in-progress.
- T1–T6 implementation and all related acceptance gates are now verified: independent PID/resource-policy contracts; native pre-exec containment/refusal/FD/descendant cleanup; truthful signed/rootless T3 flags; typed persisted native downgrade and journal-index ordering; honest required CI/delegation/negative controls; exact coordinated pins, EN/KO documentation and six non-author review layers. The original full regression suite and all required current image/SDK jobs pass. All32 task/subtask checkboxes are checked only now, after actual complete GREEN. Final enhanced DoD/status closure follows; 17-7 remains untouched.
- **Complete and ready for review (2026-10-12):** independent manifest/operator PID caps reach native and signed/rootless Worker paths; native containment/cleanup and FD refusal are exercised; actual memory-only downgrade is typed/persisted before readiness; required positive/negative CI and original whole workspace pass. Exact lower kernel/pins stay inside the measured authorization; EN/KO docs and all six non-author layers are closed. Enhanced DoD passes26/26, all32 story boxes are complete, and story/sprint status is`review`. Review/landing remains the operator next step; no main merge/push was authorized or performed.

### File List

Story-creation and preflight deliverables only:

- `_bmad-output/implementation-artifacts/17-2-worker-cgroups-applied.md`
- `_bmad-output/implementation-artifacts/17-2-flag-winston-decision.md`: operator-ratified charter and numerical rule (2026-10-11).
- `_bmad-output/implementation-artifacts/17-2-evidence/prototype.diff`
- `_bmad-output/implementation-artifacts/17-2-evidence/measurements.json`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`: 17-2 row and the new 17-7 `backlog` row.
- `_bmad-output/implementation-artifacts/deferred-work.md`: the owners of `:1119-1120` → 17-7.
- `_bmad-output/planning-artifacts/epics/epic-17-workers-and-third-party-form-w2.md`: creation corrections, the 17-7 section and §R11.
- `_bmad-output/planning-artifacts/epics/index.md`: the Epic-17 entry now counts 10 stories and lists 17.7.

Production source was unchanged by creation/preflight; implementation deliverables below are complete and verified:

- `crates/maos-manifest/src/manifest.rs`
- `crates/maos-manifest/tests/resources_pid_17_2.rs` (new)
- `crates/maos-kernel-core/src/scheduler/scheduler_loop.rs`
- `crates/maos-kernel-core/src/scheduler/mod.rs`
- `crates/maos-kernel-core/src/scheduler/resource_ceiling.rs` (deleted)
- `crates/maos-kernel-core/src/security/sandbox/linux.rs`
- `crates/maos-kernel-core/src/security/sandbox/mod.rs`
- `crates/maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs`
- `crates/maos-kernel-core/tests/resource_caps_linux.rs`
- `crates/maos-kernel-core/tests/sandbox_enforcement_windows.rs`
- `crates/maos-kernel-core/tests/cgroup_ceiling_smoke.rs`
- `crates/maos-kernel-core/tests/manifest_field_coverage.rs`
- `crates/maos-kernel-core/tests/fixtures/manifest/resources/well-formed/pids_max.toml` (new)
- `crates/maos-kernel-core/tests/fixtures/manifest/resources/malformed-rejected/pids_max.toml` (new)
- `crates/maos-kernel-core/tests/fixtures/manifest/resources/edge-case/pids_max.toml` (new)
- `crates/maos-kernel-core/tests/fixtures/native/cgroup_ceiling.rs` (new)
- `crates/maos-bin/src/worker_spawn.rs`
- `crates/maos-bin/src/main.rs`
- `crates/maos-bin/src/delegation.rs`
- `crates/maos-bin/tests/two_host_delegation_2b.rs`
- `crates/maos-bin/tests/enterprise_daemon_seam_13_5a.rs`
- `xtask/abi-ratifications.toml`
- `_bmad-output/implementation-artifacts/17-2-evidence/native-ci-attempts.json` (new)
- `crates/maos-kernel-core/src/security/sandbox/t3/argv.rs`
- `crates/maos-kernel-core/tests/t3_argv_custody_17_1.rs`
- `crates/maos-bin/tests/worker_resource_limits_17_2.rs` (new)
- `spirits/worker/src/bin/worker-cli-fixture.rs`
- `_bmad-output/implementation-artifacts/17-2-evidence/native-ci-executed.diff` (new)
- `_bmad-output/implementation-artifacts/17-2-evidence/t3-ci-attempts.json` (new)
- `_bmad-output/implementation-artifacts/17-2-evidence/t3-ci-executed.diff` (new)
- `crates/maos-domain/src/invariants/i10.rs`
- `crates/maos-kernel-core/src/journal/mod.rs`
- `crates/maos-kernel-core/tests/journal_append_16_5.rs`
- `crates/maos-kernel-core/tests/journal_resource_downgrade_replay_17_2.rs` (new)
- `crates/maos-bin/src/spirit_session.rs`
- `crates/maos-bin/tests/spawned_scope_17_3c.rs`
- `crates/maos-bin/src/supervision.rs`
- `_bmad-output/implementation-artifacts/17-2-evidence/t4-session-attempts.json` (new)
- `_bmad-output/implementation-artifacts/17-2-evidence/t4-ci-executed.diff` (new)
- `_bmad-output/implementation-artifacts/17-2-evidence/t4-after-watch-fault.json` (new)
- `_bmad-output/implementation-artifacts/17-2-evidence/t4-ci-attempts.json` (new)
- `_bmad-output/implementation-artifacts/17-2-evidence/t4-ci-diagnostic-executed.diff` (new)
- `_bmad-output/implementation-artifacts/17-2-evidence/t4-append-fixture-lock.json` (new)
- `_bmad-output/implementation-artifacts/17-2-evidence/t4-ci-corrected-executed.diff` (new)
- `_bmad-output/implementation-artifacts/17-2-evidence/t4-ci-budget-executed.diff` (new)
- `_bmad-output/implementation-artifacts/17-2-evidence/t4-ci-serial-executed.diff` (new)
- `scripts/tests/test_cgroup_ceiling_17_2.py` (new)
- `scripts/run_cgroup_ceiling_17_2.py` (new)
- `xtask/tests/cgroup_ceiling_workflow_17_2.rs` (new)
- `_bmad-output/implementation-artifacts/17-2-evidence/t5-validation.json` (new)
- `_bmad-output/implementation-artifacts/17-2-evidence/t5-mutation-executed.diff` (new)
- `_bmad-output/implementation-artifacts/17-2-evidence/t5-lifetime-executed.diff` (new)
- `_bmad-output/implementation-artifacts/17-2-evidence/t5-ci-prerequisites-executed.diff` (new)
- `.github/workflows/discipline.yml`
- `_bmad-output/implementation-artifacts/17-2-evidence/t5-ci-executed.diff` (new)
- `_bmad-output/implementation-artifacts/17-2-evidence/t5-ci-prerequisites-results.json` (new)
- `xtask/kernel-api-classes.toml`
- `docs/ci-baselines/kernel-surface-v0.1-beta.json`
- `xtask/kernel-core-baseline.toml`
- `xtask/tests/kernel_pin_content_hash_16_0.rs`
- `xtask/kloc.toml`
- `xtask/tests/recovery_lane_ceiling_rule.rs`
- `docs-site/docs/manifest/v5.md`
- `docs-site/i18n/ko/docusaurus-plugin-content-docs/current/manifest/v5.md`
- `docs-site/docs/cookbook/manifest-fields.md`
- `docs-site/i18n/ko/docusaurus-plugin-content-docs/current/cookbook/manifest-fields.md`
- `docs-site/docs/cookbook/cli-wrapper-spirit.md`
- `docs-site/i18n/ko/docusaurus-plugin-content-docs/current/cookbook/cli-wrapper-spirit.md`
- `docs-site/docs/cookbook/hello-world-spirit.md`
- `docs-site/i18n/ko/docusaurus-plugin-content-docs/current/cookbook/hello-world-spirit.md`
- `docs-site/docs/cookbook/wasm-component-spirit.md`
- `docs-site/i18n/ko/docusaurus-plugin-content-docs/current/cookbook/wasm-component-spirit.md`
- `docs-site/docs/deploy/topology.md`
- `docs-site/i18n/ko/docusaurus-plugin-content-docs/current/deploy/topology.md`
- `BREAKING.md`
- `docs/red-team/README.md`
- `docs/invariants/i9-exemptions.md`
- `_bmad-output/implementation-artifacts/17-2-evidence/t6-ci-prior-companion-results.json`
- `_bmad-output/implementation-artifacts/17-2-evidence/t6-corrected-contract-gates.log`
- `_bmad-output/implementation-artifacts/17-2-evidence/t6-a6-triage.json`
- `_bmad-output/implementation-artifacts/17-2-evidence/t6-final-ci-observer.py`
- `_bmad-output/implementation-artifacts/17-2-evidence/t6-final-ci-run-discovery.json`
- `_bmad-output/implementation-artifacts/17-2-evidence/t6-final-ci-executed.diff`
- `_bmad-output/implementation-artifacts/17-2-evidence/t6-a6-final-correction-reviews.json`
- `_bmad-output/implementation-artifacts/17-2-evidence/t6-final-mutation-applicability.json`
- `_bmad-output/implementation-artifacts/17-2-evidence/t6-final-quality-gates.log`
- `_bmad-output/implementation-artifacts/17-2-evidence/t6-a6-oracle-final.diff`
- `_bmad-output/implementation-artifacts/17-2-evidence/t6-a6-fd-oracle-corrections.json`
- `_bmad-output/implementation-artifacts/17-2-evidence/t6-a6-correction-review-1.json`
- `_bmad-output/implementation-artifacts/17-2-evidence/t6-ci-branch-run-inventory.json`
- `_bmad-output/implementation-artifacts/17-2-evidence/t6-a6-corrections.diff`
- `_bmad-output/implementation-artifacts/17-2-evidence/t6-a6-descendant-red-executed.diff`
- `_bmad-output/implementation-artifacts/17-2-evidence/t6-a6-review-layers.json`
- `_bmad-output/implementation-artifacts/17-2-evidence/t6-a6-descendant-red-results.json`
- `_bmad-output/implementation-artifacts/17-2-evidence/t6-docs-gates.log`
- `_bmad-output/implementation-artifacts/17-2-evidence/t6-grant-gates.log`
- `_bmad-output/implementation-artifacts/17-2-evidence/t6-docs-http-smoke.json`
- `_bmad-output/implementation-artifacts/17-2-evidence/t6-a6-reviewed.diff`
- `_bmad-output/implementation-artifacts/17-2-evidence/t5-workspace-local-diagnostic.log`
- `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md`
- `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/5-spirit-abi.md`
- `_bmad-output/implementation-artifacts/17-2-evidence/t6-final-ci-acceptance-results.json`
- `_bmad-output/implementation-artifacts/17-2-evidence/t6-final-ci-control-results.json`
- `_bmad-output/implementation-artifacts/17-2-evidence/t6-final-runtime-summary.json`
- `_bmad-output/implementation-artifacts/17-2-evidence/t6-halt-cleanup.json`
- `_bmad-output/implementation-artifacts/17-2-evidence/t5-workspace-diagnostic-extractor-smoke.json`
- `_bmad-output/implementation-artifacts/17-2-evidence/t5-workspace-diagnostic-executed.diff`
- `_bmad-output/implementation-artifacts/17-2-evidence/t5-workspace-diagnostic-source.json`
- `_bmad-output/implementation-artifacts/17-2-evidence/t5-workspace-diagnostic-run-discovery.json`
- `_bmad-output/implementation-artifacts/17-2-evidence/t5-workspace-diagnostic-observer.py`
- `_bmad-output/implementation-artifacts/17-2-evidence/t5-workspace-diagnostic-results.json`
- `_bmad-output/implementation-artifacts/17-2-evidence/t5-workspace-diagnostic-docs-results.json`
- `_bmad-output/implementation-artifacts/17-2-evidence/t5-workspace-diagnostic-cleanup.json`
- `_bmad-output/implementation-artifacts/17-2-evidence/t5-workspace-fixed-local-gates.log`
- `_bmad-output/implementation-artifacts/17-2-evidence/t5-workspace-fixed-executed.diff`
- `_bmad-output/implementation-artifacts/17-2-evidence/t5-workspace-fixed-source.json`
- `_bmad-output/implementation-artifacts/17-2-evidence/t5-workspace-fixed-probe-smoke.json`
- `_bmad-output/implementation-artifacts/17-2-evidence/t5-workspace-fixed-run-discovery.json`
- `_bmad-output/implementation-artifacts/17-2-evidence/t5-workspace-fixed-observer.py`
- `_bmad-output/implementation-artifacts/17-2-evidence/t5-workspace-fixed-quality-initial.log`
- `_bmad-output/implementation-artifacts/17-2-evidence/t5-workspace-fixed-constitution.log`
- `_bmad-output/implementation-artifacts/17-2-evidence/t5-workspace-fixed-empty-kernel.log`
- `_bmad-output/implementation-artifacts/17-2-evidence/t5-workspace-fixed-results.json`
- `_bmad-output/implementation-artifacts/17-2-evidence/t5-workspace-fixed-docs-results.json`
- `_bmad-output/implementation-artifacts/17-2-evidence/t6-final-accepted-runtime-summary.json`
- `_bmad-output/implementation-artifacts/17-2-evidence/t6-final-accepted-cleanup.json`
- `_bmad-output/implementation-artifacts/17-2-evidence/story-definition-of-done.json`

## Change Log

- 2026-10-11: Captured baseline, marked story/sprint in-progress, integrated T1 resource-authority and PID parser contracts with focused red/green evidence, and began T2 obsolete-mechanism retirement. Story is not ready for review.
- 2026-10-11: Integrated T2 native containment/outcome/error/cleanup changes and real-child smoke; archived three four-image native CI attempts. Latest result is two passed/two failed per image due to an identified oracle path-representation mismatch. Retry HALT reached; story remains in-progress, with all task checkboxes intentionally unchecked and T3–T6 pending.
- 2026-10-11: Operator authorized continuation after the retry HALT. Applied the identified absolute/relative path-oracle correction and relaunched the complete native image matrix; story remains in-progress pending observed acceptance results and T3–T6.
- 2026-10-11: Authorized native retry passed all four required images (4/4 tests each, zero ignored). T3 mapping red control exposed 10% as 10 cores and FD31 as PID31; corrected integer decimal-core/PID/FD argv mapping passes 5/5 custody/resource unit tests. Real signed Worker acceptance is in progress.
- 2026-10-11: Real signed rootless T3 acceptance passed both required x64 images (canary1/resource2 each, zero ignored). Began T4 typed lifecycle/session producer and concurrent journal-index ordering work; pre-fix stress control honestly recorded as green.
- 2026-10-11: Integrated the real T2 typed downgrade producer and deterministic observer/crash-ordering coverage. Full session suite improved from18/22 to21/22; the third complete verification attempt triggered mandatory retry HALT. Pending operator guidance on the remaining planned-stop return oracle; no production terminal facts were suppressed and no completion/review status is claimed.
- 2026-10-11: Operator authorized the T4 oracle-only correction. Full local admitted-session verification passed22/22 with no ignored tests; production terminal facts were preserved. Prepared a fresh four-image/two-mode disposable session CI proof. Permanent CI integration, whole-workspace gates, mutations, final measurements/pins/docs and non-author review remain open.
- 2026-10-11: Launched the actual four-image/two-mode T4 acceptance matrix and archived its exact executed source. Resolved the focused review's unbounded crash-ordering fixture failure path with real before/after fault cleanup proof; the full current session suite passes22/22. Preserved supported unavailable-observer fallback rather than adding a new policy gate; final §A6 remains pending.
- 2026-10-11: First named T4 CI matrix executed22 tests per job but failed one per mode (21/22 on all8 jobs). Archived public evidence and exact source; launched bounded diagnostic capture without changing production semantics or relaxing resource caps. Story/sprint remain in-progress; all task checkboxes remain unchecked pending complete acceptance.
- 2026-10-11: Corrected an owned disposable snapshot's duplicate-import copy defect and proved the actual isolated append-fault path must reuse parent-built fixtures rather than contend on Cargo locks. Full current local session suite passes22/22; launched the canonical third four-image/two-mode acceptance matrix without cap or production-policy relaxation. All completion gates remain open.
- 2026-10-11: Third T4 CI attempt passed all4 named negative units22/22 but positive units remain21/22. Captured actual stopped-by-fixture diagnostics and preserved exact resource caps. Reached mandatory retry HALT; archived all3 attempts and blocked remaining workflow work pending explicit operator guidance. No story/task completion or review status is claimed.
- 2026-10-11: Operator authorized the bounded budget-aligned T4 oracle correction and complete retry. Reused existing stage-envelope convention with unchanged production deadlines/resource caps; full local suite passes22/22. Launched a fresh authorized four-image/two-mode matrix; T5/T6 and final gates remain pending.
- 2026-10-11: Fourth T4 matrix passed7/8 jobs, with a real startup deadline on Ubuntu24.04 x64 positive. Preserved exact caps and production deadlines; launched the complete serial-integration matrix after all22 canonical local cases passed. Full matrix outcome remains pending.
- 2026-10-11: Full authorized T4 matrix completed8/8 jobs,176actual session cases,zeroignored, with unchanged exact resource caps/production budgets. Archived all5 attempts and removed the disposable branch. Began T5 with failing checked-execution boundary regressions; permanent CI/landing gates remain open.
- 2026-10-11: T5 checked-execution parser/CLI smoke passes; actual skipped required-job aggregate false-success reproduced before workflow cutover. New helper passed16 actual interruption/expiry/reap scenarios on all4 images. Launched14 isolated genuine resource/producer mutation controls; permanent jobs/fan-in, whole-workspace and remaining gates are pending.
- 2026-10-11: All14 exact resource/producer mutations proved green-before/red-after on every required runner. Permanent T5 jobs/fan-in/workspace envelope integrated; actual skip/failure/cancel aggregate regression and six helper boundaries pass. Complete permanent-job/whole-workspace CI is running; no final acceptance/review status is claimed.
- 2026-10-12: Applied the exact measured operator grant and coordinated lower pins; updated EN/KO/architecture migration guidance and passed frozen docs/constitutional/local-contract gates. Six non-author A6 layers exposed real descendant containment and native-oracle/report defects; four-image failing-before proof and all source corrections are archived. Final exact permanent-job/workspace/macOS and sole-mutation/lifetime CI is running; story/sprint remain in-progress and no task/review completion is claimed.
- 2026-10-12: Final current-source CI passed all8 Linux image jobs, actual native64/65 and descendant cleanup, both SDK macOS compile checks,14 genuine mutation controls and16 helper interruption/expiry/reap scenarios. The mandatory original workspace command still returned101 with three named failed targets. Archived actual source/results; reached mandatory third-attempt retry HALT. Story/sprint remain in-progress, all task checkboxes unchecked, and no further source fix or retry is performed pending operator guidance.
- 2026-10-12: Completed FW-08 remote-branch deletion and unforced owned-worktree/throwaway cleanup after archiving all actual final outcomes and exact commands/source. Recorded mandatory retry HALT and blocked remaining gates; no main push/merge, source correction or further CI retry occurred after the HALT.
- 2026-10-12: Operator cleared the retry HALT for focused diagnosis/fixes. Created and pushed a fresh owned branch with bounded actual three-target failure excerpts and unchanged original complete workspace execution. Archived exact source/commands and actual recorded-log reporter smoke; no production correction or new acceptance result is inferred.
- 2026-10-12: Captured actual full-workspace failing assertions, corrected only the membership path normalization and missing already-declared static network-probe prerequisite, and observed unchanged Worker supervision23/23. Archived complete diagnostic/companion results and deleted its disposable branch. Current original full workspace/four-image/macOS acceptance is running on a fresh exact-source branch; no final completion/review result is claimed.
- 2026-10-12: Original full workspace now exits0 on exact accepted source; all11 current matrix/workspace/macOS jobs and companion docs succeed. Confirmed formerly failed targets23/2/1, native4 and session22 on all required images, x64 Worker2/canaries, exact native FD/refusal/cleanup, and retained14 mutations/16 lifetime controls. Checked all32 task/subtask boxes only after actual full regression GREEN; completed owned branch/worktree/observer cleanup. Enhanced DoD/status closure follows.
- 2026-10-12: Enhanced dev-story definition of done passes26/26 after actual original full regression and all current required runtime/SDK/docs jobs pass. Verified all32 completed task/subtask boxes, exact complete file list, unchanged protected story sections/baseline, measured authorization and untouched17-7 handoff. Set story/sprint to`review`; archived final DoD and all source/results/cleanup evidence. No main push or merge.
