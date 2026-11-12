> Review of commit 998b36e6 (20-13 L2 kernel landing, AC7 KT3). Reviewer model family: GPT (agent `Review2013K`). Persisted verbatim by Main's delegate (`Rec2013`); text below is the agent's final message, unedited.

## Review result: 0 findings

**P0: 0 · P1: 0 · P2: 0 · P3: 0**  
**Overall correctness: correct. Confidence: 0.92.**  
Code review passes; this is **not approval to mark the story complete**.

### Headlines

- **Selected instance reaches kernel admission.** `main.rs:2240–2260` passes clones of the same `crypto` Arc to the capability registry and security manager. Both T3 verification branches consume `self.crypto`; the 7.5a smoke dispatcher also forwards that Arc. No patch-introduced production ring fallback found.
- **KT3 cutover is complete.** `Default` is removed and callers migrate to the required constructor. Manual `Debug` never formats the provider; its remaining configuration contains a path and **public** trust anchor, not secret key material. No new production serde/toml `unwrap`/`expect` introduced.
- **Gate tightening is sound.** Removing the custom impl visitor restores normal recursive visitation, not a traversal gap. Kernel admission loses its exemption. Negative controls cover direct/default/struct/alias/type/macro constructions; the recorded live plant produces the expected gate failure. The preserved `cfg(test)` skip excludes genuinely test-only code. This remains a syntax-based guard—not proof against every conceivable indirect factory—but no weakening is introduced.
- **Surface refresh is surgical.** The baseline diff changes only `SecurityManagerAdapter`’s hash, matching `kt3` authority. Before-refresh evidence flags that symbol alone; after-refresh evidence passes. This gate does not independently certify constructor/trait-implementation compatibility, so it must not substitute for `abi-diff`.
- **Witnesses are meaningful.** Adapting the helper to the old constructor is a valid pre-change API accommodation: assertions and admission inputs remain equivalent. The unchanged-final-test plant strengthens the proof: three routing failures, two passing controls, plus structural rejection. Logs support the claimed mutation; they do not independently attest the copy-back procedure.
- **Accounting agrees with evidence.** Kernel **27,044 physical / 21,202 tokei**, exactly **+1/−1**, below STOP **22,000**; xtask **−24**. Bin **38,556 → 38,563 (+7)** is openly recorded as awaiting Main adjudication. A re-pinned green gate is not that authorization.

### Outstanding completion requirements

- AC7.2’s **bin-side recording proof of kernel admission** remains absent; the existing Worker recording test is not a substitute.
- Hosted T3/container admission, final-tree hosted discipline and `abi-diff` remain unverified. No `prepush` evidence is supplied.
- The recorded bin/escape suite is **not wholly green**: the million-row erasure failure remains after the dependency-corrected rerun.

Documentation accurately describes the implemented routing and retains the FIPS boundary. Minor evidence precision: the surface reports contain **401**, not 400, items.

Read-only review; no builds, tests, plants or edits performed. Report returned here rather than written to `gpt-review.md`.
