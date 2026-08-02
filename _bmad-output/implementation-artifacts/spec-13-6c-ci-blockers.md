---
title: 'Fix Story 13.6c live CI blockers'
type: 'bugfix'
created: '2026-08-02'
baseline_commit: 'c571a2b9109cef5def9e5bcea4b52ec0209a3859'
status: 'done'
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/implementation-artifacts/epic-13-context.md'
  - '{project-root}/_bmad-output/implementation-artifacts/13-6c-three-team-three-region-substrate.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** GitHub Actions run 30734112045 proves the new per-team Postgres provisioning works, but `check-multi-tenant-loom` and `check-reza-production-path` remain blocking RED. The live tenant-boot fixture still signs a pre-V4 manifest with no host-team identity, while the two-daemon crossing test incorrectly uses the Spirit-facing guarded read API as a physical database oracle.

**Approach:** Repair only the stale test fixtures/oracles: make the tenant-boot manifest a valid signed V4 roster with `host-a → team-a` and `host-b → team-b`, and observe two-daemon physical landing/absence through raw Postgres plus the store-internal provenance reader. Preserve production fail-closed behavior and commit the verified change without a co-author trailer.

## Boundaries & Constraints

**Always:** Keep `MAOS_LOOM_HOME_TEAM` reconciliation fail-closed; retain the real daemon, signed-manifest, TLS, consent, transport, and per-team database paths; make physical assertions against the actual team-A/team-B databases; keep Story 13.6c and sprint state `in-progress` until a subsequent GitHub-hosted run supplies green evidence.

**Ask First:** Any production-code change, cohort schema behavior change, gate disposition/threshold change, or scope beyond the two failing live-test fixtures.

**Never:** Remove `MAOS_LOOM_HOME_TEAM`; accept missing or mismatched signed team identity; weaken tenant-map guards; inject a fake map merely to make `LoomLiteStore::read` pass; relax the 30 ms SLO; suppress, ignore, or reclassify a blocking gate; add `Co-authored-by` or equivalent co-author metadata to the commit.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Tenant daemon boot | Signed V4 manifest maps host-a to team-a; env names team-a and its DB | Daemon reaches the listening state and the live route remains isolated to team A | Missing/mismatched team continues to refuse before listening |
| Two-daemon crossing | Team A emits PID 7 row to consented team B over real transport | Raw physical oracle finds the expected row/value in team B and the originated row in team A | Daemon exit or timeout remains a hard test failure |
| Physical witness | Probe stores have no tenant map because the assertion is not Spirit-facing | `read_all_rows_from` observes physical rows without bypassing any production port | Do not call guarded `read` or construct a misleading global PID map |

</frozen-after-approval>

## Code Map

- `crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs` -- builds the signed daemon manifest used by the shared failing tenant-boot leg.
- `crates/maos-bin/tests/cross_team_crossing_13_6b.rs` -- owns the two-daemon live crossing and its currently invalid post-transport read oracle.
- `crates/maos-bin/src/cross_team_crossing.rs` -- production host/team reconciliation contract; must remain unchanged.
- `crates/maos-loom-lite/src/store.rs` -- separates guarded Spirit reads from `read_all_rows_from`, the internal physical/provenance oracle.
- `xtask/src/check_multi_tenant_loom.rs` and `xtask/src/check_reza_production_path.rs` -- gate wrappers used for final live verification.
- `crates/maos-bin/Cargo.toml` -- declares the existing `tokio-postgres` crate directly for the raw test-only client.

## Tasks & Acceptance

**Execution:**
- [x] `crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs` -- upgraded the fixture manifest to schema V4 and signed explicit member teams aligned with the existing team roster.
- [x] `crates/maos-bin/tests/cross_team_crossing_13_6b.rs` -- added the canonical raw Postgres connection helper and replaced guarded probe-store reads with `read_all_rows_from` physical row assertions.
- [x] `crates/maos-bin/Cargo.toml` -- declared `tokio-postgres` as a direct dev dependency required by the raw physical oracle.
- [x] `_bmad-output/implementation-artifacts/spec-13-6c-ci-blockers.md` -- recorded implementation verification; final commit follows adversarial review and must contain no co-author trailer.

**Acceptance Criteria:**
- Given the live tenant-boot fixture and real team-A/team-B databases, when the ignored exact test runs, then host-a boots as team-a and the existing wrong-database refusal still fails closed.
- Given two real daemon processes and an allowed A→B share, when the crossing exact test runs, then the value lands in team B, the origin remains in team A, and neither assertion relies on the Spirit-facing guarded read path.
- Given live Postgres substrate, when both gate wrappers run, then neither reports the fixture failures from run 30734112045.
- Given the implementation diff, when it is reviewed, then it contains test-fixture/support changes only and is ready for one final commit without co-author metadata.

## Design Notes

The raw oracle is intentional, not a tenant-guard bypass. The production daemons still construct guarded stores from the verified manifest and perform the real crossing. The parent test process only asks whether bytes are physically present in each database, which is the same independent witness used by `cross_region_live.rs` and `tenant_wall_live.rs`. A fake tenant map would be incorrect because PID 7 denotes a team-A origin row copied into team B; a global PID-to-team lookup cannot truthfully satisfy both physical stores.

## Verification

**Commands:**
- `cargo fmt --all -- --check` -- expected: no formatting drift.
- `cargo test -p maos-bin --features network --test cohort_daemon_smoke_13_5c tenant_mode_boots_on_live_substrate -- --ignored --exact --nocapture` -- expected: one live tenant-boot test passes.
- `cargo test -p maos-bin --features network --test cross_team_crossing_13_6b live_crossing_runs_through_two_daemon_processes -- --ignored --exact --nocapture` -- expected: one live two-daemon crossing test passes.
- `cargo run -q -p xtask -- check-multi-tenant-loom --json` -- expected: no blocking RED legs.
- `cargo run -q -p xtask -- check-reza-production-path --json` -- expected: no blocking RED legs.

**Observed 2026-08-02 against local Postgres 17:**
- Tenant boot exact test: 1 passed, 9 filtered.
- Two-daemon crossing exact test: 1 passed, 17 filtered.
- `check-multi-tenant-loom --json`: `oracle_green=true`, `passed=true`; all 96 legs attempted and green.
- `check-reza-production-path --json`: `oracle_green=true`, `passed=true`; all 75 legs attempted and green.
- `cargo fmt --all -- --check`: passed.
- Parallel Blind/Edge review produced two patch findings: require a parsed non-empty destination re-attestation marker with an empty source marker, and generate a per-run key that stale rows cannot satisfy. Both were applied.
- The first hardened run reded on the actual crossed namespace (`xteam:team-a:`), proving the earlier process-ID-only oracle had false-greened on stale evidence. After correcting the exact namespace witness, the exact crossing test and all 96 multi-tenant gate legs passed.

## Suggested Review Order

**Physical crossing proof**

- Start with the live two-daemon behavior and its independent physical witness.
  [`cross_team_crossing_13_6b.rs:1199`](../../crates/maos-bin/tests/cross_team_crossing_13_6b.rs#L1199)

- Raw clients deliberately avoid the Spirit-facing tenant guard.
  [`cross_team_crossing_13_6b.rs:956`](../../crates/maos-bin/tests/cross_team_crossing_13_6b.rs#L956)

- Per-run keys prevent persistent databases from supplying stale evidence.
  [`cross_team_crossing_13_6b.rs:1251`](../../crates/maos-bin/tests/cross_team_crossing_13_6b.rs#L1251)

- Destination re-attestation and source-origin markers distinguish the two physical rows.
  [`cross_team_crossing_13_6b.rs:1275`](../../crates/maos-bin/tests/cross_team_crossing_13_6b.rs#L1275)

**Signed tenant identity**

- Canonical V4 member teams let host-a truthfully boot as team-a.
  [`cohort_daemon_smoke_13_5c.rs:102`](../../crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs#L102)

**Test support**

- The raw oracle declares its already-workspace-resident client as a direct dev dependency.
  [`Cargo.toml:107`](../../crates/maos-bin/Cargo.toml#L107)

- Epic context records the inherited security and completion boundaries.
  [`epic-13-context.md:1`](epic-13-context.md#L1)
