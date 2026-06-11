# Patch Context for Story 8-15 Code Review Fixes

## Constraints
- **Zero kernel KLOC**: `maos-kernel-core/src/` MUST stay byte-identical to 8563eb4. NO edits to kernel-core.
- **Frozen import surface**: Do NOT rename/remove existing `pub` items in `maos-journey-test/src/lib.rs`. Adding new pub items is OK.
- **Workspace stays 44**: No new crates.
- **Anti-tautology**: Oracles must be external — screen render, TL rows, MockMcp writes. Never assert what the harness inserted.
- **Constant oracles only**: Oracle strings must trace to named constants in production code.
- **H-guards**: H4 = readiness-not-sleep, H5 = bounded timeouts (≤250ms steps), H3 = ephemeral port+readback.

## Key Files
- Spec: `_bmad-output/implementation-artifacts/8-15-journey-acceptance-test-harness-and-red-phase-suites.md`
- Harness: `crates/maos-journey-test/src/lib.rs` (532 lines)
- Replay provider: `crates/maos-bin/src/cassette_replay.rs` (345 lines)
- Age gate: `xtask/src/cassette_age_gate.rs` (125 lines)
- Env contract: `xtask/src/check_env_contract.rs` (100 lines)
- Nightly CI: `.github/workflows/journey-nightly.yml` (45 lines)
- Discipline CI: `.github/workflows/discipline.yml`

## Architecture
- `CassetteReplayPort` in maos-bin: reads cassette file, serves responses by sequence index
- `CassetteRecordPort` in maos-bin: wraps a real InferencePort, records entries to cassette on drop
- `ReplayProvider` in journey-test: copies a fixture cassette to a temp file, exposes cassette_path() for env seam
- `MockMcp` in journey-test: spawns HTTP server on 127.0.0.1:0, serves fixture responses, exposes writes() oracle
- `Pty` in journey-test: spawns real maos binary via portable-pty, collects output, provides vt100 screen
- `AuditDb` in journey-test: creates tempdir for MAOS_HOME/XDG_DATA_HOME, provides transparency_log_path()

## Existing Patterns
- Readiness polling (JB-1/JB-2): `loop { let s = pty.screen(); if s.contains(target) { break; } std::thread::sleep(Duration::from_millis(200)); }` with iteration counter
- Subprocess test (JB-3 pattern): `Command::new(maos_bin()).env(...).output()` with isolated env
- Guards meta: `crates/maos-journey-test/tests/guards_meta.rs` calls `assert_no_wallclock_or_fixed_sleep` per file
