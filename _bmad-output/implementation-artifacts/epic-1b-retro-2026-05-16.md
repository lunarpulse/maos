---
epic: 1b
epic_title: "Evaluator Path + Audit Spine + Capability Mediation Baseline (v0.1-β)"
retrospective_date: 2026-05-16
status: completed
participants:
  - Amelia (Senior Software Engineer — facilitator)
  - Alice (Product Manager — proxy for John)
  - Charlie (System Architect — proxy for Winston)
  - Dana (Test Architect — Murat)
  - Lunarpulse (Project Lead)
first_retrospective: false
previous_retro: _bmad-output/implementation-artifacts/epic-1a-retro-2026-05-13.md
next_epic: 2
epic_update_required: false
bridge_commits:
  - b610ac2  # fix(discipline): close I9 + NFR-Test-2 gates for Epic 1b runtime adapters
  - 95faf94  # fix(ci): repair cap-registry-smoke and onb-nfr2-timing CI scripts
---

# Epic 1b Retrospective — Evaluator Path + Audit Spine + Capability Mediation Baseline (v0.1-β)

## Epic Summary

| Delivery | Value |
|---|---|
| Stories completed | **7 of 7** (100%) — 1b.1, 1b.2, 1b.3, 1b.4, 1b.5a, 1b.5b, 1b.5c |
| Wall-clock | 3 days (2026-05-14 → 2026-05-16; 1b.1–1b.4 day 1, 1b.5a–1b.5c day 2, retro + bridge commits day 3) |
| Workspace shape | 17-crate workspace from Epic 1a → **19 crates** (`maos-audit` added in 1b.1; `maos-attrs` added in 1b.3) |
| ABI freeze | ComplianceClaim schema FROZEN in `maos-spirit-abi/src/compliance.rs`; `ABI_VERSION` bumped 0 → 1 (1b.4) |
| Flagship NFRs landed | NFR-Onb-2 5-min evaluator path, NFR-Test-13 ≥3 cases/field (60 fixture TOMLs), NFR-Sec-1 v0.1 T0/T1/T2 floor, NFR-Perf-3 cap-token validation, NFR-Obs-4 Transparency Log JSONL export, NFR-Rel-8 journal P99 |
| Flagship FRs landed | FR4 100% mediation (1000-call fixture in `maos-audit`), FR5 strictest-of floor, FR6 cgroups/setrlimit/Job Object, FR38 ComplianceClaim freeze, FR47 vendor-SDK denylist gate, FR58 hello-spirit J0 path |
| CI gates total | **28 jobs in discipline.yml** (up from 14 at end of E1a) |
| Reviewer patches | 1b.3: 5 critical (sandbox safety); 1b.5a: 9 patch + 2 defer (highest); 1b.5c: 5 patches; 1b.1/1b.2/1b.4 within ≤5 target |
| Deferred items added | ~12 entries across 1b.2–1b.5c (DF19–DF23 from 1b.2; pre-existing carry-forwards from 1b.5a/1b.5b/1b.5c) |
| Deferred items closed | 2 — DF18 (`SandboxTier::default()` T0→T2 via 1b.3); DW1 (`ComplianceClaimEnvelope` validation via 1b.4 structural validator) |
| Architectural divergences flagged | **5** — see "What Was Challenging" |
| Bridge commits in retro | 2 — `b610ac2` (I9 + NFR-Test-2 gates) + `95faf94` (CI script repairs) |

**Production incidents:** N/A (v0.1-β scaffold; no public surface yet).
**Blockers encountered during execution:** 0 hard blockers (4 CI-gate failures discovered in this retro's readiness check — all fixed in-session).
**Multi-agent execution:** Story-spec carried the discipline across all 7 stories; output quality independent of agent identity.

## What Went Well

1. **7/7 stories shipped with all flagship NFRs/FRs mechanically verified.** NFR-Onb-2 reproduces (`maosctl run hello-spirit` produces valid 4-key JSON), FR4 1000-call fixture passes (100% mediation), NFR-Test-13 walker enforces ≥3 cases per field across 20 manifest fields = 60 fixture TOMLs.

2. **A4 / A5 from Epic 1a retro held across all seven stories.** Every dev record carries a substantive "What did NOT happen this story" section with grep-checked anti-claims (e.g., 1b.5b: "no IAC Bus retract primitive" → that's Story 6.1). A5 explicitly cited in 1b.2's preflight ("applying Epic 1a retro lessons (A1/A2/A5)"). No story bled into adjacent scope.

3. **D7 / Story 1a.5 (`cargo-public-api` migration) shipped on schedule and gated 1b.4 ABI freeze.** ComplianceClaim schema frozen with serde derives + 7-malformation-class structural validator; `ABI_VERSION` bumped 0 → 1 in the same PR. Future schema breaks now trip `abi-diff` mechanically.

4. **DF16 operator action confirmed done during this retro by Lunarpulse.** Merge queue enabled, `journal-append` is a required status check. Closes the dominant E0 → E1a → E1b carry-forward.

5. **ADR-030 capability-registry decomposition shipped.** Four sub-modules wired (`cap_tokens` lock-free hot-path / `cap_policy` `Arc<ArcSwap<…>>` CoW / `cap_audit` bounded MPSC + single-writer task / `cap_quota` DashMap budget tracker). Composition root shares `Arc<PolicyTable>` between Capability Registry and Security Manager (1b.2 + 1b.3 wired in concert).

6. **Sandbox tier enforcement landed for all three platforms.** Linux: Landlock + seccomp-bpf + cgroups v2 + setrlimit fallback; macOS: SBPL + sandbox-exec; Windows: compilable stub (returns `Err(SandboxUnavailable)` per fail-closed semantics from 1b.3 review patch #1). DF18 (`SandboxTier::default()` returning T0) closed in-epic: `DEFAULT_FLOOR = T2`.

7. **Story 1b.5a review demonstrated the review process catching real defects.** 9 patches + 2 deferrals: capability_registry over-exposure narrowed to `TokenIssuer` trait; AC2 timing scope moved before `cargo clean`; `MAOS_ONE_SHOT` silent fallthrough → explicit validation; baseline JSON version mismatch restored. All caught pre-merge.

8. **Manifest field coverage (NFR-Test-13) shipped with comprehensive fixture pattern.** 20 fields × 3 cases (well-formed / malformed-rejected / edge-case) = 60 TOMLs in `crates/maos-kernel-core/tests/fixtures/manifest/`. The `manifest_field_coverage.rs` walker enforces ≥3 cases per field AND detects orphans. Pattern is reusable for Story 2.4's spirit-test SDK seed.

9. **Multi-platform execution path validated end-to-end.** `tests/integration/v01_evaluator_path.sh` chains install → hello-Spirit response → audit query → FR4 fixture → accessibility flags in one sequential CI gate (1b.5c AC4). Gates the v0.1 release tag.

## What Was Challenging

### 1. Workspace grew 17 → 19 crates without an architecture-doc update

`maos-audit` added in 1b.1 (Transparency Log adapter holder, separated from `maos-kernel-core` so external auditors can depend on the audit surface without pulling kernel runtime). `maos-attrs` added in 1b.3 (proc-macro crate hosting `#[i9_exempt]`; necessary because proc-macro crates cannot live inside the kernel core they annotate). Both crate additions are defensible and were noted in dev records, but the architecture document still says 17. Becomes **D10**.

### 2. Crate-level `#![forbid(unsafe_code)]` relaxed to per-module without an ADR

Story 1b.3 needed `unsafe` for `pre_exec` async-signal-safe syscalls in `security/sandbox/linux/`. The original Epic 1a discipline was crate-level `#![forbid(unsafe_code)]` on `maos-kernel-core`. 1b.3 relaxed this to per-module `forbid` (every module forbids except `security/sandbox/`). No ADR documents this governance change. Epic 2's `#[spirit]` proc-macro work will touch `unsafe` too — needs a precedent. Becomes **Doc3**.

### 3. Dual `SandboxTier` type hierarchy

Two parallel types now exist:
- `maos_domain::invariants::i9::SandboxTier` — kernel-side, used by admission and `cap_policy::strictest_of`
- `maos_spirit_abi::compliance::SandboxTier` — ABI-side enum, used by ComplianceClaim envelope

Neither is authoritative. Story 2.1 will land the full Spirit ABI vtable with 11 lifecycle hooks referencing `SandboxTier`. Landing 2.1 with two competing canonicals is a reconciliation crisis waiting. Becomes **D9** — must reconcile before Story 2.1 opens.

### 4. ADR-004 gate-line inconsistency with NFR-Sec-1

ADR-004 says sandbox tier T2 at v0.3. NFR-Sec-1 binds T0/T1/T2 enforcement floor at v0.1-β. Story 1b.3 shipped per NFR-Sec-1 (T2 at v0.1-β) without amending ADR-004. The doc says one thing; the code does another. Captured for a docs-pass closeout, not blocking.

### 5. Story 1b.3 `pre_exec` async-signal-safety took 5 critical review patches

Patches:
1. Windows stub fail-open → fixed to return `Err(SandboxUnavailable)`
2. Landlock/format! async-signal-unsafe → pre-compile rules in parent, only `restrict_self()` in pre_exec
3. Cgroup path used parent PID → fixed to use Spirit ID
4. `SandboxedChild` Drop order wrong → kill+wait child first, then remove cgroup
5. rlimit_cpu seconds-vs-percentage semantics wrong → removed from fallback
(+ 6. seccomp allow-list too narrow → expanded with glibc/sh syscalls)

The parent/child execution-context split is the sharpest learning from 1b: pre_exec runs after fork() but before exec(), in a single-threaded context where allocator + locks + panics are all unsafe. Future stories touching `pre_exec` MUST pre-compile in the parent.

### 6. Story 1b.5a had the highest review burden of the epic — 9 patches + 2 defers

`InferencePortAdapter::capability_registry()` originally returned `&CapabilityRegistryAdapter` (full registry surface); narrowed to `&dyn TokenIssuer` to enforce the principle of least exposure. That narrowing changed the kernel surface signature hash AND added a new `TokenIssuer` re-export — both of which became NFR-Test-2 baseline drift the team didn't catch (see #7 below).

Other 1b.5a patches were genuine defects: AC2 timing scope, FIXME(secrets) removed without fixing, `MAOS_ONE_SHOT` silent fallthrough, baseline JSON version mismatch. All caught pre-merge — process worked.

### 7. **CI gate failures hidden in plain sight across 1b.3 → 1b.5c**

`check-empty-kernel` and `check-service-boundary` were **red on every commit from 1b.3 onward**. Discovery happened in this retro's readiness check, not during any story dev cycle. Root cause was twofold:

- **Six new runtime adapter structs needed I9 exemption registration** (`InferencePortAdapter`, `SecurityManagerAdapter`, `SandboxSpec`, `HistogramSeries`, `CounterSeries`, `IacRtMetrics`). The I9 sanctioned-paths whitelist (`xtask/i9-whitelist.toml`) covers only three sanctioned persistent locations (Journal, TransparencyLog, CapTokens). The new adapters were sanctioned by Epic 1b Owns but the `#[maos_attrs::i9_exempt]` annotations + `docs/invariants/i9-exemptions.md` register entries never got applied.
- **1b.5a's `TokenIssuer` narrowing changed the kernel surface monotonically** — `InferencePortAdapter`'s signature hash changed (method added), and `TokenIssuer` became a new public re-export. Neither was reflected in `docs/ci-baselines/kernel-surface-v0.1-beta.json` or `xtask/kernel-api-classes.toml`.

Why nobody noticed: the team was reading the **`journal-append.yml`** workflow's always-success run alongside the failing `discipline.yml` run for each commit. Two separate workflows fire on every push; the success of `journal-append` masked the failure of `discipline` in the GitHub Actions dashboard at-a-glance view. Closed by **bridge commit b610ac2** in this retro session. Pattern captured as **A8**.

### 8. **Two CI smoke scripts shipped broken — passed locally, failed cold on CI**

`tests/integration/cap_registry_smoke.sh` wrapped `cargo run -p maos-bin --quiet &` in `timeout 5s bash -c '…'`. On a warm local cache `cargo run` skips compile and executes in <1s; on a cold CI cache the same `cargo run` triggers a full compile that exceeds 5s, killing the bash subshell with exit 124 before the binary ever launched.

`tests/integration/onb_nfr2_timing.sh` invoked `cargo build --release --bin maos-bin --locked` from the workspace root. The workspace declares `default-members = []` (a deliberate discipline choice forcing every cargo call to be `-p`-explicit). Bare `--bin` resolution under `default-members = []` panics immediately: `error: manifest path '…/maos' contains no package: The manifest is virtual, and the workspace has no members.` — exit 101 in <1 second. Never reached the actual workload.

Both scripts passed locally because the local target/ was warm AND the local invocation happened to use compatible cargo arguments. Neither was ever tested cold. Closed by **bridge commit 95faf94** in this retro session. Patterns captured as **A6** (cold-cache discipline) and **A7** (`-p` selection invariant).

### 9. Pre-existing 1b.1 design gaps carried forward unchanged

- **Server exit drain gap**: non-one-shot SIGINT/SIGTERM doesn't drain `audit_writer`; last N audit entries lost. Pre-existing in `crates/maos-bin/src/main.rs` server path. Noted but not fixed in 1b.5b. Becomes **D11**.
- **Journal corruption fragility**: single bad NDJSON line at journal tail → journal permanently unopenable. `JournalAdapter::open` is parse-and-reject, not skip-and-continue. Pre-existing in `journal/mod.rs:110-121`. Becomes **D12** — must address before Story 5.3 (crash supervision).
- **`capability_token_bytes` JSON-serialized vs raw**: 1b.2 chose JSON serialization for `Invocation.capability_token_bytes`; 1b.5b's FR4 join surfaced the inconsistency but kept it as deferred. Tracked as **DF23**.

## Patterns Across Stories (Deep Analysis)

### Recurring positive patterns

- **A1/A2/A3/A4/A5 disciplines held without exception.** Self-review checklists (20–25+ items per story), dep-introduction notes with `cargo tree` + blast counts, worked examples in every quantitative AC, "What did NOT happen this story" grep-checked sections, IDE-vs-cargo trust hierarchy explicitly cited. Five-discipline stack working as designed.
- **Decision Registers in every story.** Non-trivial design choices captured inline with rationale (1b.3 SandboxTier defaults; 1b.4 `ABI_VERSION` bump + telemetry library choice; 1b.5a one-shot env-var dispatch + mock-by-default; 1b.5b on-disk vs in-memory + cap-audit drain sequence; 1b.5c lifecycle-verb env-var fusion).
- **Code-review → patch → re-verify cycle internalized.** 1b.3's 5 critical patches and 1b.5a's 9 patches caught real defects pre-merge. Process produces durable improvement, not just process compliance.
- **NFR-Test-13 walker pattern is reusable.** The `<section>/{well-formed,malformed-rejected,edge-case}/<field>.toml` directory shape + walker is a primitive Epic 2's spirit-test SDK can build on.

### Recurring concerning patterns

- **Architectural-divergence accretion (5 in one epic).** 17→19 crates, dual SandboxTier types, crate-level unsafe relaxation, ADR-004 gate-line inconsistency, `maos-attrs` introduction. None are gated by an "architecture-update PR required" check. The cumulative cost is invisible per-story but compounds.
- **CI gate dev-record discipline gap.** `check-empty-kernel` and `check-service-boundary` failures were not flagged in any 1b.3 / 1b.4 / 1b.5a / 1b.5b / 1b.5c dev record's "Gates Status" section. The two-workflow-on-push pattern (discipline.yml + journal-append.yml) made reading the wrong run easy.
- **Local-only verification of integration scripts.** Both broken CI scripts (`cap_registry_smoke`, `onb_nfr2_timing`) passed locally on warm caches. Nobody ran them cold. Locally-verified ≠ CI-verified for any step that depends on cargo cache state.
- **Pre-existing design gaps accumulate as deferred items.** Server exit drain (1b.1), journal corruption fragility (1b.1), `capability_token_bytes` serialization (1b.2). Three "noted but not fixed" items from early stories that re-surface in later story dev records. Bounded, but the trend should be re-assessed at Epic 2 retro.

## Previous Retrospective Follow-Through (Epic 1a → Epic 1b)

| Epic 1a action | Status | Evidence |
|---|---|---|
| **A4** — Epic-vs-story coherence check (extends A3) | ✅ Completed | "What did NOT happen this story" sections present and grep-verified across all 7 stories; zero unflagged epic-vs-story drift |
| **A5** — IDE-vs-cargo trust hierarchy in dev records | ✅ Completed | Explicitly cited in 1b.2 dev record's preflight ("applying Epic 1a retro lessons (A1/A2/A5)"); dev records cite `cargo build --locked` output not IDE state |
| **D6** — Track 15 Epic 1a deferrals in `deferred-work.md` | ✅ Ongoing | File maintained through 1b; DF18 closed by 1b.3 (`SandboxTier::default()` now T2); DW1 closed by 1b.4 (`ComplianceClaimEnvelope` validator ships) |
| **D7 / Story 1a.5** — `cargo-public-api` migration | ✅ Completed | Sprint-status: `1a-5: done`; `abi-diff` gate now uses `cargo-public-api`; gated 1b.4 ABI freeze on schedule |
| **D8 + Doc2** — epic-1a `Owns` prose fix | ✅ Completed | Applied in 1a retro save step |
| **DF16 operator action** — GitHub merge queue + `journal-append` required status check | ✅ **Completed** | Confirmed by Lunarpulse during this retro (`journal-append.yml` workflow exists at GH workflow ID 276109679; merge queue + required check enabled per repo settings) |

**Net:** All 6 Epic 1a action items closed. No carry-forward from E1a into Epic 2.

## Next Epic Preview — Epic 2: Spirit ABI + Developer SDK + Boundary Contracts (v0.1 → v0.3)

**Goal:** A Spirit author at 9pm Tuesday clones a template, implements `on_idle`, runs the `spirit-test` harness locally, and ships a binary Spirit without ever touching kernel internals. NFR-Onb-1 v0.3 gate prerequisites land here.

### Dependencies on Epic 1b

| Epic 1b substrate | Epic 2 consumer |
|---|---|
| Frozen ComplianceClaim schema (`ABI_VERSION = 1`, 1b.4) | Story 2.1 extends `maos-spirit-abi` with full vtable + 11 lifecycle hook signatures |
| `maos-spirit-abi` frozen ABI boundary | Story 2.1 `#[spirit]` proc-macro derives against these types |
| Capability mediation + token issuance (1b.2 + `TokenIssuer` trait from 1b.5a review) | Story 2.2 P1–P4 full enforcement checks the capability mediation chain |
| Manifest field test coverage pattern + 60 fixture TOMLs (1b.5c) | Story 2.4 spirit-test SDK harness exercises manifest self-check via the same shape |
| `xtask check-service-boundary` stub (1a.3) | Story 2.2 upgrades to real ABI type reflection (depends on stable Spirit ABI) |
| `maos-attrs` proc-macro crate (1b.3) | Story 2.1 `#[spirit]` proc-macro pattern parallels `#[i9_exempt]` |
| Hexagonal port traits (Epic 1a) + adapter shells (1b.1–1b.4) | Story 2.2 supervision-tree static analysis enforces P1 (single owner per service) |

### Preparation needed (critical path)

1. **D9 — Dual `SandboxTier` reconciliation** — Story 2.1 will land 11 lifecycle hook signatures that reference SandboxTier. Two competing types in the ABI extension is a mess. Pick canonical type (recommend kernel-side `maos_domain::invariants::i9::SandboxTier` — it's used at admission and by `cap_policy::strictest_of`) and de-export / alias-only the ABI-side enum.
2. **D10 — Architecture doc workspace catch-up** — Update arch doc to reflect 19-crate workspace; document `maos-audit` (separated Transparency Log surface) and `maos-attrs` (proc-macro crate hosting i9_exempt); specify dependency directions.
3. **Doc3 — ADR for `#![forbid(unsafe_code)]` per-module relaxation** — Document the 1b.3 governance change so Story 2.1's proc-macro work has a precedent.

## Action Items

### Process Improvements

**A6 — Integration-script CI cold-cache discipline.**
- Owner: Whoever authors next integration script (and reviewer)
- Trigger: Every new `tests/integration/*.sh`
- Success criteria: Any `timeout` command wraps EXECUTION only, never COMPILATION. Build steps run outside the timed window. Self-review checklist adds: "☐ Integration script `timeout` wrapping verified cold-cache reproducible (cargo build outside the timeout block)."
- Measurement: Epic 2 retro targets zero integration scripts breaking on cold CI cache.
- Precedent: `cap_registry_smoke.sh` shipped with `timeout 5s` wrapping `cargo run`. Fixed in bridge commit 95faf94.

**A7 — `default-members = []` invariant: scripts MUST use `-p` package selection.**
- Owner: Amelia (and every story author from 2.1 onward)
- Trigger: Story 2.1 creation
- Success criteria: Any cargo invocation in shell scripts, Makefiles, or CI workflows uses `-p <crate>` (package-scoped). Bare `--bin <name>` or `--lib` from the workspace root is forbidden — under `default-members = []` it panics with `manifest is virtual, workspace has no members` (exit 101). Self-review checklist adds: "☐ All cargo invocations in scripts use `-p <crate>` selection."
- Precedent: `onb_nfr2_timing.sh` shipped with bare `--bin maos-bin`; failed silently across 1b.5a / 1b.5b / 1b.5c. Fixed in bridge commit 95faf94.

**A8 — Dev-record `Gates Status` section MUST cite the failing-or-flaky `discipline.yml` run conclusion explicitly.**
- Owner: Amelia (and every story author from 2.1 onward)
- Trigger: Story 2.1 creation
- Success criteria: Dev records' Gates Status section links the SPECIFIC `discipline.yml` run for the commit AND distinguishes it from concurrent workflow runs (`journal-append.yml`). "Gates green locally" is insufficient — the dev record MUST cite the discipline.yml run conclusion (success/failure/cancelled). Self-review checklist adds: "☐ `discipline.yml` run conclusion cited for this commit; `journal-append.yml` success NOT used as proxy for discipline success."
- Precedent: `check-empty-kernel` + `check-service-boundary` were red across 1b.3→1b.5c with zero dev-record flag.

### Technical Debt

**D9 — Reconcile dual `SandboxTier` type hierarchies.**
- Owner: Whoever opens Story 2.1
- Deadline: **Before Story 2.1 lands**
- Success criteria: One canonical `SandboxTier` exported through the ABI; the other deprecated with a clear migration path (alias-only or removed). Spec the choice in the story design doc; ratify in Story 2.1 dev record.

**D10 — Architecture doc 17 → 19-crate workspace catch-up.**
- Owner: Charlie (Architect)
- Deadline: **Before Story 2.1 opens**
- Success criteria: arch doc reflects current 19-crate workspace; documents `maos-audit` (added 1b.1 — Transparency Log adapter holder) and `maos-attrs` (added 1b.3 — `i9_exempt` proc-macro); specifies dependency directions and intended consumers.

**D11 — Server exit drain gap (carried from 1b.1).**
- Owner: Whoever opens the first Story 2.x or 3.x that touches `maos-bin/src/main.rs` server path
- Background: SIGINT/SIGTERM in non-one-shot server mode doesn't drain `audit_writer`; last N audit entries lost. Pre-existing.
- Deadline: Before Epic 3 (where IAC Bus task assignment will rely on audit chain integrity)

**D12 — Journal corruption fragility (carried from 1b.1).**
- Owner: Whoever opens Story 5.3 (crash supervision)
- Background: Single bad NDJSON line at journal tail → journal permanently unopenable. `JournalAdapter::open` is parse-and-reject, not skip-and-continue. Pre-existing in `journal/mod.rs:110-121`.
- Deadline: Before Story 5.3 (crash supervision must recover from corrupted tails)

### Documentation

**Doc3 — ADR for `#![forbid(unsafe_code)]` per-module relaxation.**
- Owner: Charlie (Architect)
- Trigger: **Before Epic 2 proc-macro work in Story 2.1**
- Background: Story 1b.3 relaxed crate-level `forbid(unsafe_code)` to per-module in `maos-kernel-core` to allow `unsafe` in `security/sandbox/` for `pre_exec` async-signal-safe syscalls. No ADR documents this. Epic 2's `#[spirit]` proc-macro will touch unsafe too. Need a governance precedent before more unsafe surface lands.
- Success criteria: New ADR explaining where unsafe is permitted, audit cadence, exemption process.

**Doc4 — ADR-004 gate-line consistency pass.**
- Owner: Charlie (Architect) — opportunistic
- Background: ADR-004 says sandbox tier T2 at v0.3; NFR-Sec-1 binds T0/T1/T2 at v0.1-β; 1b.3 shipped per NFR-Sec-1. Doc/code drift; cosmetic.
- Deadline: Before v0.3 release; not blocking Epic 2.

### Team Agreements

- A1/A2/A3 from E0 retro continue unchanged.
- A4/A5 from E1a retro continue unchanged.
- **A6/A7/A8 (this retro) added.**
- All cargo invocations in scripts use `-p <crate>` (A7).
- Dev records' Gates Status section cites the `discipline.yml` run conclusion explicitly (A8).
- Integration script `timeout` wraps execution only (A6).
- Every story file ends with the self-review checklist; no story opens a PR without it.

## Critical Path Before Epic 2

1. ✅ **DF16 operator action** — merge queue + `journal-append` required status check. CONFIRMED DONE by Lunarpulse during this retro.
2. ✅ **Bridge commit `b610ac2`** — `fix(discipline): close I9 + NFR-Test-2 gates for Epic 1b runtime adapters`. Added `#[maos_attrs::i9_exempt]` + i9-exemptions.md entries for 6 runtime adapters; refreshed kernel-surface baseline + classified `TokenIssuer`. Both gates GREEN locally and on CI.
3. ✅ **Bridge commit `95faf94`** — `fix(ci): repair cap-registry-smoke and onb-nfr2-timing CI scripts`. Build moved outside `timeout` window; `--bin` swapped for `-p`-selection. Both scripts GREEN locally; CI sweep in flight at retro save.
4. ⏳ **D9** — Reconcile dual `SandboxTier`. Required before Story 2.1 opens. Single-PR refactor.
5. ⏳ **D10** — Architecture doc 17 → 19-crate catch-up. Required before Story 2.1 opens. Doc-only.
6. ⏳ **Doc3** — `forbid(unsafe_code)` per-module ADR. Required before Story 2.1 opens. ADR-only.
7. ✅ **Sprint-status flip**: `epic-1b: in-progress → done` + `epic-1b-retrospective: optional → done`. Done in this retro save step.

## Significant Discovery Check

**Epic 2 update required: NO.** None of the 10 discovery-alert criteria escalates to "re-plan Epic 2":

- Architectural assumptions (Spirit ABI extension via `#[spirit]` macro, P1–P4 enforcement, `cargo-generate` template) all hold.
- Scope intact — all 7 stories shipped at spec scope.
- Technical approach composes cleanly with Epic 1b's substrate.
- Dependencies **D9 + D10 + Doc3** are listed Epic 2 prerequisites — captured as critical-path doc/refactor items, not re-scopes.
- No external users at v0.1-β — N/A.
- KLOC well within budget.
- ComplianceClaim schema frozen per spec; Epic 2 builds on it.
- Substrate is the consumer Epic 1b was designed for — adapter shells from 1a, runtime bodies from 1b, ABI extensions in 2.
- Multi-agent execution worked; no capacity/skill-gap concern.
- ~10 deferred items added in 1b (mostly cosmetic + pre-existing carry-forwards); none escalated to blocking.

## Readiness Assessment

| Dimension | Status |
|---|---|
| Testing & quality | ✅ Post-bridge CI sweep on origin/main confirmed **28/28 jobs green, 0 failed** (discipline.yml run 25947061453, sha `c7ab9d0b`). All 4 targeted gates green: `check-empty-kernel`, `check-service-boundary`, `cap-registry-smoke`, `onb-nfr2-timing`. `cargo test -p maos-kernel-core --lib` 110/110 pass locally. |
| Deployment | ✅ N/A — v0.1-β scaffold; `maos-bin` builds release-clean, ≤10MB stripped (verified at 6,099,824 bytes < 10,485,760 limit). |
| Stakeholder acceptance | ✅ ComplianceClaim schema frozen per E0 adversarial review; Inference Port operational with Anthropic provider; NFR-Onb-2 5-minute path mechanically reproducible. |
| Technical health | ⚠️ KLOC well under alarm; unsafe scope per-module limited to `security/sandbox/`; **arch doc out of sync (17 vs 19 crates — D10)**; **dual SandboxTier type hierarchy (D9)**. |
| Unresolved blockers | ✅ All CI gates green after bridge commits; DF16 operator action confirmed done. |
| ABI-stability | ✅ ComplianceClaim `ABI_VERSION = 1`; `cargo-public-api` baseline refreshed (4 hash updates + `TokenIssuer` re-export added). |

**Verdict:** Epic 1b is **structurally complete and code-shippable** post-bridge. Three amber items (D9 dual-SandboxTier, D10 arch-doc catch-up, Doc3 unsafe ADR) gate Epic 2 — all are documentation/refactoring items, not Epic 1b defects.

## Closure Note

Amelia (Developer): "Seven stories, seven done. Two CI gates were silently red across 1b.3→1b.5c and we caught them in the retro readiness check — bridge commits `b610ac2` + `95faf94` fixed both classes. The lesson is that 'gates green locally' is structurally meaningless when integration scripts assume warm caches and dev records read the wrong workflow. A6, A7, A8 institutionalize the fix."

Charlie (Architect): "The dual SandboxTier hierarchy concerns me most for Epic 2. Story 2.1's `#[spirit]` proc-macro will reference SandboxTier in 11 lifecycle hook signatures — we cannot land that PR with two competing canonicals. D9 must close before 2.1 opens. D10 + Doc3 are smaller — arch doc walks the now-19-crate workspace, ADR formalizes the per-module unsafe relaxation."

Alice (PM): "Three flagship NFRs landed (NFR-Onb-2 5-minute path, NFR-Test-13 ≥3 cases/field, NFR-Sec-1 T0/T1/T2 floor). Three flagship FRs landed (FR4 100% mediation mechanically verified, FR38 ComplianceClaim freeze, FR47 vendor-SDK denylist). 1b.5a's review process caught real defects pre-merge — that investment in adversarial review at E0 keeps paying."

Dana (QA): "Manifest field coverage with 60 fixture TOMLs is a reusable primitive — Story 2.4's spirit-test SDK seed gets the harness pattern for free. Multi-platform sandbox (Linux Landlock+seccomp+cgroups, macOS sandbox-exec, Windows compilable stub) is genuinely cross-platform; not 'works-on-Linux-and-the-other-two-are-stubs.' cap-registry-smoke + onb-nfr2-timing failures were our blind spot — A6/A7/A8 close it."

Lunarpulse (Project Lead): "DF16 operator action confirmed done. Bridge commits `b610ac2` + `95faf94` pushed; CI sweep on origin/main confirmed **28/28 green, 0 failed** (discipline.yml run 25947061453). Epic 1b accepted. Proceeding to Epic 2 preparation: D9 dual-SandboxTier reconciliation, D10 arch-doc 17→19 catch-up, Doc3 unsafe-relaxation ADR — all single-PR doc/refactor changes before Story 2.1 opens."

## References

- `_bmad-output/planning-artifacts/epics/epic-1b-evaluator-path-audit-spine-capability-mediation-baseline-v01.md` — epic definition
- `_bmad-output/planning-artifacts/epics/epic-2-spirit-abi-developer-sdk-boundary-contracts-v01-v03.md` — next epic
- `_bmad-output/implementation-artifacts/epic-1a-retro-2026-05-13.md` — previous retrospective
- `_bmad-output/implementation-artifacts/1b-1-three-audit-logs-transparency-approval-decision-lifecycle-journal.md` — Story 1b.1 dev record
- `_bmad-output/implementation-artifacts/1b-2-capability-registry-decomposition-runtime-cap-tokens-cap-policy-cap-audit-cap-quota.md` — Story 1b.2 dev record
- `_bmad-output/implementation-artifacts/1b-3-sandbox-tier-t0-t1-t2-enforcement-per-spirit-resource-caps.md` — Story 1b.3 dev record
- `_bmad-output/implementation-artifacts/1b-4-freeze-the-complianceclaim-schema-and-wire-the-inference-port-iac-telemetry.md` — Story 1b.4 dev record
- `_bmad-output/implementation-artifacts/1b-5a-ship-hello-spirit-reference-binary-and-hit-nfr-onb-2-5-minute-evaluator-path.md` — Story 1b.5a dev record
- `_bmad-output/implementation-artifacts/1b-5b-maosctl-audit-query-fr4-100-mediation-mechanical-verification.md` — Story 1b.5b dev record
- `_bmad-output/implementation-artifacts/1b-5c-maosctl-v0-1-lifecycle-subcommands-accessibility-flags.md` — Story 1b.5c dev record
- `_bmad-output/implementation-artifacts/deferred-work.md` — accumulated deferred items
- `docs/invariants/i9-exemptions.md` — I9 exemption register (extended in bridge commit `b610ac2`)
- `docs/ci-baselines/kernel-surface-v0.1-beta.json` — NFR-Test-2 baseline (refreshed in bridge commit `b610ac2`)
- `xtask/kernel-api-classes.toml` — kernel-API classification (`TokenIssuer` added in bridge commit `b610ac2`)
- Bridge commits: `b610ac2` (discipline gates) + `95faf94` (CI scripts)
- CI run 25946814134 (pre-bridge `ae6e49e`) — 4 gates red (2 structural + 2 script defects)
- CI run 25947061453 (script-fix `c7ab9d0b`) — discipline.yml sweep **28/28 GREEN, 0 failed** — confirms both bridge commits land clean
