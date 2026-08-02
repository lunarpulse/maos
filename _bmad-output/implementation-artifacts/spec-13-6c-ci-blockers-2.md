---
title: 'Make discipline CI green after Story 13.6c'
type: 'bugfix'
created: '2026-08-01'
baseline_commit: '4a952f81674672eab696ea676dfe38bb43268006'
status: 'done'
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/implementation-artifacts/epic-13-context.md'
  - '{project-root}/_bmad-output/implementation-artifacts/13-6c-three-team-three-region-substrate.md'
  - '{project-root}/_bmad-output/implementation-artifacts/spec-13-6c-ci-blockers.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Actions run 30738417979 proves all five Story 13.6c gates are green, but six primary pre-existing controls still red the aggregate: vulnerable Wasmtime, unregistered production env reads, implicit serde panics, stale service-boundary evidence, incomplete dev/review provenance, and unresolved private-store findings exposed by the required historical 13.5j review.

**Approach:** Close every primary diagnostic at its source, preserve the controls, harden the private filesystem store with directory-relative no-follow I/O and durable atomic replacement, record truthful provenance/review evidence, and produce one reviewed commit without co-author metadata.

## Boundaries & Constraints

**Always:** Keep fail-closed security behavior; use authoritative sprint status before exempting undeveloped stories; record `openai-codex/gpt-5.6-sol` as the user-confirmed 13.5j implementation model and the new Blind Hunter/Acceptance Auditor review as a 2026-08-01 backfill; resolve every recorded high/medium finding; update the measured kernel line pin and HISTORY rationale; keep Story 13.6c `in-progress` until the next pushed run supplies aggregate-green evidence.

**Ask First:** A new vulnerability ignore/allowlist, service-boundary or kernel KLOC ceiling increase, non-Unix security degradation, public API break, or decision to leave a high/medium finding unresolved. Approval of this spec authorizes measured kernel pin movement but not a ceiling increase.

**Never:** Disable or suppress a gate; guess model provenance; insert a review marker without a real review; allowlist the three serde violations; use pathname pre-checks as the only symlink defense; drop one side of a concurrent update; acknowledge cache hits before durable spill replacement; delete the containment baseline to silence drift; add a co-author section or trailer.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Supply chain | Wasmtime 46.0.1 lock graph | Whole family resolves to patched 46.0.2 | `cargo deny` remains blocking |
| Private spill | Existing RAM value is updated | RAM and durable spill both reflect the update | Spill failure returns error; cache is not advanced |
| Concurrent spill | Two updates to one key | Serialized atomic replacements; no lost temp/rename | Failed writer cleans its unique temp |
| Hostile path | Symlink at pid, namespace, or spill name | No traversal, read, or deletion outside verified directory handles | Fail closed with `PrivateMemoryError` |
| Stale duplicate | Legacy and canonical files share a key | Newest authoritative row is returned once; obsolete duplicates removed safely | Metadata/lookup errors are surfaced, not treated as absence |
| Dev record | `blocked` story has no implementation | Model gates skip it from authoritative sprint state | Done stories still require model and review evidence |
| Serialization | CBOR/JSON encoding unexpectedly fails | Explicit contextual fail-fast path | No `expect`/`unwrap` or silent fallback |

</frozen-after-approval>

## Code Map

- `Cargo.lock` -- Wasmtime/WASI security patch graph.
- `crates/maos-bin/src/env_contract.rs`, `crates/maos-bin/src/main.rs` -- five real env registrations, two serde sites, and two test-only P1 markers.
- `crates/maos-compliance/src/vetting/{attestation,keyring}.rs` -- two explicit CBOR failure paths.
- `docs/ci-baselines/kernel-surface-v0.1-beta.json` -- ratified current public surface.
- `xtask/src/check_dev_model_{used_populated,tier}.rs` -- shared authoritative pre-development status policy.
- `_bmad-output/implementation-artifacts/13-{5g,5i,5j,6c}-*.md` -- truthful model/review records.
- `crates/maos-kernel-core/src/memory/private.rs`, `crates/maos-kernel-core/tests/private_{spill_supersession_13_5j,forget_restart_13_5i}.rs` -- private spill correctness, containment, concurrency, durability, and compatibility contracts.
- `crates/maos-kernel-core/Cargo.toml`, `xtask/kernel-core-baseline.toml` -- safe filesystem dependency and measured kernel HISTORY evidence.

## Tasks & Acceptance

**Execution:**
- [x] `Cargo.lock` -- update the full Wasmtime 46 family to 46.0.2 without advisory ignores.
- [x] `env_contract.rs`, `main.rs`, compliance vetting -- register five env reads, add exact P1 test exemptions, and replace four implicit serde panics with explicit contextual handling.
- [x] `kernel-surface-v0.1-beta.json` -- regenerate the ratified surface after confirming the backend registry expansion.
- [x] model-gate sources and four story records -- share `blocked`/pre-dev status handling, add known frontier model aliases, record confirmed models, exact review markers, and the completed 13.5j review findings.
- [x] `private.rs` and kernel evidence -- use safe rustix directory-relative no-follow operations, atomic unique-temp replacement with fsync, operation serialization, overwrite persistence, latest-wins dedupe, and fail-closed errors; add hostile-symlink, stale-duplicate, persistence, and concurrency tests.
- [x] this spec -- record proof and commit the reviewed work once without co-author metadata.

**Acceptance Criteria:**
- Given the current branch, when all six primary CI checks run, then each reports green without allowlists or suppressed diagnostics.
- Given the historical 13.5j findings, when private-store tests exercise overwrite, races, duplicates, symlinks, and forced spill failures, then every path preserves containment and current state.
- Given kernel changes, when line and service-boundary gates run, then the exact pin/baseline and HISTORY rationale agree and no ceiling moves.
- Given the final commit, when its body/trailers are inspected, then no co-author metadata exists.

## Spec Change Log

## Design Notes

On Unix, open the configured root once, then create/open each single sanitized pid/namespace component and every spill file relative to verified directory FDs with rustix `O_NOFOLLOW`; enumerate and unlink through those FDs. Spill via a unique `CREATE|EXCL` temp, full write, file fsync, rename, and directory fsync. Non-Unix must preserve equivalent containment or halt under Ask First. A store-level lock is acceptable because this synchronous cold-tier path favors correctness over throughput.

## Verification

**Commands:**
- `cargo deny check` -- no vulnerability errors.
- `cargo run -q -p xtask -- check-{service-boundary,env-contract,serde-error-handling,dev-model-used-populated,dev-model-tier,dev-record-completeness} --json` with required baseline argument -- all green.
- `cargo test -p maos-kernel-core --test private_spill_supersession_13_5j && cargo test -p maos-kernel-core --test private_forget_restart_13_5i` -- public-adapter private-store contracts pass.
- `cargo test -p maos-compliance && cargo test -p xtask` -- changed package behavior and gates pass.
- `cargo run -q -p xtask -- check-kernel-baseline --json && cargo run -q -p xtask -- kloc-check --json` -- exact pin and ceilings pass.
- `cargo fmt --all -- --check` -- no formatting drift.

**Observed 2026-08-02:**
- `cargo test -p maos-kernel-core` -- 558 passed, 1 ignored; targeted 13.5j integration contracts passed 15/15 and 13.5i contracts passed 10/10.
- `cargo test -p maos-compliance` -- 62 passed. `cargo test -p xtask` -- 538 passed, 1 ignored.
- Full primary gate sweep passed: env contract (84 registered, 0 violations), serde handling (0 violations), service boundary, model-used, model-tier, dev-record completeness (125 stories), and review-findings resolution (134 stories).
- `check-kernel-baseline` passed at 23517/23517; `kloc-check` passed with maos-kernel-core at 18171 below the unchanged 18248 ceiling.
- `cargo deny check` passed advisories, bans, licenses, and sources; `cargo fmt --all -- --check` passed.
- One initial full-kernel run observed the existing timing-sensitive journal-fsync P99 probe at 4215 microseconds versus its 1500-microsecond budget. The isolated probe passed immediately afterward, and the final full 558-test kernel suite passed.
- Pushed run `30756780795` exposed one additional Reza corpus diagnostic contract: namespace non-directories now map to contextual `ENOTDIR`; both the two-case GDPR cascade corpus and the ten-case 13.5i erasure suite pass.

## Review Findings — 2026-08-02

Blind Hunter and Edge Case Hunter reviewed the complete tracked-plus-untracked diff from `4a952f81674672eab696ea676dfe38bb43268006` in parallel. Their six unique findings and one pushed-CI follow-up were classified as patches; no intent gap, bad spec, or deferred work remained.

- [x] **High / patch:** Preserve the prior durable spill set on every post-rename or stale-cleanup failure. Added hard-link-backed rollback transactions and delayed cache mutation until the directory commit succeeds.
- [x] **High / patch:** Persist newly created PID and namespace entries. Parent directories are now fsynced after each successful `mkdirat`.
- [x] **High / patch:** Reject numeric PID symlinks and non-directory hostile nodes during principal erasure instead of silently skipping them.
- [x] **Medium / patch:** Apply scan prefix, cache, and limit guards before decoding or deduplicating a disk key.
- [x] **Medium / patch:** Treat conflicting duplicate values with equal modification times as ambiguous and fail without deleting either.
- [x] **Medium / patch:** Surface failed temporary/backup cleanup alongside the primary pre-commit error; successful commits retain their result even if an unrecognized backup link cannot be reaped.
- [x] **High / patch:** Preserve the established I/O error variant and directory context for hostile namespace nodes so the Reza deterministic GDPR cascade can distinguish structural filesystem failure.

## Suggested Review Order

**Private spill durability and containment**

- Start at the atomic state transition: durable commit or complete rollback.
  [`private.rs:473`](../../crates/maos-kernel-core/src/memory/private.rs#L473)

- Parent fsync makes newly created PID and namespace entries crash-durable.
  [`private.rs:206`](../../crates/maos-kernel-core/src/memory/private.rs#L206)

- Equal-time conflicting legacy values fail closed before destructive deduplication.
  [`private.rs:323`](../../crates/maos-kernel-core/src/memory/private.rs#L323)

- Prefix, cache, and limit guards isolate scans before disk decoding.
  [`private.rs:685`](../../crates/maos-kernel-core/src/memory/private.rs#L685)

- Descriptor-relative erasure rejects hostile numeric PID nodes.
  [`private.rs:774`](../../crates/maos-kernel-core/src/memory/private.rs#L774)

**CI contract repairs**

- Five production environment reads join the central ownership registry.
  [`env_contract.rs:295`](../../crates/maos-bin/src/env_contract.rs#L295)

- Explicit fail-fast serialization replaces gate-forbidden implicit expectations.
  [`main.rs:338`](../../crates/maos-bin/src/main.rs#L338)

- Attestation and keyring encoders now retain contextual failure diagnostics.
  [`attestation.rs:134`](../../crates/maos-compliance/src/vetting/attestation.rs#L134)

- Test-only direct constructors carry narrow P1 ownership evidence.
  [`main.rs:12981`](../../crates/maos-bin/src/main.rs#L12981)

**Development-record provenance**

- One authoritative pre-development status policy includes blocked stories.
  [`check_dev_model_used_populated.rs:22`](../../xtask/src/check_dev_model_used_populated.rs#L22)

- Tier enforcement consumes the shared status policy rather than drifting.
  [`check_dev_model_tier.rs:115`](../../xtask/src/check_dev_model_tier.rs#L115)

- Markdown-table file lists count as populated provenance.
  [`check_dev_record_completeness.rs:209`](../../xtask/src/check_dev_record_completeness.rs#L209)

- Historical 13.5j review evidence records every resolved private-store finding.
  [`13-5j-private-tier-stale-spill-duplicate-scan.md:235`](13-5j-private-tier-stale-spill-duplicate-scan.md#L235)

**Supply-chain and governed evidence**

- The complete Wasmtime family resolves to patched 46.0.2 artifacts.
  [`Cargo.lock:5623`](../../Cargo.lock#L5623)

- Safe rustix filesystem operations are a direct, feature-scoped dependency.
  [`Cargo.toml:67`](../../crates/maos-kernel-core/Cargo.toml#L67)

- Ratified kernel surfaces preserve the intended service boundary.
  [`kernel-surface-v0.1-beta.json:697`](../../docs/ci-baselines/kernel-surface-v0.1-beta.json#L697)

- Measured physical and logical evidence moves without raising the ceiling.
  [`kernel-core-baseline.toml:438`](../../xtask/kernel-core-baseline.toml#L438)

**Behavioral proof**

- Excluded malformed spills cannot poison a matching prefix scan.
  [`private_spill_supersession_13_5j.rs:455`](../../crates/maos-kernel-core/tests/private_spill_supersession_13_5j.rs#L455)

- Equal-mtime conflicts remain intact and observable instead of losing data.
  [`private_spill_supersession_13_5j.rs:474`](../../crates/maos-kernel-core/tests/private_spill_supersession_13_5j.rs#L474)

- Rejected transactions preserve cache, disk, and temp-file hygiene.
  [`private_spill_supersession_13_5j.rs:500`](../../crates/maos-kernel-core/tests/private_spill_supersession_13_5j.rs#L500)

- Concurrent writers leave one complete durable value.
  [`private_spill_supersession_13_5j.rs:531`](../../crates/maos-kernel-core/tests/private_spill_supersession_13_5j.rs#L531)

- PID symlink erasure now fails closed while preserving the external target.
  [`private_forget_restart_13_5i.rs:258`](../../crates/maos-kernel-core/tests/private_forget_restart_13_5i.rs#L258)

- The Reza cascade corpus retains actionable structural filesystem diagnostics.
  [`gdpr_cascade_corpus_test.rs:342`](../../crates/maos-audit/tests/gdpr_cascade_corpus_test.rs#L342)
