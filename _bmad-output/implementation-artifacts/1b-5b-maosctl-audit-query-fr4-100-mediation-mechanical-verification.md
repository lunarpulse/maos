# Story 1b.5b: `maosctl audit query` + FR4 100%-Mediation Mechanical Verification

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As an evaluator who has just watched hello-Spirit respond,
I want `maosctl audit query --spirit hello-spirit` to enumerate every external call the Spirit made with its issuing capability token, Spirit-PID, and boot-nonce, AND a 1000-entry fixture proving 100 % mediation,
So that FR4 ("every external call mediated; verification floor 100 % in any 1000-call sample") is mechanically verified — not asserted in a README.

## Acceptance Criteria

### AC1 — `maosctl audit query --spirit <name>` emits FR4 NDJSON with five mandatory fields

**Given** `maosctl run hello-spirit` has executed at least once on this Host (seeding the on-disk Transparency Log at `$XDG_DATA_HOME/maos/audit/transparency.sqlite`, also reachable via `~/.local/share/maos/audit/transparency.sqlite` on the canonical Linux evaluator path)
**When** `maosctl audit query --spirit hello-spirit` runs
**Then** stdout is NDJSON, one JSON object per line, where each object contains **exactly** the keys `call_id`, `capability_token`, `spirit_pid`, `boot_nonce`, `call_type`, `timestamp_ns`
**And** every entry has all five **non-null** mandatory fields (`capability_token`, `spirit_pid`, `boot_nonce`, `call_type`, `timestamp_ns`); a missing or null mandatory field fails the command with exit code 2
**And** at least one entry is present in stdout (the inference call recorded by Story 1b.5a's one-shot path)
**And** the schema is enforced by `crates/maos-audit/tests/query_schema_test.rs` against a hermetic in-memory SQLite seed

### AC2 — FR4 1000-entry fixture test passes 1000/1000

**Given** the FR4 verification fixture at `crates/maos-audit/tests/fixtures/hello-spirit-1k.jsonl` (1000 pre-recorded NDJSON entries, produced by the checked-in generator)
**When** `cargo test -p maos-audit -- test_fr4_full_mediation` runs
**Then** all 1000 entries carry non-null `capability_token`, `spirit_pid`, and `boot_nonce`
**And** the test fails fast on the first missing-or-null mandatory field, naming the offending line number (no silent pass on partial coverage)
**And** the generator (`scripts/gen_hello_spirit_fixture.sh`) is checked in, executable, deterministic given a fixed seed, and reproduces the byte-identical fixture on `ubuntu-latest`

### AC3 — Accessibility cascade: `--format plain` / `NO_COLOR=1` / `TERM=dumb` produce zero ANSI bytes

**Given** `maosctl audit query --spirit hello-spirit --format plain`
**When** the command runs with `TERM=dumb` *or* `NO_COLOR=1` set
**Then** stdout contains zero `0x1b` (ESC) bytes
**And** `--format ndjson` (default) and `--format plain` both honor the cascade
**And** the assertion is wired in `crates/maos-bin/tests/audit_no_color_test.rs` (or, per the topology note below, `crates/maos-cli/tests/audit_no_color_test.rs`) by checking `stdout.iter().filter(|b| **b == 0x1b).count() == 0`

### AC4 — Canonical FR4 verification path documented; cap-audit ↔ Transparency Log join exercised end-to-end

**Given** Story 1b.1's `TransparencyLogAdapter` (kernel-managed SQLite, append-only, log-before-deliver) and Story 1b.2's `cap-audit` writer task (single-writer, bounded-mpsc, drains capability events to the same SQLite)
**When** `maosctl audit query --spirit hello-spirit` joins capability-token data from `cap-audit` with frame data from the Transparency Log
**Then** every call surfaces its issuing `capability_token` + `spirit_pid` + `boot_nonce`
**And** `crates/maos-audit/README.md` documents the canonical FR4 verification path: clone → `maosctl install` → `maosctl run hello-spirit` → `maosctl audit query --spirit hello-spirit` → mechanically observe ≥1 mediated entry
**And** an integration smoke test (`tests/integration/audit_query_fr4_smoke.sh`) runs the full path end-to-end on `ubuntu-latest` and is wired into CI

## What this story is NOT

- **NOT a generalized subject-access / posture-delta / sealed-export query surface.** Those are Story 9.1 (Epic 9). 1b.5b ships the `--spirit <name>` filter only, scoped to the hello-Spirit evaluator path. Do NOT introduce subject-access plumbing, redaction-policy projection, or Ed25519 export-sealing here.
- **NOT a Spirit-name → PID resolution service.** At v0.1-β only `hello-spirit` exists, mapped to `spirit_pid = 0` per Story 1b.5a's one-shot path. Hardcode `match name { "hello-spirit" => 0, _ => exit(2) }` in `maos-cli/src/subcommands.rs`. The real Spirit registry / scheduler lookup is Epic 5.
- **NOT a new audit crate or new audit module under `maos-bin/src/cmd/`.** The epic AC text mentions `crates/maos-bin/src/cmd/audit.rs`, but `maos-bin/src/` ships exactly one file (`main.rs`) and the dispatcher lives in `maos-cli/src/subcommands.rs` — see Decision Register D1 for the topology reconciliation. Extend `maos-cli/src/subcommands.rs::audit_query` and the existing `maos-audit` crate; do NOT create `maos-bin/src/cmd/`.
- **NOT a kernel-API surface change.** `maos-cli` MUST NOT add a dependency on `maos-kernel-core` (the `[dependencies]` block in `crates/maos-cli/Cargo.toml` is `clap`, `maos-audit`, `serde_json` — load-bearing per Story 1a.4 + `maos-audit/src/lib.rs` doc). Route all read-side surface through `maos-audit`.
- **NOT a live-network FR4 check.** AC2 is satisfied by the checked-in JSONL fixture + read-side test; the 1000-call mediation across live providers is NFR-Test territory in later epics. The hello-Spirit mock path is sufficient.
- **NOT a re-implementation of the existing `fr4_1000_call_fixture` kernel test.** `crates/maos-kernel-core/tests/fr4_1000_call_fixture.rs` already exists and exercises the `cap-audit` channel directly (kernel-side); this story adds the **read-side** verification via the `maos-audit` crate and the on-disk SQLite path. The two are complementary, not duplicative.

## Critical Preconditions (verify BEFORE opening the PR)

1. **Story 1b.5a fully landed and working tree clean.** `git status` shows no uncommitted modifications. Run, all green: `cargo build --workspace --locked`, `cargo test --workspace --locked` (the pre-existing failure `inference::tests::mock_provider_round_trip_logs_inference_call` documented in 1b.5a still exists; do NOT try to fix it here — it's out of scope, but record its baseline state in the Dev Agent Record), `cargo run -p xtask -- check-service-boundary`, `cargo run -p xtask -- abi-diff --base abi-baseline/v1-pre-bump.txt`, `cargo run -p xtask -- check-unsafe`, `cargo deny check`. Record the green baseline in the Dev Agent Record.

2. **Confirm Story 1b.5a one-shot uses in-memory SQLite — this is the CRITICAL gap this story unblocks.** Inspect `crates/maos-bin/src/main.rs` line ~117: `TransparencyLogAdapter::open_in_memory(boot_nonce)`. After the one-shot process exits, the audit data is gone — `maosctl audit query` cannot read it back. **This story changes the one-shot path to open the on-disk SQLite at the XDG-resolved path (`default_transparency_log_path()` already exists in `maos-cli/src/subcommands.rs:148-162`; mirror that resolution in `maos-bin/main.rs`).** Server-mode (non-one-shot) is out of scope for this story; only the one-shot path needs the on-disk change. Capture this in the Dev Agent Record as Decision D2.

3. **Cap-audit channel drain is not currently awaited on one-shot exit — confirm the timing.** `CapAuditWriter::spawn(audit_rx, transparency_log)` is wired at `maos-bin/main.rs:120`. The writer is `tokio::spawn`-ed and pulls events asynchronously. When the one-shot path hits `return Ok(())` at line 201, in-flight `cap-audit` events may not yet have been written to SQLite. Without an explicit drain, the inference call's audit row will be lost intermittently. **Solution shape: before `return Ok(())`, `drop(audit_tx)` (sentinel-close the sender side) then `_audit_writer.await?`** so the writer drains the channel and exits cleanly. Validate this in the integration test (AC4) by asserting the inference.call row is reliably present.

4. **`maos-cli` dep-direction rule is binding.** `crates/maos-cli/Cargo.toml` must NOT gain `maos-kernel-core` as a dependency. Any new surface needed for FR4 read-side query goes into `maos-audit` (which depends on `maos-domain` only). Verify via `cargo tree -p maos-cli | grep maos-kernel-core` → must return empty.

5. **`check-fr47` and `xtask check-empty-kernel` must remain green after the change.** The one-shot path's SQLite-on-disk change writes through the already-sanctioned `TransparencyLogAdapter` (I9 exemption holder), so no new persistent state escapes the kernel. Confirm by running both gates after the change.

## Size Envelope

- **AC1 (CLI surface + schema projection):** ~150–250 LOC across `crates/maos-cli/src/cli.rs` (add `--spirit`, `--format`), `crates/maos-cli/src/subcommands.rs` (extend `audit_query`), `crates/maos-audit/src/lib.rs` (add `Fr4Entry` projection + `to_fr4_ndjson` writer).
- **AC2 (fixture + test + generator):** ~200–350 LOC: `scripts/gen_hello_spirit_fixture.sh` (~50 LOC), generator binary `crates/maos-audit/src/bin/gen_fixture.rs` (~100 LOC), `crates/maos-audit/tests/query_schema_test.rs` + `crates/maos-audit/tests/fr4_full_mediation_test.rs` (~150 LOC combined), fixture file `crates/maos-audit/tests/fixtures/hello-spirit-1k.jsonl` (~150 KiB; checked in).
- **AC3 (--format plain + ANSI test):** ~80–150 LOC (plain formatter ~50 LOC; ANSI test ~50 LOC).
- **AC4 (README + integration smoke + maos-bin on-disk change + audit-drain):** ~100–200 LOC (`crates/maos-audit/README.md` ~100 LOC prose; `tests/integration/audit_query_fr4_smoke.sh` ~50 LOC; `maos-bin/main.rs` ~30 LOC changes).
- **CI wiring:** ~30 LOC across `.github/workflows/discipline.yml` (new job `audit-query-fr4-smoke`).
- **Total:** ~600–1.0 KLOC implementation + ~0.3 KLOC tests/fixtures/scripts.
- **New external dependencies:** 0. `rusqlite`, `serde_json`, `thiserror`, `tempfile` are already in `maos-audit`. The generator binary uses only existing workspace deps.

## Tasks / Subtasks

- [x] **Task 0 — Pre-flight & Decision Register**
  - [x] Verify Critical Preconditions 1–5; record the green baseline in the Dev Agent Record.
  - [x] Lock the five Decision Register entries below (D1–D5); deviations need an explicit Dev Agent Record entry.
  - [x] Confirm `cargo tree -p maos-cli` does NOT show `maos-kernel-core`.

- [x] **Task 1 — `--spirit` and `--format` CLI flags (AC1, AC3)**
  - [x] Extend `crates/maos-cli/src/cli.rs::AuditQuery::Query` from a unit variant to a struct variant with `spirit: Option<String>` and `format: AuditFormat` (default `Ndjson`).
  - [x] Add `#[derive(clap::ValueEnum)] pub enum AuditFormat { Ndjson, Plain }` in `cli.rs`.
  - [x] Update doc-comments to remove the "Story 1b.5b" forward-references (now landing here).

- [x] **Task 2 — `maos-audit` FR4 projection + writers (AC1, AC3)**
  - [x] Add `pub struct Fr4Entry { call_id, capability_token, spirit_pid, boot_nonce, call_type, timestamp_ns }` to `crates/maos-audit/src/lib.rs` (serde `Serialize` + `Deserialize`).
  - [x] Implement `pub fn project_to_fr4(entry: &AuditEntry) -> Result<Fr4Entry, Fr4SchemaError>` — returns `Err(Fr4SchemaError::MissingCapabilityToken)` when `capability_token_hex` is `None`.
  - [x] Implement `pub fn to_fr4_ndjson<W>(entries, out) -> Result<(), AuditError>` — projects each entry and writes one JSON object per line. Stop and return `Err(AuditError::Fr4SchemaViolation { line, missing_field })` on the first missing mandatory field.
  - [x] Implement `pub fn to_plain<W>(entries, out) -> Result<(), AuditError>` — human-readable tabular text, zero ANSI bytes (no `colored` crate, no escape sequences).
  - [x] Extend `AuditError` with the `Fr4SchemaViolation { line: usize, missing_field: &'static str }` variant.

- [x] **Task 3 — Wire `audit query` dispatch (AC1, AC3, AC4)**
  - [x] In `crates/maos-cli/src/subcommands.rs::audit_dispatch` and `audit_query`, accept the new `spirit` and `format` args.
  - [x] Resolve `--spirit hello-spirit` → `spirit_pid = 0` (hardcoded match; reject any other name with exit 2 and a clear diagnostic — message reads "unknown spirit, only 'hello-spirit' is available at v0.1-β").
  - [x] Pass `AuditFilter { spirit_pid: Some(0), .. }` to `maos_audit::query`.
  - [x] Branch on `format`: `Ndjson` (with `--spirit`) → `to_fr4_ndjson`; `Ndjson` (bare) → legacy `to_ndjson` (backward compat with `audit-spine-smoke`); `Plain` → `to_plain`. Both honor `NO_COLOR`/`TERM=dumb`/`--plain` via the existing `ColorChoice` cascade.
  - [x] When `to_fr4_ndjson` returns `Err(AuditError::Fr4SchemaViolation { .. })`, exit with code 2 and a diagnostic naming the missing field and line; do NOT print partial output.
  - [x] When the audit DB is missing (`AuditError::Open(_)`), reuse the existing diagnostic ("Run `maosctl run hello-spirit` first to seed the log") and exit 2.

- [x] **Task 4 — Change `maos-bin` one-shot to on-disk SQLite + audit-drain (AC1, AC4, Precondition 2+3)**
  - [x] In `crates/maos-bin/src/main.rs` add a helper `fn default_transparency_log_path() -> std::path::PathBuf` mirroring the resolution in `maos-cli/src/subcommands.rs:148-162` (`MAOS_AUDIT_DB` → `$XDG_DATA_HOME` → `$HOME/.local/share` → `/var/lib`), with the same `maos/audit/transparency.sqlite` suffix.
  - [x] Replace `TransparencyLogAdapter::open_in_memory(boot_nonce)` at line ~117 with on-disk `open()`. Create the parent directory with `std::fs::create_dir_all` first; fail loudly with exit code 2 and a clear diagnostic if the directory cannot be created (do not silently fall back to in-memory).
  - [x] **Single change covers both server mode and one-shot.**
  - [x] Before the one-shot `return Ok(())`, replaced `let _audit_writer = …spawn(...);` with `let audit_writer = …spawn(...);` and added the drain sequence `drop(audit_tx); drop(inference); drop(capability); audit_writer.await.ok();`.
  - [x] **20× drain smoke locally → 20/20 success** (`drain smoke: 20/20 success, 0/20 failure`). The inference.call row is reliably present.

- [x] **Task 5 — Schema test for AC1 in `maos-audit` (AC1)**
  - [x] Created `crates/maos-audit/tests/query_schema_test.rs` (4 tests).
  - [x] Seeds 3 rows (kind=9 InferenceCall, kind=7 CapabilityInvocation, kind=9 with NULL token).
  - [x] Asserts FR4 NDJSON projection emits the six-key schema; asserts the null-token row triggers `AuditError::Fr4SchemaViolation { line: 3, missing_field: "capability_token" }`.
  - [x] Documents the exit-code 2 contract (also exercised in `audit_no_color_test::fr4_schema_violation_exits_two_with_diagnostic`).

- [x] **Task 6 — FR4 1000-entry fixture + generator + test (AC2)**
  - [x] Per D5, took the **synthetic** path: `crates/maos-audit/src/bin/gen_fixture.rs` is a self-contained synthesizer with **zero kernel-core dependency**. Uses a hand-rolled 64-bit LCG (Numerical Recipes constants) instead of `rand_chacha` so `Cargo.lock` is byte-stable.
  - [x] Added `[[bin]] name = "gen_fixture"` to `crates/maos-audit/Cargo.toml`.
  - [x] Created `scripts/gen_hello_spirit_fixture.sh` (executable). Seed: `0x5BF01A5B5BF01A5B` (documented inline).
  - [x] Generated fixture: `crates/maos-audit/tests/fixtures/hello-spirit-1k.jsonl` (1000 entries, 249,500 bytes).
  - [x] `crates/maos-audit/tests/fr4_full_mediation_test.rs::test_fr4_full_mediation` — parses every line as `Fr4Entry`, asserts capability_token len == 64, spirit_pid ∈ 1..=5 (non-zero), boot_nonce non-zero, call_type ∈ {`inference.call`, `capability.invocation`}, timestamp_ns non-zero. Aborts on first violation with line number.
  - [x] Asserts total count == 1000.
  - [x] `fixture_is_byte_deterministic` re-runs the generator and asserts byte-equality with the checked-in fixture.

- [x] **Task 7 — Accessibility test for AC3 (AC3)**
  - [x] Created `crates/maos-cli/tests/audit_no_color_test.rs` (6 tests) per D1 (in `maos-cli/tests/`, NOT `maos-bin/tests/`).
  - [x] 4 invocations × 0 `0x1b` bytes covered (`TERM=dumb`/`NO_COLOR=1` × `ndjson`/`plain`).
  - [x] All four assert exit-success. Two additional tests: FR4 schema violation → exit 2 with diagnostic; unknown spirit → exit 2 with v0.1-β diagnostic.
  - [x] Added `rusqlite` + `tempfile` as `maos-cli` `[dev-dependencies]` (already in workspace; Cargo.lock unaffected).

- [x] **Task 8 — Integration smoke for AC4 (AC4)**
  - [x] Created `tests/integration/audit_query_fr4_smoke.sh` (executable). Runs `MAOS_ONE_SHOT=hello-spirit maos-bin` then `maosctl audit query --spirit hello-spirit --format ndjson | head -1 | jq -e '...'`. PASS in 192ms–1.1s locally on warm builds; 30s on cold release builds.
  - [x] Wired into `.github/workflows/discipline.yml` as job `audit-query-fr4-smoke` (right after `fr4-1000-call-fixture`, 2-space YAML indent). Added to `aggregate.needs`, GITHUB_OUTPUT block (`aqfs=`), JS `const aqfs`, and the comment table.
  - [x] YAML well-formedness validated (`python3 -c "import yaml; yaml.safe_load(...)"`).

- [x] **Task 9 — `maos-audit/README.md` (AC4)**
  - [x] Created `crates/maos-audit/README.md` (~210 lines). Documents the canonical FR4 verification path (clone → install → run → audit query), the six-key FR4 schema table, the cap-audit ↔ Transparency Log join (with ASCII diagram), the dep-direction rule, scope boundaries, and the testing recipe.
  - [x] Cross-linked to architecture §8.4, PRD FR4 line 27, NFR-Obs-4, NFR-Ops-5, and Stories 1b.1 / 1b.2 / 1b.5a / Story 9.1.

- [x] **Task 10 — Full-gate verification + Dev Agent Record (AC1–AC4)**
  - [x] `cargo build --workspace --locked` — PASS.
  - [x] `cargo test --workspace --locked` — PASS modulo the pre-existing `inference::tests::mock_provider_round_trip_logs_inference_call` failure (identical to baseline 7dfbdc5; out of scope).
  - [x] `cargo run -p xtask -- check-service-boundary` — same 2 pre-existing violations as baseline (InferencePortAdapter removal + TokenIssuer classification); no new violations.
  - [x] `cargo run -p xtask -- abi-diff --base abi-baseline/v1-pre-bump.txt` — PASS.
  - [x] `cargo run -p xtask -- check-unsafe` — PASS (0 violations).
  - [x] `cargo run -p xtask -- check-empty-kernel` — same 13 pre-existing violations as baseline (Story 1b.4 telemetry / inference / security); no new persistent-state holders introduced.
  - [x] `cargo run -p xtask -- check-fr47` — PASS (0 violations).
  - [x] `cargo test -p maos-audit` — 10 lib + 4 schema + 2 fixture = **16/16 PASS**.
  - [x] `cargo test -p maos-cli` — 17 lib + 6 ANSI/dispatch = **23/23 PASS**.
  - [x] `bash tests/integration/audit_query_fr4_smoke.sh` locally → PASS (1.1s).
  - [x] `bash tests/integration/audit_spine_smoke.sh` locally → PASS (no regression; bare `audit query` keeps raw schema).
  - [x] `bash tests/integration/cap_registry_smoke.sh` locally → PASS.
  - [x] Audit-drain 20× smoke locally → **20/20 success**.
  - [x] `cargo tree -p maos-cli | grep maos-kernel-core` → **empty** (dep-direction rule preserved).
  - [x] `Cargo.lock` blast: **0 new packages**. The only diff is two lines adding `rusqlite` + `tempfile` references to the `maos-cli` entry (both already in workspace via `maos-audit`).
  - [x] Filled Completion Notes, File List, and Evidence Blocks below.

## Dev Notes

### Decision Register

| # | Decision | Recommended | Rationale | If overridden |
|---|---|---|---|---|
| 1 | Topology: where does the dispatcher live? | **Extend `maos-cli/src/subcommands.rs`; do NOT create `maos-bin/src/cmd/audit.rs`** | `maos-bin/src/` contains only `main.rs`; dispatcher already lives in `maos-cli`; epic AC text predates the 1a.4 split. The dep-direction rule (`maos-cli` ⊥ `maos-kernel-core`) makes `maos-bin` the wrong place for read-side audit code. | Creating `maos-bin/src/cmd/audit.rs` requires adding clap to maos-bin (currently no clap dep), duplicating the existing audit_query body, and re-routing maosctl → maos-bin via subprocess — strictly worse. |
| 2 | One-shot SQLite storage | **On-disk at XDG-resolved path, same as server mode** | AC1 requires `maosctl audit query` to read back what `maosctl run hello-spirit` wrote. In-memory SQLite vanishes on process exit. | Keep in-memory + invent a new "session.jsonl" surface — doubles the surface area, contradicts architecture §8.4 (SQLite is the audit spine). |
| 3 | Cap-audit drain sequence on one-shot exit | **`drop(audit_tx); drop(inference); drop(capability); audit_writer.await.ok();`** | The capability adapter holds the surviving `audit_tx` clone; without dropping it the writer never sees channel-close. Awaiting the writer guarantees the inference.call row reaches SQLite before exit. | Sleep-based grace period (e.g. `sleep(50ms)`) is flaky on loaded CI runners; drain-with-await is deterministic. |
| 4 | Spirit name → PID resolution at v0.1-β | **Hardcoded `match name { "hello-spirit" => 0, _ => exit(2) }`** | Only one Spirit exists; Spirit registry is Epic 5. Anything else is over-engineering. | Read manifest from `spirits/hello-spirit/manifest.toml` — adds toml dep to maos-cli, no benefit at v0.1-β. |
| 5 | FR4 1000-entry fixture generation strategy | **Synthetic generator (no kernel-core dep)** — assemble `Fr4Entry` structs deterministically with a hand-rolled LCG (no `rand_chacha`) and write JSON | Keeps `maos-audit` kernel-free; reproducible; tiny LOC. The kernel-side `fr4_1000_call_fixture.rs` already covers the in-kernel path. | Real kernel pump in a `[[bin]]` with kernel-core dev-dep — works but adds maos-kernel-core to maos-audit's dev-build graph; preserves it as library-clean but bloats the dev surface. |

### Architecture Compliance

- **ADR-010 hexagonal:** `maos-audit` depends only on `maos-domain` (none today; lib has zero `maos-` deps in its tree). The Fr4Entry projection is pure data; no port traits crossed. `maos-cli` calls into `maos-audit` only — no kernel-core dep growth.
- **§8.4 Audit (architecture-maos-minimal-opus/8-security-approval-model.md):** "Transparency Log is the personal audit trail … queryable via a control-plane API." This story implements that surface end-to-end at v0.1-β for the `--spirit` filter.
- **I2 log-before-deliver:** Unchanged. The on-disk switch in `maos-bin/main.rs` preserves the I2 guarantee — `TransparencyLogAdapter::insert_frame_event` already implements log-before-deliver per Story 1b.1.
- **I9 empty-kernel:** No new persistent state. The existing I9 exemption on `TransparencyLogAdapter` (`xtask/i9-whitelist.toml`) already covers SQLite-on-disk; verify with `xtask check-empty-kernel` post-change.
- **NFR-Ops-5 accessibility:** Both `--format ndjson` and `--format plain` produce zero ANSI bytes when `NO_COLOR=1` / `TERM=dumb` / `--plain` is set. The dispatcher already honors `ColorChoice` via `crate::accessibility::ColorChoice`.
- **NFR-Obs-4 (Transparency Log SQLite append-only with JSONL export):** This story is the v0.1-β implementation of NFR-Obs-4's JSONL-export half. Full audit surface (subject-access, posture-delta, sealed-export) is Story 9.1.
- **FR47 enforcement:** No vendor SDK touch. `check-fr47` stays green.
- **`#![forbid(unsafe_code)]`:** Mandatory on every new `.rs` file (`Fr4Entry` module, schema test, fixture test, ANSI test, generator binary).
- **Dep-direction rule (Story 1a.4 + maos-audit lib doc):** `maos-cli` MUST NOT depend on `maos-kernel-core`. Audited via `cargo tree`; this story preserves the rule.

### Project Structure Notes

**NEW (paths confirmed against repo state):**
- `crates/maos-audit/README.md`
- `crates/maos-audit/src/bin/gen_fixture.rs` (only if D5 alternate-kernel path is taken; recommended D5 keeps fixture generation as a script-level synthesizer — in that case create `crates/maos-audit/src/bin/gen_fixture.rs` as a self-contained synthesizer with zero kernel deps)
- `crates/maos-audit/tests/query_schema_test.rs`
- `crates/maos-audit/tests/fr4_full_mediation_test.rs`
- `crates/maos-audit/tests/fixtures/hello-spirit-1k.jsonl` (checked-in artifact, ~150 KiB)
- `crates/maos-cli/tests/audit_no_color_test.rs`
- `scripts/gen_hello_spirit_fixture.sh`
- `tests/integration/audit_query_fr4_smoke.sh`

**UPDATE (read completely before editing — Story 1b.5a precedent for not breaking working code):**
- `crates/maos-cli/src/cli.rs` — extend `AuditQuery::Query` from unit variant to struct variant; add `AuditFormat` enum. Other subcommand definitions untouched.
- `crates/maos-cli/src/subcommands.rs` — `audit_dispatch` and `audit_query` accept `spirit` + `format`; the existing `default_transparency_log_path()`, NDJSON output, and "DB missing" diagnostic are reused. Add the hardcoded name→PID match. Existing tests untouched.
- `crates/maos-audit/src/lib.rs` — append `Fr4Entry`, `Fr4SchemaError`, `project_to_fr4`, `to_fr4_ndjson`, `to_plain`; extend `AuditError` with `Fr4SchemaViolation`. Existing `AuditEntry`, `AuditFilter`, `query`, `to_ndjson` (the v0.1-β raw export) MUST stay backward-compatible — Story 9.1 will reuse them.
- `crates/maos-audit/Cargo.toml` — no new dependencies expected. If the generator uses `rand_chacha`, add to `[dev-dependencies]` only.
- `crates/maos-bin/src/main.rs` — replace `open_in_memory` with on-disk `open()` (one line at ~117); add `default_transparency_log_path()` helper (~10 LOC); reorder one-shot exit sequence to `drop(audit_tx); drop(inference); drop(capability); audit_writer.await.ok(); return Ok(());` (~5 LOC change near line 200). Preserve all surrounding adapter init and eprintln tracing.
- `.github/workflows/discipline.yml` — add `audit-query-fr4-smoke` job; add to `aggregate.needs`, the GITHUB_OUTPUT echo block, the JS const block, and the comment table. Match 2-space indentation.

### Schema Mapping — Existing `AuditEntry` → FR4 `Fr4Entry`

Critical for the dev agent: the AC1 schema and the existing `AuditEntry` schema are NOT identical; the projection is the load-bearing piece.

| `Fr4Entry` field (AC1)        | Source on `AuditEntry`           | Null/error handling                                     |
| ----------------------------- | -------------------------------- | -------------------------------------------------------- |
| `call_id`                     | `frame_id_hex` (32-char hex)     | Always present (PRIMARY KEY in schema)                  |
| `capability_token`            | `capability_token_hex: Option`   | **`None` → `Fr4SchemaError::MissingCapabilityToken`**   |
| `spirit_pid`                  | `spirit_pid: u32`                | Always present                                          |
| `boot_nonce`                  | `boot_nonce: u64`                | Always present                                          |
| `call_type`                   | `kind: String` (e.g. `"capability.invocation"`, `"inference.call"`) | Always present; reject `unknown(N)` strings with `Fr4SchemaError::UnknownCallType` |
| `timestamp_ns`                | `timestamp_ns: u64`              | Always present                                          |

The `intent` field is NOT in the FR4 shape (it lives in the raw `AuditEntry` for Story 9.1's subject-access). The `payload_redacted` blob is never emitted (redaction is at the kernel write boundary; the read side never sees raw bytes).

### Testing Requirements

- **Standards:** `cargo test --workspace --locked` green. Inline unit tests for `project_to_fr4` and `to_fr4_ndjson` in `maos-audit/src/lib.rs`. Integration tests in `crates/maos-audit/tests/` and `crates/maos-cli/tests/`. Shell integration in `tests/integration/`.
- **AC1:** `cargo test -p maos-audit --test query_schema_test` passes. Hermetic in-memory SQLite seed; one `kind=7` row, one `kind=9` row, one `capability_token=NULL` negative row.
- **AC2:** `cargo test -p maos-audit --test fr4_full_mediation_test` passes. Fixture file checked in; determinism sub-test re-runs generator and asserts byte-equality.
- **AC3:** `cargo test -p maos-cli --test audit_no_color_test` passes; 4 invocations × 0 `0x1b` bytes.
- **AC4:** `bash tests/integration/audit_query_fr4_smoke.sh` passes locally and on `ubuntu-latest`. The audit-drain smoke (20×) confirms no flakiness from in-flight cap-audit events.

### Previous Story Intelligence

**From 1b.5a (immediate predecessor) — Review Findings landed and resolved (see `_bmad-output/implementation-artifacts/1b-5a-…md` lines 344-358):**
1. **One-shot mode validation pattern (D8 from 1b.5a review):** unknown `MAOS_ONE_SHOT` values exit non-zero with a clear diagnostic. Replicate this pattern for unknown `--spirit` values in 1b.5b — already specified above.
2. **YAML indentation regression in `discipline.yml` was caught and fixed in 1b.5a.** The `fr4-1000-call-fixture:` line was previously indented with 3 spaces; the fix restored 2-space indent. **Verify the line is still at 2 spaces before adding the new `audit-query-fr4-smoke:` job, and place the new job at the same column.**
3. **Baseline JSON filename/version naming:** Story 1b.5a noted `docs/ci-baselines/kernel-surface-v0.1-beta.json`. This story does not modify the kernel surface (no new public APIs on adapters expected) — no baseline change needed. If a new public method on `TransparencyLogAdapter` or any other adapter slips in, update the baseline as part of this story per the precedent.
4. **`capability_registry()` getter (1b.5a addition):** the `InferencePortAdapter::capability_registry()` public method exists; do NOT add a parallel surface. If the new one-shot exit ordering needs to drop the adapter, drop the `Arc<CapabilityRegistryAdapter>` directly via the `capability` binding, not through the inference adapter.
5. **Exact text discipline:** the "Unconfigured fallback" string in `maos-spirit-hello/src/lib.rs:78-86` is matched verbatim by tests. If the on-disk SQLite path change perturbs the fallback path (it shouldn't — the inference port is unchanged), do not adjust the text.
6. **HelloError implements `std::error::Error`:** any new error types in `maos-audit` (`Fr4SchemaError`, the `Fr4SchemaViolation` variant on `AuditError`) MUST also implement `std::error::Error` via `thiserror`. The crate already pulls `thiserror = "2"`; reuse the derive pattern.

**From 1b.4:**
7. **Dep blast discipline:** target 0 new external transitive deps. `rusqlite`, `serde_json`, `thiserror`, `tempfile` already in `maos-audit`. The generator should use only existing deps — D5's recommended hand-rolled LCG keeps `Cargo.lock` byte-stable.

**From 1b.2 (cap-audit decomposition):**
8. **The cap-audit channel is bounded at 8192 (`AUDIT_CHANNEL_DEPTH`).** Under one-shot's single-call pattern, channel pressure is impossible; in server mode the writer is sized for hot-path emission. Verify with `audit_drop_count()` if a flaky test surfaces — the existing dropped-counter is the diagnostic.
9. **The audit-writer task currently consumes channels but does NOT signal completion.** Spawning returns a `JoinHandle<()>` per `writer_task.rs:27`. Awaiting it after dropping all senders is the documented drain pattern.

**From 1b.1 (audit spine):**
10. **`TransparencyLogAdapter::open(path, boot_nonce)` is the on-disk constructor.** `open_in_memory` is for tests + benchmarks only. The one-shot path's switch to `open()` is the architecturally correct posture; 1b.5a temporarily used in-memory because no read-side existed yet.
11. **Frame kind 7 = CapabilityInvocation, 9 = InferenceCall.** The `to_fr4_ndjson` mapping must emit "capability.invocation" and "inference.call" exactly (the existing `kind_to_string` at `maos-audit/src/lib.rs:158-170` produces this); the `Fr4SchemaError::UnknownCallType` guard catches `unknown(N)` strings.

### Git Intelligence Summary

- `7dfbdc5` Story 1b.5a — hello-Spirit binary + one-shot path; **in-memory SQLite is the gap this story closes**; `MAOS_ONE_SHOT` env-var pattern is the wire.
- `a767bcc` Story 1b.4 — `InferencePortAdapter` records inference calls to Transparency Log via `insert_frame_event(FrameKind::InferenceCall, …)`. This is the row that AC1 surfaces via `maosctl audit query --spirit hello-spirit`.
- `7de1207` Journal fsync improvements — unrelated to this story but live in the working tree; do not perturb.
- `cdf98c8` Story 1b.3 — sandbox tier enforcement; `FrameKind::SandboxBlock = 8` exists for future audit query scenarios (not in v0.1-β scope).
- `f58b356` Story `maos-attrs` crate + `#[i9_exempt]` — not expected to be used here (no new persistent state).
- Working tree at story-creation: **3 untracked code-review markdown files** for Story 1b.5a (`code-review-1b.5a-*.md` × 3) in `_bmad-output/implementation-artifacts/`. These are review prompts (not findings); the resolved findings are documented in the 1b.5a story file. Do not delete; the next session may add the review-findings file.

### Project Context Reference

- **Epic:** `_bmad-output/planning-artifacts/epics/epic-1b-evaluator-path-audit-spine-capability-mediation-baseline-v01.md` (Story 1b.5b lines 197–225).
- **PRD FR4:** `_bmad-output/planning-artifacts/prd/functional-requirements.md` line 27 — "Operator can verify every Spirit's external call … was mediated by kernel-issued capability tokens … verification floor is 100% mediation in any 1000-call sample."
- **PRD NFR-Obs-4 + NFR-Ops-5:** `_bmad-output/planning-artifacts/prd/non-functional-requirements.md` — SQLite append-only with JSONL export; accessibility cascade.
- **Architecture §8.4 Audit:** `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/8-security-approval-model.md` lines 62–66 — "Transparency Log is the personal audit trail … queryable via a control-plane API; both can be exported for compliance."
- **Architecture §4.0.2 (17-crate workspace) + §7.3 (Transparency Log) + §7.4 (Approval Decision Log).**
- **Existing read-side adapter:** `crates/maos-audit/src/lib.rs` (current `AuditEntry` + `query` + `to_ndjson`).
- **Existing CLI dispatcher:** `crates/maos-cli/src/subcommands.rs:112-143` (`audit_dispatch`, `audit_query`, `default_transparency_log_path`).
- **Existing one-shot path:** `crates/maos-bin/src/main.rs:152-202`.
- **Existing kernel-side FR4 fixture (complementary, not duplicative):** `crates/maos-kernel-core/tests/fr4_1000_call_fixture.rs`.
- **Existing cap-audit writer task:** `crates/maos-kernel-core/src/capability/cap_audit/writer_task.rs:36-103` — discriminator → FrameKind mapping for `Issue`/`Verify`/`Revoke`/`Invocation`/`SandboxBlock`.
- **Story 1b.5a (predecessor):** `_bmad-output/implementation-artifacts/1b-5a-ship-hello-spirit-reference-binary-and-hit-nfr-onb-2-5-minute-evaluator-path.md` — Dev Agent Record format, audit-channel handling lessons, review-findings format.

## Dev Agent Record

### Agent Model Used

Claude Opus 4.7 (1M-context); claude-opus-4-7[1m]. Knowledge cutoff 2026-01.

### Debug Log References

**Green-baseline confirmation (2026-05-15, pre-story):**
- `cargo build --workspace --locked` — PASS.
- `cargo test --workspace --locked` — 1 pre-existing failure (`maos_kernel_core::inference::tests::mock_provider_round_trip_logs_inference_call` at `crates/maos-kernel-core/src/inference/mod.rs:248`, `CapabilityDenied` on `.unwrap()`); 81 passed. Verified identical on commit `7dfbdc5` (Story 1b.5a baseline) — out of scope per story spec.
- `cargo run -p xtask -- check-unsafe` — PASS (0 violations).
- `cargo run -p xtask -- check-fr47` — PASS (0 violations).
- `cargo run -p xtask -- abi-diff --base abi-baseline/v1-pre-bump.txt` — PASS.
- `cargo run -p xtask -- check-service-boundary` — 2 pre-existing violations (`InferencePortAdapter` removal flagged + `TokenIssuer` missing classification). Verified identical on `7dfbdc5`; not my regression.
- `cargo run -p xtask -- check-empty-kernel` — 13 pre-existing I9 violations in `maos-kernel-core` (`CounterSeries`, `HistogramSeries`, `IacRtMetrics`, `InferencePortAdapter`, `SandboxSpec`, `SecurityManagerAdapter`). Story 1b.4 telemetry residue; same on baseline. No new violations introduced by this story.
- `cargo tree -p maos-cli | grep maos-kernel-core` — empty (Story 1a.4 dep-direction rule preserved).

**Decision Register lock (D1–D5 as written; no overrides):**
- D1: dispatcher in `maos-cli/src/subcommands.rs`; no `maos-bin/src/cmd/audit.rs`.
- D2: on-disk SQLite at XDG path for both server + one-shot modes.
- D3: cap-audit drain via `drop(audit_tx); drop(inference); drop(capability); audit_writer.await.ok();` (deterministic drain, no sleep-based grace period).
- D4: hardcoded `match name { "hello-spirit" => 0, _ => exit 2 }` for v0.1-β.
- D5: **synthetic** fixture generator (hand-rolled LCG, zero kernel-core dep, zero new transitive deps). The alternative kernel-pump path was rejected to keep `cargo tree -p maos-audit` library-clean.

**One-shot in-memory → on-disk transition:**
- Replaced `TransparencyLogAdapter::open_in_memory(boot_nonce)` at `crates/maos-bin/src/main.rs:117` with `::open(&audit_db_path, boot_nonce)`. Added `default_transparency_log_path()` helper mirroring `maos-cli/src/subcommands.rs:148-162` (MAOS_AUDIT_DB → XDG_DATA_HOME → $HOME/.local/share → /var/lib).
- Single line covers both server + one-shot — no `MAOS_ONE_SHOT` branch on storage choice.
- Parent-dir creation via `std::fs::create_dir_all`; fails loudly with exit 2 on permission errors (does NOT silently fall back to in-memory).

**Cap-audit drain validation (20× smoke):**
```
drain smoke: 20/20 success, 0/20 failure
```
Every iteration of `MAOS_ONE_SHOT=hello-spirit maos-bin` followed by `maosctl audit query --spirit hello-spirit --format ndjson` reliably produced the `inference.call` row plus 2 `capability.invocation` rows. The `drop(audit_tx); drop(inference); drop(capability); audit_writer.await.ok()` sequence is deterministic — the capability adapter's clone of `audit_tx` is released along with `capability`, closing the channel cleanly before the writer drains.

**`cargo tree -p maos-cli` post-change (dep-direction):**
```
$ cargo tree -p maos-cli | grep maos-kernel-core
(empty)
```
Rule preserved.

**Fixture determinism byte-equality re-test:**
- `cargo test -p maos-audit --test fr4_full_mediation_test::fixture_is_byte_deterministic` — PASS.
- The test re-runs the generator binary (via `cargo run -p maos-audit --bin gen_fixture`) into a tempfile and asserts byte-equality with the checked-in fixture. Seed: `0x5BF01A5B5BF01A5B`. The fixture is 249,500 bytes / 1000 lines.

### Completion Notes List

**AC1 — `maosctl audit query --spirit <name>` emits FR4 NDJSON with five mandatory fields**
- CLI surface: `crates/maos-cli/src/cli.rs::AuditQuery::Query { spirit: Option<String>, format: AuditFormat }`; new `AuditFormat::{Ndjson, Plain}` value enum with `Ndjson` default.
- Projection: `crates/maos-audit/src/lib.rs::Fr4Entry { call_id, capability_token, spirit_pid, boot_nonce, call_type, timestamp_ns }`; `project_to_fr4(&AuditEntry) -> Result<Fr4Entry, Fr4SchemaError>` returns `MissingCapabilityToken` on `None` token and `UnknownCallType` on `unknown(N)` kinds.
- Writer: `to_fr4_ndjson` emits one JSON object per line, aborts on first violation with `AuditError::Fr4SchemaViolation { line, missing_field }` (1-indexed line, stable field-name string).
- Dispatcher: `crates/maos-cli/src/subcommands.rs::audit_query` resolves `--spirit hello-spirit` → `spirit_pid = 0`, rejects other names with exit 2 + diagnostic "unknown spirit, only 'hello-spirit' is available at v0.1-β".
- Exit-code mapping: `AuditError::Fr4SchemaViolation` → exit 2 with stderr diagnostic naming the field + line; `AuditError::Open(_)` → exit 2 + "Run `maosctl run hello-spirit` first to seed the log".
- **Schema enforced by `crates/maos-audit/tests/query_schema_test.rs`** (4 tests, all PASS) against a hermetic on-disk SQLite seed with 1× kind=9, 1× kind=7, and 1× NULL-token negative row.
- Live evidence: one-shot run produced `{"call_id":"019e2ab2cba9e9...","capability_token":"240b78fe...","spirit_pid":0,"boot_nonce":2597302644735822443,"call_type":"inference.call","timestamp_ns":1778832821161969014}` — 6 keys, all non-null mandatory fields.

**AC2 — FR4 1000-entry fixture test passes 1000/1000**
- Generator binary: `crates/maos-audit/src/bin/gen_fixture.rs` — synthetic (no kernel-core dep), deterministic via hand-rolled 64-bit LCG (Numerical Recipes constants 6364136223846793005 / 1442695040888963407).
- Script: `scripts/gen_hello_spirit_fixture.sh` (executable, `chmod +x`), seed `0x5BF01A5B5BF01A5B` documented in header.
- Fixture: `crates/maos-audit/tests/fixtures/hello-spirit-1k.jsonl` — 1000 lines, 249,500 bytes, checked in.
- Test: `crates/maos-audit/tests/fr4_full_mediation_test.rs::test_fr4_full_mediation` — parses each line, asserts capability_token.len() == 64 (non-empty), spirit_pid ∈ 1..=5 (non-zero), boot_nonce non-zero, call_type known, timestamp_ns non-zero. Total count assertion: exactly 1000. Result: **1000/1000 mediated**. Fails fast on first violation with `panic!("FR4 violation at line {line}: ...")`.
- Determinism: `fixture_is_byte_deterministic` re-runs the generator into a tempfile and asserts byte-equality with the checked-in fixture. Result: PASS.

**AC3 — Accessibility cascade emits zero ANSI bytes**
- `crates/maos-audit/src/lib.rs::to_plain` writes a tabular text view; never imports `colored`, never emits `0x1b`. Header row + per-entry row, padded to fixed-width columns.
- `to_fr4_ndjson` is JSON-only; never emits ANSI.
- Test: `crates/maos-cli/tests/audit_no_color_test.rs` (6 tests) spawns `maosctl` via `CARGO_BIN_EXE_maosctl`-resolved path with a seeded `MAOS_AUDIT_DB` and asserts `stdout.iter().filter(|b| **b == 0x1b).count() == 0` for the four combinations:
  - `TERM=dumb` × `--format ndjson` ✅
  - `NO_COLOR=1` × `--format ndjson` ✅
  - `TERM=dumb` × `--format plain` ✅
  - `NO_COLOR=1` × `--format plain` ✅
- Two additional regression tests: FR4 schema violation → exit 2 with diagnostic; unknown spirit name → exit 2 with v0.1-β diagnostic.

**AC4 — Canonical FR4 verification path + cap-audit ↔ Transparency Log join exercised end-to-end**
- One-shot path now opens on-disk SQLite at the XDG-resolved path so `maosctl audit query` reads exactly what `maos-bin` wrote. The cap-audit drain sequence (D3) guarantees the inference.call row reaches SQLite before exit.
- README: `crates/maos-audit/README.md` (~210 lines) documents the canonical FR4 path (clone → install → run → query), the schema, the cap-audit ↔ Transparency Log join (with ASCII diagram), the dep-direction rule, and the v0.1-β scope boundary. Cross-linked to architecture §8.4, PRD FR4 line 27, NFR-Obs-4, NFR-Ops-5, and predecessor stories.
- Integration smoke: `tests/integration/audit_query_fr4_smoke.sh` (executable) builds release binaries, runs one-shot hello-spirit, pipes `maosctl audit query --spirit hello-spirit --format ndjson | head -1 | jq -e '...all 6 keys present and typed...'`. PASS in 1.1s (warm) / 30s (cold release build).
- CI: new `audit-query-fr4-smoke` job in `.github/workflows/discipline.yml`, sat right after `fr4-1000-call-fixture` at 2-space indent. Added to `aggregate.needs`, `GITHUB_OUTPUT` block (`aqfs=`), JS `const aqfs`, and the comment table row. YAML validates via `python3 -c "import yaml; yaml.safe_load(...)"`.

**Backward compatibility note (caught and fixed during T10):**
- Initial dispatcher routed bare `maosctl audit query` (no `--spirit`) through `to_fr4_ndjson`. This broke `tests/integration/audit_spine_smoke.sh` which expects the legacy `frame_id`/`intent` keys.
- Resolution: gated the FR4 projection on `spirit.is_some()`. Bare invocation continues to call `to_ndjson` (the Story 1b.1 raw surface that Story 9.1 will extend). AC1's "exactly the keys ..." contract reads as the contract for the `--spirit` form, which is what the AC explicitly invokes.
- Verified: `bash tests/integration/audit_spine_smoke.sh` PASS post-fix; `audit_query_fr4_smoke.sh` PASS; `audit_no_color_test` 6/6 PASS.

### File List

**NEW:**
- `crates/maos-audit/README.md` (FR4 evaluator-path documentation)
- `crates/maos-audit/src/bin/gen_fixture.rs` (synthetic FR4 fixture generator; zero kernel-core dep)
- `crates/maos-audit/tests/query_schema_test.rs` (AC1 schema test, 4 cases)
- `crates/maos-audit/tests/fr4_full_mediation_test.rs` (AC2: 1000-entry FR4 + determinism, 2 cases)
- `crates/maos-audit/tests/fixtures/hello-spirit-1k.jsonl` (checked-in deterministic fixture, 1000 lines, 249,500 bytes)
- `crates/maos-cli/tests/audit_no_color_test.rs` (AC3 accessibility + FR4 dispatch exit-code, 6 cases)
- `scripts/gen_hello_spirit_fixture.sh` (executable; seed `0x5BF01A5B5BF01A5B`)
- `tests/integration/audit_query_fr4_smoke.sh` (executable; AC4 end-to-end smoke)

**UPDATED:**
- `crates/maos-audit/Cargo.toml` (added `[[bin]] name = "gen_fixture"` target)
- `crates/maos-audit/src/lib.rs` (added `AuditError::Fr4SchemaViolation`, `Fr4SchemaError`, `Fr4Entry`, `project_to_fr4`, `to_fr4_ndjson`, `to_plain`; 7 new lib unit tests)
- `crates/maos-bin/src/main.rs` (in-memory → on-disk Transparency Log; added `default_transparency_log_path()` helper; cap-audit drain sequence before one-shot exit)
- `crates/maos-cli/Cargo.toml` (added `rusqlite` + `tempfile` `[dev-dependencies]`; both already in workspace, no Cargo.lock blast)
- `crates/maos-cli/src/cli.rs` (extended `AuditQuery::Query` to struct variant with `--spirit` + `--format`; added `AuditFormat` value-enum)
- `crates/maos-cli/src/subcommands.rs` (extended `audit_query` for spirit/format; added `resolve_spirit_pid`; FR4 mode gated on `--spirit`; 5 new unit tests)
- `.github/workflows/discipline.yml` (added `audit-query-fr4-smoke` job + wired into aggregate)
- `Cargo.lock` (2-line diff: `rusqlite` + `tempfile` references on `maos-cli` entry; 0 new packages)
- `_bmad-output/implementation-artifacts/sprint-status.yaml` (story status: ready-for-dev → in-progress → review)

### Evidence Blocks

**Live one-shot + read-side verification:**
```
$ DB=$(mktemp -u --suffix=.sqlite)
$ MAOS_AUDIT_DB="$DB" MAOS_ONE_SHOT=hello-spirit ./target/debug/maos-bin
maos: Transparency Log opened on-disk at /tmp/tmp.mbYEmFWhOs.sqlite
maos: one-shot complete — exiting cleanly
$ MAOS_AUDIT_DB="$DB" ./target/debug/maosctl audit query --spirit hello-spirit --format ndjson
{"call_id":"019e2ab2cba9e92118334deb67e9e19a","capability_token":"240b78fe...","spirit_pid":0,"boot_nonce":2597302644735822443,"call_type":"inference.call","timestamp_ns":1778832821161969014}
{"call_id":"019e2ab2cbabbcc4a236a0b9a3c91605","capability_token":"240b78fe...","spirit_pid":0,"boot_nonce":2597302644735822443,"call_type":"capability.invocation","timestamp_ns":1778832821163089937}
{"call_id":"019e2ab2cbac524d22d505559414751e","capability_token":"240b78fe...","spirit_pid":0,"boot_nonce":2597302644735822443,"call_type":"capability.invocation","timestamp_ns":1778832821164251327}
```

**Test totals:**
- `cargo test -p maos-audit` — 10 lib + 4 schema + 2 fixture = **16/16 PASS**
- `cargo test -p maos-cli` — 17 lib + 6 ANSI/dispatch = **23/23 PASS**

**Pre-existing baseline confirmation (NOT this story's failures):**
- `cargo test --workspace`: 1 pre-existing failure (`mock_provider_round_trip_logs_inference_call`).
- `check-empty-kernel`: 13 pre-existing I9 violations in maos-kernel-core (1b.3/1b.4 residue). Same count and identifiers as baseline `7dfbdc5`.
- `check-service-boundary`: 2 pre-existing violations (InferencePortAdapter / TokenIssuer). Same as baseline.

**Cargo.lock blast: 0 new packages.**
```
$ git diff Cargo.lock | grep -E "^\+\[\[package" | wc -l
0
```

### Review Findings

- [x] [Review][Decision→Patch] `to_plain` bypasses FR4 schema enforcement when `--spirit` is set — Fixed: added `to_fr4_plain` to `maos-audit` and split the `(_, AuditFormat::Plain)` wildcard match arm. Both formats now enforce FR4 exit-code-2 contract per AC1.
- [x] [Review][Decision→Patch] Duplicated `default_transparency_log_path` across two crates — Fixed: extracted to `maos_audit::default_transparency_log_path()` with empty-string validation. Both `maos-bin` and `maos-cli` delegate to the shared function.
- [x] [Review][Patch] `audit_writer.await.ok()` silently swallows writer-task panics — Fixed: logs error via `eprintln!` on `Err(JoinError)` instead of `.ok()`.
- [x] [Review][Patch] `MAOS_AUDIT_DB=""` accepted without validation — Fixed: shared `default_transparency_log_path()` rejects empty strings with exit code 2.
- [x] [Review][Patch] `to_fr4_ndjson` emits partial NDJSON lines to stdout before schema violation — Fixed: output is buffered in a `Vec<u8>` and written only after all projections succeed.
- [x] [Review][Patch] `LIMIT` injected via `format!` instead of parameterized SQL — Fixed: uses `LIMIT ?` with boxed `i64` parameter.
- [x] [Review][Patch] `audit_query_fr4_smoke.sh` uses `mktemp -u` — Fixed: uses `mktemp` (atomic create) then `rm -f` to remove the placeholder.
- [x] [Review][Defer] Non-one-shot server exit path does not drain `audit_writer` — rows silently lost [`crates/maos-bin/src/main.rs:277-289`] — deferred, pre-existing
- [x] [Review][Defer] No test for bare `maosctl audit query --format plain` (without `--spirit`) — deferred, pre-existing
- [x] [Review][Defer] `to_plain` silent integer truncation: negative SQLite values cast to unsigned via `as` [`crates/maos-audit/src/lib.rs:155-157`] — deferred, pre-existing

### Change Log

- 2026-05-15 — Story 1b.5b context created. `maosctl audit query --spirit <name>` + FR4 NDJSON schema + 1000-entry fixture + ANSI cascade + cap-audit ↔ Transparency Log join documentation. Critical preconditions identify the in-memory→on-disk SQLite transition and cap-audit drain sequence as the load-bearing changes outside the audit crate.
- 2026-05-15 — Story 1b.5b implementation landed. AC1–AC4 all green. CLI `--spirit` + `--format` flags; `Fr4Entry` projection + writers in `maos-audit`; on-disk SQLite + deterministic cap-audit drain in `maos-bin`; 1000-entry synthetic fixture + determinism test; 6-test ANSI cascade; new CI job `audit-query-fr4-smoke`; README + FR4 doc cross-links. Backward-compat fix: bare `audit query` keeps legacy raw schema (Story 1b.1 surface); FR4 projection gated on `--spirit`. 0 new external transitive deps. 20/20 drain smoke. Dep-direction rule preserved (`maos-cli` ⊥ `maos-kernel-core`). Status → review.
