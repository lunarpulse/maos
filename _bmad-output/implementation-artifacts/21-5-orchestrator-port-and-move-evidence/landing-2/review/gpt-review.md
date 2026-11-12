## Review: `637721bf`

**Findings: P0 0 · P1 0 · P2 0 · P3 0.**  
**Overall correctness: correct. Confidence: 0.92.**

No provable, unintended patch-introduced defect found. This is a source-and-evidence review—not fresh verification or approval to close the story.

### Headlines

- **Move preserves sequential behaviour.** Compared all five moved files with their parent-commit kernel versions. Outside the deliberate replay repair, changes are import/re-export paths and the timestamp path, which resolves to the same `maos-capability` implementation. No dropped validation, changed journal formatting, or altered error mapping found.
- **One-lock replay closes the loss window.** Exactly `len` pop/push pairs preserve FIFO order and count. Each push follows a pop, so capacity 32 cannot overflow. An enqueue already holding the mutex completes before replay acquires it; otherwise enqueue waits. Registry lookup returns a cloned `Arc` and releases its DashMap guard before buffer locking. No new lock-order cycle or callback under the buffer guard found.
- **Concurrency behaviour deliberately changes.** Pending count no longer exposes the temporary drained queue. A racing producer can now receive `QueueFull` rather than occupy that artificial gap; consequently refusal audit rows can differ. The lost-tail `resume re-enqueue failed` stderr disappears. These are intended consequences of D-21-5-07 B, not universal schedule-by-schedule equivalence.
- **Port wiring is intact.** `None` was already supported before extraction; this is not a new silent-skip regression. Inspected constructor sites: production `main.rs` passes the shared registry as `Some(...)`. Scheduler resume forwards the Spirit ID before `fire_on_resume`; the registry implementation reaches the correct buffer without creating unknown buffers. The existing domain trait is object-safe, `Send + Sync`, synchronous and infallible; poisoned-buffer handling remains unchanged.
- **Surface removal is surgical and sanctioned.** Exact commit diff removes the intended 21 triples and refreshes only `SpiritSchedulerAdapter`: **401 → 380**. Nine classification rows disappear. ADR-070 and its index entry satisfy the explicit removal ruling, following ADR-066’s record form. Old Rust imports intentionally break; the Spirit ABI/manifest surface and ABI baselines do not change. No additional policy violation established; unknown out-of-repo Rust consumers cannot be enumerated from this repository.
- **Dependencies and controls fit the implementation.** Normal `tokio` and `maos-capability` dependencies are required by the moved gateway host. The closure scanner consumes the new forbidden-crate entry correctly. The source-text structural test is a useful literal-path tripwire, not an AST-complete prohibition; combined controls satisfy the specified scope.
- **Plants support the intended invariants.** Inspected logs substantiate missing-call, item-loss, ordering, two-lock-race and structural/dependency failures. The substituted loss/order mutations are not literally “drop-last,” but exercise the required failure classes. Recorded race evidence is **11 red before / 10 green after**; stress runs are evidence, not exhaustive concurrency proof.

### Accounting and claim boundaries

- Kernel: **27,103 → 26,062 physical (−1,041)**; **21,256 → 20,429 tokei (−827)**; **95 → 90 files**—exact priced reductions.
- Leaf: **833**, priced 829 plus documented replay +4.
- xtask: **45,462 (+1)**.
- Aggregate/hardfail: **203,250 / 203,251**.
- Hand-spliced pin changes match the emitter’s format and intended file changes. Committed gate output reports matching hashes and no drift; I did not independently recompute hashes.
- Deferred capacity-zero and FR21 rows remain open.
- Hosted validation, prepush, clean-tree ABI verification and the pre-move kernel-vector run remain explicitly unverified. No builds, tests or gates were executed during this review.

Report supplied here; no file written.