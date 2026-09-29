---
baseline_commit: "**`4f677bc1`** (main; 17-6 `done`, 17-3a `done`). Authored 2026-09-29 from six read-only scouts (WIT/bridge/runner/fixtures; spike Q1–Q5 artifacts; manifest + maos-bin admission; registry admission; TS toolchain + templates + docs; governance/gates/history) plus direct reads and measurements. Measured here at HEAD: `cargo run -q --locked -p xtask -- check-kernel-baseline` → `PASSED (maos-kernel-core/src = 25032 lines, 98 files, pinned 25032)`; `kloc-check --json` → `passed: true, alarm: true, aggregate: 168381` (hardfail 170884 → **2503 free**); per crate (tokei code / ceiling): `maos-bin` 23423/23423 (**zero**), `xtask` 44123/44123 (**zero**), `maos-kernel-core` 19309/19309 (untouched), `maos-manifest` 4214/4474, `maos-registry` 3515/4282, `maos-wasm-host` 1106/1577, `maos-spirit-abi` 1038/3000, `maos-spirit-sdk` 846/3000, `maos-host` 39, `maos-cli` 6106, `maos-domain` 9270. **Cites are SYMBOL-first, line second**, re-measured at `4f677bc1`; they supersede the epic's cites where they differ (§Premise corrections)."
depends_on: "**`17-3a-wasm-recall-and-componentize-spike` (`done`)** — Q1 (componentize-js recipe, R17-38), Q2 (corpus/provenance tax, R17-40), Q3 (lossless bridge, NOT additive — R17-41), Q5 (compatibility matrix, R17-43). Evidence spent: `spikes/story-17-3a-wasm-recall/bridge/{q3-frame-bridge-report.md,src/bridge.rs.txt,src/main.rs.txt}`, `wit-1.1-frame/spirit.wit`, `ts-guest/`, `q1-driver/q1-componentize-js-report.md`, `evidence/q1-*.log`, `q2-fixture-rebuild.log`, `q3-frame-bridge.log`, `q5-*.log`. **ADR-060** (ACCEPTED; `docs/adr/ADR-060-spirit-forms-by-trust-tier.md`, clauses 1–4 `:67-78`; Gate line `:3` names this story; `xtask/tests/decision_adrs_and_provisioning.rs:20` maps ADR-060 → this key). **ADR-031** (WASM runner, T2 process boundary, unchanged ADR-032 wire — binding). **ADR-045 §4** (ABI-extension proposals, `xtask/abi-ratifications.toml`). **PRD `DELTA-2026-09-27` (R17-57, operator-resolved):** WASM-component Spirits run at **T2 + WIT**; T4 is the reserved WASM *tool* sandbox, refused at admission. Does NOT depend on `17-6` (the T2 route) — this story spawns nothing."
blocks: "**`17-3c-wasm-spirit-on-the-bus-under-t2`** (needs this contract, the `wasm-component` form and the `WasmLaunchNotBuilt` refusal it replaces), **`17-3d-wasm-log-recall`** (needs the lossless frames for FR29's consent clause) and **`20-1-registry-client-install-verb-vetter-and-yank`** (AC4 + epic-20 exit lines 7–9: the form token, registry admission by form, the componentize-js artifact, the typed engine-off refusal — epic §R7 R17-51/R17-52). The `maos:spirit@2.0.0` break must land **before 20-1 publishes any component** (no third-party component exists anywhere today — the engine is off in every published binary, `RELEASE-HOLDS.md:28,32-38`)."
spec_alignment: "Rule 9 applies mid-epic. Creating this story disproved or tightened premises in all five ACs (§Premise corrections P1–P29). The ACs are the epic's five (`epic-17…:103-107`), tightened; none is added. Forks are decided in §Decisions (rule 7). **Filed for the dev pass to apply in the landing commit** (§Epic OLD→NEW edits): exit line 2's provenance re-point to 17-3c; R17-51's routing of R17-39(a)/(d) to 17-3b (they sit on lines this story rewrites — D-17-3b-C); 17-5 AC5's pin count (7 → 5); 20-1 AC4's `trust_tier` spelling; `requirements-inventory.md:568`; the `maos-registry` kloc ledger; `deferred-work.md:1050`."
split_from: "Created by epic §R7 (R17-51, 2026-09-26) from the retired key `17-3b-wasm-third-party-form-with-log-recall`; its execution half is `17-3c`, its recall half `17-3d`. **Sizing: WHOLE** — the WIT break, the four re-issued components, the bridge, the corpus counts and the equivalence harness go red or green together (`check-wasm-form-equiv` is Blocking at HEAD through `dev_enforced_red_blocks`, R17-29); the manifest token without maos-bin's fork turns every `wasm-component` manifest into `UnknownClass`; the registry refusal without the manifest token has nothing to admit."
kernel_grant: "**NONE — kernel-Δ 0.** Nothing under `crates/maos-kernel-core/src` changes. `ClassSection` gains a field, and its kernel-core struct-literal sites are all in `tests/` and `benches/` (`abi_stability_admit.rs:68`, `hot_swap_cross_major_migration.rs:71,:358`, `on_revocation_three_actions.rs:111`, `operator_posture_ceiling_16_6.rs:61`, `provider_switched_journal.rs:86`, `revocation_applier_pipeline.rs:55`, `sandbox_admission.rs:21`, `benches/revocation_propagation_p99.rs:53`) — outside `check-kernel-baseline`'s `src` scope. `check-kernel-baseline` must still read **25032** at landing; `kloc.toml:253` and `recovery_lane_ceiling_rule.rs:268` are not touched."
kloc_grant: "Measured at `4f677bc1` (above). Expected, [INFERENCE — not prototyped; kernel-Δ 0 so E16-A7's prototype rule does not bind, but every figure is re-measured at landing after `cargo fmt --all`, tokei 14.0.0]: `maos-wasm-host` +150…+260 (471 free — absorbs; ledger `kloc.toml:661` already books 17-3b +200–400); `maos-manifest` +50…+90 (260 free — absorbs; ledger `:395` books Epic 17 wasm-component +80–200); `maos-registry` +40…+80 (767 free — absorbs; ledger `:647` books 17-4/20-1 only, so the ledger comment is re-booked to name 17-3b, R17-20); `maos-spirit-abi` +2 (1962 free); **`maos-bin` +40…+80 at ZERO headroom** (`kloc.toml:483` = 23423) and **`xtask` net Δ at ZERO headroom** (`:326` = 44123; this story both adds `check_equiv_fixture_provenance` lines and deletes dead `templates_regen` lines, so the net may be ≤ 0) — each needs an **operator-authorized, exact formatted-measured raise in the same commit as its driver**, the raise comment naming the full key `17-3b-wasm-form-admission-and-contract` (`d11_xtask_ceiling_ratchet.rs:164` parses only `\\d+-\\d+` tokens, so `17-3b` alone resolves to no key). Aggregate ask +280…+510 against 2503 free (alarm 158608 already fires — never silenced). `crates/*/tests/` and `xtask/tests/` are uncharged; inline `#[cfg(test)]` is charged. **Operator authorization for the two zero-headroom raises GRANTED 2026-09-29 (Lunarpulse: *approved — apply the consequential changes to the tests, including the xtasks*)**: exact formatted-measured raises for `maos-bin` and, if its net is positive, `xtask`, each in the same commit as its driver, citing the full key; every test and xtask gate the story's changes break is updated in the same pass (§Dev Notes → *Consequential test and xtask changes*)."
model: "frontier-class allowlist {opus-4-6, opus-4-7, opus-4-8, gpt-5.5, gpt-5.6, glm-5.1, glm-5.2, glm-5.3, opus-5, equiv}. `FRONTIER_FAMILIES` (`xtask/src/check_dev_model_tier.rs`) is the machine-checked set; `equiv` is prose, NOT a match token. Keep the literal `allowlist {` — it is a NEGATIVE marker: `check_dev_model_used_populated.rs:302` treats any line containing it as boilerplate and SKIPS it."
review: "§A6 full-layer net **BINDING and NON-DEGRADABLE** (◆). Layers: Blind + Edge Case + Acceptance + Test-Infra + **Security** (an admission control, a registry refusal, and a bridge that lifts guest-supplied bytes into kernel frame types — hostile input) + **non-author runtime re-execution** (a non-author re-runs AC1's legs locally and reads the recorded CI run's public annotations). E15-A6 binds every probe by name: *does this test read the tree/runtime, or only what it set itself?* — and 17-6 RT-10: *show the planted diff and a warning-free build, or the fault was not planted.*"
---

# 17-3b — The third-party WASM form: its contract, its admission and its TypeScript toolchain ◆

Status: ready-for-dev

> **The capability:** *A TypeScript author runs the scaffold, builds, and gets a real `maos:spirit@2.0.0` component
> that the release `maos-wasm-runner` drives through `on-start` and one ADR-032 frame — with intent, consent envelope,
> lineage and typed scope carried losslessly both ways. The same manifest is refused by name, never as "unknown
> class": `wasm_engine_off` in every published (default) binary, `wasm_launch_not_built` in a `wasm-host` build until
> 17-3c ships the launch. The registry refuses a first-party `rust-inproc` package from anyone, at every tier, and
> admits a `wasm-component` package at every tier with a T2 floor. The contract is broken exactly once, recorded as a
> ratified ABI-extension proposal, before anyone depends on it.*

**Closes:** FR33 for TypeScript (Fork C — `prd/project-scoping-phased-development.md:18`) · FR5-by-form, **admission
half** · the form×tier policy FR36 needs in 20-1 (ADR-060 clauses 1–2) · FR62(b)'s first WIT ABI-extension record
(`requirements-inventory.md:568`) · R17-18 · R17-28 · R17-29 (kept green) · R17-30 · R17-31 · R17-38 (toolchain) ·
R17-39(a)(c)(d) · R17-40 · R17-41 · R17-43 (the `@2.0.0` break half) · R17-49 · R17-52 · R17-53 · R17-57 (ABI twin doc) ·
R17-58 · R17-59 · `deferred-work.md:1050`.

**Does NOT close, and says so:** a WASM Spirit **running** on the bus, its launch under T2 through `spawn_and_bridge`,
the spawned-class session, the `maos run` SIGSYS proof, `memfd_create`, the kind-8 row for a production T2 kill, and
moving conformance out of the unsandboxed daemon — all **`17-3c`** (R17-24, R17-34 for the cookbook, R17-47, R17-48,
R17-54, R17-55, R17-56); R17-39(b) the guest diagnostics channel (`console.log` is discarded — documented here,
designed in **`17-3c`**); `log.recall` for WASM, the `@2.1.0` recall import, the open `frame-kind-label`, FR29's
consent clause — **`17-3d`** (R17-23, R17-42, R17-43's label half); publishing and installing the component —
**`20-1`**; a Transparency-Log emitter for `AbiExtension` governance events (no production trigger exists — D-17-3b-F).

## Story

As **a third-party Spirit author writing in TypeScript (and the operator who will admit my package)**,
I want **a pinned scaffold that builds a real `maos:spirit@2.0.0` component, a WIT contract that carries every frame
field without loss, and admission that knows my form — refusing it by name until the engine and launch ship, and
refusing first-party forms from any registry**,
so that **20-1 can publish and install a component against a contract that will not break under it, and 17-3c can
launch it without re-deciding the form, the manifest shape or the frame projection.**

## Premise corrections at HEAD `4f677bc1` (measured at story creation)

| # | Premise (epic `…:97-107`, §R4–§R7) | Measured now |
|---|---|---|
| P1 | AC2: *"the spike's 1,025-byte round trip is the oracle, with the nested `PriorDistillateRef.intent_lineage` and `DecisionDispatch.working_memory_digest_refs` included"* | **FALSE as an oracle for the nested fields.** The probe frame (`spikes/…/bridge/src/main.rs.txt:32-83`) is a TaskAssign with `prior_distillate_ref: None` (`:68`) and no DecisionDispatch; the spike bridge still defaults both (`bridge.rs.txt:338`, `:350`); the spike WIT has no field for either (`wit-1.1-frame/spirit.wit:70-73`, `:130-133`). → D-17-3b-A adds two record fields; the new oracle frame's byte count is **measured at landing** (it will not be 1,025). |
| P2 | AC2 cites `frame_bridge.rs:9-21, :216, :302-306, :315, :395-397, :427-431` | All HOLD. **Missing:** `:327` (`working_memory_digest_refs: Default::default()`) and `lower` `:403-414` (infallible, 8 fields). Silent `_ =>` backstops for `#[non_exhaustive]` `PostureHint` (`:147`) and `RuptureReason` (`:196`) map unknown values to `Cautious`/`RecipientUnloaded` — the same lossy class. |
| P3 | The spike WIT is the `@2.0.0` to port | **No.** `wit-1.1-frame/spirit.wit` is `package maos:spirit@1.1.0` (`:9`) and carries `interface recall` + `import recall` (`:267`, `:372`) — 17-3d's. Frames-only port: records **16 → 20** (spike 25 − recall's 5), not 25. |
| P4 | *"The 4 provenance-pinned components are re-issued with the build toolchain recorded"* (as if the spike did it) | **The spike only rebuilt them, and they did not reproduce** (`evidence/q2-fixture-rebuild.log:100-104`, `reproducible=False` ×4; 69.7 KB vs 63.1 KB; the pinned echo imports `wasi@0.2.6`, spike builds `@0.2.9`). `tests/fixtures/wasm/equiv-fixtures.provenance.toml` has **no toolchain field**. The *"scoped-nightly byte-rebuild job"* is phantom at `xtask/src/check_equiv_fixture_provenance.rs:15-17` **and** `:104-105`, and at `equiv-fixtures.provenance.toml:11-16`; no workflow builds any `.wasm`. Guests are plain `cdylib` + `wit_bindgen` 0.44.0 built with `cargo build --target wasm32-wasip2` (not cargo-component, contra `11-1a…md:192`). |
| P5 | `wit_corpus.rs:113-160` totals, field table `:181-186` | Totals HOLD (records literal `16` at `:158`; frame-kind 17 `:117`, origin 4 `:127`, posture 3 `:135`, rupture 5 `:140`, payload 11 `:148`). Field table **MOVED → `:163-180`** (`("iac-frame", 8)` `:179`, prior-distillate-ref 2 `:167`, decision-dispatch-body 2 `:170`); `:181-187` is the assert loop. No count gate exists for a new enum/variant. |
| P6 | *"The runner refuses a `@1.x` component with a typed exit"* — some version check exists to extend | **None exists.** `Spirit::instantiate` (`runner.rs:170-174`) maps every failure — link error, type mismatch, **and an instantiate-time `Trap::OutOfFuel`** (R17-39a) — to `InvalidComponent` exit 3. `RunnerExit` (`:52-62`) = 0/1/3/4; **2 is unused**. `e2e_roundtrip.rs:258-268` pins exit 3 for invalid bytes. The `@2`-host × `@1`-guest error has **never been measured** (the spike measured 1.0↔1.1 only). |
| P7 | R17-51 routes R17-39(a) and (d) to 17-3c | Both sit on lines this story rewrites: (a) is the `runner.rs:170-174` map_err this story splits for the version refusal; (d) is `wit/spirit.wit:213-218`, and **any** later byte change to the WIT drifts all 4 `source_sha256` and forces a second 4-component re-issue. → D-17-3b-C folds both in (rule 10(a)); OLD→NEW filed. |
| P8 | AC3 cites `admission.rs:454-459` (gate 5), doc `:218-221`; R17-1 `KNOWN_CLASS_NAMES :317`; door site `main.rs:8603`; `Cargo.toml:30` | **MOVED:** gate 4 `:448-456`, gate 5 `:458-463`, gate 6 `:465-482` (17-6's `RustInprocRequiresT0` `:468-474`), gate 7 `:484-505`; `gate_manifest` `:413`; the "Epic 17 `wasm-component`" doc is on `UnknownClass` `:214-216` (`:218-221` is now `RustInprocRequiresT0`); `KNOWN_CLASS_NAMES` `:321`; door site `main.rs:8606` whose `.expect` text (`:8609-8610`) differs from the other two (`:4455-4456`, `:4881-4882`); `wasm-host` feature `crates/maos-bin/Cargo.toml:29`. `AdmissionRefusal` (`:201-244`) is **not** `#[non_exhaustive]`; its only exhaustive matches are `code()` `:248-268` and `Display` `:275-313`; the door maps with a wildcard (`operator_door.rs:628-631` → HTTP 400). |
| P9 | AC3: *"T4 … is refused"* needs a new refusal | **Already refused, at parse:** `SandboxTier::try_from_manifest_str` accepts `T0`–`T3` only (`crates/maos-domain/src/invariants/i9.rs:107-115`), so `tier = "T4"` surfaces as gate 6 `SectionParse{sandbox}` / `manifest_section_invalid`. What is missing is a typed refusal of **T0/T1/T3** on `wasm-component` (D-17-3b-I). `[sandbox]` without `tier` defaults to T2 (`manifest.rs:152`, `:158-160`). |
| P10 | Adding `[class].artifact` is a parser change | `ClassSection` (`manifest.rs:222-231`) is a plain `pub` struct with no `Default` and no `#[non_exhaustive]`; **13 struct-literal sites** outside `maos-manifest` break (the validator's own constructor `manifest.rs:427` too): `main.rs:6669`, `:6770`, `:12983`; 9 kernel-core test/bench sites (the 8 in `kernel_grant` + one more — re-grep `ClassSection {`); `xtask/tests/story_10_4a_ac1_proven_red.rs:1013`. None in kernel-core `src`. |
| P11 | AC3 *"`manifest_field_coverage` rows (`discipline.yml:632-647`)"* | Rows live in `crates/maos-kernel-core/tests/manifest_field_coverage.rs` `MANIFEST_FIELDS` `:33-74` (class rows `:34-41`); each field needs 3 fixture files `tests/fixtures/manifest/class/{well-formed,malformed-rejected,edge-case}/<field>.toml` (walker `:88-110`, orphans fail `:157-160`). The CI job is `discipline.yml:729-745`. The `(main_parsers, admission_parsers) == (3, 1)` pin (`:276-280`) counts **substrings** of `include_str!(admission.rs)`, comments included (`:259-280`) — the text `CapabilitiesRequired::from_toml_str` must not appear anywhere new in `admission.rs`. |
| P12 | The 4 → 5 bump touches `lib.rs:33`, `:112`, `:114` + `abi-ratifications.toml` | Also: doctest rows `lib.rs:156-160`; tripwire `crates/maos-spirit-abi/tests/manifest_n_minus_1_test.rs:103-107`; `manifest.rs:2576-2586` uses **5** as its above-max probe (becomes valid — move to 6, fix comment `:2579`); `gen-abi-docs --check` (the `MANIFEST_SCHEMA_VERSION = 4` header in every `docs-site/abi/v1/*.md`, `constants.md:84,:144`, `index.md:42`); `stability-matrix --check` (STABILITY.md is generated); per-version pages `docs-site/docs/manifest/v4.md` + `migrate/v3-to-v4.md` have a `v5`/`v4-to-v5` precedent (13.5d, `279510b0`) + ko twins. One hardcoded compare `degrade_for_schema_version` `< 4` (`manifest.rs:499-505`) is invisible to the xtask scanner (it matches the token `manifest_schema_version` only). **17-1 has not landed the bump** (`lib.rs:114` = 4) → it lands here (R17-51). |
| P13 | AC4 rule 11(a): *"the production caller is 20-1's install (named, not done)"* | **FALSE.** `maosctl import` already calls both entry points: `dispatch_import` (`crates/maos-cli/src/subcommands.rs:1228`) → `admit_spirit_with_attestation` `:1359`, `admit_spirit` `:1368`. AC4 changes a shipped verb; its proof runs through it. |
| P14 | AC4 names `admit_spirit` only | `admit_spirit_with_attestation` (`crates/maos-registry/src/admission.rs:278-338`) re-parses the tier and **does not delegate** on the promoted `PublicVetted` path (`:311-339`) — a check written only in `admit_spirit` is bypassed by an attested package. Both preludes carry the FKCS refusal (`:183-188`, `:286-291`); the form refusal sits beside it in both. |
| P15 | *"reads `forms` from `SignedPackage.manifest_toml` through `ClassSection::from_toml_str`"* | `from_toml_str` (`manifest.rs:234-238`) takes the **`[class]` section body**, not a document; `maos-registry` has **no `toml` dependency** (it already depends on `maos-manifest`, `Cargo.toml:12` — no new edge, no cycle). Precedent whole-document reader: `ModelProvenanceSection::from_manifest_toml` (`manifest.rs:1855`, called at registry `:403`). |
| P16 | Every registry package has a `[class]` | **None of the fixtures does.** Every `admit_spirit` fixture is legacy `[spirit]`-shaped or bare keys: inline tests (`pkg_with_tier` `:554-568`, `:636`, `:706`, `signed_pkg_with` `:757`, `:836`, `:1103`), `tests/end_to_end_test.rs` (8 sites), `runtime_drift_test.rs:19`, `vetting_attestation_gate.rs:54`, `import_air_gap_test.rs:55,200`, `registry_roundtrip_test.rs` over 19 JSON fixtures (count asserts ≥10/≥8), the maos-fkcs proxies (`crates/maos-fkcs/src/lib.rs:622`) and the maos-bin smoke arms (`main.rs:7308`, `:7375`, `:7464`, `:12312`, `:12583`, `:13147` — all `[spirit]`, unaffected). Refusing a missing `[class]` reds all of them. |
| P17 | Admission's sandbox floor is form-neutral | `verify_public_untrusted_baseline` returns **T3** when `t3_for_public_untrusted` (`:171-175`), else T0; Org/Local return T0 (`:225`, `:240`, `:254`). A `wasm-component`'s only boundary is T2 + WIT (PRD delta; ADR-031) — T0 is false and T3 (podman) has no WASM path. ADR-060 `:116-118`: *"FR5's … strictest-of-three mechanism is amended to be form-aware"*. → D-17-3b-J. |
| P18 | Editing registry `admission.rs` is ordinary | **`check-fkcs` blocks on any byte edit** — and since 2026-09-29 there is **no hold**. `xtask/fkcs-baseline.toml` `[admission_baseline]` pins the SHA-256 of `crates/maos-registry/src/admission.rs` + `crates/maos-skill/src/admission.rs` (framing `xtask/src/check_fkcs.rs` `admission_content_hash`). Decision **D8** was discharged on 2026-09-29 by operator directive, before this story starts: 13.4's change reviewed CONFORMANT, the pre-pin fixes landed (one shared `frozen_surface_gate` called first by both entry points), re-pinned `dfbbf748…` → `650f13b9…`, the hold machinery deleted; `check-fkcs --json` → `oracle_green: true` (`epic-14-preflight-decisions.md` §D8 ruling). This story's AC4 edit therefore reds `admission-path-unmodified` until it **re-pins its own reviewed diff in the same commit** — the first story under that rule. → D-17-3b-J(e). |
| P19 | ADR-060's gate is prose | `xtask/tests/decision_adrs_and_provisioning.rs` `validate_anchors` (`:262-293`) **requires** `manifest.rs` to contain `matches!(f.as_str(), "rust-inproc" \| "subprocess")` and **requires** registry `admission.rs` NOT to contain `class.forms`; fixture `:941-947`, context string `:898`, planted-red vectors `:1088-1091`, `:1150-1154`. AC3 and AC4 invert both anchors (raw substring checks, `:178-219` — the rewritten Context must keep the tokens `class.forms` and `admit_spirit`, and after the flip the registry code must literally contain `class.forms`, e.g. in the `form_policy` doc); they, the fixture, the vectors and the ADR's Context (`:12-62` — *"`wasm-component` is not yet accepted"*, *"`admit_spirit` … does not read `class.forms`"*) move together. Runs in `workspace-test-suite`. |
| P20 | R17-58 / epic-20 20-1 AC4: set the example's `trust_tier = "public_untrusted"` | **Refused.** `RawClassSection::validate` accepts hyphenated tiers only (`manifest.rs:403-411`), while `parse_manifest_trust_tier` also accepts underscores (`:61-62`). Inside `[class]`, `public_untrusted` fails gate 4 and AC4's `ClassSection` read. `maos-spirit publish --tier=public_untrusted` compares parsed enums (`publish.rs:111-118`), so `trust_tier = "public-untrusted"` works with it. OLD→NEW filed on 20-1. |
| P21 | AC5: two README sites say *"not a kernel runtime"* | **Three.** `sdks/spirit-ts/README.md:15` (verbatim), `examples/example-spirit-ts/README.md:20-25`, and **`templates/spirit-ts/README.md:20-25`** — templates-regen **skips READMEs** in check mode (`xtask/src/templates_regen.rs:96-98`) and never overwrites an existing example README (`:117-122`), so each is edited by hand. |
| P22 | *"`templates-regen --check`, `discipline.yml:1077-1078`"*; *"`example-spirit-ts-tests` leaves EOL Node 20 (`:1088`)"* | Both HOLD; the drift job's key is **`example-spirit-drift`** (`:1066-1078`). The other Node-20 pins (`check-ko-coverage` `:3035-3037`, `docs-site.yml:38`) are not componentize consumers — untouched (17-3a Q1 report). |
| P23 | The TS scaffold keeps its `[capabilities.required]` | `provider.complete = ["anthropic.claude-3-haiku-20240307"]` (`templates/spirit-ts/manifest.toml:11-12`, example same) is a retired pin **and** a capability a WASM Spirit cannot exercise (the world imports nothing, `wit/spirit.wit:219-230`). `cross_template_field_consistency` (`templates_regen.rs:404-420`) requires it; **17-5 AC5 counts both TS lines among its "exactly seven retired pins"** (`epic-17…:153`). → D-17-3b-K; OLD→NEW on 17-5. |
| P24 | R17-49: *"documents further fields the parser refuses [INFERENCE for all but `[resources]`]"* | Now read: `manifest-fields.md`'s block also teaches `posture default = "supervised"` (refused — `Posture` accepts `cautious\|assistive\|autonomous-with-halt\|autonomous`, `manifest.rs:643-653`), epistemic action `mute` (not an `EpistemicAction`, `:891-896`), trust tiers `community\|audited` (`:403-411`); `compliance-claim.md:94` teaches `trust_tier = "audited"`. Its prose says MAX schema *"currently 3"* (it is 4 → 5). The 17-6 test (`crates/maos-manifest/tests/cookbook_sections_17_6.rs:27-39`) covers en only, the first block only, `[sandbox]`+`[resources]` only, and its `section()` cannot read `[[…]]` or dotted tables. |
| P25 | Exit line 2 is 17-3c's (R7) | Its provenance bullet (`epic-17…:30`) names only **`17-3b AC1`**, so `check-exit-commands`' owner resolution (`check_exit_commands.rs:777-839`) attributes unresolved line-2 tokens to 17-3b — which reds (*owner done*) the moment this story is `done`. Re-point to `17-3c AC1` in the landing commit. |
| P26 | `deferred-work.md:1050` (R17-49) is closed by doing the work | `check-dev-record-completeness`'s owner sweep marks a row owned by a `done` story **STALE** (`check_dev_record_completeness.rs:265-266`, `:632-637`) unless its section heading is closed (`heading_is_closed` `:93-105`). Close it in the landing commit. |
| P27 | *"the **release** `maos-wasm-runner`"* | No release workflow builds the runner (`release.yml:73` builds `maos-bin` + `maos-cli`, default features). "Release" = a `--release`-profile CI build (debug `Component::new` 10,506.735 ms > `COMPILE_TIMEOUT` 10 s, `runner.rs:140` → exit 3; release 792.820 ms — `q1-componentize-js-report.md`). No CI job builds `maos-bin --features wasm-host` (`grep` of `.github/workflows` empty); `epic-17-exit` does not exist (17-1 AC1 creates it). |
| P28 | FR62(b): *"the WIT extension is the first emitted event"*; *"`abi-ratifications.toml` covers the Rust-side types"* | `requirements-inventory.md:568` already reads *"the `maos:spirit@2.0.0` break is the first emitted event"*. **No production code emits `GovernanceEventKind::AbiExtension`** (`crates/maos-domain/src/governance.rs:161`; only reader `maos-audit/src/lib.rs:526-565`). `abi-diff`/`check-abi-ratification` see `maos-spirit-abi` only (`xtask/src/abi_diff.rs:1,8`); `abi-diff` runs in CI (`discipline.yml:237-249`), `check-abi-ratification` has **no CI job**; precedent 9.3b recorded its FR62 proposal as an `abi-ratifications.toml` entry with no TL emission. → D-17-3b-F. |
| P29 | Kloc cites (epic `:42`) | All MOVED: `maos-bin` `:483` = 23423 (measured 23423, zero); `xtask` `:326` = 44123 (zero); `maos-manifest` `:396` = 4474 (4214); `maos-registry` `:648` = 4282 (3515; ledger `:647` books 17-4/20-1, R17-20); `maos-wasm-host` `:662` = 1577 (1106; ledger `:661` books 17-3b); alarm `:664`, hardfail `:665` = 170884; aggregate **168381**. `SCANNED_SOURCE_FILES` is `cohort_daemon_smoke_13_5c.rs:922` (`[_; 24]`, assert `:1026-1030`) — this story adds **no** maos-bin `src` module. |

## Decisions (rule 7 — one per fork)

| # | Fork | Decision |
|---|---|---|
| D-17-3b-A | The `@2.0.0` WIT (AC2, R17-41 option (a)) | `package maos:spirit@2.0.0;`. `interface frames` = the spike's `wit-1.1-frame` frames projection **without** `interface recall`/`import recall` (17-3d's `@2.1.0`), **plus** the two nested fields P1 found missing: `prior-distillate-ref { digest-frame-id, distillation-depth, intent-lineage: list<string> }` and `decision-dispatch-body { decision-id, approved, working-memory-digest-refs: list<string> }`. New types: `enum intent-class` (3), `record consent-envelope` (5), `variant scope` (20 cases, one per `Scope` variant, `i1.rs:59-131`), records `scope-mcp-call` (2), `scope-cli-subprocess-spawn` (3; `argv-prefix-hash: list<u8>` = 32 bytes), `scope-gateway-send` (2); `task-assign-body.scope: list<scope>`; `iac-frame` 8 → **11** fields (`intent`, `consent-envelope: option<consent-envelope>`, `intent-lineage: list<string>`). World unchanged: `use frames.{iac-frame, halt}`, three exports, **no imports**. Counts after: records **20**, iac-frame 11, prior-distillate-ref 3, decision-dispatch-body 3, task-assign-body 5, frame-kind 17, origin 4, posture 3, rupture 5, payload 11, intent-class 3, scope 20. The header comment and the world comment (`:213-218`) are rewritten in the same edit (D-17-3b-C). No dual world, no `@1.x` host support: no `@1.x` component was ever published. |
| D-17-3b-B | Losslessness and the future-variant policy (AC2) | `lower` becomes `fn lower(&domain::IacFrame) -> Result<wit::IacFrame, BridgeError>`; `lift` stays `Result`. **Any domain value the WIT cannot name is refused, typed — never mapped**: new `BridgeError::Unrepresentable { ty: &'static str }` returned for a future `Scope`, `PostureHint` or `RuptureReason` variant (the `_ =>` arms at `:147`, `:196` stop mapping to `Cautious`/`RecipientUnloaded`). Lift is exhaustive over closed WIT types; byte lengths are checked (`frame-id`, `digest-frame-id`, `consent-id`, `rupture-id`, `original-frame-id` = 16 → `BadFrameIdLen`; `argv-prefix-hash` = 32 → new `BadHashLen(usize)`). `A2AIntent` and `WorkingMemoryDigestRefs` strings lift through their unvalidated constructors (`A2AIntent::new`, `i8.rs:53` — ADR-012's vocabulary is open; canonical-form checking is the consent router's, `is_canonical` `:85`) — the bridge is a **codec, not an authorization point**: a guest-emitted intent or consent envelope is a claim the kernel's frame ingress judges (17-3c AC2), exactly as for a native Spirit. The dead `BridgeError::{UnknownFrameKind, PayloadKindMismatch}` (never constructed, `:30-37`) are deleted. The runner maps a lower failure on an inbound frame to a new exit **5 `UnrepresentableFrame`** naming the type. |
| D-17-3b-C | The typed `@1.x` refusal, and the two R17-39 rows on the same lines (AC2) | Before `Spirit::instantiate`, the runner walks the compiled component's imports (`Component::component_type()` → `types::Component::imports(&engine)` yielding `(&str, ComponentExtern)` — verified in the locked source, `wasmtime-46.0.3/src/runtime/component/component.rs:396`, `types.rs:1042-1049`) for names starting `maos:spirit/`; any `maos:spirit/frames@<v>` whose major ≠ 2 → new exit **2 `IncompatibleWorld`**, stderr `maos-wasm-runner: IncompatibleWorld: component imports maos:spirit/frames@<v>; this runner implements maos:spirit@2.0.0`. The measured `@2`-host × `@1`-guest behaviour (P6 — never measured) is recorded in T1 before the check is written. **R17-39(a):** an instantiate-time `Trap::OutOfFuel` goes through `classify_trap` → exit 4 (not 3). **R17-39(d):** the WIT header and world comment and the `runner.rs:30-37` / `host_state.rs:8-9` docs stop claiming the runner *is* sandboxed: *"the runner runs inside ADR-031's T2 process boundary once launched by `spawn_and_bridge` — the production launch is `17-3c-wasm-spirit-on-the-bus-under-t2`"* (rule 11(a): a named, not-done key). Negative fixture: a committed **WAT-text** component (`tests/fixtures/wasm/spirit_frames_v1_world.wat`, a few lines importing `maos:spirit/frames@1.0.0`) — source-controlled like the `.wat` core fixtures, outside the provenance gate; parsed by wasmtime's `wat` feature, which is on (`maos-wasm-host/Cargo.toml:40` keeps wasmtime's default features; `wat` is in its `default` list). |
| D-17-3b-D | The 4 pinned components and their provenance (AC2, R17-28, R17-40) | Rebuild all four (`guests/echo-spirit`, `guests/equiv-fixture/{wasm-guest,divergent-guest,cosmetic-guest}`) against `@2.0.0` with `cargo build --locked --release --target wasm32-wasip2 --manifest-path <guest>/Cargo.toml` under a **private** `--target-dir` (F9), rename per the existing names, re-pin all **8** hashes. `equiv-fixtures.provenance.toml` gains a **required** `[toolchain]` table — `rustc` (full `rustc -V`), `wit_bindgen` (locked version), `target = "wasm32-wasip2"`, `wasi_imports` (the `wasi:*@` version the blobs import, read with `wasm-tools component wit`) — which `check-equiv-fixture-provenance` parses with `deny_unknown_fields` and requires non-empty, so a re-issue that does not record its toolchain reds. **No CI rebuild-and-compare step**: rebuilds are measured non-reproducible (P4, F10); the phantom *scoped-nightly* text is **deleted** from `check_equiv_fixture_provenance.rs:15-17,:104-105` and the TOML header `:11-16`, replaced by the true boundary: *"blob bytes and source bytes are pinned per commit; the toolchain is recorded; reproduction is a manual recipe (story 17-3b §Dev Notes), not a CI control"*. The native twin (links `maos-domain`, not the WIT) needs no rebuild. |
| D-17-3b-E | The equivalence oracle after losslessness | `F3_EXCLUDED_FIELDS` (`equiv_harness.rs:284`) becomes **empty**; `NormalizedFrame` carries `intent`, `consent_envelope`, `intent_lineage`, and `normalize` stops collapsing `scope` (`:320-327`) — the four fields become compared invariants (a stronger oracle). `f3_allowlist_is_pinned_to_the_dropped_set` (`:1855-1862`, a wording pin) is deleted; `normalize_ignores_exactly_the_f3_excluded_fields` becomes a behavioural test that frames differing only in one of the four fields normalize **unequal**. `forged_consent_is_non_invariant` (`:1654-1722`) is rewritten to its real claim: within **one** form, a run with a forged in-band `consent_envelope`/`intent` and a clean run reach the same kernel verdict (the kernel derives enforcement), for both the native and the WASM form. Docs `:32-37`, `:270-296` rewritten. `check-wasm-form-equiv`'s anti-canned tripwire (`fault_inject_passed > base_passed`, `discipline.yml:3339-3344`) stays green. |
| D-17-3b-F | How the break is recorded (FR62(b), AC2 — "fixed at creation", rule 9) | (1) Two `[[ratification]]` entries in `xtask/abi-ratifications.toml` (ADR-045 §4 shape, precedent `9.3b`/`13.5d`): `17-3b-AC2-wit-spirit-2.0.0` (`adr_ref = "ADR-031,ADR-045,ADR-060"`; `covered_changes` = `maos:spirit@2.0.0`, `iac-frame`, `scope`, `intent-class`, `consent-envelope`, `scope-mcp-call`, `scope-cli-subprocess-spawn`, `scope-gateway-send`, `prior-distillate-ref`, `decision-dispatch-body`) and `17-3b-AC3-manifest-schema-5` (`MANIFEST_SCHEMA_VERSION`, `ClassSection`, `RawClassSection`). (2) **The control:** a new uncharged test in `crates/maos-wasm-host/tests/wit_corpus.rs` reads the package version from `wit/spirit.wit` with `wit-parser` and asserts a `status = "ratified"` entry whose `covered_changes` contains exactly `maos:spirit@<that version>` — any future WIT version without a ratified proposal reds (proven red: plant `@2.0.1`). (3) **Not built:** a Transparency-Log `AbiExtension` emitter — no production trigger exists, and 9.3b's FR62 proposal set the precedent of the manifest entry alone. `requirements-inventory.md:568` is reworded to "first recorded WIT ABI-extension proposal" (OLD→NEW). |
| D-17-3b-G | The manifest shape (AC3) | `RawClassSection` gains `#[serde(default)] artifact: Option<String>`; `ClassSection` gains `pub artifact: Option<String>` (every literal site — 13, re-grep — + `artifact: None`). The forms validator (`manifest.rs:385-398`) accepts `rust-inproc`, `subprocess`, `wasm-component`; `wasm` stays invalid (ADR-060 cl. 2; test `:2588-2593` unchanged); `ts-inproc` is an orphan and simply leaves the two TS manifests. Validation (typed `validation_msg("class.forms"/"class.artifact", …)`): `wasm-component` must be the **only** form; `artifact` is **required** with `wasm-component` and **forbidden** otherwise; `artifact` is non-empty UTF-8 ≤ 4096 bytes, **relative**, no `..` component, ends in `.wasm` (resolved against the manifest's directory by 17-3c — this story never opens it); `wasm-component`/`artifact` require a declared `manifest_schema_version` ≥ a **private named const** `WASM_COMPONENT_SINCE_SCHEMA: u32 = 5` in `manifest.rs` (the xtask scanner forbids literal compares of `manifest_schema_version`, `check_manifest_schema_version.rs:67`). `class.artifact` is a field, not a section: **not** added to `POST_V1_SCHEMA_SECTIONS` (`:253`), so `RATIFIED_POST_V1_SCHEMA_SECTIONS` (`xtask/src/check_manifest_schema_version.rs:37-43`) is unchanged and no v1–v4 manifest gains a warning. The `[class].abi = "1.0"` rule (`:331-337`) is unchanged — it is the Spirit ABI, not the WIT version. |
| D-17-3b-H | Schema 5 | Lands here (P12; R17-51 — 17-1 and 17-2 later add their fields to v5 with no second bump). `MANIFEST_SCHEMA_VERSION` 4 → 5 (`lib.rs:114`), ledger row `:33`, doctests `:112`, `:156-160` (+ a `5` row); tripwire `manifest_n_minus_1_test.rs:103-107`; above-max probe → 6 (`manifest.rs:2576-2586`); `gen-abi-docs` + `stability-matrix` regenerated (never hand-edited); `docs-site/docs/manifest/v5.md` + `docs-site/docs/migrate/v4-to-v5.md` (+ ko twins, glossary-lock) on the 13.5d precedent; `manifest_field_coverage` gets `("class","artifact")` + 3 fixtures; the 17-3b-AC3 ratification entry. A v4 manifest still loads (N-1); a v4 manifest declaring `wasm-component` is refused by D-17-3b-G's schema rule. |
| D-17-3b-I | maos-bin admission (AC3) | In `gate_manifest` (`admission.rs:413`), after gate 4 binds `class_section`: `let wasm = class_section.forms == ["wasm-component"]` (the validator already guarantees it is then the sole form). **Gate 5 runs only for non-wasm** (third-party class names are arbitrary). Gate 6: `[sandbox]` stays required and parsed by the same `SandboxConfig::from_toml_str`; for `wasm`, `tier != T2` → new **`WasmComponentRequiresT2 { tier }`** (code `wasm_component_requires_t2`; T4 never reaches it — P9); `rust-inproc` keeps `RustInprocRequiresT0`. Gates 6–7 then parse every remaining section for both forms (an author gets every section error even in a default build). **Last step of `gate_manifest`, wasm only:** `#[cfg(not(feature = "wasm-host"))]` → **`WasmEngineOff`** (code `wasm_engine_off`; Display names Hold 2 and the `wasm-host` export-control precondition, `RELEASE-HOLDS.md:28,32-38`); `#[cfg(feature = "wasm-host")]` → **`WasmLaunchNotBuilt`** (code `wasm_launch_not_built`; Display names `17-3c`). Compile-time, because the published binary's truth is its feature set (epic-20 exit line 9). **The Display of all three new variants begins with its code** (`wasm_engine_off: …`, `wasm_launch_not_built: …`, `wasm_component_requires_t2: …`): `maos run` prints only `format!("maos run: {refusal}")` (`main.rs:4880`) and no existing Display carries a code (`admission.rs:275-313`), so without it the code never reaches `maos run`'s stderr. No new module (`SCANNED_SOURCE_FILES` unchanged); no new `CapabilitiesRequired` parser call (P11). The three `.expect` sites stay true (a wasm manifest never passes `gate_manifest` here); the `UnknownClass` doc `:214-216` is rewritten. The door returns HTTP 400 with the code through its wildcard arm (`operator_door.rs:628-631`) — no door change. |
| D-17-3b-J | Registry admission by form (AC4) | (a) **Reader:** new `ClassSection::from_manifest_toml(manifest_toml: &[u8]) -> Result<Option<ClassSection>, ManifestError>` in `maos-manifest` (the `ModelProvenanceSection::from_manifest_toml` shape): `None` iff the document has no `[class]` table; otherwise the full `from_toml_str` validation. (b) **One private helper**, `fn form_policy(pkg: &SignedPackage) -> Result<FormPolicy, AdmissionError>`, called in **both** `admit_spirit` and `admit_spirit_with_attestation` right after the FKCS refusal, before any tier logic: no `[class]` → `FormPolicy::Legacy` (behaviour unchanged — maos-bin gate 4 refuses such a package at load anyway); invalid `[class]` → new **`AdmissionError::ClassSectionInvalid(String)`**; `forms` contains `rust-inproc` → new **`AdmissionError::FirstPartyFormFromRegistry`** (ADR-060 cl. 1), at **every** tier and with or without attestation; `wasm-component` → `FormPolicy::WasmComponent`; `subprocess` → `FormPolicy::Legacy` (ADR-060 cl. 3, unchanged). (c) **Every tier** admits `wasm-component` through today's tier arms (signature / org key / envelope obligations unchanged — R17-58); the returned `sandbox_tier_floor` for a `wasm-component` package is **`SandboxTier::T2` at every tier**, and `t3_for_public_untrusted` does not apply to it (P17). (d) `AdmissionError` stays `Clone`; both variants get rows in `xtask/fr63-typed-errors.toml` (convention — that catalog has no gate; `error-catalog.toml` does not scan the registry) and the module doc `:1-10` is corrected (it still says PublicVetted is rejected). (e) **FKCS (P18) — re-pin its own reviewed diff, same commit.** D8 is closed and the hold is gone. After `cargo fmt --all`, read the new hash from `cargo run -p xtask -- check-fkcs --json` (the `admission-path-unmodified` detail) and set `xtask/fkcs-baseline.toml` `[admission_baseline].sha256` in the same commit, with a comment naming this story, what changed in the admission path (form policy; structural FKCS read — (g)) and the §A6 review that covered it. `xtask/tests/fkcs_oracle.rs::every_declared_admission_file_reds_a_one_byte_mutation` stays green only if the pin matches. The `form_policy` call goes **inside or right after** `frozen_surface_gate` in both entry points — never a second copied prelude (the L4 landmine, ADR-052 §3 and its 2026-09-29 amendment). (g) **The FKCS parse becomes structural** (routed here by D8, `deferred-work.md` D8 section): `extract_fkcs_internal_references` / `parse_manifest_string_array` are line-based, so a multi-line `internal_references = [` array, a `[fkcs] # comment` header or a dotted `fkcs.internal_references` key evades the refusal. Replace them with a structural read in `maos-manifest` beside `ClassSection::from_manifest_toml` (one TOML parse of the document serves both); a manifest that is not valid TOML is refused before any tier work (typed, existing or new variant — state which); vectors for each evasion shape plus the existing single-line form; the FKCS negative control (`cargo test -p maos-fkcs`) stays green. (f) `kernel-api-classes.toml:585` (`admit_spirit` = `data-movement`, name-keyed) is unaffected; signatures do not change. |
| D-17-3b-K | The TypeScript scaffold (AC5, R17-53) | `templates/spirit-ts` (the example regenerates from it): **manifest** — schema 5, `forms = ["wasm-component"]`, `artifact = "dist/spirit.wasm"`, `trust_tier = "local"` (20-1 sets `"public-untrusted"`, **hyphen** — P20), `[sandbox] tier = "T2"`, **no `[capabilities.required]`** (P23), `min_substrate_version` = the workspace's `0.1.0` line (not `0.5.0`), posture/output_shape/budget/resources kept valid. **`wit/spirit.wit`** — a byte copy of the root `wit/spirit.wit` (a scaffolded project outside the repo has no `../../wit`), guarded by an uncharged test asserting root == template == example bytes. **`package.json`** — exact pins `@bytecodealliance/componentize-js` `0.23.0` (the direct CLI; never `jco componentize`, which nests 0.22.0), `@bytecodealliance/jco` `1.35.0` (guest-types only), `typescript` `5.9.3`, `vitest` `1.6.1` (what the example lockfile resolves today, `package-lock.json:1767-1768`; Node 24 compatibility [INFERENCE — confirmed by T9's run]); `"engines": { "node": "^22.20 \|\| ^24.12 \|\| >=25" }` (the locked `@napi-rs/lzma@1.5.1` range); scripts `guest-types` = `jco guest-types wit --world-name spirit --out-dir src/bindings --strict`, `build` = guest-types + `tsc`, `componentize` = `componentize-js dist/index.js --wit wit --world-name spirit --disable http fetch-event -o dist/spirit.wasm`, `test` = `vitest run`. `src/bindings/` is generated and git-ignored. **`src/index.ts`** — `import type { IacFrame } from "maos:spirit/spirit@2.0.0"`; exports `handleFrame`, `onStart`, `onShutdown`; a comment documents the thrown-halt shape (`throw { tag: "fault", val: "<reason>" }` → `Halt::Fault`, because `jco guest-types` declares `handleFrame(…): IacFrame[]` and omits `result<…, halt>` — R17-39c) and that `console.log` is discarded (R17-39b, `host_state.rs:25`). **`tests/spirit.test.ts`** — pure vitest over the exports: `handleFrame` before `onStart` throws `{ tag: "fault", … }`; after `onStart` it returns the frame. **The v0.5 SDK `sdks/spirit-ts` is RETIRED (operator ruling 2026-09-29, Q3)** and the scaffold is **self-contained**: it depends only on public npm packages (componentize-js, jco, typescript, vitest), so a project generated outside this repo can `npm ci` — the unpublished-SDK defect ADR-043 `:33` records. The scaffold carries its own `src/halt.ts` (`export function halt(reason: string): never { throw { tag: "fault", val: reason }; }` — the typed shape `jco guest-types` omits, R17-39c) and its tests import from `../src`. Deleted in the same commit: `sdks/spirit-ts/` (the whole package; `sdks/` itself if then empty, with the `README.md:333` tree line), the `example-spirit-ts-tests` "Build TS SDK" step (`discipline.yml:1095-1096`), `templates_regen.rs`'s dead `rewrite_npm_sdk_dep_to_path` (`:265-274`) and `{{package_name}}` substitution with their tests adjusted, and the `sdks/spirit-ts` existence leg of `check_7_1_ts_template_baseline` (`xtask/src/check_epic_6_bridge.rs:2143-2152` — the check keeps the template and example legs). `cross_template_field_consistency` is rewritten to what is true (both templates are valid manifests of their form — no shared `provider.complete`). The example's `package-lock.json` is regenerated (example-only, unchecked by regen). Both scaffold READMEs (`:20-25`) are rewritten; `discipline.yml:1121`'s retirement comment gains *"SDK retired by 17-3b"*. The synthetic `'@maos/spirit-ts/spirit_test'` string in `coverage_matrix_nfr_test_3.rs:290` is a walker fixture, not a dependency — leave it. |
| D-17-3b-L | Where AC1 runs in CI | `example-spirit-ts-tests` (`discipline.yml:1080-1098`; key kept — aggregate `:4310-4311` and three report tables need it) becomes the 17-3b job: `actions/setup-node` **`'24'`** (Active LTS; ≥ 24.12; 22 is maintenance-only to 2027-04-30); step "scaffold" `cd examples/example-spirit-ts && npm ci && npm test && npm run build && npm run componentize`; `actions/upload-artifact` for `dist/spirit.wasm` (never committed — 12.26 MB, non-reproducible); `cargo build --locked --release -p maos-wasm-host --bin maos-wasm-runner`; `MAOS_TS_COMPONENT=$PWD/examples/example-spirit-ts/dist/spirit.wasm cargo test --locked --release -p maos-wasm-host --test ts_component_17_3b -- --ignored --nocapture`; `cargo build --locked -p maos-cli --bin maosctl` (cargo never builds a sibling package's `[[bin]]` for `-p maos-bin` — the 16-6 precedent, `maosctl_load_16_6.rs:68-83`); `cargo test --locked -p maos-bin --test wasm_form_admission_17_3b -- --nocapture` (default build → `wasm_engine_off`); `cargo test --locked -p maos-bin --features wasm-host --test wasm_form_admission_17_3b -- --nocapture` (→ `wasm_launch_not_built`; a test build, never published). AC1(a)'s drift check stays in `example-spirit-drift` (`:1066-1078`) — same run, cited by run id. Every verdict is a `::notice`/`::error` annotation (job logs are 403 without a token). `ubuntu-latest` is acceptable here: no step proves a sandbox (epic D6/R17-50 bind sandbox jobs only). |
| D-17-3b-M | The frame driver for AC1 | No spike driver merges (`q1-driver` is `.rs.txt`). New uncharged `crates/maos-wasm-host/tests/ts_component_17_3b.rs`, modelled on `e2e_roundtrip.rs:32-62,179`: resolves the release runner beside the test binary (asserts it exists), reads `MAOS_TS_COMPONENT`. The test is `#[ignore = "needs the componentize-js artifact; run by example-spirit-ts-tests with --ignored"]` — the input exists in one job only, and `wasm-host-tests` (`cargo test -p maos-host -p maos-wasm-host`, `discipline.yml:2981`, four images) and `workspace-test-suite` (`:4043`) run the crate's tests without it; in an `--ignored` run an absent `MAOS_TS_COMPONENT` **panics** with a named rule-11(b) reason (never a skip). Feeds one fully populated `@2.0.0` frame (intent, consent envelope, lineage, every `Scope` shape, a `prior_distillate_ref` with lineage) over real pipes, reads exactly one emitted frame, asserts canonical-CBOR byte equality with the input, runner exit 0, and prints `TS-COMPONENT-OUTCOME …` for the annotation step. |
| D-17-3b-N | Cookbook and ADR texts (AC5, R17-49, R17-57) | **New page** `docs-site/docs/cookbook/wasm-component-spirit.md` (+ ko twin; index table `cookbook/index.md:13-17`): the third-party TypeScript path — scaffold, `npm run build && npm run componentize`, the manifest, the admission outcome today (`wasm_engine_off` in published builds until Hold 2; `wasm_launch_not_built` in a `wasm-host` build until 17-3c), the halt shape, `console.log` discarded, Node range. `hello-world-spirit.md` is re-framed as **in-tree first-party** authoring (`rust-inproc` / `T0` is right for that audience) with a pointer to the third-party page — and its block (`:24-48`) is **completed to an admissible manifest**: today it fails `gate_manifest` three ways (no `[output_shape]`, no `[posture]` — both `required()`, `admission.rs:476-483`; `[budget] max_inference_calls` is refused by `RawBudget`'s `deny_unknown_fields`, which also requires `context_window_size`, `manifest.rs:846-850`) → add `[posture]` (`assistive`/`assistive`) and `[output_shape] required_fields`, replace `[budget]` with `context_window_size` + `time_cap_seconds`. `manifest-fields.md`'s reference block becomes a manifest that passes admission (refused values replaced; `forms`/`artifact`/schema 5 documented; *"currently 3"* fixed); `compliance-claim.md:94`'s `audited` becomes a real tier (same class, same test). `cli-wrapper-spirit.md` stays **17-1 AC5**'s. **Test:** `crates/maos-manifest/tests/cookbook_sections_17_6.rs` is superseded by uncharged `crates/maos-bin/tests/cookbook_manifests_17_3b.rs`, which (measured inventory: only `hello-world-spirit.md`, `manifest-fields.md`, `compliance-claim.md` and `cli-wrapper-spirit.md` carry a `[class]`-bearing ```` ```toml ```` block) runs the three **whole-manifest** blocks — `hello-world-spirit.md`, `manifest-fields.md`, the new `wasm-component-spirit.md` — through **`maos_bin::admission::gate_manifest(block, &\|_\| true)`** (the daemon's own section parsers, every section), expecting `Ok` for the `rust-inproc`/T0 blocks and exactly `WasmEngineOff` for the `wasm-component` block (the refusal is `gate_manifest`'s last step, so it proves every section parsed); parses the `compliance-claim.md` **fragment** table by table (`[class]` → `ClassSection::from_toml_str`, `[model_provenance]` → `ModelProvenanceSection::from_manifest_toml`); leaves `cli-wrapper-spirit.md` to 17-1 AC5; and asserts each ko twin's blocks are byte-identical to en — **red at HEAD** (the ko pages translate comments inside the blocks: hello-world en `:39` vs ko `:40`, manifest-fields en `:35` vs ko `:36`), so the ko blocks are made byte-identical (code comments stay English; prose around them is translated). The cookbook index row (`cookbook/index.md:13-17`, *"all schema v3 sections"*) is corrected. **ADRs:** ADR-060's Context (`:12-62`) restated to post-17-3b facts, plus a dated *Implementation note (17-3b)* recording "every tier" (R17-58) and the form-aware T2 floor — clause text itself unchanged; ADR-031's `@1.0` references (`:3`, `:47`, `:77`) → `@2.0.0` with a dated note; `compliance.rs:228-229` → *"T4 — the reserved WASM tool sandbox; refused at admission. WASM-component Spirits run at T2 (ADR-031) + WIT."* (matching `i9.rs:85`); `STABILITY.md`'s `PRESERVED:export` fence gains the refusal code `wasm_engine_off` (keeping `wasm-host` + `5D002`, which `decision_adrs_and_provisioning.rs:780-782` requires). |

## Acceptance Criteria

Each AC cites the code it changes. *Proven red* = a planted fault turns the control red — in CI with the run id
recorded, or locally with command + output committed when CI cannot host it (rule 11(c)); the planted diff and a
warning-free build are recorded with it (17-6 RT-10). No test re-pins wording.

- **AC1 (command) — the form, built and refused by name.** In CI (`example-spirit-ts-tests`, D-17-3b-L), on the
  recorded run: (a) `cargo run -p xtask -- templates-regen --check --json` passes with `examples/example-spirit-ts`
  regenerated from `templates/spirit-ts` (D-17-3b-K); (b) the example builds under Node 24 with the direct pinned
  `componentize-js` 0.23.0 into `dist/spirit.wasm` (uploaded as a CI artifact, never committed); (c) the
  **release** `maos-wasm-runner` runs it for `on-start` and one populated ADR-032 frame against
  `maos:spirit@2.0.0` — `ts_component_17_3b` (D-17-3b-M) shows exit 0 and a byte-identical canonical-CBOR round trip
  of intent, consent envelope, lineage and every `Scope` shape; (d) in the **default** build `maos run
  examples/example-spirit-ts/manifest.toml --once` (after `maos init` in a scratch `HOME`/`MAOS_HOME`/`XDG_DATA_HOME`,
  the `maosctl_load_16_6.rs:583-600` idiom) exits non-zero with `wasm_engine_off` — never `unknown_spirit_class`,
  `class_section_invalid` or *unknown form*; (e) in a `--features wasm-host` build the same command refuses with
  `wasm_launch_not_built`. (d) and (e) are `crates/maos-bin/tests/wasm_form_admission_17_3b.rs`, which also drives
  `maosctl load` over the door for the same manifest (a row in the `maosctl_load_16_6.rs:530-575` shape: HTTP 400 +
  the code, roster unchanged). Every verdict is a public annotation; the run id is recorded in the Dev Agent Record.
  **Proven red:** delete the fork (D-17-3b-I) → (d)'s `maos run` stderr reads `unknown Spirit class '…'` and the door row reads `unknown_spirit_class`; drop the `--disable http
  fetch-event` flag or the Node pin → (b)/(c) red (measured once, recorded).
- **AC2 — the contract, broken once, before anyone depends on it.** `wit/spirit.wit` is `maos:spirit@2.0.0`
  (D-17-3b-A). `frame_bridge` (`crates/maos-wasm-host/src/frame_bridge.rs` — `lower` `:403-414`, `lift` `:419-433`,
  `payload_to_wit` `:211-290`, `payload_from_wit` `:297-387`, `scope_from_debug_string` `:395-397` deleted) lowers and
  lifts intent, consent envelope, lineage, typed scope, `PriorDistillateRef.intent_lineage` and
  `DecisionDispatch.working_memory_digest_refs` **losslessly**; an unrepresentable future variant is refused, typed
  (D-17-3b-B). **Oracle:** a permanent test in `crates/maos-wasm-host/tests/frame_bridge_roundtrip.rs` round-trips one
  fully populated frame per payload shape that carries a nested field (TaskAssign with `prior_distillate_ref` + lineage
  + all 20 `Scope` variants; DecisionDispatch with digest refs; a populated `ConsentEnvelope`), asserting
  `encode_cbor(lift(lower(f)?)?) == encode_cbor(f)` byte-for-byte; the byte count of the TaskAssign oracle is recorded
  (it replaces the spike's 1,025). The lossy-pin tests `:181`, `:387-405`, `:408-418` are **deleted** (house rule), not
  re-pinned. **Runner:** exit 2 `IncompatibleWorld` for a `maos:spirit/frames@1.*` import, exit 4 for an
  instantiate-time fuel trap, exit 5 `UnrepresentableFrame` (D-17-3b-C; `RunnerExit` `:52-62`, `:170-174`); the four
  re-issued components run under it. **Fixtures:** the 4 components re-issued, 8 hashes re-pinned, toolchain recorded
  and required (D-17-3b-D); phantom nightly text gone. `wit_corpus.rs` counts move to D-17-3b-A's figures, plus count
  gates for `intent-class` (3) and `scope` (20). `check-wasm-form-equiv` green (both legs non-vacuous; D-17-3b-E).
  **Recording:** the two ratification entries + the version-binding test (D-17-3b-F). **Proven red:** (i) restore the
  `:327` default → the DecisionDispatch oracle reds; (ii) restore a `_ => Cautious` backstop → the unrepresentable
  vector reds; (iii) run the runner on `spirit_frames_v1_world.wat` → exit 2 (and on a copy of today's pinned `@1.0`
  echo blob, kept out of the tree, → exit 2 — recorded); (iv) bump the WIT to `@2.0.1` without a ratification entry
  → the binding test reds; (v) edit the WIT without re-issuing → `check-equiv-fixture-provenance` reds.
- **AC3 — manifest and daemon admission.** `ClassSection.forms` accepts `wasm-component` and still refuses `wasm`
  (`crates/maos-manifest/src/manifest.rs:385-398`, test `:2588-2593`); `[class].artifact` carries the component path
  (D-17-3b-G). **Schema 5 lands here** with N-1 compat, `manifest_field_coverage` rows + fixtures, and its
  ratification entry (D-17-3b-H). maos-bin `gate_manifest` forks on form **before** gate 5
  (`crates/maos-bin/src/admission.rs:458-463`): a `wasm-component` manifest is refused, typed, as `WasmEngineOff`
  (default build) or `WasmLaunchNotBuilt` (`wasm-host` build, until 17-3c replaces it) — never `UnknownClass`; its
  `[sandbox] tier` must be `T2` (`WasmComponentRequiresT2`; `T4` refused at parse as `manifest_section_invalid`)
  (D-17-3b-I). `crates/maos-spirit-abi/src/compliance.rs:228-229` is rewritten (R17-57). Vectors (in
  `wasm_form_admission_17_3b.rs` and `manifest.rs` tests): `wasm-component` + `T0`/`T1`/`T3` → `wasm_component_requires_t2`;
  + `T4` → `manifest_section_invalid`; `[sandbox]` without `tier` → admitted to the engine refusal (defaults T2);
  `["wasm-component","rust-inproc"]` → `class.forms` refusal; `wasm-component` without `artifact`, with an absolute,
  `..`-bearing or non-`.wasm` artifact, or at schema 4 → `class.artifact`/schema refusals; `rust-inproc` with an
  `artifact` → refused; a v4 `rust-inproc` manifest still loads (N-1). **Proven red:** move the engine refusal before
  gate 6 → the `T3` vector reads `wasm_engine_off` (the tier check became unreachable).
- **AC4 — registry admission by form.** `admit_spirit` (`crates/maos-registry/src/admission.rs:179`) **and**
  `admit_spirit_with_attestation` (`:278`) read `[class]` through `ClassSection::from_manifest_toml` (D-17-3b-J(a))
  and refuse any package whose `forms` include `rust-inproc` with `AdmissionError::FirstPartyFormFromRegistry` at
  every trust tier, attested or not (ADR-060 cl. 1); they admit `wasm-component` at **every** tier (R17-58 — the old
  *"≥ `PublicUntrusted`"* is ill-posed, `compliance.rs:194-203`) with `sandbox_tier_floor = T2`; a malformed `[class]`
  is `ClassSectionInvalid`; a legacy `[spirit]`/bare-key package is unchanged. **Wiring (rule 11(a)):** the production
  caller is **`maosctl import`** (`crates/maos-cli/src/subcommands.rs:1359`, `:1368` — P13); 20-1's install is the
  second. Vectors: a `rust-inproc` package refused at Local, OrgInternal, PublicUntrusted, and PublicVetted-with-a-valid-attestation;
  a `wasm-component` package admitted at Local / OrgInternal / PublicUntrusted (valid envelope) with floor T2 and with
  `t3_for_public_untrusted = true`, and at **PublicVetted with a valid attestation** with floor T2 (that path does not
  delegate to `admit_spirit` — its floor comes from `verify_public_untrusted_baseline`, `:315`, `:330`); `trust_tier = "public_untrusted"` inside `[class]` → `ClassSectionInvalid` (P20);
  every existing registry test and the 19 roundtrip fixtures green. **Through the production verb:** a
  `maos-cli` integration test builds an import bundle whose manifest declares `rust-inproc` and runs the real
  `maosctl import` binary → non-zero exit naming the refusal. Its needs, measured: the bundle builders in
  `crates/maos-registry/tests/import_air_gap_test.rs:11,29` are private test fns (copy them); `maos-cli` has no `tar`
  dev-dependency (`crates/maos-cli/Cargo.toml:52-57` — add it); without `--dry-run` import writes under
  `$HOME/.local/share/maos/registry` (`storage.rs:90-96` — scratch `HOME`); the `[class]` carries all 8 fields with a
  hyphenated tier, or `ClassSectionInvalid` fires before the form refusal.
  `check-fkcs --json` → `oracle_green: true` after this story re-pins its own reviewed diff (D-17-3b-J(e)); the FKCS parse is structural, with a vector per evasion shape (D-17-3b-J(g)); ADR-060's anchors, fixture and planted-red vectors flipped
  (P19) with every direction the gate supports keeping a vector (the file's doctrine, `:4-11`). **Proven red:** remove
  the check from `admit_spirit_with_attestation` only → the attested `rust-inproc` vector admits.
- **AC5 — FR33, and the texts that teach it.** `templates/spirit-ts` is the `wasm-component` / schema-5 / `T2`
  scaffold of D-17-3b-K; its `src/index.ts` implements the world's three exports, typed through `jco guest-types`,
  with the thrown `halt` shape documented (R17-39c); the example regenerates from it (R17-53). `example-spirit-ts-tests`
  leaves EOL Node 20 (`discipline.yml:1088` → `'24'`). `sdks/spirit-ts` is **retired** (its README `:15` goes with it — Q3, D-17-3b-K); `examples/example-spirit-ts/README.md:20-25`
  **and** `templates/spirit-ts/README.md:20-25` stop saying *"test harness only"* / *"future kernel-side TS runtime"* and
  document the wasm-component path (P21). A scaffold generated outside the repo builds with public npm packages only. The cookbook teaches the
  third-party form around `wasm-component`, and **every** cookbook manifest block parses through the daemon's
  admission path, en + ko (D-17-3b-N — closes `deferred-work.md:1050`, R17-49). **Proven red:** restore
  `posture default = "supervised"` in `manifest-fields.md` → `cookbook_manifests_17_3b` reds naming the section;
  edit one ko block → the byte-equality assert reds.

## Kernel and kloc budget

- **Kernel-Δ 0** (`kernel_grant`). `cargo run -q --locked -p xtask -- check-kernel-baseline` must read `25032` at
  landing. If any change would touch `crates/maos-kernel-core/src`, STOP — that is a FLAG-Winston fork, not this story.
- **Measured at `4f677bc1` → expected** (`kloc_grant`; re-measure after `cargo fmt --all`, tokei 14.0.0):

| Crate | Measured / ceiling | Expected Δ | Disposition |
|---|---|---|---|
| `maos-wasm-host` | 1106 / 1577 | +150…+260 | absorbs (ledger `:661` books 17-3b) |
| `maos-manifest` | 4214 / 4474 | +50…+90 | absorbs (ledger `:395`) |
| `maos-registry` | 3515 / 4282 | +40…+80 | absorbs; re-book ledger `:647` to name 17-3b (R17-20) |
| `maos-spirit-abi` | 1038 / 3000 | +2 | absorbs |
| **`maos-bin`** | **23423 / 23423** | +40…+80 | **exact raise** — AUTHORIZED 2026-09-29; `kloc.toml:483`, full key cited |
| **`xtask`** | **44022 / 44123** after the D8 change (−101: the hold machinery deleted) | net −30…+60 | absorbs if it stays ≤ 44123; otherwise an **exact raise** — AUTHORIZED 2026-09-29; `:326`, full key cited |
| aggregate | 168280 / 170884 after D8 | +280…+510 | fits (2604 free); alarm stays firing |

- Commands, in order: `cargo fmt --all` · `cargo run -q -p xtask -- kloc-check --json` · `cargo run -q -p xtask --
  check-kernel-baseline` · `cargo run -p xtask -- check-manifest-schema-version` · `cargo run -p xtask --
  gen-abi-docs --check` · `cargo run -p xtask -- stability-matrix --check --json` · `cargo run -p xtask --
  check-equiv-fixture-provenance` · `cargo run -p xtask -- check-wasm-form-equiv --json` · `cargo run -p xtask --
  check-fkcs --json` · `cargo run -p xtask -- templates-regen --check --json` · `cargo run -p xtask --
  check-exit-commands` · `cargo test -p xtask --test decision_adrs_and_provisioning --test recovery_lane_ceiling_rule
  --test d11_xtask_ceiling_ratchet`.

## Review obligations (§A6 — each EXECUTED by a non-author, never read-only)

- (a) Re-run AC1 locally: Node 24, `npm ci && npm test && npm run build && npm run componentize` in the example; the
  release runner; `ts_component_17_3b -- --ignored` with `MAOS_TS_COMPONENT` set (exit 0, byte-equal) and unset (a
  named panic); a plain `cargo test -p maos-wasm-host` reports it ignored; `maosctl` built; both
  `wasm_form_admission_17_3b` builds.
- (b) Fetch the recorded run's annotations (unauthenticated `GET /repos/lunarpulse/maos/check-runs/<job>/annotations`)
  and match every AC1 leg to one.
- (c) Plant each fault once, **diff + warning-free build recorded**: every AC's *Proven red*, plus: a lossy `lift`
  (`intent: Readonly`) → the oracle reds; `F3_EXCLUDED_FIELDS` re-populated → the behavioural normalize test reds.
- (d) **Security (hostile guest):** a guest emitting a `frame-id` of 15 bytes, an `argv-prefix-hash` of 31 bytes, a
  4 MiB `intent-lineage` string list — each is refused or bounded with a typed error, never panics the runner (the
  16 MiB ADR-032 frame cap `codec.rs:26` bounds input; say what bounds output). State in the record that `lift` is not
  an authorization point and name the 17-3c ingress that judges guest claims.
- (e) **Security (registry):** the attested path, the `[spirit]`-legacy path, a document with **two** `[class]`-like
  tables or a `[class]` inside a string (TOML parse, not line scan), a manifest with `forms` spelled via dotted keys
  (`class.forms = [...]` at root) — each reaches the typed outcome the story names.
- (f) `@2`-host × `@1`-guest behaviour was measured before the import walk was written (T1), and the walk does not
  refuse a component that imports no `maos:spirit/*` (that is `InvalidComponent`, exit 3 — say so).
- (g) Every `gate_manifest` bypass path in `main.rs` (17-6 obligation (j): `:3665` shell hello-spirit, `:6785-6788`
  smoke-spirit, `:7959/:7972` hello-spirit boot, `:9826/:9848-9849` daemon control Spirit, `:13001-13007` smoke-abi — re-grep)
  constructs a first-party class and cannot load a `wasm-component` manifest; listed with evidence.
- (h) `check-fkcs --json` → `oracle_green: true` with the re-pinned hash; the reviewer recomputes the hash independently
  (the gate's framing: per file, relative path + NUL + little-endian u64 length + bytes) and confirms the pin comment
  names this story's reviewed changes; each FKCS evasion shape (multi-line array, header comment, dotted key) is refused.
- (i) Every `ClassSection` literal site compiles; kernel pin 25032; `kloc-check` rows match the table; zero-headroom
  raises exact and full-key-cited.
- (j) `check-exit-commands` passes, and line 2's provenance names `17-3c AC1`.
- (k) The four re-issued blobs import `maos:spirit/frames@2.0.0` (`wasm-tools component wit`), and the recorded
  `wasi_imports` matches what they import.

## Declared cut lines (rule 10 — each names an owner, never a bucket)

- **Launch, session, SIGSYS proof, `memfd_create`, kind-8 row, conformance out of the daemon, the guest diagnostics
  channel (R17-39b)** → **`17-3c-wasm-spirit-on-the-bus-under-t2`** (R17-24, R17-47, R17-48, R17-54, R17-55, R17-56).
- **`maos:spirit@2.1.0` recall import, the open `frame-kind-label`, FR29's consent vector** → **`17-3d-wasm-log-recall`**.
- **Publishing/installing the component; the example's `trust_tier = "public-untrusted"`** → **`20-1-registry-client-install-verb-vetter-and-yank`** (AC4 — hyphen, P20).
- **The vetting attestation binds the manifest, not the artifact** (found by the D8 review) → **`20-1-registry-client-install-verb-vetter-and-yank`**, before it ships `maos-spirit vet issue` (`deferred-work.md` D8 section; ADR-056 amendment).
- **The cookbook's `cli-wrapper-spirit.md` isolation claims** → **`17-1-worker-egress-allowlist-and-scoped-credential`** AC5.
- **`check-wasm-form-equiv` reporting a compile failure as "vacuous"** → **`20-3a-gate-honesty`** (`deferred-work.md:1046`).
- **A Transparency-Log emitter for `AbiExtension` governance events** — not a deferral: no production trigger exists and
  none is claimed (D-17-3b-F); FR62(b)'s record is the ratified manifest entry, as 9.3b's was.

## Dev Notes

### Current state of every file this story edits (READ each fully before editing)

- **`wit/spirit.wit`** (230 lines) — `package maos:spirit@1.0.0;` `:18`; header comment `:1-16`; `interface frames`
  `:20-211`; `record iac-frame` `:193-202` (8 fields); `halt` `:207-210`; false T2 comment `:213-218`; world
  `:219-230` (`use` `:220`, exports `:225`, `:228`, `:229`, no imports). Byte-feeds the 4 provenance `source_sha256`.
- **`crates/maos-wasm-host/src/frame_bridge.rs`** (433) — module gap doc `:1-21`; `BridgeError` `:30-37`; role/address/
  kind/origin/posture/rupture mappers `:39-205` (backstops `:147`, `:196`); `payload_to_wit` `:211-290` (Debug-string
  scope `:216`); `payload_from_wit` `:297-387` (scope filter `:302-306`, nested defaults `:315`, `:327`);
  `scope_from_debug_string` `:395-397`; `lower` `:403-414`; `lift` `:419-433`. Port from
  `spikes/story-17-3a-wasm-recall/bridge/src/bridge.rs.txt` (`scope_to_wit` `:412-455`, `scope_from_wit` `:457-492`,
  `intent_*` `:494-508`, `consent_*` `:510-530`, `lower` `:533`, `lift` `:555`) — then add the nested fields it lacks.
- **`crates/maos-wasm-host/src/runner.rs`** (271) — doc `:1-37` (false T2 claim `:30-37`); `RunnerExit` `:52-62`;
  `main` `:64-88`; `parse_args` `:107-134`; `COMPILE_TIMEOUT` `:140`; `run` `:142-198` (instantiate `:170-174`);
  `compile_with_timeout` `:203-222`; `classify_trap` `:224-231`; `pump_frames` `:233-271` (`lower` `:250`, `lift` `:263`).
  **Preserve:** exit 0/1/3/4 meanings, fuel metering, the compile watchdog, on-shutdown after a pump error.
- **`crates/maos-wasm-host/src/{wit_guest.rs:1,8-11, conformance.rs:1,17, adapter.rs:78,86, lib.rs:9, host_state.rs:8-9,25}`**
  — `@1.0` / T2-claim texts only; `bindgen!` path unchanged. `conformance.rs`'s in-daemon probe stays (17-3c AC3).
- **`crates/maos-host/src/lib.rs:133-136`** — `@1.0` text in `resolve_launch`'s doc. No enum change here: `SpiritHostError`
  is pinned by `abi-baseline/maos-host-v1.txt:5-11` (`check-host-surface`) — do not add variants.
- **Tests:** `frame_bridge_roundtrip.rs` (helper `:104` — `lower` becomes fallible; lossy pins `:181`, `:387-405`,
  `:408-418` deleted), `wit_corpus.rs` (`:117-187`), `equiv_harness.rs` (`:32-37`, `:270-327`, `:720`, `:1654-1680`,
  `:1855-1862`), `e2e_roundtrip.rs` (`:6`, frame literal `:65-91`, exit-3 pin `:258-268` stays — invalid bytes are
  still `InvalidComponent`), `fuel_t2_matrix.rs` (core `.wat` modules — unaffected).
- **Guests:** `crates/maos-wasm-host/guests/echo-spirit/src/lib.rs:1,8-11`; `guests/equiv-fixture/{logic/src/lib.rs:9,
  wasm-guest/src/lib.rs:4,11-14, divergent-guest/src/lib.rs:11-14, cosmetic-guest/src/lib.rs:15-18}` — each its own
  `[workspace]`, `cdylib`, `wit-bindgen = "0.44"`. They never construct an `IacFrame`; the equiv logic passes frames
  through — confirm it compiles against the new record shapes.
- **`tests/fixtures/wasm/equiv-fixtures.provenance.toml`** (header `:1-16`, 4 `[[fixture]]` rows) and
  **`xtask/src/check_equiv_fixture_provenance.rs`** (`:15-17`, `:104-105`) — D-17-3b-D.
- **`crates/maos-manifest/src/manifest.rs`** — `parse_manifest_trust_tier` `:35-69`; `ClassSection` `:218-231`,
  `from_toml_str` `:234-238`; `POST_V1_SCHEMA_SECTIONS` `:252-253`; `RawClassSection` `:289-300`; `validate`
  `:302-437` (forms `:385-398`, trust `:403-411`); `degrade_for_schema_version` `:499-505` (leave its literal — not
  this story's; note it); `ModelProvenanceSection::from_manifest_toml` `:1855` (reader precedent); tests `:2498`,
  `:2576-2593`. **Preserve:** the `ManifestError` public shape (1b.3 froze `TierParse`/`CapOutOfRange`/`Toml` — use
  `validation_msg`).
- **`crates/maos-spirit-abi/src/lib.rs`** `:33`, `:112`, `:114`, `:135`, `:156-162`; **`compliance.rs`** `:194-203`
  (TrustTier — untouched), `:219-230` (SandboxTier doc `:228-229`).
- **`crates/maos-bin/src/admission.rs`** (777; `#![cfg(feature = "network")]` `:1`) — enum doc `:190-199`;
  `AdmissionRefusal` `:201-244`; `code()` `:248-268`; `Display` `:275-313`; `KNOWN_CLASS_NAMES` `:321-330` (unchanged —
  a wasm class is not a known first-party class); `gate_manifest` `:413-520`. **Preserve:** gate order for non-wasm
  manifests byte-for-byte in behaviour; 17-6's `RustInprocRequiresT0`; the `(3,1)` parser pin.
- **`crates/maos-bin/src/main.rs`** — `ClassSection` literals `:6669`, `:6770`, `:12983`; nothing else (the three
  `gate_manifest` sites `:4446`, `:4877`, `:8606` keep their `.expect`s).
  The `smoke-spirit-author-7-1` verb (`smoke_spirit_author_7_1` `:12112-12217`, `--define package_name=…` `:12163`)
  cargo-generates the TS scaffold and runs `npm ci`/`npm test`; no CI job runs it (retired, `discipline.yml:1124`).
  Run it once after T9 and record the outcome (the template has never shipped a `package-lock.json`, so `npm ci` may
  already fail at HEAD — measure before and after, do not assume).
- **`crates/maos-registry/src/admission.rs`** — ⚠ **re-measured after the D8 commit (2026-09-29); every other cite of
  this file in this story is at `4f677bc1` and has shifted by ~+22 — go symbol-first.** Module doc `:1-13` (corrected
  by D8); `AdmissionError` `:37`; `verify_public_untrusted_baseline` `:118` (T3 floor `:175`); **`frozen_surface_gate`
  `:193`** (the one FKCS gate); `admit_spirit` `:206` (gate call `:210`); `admit_spirit_with_attestation` `:300` (gate
  call `:308`); `strictest_of` `:455` (its own lattice — PublicVetted top; untouched);
  `extract_fkcs_internal_references` `:501` and `parse_manifest_string_array` `:527` (replaced, D-17-3b-J(g)).
- **`xtask/fkcs-baseline.toml`** `[admission_baseline]` (re-pinned by this story, D-17-3b-J(e)); `extract_fkcs_internal_references`
  / `parse_manifest_string_array` in registry `admission.rs` (replaced, D-17-3b-J(g)); `frozen_surface_gate` (the one FKCS
  gate since D8 — extend, never copy).
- **`xtask/tests/decision_adrs_and_provisioning.rs`** — anchors `:262-293`, context `:898`, fixture `:941-947`,
  vectors `:1088-1091`, `:1150-1154`, `:1207-1211`; export-fence assert `:780-782`.
- **`crates/maos-kernel-core/tests/manifest_field_coverage.rs:33-74`** + `tests/fixtures/manifest/class/*/artifact.toml`
  (uncharged; kernel `src` untouched).
- **`templates/spirit-ts/*`, `examples/example-spirit-ts/*`, `sdks/spirit-ts/README.md`, `xtask/src/templates_regen.rs`**
  (TS pair `:67-83`, check `:95-117`, `rewrite_npm_sdk_dep_to_path` `:269-274`, tests `:286-423`).
- **`.github/workflows/discipline.yml`** — `example-spirit-ts-tests` `:1080-1098`; aggregate needs `:4310-4311`; report
  rows `:4422-4424`, `:4485-4487`, `:4547-4549` (names unchanged → no edit expected).
- **Docs:** `docs-site/docs/cookbook/{hello-world-spirit.md (:24-49), manifest-fields.md (:17-156), compliance-claim.md
  (:93-94), index.md (:13-17)}` + ko twins (+1 line offset) under
  `docs-site/i18n/ko/docusaurus-plugin-content-docs/current/cookbook/`; `docs-site/docs/manifest/`, `…/migrate/`;
  `docs/adr/ADR-060-…md` (Context `:12-62`, form-aware sentence `:116-118`, Consumers `:121-132`), `docs/adr/ADR-031-…md` (`:3`, `:47`, `:77`); `STABILITY.md` fence.
- **Also touched by the `@1.0` sweep** (text only): `crates/maos-wasm-host/Cargo.toml:27`, `equiv_harness.rs:32,:720`,
  `e2e_roundtrip.rs:6`. Planning prose mentioning `@1.0` (`prd/product-scope.md:70`, …) is history — leave it.

### Consequential test and xtask changes (APPROVED 2026-09-29 — apply every one; none is optional)

Every row is a test or gate this story's changes turn red, or a test it must add. Update, delete (wording pins) or
add in the same commit as the change that breaks it; never skip, `#[ignore]` (except D-17-3b-M's CI-input test) or
re-pin wording.

| Where | What changes |
|---|---|
| `crates/maos-wasm-host/tests/frame_bridge_roundtrip.rs` | helper `:104` → fallible `lower`; lossy pins `:181`, `:387-405`, `:408-418` **deleted**; the lossless oracle + unrepresentable + hostile-length vectors added (AC2) |
| `crates/maos-wasm-host/tests/wit_corpus.rs` | counts `:117-187` → D-17-3b-A figures; new `intent-class` (3) / `scope` (20) count gates; the version-binding test (D-17-3b-F) |
| `crates/maos-wasm-host/tests/equiv_harness.rs` | `F3_EXCLUDED_FIELDS` → ∅, `NormalizedFrame`/`normalize`, `f3_allowlist_is_pinned_to_the_dropped_set` deleted, behavioural normalize test, forged-consent control rewritten, docs `:32-37`, `:270-296`, `:720` (D-17-3b-E) |
| `crates/maos-wasm-host/tests/e2e_roundtrip.rs` | `@1.0` text `:6` only; exit-3 pin for invalid bytes stays |
| NEW `crates/maos-wasm-host/tests/ts_component_17_3b.rs`; WIT-copy byte-equality test; `tests/fixtures/wasm/spirit_frames_v1_world.wat` | D-17-3b-M, D-17-3b-K, D-17-3b-C |
| `crates/maos-manifest/src/manifest.rs` inline tests | above-max probe 5 → 6 (`:2576-2586`); `wasm-component`/`artifact`/schema-≥5 vectors; `wasm` refusal test `:2588-2593` unchanged |
| `crates/maos-manifest/tests/cookbook_sections_17_6.rs` | **deleted**, superseded by `crates/maos-bin/tests/cookbook_manifests_17_3b.rs` (D-17-3b-N) |
| `crates/maos-spirit-abi` | doctests `lib.rs:112`, `:156-160`; tripwire `tests/manifest_n_minus_1_test.rs:103-107` → 5 |
| `crates/maos-kernel-core/tests/manifest_field_coverage.rs` | `("class","artifact")` row + 3 fixture files; `(3,1)` pin untouched |
| `ClassSection {` literals (13) | `main.rs:6669`, `:6770`, `:12983`; 9 kernel-core test/bench sites; `xtask/tests/story_10_4a_ac1_proven_red.rs:1013` → `artifact: None` |
| NEW `crates/maos-bin/tests/wasm_form_admission_17_3b.rs` | every AC1(d)(e)/AC3 vector, both builds, the door row (needs `maosctl` built) |
| `crates/maos-registry/src/admission.rs` tests + `tests/*` | new form vectors across tiers/attestation; every existing test and the 19 roundtrip fixtures stay green unmodified |
| NEW `crates/maos-cli` import test (+ `tar` dev-dependency) | AC4 production-verb vector |
| `xtask/tests/decision_adrs_and_provisioning.rs` | ADR-060 anchors `:262-293` flipped, fixture `:941-947`, context `:898`, vectors `:1088-1091`, `:1150-1154`, `:1207-1211` — every direction keeps a planted red |
| `xtask/src/check_equiv_fixture_provenance.rs` (+ its tests) | required `[toolchain]` parse (`deny_unknown_fields`), missing-toolchain red vector, phantom text `:15-17`, `:104-105` deleted |
| `xtask/src/templates_regen.rs` (+ tests `:286-423`) | dead `rewrite_npm_sdk_dep_to_path` / `{{package_name}}` removed (SDK retired — Q3), `ts_template_renders_correctly` and `cross_template_field_consistency` rewritten to what is true |
| `xtask/fkcs-baseline.toml` | re-pinned to this story's reviewed admission diff (D-17-3b-J(e)); `check_fkcs.rs` itself is unchanged (D8 already removed the hold) |
| `xtask/src/check_epic_6_bridge.rs:2143-2152` | `sdks/spirit-ts` existence leg removed from `check_7_1_ts_template_baseline` (SDK retired — Q3) |
| `sdks/spirit-ts/` (whole package), `README.md:333`, `discipline.yml:1095-1096` | deleted with the SDK (Q3) |
| generated: `docs-site/abi/v1/*.md`, `STABILITY.md` | `gen-abi-docs`, `stability-matrix` regenerated (never hand-edited) |
| data: `xtask/abi-ratifications.toml`, `xtask/fr63-typed-errors.toml`, `xtask/kloc.toml`, `tests/fixtures/wasm/equiv-fixtures.provenance.toml` | two ratification entries; two registry error rows; the exact raises + registry ledger re-book; 8 hashes + `[toolchain]` |
| CI `.github/workflows/discipline.yml` `example-spirit-ts-tests` | D-17-3b-L (Node 24, `maosctl` build, `--ignored` driver run, both admission builds, annotations) |

### Guardrails

- **One commit for the break.** WIT + bridge + runner + the 4 re-issued blobs + 8 hashes + corpus counts + equiv
  harness land together, or `check-wasm-form-equiv` (Blocking at HEAD) reds between commits.
- **Refuse, never map.** No `_ =>` arm in the bridge may produce a value; an unrepresentable value is a typed error.
- **The bridge is not a security boundary.** Do not add validation that silently rewrites a guest's claim; lengths
  are checked because the domain types are fixed-size.
- **Never commit a `.wasm` produced by componentize-js** (12.26 MB, non-reproducible) and never place a binary in
  `templates/spirit-ts` (templates-regen reads every file with `read_to_string` and aborts on a binary).
- **Never run `npm` inside `templates/spirit-ts`.** `templates-regen` reads every file under the template dir with
  `read_to_string` and is not gitignore-aware (`templates_regen.rs:173-199`): a local `node_modules/`, `dist/` or
  generated `src/bindings/` there becomes drift or aborts regen. Build and test in the example only.
- **Release runner only** for the TS component (debug compile > 10 s → exit 3). Do not raise `COMPILE_TIMEOUT`.
- **Private target dirs** for guest and component builds (`--target-dir`; this host exports a global
  `CARGO_TARGET_DIR=/mnt/build/cargo`, 17-3a F9).
- **Re-pin the FKCS baseline in the same commit as the admission change, after review** (D-17-3b-J(e)); never add a
  hold, and never copy the FKCS prelude — extend `frozen_surface_gate`.
- **Hyphenated trust tiers inside `[class]`** everywhere this story writes one.
- **No kernel `src` line.** No new `maos-bin/src` module. No `SpiritHostError` variant.
- **Zero-headroom raises**: exact, formatted, measured, same commit as the driver, full story key in the comment.

### Previous story intelligence

- **17-6 (`done` at `4f677bc1`):** gate 6 already refuses `rust-inproc` above T0 (`RustInprocRequiresT0`) — the wasm
  arm sits beside it, not instead of it. CI proof without a token: a throwaway branch pushed over SSH, a temporary
  `on: push` workflow if needed, every verdict a `::notice`/`::error` annotation (≤10 per step), captures committed as
  `*.log` under `_bmad-output/implementation-artifacts/17-3b-evidence/`, branch deleted. Zero-headroom raises were exact
  (+22 each) with the key in the comment. Its review found TOCTOU and hostile-input holes the first draft missed — run
  the Security layer on `lift` and on `from_manifest_toml` deliberately. Its four-image review-patch annotations were
  never observed; do not cite 17-6 runs as proof of anything here. `examples/example-spirit/Cargo.toml` lacks the SDK
  `spirit_test` feature (`cargo test -p example-spirit` E0432) — seen, not this story's.
- **17-3a (`done`):** Q1 recipe (`q1-driver/q1-componentize-js-report.md` §Reproduce) — direct `npx componentize-js
  … --disable http fetch-event` (no `wasi:http` import); fuel on-start 303,158, one frame 2,617,112 of the 10 M
  default; componentize-js resolves relative imports only (bare specifiers need bundling). Q3's probe needed
  `--offline` (its `--locked` recipe did not reproduce, `q3-frame-bridge.log:18-22`). Phrase sub-agent tasks
  concretely and defensively (17-3a's Q4b slice was refused mid-flight).

### Git intelligence (`main`)

`4f677bc1` 17-6 (touched `crates/maos-bin/src/{admission.rs,main.rs,worker_spawn.rs}`, `crates/maos-wasm-host/{src/lib.rs,
tests/{t2_sandbox_kill,fuel_t2_matrix,equiv_harness}.rs}`, `crates/maos-host/src/lib.rs`, the cookbook en+ko,
`crates/maos-manifest/tests/cookbook_sections_17_6.rs`, `xtask/kloc.toml`, epics 17/18/20) · `f86b5d4f` records (17-3b
split; ADR-060 §D-C amended; PRD T4 delta) · `c0a397ea` / `5fa8843b` 17-3a · `a388fe62` operator decisions (rule 11,
ADR-068) · `92911f59` v0.1.0-alpha.1. Working tree clean at creation.

### Latest technical facts (fetched 2026-09-29 from npm, crates.io, nodejs.org)

- **`@bytecodealliance/componentize-js`**: latest **0.23.0** (2026-09-21, published from Node 24.20.0; depends on
  `@bytecodealliance/jco ^1.15.1`, `weval ^0.5.0`, `wizer ^10.0.0`; bin `componentize-js`). Pin exactly `0.23.0`.
- **`@bytecodealliance/jco`**: latest **1.35.0** (2026-09-24), still depends on `componentize-js ^0.22.0` — use jco
  for `guest-types` only.
- **`@napi-rs/lzma`** 1.5.1 (latest) engines `^22.20 || ^24.12 || >=25`.
- **Node**: 20 EOL 2026-04-30; 22 maintenance → 2027-04-30 (latest 22.23.3); **24 Active LTS → maintenance 2026-10-20,
  EOL 2028-04-30** (latest 24.21.0); 26 current (LTS 2026-10-28). Choose `'24'`.
- **wasmtime**: workspace locks **46.0.3** (`Cargo.lock:6139`; `decision_adrs_and_provisioning.rs:802-829` pins the
  post-RUSTSEC version) — latest is 49.0.1: **do not bump** in this story. `wit-bindgen` guests locked 0.44.0 (latest
  0.62.0) — keep 0.44.0 for the re-issue and record it. `wasm-tools` 1.259.0 (latest) — inspection only.
- Component-model linking: wasmtime applies semver-compatible lookup only when both versions describe the same API
  (`component/linker.rs:41-54`, 17-3a Q5); a record shape change is lock-step.

### Testing standards

Integration tests in `crates/*/tests/` (uncharged); deterministic; each asserts **why** (exit code + stderr name,
refusal code, byte equality). No test re-pins wording; wording-pin tests met on the way are deleted. CI-only inputs
follow rule 11(b): a missing input panics under `CI`, prints one named `SKIP` line off-CI. Planted-fault runs are
throwaway — commands, diff and output go in the Dev Agent Record.

### Project Structure Notes

- The bridge and runner live in `maos-wasm-host` (behind `maos-bin`'s off-by-default `wasm-host` feature — the
  export-control precondition; never enable it in a published build).
- The form reader lives in `maos-manifest`; the registry and maos-bin both call it — no second parser, no `toml`
  dependency in `maos-registry`.
- The TS scaffold is `templates/spirit-ts`; the example is generated. `sdks/spirit-ts` is retired by this story (Q3).

### References

- Epic `…/epics/epic-17-workers-and-third-party-form-w2.md` — header `:3-48` (rules 9–11 `:44-46`, schema `:40`, kloc
  `:42`), 17-3b `:97-107`, 17-3c `:109-118`, 17-3d `:120-128`, §R4.2 R17-18 `:238`, §R5 `:246-266`, part B `:268-282`,
  §R6 `:283-293`, §R7 `:295-309`, Dependencies `:311-319`.
- Spike `_bmad-output/implementation-artifacts/17-3a-wasm-recall-and-componentize-spike.md` (Go/No-Go `:256-265`,
  findings F3–F7/F10 `:372-379`) and `spikes/story-17-3a-wasm-recall/` (README; `bridge/`; `wit-1.1-frame/`;
  `ts-guest/`; `q1-driver/`; `evidence/`).
- 17-6 `_bmad-output/implementation-artifacts/17-6-t2-sandbox-repair-and-proven-red.md` (AC6 `:210-241`, D-17-6-E
  `:133`, review obligations `:313-340`, cut lines `:342-355`).
- ADR-060, ADR-031, ADR-032, ADR-045; `RELEASE-HOLDS.md:28,32-38`; PRD `functional-requirements.md` FR5 `:28`, FR33
  `:83`, FR36 `:86`, FR62 `:105`; `project-scoping-phased-development.md:18`, `:188`; `non-functional-requirements.md:34`;
  `requirements-inventory.md:568`; `epic-14-preflight-decisions.md` §D8 ruling (2026-09-29); `deferred-work.md:1050`.

## Tasks / Subtasks

- [ ] **T0 — Baseline (AC all).** Re-run `check-kernel-baseline` (25032) and `kloc-check --json` at the starting HEAD;
  if any figure moved from `baseline_commit`, re-derive the budget table before editing. Confirm `lib.rs:114` is still
  4 (else 17-1 landed schema 5 — drop D-17-3b-H's bump and add fields to v5).
- [ ] **T1 — Measure, then write the runner refusal (AC2, D-17-3b-C).**
  - [ ] Build today's runner and the spike's `@1.1`-frame bridge scratch; record the `@2`-host × `@1`-guest failure
    text and where it surfaces (link vs type-check) → `17-3b-evidence/t1-v1-guest-on-v2-host.log`.
  - [ ] Using the import walk (D-17-3b-C), write `IncompatibleWorld` (exit 2), fuel at
    instantiate → 4, `UnrepresentableFrame` (5); `spirit_frames_v1_world.wat`; doc `:1-62` rewritten.
- [ ] **T2 — The `@2.0.0` WIT and the lossless bridge (AC2, D-17-3b-A/B).**
  - [ ] Port the frames projection (no recall), add the two nested fields, rewrite header + world comment.
  - [ ] Port `bridge.rs.txt`, add nested fields, fallible `lower`, `Unrepresentable`, `BadHashLen`; delete dead
    variants and `scope_from_debug_string`; runner `:250`/`:263` call sites.
  - [ ] Oracle tests (all nested shapes, all 20 scopes, byte count recorded); delete lossy pins; hostile-guest vectors.
- [ ] **T3 — Re-issue the 4 components (AC2, D-17-3b-D).** Rebuild with a private target dir; `wasm-tools component
  wit` each (imports `frames@2.0.0`, record `wasi@`); re-pin 8 hashes; `[toolchain]` table + gate parse; delete the
  phantom text; `check-equiv-fixture-provenance` green; planted red (edit WIT w/o re-issue).
- [ ] **T4 — Corpus, equivalence, record (AC2, D-17-3b-E/F).** `wit_corpus.rs` counts + `intent-class`/`scope` gates;
  equiv harness F3 → ∅ and the rewritten forged-consent control; `check-wasm-form-equiv --json` non-vacuous both legs +
  tripwire; two ratification entries; the version-binding test + its planted `@2.0.1` red.
- [ ] **T5 — Manifest + schema 5 (AC3, D-17-3b-G/H).** `artifact` field + validation + named const; forms validator;
  `from_manifest_toml`; every `ClassSection {` literal (13); schema bump artifacts (lib.rs rows/doctests, tripwire, probe → 6,
  `gen-abi-docs`, `stability-matrix`, `manifest/v5.md`, `migrate/v4-to-v5.md` + ko); `manifest_field_coverage` row +
  3 fixtures; `check-manifest-schema-version` green; `compliance.rs:228-229`.
- [ ] **T6 — maos-bin admission (AC1/AC3, D-17-3b-I).** Fork, three variants + codes + Display; `UnknownClass` doc;
  `wasm_form_admission_17_3b.rs` (every AC3 vector, `maos run` default + `wasm-host` legs, the door row); planted red
  (engine refusal moved before gate 6).
- [ ] **T7 — Registry (AC4, D-17-3b-J).** `form_policy` in both entry points; two `AdmissionError` variants; T2 floor
  for wasm; vectors across tiers/attestation; `maosctl import` binary test; fr63 rows; module doc; ledger re-book;
  planted red (attested path unchecked).
- [ ] **T8 — Gates that pin the old world (AC3/AC4).** FKCS re-pinned to this story's reviewed diff (`check-fkcs --json` → `oracle_green: true`; D-17-3b-J(e)); ADR-060 anchors/fixture/vectors flipped with both directions still planted; ADR-060 Context + note;
  ADR-031 note; STABILITY fence sentence.
- [ ] **T9 — TS scaffold (AC1/AC5, D-17-3b-K).** Retire `sdks/spirit-ts` (package, CI step, `README.md:333`,
  `check_7_1_ts_template_baseline`'s SDK leg). Template rewrite (manifest, `wit/`, package.json, index.ts, `halt.ts`, tests,
  .gitignore, README); `templates-regen` regen; example `package-lock.json` regenerated under Node 24; dead regen code
  removed and tests adjusted; WIT-copy byte-equality test; three READMEs. `coverage-matrix --measure-nfr-test-3
  --dry-run` still reads 100 for `example-spirit-ts` (`tests/coverage-matrix.yaml:1775-1780`) because a manifest with
  no declared capabilities scores 100 (`xtask/src/coverage_matrix_nfr_test_3.rs:139-141`) — confirm, do not re-pin.
- [ ] **T10 — AC1 driver and CI (D-17-3b-L/M).** `ts_component_17_3b.rs`; `example-spirit-ts-tests` rewrite with
  annotations; one CI run (throwaway branch over SSH if needed) with every leg green, and the planted reds of AC1 once;
  run ids + annotation captures in `17-3b-evidence/`.
- [ ] **T11 — Cookbook (AC5, D-17-3b-N).** New en+ko page + index row; hello-world re-frame; reference block and
  `compliance-claim.md:94` fixed; `cookbook_manifests_17_3b.rs` replaces `cookbook_sections_17_6.rs`; `check-ko-coverage`
  and glossary-lock green.
- [ ] **T12 — Budget.** `cargo fmt --all`; `kloc-check`; exact raises (maos-bin; xtask if net > 0) with full-key
  comments; `check-kernel-baseline` 25032; the ceiling tests.
- [ ] **T13 — Records (landing commit).** Apply §Epic OLD→NEW edits; close `deferred-work.md:1050`'s section; tracker
  row; the full workspace suite (`cargo test --workspace --locked`) and `check-exit-commands` once, results recorded.

## Epic OLD→NEW edits

**Filed at creation (2026-09-29, operator Q5 directive)** — already applied: epic-17 `:30` (exit line 2 → 17-3c), §R8
R17-60…R17-67, the R8 notes in the 17-1/17-3c/17-3d sections and 17-5 AC5, epic-20 20-1 AC4, `requirements-inventory.md:568`,
tracker rows 17-1/17-3c/17-3d/17-5/20-1. **Still to apply in the landing commit** (they describe the landed state —
re-grep each anchor, symbol-first):

| Anchor | OLD | NEW |
|---|---|---|
| `epic-17…:40` manifest schema paragraph (applies once schema 5 has landed) | *"landed once, by whichever of 17-1 and 17-3b lands first"* | landed by **17-3b** (4 → 5, with `forms`/`artifact`); 17-1/17-2 add fields to v5 |
| `epic-17…:103-107` 17-3b ACs | the pre-creation cites | the measured cites of this story's ACs (P2, P8, P11, P13–P15, P21) |
| `xtask/kloc.toml:647` registry ledger | *"17-4 +60–150; 20-1 +230–440"* | add *"17-3b +<measured>"* |
| `deferred-work.md:1048-1050` | open | section closed (`heading_is_closed`), resolution naming this story |
| `sprint-status.yaml` 17-3b row | backlog note | done-note with run ids (at close) |

## Operator rulings and open questions

- **Q1 — zero-headroom raises: APPROVED 2026-09-29** (`maos-bin`, and `xtask` if net > 0), with every consequential
  test and xtask change applied (§Consequential test and xtask changes).
- **Q2 — FKCS / D8: RESOLVED 2026-09-29, outside this story, before it starts** (operator: *repin it now after
  reviews … do it properly*). No new story was needed: D8 is a decision-register item, discharged by a ruling plus a
  small gate change. Two non-author reviews found 13.4's `148a33ee` CONFORMANT-WITH-FIXES; the fixes (one shared
  `frozen_surface_gate`, three stale docs) landed first; the baseline was re-pinned `dfbbf748…` → `650f13b9…` and the
  hold deleted; `check-fkcs` is `oracle_green: true`. This story now re-pins its **own** reviewed diff (D-17-3b-J(e)) and
  takes the review's routed finding on the line-based FKCS parse (D-17-3b-J(g)); the attestation-binding finding went
  to 20-1. Evidence: `epic-14-preflight-decisions.md` §D8 ruling; ADR-052 and ADR-056 2026-09-29 amendments.
- **Q3 — `@maos/spirit-ts`: RULED 2026-09-29 — RETIRE** (operator: *retire it as you recommended*). The v0.5 SDK
  models an API no runtime runs (`onIdle`/`Ctx`/`MockBus`) and was never published, so scaffolds generated outside the
  repo could not `npm ci` (ADR-043 `:33`). The WIT world is the one TS contract; the scaffold is self-contained
  (D-17-3b-K). A future WIT-generated, published TS kit gets its own story when there is demand — none is owed now.
- **Q4 — registry floor: CONFIRMED 2026-09-29** — `wasm-component` → `sandbox_tier_floor = T2` at every tier;
  `t3_for_public_untrusted` does not apply (epic §R8 R17-61).
- **Q5 — scope folds: CONFIRMED 2026-09-29, and filed so no later story misses them** — R17-39(a)(d) are this
  story's (epic §R8 R17-60; 17-3c's section and tracker row say so), `compliance-claim.md:94` is fixed here
  (R17-64). Every consequence for other stories was filed **at creation**, not at landing: epic-17 §R8
  R17-60…R17-67, notes in the 17-1 / 17-3c / 17-3d / 17-5 sections, epic-20 20-1 AC4, `requirements-inventory.md:568`,
  and the 17-1 / 17-3c / 17-3d / 17-5 / 20-1 tracker rows.

## Dev Agent Record

### Agent Model Used

### Debug Log References

### Completion Notes List

- Ultimate context engine analysis completed — comprehensive developer guide created (2026-09-29, story creation;
  six scouts + direct measurement at `4f677bc1`).
- Checklist validation 2026-09-29 (fresh-context non-author reviewer): 5 critical fixes applied (ignored-by-default AC1
  driver so two Blocking jobs cannot panic; refusal codes lead the Display so `maos run` shows them; `maosctl` built
  for the door leg; the hello-world block completed to an admissible manifest; ko blocks made byte-identical), 9
  enhancements and ~20 drifted cites corrected. Baselines at HEAD: `check-exit-commands` PASS (no Epic-17 token owed);
  `decision_adrs_and_provisioning` 3/3; `check-fkcs` passed only through the held advisory (`oracle_green: false`) — since fixed by the D8 discharge (Q2).

### File List

### Change Log

- 2026-09-29 — Story created (`backlog` → `ready-for-dev`).
