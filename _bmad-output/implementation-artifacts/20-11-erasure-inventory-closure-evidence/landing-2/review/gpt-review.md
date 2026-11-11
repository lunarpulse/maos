## Verdict: incorrect

**3 findings: 1 P1, 2 P2.** Reviewed commit `689a6a53`. No builds or tests executed; evidence below comes from source and committed logs. Report supplied here under the read-only restriction.

### [P1] Fence in-flight schedule candidates before clearing watchdog state

**Location:** `/home/lunarpulse/dev_ws/crates/maos-kernel-core/src/scheduler/schedule_watchdog.rs:136–140`  
**Confidence:** 0.99

`forget_spirit` clears the maps without coordinating with `check_and_fire`, which retains candidate SCBs outside the SCB-map lock and awaits hooks between candidates. If a poll snapshots Spirits A and B, then waits in A’s hook, the door can unload B, clear its state, observe zero residue and sign `scheduled_invocations: Removed`. Resuming A then processes the stale B candidate: it recreates B’s bucket, emits `schedule.fire`, dispatches B’s hook and recreates its timestamp. `HookDispatcher::hook_allowed` checks only the manifest, not lifecycle state. **Fix:** add a retirement fence shared by candidate admission, state writes and `forget_spirit`; invalidate queued candidates and finish admitted firings before clearing and reporting completion. **Required witness:** block A’s schedule hook after both candidates are captured, uninstall B, release A, and assert no later B dispatch, fire row, bucket or stamp.

### [P2] Deduplicate durable invocations before rebuilding pending halts

**Location:** `/home/lunarpulse/dev_ws/crates/maos-bin/src/main.rs:11080–11084`  
**Confidence:** 0.99

A repeated `invoke_halt` with the same ID writes another TL invocation before registry insertion returns `DuplicateHaltId`; this existing behavior is exercised by `halt_invoke_test.rs`. If that root exits without terminating the pending halt, `orphaned_halts` returns both invocation rows. The new reconstruction loop inserts the first and propagates the second insertion’s duplicate error before calling `terminate_spirit`, so uninstall fails repeatedly and never closes that halt. **Fix:** collapse duplicate invocation rows by `(boot_nonce, pid, halt_id)` before reconstructing the transient registry, producing one terminal receipt per logical halt. **Required witness:** invoke the same payload twice, assert the second typed duplicate error, abandon the registry, then run real offline uninstall and require one `unplanned_crash` receipt and `Removed{1}`.

### [P2] Reject reuse of one receipt across distinct halt removals

**Location:** `/home/lunarpulse/dev_ws/crates/maos-audit/src/erasure/proof.rs:520–522`  
**Confidence:** 0.97

The new halt arm validates receipt-frame membership but deduplicates only `halt_id`. Consequently, two differently named halt entries pointing to the same receipt frame satisfy `Removed{count:2}` in both Rust and Python. One terminal receipt can therefore substantiate an arbitrarily inflated halt count in an otherwise correctly signed bundle; signature checking does not reject malformed evidence signed by the attester. **Fix:** independently enforce uniqueness of decoded halt receipt-frame IDs in both verifiers, alongside halt-ID uniqueness. **Required witness:** add a re-signed corpus vector containing distinct halt IDs sharing one receipt frame and require both verifiers to reject it; retain a positive vector with distinct receipts.

## Other review conclusions

- O4’s close-before-drain and invocation lock lifetime implement the granted fence. Reviewed production callers propagate or visibly render refusal; no new silent swallowing was found.
- The Weak-slot construction removes the unconditional strong watchdog owner responsible for the recorded drain failures. The existing AST guard checks shutdown ordering, not comprehensive transitive sender ownership.
- Raw logs corroborate all six witness groups and their recorded plants. Final focused logs are green; the final workspace log records **5,986/5,987 passed**, with the disclosed AC5 million-row failure.
- Recorded pin evidence matches **26,880 physical / 21,069 tokei**, the set/per-file hashes and the single surface-hash refresh. The **8,313 audit overdraft** is explicitly adjudicated.
- Offline receipt nonce attribution is disclosed. AC5/EC-13, live AC6/T3, hosted final-tree verification and the private/index hold lease remain explicitly open; the story remains `in-progress`.

**Overall confidence:** 0.97.