# Deferred Work

## Deferred from: code review of 0-2-enforce-empty-kernel-invariants-via-structural-ci-lints (2026-05-11)

- **DF1 — DRY violation: walk_mod and walk_inline_mod_item.** The same 8 `match` arms for `pub fn`/`pub struct`/`pub enum`/`pub trait`/`pub type`/`pub const`/`pub static`/`pub use`/`pub mod` are copy-pasted between `walk_mod` and `walk_inline_mod_item` in `xtask/src/check_service_boundary.rs:200-260`. A bug fix in one won't automatically propagate to the other. Defer to cleanup story.
- **DF2 — Hardcoded baseline path in check-service-boundary CI job.** The CI gate in `.github/workflows/discipline.yml` always compares against the committed `kernel-surface-v0.1-alpha.json`, not a branch-specific baseline. Acceptable for v0.1-α stub; Story 2.2 may want a PR-target-branch diff mode.
- **DF3 — service_boundary_integration test builds baseline on-the-fly.** The violation test at `xtask/tests/service_boundary_integration.rs:22-40` snapshots the clean fixture to establish a baseline, then diffs against the violation fixture. Works correctly but is fragile — changes to the clean fixture silently change the test's expected behavior.

## Deferred from: code review of 0-3-content-addressed-corpora-infrastructure-coverage-matrix-ci-gate (2026-05-12)

- **W1 — `serde_yaml` 0.9.34 is explicitly deprecated.** Spec acknowledges this and defers migration. The crate works for the current contract; future concern.
- **W2 — `calibrate.rs` `successes = n` hardcoded placeholder.** At v0.1-alpha with no real corpora this is intentional scaffolding. Story 0.4 must replace with actual pass/fail from `expected_judgment` comparison.

## Closed deferred items

- **W2 — `calibrate.rs` `successes = n` hardcoded placeholder.** Closed by Story 0.4 — calibrate now scans `tests/corpora/<corpus_name>.jsonl` and computes pass_rate from `expected_judgment` equality via OfflineMode judge. See Story 0.4 AC8. The literal string `successes = n;` no longer appears anywhere in `xtask/src/calibrate.rs`.

## Deferred from: code review of 0-4-complianceclaim-schema-adversarial-review-calibration-seed-corpus (2026-05-12)

- **DF4 — `Cargo.lock` bloat from `tempfile` dev-dependency.** Adding `tempfile = "3"` transitively pulled in `getrandom 0.4.2` and ~25 WASI/WebAssembly ecosystem crates (`wasip2`, `wasip3`, `wit-bindgen`, `wasm-metadata`, `wasmparser`, etc.), increasing audit surface and build times. Pre-existing dependency resolution concern; not caused by this story's code logic. Address in a future dependency-audit story.

## Deferred from: 0-5-parameterized-corpus-generators-secret-redaction-red-team-frameworks (2026-05-12)

- **DF5 — `corpus-rebaseline.yml` workflow does NOT execute the 10⁵ quarterly secret-leakage generator run.** The quarterly run is generator-regenerable from seed + rule version, but no scheduled CI job currently invokes it. Wire in the NFR-Sec-4 redaction-filter story (v0.5) or a sibling story; until then, the 10⁵ corpus only exists when manually regenerated.
- **DF6 — Determinism integration tests are not yet wired into `discipline.yml` as a per-commit gate.** The tests pass locally and via `cargo test -p maos-corpus-gen` but are not a CI gate at v0.1-α. v0.5 should wire `cargo test -p maos-corpus-gen --test determinism_integration` into discipline.yml as a non-blocking step initially, promoting to blocking when NFR-Sec-4 ships.

## Deferred from: code review of 0-5 Chunk 1 (2026-05-12)

- **DF7 — "test" synthetic indicator check is over-broad in secret_redaction/validation.rs:52.** `item.raw.to_lowercase().contains("test")` matches "latest", "protest", "attest", etc. Acceptable heuristic at v0.1-α; real regex-based validation arrives with the NFR-Sec-4 redaction filter at v0.5.
- **DF8 — write_jsonl buffers entire corpus in memory (main.rs:115-121).** For quarterly mode (100k items) this consumes significant memory. Acceptable for a CLI dev-tool at v0.1-α; streaming write improvement deferred.
- **DF9 — Empty seeds array accepted without error (seeds.rs:29).** A valid TOML with `seeds = []` produces a degenerate generator. Defensive validation deferred; current seed files are non-empty.
- **DF10 — validate_all() hardcodes expansion size 10,000 (secret_redaction/mod.rs:83).** Coupled to corpus name "secret-redaction-1e4". Convenience wrapper; callers can use expand+validate directly for other sizes.

## Deferred from: code review of 0-5 Chunks 2-4 (2026-05-12)

- **DF11 — All 200 SR seeds identical within each class (only 11 unique patterns).** Seeds within each class share the same pattern_regex, false_positive_negative_anchors, and example_redacted_form. Only `id` differs. Team/expert review needed: is this acceptable (generator creates diversity via variant combos) or should each seed have a distinct pattern? Deferred to team decision.
- **DF12 — oauth_token regex cannot match real-format Slack tokens.** Pattern `(xoxb|xoxp|ghp_|ghs_|gho_|pat)-[A-Za-z0-9_]{20,}` uses char class without hyphen, but real Slack tokens have hyphens. False-negative gap. Fix when NFR-Sec-4 redaction filter ships at v0.5.
- **DF13 — api_key_openai regex collides with api_key_anthropic.** Pattern `sk-(test-|proj-)?[A-Za-z0-9_-]{20,}` matches Anthropic keys too. Ambiguous multi-class hits for downstream redaction. Fix at v0.5 when redaction filter ships.
- **DF14 — azure_credentials sig= pattern excessively broad.** Matches non-Azure URLs with `sig=` query params. Fix at v0.5 with tighter regex.
- **DF15 — Determinism test doesn't assert sort order explicitly.** AC4 requires items sorted by id but no test asserts `items[i].id < items[i+1].id`. SHA-pinning catches order changes indirectly. Deferred: add explicit sort-order assertion in a follow-up.

## Deferred from: Epic 0 retrospective (2026-05-13) — readiness-check discoveries

- **DF16 — `docs/invariants/journal.jsonl` never receives entries on merge (Story 0.1 AC5 structural gap). RESOLVED 2026-05-13 via Option (c).** Code work complete: xtask refactor (`--write-journal`, `--journal-output`, `--pr-body`, `detect_revert`), `.github/workflows/journal-append.yml` (merge-time artifact upload), `.github/workflows/journal-aggregate.yml` (operator-triggered aggregator), `discipline.yml` per-push job updated to validate-only. 16 invariant_lock unit tests pass. See `docs/dev-discipline/df16-resolution-option-c.md` for the full design. **Remaining operator action:** enable GitHub merge queue + add `journal-append` to required status-checks list; run synthetic-PR end-to-end verification. Once both complete, DF16 is fully closed.

- **DF17 — `invariant-lock` gate has only been exercised on 1-invariant diffs.** Story 0.2 dogfooded on a single I9 touch; `parse_cadence` bugs surfaced and were fixed. The gate has not been tested against a 14-invariant diff. Story 1a.1's plan touches all of I1–I14 simultaneously. **Required pre-flight:** author `xtask/tests/fixtures/clean-invariant-lock-14/` and `xtask/tests/fixtures/violation-invariant-lock-14-regression/` fixtures and verify the gate handles the multi-invariant case before opening 1a.1's PR. See `docs/dev-discipline/1a1-adr-landing.md` for the validation criteria.

## Deferred from: code review of 1a-1-initialize-17-crate-cargo-workspace-frozen-abi-types-starter-template (2026-05-13)

- **DW1 — `ComplianceClaimEnvelope` fields lack size validation.** No constructor validation for `signature: [u8; 64]`, `claim_bytes` emptiness, etc. At v0.1-α the type is structural only; serde/builder validation lands in Story 1b.4 when the freeze ships.
- **DW2 — `invariant-lock` gate not verified end-to-end.** Requires `gh` CLI which is not available in the dev environment. The 14-invariant clean fixture was verified as present per Epic 0 retro Step 6. The gate execution itself must be verified before the PR merges.
- **DW3 — `kloc.toml` references non-existent crates (`maos-cap-registry`, `maos-wire`, `maos-journal`).** These are pre-existing entries from the architecture that predate Story 1a.1. The `kloc-check` gate handles missing crates gracefully. Address in a future architecture-alignment story.

## Deferred from: code review of 1a-2-wire-the-five-service-kernel-skeleton-with-a-multi-threaded-tokio-composition-root (2026-05-13)

- **Surface walk `api::crate::*` path artifact** — the syn walker resolves `pub use crate::...` literally in `api.rs`, embedding `crate::` in the path string (e.g., `maos_kernel_core::api::crate::scheduler::SpiritSchedulerAdapter`). Classification table matches these paths. If the walker is fixed later, 7 TOML entries and the baseline JSON will need updating. Pre-existing walker behavior, not caused by this diff.
- **`LogBeforeDeliver::new()` is `pub` at v0.1-α** — the typestate guarantee on `IacBusPort` return types is advisory. I2's TODO notes `pub(crate)` restriction planned for Story 1b.2. Pre-existing design limitation.
- **`SandboxTier(pub u8)` has no value constraint** — raw u8 newtype accepts any value (0–255); T0-T2 enforcement with validation lands in Story 1b.3 per explicit scope deferral in story spec.

## Deferred from: code review of 1a-3-cryptoprovider-trait-xtask-service-boundary-stub-implementation (2026-05-13)

- **No `unseal_for_import` — seal-only half-API** — The `CryptoProvider` trait declares `seal_for_export` but no corresponding `unseal_for_import`. Intentional: unseal lands in Story 7.3 (ComplianceClaim envelope verify). Not a bug; the trait surface is complete for v0.1-α scope.
- **`sign_capability_token` `&[u8]` seed with no compile-time size hint** — The Ed25519 seed's 32-byte requirement is runtime-enforced via `from_seed_unchecked`. A `[u8; 32]` fixed-array parameter would give compile-time guidance but breaks the trait's `&[u8]` convention used by all other methods. Future newtype wrapper possible at Story 1b.2.
- **P1–P3 stub functions take unused parameters** — `p1_status_for`, `p2_status_for`, `p3_status_for` accept `_workspace_root` and `_service` params but return static strings. Parameters exist for the Story 2.2 enforcement upgrade; noise at v0.1-α.
- **`CryptoError::MalformedKey(&'static str)` can't carry dynamic diagnostics** — Error variant uses `&'static str`, preventing dynamic messages like "key was 31 bytes, expected 32". Coarse taxonomy per spec at v0.1-α; refinements land at Story 7.3.
- **No early guard on `signature_bytes` length in `verify_signature`** — ring handles wrong-length signatures internally via `Unspecified`. An early `signature_bytes.len() != 64` guard would improve error clarity but adds no correctness.
- **No AES-GCM plaintext size limit documentation** — AES-GCM has a practical limit of ~64 GB per (key, nonce) pair. No runtime guard or doc note. Caller responsibility at v0.1-α.
- **Empty plaintext → 16-byte ciphertext may surprise callers** — `seal_for_export` on empty input produces a 16-byte AES-GCM tag with no ciphertext. Standard AES-GCM behavior but could confuse downstream consumers.

## Deferred from: code review of 1a-4-ship-the-maosctl-cli-scaffold-with-security-md-and-accessibility-defaults (2026-05-13)

- **ColorChoice resolved but unused in stub dispatch (_color param)** — `accessibility::ColorChoice::resolve()` is called in `lib.rs` but the result is passed as `_color: ColorChoice` to `dispatch()` which discards it. By design for v0.1-α stubs; will be consumed when real output lands. `crates/maos-cli/src/subcommands.rs:10`
- **check_security_md swallows all I/O errors as "file missing"** — `std::fs::read_to_string` errors (permission denied, disk failure) are indistinguishable from file-not-found. In CI, failing the gate on any read error is reasonable behavior. `xtask/src/check_security_md.rs:33-40`
- **TERM="dumb " trailing whitespace falls through to Auto** — Exact `OsString` comparison fails on whitespace-padded values from shell profile typos. Spec worked example uses exact comparison; resilience to typos is a v0.5+ concern. `crates/maos-cli/src/accessibility.rs:58-61`
- **check_security_md follows symlinks without verifying regular file** — `std::fs::read_to_string` follows symlinks. An out-of-repo symlink could pass the gate. CI runs on fresh checkouts where this is not a concern. `xtask/src/check_security_md.rs:32`
- **e.exit() in lib.rs makes parse-error paths untestable** — `e.exit()` calls `std::process::exit()`. Spec worked example explicitly shows this pattern. Alternative would be testable but deviates from spec intent. `crates/maos-cli/src/lib.rs:28`
- **Unnecessary .collect() allocation in main.rs binary entry point** — `std::env::args_os().collect()` allocates a `Vec<OsString>`. Could change `run()` signature but changes public API for negligible v0.1-α benefit. `crates/maos-cli/src/main.rs:8`

## Deferred from: code review of 1a-5-migrate-abi-diff-to-cargo-public-api (2026-05-13)

- **Migration doc 175 lines vs AC4 "~200–400 lines"** — RESOLVED 2026-05-13. Expanded to 237 lines with nightly policy consensus, gate modes documentation, and fixture architecture section. Now within AC4 range.

## Deferred from: code review of 1b-2-capability-registry-decomposition-runtime-cap-tokens-cap-policy-cap-audit-cap-quota (2026-05-14)

- **DF18 — `Default for SandboxTier` returns T0 (most permissive).** Security-sensitive type should default to most restrictive tier. Pre-existing design decision predating this story. Revisit at 1b.3 when sandbox enforcement lands.
- **DF19 — `CapAuditWriter` is a unit struct serving as namespace.** No fields, exists only for `spawn()`. Cosmetic; a free function would be more idiomatic.
- **DF20 — `Intent` enum duplicates `Scope` enum shape.** Two parallel type hierarchies for the same conceptual space. Reconcile at architecture-doc reconciliation in 1b retro.
- **DF21 — `_payload` parameter discarded in `record_invocation`.** Reduces audit fidelity but not a correctness bug. Track for v0.3 when IAC Bus ships.
- **DF22 — `set_revoked` returns `Result<bool, ()>` where `Err(())` is never returned.** Cosmetic API cleanup.
- **DF23 — `capability_token_bytes` in Invocation is JSON-serialized instead of raw token bytes.** Audit-format decision; may need reconciliation for FR4 join query at 1b.5b.

## Closed deferred items (Story 1b.3 code review)

- **DF18 — `Default for SandboxTier` returns T0 (most permissive).** Closed by Story 1b.3 — `SandboxTier::default()` now returns `T2` (most restrictive enforceable tier). Resolves the fail-open default identified in Story 1a.2 code review.
- **1a.2 — `SandboxTier(pub u8)` has no value constraint.** Closed by Story 1b.3 — added `try_from_u8`, `try_from_manifest_str`, associated constants T0–T4, `DEFAULT_FLOOR`, and `SandboxTierError` validation. The three `unwrap_or(SandboxTier(0))` fail-open fallbacks in `cap_policy/mod.rs` are fixed to `DEFAULT_FLOOR`.

## Deferred from: code review of 1b-3-sandbox-tier-t0-t1-t2-enforcement-per-spirit-resource-caps (2026-05-14)

- **`SandboxTierError(pub u8)` misrepresents non-numeric string errors.** When `try_from_manifest_str("foo")` fails, the error carries `u8::MAX` (255) instead of the original string. Developer UX issue only; not a correctness bug. Deferred to a future cleanup story.

## Deferred from: code review of 1b-5a-ship-hello-spirit-reference-binary-and-hit-nfr-onb-2-5-minute-evaluator-path (2026-05-15)

- **Exit code truncation via `as u8` cast in subcommands.rs.** `ExitCode::from(s.code().unwrap_or(2) as u8)` — Unix exit codes are 0-255 so safe on POSIX, but `as u8` truncates any value >255 to 0 on Windows. Pre-existing pattern not introduced by this story.
- **Timing script uses 1-second granularity for 300s NFR gate.** `date +%s` gives integer seconds; sub-second regression detection impossible. Acceptable for a 300s budget but provides coarse signal. Pre-existing design choice in `tests/integration/onb_nfr2_timing.sh`.

## Deferred from: code review of 1b-5b-maosctl-audit-query-fr4-100-mediation-mechanical-verification (2026-05-15)

- **Non-one-shot server exit path does not drain `audit_writer` — rows silently lost.** In server (non-one-shot) mode, SIGINT/SIGTERM triggers immediate return without awaiting the audit writer. Last N audit entries are permanently lost. Pre-existing gap in the server-mode exit path, not introduced by this story's one-shot drain fix.
- **No test for bare `maosctl audit query --format plain` (without `--spirit`).** The bare-plain path has zero coverage in integration tests. Works correctly but untested.
- **`to_plain` silent integer truncation: negative SQLite values cast to unsigned via `as`.** `row.get::<_, i64>(1)? as u64` silently wraps negative SQLite INTEGER values to two's-complement garbage. No CHECK constraints on the SQLite schema prevent negative values. Pre-existing pattern from Story 1b.1's original `query` function.

## Deferred from: code review of 1b-5c-maosctl-v0-1-lifecycle-subcommands-accessibility-flags (2026-05-15)

- **`resolve_spirit_pid` PID unused in lifecycle verbs** — by design at v0.1-β; journal keys by `spirit_id: String`, not `spirit_pid: u32`. Pre-existing pattern from 1b.5b. No action needed until Epic 5 introduces real process control.
- **Orphan-fixture detection misses `.toml` files in non-standard category subdirectories** — `find_orphan_fixtures` only walks `well-formed/`, `malformed-rejected/`, `edge-case/`. Files in unrecognized subdirs pass silently. Pre-existing design limitation of the NFR-Test-13 walker.
- **Corrupted last journal line makes journal permanently unopenable** — `JournalAdapter::open` fails on first unparseable NDJSON line (parse-and-reject, not skip-and-continue). Pre-existing in Story 1b.1's journal design (`journal/mod.rs:110-121`). A truncated tail from SIGKILL mid-write blocks all future opens.

## Closed deferred items (Epic 1b retro readiness-check + bridge commits 2026-05-16)

- **6 missing I9 exemption registrations** — `InferencePortAdapter`, `SecurityManagerAdapter`, `SandboxSpec`, `HistogramSeries`, `CounterSeries`, `IacRtMetrics` had non-empty fields outside the three-path I9 whitelist but no `#[maos_attrs::i9_exempt]` annotation + register entry. Closed by bridge commit `b610ac2` — added annotations + entries in `docs/invariants/i9-exemptions.md`.
- **NFR-Test-2 kernel surface stale** — Story 1b.5a's `capability_registry()` → `TokenIssuer` narrowing changed the `InferencePortAdapter` signature hash AND introduced a new `TokenIssuer` re-export; neither was reflected in `docs/ci-baselines/kernel-surface-v0.1-beta.json` or `xtask/kernel-api-classes.toml`. Closed by bridge commit `b610ac2` — refreshed 4 hashes, added `TokenIssuer` entry + `data-movement` classification.
- **`cap_registry_smoke.sh` `timeout 5s` wraps compile** — `timeout 5s bash -c 'cargo run -p maos-bin --quiet &amp; …'`. On cold CI cache, the compile alone exceeds the window and the script exits 124 before the binary launches. Closed by bridge commit `95faf94` — `cargo build` moved outside the timeout; window bumped to 8s for runner slack.
- **`onb_nfr2_timing.sh` `--bin maos-bin` + `default-members = []` = panic 101** — bare `--bin` resolution from workspace root under `default-members = []` panics immediately with "manifest is virtual, workspace has no members." Closed by bridge commit `95faf94` — swapped to `cargo build -p maos-bin --release --locked`.

## Deferred from: Epic 1b retrospective (2026-05-16) — architectural divergences flagged

- **ADR-004 gate-line inconsistency with NFR-Sec-1.** ADR-004 says sandbox tier T2 at v0.3; NFR-Sec-1 binds T0/T1/T2 enforcement floor at v0.1-β. Story 1b.3 shipped per NFR-Sec-1 without amending ADR-004. Doc/code drift; cosmetic. Tracked as Epic 1b retro **Doc4** — opportunistic; not blocking Epic 2.

## Closed deferred items (Story 1b.6 — Epic 2 prep bundle, 2026-05-16)

- **D9 — Dual `SandboxTier` type hierarchy.** Closed by Story 1b.6 — reconciled via explicit conversion: `From<maos_spirit_abi::compliance::SandboxTier> for maos_domain::invariants::i9::SandboxTier` (ABI→operational) + `SandboxTier::to_abi() -> Option<…>` (operational→ABI). Module-level docs cross-reference each other and explain the wire-vs-operational design choice. The retro's original "one canonical, other deprecated" recommendation was incompatible with `maos-spirit-abi`'s `#![no_std]` boundary + frozen `ABI_VERSION = 1` wire format; pragmatic resolution preserves both invariants. See `1b-6-epic-2-prep-d9-d10-doc3.md` for the design rationale.
- **D10 — Architecture doc 17 → 19-crate workspace catch-up.** Closed by Story 1b.6 — `architecture-maos-minimal-opus/4-kernel-design.md` §4.0.2 Layout updated with `maos-audit` (1b.1), `maos-attrs` (1b.3), `maos-corpus-gen` (E0), and `xtask` (workspace member). Total declared: 18 lib/bin crates + xtask = 19 workspace members, matching `Cargo.toml`. "Dependencies point inward" prose updated to cite the two explicit exceptions (Spirit ABI traits inversion + 1b.6 SandboxTier conversion direction) and the `default-members = []` invariant (per retro action A7).
- **Doc3 — ADR for `#![forbid(unsafe_code)]` per-module relaxation.** Closed by Story 1b.6 — ADR-039 added at `docs/adr/ADR-039-per-module-unsafe-code-policy.md` with `binding-v0.1` status. Formalizes: enforcement via `xtask check-unsafe` + `xtask/unsafe-allowlist.toml`; allowlist seeded with `crates/maos-kernel-core/src/security/sandbox/` per Story 1b.3; amendment process via ADR-037. Cross-references ADR-004, ADR-010, ADR-030, ADR-037, and the Story 1b.3 dev record. `docs/adr/index.md` updated with the new ADR row (14 → 15 binding-v0.1 ADRs).
