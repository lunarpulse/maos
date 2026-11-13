# Epic 24 — Multi-Operator Control Plane (W9)

**Status:** `backlog` — created by correct-course 2026-11-13 (`sprint-change-proposal-2026-11-13.md` §3–§6, operator-approved 2026-11-13). Third epic of the Enterprise-Fit lane (Epics 22–26). The lane starts after `v1.0.0` (decision 1); one minor release per epic. `sprint-status.yaml` is authoritative for status.

**Wave:** W9; 6 stories, ≈ 8 median-story units `[INFERENCE]` · **Makes true:** A named operator and a policy decision on every door command; one inbox; approvals that wait for a human; budgets; automatic containment.

**Closes:** FR70 and NFR-Ops-13 (operator principal and door authorization), FR71 (inbox), FR72 (interactive approval), FR74 (policy-driven containment), FR73 and NFR-Cost-2 (budgets); runbook `docs/runbooks/ent-1-enterprise-daemon-posture.md:181` (PDP subject granularity is per-posture). Re-owned row: `RELEASE-HOLDS.md` v2.2 Reza claim boundary row 2 (`:76`, operator-axis authorization absent from the `MAOS_ONE_SHOT` legal-hold-release and collective-erase) → `24-1`.

**Hermetic exit command (CI, no secrets; red until green):**

```
cargo test -p maos-bin --test operator_plane_24   # 1 — a named operator and an operator-policy decision on every mutating door command; extended by each later Epic 24 story (24-1 AC1)
```

**Exit-block provenance (measured at `1a57035e`):**
- Line 1 — `operator_plane_24` is created by 24-1 AC1 and extended by each later Epic 24 story (applied clarification, 2026-11-13: the proposal names no creating story). **HEAD: red** — no such test target (`ls crates/maos-bin/tests` has no `operator_plane_24.rs`); `cargo test` exits non-zero. Green when every story's leg in that target passes.
- Shape: one `cargo test` line with a trailing `#` comment; no secret, key, human, wall-clock or network (the SSO assertion and the policy files are fixtures).

**Kernel-Δ:** `24-3` only, forecast 0…+120 tokei `[INFERENCE]`; the kernel row at HEAD is 20,429 tokei (`xtask/kloc.toml:264`) against the 23,000 envelope (`xtask/kernel-core-baseline.toml:514`). Every other story: forecast 0. Measured at landing.

**Lane rules (proposal §4, binding for Epics 22–26).** (1) Adapters stay outside the kernel behind existing ports; `24-3` is the only kernel-touching story. (2) No new `xtask` gate; an exit is a `cargo test` target or a `maos` verb created by a named AC. (3) One hermetic exit per epic. (4) Each external standard is pinned by version in its story, which re-reads the specification. (5) A control is wired or it is labelled. (6) Each epic is preflighted at the previous retrospective.

**Confidence rules.** Rules 1–11 are those printed in `epic-21-multi-host-on-live-substrate-w6.md` under **Confidence rules** and **Confidence rule 11**; they bind here and are not repeated.

**Decision 2 (proposal §3).** Interactive approval is a claim boundary at v1.0; `24-3` is built after GA and designed with no Spirit-contract change. The Spirit keeps receiving `capability-denied` and succeeds on retry once an operator has approved; the hold lives on the operator side. The Spirit contract's error variants are closed (`wit/spirit.wit:342-347, 381`). The unused auto-allow path is deleted inside 21-10a's I4 work, so v1.0 ships no approval code that is not real.

---

## Stories

| Key | Story | Closes · size |
|---|---|---|
| `24-1-operator-principal-and-door-authz` | Operator principal and door authorization | FR70, NFR-Ops-13, RELEASE-HOLDS v2.2 row 2 · L |
| `24-2-needs-you-inbox` | The needs-you inbox | FR71 (inbox) · M |
| `24-3-interactive-approval-hold` | Interactive approval hold | FR72 · L |
| `24-4-policy-driven-containment` | Policy-driven containment | FR74 · M |
| `24-5-per-spirit-pdp-subjects` | Per-Spirit PDP subjects | Runbook `ent-1…:181` · M |
| `24-6-budgets-as-policy` | Budgets as policy | FR73, NFR-Cost-2 · M |

AC1 is the command and what the operator observes. `bmad-create-story` re-measures every cite and may tighten, not add uncited ACs.

### 24-1-operator-principal-and-door-authz — Operator principal and door authorization

*Closes · Δ:* FR70, NFR-Ops-13, `RELEASE-HOLDS.md` v2.2 row 2 (`:76`) · size L · kernel-Δ forecast 0. Writes its part of the ADR-062 amendment.

- *HEAD at `1a57035e`:* one shared bearer, compared in constant time (`crates/maos-control/src/lib.rs:1064`); loopback only (`:697`, "operator HTTP must bind a loopback address").
- **AC1 (command).** Under SSO each door request carries an assertion and the principal is written into the completion row and the Approval Decision Log. This AC creates `cargo test -p maos-bin --test operator_plane_24` (exit line 1), which each later Epic 24 story extends (applied clarification, 2026-11-13).
- **AC2.** Cedar evaluates operator × verb × target before submit; a deny is a typed 403 and a journal row.
- **AC3.** Without SSO the single bearer is one operator, and the output says so.
- **AC4.** The legal-hold-release and collective-erase one-shots require the operator axis.

### 24-2-needs-you-inbox — The needs-you inbox

*Closes · Δ:* FR71 (inbox) · size M · kernel-Δ forecast 0.

- *HEAD at `1a57035e`:* three separate lists and no live halt read; the only halt route is `["v1", "halts", halt_id, "resolve"]` (`crates/maos-control/src/lib.rs:1247`).
- **AC1 (command).** `maosctl inbox` lists pending halts, skills, re-pins, approvals and budget halts, each with its decision verb.
- **AC2.** `--watch` polls.

### 24-3-interactive-approval-hold — Interactive approval hold

*Closes · Δ:* FR72 · size L · kernel-Δ forecast 0…+120 tokei `[INFERENCE]`, the lane's only kernel-touching story (lane rule 1).

- *HEAD at `1a57035e`:* an approval-required capability is refused (`crates/maos-kernel-core/src/capability/mod.rs:221-223`, `RequireApproval` → `CapError::PolicyDenied`). The auto-allow `ApprovalManager` is constructed at `crates/maos-bin/src/main.rs:3443` and its `prompt` has no production caller outside `crates/maos-kernel-core/src/security/approval.rs`.
- **AC1 (command).** A `RequireApproval` decision records a pending approval with a TTL; `maosctl approve <id> --intent "<why>"` or `deny` resolves it; the approval row names the operator and intent and is written before any mint.
- **AC2.** The Spirit receives the existing `capability-denied` and succeeds on retry inside the approved window; no contract change (decision 2).
- **AC3.** Expiry is a denial.

### 24-4-policy-driven-containment — Policy-driven containment

*Closes · Δ:* FR74 · size M · kernel-Δ forecast 0.

- *HEAD at `1a57035e`:* the Observer only notifies (`spirits/observer/src/lib.rs:370-383`: `AnomalyKind` and `ObserverSurface`, which converts to `NotificationEvent::AnomalyFlagged`).
- **AC1 (command).** A rule file maps a signal and threshold to `pause`, a posture shift, a revocation or `unload`; a reconciler submits the command attributed `policy:<rule>`.
- **AC2.** No rule, no action.
- **AC3.** A drift fixture with a rule stops the Spirit within one reconcile interval.

### 24-5-per-spirit-pdp-subjects — Per-Spirit PDP subjects

*Closes · Δ:* runbook `docs/runbooks/ent-1-enterprise-daemon-posture.md:181` ("PDP subject granularity is per-posture") · size M · kernel-Δ forecast 0.

- *HEAD at `1a57035e`:* `principal_attributes` exists on the request (`crates/maos-domain/src/ports/policy_decision.rs:112`) and is not modeled into Cedar (`crates/maos-pdp/src/adapter.rs:75-76`).
- **AC1 (command).** A governed collective operation is evaluated with the originating Spirit as subject and the principal's attributes in context (`crates/maos-domain/src/ports/policy_decision.rs:112`; unused today, `crates/maos-pdp/src/adapter.rs:75-76`).

### 24-6-budgets-as-policy — Budgets as policy

*Closes · Δ:* FR73, NFR-Cost-2 · size M · kernel-Δ forecast 0. Writes its part of the ADR-046 amendment.

- *HEAD at `1a57035e`:* `PolicyTable::update()` (`crates/maos-kernel-core/src/capability/cap_policy/mod.rs:251`) replaces the operator policy, whose `per_spirit_capability_deny` (`:42`) is read at `:141-144`.
- **AC1 (command).** A policy file declares budgets; a reconciler sums cost frames and on breach adds a per-Spirit deny for inference through `PolicyTable::update()` (`crates/maos-kernel-core/src/capability/cap_policy/mod.rs:251`; field `:42`, read `:143`) and raises a halt.
- **AC2.** An authorized override raises the budget and writes the approval row.
- **AC3.** The overshoot bound is stated and tested.

## Dependencies

- Epic 23 is done and its retrospective preflights this epic (lane rule 6).
- `24-1` comes first: it creates the exit target and the operator principal; `24-3` AC1 and `24-6` AC2 write approval rows that name an operator.
- `24-6` reads spend from cost frames; the spend figure comes from story S2 of the reasoning-model adapter, which lands before GA.
- `24-3` waits for nothing on the Spirit contract (decision 2). The auto-allow path is removed earlier, inside 21-10a's I4 work.
- ADR-062 amendment: `22-4`, `24-1`. ADR-046 amendment: `24-6`, `26-3`.
