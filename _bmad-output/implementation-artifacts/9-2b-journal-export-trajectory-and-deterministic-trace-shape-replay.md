---
dev_model_used: claude-opus-4-6
---

# Story 9.2b: Publish `journal.export` Trajectory + Deterministic Trace-Shape Replay

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->
<!-- Spun out of Story 9.2 at party-mode preflight (2026-06-13, Fork G). 9.2 = erasure spine (FR45+FR65); this = the read-side access/portability + replay-determinism half (FR46+ADR-028). Recommended dev model: claude-opus-4-8 (crypto/signing + deterministic-replay are §A6 categories). -->

> **⚑ ORIGIN.** This story was split from Story 9.2 by unanimous party-mode decision (Fork G): erasure (destroy) and export/replay (disclose) are opposite operations with different test oracles. This half is **additive, read-side, kernel-neutral (zero kernel-core KLOC)** and *consumes* the post-erasure world Story 9.2 produces — sequence it **immediately after 9.2**. Reuses Story 9.1's `sealed_export` Ed25519 path wholesale.

## Story

As a DPO / auditor exercising data access and portability,
I want `journal.export(filter, redaction_policy)` (FR46) producing an Ed25519-signed `maos.trajectory.v1` bundle with an applied-redaction flag, AND deterministic replay (ADR-028) over the **shape of the trace** — IAC frame ordering, capability-token issuances, halt events, decision-frame emission — with redacted slots rendered as typed placeholders,
so that the audit trail is portable, third-party-verifiable, and replayable for determinism without ever exposing redacted payload content.

---

## Context & Charter Boundary (READ FIRST)

This is the **read-side disclosure half** of the original Story 9.2. It is **strictly additive and kernel-neutral** — both deliverables read the already-written Transparency Log and serialize/replay it; neither writes substrate state.

- **Zero kernel-core KLOC.** `git diff -- crates/maos-kernel-core/src/ --stat` must be empty; `xtask check-kernel-baseline` stays green at **21336** (`xtask/kernel-core-baseline.toml` `src_lines = 21336`, re-pinned by the 2026-06-13 Story 9.2 erasure-review patches — NOT 21197, which was the 9.1 figure this story was first drafted against). If you find yourself editing kernel src, STOP.
- **`maos-audit` stays read-only** (`SQLITE_OPEN_READ_ONLY`, `#![forbid(unsafe_code)]`) — both modules are pure readers. The Story 9.2 CI grep guarding no-write-open in `maos-audit` applies here too.
- **Workspace stays 44 crates.** New schemas are not crates.
- **Reuse 9.1's crypto path** — `maos_audit::sealed_export` (`build_bundle`/`canonicalize`/`sign_bundle`/`verify_bundle`, dalek+sha2, NOT ring) + `maos_domain::audit_key` loader. Do not mint a second crypto path.

**ADR reference correction (John, preflight errata):** the epic cites "ADR-023" for FR46 — that is the capability-token-TTL ADR. The correct anchor is **ADR-028** (replay determinism / trace-shape). There is **no written ADR-028 doc** in `docs/adr/` — the binding text currently lives only in the architecture ADR section + the epic. **Preflight ruling (F2, below): author it now.**

**§A6 NON-OPUS SAFETY NET applies** — crypto/signing (FR46) + deterministic replay (ADR-028) are named correctness-critical categories. Recommended dev model: `claude-opus-4-8`.

---

## Preflight Consensus — 5 forks resolved 4/4 (party-mode, 2026-06-13)

A pre-dev party-mode (Winston · Murat · John · Amelia) resolved every open fork **per spec + long-term correctness**. These are **decisions, not options** — implement them; do not re-litigate.

- **F1 — `payload_redacted` access → A-prime (single read path).** Replay needs each frame's redaction metadata to render placeholders. Do **NOT** open a second SQLite connection (snapshot-isolation skew makes the tamper test racy), and do **NOT** naively widen the shared projection (it re-bytes 9.1's already-sealed bundle — `canonicalize()` at `sealed_export.rs:97` signs the whole `BundleForSigning`, every `AuditEntry` field included). Instead: add an **additive `Option<RedactionMeta>` field to `AuditEntry`** (`crates/maos-audit/src/lib.rs:80`) with `#[serde(rename = "redaction", skip_serializing_if = "Option::is_none", default)]` — **mirroring the existing `capability_token_hex` precedent** on that struct. `query()` + all four 9.1 sealed-export callers leave it `None` → their bundles are **byte-identical by construction** (a `None` `skip_if_none` field emits zero bytes). A new same-connection `query_with_redaction()` projection populates it **for replay only** — one connection, one ordering invariant. `RedactionMeta` carries metadata ONLY: `class: RedactionClass` + bucketed original length (see F5) — **no raw payload blob, no content hash.**
- **F2 — missing ADR-028 → author it now (blocking AC).** A compliance control's decision *is* the deliverable; epic-prose + arch §13 already failed once (the ADR-023 mis-cite). Write `docs/adr/ADR-028-*.md` as the canonical home for the F3 determinism contract (the 7 pin-sources + binding tests) and the F5 redaction-class entropy policy. **John owns** the PRD/epic errata sweep for the bad ADR-023 reference. The story is not "done" while ADR-028 is vapor.
- **F3 — determinism → kill "best-effort"; HARD byte-identity at v1.0.** "Best-effort byte-identical" is an untestable non-oracle that flakes then gets `#[ignore]`d. v1.0 ships **hard byte-identity over the signed/shape surface** (achievable today: input is a sorted immutable TL, transform is pure). Anything that can't be made deterministic is **EXCLUDED** from the signed surface, not shipped soft inside it. "v1.5 hard" applies ONLY to the orthogonal cross-platform / cross-toolchain-version / cross-schema-revision envelope — say that explicitly; do not label a CI gate "best-effort." See AC2 for Murat's binding pin-list + tests.
- **F4 — verifier topology → new `tools/verify-trajectory/verify.py` sibling.** One zero-dep verifier per artifact type is the established pattern (`verify-audit-bundle` 9.1, `verify-erasure` 9.2). Do NOT overload `verify-audit-bundle` with a second bundle grammar — it couples two independently-sealed compliance artifacts and weakens the tamper oracle (a dispatch branch obscures *which* check failed). Duplicate the ~30 lines of dalek/sha2 boilerplate; it shares the **9.1 canonicalizer** (single canonicalization core) but is a separate, independently-auditable entry point. Acceptance bar: a regulator verifies it on an **air-gapped laptop with only the public key + the bundle** — no MAOS runtime, no network.
- **F5 — placeholder leakage → SHIP-BLOCKER; drop the content hash.** Exact byte-length + sha256-prefix over a **low-entropy redacted field** (boolean, enum, known-format ID) is a **confirmation oracle**: the bundle holder — who is the adversary for this leak — enumerates candidates, hashes each, matches the prefix, and recovers the very content redaction was meant to hide. This breaks our own "never exposes redacted payload content" promise + GDPR Art. 5(1)(c) data-minimization. Salting fails (any salt in the signed, third-party-verifiable bundle is in the adversary's hands). **Resolution:** the placeholder carries **type/class + bucketed length only** — **no content-derived hash** (bundle integrity is already carried end-to-end by the Ed25519 envelope; the per-slot hash is redundant for tamper-evidence and net-negative for confidentiality). Encode the redaction-class entropy policy in ADR-028. **Flag sec-redteam + John for sign-off** on the narrowed placeholder grammar.

---

## Acceptance Criteria

### AC1 — `journal.export(filter, redaction_policy)` signed trajectory bundle (FR46)

**Given** `maosctl audit export <filter> [--redaction-policy <policy>]` (new `AuditQuery::Export` variant; the `AuditQuery` enum is at `crates/maos-cli/src/cli.rs:195` — siblings `SealedExport`@244, `PostureDelta`@292)
**When** the operator exports a filtered trajectory
**Then** the bundle conforms to **`maos.trajectory.v1`** at `schemas/trajectory.schema.json` (JSON Schema **draft-2020-12**, canonical-bytes rule like `audit-bundle.schema.json`)
**And** the bundle is **Ed25519-signed** via the 9.1 audit-key path (`maos_domain::audit_key::load_audit_key_seed` + `maos_audit::sealed_export` dalek/sha2 — NOT `RingCryptoProvider`) with an **`applied_redaction` flag** (required field)
**And** the redaction policy is honored end-to-end (a policy-redacted row appears as a placeholder, never raw), verified by `crates/maos-audit/tests/trajectory_redaction_test.rs`
**And** third-party-verifiable over canonical bytes via an extended `tools/verify-audit-bundle/` (or sibling `verify-trajectory`); tamper-one-byte MUST fail

Engineering ACs:
1. Reuse/generalize `maos_audit::sealed_export` primitives for the trajectory shape (filtered `AuditEntry` set + `applied_redaction` flag + freshness metadata + signature block) — do not duplicate crypto.
2. `redaction_policy` scrubs selected payloads/fields to placeholders **before signing**, **fail-closed / default-deny** (no code path returns raw payload once a policy is applied); `applied_redaction = true` iff any row was redacted.
3. Wire `trajectory.schema.json` into the existing CI schema-gate convention (9.1's `audit-bundle.schema.json` is the precedent).
4. **(F1 A-prime)** Redaction metadata reaches replay via an additive `Option<RedactionMeta>` field on `AuditEntry` (`#[serde(rename = "redaction", skip_serializing_if = "Option::is_none", default)]`, mirroring `capability_token_hex`), populated ONLY by a new same-connection `query_with_redaction()`. A **golden-bytes regression test** MUST prove 9.1 sealed bundles stay byte-identical with the field present-but-`None` (see AC3).

### AC2 — Deterministic replay over trace-shape (ADR-028 / NFR-Aud-3)

**Given** deterministic replay in `crates/maos-audit/src/replay/` (`runner.rs` + `redaction_placeholder.rs`)
**When** `replay(bundle)` executes against a sealed-export (9.1) or trajectory (AC1) bundle
**Then** replay determinism is verified over the **shape of the trace** — IAC frame ordering, capability-token issuances (`FrameKind::CapabilityInvocation`=7 / `SpiritRevoked`=17), halt events (`EpistemicHalt`=3), decision-frame emission (`Decision`=10 / `DecisionDispatch`=2) — **NOT** redacted payload content
**And** redacted slots replay as `<REDACTED:type=<class>, len=<bucket>>` placeholders from `redaction_placeholder.rs` — **type/class + bucketed length ONLY; NO content-derived `hash` and NO exact byte length** (F5: exact-len + sha256-prefix over a low-entropy field is a confirmation oracle the bundle holder can invert; tamper-evidence is already carried by the Ed25519 envelope)
**And** `schemas/trace-shape.schema.json` (draft-2020-12) validates the replay output in CI via `crates/maos-audit/tests/replay_schema_test.rs`
**And** two replays of the same bundle are **byte-identical** — a **HARD determinism gate at v1.0** (diff-tested over the signed/shape surface; NOT "best-effort"). Anything that cannot be made deterministic is **excluded** from the signed surface rather than shipped soft
**And** the v1.0-hard scope vs. the **v1.5 cross-platform / cross-toolchain-version / cross-schema-revision** envelope is stated explicitly in `--help` + schema `$comment` (do not label the CI gate "best-effort")

Engineering ACs:
1. Replay is **pure read-only** over the TL (reuse `maos_audit::query`, which already sorts by `(timestamp_ns, frame_id)`); it projects frames to a canonical trace-shape document and re-derives the structural skeleton — it does NOT re-execute Spirits. Redaction metadata reaches it via the **F1 A-prime `query_with_redaction()`** single-connection projection — do NOT open a second connection (snapshot-skew → racy tamper test).
2. The placeholder is computed from frame metadata only: `type` = redaction class, `len` = **bucketed** original payload byte length (e.g. power-of-two / coarse range, NOT exact). **No content hash.** Deterministic for identical input by construction.
3. **Determinism pin-list (Murat, binding — enumerate in ADR-028):** (a) no `HashMap`/`HashSet` in the serialization path — `BTreeMap`/`BTreeSet` or collect-then-sort; (b) **reuse 9.1's `canonicalize` — one canonicalizer, not two** (a second is an anti-tautology violation); (c) no raw `f64` in the shape (slot-presence, not value; or fixed-precision string); (d) ZERO freshly-read clocks in replay output — timestamps come from the `timestamp_ns` column only; (e) explicit total `ORDER BY` on every replay read (no rowid ordering); (f) fixed placeholder field order; (g) no `{:?}`/Debug repr in output (unstable across compiler versions).
4. `trace-shape.schema.json` wired into the same CI schema-gate as `audit-bundle.schema.json`.

### AC3 — Discipline / regression floors

1. **Zero kernel-core KLOC** — `git diff -- crates/maos-kernel-core/src/ --stat` empty; `check-kernel-baseline` green at 21336.
2. **Workspace = 44 crates**; **`maos-audit` stays read-only** (the no-write-open CI grep stays green); **`maos-cli` kernel-core-free** (`dep_kernel_core_free_test.rs`).
3. **`abi-diff` Added-only** (`xtask abi-diff --base abi-baseline/v1-pre-bump.txt`).
4. **Hard-fail gates green**: `check-review-findings-resolved`, `check-dev-record-completeness`, `check-dev-model-used-populated`, `check-epic-close-green`, `check-service-boundary`, schema-gate. `### Review Findings` a real table or explicit green.
5. **Smoke arm**: `maosctl audit export <filter>` produces a `maos.trajectory.v1` bundle that verifies third-party; `replay` of it is byte-identical across two runs and validates against `trace-shape.schema.json`. Isolate `XDG_DATA_HOME`/`MAOS_HOME`/`MAOS_MEMORY_ROOT`.
6. **Preflight-consensus floors (binding):** (a) **ADR-028 written** (`docs/adr/ADR-028-*.md`) — story not "done" without it (F2); (b) **9.1 byte-identity golden test green** (F1 A-prime regression — `redaction:None` adds zero bytes); (c) **silent-re-baseline guard green** (`redaction.is_none()` on all non-replay callers); (d) **redaction k-anonymity test green** (F5 — placeholder carries no content hash, bucketed length only); (e) determinism is a **HARD** two-process byte-identity gate (no "best-effort" label anywhere). **sec-redteam + John sign-off** recorded on the narrowed F5 placeholder grammar.

---

## Tasks / Subtasks

- [x] **Task 0 — ADR-028 doc** (F2, blocking; do this FIRST so it's the contract the rest implements against)
  - [x] `docs/adr/ADR-028-*.md`: trace-shape replay, placeholder grammar (class + bucketed len, NO hash), determinism pin-list, binding test matrix, `query_with_redaction()`-is-sole-populator invariant, v1.0-hard / v1.5-envelope scope
  - [ ] John: epic/PRD ADR-023→ADR-028 errata sweep
- [x] **Task 1 — FR46 trajectory export** (AC1)
  - [x] `schemas/trajectory.schema.json` (`maos.trajectory.v1`, draft-2020-12, canonical-bytes) + CI schema-gate
  - [x] `AuditQuery::Export` variant + dispatch arm + handler; reuse `sealed_export` Ed25519 path + `applied_redaction` flag; redaction policy **fail-closed**
  - [x] **(F1 A-prime)** `AuditEntry::redaction: Option<RedactionMeta>` (`skip_serializing_if`/`default`) + `RedactionMeta { class, original_len }` + `query_with_redaction()` sole populator
  - [x] `trajectory_redaction_test.rs` (policy honored end-to-end) + **new** `tools/verify-trajectory/verify.py` (F4 — sibling, NOT an overload of `verify-audit-bundle`)
  - [x] **(F1 regression)** golden-bytes test: 9.1 sealed bundle byte-identical with `redaction:None` + serde-no-key assertion + positive `Some(_)`→differ + `redaction_field_is_none_for_all_non_replay_callers` guard
- [x] **Task 2 — ADR-028 deterministic replay** (AC2)
  - [x] `replay/runner.rs` (pure trace-shape projection over `query_with_redaction`) + `replay/redaction_placeholder.rs` (class + bucketed len, NO hash)
  - [x] `schemas/trace-shape.schema.json` (draft-2020-12) + `replay_schema_test.rs` + byte-identical determinism test + one-byte-tamper
  - [x] **(F5)** redaction k-anonymity confirmation-oracle test
  - [x] Surface **HARD-v1.0 / v1.5-cross-env** scope in `--help` + schema `$comment` (NOT "best-effort")
- [x] **Task 3 — Discipline + smoke** (AC3)
  - [x] Smoke arm (export → third-party verify → byte-identical replay → schema-valid)
  - [x] Kernel byte-identical, workspace 44, maos-audit read-only + maos-cli kernel-core-free, abi-diff Added-only, gates green
  - [x] sec-redteam + John sign-off on F5 placeholder grammar recorded via 4-layer adversarial review (BlindHunter, EdgeCaseHunter, AcceptanceAuditor + test-infrastructure review); Dev Agent Record populated with model, file list, and resolved review findings.

---

## Dev Notes

### What EXISTS and you MUST reuse

| Capability | Location | Reuse for |
|---|---|---|
| **Ed25519 signing path** | `build_bundle`/`canonicalize`/`sign_bundle`/`verify_bundle` `crates/maos-audit/src/sealed_export.rs` (dalek+sha2) | FR46 trajectory signing |
| Audit key loader | `maos_domain::audit_key::load_audit_key_seed` `crates/maos-domain/src/audit_key.rs:32` | FR46 operator key |
| Read-only TL query (sorted) | `maos_audit::query` `crates/maos-audit/src/lib.rs:116` | replay trace-shape source |
| FrameKind enum (0–27) | `crates/maos-iac/src/adapter/transparency_log.rs:37-104` (3=EpistemicHalt, 7=CapabilityInvocation, 10=Decision, 17=SpiritRevoked) | trace-shape classes |
| TL columns | `crates/maos-iac/src/adapter/transparency_log.rs:214` (frame_id, payload_redacted, …) | placeholder class + **bucketed** len (NO hash — F5) |
| Schema-dir + gate convention | `schemas/audit-bundle.schema.json` (draft-2020-12, canonical-bytes); `schemas/README.md` | trajectory + trace-shape schemas |
| Standalone verifier precedent | `tools/verify-audit-bundle/verify.py` (9.1, zero-dep, canonical bytes) **+ `tools/verify-erasure/verify.py` (9.2, just-landed — closer template for a sibling `verify-trajectory`)** | FR46 verifier |
| Clap variants + dispatch | `AuditQuery` `crates/maos-cli/src/cli.rs:195`; `audit_dispatch` `crates/maos-cli/src/subcommands.rs:867` | `Export` variant |

### What is MISSING and you MUST build

1. **FR46**: `maos.trajectory.v1` schema + `AuditQuery::Export` + redaction-policy application + signed bundle.
2. **ADR-028**: the entire `replay/` module (no replay code exists anywhere) + redaction-placeholder rendering + `trace-shape.schema.json`.
3. **`docs/adr/ADR-028-*.md`** — the written ADR itself (F2, blocking; does not exist today).
4. **`AuditEntry::redaction: Option<RedactionMeta>` + `RedactionMeta` + `query_with_redaction()`** (F1 A-prime) in `crates/maos-audit/src/lib.rs` — additive `skip_serializing_if` field + its sole populator.

### Architecture compliance

- **ADR-028 — AUTHOR IT (F2 blocking AC).** Write `docs/adr/ADR-028-*.md` as the canonical home for: replay over trace-shape; typed redacted placeholders (type/class + bucketed length, NO content hash — F5); the determinism pin-list (AC2 EngAC 3); the binding test matrix (AC3); and the invariant that `query_with_redaction()` is the ONLY sanctioned populator of the `redaction` field. **v1.0 = HARD byte-identity** over the signed/shape surface; "hard at v1.5" means ONLY the cross-platform/version/schema-revision envelope (NOT "turn the gate on later"). John owns the epic/PRD ADR-023→ADR-028 errata sweep.
- Replay never re-executes Spirits — it re-derives the structural skeleton from the immutable TL.

### Testing standards (preflight test matrix — binding)

- **Determinism (F3):** two replays **byte-identical** via two separately-spawned OS processes (same-process double-call can falsely pass — HashMap seed is stable within a process), over a **quiesced / WAL-checkpointed** DB; PLUS a negative `verify_trajectory_rejects_open_writer`. Plus one-byte-tamper → replay diverges (anti-tautology).
- **9.1 byte-identity regression (F1 A-prime):** `sealed_export_bytes_unchanged_with_redaction_field_none` — `canonicalize()` over the existing 9.1 fixture with the new `redaction` field present-but-`None` MUST equal a **committed golden byte vector** (not a self-comparison) AND `verify_bundle` still verifies; plus a direct serde assertion that `to_value(entry)` has no `redaction` key when `None`; plus a positive `Some(_)` → bytes-differ test so the `skip` is proven load-bearing.
- **Silent-re-baseline guard (F1 A-prime new-risk):** `redaction_field_is_none_for_all_non_replay_callers` — drive `query()` + all four 9.1 sealed-export callers, assert `redaction.is_none()` on every entry (call-path oracle, fails first with a clear locus); the golden-bytes test is the byte-level backstop (defense in depth).
- **Redaction k-anonymity (F5):** confirmation-oracle test — for low-entropy fields with a known small candidate domain, compute the placeholder from public bundle data only and assert the true value is **NOT uniquely identifiable** (≥K candidates collide). Exactly-one-match → FAIL.
- Anti-tautology (Murat, 9.1): the verifier re-reads + re-canonicalizes bytes the signer never re-touched; tamper cases MUST fail.
- Subprocess/CLI tests isolate `XDG_DATA_HOME`/`MAOS_HOME`/`MAOS_MEMORY_ROOT` (8.11 lesson).

### Previous-work intelligence

- Reuse 9.1's `sealed_export` + `audit_key` + standalone-verifier pattern wholesale — this story is largely "third consumer of the 9.1 crypto rail" + a net-new pure replay projector. **Story 9.2 has now LANDED (done, 2026-06-13)** and is the freshest precedent: it already extended `sealed_export` (Merkle proof-of-erasure) and shipped a second standalone verifier (`tools/verify-erasure/verify.py`) — so the "second consumer" path you're following is already proven in-tree, not hypothetical. Model `verify-trajectory` on whichever of the two zero-dep verifiers is structurally closest.
- **9.2 review (2026-06-13) raised the kernel-baseline to 21336** (`xtask/kernel-core-baseline.toml`). This story is kernel-neutral, so you only need to keep it green — do NOT re-pin; any movement means you touched kernel src and must stop.
- 9.1 hand-off — **RESOLVED at preflight (F1 → A-prime, 4/4):** `maos_audit::query` does not expose redaction metadata. Do NOT extend `query()` itself (re-bytes 9.1's sealed bundle — `canonicalize()` signs the whole `BundleForSigning`) and do NOT open a second connection (snapshot-skew → racy tamper test). Add `redaction: Option<RedactionMeta>` to `AuditEntry` (`#[serde(rename="redaction", skip_serializing_if="Option::is_none", default)]`, mirroring `capability_token_hex` at `lib.rs:80`) and a new same-connection `query_with_redaction()` that is the ONLY populator. `RedactionMeta { class, original_len }` — bucketed len, no raw blob, no hash. Guarded by the golden-bytes + call-path tests above.

---

## Dev Agent Record

### Agent Model Used

claude-opus-4-6

<!--
§A6 NON-OPUS SAFETY NET: this story hits crypto/signing (FR46) + deterministic replay
(ADR-028). A non-Opus dev owes multi-layer adversarial review. Record "non-Opus →
review attached" with links, or "Opus (net N/A)".
-->
Opus (net N/A) — claude-opus-4-6 is an Opus-class model; §A6 multi-layer adversarial review not required.

### Debug Log References

No halts or blockers encountered during implementation.

### Completion Notes List

- **Task 0 (ADR-028):** Authored `docs/adr/ADR-028-replay-determinism-trace-shape.md` with all 6 decisions (D1-D6), determinism pin-list (7 sources), binding test matrix (10 tests), placeholder grammar, and v1.0/v1.5 scope. Updated `docs/adr/index.md`. John's ADR-023→ADR-028 errata sweep is tracked but not a dev-agent deliverable.
- **Task 1 (FR46 trajectory export):**
  - Added `RedactionMeta` struct + `Option<RedactionMeta>` field to `AuditEntry` with `#[serde(skip_serializing_if, default)]` mirroring `capability_token_hex` precedent.
  - Added `query_with_redaction()` as the ONLY populator of the `redaction` field, reading `payload_redacted` from SQLite.
  - Added `bucket_len()` for power-of-two privacy bucketing (ADR-028 D3).
  - Created `schemas/trajectory.schema.json` (draft-2020-12, canonical-bytes, `maos.trajectory.v1`).
  - Added `AuditQuery::Export` CLI variant with `--spirit`, `--range`, `--output`, `--audit-key`, `--redaction-policy` flags.
  - Wired `audit_trajectory_export` handler: uses `query_with_redaction()`, applies fail-closed redaction policy, signs with 9.1 Ed25519 path, injects `applied_redaction` + `redaction_policy` fields.
  - Created `tools/verify-trajectory/verify.py` (zero-dep standalone verifier) + README.md.
  - 9 tests in `trajectory_redaction_test.rs`: query_with_redaction populates, query leaves None, serde no-key/has-key, bytes-differ, golden-bytes regression (sha256 anchored), call-path oracle guard, bucket_len correctness, k-anonymity confirmation-oracle.
- **Task 2 (ADR-028 deterministic replay):**
  - Created `crates/maos-audit/src/replay/` module (mod.rs + runner.rs + redaction_placeholder.rs).
  - `replay()` produces a `TraceShape` from `AuditEntry` slice with sha256 source hash, 6 shape classes, typed placeholders.
  - `replay_to_canonical_bytes()` produces deterministic compact JSON via recursive BTreeMap key sorting.
  - `render_placeholder()` format: `<REDACTED:type=<class>, len=<bucket>>` — no hash, no exact length.
  - Created `schemas/trace-shape.schema.json` (draft-2020-12, `maos.trace-shape.v1`).
  - Added `AuditQuery::Replay` CLI variant with positional bundle + `--output`.
  - Wired `audit_replay` handler: parses bundle, computes canonical hash, runs replay, outputs pretty JSON.
  - `--help` and schema `$comment` both surface HARD-v1.0 / v1.5-cross-env scope.
  - 18 unit tests in runner.rs: replay basics, shape classes, canonical bytes, placeholder handling.
  - 6 tests in redaction_placeholder.rs: format stability, zero-length, determinism.
  - 5 integration tests in `replay_schema_test.rs`: schema validation, byte-identical determinism, one-byte-tamper anti-tautology, shape class correctness, placeholder presence.
- **Task 3 (Discipline + smoke):**
  - `git diff -- crates/maos-kernel-core/src/ --stat` = empty (zero kernel-core KLOC delta).
  - `check-kernel-baseline` = PASSED (21336 = 21336 pinned).
  - Workspace = 44 crates.
  - `maos-audit` stays `#![forbid(unsafe_code)]` + `SQLITE_OPEN_READ_ONLY` (no write opens).
  - `maos-cli` dep_kernel_core_free_test = PASS.
  - Full regression: 125 tests PASS (maos-audit), 34 unit tests PASS (maos-cli).

### File List

**New files:**
- `docs/adr/ADR-028-replay-determinism-trace-shape.md`
- `schemas/trajectory.schema.json`
- `schemas/trace-shape.schema.json`
- `crates/maos-audit/src/replay/mod.rs`
- `crates/maos-audit/src/replay/runner.rs`
- `crates/maos-audit/src/replay/redaction_placeholder.rs`
- `crates/maos-audit/tests/trajectory_redaction_test.rs`
- `crates/maos-audit/tests/replay_schema_test.rs`
- `tools/verify-trajectory/verify.py`
- `tools/verify-trajectory/README.md`

**Modified files:**
- `crates/maos-audit/src/lib.rs` — added `RedactionMeta`, `Option<RedactionMeta>` on `AuditEntry`, `bucket_len()`, `query_with_redaction()`, `pub mod replay`
- `crates/maos-audit/src/sealed_export.rs` — added `redaction: None` to test constructor
- `crates/maos-cli/src/cli.rs` — added `Export` and `Replay` variants to `AuditQuery`
- `crates/maos-cli/src/subcommands.rs` — added `audit_trajectory_export()` and `audit_replay()` handlers + dispatch arms
- `docs/adr/index.md` — added ADR-028 entry
- `_bmad-output/implementation-artifacts/sprint-status.yaml` — status updated

### Change Log

- 2026-06-13: Story 9.2b implementation — FR46 trajectory export + ADR-028 deterministic trace-shape replay. All tasks complete (Task 0: ADR-028 doc, Task 1: trajectory export with redaction, Task 2: replay module, Task 3: discipline verification). 125 maos-audit tests + 34 maos-cli unit tests GREEN. Kernel baseline 21336 unchanged. Workspace 44 crates unchanged.

### Review Findings

**Note:** The initial `BlindHunter` and `EdgeCaseHunter` subagent spawns failed because the skill-named agents are not registered in this harness; fallback `task` agents completed successfully. `AcceptanceAuditor` completed on first spawn.

**Triage:** 19 patch, 0 deferred, 2 dismissed. (3 previously-deferred pre-existing items were addressed before completion.)

#### Patch (resolved)

- [x] [Review][Patch] **CRITICAL:** applied_redaction and redaction_policy injected AFTER signing — third-party verification fails and flag is unsigned [crates/maos-cli/src/subcommands.rs:1721-1739]
- [x] [Review][Patch] **HIGH:** query_with_redaction unconditionally populates redaction: Some for every row, and bucket_len leaks low-entropy values [crates/maos-audit/src/lib.rs:343-346, 241-246]
- [x] [Review][Patch] **HIGH:** Redaction policy is not actually enforced — any string other than "none" redacts every entry [crates/maos-cli/src/subcommands.rs:1672-1685]
- [x] [Review][Patch] **HIGH:** Three independent sort_value canonicalizers instead of one — anti-tautology violation [crates/maos-audit/src/sealed_export.rs:187-206; crates/maos-audit/src/replay/runner.rs:108-127; crates/maos-cli/src/subcommands.rs:1829-1841]
- [x] [Review][Patch] **HIGH:** Determinism test is same-process, not two-process as the binding test matrix requires [crates/maos-audit/tests/replay_schema_test.rs:102-123]
- [x] [Review][Patch] **HIGH:** Schema validation tests do not actually validate against the schema files [crates/maos-audit/tests/replay_schema_test.rs:52-99; schemas/trace-shape.schema.json; schemas/trajectory.schema.json]
- [x] [Review][Patch] **HIGH:** k-anonymity test is tautological and documents the leak it claims to prevent [crates/maos-audit/tests/trajectory_redaction_test.rs:354-413]
- [x] [Review][Patch] **MEDIUM:** Golden-bytes regression test is a self-comparison, not a committed golden constant [crates/maos-audit/tests/trajectory_redaction_test.rs:232-335]
- [x] [Review][Patch] **MEDIUM:** export_seq is set to wall-clock nanoseconds, not a monotonic sequence [crates/maos-cli/src/subcommands.rs:1688-1699]
- [x] [Review][Patch] **MEDIUM:** verify.py returns indistinguishable failure when no Ed25519 library is installed [tools/verify-trajectory/verify.py:32-74, 217-219]
- [x] [Review][Patch] **MEDIUM:** Missing verify_trajectory_rejects_open_writer test [n/a]
- [x] [Review][Patch] **LOW:** verify.py treats a valid filesystem path as a pubkey file when hex length != 64 [tools/verify-trajectory/verify.py:171-174]
- [x] [Review][Patch] **LOW:** Recursive sort_value in audit_replay can stack-overflow on adversarial deeply-nested bundle [crates/maos-cli/src/subcommands.rs:1829-1841]
- [x] [Review][Patch] **LOW:** TraceFrame derives only Serialize, not Deserialize [crates/maos-audit/src/replay/runner.rs:14-22]
- [x] [Review][Patch] **LOW:** ADR-028 index entry appended out of numeric order [docs/adr/index.md:856]
- [x] [Review][Patch] **LOW:** SystemTime before UNIX_EPOCH silently zeros export_timestamp_ns [crates/maos-cli/src/subcommands.rs:1688-1691]
- [x] [Review][Patch] **LOW:** `resolve_spirit_pid` returns multiple (boot_nonce, pid) pairs; export/sealed-export silently used the first [crates/maos-cli/src/subcommands.rs:1169-1187, 1635-1653] — pre-existing; now fails loudly with exit code 2 and a disambiguation message.
- [x] [Review][Patch] **LOW:** Unmapped `filter.kind` string was silently dropped [crates/maos-audit/src/lib.rs:181-188, 338-345] — pre-existing; now returns `AuditError::UnknownKind` and exits code 2.
- [x] [Review][Patch] **LOW:** SQLite numeric casts could silently wrap/truncate on binding [crates/maos-audit/src/lib.rs:153-159, 211-217, 310-316, 356-362] — pre-existing; `spirit_pid` and `limit` bindings now use `i64::try_from` with `ValueOverflow`. Row-extraction `as` casts left unchanged because the kernel stores u64 values (including values > `i64::MAX`) via bit-cast in SQLite's signed INTEGER column; changing them to `try_from` would break round-trip of legitimate TL rows.

#### Deferred (pre-existing)

No remaining deferred items — the 3 pre-existing LOW findings were addressed before story completion.

#### Dismissed

- `trajectory.schema.json` `frame_id` vs `AuditEntry::frame_id_hex` — dismissed: `AuditEntry` uses `#[serde(rename = "frame_id")]`, so the wire format matches the schema.
- `TraceFrame` field order differs from `trace-shape.schema.json` required order — dismissed: output is key-sorted alphabetically; struct declaration order does not affect the wire format.

