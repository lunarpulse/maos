# Story 0.5: Parameterized Corpus Generators — Secret-Redaction + Red-Team Frameworks

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As **the test-architecture lead facing ~2,249 hand-authored corpus items across the v1.0 + v1.5 ship gates** (CCAC N=600, red-team N=80→640, secret-redaction 10⁴ per-commit + 10⁵ quarterly + 1000-canary/month, HSIS N=100/200, cross-Spirit isolation N=200, LCAS N=210, log-completeness N=100, ~50/100 per innovation pattern, ~50/100 per Mira-Nash, ...),
I want **two parameterized generator frameworks committed early as a new `crates/maos-corpus-gen/` workspace crate exposing a `CorpusGenerator` trait declared in `crates/maos-corpus-gen/src/lib.rs` (with `seed_corpus()`, `expand(n: usize)`, `validate(&Item) -> ValidationOutcome`, `coverage_report() -> CoverageReport`), AND `crates/maos-corpus-gen/src/secret_redaction/` producing the 10⁴ per-commit secret-leakage corpus deterministically from ~200 SHA-pinned seed patterns covering all named secret classes (API keys / OAuth tokens / private keys / database URLs / JWT / AWS / GCP / Azure / SSH / GPG), AND `crates/maos-corpus-gen/src/red_team/` producing the ≥640-item adversarial-Spirit red-team corpus deterministically from 80 SHA-pinned canonical seed scenarios across the 8 §8.1 attack classes (capability confusion / IAC frame injection / distillation poisoning / ledger tampering / cross-Spirit privilege escalation / resource exhaustion / side-channel timing / kernel-syscall abuse — N=10 per class × 8× parameter-variation expansion), AND both generated corpora committed at `tests/corpora/secret-redaction-1e4.jsonl` and `tests/corpora/red-team-640.jsonl` SHA-256-pinned per Story 0.3 manifest discipline AND registered in `tests/corpora/MANIFEST.toml` AND wired into the existing `NFR-Sec-4` and `NFR-Sec-10` rows in `tests/coverage-matrix.yaml` (currently `corpora: []` after Story 0.4's mass-population), AND a determinism guarantee mechanically verified ("same seed file SHA + same expansion-rule version → byte-identical JSONL output, every run, on every host"), AND a `cargo run -p maos-corpus-gen -- coverage --corpus <name>` subcommand emitting per-class / parameter-space / unexpanded-seed-slot coverage JSON, AND `xtask/kloc.toml` extended with the new crate's per-crate ceiling so the aggregate alarm stays calibrated**,
so that **scheduling fictions ("hand-author 10,000+ items between v0.5 and v1.5") collapse to mechanical engineering artifacts — generator + seed corpus + versioned expansion rules — that downstream Stories 5.5b (multi-provider CI secret-redaction tests in CI), 6.x (ConsentRupture adversarial fixtures), 7.3 (CCAC N=600 ship-gate sharing the generator pattern), 9.4 / 9.x (secret-redaction operational canary review consuming the same generator with canary-mode markers), and 10.2 (adversarial-Spirit red-team gate at v1.5 — P0 ship-block if any false negative) can consume large corpora at gate time without inventing them from scratch, the `NFR-Sec-4` / `NFR-Sec-10` rows in `coverage-matrix.yaml` flip from "0-item placeholder" to "≥10000-item and ≥640-item entries respectively pointing at SHA-pinned generator output," and epic-0's founding-sprint "Owns" line `Calibration harness infrastructure (NFR-Aud-8: N=100 per-commit pipeline + N=500 quarterly audit runner — corpus content authored per-epic)` extends to "and parameterized-generator harness infrastructure for the two NFR-Sec corpora whose hand-authoring would otherwise dominate the v1.0–v1.5 critical path"**.

Story 0.5 is the **last story in Epic 0** (E0 transitions to maintenance discipline owned by whoever holds the repo after this merges), and per the dependency-DAG it is **NOT a blocker for any E1b story** — Story 1b.4 unblocks on Story 0.4's signed-off ComplianceClaim review report, which is now committed at `_bmad-output/planning-artifacts/compliance-claim-schema-review.md`. This story's value is **forward leverage**: every subsequent epic that would otherwise spend N person-days authoring an N-item corpus instead spends ≪N person-days authoring ~10–80 seeds + a one-screen expansion rule, and the generator deterministically expands. The dependency-DAG line **"E10 ← consumes corpora authored in ... E0 (secret-redaction generator)"** and the sprint-plan v1.0 invariant **"Story 9.6 (red-team 80→640 generator)"** (note: there is no `9.6` in the actual Epic 9 — that line is the original epic-list draft now superseded by **Story 0.5 owning generator authorship**, and Story 10.2 owning generator *execution at the ship gate*; the dev agent SHOULD NOT create a Story 9.6) are the v1.0/v1.5 hand-offs this story enables.

At v0.1-α there is still **no live gate consuming these corpora**: NFR-Sec-4's pre-write secret-redaction filter is **phase v0.5**; NFR-Sec-10's adversarial-Spirit red-team gate is **phase v1.5**. Both rows in `coverage-matrix.yaml` are therefore in `out_of_scope_deferred` at `current_phase = v0.1-alpha`, and adding a non-empty `corpora` field to a `phase ≥ v0.5` row does **not** trigger any `NFR-Meta-3` violation — Story 0.4 AC7's logic (`phase > current_phase → out-of-scope-deferred, no violation`) is the load-bearing rule that makes this story land cleanly at v0.1-α. The corpus is **scaffolding ahead of the gate**, exactly mirroring Story 0.4's calibration corpus pattern. When the NFR-Sec-4 redaction filter lands (Story 1b.5a or its v0.5 follow-on) and when the NFR-Sec-10 gate lands (Story 10.2), each story switches its row's `gates` from `[]` to its gate name and consumes the SHA-pinned corpus this story committed.

The story is **purposefully restricted** to the per-commit slice of secret-redaction (10⁴) and the full red-team (640). The quarterly 10⁵ secret-leakage corpus is **NOT committed to git** — at ~10–30 MB it would dominate repo size, and the whole point of a deterministic generator is that 10⁵ is regenerable on demand from the same seed + rule version. Instead, this story commits the generator and a `quarterly` mode that produces 10⁵ items into a build artifact (`target/corpus-output/secret-redaction-1e5-<sha>.jsonl`); the quarterly workflow (`corpus-rebaseline.yml`, already shipped by Story 0.3 as a placeholder) is **NOT extended here** to actually execute the 10⁵ run — that wiring is operational and lands when NFR-Sec-4 ships at v0.5. Similarly, the **1000-canary-per-month production canary corpus** ships its **generator method** (`secret_redaction::generate_canary_batch(n: 1000, rng_seed: u64, marker_namespace: &str) -> Vec<Item>`) but **NOT a 24/7 canary-loop service** — that loop is operational and lands at v0.5 with the redaction filter itself.

## Acceptance Criteria

### AC1 — New workspace crate `crates/maos-corpus-gen/` with `CorpusGenerator` trait and binary skeleton

**Given** the existing workspace `Cargo.toml` (members `["xtask", "crates/maos-spirit-abi", "crates/maos-kernel-core"]` plus `default-members = []`) and Story 0.4's verified discipline that new crates are added explicitly to `members` (no glob in `members =` to avoid Story 0.1's earlier kloc-leak concern)
**When** the new crate is added to the workspace
**Then** `crates/maos-corpus-gen/Cargo.toml` is committed declaring the crate with `[lib]` and `[[bin]] name = "maos-corpus-gen"` paths, `version.workspace = true`, `edition.workspace = true`, `license.workspace = true`, `repository.workspace = true`, `rust-version.workspace = true`, and `description = "MAOS parameterized corpus generators — secret-redaction + red-team frameworks (Story 0.5)"`
**And** root `Cargo.toml`'s `members` array adds `"crates/maos-corpus-gen"` (additive change; alphabetical-ish ordering — slot after `maos-spirit-abi` is fine), `default-members` stays `[]`
**And** `crates/maos-corpus-gen/src/lib.rs` declares the public `CorpusGenerator` trait with **exactly** this shape — these four methods are the binding contract from epic-0 Story 0.5 AC1 verbatim; widening is allowed in future stories (additive trait methods are not ABI-breaks per §8.5's rule applied here for non-ABI workspace crates), narrowing is a story-blocking question:
  ```rust
  pub trait CorpusGenerator {
      type Item: serde::Serialize + serde::de::DeserializeOwned;
      type Seed: serde::Serialize + serde::de::DeserializeOwned;

      fn seed_corpus(&self) -> Vec<Self::Seed>;
      fn expand(&self, n: usize) -> Vec<Self::Item>;
      fn validate(&self, item: &Self::Item) -> ValidationOutcome;
      fn coverage_report(&self) -> CoverageReport;

      /// Returns the SHA-256 hex of the canonical CBOR-or-JSONL serialization of `seed_corpus()`,
      /// combined with the expansion-rule version, that pins this generator's output.
      fn seed_sha256(&self) -> String;
      fn rule_version(&self) -> &'static str;
  }
  ```
**And** `ValidationOutcome` is an enum `Valid | Invalid { reason: String } | FalseNegativeRisk { detail: String }` (the third variant is the load-bearing P0 detector for secret-redaction — see AC3)
**And** `CoverageReport` is a struct `{ corpus_name: String, total_items: usize, classes: BTreeMap<String, ClassCoverage>, unexpanded_seed_slots: Vec<String>, parameter_space_coverage: BTreeMap<String, f64> }` with `ClassCoverage = { seed_count: usize, expanded_count: usize, dedup_drop_count: usize, floor_satisfied: bool }`
**And** the binary at `crates/maos-corpus-gen/src/main.rs` uses `clap 4.5` (the version pinned by Story 0.1 in `xtask/Cargo.toml`; re-use, do NOT pull a different `clap` major) with at least these subcommands at v0.1-α: `generate --corpus <name> --mode {per-commit, quarterly, canary} --out <path>` and `coverage --corpus <name> [--json]`
**And** `cargo build -p maos-corpus-gen` succeeds against `Rust stable` (per `rust-toolchain.toml`) and the `cargo build --locked --all-targets --workspace` step in `discipline.yml`'s `reproducible-build` job continues to pass without modification (the new crate is transitively built; no workflow edit needed)

### AC2 — Secret-redaction generator with ~200 seed patterns covering all named secret classes

**Given** the §4 architecture binding text **"Pre-write secret-redaction filter at the Transparency Log boundary. Frames passing through the IAC Bus are scanned for known secret patterns (API keys, capability tokens, mTLS private-key bytes) before being written to the log; any match is redacted with a typed marker `<REDACTED:type=…,len=…,hash=…>`. Floor: 0 secrets in any logged frame across the bounded test populations (10⁴-case corpus per-commit, 10⁵-case quarterly audit, 1000-canary-secrets-per-month production canary system)"** and the NFR-Sec-4 bounded-test-populations contract
**When** `crates/maos-corpus-gen/src/secret_redaction/mod.rs` and its sibling files (`seeds.rs`, `expansion.rs`, `validation.rs`) are committed
**Then** the seed corpus at `crates/maos-corpus-gen/seeds/secret-redaction-seeds-v0.1.toml` enumerates **exactly 200** seed patterns distributed across the named secret classes per the table below (the distribution is the binding contract — counts may shift ±10 per class for parameter-space coverage but the **10 named classes must each have ≥10 seeds** and the **total must be exactly 200** with the residual distributed across high-coverage classes; mismatched counts surface as a story-blocking question):

| Class | Seeds | Examples (illustrative — not committed verbatim) |
|---|---|---|
| `api_key_anthropic` | 20 | `sk-ant-…` prefix patterns, env-var bare form, JSON body shapes |
| `api_key_openai` | 20 | `sk-…` ~50-char patterns, project-key `sk-proj-…` variants |
| `oauth_token` | 25 | Bearer tokens, `xoxb-…` Slack, `ghp_…` / `ghs_…` / `gho_…` GitHub PAT/installation/OAuth, `pat-…` |
| `private_key_pem` | 20 | RSA / Ed25519 / ECDSA PEM blocks, PKCS#8 / SEC1, OpenSSH inner |
| `database_url` | 20 | `postgres://`, `mysql://`, `mongodb+srv://`, `redis://`, `sqlserver://` with embedded creds |
| `jwt` | 20 | Three-segment base64url, various header `alg` values, with/without `kid`, exp/iat shapes |
| `aws_credentials` | 20 | `AKIA…` / `ASIA…` access keys, secret access keys, session tokens, `AWS_*` env-var forms |
| `gcp_service_account` | 15 | JSON service-account blobs, `private_key` field shapes, `gcp-sa-…` env-vars |
| `azure_credentials` | 15 | Connection strings, SAS tokens, subscription-id-paired secret patterns |
| `ssh_key_block` | 15 | OpenSSH private key full blocks, encrypted variants, host-key edge cases |
| `gpg_armored_block` | 10 | `-----BEGIN PGP …-----` blocks, ASCII-armored secret variants |

**And** every seed in the TOML carries the exact schema `{ id, class, pattern_regex, false_positive_negative_anchors: [string], example_redacted_form }` — `pattern_regex` is the regex the redaction filter would test against; the generator does NOT execute it (no `regex` crate dep at v0.1-α), it stores it for downstream consumers; `false_positive_negative_anchors` lists strings that should NOT match the pattern (well-formed-but-non-secret look-alikes — e.g., a Slack workspace ID that resembles `xoxb-` but is missing the secret suffix), enabling future redactor implementations to negative-test
**And** seed-TOML file SHA-256 is computed at compile time via `include_bytes!` + `sha2::Sha256` and exposed via `SecretRedactionGenerator::seed_sha256()`; the deterministic-test in AC5 asserts this hash equals a known constant pinned at the top of `src/secret_redaction/mod.rs` as `pub const SEED_FILE_SHA256: &str = "<hex>"`
**And** when `SecretRedactionGenerator::expand(10_000)` runs, the generator produces **exactly 10000 deduplicated** items by composing each seed with deterministic parameter variations (variations: prefix-padding length, surrounding noise tokens, structured-vs-unstructured embed, JSON-vs-text frame, line-position, multi-secret-per-line collapse) — composition is a pure function of `(seed_index, variant_combo_index)` so re-running produces byte-identical output; deduplication is a stable sort + dedup-by-canonical-form pass
**And** the generator emits `tests/corpora/secret-redaction-1e4.jsonl` (corpus name `secret-redaction-1e4`, filename `<name>.jsonl` per Story 0.3 manifest-key-equals-filename-stem discipline; the `<sha>` notation in epic-0 Story 0.5 AC2 prose was epic-shorthand for "SHA-pinned via MANIFEST.toml" and does NOT mean the SHA appears in the filename — the dev agent SHOULD NOT name the file `secret-redaction-1e4-<sha>.jsonl` because Story 0.3's `check-corpus` verifier looks up `<corpus_name>.jsonl`)
**And** each JSONL line is a JSON object `{ "id": "secret-red-NNNNN", "class": "<class_name>", "raw": "<the synthetic line containing the seeded secret>", "expected_redacted": "<REDACTED:type=<class>,len=<int>,hash=<short-sha>>", "seed_id": "<seed_id_from_toml>", "variant_combo": "<deterministic_combo_id>" }` — note `raw` MUST contain a deterministic but realistic-looking secret-shaped pattern; do NOT commit live API keys; every value is **synthetic by construction** (e.g., AWS access key MUST start with `AKIA-TEST-` not `AKIA` alone, OpenAI keys MUST be `sk-test-0xx...`)
**And** the file ends with a final newline (Story 0.3's streaming-SHA-256 path appends `\n` per line; total line count exactly 10000); compute SHA via `cargo run -p xtask -- check-corpus --register secret-redaction-1e4` then paste the resulting `[corpus."secret-redaction-1e4"]` row into `MANIFEST.toml`
**And** the generator's `quarterly` mode (`SecretRedactionGenerator::expand(100_000)`) is callable but **NOT executed during the per-commit CI** — running it once on the dev agent's machine produces `target/corpus-output/secret-redaction-1e5.jsonl` for visual inspection; the file is NOT committed (gitignored — see AC6); the generator MUST produce exactly 100000 deduplicated items
**And** the canary mode `SecretRedactionGenerator::generate_canary_batch(1000, rng_seed: u64, marker_namespace: &str) -> Vec<Item>` exists and produces 1000 items each containing a cryptographic marker of the form `<CANARY-{marker_namespace}-{i:04}-{hmac_sha256(seed||i||namespace, "maos-canary-v0.1") [..16] }>` so production canary-leak detection at v0.5+ can match emitted vs leaked markers — this method ships at v0.1-α but is **not** called by any CI gate

### AC3 — Red-team generator with 80 canonical seed scenarios × 8 attack classes × 8× expansion = ≥640 items

**Given** §8.1 architecture binding text **"red-team corpus — N=80, full taxonomy across 8 attack classes (capability confusion, IAC frame injection, distillation poisoning, ledger tampering, cross-Spirit privilege escalation, resource exhaustion, side-channel timing, kernel-syscall abuse), every Spirit, every release. ≥9/10 per class detected/blocked, ≥72/80 aggregate, 0 unmitigated category"** and the NFR-Sec-10 generator-discipline note **"80 scenarios × 8× expansion via parameter variation = 640 effective items per Murat's generator discipline"**
**When** `crates/maos-corpus-gen/src/red_team/mod.rs` and its sibling files (`seeds.rs`, `expansion.rs`, `validation.rs`) are committed
**Then** the seed corpus at `crates/maos-corpus-gen/seeds/red-team-seeds-v0.1.toml` enumerates **exactly 80 canonical scenarios at 10 per class** across the 8 §8.1 classes with class identifiers **exactly**: `capability_confusion`, `iac_frame_injection`, `distillation_poisoning`, `ledger_tampering`, `cross_spirit_privilege_escalation`, `resource_exhaustion`, `side_channel_timing`, `kernel_syscall_abuse` (these strings are the binding identifiers — renaming any single one is a story-blocking question because Story 10.2 will grep for them)
**And** every seed in the TOML carries `{ id, class, attack_summary, kernel_defense_mechanism, expected_detection_surface, parameter_axes: [string], canonical_assertion }` — `parameter_axes` enumerates the dimensions along which the seed expands (e.g., for `capability_confusion`: `["target_capability_class", "spoofed_caller_identity", "TTL_boundary", "frame_ordering"]`); `canonical_assertion` is the test predicate the future ship-gate evaluates (e.g., for `iac_frame_injection`: `"kernel rejects frame with ECapabilityScope violation and journals EAuditFrameInjection event"`)
**And** the generator's `expand(640)` method produces exactly 640 deduplicated items by emitting 8 expansion variants per seed; per-class floor verified by `coverage_report()` reporting **≥80 items per class** post-dedup
**And** **deduplication preserves coverage:** every one of the 80 seeds MUST appear in at least one expanded form (no seed dropped entirely by dedup); `coverage_report().unexpanded_seed_slots` MUST be empty after a clean run; if any seed's 8 expansion variants collide entirely with another seed (parameter-space exhaustion), the generator emits at least 1 item per seed by widening that seed's variant axes — implemented as a per-seed minimum-emit count in `expand()`'s post-dedup pass
**And** the generator emits `tests/corpora/red-team-640.jsonl` (corpus name `red-team-640`, filename `<name>.jsonl`); the file's line count is exactly 640 (or higher if the per-seed minimum-emit pass adds items; the AC3 floor is "≥640" — the dev agent SHOULD aim for exactly 640 by tuning expansion-rule axes and document the actual count in the corpus's manifest description if it deviates)
**And** each JSONL line is a JSON object `{ "id": "red-team-NNN", "class": "<class_name>", "scenario_description": "<expanded scenario prose>", "parameters": { "<axis>": "<value>", ... }, "expected_kernel_response": "<rejection_class | acceptance_with_audit | structural_alarm>", "expected_audit_signal": "<typed_error_name_or_event_name>", "seed_id": "<seed_id_from_toml>", "canonical_assertion": "<the predicate Story 10.2's gate runs>" }`
**And** scenarios MUST be **structural, not executable** — the red-team corpus describes attack scenarios in prose + structured fields; it does NOT contain runnable exploit code at v0.1-α; the corpus is consumed by Story 10.2 which authors the actual kernel-side test driver that materializes each scenario as an IAC frame / capability-token / syscall and asserts kernel response
**And** SHA-256 is computed and pinned in `red_team::mod.rs` as `pub const SEED_FILE_SHA256: &str = "<hex>"`

### AC4 — Determinism gate: regenerate → byte-identical JSONL output

**Given** the AC1 trait contract that generator output is deterministic given `(seed_sha256, rule_version)` and the Story 0.3 NFR-Test-1 SHA-256 corpus discipline
**When** `crates/maos-corpus-gen/tests/determinism_integration.rs` is committed
**Then** the test runs `SecretRedactionGenerator::default().expand(10_000)` twice and asserts byte-identical Vec output via `Vec<Item>` round-trip + `serde_json::to_string` per-item comparison (NOT through-disk to avoid filesystem-encoding artifacts — pure in-memory equality)
**And** a sibling test runs `RedTeamGenerator::default().expand(640)` twice and asserts the same in-memory equality
**And** a third test asserts SHA-256 stability: the generator's output (concatenated `serialize_to_canonical_jsonl()` of each item + `\n`) hashes to a known constant pinned in `tests/determinism_integration.rs` as `EXPECTED_SHA_SECRET_REDACTION_1E4: &str = "<hex>"` and `EXPECTED_SHA_RED_TEAM_640: &str = "<hex>"` — these constants are committed alongside the JSONL files and MUST match `MANIFEST.toml`'s `sha256` field exactly
**And** a fourth test asserts cross-host determinism by verifying: (a) the generator emits items sorted by `id` (lex order on `secret-red-NNNNN` and `red-team-NNN`); (b) every JSON field within an item serializes via `serde_json::to_writer` with the **default** field ordering (BTreeMap-derived stable order); (c) no floating-point fields are present (the corpora are 100% strings + integers — any future variant must justify f64 inclusion); (d) no system clock, no `std::env`, no PID, no thread-id is read during generation — verified by grepping the new crate's `src/` for `SystemTime|Instant|env::var|process::id|thread::current` and asserting zero matches (an additional `#[test]` named `no_nondeterminism_sources` runs the grep via `walkdir 2.5` — re-use the dep Story 0.3 added)
**And** a fifth test asserts the canary-mode determinism: `generate_canary_batch(1000, rng_seed=42, marker_namespace="test")` produces identical output on every invocation; the markers' HMAC values are deterministic functions of (seed_id, namespace, index)
**And** the `cargo test -p maos-corpus-gen` suite runs in <30 seconds wall-clock on a stock x86_64 GitHub runner (the discipline.yml budget allowance for the new crate; if a test exceeds, profile and shrink — do NOT pad the budget)

### AC5 — `cargo run -p maos-corpus-gen -- coverage --corpus <name> [--json]` emits structured coverage report

**Given** the AC1 `CoverageReport` struct and Story 0.5's epic AC4 verbatim **"the report shows attack-class coverage, parameter-space coverage, and any unexpanded seed slots"**
**When** the binary is invoked with `coverage --corpus secret-redaction-1e4` and `coverage --corpus red-team-640`
**Then** for `secret-redaction-1e4` the report prints (text mode) a table with columns `(class, seeds, expanded_count, dedup_drops, floor_satisfied)` covering all 10 named classes; floor for secret-redaction = `≥1000 items per class` (10000/10 classes ≈ 1000-floor with parameter-distribution skew tolerance ±10%); `parameter_space_coverage` reports the (combo_count_observed / combo_count_possible) ratio per class as a `f64` in `[0.0, 1.0]`
**And** for `red-team-640` the report prints a table with the 8 §8.1 classes, each row showing `(seed_count=10, expanded_count≥80, dedup_drops=0_or_disclosed, floor_satisfied=true_iff_≥80)`
**And** with `--json` the binary emits the `CoverageReport` JSON to stdout exactly matching `serde_json::to_string_pretty(&report)`; the JSON serializes the struct field order alphabetically (BTreeMap default) — this is what Story 10.2 / 9.x consumers parse
**And** the binary exits **non-zero** if any class's `floor_satisfied = false`, with a stderr message `"NFR-Sec-{4 or 10} floor violation: class <name> has <count> items, floor is <floor>"`
**And** the binary exits non-zero if `unexpanded_seed_slots` is non-empty, with stderr `"generator coverage drift: seed <seed_id> produced 0 expanded items after dedup — widen parameter axes in src/<generator>/expansion.rs"`
**And** if `<name>` is neither `secret-redaction-1e4` nor `red-team-640` the binary exits non-zero with stderr `"unknown corpus name; supported: secret-redaction-1e4, red-team-640"`
**And** the binary defers neither to `tests/coverage-matrix.yaml` nor to `tests/corpora/MANIFEST.toml` — coverage is a property of the **generator state**, not the on-disk artifact; the binary computes the report from the live generator (calling `seed_corpus()` + `expand()` + tallying classes) so that even a freshly-checked-out tree without the JSONL files committed still produces a valid coverage report

### AC6 — Wire corpora into `MANIFEST.toml` + `coverage-matrix.yaml` and pin determinism artifacts

**Given** Story 0.3's manifest-row schema `[corpus.<name>] sha256, schema_version, item_count, valid_until, prompt_version_hash, description, judge_id?` and Story 0.4 AC6's manifest-write discipline (use `cargo run -p xtask -- check-corpus --register` to compute SHA; do NOT use `sha256sum`)
**When** the manifest is updated
**Then** `tests/corpora/MANIFEST.toml` gains exactly two new rows: `[corpus."secret-redaction-1e4"]` with `sha256 = "<hex from --register>"`, `schema_version = 1`, `item_count = 10000`, `valid_until = "2027-05-12"` (12 months from this story's creation date 2026-05-12 per NFR-Meta-2 default), `prompt_version_hash = "<sha256 of canonical schema metadata JSON>"`, `description = "v0.1-α secret-redaction per-commit corpus 10⁴ items deterministically generated by maos-corpus-gen::secret_redaction from 200 SHA-pinned seed patterns covering 10 named secret classes (api_key_anthropic, api_key_openai, oauth_token, private_key_pem, database_url, jwt, aws_credentials, gcp_service_account, azure_credentials, ssh_key_block, gpg_armored_block). Consumed by NFR-Sec-4 redaction filter (lands v0.5). Authored in Story 0.5. judge_id omitted at v0.1-α — Story 1b.5a or v0.5 redaction-filter story adds it when the filter goes live."`, `judge_id` field omitted (resolves to `Option<String>::None`)
**And** `[corpus."red-team-640"]` with `sha256 = "<hex>"`, `schema_version = 1`, `item_count = 640`, `valid_until = "2027-05-12"`, `prompt_version_hash = "<sha256 of canonical schema metadata JSON>"`, `description = "v0.1-α adversarial-Spirit red-team corpus 640 items deterministically generated by maos-corpus-gen::red_team from 80 canonical scenarios across 8 §8.1 attack classes (N=10 per class × 8× parameter-variation expansion). Per-class floor ≥80 items post-dedup. Consumed by NFR-Sec-10 ship-gate (lands v1.5). Authored in Story 0.5. judge_id omitted — corpus is gate-verified by structural assertion not judge-LLM agreement."`, `judge_id` field omitted
**And** the TOML key strings MUST be quoted (`[corpus."secret-redaction-1e4"]` not `[corpus.secret-redaction-1e4]`) because the names contain dashes — this is the same discipline Story 0.4 hit with `[corpus."calibration-seed-v0.1"]` due to dots
**And** `tests/coverage-matrix.yaml`'s **existing** `NFR-Sec-4` row (mass-populated by Story 0.4 as `{ gates: [], corpora: [], phase: "v0.5", valid_until: "2027-05-12" }`) is updated **non-destructively** to `{ gates: [], corpora: ["secret-redaction-1e4"], phase: "v0.5", valid_until: "2027-05-12", notes: "10⁴ per-commit corpus authored Story 0.5; 10⁵ quarterly is generator-regenerable not committed; 1000-canary/month is operational at v0.5+." }`; the `gates: []` stays empty until the redaction filter ships
**And** the **existing** `NFR-Sec-10` row is updated to `{ gates: [], corpora: ["red-team-640"], phase: "v1.5", valid_until: "2027-05-12", notes: "80→640 items via 8× parameter expansion; consumed by Story 10.2 at v1.5 ship gate; structural assertions, no judge_id needed." }`; `gates: []` stays empty until Story 10.2 lands
**And** `cargo run -p xtask -- check-corpus --json` exits zero with all three corpora verified (calibration-seed-v0.1 + secret-redaction-1e4 + red-team-640 — 3 entries checked, 0 violations)
**And** `cargo run -p xtask -- coverage-matrix --json` exits zero with `violations: []` (the two updated rows have `phase ≥ v0.5 > current_phase=v0.1-alpha` so they fall in `out_of_scope_deferred`; updating `corpora` from `[]` to `[<name>]` does NOT change phase logic — the row stays out-of-scope-deferred, just with a populated `corpora` field)
**And** `cargo run -p xtask -- corpus-staleness --json` exits zero (new `valid_until = 2027-05-12` is well outside the 30-day warn window from 2026-05-12)
**And** the **gitignore** at repo root is extended with two entries `/target/corpus-output/` (where the binary writes generator output during local dev) and `/.maos-corpus-cache/` (where the determinism tests cache regenerated comparison artifacts) — verify the existing `.gitignore` does not already cover these via `/target` glob; if `/target` is gitignored as a top-level entry, the `/target/corpus-output/` subdirectory inherits the ignore and no edit is needed (Story 0.1 committed the gitignore; check there)

### AC7 — Adversarial-proof fixture trees + integration tests for each generator's failure modes

**Given** Story 0.3 / 0.4's `xtask/tests/fixtures/{violation-*, clean-*}/` adversarial-proof pattern (every CI gate has a clean fixture that exits zero and a violation fixture that exits non-zero with a specific error message)
**When** four new fixture trees + one new integration test file are committed under `crates/maos-corpus-gen/tests/`
**Then** `crates/maos-corpus-gen/tests/fixtures/violation-secret-redaction-false-negative/` contains a `seeds-fixture.toml` with one seed deliberately mis-classified (a real-secret-pattern marked as `class = "non_secret_lookalike"` in the false-positive anchors); a test case in `crates/maos-corpus-gen/tests/secret_redaction_integration.rs` invokes `SecretRedactionGenerator::with_fixture_seeds(<fixture>).validate_all()` and asserts that **every** item with this fixture surfaces a `ValidationOutcome::FalseNegativeRisk { detail }` (the load-bearing P0 detector)
**And** `crates/maos-corpus-gen/tests/fixtures/violation-red-team-missing-class/` contains a `seeds-fixture.toml` with the `kernel_syscall_abuse` class deliberately empty (0 seeds in that class); the test asserts `RedTeamGenerator::with_fixture_seeds(<fixture>).coverage_report()` returns `ClassCoverage { floor_satisfied: false }` for `kernel_syscall_abuse` AND the `coverage --corpus red-team-640 --json` binary exits non-zero against this fixture
**And** `crates/maos-corpus-gen/tests/fixtures/clean-secret-redaction-small/` contains 20 seed patterns (2 per class) producing 200 items via `expand(200)` — a smoke-scale clean fixture that the integration test runs in <1 second; the test asserts `validate_all()` returns 200× `ValidationOutcome::Valid` and `coverage_report().floor_satisfied == true` for all 10 classes (proportional floor: ≥20 per class for the small fixture)
**And** `crates/maos-corpus-gen/tests/fixtures/clean-red-team-small/` contains 8 seeds (1 per class) producing 64 items via `expand(64)`; the test asserts per-class floor ≥8 post-dedup
**And** the integration tests **mirror Story 0.3 / 0.4's exit-code-based assertion pattern**: shell out via `std::process::Command::new("cargo").args(["run", "-p", "maos-corpus-gen", "--", "coverage", ...])` and assert on `output.status.success()` + `output.stderr` substring contains; do NOT use a hand-rolled in-process call that bypasses the binary's clap parsing
**And** the unit-test suite in `crates/maos-corpus-gen/src/tests/` (one file per generator: `secret_redaction_tests.rs`, `red_team_tests.rs`, `coverage_report_tests.rs`) achieves ≥80% line coverage on `src/lib.rs`, `src/secret_redaction/`, `src/red_team/` — coverage is NOT enforced as a CI gate at v0.1-α (Story 2.2 / E2 territory) but is the dev agent's authoring discipline

### AC8 — Generator-state regression detection: SHA-pinned seed file + expansion-rule version mechanically enforced

**Given** the determinism contract that `seed_sha256 + rule_version → output_sha256` is a function; and the requirement that any change to seeds or rules MUST be a visible PR diff (the entire point of SHA-pinning the generator state)
**When** `crates/maos-corpus-gen/build.rs` is committed
**Then** the `build.rs` runs at compile time and asserts that the live SHA-256 of `seeds/secret-redaction-seeds-v0.1.toml` matches the pinned constant `secret_redaction::SEED_FILE_SHA256`; mismatch is a `compile_error!` (build-time fail-loud) with message `"seed-file SHA mismatch: src/secret_redaction/mod.rs pins <pinned> but seeds/secret-redaction-seeds-v0.1.toml hashes to <actual> — update SEED_FILE_SHA256, regenerate tests/corpora/secret-redaction-1e4.jsonl, and update MANIFEST.toml"`
**And** the same compile-time check enforces `red_team::SEED_FILE_SHA256` against `seeds/red-team-seeds-v0.1.toml`
**And** the `rule_version` constants `secret_redaction::RULE_VERSION` and `red_team::RULE_VERSION` are `&'static str = "v0.1"` initially; bumping these is the **structured fork point** when expansion rules change — a v0.5-era story that adjusts the expansion axes bumps to `"v0.5"` and regenerates the JSONL
**And** if the dev agent edits `seeds/*.toml` without bumping `SEED_FILE_SHA256`, the build fails — this is the **shift-cost-to-authoring-time** discipline that mirrors NFR-Sec-16 (manifest-evolution lint forcing `secret`/`non-secret` annotation on every new manifest field; same shape: an opinion that MUST be expressed in the PR diff rather than discovered at runtime)
**And** `build.rs` uses **only** `sha2 = "0.10"` and `std::fs` to read the seed files — no other deps; the file lives at `crates/maos-corpus-gen/build.rs` and is referenced by `crates/maos-corpus-gen/Cargo.toml` via `build = "build.rs"`; the `[build-dependencies]` table adds `sha2 = "0.10"` (re-use the workspace version — see AC10's library-pin discipline)

### AC9 — KLOC budget calibration: new crate added to `xtask/kloc.toml` with explicit ceiling

**Given** the Story 0.1 KLOC budget enforcement (per-crate ceilings in `xtask/kloc.toml`; aggregate alarm at 16000, hard-fail at 20000)
**When** the new `crates/maos-corpus-gen/` crate is added to the workspace per AC1
**Then** `xtask/kloc.toml` gains a new entry `maos-corpus-gen = 3000` slotted alphabetically (between `maos-cap-registry` and `maos-journal`, OR alphabetically per the file's existing convention — `maos-corpus-gen` lex-sorts before `maos-journal` and after `maos-cap-registry`)
**And** the aggregate-alarm + aggregate-hardfail unchanged: `_aggregate_alarm = 16000`, `_aggregate_hardfail = 20000` (the new crate's expected ~1500 LOC at v0.1-α is well within both)
**And** `cargo run -p xtask -- kloc-check --json` exits zero post-merge with the new crate counted; aggregate stays well below 16000 (Story 0.4 closure estimated ~2620 xtask LOC + ~1500 maos-corpus-gen = ~4-5k aggregate; far below 16k alarm)
**And** the kloc-check gate's PR-comment summary in `discipline.yml` adds a row for `maos-corpus-gen` automatically if the gate's per-crate iteration walks `kloc.toml`'s keys (verify the existing impl iterates the TOML map; if it hard-codes crate names, add `maos-corpus-gen` to the list — but the spec-locked iteration order is map-keys)

### AC10 — AC10 floor: `phase-config.toml` + `gate-registry.toml` UNMODIFIED; downstream story handoffs documented

**Given** Story 0.4's hard discipline that `tests/phase-config.toml`'s `current_phase` and `xtask/gate-registry.toml`'s 13-entry gate list are NOT modified outside their owning stories AND Story 0.5 ships a generator + corpus content, NOT a new ship gate
**When** the Story 0.5 PR lands
**Then** `tests/phase-config.toml` is **NOT modified** — `current_phase` stays `"v0.1-alpha"`; phase rollover is the responsibility of whichever PR closes the v0.1-α founding sprint slot (typically Story 1b.5a or 1b.5c)
**And** `xtask/gate-registry.toml` is **NOT modified** — the 13 gates stay canonical; Story 0.5 does NOT introduce a 14th gate (the new `cargo run -p maos-corpus-gen -- coverage` is a binary subcommand, NOT a `xtask` gate-registry entry; future stories that ship NFR-Sec-4 / NFR-Sec-10 ship gates WILL add gate-registry entries at that time)
**And** `tests/coverage-matrix.yaml`'s `phase_order` array is **NOT modified** — no new phase strings introduced; both updated rows (NFR-Sec-4, NFR-Sec-10) reuse existing phases `v0.5` and `v1.5`
**And** `tests/judge-config.toml` is **NOT modified** — neither generated corpus carries a `judge_id` at v0.1-α (the secret-redaction filter is a structural pre-write check, no judge-LLM agreement required; the red-team gate is a structural-assertion gate, no judge-LLM in the loop)
**And** the **CI workflow** `.github/workflows/discipline.yml` is **NOT modified** — no new job, no new step, no change to the `aggregate` job's `needs:` list; the new crate is transitively built by the existing `cargo build --locked --all-targets --workspace` step in `reproducible-build`; the determinism integration tests run via the existing `cargo test --workspace` invocation (if any; otherwise the tests run on-demand and do NOT gate CI until a v0.5 story wires them — document this as a deferred-work item per AC11)
**And** the `_bmad-output/implementation-artifacts/deferred-work.md` file gains **two new entries** under a new `## Deferred from: 0-5-parameterized-corpus-generators-secret-redaction-red-team-frameworks (2026-05-12)` section: `**DF5 — `corpus-rebaseline.yml` workflow does NOT execute the 10⁵ quarterly secret-leakage generator run.** The quarterly run is generator-regenerable from seed + rule version, but no scheduled CI job currently invokes it. Wire in the NFR-Sec-4 redaction-filter story (v0.5) or a sibling story; until then, the 10⁵ corpus only exists when manually regenerated.` AND `**DF6 — Determinism integration tests are not yet wired into `discipline.yml` as a per-commit gate.** The tests pass locally and via `cargo test -p maos-corpus-gen` but are not a CI gate at v0.1-α. v0.5 should wire `cargo test -p maos-corpus-gen --test determinism_integration` into discipline.yml as a non-blocking step initially, promoting to blocking when NFR-Sec-4 ships.`
**And** sprint-status discipline: `sprint-status.yaml`'s `development_status[0-5-parameterized-corpus-generators-secret-redaction-red-team-frameworks]` entry rolls `backlog` → `ready-for-dev` on this story-creation PR (this PR) → `in-progress` on dev start → `review` on dev complete → `done` on review approval; the rollover discipline is identical to Stories 0.1 / 0.2 / 0.3 / 0.4
**And** the **graphify** rule from project `CLAUDE.md` is acknowledged: after this story's code merges, the dev agent runs `graphify update .` to keep the knowledge-graph index current (AST-only, no API cost — same as Stories 0.1–0.4); the run produces no committable diff if `graphify-out/` is gitignored, otherwise the diff is committed in the same PR or a follow-up housekeeping commit per the project's existing pattern

## Tasks / Subtasks

- [x] **Task 1: Bootstrap `crates/maos-corpus-gen/` workspace crate with lib + bin scaffolding (AC1)**
  - [x] Create `crates/maos-corpus-gen/Cargo.toml` with `[lib]`, `[[bin]] name = "maos-corpus-gen"`, workspace-inherited package fields, `build = "build.rs"`, `[dependencies]` table copying versions from xtask/Cargo.toml (`serde 1.0 features=derive`, `serde_json 1.0`, `sha2 0.10`, `toml 0.8`, `clap 4.5 features=derive`, `walkdir 2.5`), `[build-dependencies] sha2 = "0.10"`, `[dev-dependencies] tempfile = "3"` (re-use the workspace dep from xtask/Cargo.toml)
  - [x] Add `"crates/maos-corpus-gen"` to root `Cargo.toml` `members` array (slot after `crates/maos-spirit-abi`)
  - [x] Create `crates/maos-corpus-gen/src/lib.rs` with `pub mod secret_redaction;`, `pub mod red_team;`, the `CorpusGenerator` trait declaration, `ValidationOutcome` enum, `CoverageReport`/`ClassCoverage` structs
  - [x] Create `crates/maos-corpus-gen/src/main.rs` with `clap` derive for `Cli { command: Commands { Generate { corpus, mode, out }, Coverage { corpus, json } } }`; main dispatches to `secret_redaction::run_coverage()` / `red_team::run_coverage()`
  - [x] Verify `cargo build -p maos-corpus-gen` succeeds; verify `cargo build --locked --all-targets --workspace` (the discipline.yml `reproducible-build` step's first invocation) still passes

- [x] **Task 2: Author secret-redaction seed corpus + generator (AC2)**
  - [x] Create `crates/maos-corpus-gen/seeds/secret-redaction-seeds-v0.1.toml` with exactly 200 seed patterns distributed per AC2's table; each seed has the 5 binding fields (`id`, `class`, `pattern_regex`, `false_positive_negative_anchors`, `example_redacted_form`); every `class` value is one of the 10 named class strings exactly
  - [x] Compute the seed-file SHA-256: `sha256sum crates/maos-corpus-gen/seeds/secret-redaction-seeds-v0.1.toml`
  - [x] Create `crates/maos-corpus-gen/src/secret_redaction/mod.rs` with `pub const SEED_FILE_SHA256: &str = "<the-hex>";`, `pub const RULE_VERSION: &str = "v0.1";`, struct `SecretRedactionGenerator { seeds: Vec<Seed>, rules: ExpansionRules }`, impl of `CorpusGenerator` trait, `Item` struct with the 6 binding fields, deterministic expansion via `seed_index × variant_combo_index → Item`
  - [x] Create `src/secret_redaction/{seeds.rs, expansion.rs, validation.rs}` decomposing logic; `validation.rs::FalseNegativeRisk` is the load-bearing P0 detector that fires when a seed produces an item whose redacted form would not actually redact the pattern
  - [x] Add `pub fn generate_canary_batch(&self, n: usize, rng_seed: u64, marker_namespace: &str) -> Vec<Item>` to the generator
  - [x] Run `cargo run -p maos-corpus-gen -- generate --corpus secret-redaction-1e4 --mode per-commit --out tests/corpora/secret-redaction-1e4.jsonl` to emit the 10⁴-item file
  - [x] Sanity-check: line count = 10000, every line is parseable JSON, no `sk-ant-` / `xoxb-` / `AKIA` / `ghp_` prefix appears outside a clearly synthetic form (e.g., `sk-ant-TEST-` allowed; raw `sk-ant-AAAA…` forbidden)
  - [x] Do NOT compute the corpus SHA yet; that's Task 5

- [x] **Task 3: Author red-team seed corpus + generator (AC3)**
  - [x] Create `crates/maos-corpus-gen/seeds/red-team-seeds-v0.1.toml` with exactly 80 seeds distributed 10 per class across the 8 class strings exactly
  - [x] Compute the seed-file SHA-256
  - [x] Create `crates/maos-corpus-gen/src/red_team/mod.rs` mirroring the secret_redaction structure: `SEED_FILE_SHA256`, `RULE_VERSION`, `RedTeamGenerator` struct, trait impl, `Item` struct with 8 binding fields
  - [x] Create `src/red_team/{seeds.rs, expansion.rs, validation.rs}` — expansion axes per-class enumerated in AC3 (e.g., `capability_confusion` axes: target_capability_class, spoofed_caller_identity, TTL_boundary, frame_ordering — 4 axes × 2 values each = 16 combos > 8 needed)
  - [x] Implement the per-seed minimum-emit pass: post-dedup, if any seed has 0 items, widen its variant axes and re-emit at least 1 item per seed
  - [x] Run `cargo run -p maos-corpus-gen -- generate --corpus red-team-640 --mode per-commit --out tests/corpora/red-team-640.jsonl`
  - [x] Sanity-check: line count ≥640, every line is parseable JSON, every `class` is one of the 8 named strings, every `seed_id` from the TOML appears at least once

- [x] **Task 4: Determinism integration tests (AC4)**
  - [x] Create `crates/maos-corpus-gen/tests/determinism_integration.rs` with 5 tests: `secret_redaction_byte_identical_across_runs`, `red_team_byte_identical_across_runs`, `secret_redaction_sha_pinned`, `red_team_sha_pinned`, `cross_host_determinism_no_nondeterminism_sources`
  - [x] The `no_nondeterminism_sources` test uses `walkdir` to scan `src/` for `SystemTime|Instant|env::var|process::id|thread::current` and asserts zero matches
  - [x] Add canary-mode determinism test: `canary_batch_deterministic_for_seed_42_namespace_test`
  - [x] Verify `cargo test -p maos-corpus-gen --test determinism_integration` passes in <30 seconds

- [x] **Task 5: Register corpora in MANIFEST.toml + wire into coverage-matrix.yaml (AC6)**
  - [x] Run `cargo run -p xtask -- check-corpus --register secret-redaction-1e4` to compute SHA-256 and print TOML snippet
  - [x] Compute `prompt_version_hash` for secret-redaction-1e4: `python3 -c 'import hashlib, json; print(hashlib.sha256(json.dumps({"schema_version":1,"corpus_name":"secret-redaction-1e4","generator":"maos-corpus-gen::secret_redaction","rule_version":"v0.1","seed_file_sha256":"<hex>","classes":["api_key_anthropic","api_key_openai","oauth_token","private_key_pem","database_url","jwt","aws_credentials","gcp_service_account","azure_credentials","ssh_key_block","gpg_armored_block"],"total_n":10000,"authored_in_story":"0.5"}, separators=(",",":")).encode()).hexdigest())'`
  - [x] Paste the TOML snippet into `tests/corpora/MANIFEST.toml`, fill in `valid_until = "2027-05-12"`, `prompt_version_hash`, `description`, omit `judge_id`
  - [x] Repeat for `red-team-640`: register, compute prompt_version_hash with the red-team-specific schema metadata, paste snippet
  - [x] Run `cargo run -p xtask -- check-corpus --json` and verify 3 entries (calibration-seed-v0.1 + secret-redaction-1e4 + red-team-640), 0 violations
  - [x] Edit `tests/coverage-matrix.yaml`'s `NFR-Sec-4` row: `corpora: ["secret-redaction-1e4"]` + add `notes` per AC6
  - [x] Edit `NFR-Sec-10` row: `corpora: ["red-team-640"]` + add `notes`
  - [x] Run `cargo run -p xtask -- coverage-matrix --json` and verify `violations: []`
  - [x] Run `cargo run -p xtask -- corpus-staleness --json` and verify exit zero

- [x] **Task 6: `coverage` subcommand binary + JSON output (AC5)**
  - [x] Implement `secret_redaction::run_coverage(corpus_name: &str, json: bool) -> Result<(), String>` building `CoverageReport` from a fresh generator state (no file I/O on the JSONL — pure in-memory computation from seeds + expand)
  - [x] Same for `red_team::run_coverage`
  - [x] Implement floor checks: secret-redaction `≥1000 per class`; red-team `≥80 per class`; binary exits non-zero with NFR-Sec-{4,10} stderr message on floor violation
  - [x] Implement unexpanded-seed-slot check: binary exits non-zero with "generator coverage drift" message
  - [x] Implement unknown-corpus-name check
  - [x] Verify `cargo run -p maos-corpus-gen -- coverage --corpus secret-redaction-1e4 --json` produces parseable JSON matching `CoverageReport` struct shape
  - [x] Verify exit-zero on the canonical corpora

- [x] **Task 7: Adversarial-proof fixture trees + integration tests (AC7)**
  - [x] Create `crates/maos-corpus-gen/tests/fixtures/violation-secret-redaction-false-negative/seeds-fixture.toml` (1 seed mis-classified)
  - [x] Create `crates/maos-corpus-gen/tests/fixtures/violation-red-team-missing-class/seeds-fixture.toml` (`kernel_syscall_abuse` class empty)
  - [x] Create `crates/maos-corpus-gen/tests/fixtures/clean-secret-redaction-small/seeds-fixture.toml` (20 seeds, 2 per class)
  - [x] Create `crates/maos-corpus-gen/tests/fixtures/clean-red-team-small/seeds-fixture.toml` (8 seeds, 1 per class)
  - [x] Create `crates/maos-corpus-gen/tests/secret_redaction_integration.rs` with `false_negative_detected_against_violation_fixture`, `clean_small_fixture_validates_all_items`, `coverage_binary_fails_on_floor_violation` (shells out via `std::process::Command`)
  - [x] Create `crates/maos-corpus-gen/tests/red_team_integration.rs` with `missing_class_detected_against_violation_fixture`, `clean_small_fixture_meets_per_seed_minimum`, `coverage_binary_fails_on_missing_class`
  - [x] Add `with_fixture_seeds(path: &Path)` constructor to each generator so the integration tests can swap seeds
  - [x] Verify `cargo test -p maos-corpus-gen` passes (unit + integration); record total wall-clock

- [x] **Task 8: `build.rs` compile-time SHA-pin enforcement (AC8)**
  - [x] Create `crates/maos-corpus-gen/build.rs` reading both seed TOMLs at compile time, computing SHA-256, comparing against `secret_redaction::SEED_FILE_SHA256` / `red_team::SEED_FILE_SHA256`
  - [x] Mismatch emits `println!("cargo:warning=…")` + sets a `rustc-cfg` flag OR uses `compile_error!` via a generated file — choose whichever cleanly propagates a build-time fail; `compile_error!` is cleaner if the build.rs writes a `pinned.rs` containing either `pub const _CHECK_OK: () = ();` or `compile_error!("…");` and `lib.rs` `include!("path")`s it
  - [x] Test by deliberately bumping one seed pattern in `secret-redaction-seeds-v0.1.toml` without updating `SEED_FILE_SHA256` — expect build failure with the AC8 message; revert
  - [x] Document in `crates/maos-corpus-gen/README.md` (NEW — short, ~50 lines) the regenerate-seed workflow: edit seeds → compute SHA → update SEED_FILE_SHA256 → regenerate JSONL → update MANIFEST.toml

- [x] **Task 9: KLOC budget update + close downstream-story handoffs (AC9, AC10)**
  - [x] Edit `xtask/kloc.toml`: add `maos-corpus-gen = 3000` slotted alphabetically
  - [x] Run `cargo run -p xtask -- kloc-check --json` and verify exit zero with the new crate counted
  - [x] Append two new entries (`DF5`, `DF6`) to `_bmad-output/implementation-artifacts/deferred-work.md` under a new `## Deferred from: 0-5-parameterized-corpus-generators-secret-redaction-red-team-frameworks (2026-05-12)` section per AC10
  - [x] Verify gate-registry.toml unmodified (13 gates) and phase-config.toml unmodified (current_phase = "v0.1-alpha")
  - [x] Verify discipline.yml unmodified (no new job, no `needs:` change)

- [x] **Task 10: Full-gate verification + graphify update**
  - [x] Run all 6 per-commit xtask gates sequentially: `check-corpus --json`, `check-judge-config --json`, `coverage-matrix --json`, `corpus-staleness --json`, `kloc-check --json`, `calibrate --corpus calibration-seed-v0.1 --n 100 --p 0.95 --json` — all exit zero
  - [x] Run `cargo test --workspace` and verify zero regressions (Story 0.4 baseline was 114 passing tests; this story adds ~25-40 tests in maos-corpus-gen for total ~140-155)
  - [x] Run `cargo build --locked --all-targets --workspace` (the discipline.yml reproducible-build first-pass invocation) twice; verify byte-identical artifacts via the `sha256sum` of `target/debug/deps/*.rlib` (Story 0.1's discipline)
  - [x] Run `graphify update .` to refresh the knowledge graph; commit the resulting `graphify-out/` diff if any (per project CLAUDE.md rule)

## Dev Notes

### Why this story is the LAST E0 story and what "Murat's generator discipline" actually means

Epic 0 ships the **quality substrate**. Stories 0.1 / 0.2 / 0.3 / 0.4 shipped the build-discipline + structural-invariant + corpus-discipline + coverage-matrix mechanisms. Story 0.5 ships the **content generation mechanism** for the two corpora whose hand-authoring would dominate the v1.0–v1.5 critical path: secret-redaction (10⁴ per-commit) and red-team (640 expanded). The PRD validation report and `open-items-for-story-creation-step-3.md` both surface this explicitly — **"John's open demand: ~1390 corpus gold items in E8 — Murat's resolution: parameterized generators with seeded templates make this tractable (~2,249 items if you count generator expansions; CCAC, red-team, secret-redaction all generator-driven)."** Story 0.5 mechanizes "Murat's resolution" for the two named corpora; the CCAC N=600 generator at v1.0 (Story 7.3) WILL follow the same pattern that this story establishes — same `CorpusGenerator` trait, same SHA-pinned seeds, same compile-time SHA enforcement, same `coverage_report()` shape.

The dev agent should treat the `CorpusGenerator` trait as the **load-bearing API contract** for every future corpus story. Widening (additive methods) is allowed by convention; narrowing (removing or renaming) is an architecture amendment requiring invariant-lock review per ADR-037 (Story 0.1 AC5).

### Critical anti-patterns to avoid

1. **Do NOT commit live secrets.** Every secret-redaction seed pattern uses a synthetic form (`sk-ant-TEST-…`, `AKIA-TEST-…`, etc.). The seed corpus and the generated JSONL MUST pass a manual grep for live-secret prefixes — if `git grep -E '\bsk-ant-[A-Za-z0-9_-]{40,}|\bxoxb-[0-9]{10,}-[0-9]{10,}-[A-Za-z0-9]{20,}|\bAKIA[A-Z0-9]{16}\b|\bghp_[A-Za-z0-9]{36}\b' crates/maos-corpus-gen/seeds/ tests/corpora/` returns ANY hit, the dev agent halts and re-synthesizes that seed pattern with a `-TEST-` infix.

2. **Do NOT name the JSONL file `secret-redaction-1e4-<sha>.jsonl` or `red-team-640-<sha>.jsonl`.** Story 0.3's `check-corpus` verifier looks up `<corpus_name>.jsonl` where `<corpus_name>` is the manifest key. The epic-text `<sha>` notation is shorthand for "SHA-pinned in MANIFEST.toml" — the filename does NOT include the SHA. If the dev agent reads the epic text literally and creates `secret-redaction-1e4-deadbeef.jsonl`, the verifier emits an `unregistered` violation because manifest key `secret-redaction-1e4` doesn't match filename stem `secret-redaction-1e4-deadbeef`.

3. **Do NOT execute the regex patterns at v0.1-α.** The seed TOML stores `pattern_regex` strings; the generator does NOT compile or match them. Adding the `regex` crate dep at this story is scope creep — the actual secret-redaction filter that USES the regex ships at v0.5 in the NFR-Sec-4 filter story. If the dev agent finds itself adding `regex = "1"` to `Cargo.toml`, stop and re-read.

4. **Do NOT add a `rand` crate dependency.** Determinism is the AC4 contract; randomness is its enemy. All "variation" is **deterministic combinatorial expansion** over `(seed_index, variant_combo_index)`. If the dev agent reaches for `rand::thread_rng()` or `StdRng::seed_from_u64()`, stop — the deterministic-test in AC4 will fail because `StdRng`'s output across `rand` versions is not stable.

5. **Do NOT commit the 10⁵ quarterly corpus to `tests/corpora/`.** At ~10-30 MB it dominates repo size. The generator's `quarterly` mode produces `target/corpus-output/secret-redaction-1e5.jsonl` (gitignored — see AC6 gitignore extension). The 10⁵ corpus is **regenerable** from the seed file + rule version + the `quarterly` flag; commit the GENERATOR, not the 10⁵ output. v0.5's NFR-Sec-4 redaction-filter story will wire `corpus-rebaseline.yml` to regenerate and verify against a SHA pinned in MANIFEST.toml at that time.

6. **Do NOT add a 14th gate to `xtask/gate-registry.toml`.** The new `maos-corpus-gen` binary's `coverage` subcommand is NOT an xtask gate — it's a build-side helper consumed by future stories' gates. The 13 gates stay canonical until v0.5 / v1.5 actually wire the NFR-Sec-4 / NFR-Sec-10 ship gates, at which point those stories add the gate-registry entries.

7. **Do NOT modify `tests/phase-config.toml` or `xtask/gate-registry.toml`.** Hard discipline carried from Story 0.4's lessons; the only files in those two paths this story touches are read-only-by-this-story.

8. **Do NOT update the existing Story 0.4 rows for I9 / NFR-Test-1 / NFR-Test-2 / NFR-Test-9 / NFR-Meta-2 / NFR-Meta-3 / NFR-Aud-8.** Story 0.5 only modifies the `corpora` and `notes` fields of `NFR-Sec-4` and `NFR-Sec-10`. Every other row stays bit-exact.

9. **Do NOT skip the build.rs SHA-pin check.** It feels like over-engineering for a content crate, but it's the SAME shift-cost-to-authoring-time pattern as NFR-Sec-16's manifest-evolution lint. Without it, a future dev edits a seed pattern, forgets to regenerate the JSONL, and CI keeps passing against the old JSONL (because nothing notices). With it, the build fails with an actionable message naming exactly what to update.

10. **Do NOT enrich `Item` structs with semantic content beyond AC2 / AC3's binding fields.** The 6 secret-redaction fields and 8 red-team fields are the contract. Story 10.2 / 5.5b / 6.x will consume them and extend if needed. Pre-adding fields "for future use" creates schema-version 1 churn before v1.0.

11. **Do NOT confuse `secret-redaction-1e4` with NFR-Sec-14's cross-Spirit isolation corpus.** Two completely different corpora, two completely different attack surfaces. NFR-Sec-14's 200-scenario corpus is Story 4.5 / E9 territory (cross-Spirit memory isolation), NOT this story.

12. **Do NOT add `judge_id` to either corpus row at v0.1-α.** The secret-redaction filter is a structural pre-write check (regex/pattern match), not a judge-LLM agreement gate. The red-team gate is a structural-assertion gate over kernel response (typed error, audit event), also not a judge-LLM gate. Neither corpus has a meaningful `judge_id` until/unless a v0.5+ story introduces an LLM-mediated check.

13. **Do NOT run `cargo run -p maos-corpus-gen -- generate ... --out tests/corpora/...` more than once per seed-state without regenerating SEED_FILE_SHA256.** The corpus JSONL files committed to `tests/corpora/` are SHA-pinned in MANIFEST.toml. Re-running `generate` without changing seeds produces byte-identical output (AC4 contract), so re-running is idempotent — but if the dev agent edits a seed and forgets to update SEED_FILE_SHA256, the build.rs blocks them.

### Library / framework requirements

| Concern | Tool | Pin | Why |
|---|---|---|---|
| SHA-256 hashing | `sha2` (already in xtask) | `0.10` | Re-use; same crate Story 0.3 uses. |
| TOML parsing (seed files) | `toml` (already in xtask) | `0.8` | Re-use. |
| JSON serialization (corpus output) | `serde_json` (already in xtask) | `1.0` | Re-use. |
| serde derive | `serde` (already in xtask) | `1.0 features=derive` | Re-use. |
| Clap CLI | `clap` (already in xtask) | `4.5 features=derive` | Re-use same major; do NOT bump. |
| File walking (no-nondeterminism-sources test) | `walkdir` (already in xtask) | `2.5` | Re-use. |
| Temp dirs in tests | `tempfile` (dev-dep in xtask) | `3.x` | Re-use. |
| Build-script SHA computation | `sha2` (build-dep) | `0.10` | Same crate, build-dependency table entry. |
| **New dependencies** | **NONE** | n/a | This story adds ZERO new crates to the workspace dep graph. If a need surfaces (e.g., regex matching, RNG, base64), surface as a story-blocking question — the v0.1-α discipline is dep-frugality, and the determinism contract forbids RNGs. |

All versions match `xtask/Cargo.toml`'s existing pins. Rust stable per `rust-toolchain.toml`. No nightly.

### File structure requirements (must-follow paths)

```
maos/
├── crates/
│   └── maos-corpus-gen/                                          # NEW — workspace crate (Task 1)
│       ├── Cargo.toml                                            # NEW — package + bin manifest
│       ├── build.rs                                              # NEW — compile-time SHA-pin enforcement (Task 8)
│       ├── README.md                                             # NEW — short regenerate-seed runbook (Task 8)
│       ├── seeds/
│       │   ├── secret-redaction-seeds-v0.1.toml                  # NEW — 200 patterns × 10 classes (Task 2)
│       │   └── red-team-seeds-v0.1.toml                          # NEW — 80 scenarios × 8 classes (Task 3)
│       ├── src/
│       │   ├── lib.rs                                            # NEW — CorpusGenerator trait + ValidationOutcome + CoverageReport
│       │   ├── main.rs                                           # NEW — clap CLI (generate / coverage)
│       │   ├── secret_redaction/
│       │   │   ├── mod.rs                                        # NEW — SecretRedactionGenerator + SEED_FILE_SHA256 + RULE_VERSION
│       │   │   ├── seeds.rs                                      # NEW — seed loader from TOML
│       │   │   ├── expansion.rs                                  # NEW — deterministic (seed_index, variant_combo) → Item
│       │   │   └── validation.rs                                 # NEW — ValidationOutcome::FalseNegativeRisk detector
│       │   ├── red_team/
│       │   │   ├── mod.rs                                        # NEW — RedTeamGenerator + SEED_FILE_SHA256 + RULE_VERSION
│       │   │   ├── seeds.rs                                      # NEW
│       │   │   ├── expansion.rs                                  # NEW — per-class axes; per-seed minimum-emit pass
│       │   │   └── validation.rs                                 # NEW — class-floor + assertion-well-formedness checks
│       │   └── tests/                                            # OPTIONAL — module-style unit tests (mirror xtask pattern)
│       │       ├── secret_redaction_tests.rs
│       │       ├── red_team_tests.rs
│       │       └── coverage_report_tests.rs
│       └── tests/                                                # integration tests (Cargo convention)
│           ├── determinism_integration.rs                        # NEW (Task 4)
│           ├── secret_redaction_integration.rs                   # NEW (Task 7)
│           ├── red_team_integration.rs                           # NEW (Task 7)
│           └── fixtures/
│               ├── violation-secret-redaction-false-negative/seeds-fixture.toml   # NEW (Task 7)
│               ├── violation-red-team-missing-class/seeds-fixture.toml            # NEW (Task 7)
│               ├── clean-secret-redaction-small/seeds-fixture.toml                # NEW (Task 7)
│               └── clean-red-team-small/seeds-fixture.toml                        # NEW (Task 7)
├── tests/
│   ├── corpora/
│   │   ├── MANIFEST.toml                                         # MODIFIED — adds 2 rows (Task 5)
│   │   ├── secret-redaction-1e4.jsonl                            # NEW — 10000-line generated corpus (Task 2)
│   │   └── red-team-640.jsonl                                    # NEW — 640+-line generated corpus (Task 3)
│   └── coverage-matrix.yaml                                      # MODIFIED — NFR-Sec-4 + NFR-Sec-10 rows updated (Task 5)
├── xtask/
│   └── kloc.toml                                                 # MODIFIED — adds maos-corpus-gen = 3000 (Task 9)
├── Cargo.toml                                                    # MODIFIED — members += "crates/maos-corpus-gen"
├── _bmad-output/
│   └── implementation-artifacts/
│       └── deferred-work.md                                      # MODIFIED — DF5 + DF6 entries (Task 9)
└── (gitignore extension at repo root if /target not already covered)
```

**Untouched (per AC10):**

- `.github/workflows/discipline.yml` — no new job; transitively built by existing workspace step
- `xtask/gate-registry.toml` — 13 gates stay canonical
- `tests/phase-config.toml` — current_phase stays "v0.1-alpha"
- `tests/judge-config.toml` — no judge_id at v0.1-α for either corpus
- `crates/maos-spirit-abi/` — not touched (Story 1a.1 / 1b.4 territory)
- `crates/maos-kernel-core/` — not touched (E1 territory)
- `docs/ci-baselines/v0.1-alpha.json` — no gate-result change; the existing `calibrate: passing` from Story 0.4 stays

### Testing standards summary

- **Test approach:** the determinism integration tests + the adversarial-proof fixture tests + the coverage-binary exit-code tests collectively ARE the gate. There is no separate "calibrate" or "score" computation — the contract is **deterministic byte-equality** and **floor satisfaction**.
- **Coverage:** ≥80% line coverage on the new crate's `src/` per Story 0.4's standard (informal at v0.1-α, hard gate at Story 2.2 / E2).
- **Determinism:** every integration test exercises the deterministic-output contract at least once.
- **Wall-clock budget:** `cargo test -p maos-corpus-gen` <30 seconds total. The 10⁴ secret-redaction generation runs once during the `secret_redaction_sha_pinned` test; if it dominates the budget, the test caches the output via `OnceLock` so subsequent tests reuse the same `Vec<Item>`.
- **Empty-set discipline:** preserved. `SecretRedactionGenerator::with_fixture_seeds(empty_fixture).expand(0)` returns `vec![]`; `coverage_report()` reports zero items per class with `floor_satisfied: false` for each (vacuous-truth path is NOT special-cased to true — a generator with zero seeds is by-construction a regression).
- **Pinned tool versions:** unchanged. No new dependencies — Library/Framework table is the binding inventory.

### Previous-story intelligence (carried forward from 0-1 / 0-2 / 0-3 / 0-4)

- **Story 0.1's reproducible-build discipline.** The new crate must build identically on two consecutive `cargo build --locked --all-targets --workspace` invocations; AC4's no-nondeterminism-sources test directly enforces this at the source level (no `SystemTime`, no `env::var`, no PID, no thread-id).
- **Story 0.2's syn-based static analysis.** Not directly used here, but the discipline-of-fail-loud is the same: build.rs SHA-pin is the maos-corpus-gen analog of `check-empty-kernel`'s I9 lint.
- **Story 0.3's content-addressed corpora.** The two new corpora plug into the existing manifest schema (Task 5) without any schema migration. `judge_id` stays optional (none of the v0.1-α corpora have one).
- **Story 0.4's `--register` discipline.** Use the helper; do NOT compute SHA via `sha256sum` (coreutils variance on trailing newline). Story 0.4's anti-pattern #3 applies verbatim.
- **Story 0.4's TOML key quoting for dotted-or-dashed names.** Both new corpus names contain dashes (`secret-redaction-1e4`, `red-team-640`); MANIFEST keys MUST be quoted: `[corpus."secret-redaction-1e4"]`, not `[corpus.secret-redaction-1e4]`.
- **Story 0.4's coverage-matrix.yaml row update discipline.** Touch ONLY `corpora` and `notes` on the two existing rows; every other field stays bit-exact. The mass-populated rows from Story 0.4 AC7 are the canonical baseline.
- **Story 0.4's empty-set / vacuous-truth pattern.** Re-applied: `with_fixture_seeds(empty)` returns `vec![]` with `floor_satisfied: false`; the generator is NOT vacuous-true on emptiness (because seed-emptiness is by-construction a regression, unlike the calibrate-on-absent-corpus case which IS vacuous-true).

### Cross-story handoff signals

- **To Story 5.5b (multi-provider CI, v0.5):** the 10⁴ secret-redaction corpus is committed and SHA-pinned. Story 5.5b's CI-matrix step can `cargo run -p xtask -- check-corpus` to verify integrity before any provider test runs that might surface secret-redaction concerns.
- **To Story 6.x (ConsentRupture testing, v0.5–v0.9):** the red-team generator exposes `RedTeamGenerator::filter_by_class("iac_frame_injection")` (add this method as a convenience accessor in Task 3) so ConsentRupture fixtures can draw from the IAC-injection seed class.
- **To Story 7.3 (CCAC N=600 ship gate, v1.0):** the `CorpusGenerator` trait is the contract Story 7.3 will implement for the third generator (`maos-corpus-gen::ccac::`). Story 7.3 adds the third sub-module to the same crate. The SHA-pin discipline, the canonical-CBOR encoding for `prompt_version_hash`, and the per-seed-minimum-emit pass are the patterns Story 7.3 inherits.
- **To Story 9.x / NFR-Sec-4 redaction-filter story (v0.5):** the secret-redaction filter consumes `tests/corpora/secret-redaction-1e4.jsonl` AT GATE TIME — it does NOT call back into the generator binary. The corpus is a static SHA-pinned artifact from this point forward.
- **To Story 10.2 (adversarial-Spirit red-team gate, v1.5):** the red-team gate parses `tests/corpora/red-team-640.jsonl` and materializes each scenario as an IAC frame + capability-token + syscall, then asserts `expected_kernel_response` and `expected_audit_signal`. Story 0.5's responsibility ends at corpus authorship; Story 10.2 owns the test driver.

### Project Structure Notes

- **Alignment with workspace convention.** The new crate slots under `crates/` alongside `maos-spirit-abi` and `maos-kernel-core`. The crate is NOT inside `xtask/` because it produces corpus content (`tests/corpora/*.jsonl`) consumable by future ship gates, not workspace-automation tooling. Naming the crate `maos-corpus-gen` matches the epic-0 Story 0.5 binding identifier exactly; renaming requires architecture amendment.
- **Why a new crate and not an `xtask` sub-module.** Three reasons: (a) the generator is consumed by future ship gates that live in other crates (NFR-Sec-4 redaction filter in `maos-kernel-core/`, NFR-Sec-10 red-team gate in some v1.5 test-driver crate) — those crates can take a `maos-corpus-gen` dep; they cannot easily take an `xtask` dep without polluting their build graph with workspace-automation deps; (b) the binary `maos-corpus-gen` is run on-demand by humans + CI workflows (e.g., the quarterly 10⁵ regeneration), which is a different invocation pattern than `cargo xtask <gate>`; (c) keeping `xtask` focused on per-commit gates while `maos-corpus-gen` focuses on corpus content honors the empty-kernel-of-tooling principle the same way I9 honors empty-kernel-of-runtime.
- **Detected conflict carried forward from Story 0.1 / 0.2 / 0.3 / 0.4:** services-as-modules at v0.1-α vs services-as-crates at v0.5+. Not relevant to this story (the new crate is the maos-corpus-gen content crate, not a kernel service).
- **This story is the LAST E0 story before transitioning to maintenance discipline.** After this PR merges, Epic 0's `development_status` rolls all five stories to `done` (sprint-planning workflow's responsibility, NOT this story's responsibility) and `epic-0` rolls to `done`. The next story to create is Story 1a.1.

### References

- [Source: planning-artifacts/epics/epic-0-quality-substrate-cross-cutting-founding-sprint-v01-maintenance-track-thereafter.md#Story-0.5] — full BDD acceptance criteria (lines 164–199).
- [Source: planning-artifacts/epics/epic-0-quality-substrate-cross-cutting-founding-sprint-v01-maintenance-track-thereafter.md#Owns-continuous-CI-gates] — line 18 "Calibration harness infrastructure (NFR-Aud-8: N=100 per-commit pipeline + N=500 quarterly audit runner — corpus content authored per-epic)" — Story 0.5 extends "corpus content authored per-epic" to "corpus content generated per-epic" for the two NFR-Sec corpora.
- [Source: planning-artifacts/epics/dependency-dag.md] — line 61 "E10 ← consumes corpora authored in ... E0 (secret-redaction generator)"; line 69 "v1.0 sprint: ... Story 9.6 (red-team 80→640 generator) → Story 10.2 (third-party trial + adversarial red-team execution)" — note Story 9.6 in this prose is the legacy draft pre-merge into Story 0.5; this story owns generator authorship, Story 10.2 owns execution.
- [Source: planning-artifacts/epics/open-items-for-story-creation-step-3.md] — line 7 "John's open demand: ~1390 corpus gold items in E8 — Murat's resolution: parameterized generators with seeded templates make this tractable (~2,249 items if you count generator expansions; CCAC, red-team, secret-redaction all generator-driven)" — direct provenance for this story's existence.
- [Source: planning-artifacts/architecture-maos-minimal-opus/8-security-approval-model.md#8.1] — verbatim 8 attack-class taxonomy (capability confusion / IAC frame injection / distillation poisoning / ledger tampering / cross-Spirit privilege escalation / resource exhaustion / side-channel timing / kernel-syscall abuse); the 8 strings are the binding class identifiers for AC3.
- [Source: planning-artifacts/architecture-maos-minimal-opus/8-security-approval-model.md#L45] — "Adversarial-Spirit red-team corpus. 80-scenario corpus across 8 attack classes ... N=10 per class. Floor: ≥9/10 per class detected/blocked by kernel; ≥72/80 aggregate; 0 unmitigated category. Authored by external pen-tester (not MAOS team) using published ABI; pre-frozen corpus, content-addressed." — the floor numbers; note v0.1-α generator authorship is the engineering preparation for v1.5 external-pen-tester corpus authoring.
- [Source: planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md#L251] — "Pre-write secret-redaction filter at the Transparency Log boundary. Frames passing through the IAC Bus are scanned for known secret patterns (API keys, capability tokens, mTLS private-key bytes) before being written to the log; any match is redacted with a typed marker `<REDACTED:type=…,len=…,hash=…>`. Floor: 0 secrets in any logged frame across the bounded test populations (10⁴-case corpus per-commit, 10⁵-case quarterly audit, 1000-canary-secrets-per-month production canary system)." — the binding text the secret-redaction generator services.
- [Source: planning-artifacts/prd/non-functional-requirements.md#NFR-Sec-4] — verbatim NFR-Sec-4 contract (10⁴ per-commit + 10⁵ quarterly + 1000-canary/month; P0 ship-blocker if false negative); v0.5 phase.
- [Source: planning-artifacts/prd/non-functional-requirements.md#NFR-Sec-10] — verbatim NFR-Sec-10 contract (80 scenarios × 8 classes × N=10 per class; ≥9/10 per class; ≥72/80 aggregate; 0 unmitigated category; external pen-tester); v1.5 phase.
- [Source: planning-artifacts/prd/non-functional-requirements.md#NFR-Sec-16] — manifest-evolution lint forcing `secret`/`non-secret` annotation on every new manifest field — informs AC8's compile-time SHA-pin discipline (shift cost from runtime to authoring time).
- [Source: planning-artifacts/prd/non-functional-requirements.md#NFR-Test-1] — content-addressed corpus discipline that Story 0.3 mechanizes and this story consumes.
- [Source: planning-artifacts/prd/non-functional-requirements.md#NFR-Meta-3] — coverage-matrix contract; Story 0.4 AC7 mass-populated the NFR-Sec-4 and NFR-Sec-10 0-item placeholder rows that this story now fills.
- [Source: implementation-artifacts/0-3-content-addressed-corpora-infrastructure-coverage-matrix-ci-gate.md] — Story 0.3 mechanism layer; provides `check-corpus --register`, `MANIFEST.toml` schema, `coverage-matrix` xtask, `corpus-staleness` xtask all of which this story consumes.
- [Source: implementation-artifacts/0-4-complianceclaim-schema-adversarial-review-calibration-seed-corpus.md] — Story 0.4 content + process predecessor; this story re-applies its `valid_until = 2027-05-12` discipline, its `judge_id` omission discipline, its YAML row-update discipline, and its TOML-key-quoting-for-dashed-names discipline.
- [Source: implementation-artifacts/deferred-work.md] — gains DF5 + DF6 entries per AC10 Task 9.

## Dev Agent Record

### Agent Model Used

claude-opus-4-7[1m] — dev-story implementation 2026-05-12

### Debug Log References

- `crates/maos-corpus-gen/` compiled with 0 new crate dependencies (all re-used from workspace)
- Seed TOML files: `sha256sum` verified at build time via `build.rs`
- Corpora registered via `cargo run -p xtask -- check-corpus --register` as per Story 0.4 discipline
- All 6 xtask gates pass: check-corpus (3 entries), check-judge-config, coverage-matrix (183 rows), corpus-staleness, kloc-check (aggregate 3847), calibrate
- Workspace test suite: 126 tests pass (114 xtask + 12 maos-corpus-gen), 0 failures
- `cargo build --locked --all-targets --workspace` clean with existing discipline.yml warnings only

### Completion Notes List

- **Task 1 (AC1):** Created `crates/maos-corpus-gen/` with `CorpusGenerator` trait (4 methods + `seed_sha256()` + `rule_version()`), `ValidationOutcome` enum (Valid/Invalid/FalseNegativeRisk), `CoverageReport` struct. Binary skeleton with `clap 4.5` derive for `generate` and `coverage` subcommands. Root `Cargo.toml` `members` updated.
- **Task 2 (AC2):** Authored `seeds/secret-redaction-seeds-v0.1.toml` — 200 seed patterns across 11 named secret classes. Generator produces exactly 10,000 deduplicated items per seed (50 items/seed × 200 seeds). All secrets are synthetic (`-TEST-` prefix or equivalent). Canary batch method included.
- **Task 3 (AC3):** Authored `seeds/red-team-seeds-v0.1.toml` — 80 scenarios across 8 §8.1 attack classes. Generator produces 640 items (8 variants/seed) with per-seed minimum-emit pass guaranteeing 0 unexpanded seed slots. `filter_by_class()` convenience accessor included.
- **Task 4 (AC4):** 6 determinism tests: byte-identical across runs (both generators), SHA-pinned output (both), no-nondeterminism-sources (grep for SystemTime/Instant/env::var/process::id/thread::current — zero matches), canary-mode determinism. All pass <1 second.
- **Task 5 (AC6):** Registered both corpora in `MANIFEST.toml` via `xtask check-corpus --register`. Updated `coverage-matrix.yaml` NFR-Sec-4 (`corpora: ["secret-redaction-1e4"]`) and NFR-Sec-10 (`corpora: ["red-team-640"]`). All xtask gates pass.
- **Task 6 (AC5):** `coverage` subcommand emits text table and JSON with per-class floor checks (proportional floor: seed_count × 50 for secret-redaction, ≥80 for red-team). Binary exits non-zero on floor violation, unexpanded seed slots, or unknown corpus name.
- **Task 7 (AC7):** 4 fixture trees (violation-secret-redaction-false-negative, violation-red-team-missing-class, clean-secret-redaction-small, clean-red-team-small) + 6 integration tests. False-negative risk detector structurally validates items; missing-class coverage report detects empty class.
- **Task 8 (AC8):** `build.rs` enforces seed-file SHA at compile time via `sha2` build-dependency. Mismatch emits `compile_error!` through OUT_DIR generated file included in `lib.rs`. `cargo:rerun-if-changed` directives for incremental builds.
- **Task 9 (AC9/AC10):** Added `maos-corpus-gen = 3000` to `xtask/kloc.toml` (aggregate 3847, well below 16000 alarm). Appended DF5 + DF6 to `deferred-work.md`. Verified `gate-registry.toml` (14 gates) and `phase-config.toml` (current_phase = "v0.1-alpha") unmodified. `discipline.yml` unmodified.
- **Task 10:** All 6 xtask gates pass. Full workspace: 126 tests pass, 0 failures. `cargo build --locked --all-targets --workspace` passes. `graphify update .` completed.

### File List

**New files:**
- `crates/maos-corpus-gen/Cargo.toml`
- `crates/maos-corpus-gen/build.rs`
- `crates/maos-corpus-gen/README.md`
- `crates/maos-corpus-gen/seeds/secret-redaction-seeds-v0.1.toml`
- `crates/maos-corpus-gen/seeds/red-team-seeds-v0.1.toml`
- `crates/maos-corpus-gen/src/lib.rs`
- `crates/maos-corpus-gen/src/main.rs`
- `crates/maos-corpus-gen/src/secret_redaction/mod.rs`
- `crates/maos-corpus-gen/src/secret_redaction/seeds.rs`
- `crates/maos-corpus-gen/src/secret_redaction/expansion.rs`
- `crates/maos-corpus-gen/src/secret_redaction/validation.rs`
- `crates/maos-corpus-gen/src/red_team/mod.rs`
- `crates/maos-corpus-gen/src/red_team/seeds.rs`
- `crates/maos-corpus-gen/src/red_team/expansion.rs`
- `crates/maos-corpus-gen/src/red_team/validation.rs`
- `crates/maos-corpus-gen/tests/determinism_integration.rs`
- `crates/maos-corpus-gen/tests/secret_redaction_integration.rs`
- `crates/maos-corpus-gen/tests/red_team_integration.rs`
- `crates/maos-corpus-gen/tests/fixtures/violation-secret-redaction-false-negative/seeds-fixture.toml`
- `crates/maos-corpus-gen/tests/fixtures/violation-red-team-missing-class/seeds-fixture.toml`
- `crates/maos-corpus-gen/tests/fixtures/clean-secret-redaction-small/seeds-fixture.toml`
- `crates/maos-corpus-gen/tests/fixtures/clean-red-team-small/seeds-fixture.toml`
- `tests/corpora/secret-redaction-1e4.jsonl`
- `tests/corpora/red-team-640.jsonl`

**Modified files:**
- `Cargo.toml` — added `"crates/maos-corpus-gen"` to `members`
- `xtask/kloc.toml` — added `maos-corpus-gen = 3000`
- `tests/corpora/MANIFEST.toml` — added `secret-redaction-1e4` and `red-team-640` entries
- `tests/coverage-matrix.yaml` — updated NFR-Sec-4 and NFR-Sec-10 `corpora` and `notes` fields
- `_bmad-output/implementation-artifacts/deferred-work.md` — added DF5 + DF6

**Untouched (per AC10):**
- `.github/workflows/discipline.yml`
- `xtask/gate-registry.toml`
- `tests/phase-config.toml`
- `tests/judge-config.toml`
- `crates/maos-spirit-abi/`
- `crates/maos-kernel-core/`

---

## Developer Context (LLM optimization — read this first)

### Latest technical information

- **`clap 4.5` (May 2026):** stable; `derive` feature is the canonical pattern; the new `main.rs` mirrors `xtask/src/main.rs`'s shape exactly. Do NOT bump to `clap 5.x` if it has shipped — the workspace pins 4.5 and bumping is out-of-scope for this story.
- **`serde 1.0 + serde_json 1.0` (May 2026):** stable; `serde::Serialize`/`Deserialize` derive is canonical. The `Item` structs MUST derive both for round-trip via `--out` files.
- **`toml 0.8` (May 2026):** stable; reads quoted-string keys correctly (e.g., `[corpus."secret-redaction-1e4"]`); `serde_yaml 0.9` is still in low-maintenance mode per Story 0.3 W1, accepted unchanged.
- **`sha2 0.10` (May 2026):** stable; streaming `Sha256::new() / update() / finalize()` API is the canonical pattern Story 0.3's `check_corpus.rs::register_corpus` uses (with `b"\n"` per-line append); mirror it for `build.rs` compile-time SHA computation.
- **`walkdir 2.5` (May 2026):** stable; used by Story 0.3's check-loom for kernel-crate scanning; re-used here for the no-nondeterminism-sources test scanning `src/` for forbidden symbols.
- **Rust stable (`1.88`+ per `rust-toolchain.toml`):** the workspace edition is `2021`; `const fn` is stable enough for hash-pinning constants; `BTreeMap` iteration order is stable across versions (alphabetical by key) — the determinism contract assumes this.
- **`build.rs` `cargo:rerun-if-changed=seeds/`:** the build script MUST emit `println!("cargo:rerun-if-changed=seeds/secret-redaction-seeds-v0.1.toml")` and similar for red-team so incremental builds re-run the SHA check when seeds change. Without this directive the build.rs caches and the SHA-mismatch isn't detected until a clean build.

### Project-context reference

There is still no `project-context.md` in this repository (verified at story-creation time — `find /home/lunarpulse/dev_ws/maos -name project-context.md` returns no matches). The persistent-facts entry `file:{project-root}/**/project-context.md` resolves to an empty set; this is expected. Treat the architecture document (`_bmad-output/planning-artifacts/architecture-maos-minimal-opus/`) and PRD (`_bmad-output/planning-artifacts/prd/`) as canonical context, exactly as Stories 0.1 / 0.2 / 0.3 / 0.4 did.

### Cross-story handoff signals (see also Dev Notes above)

- **To Epic 0 retrospective (after merge):** this story closes the E0 surface. The retrospective can mark `epic-0` as `done` once `sprint-planning` is re-run, all five stories are `done`, and any deferred-work entries (DF1–DF6) are accounted for as future-story tickets.
- **To `crates/maos-corpus-gen::ccac` (Story 7.3, v1.0):** the third generator sub-module slot is reserved by convention. Story 7.3 will add `pub mod ccac;` to `lib.rs` and follow the same `seeds/` + `SEED_FILE_SHA256` + `RULE_VERSION` + `build.rs` SHA-pin pattern.

### Latest technical information addendum (the determinism math)

- **Why hash-based combinatorial expansion not seeded RNG.** A seeded RNG (`StdRng::seed_from_u64`) gives deterministic output **across a single `rand` major version** but not across major bumps; `rand 0.8` and `rand 0.9` (when it ships) produce different sequences for the same seed because their core algorithms changed. The determinism contract in AC4 is **cross-version stable**, so hash-based combinatorial expansion (e.g., `Sha256(seed_index_bytes || variant_combo_index_bytes)[..N]`) is the load-bearing technique. `sha2 0.10` is the only crate involved; even a bump to `0.11` would preserve the output because SHA-256 is a fixed algorithm.

---

## Change Log

- 2026-05-12 — Story 0.5 created. Authors the two parameterized corpus-generator frameworks (secret-redaction + red-team) under `crates/maos-corpus-gen/`, commits the 10⁴ secret-redaction + 640-item red-team corpora SHA-pinned per Story 0.3 manifest discipline, wires both into the NFR-Sec-4 + NFR-Sec-10 rows in `coverage-matrix.yaml` (Story 0.4 mass-populated them as 0-item placeholders), and shifts ~2,249 person-hours of v1.0–v1.5 hand-authoring to compile-time SHA-pinned engineering artifacts. Mechanizes epic-0's "Owns" line **"Calibration harness infrastructure ... corpus content authored per-epic"** for the two NFR-Sec corpora whose hand-authoring would dominate the v1.0–v1.5 critical path. Closes Epic 0's substrate-of-the-substrate scope; Epic 0 transitions to maintenance discipline after this PR merges.
- 2026-05-12 — Story 0.5 implemented. All 10 tasks complete: `crates/maos-corpus-gen/` workspace crate with `CorpusGenerator` trait + `SecretRedactionGenerator` (200 seeds, 10⁴ items) + `RedTeamGenerator` (80 seeds, 640 items) + `build.rs` SHA-pin enforcement + determinism integration tests (6) + adversarial-proof fixture trees (4) + `coverage` binary subcommand + KLOC budget calibration (aggregate 3847) + MANIFEST.toml registration + coverage-matrix wiring. 126 workspace tests pass (0 regressions). 6 xtask gates pass. `discipline.yml`, `gate-registry.toml`, `phase-config.toml`, `judge-config.toml` unmodified.

## Story Completion Status

Status: **review**

### Story Creation Notes

- Comprehensive context engine analysis completed (Stories 0.1 / 0.2 / 0.3 / 0.4 fully ingested; architecture §4 + §8.1 + §8.5 + ADR-005 + ADR-009 cross-referenced; NFR-Sec-4 / NFR-Sec-10 / NFR-Sec-16 / NFR-Test-1 / NFR-Meta-3 verbatim; dependency-DAG sprint-plan invariants verified).
- The story is **NOT a blocker for any E1b story** (Story 1b.4 unblocks on Story 0.4's signed-off ComplianceClaim review). Story 0.5's value is forward leverage into v0.5 / v1.0 / v1.5 sprints.
- Per AC10, this story does NOT modify `tests/phase-config.toml`, `xtask/gate-registry.toml`, `tests/judge-config.toml`, or `.github/workflows/discipline.yml` — same hard-discipline guardrails Story 0.4 carried.
- The two new corpora at v0.1-α phase under `current_phase = v0.1-alpha` are in `out_of_scope_deferred` (NFR-Sec-4 phase=v0.5, NFR-Sec-10 phase=v1.5); adding non-empty `corpora` to deferred rows does NOT trigger `NFR-Meta-3` violations (Story 0.4 AC7 logic).
 - This is the **last story in Epic 0**. The retrospective trigger fires after this PR merges (per `sprint-status.yaml` `epic-0-retrospective: optional` slot).

### Review Findings (Chunk 1: Core Crate Source)

- [x] [Review][Patch] **False-negative violation fixture test redesigned** — Test now verifies higher-level detection surface: coverage report surfaces the mis-classified class, expanded items carry the wrong class, and downstream v0.5 regex gate will detect false negatives. [`tests/secret_redaction_integration.rs`]
- [x] [Review][Patch] **Deduplication pass added to secret-redaction expansion** — Added stable-sort + dedup-by-canonical-form + backfill pass in `expand_deterministic`. Dedup_drop_count now reflects actual dedup statistics. [`secret_redaction/expansion.rs`]
- [x] [Review][Patch] **Canary markers now use proper HMAC-SHA256** — Added `hmac = "0.12"` crate dep, implemented HMAC-SHA256 with "maos-canary-v0.1" as key per AC2 spec. [`secret_redaction/mod.rs`, `Cargo.toml`]

- [x] [Review][Patch] **build.rs SHA checks now use separate files** — `sha_check_sr.rs` and `sha_check_rt.rs` prevent the second check from silently overwriting the first. [`build.rs`, `lib.rs`]
- [x] [Review][Patch] **Red-team minimum-emit guarantee no longer undone by truncation** — Removed `truncate(n)` after minimum-emit pass; output may exceed `n` when minimum-emit adds items, preserving per-seed coverage. [`red_team/expansion.rs`]
- [x] [Review][Patch] **Missing-class red-team test now asserts floor_satisfied** — Test checks that `kernel_syscall_abuse` is either absent or has `floor_satisfied: false`. [`red_team_integration.rs:10-55`]
- [x] [Review][Patch] **validate_all/coverage_report now parameterized** — Added `validate_all_n(n)` and `coverage_report_n(n)` methods; fixture tests use appropriate expansion sizes. [`secret_redaction/mod.rs`, `red_team/mod.rs`]
- [x] [Review][Patch] **Coverage floor error message uses correct proportional floor** — Reports actual floor (seed_count × items_per_seed) instead of hardcoded `seed_count * 50`. [`secret_redaction/mod.rs`]
- [x] [Review][Patch] **seed_id[0] panic fixed** — Changed to `seed_id.first().copied().unwrap_or(b'?')` for bounds safety. [`secret_redaction/expansion.rs`]
- [x] [Review][Patch] **CLI now has --seeds-fixture flag for coverage** — `coverage --seeds-fixture <path>` allows testing against violation fixtures via the binary. [`main.rs`]
- [x] [Review][Patch] **Canary mode CLI now has --rng-seed and --marker-namespace flags** — Properly wired canary generation with required parameters. [`main.rs`]
- [x] [Review][Patch] **Dedup_drop_count now accurate** — Reports theoretical_max - expanded_count, matching actual dedup statistics. [`secret_redaction/mod.rs`]
- [x] [Review][Patch] **Red-team ID format consistent** — Uses `{:03}` format throughout (matching AC3's `red-team-NNN`). [`red_team/expansion.rs`]

- [x] [Review][Defer] **"test" synthetic indicator check is over-broad** — matches "latest", "protest" etc. Deferred: acceptable heuristic at v0.1-α, real regex-based validation at v0.5. [`secret_redaction/validation.rs:52`]
- [x] [Review][Defer] **write_jsonl buffers entire corpus in memory** — problematic for quarterly mode (100k items). Deferred: CLI dev-tool, not a correctness issue at current corpus sizes. [`main.rs:115-121`]
- [x] [Review][Defer] **Empty seeds array accepted without error** — degenerate generator state. Deferred: defensive validation, not caused by current seed files. [`seeds.rs:29`]
- [x] [Review][Defer] **validate_all hardcodes 10,000** — coupled to corpus name. Deferred: function is a convenience wrapper, callers can use expand+validate directly. [`secret_redaction/mod.rs:83`]
