---
Status: binding-v0.1
Phase: binding-v0.1
Gate: hot-path token verify <5µs P99 benchmark
Decided: 2026-04-15
Accepted-in-PR: <PR_NUMBER>
Revisits: §4.6
---

# ADR-030 — Capability Registry decomposition

**Decision.** The Capability Registry is internally split into four sub-services: `cap-tokens` (hot path, lock-free token issue/verify), `cap-policy` (consent rules + intent allowlist), `cap-audit` (transparency log writer, slow path), `cap-quota` (per-Spirit budget tracking). IAC traverses only `cap-tokens` on the hot path; the audit/lineage path is async via bounded MPSC.

**Rationale.** A monolithic Capability Registry is a god-service. Decomposing it preserves the Capability Registry as a single mediation surface from the Spirit-API perspective while internally separating the hot path from the slow path so audit writes do not block frame delivery.

**Alternatives considered.** Monolithic Capability Registry (rejected: serializes IAC hot path). Per-Spirit Capability Registry instances (rejected: cross-Spirit mediation becomes ad-hoc).

**What would force a revisit.** A new capability surface emerges that does not fit into the four sub-service shapes.

## Implementation note — Story 17-9

`cap-quota` now tracks **capability issuances**, not context tokens: one admitted
scheduler pid (one incarnation) receives **60,000 issuances per 60 s tumbling
window**, defined by `DEFAULT_ISSUANCE_QUOTA` using a monotonic clock. Capacity
is inclusive (exactly B), with a possible ≤ 2·B burst across one boundary.
Mediation runs **policy → quota → tokens**; only successful issuance is charged,
failed signing refunds its reservation, and later revocation does not refund it.

Kind-7 audit rows `cap.quota:pressure` (⌈80 %⌉),
`cap.quota:limit` (⌈95 %⌉), and `cap.quota:exhausted` (first refusal) are
operator-facing only, once per signal/pid/window; kind 7 is never
Spirit-recallable. Each signal gets at most one non-blocking enqueue attempt.
Rows are durable when written, not guaranteed under audit saturation or writer
closure; drops count under `AuditDropSite::Quota` and are never retried.

Exhaustion is `CapError::IssuanceQuotaExhausted`, mapped by the governed wrapper
to `GovernedMintError::QuotaExhausted`. Spawned-session `iac.send` records
verdict `exhausted`; recall/fetch return `capability-denied`; MCP writes
`mcp.call.refused:capability-denied`. CLI-wrapper spawn refuses on both local
and remote paths. D1's proceed posture is retained only for the absent
`proc.exec` grant, not exhaustion.

**Limits (not controls):** hot swap retains the pid's window; cold swap, crash
restart, unload/re-admission and every fresh Worker bind receive fresh capacity.
There is no stable-Spirit, principal-wide or Worker-spawn-churn bound. Other
governed callers expose `QuotaExhausted` in their string errors; `ShellHost`
stringifies the kernel error on the operator REPL. No operator- or
manifest-configurable budget is provided. No Spirit-facing issuance pre-warning
is provided: a Spirit learns exhaustion through each path's typed denial.
`ScheduleWatchdog` consumes its rate bucket before minting; its narrowed tokens
are audit attestations of authority at fire time, not hook authority — the hook
ABI carries no token.

The issuance quota's `cap.quota:*` signals are distinct from ADR-016's
`ContextPressure`/`ContextLimit`/`EContextExhausted` for Spirit-self-reported
context tokens versus manifest `[budget].context_window_size`. ADR-016 is not
implemented by 17-9; its owner is `18-5-context-token-budget-accounting`.
`CapError::ContextExhausted` remains available for that context-token error.
