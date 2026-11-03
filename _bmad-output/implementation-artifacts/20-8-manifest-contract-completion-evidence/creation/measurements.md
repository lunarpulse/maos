# 20-8 creation — measurements (2026-11-03, HEAD `8c809f9c`)

No cargo ran (parent rule): no build, no test, no target directory was set or cleaned. No crate source in
`/home/lunarpulse/dev_ws` was modified. Each prototype was made on its own `git archive 8c809f9c` copy
under `/tmp/proto208/<name>/`. Every changed file was run through `rustfmt --edition 2021` (rustfmt 1.9.0;
the diffs show only the prototype's own lines). Each copy was measured from its own root with the
prebuilt gate binary:
`env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/tmp/proto208/<name>/xtask /mnt/build/cargo-maos/debug/xtask {check-kernel-baseline,kloc-check} --json`
(tokei inside `kloc-check`). Raw results are in `kloc-kernel-8c809f9c-and-prototypes.json`; diffs are in
`prototypes/`. Every diff passes `git apply --check` against the HEAD copy. Against the working tree, the
kernel-only and `man` diffs apply, while `gate` and `wit` conflict in `crates/maos-bin/src/admission.rs`
with the uncommitted 20-4 L1 edits. Prototypes were not compiled ([INFERENCE] that they build).
The copies were deleted after measurement.

## Instrument replication (untouched copy `base`)

| Figure | Method | Result | Pin |
|---|---|---|---|
| Kernel physical | `check-kernel-baseline --json` | **26841**, 95 files, set hash `db39bc4a…`, changed/added/removed `[]`, rc 0 | `xtask/kernel-core-baseline.toml` |
| Kernel tokei | `kloc-check --json` | **21042** | `xtask/kloc.toml:264` |
| Per-crate tokei | `kloc-check --json` (`passed: true`, `alarm: true`, `over_budget: []`, rc 0) | maos-manifest **4604** · maos-domain **10099** · maos-bin **34191** · maos-wasm-host **2412** · maos-spirit-abi **1042** · maos-registry **4248** · maos-skill **1234** · maos-iac **8270** · xtask **44406** · aggregate **192519** | `:415` · `:425` · `:517` · `:703` · `:270` (3000) · `:688` · `:693` · `:406` · `:339` · `_aggregate_hardfail` `:718` = 192520 |

Aggregate room: **1 line**. Every named crate is pinned exactly, except maos-spirit-abi (3000, numeric room). Under
D02 A every added line needs a grant.

## Kernel-touching options, priced by throwaway prototype (◆ FLAG-Winston)

| # | Prototype (D-row) | Diff (kernel-only) | Kernel physical | Kernel tokei | Also |
|---|---|---|---:|---:|---|
| KB | `SpiritManifestBundle.swap_invariants` + `Default` + `load_bundle_from_file` parses `[swap_invariants]` (D-20-8-07 B) | `kb-kernel-only.diff` | **+10** (26851) | **+8** (21050) | bin +1 (`bundle()`) |
| KB4 | all four declarations on the SCB + kernel parser (D-20-8-07 D) | `kb4-kernel-only.diff` | **+34** (26875) | **+32** (21074) | bin +4 |
| **KS = KB + the coordinator refusal (recommended)** | KB, plus `HotSwapCoordinator` refuses a successor that drops a predecessor-declared invariant. The refusal runs after halt continuity and before `on_swap_out`, with saga `RestorePredecessor` and `HotSwapError::SwapInvariantDropped` (D-20-8-07 A) | `ks-kernel-only.diff` | **+38** (26879) | **+33** (21075) | domain +2, bin +1 |
| K0 | kernel-free: declarations held in maos-bin only; 20-9 prices its own carriage (D-20-8-07 C) | — | 0 | 0 | — |

Changed kernel files: KB/KB4 `scheduler/control_block.rs`, `lifecycle/upgrade.rs`; KS adds
`hot_swap/coordinator.rs`. Each moves `src_lines`, the content hashes of the changed files and the set hash
(KS copy set hash `408c9896…`). KB's kernel parser does not yet mirror the gate's schema-7 rule. Mirroring
it adds [INFERENCE] +2…+4 inside the ×2 ceiling.

**ADR-069 / cumulative (tokei):** creation base 21042 already includes 20-7's landed K1w+K2r. The pending
granted rows are 20-1b +68, 20-4 +9 and 20-11 +6, so the expected cumulative is **21125**. With KS +33 it is
**21158**; with KS at its ×2 ceiling (+66) it is **21191**. Both are ≤ the **21209** cumulative STOP, leaving 51
and 18 lines respectively, and 842 / 809 below the 22000 alarm. KB gives 21133 (76 left); KB4 gives 21157.
Physical: 26841 + 78 + 13 + 10 + 38 = **26980** expected. The sum of standalone ceilings is
21042 + 136 + 18 + 12 + 66 = 21274, above the STOP. As ruled with 20-7/20-4, the measured cumulative STOP
governs, not the ceiling sum. No D-row for split/exchange/bump is raised. **Risk:** after KS, 20-9
(watchdog source hook, D07 suppression, lifecycle ABI) has ≤ 51 lines under the STOP and will almost
certainly need its own bump row.

## Out-of-kernel, priced by prototype

| # | Prototype | Diff | Measured (tokei) |
|---|---|---|---:|
| MAN | `SkillsSection{uses}`, `ExplanationShapeSection{required_fields}`, `IntentPromotionSetSection{allowed}` (+ `to_allowed_set()` → I13 `AllowedPromotionSet`), `SwapInvariantsSection{preserve}`. Each is `deny_unknown_fields` and goes through one bounded validator `declared_names` (1..=32 entries, 1..=128 bytes of `[a-z0-9_.:-]`, no duplicates). The four names are added to `POST_V1_SCHEMA_SECTIONS` and to the gate's ratified list. `MANIFEST_SCHEMA_VERSION` 6 → 7 (D-20-8-03 A) | `man-four-sections-schema7.diff` | **maos-manifest +103, xtask +4**, maos-spirit-abi 0 |
| GATE | `FR8_DECLARATIONS_SINCE_SCHEMA = 7` (manifest +1). `gate_manifest` reads the four sections only at schema ≥ 7 (`fr8_section`) into `GatedManifest`, plus `publisher: Option<[u8; 32]>`. `verify_installed` returns the verified publisher key and `InstalledSpirits::boot` sets it. New `GovernanceEventKind::ManifestDeclaration{spirit_id, version, manifest_sha256, publisher, declared}` is journaled after the granted admission. A journal failure rolls the load back (D-20-8-08 A, D-20-8-09 A) | `gate-admission-journal-signer.diff` (includes MAN) | **maos-bin +95, maos-domain +9, maos-manifest +1**, maos-registry 0 |
| WIT | `maos:spirit@2.3.0`: `interface skills { record queued; variant skill-error {capability-denied, refused(string), unavailable}; author; propose }`, imported by `world spirit`. Wire: `RecallRequest::{SkillAuthor, SkillPropose}`, `RecallReply::Skill`, `WireError::Refused`. Host: `skills::Host` on `HostState`, linker registration, `skills@` in the major-version check. Daemon: `MediatedLogRecall.skill`, two `serve_wire` arms → 18-7's `SkillAuthorPort`, `skill_reply`. Session: `SessionDeps.skill`, `SessionLaunch.scopes`. `main.rs` passes 18-7's binder to both `SessionDeps` sites (D-20-8-10 A, D-20-8-11 A) | `wit-skills-import-bridge.diff` (includes MAN + GATE) | **maos-wasm-host +51, maos-bin +60** (over GATE) |
| **Measured total (recommended)** | MAN + GATE + WIT + KS | — | manifest **+104**, bin **+156**, wasm-host **+51**, domain **+11**, xtask **+4**, kernel **+33** (physical +38); aggregate **+359** |

## Not measured here (owed to the dev pass; a figure above the ruled envelope is STOP + D-row)

1. Compilation of every prototype (`cargo check --locked -p maos-manifest -p maos-domain -p maos-bin -p maos-wasm-host -p maos-kernel-core --features maos-bin/wasm-host`). In particular: the `bindgen!` names generated for `interface skills` (`maos::spirit::skills::{Host, Queued, SkillError}`); that `researcher::SkillAuthorPort` is reachable from `recall_mediation.rs` under `wasm-host`; that `scheduler.scbs()` holds the spawned SCB the port reads (`skill_authoring.rs:104-109`).
2. Whether every `SpiritManifestBundle { .. }` literal in tests uses `..Default::default()` (36 literals under `crates/*/tests`, `spirits`). Explicit ones need the new field (uncharged).
3. `[skills] uses` admission check: each id is discovered, `Admitted`, and its `required_capabilities` ⊆ the Spirit's declared actions (D-20-8-04 A). Forecast maos-bin +30…+60, maos-skill 0…+20 [INFERENCE].
4. I13 consumer check at IAC delivery of a `TaskAssign.prior_distillate_ref` to a recipient with a declared set, plus the per-recipient registration at admission (D-20-8-06 A). Forecast maos-iac +40…+80 [INFERENCE] (pin 8270, zero headroom), maos-domain +2…+4 for `IacBusError::IntentPromotionDenied` (`crates/maos-domain/src/iac_bus_types.rs:15`); maos-bin registration inside item 3's range.
5. Typed refusal codes and door mapping of `SwapInvariantDropped` (HTTP 409) and the skills refusals. Forecast maos-bin +10…+28 [INFERENCE].
6. A Rust guest exercising `skills.author`/`propose` (extension of `guests/recall-probe` or a new `guests/skills-probe`). Guests are charged to maos-wasm-host (F11). Forecast +40…+80, plus in-file `recall_wire` vectors +20…+40 [INFERENCE].
7. `build-test-fixtures` for the new guest and re-issued equivalence fixtures (`tests/fixtures/wasm/equiv-fixtures.provenance.toml`; every WIT byte feeds `source_sha256`, `wit/spirit.wit:21-23`). Forecast xtask +5…+15 [INFERENCE]. Fixture bytes are uncharged.
8. TS SDK (`templates/spirit-ts`, `examples/example-spirit-ts`: `wit/spirit.wit` copies, `src/index.ts`, README), docs (`docs-site/docs/cookbook/wasm-component-spirit.md` en + ko, `docs-site/docs/manifest/v7.md` en + ko), `gen-abi-docs` output for the maos-spirit-abi constant. All uncharged; componentize-js runs on the hosted leg.
9. The hosted T2 legs (`wasm-host-tests`, `check-wasm-form-equiv`, `epic-20-exit`) on the final tree.
