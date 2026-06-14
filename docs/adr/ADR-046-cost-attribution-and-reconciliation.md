# ADR-046: Cost Attribution and Reconciliation (FR64 / NFR-Cost-1)

## Status

**Proposed** — stub authored at Story 9.3b party-mode preflight (2026-06-13, Winston·Murat·John·Amelia). To be ratified/finalized at Story 9.3b Task 0 before implementation.

## Context

FR64 requires per-Spirit per-task per-principal cost attribution in the Transparency Log; NFR-Cost-1 requires ≥98% reconciliation against provider billing, sampled monthly. Today `TokenUsage` + `ProviderAttribution` are returned by providers then **discarded** at the inference emission site (`crates/maos-kernel-core/src/inference/mod.rs:350`); there is **no pricing model** anywhere; `task` and `principal` are not in the inference ABI. Reconciliation "against provider billing" is untestable in CI as literally written (no real invoices in CI, no `$` basis) — the same untestable-non-oracle failure mode killed in [ADR-028](ADR-028-replay-determinism-trace-shape.md)/Story 9.2b (F3).

## Decision (preflight consensus — to ratify)

1. **Capture at emission (authorized kernel delta).** Journal `TokenUsage` + `ProviderAttribution` as a cost-attribution frame (RateLimited frame is the emission template). Minimal additive kernel-core delta (~70–120 LOC est.), re-pinned jointly with FR62 (FLAG-Winston), recorded as the first ratified `AbiExtensionProposal` (ADR-045 F6).
2. **F8 — attribution-at-emission, never caller-supplied.** `principal_id` is a security-attribution field; a Spirit must not be able to populate its own (spoofing). **Principal** is lineage-derived via `transparency_log.principal_ids_for_spirit_pid` (verified reachable); sec-redteam rules on the Vec/single policy and the memory-write-proxy-vs-session-principal semantics. **Task** is an additive `task_ref` **correlation-id** (verified: lineage cannot resolve the current task; the SCB has no current-task marker) — not an authz field; `abi-diff` stays Added-only.
3. **F9 — independent oracle; kill the fuzzy CI threshold.** `ProviderPricingConfig` (static `(provider, model) → rate`, loaded at init, no live fetch) in `maos-domain`. Cost is computed in **integer micro-units — no `f64` in the accumulation path.** The oracle has four layers:
   - **Independent golden cost vectors** (~15–20 scenarios), `expected` computed by a route that does **NOT import the pricing function** (anti-tautology).
   - **Property tests** on aggregation + rounding (round-then-sum vs sum-then-round drift, monotonicity, `.5`-boundary determinism).
   - **CI gate = 100% / deterministic** against a committed synthetic price-book fixture (the arithmetic is exact-to-rounding; the "≥98%" is NOT a CI threshold).
   - **Operational ≥98% SLO** vs real provider invoices via a **weekly sampling runbook — explicitly NOT a CI gate** (catches provider rate changes a synthetic fixture cannot).
4. **CPU-time / storage-I/O dimensions:** FR64 lists them; neither exists today. The v1.0 scope line (record vs defer) is set here at ratification.

## Consequences

- The inference emission site gains cost capture + lineage-derived attribution; the kernel-core baseline moves once, jointly with FR62.
- NFR-Cost-1 is split into a deterministic CI claim (100% vs synthetic) and an operational claim (≥98% vs real, runbook) — errata applied to `requirements-inventory.md`.

## Gate

CI cost-reconcile oracle (golden vectors not importing the pricing fn + rounding/aggregation property tests + integer micro-units); `cargo xtask check-kernel-baseline` green at the jointly re-pinned figure.

## Sign-off required

sec-redteam on F8 (principal semantics) + F9 (cost oracle design).
