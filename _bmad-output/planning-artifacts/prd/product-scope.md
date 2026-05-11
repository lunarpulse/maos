# Product Scope

## MVP — Minimum Viable Product (v0.1)

**Validation milestone:** the Architect reference Spirit drives a real coding task on a local repository end-to-end with approval prompts.

**In scope:**
- Kernel skeleton: scheduler + memory manager + capability registry + IAC bus + telemetry stream (architecture §4.1–§4.7); Security Manager and I/O Subsystem stubbed minimally.
- Domain crate (`maos-domain`): pure types + invariants I1–I10 declared; property tests for I1, I2, I10.
- Spirit ABI v0.1 (`maos-spirit-abi`): trait + wire-protocol shapes.
- Spirit SDK v0.1 (`maos-spirit-sdk`): trait export, harness library, declare_spirit! macro.
- One reference Spirit: **Architect** (in-process Rust, `rust-inproc` form only — subprocess and WASM deferred).
- One LLM provider adapter: Anthropic only (`anthropic` feature in `maos-providers`).
- T0/T1 sandbox only (no real OS-native sandbox); T2/T3 deferred to v0.5; T4 WASM-tool deferred to v1.0.
- SQLite persistence (`maos-persistence` with `sqlite` feature); Postgres deferred to v1.5.
- OS keychain for secrets (`maos-secrets` with `keyring` feature).
- HTTP control plane only (Unix socket deferred to v0.5).
- Basic MCP client (`maos-mcp`) for tool-server smoke tests.
- `maosctl` CLI: load / invoke / unload commands.
- `maos` binary as composition root.
- Six AC-style acceptance criteria:
  - **AC-V01-1:** `cargo run -- run examples/two-spirit-handshake.toml` exits 0.
  - **AC-V01-2:** One Spirit sends an Envelope to another via the kernel; both Witnesses appear in `./witnesses.jsonl` in causal order.
  - **AC-V01-3:** A Spirit attempting an action outside its declared Capability set is denied; denial Witness emitted; kernel does not panic.
  - **AC-V01-4:** `cargo test -p maos-kernel` ≥80% line coverage on `scheduler.rs` and `capability.rs`.
  - **AC-V01-5:** Reproducible build: `cargo build --locked` on Rust stable, no nightly, no `unsafe` in `maos-kernel` core.
  - **AC-V01-6:** One end-to-end test green in CI in <30s.

**Out of scope for v0.1:** subprocess Spirit form (v1.0), WASM Spirit form (v2.0), A2A peer mesh (v1.0), ACP server (v1.0), all reference Spirits beyond Architect (v0.5), Approval Manager prompt UX (v0.5), Transparency Log persistence (v0.5), Loom (v1.5), Enterprise Spirit + PDP (v2.0), Spirit registry (v2.0).

**v0.1 crate scope question (carry-forward from Step 2c):** four crates (lean position: `maos-domain`, `maos-kernel`, `maos-spirit`, `maos-bin`) versus 14 crates (kernel implementation guide's full topology). **Decision deferred to Step 8 (Scoping).**

## Growth Features (Post-MVP — v0.5 and v1.0)

**v0.5 (Realistic single-user Host):**
- Five additional reference Spirits: Butler, Researcher, Observer, Diagnostic Engineer (skeleton), Enterprise (stub).
- T2 (container) and T3 (OS-native: Landlock+seccomp / Seatbelt / Win restricted-token) sandbox tiers.
- Approval Manager prompt UX surfaces (TUI, control-plane HTTP).
- Transparency Log persistence and `maosctl audit` query CLI.
- Encrypted-file secrets backend (`maos-secrets` with `encrypted-file` feature).
- Unix-socket control plane.
- Spirit dev experience: `cargo generate maos-spirit` template; first three "bait" reference Spirits ready for adjacent OSS communities (Aider users, Continue.dev users, Neovim AI plugin authors).

**v1.0 (Team-ready, third-party Spirit ecosystem opens):**
- **Subprocess Spirit form** — first third-party-shippable form. JSON-RPC over stdio; stable Spirit Wire Protocol v1.0.
- A2A peer mesh: mTLS + TOFU + per-frame consent gates; role queries; `a2a.json` discovery.
- ACP server: editor-bridged Spirit invocation (Zed, VS Code with ACP plugin).
- T4 WASM tool sandbox: capability-isolated third-party MCP tools running under Wasmtime + WIT.
- Kernel-rendered notification surface across TUI / editor / push.
- Six reference Spirits in production-ready form (subprocess form for those that benefit).
- **Spirit registry v1.0:** MCP-Streamable-HTTP server endpoints (`registry.search`, `registry.manifest`, `registry.artifact`, `registry.publish`, `registry.deprecate`); Ed25519 signing; four trust tiers (`local`, `org-internal`, `public-untrusted`, `public-vetted`); strictest-of-(manifest, tier) sandbox/posture floor.
- Halt-recall and halt-precision benchmarks published per reference Spirit.
- Performance budgets enforced as CI gates.
- All 14 kernel invariants empirically verified.
- **Ecosystem milestone:** first "Spirit Jam" event held at v0.3; ≥5 community-authored Spirits in registry by v0.5; ≥1 non-Lunarpulse Spirit by v1.0; first cohort project interoperating cleanly with MAOS via ACP/MCP/A2A.

## Vision (v1.5 and v2.0)

**v1.5 (Diagnostic-architect pair, Loom-lite):**
- Diagnostic Engineer Spirit class with full asymmetric capability gates (read-only on production runtime knobs; bash-exec whitelist for containment actions; cross-environment telemetry queries to Architect-class Spirits).
- Per-tag epistemic policy operational — `diagnosis.root_cause` halts at `confidence_below = 0.6` or evidence conflict; `diagnosis.observation` is `verbalize_only`; `containment.action` halts at `confidence_below = 0.5`.
- Post-deploy feedback IAC topic; Architect-class Spirits subscribe to Diagnostic-class post-deploy validation results.
- Loom-lite: single-instance Postgres-backed pattern library, exposed as MCP-Streamable-HTTP server.
- `maos-persistence` Postgres support.
- **Validation:** J4 (Elena Mira-Nash) reproducible — diagnostic-architect Spirit pair closes prod-incident-to-deployed-fix loop in ≤90 minutes.

**v2.0 (Enterprise & Cortex, WASM Spirit ecosystem):**
- **WASM-component Spirit form** — third-party ecosystem capability-isolated by construction; single portable artifact; WIT contract `maos:spirit@1.0`.
- Spirit registry v2.0: vetting attestations; community-vetting authorities; OSS-style RFC process for Spirit ABI extensions; OCI-compatibility evaluation.
- Enterprise Spirit class with PDP (Policy Decision Point) integration (OPA / Cedar / Vault-style); SSO/OIDC identity assertions; encrypted-at-rest memory with org KMS; SIEM telemetry export.
- Multi-instance Loom with cross-region replication; consensus on cross-incident pattern propagation.
- Sentinel-validated canary auto-rollback; pre-deployment scanning against pattern library.
- **Validation:** Reza Cortex (single-org cross-team) reproducible at small scale — 3-region pilot deployment with ≥10 agents minimum; published case study from a federated research consortium, OSS project's own infrastructure (Debian / Wikimedia / Apache Foundation), or university public-good consortium (target consortium named by v0.3 per Step 2c carry-forward).
- **Ecosystem maturity:** ≥20 external Spirits in registry; ≥3 protocol citations from independent agent projects; ≥1 cohort project formally citing MAOS as substrate or interop reference.
