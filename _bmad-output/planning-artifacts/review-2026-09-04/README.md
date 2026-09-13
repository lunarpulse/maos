# Full review 2026-09-04 — scout evidence (HEAD 4657cace)

Published report: **MAOS Promise Ledger** — https://claude.ai/code/artifact/e9b88de1-d01d-4e6b-94c9-932810ca82c5
Proposal: `../sprint-change-proposal-2026-09-04.md`

Eight read-only scouts, one axis each. Every reality claim cites `file:line` at `4657cace`; prose in `_bmad-output/` was used only as the statement of the promise.

| File | Axis |
|---|---|
| 01-process-scout.md | git/release reality, artifact + planning footprint, Epic-14 shape, process-weight signals |
| 02-build-ci-scout.md | cargo check, full `cargo test --workspace`, 12 gate runs, gate registry, discipline.yml |
| 03-kernel-size-scout.md | per-crate LOC vs ceilings, kernel pin instruments, maos-bin shape, env reads, dead crates, duplication |
| 04-security-ops-scout.md | unsafe, deny, secrets, retired pins, TOFU/rotation/revocation, sandbox tiers, install/uninstall, air-gap, fuzz |
| 05-fr1-20-scout.md | FR1–20, FR47–51 classification (LIVE/PARTIAL/TEST-ONLY/CANNED/ABSENT) |
| 06-spirits-journeys-scout.md | 11 Spirits (LLM class, hooks, launch path, model pin), 8 journeys (class, command, entry) |
| 07-nfr-scout.md | 26 load-bearing NFRs: measurement, gate, posture, floor met/relaxed/unmeasured; coverage-matrix counts |
| 08-fr21-65-scout.md | FR21–46, FR52–65 classification |

Not executed: Postgres-dependent (107) and live-key (60) ignored tests; no paid model calls.

**Concurrency note.** During scouting another session was developing Story 14-2c in this working tree (20 files, +1114/−136 uncommitted; `main.rs`, `cert_rotation.rs`, `maos-control/src/lib.rs`, `cohort/state.rs`, `kloc.toml` among them). Structural findings were re-verified against committed `HEAD 4657cace` via `git show`; line numbers cited into the in-flight files may be offset. Use `git show 4657cace:<path>` when re-checking a citation.
