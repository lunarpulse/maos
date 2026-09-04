# Epic 15 — W0: Green at HEAD, One Phase, First Signed Tag

**Status:** `backlog` — created 2026-09-04 by `sprint-change-proposal-2026-09-04.md` (bmad-correct-course, Lunarpulse-ratified). One of six recovery epics (15–20); `sprint-status.yaml` is authoritative for status.

**Wave / duration:** 1 week · **Makes true:** release discipline (prerequisite for every phase)

**Closes:** 4 red aggregate gates + 1 deterministic test red; three `CURRENT_PHASE` sources; the retired-model incident class (hotfix lane); zero release tags

**Exit command (the epic is done when an operator runs this on a clean machine and sees what it says):**

```
git tag v0.1.0-alpha.1 && git push --tags && gh release view v0.1.0-alpha.1   # Ed25519-signed SHA256SUMS present; discipline aggregate green on tree content
```

**Kernel-Δ:** **ZERO kernel-Δ @24472** by default. If an I9 whitelist fix needs a kernel line, FLAG-Winston and measure at landing.

**Dev-gate:** external holds (pen-test NFR-Sec-7 + export counsel NFR-Comp-1) = GA ledger only, non-gating for dev. **Model/review discipline:** frontier-class dev allowlist + §A6 full-layer review net is the binding control (E11 retro A1). **Story rules (CP-2):** AC1 of every story is a command plus what the operator observes; a story splits once; story files ≤3,000 words; governance work ≤20% of the epic.

---

## Stories

| Key | Story | Closes |
|---|---|---|
| `15-1-green-at-head` | Green at HEAD | S4 (partly), D17 |
| `15-2-single-phase-source` | One phase source | S4, E12-B1 residual (D20) |
| `15-3-first-signed-tag` | First signed tag | S5 |

Per-story ACs below are sketches; `bmad-create-story` finalizes them at preflight (≤6 ACs, AC1 unchanged in kind).

### 15-1-green-at-head — Green at HEAD

*Closes:* S4 (partly), D17

- **AC1 (command).** `cargo run -p xtask -- check-empty-kernel && cargo run -p xtask -- check-service-boundary && cargo run -p xtask -- kloc-check && cargo run -p xtask -- check-env-contract && cargo test --workspace --no-fail-fast` all exit 0 on tree content, and the operator sees the discipline `aggregate` job green on a push with no gate edited to advisory.
- **AC2.** I9: the two undocumented `#[i9_exempt]` are documented in `docs/invariants/i9-exemptions.md`; `ScbRuntimeSnapshot`, `SecurityManagerAdapter`, `VerifiedImageLock` are either whitelisted with a written rationale or made non-persistent.
- **AC3.** NFR-Test-2: the nine kernel symbols in class `other` are classified in `xtask/kernel-api-classes.toml` or made private; the four P3 violations closed.
- **AC4.** kloc: `maos-domain` +51 is a measured grant or a trim; `_aggregate_hardfail` re-derived (register D17 closes here).
- **AC5.** `MAOS_OPERATOR_BEARER_TOKEN` / `MAOS_OPERATOR_HTTP_BIND` registered (`main.rs:2669-2670`).
- **AC6.** `t_14_2a_post_grace_journal.rs:362` passes under default test threads (per-test `tracing::subscriber::with_default`, not `--test-threads=1`).

### 15-2-single-phase-source — One phase source

*Closes:* S4, E12-B1 residual (D20)

- **AC1 (command).** `grep -rn "CURRENT_PHASE" xtask/src tests | grep -c "const CURRENT_PHASE"` prints `1`; every gate reads `gate_common::CURRENT_PHASE`; `tests/phase-config.toml` is generated from it or deleted.
- **AC2.** `check_escape_detector.rs` uses `BindingClass` like the other 11 advisory gates (register D20 partial).
- **AC3.** `coverage_matrix.rs:98` reads the same source; proven-red: flipping the constant reds exactly the expected gate set and nothing else.

### 15-3-first-signed-tag — First signed tag

*Closes:* S5

- **AC1 (command).** `git tag v0.1.0-alpha.1 && git push --tags` runs `release.yml` for the first time; `gh release view v0.1.0-alpha.1` shows `SHA256SUMS` + Ed25519 signature; the verify step passes on a clean host.
- **AC2.** `Cargo.toml` workspace version = `0.1.0-alpha.1`; `STABILITY.md` regenerates unchanged (LTS clock still starts at `1.0.0`).
- **AC3.** `docs/release/` carries the tag procedure; rule recorded: a tag is cut at the close of every epic from 15 on.

## Dependencies

None. The three model-pin hotfix rows (`model-pin-currency-gate-and-retired-pin-sweep`, `provider-model-env-overrides`, `provider-error-surfacing`, approved 2026-09-01) run in parallel inside this epic's window and keep their keys (W0-3).

## Not in this epic

Anything whose only deliverable is a gate, ledger, registry or ceiling and is not named above. v2.5 parking rows (`v25-*`) stay parked.
