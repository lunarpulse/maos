**3 P2 findings; 1 unresolved grant decision. Verdict: incorrect.** Report provided here because the available tools are read-only.

## Findings

### 1. Restrict the accessor to cost-scope operations
**P2 · Confidence 0.99**  
`/home/lunarpulse/dev_ws/crates/maos-kernel-core/src/capability/mod.rs:282–284`

Returning `&CapTokensShardRing` exposes mutation through interior mutability, not merely attribution reads. Any registry holder can now call `tokens().issue()`, bypassing the policy, context-budget and issuance-quota checks in `issue_with_mediation`; the resulting token passes ordinary ring verification. It also exposes raw verification and revocation operations. No production Spirit currently receives the registry, limiting immediate exploitability. **Fix:** remove `tokens()` and provide only narrow adapter delegates for `bind_cost_scope` and `cost_scope`, updating their callers. **Witness needed:** demonstrate policy-denied/quota-exhausted issuance cannot be bypassed through the adapter’s public surface.

### 2. Preserve accepted principal identifiers through TL redaction
**P2 · Confidence 0.99**  
`/home/lunarpulse/dev_ws/crates/maos-kernel-core/src/inference/mod.rs:416–420`

The new branch journals the accepted principal verbatim, but the downstream TL redactor rewrites valid identifiers such as `0123456789abcdef0123456789abcdef` or `sk-alice`. Reconcile therefore reports a redaction marker while retaining `session_principal`/`Exact`, rather than the boot-bound identity. Worse, accepted `sk-a"b` serializes with an escaped quote; the byte-level redactor consumes its backslash but stops at the quote, producing malformed JSON that reconcile skips entirely, losing those tokens from totals. **Fix:** use a reversible, redactor-safe representation for these public identity fields, with decoding in frame readers and erasure matching; follow the existing colon-hex precedent for hash-shaped text without weakening general secret redaction. Rejecting additional otherwise-valid identifiers would require changing the ruled input contract. **Witness needed:** real emission → reconcile → forget vectors for lowercase-hex, prefix-shaped and escaped identifiers, preserving dimensions and an unrelated frame.

### 3. Avoid rescanning every historical task on each mint
**P2 · Confidence 0.98**  
`/home/lunarpulse/dev_ws/crates/maos-capability/src/cap_tokens/mod.rs:275–278`

Every bind scans all previous scopes while holding the global write lock. Each `get_scope` also clones the stored authorization scope. Production revocation only marks token state, and the ring has no caller of the shard’s `evict_expired`; consequently historical production entries remain, making N task bindings perform quadratic cumulative work and blocking emission-side scope reads during each scan. The ghost-token test does not exercise this lifecycle. **Fix:** attach the optional attribution scope to its existing token state and bind/read through that token’s shard, eliminating the separate full-map scan. **Witness needed:** sustained mint/revoke/expiry coverage demonstrating bounded per-bind work and correct attribution for paid in-flight completions.

## Grant and surface judgment

- **Shape B widens the public API in substance.** Its unchanged hash reflects the scanner’s omission of impl methods, not equivalent authority. Choosing this shape specifically to avoid the denied refresh is a mechanical dodge of the instrument, not substantive compliance with ADR-066’s intent. Narrow cost-scope delegates avoid exposing unrelated token authority.
- **The +11 capability overdraft remains unresolved.** `/home/lunarpulse/dev_ws/xtask/kloc.toml:265` describes an adjudicated overdraft, while the committed decision record explicitly says Main’s ruling is pending. The measurement is coherent, but pending adjudication is not approval. Record the authorized ruling—or redesign within the existing grant—before treating this decision as discharged.

## Remaining review conclusions

- Verification precedes provider dispatch; scope lookup uses that verified token ID. Unknown/forged tokens fail before emission. The getter itself accepts unknown IDs, and missing attribution silently defaults; this is the explicitly specified fallback for unbound valid tokens.
- No cross-token reuse or reverse lock-order deadlock was found. Revocation preserves attribution for paid completions, followed by the existing response-denying recheck. The expiry/pruning race suggested by a standalone shard sweep is **not currently production-reachable**.
- Bound principals bypass the proxy; unbound attribution retains `Ambiguous { count }` without members. Env registration and colon/control/non-UTF-8 rejection are present; blank values intentionally bind none.
- Schema-2 wrapper emission and schema-1 reading connect correctly, including `none (schema 1)`. Existing decoded-value SR-3 scrubbing covers principal/task fields and preserves dimensions. The redactor interaction above remains uncovered.
- All three production bind sites are present. Smoke and spawned-session lease exclusions match their inspected paths.
- Recorded kernel red is **HEAD plus plumbing**, not untouched HEAD. Bin “red-before” is a restored-tree plant, not an untouched-HEAD run. SR-4 was honestly green-before.
- Eight committed plant logs record expected assertion failures and successful restoration comparisons. The report-key plant demonstrates missing task output; its failures do not independently prove same-principal task-key separation.
- Recorded arithmetic agrees: kernel **27,040/21,201**, **20** below ceiling and **49** below STOP; bin **38,549**; aggregate **202,855**, hardfail **202,856**. These are recorded-tree measurements, not fresh clean-tree verification.
- Recorded focused results: kernel **4 passed**, bin **6 passed**. Workspace result remains **6,027 passed, 1 failed**. Hosted discipline, clean-tree ABI diff, prepush and WASM witness remain explicitly unverified.
- No introduced production `unwrap`/`expect` after serde parsing was found.

**Overall correctness:** incorrect.  
**Explanation:** The landing exposes unintended token authority, mishandles accepted principal identities at the TL boundary, and adds historical-token-dependent mint cost. The capability grant decision also remains open.  
**Confidence:** 0.98.

No builds, tests, plants or gates were rerun.
