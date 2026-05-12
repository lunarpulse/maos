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
