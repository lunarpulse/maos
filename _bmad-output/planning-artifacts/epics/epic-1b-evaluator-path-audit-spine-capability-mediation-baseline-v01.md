# Epic 1b: Evaluator Path + Audit Spine + Capability Mediation Baseline (v0.1-β)

**Goal:** An evaluator clones the repo, runs `maosctl install && maosctl run hello-spirit` within 5 minutes, gets a structured hello-spirit response, AND verifies via the Transparency Log that every external call was capability-mediated.

**Owns:**
- Inference Port implementation (Anthropic provider at v0.1; ADR-005).
- Sandbox tiers T0/T1/T2 enforcement (ADR-004: T0 trusted local-tier; T1 process isolation/UID separation; T2 Linux Landlock+seccomp / macOS Seatbelt / Windows restricted-token).
- **ComplianceClaim schema FROZEN** (after E0 adversarial review) + structural validator (~200 LOC in `maos-kernel-core/compliance/`).
- Transparency Log per-Host SQLite (append-only; `log-before-deliver` I2 guarantee).
- Approval Decision Log (separate from Transparency Log; full intent + decision + reasoning chain per I4).
- Lifecycle Journal (append-only on-disk log of lifecycle transitions; fsync per transition; ring-buffer flush <1ms).
- Telemetry IAC round-trip metrics (binding from v0.1: `iac_rt_duration_us` histogram with buckets `[50, 75, 100, 150, 200, 300, 450, 700, 1000, 1500, 2200, 3300, 5000, 7500, 11000, 16000, 25000, +Inf]`, labels `service ∈ {security, memory, iac, capability, spirit_scheduler}`).
- Per-Spirit resource caps basic (cgroups v2 on Linux; setrlimit on macOS; Job Object on Windows — declared in manifest `[resources]`).
- `maos-spirit-hello` reference Spirit (validates SDK end-to-end at v0.1 ABI level — structured introduction with capability scope, posture, expected halt-tags, Transparency Log link).
- `maosctl` v0.1 commands: `install`, `start`, `stop`, `unload`, `run`, `--plain` flag, `NO_COLOR`/`TERM=dumb` accessibility (NFR-Ops-5).
- Capability Registry runtime decomposition (cap-tokens hot-path: sharded `Arc<[CapShard; 64]>` lock-free, P99 <5µs; cap-policy read-mostly copy-on-write; cap-audit bounded `mpsc::channel(8192)` to single audit-writer task; cap-quota per-Spirit atomic counters).
- Manifest field test coverage ≥3 cases per field (NFR-Test-13: well-formed / malformed-rejected / edge-case).
- Pluggable Inference Port routing — Spirit binaries do not import vendor LLM SDKs directly (FR47 closure).

**FRs covered:** FR4 (basic 100% mediation in any 1000-call sample), FR5 (T0/T1/T2 strictest-of-(manifest, trust-tier, operator-policy) floor), FR6 (basic resource caps via cgroups v2 / setrlimit / Job Object), FR9 (basic load/start/unload via authenticated control plane), FR38 (ComplianceClaim schema FROZEN — envelope + admission verification deferred to E7), FR47 (Inference Port Anthropic implementation), FR58 (v0.1 hello-spirit J0 evaluator path).

**Key NFRs:** NFR-Onb-2 (5-min installer J0 path), NFR-Obs-4 (Transparency Log SQLite append-only with JSONL export), NFR-Rel-8 (lifecycle journal fsync per transition, flush <1ms), NFR-Perf-3 (capability-token validation P99 <100µs with 100% re-validation at use against current state — TOCTOU correctness, NFR-Maint-8), NFR-Test-13 (manifest field ≥3 cases), NFR-Sec-1 v0.1 slice (T0/T1/T2 enforcement floor).

**Corpora authored in E1b:**
- Kernel-API invariant cases ~30 (NFR-Test-2 population).
- Manifest field cases ~15 (NFR-Test-13 population for kernel manifest fields).

**KLOC budget:** ~1–2 KLOC. The three sanctioned persistent locations (Journal, TransparencyLog, CapabilityRegistry::tokens) populated here.

**Acceptance demo:** `maosctl run hello-spirit` within 5 minutes of fresh install; Transparency Log query shows every external call with capability token + Spirit-PID + boot-nonce; J0 evaluator hello-spirit budget met.

### Stories

## Story 1b.1: Three Audit Logs — Transparency / Approval Decision / Lifecycle Journal

As a substrate auditor,
I want three distinct, durable, append-only logs (Transparency Log per-Host SQLite, Approval Decision Log per I4, Lifecycle Journal per I10) with kernel-level log-before-deliver guarantees,
So that every external call, every approval-prompt resolution, and every lifecycle state transition is journaled before it takes effect — making the substrate's behavior mechanically auditable.

**Acceptance Criteria:**

**Given** a Spirit emits an IAC frame routable through the kernel
**When** the kernel writes the frame to the Transparency Log
**Then** the write completes before routing the frame to the recipient mailbox (I2 log-before-deliver guarantee)
**And** if the log write fails, the kernel panics rather than silently dropping the frame
**And** the log entry includes the capability token, Spirit-PID, boot-nonce, and timestamp

**Given** an Approval Manager prompt resolution event (intent + decision + reasoning)
**When** the kernel processes the approval decision
**Then** the resolution is written to the Approval Decision Log (distinct from Transparency Log per I4)
**And** the log entry captures `(actor, target, capability, intent, decision, reasoning_if_any)`

**Given** a Spirit lifecycle state transition (load / start / pause / resume / unload / crash / swap)
**When** the kernel records the transition
**Then** the transition is appended to the Lifecycle Journal (I10)
**And** an `fsync` is issued per state transition
**And** the ring-buffer flush latency is < 1ms P99 (NFR-Rel-8)
**And** crash recovery on next boot rehydrates Spirit state from the journal

**Given** all three logs are SQLite-backed per-Host
**When** an operator queries any log via `maosctl audit` (full audit surface in E9)
**Then** logs export to JSONL with applied redaction policy
**And** the pre-write secret-redaction filter (corpus from E0; adapter wired here) blocks secret leakage at the Transparency Log boundary
**And** logs are append-only — no deletion path except via GDPR Article 17 cascade (E9)

## Story 1b.2: Capability Registry Decomposition Runtime — cap-tokens / cap-policy / cap-audit / cap-quota

As a capability-mediation guarantor,
I want the Capability Registry decomposed at runtime per ADR-030 into four cooperating components (`cap-tokens` lock-free hot path, `cap-policy` read-mostly copy-on-write, `cap-audit` slow-path single-writer task, `cap-quota` per-Spirit atomic counters) with capability-token TTL ≤60s + Spirit-PID/boot-nonce binding + TOCTOU re-validation at every use,
So that every external call (file op, network, exec, sub-Spirit spawn) is mechanically mediated and the mediation hot path stays under 5µs P99.

**Acceptance Criteria:**

**Given** the `cap-tokens` shard (`Arc<[CapShard; 64]>` lock-free)
**When** a capability-token issuance or verification is dispatched
**Then** the hot-path latency is P99 < 5µs (per architecture-minimal-opus performance budgets)
**And** validation is 100% re-validation at use against current state, never cached state (TOCTOU correctness, NFR-Maint-8)
**And** tokens are bound to `(Spirit-PID + boot-nonce + expiry)` and Ed25519-signed via `CryptoProvider`
**And** capability-token TTL ≤60s for high-privilege operations (ADR-023)

**Given** the `cap-policy` component
**When** an operator updates a policy at runtime
**Then** the update is read-mostly copy-on-write — readers never block on writers
**And** policy reads include the strictest-of-(manifest, trust-tier, operator-policy) floor

**Given** the `cap-audit` component
**When** a capability use is observed
**Then** the audit event is enqueued onto a bounded `tokio::sync::mpsc::channel(8192)` to a single audit-writer task
**And** the audit-writer task writes to the Transparency Log
**And** the hot path never blocks on audit writes

**Given** the `cap-quota` component
**When** a Spirit's quota approaches its budget
**Then** the kernel emits `ContextPressure` at 80% utilization
**And** emits `ContextLimit` at 95%
**And** rejects further capability requests with `EContextExhausted` above 100%

**Given** the v0.1 ship gate
**When** capability-token validation is benchmarked on a typical Linux box (NVMe + 16-core tier)
**Then** P99 < 100µs is verified per NFR-Perf-3
**And** a corpus of 1000 capability calls shows 100% mediation (FR4 floor)

## Story 1b.3: Sandbox Tier T0/T1/T2 Enforcement + Per-Spirit Resource Caps

As an operator,
I want sandbox tiers T0 (trusted local) / T1 (process isolation + UID separation) / T2 (Linux Landlock+seccomp; macOS Seatbelt; Windows restricted-token) enforced per-Spirit with strictest-of-(manifest, trust-tier, operator-policy) floor, AND per-Spirit resource caps enforced via cgroups v2 / setrlimit / Job Object,
So that a `public-untrusted` Spirit declaring T0 is forced to T2 — sandbox enforcement cannot be downgraded by a Spirit's manifest claim.

**Acceptance Criteria:**

**Given** a Spirit manifest declaring a sandbox tier
**When** the Security Manager admits the Spirit
**Then** the effective tier is the strictest of (manifest, trust-tier, operator-policy)
**And** a `public-untrusted` Spirit declaring T0 is forced to T2 regardless of manifest
**And** the effective tier is journaled to the Lifecycle Journal

**Given** a Spirit running under T2 on Linux
**When** the Spirit attempts a syscall outside its declared capability scope
**Then** Landlock + seccomp blocks the syscall at the kernel boundary
**And** the block is recorded in the Transparency Log via `cap-audit`

**Given** a Spirit running under T2 on macOS
**When** the Spirit attempts a forbidden operation
**Then** the Seatbelt `.sbpl` profile blocks the operation
**And** the block is journaled

**Given** a Spirit running under T2 on Windows
**When** the Spirit attempts an operation outside its restricted token
**Then** the Windows access check fails
**And** the failure is journaled

**Given** per-Spirit resource caps declared in manifest `[resources]` (FR6 basic)
**When** the kernel spawns the Spirit
**Then** cgroups v2 (Linux) / setrlimit (macOS) / Job Object (Windows) enforces CPU and memory caps OS-natively, not via Tokio cooperation
**And** the strictest-of (manifest, operator-policy) applies

## Story 1b.4: Freeze the ComplianceClaim Schema and Wire the Inference Port + IAC Telemetry

As a kernel observability lead,
I want the ComplianceClaim schema FROZEN (after the E0 adversarial review report is signed off), the Inference Port operational with the Anthropic provider, AND the IAC round-trip telemetry metrics binding from v0.1 (`iac_rt_duration_us` histogram with documented buckets, `iac_rt_inflight` gauge, `iac_rt_errors_total` counter),
So that the substrate's compliance posture is ABI-stable, every Spirit obtains LLM inference exclusively through the kernel (FR47), and operators can observe runtime SLOs from day one.

**Acceptance Criteria:**

**Given** the E0 adversarial-review report for the ComplianceClaim schema is signed off
**When** the schema is frozen in `maos-spirit-abi/src/compliance.rs`
**Then** the schema's `ABI_VERSION` is committed
**And** the structural validator (~200 LOC in `maos-kernel-core/compliance/`) accepts well-formed claims with 100% schema validation and 100% emit-rate
**And** any future schema change to required fields, removed fields, renames, type-changes, or `Verdict`/`PrincipleRef`/`EvidenceKind` enum reorderings triggers an ABI break (`ABI_VERSION` bump) per §8.5

**Given** a Spirit attempts to import a vendor LLM SDK directly (e.g., `anthropic` crate)
**When** the kernel-API surface invariant runs (Story 0.2) or the manifest-time capability check runs
**Then** the build fails with `FR47 violation: Spirit must obtain inference via kernel Inference Port`

**Given** the Inference Port with Anthropic provider configured
**When** a Spirit invokes `kernel.infer(prompt, options)`
**Then** the call is routed through `maos-providers` to Anthropic
**And** the call is recorded in the Transparency Log with provider attribution
**And** the response is returned to the Spirit without exposing provider-specific SDK types

**Given** the IAC telemetry binding-v0.1
**When** any kernel service call traverses the IAC pipeline
**Then** `iac_rt_duration_us` is observed with labels `service ∈ {security, memory, iac, capability, spirit_scheduler}` and `outcome ∈ {ok, err, timeout}`
**And** the histogram buckets are exactly `[50, 75, 100, 150, 200, 300, 450, 700, 1000, 1500, 2200, 3300, 5000, 7500, 11000, 16000, 25000, +Inf]` (anchored on 1500µs SLO)
**And** `iac_rt_inflight` and `iac_rt_errors_total` are exposed as Prometheus-compatible metrics

## Story 1b.5a: Ship hello-Spirit Reference Binary and Hit NFR-Onb-2 5-Minute Evaluator Path

As a first-time evaluator,
I want to clone the MAOS repo and reach a structured hello-Spirit response within 5 minutes on a fresh OS install,
So that NFR-Onb-2 (the v0.1 ship gate that says "trust the substrate in 5 minutes") is mechanically reproducible — not aspirational.

**Acceptance Criteria:**

**Given** `spirits/hello-spirit/src/lib.rs` compiled against the frozen `maos-spirit-abi` crate from Story 1a.1
**When** `cargo test -p maos-spirit-hello -- test_manifest_validates` runs
**Then** the manifest at `spirits/hello-spirit/manifest.toml` parses without error against the schema published by Story 1b.4
**And** the manifest declares non-empty `capability_scope`, `expected_halt_tags`, and `transparency_log_url` fields
**And** the test is wired into the CI matrix

**Given** `maosctl run hello-spirit` on a clean Linux or macOS install (no prior cargo cache, no MAOS state directories)
**When** an operator times the path from `git clone` to first structured response on stdout
**Then** elapsed wall-clock is ≤5 minutes (NFR-Onb-2)
**And** the response JSON contains keys `introduction`, `capability_scope`, `halt_tags`, `transparency_log`
**And** `tests/integration/onb_nfr2_timing.sh` reproduces this measurement in CI and fails if elapsed > 300s

**Given** `spirits/hello-spirit/src/lib.rs` invoking the Inference Port from Story 1b.4
**When** the latency benchmark `crates/maos-bench/benches/hello_spirit_p95.rs` (using `criterion`) runs over 20 consecutive calls
**Then** P95 latency is ≤400ms (J0 budget per §13.1)
**And** the bench is CI-gated via `cargo bench --bench hello_spirit_p95 -- --test` in fail-on-regress mode

**Given** the ABI freeze in Story 1a.1
**When** `cargo build -p maos-spirit-hello --locked` runs
**Then** the build succeeds with zero `unsafe` blocks outside `crates/maos-kernel-core/`
**And** the stripped Spirit binary is ≤10MB on Linux x86_64

## Story 1b.5b: maosctl audit query + FR4 100%-Mediation Mechanical Verification

As an evaluator who has just seen hello-Spirit respond,
I want `maosctl audit query --spirit hello-spirit` to enumerate every external call the Spirit made with its issuing capability token, Spirit-PID, and boot-nonce, AND a 1000-call fixture proving 100% mediation,
So that FR4 (every external call mediated) is mechanically verified — not asserted in a README.

**Acceptance Criteria:**

**Given** `crates/maos-bin/src/cmd/audit.rs` implementing `maosctl audit query --spirit <name>`
**When** the command runs against a hello-Spirit session log at `~/.local/share/maos/audit/<session-id>.jsonl`
**Then** stdout is NDJSON where each line contains `{ "call_id", "capability_token", "spirit_pid", "boot_nonce", "call_type", "timestamp_ns" }`
**And** missing any of the five fields fails with exit code 2
**And** the schema is enforced by `crates/maos-audit/tests/query_schema_test.rs`

**Given** the FR4 verification fixture at `crates/maos-audit/tests/fixtures/hello-spirit-1k.jsonl` (1000 pre-recorded audit entries generated by `scripts/gen_hello_spirit_fixture.sh`)
**When** `cargo test -p maos-audit -- test_fr4_full_mediation` runs
**Then** 1000/1000 entries carry non-null `capability_token`, `spirit_pid`, and `boot_nonce`
**And** the test fails fast on the first missing field (no silent pass)
**And** the fixture-generation script is checked into the repo and reproducible

**Given** `maosctl audit query --spirit hello-spirit --format plain`
**When** `TERM=dumb` or `NO_COLOR=1` is set in the environment
**Then** stdout contains no ANSI escape codes
**And** the assertion is wired in `crates/maos-bin/tests/audit_no_color_test.rs` by checking `stdout.bytes().filter(|b| *b == 0x1b).count() == 0`

**Given** the Transparency Log adapter in `crates/maos-audit/src/journal.rs` from Story 1b.1
**When** `maosctl audit query` joins capability-token data from `cap-audit` (Story 1b.2) with frame data from the Transparency Log
**Then** every call surfaces its issuing token + Spirit-PID + boot-nonce
**And** the join is documented as the canonical FR4 verification path in `crates/maos-audit/README.md`

## Story 1b.5c: maosctl v0.1 Lifecycle Subcommands + Accessibility Flags

As an operator scripting MAOS into a deployment pipeline,
I want `maosctl install`, `start`, `stop`, `unload`, `run` working reliably on Linux + macOS with `--plain` / `NO_COLOR` / `TERM=dumb` honored across every subcommand AND manifest field test coverage ≥3 cases per field (NFR-Test-13),
So that maosctl integrates with CI tooling and screen readers without ad-hoc workarounds.

**Acceptance Criteria:**

**Given** the five maosctl v0.1 subcommands (`install`, `start`, `stop`, `unload`, `run`) shipped in `crates/maos-bin/src/cmd/`
**When** each command runs against hello-Spirit on a fresh install
**Then** each subcommand exits 0 with the expected side-effect (lifecycle journal entry, Transparency Log row, or process state change)
**And** smoke-tested in `tests/integration/maosctl_smoke.sh` (required CI gate before v0.1 ships)

**Given** any maosctl subcommand
**When** invoked with `--plain` or with `NO_COLOR=1` or with `TERM=dumb`
**Then** stdout/stderr contain no ANSI escape codes
**And** the assertion runs in `crates/maos-bin/tests/accessibility_test.rs` for all five subcommands

**Given** the manifest field test coverage gate (NFR-Test-13)
**When** the kernel-side manifest parser is exercised against `crates/maos-kernel-core/manifest/tests/fixtures/`
**Then** every manifest field has ≥3 fixture cases (well-formed, malformed-rejected, edge-case)
**And** CI enforces ≥3 cases per field via the coverage-matrix gate (Story 0.3)
**And** missing-fixture-coverage on any field is a build break

**Given** an integration test running the full v0.1 evaluator path (Story 1b.5a + 1b.5b + 1b.5c)
**When** `tests/integration/v01_evaluator_path.sh` runs end-to-end
**Then** install completes, hello-Spirit responds, audit query enumerates calls, FR4 fixture passes, and accessibility flags work — all green in one sequential CI job
**And** this composite test gates the v0.1 release tag

---
