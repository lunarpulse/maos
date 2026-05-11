# Developer Tool Specific Requirements

MAOS' primary classification is `developer_tool` (per Step 2). The substrate's customers are *Spirit authors, kernel implementers, and operators*. Secondary traits — `cli_tool` (`maosctl`), `api_backend` (ACP server, A2A peer, control-plane HTTP, MCP outbound), `desktop_app` (one Host process per machine) — surface as integration concerns, not primary product shape. This section pulls the developer-tool-specific commitments scattered across the architecture and implementation guides into a single PRD reference, plus the operational, testing, and documentation commitments needed to ship a substrate-class OSS developer tool credibly.

## Project-Type Overview

The Spirit-author-as-customer relationship is load-bearing. The substrate's value compounds when third-party Spirit authors can ship Spirit binaries independently of the MAOS source tree, in any language, signed, with capability scopes and trust tiers verified at install. Two adjacent customer relationships sit alongside:

- **Kernel implementer** — the Rust developer building the `maos` binary itself. Audience for `maos-kernel-implementation-guide.md`.
- **Operator** — the person running MAOS on a Host. Audience for `maosctl` and the deployment-topology docs.

The three customers share Spirit ABI as the contract; they diverge on tooling needs.

## Language Matrix

Three Spirit forms over the v0.1 → v1.0 → v2.0 timeline (per ADR-007):

| Form | Phase | Languages | Toolchain | Reference |
|---|---|---|---|---|
| `rust-inproc` | v0.1+ | Rust only. Spirit binary linked into kernel binary. | `cargo build` against `maos-spirit-sdk`. | `spirit-development-and-sharing.md` §4.1 |
| `subprocess` (incl. CLI-wrapper Spirits per ADR-014/015) | v0.5+ | Any language with a Spirit Wire Protocol implementation. Reference SDKs: Rust (canonical), TypeScript (v0.5), Python (v1.0), Go (v1.5+). For CLI-wrapper Spirits: any agent CLI process loaded with `maos-bridge` + persona skills. | Spirit-author's preferred toolchain; `spirit-test` SDK harness. | `spirit-development-and-sharing.md` §4.2; ADR-014, ADR-015 |
| `wasm-component` | v2.0+ | Any WASM Component Model language: Rust, C/C++, JS/TS (Jco), Python (componentize-py), Go (TinyGo). | `cargo component` or language-specific component-model toolchain. | `spirit-development-and-sharing.md` §4.3; ADR-007 |

**Cross-form portability commitment.** Same crate, three feature flags, shared core: `cargo build --features=form-{rust-inproc,subprocess,wasm-component}` — author writes against `Spirit` once, form glue is feature-gated. *Capability scopes are not portable*: a Spirit calling `std::process::Command` builds under `subprocess` and `rust-inproc` but is rejected by the `wasm-component` build at compile time. Manifest declares `forms = ["subprocess", "rust-inproc"]` if WASM is impossible; the registry refuses WASM builds for that class. Don't promise three-form portability as default — promise *form-explicit* portability where the author opts into the forms they support, and `spirit-test` is the source of truth.

Skill packages are markdown + frontmatter — language-agnostic by design. Bridge skills require no per-language compilation.

## Installation Methods

**Kernel (`maos` binary):**
- v0.1: source build (`cargo install --path crates/maos-bin`); Linux + macOS only.
- v0.5: pre-built binaries for Linux (amd64, arm64) and macOS (arm64) via GitHub Releases. SHA256 + Ed25519 signature verification mandatory.
- v1.0: Homebrew tap, AUR (Arch), Debian/Ubuntu deb, RHEL/Fedora rpm. Container images on Docker Hub / GHCR. Windows binary at v1.5.
- v2.0: official Linux distro packages (Debian/Ubuntu main, Fedora repo). One-line install script (`curl install.maos.dev | sh`) for the founder-loop demo.

**Spirits:**
- Reference Spirits ship with the `maos` binary.
- Third-party Spirits install via `maosctl install <spirit-id>[@version]`. Per ADR-008: MCP-Streamable-HTTP call to the Spirit registry; Ed25519 signature verified; trust-tier floor enforced (ADR-009); ComplianceClaim verified at admission (ADR-015 / Step 5); manifest validated.
- Custom Spirits load from local filesystem (`maosctl install --from-path ./my-spirit/`).

**Skills:**
- v0.1–v1.0: filesystem only; conventional locations (`~/.maos/skills/`, `_bmad/skills/`, `/usr/share/maos/skills/`).
- v2.0: optional skill registry, separate from Spirit registry, content-addressed with Ed25519 signing.
- BMAD skills (`bmad-create-story`, `bmad-dev-story`, `bmad-code-review`, etc.) work as-is — no MAOS-specific port required.

**Agent CLIs (Worker Spirits):** brought in via existing distribution — `npm install`, `pip install`, `brew install`. MAOS does not redistribute or vendor agent CLIs.

## Spirit Lifecycle — install, upgrade, yank, uninstall, revoke

A first-class commitment: every install verb has a documented inverse and a kernel-side enforcement story. The substrate's install promise obligates the substrate's revoke promise (Mary's launch-day-embarrassment gap).

| Verb | Trigger | Kernel behavior | Effect on running instances | Effect on claims/audit |
|---|---|---|---|---|
| **install** | `maosctl install <spirit-id>[@version]` | Pull from registry → verify signature → verify ComplianceClaim envelopes if present → enforce trust-tier sandbox floor → register Spirit class in local index | None (this is admission, not instantiation) | None |
| **upgrade** | `maosctl upgrade <spirit-id>[@version]` | Pull new version → verify → if running instances exist, defer to `maosctl swap` (hot-swap with ADR-020 migration policy applied) | Hot-swap per ADR-017/020/I14; running instances migrate or drain | Outstanding ComplianceClaims re-verified against new runtime context (ADR-015); migrate forward if compatible, halt if drift detected |
| **yank** | Registry author marks version withdrawn (publication event) | Kernel polls registry every 5 min for yank events; on yank, emits typed `SpiritYanked{spirit-id, version, reason}` to operator surface; **does NOT auto-stop running instances** | Instances continue running unless operator explicitly stops; operator notification is mandatory | Audit log records the yank event with timestamp and reason |
| **uninstall** | `maosctl uninstall <spirit-id>[@version]` | Remove from local index; refuse if running instances exist (force flag required: `--force`); refuse if outstanding ComplianceClaims (review flag required: `--orphan-claims`) | Operator must stop instances first (or use `--force` with explicit confirmation) | Outstanding ComplianceClaims become orphaned and tagged as such in the Approval Decision Log; audit trail preserved (Spirit history not deleted) |
| **revoke** | Operator-issued or registry-issued *trust event* (signed, dated, distributable offline) | Kernel honors revocation list (CRL-shaped artifact: `(spirit-id, version, revocation-key, reason, ts)` signed with operator or distributor key); on revocation, **immediately blocks new instantiation and emits typed `SpiritRevoked` to all running instances** | Running instances receive `SpiritRevoked` and follow declared policy: `terminate-immediately` (default for security) / `drain-then-terminate` (configurable per Spirit posture) / `quarantine` (running but no new tool calls) | Audit log records the revocation event with full chain |

**Signed Revocation List (CRL artifact).** v1.0 ships with two distribution paths: (a) registry-pushed (kernel polls registry's `/revocations` MCP endpoint every 5 min); (b) offline-import (`maosctl revocations import <bundle.crl>` for air-gapped deployments). CRL signing follows the same Ed25519 chain as Spirit signing; operator can pin trusted revocation signers.

**The yank vs revoke distinction.** Yank is a registry-side publication event (the registry will not serve this version to fresh consumers); revoke is a kernel-side trust event (the kernel will not run this version on this Host regardless of where it came from). They are different artifacts with different signing chains. The PRD commits to both.

**Substrate Operations Checklist** (Mary's organizing line):

| Concern | Owner | Target version | Artifact |
|---|---|---|---|
| Install/upgrade UX | core team | v0.1 | `maosctl install` + tests |
| Yank notification | core team | v0.5 | Registry polling + operator notification |
| Uninstall semantics | core team | v0.5 | `maosctl uninstall` with claim-check guards |
| Signed revocation list | core team | v1.0 | CRL artifact spec + distribution paths |
| Audit query / SIEM export | core team | v1.0 (basic), v2.0 (signed export) | `maosctl audit query` + sealed-export |
| Telemetry opt-out | core team | v1.0 | Opt-in default + `PRIVACY.md` + per-field redaction layer |
| LTS window | maintainer team | v1.0 announcement | `STABILITY.md` + LTS branch policy |
| Namespace grammar | architecture | v0.5 | New ADR (flat vs scoped Spirit names) |

## Namespace Grammar (Mary's gap)

**Commitment:** by v0.5, an ADR locks the Spirit namespace grammar — flat names (`bmad-orchestrator`) vs scoped (`@bmad/orchestrator`, `org.bmad.orchestrator`). Without a grammar, the first publication race decides the namespace forever and trademark / squatting become permanent. Default working assumption: scoped (`@scope/name`) following npm/Cargo convention; final lock in the v0.5 ADR.

## API Surface (Spirit ABI)

Three logical surfaces backed by `maos-spirit-abi` (per `maos-kernel-implementation-guide.md` §3.2):

**1. The `Spirit` trait — kernel calls into Spirit.** Lifecycle hooks: `on_load`, `on_start`, `on_frame`, `on_telemetry`, `on_idle`, `on_swap_in`, `snapshot`, `epistemic_resolve`, `on_pause`, `on_resume`, `on_unload`. Plus, per ADR-020, optional `migrate(predecessor_state)` for cross-major migration.

**2. The `KernelHandle` trait — Spirit calls into kernel.** IAC (`iac.send`/`iac.receive`/`iac.broadcast` with ADR-012 typed-intent consent), Memory (`memory.read`/`memory.write` with I5/I11/I13 enforcement), Capabilities (`capability.invoke`), Provider (`provider.stream`), Log (`log.recall`/`log.fetch` per ADR-013), Halt (`epistemic.halt`), Approval (`approval.request`).

**3. Manifest schema (TOML).** `[class]`, `[capabilities.required]`, `[posture]`, `[output_shape]`, `[explanation_shape]`, `[epistemic_policy]`, `[budget]`, `[skills.search_path]`, `[forms]` (cross-form portability declaration), `[hot_swap]` with `state_schema_uri` + `state_schema_version` (ADR-017), `[halt_protocol_compatibility]` (I14), `[intent_promotion_set]` (I13), `[migrates_from]` (ADR-020), `[swap_invariants]` (HSIS — Murat's gate). Full schema in `architecture-maos.md` §5.1.

## ABI Stability Triple (Winston's commitment; matrix in `STABILITY.md`)

Compatibility is `(kernel_version, abi_version, manifest_schema_version)` — a triple, not a pair. `abi_version` governs the `Spirit`/`KernelHandle` vtable + capability ID space; `manifest_schema_version` governs the TOML surface independently; `kernel_version` is product-facing.

**Rule:** Spirit declares `abi`; kernel adapts down via `Compat` shim; **N-1 supported, N-2 hard refusal** with typed `EAbiTooOld`.

**Deprecation timeline:** 2 minor releases of warning, 1 major to remove. Spirit-side `kernel.deprecation_warnings()` channel surfaces deprecations in `spirit-test`.

**Live matrix:** lives in `STABILITY.md` (separate doc; grows over time without re-approving the PRD). PRD commits to the triple's existence and the N-1/N-2 rule.

## CLI-Wrapper Spirit Specification (Winston's gap; ADR-021)

CLI-wrapper Spirits (Path A migration) use the kernel-builtin `CliWrapperSpirit` class, configured with: CLI binary path; skill bundle (`maos-bridge` + persona skills); **`output_shape_version: "<semver>"`** (ADR-021 — kernel asserts on startup; refuses to start with typed `EOutputShapeAdapterMismatch` if observed != declared); posture declaration (stdio shape, control-channel mechanism, shutdown signal); capability scope mapping; output-shape adapter implementation (registered as `cli-wrapper-template:<cli-name>:<shape-version>` in the Spirit registry); crash semantics (kernel observes EOF on stdio + non-zero exit → `SpiritDied` event journaled; recovery policy declared in wrapper config: `respawn-with-context` / `respawn-fresh` / `escalate`).

**Fail-loud rule:** wrappers cannot fall back to "best-effort parsing" on shape mismatch. Audit drift is the failure mode the substrate cannot tolerate.

**Realistic 30-minute claim:** valid only when a wrapper template for the target CLI already exists in the registry. First-time-wrapping a net-new CLI class (kimi-cli, codex, future CLIs) is **half-day minimum** because the author is also authoring the output-shape adapter. The PRD distinguishes both numbers honestly.

## Hot-Swap Migration Policy (Winston's decision tree; ADR-020)

Four cells keyed on (schema-evolution × persistent-state):

| Schema evolution | No persistent state | With persistent state |
|---|---|---|
| Same major, additive | Auto-migrate | Auto-migrate |
| Same major, breaking | Forbidden (use major bump) | Forbidden (use major bump) |
| Cross-major, no archives | Swap permitted; predecessor archives refused | N/A |
| Cross-major, archives present | Migrator Spirit required | Migrator Spirit required |

Manifest field `migrates_from = ["1.x", "2.x"]` declares which predecessor versions a Spirit can hot-swap from. Cross-major migration with persistent state requires a `migrate(predecessor_state) -> Result<successor_state, Error>` entry point. Kernel refuses load with `EMigratorMissing` if predecessor archive exists and no migrator is declared. **Predecessor's historical journal stays in cold storage**, addressed by `(class, version, instance_id)`; successor reads via capability but does not own (preserves I10 across version boundaries).

## Code Examples

Three canonical examples in the Spirit-development guide:

1. **Minimal Rust in-process Spirit** (~40 lines + manifest) — the "30-minute first Spirit" tutorial in §4.1.
2. **Subprocess Spirit in TypeScript** — Diego's `code-reviewer-pro` (Step 4 J6); demonstrates JSON-RPC over stdio Spirit Wire Protocol, output_shape enforcement, signing for `public-untrusted` registry submission.
3. **Skill-package overlay** — `developer` + `maos-bridge` skills loaded into a `claude-code` process; demonstrates Pattern A (Bash-invoke external CLI) and Pattern B (direct slash-command).

**Worked example — the founder's epic-7 loop.** End-to-end trace from `@orchestrator run epic-7` through `task.assign` IAC frame routing, Worker Spirit skill loading, distillation pattern execution, halt-on-AC-ambiguity resolution, and morning digest production.

## Migration Guide

Two paths:

**Path A — agentic CLI tool already exists.** Install `maos-bridge` skill + persona skill; configure CLI to start with both loaded; declare `output_shape_version` (ADR-021); register with `maosctl spirit register --form=cli-wrapper`. **30 minutes for CLIs with published wrapper templates; half-day for net-new CLI classes** (output-shape adapter authoring).

**Path B — third-party agentic framework or tool.** `cargo generate maos-spirit --form=subprocess --lang=<lang>`; implement `Spirit` trait or Wire Protocol equivalent; author manifest declaring capability scopes, posture, output_shape, epistemic_policy; run `spirit-test`; publish: `maos-spirit publish --tier=public-untrusted`; iterate to community-vetted via attestations (ADR-009). **Effort: weeks per port, mostly behavior-code authoring.**

Diego's *"Why I deleted 4,000 lines of HTTP/SDK glue code by becoming a MAOS Spirit"* (Step 4 J6) is the canonical Path-B success narrative.

## Numeric Ship-Gate Floors (Murat's audit)

Every developer-tool quality claim gets a falsifiable numeric floor. Aspirational language ("substrate quality") is replaced with verifiable thresholds.

| Gate | Floor | Phase | Owner |
|---|---|---|---|
| `spirit-test` fixture-corpus pass rate | ≥ 98% | v1.0 ship to public-untrusted | Spirit author |
| `spirit-test` class regression-corpus pass rate | ≥ 95% | v1.0 (corpus sealed by registry) | Spirit author |
| Manifest self-check (declared output_shape matches produced) | 100% | v0.1 | Spirit author / kernel |
| Cross-form Semantic equivalence (rust-inproc ↔ subprocess) | ≥ 90% on 200-scenario class corpus | v1.0 | Cross-form harness |
| Cross-form Semantic equivalence (any-rust ↔ wasm-component) | ≥ 75% (lower because wasm has different determinism) | v2.0 | Cross-form harness |
| CLI-wrapper Behavioral-Distributional equivalence | Mann-Whitney U-test p > 0.05 over 30 runs per scenario | v1.0; separate registry label `conformance: behavioral-distributional` | Wrapper author |
| ABI compatibility matrix coverage within current major | 100% (every minor pair, both directions) | v0.1 | Kernel team |
| ABI N-1 major boundary | 100% incl. negative typed-error cases (`EAbiTooOld`) | v1.0 | Kernel team |
| Manifest field test coverage | ≥ 3 cases per field (well-formed / malformed-rejected / edge-case) | v0.1, CI-enforced | Kernel team |
| **Manifest parser fuzz** | 24h `cargo-fuzz`, zero crashes / OOMs / infinite loops | **v1.0 ship gate** (log4shell territory) | Kernel team |
| Wire protocol cross-language byte-equal golden corpus | 100% per frame variant per SDK (Rust / TS / Python / Go) | v1.0 | Kernel + SDK teams |
| Wire protocol schema-evolution coverage | 4 cases per frame variant (old→new additive; new→old additive-only; new→old with deprecated; new→old breaking → typed reject) | v1.0 | Kernel team |
| Wire protocol adversarial-input fuzz | 24h fuzz, zero crashes | v1.0 ship gate | Kernel team |
| **Hot-Swap Invariant Suite (HSIS)** — pass rate per Spirit class | **≥ 95%, zero invariant violations (CVSS-7 class)** | **v1.0 ship gate** | Spirit author + kernel |

**HSIS specification.** For every Spirit class, run a 50-scenario corpus where a swap is injected at randomized lifecycle phases. Floor:
1. Successor emits `on_swap_in` ack within 100ms of swap signal.
2. Successor preserves declared invariant set (manifest `swap_invariants: [...]` field).
3. Final task output passes ≥ 95% Semantic equivalence vs no-swap control.
4. Total work-product divergence within manifest-declared `tolerance_band`.

**Cross-language byte-equal golden corpus.** Every frame variant gets a `golden/<frame_name>.json` committed to repo; every language SDK serializes a constructed frame → byte-equal golden; deserializes golden → structurally-equal frame. Canonical encoding: sorted keys, no whitespace, UTF-8 NFC.

## Typed Error Catalog (Paige's must-not-ship-without)

The substrate's most-trafficked reference page must exist on day one. **PRD ship-gate commitment:** every typed error declared in `maos-spirit-abi` has a corresponding catalog page generated from a structured docstring; CI fails if a new error variant lacks the catalog metadata.

**Catalog format (per error):**
- Error name (`EDigestAuditChainMissing`)
- One-line description
- What caused it (typically: which kernel check failed, which precondition was violated)
- How to recover (Spirit-author-side fix)
- Code example of the trigger
- Code example of the handler
- Related errors (cross-references)
- Stability: which kernel version introduced it; deprecation status if any

**Stable URL pattern:** `https://docs.maos.dev/errors/<ERR_NAME>`. Versioned per kernel release; archived versions retained ≥ 2 minor releases back.

**v1.0 covered errors (current named set, will grow):** `EDigestAuditChainMissing`, `EIntentPromotionDenied`, `EHaltContinuityViolation`, `EContextExhausted`, `EComplianceContextDrift`, `ESwapSchemaMismatch`, `ERegionLockViolation`, `EAbiTooOld`, `EMigratorMissing`, `EOutputShapeAdapterMismatch`, `SpiritYanked`, `SpiritRevoked`, `SpiritDied`, `EChannelClosed`. CI lint enforces the catalog metadata presence per variant.

## Documentation Artifacts (Paige's missing-list)

**Diátaxis honesty.** The current doc set is *Diátaxis-aware at the section level, Diátaxis-violating at the document level*. The PRD does not claim Diátaxis compliance; it commits to specific artifacts that move toward it.

| Artifact | Form | v0.5 | v1.0 | v2.0 |
|---|---|---|---|---|
| **API reference site** at `https://docs.maos.dev/abi/<version>/` | `cargo doc` published to GitHub Pages on every release tag, with `abi/latest` alias; versioned, searchable (Algolia DocSearch or Pagefind), deep-linkable | ✓ basic | ✓ search + version dropdown + archived ≥ 2 minor back | ✓ multi-locale builds |
| **Manifest schema reference (human-rendered)** | Rendering of the JSON Schema with examples; every field documented | ✓ | ✓ comprehensive | ✓ multi-locale |
| **Typed error catalog** (per Paige; see above) | One page per error; CI-enforced metadata | ✓ initial set | ✓ all errors covered | ✓ multi-locale |
| **Pattern cookbook** | Orchestrator pattern, distillation pattern, multi-CLI parallelism, halt-on-AC-ambiguity, plus future patterns | partial | ✓ initial canonical patterns | ✓ community contributions |
| **Migration runbooks** (Path A / Path B + per-source-tool runbooks) | Preconditions / step list / verification / rollback / known failures | sketches | ✓ Path A + Path B fully run-bookable | ✓ per-tool runbooks (LangChain, Cursor, etc.) |
| **Troubleshooting guide** | Symptom → cause → diagnostic command → fix; cross-references typed error catalog | partial | ✓ comprehensive | ✓ multi-locale |
| **Deployment topology guide** | Solo / team / Cortex shapes; how-to flavor (vs architecture §11 reference flavor) | sketches | ✓ comprehensive | ✓ multi-locale |
| **`LOCALES.md`** with glossary lock | Translation contribution flow; terms never translated (`Spirit`, `Worker`, `kernel`, ADR ids, error codes); review process; staleness policy | ✓ | ✓ | ✓ |
| **Doc tooling pipeline** | mdBook + i18n / Docusaurus / VitePress with versioning — pick one | pick + commit | ✓ in production | ✓ |
| **Three-door page** at `docs.maos.dev` | "I want to write a Spirit" / "I want to run MAOS" / "I want to understand MAOS" — reader-task-first navigation | ✓ | ✓ | ✓ |

**Localization v1.0 targets:** Korean (shipped); Japanese (paperclip + Spirit-author overlap); Chinese-simplified (kimi-cli community leverage). Spanish/German/French defer to community pull.

**Doc-quality coverage targets (v1.0):**
- Every public ABI method has ≥ 1 doctested example (CI-enforced).
- Every typed error has cause / recovery / example trigger / example handler (CI-enforced).
- Doc site builds on every kernel PR; broken links + out-of-sync code samples block merge.
- WCAG AA — color contrast, keyboard nav, screen reader on code blocks, alt text on every diagram including Mermaid renderings.

## 30-Minute First Spirit Validation Gate (Paige's gate)

The "Build your first Spirit in 30 minutes" tutorial (`spirit-development-and-sharing.md` §4.1) becomes a v1.0 ship gate, not aspirational copy.

**Validation protocol:**
- 5 Spirit authors, **none of whom are MAOS contributors**
- Fresh machine (no Rust toolchain assumed; install commands part of the tutorial)
- Recorded sessions (with consent) reviewed for friction points
- **Floor: ≤ 45 minutes median, ≤ 90 minutes p95**
- Tutorial revised iteratively until target met
- Re-validation required after every breaking tutorial update (any change touching install, manifest, or first-Spirit code template)

If the floor is not met by v1.0, **the substrate ships with a tightened tutorial scope** (e.g., "Build your first Spirit in 60 minutes") rather than ship the unverified 30-minute claim.

## Onboarding and Governance Artifacts (Mary's gaps)

| Artifact | Phase | Owner |
|---|---|---|
| `RFC_TEMPLATE.md` | v0.5 | Maintainer team |
| `GOVERNANCE.md` (maintainers list, lazy-consensus on RFCs, tiebreak by maintainer vote) | v0.5 | Founder + initial maintainers |
| `CODE_OF_CONDUCT.md` (Contributor Covenant baseline) | v0.5 | Maintainer team |
| `PRIVACY.md` (telemetry schema, retention, jurisdiction, deletion path; GDPR Art. 17 compliance) | v1.0 (before telemetry endpoint ships) | Maintainer team |
| `BREAKING.md` (every breaking change requires an entry with migration steps; CI grep-enforced) | v0.5 | Kernel team |
| `STABILITY.md` (live (kernel, abi, manifest_schema) compatibility matrix; LTS branch policy) | v1.0 | Kernel team |
| `LOCALES.md` (translation contribution flow + glossary lock) | v0.5 | Doc team |

**`maosctl` accessibility (Mary's CLI-tool concern):**
- Respect `NO_COLOR` environment variable
- Respect `TERM=dumb` (no spinners, no Unicode box-drawing)
- `--plain` flag for screen-reader-friendly output
- Target: usable for blind operators in production-adjacent environments
- v0.5 ship gate

## LTS and Deprecation Policy (Mary's commitment numbers)

**LTS commitment (Mary's "pick a number"):**
- 2-year LTS on minor lines starting at v1.0
- Security-only patches after year 1 of LTS
- Two LTS lines maintained concurrently (current + previous)

**Deprecation timeline (Winston's commitment):**
- 2 minor releases of warning before removal
- 1 major release to actually remove
- Spirit-side `kernel.deprecation_warnings()` channel surfaces deprecations in `spirit-test`
- All deprecations entered in `BREAKING.md` (CI grep-enforced)

**Telemetry default and policy (Mary's three answers):**
- **Opt-in default.** Operator must explicitly enable.
- **Schema published** in `PRIVACY.md`: every field, every value type, redaction layer documented and source-published.
- **Storage:** v1.0 in maintainer-controlled aggregator with documented retention (90 days); GDPR Art. 17 deletion path via signed request.

## Implementation Considerations

**Documentation generation.** Rust API docs auto-generated via `cargo doc` for `maos-spirit-abi` and `maos-spirit-sdk` crates, published to versioned URL per Documentation Artifacts table. Manifest schema as versioned JSON Schema (machine-checked).

**Versioning and release.** Kernel SemVer with explicit ABI-stability promises per the triple (kernel_version, abi_version, manifest_schema_version). Spirit ABI machine-checked diff on every PR. Reference Spirits independent SemVer per Spirit. Skills SemVer per skill package. Release cadence: kernel monthly during v0.x; quarterly during v1.x; semi-annually during v2.x stable. LTS policy applies from v1.0.

**Telemetry and feedback loops.** v0.5 ships OpenTelemetry spans for every IAC frame, every capability invocation, every halt. v1.0 adds an opt-in anonymous-telemetry endpoint per `PRIVACY.md`. Feedback mechanism: GitHub issues + RFC process per `RFC_TEMPLATE.md` and `GOVERNANCE.md`.

**Skipped sections per CSV (visual_design / store_compliance).** MAOS has no first-party UI design system — operator surfaces are CLI + ACP-mediated editor banners + Transparency Log JSONL output. MAOS does not distribute through app stores — distribution is OSS package managers, GitHub Releases, and the Spirit registry.
