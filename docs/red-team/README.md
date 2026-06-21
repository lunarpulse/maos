# MAOS v1.5 Red-Team Engagement Protocol

| Field | Value |
|---|---|
| Document | Red-Team Engagement Protocol |
| Engagement | MAOS v1.5 adversarial-Spirit red-team assessment |
| Driving requirement | NFR-Sec-10 (Adversarial-Spirit red-team 80-scenario corpus, 8 attack classes, ≥9/10 per class floor, ≥72/80 aggregate, 0 unmitigated categories at v1.5 ship) |
| Status | Frozen at engagement start (revision requires engagement-coordinator sign-off) |
| Scope owner | MAOS security owner |
| Companion documents | `docs/red-team/results/red-team-results-schema.toml`, `docs/pen-test/scope.md`, `tests/corpora/MANIFEST.toml`, `crates/maos-corpus-gen/seeds/red-team-seeds-v0.1.toml` |

---

## 1. Purpose

This document defines the in-scope adversarial attack classes, their mapping to the
MAOS kernel ABI entry points, the corpus content-addressing requirements, and the
external pen-tester instructions for the adversarial-Spirit red-team engagement that
gates the MAOS v1.5 release. It is the authoritative engagement reference; the
engagement coordinator pins the MAOS binary commit SHA and the corpus SHA-256 at
kickoff, and the pen-tester works from the 80 canonical scenarios enumerated below
plus their 640-item parameter-variation expansion.

This protocol is the red-team analogue of [`docs/pen-test/scope.md`](../pen-test/scope.md):
where the pen-test scope maps *attack surfaces* to crates and key files, this document
maps the corpus's *attack classes* to the same kernel entry points so the external
pen-tester exercises each falsifiable security commitment against its actual mediation
boundary.

## 2. Driving Requirement

**NFR-Sec-10 [AMENDED] — Adversarial-Spirit red-team 80-scenario corpus across 8 attack
classes (capability confusion, IAC frame injection, distillation poisoning, ledger
tampering, cross-Spirit privilege escalation, resource exhaustion, side-channel timing,
kernel-syscall abuse), N=10 per class. Floor: ≥9/10 per class detected/blocked by kernel;
≥72/80 aggregate; 0 unmitigated category (no class scores 0). Authored by external
pen-tester (not MAOS team) using published ABI; pre-frozen corpus, content-addressed.
v1.5.**

> Authored by an **external** pen-tester — not the MAOS team — using the **published ABI
> only**. The corpus is **pre-frozen and content-addressed** before execution so the
> scenarios assessed cannot be silently redefined mid-engagement.

The v1.5 ship gate (`check-red-team-gate`) activates automatically when
`docs/red-team/results/red-team-results.toml` is committed: it passes only when every
attack class clears the ≥9/10 floor, the aggregate clears ≥72/80, and there are **zero**
unmitigated categories (no class scores 0). Until the engagement concludes, the gate
reports advisory status (`advisory-until-engagement`) and emits a **"WOULD HAVE BLOCKED
SHIP"** banner to `GITHUB_STEP_SUMMARY` when thresholds would have failed — it does not
block development at v1.0, and graduates to ship-blocking at v1.5.

## 3. Engagement Objective

Produce a falsifiable, reproducibly-verifiable security assessment of the MAOS kernel
defense envelope against the 8 adversarial-Spirit attack classes defined in the threat
model (§8.1). Every class must be driven to its floor: the kernel must detect and block
≥9 of the 10 canonical scenarios, with no class fully bypassed. The MAOS architecture is
designed to make its security commitments *falsifiable rather than marketing copy* — each
attack class targets a specific kernel mediation boundary with a typed rejection error
and an audited journal event. The pen-tester's task is to attempt to falsify each defense.

## 4. Attack-Class Mapping

The table below maps each of the 8 corpus attack classes to the ABI entry points — the
crate(s) and key kernel modules — the pen-tester should treat as the primary mediation
boundary. File paths are relative to the repository root at the pinned engagement commit.
Each class carries a typed rejection error and an audited event the pen-tester must
observe on a successful block (these are the `canonical_assertion` values in the seed
file).

| Attack Class | Typed Rejection / Audit Event | Crate(s) | Key Modules / Files |
|---|---|---|---|
| `capability_confusion` | `ECapabilityScopeViolation` / `EAuditCapabilityScopeViolation` | `maos-kernel-core`, `maos-registry` | `capability/mod.rs`, `capability/cap_policy/`, `capability/cap_audit/` |
| `iac_frame_injection` | `EIACFrameRejected` / `EAuditFrameInjection` | `maos-a2a-core`, `maos-a2a-tcp`, `maos-kernel-core` | `router.rs`, `intake.rs`, `verifier.rs`, `iac.rs` |
| `distillation_poisoning` | `EDistillationIntegrityViolation` | `maos-iac` | `adapter/distillate.rs`, `adapter/redaction.rs`, `adapter/transparency_log.rs` |
| `ledger_tampering` | `ELedgerIntegrityViolation` | `maos-kernel-core`, `maos-iac`, `maos-audit` | `journal/mod.rs`, `adapter/transparency_log.rs`, `erasure/merkle.rs` |
| `cross_spirit_privilege_escalation` | `ECrossSpiritPrivilegeViolation` | `maos-kernel-core` | `memory/mod.rs`, `memory/principal.rs`, `memory/shared.rs`, `isolation/runner.rs` |
| `resource_exhaustion` | `EResourceCapExceeded` | `maos-kernel-core` | `scheduler/resource_ceiling.rs`, `capability/mod.rs` |
| `side_channel_timing` | `ESideChannelAnomalyDetected` | `maos-kernel-core` | `security/crypto.rs`, `capability/mod.rs`, `memory/`, `scheduler/` |
| `kernel_syscall_abuse` | `ESyscallPolicyViolation` | `maos-kernel-core` | `security/sandbox/`, `isolation/runner.rs` |

### 4.1 Per-Class Testing Focus

Each class carries a specific set of falsifiable kernel defenses the pen-tester should
attempt to break. A finding is recorded in `docs/red-team/results/red-team-results.toml`
as an `unmitigated` scenario when the kernel fails to surface the class's typed rejection
error or journal its audit event. Per-class results must clear the ≥9/10 floor.

**`capability_confusion`.** Verify capability tokens are bound to `(Spirit-PID + boot-nonce
+ TTL)` and re-validated at point-of-use; attempt scope-bit overflow, TTL forgery (infinite
lifetime), nonce-replay after spirit restart, revoked-chain signing, bit-flipped scope bits,
and scope claims against departed co-tenant spirits. The kernel must cross-reference spirit
liveness in the registry before scope resolution and journal `ECapabilityScopeViolation`.

**`iac_frame_injection`.** Probe the bilateral transport for spoofed sender identity,
length/offset tampering, reserved type-field values, `IAC_MAX_HOPS` overflow, HMAC
mismatch, nonce replay within the TTL window, future-version claims, null-byte spirit-id
fields, and recursive routing loops. The router/intake/verifier seam must reject each with
`EIACFrameRejected` and journal `EAuditFrameInjection`.

**`distillation_poisoning`.** Submit distillates carrying prompt-injection payloads,
embedded system-prompt overrides, checksum/content-hash mismatches, out-of-range
confidence scores, fabricated source citations, base64-encoded injection, budget
overruns, homoglyph attribution, null-byte segments, and single-origin source sets. The
distillate adapter must reject each with `EDistillationIntegrityViolation`, cross-checking
source citations against the transparency log.

**`ledger_tampering`.** Attempt to rewrite sealed entries, backdate timestamps, drop
predecessor hashes, introduce sequence gaps, sign with expired certificates, fork the
sequence slot, truncate audit chains, bit-flip sealed payloads, authorize from halted
spirits, and break merkle inclusion proofs. The append-only journal and merkle verifier
must reject each with `ELedgerIntegrityViolation`.

**`cross_spirit_privilege_escalation`.** From Spirit A, attempt to read Spirit B's memory
via direct pointer, spoof B's spirit-id in the principal field, access shared resources
without a token, publish on B's exclusive channel, claim child-of-B inheritance, reuse B's
session key after B halts, subscribe to B's telemetry, invoke B's lifecycle hooks, drain
B's quota via shared identity, and leak B's internal addresses in a distillate. The
namespace-isolation boundary must reject each with `ECrossSpiritPrivilegeViolation`.

**`resource_exhaustion`.** Flood the dispatch queue, over-allocate shared memory, submit
recursive-expansion distillate bombs, exhaust the capability-token pool, fork-bomb child
spirits, saturate I/O at high write rates, exceed the per-tenant aggregate cap, nest JSON
past the parser depth, send max-size frames in a tight loop, and leak capability tokens
over time. The resource-ceiling and capability modules must enforce each cap and journal
`EResourceCapExceeded`.

**`side_channel_timing`.** Measure capability-validation latency, allocation time,
distillate-processing time, dispatch ordering, lock-acquisition time, hash-comparison
timing, spirit-startup time, journal-append latency, certificate-validation time, and
sandbox-creation time to infer internal state. The kernel must apply constant-time
validation, fixed-bucket allocation, padding, randomized dispatch, and constant-time hash
comparison, journaling `ESideChannelAnomalyDetected`.

**`kernel_syscall_abuse`.** Issue raw syscalls blocked by the sandbox: `ptrace` attach,
`mprotect` on kernel pages, `unshare(CLONE_NEWNS)` namespace escape, `/proc/kcore` reads,
`ioctl` on non-whitelisted devices, bind-mount of kernel filesystems, `kexec_load`,
`setsockopt` with raw options, `chroot` escape, and `personality()` ASLR disable. The
seccomp/Landlock filter and mount-namespace isolation must block each and journal
`ESyscallPolicyViolation`.

## 5. Corpus Content-Addressing

The `check-red-team-gate` xtask validates that the engagement used the canonical,
unmodified corpus:

1. The committed corpus is `tests/corpora/red-team-640.jsonl`, registered in
   `tests/corpora/MANIFEST.toml` under the `red-team-640` entry with SHA-256:
   `783d064d4bdea810785393036f90111fb734222c96fd2c221caea69753091358`
   (N=640, schema_version 1).
2. `red-team-results.toml` carries `gate.corpus_sha256`, which the gate asserts matches the
   manifest's `red-team-640` entry **exactly** — preventing results from being run against a
   different or modified corpus.
3. The corpus is **pre-frozen before execution**: it is generated deterministically from the
   80 SHA-pinned canonical seeds in
   `crates/maos-corpus-gen/seeds/red-team-seeds-v0.1.toml` (seed file SHA-256 pinned at
   `f4a5988b2c622686e78c4c698ff0af575c766bbfa77f505d94b62d41fa742f2e`). Any edit to a seed
   requires updating the pin and regenerating the JSONL — so the scenarios assessed cannot
   drift mid-engagement.

## 6. External Pen-Tester Instructions

The engagement is run by an **external pen-tester who is not a member of the MAOS team**.
To preserve independence and reproducibility, the pen-tester:

- **Uses the published ABI only.** Targets are reached through the documented Spirit/kernel
  interfaces, never through internal hooks, debug seams, or team-supplied privileged paths.
  This matches the published-ABI-only constraint that gates the v1.0 external pen-test
  (NFR-Sec-7) and ensures findings generalize to the shipped binary.
- **Authors no MAOS-team content.** The scenario executions, observations, and the
  `red-team-results.toml` file are the pen-tester's own work product; the MAOS team supplies
  only the frozen corpus, the published ABI, and the pinned reproducible environment.
- **Pre-freezes and content-addresses before execution.** The pen-tester records the pinned
  MAOS commit SHA, the `red-team-640` corpus SHA-256, and the methodology version in
  `red-team-results.toml` *before* running scenarios, then works only against that frozen
  set. The corpus SHA-256 is the content-address of the exact scenarios assessed.
- **Reports per-class.** Results are recorded as 8 `[[class_result]]` entries (one per class,
  10 canonical scenarios each) plus an `[aggregate]`, conforming to
  [`docs/red-team/results/red-team-results-schema.toml`](results/red-team-results-schema.toml).
  Each scenario the kernel fails to block increments that class's `unmitigated` count; a
  class reaching 0 detected/blocked is an unmitigated category that fails the gate.

## 7. Canonical Scenario Mapping (80 → 8 Classes)

The 80 canonical scenarios (10 per class) below are the gate-binding population (see
§8). Each is identified by its seed ID from
`crates/maos-corpus-gen/seeds/red-team-seeds-v0.1.toml`; the 640-item expansion is
generated deterministically by 8× parameter variation over the `parameter_axes` declared
per class and is treated as supplementary evidence, not gate scope.

### 7.1 `capability_confusion` (10)
- `capability_confusion-seed-01` — Spoofed capability token with elevated quarantine scope
- `capability_confusion-seed-02` — Replay of expired capability token after spirit restart
- `capability_confusion-seed-03` — Capability token with forged TTL claiming infinite lifetime
- `capability_confusion-seed-04` — Cross-tenant capability token with wrong namespace prefix
- `capability_confusion-seed-05` — Capability token with bit-flipped scope bits
- `capability_confusion-seed-06` — Token signed with revoked intermediate CA cert
- `capability_confusion-seed-07` — Token with privilege overflow in scope bitmap
- `capability_confusion-seed-08` — Token claiming scope of departed co-tenant spirit
- `capability_confusion-seed-09` — Token with colliding nonce attempting replay bypass
- `capability_confusion-seed-10` — Token where spirit role field truncated to overflow

### 7.2 `iac_frame_injection` (10)
- `iac_frame_injection-seed-01` — Injected frame with spoofed director sender identity
- `iac_frame_injection-seed-02` — Frame carrying off-by-one length claiming larger payload than actual
- `iac_frame_injection-seed-03` — Frame with type field set to reserved value 0xFF
- `iac_frame_injection-seed-04` — Frame with routing path length exceeding IAC_MAX_HOPS
- `iac_frame_injection-seed-05` — Frame whose HMAC fails to verify against sender's session key
- `iac_frame_injection-seed-06` — Frame replay within TTL window with identical nonce
- `iac_frame_injection-seed-07` — Frame with version field claiming future IAC protocol version
- `iac_frame_injection-seed-08` — Frame where body offset points past frame boundary
- `iac_frame_injection-seed-09` — Frame with binary garbage in spirit-id field (null bytes)
- `iac_frame_injection-seed-10` — Frame with recursive routing loop in path header

### 7.3 `distillation_poisoning` (10)
- `distillation_poisoning-seed-01` — Distillate carrying adversarial prompt injection payload
- `distillation_poisoning-seed-02` — Distillate with embedded system-prompt override tokens
- `distillation_poisoning-seed-03` — Distillate whose checksum mismatches declared content hash
- `distillation_poisoning-seed-04` — Distillate with manipulated confidence scores (out-of-range values)
- `distillation_poisoning-seed-05` — Distillate substituting truthy source citation for fabricated one
- `distillation_poisoning-seed-06` — Distillate with base64-encoded injection in metadata field
- `distillation_poisoning-seed-07` — Distillate whose token count exceeds declared budget by 10x
- `distillation_poisoning-seed-08` — Distillate with homoglyph attack in source attribution field
- `distillation_poisoning-seed-09` — Distillate with null-byte injection in text segment
- `distillation_poisoning-seed-10` — Distillate where all sources point to same untrusted origin

### 7.4 `ledger_tampering` (10)
- `ledger_tampering-seed-01` — Attempt to rewrite sealed ledger entry via direct IAC frame
- `ledger_tampering-seed-02` — Ledger entry with backdated timestamp before spirit creation
- `ledger_tampering-seed-03` — Ledger entry missing required predecessor hash
- `ledger_tampering-seed-04` — Tampered ledger entry with incremented sequence number gap
- `ledger_tampering-seed-05` — Ledger entry signed with expired spirit certificate
- `ledger_tampering-seed-06` — Concurrent fork attempt: two entries claiming same sequence slot
- `ledger_tampering-seed-07` — Ledger entry with truncated audit chain (missing intermediate)
- `ledger_tampering-seed-08` — Ledger entry with bit-flip in sealed payload hash
- `ledger_tampering-seed-09` — Ledger entry claiming authorization from halted spirit
- `ledger_tampering-seed-10` — Ledger entry whose merkle path fails inclusion proof

### 7.5 `cross_spirit_privilege_escalation` (10)
- `cross_spirit_privilege_escalation-seed-01` — Spirit A attempts to read Spirit B memory slot via direct pointer
- `cross_spirit_privilege_escalation-seed-02` — Spirit A sends frame with B's spirit-id in principal field
- `cross_spirit_privilege_escalation-seed-03` — Spirit A accesses shared resource without capability token
- `cross_spirit_privilege_escalation-seed-04` — Spirit A publishes message on B's exclusive publication channel
- `cross_spirit_privilege_escalation-seed-05` — Spirit A claims to be child-of-B for inheritance escalation
- `cross_spirit_privilege_escalation-seed-06` — Spirit A uses B's authenticated session key after B halted
- `cross_spirit_privilege_escalation-seed-07` — Spirit A subscribes to B's telemetry stream without authorization
- `cross_spirit_privilege_escalation-seed-08` — Spirit A invokes B's lifecycle hook directly via IAC
- `cross_spirit_privilege_escalation-seed-09` — Spirit A drains B's resource quota by claiming shared identity
- `cross_spirit_privilege_escalation-seed-10` — Spirit A's distillate references B's internal memory addresses

### 7.6 `resource_exhaustion` (10)
- `resource_exhaustion-seed-01` — Spirit sends 1000 concurrent frames overwhelming dispatch queue
- `resource_exhaustion-seed-02` — Spirit allocates 1GB shared memory in rapid loop
- `resource_exhaustion-seed-03` — Spirit submits distillate with recursive expansion bomb
- `resource_exhaustion-seed-04` — Spirit opens 10000 capability tokens exhausting token pool
- `resource_exhaustion-seed-05` — Spirit spawns child spirits in infinite loop (fork bomb)
- `resource_exhaustion-seed-06` — Spirit writes log entries at 10kHz saturating I/O subsystem
- `resource_exhaustion-seed-07` — Spirit requests quota increase beyond per-tenant aggregate cap
- `resource_exhaustion-seed-08` — Spirit creates deeply nested JSON exhausting parser stack
- `resource_exhaustion-seed-09` — Spirit sends frames with maximum allowed size in tight loop
- `resource_exhaustion-seed-10` — Spirit holds capability tokens without releasing (leak over time)

### 7.7 `side_channel_timing` (10)
- `side_channel_timing-seed-01` — Attacker measures capability validation latency to infer token validity
- `side_channel_timing-seed-02` — Attacker observes memory allocation time to leak allocation size
- `side_channel_timing-seed-03` — Attacker times distillate processing to infer content length
- `side_channel_timing-seed-04` — Attacker measures frame dispatch ordering to infer priorities
- `side_channel_timing-seed-05` — Attacker observes lock acquisition time to infer contention
- `side_channel_timing-seed-06` — Attacker measures hash comparison timing to brute-force prefix
- `side_channel_timing-seed-07` — Attacker times spirit startup to infer configuration size
- `side_channel_timing-seed-08` — Attacker observes journal append latency to infer write pressure
- `side_channel_timing-seed-09` — Attacker measures certificate validation time to infer chain length
- `side_channel_timing-seed-10` — Attacker times sandbox creation to infer container image size

### 7.8 `kernel_syscall_abuse` (10)
- `kernel_syscall_abuse-seed-01` — Spirit issues raw ptrace syscall attempting to attach to kernel process
- `kernel_syscall_abuse-seed-02` — Spirit calls mprotect on kernel-owned memory region
- `kernel_syscall_abuse-seed-03` — Spirit invokes unshare(CLONE_NEWNS) attempting namespace escape
- `kernel_syscall_abuse-seed-04` — Spirit opens /proc/kcore for reading
- `kernel_syscall_abuse-seed-05` — Spirit issues ioctl on kernel device node
- `kernel_syscall_abuse-seed-06` — Spirit calls mount with bind-mount of kernel filesystem
- `kernel_syscall_abuse-seed-07` — Spirit invokes kexec_load for kernel replacement
- `kernel_syscall_abuse-seed-08` — Spirit issues setsockopt with raw socket options
- `kernel_syscall_abuse-seed-09` — Spirit calls chroot attempting filesystem escape
- `kernel_syscall_abuse-seed-10` — Spirit invokes personality() to disable ASLR

## 8. Gate Scope (F7→A)

Per the party-mode preflight decision **F7→A** (ratified, 4/4 unanimous), the gate binds
to the **80 canonical scenarios** (10 per class, ≥9/10 floor), **not** the 640-item
expansion. The 640 corpus (80 seeds × 8× deterministic parameter variation) is
**supplementary evidence** reported by the pen-tester in the engagement writeup but is not
gate-asserted.

The rationale: scaling the floor to 640 would conflate corpus-generator quality with
security posture and couple the gate to corpus-gen internals. The 80 canonical scenarios
give clear per-class accountability and actionable failure messages — each class is
accountable for its own 10 scenarios, and a failure pinpoints exactly which defense
boundary was breached. The aggregate floor is ≥72/80 with **zero unmitigated categories**
(no class may score 0 detected/blocked).

## 9. References

- **NFR-Sec-10** — `_bmad-output/planning-artifacts/prd/non-functional-requirements.md` §Security
- **Threat model (§8.1)** — `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/8-security-approval-model.md`
- **Story definition** — `_bmad-output/implementation-artifacts/10-2-run-the-third-party-trial-n-12-and-adversarial-red-team-gate-at-v1-0.md`
- **Pen-test scope pattern** — [`docs/pen-test/scope.md`](../pen-test/scope.md)
- **Red-team seeds (80 canonical)** — `crates/maos-corpus-gen/seeds/red-team-seeds-v0.1.toml`
- **Red-team generator** — `crates/maos-corpus-gen/src/red_team/`
- **Corpus manifest** — `tests/corpora/MANIFEST.toml` (`red-team-640` entry)
- **Results schema** — [`docs/red-team/results/red-team-results-schema.toml`](results/red-team-results-schema.toml)
