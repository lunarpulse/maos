## Verdict: incorrect

**3 findings: 1 P1, 2 P2.** Reviewed commit `5ff0ead`; no builds or tests executed.

### [P1] Preserve policy halts when provenance exceeds the new cap

**Location:** `/home/lunarpulse/dev_ws/crates/maos-domain/src/frame.rs:274–275`  
**Confidence:** 0.99

A cold Butler assessment containing 40 mutually overlapping confirmed events with short IDs produces 780 conflict pairs and over 4096 bytes of `derived_from` (`spirits/butler/src/lib.rs:653–665`). Its `belief_variance` is 0.99, above the shipped 0.7 halt threshold. The scalar write persists, but payload construction now returns `FieldTooLong` before `invoke_halt`; Butler swallows that error as `last_halt_receipt = None` (`:1151–1152`) and finishes normally. Thus a valid policy halt disappears without a receipt or visible refusal. **Fix:** migrate the provenance producer to a bounded, durable reference and make scalar-port failures visibly refuse the pass rather than treating them as “no halt.” **Required witness:** run that calendar scenario through Butler and the real scalar orchestrator; assert a durable pending halt and receipt, not silent continuation.

### [P2] Preserve fail-stop handling for duplicate kernel frame IDs

**Location:** `/home/lunarpulse/dev_ws/crates/maos-kernel-core/src/halt/mod.rs:558–560`  
**Confidence:** 0.98

The new explicit-ID insertion returns `LogBeforeDeliver<FrameRowWrite>`, but its result is discarded. If the minted ID collides with an existing TL primary key, the adapter returns `Duplicate` without writing this halt (`transparency_log.rs:1408–1410`); `invoke_halt` nevertheless appends the lifecycle entry, registers the halt and returns a receipt citing the existing row. The former `insert_frame_event` path explicitly panicked on this condition, preserving I2. **Fix:** consume the result and require `FrameRowWrite::Written`, failing stop on `Duplicate`, as other kernel-minted writers do. **Required witness:** deterministically inject a minted-ID collision with a foreign row and assert no successful receipt or registry insertion.

### [P2] Bound resolution-history collection alongside the halt scan

**Location:** `/home/lunarpulse/dev_ws/crates/maos-bin/src/main.rs:11065–11074`  
**Confidence:** 0.97

Before applying the 50,000-row ceiling, the new scanner unconditionally reads all matching `halt.resolve` history into a `HashSet`. The query has neither a limit nor a supporting capability/target index; the schema indexes only `(actor, timestamp_ns)`. A Spirit with a large resolved-halt history therefore causes unbounded allocation on every uninstall scan—even when it has no orphan candidates—and the post-closure scan repeats it. Previously, resolution checking was a SQL `NOT EXISTS`, not an eagerly materialized history set. **Fix:** determine bounded candidates first, collect resolution membership only for those IDs, and cap decision-row examination with an explicit `CoverageGap` on exhaustion. **Required witness:** a large approval-history fixture with zero or one orphan; assert bounded retained state and honest exhaustion semantics.

## Closure and evidence

- The watchdog fence closes the reported stale-candidate/stamp race. SCB-map guards are released before awaiting it; hook dispatch has a budget timeout, and dropping the async mutex guard releases it. No concrete new deadlock found.
- Invocation dedup and decoded receipt-frame uniqueness are implemented in both verifiers; committed corpus and independent plants support closure.
- Guest/session ingress rejects halt frames. Other typed ingress serializes tagged payloads; raw enqueue/broadcast logs `TaskAssign`, so empty IDs alone do not recreate the invocation shape.
- Empty-ID handling produces a gap. The scan ceiling and potentially large receipt-closure batch are explicitly disclosed; F5 remains deferred.
- Recorded pins match **26,891/21,075**, within **26,893/21,078**; only the additional `ScheduleWatchdog` surface hash changed. Audit/bin/domain counts and the domain’s one-line excess are disclosed.
- Plant logs support their stated failures. Recorded workspace results are **5,995/5,997 passed**, with the two disclosed failures. AC5/EC-13, AC6 live T3 and final-tree hosted verification remain open; the story remains `in-progress`.

**Headlines:** silent loss of oversized-provenance policy halts; ignored duplicate-ID outcome weakens I2; unbounded resolution-history materialization bypasses the scan ceiling.