## Findings

### P2 — Compare invariants against the live SCB, not optional retained text
- **File:** `crates/maos-bin/src/operator_door.rs:1773–1777`
- **Confidence:** 0.98
- **Body:** When `admitted_manifest` returns `None`, `zip` silently skips the invariant comparison, and the coordinator precheck still checks only halts and state schemas. This occurs for installed components: `InstalledSpirits::boot` admits their bundles but never populates `admitted_manifests` (`installed_spirits.rs:57–111`; `main.rs:4454–4490`). A schema-7 installed predecessor can therefore receive `safe: true` for a dropping successor that the actual coordinator refuses. Retained text can also disagree with the live declaration after `auto_revert`, which restores the SCB runtime without updating that cache, creating the opposite false refusal.
- **Fix sketch:** Read the predecessor declaration from its live SCB runtime; do not interpret missing retained text as no invariants.
- **Witness needed:** Installed-component/missing-cache drop case; reverted-runtime case where the retained successor declared an additional invariant. Assert precheck and actual swap agree.

### P2 — Preserve parse failures before emitting the new invariant verdict
- **File:** `crates/maos-bin/src/operator_door.rs:1777–1784`
- **Confidence:** 0.99
- **Body:** For a predecessor declaring an invariant and a schema-7 successor containing top-level `swap_invariants = false`, the helper suppresses the section parse failure (`admission.rs:697`) and treats the successor as declaring nothing. The new early return consequently reports `SwapInvariantDropped`, bypassing the coordinator parser entirely. Before this change, precheck returned `precheck_failed` for that malformed manifest; the corrected real upgrade parser still returns `UpgradeError::ManifestParse`. This masks the malformed-input diagnosis with a purported structural comparison.
- **Fix sketch:** Validate the successor’s invariant section before comparing, preserving parse errors instead of conflating them with absence.
- **Witness needed:** Predecessor with invariants plus scalar/malformed successor; assert a parse refusal rather than `SwapInvariantDropped`. Include the no-invariant predecessor control.

## Other review results

- **Actual enforcement:** Hot-swap and migrator both dispatch through `initiate_swap`, which compares the live bundle before hooks. Planned upgrades invoke that gate on every hop. The cold-swap correction compares before unload; its missing-predecessor path refuses closed.
- **Unreadable target:** Does **not** become safe in the examined network path. The vetting precondition reads the target first and returns an unsafe verdict on failure; the coordinator loader also rejects unreadable files. The evidence’s “falls through” description omits that earlier gate.
- **Wire consumer:** The new verdict reaches the CLI correctly: `map_response` prints the response body and honors `terminal_code: 2`; no enum dispatcher silently drops it.
- **Redaction:** Labelled colon hex survives the examined credential rules; typed errors retain raw names. No new redactor bypass exemption was introduced.

## Witness quality and accounting

- Saved logs substantiate the ordinary dropping/superset precheck vector, its red-before and comparison-removal plant, and the L2 scalar/cold-swap corrections.
- **Evidence caveat:** `plant-raw-name-under-hex-label-diff.txt` appends a literal formatted `0` through `hex.len() * 0`. Its 32-hex input becomes odd-length even without redaction. The observed `OddLength` failure therefore does not independently demonstrate that redaction caused the failure. Use a plant without that extra byte and inspect the persisted redaction marker.
- Correction evidence says **five plants**, but metadata records **four**.
- Accounting is internally consistent: bin **38602 → 38619 (+17)**; aggregate hardfail **203251 → 203268 (+17)**, retaining measured-plus-one. L2’s recorded kernel **27014/21181** remains within the documented ceilings. The pending change adds no kernel/domain variant or new public symbol. Independent authorization of the recorded adjudicated overdraft was not verifiable from these artifacts alone.

## Rejected candidates

1. Migrator or intermediate-hop bypass: both reach the coordinator gate.
2. Schema-6 successor preserving a schema-7 commitment: ignored declarations cause refusal, not bypass.
3. Cold-swap target pathname substitution: the door supplies its private snapshot.
4. Secret-shaped invariant names disappearing from the corrected abort reason: colon-separated encoding prevents the examined matches.
5. Coordinator halt-precheck mismatch, non-self-serialization, and designed below-schema degradation: pre-existing/deferred, not introduced here.

**Counts:** 2 P2 findings; 0 P0/P1/P3 findings. Evidence caveats above are non-blocking documentation/witness corrections.

**Overall correctness: incorrect.** The L2 corrections enforce the examined actual swap paths, but the pending precheck fix remains inconsistent with enforcement and masks malformed declarations. Confidence: **0.98**.

Read-only source/evidence review only; no builds, tests, plants, or hosted checks were executed.