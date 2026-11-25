# Story 23-7 evidence (implementer: anthropic/claude-sonnet-5-5; reviewer must be another family)

## AC map

| AC | Proof | Result |
|---|---|---|
| AC2 a,b,c | `crates/maos-bin/tests/self_leaf_rotation_23_7.rs` (real daemon; polls an operator read, order and end state only) | pass |
| AC2 d | `maos-a2a-tcp/tests/peer_rotation_23_7.rs::a_swap_moves_plane_c_...` | pass |
| AC2 e | `maos-cohort/tests/cert_rotation_14_2a.rs::the_local_leaf_observer_runs_off_the_cached_lock` (in-process gate) | pass |
| AC3 | cohort `t_grace_matches_...` (equality on G) + `peer_rotation_23_7::g_covers_one_pull_lag_...`; daemon leg fixture `t_stale_secs = 30` (G = 20 s) | pass |
| AC4 | `t_14_2a_post_grace_journal.rs` (dial, listen under `required`, never-promoted, SAN-less); frame bridge; `"063"` gate arm | pass |
| AC5 | `san_scoping_23_7.rs` + `san.rs` unit test | pass |
| AC6,AC7 | `peer_rotation_23_7.rs` on the real `FilePinStore`; cohort floor test | pass |
| AC8 | `maos-persistence/src/lib.rs` unit tests (rename fail, dir-sync fail, crash after pre-fence, v1 load) | pass |
| AC9 | see "hosted" and "runs" | partial |
| AC10 | ADR-063 amendment, runbook `docs/runbooks/peer-identity-rotation.md`, stale docs rewritten, RELEASE-HOLDS rows | done |

## Runs (local)

- Full suites green: maos-persistence, maos-a2a-core, maos-a2a-tcp, maos-cohort, maos-domain, maos-wasm-host (fixtures built with `build-test-fixtures`): 79 test binaries `ok`.
- maos-bin (`--features network`, random 0600 `MAOS_AUDIT_KEY`): `self_leaf_rotation_23_7` (2), `cert_rotation_trigger_14_2a`, `durable_pins_21_3`, `t_14_2b`, `t_14_2c`, `region_reconcile_22_1`, `bounded_postures_2b`, `cross_team_crossing_13_6b`, `repin_door_19_7`: all ok.
- New in-process tests (a2a-tcp scenes, SAN, post-grace, cohort file): 10x under 16 busy loops: 0 failures. Daemon legs (`self_leaf_rotation_23_7`, `cert_rotation_trigger_14_2a`, `durable_pins_21_3`): 3x under 16 busy loops: 0 failures (`t_14_2_rotation_scene`, `t_10_4b_rotation_real_timing` untouched by code; not re-run 3x).
- The own-leaf reconciler ticks every `confirmation_interval()` (15 s at the fixture's lease); a separate process has no clock hook, so the daemon legs poll with a 120 s outer deadline and never assert elapsed time.

## Build shapes (`cargo check --locked -p maos-bin --lib --bins --tests`)

| Shape | Result |
|---|---|
| default | ok |
| `--features network` | ok |
| `--features wasm-host` | ok |
| `--no-default-features` | lib + bins ok; test targets fail in untouched files (`eval_halt_18_2`, `epic_dispatch_19_1`, `epic_args_19_1`, ...): not caused by this story (new test files carry `#![cfg(feature = "network")]`) |
| `--no-default-features --features network,wasm-host` | ok |

Not run: the same shapes on the base (no worktree allowed); the `--no-default-features` test failures are in files this story does not touch.

## Gates

`check-composition-root-completeness` pass; `check-env-contract` pass, 162 registered (no new `MAOS_*`); `check-dependency-closure` pass; `check-kernel-baseline` pass (28,339 = pinned 28,339 on this tree; no kernel file touched); `kloc-check` pass (`over_budget: []`); xtask test `decision_adrs_and_provisioning` 3 passed; `cargo fmt --all -- --check` clean (formatter applied to touched files). `check-cert-rotation-trigger` is NOT a registered gate name in `gate-registry.toml` here ("not found"): not run. `cargo deny check`: `bans FAILED` on duplicate `const-oid` 0.9.6/0.10.2: both were already in the base `Cargo.lock`; advisories, licenses and sources ok; x509-cert adds no ban hit. Clippy: no new warnings in touched code except a `needless-loop-counter` style note in `san_scoping_23_7.rs`; pre-existing warnings elsewhere.

## Dependency tree

`x509-cert` 0.2.5 (default features off), `der_derive` 0.7.3, `flagset` 0.4.7; `der` gains `derive`/`flagset`. Standalone lockfiles: no `fuzz/Cargo.lock` or guest lockfile includes a2a-tcp, a2a-core, cohort or persistence (grep), so none gains the edge.

## kloc (tokei, `src` only; base = `HEAD`)

| Crate | Base | Now | Delta | Forecast |
|---|---|---|---|---|
| maos-a2a-tcp | 1508 | 1838 | +330 | +90..+180 (over; includes `san.rs` and in-file tests) |
| maos-a2a-core | 5265 | 5323 | +58 | +30..+70 |
| maos-cohort | 5873 | 5983 | +110 | +40..+90 (over by 20) |
| maos-persistence | 442 | 610 | +168 | +40..+90 (over; fence tests in `src`) |
| maos-domain | 11178 | 11179 | +1 | +3..+8 (under) |
| maos-wasm-host | 2289 | 2290 | +1 | +1..+3 |
| maos-bin | 46426 | 46546 | +120 | +100..+220 |

No ceilings apply (decision 11). Kernel unchanged.

## Proven-red plants (marker `E237-PLANT`)

All six were applied at once, then the owning test binaries were run once; each binary failed (details in `proven-red/`). All plants were removed; `grep -rn E237-PLANT crates .github docs RELEASE-HOLDS.md Cargo.lock xtask` is empty, and the suites were green again afterwards. AC6's plant was not run (see deviations).

## Deviations

1. `PinPersistence::store` signature changed (`floor` added; `store_narrowing`, `load_floor` added), so existing test ports and `store(&..)` call sites, `PinRecord` literals and the "unknown version 2" fixture (now 3) were edited mechanically.
2. `TcpA2AConfig` gained `peer_san`, `peer_trust_domain`: 14 literals edited.
3. AC3 scene: X swaps one pull lag after P1 applies. The AC text says the plant (base grace only) fails "the scene's P2 assertion"; here it fails the P1-before-X-swap assertion (P1's window closed before X swapped) and the cohort `armed_windows == G` equality. The property the AC protects (G covers a pull lag) is asserted; the literal "P2 assertion" is not. Acceptance between X's swap and P2's apply cannot be asserted (P2 has not applied the reissue yet; the protocol refuses by design).
4. `RELEASE-HOLDS.md` rows edited by the implementer (AC10(b) assigns them to the integration owner).
5. `maos-persistence` is a dev-dependency of `maos-a2a-tcp`.
6. Plants were batched (one build, one run per owning test binary) instead of one run per plant; per-plant attribution is by the AC's owning test, not isolated runs. AC6's plant ("accept a leaf on first sight after a restart") was not run.
7. The dial-side `CertPostGraceReject` subject is the dial target (`expected_peer`), not read back from the io error; the listen side reads it from the typed `SubjectRefusal`.
8. `cargo fmt` was run to fix formatting of touched files.
9. `check-cert-rotation-trigger` is not a registered xtask gate in this tree.

## Hosted-only / not verified

macOS (`aarch64-apple-darwin`) compile; deb/rpm/arch, systemd, cgroup and air-gap netns legs (untouched, not run); first hosted run id; base-tree build-shape comparison.

## Binary size (STOP condition)

`cargo build --locked --release -p maos-bin` then `strip`: **39,070,192 B** vs limit 39,069,022 B (+1,170 B over; main was 38,911,056 B, so this story adds about 159,136 B). Nothing was raised; reported for the orchestrator. Measured here with the default release profile, strip via binutils `strip -o`; the xtask size gate was not run.
