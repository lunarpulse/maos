# Epic 25 — Proof You Can Run (W10)

**Status:** `backlog` — created by correct-course 2026-11-13 (`sprint-change-proposal-2026-11-13.md` §4–§6, operator-approved 2026-11-13). Fourth epic of the Enterprise-Fit lane (Epics 22–26). The lane starts after `v1.0.0` (decision 1); one minor release per epic. `sprint-status.yaml` is authoritative for status.

**Wave:** W10; 2 stories, ≈ 2.75 median-story units `[INFERENCE]` · **Makes true:** A buyer verifies the guarantees on their own install and reads a control map.

**Closes:** FR75 (an installed Host verifies its own guarantees and writes a signed report), NFR-Comp-6 (every control-map row cites an invariant or gate and an evidence command).

**Hermetic exit command (CI, no secrets; red until green):**

```
maos verify --offline   # 1 — on the built artifact, exit 0 with every verdict printed (25-1 AC1)
```

**Exit-block provenance (measured at `1a57035e`):**
- Line 1 — `maos verify --offline` is created by 25-1 AC1. **HEAD: red** — no such verb: the `maos` verb table has no top-level `verify` (`crates/maos-bin/src/verbs.rs:92-140`: `init`, `run`, `shell`, `audit`, `traceback`, `backup`, `install`, `purge`, `eval`); `verify` exists only as `maos backup verify` (`crates/maos-bin/src/main.rs:1607`). Green when the built artifact exits 0 and prints every verdict.
- Shape: one `maos` line with a trailing `#` comment; no secret, key, human, wall-clock or network (`--offline`, 25-1 AC3). `check-exit-commands` reads the verb as owed by the open story named on this line.

**Kernel-Δ:** forecast 0 (lane rule 1: `24-3` is the only kernel-touching story; the proposal's §6 prints no separate figure for this epic).

**Lane rules (proposal §4, binding for Epics 22–26).** (1) Adapters stay outside the kernel behind existing ports; `24-3` is the only kernel-touching story. (2) No new `xtask` gate; an exit is a `cargo test` target or a `maos` verb created by a named AC. (3) One hermetic exit per epic. (4) Each external standard is pinned by version in its story, which re-reads the specification. (5) A control is wired or it is labelled. (6) Each epic is preflighted at the previous retrospective.

**Confidence rules.** Rules 1–11 are those printed in `epic-21-multi-host-on-live-substrate-w6.md` under **Confidence rules** and **Confidence rule 11**; they bind here and are not repeated.

---

## Stories

| Key | Story | Closes · size |
|---|---|---|
| `25-1-maos-verify` | `maos verify` | FR75 · L |
| `25-2-control-map` | Control map | NFR-Comp-6 · S–M |

AC1 is the command and what the operator observes. `bmad-create-story` re-measures every cite and may tighten, not add uncited ACs.

### 25-1-maos-verify — `maos verify`

*Closes · Δ:* FR75 · size L · kernel-Δ forecast 0.

- *HEAD at `1a57035e`:* no top-level `verify` verb (`crates/maos-bin/src/verbs.rs:92-140`). The sandbox-escape corpus is `crates/maos-eval/fixtures/t3-escape-corpus-v1` (capability, filesystem, network, process and runtime escape sets).
- *Input from Epic 21 (correct-course 2026-11-13):* `21-10-invariant-ledger-proofs` writes each predicate as a callable function, so this story reuses it.
- **AC1 (command).** `maos verify --offline` on the built artifact exits 0 with every verdict printed (exit line 1). `maos verify` runs the 21-10 predicates, the sandbox-escape corpus (`crates/maos-eval/fixtures/t3-escape-corpus-v1`) and audit-bundle verification, and writes a signed report with version, posture and each verdict.
- **AC2.** A leg that cannot run is printed SKIPPED with its reason and fails the run unless allowed.
- **AC3.** It needs no network.

### 25-2-control-map — Control map

*Closes · Δ:* NFR-Comp-6 · size S–M · kernel-Δ forecast 0. Clause numbers are read from the primary texts at story creation (lane rule 4).

- *HEAD at `1a57035e`:* `STABILITY.md:77-83` maps SOC 2, ISO 27001 and FedRAMP only (heading `:77`, table `:79-83`).
- **AC1 (command).** `docs/compliance/control-map.md` adds EU AI Act record-keeping and human oversight, ISO/IEC 42001, NIST AI RMF and AIUC-1; each row names the mechanism, the invariant or gate, the evidence command and what the operator still owns. Clause numbers are read from the primary texts at creation.
- **AC2.** An in-crate test enforces NFR-Comp-6.

## Dependencies

- Depends on `21-10-invariant-ledger-proofs` (its predicates are the verdicts `25-1` runs).
- Epic 24 is done and its retrospective preflights this epic (lane rule 6).
