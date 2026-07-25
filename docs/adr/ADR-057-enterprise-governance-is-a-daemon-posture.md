---
Status: accepted, binding-v2.2
Gate: Story 13.5a — `check-multi-tenant-loom` legs `enterprise-governance-reaches-cohort-daemon`, `enterprise-governance-daemon-dead-wire-negative`, `enterprise-governance-daemon-dispatch-threaded` (all blocking at v2.2)
Decided: 2026-07-25
Accepted-in-PR: Story 13.5a
Extends: ADR-050 (enterprise PDP integration), ADR-051 (enterprise identity / at-rest / SIEM)
Reuses: ADR-054 (cohort mesh), ADR-055 (multi-tenant Loom)
---

# ADR-057 — Enterprise governance is a daemon posture, not Spirit code

## Context

Epic 13 sketched an "enterprise-governed reference Spirit **class**" — an eleventh reference Spirit that composes SSO/OIDC, an enterprise PDP, at-rest AEAD, and SIEM export *Spirit-side*, at zero delta.

That sketch is a category error, and the code says so plainly. The enterprise crates — `maos-pdp`, `maos-sso`, `maos-secrets`, `maos-siem` — are dependencies of `maos-bin` and of nothing else (`crates/maos-bin/Cargo.toml:44-48`). `maos-spirit-abi`, `maos-spirit-sdk`, and `maos-spirit-derive` carry no dependencies at all, and `maos-spirit-hello` depends only on `maos-domain` + `maos-kernel-core`. **No Spirit crate can reach an enterprise adapter.** A Spirit that "composes SSO" would have to add `maos-sso` to a Spirit crate, which inverts the hexagonal direction ADR-010 fixes and drags the whole enterprise dependency closure into every Spirit build — including air-gap builds, where those deps are compiled out on purpose.

A second premise also failed. Minting a *new* collective-only enterprise Spirit re-hits the mandatory non-empty `provider.complete` blocker (`crates/maos-manifest/src/manifest.rs:473-480`) — the same wall Story 13.5d only cleared by hosting its port on the existing `researcher`.

Meanwhile the thing the epic assumed was already done was not. The SSO→PDP→at-rest→SIEM composition does run in `one-shot` / `spirit-spawn` / `default-server` modes. In `cohort-a2a-daemon` mode it ran nowhere: `EnterpriseRuntime` was constructed at the composition root and the daemon dispatch returned before every single reach of it. Multi-tenant collective operations — precisely the ones an enterprise operator most needs governed — were served ungoverned, silently.

## Decision

### 1. The "enterprise-governed Spirit class" is a daemon posture

There is no enterprise Spirit crate, no new `LoadedSpiritKind` variant, and no new `classify_spirit` arm. The class is an **operator-instantiable posture** of the `cohort-a2a-daemon`, assembled at the composition root from four already-registered environment groups:

| Group | Variables | Effect when set |
|---|---|---|
| SSO | `MAOS_SSO_JWKS`, `MAOS_SSO_ISSUERS`, `MAOS_SSO_AUDIENCE`, `MAOS_SSO_ALGS`, `MAOS_SSO_ASSERTION` | every governed collective operation requires a verified OIDC principal |
| PDP | `MAOS_PDP_POLICY_FILE` \| `MAOS_PDP_POLICY_INLINE`, `MAOS_PDP_REFRESH_INTERVAL_MS`, `MAOS_PDP_STALENESS_TTL_MS` | Cedar mediates issuance with the principal's attributes |
| At-rest | `MAOS_KMS_MASTER_KEY` | the governed record is sealed with the collective store's AEAD hook |
| SIEM | `MAOS_SIEM_FILE` | the redacted Transparency Log tail is projected to the localhost sink |

Zero groups set ⇒ the daemon is byte-for-byte its pre-13.5a self.

### 2. Governance attaches at the daemon's collective-serve seam

The daemon's collective operation is the cross-team **cohort digest read**. Its serve-side chokepoint is `maos_a2a_core::DigestReadPort::note_admitted_request`, which the router calls only after the consent gate ACKs an inbound request and whose `Err` rewrites that ACK into a fail-closed NACK (`crates/maos-a2a-core/src/router.rs:1287-1293`).

Under an enterprise posture the composition root **decorates** that port. The decorator runs, in order:

1. `issue_enterprise_governed_capability` — SSO principal → Enterprise PDP → kernel mint (`Scope::LoomRead`) → `identity.asserted` persist. Reused verbatim; the composition root keeps exactly **one** direct `.issue_with_mediation(` call site, the 11.4c bypass-absence invariant.
2. At-rest seal of the governed grant through `EnterpriseRuntime::at_rest_seal_hook()` held in `maos_loom_lite::seal::AtRestSealer` — the same `Arc` closure and the same wrapper `LoomLiteStore::with_at_rest_seal` installs, so seal semantics and fail-closed behaviour are the collective store's.
3. A correlated Transparency Log row (`cohort:digest-read-governed`, keyed by the request's `request_id`) carrying the ciphertext.
4. A SIEM forward of the resulting tail.

Steps 1 and 2 fail **closed**: a refusal leaves no grant, no reply obligation, and no cohort audit row. Step 4 is a projection that runs *after* the record is durable, so a sink failure is operator-visible and buffered — never silent, and never a refusal, because refusing would take the daemon down with the sink without improving auditability.

### 3. One control-Spirit subject, admitted canonically

The daemon governs under the control Spirit named in its own TOML, admitted at composition through `SecurityManagerAdapter::admit_spirit` from `spirits/<control_spirit>/manifest.toml`. Seeding `manifest_scopes` from the composition root stays forbidden. A control Spirit that does not declare `[capabilities.required.loom] read = true` boots and then refuses every governed collective read — fail-closed by the kernel's own unknown-Spirit deny (`cap_policy/mod.rs:129-133`), not by a new check.

Consequence, stated rather than hidden: enterprise PDP subject-deny binds at that one pid, so it is **per daemon posture**, not per tenant Spirit.

### 4. Delta boundary

**ZERO `maos-kernel-core` delta** — 23228 == pin, no file under `crates/maos-kernel-core/src/` touched. `identity.asserted` stays a raw kind-30 Transparency Log row written by the out-of-kernel `append_identity_asserted`, and the governed record rides the existing `FrameKind::TelemetryEvent`. No kernel `FrameKind` variant is added; that would be a forbidden L1 delta.

**NON-ZERO `maos-bin` delta** — the composition root gains the posture type, the port decorator, the signature change that threads it into `run_cohort_a2a_daemon` / `build_cohort_a2a_daemon_runtime`, and the boot proof.

## Consequences

- The enterprise posture is unavailable in `air-gap` / reduced builds: `maos-sso`, `maos-secrets` and `maos-siem` are optional deps behind the `network` feature and compile out. `maos-pdp` is non-optional. An air-gap host therefore has PDP mediation available and the other three not.
- `maos-audit::query_with_redaction` still requires a quiesced database for deterministic projection. Live daemon forwarding preserves that invariant by exporting a transactionally consistent `VACUUM INTO` snapshot of the active WAL database, then removing the snapshot. The recording SIEM port receives the actual projection calls inside the governed daemon lifecycle.
- The SIEM watermark is process-local and resets on restart. Once-per-record holds within one daemon lifetime; exactly-once across restarts is not claimed.
- The 11.4b audit escape-anomaly detector remains a second, separate dead-wire — tracked as an honesty-ledger string, not gated, and explicitly not addressed here.

## Alternatives rejected

- **An eleventh enterprise Spirit crate.** Impossible by dependency direction, and blocked again by the mandatory non-empty `provider.complete` manifest rule.
- **Composing enterprise adapters into an existing Spirit.** Same dependency inversion, plus it would make every consumer of that Spirit carry the enterprise closure.
- **Leaving the daemon ungoverned and documenting it.** This is the exact "isolated tripwire greens while the production path is dead-wired" failure the Epic-11 retrospective named.
