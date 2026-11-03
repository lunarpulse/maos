# 20-8 creation — cites re-measured at `8c809f9c` (2026-11-03)

Method: each cite was re-read in a `git archive 8c809f9c` copy at `/tmp/proto208/base`, not in the working
tree. The working tree carries the uncommitted 20-4 L1 edits by `Dev204L1`.
`crates/maos-manifest/src/manifest.rs` differs from HEAD only at `:2413` and in two test literals
(`:4325-4337`, `:4388-4399`). The line count is unchanged, so every manifest cite below holds in
both trees. `crates/maos-bin/src/admission.rs` differs in the worktree (20-4 `[providers]` gate). The
`gate`/`wit` prototypes therefore apply to HEAD, not to the worktree. The dev re-cuts them.

E20 = `_bmad-output/planning-artifacts/epics/epic-20-ship-it-w5.md`; PF = `epic-20-preflight-2026-10-30.md`;
S = `sprint-status.yaml`; D = `deferred-work.md`; RA = `routing-audit-2026-10-22.md`; RI = `epics/requirements-inventory.md`.

## C — corrections (OLD → NEW)

| # | Where | OLD | NEW at `8c809f9c` |
|---|---|---|---|
| C1 | E20:258, PF:242 | "Manifest fields missing at `crates/maos-manifest/src/manifest.rs:228-237,2423-2454`" | The four fields are absent from the whole tree: `grep -w` finds 0 hits for `explanation_shape`, `intent_promotion_set` and `swap_invariants` in `crates/*/src` and `spirits/*/src`, except `maos-eval/src/hsis_corpus.rs:55` (an HSIS fixture field). `skills` exists only as CLI/discovery text. The cited ranges are `ClassSection`'s tail fields (struct `:223-238`) and `ProvidersSection`/`ProviderConfig` (`:2423-2428`, `:2446-2455`). Neither range is where the fields belong. New sections go after `HaltProtocolCompatibilitySection` (`:4087-4101`). The schema list is `POST_V1_SCHEMA_SECTIONS` `:336-337` |
| C2 | E20:261, PF:243 | `deferred-work.md:1210-1216` | `:1342-1350` (§ "Received from 18-7-live-skill-authoring-and-revisions", row `:1344-1350`). `:1208-1216` is now the 17-7 T3 flake row |
| C3 | E20:258, RA:22, RI:593 | "After 20-1" | 20-1 was split on 2026-11-01/02 (D-20-1-01 B). The reused artifact admission is 20-1a's, at S:296 `ready-for-dev` with landing 1 implemented: `verify_publisher_sig` (`crates/maos-registry/src/admission.rs:546-562`), `install::verify_installed` (`crates/maos-registry/src/install.rs:109-150`), boot re-verification `crates/maos-bin/src/installed_spirits.rs:16-124`. 20-1b (S:297, `backlog`) changes admission identity (its AC10). See D-20-8-02 |
| C4 | RA:22, RI:593, S:308 | "unsigned admission" | Partly stale. A registry-installed package is signature-verified at install (`install.rs:61`) and at every boot (`:116`, manifest and artifact bytes compared `:119-123`). Unsigned admission remains for local manifests (`admission.rs:310-338`, `allow_unsigned_local`; `maos run`/door load read local files). Admission journals only `VetterKey` {id, version, tier, note} (`crates/maos-bin/src/admission.rs:63-117`, call sites `:926-947`). It records no manifest digest, no signer and no declarations |
| C5 | E20:261 | "`wit/spirit.wit:378-381` currently imports recall/context" | Holds. Package `:30` `maos:spirit@2.2.0`, world `:378-391`, imports `:380-381`, exports `:386-390` |
| C6 | E20:261 | "Consume schema v6 `[capabilities.required.skill]`" | Exists: `SkillCapabilities` `manifest.rs:626-632`, listed in `POST_V1_SCHEMA_SECTIONS` `:337`, minted as `Scope::SkillAuthorSelf` by `capabilities_required_to_scopes` `:884-935`. Native port: `crates/maos-bin/src/skill_authoring.rs:27-294` (trait `spirits/researcher/src/skill_author.rs:156-169`) |
| C7 | E20:81, PF:82, PF:126 | "kernel-Δ measured FLAG-Winston" / "No numeric forecast" / "Yes; unmeasured" | Priced by reverted prototype (`measurements.md`): KS +38 / +33, KB +10 / +8, KB4 +34 / +32, K0 0. Out-of-kernel measured: manifest +104, bin +156, wasm-host +51, domain +11, xtask +4 |
| C8 | D:1346 | "`maos-wasm-host` bridge (zero headroom at 2412)" | Holds: pin `xtask/kloc.toml:703` = 2412 = measured |
| C9 | E20:260 | "verified, admitted and journaled" (implied existing journal) | No FR8 declaration is journaled at HEAD (C4). `emit_vetter_key_event` is the only admission row |
| C10 | E20:261 | "WIT skills author/propose import, host bridge" | The in-band channel exists: `RecallRequest`/`RecallReply` (`crates/maos-wasm-host/src/recall_wire.rs:69-131`), host client `host_state.rs:61-82`, daemon `serve_wire` (`crates/maos-bin/src/recall_mediation.rs:266-359`, context arm `:277-280`), session dispatch `spirit_session.rs:461-480`. Semver-compatible minor imports already resolve (`runner.rs:359-373`) |
| C11 | I13 (`docs/invariants/I13.md`, `crates/maos-domain/src/invariants/i13.rs:9-12`) | "v0.5: runtime — consumer admission rejects with `EIntentPromotionDenied`" | No production caller. `DistillationPort::admit_for_consumer` (`crates/maos-iac/src/adapter/distillate.rs:430-465`) is called only from tests (`maos-iac` inline, `maos-kernel-core/tests/distillation_i11_audit_chain.rs:176,193`, `spirits/orchestrator/tests/distillate_dispatch.rs:181`). The one digest-consumption path at HEAD is `TaskAssign.prior_distillate_ref` (`crates/maos-iac/src/adapter/orchestrator_dispatch.rs:62-140`, called `adapter.rs:406`). Distillate frames (kind 11) are not recallable through `MediatedLogRecall` (participant kinds 0–6, 18; cookbook `wasm-component-spirit.md:87`) |
| C12 | architecture `5-spirit-abi.md:44-46` | `[explanation_shape]` "for decision.* frames" | No frame carries an explanation body. `DecisionDispatchPayload` = {decision_id, approved, working_memory_digest_refs} (`crates/maos-domain/src/frame.rs:208-217`). No emit-side shape check exists for WASM sessions (`spirit_session.rs`: no `OutputShape` use). See D-20-8-05 |
| C13 | architecture `5-spirit-abi.md:69-70` | `[skills.search_path] paths = [...]` | The roots are fixed (`crates/maos-skill/src/discovery.rs:38-49`, `default_search_path`). No manifest field reads them. See D-20-8-04 |
| C14 | architecture `5-spirit-abi.md:88-89`, PRD `developer-tool-specific-requirements.md:165` | `[swap_invariants] preserve = [...]` (HSIS) | HSIS fixtures key invariants by name (`crates/maos-eval/fixtures/hsis-corpus-v0`: 300 scenarios `{"open_pr_state": [], "review_queue": []}`). The swap coordinator reads `halt_protocol_compatibility` only (`crates/maos-kernel-core/src/hot_swap/coordinator.rs:151-188`). The successor is parsed by the kernel's own `load_bundle_from_file` (`crates/maos-kernel-core/src/lifecycle/upgrade.rs:307-429`, caller `coordinator.rs:464`), which yields `SpiritManifestBundle` (`scheduler/control_block.rs:195-238`) |

## F — facts the story relies on (held at `8c809f9c`)

| # | Fact | Cite |
|---|---|---|
| F1 | `gate_manifest` parses every admission section; `GatedManifest` carries them; `bundle()` feeds the SCB | `crates/maos-bin/src/admission.rs:438-497,590-725`; `optional_section` `:727-745` |
| F2 | `load_admit_start` runs FR11, model provenance, load, admit, journals the grant, archives the manifest text | `admission.rs:838-1014` (grant `:940-947`, archive `:950`) |
| F3 | Schema constants; gate forbids literal compares in `manifest.rs`; ratified post-v1 list | `crates/maos-spirit-abi/src/lib.rs:122,143,171`; `xtask/src/check_manifest_schema_version.rs:1-48` |
| F4 | `GovernanceEventKind` is `#[non_exhaustive]`, tag `governance_type`; no kernel match | `crates/maos-domain/src/governance.rs:170-189`; `grep GovernanceEventKind:: crates/maos-kernel-core/src` = 0 |
| F5 | `HotSwapError` lives in maos-domain | `crates/maos-domain/src/hot_swap.rs:90-133` |
| F6 | Skill port binds pid/class, attaches telemetry evidence itself, journals refusals | `skill_authoring.rs:92-193` (evidence `:166-168`), receipts `:195-228` |
| F7 | WASM session deps and launch | `spirit_session.rs:60-82,84-152`, recall construction `:356-369` |
| F8 | ABI ratification precedent for a WIT minor | `xtask/abi-ratifications.toml:259-277` (`18-5-AC3-wit-spirit-2.2.0`) |
| F9 | Hosted jobs | `.github/workflows/discipline.yml:2898` `epic-20-exit`, `:4550` `wasm-host-tests`, `:5236` `check-wasm-form-equiv` |
| F10 | Tamper witnesses exist (20-1a) | `crates/maos-bin/tests/installed_spirit_dispatch_20_1.rs:82`; `crates/maos-cli/tests/spirit_install_20_1.rs:18` |
| F11 | Guests under `crates/maos-wasm-host/guests/` are charged to maos-wasm-host; `tests/`, `spirits/`, `examples/` are not | `xtask/src/kloc_check.rs:294-317`; `kloc.toml:702` |
| F12 | Kernel pins | `xtask/kernel-core-baseline.toml` 26841 / 95 files / set `db39bc4a…`; `xtask/kloc.toml:264` 21042 |
