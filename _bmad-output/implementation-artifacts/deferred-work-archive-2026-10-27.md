# Deferred-work archive (2026-10-27)

Scope: verbatim relocation of classifier-closed deferred-work bodies from `deferred-work.md` (operator ruling 2026-10-27, Tier A). Nothing is rewritten; live obligations remain in `deferred-work.md`. Each section below carries the original heading text and the exact original body.

## ~~Daemon-side skill-admission ENFORCEMENT — filed to Epic 10 (Story 9.7 preflight 2026-06-17, John's F6b finding)~~ **RESOLVED 2026-06-19**

**RESOLVED:** The `maos run` daemon now consults persisted admission state before spirit-load. Skills in `Rejected` state block daemon startup with a typed error; `Pending` skills emit a warning. The enforcement uses `maos_cli::subcommands::admission_view` (discovery + TL reconcile + LWW decided-set) at the composition root, before any spirit is loaded.

- ~~**Gap:** an operator who `maosctl skills reject <id>` expects that skill blocked from loading.~~ **FIXED:** Rejected skills now emit `FATAL` + block daemon startup. Pending skills emit `WARNING`.
- ~~**Follow-up story (Epic 10):** the `maos run` daemon honors persisted admitted/rejected skill state at spirit-load.~~ **IMPLEMENTED** in `maos-bin/src/main.rs` (composition root), reusing `admission_view` from `maos-cli`.

## Security — A2A cross-Host (closed by Story 8.9)

- **G8 — peer-identity bypass (CRITICAL).** The live mTLS verifier learns the true peer (`crates/maos-a2a-tcp/src/verifier.rs:177` — `find_active_pin_by_fingerprint → Some(_peer)`) then discards it; `serve_connection` hands `handle_intake` the *request*, which re-derives identity from attacker-supplied `frame.from.host_id` (fallback `"loopback"`) at `crates/maos-a2a-core/src/router.rs:438-441`, and the intake `verify_pinned` compares the config fingerprint against itself (`X==X`). A mesh peer with any one validly-pinned leaf can set `frame.from.host_id="nash"` and inherit Nash's accept-allowlist — the confused-deputy J4 exists to prevent. `fix: thread the verified PeerId into handle_intake; reject frame.from != tls_verified_peer`.
- **G1 — consent granter replay (HIGH).** No check that `consent_envelope.granter == frame.from` (`router.rs:531`); a leaked envelope can be replayed by a different sender (compounds with G8). `guard: if env.granter != frame.from { return Err(ConsentGranterMismatch) }`.
- **G10 — consent-expiry dead code (HIGH).** 8.6 fixed the consent clock to fail-closed (`wall_now_ns`), but `prepare_outbound` (`router.rs:339-377`) never populates `consent_envelope.valid_until_ns`, so the `if let Some(valid_until_ns)` expiry check (`router.rs:531-534`) is unreachable on every real cross-Host frame. Production has NO consent-expiry enforcement; all "expiry" tests pass against hand-built unit envelopes.
- **G2 — expired-consent masked by ordering (MEDIUM).** `accept_admits` runs before `is_expired` (`router.rs:517` before `:531`); an expired+denied frame reports intent-denial, masking expiry. Becomes a wrong-root-cause incident timeline once G10 is fixed.
- **G9 — intent length-bound contradiction (MEDIUM).** Router accepts `intent.len() <= 1024` (`router.rs:260`) but canonical caps at `MAX_CANONICAL_INTENT_LEN = 128` (`crates/maos-domain/src/invariants/i8.rs:44`); a 129–1024-byte intent is used as the match key yet can never be canonical → guaranteed non-match + warn-dedup amplification.
- **G4/G5/G6 (MEDIUM/LOW).** TOFU pin-store restart-TOCTOU + duplicate-peer silent overwrite (`router.rs:139-145,:474-514`); brittle webpki/io `Debug`-string error classification that can misclassify on dep wording drift (`verifier.rs:202`, `mtls.rs:63`); intake timeout covers only the socket read, not `handle_intake` + NACK write (`transport.rs:403,:433,:437`).

## Invariant enforcement holes (closed by Story 8.10)

- **I11 citer-authorization gap.** `write_distillate(spirit_pid, request)` (`crates/maos-iac/src/adapter/distillate.rs:289-369`) verifies cited frames EXIST but never that `spirit_pid` is authorized to cite them; Researcher forwards a caller-supplied pid (`spirits/researcher/src/lib.rs:528,533`). A Spirit can mint a digest whose lineage derives from another principal's raw frames. Compounded: `TransparencyLogAdapter::insert_frame_event` is `pub` and its doc admits direct `FrameKind::Distillate` inserts bypass the canonical producer (`transparency_log.rs:58`). `fix: gate insert_frame_event for Distillate to DistillateWriter; add principal-namespace citer check`.
- **I12 records nothing (silent fail-open).** `decision_logger.rs` v0.3-β `digest_provider` returns an empty refs set; `frame_carries_i12_refs` returns `true` unconditionally (doc: "always-true; production SHOULD NOT branch"). NFR-Aud-5's "100%" is satisfied by shape, not content. `fix: wire the Memory-Manager digest source-of-truth (Story 4.3's deferred seam)`.
- **Observer watchdog fails-open on NaN.** `spirits/observer/src/lib.rs:460,521` silently `return None` on NaN drift/structural magnitude → the safety watchdog goes quiet exactly when a Spirit emits garbage; `WatchThreshold::new` accepts NaN/Inf. `fix: reject-and-flag`.
- **pub-field constructor bypass class.** `anomaly_flagged` / `EpistemicHaltPayload` / `StructuralSignal` / `DistillationRequest` allow NaN/empty via struct-literal, skipping the validating `new()`.

## Story-correctness (closed by Story 8.10·AC1)

- **Butler AC2 — production halt never fires (regression of a `done` story).** `spirits/butler/src/lib.rs:250-270` production `on_idle` computes the assessment and drops it — no scalar write, no halt. The scalar→policy→halt path runs only in `tests/corpus_halt.rs`. The 8.1 review marked the `[Patch]` "Must fix — test-only proof insufficient for v0.3" as applied `[x]`; the hook code was never changed.

## Journey-presentability gaps (closed by Stories 8.11–8.14; asserted by harness 8.15)

- **No live LLM path** — zero reference Spirit calls the Inference Port; all cognition is deterministic (→ 8.11).
- **No runnable daemon** — `maos-bin` is 54 env-gated smoke arms, no serving loop (→ 8.11 + 8.14a).
- **CliWrapper subprocess bridge unbuilt** — `crates/maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs` is `argv_prefix_hash` under a doc comment; Worker spawns no real CLI (→ 8.12).
- **Live transport carries no Spirit traffic** — `smoke_a2a_tcp_8_6`'s `mira`/`nash` locals are router handles, not the cognitive structs; Mira/Nash cognition runs only on loopback (→ 8.13).
- **MCP is fixture-replay** — `spirits/butler/src/lib.rs:76-77`; no real Calendar/Slack/arXiv drivers (→ 8.14b/c).
- **`hello-spirit` is 0 LOC; no `maos init` / shell / `maos audit query`** (→ 8.14a).

## CLOSED in this round (example-spirit-ts-tests, now GREEN)

- Generated + committed `package-lock.json` for `sdks/spirit-ts` and `examples/example-spirit-ts`.
- Fixed `sdks/spirit-ts` compile errors: `ctx.ts` import `../spirit.js`→`./spirit.js`; type-only
  re-exports → `export type` (index.ts, spirit_test/types.ts); re-export `MockCtx` from spirit_test.
- `examples/example-spirit-ts` dep `@maos/spirit-ts: "^0.5.0"` → `file:../../sdks/spirit-ts`.

## ~~NEW STORY NEEDED — rebuild J4/J6 real `kernel_measurement` harnesses (maos-bench)~~ CLOSED

> **CLOSED by Story 10.4c** (2026-06-24):
> - **J4 (`harness/j4.rs::run_j4_kernel`) → DONE.** Real in-kernel `scalar.tap` measurement rebuilt from the verified template (`scalar_tap_subscriber.rs:24-77`). Gate renamed `check-j4-placeholder-red` → `check-j4-latency`. P95=1µs at HEAD (well within 10ms budget). Falsifiability proven: `bench-fault-inject` feature injects ≥15ms delay → P95 crosses 10000µs → RED.
> - **J6 (`harness/j6.rs::run_j6_kernel`) → CUT, FF-J6-guarded.** J6 cold-start latency harness is CUT from 10.4c — §13.1 declares it non-binding ("correctness gate dominates") and it is out of v1.5 scope; it is revived only when a J6 latency assertion or user-facing J6 latency claim is introduced, which CI guard FF-J6 (`xtask check-ff-j6`) blocks until the harness is rebuilt. `run_j6_kernel` now returns `JourneyResult::not_measured("J6")` instead of a plausible fake number. `mira`/`nash` dev-deps and the `FrameOrigin` path fix were NOT pulled in.

## FIXED in this round (nfr-perf gates now build + RUN)

- `orchestrator_fanout_nfr_perf_8.rs`: (a) `handle.recv()` now yields
  `(FrameKind, IacFrame)` — destructure `(_kind, frame)`; (b) removed the
  criterion `bench_function` wrapper (it sampled a 15s sustained-load op → 1
  sample → criterion `slice.len() > 1` panic) in favor of a plain `main`
  (`harness = false` already set) that runs the fan-out once and emits the
  JourneyResult report. nfr-perf-8 → exit 0, p99≈147µs ≪ 500ms, 0 dropped.

## ✅ RESOLVED — both `if: false` gates RETIRED per ADR-043 (no longer parked)

Story 8.16 (Epic 8→9 readiness bridge, retro §A1) closed this entry. Per the
retro ruling, `if: false` is not an acceptable terminal state — each gate was
RETIRED with a ratified ADR (ADR-043), not re-parked. The new `check-epic-close-green`
gate now hard-fails on ANY `if: false` workflow job, so this failure mode cannot recur.

1. **`smoke-spirit-author-7-1`** — **RETIRED** (ADR-043 Decision 2). Advisory
   spirit-authoring smoke broken by Epic-7 template bit-rot. Spirit-authoring stays
   covered by `example-spirit-tests` / `example-spirit-drift` / `example-spirit-ts-tests`
   / `spirit-test-tests` (all live). A future template-repair story may re-introduce
   a fixed authoring smoke. Job removed from discipline.yml; `if: false` deleted.

2. **`check-epic-6-bridge`** — **RETIRED** (ADR-043 Decision 1). Legacy Epic-6
   debt beacon whose §A2/§A3/§A5/§A6 enforcement migrated to live hard-fail gates
   in Story 7.1.5; its last red (`A4-Debt-1`) was a stale entry-counting predicate
   (i9-whitelist.toml + i9-exemptions.md both exist; I9 enforced live by
   check-empty-kernel + check-service-boundary). Job removed from discipline.yml;
   `if: false` deleted; xtask module kept in tree as archived history.

Aggregate `needs:`/`report-aggregate` updated to drop both retired jobs (no dangling
`needs.<job>.result`); the `²` DISABLED footnote removed. `check-kernel-baseline` (§A4)
and `check-epic-close-green` (§A5) added. YAML re-validated.

## ~~Deferred from: code review of 9-2b-journal-export-trajectory-and-deterministic-trace-shape-replay (2026-06-13)~~ **RESOLVED BEFORE COMPLETION**

- ~~**resolve_spirit_pid may return multiple matches; export silently uses the first** [crates/maos-cli/src/subcommands.rs:1625-1630]~~ — **FIXED 2026-06-13**: `audit_trajectory_export` and `audit_sealed_export` now fail with exit code 2 and a clear disambiguation message when `resolve_spirit_pid` returns more than one `(boot_nonce, spirit_pid)` pair.
- ~~**Unmapped kind string in filter.kind is silently dropped** [crates/maos-audit/src/lib.rs:302-307]~~ — **FIXED 2026-06-13**: both `query()` and `query_with_redaction()` now return `AuditError::UnknownKind` for unmapped `filter.kind` strings instead of silently omitting the filter.
- ~~**SQLite numeric casts can silently wrap/truncate** [crates/maos-audit/src/lib.rs:349-352]~~ — **PARTIALLY FIXED 2026-06-13**: binding casts for `spirit_pid` and `limit` now use `i64::try_from` with `AuditError::ValueOverflow`. Row-extraction `as` casts left unchanged because the kernel stores u64 values (including values > `i64::MAX`) via bit-cast in SQLite's signed INTEGER column; changing them to `try_from` would break round-trip of legitimate TL rows.

## Deferred from: code review of story-10.2 (2026-06-21) — RESOLVED pre-completion

Both items below were resolved during story 10.2 completion (per Lunarpulse).
Extracted to `xtask/src/gate_common.rs` and applied to all 4 gate modules:

- ✅ **Date validation** — now uses `chrono::NaiveDate::parse_from_str("%Y-%m-%d")` in `gate_common::validate_dates`; rejects impossible dates (`2026-99-99`), enforces `start <= end` ordering.
- ✅ **`--json` mode workflow commands** — `gate_common::emit_command` documents the stderr/stdout split; structured warning/error fields in the JSON payload (`advisory`, `failures`, `consistency_ok`) let programmatic consumers assert on JSON, not stderr.

## Outcome

`check-multi-region-slo` is **`oracle_green: true`**, all five legs green, `roundtrip-slo` `passed: 2` (live + mutation). Clean p95 **17 802 µs / 17 866 µs** across two runs against the untouched **30 000 µs** floor. One test file changed; zero production lines.

## What was actually wrong: the probe counted a verification step that had MOVED

`verify_replication_bundle` is **7 137 µs** per call in this debug `cargo test` build. That single fact dominates everything — component attribution for one leg (p50 µs):

| component | µs |
|---|---|
| `write_with_source` | 564 |
| `read_all_rows_from` + `from_row` | 193 |
| `build_replication_bundle` (Merkle + Ed25519 **sign**) | 230 |
| **`verify_replication_bundle` (1 Ed25519 verify)** | **7 137** |
| `apply_replication_bundle` (internal verify + 1 write) | 7 956 |
| `apply` **minus** its internal verify | **819** |

Four verifies per round trip × 7 137 = 28 548 µs of a 31 596 µs modelled round trip — **~90%**; modelled 31 596 vs measured 31 059, so the model closes. The 7 ms is the signature verification itself, not key derivation: `derive_region_pubkey` measures **102 µs** (HKDF alone 19 µs).

**Root cause.** When this probe was written at 11.2b, `apply_replication_bundle` did **not** verify, so an explicit `verify_replication_bundle` before `apply` was genuinely part of the production path. **Story 13.2 moved verification INSIDE `apply`** (`bundle.rs:866` — before any store access, the Fork-4 payoff). The probe was never updated, so from 13.2 onward it performed **two verifies per leg where production performs one**. Production's sole caller of `apply_replication_bundle` — `crates/maos-bin/src/cross_team_crossing.rs:255` — calls it directly with no pre-verify.

So the apparent **1.94×** regression was **~85% probe infidelity**. With the redundant verify removed, HEAD measures **17 802 µs against 11.2b's 16 535 µs = 1.08×** — ~8% genuine machinery growth over five stories, which is a healthy tripwire margin, not a regression. **Coverage is unchanged**: a bundle that fails verification makes `apply` return `SignatureVerificationFailed` and the `.expect` panics.

## A hypothesis that measurement killed — recorded because the reasoning was persuasive and wrong

The 5 → 17 SQL-statement expansion on the write path (`store.rs:619-708`: `BEGIN` + `pg_advisory_xact_lock` + erasure-tombstone `SELECT` + 18-column upsert + `COMMIT`) was the leading explanation, with arithmetic that appeared to fit. A probe harness measured it per-write (p50 µs, N=200):

| variant | p50 | vs baseline |
|---|---|---|
| V0 autocommit, unprepared (11.2b shape) | 141 | 1.00× |
| V1 full path, **unprepared** (HEAD shape) | 380 | **2.70×** |
| V2 full path, **`prepare_cached`** | 178 | 1.26× |
| V3 no advisory lock | 143 | 1.01× |
| V4 no tombstone `SELECT` | 143 | 1.01× |

**The controls are nearly free (~35 µs each); statement PARSING was the entire write-path cost.** But 3 writes × 239 µs = 0.7 ms against a 15.3 ms round-trip delta — it explained **under 5%** of the regression, which is what redirected the investigation to the verify count. *The arithmetic that "fit" fit because two wrong numbers were multiplied.*

## Also landed (required by the governing rule regardless of disposition)

The diagnostic `eprintln!` in **both** `cross_region_roundtrip_live` and `..._mutation` moved **above** their asserts. Every assert panics, so emitting the distribution afterwards meant a RED run reported p95 and nothing else — the failure's shape was unmeasurable by construction, which is why "is this jitter?" stayed an opinion for two days. The distribution is what settled it: std_dev 499 µs and p95−p50 = 1 005 µs around a p50 that was *itself* over the floor — a uniform shift, not a tail. Both gate legs already pass `--nocapture`, so this surfaces in CI with no workflow change.

## ~~The RED that closing `check_fkcs.rs:325` uncovered — HELD ADVISORY, not fixed, not re-pinned~~ — CLOSED 2026-09-29 by decision D8

- **Resolution (2026-09-29, decision D8, operator-directed):** Story 13.4's `148a33ee` was reviewed CONFORMANT against ADR-052 §3 by two non-author reviews; the pre-pin fixes (one shared `frozen_surface_gate` called first by both entry points; three stale doc comments) landed in the pinned bytes; `xtask/fkcs-baseline.toml` re-pinned `dfbbf748…` → `650f13b9…`; the hold machinery is deleted from `xtask/src/check_fkcs.rs`; `check-fkcs --json` → `oracle_green: true`. Evidence and the two routed findings: `_bmad-output/planning-artifacts/epics/epic-14-preflight-decisions.md` §D8 ruling. Rows below are history.

- **`check-fkcs`'s `admission-path-unmodified` leg has been RED since Story 13.4** [`xtask/fkcs-baseline.toml:22`, `crates/maos-registry/src/admission.rs`, `crates/maos-skill/src/admission.rs`] — 13.4 (FR37 vetting machinery, `148a33ee`) changed both admission sources and never re-pinned `admission_baseline.sha256`; the last re-pin was `5767cf0d` (Story 12.6). The gate's exit tested `blocking_now` while its JSON `passed` field tested `dev_blocks`, so it printed `"passed": false` and exited **0** — the one-identifier defect Story 13.6e's AC2 named. **Measured both ways at 13.6e:** with the original identifier the gate exits 0 over the same RED leg; with the fix it exits 1.
  - **The harness is correct.** The admission path genuinely is not unmodified. Re-pinning here would re-can a fixture over another story's unreviewed admission-path change, which the governing rule (`13-6c…md:154`) forbids: *"Never re-can a fixture, never silently relax a floor."*
  - **Disposition:** held advisory with a loud banner, a named owner and this entry. The hold is bound to the exact recorded baseline SHA-256 and current Story-13.4 worktree SHA-256; malformed baseline data, unreadable sources, or any later admission drift blocks instead of inheriting the hold. `check-fkcs` emits a `WOULD HAVE BLOCKED` banner and reports `held_advisory_legs` in JSON. The gate-level `advisory` field now matches its actual verdict.
  - **Historical — D8 CLOSED 2026-09-29; `check-fkcs` holds nothing, so this line routes nothing.** It once read "Owner: 20-3a-gate-honesty", handed by the Epic-13 retrospective §4 (2026-08-11): 14.3 owns FKCS infrastructure readiness and must decide the conformant re-pin or retain the exact bounded hold. *Dispositioned by Story 13.6 / AC5, 2026-08-08 (mechanical stale-owner sweep).* The original record read “with Story 11.5 (FKCS infrastructure)”; Story 11.5 is `done`, so this residual was STALE AT BIRTH — authored inside the story that built the judge, naming a completed owner on the day it was written. The FKCS infrastructure it depends on is 11.5's shipped output, not a live owner. The decision is whether 13.4's admission-path change is frozen-kernel-conformant; if it is, re-pin `admission_baseline.sha256` and delete the hold entry — the gate then blocks on the leg again with no code change.

## ~~Deferred from: Story 17-6 preflight round-table (2026-09-26)~~ — CLOSED by Story 17-3b-wasm-form-admission-and-contract (2026-09-29)

*Resolution (17-3b, R17-49):* the cookbook now teaches third parties the `wasm-component` form (`cookbook/wasm-component-spirit.md`, en + ko); `hello-world-spirit.md` is re-framed as in-tree first-party authoring with an admissible manifest; the `manifest-fields.md` reference block and `compliance-claim.md`'s tier now pass admission; `crates/maos-bin/tests/cookbook_manifests_17_3b.rs` runs every whole-manifest block through `gate_manifest` and asserts ko blocks byte-identical (superseding `cookbook_sections_17_6.rs`). `cli-wrapper-spirit.md` stays 17-1 AC5's.

- **The cookbook teaches third parties the first-party `rust-inproc` form, and its reference block documents fields the parser refuses** [`docs-site/docs/cookbook/hello-world-spirit.md:24-48`, `docs-site/docs/cookbook/manifest-fields.md:17-155`, ko twins under `docs-site/i18n/ko/docusaurus-plugin-content-docs/current/cookbook/`] — the hello-world page walks an outside author through `forms = ["rust-inproc"]`, which ADR-060 clause 1 reserves for code compiled into the daemon; the 138-line reference block also lists `trust_tier = "local" # local | community | audited`, posture and budget fields that were never checked against the parser [INFERENCE — only `[sandbox]` and `[resources]` were measured refused]. 17-6 AC6 fixes exactly `[sandbox]` + `[resources]` in both blocks and adds a test that parses those two sections (RT-8). *Reason deferred:* the rest is a lesson about the third-party form, which does not exist until 17-3b ships it (epic R17-49). **Owner: `17-3b-wasm-form-admission-and-contract`** — rewrite the authoring cookbook around `wasm-component` and make the whole reference block parse (en + ko), extending 17-6's cookbook test to every section.

## ~~The FKCS off-surface parse is line-based~~ — RESOLVED by Story 17-3b-wasm-form-admission-and-contract (2026-09-29): `frozen_surface_gate` reads `[fkcs]` and `[class]` through one structural `maos_manifest::ManifestDocument` parse; multi-line arrays, header comments and dotted keys are refused (`crates/maos-registry/tests/form_admission_17_3b.rs`); FKCS baseline re-pinned `62d99cfa…`.

- **The FKCS off-surface parse is line-based** [`crates/maos-registry/src/admission.rs` `extract_fkcs_internal_references` / `parse_manifest_string_array`] — a multi-line `internal_references = [` array, a `[fkcs] # comment` header or a dotted `fkcs.internal_references` key is not seen, so the declaration admits. Pre-existing since Story 11.5 (ADR-052 amendment 2026-09-29). **Owner: `17-3b-wasm-form-admission-and-contract`** (AC4) — it rewrites this prelude with a structural manifest reader and re-pins its own reviewed diff; vectors for each evasion shape.

## CLOSED by Story 17-7 — terminal facts and startup identity

- **CLOSED — authoritative T3 signal/OOM facts replace runtime-client status.** The payload's verified pidfd status is authoritative; bounded runtime OOM evidence labels only the ruled SIGKILL/unknown-137 cases. One immutable terminal snapshot feeds both exit classifiers before `task.orphaned`; unavailable evidence remains Truncated, never a fabricated clean exit. **Owner: `17-7-t3-worker-retained-lifecycle-and-terminal-facts`** (original routing: operator 2026-10-11, D-17-2-FW-07). Closure evidence: final run [38708067558](https://github.com/lunarpulse/maos/actions/runs/38708067558), S2/S3/S4 on both Ubuntu x64 images and domain/observer/finalizer regression coverage; exact source and red-control assertions in `17-7-evidence/review-ci.json`. Founder approval exception D-17-7-FW-10 satisfies the remaining signature gate.
- **CLOSED — short-lived T3 Workers no longer depend on post-exit PID inspection or `--rm`.** Runtime-owned `create` → CID receipt → `init` → held PID receipt/pidfd → `start --attach` captures startup identity before execution; explicit bounded teardown/removal is by owned CID, never the requested name. **Owner: `17-7-t3-worker-retained-lifecycle-and-terminal-facts`** (original routing: operator 2026-10-11, D-17-2-FW-07; former proposed owner 17-2). Closure evidence: final run [38708067558](https://github.com/lunarpulse/maos/actions/runs/38708067558), immediate-exit S1/S2 and real CID/name-reuse coverage on both Ubuntu images, with independent re-added-`--rm` and remove-by-name red controls; archive `17-7-evidence/review-ci.json`.

## CLOSED by Story 17-7 — unload failure cleanup

- **CLOSED — a failing or over-budget `WorkerSpirit::on_unload` cannot bypass registry-owned T3 stop.** Registry fallback after unload shares CID custody and bounded teardown with the bridge and root shutdown; the hook's stop is an idempotent fast path, not the sole cleanup authority. **Owner: `17-7-t3-worker-retained-lifecycle-and-terminal-facts`** (original rule-10(b) routing 2026-10-11 from the closed 17-1 hook-failure row). Closure evidence: final run [38708067558](https://github.com/lunarpulse/maos/actions/runs/38708067558), real hook panic/timeout S5 and post-unload CID removal on both Ubuntu images; actual unload timing is inside the bound. Exact commands/assertions are archived in `17-7-evidence/review-ci.json`.

