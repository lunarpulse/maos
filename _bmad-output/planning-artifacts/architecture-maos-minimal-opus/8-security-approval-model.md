# 8. Security & Approval Model

## 8.0 Non-negotiable testability floors

The seven floors below cannot be deferred, weakened, or staged without a major-version bump (v0.x → v1.x → v2.x) and an ADR documenting the regression rationale (per ADR-037 `invariant-lock` gate). Cadence and tooling around each floor may evolve; the floors themselves are versioned commitments.

1. **§8.1 red-team corpus** — N=80, full taxonomy across 8 attack classes (capability confusion, IAC frame injection, distillation poisoning, ledger tampering, cross-Spirit privilege escalation, resource exhaustion, side-channel timing, kernel-syscall abuse), every Spirit, every release. ≥9/10 per class detected/blocked, ≥72/80 aggregate, 0 unmitigated category. Pre-frozen, content-addressed, externally authored.
2. **§5.2 wire-protocol fuzz** — per-target floor ≥72 CPU-hours per fuzz target across the 90 days preceding GA; aggregate floor ≥1,000 CPU-hours pre-GA across all targets. Zero crashes / zero auth bypasses / zero TLS downgrade paths on T3. Tiered cadence (T1 10-min per-commit / T2 4h nightly / T3 24h pre-release) is the execution model; per-target + aggregate floors are the gate. (The earlier "≥168h cumulative" framing was a single-bucket aggregate floor; the per-target rewrite in §5.2 supersedes it because aggregate-only allows one well-fuzzed target to mask several under-fuzzed siblings.)
3. **§8.1 secret leakage** — 0% on per-commit canary (10⁴ synthetic secrets), 100% detection on quarterly audit (10⁵ + adversarial mutations), p95 ≤24h discovery latency on production canary (1000 live tokens/month).
4. **§8.5 ComplianceClaim cross-Spirit agreement** — ±2% agreement floor on whatever corpus is current for the version (see App-E staging table). v0.5 calibration ±5%, v0.9 ±2%, v1.0 ±2% active + ≤0.5% drift quarter-over-quarter.
5. **§7.2 mTLS rotation chaos** — quarterly forced rotation under live load; revocation latency median ≤60s, p99 ≤5min; zero data-plane errors during rotation.
6. **§6.3 halt precision-recall** — halt-precision ≥0.85, halt-recall ≥0.95 on labeled corpus N≥150 per safety-critical Spirit. Asymmetry (recall > precision) encodes the cost model: missed halt is unrecoverable, over-halting is operator-resumable.
7. **§8.1 isolation corpus** — 200 scenarios, no Spirit-to-Spirit info leakage. Sec-14a (same-Host: namespace, seccomp, capability-token forgery) and Sec-14b (cross-Host: A2A frame injection, mTLS pinning, replay) per ADR-040.

**Rule against drift.** Staging refers to delivery cadence and *enforcement onset*, not to whether a floor exists. Every floor in this section has a defined value at v1.0; some floors enter advisory reporting earlier and graduate to enforcement at a specified version (e.g., §8.5 ComplianceClaim ±2% agreement: advisory at v0.5 per the App-E v0.5 non-degeneracy criterion, enforced at v0.9). A floor whose value is undefined at v1.0 is not a floor — it is an aspiration and must be removed or rewritten. A floor whose enforcement is deferred past v1.0 is also not a floor — staging cannot push enforcement past the version where the floor is claimed to exist.

**Corpus-pending floors.** Floor 6 (halt precision-recall) and Floor 4 (CCAC ±2% agreement) are corpus-dependent — the floor value exists from v0.5 onward as advisory, but enforcement requires the corpus itself, which is itself a staged deliverable. Until the corpus exists, the floor is reported but not gated. See §6.6 and App-E for the per-corpus staging detail.

## 8.1 Threat model

The substrate is designed against the following:

| Threat | Mitigation |
|---|---|
| Compromised LLM provider returning malicious tool-call args | Sandbox tier on every exec; arg validation at Capability Registry; approval prompts on `exec_capable` and `mutating` |
| Compromised MCP server running arbitrary code on the Host | Container sandbox (T3) for less-untrusted MCP; allowlist for first-party MCP |
| Prompt-injection via tool output (e.g., a search result containing instructions) | Output redaction at the Transparency Log boundary; explicit "tool output is data, not instructions" framing in system prompts; intent-vs-source mismatch detection |
| Compromised peer Host in bilateral A2A pair | mTLS + TOFU pin verification at every connection; explicit consent envelope on every frame; per-frame intent allow-list |
| Spirit escalating its own posture beyond manifest ceiling | Posture changes are kernel-managed; manifest sets a hard ceiling; posture restricts only |
| Spirit reading another Spirit's private memory | Memory Manager namespace enforcement (I5) |
| Spirit silently exfiltrating data | Transparency Log (I2) — every IAC frame logged before delivery; user can audit |
| Approval prompt fatigue → user clicks through | Approval batching with explicit scope; `prompt_with_diff` makes the cost of cleanup visible before approval |
| Capability token replay | Tokens bound to (Spirit-PID + boot-nonce + expiry); re-validation at use against current state (TOCTOU correctness) |
| LLM jailbreak via adversarial input (paste-into-context) | Input provenance tagging at IAC frame creation; intent-vs-source mismatch detection; epistemic halt on adversarial-intent indicators |
| Capability-token leak via logs / digests / distillates | Pre-write secret-redaction filter at the Transparency Log boundary (universal to all logged frames); §9.5 fifth metric `digest-secret-leakage = 0%`; production canary system catches leaks at runtime |
| Provider supply-chain compromise | Response cross-validation across providers for high-stakes decisions (Orchestrator may dispatch the same task to two Workers on different providers and compare digests for divergence); provider-driver integrity checks at MAOS startup |
| Sandbox-escape via syscall-pattern divergence | Anomaly detector on top of Landlock/seccomp (syscall-pattern divergence from manifest declaration, fd-table growth, unexpected outbound IAC connections); structural alarm only — kernel does not classify intent |

The substrate does **not** design against:

- Ring-0 host compromise (out of scope for the kernel; that is the OS's job).
- Side-channel timing attacks against the LLM (out of scope).
- Adversarial model fine-tunes producing specific bad behaviors (out of scope; framework cannot fix model alignment).

**Adversarial-Spirit red-team corpus.** 80-scenario corpus across 8 attack classes (capability confusion, IAC frame injection, distillation poisoning, ledger tampering, cross-Spirit privilege escalation, resource exhaustion, side-channel timing, kernel-syscall abuse), N=10 per class. Floor: ≥9/10 per class detected/blocked by kernel; ≥72/80 aggregate; 0 unmitigated category. Authored by external pen-tester (not MAOS team) using published ABI; pre-frozen corpus, content-addressed.

**Cross-Spirit memory isolation corpus.** 200-scenario adversarial corpus where Spirit-A actively attempts to enumerate, read, side-channel, or timing-attack Spirit-B's substrate state. Categories: namespace enumeration, working-memory read-across, decision-frame observation, halt-signal observation, transparency-log cross-read, working-memory-digest cross-read, capability-token forgery cross-Spirit, sandbox-escape lateral. Split into Sec-14a (same-Host attack vectors) and Sec-14b (cross-Host bilateral attack vectors). Floor: 200/200 isolation maintained; any leak = P0 ship-blocker.

## 8.2 Sandboxing — re-cap of §4.3.1

OS-native primitives per Spirit form and trust tier. For v1.0:
- Linux: bwrap + Landlock + seccomp inside Docker for T3; Landlock + seccomp narrow for T2.
- macOS: Seatbelt with `.sbpl` profiles. Codex's `seatbelt_base_policy.sbpl` and `seatbelt_network_policy.sbpl` are the prior art.
- Windows: restricted-token sandbox + Job Object resource constraints.

**Strictest-of-(manifest, trust-tier, operator-policy) floor.** A `public-untrusted` Spirit declaring T0 in its manifest is forced to T2 by the trust-tier floor.

## 8.3 Approval class taxonomy — re-cap of §4.3.3

`readonly_scoped`, `readonly_search`, `mutating`, `exec_capable`, `control_plane`, `interactive`. Lifted from openclaw because it is the most expressive taxonomy in the survey and because openclaw has already proven it scales to 100+ tool types.

## 8.4 Audit

The Transparency Log is the personal audit trail. The Approval Decision Log is a separate kernel-managed SQLite table that records every approval prompt's `(actor, target, capability, intent, decision, reasoning_if_any)`. Both logs are queryable via a control-plane API; both can be exported for compliance.

Both logs additionally stream to OpenTelemetry endpoints when configured, for integration into the operator's existing observability stack.

## 8.5 ComplianceClaim envelope

A first-class kernel object: `ComplianceClaim`. Ed25519-signed by an attesting party, references an *execution-context fingerprint* — the precise tuple of (manifest hash + version + trust tier + sandbox tier + capability scope set + provider-endpoint pinning + crypto-provider identity) under which the claim applies.

The kernel verifies `ComplianceClaim` envelopes at admission time and refuses to load Spirits whose runtime context drifts from the attested context (typed error `EComplianceContextDrift`). This makes attestations falsifiable rather than marketing copy.

**Schema location.** The ComplianceClaim schema is defined in [`maos-spirit-abi/src/compliance.rs`](../crates/maos-spirit-abi/src/compliance.rs); any change to its wire shape bumps `ABI_VERSION`. The structural validator and emit pipeline live in `maos-core::compliance` and `maos-core::pipeline` respectively, both shipped at v0.1. The semantic evaluator (principle engine, N=600 corpus, ±2% agreement target) lives in `maos-compliance` and ships at v0.9 — see **App-E "v0.9+ Compliance Roadmap"** for the staging table, generation mechanism (Mechanisms A/B/C), and per-phase ship-blocking gates.

**v0.1 ship-blocking surface (binding here).** The schema is frozen, the structural validator is implemented, the emit pipeline is live on every Spirit decision. Schema validation 100%, emit-rate 100%. No semantic eval, no corpus, no agreement floor — those are App-E.

**ABI break rule.** Adding any required field, removing any field, renaming, type-changing, or removing/reordering enum variants of `Verdict` / `PrincipleRef` / `EvidenceKind` bumps `ABI_VERSION`. Adding optional fields with `#[serde(default, skip_serializing_if = "Option::is_none")]`, additive enum variants at the end with explicit `#[repr(u8)]` discriminants and `#[serde(other)]` fallback, or loosening bounds — does NOT bump.

## 8.6 Pluggable crypto provider

The kernel's cryptographic operations (signing, mTLS, secret encryption) are mediated by a `CryptoProvider` trait with a default implementation (`ring` / `rustls` / equivalent). Alternate implementations can be swapped at composition root for FIPS 140-3-validated module compatibility, hardware-backed crypto, or air-gapped deployments using on-prem HSMs. v1.0 architectural commitment: the seam exists; specific FIPS modules are downstream distributor concern.

## 8.7 Constitutional substrate evolution

Architecture Decision Records (ADRs) are the substrate's evolution mechanism. ADR amendments touching invariants I1–I14 require: (a) machine-checkable diff against the invariant set, (b) a corpus delta showing the test surface that exercises the change, (c) a phase-commitment update — all three enforced by CI gate `invariant-lock`, not by founder discipline.

The "no fifth protocol unless..." commitment is enforced architecturally — adding a fifth wire protocol requires a passing `invariant-lock` gate plus a new ADR with two-reviewer sign-off. The kernel's evolution is bounded by structurally-enforced governance, not personality.
