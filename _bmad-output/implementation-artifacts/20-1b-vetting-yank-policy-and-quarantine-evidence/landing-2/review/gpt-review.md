## Verdict

**Incorrect — 2 findings: 1 P1, 1 P2.** Reviewed commit `608735d28b7c8ba6b815a6cb812e755b2d240bee`. Confidence: **0.97**.

Report supplied here; no files written. No builds, tests or gates executed.

## Findings

### 1. [P1] Fence actual memory operations before retaining quarantined Spirits

**File:** `/home/lunarpulse/dev_ws/crates/maos-kernel-core/src/revocation/applier.rs:173–177`  
**Confidence:** 0.98

Replacing quarantine’s eventual unload with a permanent capability fence leaves actual Spirit-facing memory operations usable indefinitely. `PidBoundPrincipalPort::{write,read}` reaches `MemoryManagerAdapter` through `for_spirit` (`butler_root.rs:80–108`), but the private/shared `write`, `read` and `scan` implementations (`memory/mod.rs:699–895`) never consult the fence. The newly added “memory” witness only attempts to **mint a `MemWrite` token**; it never performs a memory write. Thus a quarantine-declaring Spirit can remain Running past the former drain deadline and continue modifying durable memory, contrary to AC11.2’s claimed refused memory effect. Scalar mutation also bypasses the fence through `CapabilityRegistryAdapter::set_scalar` and `WorkingMemoryOrchestrator`.

**Exact fix:** Check the pid’s quarantine/pause fence at the actual Spirit-facing memory and scalar mutation boundaries, before persistence/publication, and propagate refusal. Do not substitute token-issuance assertions for effect assertions.

**Witness needed:** Retain the real pid-bound principal/scalar ports, apply quarantine, advance past the old deadline, attempt writes, and assert refusal plus unchanged stored values and publication counters. Include an unfenced bystander.

### 2. [P2] Retain classification for persisted quarantine-request rows

**File:** `/home/lunarpulse/dev_ws/crates/maos-audit/src/fr4_classifier.rs:413–417`  
**Confidence:** 1.00

Renaming the existing writer-shape entry removes classification for previously persisted tokenless `spirit.quarantine_requested` rows with `quarantine_requested: true`. Those rows were emitted by the parent commit’s CRL quarantine arm. After this upgrade, `classify_fr4_row` falls through to `Call`; `to_fr4_plain`/`to_fr4_ndjson` then raises `Fr4SchemaViolation` for the missing capability token, and `maosctl audit query --spirit …` exits 2. Changing the producer does not rewrite existing Transparency Logs; the classifier itself explicitly describes its history table as permanent.

**Exact fix:** Keep the historical intent/payload-shape entry alongside the new `spirit.quarantined` classification. Continue emitting only the new marker.

**Witness needed:** Feed both historical and current rows through both FR4 projections. Both must classify as non-call events; malformed lookalikes must still fail closed.

## Requested boundary assessment

| Surface | Result |
|---|---|
| Capability issuance | Fenced through `issue_with_mediation`, including its trait delegates. |
| Token verification | Fenced through `verify_and_audit`. |
| Post-call recheck | Fenced through `recheck`. |
| MCP | Pre-call verification and post-call recheck both consume the fence. |
| Inference/provider adapter | Pre-call verification and successful-response recheck consume it. Already-started paid/remote effects are not undone. |
| Collective memory | Explicit token-bearing APIs verify before the port call. |
| Private/shared/principal memory | **Not fenced: finding 1.** |
| Scalar memory/publication | **Not fenced:** `set_scalar` mutates/publishes without checking quarantine. |
| Native distillation | No capability-fence check in the directly bound `DistillationPort`; the writer’s `DistillateWriteToken` is an insertion guard, not pid authorization. The broad isolation claim needs this boundary addressed or explicitly narrowed. |
| Spawned IAC | New frame authorization mints a governed `iac.send` token and is therefore fenced. |
| Native/queued IAC | No dedicated quarantine check at bus/DRR delivery. KQ2 was explicitly not selected; this is not an accidental omission of that ruling. |
| HTTP/process/container activity | Not universally dynamically fenced. `IoSubsystemAdapter` has no pid input; Worker proxy requests use a separate `BearerTable`, not `CapabilityRegistryAdapter`. Existing sandbox rights and egress leases are not equivalent to the new capability fence. Hosted T3 measurement remains necessary. |
| Recall/skill authoring | Governed mint/verification paths consume the fence. |
| Heartbeat/context/telemetry | Administrative/reporting paths remain available; quarantine does not stop execution or hook dispatch. |

### Lifecycle, ordering and races

- **Resume:** Correctly clears only `paused`; quarantine survives.
- **Unload:** Clears quarantine on the normal loaded-SCB teardown path. It does not make revoked tokens valid again, and CRL admission rules can still reject a subsequent load.
- **Absent pid:** `set_quarantined` accepts any pid. `unload` returns early for an absent SCB without clearing that entry.
- **Pid reuse:** Allocation uses wrapping `AtomicU32::fetch_add`, not a generation-bound identity. Normal allocation does not immediately reuse unloaded pids, but stale entries can affect a future allocation after wrap—or an explicitly fenced future pid. Crash teardown also does not clear the quarantine set. These are lifecycle limitations, not evidence of an ordinary immediate-reuse failure.
- **Hot swap:** The coordinator preserves the SCB/pid and replaces its runtime. The quarantine therefore survives a same-pid swap, including a concurrent replacement; it is not artifact/version-bound.
- **In-flight work:** MCP/inference rechecks suppress successful responses after quarantine, not remote effects already initiated. No atomic transaction couples the fence with every effect.
- **CRL arm:** No drain or termination is spawned. The quarantine fence precedes `spirit.quarantined`; the generic `spirit.revoked` frame precedes the arm.
- **Journal failure:** TL insertion returns `LogBeforeDeliver`, **not a discarded `Result`**. SQLite failure panics under existing I2 behavior. A quarantine-marker write failure leaves the in-memory fence set; panic/task containment is not newly proven to halt the entire daemon.
- New receipt payloads contain no hexadecimal identifiers requiring colon encoding. They use the standard pre-write redaction path.

## Policy and poller observations

- Publisher verification compares the signer with the installed/pinned publisher. A publisher-origin yank signed by another publisher is rejected. Vetter authority follows enrolled-vetter authorization, not publisher equality.
- Forgery, wrong audience and unknown publisher are rejected before the new effect. Wrong installed version/origin is verified but non-actionable.
- Running/install association remains **name+version**, not publisher/artifact identity; that limitation was already explicitly dispositioned in L1.
- Only `Running` SCBs are selected. Paused/Loaded instances and installed-version-versus-running-version divergence are not repaired here. Consequently, a yank arriving during those states can be consumed without fencing that instance. This selection predates the patch and matches the recorded narrow Running-only shape; it needs an operator scope decision rather than mislabelling it a newly introduced filter.
- Immediate same-runtime replay is deduplicated by `observed`. Restart persistence of the cursor does **not** restore quarantine.
- **AC12 matches the ruled first-failure/until-recovery design, but it is not lossless error deduplication.** For example, `AlreadyApplied` followed by `SignatureInvalid` without an intervening successful poll hides the signature failure. Telemetry records only generic `ErrorKind::App`; recovery reports total failed polls, not error classes. Decide whether changed-error reporting is required.
- Raw error display remains susceptible to control characters in error strings/paths; this logging behavior predates the patch. No new log-sanitization guarantee should be claimed.

## Acceptance, grants and evidence

- **AC13 is not closed.** The retained negative-control artifact reports **39 stale-owner violations** after flipping the story to `done`. The real-tree pass depends on leaving status unflipped.
- Restart persistence is explicitly disclaimed in the landing record, decision record, architecture addition and deferred row. AC7 promises cursor restart safety, not durable quarantine. Deferral documents the gap; it does not justify claiming persistent isolation or closing remaining obligations.
- Recorded kernel figures reconcile: **26,967 / 21,141**, delta **+78 / +68**, below rebased **27,045 / 21,209** and shared **21,250** STOP. These were inspected as retained evidence, not independently remeasured.
- The committed surface-baseline patch changes exactly the `CapabilityRegistryAdapter` hash.
- FR4 additions are functionally necessary. However, literal claims that **38,392 is inside 35,671**, or **8,297 is inside 8,204**, are false. Any authorization argument needs explicit rebased ceilings; unused room from another story is not itself authority.
- The serde violation is real and **pre-existing**: `git show 5ff0ead1` contains the introduced `serde_json::from_str(&text).unwrap_or_default()`. Landing evidence reports it at `main.rs:11090`; the parent location was `11092`. The hosted workflow runs this gate as hard-fail, so a deferred row alone will not make that job green. No hosted result was observed.
- All six retained plant logs show assertion failures, not merely compilation failures. Restoration booleans remain recorded attestations, not independently reproduced proof.
- `diff/bin-ac8-effect.diff` is an intermediate snapshot: it includes receipt `origin`/`cause` fields absent from committed HEAD. It should not be cited as the exact final patch.
- Hosted CI, T3, TLS/macOS legs and prepush are honestly marked unverified. No new production serde `unwrap`/`expect` was found in this patch’s changed paths.

## Operator decisions

1. Fix the two findings before accepting the landing.
2. Resolve the claimed isolation boundary: actual memory/scalar/distillation effects versus capability-token-only fencing.
3. Accept or separately price changed-error reporting, persistent quarantine and stronger instance/provenance binding.
4. Record valid rebased audit/bin authorization; keep AC13 open.
5. Route the inherited serde hard-fail to 20-11 before expecting hosted discipline green.