# 20-11 landing 1 adversarial review — cbed4dad

## Scope and method

Reviewed commit `cbed4dad` production/test changes under crates, tools, tests and xtask against story AC1, AC2, AC4–AC7 and ruled D rows. AC3, O4 and KS are expressly final-kernel work; 20-1a/20-10 closing documents are excluded. Read skill `bmad-code-review` and steps 01–04; ran Blind Hunter, Edge Case Hunter, Acceptance Auditor and Test Infrastructure Auditor concurrently. No security layer was spawned (orchestrator owns it). No tests, cargo, formatters or mutation witnesses were executed by this review. Proposed witnesses below are requirements, not observed results. Source locations were read with surrounding callers/guards before triage.

Assignment overrides workflow interaction, story/tracker writes and patching: this artifact is the sole review write. Reviewers returned prose/tables rather than the edge JSON format; normalized below. All layers returned, but Acceptance explicitly left scan/test sections to the other layers. Existing evidence claims are observations of the developer report, not fresh reviewer verification.

**Result: 9 patch, 2 defer, 6 dismiss; no new operator choice blocks the patches.** Security review may add or merge findings. Do not mark this landing clean.

## Patch findings

### P1 — high — Final partial rowid page skips concurrently appended rows

- **Sources:** edge + test infrastructure.
- **Evidence:** `crates/maos-iac/src/adapter/transparency_log.rs:1676-1706`. The scan reads `last`, queries through `after + 4096`, then advances `after` to that unbounded endpoint. After releasing the lock, an append below that endpoint is skipped by the next `after >= last` check. Example: observed rowids 1–10, then rowid 11 committed in the deliberate between-window writer opportunity; cursor is already 4096. Applies to principal and writer/lineage scans and the late sweep. Violates AC5.1's explicit appended-row guarantee.
- **Exact fix:** clamp each endpoint to `min(after.saturating_add(FORGET_SCAN_PAGE), last)` and advance only through that observed endpoint; retain the fresh high-water read and error propagation.
- **Failing-before requirement:** deterministic once-only test hook after dropping the TL guard inserts a matching row in the final partial page. Assert exact returned/scrubbed IDs, no duplicates, timestamp/frame ordering, and that a second window observes the append. Include both sub-page and multi-page fixtures. Current code must miss the appended row before correction.

### P2 — high — Real-door 25% latency criterion is no longer asserted

- **Sources:** edge + test infrastructure.
- **Evidence:** `crates/maos-bin/tests/shell_host_16_2.rs:1627-1635` calculates and prints `max_insert / elapsed` but never asserts it. Story AC5.2 at `20-11-erasure-inventory-closure.md:256-258` requires <25%; clarification at :266-269 says keep that bound and **also** bind VM work. The independent VM tests at `crates/maos-iac/src/adapter/erasure_scan_work.rs:77-91` are real production-statement accounting, but cannot measure scheduler/lock/fsync latency and do not replace this criterion.
- **Exact fix:** restore `assert!(ratio < 0.25, ...)` for both real disk/WAL door sizes. Keep the VM assertions and unchanged deadlines/budgets. Do not relabel a loaded-host red as a pass or retry it to green; any change to the requirement needs an explicit operator ruling, not a comment in the test.
- **Failing-before requirement:** demonstrate the real-door oracle fails for a measured >25% writer delay (use the existing original-source/unbounded-page control where possible), while the current print-only test incorrectly succeeds. Preserve raw timings and restored plant evidence.

### P3 — high — Python accepts re-signed malformed v2 category schemas

- **Source:** acceptance.
- **Evidence:** `tools/verify-erasure/verify.py:306-325` checks duplicate names, identity, equality and inclusion but never validates category/status types. Rust's typed contract is `crates/maos-audit/src/erasure/proof.rs:20-32,56-62`: `Removed.count` is u64, status is a closed enum, name/reason/identity fields are strings. A correctly re-signed envelope with `Removed.count = -1` in both category copies passes Python's equality/inclusion checks but cannot deserialize as Rust `ErasureProof`. Unknown tags and missing status have the same problem. This is not a signature forgery; it violates AC1.3 and the requested malformed-evidence equivalence contract.
- **Exact fix:** validate v2 category and category-evidence schemas in Python before accepting them: exact typed fields, supported enum tags, string fields, and integral non-bool counts in [0, 2^64). Align unknown-field behavior with Rust's signed typed reserialization (do not let arbitrary extra signed fields gain an OK verdict in Python while Rust drops them and rejects the signature).
- **Failing-before requirement:** parameterized, freshly re-signed vectors for negative/overflow/fractional/bool counts, unknown/missing status, wrong name/reason/identity types, and extra category/evidence keys. Both verifiers must reject, distinguishing schema failures from stale signatures. Keep valid Removed/VerifiedEmpty/CoverageGap controls.

### P4 — medium — Category frame membership differs by textual hex spelling

- **Source:** acceptance.
- **Evidence:** Python `tools/verify-erasure/verify.py:319` compares the raw evidence ID string to `post_leaves`; Rust `crates/maos-audit/src/erasure/proof.rs:430-437` parses IDs and compares bytes. Re-sign a bundle with an uppercase or colon-free spelling of the same category ID: Rust accepts and Python rejects. Producer output is canonical, limiting ordinary impact, but verifiers disagree on the same validly signed v2 envelope.
- **Exact fix:** parse post-leaf IDs once and compare `fid` against the byte-ID set in Python, matching Rust. Alternatively impose and test one canonical spelling in both verifiers; do not leave divergent acceptance languages.
- **Failing-before requirement:** re-signed case/colon variants produce different verdicts before correction and identical verdicts afterward; malformed decoded lengths remain rejected.

### P5 — high — Package deletion failure can reinstall torn bytes and discard the only signed partial-erasure proof

- **Source:** blind.
- **Evidence:** `crates/maos-registry/src/storage.rs:121-147` recursively deletes staging before disarming rollback. `remove_dir_all` can remove some files and then fail; Drop renames that incomplete generation back and restores its index entry. `crates/maos-bin/src/main.rs:11210-11217` then deletes the already-persisted proof even though principal/lineage scrubs are durable. The ordinary proof-write rollback test at `crates/maos-bin/tests/erasure_inventory_20_11.rs:449-471` never reaches this failure. A package-stage error at main :11148 can likewise occur after durable erasure with no partial-proof handling. AC4.3 promises Failed retention, not an indexed corrupt generation or erased data without the available receipt.
- **Exact fix:** make package finalization an explicit, fallible transaction, not fallible recovery hidden in Drop. Preserve a complete recoverable generation/index until the destructive phase can be finalized; never restore an index entry for partially deleted bytes. On stage/final-delete failure, retain/restore the full package and produce a truthful signed partial proof whose installed-package status is a CoverageGap; include its path in Failed. Do not delete the sole proof of already-completed erasures. Merely setting `removed = None` before deletion prevents corrupt re-indexing but is **not sufficient**: it breaks Failed-retention and can leave residue falsely labelled Removed. If an implementation cannot meet the ruled retention contract, bring the exact failure semantics back for a ruling rather than silently weaken it.
- **Failing-before requirement:** inject deletion failure after at least one real generation file is removed; compare every original byte and installed index after Failed, verify the retained partial proof, and assert no corrupt generation is advertised as installed. Separately force index restoration/stage failure and require a surfaced recovery error, not only eprintln. Test ordinary proof-write rollback remains green.

### P6 — high — Offline lifecycle-journal failure occurs after package commit

- **Source:** blind + coordinator.
- **Evidence:** `crates/maos-bin/src/main.rs:11213` irreversibly commits package deletion before returning the cascade terminal. Offline caller :7457-7465 subsequently appends the lifecycle record and converts an append/open error to Failed. The installed package is already absent, contradicting D-05 A / AC4.3. The signed proof remains (the layer's suggestion that this path also deletes it was inaccurate).
- **Exact fix:** retain the pending package transaction through every operation that can change this uninstall's terminal to Failed, including offline lifecycle-journal completion. Commit only when the final successful terminal is established; on journal failure explicitly roll back and retain a truthful partial proof. Do not suppress the journal error or fabricate successful lifecycle completion.
- **Failing-before requirement:** installed principal-free Spirit, readable but non-appendable journal (or a narrow deterministic real-open failure hook after inventory). Require Failed plus byte-identical installed generation and restored index; current code returns Failed after deleting them. Ensure journal observation itself succeeds so the test reaches the actual post-cascade failure.

### P7 — high — Resolver I/O/schema errors are converted to verified emptiness for installed Spirits

- **Source:** blind.
- **Evidence:** `crates/maos-bin/src/main.rs:10742-10744` swallows **every** resolver error when a package is installed. `crates/maos-audit/src/lib.rs:1696-1730` shows errors include readonly open, SQL prepare/query and row decoding, not just unknown Spirit. Empty incarnations bypass the all-boots lookup (:10759-10764); lineage and authored-skill observations then report VerifiedEmpty (:11078-11079, :10544-10560, :10611-10612), despite an uncompleted identity query.
- **Exact fix:** distinguish a genuine no-identity result from resolver failures (typed outcome or equivalent explicit result); allow refused-before-admission installed packages only on the former. Propagate real query/read/decode errors before destructive package work, or sign explicit unknown-coverage gaps and a Failed terminal—never VerifiedEmpty.
- **Failing-before requirement:** installed package plus existing owned lineage/skill rows, inject a real malformed admission-row decode or resolver query error while the already-open TL remains readable for the other scans. Current implementation must incorrectly proceed to VerifiedEmpty; corrected implementation fails closed and retains package. Keep the legitimate refused-at-boot installed-package recovery control.

### P8 — medium — New full scans still execute directly on a Tokio worker

- **Source:** blind + coordinator.
- **Evidence:** async `run_uninstall_cascade_inner` calls `distillates_written_by` directly at `crates/maos-bin/src/main.rs:10764` and typed retained-store observations at :10783-10786 (including the unbounded Lifecycle Journal file scan :10567). Only memory forget (:10819) and individual lineage mutation (:10878) enter `block_in_place`. The window helper's `std::thread::yield_now` (`transparency_log.rs:1706`) yields the OS thread, not its Tokio task. Existing all-frame enumeration/proof construction and filesystem operations in the same cascade also remain synchronous. This fails AC5.4's structural off-worker requirement for the newly introduced large-log query.
- **Exact fix:** move synchronous scan/inventory/proof/package I/O regions off the async worker with the existing multi-thread-runtime `block_in_place` convention (or owned spawn_blocking work), preserving async hook dispatch between regions. At minimum include the complete new lineage enumeration and retained-store scans; audit pre/post enumeration and proof construction rather than declaring the whole cascade off-worker based on two call sites.
- **Failing-before requirement:** AC5.4 explicitly does not demand a nondeterministic starvation test. Provide structural before/after callsite evidence and a focused review assertion/check that the real new scans are inside the blocking boundary; do not invent a timing-green witness.

### P9 — high — Newly held late frames are ignored by uninstall's package/terminal decision

- **Source:** blind (corrected disposition).
- **Evidence:** `crates/maos-bin/src/hook_pump.rs:794-796,848-852` correctly retains late frames under the newly placed own-principal hold and returns `held_frame_ids`. `crates/maos-bin/src/main.rs:10820-10824` consumes only `redistill_outcomes`; `held_principal_ids` is populated only from the earlier initial `ForgetOutcome::Suspended` (:10860-10865). Thus a hold arriving between initial forget and late sweep can leave retained data but no legal_hold category, allow package removal (:11138-11150), and return Erased. Existing initial removed counts need not themselves be false; the defect is missing retained coverage and wrong terminal/package disposition.
- **Exact fix:** keep the ruled own-hold protection (do **not** restore its exclusion). Consume held/malformed/late-frame observations from `finish_forget`, recheck the relevant durable hold under the existing serializer, and propagate retained held-frame identities into the signed category gaps and Held/Failed package-retention decision. Do not treat a held late sweep as a completed full erasure.
- **Failing-before requirement:** real uninstall hook plants P's durable hold and one late P-bearing frame between initial forget and late sweep. Assert bytes retained, signed gap naming retained frame/hold, non-Erased terminal as specified by existing mixed-held semantics, and package/index retained. Current caller loses this information even though the standalone late-sweep protection witness passes.

## Defer — pre-existing, not introduced by L1

### D1 — medium — Purge destroys erasure-proof retention; prior route is not a completed policy decision

`crates/maos-bin/src/purge.rs:59` resolves the proof directory for product purge. No L1 resolver/purge change worsens this. Existing `_bmad-output/implementation-artifacts/deferred-work.md:1411` explicitly records 20-10's closing disposition: **ownerless/open**, operator keep/export/delete ruling and named implementation vehicle still required. The old story sentence routing this to 20-10 is stale; do not claim 20-10 has repaired retention. Preserve the existing disposition; no duplicate implementation request in this landing. An eventual operator decision is already owed, not a new review blocker.

### D2 — medium — Rust silently drops malformed leaves while Python rejects them

`crates/maos-audit/src/erasure/proof.rs:355-364` uses filter_map on parsed pre/post leaves; Python parses every leaf at `tools/verify-erasure/verify.py:249-250`. Re-signed malformed extra leaves can be discarded by Rust while Python rejects. These lines predate the landing; carry as existing verifier-hardening debt, separate from P3/P4's new category-evidence language. Future fix: collect fallibly and reject every malformed leaf; use freshly re-signed vectors.

## Dismissed / handled elsewhere

1. **Acceptance: offline schedule removal never unloads a live root.** Dismiss: offline path is explicitly the locked no-live-root recovery path (`main.rs:7431-7435`); AC4.2 authorizes installed-manifest enumeration and package removal prevents next-boot admission. No reachable concurrent loaded root was demonstrated. Live watchdog state remains explicitly a gap pending KS.
2. **Acceptance: Removed intent_lineage conflicts with retained IAC arrays.** Dismiss: D-01 A expressly defines erasure as owned kind-11 body scrubbing and retains IAC audit arrays with a named reason. `main.rs:11075-11097` names that retained surface in the signed bundle. Not an unauthorized erasure-completeness claim.
3. **Blind: fix late holds by excluding the principal's own hold.** Dismiss that proposed fix: it directly reverses AC5.3(a). The actual propagation defect is P9.
4. **Blind: body-scrub/marker and category/proof/package crash windows independently invalidate L1.** Dismiss as separate patch findings: known cross-store crash atomicity is explicitly acknowledged in `l1-final-report.md:75`; no new full crash transaction is granted. Ordinary-error proof loss and torn rollback are retained as P5/P6, not hidden behind that limitation.
5. **Blind: cached protected set necessarily erases a concurrently appended held derivation.** Dismiss standalone allegation: no concrete reachable fixed-target sequence was established beyond P1's scan append bug. Hold-set changes trigger recomputation (`transparency_log.rs:1543-1549,1794-1803`); new candidate bodies/edges require a precise interleaving before rating. Orchestrator security review separately owns cross-adapter serialization analysis.
6. **Test infrastructure: million-row fixture budget overrun requires a new exception now.** Dismiss as a new finding/operator demand: the concrete 48.677s aggregate/35.441s case is honestly recorded at `l1-final-report.md:64-67`, and existing `deferred-work.md:7` owns this exact new workload under 20-7 without budget widening or retries. Hosted timing remains unverified and the blocking CI budget must still pass; this is not a clean-gate claim or permission to waive it.

## Additional scope observations

- No AC3/halt-count or KS cleanup claim was accepted as L1 complete; `pending_halts` remains a signed explicit gap and the report labels hosted Worker proof unverified.
- FR4 new marker shape is narrowly bound to `spirit_id` plus frame ID (`fr4_classifier.rs:565-575`); no independent production-writer escape was established by these layers. Test-only file guard and its production-negative control still need final CI evidence, not an invented local reviewer run.
- Hosted live fixture uses a deliberately short temporary home (`erasure_live_20_11.rs:36`), rather than nesting socket paths under the checkout. No new hosted-length defect established.
- The two work-count tests seed actual 100k/1M rows and measure VM steps of the real production statements, with a fixed independent absolute cap (`erasure_scan_work.rs:30-91`); they are not tautological page-budget assertions. P2 concerns the missing independent wall assertion, and P1 concerns missing append correctness coverage.
- Category inclusion proves frame-ID membership, not TL payload contents: the bundle's matching category copy is signature-attested. This residual trust model was not relabelled proof of payload content in this review.

## Handoff

Patch items are actionable without a new scope decision. For P5, an inability to preserve the ruled failure-retention guarantee must be escalated explicitly rather than adopting the layer's insufficient 'disarm Drop before delete' suggestion. No patches applied. Main/coder must run the focused affected suites and required gates after repairs; do not count any proposed failing-before vector above as already exercised. Keep hosted/final-kernel proof and the existing purge-retention decision outstanding.
