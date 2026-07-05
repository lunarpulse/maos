---
Status: binding-v2.0 (architecture + mechanism ratified 2026-07-05 party-mode, unanimous — Winston · Murat · John · Amelia + Mary; binding at Story 11.4a gate green, AC1–AC4 observed red→green)
Gate: Story 11.4a — `check-enterprise-pdp` (7 per-leg-independent legs: real-evaluation, deny-proven-red, issue-path-deny, fail-closed, ceiling-and-zero-config, kernel-abi-diff, release-graph-absence); `{ v1_0 = advisory, v1_5 = advisory, v2_0 = blocking }`; absent/unmeasured → BLOCK@v2.0
Decided: 2026-07-05 (party-mode preflight, unanimous)
Accepted-in-PR: Story-11.4a
Supersedes: none (extends ADR-030 capability-registry decomposition + ADR-006 kernel-learns-no-patterns to the enterprise-authorization case)
Revisits: ADR-006 (kernel learns nothing — the operator's policy is the operator's data); ADR-010 (hexagonal port/adapter ring); ADR-030 (cap-policy decomposition, hot-path budget); ADR-041 (port-trait extraction)
---

# ADR-050 — Enterprise PDP integration via out-of-kernel policy port (Cedar reference)

> **Lead-with-it.** Per the Epic-11 §A7 reflex, this ADR is the binding anchor for NFR-Sec-17 and the `check-enterprise-pdp` gate; it documents *why the epic's flat ZERO-Δ lean did not survive* the party-mode preflight.

**Decision.** Capability-authorization decisions for an enterprise deployment are sourced from a **real external Policy Decision Point** behind a NEW out-of-kernel **`PolicyDecisionPort`** trait in `maos-domain`. The reference adapter (`maos-pdp`) holds a real **in-process Cedar engine** (`cedar-policy`, pure-Rust, Apache-2.0) — chosen so the deny tripwire is a **real per-commit gate**, not an advisory-skipped live leg like an external OPA/Vault server. The operator's policy (Cedar policy-set source) is evaluated **off-hot-path** at daemon startup by a reconciler that materializes the org `forbid` set into the bounded **`OperatorPolicyConfig.per_capability_deny`** kernel layer (F2), consumed by a new **deny-wins arm** in `PolicyTable::evaluate` placed BEFORE the grant checks (Cedar `forbid`-beats-`permit`). The kernel **keeps the ceiling** (ADR-006 / I1): the PDP layer can only **subtract** (deny), never grant beyond the Spirit's manifest. The PDP is **NEVER on the token-verify hot path** (ADR-030 `<5µs` P99) — decisions are materialized into the read-mostly CoW `cap-policy` snapshot, and `evaluate` stays a table walk.

## Context

The v2.0 enterprise commitment (product-scope §70; success-criteria v2.0 "PDP integration") requires that an organization's policy — expressed in the PDP's native language, evaluated by the PDP's real engine — governs which Spirits may hold which capabilities. The kernel today mediates capability issue through `CapabilityRegistryAdapter::issue_with_mediation` → `PolicyTable::evaluate` (`cap_policy/mod.rs`), a read-mostly CoW table walk. The integration must not break the v1.0-frozen ABI, must not draw the kernel-core budget beyond a bounded re-pin, and must keep the kernel the "small, dumb mediator" (ADR-006).

The story is the epic's **most canned-green-exposed**: a stubbed PDP returning canned `Allow` is indistinguishable from a real one until the policy is swapped. The anti-canned discipline (real evaluation proven by a policy-swap-flips-verdict; the `pdp-fault-inject` falsifier that stubs the engine and reds the deny) IS the reason this ADR exists.

## Decision

### 1. The port — `PolicyDecisionPort` in `maos-domain`

A NEW sync port trait (`crates/maos-domain/src/ports/policy_decision.rs`), mirroring the `CollectiveMemoryPort` external-service template: typed `PolicyDecisionError { Unreachable, Timeout, Transport, InvalidPolicy }`, `Send + Sync`, `/// Class: supervision` per method, `Option<Arc<dyn PolicyDecisionPort>>` injection. The request type carries an **optional opaque `principal_attributes`** (F7) shaped NOW so 11.4c (SSO→capability-token issuance) layers an authenticated principal additively, without an ABI break — the 11.4a subject stays the existing `spirit_pid`.

### 2. The reference adapter — `maos-pdp` (in-process Cedar, F3)

A NEW out-of-kernel crate (`crates/maos-pdp/`) implementing the port with a **real Cedar engine**. Cedar is chosen over OPA/Vault for the reference because it is **in-process** → the deny tripwire runs in a normal `cargo test` job and is a **real per-commit blocking-capable gate**, directly avoiding the advisory-skipped-live-leg trap the Postgres legs of `check-multi-region-slo` / `check-scale-churn` fall into. Cedar is pure-Rust + Apache-2.0 (deny.toml-friendly) and trivially kept out of the kernel/domain dependency closure. OPA / Vault adapters are **additive-per-port** (ADR-010) and would carry a `services:` block (F5) — documented here, out of scope for 11.4a.

### 3. The bounded kernel layer — `per_capability_deny` (F2, FLAG-Winston)

The party-mode preflight **OVERRULED** the epic's flat ZERO-Δ lean on long-term-correctness evidence. Mapping an org **forbid** onto the existing `per_capability_approval` becomes a **latent authorization bypass** the day the auto-allowing `ApprovalManager` gets a real prompt; overwriting `manifest_scopes` (the ABI **ceiling**) with the intersected org set makes an out-of-kernel adapter the **ceiling authority** (inverts I1) and **destroys the audit distinction** manifest-gap-vs-org-forbid; and subtraction cannot express Cedar's **`forbid`-beats-`permit` override**. The authorized surface is therefore a NEW **`OperatorPolicyConfig.per_capability_deny: HashSet<String>`** + a **deny-wins arm** in `evaluate` placed BEFORE the grant checks — ~+17 src LOC (re-pin 23023 → 23040, HISTORY-disclosed). The kernel keeps the ceiling: this field can only ever SUBTRACT (a PDP-permit-beyond-manifest is still denied). `SelfTelemetryRead` (FR56) is a kernel invariant evaluated before the inner load and is intentionally NOT PDP-overridable.

### 4. The composition root — off-hot-path reconciliation

`maos-bin/src/main.rs` builds `Option<Arc<dyn PolicyDecisionPort>>` from `MAOS_PDP_POLICY` (inline Cedar text OR a `.cedar` file path). When `Some`, the reconciler (`maos_pdp::reconcile_org_denies`) evaluates the operator policy over the governed capability set and materializes `Deny` verdicts into `per_capability_deny` via the public CoW `PolicyTable::update()` — **no kernel-core edit at the call site**. When `None`, `per_capability_deny` stays empty and behavior is byte-identical to pre-11.4a (AC1). **Fail-closed (F4):** a configured-but-broken PDP materializes ALL governed denies (refuses PDP-scoped grants, log screaming) — never relaxes to permissive. The adapter is kept OUT of `api.rs` (L9 — `check-composition-root-completeness` stays green).

### 5. Fail-closed semantics (F4)

An enterprise PDP failing **open** (Allow on unreachable) is a P0 (L4). The port's typed errors + the reconciler's fail-closed posture guarantee: (1) no PDP configured → kernel default, byte-identical (AC1); (2) configured-but-broken at startup → fail closed LOUD (all governed denied); (3) the deny is observable via the existing `Err(CapError::PolicyDenied)` issue path (CATCH-B — rich forbid-rule attribution is 11.4c's SIEM slice; the kernel `PolicyTable` holds no audit handle, so emitting a cap-audit event from the F2 arm would need a distinct call site beyond the named surface). The runtime-drop (leg 3) + staleness-TTL (leg 4) timelines are inherent to a REMOTE PDP and are documented as the remote-adapter contract (provisioned when OPA/Vault lands); for in-process Cedar they are N/A (the engine does not drop).

### 6. The anti-canned discipline (the story's thesis)

Decisions come from REAL engine evaluation, not a `HashMap` literal. The `check-enterprise-pdp` gate's `real-evaluation` leg proves a policy-swap-flips-verdict over two DISTINCT engine evaluations (anti-memoize); the `deny-proven-red` leg runs the `pdp-fault-inject` falsifier (a dev/CI-only feature that stubs the engine to a canned `Allow`) and watches the deny test RED — proving the deny is engine-derived. The `pdp-fault-inject` feature is guarded by a `compile_error!` (release build + feature = fail) and the gate's `release-graph-absence` leg (a release build WITH the feature MUST compile_error).

## Alternatives considered and rejected

- **ZERO-Δ (materialize forbids into `manifest_scopes` / `per_capability_approval`).** Rejected (§3): latent authz bypass, inverts the kernel ceiling (I1), destroys the audit distinction, can't express `forbid`-beats-`permit` override.
- **Live per-request PDP call inside `issue_with_mediation`.** Rejected (ADR-030): a synchronous engine call on the issue path blows the hot-path budget; decisions are materialized off-hot-path into the CoW snapshot.
- **Three production adapters (OPA + Cedar + Vault) in 11.4a.** Rejected (over-scope): the port is engine-agnostic; one Cedar reference proves real evaluation end-to-end. OPA/Vault are additive-per-port.
- **External Cedar server.** Rejected (F3/F5): an external server makes the deny leg advisory-skipped when unprovisioned (the `check-multi-region-slo`/`check-scale-churn` trap); in-process Cedar keeps the tripwire real per-commit.
- **Putting the engine in `maos-kernel-core`.** Rejected (ADR-006 / L7): the kernel learns nothing; the engine stays quarantined in `maos-pdp` (`check-dependency-closure` enforces cedar-policy's absence from the kernel/domain closure).

## Consequences

- The kernel takes ONE authorized bounded delta — `per_capability_deny` + the deny-wins arm — re-pinned 23023 → 23040 (FLAG-Winston, HISTORY-disclosed).
- A NEW workspace crate `maos-pdp` (count 48 → 49; the 11.1a maos-host+wasm-host sentinel drift corrected alongside).
- `cedar-policy` + its transitive dup-versions are added to `deny.toml [bans] skip` (provenance commented); the license (Apache-2.0) is already allowed.
- The enterprise-authorization path is real-evaluation-governed and deny-falsifiable; the org deny is observable (never silent); rich attribution is 11.4c.
- The port is shaped for 11.4c's SSO principal (additive `principal_attributes`).

## Gate

Binding at **Story 11.4a** (binding-v2.0 **after** AC1–AC4 observed red→green — do NOT flip while any leg is unproven). The `check-enterprise-pdp` gate has **seven independently-falsifiable legs** (each its OWN oracle invocation): `real-evaluation` (AC2), `deny-proven-red` (AC3, +`pdp-fault-inject`), `issue-path-deny` (AC3 end-to-end), `fail-closed` (AC4), `ceiling-and-zero-config` (AC1), `kernel-abi-diff` (re-pinned 23040), `release-graph-absence` (D8 ship-blocker). `{ v1_0 = advisory, v1_5 = advisory, v2_0 = blocking }`; §A7.5 WOULD-HAVE-BLOCKED banner in the advisory window; absent/unmeasured → BLOCK@v2.0.

## Ratification

Architecture + mechanism ratified by the Epic-11 party-mode preflight (Winston · Murat · John · Amelia + Mary walk-on, 2026-07-05, UNANIMOUS). F1 escalated the model tier to opus-4-8 MANDATORY + full §A6 on the canned-green evidence (the dev agent records the non-Opus disclosure + pre-booked §A6 net). F2 OVERRULED the ZERO-Δ lean. F3 chose Cedar in-process. F4 ratified freeze-last-known-good + fail-closed. F6 authored NFR-Sec-17 + this ADR (lead-with-it).
