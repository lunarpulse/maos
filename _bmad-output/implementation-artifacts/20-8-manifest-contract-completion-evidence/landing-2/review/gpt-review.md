## Review: `692b1e59181bd06069a87b9844742f30ed9c14e0`

**2 findings: 0 P0, 0 P1, 2 P2. Overall correctness: incorrect. Confidence: 0.97.**

Report returned here under the read-only review policy; no file written. No builds, tests, gates, or plants rerun.

### 1. [P2] Propagate malformed swap-declaration serialization errors

**Location:** `/home/lunarpulse/dev_ws/crates/maos-kernel-core/src/lifecycle/upgrade.rs:395–397`  
**Confidence:** 0.99

A schema-7 successor containing the top-level declaration `swap_invariants = false` is treated as having no declaration: `toml::to_string` rejects the scalar as a document, and `.ok()` silently converts that error into `None`. With an undeclared predecessor and a native successor such as Observer, whose constructor does not run `gate_manifest`, the upgrade can complete despite admission rejecting those same manifest bytes. This newly added extraction therefore bypasses declaration-shape validation.

**Exact fix:** Preserve absence/schema degradation separately from serialization failure. Serialize a present schema-7 section with `map_err(...)?`, then invoke `SwapInvariantsSection::from_toml_str`; never turn serialization errors into absence.

**Witness needed:** Upgrade an undeclared native predecessor to a schema-7 successor with scalar `swap_invariants`; require `UpgradeError::ManifestParse`, unchanged runtime, and zero swap hooks. Retain the schema-6 ignore witness.

### 2. [P2] Preserve dropped invariant identifiers through receipt redaction

**Location:** `/home/lunarpulse/dev_ws/crates/maos-kernel-core/src/hot_swap/coordinator.rs:205–207`  
**Confidence:** 0.99

Valid invariant names can contain 32 consecutive hexadecimal characters or credential-shaped prefixes such as `sk-queue`; unlike skill identifiers, `SwapInvariantsSection` does not reject those names. Interpolating such a name into the saga reason sends it through `TransparencyLogAdapter`’s credential redactor, which irreversibly replaces it in both `reason` and the compensation description. The returned typed error retains the name, but the durable `hot_swap.aborted` receipt loses it, and the lifecycle journal contains no fallback payload identifying the dropped invariant.

**Exact fix:** Encode the invariant in the saga reason using explicitly labelled, reversible colon-separated hexadecimal UTF-8 bytes; keep the raw name in `HotSwapError`. Do not weaken the redactor or introduce a secret-exempt field.

**Witness needed:** Drop a valid 32-hex invariant and a valid `sk-queue` invariant. Read persisted abort receipts and recover the exact identifier; also assert the raw typed error, unchanged predecessor, and zero hooks.

## Remaining judgments

- **Refusal ordering and state:** Correctly follows halt continuity and precedes `on_swap_out`/snapshot. Halt continuity is non-mutating; `RestorePredecessor` journals the refusal without replacing runtime state.
- **Sources and TOCTOU:** The production door snapshots target bytes; each migration hop creates another private snapshot used by construction and kernel parsing. The door’s process-wide upgrade lock protects the shared source slot. No concrete pathname-based declaration bypass found.
- **Carriage and semantics:** Admission and successor parsing populate the same SCB field. Exact-name superset checking handles arbitrary valid labels; duplicate names, uppercase, Unicode, and explicit empty lists are rejected by the shared validator. Absence imposes no obligations; successor-added invariants become the next predecessor’s obligations; schema-6 successors cannot satisfy schema-7 obligations.
- **Dispatch:** The new error reaches `UpgradeFailure.conflict`, `OperatorOutcome::Conflict`, and HTTP 409. Mid-plan failure preserves the typed mapping.
- **Spawned interplay:** A pre-warm declaration row is honestly labelled `declared`, not admitted. The drop callback supplies the refusal outcome structurally; the missing WASM-component witness remains explicitly unverified.
- **Pins/surface:** Committed evidence supports 27,009 physical / 21,178 tokei, inside 27,043 / 21,207 and 72 below the shared STOP. Only the `SpiritManifestBundle` struct surface hash changed.
- **Precheck:** A disclosed UX/planning gap, not an execution-safety bypass. Ownership by 20-8 and separately pricing the verdict/consumer changes are appropriate.
- **Evidence/claims:** All four plant diffs match their recorded behavioral assertion failures. Existing serde fallback and post-swap `expect` issues are outside this patch. Hosted AC7, spawned refusal, clean ABI verification, and incomplete verification are disclosed; `in-progress` is appropriate.