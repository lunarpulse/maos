---
story_key: later-registry-quarantine-closure
created: 2026-11-21
status: in-progress
context_commit: de16fa0af0f3644f6a3fa21610ab24ac10d4ee3f
baseline_commit: de16fa0a
security_sensitive: true
kernel_grant: "Forecast only; no prototype or grant requested. Kernel +36…+116 tokei with the recommended D-row options (worst listed option +151; method and per-option deltas below). ADR-069 kfree applies inside the envelope: measure at landing with check-kernel-baseline / kloc-check --json, re-pin the kernel exact pin with scripts/repin.py and write its HISTORY row in the same commit. No forecast reaches the 22,000 alarm."
kloc_grant: "Read-the-files forecast, not measured growth or permission: maos-kernel-core +36…+116; maos-bin +140…+360; maos-iac +20…+45; maos-domain +6…+16; maos-capability +2…+5; maos-control +12…+35; maos-cli +15…+45; maos-egress 0…+15; maos-manifest +20…+60 (D-LRQC-04 A) or 0 (B/C); maos-audit 0…+10. Re-measure after the D-rows are ruled."
review: "DIFFICULT: kernel growth, a security fence (FR13), durability and concurrency (fence/admission ordering). Follow docs/testing/proof-contract.md and D-E20-6. Story creation anthropic/claude-opus-5-5 (DraftQuarantineClosure). No independent review or execution claimed."
---

# later-registry-quarantine-closure — FR13 quarantine gaps

Status: in-progress

**Existing key, newly created story.** This is a GA precondition (`ga3`, `decisions-2026-11-12-dedrift.md:28`). SCP 2026-11-13 §2 kept it whole (`sprint-change-proposal-2026-11-13.md:57`): "Narrowing it leaves I5 at `asserted` and the ledger at 13 of 14." The operator approved D-LRQC-01…10 on 2026-11-21 UTC. The record-only approval left the story in `backlog`; implementation has since started, and the requirement remains open. Authority for the scope: `planning-artifacts/epics/epic-21-multi-host-on-live-substrate-w6.md:314-331`; acceleration ruling `q20` (`planning-artifacts/decisions-2026-11-12-acceleration.md:41`); retro action E20-A9 (`implementation-artifacts/sprint-status.yaml:780-781`, `epic-20-retro-2026-11-12.md:214`); the tracker row `sprint-status.yaml:503`; and the ten owner rows in `deferred-work.md` (`:1268`, `:1579-1587`). Paths under `planning-artifacts/` and `implementation-artifacts/` are relative to `_bmad-output/`.

## Story

As **an operator who quarantines a Spirit through a CRL or a registry yank**,
I want **the quarantine to stop the Spirit's own effects (memory and scalar writes, IAC sends, egress, event-driven hooks), to survive restart and unload/reload, to block cold upgrade and hot swap until explicit journaled release, to reach a T3 Worker through a CRL, and to read back separately from a director pause**,
so that **FR13's "quarantine" revocation policy does what it says on every path, without claiming process or container isolation**.

**Claim boundary that must survive the landing.** The verbatim claim is at `20-1b-…-evidence/closure/closure-note.md:7`. This story narrows that claim only where it adds witnessed fences. It never turns the claim into isolation: a quarantined Spirit's process or container keeps running (`worker_quarantine_20_1b.rs::t3_worker_quarantine_refuses_next_mediated_call_and_records_activity_ac11_5`, `container_activity_continues: true`), and ruling `q` ("not isolation") stays in force. The release text names every fenced surface and every retained limitation.

**Out of scope:** process or container isolation, network-namespace or cgroup confinement, memory reads (retained limitation, below), the CRL signature-role migration (`deferred-work.md:1568`), vetter-key lifecycle rows (`:1566-1574`), and the `q20`-dispositioned W/D/F rows (`:17-25`, `:321`). This story does not describe exploit construction or attack replication. All security criteria are defensive refusal and durability properties with typed outcomes.

## Inventory at HEAD — source measurements, not execution

All observations below were read at `de16fa0a` in `/home/lunarpulse/dev_ws-course28`. Every cited source file is unmodified in the working tree. The one exception, `maos-bin/src/admission.rs`, has uncommitted edits by another lane, so it was read at HEAD through `git show`. No Cargo, build, test, gate, formatter, prototype or remote dispatch ran. A failure inferred from source is marked `[INFERENCE]` and is not red-before evidence.

| Surface | State at HEAD | Preserve / required change |
|---|---|---|
| Fence primitive | `CapabilityRegistryAdapter` holds `paused` (`capability/mod.rs:137`) and `quarantined` (`:140`) pid sets. `is_paused` = quarantine OR pause (`:366-373`). `set_quarantined`/`is_quarantined` are at `:376-391`. The fence is enforced only at `issue_with_mediation` (`:207`), `verify_and_audit` (`:333`) and `recheck` (`:359`). | Keep KQ1: sticky, a director resume does not lift it, and `unload` ends it (`scheduler_loop.rs:561`). New fences consult the same state. Do not add a second fence registry with its own semantics. |
| CRL Quarantine arm | `revocation/applier.rs:176-193` sets the fence and journals one `spirit.quarantined` frame, with no drain and no terminate. A class-less SCB is skipped at `:99-101`. `_iac` is held but unused (`:32,61`). | Keep the one typed receipt and the never-terminate behavior. Under KQ2 the sender fence is set here as well (priced diff `20-1-…-evidence/creation/kq2-sticky-quarantine-plus-iac-egress.diff:69`). |
| CRL durability | `rules.rs:76-92` `rejects_manifest_locked` refuses admission of a class-matching manifest. The poller re-reads `latest.signed.json` every tick (`maos-domain/src/revocation.rs:501-506`; cadence `poller.rs:82-100`). | Keep this: after a restart, a CRL-matched Spirit is **refused at load**, which is stronger than loaded-and-fenced. Under D-LRQC-01 this stays the CRL behavior. |
| Yank quarantine | `yank_policy.rs:90-104` fences `Some(pid)` and journals a receipt. The observation key is `(pid, version, cause)` (`:140`). `reevaluate_at` considers only `Running` SCBs (`:190`) that have an installed package and a matching class version (`:193-204`). The cursor persists (`maos-registry/src/yank.rs:284`). | This is the F-1 defect (`deferred-work.md:1580`): the effect is one-shot, keyed by a dead pid, and a paused Spirit is skipped. D-LRQC-01/02 choose the durable key and store. |
| Unload | `scheduler_loop.rs:558-561` revokes tokens and clears both pause and quarantine, then fires `on_unload` (`:567`) after the fence is cleared. | `unload` stays the operator's terminal action for the current pid, not a release of the hold. A durable record (AC2) must survive an unload followed by a reload. |
| Memory writes | `MemoryManagerAdapter::write` (`memory/mod.rs:699-757`) is the single store chokepoint: region check, principal-write authorization (`:727-737`), then the tier match. The adapter already holds `capabilities: Option<Arc<CapabilityRegistryAdapter>>` (`:107`, setter `:178-184`, wired at `maos-bin/src/main.rs:2616`). Writes that originate in a Spirit: `SpiritMemoryView::write` (`for_spirit.rs:31-40`), `PidBoundPrincipalPort::write` (`maos-bin/src/butler_root.rs:84-101`), and in-process `KernelCtx::memory()` (`scheduler/kernel_ctx.rs:122`). One write originates in the kernel/operator: `halt/resolver.rs:173-181` writes the operator's halt context into the Spirit's private tier. | D-LRQC-06 decides where the fence sits. The operator's halt-context write and the erasure/forget paths (`mod.rs:448-617`) must not be refused by a quarantine. |
| Scalar writes | `CapabilityRegistryAdapter::set_scalar` (`capability/mod.rs:294-309`) and `WorkingMemoryOrchestrator::process_scalar_write` (`capability/working_memory/orchestrator.rs:68,115`) call the store directly. `SetScalarError` (`maos-capability/src/working_memory/mod.rs:68-77`) has no fence variant. | Refuse with a typed `SetScalarError` variant at both kernel entry points. A refused write publishes no `scalar.tap` event (`:305-307`). |
| Native IAC send (KQ2) | `IacBusAdapter::deliver_typed(frame, spirit_pid, capability_token)` (`maos-iac/src/adapter.rs:428-439`) has no sender fence. `IacBusError` (`maos-domain/src/iac_bus_types.rs:15-40`) has no quarantine variant. Priced diff: `fenced_senders` plus `IacBusError::SenderQuarantined` (`kq2-…diff:233-307`, maos-iac +24, maos-domain +2). | Refuse **before** the log append: a refused frame is neither logged nor delivered (I2 log-before-deliver is unchanged for frames that are allowed). This one seam is both AC1's "native IAC sends" and AC8 (KQ2). |
| Distillation | `DistillationPort::write_distillate(spirit_pid, …)` (`maos-domain/src/ports/distillation.rs`). The only implementation is `DistillateWriter` (`maos-iac/src/adapter/distillate.rs:340`). It has no fence. | Covered by AC1 because `deferred-work.md:1581` names it. Refusals are typed `DistillationError`. |
| IO / egress | `IoSubsystemPort` takes no pid (`maos-domain/src/ports/io_subsystem.rs:15-30`). Its Spirit-reachable callers are MCP and providers, which are already token-mediated (`recheck`). A T3 Worker reaches the network through the egress proxy's `BearerTable` (`maos-egress/src/bearer.rs:64,98,126`), which does not consult the fence. | AC1 "IO sends" = Worker egress through `BearerTable::verify`/`revoke` plus the existing token-mediated MCP/inference fence. Do not add a pid to `IoSubsystemPort`. |
| Hooks / telemetry | `HookDispatcher` (`scheduler/hook_dispatch.rs:53-68`) holds `capability: Option<…>`. Hook entry points are at `:271-348`. `hook_allowed` checks only the manifest (`:174-184`). `HookOutcome` (`:25-41`) has no quarantine outcome. `SkippedManifest` is matched in `hook_pump.rs` (3), `hot_swap/coordinator.rs` (2), `hook_dispatch.rs` (2), `idle_watchdog.rs` (1) and `scheduler_loop.rs` (1). Telemetry is delivered via `hook_pump.rs:1478` → `fire_on_telemetry_event` (scope constant `:74`). | D-LRQC-07 decides which hooks are refused. A refused hook must not count as a crash, silent failure or strike. Otherwise supervision unloads the Spirit and sheds the fence (`scheduler_loop.rs:561`). |
| Release / readback | `set_quarantined(pid, false)` has exactly one production caller, `unload`. The door reports `fenced = capability.is_paused(pid)` only for pause/resume (`maos-bin/src/operator_door.rs:1052-1055`). Door verbs take a `spirit_id` (`maos-control/src/lib.rs:303-307`; dispatch `operator_door.rs:777-800`; verb labels `:2933-2950`; CLI `maos-cli/src/subcommands.rs:2176-2179`). | `fenced` is read by `worker_freeze_19_2.rs:458`. Keep its meaning and add distinct booleans (AC6). |
| Worker bind | `worker_spawn.rs:919-923` builds `SpiritManifestBundle { on_crash, supervision, ..Default::default() }`: no `class`, no `on_revocation`. The Worker `spirit_id` is a sequence (`worker`, `worker-2`, …; `supervision.rs:2299-2306`), not an identity. A Worker manifest declares `[cli_wrapper]`, which is mutually exclusive with `[class]` (`spirits/worker/manifest.toml:1-5`; `EManifestSchemaConflict`, `admission.rs:425-428` at HEAD). The schema window is `1..=1` (`manifest.toml:8`; checked at `worker_spawn.rs:615`). `CliWrapperConfig` has no name or version field (`maos-manifest/src/manifest.rs:4600-4626`). The default revocation action is `TerminateImmediately` (`maos-domain/src/revocation.rs:326-334`; SCB `control_block.rs:332-336`). | AC4 needs a CRL-addressable identity **and** a declared action. D-LRQC-04/05 decide both. The class-less skip in the kernel (`applier.rs:99-101`) stays under the recommended options. |
| Poller F-4 | `poller.rs:43-63` prints only the first failure; the recovery line carries a bare count; telemetry gets one `ErrorKind::App` bucket. `poll_once` propagates every `apply_crl` error (`:70-77`), including `AlreadyApplied` (`applier.rs:79-83`). `[INFERENCE]` With an unchanged `latest.signed.json`, every tick after the first apply is counted as a failure. Not executed. | Keep the one-line-per-persistent-fault contract (`one_daemon_one_door_16_1.rs:698-716`). D-LRQC-09 decides the form. |
| I5 ledger | `docs/invariants/I5.md:4` `claim: asserted`; `:5` `owner: 20-1b-vetting-yank-policy-and-quarantine`, a key that is `done`; `:11` `FR13: asserted, owner 20-1b-…`; `README.md:33` repeats it. ADR-037 (`docs/adr/ADR-037-constitutional-amendment-process.md:11`) requires two reviewers plus an invariant-test diff and a corpus delta. | D-LRQC-10 decides who edits which I5 line and when. |

Unqualified kernel file names are under `crates/maos-kernel-core/src/`.

## Decisions — settled authority and operator approvals

### Already settled; do not ask again

| Decision | Binding disposition |
|---|---|
| `q` / `q20` (acceleration `:41`) | FR13 is a capability-mediated fence, not isolation. 20-1b is closed and its gaps live here. Nothing in this story re-opens the 20-1b record. |
| `ga3` (DD:28), SCP 2026-11-13 §2 (`:57`) | GA precondition, kept whole. Nothing is narrowed. |
| `kfree` (DD:33; ADR-069 amendment `:74`) | Kernel growth inside the envelope is self-service: no FLAG-Winston row and no prototype. This supersedes the "own FLAG-Winston row and creation-time prototype measurement" premise in `deferred-work.md:1585` and the tracker row `:503` (cite corrections below). |
| D-20-1-02 A (KQ1 chosen over KQ2, `deferred-work.md:1587`) | KQ2 was priced and not selected for 20-1. It is in this story's scope by the tracker row and E20-A9, so AC8 takes it, without re-ruling D-20-1-02 for 20-1. |
| Epic AC5/AC9 | I5's FR13 half feeds the [21-10 invariant ledger draft](21-10-invariant-ledger-proofs.md). The tracker still has one `21-10` key and its landing split is undecided. I5 flips only when both halves have proof. D-LRQC-10 A sets who edits which lines and when, not whether both halves are needed. |

### D-rows — operator approved 2026-11-21 UTC (record only)

| ID | Material choice and bounded options | Recommendation and evidence | State |
|---|---|---|---|
| **D-LRQC-01** | AC2 durable quarantine mechanism. **A** A daemon-owned durable quarantine store (atomic temp + fsync + rename + directory fsync), written **before** the fence takes effect and before the receipt. An active record is re-applied on permitted admission, restart and reload before the SCB reaches `Running`. D-LRQC-08 A refuses cold upgrade and hot swap while the hold is active. Only the release verb (AC3) removes an entry. `maos purge` enumerates the file (precedent `env_contract.rs:653`). **B** Route yank quarantine through a durable local CRL, as auto-revoke does (`yank_policy.rs:80-89`). After a restart, `rules.rs:76-92` then **refuses the load** instead of fencing a loaded Spirit, and the running effect needs the manifest to declare `[on_revocation] action = "quarantine"`. **C** Re-key yank observation by identity and replay the yank journal at boot so the policy re-applies. This depends on the registry being reachable, so an air-gapped boot leaves the Spirit unfenced. | **A.** B changes FR59's "quarantine" response into a refusal at load and depends on a manifest declaration a yanked artifact may lack. C fails open offline. A also keeps the CRL path unchanged (it is already durable by refusal, `deferred-work.md:1580`). Kernel 0 under A: admission and reload are bin-side, and `set_quarantined` is public. Contingent +0…+6 if a kernel sink must report applier-originated quarantines. | **Approved A** |
| **D-LRQC-02** | Durable identity key. **A** `(spirit_id, admitted version)`: every admission of that name@version is fenced, whatever the artifact. The publisher and signed-artifact digest, where present, go in the journal row. **B** `(spirit_id, version, artifact digest)`: only the exact artifact is fenced. **C** `spirit_id`, all versions: an upgrade cannot remediate until release. | **A.** It matches yank-row granularity (`local_yank_crl` `={version}`, `yank_policy.rs:509-514`) and is a strict superset of B. Registry immutability (20-1b AC14, `spirit_install_20_1.rs:307`) makes artifact identity redundant for installed packages. B lets a locally re-signed same-name@version artifact through. C also fences admission of a different version while the identity remains held; under D-LRQC-08 A, upgrade of the quarantined Spirit itself requires explicit release regardless of the identity key. Disclose the F-5 "name+version, not publisher/artifact" association as retained for local manifests. | **Approved A** |
| **D-LRQC-03** | AC3 release verb. **A** `maosctl quarantine-release <spirit_id>` (door convention: verbs take a `spirit_id`, `maos-control/src/lib.rs:307`). It resolves the current pid. It is refused, typed, while an applied CRL still matches the Spirit (the operator must forget the CRL first, `applier.rs:290-292`). It journals the completion row `operator.quarantine-release.<op>:<outcome>`. It then clears the capability, sender and bearer fences and the durable record, in that order, after the journal row. **B** Like A, but release overrides a matching CRL (the journal row names the CRL IDs). **C** Release by pid (wording of `deferred-work.md:1582`). | **A.** A CRL is the signed authority. Letting a door verb override it silently would split FR13 from the CRL's own durability (`rules.rs:76-92`). Refusing keeps one reversal order: forget the CRL, then release. C breaks the door's spirit-id convention and would release a stale pid. | **Approved A** |
| **D-LRQC-04** | AC4 CRL-addressable Worker identity. **A** Version-governed Worker identity: the `[cli_wrapper]` schema window goes 1→2 with a required `class_name` + semver `class_version` (still not `[class]`, so `EManifestSchemaConflict` holds). The bind puts a `ClassSection` in the SCB bundle, and the kernel CRL match is unchanged. **B** No schema change: the bind synthesizes a class from fields that are already validated (name `worker:<command>`, version `output_shape_version`). This identity is command-wide, so every Worker wrapping the same CLI matches. **C** A kernel CRL entry kind for class-less SCBs, keyed by the attested image digest. This changes `RevocationEntry` and the CRL signature preimage, which crosses `deferred-work.md:1568`. | **A.** The signed manifest names its own revocable identity, and the 20-5 schema-window discipline already exists (`worker_spawn.rs:615`). B's identity cannot tell two Workers apart, so a CRL aimed at one revokes all of them. C reopens the CRL signature contract and costs kernel +15…+35, maos-domain +20…+50. A costs maos-manifest +20…+60 plus migration of the Worker manifests and templates (5 under `spirits/worker/`). | **Approved A** |
| **D-LRQC-05** | Worker revocation action. **A** Parse a Worker's `[on_revocation]` with the existing parser (`admission.rs:867-868` pattern) into the bundle. The default stays `TerminateImmediately`. **B** A Worker always quarantines on a CRL match. **C** An undeclared Worker defaults to `Quarantine`. | **A.** It keeps the one shared default (`revocation.rs:330-331`) and the one parser. B and C give Workers a revocation semantics different from every other form. The AC4 witness declares `quarantine`. | **Approved A** |
| **D-LRQC-06** | Memory-write fence placement. **A** Spirit-facing surfaces only (`SpiritMemoryView`, `PidBoundPrincipalPort`, a fenced view behind `KernelCtx::memory`). **B** The `MemoryManagerAdapter::write` chokepoint (`mod.rs:699`) refuses a quarantined pid. Kernel- and operator-originated writes (`halt/resolver.rs:173-181`) use an explicit kernel-origin entry that is documented and class-registered. **C** The chokepoint refuses everything, including the operator's halt context. | **B.** One chokepoint cannot be missed by a future Spirit-facing path, whereas A depends on an enumeration (`KernelCtx::memory` returns the whole adapter, `kernel_ctx.rs:122`). C blocks an operator halt resolution behind the quarantine. B touches the same function as 21-6's quota check: coordinate the order (fence before charge). | **Approved B** |
| **D-LRQC-07** | Hook fence scope. **A** Refuse the event-driven hooks (`on_frame`, `on_telemetry_event`, `on_schedule`, `on_idle`, `on_consolidate`) with a typed `HookOutcome` that is never a strike, crash or silent failure. Operator lifecycle hooks (load/start/pause/resume/unload/snapshot/swap) still fire for permitted operations; D-LRQC-08 A refuses swap until release. **B** Refuse every hook except `on_unload`. **C** Refuse every hook. | **A.** Permitted operator verbs stay usable on a quarantined Spirit, and `on_unload` already fires after the fence is cleared (`scheduler_loop.rs:561,567`). B and C break pause/resume and swap handling. The new outcome must reach the 9 `SkippedManifest` match sites. | **Approved A** |
| **D-LRQC-08** | Swap of a quarantined Spirit. **A** The door gives a typed refusal for cold upgrade and hot swap of a quarantined Spirit until explicit release. **B** The fence carries to the successor (cold: new pid gets fenced; hot: pid kept). **C** Allowed, and the swap lifts the fence. | **A.** It is fail-closed and gives one explicit reversal path (release, then upgrade, both journaled). B is implicit and puts a fence on new code the operator chose. C is the F-1 shed. Kernel 0 if enforced at the door, +3…+8 if in `hot_swap/coordinator.rs`. | **Approved A** |
| **D-LRQC-09** | AC7 poller error-change reporting. **A** Print when the error's typed class (fetch / trust anchor / parse / apply) or its text changes, keep counting repeats, and have the recovery line name the last error. An `AlreadyApplied` for an unchanged CRL counts as a success, not a failure. **B** Only include the last error in the recovery line (the alternative in `deferred-work.md:1583`). **C** A, without reclassifying `AlreadyApplied`. | **A.** It closes F-4 as written ("an error transition … is invisible") and keeps one line per persistent fault (`one_daemon_one_door_16_1.rs:698-716`). `[INFERENCE]` C keeps counting steady-state ticks as failures (see the inventory row). B still hides transitions while a fault lasts. Kernel +6…+18 (A) or +3…+8 (B). | **Approved A** |
| **D-LRQC-10** | I5 edit ownership and sequencing (epic AC5/AC9). **A** After AC1–AC8 are proven, this story flips I5 `requirement_claims.FR13` and README `:33` to `proven by later-registry-quarantine-closure` under ADR-037 (two reviewers plus an invariant-test diff and a corpus delta). It also corrects the stale `owner:` (`I5.md:5`, `:11`). The invariant's `claim:` stays `asserted` until 21-10 proves its half and flips the whole entry. **B** This story only hands evidence to 21-10, which makes every I5 edit. **C** This story stays open until 21-10's half lands, then flips FR13 and I5 together. | **A.** The FR13 line is this key's requirement, and I5 is 21-10's invariant. Splitting them avoids a GA-path deadlock between two GA stories. B leaves the ledger naming a `done` owner. C makes this key wait on a story that does not exist yet. | **Approved A** |

No kernel permission, surface-hash permission or STOP exception is requested. A forecast above 22,000 would need an **ENVELOPE ALARM** D-row, and one above 23,000 a new operator decision (ADR-069). Neither is forecast.

## Acceptance Criteria

The numbering follows the epic (`:320-328`). AC10–AC12 come from the owned ledger rows. Every refusal is a named, typed outcome with a deterministic witness. No criterion asks for attack replication.

1. **AC1 — effects fenced (DW:1581, F-5 DW:1584).** For a quarantined pid, each of the following is refused with a named typed outcome, and each has its own witness:
   - private and shared memory writes (D-LRQC-06), including the principal port;
   - scalar mutation at `set_scalar` and both orchestrator entries, with no `scalar.tap` publish;
   - native IAC sends (the AC8 seam);
   - distillate writes (`DistillationError`);
   - T3 Worker egress through `BearerTable`;
   - event-driven hooks and telemetry delivery (D-LRQC-07).

   Kernel- and operator-originated writes, forget/erasure, `heartbeat` and supervision bookkeeping are **not** refused, and a refusal never becomes a crash, strike or silent-failure signal. The existing MCP/inference/memory-by-token/IAC-by-token fence (`quarantine_fr13_20_1.rs`) stays green.
2. **AC2 — durable (DW:1580).** Under D-LRQC-01/02, prove that quarantine survives daemon restart, unload and reload (including boot reload), and crash disposition and reload. On any permitted admission, restart or reload of a held identity, reapply the active record before the SCB reaches `Running`, with a reapplication receipt. Under approved D-LRQC-08 A, prove that the door returns a typed refusal for both cold upgrade and hot swap while the hold remains active. Explicit release must precede a permitted swap; a PR must not perform a forbidden swap as proof, and a swap does not lift the hold. A yank-originated quarantine also applies to a Spirit that was Paused or Loaded when the yank was observed (today `Running` only, `yank_policy.rs:190`). The durable write precedes the effect. A failed durable write fails closed: the fence still applies in memory and the failure is journaled. The CRL path keeps refusing at load (`rules.rs:76-92`).
3. **AC3 — release verb (DW:1582).** A door-only, journaled release verb exists under D-LRQC-03. It is unavailable to Spirits and to non-door callers. It clears every fence AC1 added plus the durable record. Release of an unknown or unquarantined Spirit is refused, typed. Release does not lift a director pause.
4. **AC4 — CRL reaches a Worker (DW:1579; 20-1b AC11.5).** With D-LRQC-04/05 applied, a signed CRL entry naming the Worker's identity quarantines a **running** T3 Worker through the CRL itself. `crl_matched_count: 1`; the receipt is journaled; the next MCP and inference issue and the held-token verify are refused; the Worker's egress is refused (AC1). The proof uses the built binary. The AC11.5 test drops the direct `set_quarantined` route and requires the CRL to fence (trigger of DW:1579). `container_activity_continues` is still recorded, not denied.
5. **AC5 — I5 handoff.** I5's FR13 half is handed to `21-10` (or `21-10b`) as named, tree-bound evidence: test names, run, job and SHA. The ledger entry follows D-LRQC-10.
6. **AC6 — distinct readback (DW:1582).** The door reads back separate `quarantined` and `director_paused` booleans. `fenced` keeps its union meaning for its existing consumer (`worker_freeze_19_2.rs:458`). Both values are read from state, never assumed.
7. **AC7 — F-4 poller (DW:1583).** Under D-LRQC-09, a change in the poller's error state is reported on stderr and in telemetry, and steady-state behavior produces no spurious failure lines. The one-line-per-persistent-fault test stays green.
8. **AC8 — KQ2 IAC egress fence (DW:1587).** `deliver_typed` refuses a quarantined sender with `IacBusError::SenderQuarantined` **before** the log append: no TL row is written and nothing is delivered. The CRL arm, yank policy, re-application at load and release all set or clear the sender fence together with the capability fence. A director pause does not set the sender fence, so pause semantics are unchanged.
9. **AC9 — ADR-037 FR13 flip (DW:1586).** The flip happens only after AC1–AC8 are proven on the final tree, under ADR-037 (two reviewers plus an invariant-test diff and a corpus delta), and in the form D-LRQC-10 rules. The flipped text keeps "not isolation".
10. **AC10 — honest claim text.** The closure-note claim (`closure-note.md:7`), the tracker row's claim text (handed to its owner), `crates/maos-registry/SECURITY.md:173-174`, ADR-056 `:114-117`, `env_contract.rs:16` and the release-notes handoff each state exactly which surfaces are fenced. They also state the retained limits: memory reads; process/container execution continues; pid wrap-around; the publisher/artifact association for local manifests (D-LRQC-02).
11. **AC11 — ledger.** All ten owned rows are dispositioned by stable title (table below). A row closes only with named, tree-bound evidence. Any retained remainder keeps its reason and trigger.
12. **AC12 — kernel accounting at landing.** `check-kernel-baseline --json` and `kloc-check --json` are measured on the landing tree; `scripts/repin.py` re-pins the exact pin; the HISTORY row (driver, files, before→after) and any caused ADR-066 surface refresh land in the same commit. The landing does not cite this story's forecast as a measurement.

## Tasks / Subtasks

- [ ] **T0 — rulings.** Apply the approved D-LRQC-01…10 selections as binding scope. All approved selections match the recommendations. Apply D-E20-6 for security review eligibility (difficult tier).
- [ ] **T1 — kernel fences (AC1, AC8 kernel side).** Implement the D-LRQC-06 memory fence and the kernel-origin entry used by `halt/resolver.rs`. Add the fence variants to `MemoryError` and `SetScalarError` and shift the serde allowlist (`xtask/serde-error-allowlist.toml:418-424`) in the same change. Fence the scalar entry points. Add the D-LRQC-07 hook outcome and migrate the 9 `SkippedManifest` sites so supervision never counts it. Make the `applier.rs` Quarantine arm and `scheduler_loop.rs` unload set and clear the sender fence. Register each new public symbol in `xtask/kernel-api-classes.toml` before the surface pin.
- [ ] **T2 — IAC, distillation, egress (AC1, AC8).** Port the KQ2 sender fence into `maos-iac` `deliver_typed` with refusal before the log, using the priced diff as reference, not by blind application. Share the fence with `DistillateWriter`. Fence `BearerTable::verify` for a quarantined Worker pid, or revoke on fence and re-mint only on release.
- [ ] **T3 — durability (AC2).** Implement D-LRQC-01/02 in `maos-bin`: durable store, re-application of the active record at permitted admission, restart and reload before `Running`, a re-keyed yank observation map, and Paused/Loaded coverage. At the door, implement the D-LRQC-08 A typed refusal for cold upgrade and hot swap until explicit release. Prove persistence across daemon restart, unload/reload (including boot reload), and crash disposition/reload, and prove refusal without performing a forbidden swap. Make `maos purge` enumerate the file. Preserve the CRL refuse-at-load behavior.
- [ ] **T4 — release and readback (AC3, AC6).** Add the `OperatorCommand` variant, door handler, verb label (`operator_door.rs:2933-2950`), maosctl verb, journal row and FR4 classifier site if needed. Add the distinct readback fields while keeping `fenced`.
- [ ] **T5 — Worker CRL route (AC4).** Implement D-LRQC-04 A / D-LRQC-05 A in `worker_spawn.rs:919-923`, including schema/ABI ratification and migration of the Worker manifests and templates. Rewrite the AC11.5 witnesses so the CRL fences the Worker.
- [ ] **T6 — poller (AC7).** Implement D-LRQC-09 in `revocation/poller.rs` and keep `ac12_20_1b_a_failing_revocation_poll_is_printed_once` green.
- [ ] **T7 — claims, ledger, I5 (AC5, AC9–AC11).** Update the claim documents. Disposition the ten rows. Hand the I5 evidence to 21-10 and edit I5 only as D-LRQC-10 rules. Only the orchestrator edits the tracker, epic and `deferred-work.md`.
- [ ] **T8 — landing, orchestrator-owned (AC12).** Run focused proof per `docs/testing/proof-contract.md`, the hosted T3 leg for AC4 (the live-container leg is hosted-only, `closure-note.md:51`), independent difficult-tier review and eligible security review. Then measure, re-pin and record HISTORY in the same commit. Subagents run no mid-flight builds or gates.

## Owned ledger — ten rows, re-anchored by title

`DW` = `implementation-artifacts/deferred-work.md` at `de16fa0a`.

| DW (old → HEAD) | Stable title | Disposition / trigger |
|---|---|---|
| ~1267 → 1268 | FR13 `quarantine` umbrella (KQ1 fence; `@commit` stamp Main-owned) | Closes when AC1–AC9 close. Main still owns the stamp. |
| 1578 → 1579 | A CRL cannot quarantine a Worker | AC4. |
| 1579 → 1580 | Per-pid, in-memory, shed; yank never re-applied (F-1) | AC2 + D-LRQC-01/02/08. |
| 1580 → 1581 | Memory/scalar/DRR/IAC/hooks/telemetry/IO unfenced (GPT P1) | AC1, AC8. Memory **reads** and `scan` are retained limitations, with the trigger "the next Spirit-facing read-path change" (not in the epic AC1 list). |
| 1581 → 1582 | No release verb; `fenced` conflates (F-3) | AC3, AC6. |
| 1582 → 1583 | Poller dedup hides changing errors (F-4) | AC7. |
| 1583 → 1584 | The fence mediates tokens only (F-5) | AC1 narrows it. AC2 resolves the absent-pid/crash stale entries and the Running-only gap. Pid wrap-around and the local-manifest artifact association stay disclosed (AC10). |
| 1584 → 1585 | Priced follow-up "quarantine closure" | Superseded by `kfree` (no FLAG-Winston row or prototype). Closes with the AC12 HISTORY row. |
| 1585 → 1586 | I5 FR13 not flipped (ADR-037) | AC9 / D-LRQC-10. |
| 1586 → 1587 | KQ2 egress fence priced, not selected | AC8. |

Old numbers follow `planning-artifacts/epic-21-preflight-2026-11-12.md:102` ("`:1268, :1578-1586`"). Use the titles, not line numbers.

## Proof plan — existing anchors and named new targets, no execution claimed

### Existing anchors to preserve (must stay green; they are not evidence for the new boundaries)

| Boundary | Anchor |
|---|---|
| KQ1 kernel fence | `crates/maos-kernel-core/tests/quarantine_fr13_20_1.rs`: `crl_quarantine_journals_one_typed_receipt_and_no_termination`, `crl_quarantine_refuses_every_mediated_action_and_reaches_no_effect`, `crl_quarantine_keeps_the_spirit_loaded_and_running_past_the_drain_deadline`, `crl_quarantine_survives_director_resume_and_ends_only_at_unload`; `on_revocation_three_actions.rs`; `revocation_applier_pipeline.rs` (`AlreadyApplied` `:238,277`). |
| Yank policy | `crates/maos-bin/tests/yank_policy_20_1b.rs`: `signed_yank_decisions_warn_request_quarantine_and_apply_crl_ac8`, `quarantine_policy_fences_only_the_signed_yank_target_and_stays_sticky_ac8`, `failing_auto_revoke_retries_policy_without_repeating_observation_p12`, `poller_runtime_clock_and_cursor_restart_reach_expiry_p6`. |
| Worker fence | `crates/maos-bin/tests/worker_quarantine_20_1b.rs`: `a_bound_worker_scb_is_fenced_and_its_next_mediated_calls_are_refused_ac11_5`, `t3_worker_quarantine_refuses_next_mediated_call_and_records_activity_ac11_5`. Both are **rewritten** for AC4, not deleted. |
| Pause readback | `crates/maos-bin/tests/worker_freeze_19_2.rs`: `twenty_door_pauses_freeze_a_chatty_worker_each_within_two_seconds` (`fenced` at `:458`). |
| Poller print | `crates/maos-bin/tests/one_daemon_one_door_16_1.rs::ac12_20_1b_a_failing_revocation_poll_is_printed_once`. |

### New targets (names fixed here; their bodies follow the rulings)

- `crates/maos-kernel-core/tests/quarantine_closure_lrqc.rs`:
  - `quarantined_pid_memory_write_is_refused_typed_and_kernel_origin_write_is_not` (AC1, D-06)
  - `quarantined_pid_scalar_write_is_refused_and_publishes_no_tap` (AC1)
  - `quarantined_pid_event_hooks_are_refused_without_a_strike` (AC1, D-07)
  - `crl_quarantine_sets_and_unload_clears_the_sender_fence` (AC8)
  - `poller_reports_an_error_class_change_once_and_counts_repeats` (AC7)
- `crates/maos-iac` unit tests: `a_quarantined_sender_frame_is_neither_logged_nor_delivered`, `a_quarantined_pid_distillate_write_is_refused_typed`.
- `crates/maos-bin/tests/quarantine_closure_lrqc.rs`:
  - `yank_quarantine_survives_restart_and_reload_and_reapplies_before_running` (AC2: daemon restart, unload/reload including boot reload, and crash disposition/reload; witness reapplication and receipt before `Running` on permitted paths)
  - `a_paused_spirit_matched_by_a_yank_is_fenced_on_resume` (AC2)
  - `release_verb_is_door_only_journaled_and_refused_while_a_crl_matches` (AC3)
  - `door_reads_back_quarantined_and_director_paused_separately` (AC6)
  - `swap_of_a_quarantined_spirit_is_refused_until_release` (AC2/D-08 A: typed refusal for cold upgrade and hot swap, with the hold left intact; no forbidden swap required)
  - `a_quarantined_worker_egress_bearer_is_refused` (AC1)
  - `operator_halt_context_still_reaches_a_quarantined_spirit` (AC1 preserve)

Each new security vector needs one planted red: remove the specific fence, observe the named test fail, restore. Record that per `proof-contract.md`. Planting and observing a red is a verification step, not attack replication. Manifest-schema changes under D-04 A widen the run to all manifest readers and `check-corpus`. AC4 needs a hosted T3 run on both images, recording run, job and SHA.

## Kernel and kloc forecast — method, not a prototype

**Method:** read each kernel routine the ACs touch and count the additions each recommended option needs (a fence check and typed return, a new enum variant and its match sites, a call that sets or clears a fence, error-state tracking). No source prototype, compiler output, gate output or tokei measurement was produced. The forecasts are `[INFERENCE]`.

The baseline is the latest checked-in exact-pin HISTORY: **26,061 physical / 20,428 tokei / 90 files** (`xtask/kernel-core-baseline.toml:505-509`, 27-1). The `maos-kernel-core` ceiling row reads **20,429** (`xtask/kloc.toml:272`). Kernel sources are unchanged since `0706d5df`. These are recorded values, not a new measurement.

| Kernel file (read) | Recommended option → net tokei | Basis |
|---|---|---|
| `memory/mod.rs` | D-06 B: +8…+20 | Fence check in `write` plus a kernel-origin entry. (A: −8…−20 here, +5…+15 in `for_spirit.rs`/`kernel_ctx.rs`; C: +4…+8.) |
| `halt/resolver.rs` | 0…+2 | Switch to the kernel-origin entry. |
| `capability/mod.rs` | +6…+14 | `set_scalar` fence plus a `director_paused` accessor for AC6. |
| `capability/working_memory/orchestrator.rs` | +4…+10 | Two scalar entries (`:68,:115`). |
| `scheduler/hook_dispatch.rs` + `hot_swap/coordinator.rs`, `idle_watchdog.rs`, `scheduler_loop.rs` match sites | D-07 A: +10…+30 | New `HookOutcome` variant, the check, and non-strike handling at the kernel match sites. |
| `revocation/applier.rs` | +1…+4 | `_iac` becomes used; sender fence set (KQ2). D-04 C would add +15…+35 here and in `rules.rs`. |
| `scheduler/scheduler_loop.rs` (unload) | +1…+4 | Clear the sender fence (`_iac` at `:86`). |
| `revocation/poller.rs` | D-09 A: +6…+18 | Error-class tracking, print on change, recovery line, `AlreadyApplied` as steady state. (B: +3…+8.) |
| `hot_swap/coordinator.rs` (swap refusal) | D-08 A at the door: 0 | B: +3…+8. |
| Durable store, Worker identity, release verb | 0 | maos-bin / maos-manifest / maos-control. D-01 A has a +0…+6 contingency if an applier sink is needed. |
| **Total** | **+36…+116** | Worst listed option combination: +151. |

Projected from the recorded 20,428: **20,464…20,544** (worst 20,579), below the 22,000 alarm and the 23,000 envelope. For concurrent GA stories, the drafted upper bounds of 21-9 (+250) and 21-6 (+440), added to this upper bound, give 21,234 `[INFERENCE]`. That is also below 22,000. It is not a reservation, and every landing measures its own base.

Other production forecasts (ceiling rows read at HEAD):

| Crate | Forecast | Ceiling row |
|---|---|---|
| `maos-bin` | +140…+360 (durable store and re-application, door verb and readback, Worker bind, yank re-key, bearer wiring) | 39,699 (`kloc.toml:527`) |
| `maos-iac` | +20…+45 (sender fence and distillate fence; priced +24) | 9,053 (`:416`) |
| `maos-domain` | +6…+16 (`IacBusError`, `MemoryError` variants) | 10,483 (`:435`) |
| `maos-capability` | +2…+5 | 1,554 (`:273`) |
| `maos-control` | +12…+35 | 1,596 (`:494`) |
| `maos-cli` | +15…+45 | 6,745 (`:499`) |
| `maos-egress` | 0…+15 | 1,869 (`:680`) |
| `maos-manifest` | +20…+60 (D-04 A) or 0 | 4,889 (`:425`) |
| `maos-audit` | 0…+10 | 8,487 (`:419`) |

Non-kernel rows are retrospective ceilings, not per-landing exact re-pins (ADR-069 amendment). Tests and documents are outside the production budgets.

At landing, measure with `env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=$PWD/xtask <xtask> check-kernel-baseline --json` and `kloc-check --json`. Re-pin with `scripts/repin.py`, write the HISTORY row in the same commit, and refresh any caused ADR-066 surface hashes under `kfree`. Do not run reverted kernel prototypes or revive per-story FLAG-Winston requests.

## Cross-story dependencies and claim limits

- **21-6 (memory quota):** both stories touch `MemoryManagerAdapter::write` and `MemoryError`, plus the same serde allowlist lines. Whichever lands second rebases onto the first. Order inside `write`: the quarantine fence comes before quota charging, so a refused write is not charged.
- **21-9 (WASM default-on):** spawned and WASM forms receive hooks through the kernel dispatcher. Under D-07 A, AC1's hook fence covers them; witness one spawned form. 21-9's telemetry scope declaration (D-21-9-02) is unaffected.
- **21-10:** consumes AC5's evidence. D-LRQC-10 sets which side edits which I5 lines.
- **20-1b (`done`):** its record is not reopened. The AC11.5 hosted evidence stays historical.
- **Release lane:** the claim text (AC10) is handed to the release-notes owner. NFR-Sec-7 and isolation are not claimed.

## Re-measured cite corrections — parent-owned source updates

Only this story file is written. These are OLD → NEW handoffs to each document's owner, not edits:

| Source premise | OLD → NEW at `de16fa0a` |
|---|---|
| Epic §later-registry-quarantine-closure `:316`, `:325-327`; Epic §21-10 `:264` | Tracker row `sprint-status.yaml:448` → `:503`; E20-A9 `:724-725` / `:725` → `:780-781` / `:780`. |
| `epic-21-preflight-2026-11-12.md:8,82,102` | `S:448` → `:503`; DW owner rows `:1268, :1578-1586` → `:1268, :1579-1587`. |
| `sprint-change-proposal-2026-11-12.md:162` | `:448` → `:503`. |
| 20-1b closure note `:25`, story `:216`, retro `:156` | FR13 row `~1267` → `:1268`. |
| Tracker row `:503` and DW `:1585` | "needs its own FLAG-Winston grant and a prototype" / "creation-time prototype measurement" → superseded by `kfree` (DD:33; ADR-069 amendment `:74`): forecast only, measured at landing. |
| `decisions-2026-11-12-acceleration.md:54-66` "Kernel room" | 21,201 tokei at the ruling → recorded 20,428 now (after 21-5's −827 and 27-1). The +30…+80 figure → this story's +36…+116 (method above). |
| `docs/invariants/I5.md:5,11`, `README.md:33` | `owner: 20-1b-vetting-yank-policy-and-quarantine` (a `done` key) → owner per D-LRQC-10 (this key for FR13; 21-10 for I5). |
| DW `:1582` | `quarantine-release <pid>` → door convention `<spirit_id>` (D-LRQC-03). |
| Epic AC1 "IO sends" | No pid-bearing IO port exists (`io_subsystem.rs:15-30`). The Spirit-reachable IO is token-mediated MCP/inference plus Worker egress via `BearerTable`; AC1 names those. |
| Held cites | `worker_spawn.rs:919-922` holds (the closing `};` is `:923`); C1b lines 7, 9, 53 hold; `I5.md:11` and `README.md:33` hold. |

## Dev Agent Record

### Agent Model Used

Creation: **anthropic/claude-opus-5-5** (`DraftQuarantineClosure`). No implementer or independent-review attribution is claimed.

Approval recording: **openai-codex/gpt-6.2-sol** (`QuarantinePolicyRecord`). No implementation or verification was performed.

### Completion Notes List

- Created the existing key's story at `de16fa0a` in `/home/lunarpulse/dev_ws-course28`. Only this file was written. No tracker, epic, `deferred-work.md`, invariant or source edits; no staging, commit or push.
- Read `bmad-create-story`, exemplars `21-9-wasm-host-default-on.md`, `21-6-memory-tier-quota-enforcement.md` and `ops-ci-release-ready.md`, the authority chain (`q20`, `ga3`, `kfree`, E20-A9, epic §, closure note), the ten owner rows, the cited kernel/bin/iac/domain sources and the KQ2 priced diff.
- No builds, Cargo, tests, gates, formatters, prototypes or remote jobs. Kernel counts are checked-in HISTORY values.
- **Open at creation:** D-LRQC-01…10, all operator-owned. Also open: 21-10's split, and therefore the exact AC5 recipient. The hosted AC4 leg remains a landing obligation.

### 2026-11-21 static review: open findings

Fixed after the review: Worker release CRL identity (`operator_door.rs` `quarantine_release_command`), live CRL propagation now checks every entry's range (`revocation/applier.rs`, same predicate as admission), spawned-session ordinary frames refused while fenced (`spirit_session.rs`), Worker schema-2 readers migrated and canonical SemVer enforced (`maos-manifest/src/manifest.rs` `take_worker_identity`), yank row carries the artifact digest (`yank_policy.rs` `policy_request`). Deferred (not fixed; each needs its own change):

- Release vs concurrent CRL import race: no guard between the CRL check and the release (`operator_door.rs` `quarantine_release_command`).
- Hold/admission serialization: `QuarantineGate::quarantine` and `reapply` (`quarantine.rs`) are not serialized with admission, so a hold written during an admission can miss that pid.
- CRL durability at boot: an applied CRL is not reloaded before admission (`main.rs` boot order, `revocation/poller.rs`).
- Swap commit guard: the quarantined-swap refusal is checked once at `upgrade_command` entry, not at the coordinator commit.
- Effect-boundary fences: a fence set mid-effect does not abort an in-flight write/send (`memory/mod.rs` `write`, `maos-iac` `deliver_typed`).
- `on_load` ordering: `on_load` fires inside `load_obj` before `quarantine.reapply` runs (`admission.rs` `admit_inner`).
- Release journal recoverability: if the release approval row succeeds and removal fails, the row is not compensated (`quarantine.rs` `release`, `operator_door.rs`).
- Poller telemetry: per-stage error kinds are coarse (`revocation/poller.rs` `poll_once`).
- Directory fsync: only the store parent is synced; the `holds.json.tmp` leftover is not cleaned (`quarantine.rs` `persist`).
- The spawned-session fix has no test (the session harness needs a live T2 runner).

### File List

- `_bmad-output/implementation-artifacts/later-registry-quarantine-closure.md` — new, measured story only.

## §A6 independent review artifact — 2026-11-21 (static)

An independent light reviewer agent performed a read-only static review (no builds or tests) and found **8 HIGH, 5 MEDIUM, and 2 LOW** findings. The fixes and remaining gaps below are recorded separately from the orchestrator's local execution evidence.

### Fixed after review

- `quarantine-release` checks CRLs against the loaded SCB's declared `(class.name, class.version)` as well as the hold identities. A Worker instance ID such as `worker-2` is not its class name.
- Live CRL propagation uses the same class-and-range predicate as admission, so a later matching range is no longer ignored.
- A quarantined spawned incarnation no longer starts a mailbox frame turn: ordinary frames are refused through `journal_undeliverable`, while lifecycle requests are unchanged. No session test was run; the harness needs a live T2 runner.
- The `spirit_smoke` and `worker_manifests_2a` readers migrated to Worker schema 2, and the Worker class version must use canonical SemVer.
- Yank audit rows carry `artifact_sha256`.

### Deferred / open — recorded, not closed

- The release check is not serialized with the CRL admission guard, so release can race a new matching CRL.
- Yank quarantine can miss concurrent admission or reload, and a stale snapshot can reinsert a PID fence after unload. A failed hold write protects only the already-known PID; there is no pending fail-closed identity entry.
- Installed Spirits boot before the CRL poller starts. Door-imported CRLs are applied in memory but are not persisted.
- Swap refusal is a preflight check, not a commit guard. A fence applied after the check can still let a cold, hot, or migrator swap proceed.
- Effect checks do not synchronize with the effects they protect: IAC append, hooks queued on a blocking worker, and memory, scalar, and distillate check-then-mutate paths. The CRL applier writes `spirit.revoked` before setting fences.
- Hold reapplication runs after `load_obj` / `on_load`, and the Worker egress fence is cleared before the Worker is stopped.
- Release does not meet D-LRQC-03 journal ordering: the completion row is written after dispatch. There is no authenticated operator route to retire a CRL.
- Poller telemetry does not distinguish a changed reason within one error bucket. First-write durability does not fsync newly created ancestor directories.

Schema-1 Workers remain unaddressable by a CRL. This limitation was approved and disclosed in `SECURITY.md`.

### Local execution and proof limits

The orchestrator reported these local runs green. No hosted receipts were supplied.

| Crate | Green local targets |
|---|---|
| `maos-kernel-core` | `quarantine_closure_lrqc`, `quarantine_fr13_20_1`, `--lib` |
| `maos-iac` | `quarantine_sender_fence_lrqc`, `--lib` |
| `maos-bin` | `quarantine_closure_lrqc`, `worker_quarantine_20_1b`, `worker_manifests_2a`, `yank_policy_20_1b` |
| `maos-manifest` and the Worker crate | Green; individual targets not supplied. |

`check-service-boundary`: **0 violations** after refreshing the `RevocationApplier` and `HookOutcome` surface hashes and adding a `HookOutcome` supervision class row. The ADR-066 refresh is **not ratified**.

**Not run:** planted reds; the hosted AC4 T3 leg through the built binary; door-level witnesses for release refusal while a CRL matches and for `quarantined_swap_refused`.

**KLOC:** kernel 20,429 → 20,563 (+134, about 18 above the +36…+116 forecast); `maos-bin` 39,909 → 40,273 (+364, 4 above the +140…+360 forecast ceiling); kernel pin `src_lines`: 26222. Accounting against the ADR-069 22,000 alarm across 21-9, 21-12, 21-6, 21-10, and this story remains an open operator ruling.

**I5 FR13 stays `asserted`:** the witnesses have not been proven red. **This story does not close the registry quarantine requirement.**

## Change Log

- 2026-11-21: created at `de16fa0a`. Recorded the full epic scope (AC1–AC9) plus the ledger, claim and accounting ACs, the current-tree seams, ten open D-rows, conditional kernel/kloc forecasts and parent-owned cite corrections. Status `backlog` pending rulings. Tracker untouched (`sprint-status.yaml:503` still `backlog`).
- 2026-11-21 UTC: recorded operator approval of D-LRQC-01…10 (A/A/A/A/A/B/A/A/A/A). Aligned durability and swap proof with D-LRQC-08 A; preserved the FR13-to-I5 handoff and proof requirements. Status remains `backlog`; all tasks remain open.
- 2026-11-21: recorded implementation on branch `ga-quarantine-2026-11-21`, the independent static review, post-review fixes, and open findings. Status: `in-progress`; local evidence only, no hosted receipts.
