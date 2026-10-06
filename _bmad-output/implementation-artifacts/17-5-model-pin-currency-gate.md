---
story_key: 17-5-model-pin-currency-gate
status: done
updated: 2026-10-06
dev_model_used: anthropic/claude-opus-5-5
baseline_commit: 0d73cac32bb6155e46bdc4374f77ff2672f3f060
depends_on: "None. Independent of 17-1…17-4 (touches `xtask` + data files only: no `main.rs` line, no kernel line). **Deadline (tree-derivable): before `18-1` leaves `backlog`** — 18-1 AC4 cites `spirits/butler/manifest.toml:21` and `spirits/researcher/manifest.toml:47` and depends on this lane."
blocks: "`18-1` (leaving `backlog`)."
spec_alignment: "Rule 9 applied at creation against `0d73cac3` (§Premise corrections P1–P10): the epic's seven/five-pin count, the one-row price book, the two-literal `main.rs` scan, the zero-headroom `kloc.toml:320` cite and two of three AC7 fixture cites are stale. OLD→NEW edits are applied to the epic file in the same change (§Epic OLD→NEW edits)."
kernel_delta: "0 — nothing under `crates/maos-kernel-core/src`; `check-kernel-baseline` must still read 25293."
kloc_grant: "xtask only. GRANTED by operator ruling 2026-10-06 (up to the measured 44362); applied at the exact re-measured 44361 after the frontier re-run's review patches — see §Operator grant requested."
model: "frontier-class allowlist {opus-4-6, opus-4-7, opus-4-8, gpt-5.5, gpt-5.6, gpt-6.1, glm-5.1, glm-5.2, glm-5.3, opus-5, equiv}. `FRONTIER_FAMILIES` (`xtask/src/check_dev_model_tier.rs`) is the machine-checked set. Dev of record: `anthropic/claude-opus-5-5` (frontier re-run, operator ruling 2026-10-06). The initial draft and its review were produced by `anthropic/claude-sonnet-5-5`, which is NOT in the set; the operator declined to add it and ruled the dev pass be re-run on a frontier model instead."
review: "bmad-code-review (Blind Hunter + Edge Case Hunter + Acceptance Auditor; the Test Infrastructure Auditor is a no-op — the dev model is Claude). The gate parses hostile-free, repo-local files only, so no Security layer is bound. Run twice: the draft's pass (sonnet-5-5) and a fresh frontier §A6 pass (opus-5-5, 2026-10-06), both recorded under §Review Findings."
---

# 17-5 — Model-pin currency gate and the retired-pin sweep

Status: done

> **The capability:** *A vendor model id pinned anywhere the project ships — a Spirit manifest, a scaffold, the price
> book, a `main.rs` provider constructor — that the vendor has retired (or that nobody has vetted) turns CI red and
> names `file:line` and the replacement; and at HEAD no such pin exists.*

**Closes:** E14-A3 / E15-A2 / **E16-A4** (option A, operator-selected 2026-09-20) — the three-times-slipped lane
(`tracker-notes-archive-2026-09-04.md:705-707`, OD1 ratified).

**Does NOT close, and says so:** hermetic `#[cfg(test)]` fixtures and fuzz seeds that pin retired ids (AC7 — inert,
disclosed, not swept); documentation is not a scanned surface (the schema-v4 example `docs-site/docs/manifest/v4.md:26`
(+ko) was swept by operator ruling 2026-10-06; three cookbook examples are deferred); a way for the gate to
*learn* a new vendor retirement (it is offline by design — a human edits `xtask/model-currency.toml`);
`covered_model_id` provenance strings (D3).

## Story

As the project's pin-policy owner (Winston) and gate owner (Murat),
I want an offline gate that reds on every retired or unvetted vendor model id in the shipped surfaces, plus the sweep
that makes HEAD green,
so that the lane that slipped three retros cannot rot silently again and `18-1` can cite clean pins.

## Premise corrections at HEAD `0d73cac3` (rule 9 — measured at creation, not copied from the epic)

| # | Epic / tracker said | Measured at `0d73cac3` | Disposition |
|---|---|---|---|
| P1 | AC5: "exactly seven" retired pins; R8/R17-63: five once 17-3b lands | 17-3b **has** landed: `templates/spirit-ts/manifest.toml:12` and `examples/example-spirit-ts/manifest.toml:12` are `[capabilities.required.iac]` and declare **no** `provider.complete`. Manifest pins: **4** — `spirits/butler:21`, `spirits/researcher:47`, `templates/spirit-rust:12`, `examples/example-spirit:12` | The five-pin figure holds **for the manifests + price-book id the epic named**; see P2 for the true price-book count |
| P2 | `xtask/provider-pricing.toml:18` is the one retired price-book id | The book has **three** non-current ids: `:6 claude-3` (a family label, never an API id), `:12 gpt-4` (OpenAI-**deprecated**, shutdown 2026-10-23), `:18 claude-3-5-sonnet` (unversioned). AC3 scans the file, so the gate would red on all three | Sweep all three. Data-line edits = 4 manifest lines + 3 price-book lines = **7** (the epic's "7" was right by accident of a different composition). Header comment updated |
| P3 | `main.rs:3483` / `:3502`; two provider literals, both current | Lines are **`:3487` `claude-haiku-4-5-20251001`** and **`:3506` `gpt-4o-mini`** (+4 drift) **and a third constructor the epic omitted: `OllamaProvider::new` with `llama3.1:8b` at `:3532`**. All three are current/vetted | Gate scans all three constructors; `ollama` gets an `allowed` row |
| P4 | `kloc.toml:320` `xtask = 44101`, **ZERO HEADROOM** | The row is **`kloc.toml:328 xtask = 44123`** (17-6's +22 raise). Measured xtask at HEAD with the CI tokei args: **44040 → 83 lines of headroom**, not zero | The grant is still an exact-measured figure in the landing commit, but it is +239 over today's ceiling (44362), not +322 (§Operator grant requested) |
| P5 | AC7 fixture cites `maos-audit/src/lib.rs:2744` and `maos-manifest/src/manifest.rs:2624` | `maos-audit/src/lib.rs:2744` is `intent: "claude-3-haiku".into()` — an intent label, **not** a model pin. `manifest.rs:2624` is no longer a model id; the pin is at **`manifest.rs:2902`**. `anthropic.rs` cites (284,285,304,324,342,356,382) hold | Corrected in the epic. Additional inert pins found: `crates/maos-bin/tests/spawned_scope_17_3c.rs:307`, `crates/maos-kernel-core/tests/fixtures/manifest/capabilities/{edge-case,well-formed}/provider_complete.toml:1`, fuzz seeds `crates/maos-manifest/fuzz/corpus/manifest_parser/seed_{capabilities,full_manifest}.toml`, `tests/corpora/spirit-boundary-v0.1.jsonl:1-3` — all outside AC3, not swept |
| P6 | AC1: gate and data absent; zero `model-currency` hits in `main.rs`/`discipline.yml` | Confirmed absent at HEAD | — |
| P7 | AC4 enrolment sites: `gate-registry.toml:168` row, `[[ship_gate]]` shape `:314-316` | Both hold. Discipline job precedent `check-exit-commands` `discipline.yml:476`; ship-gate `needs` `:4351`; summary echo `:4395`; failure echo `:4434`; `EXPECTED_GATES` `check_ship_gate_completeness.rs:20-117` | Mirrored exactly |
| P8 | AC6: `spirits/hello-spirit/manifest.toml:12` already current | Confirmed (`anthropic.claude-haiku-4-5-20251001`) | **Not edited** |
| P9 | "current set" unspecified | Vendor docs fetched 2026-10-06 (§Latest technical facts). `claude-3-haiku-20240307` retired 2026-04-20; `claude-3-5-sonnet-20241022` retired 2025-10-28; `gpt-4` shuts down 2026-10-23. **`claude-haiku-4-5-20251001` is Active but its earliest retirement is 2026-10-15** | Recorded as D6 / operator question; the gate cannot pre-empt an offline fact |
| P10 | kernel pin `25293` | Verified by `check-kernel-baseline` at HEAD (§Dev Agent Record) | Kernel-Δ 0 holds |

## Decisions (rule 7)

- **D1 — pin targets.** `butler`, `templates/spirit-rust`, `examples/example-spirit` → `anthropic.claude-haiku-4-5-20251001`
  (Anthropic's own recommended replacement for `claude-3-haiku-20240307`; same tier; already hello-spirit's pin and the
  host's `main.rs` default). `researcher` → `anthropic.claude-sonnet-5-5` (its comment requires ≥Sonnet tier; Active,
  retirement not sooner than 2027-09-28 vs `claude-sonnet-4-6` 2027-02-17 — the vendor's 2025-08 recommendation is a
  lineup-older hop).
- **D2 — price book.** The **ids** change to the ones the host actually runs or pins: `claude-3` →
  `claude-haiku-4-5-20251001`, `gpt-4` → `gpt-4o-mini`, `claude-3-5-sonnet` → `claude-sonnet-5-5`. *Creation text kept
  the old synthetic prices (ADR-046 §7: "a committed synthetic price-book fixture"). Review B1 overturned that:*
  `cost-reconcile` looks rows up by exact `(provider, model)` against real journaled frames, so a real id on a fake
  price invents dollar figures where the old unmatched ids priced traffic at 0. **The rows now carry the vendors' list
  prices as of 2026-10-06** (Anthropic haiku-4-5 $1/$5 and sonnet-5-5 $2/$10 per MTok; OpenAI gpt-4o-mini
  $0.15/$0.60). No test or CI step reads this file (measured); `maosctl audit cost-reconcile` defaults to it.
  **Operator ruling 2026-10-06 (Q2): keep the vendor list prices.** Re-verified the same day against the Anthropic
  pricing page (Haiku 4.5 $1/$5, Sonnet 5.5 $2/$10 base input/output per MTok) and the OpenAI pricing and GPT-4o Mini
  model pages (standard $0.15 input / $0.60 output per MTok); 1 $/MTok = 1000 µ$ per 1k tokens, so 1000/5000,
  2000/10000 and 150/600 are correct. No fix needed.
- **D3 — scanned keys.** `capabilities.required.provider.complete` (dotted or table form), the Story 5.5b `[providers]`
  `primary`/`fallback` `model_id` pins (added by the frontier re-run review: schema-defined pins in a scanned file), the
  price-book `(provider, model)` rows, and the **third argument** (`model_id`) of each of the three `main.rs`
  provider-constructor calls, read inside the call's own argument list with comments dropped.
  `model_provenance.covered_model_id` is **not** scanned: the only instance (`spirits/mira`,
  `maos.mira.deterministic-diagnostic-v1`) is a first-party non-vendor id and AC3 names pins. Revisit if a manifest ever
  records a vendor id there.
- **D4 — data semantics.** `retired` = vendor-retired **or deprecated** (a deprecated pin is a countdown). Every entry has
  a display-only `replacement` that must be in the same provider's `allowed` (so the guidance cannot lie); the value is
  the tier-equivalent current id. An id in neither list is UNKNOWN and reds; so is an unlisted provider.
- **D5 — failure modes.** Blocking class, no `CURRENT_PHASE` coupling. Unreadable/inconsistent data (incl. a blank or
  twice-retired id), an unreadable surface (incl. a dangling manifest link or an unreadable directory entry), zero
  manifests → `Err`; an unparseable manifest, a `provider` table with no `complete` array, an empty price book (no
  `[[entries]]` or `entries = []`), a host provider constructor (all three of anthropic/openai/ollama are required)
  whose third argument is not a readable literal → a finding at that file (never green). A dotless `provider.complete`
  entry grants a whole provider and pins no model.
- **D6 — Haiku 4.5.** Stays `allowed`. When the vendor deprecates it, a human moves it to `retired` and the gate reds
  hello-spirit, the swept pins and `main.rs` together — that is the gate working, not a defect. **Operator ruling
  2026-10-06 (Q1): keep it and re-sweep later; do not pre-empt.** Dated watch item, trigger = the Anthropic
  deprecation notice for Claude Haiku 4.5: `deferred-work.md` §17-5 and the `xtask/model-currency.toml` header.
- **D7 — tests.** `xtask/tests/model_currency_gate.rs` (outside the `kloc` count by the `-e tests` rule; run in the CI
  job's second step). Vectors plant into a **copy of the real tree** and call the real `audit()`.

## Acceptance Criteria

1. **AC1 (command).** `cargo run -p xtask -- check-model-currency` exits 0 at HEAD-after-this-story and, for every
   retired or unknown model id, prints `file:line` and (for retired ids) the replacement. Offline: no socket.
2. **AC2 (data).** `xtask/model-currency.toml` carries per-provider `allowed` and `retired`; every `retired` entry has a
   `replacement` (display-only; the gate never rewrites a manifest), and an entry without one is refused.
3. **AC3 (surfaces, exactly).** `spirits|templates|examples/*/manifest.toml`, `xtask/provider-pricing.toml`, and the
   `main.rs` provider-construction literals (three, P3). `.claude/worktrees/**` and `#[cfg(test)]`/fuzz fixtures are not
   scanned.
4. **AC4 (five-point enrolment).** (1) subcommand in `xtask/src/main.rs`; (2) `discipline.yml` job **and** `v1.0-ship-gate`
   `needs:`; (3) `gate-registry.toml` flat `gates` + `[[ship_gate]]` block; (4) `EXPECTED_GATES`; (5) proven red — a
   retired id planted in a scanned manifest reds naming that `file:line`, removed it is green (both executed; §Dev Agent
   Record).
5. **AC5 (sweep).** The 4 manifest pins and 3 price-book ids (P1, P2) move to the current set; `hello-spirit` is not
   edited; `templates-regen --check` stays green (template and example change together).
6. **AC6.** `spirits/hello-spirit/manifest.toml` is untouched.
7. **AC7.** Inert fixtures are not swept and the gate is green with them present (the real-tree test asserts it).
8. **AC8 (governance).** `xtask/kloc.toml` is NOT edited; the exact measured figure is recorded as a grant request with a
   scratch-copy proof. — **Superseded by operator ruling 2026-10-06:** the grant was approved (up to the measured
   44362) and is applied in this change at the exact re-measured figure, 44361 (§Operator grant requested).

## Tasks / Subtasks

- [x] **T1 — data (AC2).** `xtask/model-currency.toml` from the vendor docs (§Latest technical facts).
- [x] **T2 — gate (AC1, AC3).** `xtask/src/check_model_currency.rs`: `parse_data`, `judge`, the three scanners, `audit`, `run`.
- [x] **T3 — enrolment (AC4 1–4).** `main.rs` (mod + variant + dispatch), `lib.rs` (`pub mod`), `gate-registry.toml` (flat +
  `[[ship_gate]]`), `EXPECTED_GATES`, `discipline.yml` (job + `needs` + summary + failure echo).
- [x] **T4 — sweep (AC5, AC6).** Seven data-line edits + price-book header.
- [x] **T5 — tests (AC4-5, AC7).** `xtask/tests/model_currency_gate.rs` (12 vectors).
- [x] **T6 — measure and gates (AC8).** `cargo fmt --all`, `kloc-check --json`, the 14 gates, `cargo test -p xtask`, the
  manual red/green proof, the grant scratch-proof.
- [x] **T7 — tracking.** Epic OLD→NEW edits, sprint row, `deferred-work.md`.
- [x] **T8 — frontier re-run (operator ruling 2026-10-06).** Re-verify every AC on `anthropic/claude-opus-5-5`; apply the
  operator grants (KLOC, ratified `allowed` set, Haiku watch item, list prices, v4 docs pin); fresh §A6 review; close.

### Review Findings

**Draft pass (dev and review by `anthropic/claude-sonnet-5-5`; kept as history).** bmad-code-review, 2026-10-06, diff = every uncommitted non-tracking file (1015 lines, 13 files). Layers: Blind Hunter (7 findings), Edge Case Hunter (5), Acceptance Auditor (3); no layer failed. Test Infrastructure Auditor is a no-op (dev model is Claude). 15 raw → **12 unique** (three duplicates merged). 10 `patch`, 2 tracking items, 0 `decision-needed`, 0 `defer`, 0 dismissed.

- [x] [Review][Patch] The price-book retarget made `cost-reconcile` price real traffic at the old synthetic prices (gpt-4o-mini at 30000/60000 µ$ per 1k, ~100–200× the real figure) — fixed: the three rows carry the vendors' list prices as of 2026-10-06 and the header says so; D2 amended [`xtask/provider-pricing.toml`:1-25] (medium)
- [x] [Review][Patch] A `provider.complete` that is a string, or a misspelled key (`compelte`), yielded zero pins and passed — fixed: a `capabilities.required.provider` table without a `complete` array is a finding; test `a_manifest_that_hides_its_pins_from_the_scan_reds` [`xtask/src/check_model_currency.rs`:162-193] (medium)
- [x] [Review][Patch] One host provider constructor dropping out of `main.rs` left the other two keeping the scan green — fixed: all three of `anthropic`/`openai`/`ollama` must be found; test `a_host_provider_constructor_that_goes_missing_reds_even_if_the_others_remain` [`xtask/src/check_model_currency.rs`:226-270] (medium)
- [x] [Review][Patch] The constructor scan missed `::with_api_key`, a model literal on the constructor line, a trailing comment or `.to_string()`, and let a `//` comment open a call — fixed: the model literal is the first non-URL literal inside the call's own argument list (to the first line opening with `)`), comment lines open nothing; the api-key literal is never judged; test `main_rs_constructors_are_read_in_every_layout_and_comments_open_no_call`. The module doc now states that `main.rs` is read whole, its test module included [`xtask/src/check_model_currency.rs`:226-270] (low)
- [x] [Review][Patch] `locate()` named the wrong line for a TOML literal string, a trailing `# was "id"` comment, a duplicate id in one array, and two price rows sharing a model — fixed: both quote forms, text after `#` ignored, `nth` occurrence; tests `a_provider_table_reds_on_the_entry_line_through_comments_literals_and_duplicates` and `two_price_rows_sharing_a_model_each_name_their_own_line` [`xtask/src/check_model_currency.rs`:147-160] (low)
- [x] [Review][Patch] A dotless `provider.complete` entry (a whole-provider grant that pins no model) was misreported as `UNKNOWN provider for `.anthropic`` — fixed: it pins nothing and is skipped, covered in the table test above [`xtask/src/check_model_currency.rs`:181-183] (low)
- [x] [Review][Patch] The CI job comment said the gate reads only `xtask/model-currency.toml` — fixed: it reads repo-local files only and opens no network [`.github/workflows/discipline.yml`:502-508] (low)
- [x] [Review][Patch] After `cargo fmt` reordered the `pub mod` lines, the 17-5 why-comment sat above `check_mock_not_in_release` rather than `check_model_currency` (reported by two layers) — fixed [`xtask/src/lib.rs`:15-18] (low)
- [x] [Review][Patch] The module doc claimed a missing constructor literal is an `Err`; it is a finding at that file (still never green) — fixed in the doc [`xtask/src/check_model_currency.rs`:28-34] (low)
- [x] [Review][Patch] `Report` lacked `Debug`, hiding `unwrap_err()` in tests — fixed by asserting through a `refusal()` helper that accepts an `Err` or findings and never a pass [`xtask/tests/model_currency_gate.rs`] (low)
- [x] [Review][Patch] Acceptance Auditor: AC8 and the AC4(5) evidence were not yet recorded in the story when the layer ran — closed in this pass (§Operator grant requested, §Debug Log References) [`_bmad-output/implementation-artifacts/17-5-model-pin-currency-gate.md`]
- [x] [Review][Patch] Acceptance Auditor: T7 (epic OLD→NEW block, sprint row, deferred-work) was not yet applied when the layer ran — closed in this pass [`_bmad-output/planning-artifacts/epics/epic-17-workers-and-third-party-form-w2.md`:180-187]

**Frontier re-run pass (§A6, `anthropic/claude-opus-5-5`, 2026-10-06) — a fresh review; the draft's conclusions were not reused.** Diff = `git diff HEAD` (code, data, CI, docs; tracking files excluded) plus the three new files: 1209 lines, 16 files. Layers, each a fresh `@default`-model subagent: Blind Hunter (diff only, 11 findings), Edge Case Hunter (diff + repo, 9), Acceptance Auditor (diff + story + epic + the operator rulings, 2); no layer failed. Test Infrastructure Auditor: no-op (dev model is Claude). 22 raw → **20 unique** (two merges). **11 `patch` (all applied), 1 `defer`, 0 `decision-needed`, 8 dismissed.** Every patch to gate behaviour has a vector that reds on the draft gate and passes on the patched gate (both runs executed, §Debug Log References).

- [x] [Review][Patch] A `//` comment inside a constructor's argument list could stand in for the model. The draft took the first non-URL literal in the window, so `// was "claude-haiku-4-5-20251001".into(),` above a live retired literal passed green (fail-open) — fixed: the argument list is lexed with comments dropped; test `a_comment_inside_a_constructor_never_stands_in_for_its_model` [`xtask/src/check_model_currency.rs`:232-295] (medium; edge)
- [x] [Review][Patch] A call that closes on its own line was read up to 8 lines past its `)`. With a non-literal model, the next unrelated literal was judged instead, and the `no readable model literal` refusal was bypassed — fixed: the window is the call's own argument list (paren depth, literals respected); test `a_one_line_call_with_a_non_literal_model_refuses_instead_of_reading_past_it` [`xtask/src/check_model_currency.rs`:232-295] (medium; blind+edge)
- [x] [Review][Patch] Where the model was not a literal, the api-key literal of `with_api_key` was judged as the model and echoed in the finding — fixed: the model is the THIRD argument (`transport, endpoint_url, model_id, …` in every `maos-providers` constructor) and must be a `"…".into()`-style literal; test `the_api_key_argument_is_never_judged_as_the_model` [`xtask/src/check_model_currency.rs`:232-295] (low; edge)
- [x] [Review][Patch] `entries = []` in the price book passed green with zero pins from that surface — fixed: an empty `entries` array is the same "price book would govern nothing" finding; test `an_empty_entries_array_is_an_empty_price_book` [`xtask/src/check_model_currency.rs`:207-230] (medium; edge)
- [x] [Review][Patch] `locate()` counted matching lines, not occurrences. A duplicate id on one line (the inline-array form every manifest uses) fell back to `:1` — fixed: occurrences are counted per file through a `seen` map; test `a_duplicate_on_one_line_names_that_line_not_line_one` [`xtask/src/check_model_currency.rs`:150-167] (low; blind+edge)
- [x] [Review][Patch] The Story 5.5b `[providers]` `primary`/`fallback` `model_id` field is a schema-defined model pin in a scanned file, and it escaped the scan. No shipped manifest uses it, and every `admit_spirit` call passes `providers: None` — fixed: scanned; test `a_providers_section_model_id_is_a_pin` [`xtask/src/check_model_currency.rs`:169-205] (low; edge)
- [x] [Review][Patch] A dangling `manifest.toml` symlink (`is_file()` false) or an unreadable directory entry (`.flatten()`) was skipped silently, not refused — fixed: entries are listed fallibly and a manifest is detected by `symlink_metadata`, so an unreadable one is an `Err`; test `a_dangling_manifest_link_is_a_refusal_not_a_skip` [`xtask/src/check_model_currency.rs`:301-353] (low; edge)
- [x] [Review][Patch] `parse_data` accepted a blank retired id and one id retired twice with two replacements (`judge` silently took the first) — fixed: refused; test `the_data_file_refuses_an_empty_provider_map_and_blank_or_repeated_retired_ids` [`xtask/src/check_model_currency.rs`:96-127] (low; blind)
- [x] [Review][Patch] The `no providers` refusal branch was never executed: serde's `missing field` error satisfied the `"providers"` fragment first — fixed: `providers = {}` vector in the same test [`xtask/tests/model_currency_gate.rs`] (low; edge)
- [x] [Review][Patch] Tracking still said the v4 docs pin was unswept after ruling 5 swept it — fixed in the epic Rule-9 block (AC7 bullet) and `deferred-work.md` §17-5 (row closed; a docs-scope row names the three cookbook examples that remain) [`_bmad-output/planning-artifacts/epics/epic-17-workers-and-third-party-form-w2.md`:186] (low; auditor)
- [x] [Review][Patch] The module doc and the data-file header said the gate's "whole world" is `xtask/model-currency.toml`, but it reads the manifests, the price book and `main.rs` too — fixed: "reads only repo-local files; the data file is its only source of vendor facts" [`xtask/src/check_model_currency.rs`:11-16, `xtask/model-currency.toml`:3-7] (low; auditor)
- [x] [Review][Defer] Docs are outside the gate's scan set (AC3), so a docs example can rot. Three cookbook examples still show non-current ids (`capability-scoping.md:22`, `manifest-fields.md:58` `anthropic/claude-3`; `compliance-claim.md:98` `covered_model_id anthropic.claude-3-opus`; ko twins) [`docs-site/docs/cookbook/`] — deferred: the AC3 scope boundary predates this pass, and the operator ruling covered only the v4 page; recorded in `deferred-work.md` §17-5 (blind)

Dismissed (8): a `#` inside a quoted string, a `\u` escape, or the same quoted id under an unrelated earlier key can misplace a finding's line, but the verdict is unaffected and no valid vendor id contains those forms; scanner-error findings report `:1` and `main.rs` stops at its first unreadable constructor, but the detail names `line N` and the gate is never green; the price-book retarget has no hidden consumer (grep: only `maosctl audit cost-reconcile --pricing`'s default reads the file; no test, golden or CI step); other `provider` verbs cannot exist (`RawProviderCapabilities` is `deny_unknown_fields` with only `complete`); a misspelled `providers`/`requried` key is refused by the manifest parser's `deny_unknown_fields` or grants nothing, so no live pin escapes; `--json` prints no JSON on an `Err`, which matches the house shape (`check_decision_register` returns `audit(..)?` the same way), and the exit code is non-zero; `files_scanned` counts files that were read even when they produced a finding (`passed` is false then); "Haiku 4.5 may retire in nine days" was settled by operator ruling Q1.

## Epic OLD→NEW edits

Applied to `epic-17-workers-and-third-party-form-w2.md` §17-5 as the *Rule-9 OLD→NEW edits* block (after AC7, `:180-187`):

| Where | OLD | NEW |
|---|---|---|
| AC3 / AC1.3 | `main.rs:3483`, `:3502`; two provider literals | `:3487`, `:3506`, **and `OllamaProvider::new` `llama3.1:8b` at `:3532`**; all three scanned and required |
| AC5 / AC2.1 | "exactly seven" retired pins (five after 17-3b) | 17-3b landed: **4 manifest pins + 3 price-book ids** (`:6 claude-3`, `:12 gpt-4`, `:18 claude-3-5-sonnet`) = 7 data-line edits |
| AC5 targets | "the current set" | butler/spirit-rust/example-spirit → `anthropic.claude-haiku-4-5-20251001`; researcher → `anthropic.claude-sonnet-5-5`; price book → the ids the host runs, at vendor list prices |
| Δ line | `kloc.toml:320` `xtask = 44101`, ZERO HEADROOM | `kloc.toml:328` `xtask = 44123`; measured HEAD 44040 → **83 headroom**; granted 2026-10-06, applied at the exact re-measured **44361** |
| AC7 | `maos-audit/src/lib.rs:2744`, `maos-manifest/src/manifest.rs:2624` pin the id | `2744` is an intent label, not a pin; the manifest pin is `manifest.rs:2902`; further inert pins listed; `docs-site/docs/manifest/v4.md:26` (+ko) swept by operator ruling 2026-10-06 (docs stay unscanned) |
| AC1 | both files absent | landed, with all five enrolment points |

## Operator questions

- **Q1.** `claude-haiku-4-5-20251001` is Active but its earliest retirement date is 2026-10-15 (nine days after this pass). It is pinned by hello-spirit, the three swept manifests and the `main.rs` default. Re-sweep when Anthropic deprecates it, or pre-empt now?
- **Q2.** The price-book rows now carry vendor list prices and therefore price real traffic in `maosctl audit cost-reconcile` (review B1). Keep, or restore synthetic prices on ids no real frame produces (and accept that the book then prices nothing real)?
- **Q3.** Ratify the initial `allowed` set in `xtask/model-currency.toml` (Winston, pin policy), notably `ollama: ["llama3.1:8b"]`, `openai: gpt-4o-mini` (catalogued, undeprecated, but a previous generation) and the retention of `claude-sonnet-5` / `claude-opus-4-x` as allowed.
- **Q4.** `docs-site/docs/manifest/v4.md:26` (+ ko `:27`) still shows a retired id as a schema-v4 example — edit it, or add `docs-site/docs/manifest/*.md` to the scanned surfaces?

**Operator rulings 2026-10-06 (applied by the frontier re-run):**

- **Q1 → keep Haiku 4.5, re-sweep later, do not pre-empt.** A dated watch item names the trigger, the Anthropic deprecation notice, in `deferred-work.md` §17-5 and the `xtask/model-currency.toml` header (D6).
- **Q2 → keep the vendor list prices.** Re-verified against the vendor pricing pages; all three rows are correct (D2).
- **Q3 → the initial `allowed` set is ratified.** Re-verified the same day. Every Anthropic `allowed` id is `Active` on the model-deprecations page. `claude-opus-4-5-20251101` (not sooner than 2026-11-24) and `claude-haiku-4-5-20251001` (not sooner than 2026-10-15) are the nearest commitments, and neither is deprecated. `claude-haiku-4-5` is that snapshot's alias. Every OpenAI `allowed` id is on the models page, and none is on the deprecations page. Only `gpt-4o-mini`'s `-tts`/`-transcribe`/`-realtime`/`-audio`/`-search-preview` variants are deprecated; the text model `gpt-4o-mini` is not. `ollama: ["llama3.1:8b"]` has no vendor feed. **No id was dropped.**
- **Q4 → edit the example.** The gate's scope stays exactly AC3 (docs unscanned), and editing an example does not change it. A historical schema page may describe old schema fields, but its example must not teach a retired model id. `v4.md:26` and ko `:27` now show `anthropic.claude-sonnet-5-5`, the tier-equivalent current id and the data file's `replacement` for `claude-3-5-sonnet-20241022`. No docs test pins the page content (the docs-site scripts and Playwright specs were checked).

## Dev Notes

### Current state of every file this story edits (read before editing)

- `xtask/src/main.rs` — `mod` list is alphabetical; `Commands` variants carry `#[command(name = "…")]` + `#[arg(long)] json`;
  dispatch is one arm per variant. Preserve all three orderings.
- `xtask/src/lib.rs` — only modules that integration tests must drive are `pub mod` here, each with a why-comment. A
  module in the lib may depend only on lib modules (`crate::gate_common` is one).
- `xtask/gate-registry.toml` — flat `gates` list ends at `workspace-test-suite` (`:173`); `[[ship_gate]]` blocks follow with
  a why-comment each. `v1_0` through `v2_2` all `blocking` is the hermetic-gate shape (14-0, 15-1).
- `xtask/src/check_ship_gate_completeness.rs` — `EXPECTED_GATES` is enrolled LAST (15-3 comment), because it reds on a
  missing job/`needs`/registry row.
- `.github/workflows/discipline.yml` — every job carries `timeout-minutes` (E16-A2 rule: ≥30 for warm-cache jobs); the
  ship-gate `needs`, its summary table and its failure list are three separate lists to keep in step.
- `xtask/provider-pricing.toml` — three `[[entries]]`; consumed only by `maosctl audit cost-reconcile --pricing` default.
- Manifests — each pin line is a single dotted `provider.complete = ["<provider>.<model>"]`.

### Guardrails

- Do **not** edit `xtask/kloc.toml`, `kernel-core-baseline.toml`, `abi-ratifications.toml` or `FRONTIER_FAMILIES`. *(Frontier re-run: `xtask/kloc.toml` was edited under the operator's 2026-10-06 grant only; the other three are untouched.)*
- Do **not** touch `spirits/hello-spirit/manifest.toml` (AC6) or any `#[cfg(test)]` fixture (AC7).
- No new crate dependency (`toml`, `serde`, `regex`, `tempfile` are already in `xtask/Cargo.toml`).
- No network from the gate or its tests.
- A proven-red vector must change the surface it plants into (the plant asserts it), and must read through `audit()`.

### Previous story intelligence

- **15-3 / 14-0** (`check-exit-commands`, `check-decision-register`): the module-doc-states-the-defect style, `audit()` exposed
  through `lib.rs`, FAILS-CLOSED rule (an empty scan is an error), proven-red vectors in `xtask/tests/` with a GREEN control
  beside every red, and the CI job running the vectors beside the gate.
- **17-3b/17-3c:** TS scaffolds dropped `[capabilities.required]` provider pins (P1); 17-3c added `gpt-6.1` to
  `FRONTIER_FAMILIES` by operator ratification — the precedent for this story's item-2 request (declined 2026-10-06;
  see §Operator grant requested).

### Git intelligence

`0d73cac3` (17-3c done) is HEAD; recent xtask edits were gate registrations and the 17-6 raise to 44123.

### Latest technical facts (fetched 2026-10-06 — the only network this story used)

- Anthropic models overview: Active IDs `claude-fable-5-1`, `claude-opus-5-5`, `claude-sonnet-5-5`,
  `claude-haiku-4-5-20251001` (alias `claude-haiku-4-5`); legacy-still-available `claude-fable-5`, `claude-opus-5`,
  `claude-opus-4-8/-4-7/-4-6`, `claude-opus-4-5-20251101`, `claude-sonnet-5`, `claude-sonnet-4-6`. Haiku 4.5 retirement
  "not sooner than October 15, 2026".
- Anthropic deprecations: retired — `claude-3-haiku-20240307` (2026-04-20), `claude-3-5-haiku-20241022`,
  `claude-3-7-sonnet-20250219` (2026-02-19), `claude-3-5-sonnet-20240620/-20241022` (2025-10-28), `claude-3-opus-20240229`
  (2026-01-05), `claude-3-sonnet-20240229` (2025-07-21), `claude-sonnet-4-20250514`, `claude-opus-4-20250514` (2026-06-15),
  `claude-opus-4-1-20250805` (2026-08-05); deprecated — `claude-sonnet-4-5-20250929` (retires 2026-11-30).
- OpenAI models/deprecations: current `gpt-6-astra`, `gpt-6.1-sol`, `gpt-6-sol`, `gpt-6-luna`, `gpt-5.6-{sol,terra,luna}`;
  `gpt-4o-mini` is catalogued and on no deprecation list. Shutdown 2026-10-23: `gpt-4`, `gpt-4-turbo`, `gpt-3.5-turbo`,
  `gpt-4.1-nano` (substitutes `gpt-5.6-sol/-terra/-luna`).
- **Re-verified 2026-10-06 by the frontier re-run** (models overview, model-deprecations and pricing pages for
  Anthropic; models, deprecations, pricing and GPT-4o Mini pages for OpenAI): every fact above holds. Additional facts:
  `claude-opus-4-5-20251101` is Active, not sooner than 2026-11-24; Anthropic gives ≥60 days' notice before a
  retirement; OpenAI's deprecations list `gpt-4o-mini-{tts,transcribe,realtime,audio,search-preview}` variants but not
  the text model `gpt-4o-mini`; list prices: Haiku 4.5 $1/$5, Sonnet 5.5 $2/$10, gpt-4o-mini $0.15/$0.60 per MTok.

### Testing standards

`xtask/tests/*.rs` integration tests over the lib's real `audit()`; a GREEN control beside every red; plants assert they
changed the surface; CLI wiring proved through `CARGO_BIN_EXE_xtask`.

### Operator grant requested

Two grants are requested. **Neither governance file was edited** — `xtask/kloc.toml` and `xtask/src/check_dev_model_tier.rs` are byte-identical to `0d73cac3`.

1. **`xtask/kloc.toml`, key `xtask`: `44123` → `44362` (+239 over the ceiling).** Exact measured figure, taken after `cargo fmt --all` with tokei 14.0.0 (the CI pin) through `cargo run -p xtask -- kloc-check --json` (`over_budget: ["xtask 44362 > 44123"]`). Gross story delta **+322** against a measured HEAD base of **44040** (83 lines of headroom absorb 83 of them — P4): `check_model_currency.rs` +313, `main.rs` +7, `lib.rs` +1, `check_ship_gate_completeness.rs` +1. `xtask/tests/model_currency_gate.rs` is excluded from the count (`kloc-check` passes `-e tests`). Inside the epic's +260–480 range. Aggregate 170756 stays under the 170884 hard-fail (the alarm that already fired is not silenced).
   - **Scratch proof (executed 2026-10-06):** a scratch copy of `xtask/kloc.toml` with `xtask = 44362` (placed at `target/kloc-scratch.toml`, so the workspace root resolves to the worktree, then deleted) → `cargo run -p xtask -- kloc-check --config target/kloc-scratch.toml --json` → `passed: true`, `over_budget: []`, aggregate 170756. The real file is unchanged (`git status` clean for it).
2. **`xtask/src/check_dev_model_tier.rs` `FRONTIER_FAMILIES`: add token `"sonnet-5-5"`** (dated rationale as the opus-5 / glm-5.3 / gpt-6.1 precedents). This story's dev model is `anthropic/claude-sonnet-5-5` (Anthropic's overview: "the best combination of speed and intelligence", 1M context); the blocking `check-dev-model-tier` reds on it and on nothing else. Not a waiver: the §A6 net ran. If the operator does not rule the model frontier, the alternative is re-attributing the dev pass, which would be false.
   - **Scratch proof (executed 2026-10-06):** with `"sonnet-5-5"` added to `FRONTIER_FAMILIES` as a temporary edit, `cargo run -p xtask -- check-dev-model-tier` → `PASS — 59 frontier-era stories, all on allowlisted models with a §A6 artifact`, rc 0. The edit was reverted; `git status` is clean for the file.

Both gates, with the real files, fail only on these two figures: `kloc-check` (`xtask 44362 > 44123`, which also fails the unit test `kloc_check_runs_on_workspace`) and `check-dev-model-tier` (this story's model).

**Operator rulings 2026-10-06:**

1. **KLOC — GRANTED** up to the measured 44362, applied at the exact re-measured figure if lower. After the frontier
   re-run's review patches (and a compaction of the same module), `kloc-check --json` measures **xtask 44361**
   (`check_model_currency.rs` +312, `main.rs` +7, `lib.rs` +1, `check_ship_gate_completeness.rs` +1 = +321 over 44040),
   so `xtask/kloc.toml:328` is set to **44361** with its dated driver. Aggregate **170754** < `_aggregate_hardfail`
   170884. No other governance figure changed.
2. **`FRONTIER_FAMILIES` `sonnet-5-5` — DECLINED.** The dev pass was re-run on a frontier model instead
   (`anthropic/claude-opus-5-5`, matched by the existing `opus-5` token). `xtask/src/check_dev_model_tier.rs` is untouched.

### Project Structure Notes

Gate module `xtask/src/check_model_currency.rs`; data `xtask/model-currency.toml`; tests `xtask/tests/model_currency_gate.rs`.
No conflicts with the unified structure.

### References

- `_bmad-output/planning-artifacts/epics/epic-17-workers-and-third-party-form-w2.md` §17-5 (≈:162-180) and table row `:66`
- `_bmad-output/implementation-artifacts/sprint-status.yaml` row `17-5-model-pin-currency-gate`
- `_bmad-output/implementation-artifacts/tracker-notes-archive-2026-09-04.md:705-707`
- `xtask/src/check_decision_register.rs`, `xtask/src/check_exit_commands.rs`, `xtask/tests/decision_register_gate.rs` (house shape)
- `docs/adr/ADR-046-cost-attribution-and-reconciliation.md` §7 (synthetic price book)

## Dev Agent Record

### Agent Model Used

anthropic/claude-opus-5-5

Frontier re-run (operator ruling 2026-10-06): the initial draft and its review were produced by
anthropic/claude-sonnet-5-5; this pass re-verified every AC, applied the operator grants (KLOC 44123 → 44361
exact measured; the ratified `allowed` set, re-verified with no drops; the dated Haiku 4.5 watch item; vendor list
prices, re-verified; the v4 docs pin, en + ko) and 11 review patches over the draft (the `main.rs` constructor scan
reads the third argument of the call's own argument list with comments dropped; `[providers]` `model_id` pins scanned;
`entries = []` refused; occurrence-based `locate`; fallible manifest listing; blank or twice-retired ids refused; the
"whole world" doc claim corrected; tracking text corrected), and re-ran §A6 review.

### Debug Log References

**Frontier re-run (`anthropic/claude-opus-5-5`, 2026-10-06), worktree `/tmp/maos-17-5-wt` at `32420928` + the uncommitted draft.**
Env: `unset CARGO_TARGET_DIR` (the worktree's `target` symlink → its own dir); final verification on `RUSTUP_TOOLCHAIN=1.99.0`
(CI stable), `--offline --locked`.

- **AC1 (command):** `xtask check-model-currency` at HEAD → `PASS (11 pins across 17 files)`, rc 0; `--json` →
  `{"passed":true,"files_scanned":17,"pins_checked":11,"findings":[]}`. The 11 pins: 5 manifest (butler, hello-spirit,
  researcher, spirit-rust, example-spirit), 3 price-book, 3 `main.rs` (`:3487`, `:3506`, `:3532`). Offline: the module
  and its tests use no `std::net`/HTTP client (grep).
- **AC4(5) proven red, executed on the final gate:** `spirits/butler/manifest.toml:21` → `anthropic.claude-3-haiku-20240307`
  → `FAIL — 1 finding(s) over 11 pin(s): - spirits/butler/manifest.toml:21 — RETIRED model id … replacement …
  claude-haiku-4-5-20251001`, rc 1 (`--json`: `passed:false`, `line:21`); restored → `PASS (11 pins across 17 files)`,
  rc 0; `git diff` of the file is the sweep line only.
- **AC4(1–4):** `check-ship-gate-completeness` → `all 42 expected gates present`; the job, `needs`, summary and failure
  echoes are in `discipline.yml`; flat + `[[ship_gate]]` rows in `gate-registry.toml`; `EXPECTED_GATES` row present.
- **AC5/AC6:** 4 manifest pins + 3 price-book rows swept (diff); `templates-regen --check` rc 0; `git diff --quiet HEAD --
  spirits/hello-spirit/manifest.toml` → untouched.
- **AC7:** `the_swept_tree_is_green_and_governs_its_pins` runs `audit()` on the real root, inert fixtures present → green.
- **AC2:** `the_data_file_is_refused_when_it_would_make_the_guidance_lie` (missing `replacement` refused) and
  `the_shipped_data_names_a_replacement_for_every_retired_id` pass.
- **Vendor re-verification:** the Anthropic models overview, model-deprecations and pricing pages, and the OpenAI
  models, deprecations, pricing and GPT-4o Mini pages were fetched on 2026-10-06. Every `allowed` id is current, so
  none was dropped. Every retired/deprecated row matches. The price-book figures match.
- **Review patches proven against the draft:** the draft gate (extracted from the review diff) was swapped into the
  tree, and the vector file was run against it: **12 passed, 8 failed**. The 8 failures are exactly the 8 new vectors.
  The patched gate was then restored (`cmp` identical) and the same file run again: **20 passed, 0 failed**.
- **KLOC:** after the patches, xtask measured 44384 (+22 over the grant). The module was compacted (no behaviour
  change; the vectors stayed 20/20), and `kloc-check --json` then gave **xtask 44361, aggregate 170754, `passed:
  true`, `over_budget: []`**. `check_model_currency.rs` is 312 tokei code lines. `xtask/kloc.toml:328` is set to
  44361.
- `cargo fmt --all --check` → rc 0.
- `cargo test --offline --locked -p xtask --no-fail-fast` (1.99.0) → **961 passed, 0 failed, 1 ignored** (base 941 +
  20 vectors), and `kloc_check::tests::kloc_check_runs_on_workspace` is ok.
- **Gates (1.99.0), every one rc 0:**
  - `check-kernel-baseline` PASSED (25293 = pin)
  - `kloc-check --json` passed
  - `check-dev-record-completeness` PASSED (165)
  - `check-exit-commands` PASS
  - `check-dev-model-tier` PASS (59 stories)
  - `check-dev-model-used-populated` PASS. Its WARN `unknown model: anthropic/claude-opus-5-5` comes from the
    `KNOWN_MODELS` warning list, not a failure.
  - `check-fkcs --json` passed
  - `templates-regen --check`
  - `gen-abi-docs --check`
  - `check-manifest-schema-version` PASSED
  - `stability-matrix --check` PASS
  - `check-equiv-fixture-provenance` PASSED
  - `check-wasm-form-equiv --json` passed
  - `check-service-boundary` PASSED
  - `check-serde-error-handling --json` passed (0)
  - `check-ship-gate-completeness` PASSED
  - `check-model-currency` PASS
- Cleanup: the empty `/tmp/maos-exit-cmd-help-<pid>` dirs these runs leaked were removed, along with this pass's
  review scratch. No `worker-cli-fixture` process was left running.

**Draft pass (`anthropic/claude-sonnet-5-5`; kept as history):**

- Build/gate env: `CARGO_TARGET_DIR=/mnt/build/maos-17-5-tgt` set explicitly — the ambient `CARGO_TARGET_DIR=/mnt/build/cargo` overrides the worktree's `target` symlink and would contend with other checkouts' builds.
- `cargo test --offline --locked -p xtask --test model_currency_gate` → **12 passed, 0 failed** (RED→GREEN: the first run of the vectors found `Report` has no `Debug` and a fail-closed case that returns a finding, not an `Err`; both fixed in the tests, not by loosening the gate).
- Manual proven-red (AC4-5): `spirits/butler/manifest.toml:21` set to `anthropic.claude-3-haiku-20240307` → `check-model-currency: FAIL — 1 finding(s) over 11 pin(s): - spirits/butler/manifest.toml:21 — RETIRED model id `anthropic.claude-3-haiku-20240307` — replacement (display-only guidance): `claude-haiku-4-5-20251001``, exit 1; reverted → `PASS (11 pins across 17 files)`, exit 0. The same plant runs in the test suite through `CARGO_BIN_EXE_xtask`.
- `cargo fmt --all --check` → 0.
- `cargo test --offline --locked -p xtask --no-fail-fast` (after the final patch) → **952 passed, 1 failed**; the one failure is `kloc_check::tests::kloc_check_runs_on_workspace` with `expected to pass: ["xtask 44362 > 44123"]` — the pending grant and nothing else. Base 941 + 12 new vectors = 953 total, so every other test is green.
- 14 named gates at the final state: `check-kernel-baseline` PASSED (25293 = pinned 25293), `check-dev-record-completeness` PASSED (165 done stories), `check-exit-commands` PASS, `check-fkcs` passed, `templates-regen --check` 0, `gen-abi-docs --check` 0, `check-manifest-schema-version` PASSED, `stability-matrix --check` PASS, `check-equiv-fixture-provenance` PASSED, `check-wasm-form-equiv` passed, `check-service-boundary` PASSED, `check-dev-model-used-populated` 0 (one WARN: this story's model is not in `KNOWN_MODELS`, a warning list, not a failure). **Red only on the two pending grants:** `kloc-check` (`xtask 44362 > 44123`) and `check-dev-model-tier` (`anthropic/claude-sonnet-5-5` not in `FRONTIER_FAMILIES`). Also green: `check-ship-gate-completeness` (all 42 expected gates), `check-model-currency`, `check-review-findings-resolved`, `check-bare-review-findings`, `check-epic-close-coherence`, `check-epic-close-green`.
- Grant scratch-proofs: see §Operator grant requested.

### Completion Notes List

**Frontier re-run (`anthropic/claude-opus-5-5`, 2026-10-06):**

- The draft was adopted, then patched. Every AC was re-verified by execution (§Debug Log References). The sweep, the
  data file, the enrolment and the price book were sound. The gate had three fail-open paths in the `main.rs` scan, one
  in the price book (`entries = []`), one schema pin form it did not read (`[providers]` `model_id`) and one fail-open
  listing path (a dangling or unreadable manifest). It also had wrong-line reporting for a duplicate id on one line,
  and a data cross-check that accepted blank or twice-retired ids. All are fixed, each with a vector that reds on the
  draft.
- The gate (`xtask/src/check_model_currency.rs`, now **312** tokei code lines) reads each `main.rs` provider
  constructor's model as its third argument (`transport, endpoint_url, model_id, …`). The argument comes from the
  call's own argument list, lexed with paren depth and string literals respected and `//` comments dropped, and it must
  be a `"…".into()`-style literal; anything else is a finding. A call after `//` on its line opens nothing.
- Operator rulings applied:
  - KLOC grant applied at the exact re-measured 44361.
  - The `allowed` set is ratified and was re-verified against the vendor pages; no id was dropped.
  - The Haiku 4.5 watch item is dated, with its trigger, in `deferred-work.md` and the data-file header.
  - The list prices are kept; re-verification found them correct.
  - The v4 docs example (en + ko) now shows `anthropic.claude-sonnet-5-5`. The gate's scope stays exactly AC3, and the
    three cookbook examples are deferred.
- `FRONTIER_FAMILIES`, `kernel-core-baseline.toml` and `abi-ratifications.toml` are untouched; `check-kernel-baseline`
  reads 25293. Nothing was committed, pushed, stashed or reset.

**Draft pass (`anthropic/claude-sonnet-5-5`; kept as history):**

- The gate (`xtask/src/check_model_currency.rs`, 313 tokei code lines) reads only repo-local files. `parse_data` refuses a data file whose guidance could lie (replacement missing, not in the provider's `allowed`, or an id both allowed and retired; unknown keys denied). `judge` returns RETIRED (with the display-only replacement), UNKNOWN model, or UNKNOWN provider. Manifest `provider.complete` is read through the TOML parser (dotted and table forms are one shape), the price book row by row, and the three `main.rs` provider constructors by argument-list scan.
- Premise corrections (§Premise corrections P1–P10) and the epic OLD→NEW block are applied; the biggest: the price book carried **three** non-current ids (one was OpenAI-deprecated, shutdown 2026-10-23), `main.rs` carries **three** provider literals (Ollama was missed), and xtask had **83 lines of headroom**, not zero.
- Sweep (7 data-line edits): `spirits/butler/manifest.toml:21`, `templates/spirit-rust/manifest.toml:12`, `examples/example-spirit/manifest.toml:12` → `anthropic.claude-haiku-4-5-20251001`; `spirits/researcher/manifest.toml:47` → `anthropic.claude-sonnet-5-5`; price book `claude-3`/`gpt-4`/`claude-3-5-sonnet` → `claude-haiku-4-5-20251001`/`gpt-4o-mini`/`claude-sonnet-5-5`. `spirits/hello-spirit/manifest.toml` untouched; template and example move together so `templates-regen --check` stays green.
- **Behaviour change to flag (review B1):** the price-book rows now match the `(provider, model)` of real journaled cost frames, so `maosctl audit cost-reconcile` (default `--pricing`) prices real traffic where the old ids matched nothing and priced it at 0. The rows therefore carry the vendors' published list prices (Anthropic haiku-4-5 $1/$5, sonnet-5-5 $2/$10 per MTok → 1000/5000 and 2000/10000 micro-USD per 1k; OpenAI gpt-4o-mini $0.15/$0.60 → 150/600), replacing the synthetic 3000/15000 and 30000/60000. Operator question Q2.
- The dev model is `anthropic/claude-sonnet-5-5`; `check-dev-model-tier` is red only because that token is not in `FRONTIER_FAMILIES` — grant request 2, not self-granted.
- Nothing was committed, pushed or stashed; kernel untouched (`check-kernel-baseline` reads 25293).

### File List

- New: `_bmad-output/implementation-artifacts/17-5-model-pin-currency-gate.md`
- New: `xtask/model-currency.toml`
- New: `xtask/src/check_model_currency.rs`
- New: `xtask/tests/model_currency_gate.rs`
- Modified: `.github/workflows/discipline.yml`
- Modified: `_bmad-output/implementation-artifacts/deferred-work.md`
- Modified: `_bmad-output/implementation-artifacts/sprint-status.yaml`
- Modified: `_bmad-output/planning-artifacts/epics/epic-17-workers-and-third-party-form-w2.md`
- Modified: `docs-site/docs/manifest/v4.md` (frontier re-run, operator ruling 5)
- Modified: `docs-site/i18n/ko/docusaurus-plugin-content-docs/current/manifest/v4.md` (frontier re-run, operator ruling 5)
- Modified: `examples/example-spirit/manifest.toml`
- Modified: `spirits/butler/manifest.toml`
- Modified: `spirits/researcher/manifest.toml`
- Modified: `templates/spirit-rust/manifest.toml`
- Modified: `xtask/gate-registry.toml`
- Modified: `xtask/kloc.toml` (frontier re-run, operator ruling 1)
- Modified: `xtask/provider-pricing.toml`
- Modified: `xtask/src/check_ship_gate_completeness.rs`
- Modified: `xtask/src/lib.rs`
- Modified: `xtask/src/main.rs`

### Change Log

- 2026-10-06 — Story created (bmad-create-story, rule-9 preflight at `0d73cac3`, vendor model facts fetched).
- 2026-10-06 — Dev pass (bmad-dev-story): gate, data, enrolment, sweep, 8 proven-red vectors; measured xtask 44040 → 44318.
- 2026-10-06 — bmad-code-review (Blind Hunter, Edge Case Hunter, Acceptance Auditor): 12 unique findings, 10 patched (+4 vectors → 12), 2 tracking items closed; final xtask 44362; status `done` with two operator grants pending.
- 2026-10-06 — Frontier re-run (operator ruling 2026-10-06; dev of record `anthropic/claude-opus-5-5`). Every AC was
  re-verified and the operator rulings applied:
  - KLOC 44123 → 44361, the exact re-measured figure.
  - The `allowed` set is ratified (re-verified, none dropped).
  - The Haiku 4.5 watch item is dated.
  - The list prices are kept (re-verified).
  - The v4 docs pin is swept (en + ko).

  A fresh §A6 bmad-code-review produced 20 unique findings: 11 patched (+8 vectors → 20, each red on the draft),
  1 deferred, 8 dismissed. `cargo test -p xtask` gave 961/0, and all 14 gates plus `check-serde-error-handling` and
  `check-ship-gate-completeness` are green on rustc 1.99.0. Status `done`.
