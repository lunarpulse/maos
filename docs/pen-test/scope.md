# MAOS v1.0 Pen-Test Scope Document

| Field | Value |
|---|---|
| Document | Pen-Test Scope Document |
| Engagement | MAOS v1.0 external security assessment |
| Driving requirement | NFR-Sec-7 (External pen-test report with zero P0/P1 findings open at v1.0 ship) |
| Status | Frozen at engagement start (revision requires engagement-coordinator sign-off) |
| Scope owner | MAOS security owner |
| Companion documents | `docs/pen-test/owasp-risk-rating-v1.0-frozen.md`, `docs/pen-test/engagement-manifest.toml`, `docs/pen-test/triage-protocol.md`, `docs/pen-test/findings/summary-schema.toml` |

---

## 1. Purpose

This document defines the in-scope attack surfaces, the out-of-scope boundaries, the
rating methodology, and the required deliverables for the external penetration test that
gates the MAOS v1.0 release. It is the authoritative scope reference for the engagement;
the engagement coordinator pins the MAOS binary commit SHA in `engagement-manifest.toml` at
kickoff, and the pen-tester works from the surfaces enumerated below.

## 2. Driving Requirement

**NFR-Sec-7 — External pen-test report with zero P0/P1 findings open at v1.0 ship.**

> Triage by joint panel of pen-test lead + MAOS security owner; disagreements escalate to
> PRD-author tiebreak. P0/P1 definitions per OWASP Risk Rating Methodology, frozen at
> engagement start. Pen-tester engagement scheduled 6–8 weeks before v1.0 ship as
> critical-path dependency.

The v1.0 ship gate (`check-pentest-gate`) activates automatically when
`docs/pen-test/findings/summary.toml` is committed: it passes only when
`p0_open == 0` **and** `p1_open == 0`. Until the engagement concludes, the gate reports
advisory status (`advisory-until-engagement`); it does not block development. The pen-tester's
job is to drive that summary from advisory to a clean, reproducible zero-P0/P1 state — or to
surface findings that must be remediated before ship.

## 3. Engagement Objective

Produce a falsifiable, reproducibly-verifiable security assessment of the MAOS kernel
security envelope at v1.0. Every claim the pen-tester makes must be backed by a commit SHA,
a reproduction vector, and an OWASP P0/P1 rating. The MAOS architecture is designed to make
its security commitments *falsifiable rather than marketing copy* (e.g., a Spirit whose
runtime context drifts from its attested `ComplianceClaim` is rejected with a typed error).
The pen-tester's task is to attempt to falsify each such commitment.

## 4. Attack Surface Scope

The table below maps each in-scope attack surface to the crate(s) and key files the
pen-tester should treat as the primary entry points. File paths are relative to the
repository root at the pinned engagement commit.

| Attack Surface | Crate(s) | Key Files |
|---|---|---|
| Spirit admission & ComplianceClaim | `maos-compliance`, `maos-registry` | `evaluator.rs`, `compliance_verify` |
| Capability mediation | `maos-kernel-core` | `capability/mod.rs`, `security_manager.rs` |
| Namespace isolation | `maos-kernel-core` | `memory/mod.rs` (`validate_namespace_write`) |
| A2A frame integrity | `maos-a2a-core`, `maos-a2a-tcp` | `router.rs`, `intake.rs`, `verifier.rs` |
| Daemon admission | `maos-bin` | `main.rs` (admission_view) |
| Sandbox enforcement | `maos-kernel-core` | `sandbox/` |
| Cryptographic operations | `maos-crypto` | `provider.rs` |
| Transparency log integrity | `maos-iac` | `transparency_log.rs` |
| Skill queue persistence | `maos-skill` | `store.rs`, `queue.rs` |
| mTLS transport | `maos-a2a-tcp` | `tls.rs`, `connector.rs` |

### 4.1 Per-Surface Testing Focus

Each surface carries a specific set of falsifiable commitments the pen-tester should attempt
to break. Findings are rated per the OWASP Risk Rating Methodology (Section 5) and recorded
in the deliverables (Section 7).

**Spirit admission & ComplianceClaim.** Verify that malformed `ComplianceClaim` envelopes are
rejected — a signed claim whose runtime execution-context fingerprint drifts from the kernel's
actual context must surface as a typed `EComplianceContextDrift`, not be silently admitted.
Probe the Ed25519 signature verification, the canonical-CBOR decode path (unknown enum values
must reject as `MalformedClaim`, never coerce to a silent default), and confirm the evaluator
compares the claim against *runtime* context (operator-policy effective tier, strictest-of
sandbox tier, runtime provider/crypto identity) rather than manifest-declared values alone.

**Capability mediation.** Confirm capability tokens are bound to `(Spirit-PID + boot-nonce +
expiry)` and re-validated at point-of-use against current state (TOCTOU correctness), then
attempt token replay, forgery, and escalation beyond the manifest-declared ceiling. Verify the
capability registry predicates and approval-class gating cannot be confused or bypassed, and
that posture changes only ever *restrict* (never widen) beyond the manifest ceiling.

**Namespace isolation.** Confirm `validate_namespace_write` enforces per-principal write
authorization rather than returning an unconditional affirmative; attempt cross-Spirit reads
and writes against `principal:`, working-memory, and shared namespaces. The architectural
intent (Sec-14 corpus) is that no Spirit can enumerate, read, side-channel, or timing-attack
another Spirit's substrate state — the pen-tester should attempt to falsify each vector.

**A2A frame integrity.** Probe the bilateral transport for IAC frame injection, replay, and
consent-envelope bypass; verify the per-frame intent allow-list and intent-vs-source mismatch
detection reject frames whose declared intent diverges from their origin. On a
compromised-peer-Host path, attempt TOFU-pin spoofing and frame tampering that should be
rejected by the router/verifier.

**Daemon admission.** Verify the composition root consults persisted admission state *before*
any Spirit loads, and that skills in a `Rejected` state hard-block daemon startup (not merely
emit a warning). Trace the path from skill discovery through `admission_view` / `decide_skill`
and attempt to reach a loadable or executable state for a rejected or pending skill, including
via cache desynchronization.

**Sandbox enforcement.** Verify the strictest-of-(manifest, trust-tier, operator-policy)
resolution yields the highest tier across all input combinations, and that a
`public-untrusted` Spirit cannot escape its declared ceiling. Probe syscall-pattern divergence
detection (the Landlock/seccomp anomaly surface) and attempt sandbox-tier downgrade paths
where a stricter floor should have applied.

**Cryptographic operations.** Verify the `CryptoProvider` seam is the single mediation point
for signing, capability-token signing, and secret encryption/export, and that swapping the
provider (FIPS module / HSM) cannot weaken or bypass verification. Probe nonce handling in
sealing/export, signature verification on tampered inputs, and capability-token signing
integrity; confirm zero `unsafe` on the capability-validation crypto path (NFR-Sec-9).

**Transparency log integrity.** Verify the log-before-deliver invariant (I2: every IAC frame
is appended to the journal *before* it is delivered) and that the append-only log cannot be
reordered, silently dropped, or back-dated. Probe the pre-write secret-redaction filter for
capability-token and secret leakage in logged frames, digests, and distillates.

**Skill queue persistence.** Probe `atomic_write` durability under concurrent writers and
crash-mid-write, and verify the persisted skill-queue cache cannot be desynchronized from the
Transparency Log ground truth. Confirm a rejected skill's persisted state cannot be bypassed by
cache manipulation, partial writes, or a torn journal.

**mTLS transport.** Verify mutual TLS with TOFU pin verification runs on *every* connection and
that certificate rotation / revocation does not open a downgrade path or produce data-plane
errors. Probe trust-on-first-use pinning bypass, certificate-chain validation gaps, and replay
against rotated credentials.

## 5. Methodology

Findings are rated using the **OWASP Risk Rating Methodology**, whose P0/P1 (and lower)
definitions are **frozen at engagement start** in the companion document
[`docs/pen-test/owasp-risk-rating-v1.0-frozen.md`](owasp-risk-rating-v1.0-frozen.md). That
freeze is immutable after commit: a SHA-256 companion hash is verified by CI on every push, so
the severity scale used to triage a finding cannot be silently redefined mid-engagement.

Triage is performed by a **joint panel** of pen-test lead + MAOS security owner; disagreements
escalate to PRD-author tiebreak. P0 findings generate blocking remediation stories before v1.0
ship. The full panel process, escalation path, and P0/P1 classification examples are documented
in [`docs/pen-test/triage-protocol.md`](triage-protocol.md).

The pen-tester uses the published MAOS ABI and the reproducible environment defined in
[`docs/pen-test/engagement-manifest.toml`](engagement-manifest.toml) (pinned commit SHA,
buildable workspace crates, reference Spirits, operational `maosctl`).

## 6. Out of Scope

The following are **not** part of this engagement:

- **UI / UX testing** — visual design, usability, accessibility, and front-end interaction
  quality are out of scope.
- **Performance testing** — throughput, latency benchmarks, and capacity envelopes are covered
  by other v1.0 gates (e.g., NFR-Rel-3, NFR-Aud-9) and are not pen-test deliverables.
- **Availability / Denial-of-Service** — resource-exhaustion and DoS vectors are out of scope,
  *except* where a DoS condition reveals or enables a security bypass (e.g., a crash that
  bypasses a capability check, or an exhaustion path that weakens an isolation boundary). Such
  cases are in scope and rated as the underlying security flaw.

Per the MAOS threat model (§8.1), the substrate does **not** design against the following, and
they are therefore out of scope:

- **Ring-0 host compromise** — kernel/OS-level compromise is the operating system's
  responsibility, not the MAOS kernel's.
- **Side-channel timing attacks against the LLM** — out of scope for the substrate.
- **Adversarial model fine-tunes** — producing specific bad behaviors via model alignment
  manipulation is not a defect the framework can remediate.

## 7. Deliverables

The engagement produces the following artifacts, all committed under `docs/pen-test/findings/`:

1. **Findings summary** — `docs/pen-test/findings/summary.toml`, conforming to the schema in
   [`docs/pen-test/findings/summary-schema.toml`](findings/summary-schema.toml). This is the
   input to the `check-pentest-gate` CI job: it carries the `p0_open` / `p1_open` counters, the
   engagement start/end dates, and the OWASP methodology commit reference. The gate passes only
   when both open-counters are zero.
2. **Per-finding writeups** — one structured writeup per finding under `docs/pen-test/findings/`,
   each containing: title, OWASP severity, affected surface and crate/file (with commit SHA),
   reproduction steps, observed vs. expected behavior, and remediation recommendation.
3. **Executive summary** — a concise narrative of engagement scope, methodology, findings
   distribution by severity, and the go/no-go posture for the v1.0 ship gate, written for the
   joint triage panel and release decision-makers.

Each finding references the pinned MAOS commit SHA so that the summary can be regenerated and
re-verified against the exact binary the engagement assessed.

## 8. References

- **NFR-Sec-7** — `_bmad-output/planning-artifacts/prd/non-functional-requirements.md` §Security
- **Threat model** — `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/8-security-approval-model.md` §8.1
- **Story definition** — `_bmad-output/implementation-artifacts/10-1b-pen-test-engagement-harness-and-gate-infrastructure.md`
- **Companion docs** — `docs/pen-test/owasp-risk-rating-v1.0-frozen.md`,
  `docs/pen-test/engagement-manifest.toml`, `docs/pen-test/triage-protocol.md`,
  `docs/pen-test/findings/summary-schema.toml`
