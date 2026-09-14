# Story 16-1 AC4 fixture spike — preserved evidence

Run 2026-09-13 at `f72b557b` in a throwaway git worktree, before dev. Nothing here is merged code. The story's §17
spike table cites these files. T0 re-runs every probe rather than trusting them.

| File | What it is |
|---|---|
| `spike_16_1.diff` | The throwaway hook (gated on `MAOS_SPIKE_16_1`, placed just before the shutdown `select!` in `main.rs`) that calls the daemon's own `capability`, `revocation_applier`, `upgrade_orchestrator` and `scheduler`, standing in for a door handler. It also holds the 3-line `maos-capability` re-revoke change measured for D-16-1-V. |
| `spike_crl_fixture_16_1.rs.txt` | The ≈45-line signed-CRL fixture (D-16-1-X, AC4 root B). The Ed25519 seed is the test constant `[42u8; 32]`, not a secret. |
| `fixtures/butler-0.3.1.toml` | The upgrade target (butler's manifest, `version = "0.3.1"`, empty `[scheduling]`/`[lifecycle]`). The other three files are negative variants: missing sections, a downgrade to 0.2.0, and a same-version target. |
| `run_butler.sh`, `drive_token.sh` | Launchers with isolated `HOME`/`MAOS_HOME`/`XDG_DATA_HOME`/`MAOS_AUDIT_DB`. |
| `revokes.txt` | A grep of every `revoke` caller (the D-16-1-V caller audit). |
| `spikeD4.out` | Researcher replay: the `cap.issue` row read from the TL mid-run (uppercase hex) and the lowercase token id derived from it (Q1, Trap 18). |
| `logs/<run>/std{out,err}.log` | Per-run daemon output. `run1`/`run2`/`runR`: butler replay lifetime (Q1/Q4). `spikeA`: butler token probe. `spikeB`: no butler successor loader. `spikeB2`–`B6`: upgrade to 0.3.1 and the 0.2.0 downgrade (Q3). `spikeC`: CRL `matched_count 1` (Q2). `spikeD`–`D4`: revoke, re-revoke `Revoked`, `UnknownToken` (Q1). `spikeW`: the successor watch. `os_*`: the one-shot arms (panics in debug builds). TL sqlite files were not preserved (binary, reproducible). |

**Gap, stated:** no preserved log records a `MAOS_IDLE_FAST=1` run. The faithful-successor halt was measured at
default timing, 30.007 s after the swap. AC4 root E's fast-idle timing is therefore unmeasured, and T0 measures it.
