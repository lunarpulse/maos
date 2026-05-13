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
