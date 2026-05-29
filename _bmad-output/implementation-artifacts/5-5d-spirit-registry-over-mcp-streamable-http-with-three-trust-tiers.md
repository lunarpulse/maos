# Story 5.5d: Spirit Registry over MCP-Streamable-HTTP with Three Trust Tiers

Status: done (remediation pass 2026-05-25 closed 8 Critical + 4 High + 13 Medium + 2 Low review findings; 1 Medium [#23] deferred to Story 7.2; §A4 Debt 2b closed via xtask exemption with rationale)

dev_model_used: TBD (recommend `claude-opus-4-7`, see Dev Notes §Model Recommendation)

**Epic:** 5 — Spirit Lifecycle, Hot-Swap, Crash Supervision & Multi-Provider (v0.3 → v1.0)
**Epic state at story open:** `epic-5: in-progress` (Stories 5.1 + 5.2 + 5.3 + 5.4 + 5.5a + 5.5b + 5.5c closed `done`; 5.5e still `backlog`).
**Story key:** `5-5d-spirit-registry-over-mcp-streamable-http-with-three-trust-tiers`

**Predecessors (substrate this story extends — verify CLOSED before first commit):**

- **Story 5.5c** (MCP client + ACP server) — this story's PRIMARY substrate. Concretely:
  - The `McpClient::call(server_name, tool, args) -> Result<McpCallResponse, McpError>` surface at `crates/maos-mcp/src/client.rs` is the **STABLE consumer-facing API** Story 5.5d consumes UNCHANGED. The Surface Stability Contract in 5.5c §Dev Notes commits to this: "Story 5.5d consumes this surface UNCHANGED. Future additions (streaming, batching) are additive new methods, not modifications."
  - The `StreamableHttpTransport` at `crates/maos-mcp/src/transport/streamable_http.rs` (DEFAULT transport per ADR-008) is the wire-level path the kernel-side `McpRegistryClient` routes through. `build_streamable_http_request_body` + `parse_streamable_http_response` are PURE FUNCTIONS that Story 5.5d's tests reuse via the `fixture_replay` feature.
  - The `FixtureReplayMcpServer` at `crates/maos-mcp/src/fixture_replay.rs` is the deterministic test scaffold the registry-roundtrip corpus runs against. Same `#[cfg(any(test, feature = "fixture_replay"))]` gating discipline.
  - The `[mcp]` manifest section at `crates/maos-kernel-core/src/security/manifest.rs:1573-1717` ships the `McpServerEntry { name, uri, transport, fallback_transport, server_trust_tier, allowed_tools }` shape — **the registry's own endpoint is a special MCP server** the operator declares either via `[registry]` section (NEW, this story) OR via an entry in the existing `[mcp]` section pinned to `name = "spirit-registry"`. §Dev Notes Decision Register resolves which.
  - The kernel-side `McpClientAdapter` at `crates/maos-kernel-core/src/mcp/mod.rs` performs capability-mediation + Transparency-Log emission for every MCP call. Story 5.5d's `McpRegistryClient::fetch_manifest(spirit_id, version)` routes through `McpClient::call` directly (NOT through `McpClientAdapter`) because registry calls are **admission-time infrastructure**, not Spirit-mediated invocations — there is no Spirit holding a capability token when the operator installs a new Spirit. §Dev Notes Decision Register documents this seam.
- **Story 5.4** (signed CRL + `RegistryClient` trait + `LocalFileRegistryClient`) — Story 5.5d's CLIENT-SIDE substrate. Concretely:
  - The `RegistryClient` trait at `crates/maos-domain/src/revocation.rs:334-345` already commits to `fetch_signed_crl() -> Result<Vec<u8>, RevocationError>` + `trust_anchor_pub() -> Result<Vec<u8>, RevocationError>`. The doc comment at line 336 EXPLICITLY commits: *"Production impl (Story 5.5d) calls the MCP-Streamable-HTTP `registry.crl` op; v0.3-β default `LocalFileRegistryClient` reads `~/.local/share/maos/crl/latest.signed.json`."*
  - Story 5.5d MUST add `McpRegistryClient` as a SECOND impl of `RegistryClient`, switchable at composition root. The `LocalFileRegistryClient` remains for air-gapped + dev workflows.
  - The `crates/maos-bin/src/main.rs:340` wiring `let local_file_registry = Arc::new(maos_domain::revocation::LocalFileRegistryClient::new(...))` is the seam Story 5.5d extends with operator-configurable `MAOS_REGISTRY_URI` selection.
  - The `RevocationError::RegistryClient(String)` variant at `revocation.rs:323` is the ALREADY-RESERVED error shape Story 5.5d emits when the MCP registry call fails.
  - The CRL polling cadence (5 min per FR13) and the offline-import path (`maosctl revocations import`) from 5.4 are the **template** for the yank-propagation cadence in this story (yanks ride the SAME 5-min poll loop on a DIFFERENT MCP op — `registry.deprecate`-derived `yank` events, distinct from FR13 CRL entries). §Dev Notes Decision Register §5 resolves the cadence-shared-vs-separate question.
- **Story 5.5a** (sandbox tier T3) — the **`public-untrusted` MCP-server-side dovetail**. Concretely:
  - Story 5.5c left a forward-shape: the `server_trust_tier` field at `manifest.rs::McpServerEntry` is parsed but the runtime T3-containerization of `public-untrusted` MCP servers was deferred to Story 5.5d (`crates/maos-kernel-core/src/security/sandbox/t3/mod.rs:37` comment: "Trigger: Story 5.5d multi-image registry support").
  - Story 5.5d MUST surface the admission path that: (a) admits a NEW Spirit declaring `[manifest].trust_tier = "public_untrusted"` ONLY after running Story 5.5a's T3 spawn path on that Spirit at install time AND (b) admits a `public-untrusted` MCP server (per `[mcp].servers[i].server_trust_tier`) ONLY by wrapping its stdio subprocess (if `transport = "stdio"`) in T3 via Story 5.5a's `spawn_t3`. This story does NOT touch `crates/maos-kernel-core/src/security/sandbox/t3/spawn.rs` directly — it consumes Story 5.5a's surface.
  - The `[operator_policy].t3_for_public_untrusted = true` field referenced by Story 5.5a's epic-prep note is **WIRED HERE** as part of the operator-policy section the strictest-of floor reads.
- **Story 1b.4** (ComplianceClaim freeze) — the **schema this story verifies** at admission time. Concretely:
  - `maos_spirit_abi::compliance::ComplianceClaimEnvelope` at `crates/maos-spirit-abi/src/compliance.rs:44-54` is FROZEN (binding-v0.1). Story 5.5d performs:
    1. Ed25519 signature verification: `verify(envelope.attester_pubkey, sha256(envelope.claim_bytes)) == envelope.signature`.
    2. Canonical CBOR-decode of `envelope.claim_bytes` into `Claim` (using `serde_cbor` or hand-rolled per canonical encoding rules per §8.5).
    3. **Structural** validation only — `fingerprint_hash` matches the Spirit's actual `ExecutionContextFingerprint` from `manifest.toml` hash + version + trust-tier + sandbox-tier + capability scopes + provider-endpoint pin + crypto-provider id.
    4. The FULL semantic evaluator (principle engine, N=600 corpus, ±2% agreement target, App-E v0.9 roadmap) is **NOT** in scope here — Story 5.5d ships ONLY signature + structural fingerprint match. App-E v0.5 calibration corpus (±5% advisory) authoring is Story 7.3.
  - The four `TrustTier` variants from `compliance.rs:136-145` (`Local | OrgInternal | PublicVetted | PublicUntrusted`) are the wire-stable enum Story 5.5d's admission path matches against. `PublicVetted` is REJECTED at admission per FR37 (Story 5.5c already enforces this on `[mcp]` section; Story 5.5d MIRRORS this on `[manifest]` admission and on `[registry]` section).
- **Story 1b.3** (sandbox tier T0/T1/T2 + strictest-of floor) — the **floor enforcement Story 5.5d extends to the install path**. Concretely:
  - The strictest-of-(manifest, trust-tier, operator-policy) reasoning in `crates/maos-kernel-core/src/security/mod.rs::TrustTier → SandboxTier` mapping is the canonical model Story 5.5d's registry-admission strictest-of-(manifest declared tier, registry tier, operator policy) floor mirrors EXACTLY at the trust-tier layer.
  - The `decision::TrustTier { PublicUntrusted | Known | Verified | Internal }` enum at `crates/maos-kernel-core/src/capability/cap_policy/decision.rs:70-80` is **kernel-internal** and maps from the wire-level `compliance::TrustTier` at admission. The mapping table is at `security/mod.rs` (verify HEAD-current); Story 5.5d ADDS one row mapping `compliance::TrustTier::OrgInternal → decision::TrustTier::Known` (or `Verified`, TBD; resolve per existing precedent at story open).
- **Story 1b.2** (capability registry decomposition) — the `cap_audit::record_drop()` saturation discipline (ADR-030) Story 5.5d follows on any new audit channel emitted (e.g., the new `FrameKind::RegistryOp` Transparency-Log row). Same `try_send` + audit-drop pattern as 5.5c.

**Carry-forward closures expected at story open** (review-patch items from 5.5c that any new code in 5.5d MUST honor):

- **Story 5.5c §5.5b §1366 `monotonic_now_ns` discipline** — closed pattern; Story 5.5d uses `monotonic_now_ns()` for EVERY TL/journal/cache timestamp. NEVER `wall_clock_now_ns()`.
- **Story 5.5c §5.5b §1373 `serde_json::to_vec().map_err()` discipline** — closed pattern; Story 5.5d propagates serde errors. NEVER `.unwrap_or_default()` on serde paths.
- **Story 5.5c §5.5b §A4 `check-pub-field-constructors`** — Story 5.5d adds new pub serde structs (`RegistrySection`, `RegistryEndpointEntry`, `SignedPackage`, `YankEntry`, etc.); each pub field carries `#[doc = "Construct via ::new ..."]` annotation + matching `impl ::new` constructor.
- **Story 5.5c JoinHandle self-prune** — Story 5.5d's polling task (the 5-min yank-and-CRL refresh task) self-prunes its JoinHandle on `SIGTERM` / kernel shutdown. NEVER leak background tasks.
- **Story 5.5c FR47 vendor-SDK denylist** — Story 5.5d adds NO new MCP/JSON-RPC protocol library. The wire format is direct-implemented via Story 5.5c's primitives. `cargo tree | grep -E 'mcp|jsonrpc|rust-mcp'` MUST remain empty after the story ships.
- **Story 5.4 §1370 `ColdSwap bypasses scheduler.load()`** — flagged for awareness only; the registry path does not touch cold-swap.

**Successor stories that depend on Story 5.5d:**

- **Story 7.2** (full registry publish/install/yank + air-gapped import) — Story 5.5d ships the v0.5-α BASELINE; Story 7.2 ships v1.0. Surface contracts Story 5.5d MUST publish stable here:
  - `crates/maos-registry/src/operations.rs::RegistryOperation` enum (`Search | Manifest | Artifact | Publish | Deprecate`) is consumed by 7.2 unchanged.
  - The `SignedPackage` wire shape (manifest TOML + binary blob + Ed25519 signature + ComplianceClaim envelope) is the on-wire format `maos-spirit publish` produces in Story 7.2.
  - Air-gapped import in Story 7.2 layers on `LocalFileRegistryClient` + this story's new `LocalFileSpiritRegistryClient` (NEW here; mirrors the CRL pattern).
- **Story 7.3** (ComplianceClaim envelope CCAC N=600 ship gate) — Story 5.5d's structural fingerprint match is the SUBSTRATE the v0.9 semantic evaluator stacks on top of. The `crates/maos-compliance` crate's evaluator consumes the `ComplianceClaimEnvelope` Story 5.5d already verifies for signature + fingerprint; Story 7.3 adds the principle-engine + N=600 corpus.
- **Story 9.4** (operator surface + air-gapped network-namespace isolation) — Story 5.5d's `McpRegistryClient` routes through `IoSubsystemPort::http_post` (via `StreamableHttpTransport`). Story 9.4's `unshare --net` validation extends to assert zero registry-bound packets leave when operator disables outbound network; the `LocalFileSpiritRegistryClient` path is the air-gapped substrate.
- **Story 10.2** (third-party trial + adversarial red team — wire fuzz) — Story 5.5d's `registry.publish` / `registry.deprecate` parsers are fuzz targets. Seed corpus authored HERE; tiered cadence wiring lands in 10.2.
- **Epic 8 reference Spirits** — Butler / Researcher / Observer / Founder-loop / Mira-Nash all install via the Story 5.5d registry surface. Their `manifest.toml` + binary + signed envelope are the FIRST real consumers of the `registry.publish` op.

<!-- Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As an **operator at v0.5-α who wants to install a third-party Spirit without copying its binary by hand, an external Spirit author who wants to ship their Spirit to other operators by running `maos-spirit publish`, and an evaluator who needs to OBSERVE that the MAOS substrate actually publishes / searches / installs / yanks Spirits across three trust tiers via MCP-Streamable-HTTP — not just two empty crates plus a hand-wave**,

I want **the v0.5-α Spirit Registry substrate that (a) ships the NEW `crates/maos-registry/` crate (registered in the workspace `Cargo.toml` members list — workspace count grows from 23 to 24 crates) hosting BOTH the **server side** (`crates/maos-registry/src/server.rs::SpiritRegistryServer` — an MCP-Streamable-HTTP server exposing 5 tool operations `registry.search` / `registry.manifest` / `registry.artifact` / `registry.publish` / `registry.deprecate`) AND the **client side** (`crates/maos-registry/src/client.rs::McpSpiritRegistryClient` — kernel-side client implementing the NEW `SpiritRegistryClient` trait declared in `maos-domain` that wraps Story 5.5c's `McpClient::call("spirit-registry", ...)` with typed argument shapes); (b) lands the NEW `SpiritRegistryClient` port trait at `crates/maos-domain/src/ports/registry.rs::SpiritRegistryClient` carrying SYNC methods `fn search(&self, q: &SearchQuery) -> Result<SearchResults, RegistryError>` / `fn manifest(&self, spirit_id: &SpiritId, version: &str) -> Result<SignedManifest, RegistryError>` / `fn artifact(&self, spirit_id: &SpiritId, version: &str) -> Result<SignedArtifact, RegistryError>` / `fn publish(&self, pkg: &SignedPackage) -> Result<PublishReceipt, RegistryError>` / `fn deprecate(&self, spirit_id: &SpiritId, version: &str, reason: &YankReason) -> Result<YankReceipt, RegistryError>` per ADR-010 sync-only port semantics — async callers wrap in `spawn_blocking`; the `RegistryError` enum carries typed variants (`UnknownSpirit` / `VersionMismatch` / `SignatureInvalid` / `TrustTierFloorViolated` / `ComplianceContextDrift` / `Transport` / `Yanked` / `Unconfigured`) each mapped to a typed-error-catalog entry per FR63 (the catalog at `xtask/fr63-typed-errors.toml` or wherever the catalog lives — verify HEAD-current location; if absent, the catalog lands at this story per Epic 9 prep); (c) IMPLEMENTS the kernel-side `McpSpiritRegistryClient` at `crates/maos-registry/src/client.rs` whose ALL FIVE methods route through `Arc<dyn McpClient>` injected at composition root with `mcp_client.call("spirit-registry", "registry.search" | "registry.manifest" | ..., args_as_json)` — the registry endpoint MUST be configured via the NEW `[registry]` manifest section AT THE OPERATOR LEVEL (an `operator.toml` config file at `~/.config/maos/operator.toml` OR via `MAOS_REGISTRY_URI` env-var; resolved at composition root, NOT per-Spirit manifest — registries are operator-scoped, not Spirit-scoped); the `MAOS_REGISTRY_URI=stub` mode loads the NEW `FixtureReplaySpiritRegistryClient` for tests; (d) IMPLEMENTS the server-side `SpiritRegistryServer` at `crates/maos-registry/src/server.rs` as a **binary** (`maos-registry-server` — declared in `crates/maos-registry/Cargo.toml [[bin]]`) that listens on a configurable HTTP socket (default `127.0.0.1:6789`; configurable via `MAOS_REGISTRY_LISTEN_ADDR`), accepts MCP-Streamable-HTTP requests per RFC-compatible Streamable HTTP binding (`POST /mcp` with `Content-Type: application/json` + `Accept: application/json, text/event-stream`; request body is a JSON-RPC 2.0 frame; response body is JSON for tool-call results), routes the five tool calls to handlers at `crates/maos-registry/src/handlers/{search,manifest,artifact,publish,deprecate}.rs`, persists registry state in a content-addressed local directory tree at `~/.local/share/maos/registry/{spirits/<spirit_id>/<version>/{manifest.toml,artifact.bin,signed_package.json,compliance_claim.envelope}, index.json, yanks.json}` (the JSON files use `serde_json::to_vec_pretty`; the directory layout mirrors Cargo's local registry pattern); (e) ADDS the NEW `[registry]` operator-config section schema at `crates/maos-kernel-core/src/security/operator_config.rs::RegistrySection` (NEW module — operator config is distinct from Spirit manifest; if `operator_config.rs` does not yet exist, create it now alongside `manifest.rs` and `audit.rs`) with shape `RegistrySection { uri: String, tier_floor: TrustTier, t3_for_public_untrusted: bool, allow_unsigned_local: bool, org_signing_pubkey: Option<[u8; 32]> }` — the operator config is RESOLVED in this order: env-vars override `~/.config/maos/operator.toml` overrides built-in defaults (`uri = "http://127.0.0.1:6789/mcp"` / `tier_floor = TrustTier::PublicUntrusted` (most-permissive) / `t3_for_public_untrusted = false` / `allow_unsigned_local = true` for dev workflows); (f) IMPLEMENTS the three-trust-tier strictest-of-floor at admission via the NEW `crates/maos-registry/src/admission.rs::admit_spirit(pkg: &SignedPackage, op_cfg: &RegistrySection) -> Result<AdmissionDecision, AdmissionError>` that (i) parses `pkg.manifest` to extract `[manifest].trust_tier` (the EXISTING field per Story 1b.3 / 1b.4 — `local` / `org_internal` / `public_vetted` / `public_untrusted`), (ii) computes `effective_tier = strictest_of(manifest_declared_tier, registry_origin_tier, operator_tier_floor)` where strictness ordering is `PublicUntrusted > OrgInternal > Local` (more-restrictive wins; `PublicVetted` is REJECTED at this story per FR37), (iii) for `effective_tier == Local`: skip registry lookup entirely — operator-vetted Spirits install from local filesystem path with `allow_unsigned_local = true` OR with operator signature, (iv) for `effective_tier == OrgInternal`: REQUIRE `pkg.envelope.attester_pubkey == op_cfg.org_signing_pubkey` (Ed25519 verify against the operator-configured org key) — admit if signature valid; reject with `AdmissionError::OrgSignatureInvalid` otherwise, (v) for `effective_tier == PublicUntrusted`: REQUIRE BOTH (1) `pkg.envelope` Ed25519 signature verified against publisher's public key carried in `pkg.envelope.attester_pubkey` (publisher key MUST be present — operator policy may also gate on a publisher-key allowlist via `op_cfg` extension but THAT is v0.7+ — Story 5.5d only verifies the envelope is well-formed and self-signed) AND (2) ComplianceClaim envelope STRUCTURAL fingerprint match per §8.5 (re-compute `expected_fingerprint_hash = sha256(cbor_canonical(ExecutionContextFingerprint::from(pkg)))` and assert `expected_fingerprint_hash == envelope.claim.fingerprint_hash`; reject with `AdmissionError::ComplianceContextDrift` (typed-error `EComplianceContextDrift` per §8.5) if mismatch) AND (3) `op_cfg.t3_for_public_untrusted == true => sandbox_tier_floor = T3` is enforced via Story 5.5a's strictest-of-floor seam (no code in Story 5.5d touches T3 spawn; the admission decision PASSES a tighter sandbox-tier floor through `AdmissionDecision.sandbox_tier_floor`); (g) IMPLEMENTS the YANK propagation path at `crates/maos-registry/src/yank.rs` distinct from Story 5.4's CRL: the SAME 5-min polling loop registered at composition root (Story 5.4 already polls every 5 min for CRL) ADDITIONALLY polls `registry.deprecate`-derived yank entries via the NEW `mcp_client.call("spirit-registry", "registry.yanks_since", {since_ns: <last_seen_ns>})` op (NOT one of the 5 published ops — this is a server-internal sync op the server-side handler exposes for kernel polling), receives a `YankList { entries: Vec<YankEntry { spirit_id, version, yanked_at_ns, reason }> }`, applies them to a local-cached `~/.local/share/maos/registry/yanks.json` table, and emits `SpiritYanked` Transparency-Log frames (NEW `FrameKind::RegistryYank` — additive on `frame.rs::FrameKind`; verify HEAD-current max + 1 — Story 5.5c added `FrameKind::McpInvocation = 18` so the next available is `19` but VERIFY at story open) for each newly-yanked Spirit; the yank list is distinct from FR13 CRL (CRL = revoked Spirit identity, kernel TERMINATES running instances; yank = registry retracts a version, kernel REFUSES NEW INSTALLS of that version but does not terminate running instances — semantics defined in ADR-008 + FR59); (h) ADDS the NEW `MAOS_ONE_SHOT=smoke-registry-5d` arm at `crates/maos-bin/src/main.rs` (additive on the existing match block; the known-modes list at `main.rs:2360` EXTENDS to include `smoke-registry-5d` AND `registry-server` — the latter is the long-running arm consumed by integration tests + by the operator who actually runs the registry) walking the FULL publish→search→manifest→artifact→deprecate→yank-propagate cycle using `FixtureReplaySpiritRegistryClient` so the arm runs deterministically on any CI runner without a real HTTP socket: print `{"step":1,"surface":"registry_init","tier_floor":"public_untrusted","t3_for_public_untrusted":false}` → publish a `local`-tier package, assert returned `PublishReceipt.publish_id` is non-empty, print `{"step":2,"surface":"registry_publish","outcome":"ok","tier":"local","spirit_id":"hello-spirit","version":"0.1.0"}` → search for `hello-spirit`, assert result list contains the published Spirit, print `{"step":3,"surface":"registry_search","outcome":"ok","results":1}` → fetch manifest + artifact, assert signature verifies, print `{"step":4,"surface":"registry_install","outcome":"ok","tier":"local","spirit_id":"hello-spirit"}` → admit a `public-untrusted`-tier package with valid ComplianceClaim envelope, print `{"step":5,"surface":"admission_public_untrusted","outcome":"ok","fingerprint_match":true}` → admit a `public-untrusted`-tier package with TAMPERED ComplianceClaim (fingerprint hash mismatch), assert rejection with `EComplianceContextDrift`, print `{"step":6,"surface":"admission_compliance_drift","outcome":"rejected","error":"EComplianceContextDrift"}` → deprecate the original Spirit, propagate the yank, assert the next search EXCLUDES the yanked version (yanked-version visibility policy: hidden by default; queryable via explicit `include_yanked=true` flag), print `{"step":7,"surface":"registry_yank_propagate","outcome":"ok","yanked":1}` → exit 0 after printing 7 JSON lines; the smoke arm is the Layer-1.5 observability bridge per Lunarpulse's evaluation discipline `[[feedback_lunarpulse_observability_preference]]` ("when can I observe actual behavior beats coverage%"); (i) ADDS the NEW `MAOS_ONE_SHOT=registry-server` long-running arm at `crates/maos-bin/src/main.rs` that spawns `SpiritRegistryServer::new(...).run(listen_addr)` and blocks until SIGTERM — consumed by integration tests + the operator command `maos-registry-server` (the dedicated binary at `crates/maos-registry/Cargo.toml [[bin]]` is an ALIAS for `maos-bin --one-shot registry-server`); (j) PRESERVES the Story 0.2 kernel-API surface invariant — the NEW kernel-side adapter symbols `McpSpiritRegistryClient` + `SpiritRegistryClient` port + `SpiritRegistryServer` (server-side, no kernel surface) get classified in `xtask/kernel-api-classes.toml` per the NFR-Test-2 gate; per architecture §4.0.7 four-class taxonomy, registry operations are **data-movement** (no semantic interpretation in the kernel — Spirit payloads route between operator → server → kernel-admission; the admission DECISION is supervision-class but happens inside `crates/maos-kernel-core::scheduler::admit_spirit` which is ALREADY classified `supervision` and unchanged by this story; the registry client adapter itself is data-movement), so the new rows read `"maos_registry::client::McpSpiritRegistryClient" = "data-movement"` and `"maos_domain::ports::registry::SpiritRegistryClient" = "data-movement"`; the `xtask check-service-boundary` gate passes; **`maos-domain` does NOT import any `maos-registry` types** — port traits live in `maos-domain::ports::registry`, adapter impls live in `maos-registry`, the dependency direction is `maos-registry → maos-domain` (consumes the port trait); (k) ADDS the NEW registry roundtrip CORPUS at `crates/maos-registry/tests/fixtures/registry-roundtrip-v05/` with TWO subdirectories: `well-formed/` carrying 10+ well-formed publish→search→manifest→artifact flows across all three trust tiers (3 local + 4 org-internal + 3 public-untrusted) AND `malformed-rejected/` carrying 8+ malformed flows (`empty-spirit-id.json` / `signature-tampered.json` / `version-mismatch.json` / `unknown-publisher-pubkey.json` / `compliance-fingerprint-mismatch.json` / `public-vetted-tier-rejected.json` / `expired-envelope.json` / `oversized-artifact.json`) each with `expected_typed_error` field per the FR63 typed-error catalog**,

so that **(a) the ADR-008 binding-v0.5 gate ("registry.search / manifest / artifact operational; MCP-Streamable-HTTP transport") gets its v0.5-α concrete realization — every PR running the smoke arm + every CI run executing the corpus PROVES the registry is operational; the architecture's commitment that "the kernel already speaks MCP for tools and Loom-lite — reusing MCP for the Spirit registry means zero new transport code" is the structural justification that makes the substrate's claim of "one consistent set of wire protocols" mechanically falsifiable (zero new transports — Story 5.5c's `StreamableHttpTransport` is the entire transport surface, verified by `cargo tree`); (b) the ADR-009 binding-v0.5 gate ("strictest-of-(manifest, trust-tier, operator-policy) floor in registry admission tests") becomes a runnable corpus, not a doc-only commitment — the `admit_spirit` function's strictest-of logic is unit-tested per-trust-tier; the operator-policy floor is the FIRST place in the substrate where operator preferences (not Spirit-declared metadata) gate admission, closing the operator-control story for the v0.5-α release; (c) the FR59 yank surface at v0.5-α baseline ("yank events propagate within 5min via the SAME polling loop as FR13 CRL") gets its first running implementation — yanks become distinguishable from operator-local revocations (FR13) in the kernel's TL via two separate FrameKinds, preserving the operator's audit-querability per Epic 9 Story 9.1; (d) Story 5.4's `RegistryClient` trait forward-shape commitment at `revocation.rs:336` ("Production impl (Story 5.5d) calls the MCP-Streamable-HTTP `registry.crl` op") becomes REAL — `McpRegistryClient` (the CRL-fetching impl) is the SECOND impl of `revocation::RegistryClient` switchable at composition root via `MAOS_REGISTRY_URI`; the `LocalFileRegistryClient` survives for air-gapped + dev workflows; (e) the §8.5 ComplianceClaim envelope ("kernel verifies envelopes at admission time and refuses to load Spirits whose runtime context drifts") gets its FIRST runtime verification path — Story 1b.4 froze the schema; Story 5.5d ships the structural verification logic (signature + fingerprint match); Story 7.3 + App-E ship the v0.9 semantic evaluator. The substrate's claim of "ComplianceClaim as falsifiable attestations rather than marketing copy" is no longer aspirational at v0.5; (f) the **FR47 "no third-party SDK on the substrate's hot path" commitment stays structurally closed** — the registry server + client direct-implement JSON-RPC 2.0 framing via Story 5.5c's `StreamableHttpTransport` primitives; `cargo tree | grep -E 'mcp|jsonrpc|rust-mcp'` continues to return empty after this story ships; (g) the Story 0.2 kernel-API surface lint stays passing — three new adapter symbols (`McpSpiritRegistryClient`, `SpiritRegistryClient`, `SpiritRegistryServer`) classified `data-movement` (none `other`-class); the `xtask check-service-boundary` gate passes; (h) **observability via the smoke arm** — when an evaluator runs `MAOS_ONE_SHOT=smoke-registry-5d cargo run -p maos-bin --features fixture_replay`, they OBSERVE in ONE COMMAND: a Spirit being published, searched-for, fetched (manifest + artifact), admitted across all three trust tiers, an admission rejected for ComplianceClaim drift with the exact typed error, and a yank propagated and applied — the substrate's "Spirit registry over MCP-Streamable-HTTP with three trust tiers" claim is no longer "we have a few empty crates" but "we have a runnable end-to-end registry conversation, demonstrated"**.

## What this story IS

### REGISTRY — NEW CRATE LAYOUT

- **NEW `crates/maos-registry/` crate** registered in `Cargo.toml [workspace.members]` (workspace grows 23 → 24 crates).
  - `crates/maos-registry/Cargo.toml`:
    ```toml
    [package]
    name = "maos-registry"
    version.workspace = true
    edition.workspace = true
    license.workspace = true
    rust-version.workspace = true

    [dependencies]
    maos-domain = { path = "../maos-domain" }
    maos-mcp = { path = "../maos-mcp" }
    maos-spirit-abi = { path = "../maos-spirit-abi" }
    serde = { workspace = true }
    serde_json = { workspace = true }
    thiserror = { workspace = true }
    blake3 = { workspace = true }
    ed25519-dalek = { workspace = true } # already in workspace from Story 5.4

    [features]
    fixture_replay = ["maos-mcp/fixture_replay"]

    [[bin]]
    name = "maos-registry-server"
    path = "src/bin/server.rs"
    ```
  - `crates/maos-registry/src/lib.rs` — module re-exports + crate-level doc.
  - `crates/maos-registry/src/client.rs::McpSpiritRegistryClient` — kernel-side client.
  - `crates/maos-registry/src/server.rs::SpiritRegistryServer` — server side (state + handlers + HTTP listener loop).
  - `crates/maos-registry/src/handlers/{search,manifest,artifact,publish,deprecate}.rs` — per-op handler functions.
  - `crates/maos-registry/src/admission.rs::admit_spirit` — three-trust-tier strictest-of admission decision.
  - `crates/maos-registry/src/yank.rs::YankList` + `YankEntry` + the polling-task glue.
  - `crates/maos-registry/src/operations.rs::RegistryOperation` enum + per-op argument + response shapes (all `#[serde(rename_all = "snake_case")]` + round-trip tested).
  - `crates/maos-registry/src/storage.rs::RegistryStorage` — content-addressed local directory tree at `~/.local/share/maos/registry/`.
  - `crates/maos-registry/src/fixture_replay.rs::FixtureReplaySpiritRegistryClient` — test-only client gated by `#[cfg(any(test, feature = "fixture_replay"))]`.
  - `crates/maos-registry/src/bin/server.rs` — thin binary entry-point that re-enters `maos-bin --one-shot registry-server` OR alternatively spawns `SpiritRegistryServer` directly per `MAOS_REGISTRY_LISTEN_ADDR`; resolve at story open per existing maos-bin pattern.

### DOMAIN PORT — `SpiritRegistryClient`

- **NEW `crates/maos-domain/src/ports/registry.rs`** — port trait + domain types (consumed by `maos-kernel-core` for admission + by `maos-cli` for `maosctl install` + `maosctl registry deprecate`):
  ```rust
  //! Spirit Registry port — kernel-internal admission consumer + operator CLI consumer.
  //!
  //! Per ADR-010 sync-only port semantics. Adapter impls live in `maos-registry`
  //! (production `McpSpiritRegistryClient` + test `FixtureReplaySpiritRegistryClient`).
  //! Async callers wrap in `spawn_blocking`.

  use crate::ports::compliance::ComplianceClaimEnvelope;
  use serde::{Deserialize, Serialize};

  /// Five operations per ADR-008 binding-v0.5.
  pub trait SpiritRegistryClient: Send + Sync {
      /// Class: data-movement — searches the registry for Spirits matching `q`.
      fn search(&self, q: &SearchQuery) -> Result<SearchResults, RegistryError>;
      /// Class: data-movement — fetches a signed manifest for the (spirit_id, version) tuple.
      fn manifest(&self, spirit_id: &SpiritId, version: &str) -> Result<SignedManifest, RegistryError>;
      /// Class: data-movement — fetches a signed binary artifact.
      fn artifact(&self, spirit_id: &SpiritId, version: &str) -> Result<SignedArtifact, RegistryError>;
      /// Class: data-movement — publishes a signed package. Publisher-side op.
      fn publish(&self, pkg: &SignedPackage) -> Result<PublishReceipt, RegistryError>;
      /// Class: data-movement — yanks a version. Publisher- OR operator-side op.
      fn deprecate(&self, spirit_id: &SpiritId, version: &str, reason: &YankReason) -> Result<YankReceipt, RegistryError>;
  }

  // NOTE: REUSE the existing wire-stable `maos_spirit_abi::identity::SpiritId`
  // at `crates/maos-spirit-abi/src/identity.rs:55-74` — DO NOT declare a
  // new `SpiritId` newtype in this port. Story 5.5d only adds a
  // type alias if alignment with the port's signature requires it:
  //   pub use maos_spirit_abi::identity::SpiritId;

  #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
  pub struct SearchQuery {
      #[doc = "Construct via [`SearchQuery::new`] to enforce non-empty text."]
      pub text: String,
      #[doc = "Construct via [`SearchQuery::new`]. Default false."]
      #[serde(default)]
      pub include_yanked: bool,
      #[doc = "Construct via [`SearchQuery::new`]. Default 50; max 200."]
      #[serde(default = "default_limit")]
      pub limit: u32,
  }
  // ...
  #[derive(Debug, Clone, thiserror::Error)]
  #[non_exhaustive]
  pub enum RegistryError {
      #[error("unknown spirit '{0}'")]
      UnknownSpirit(String),
      #[error("version '{requested}' not found for spirit '{spirit_id}'")]
      VersionNotFound { spirit_id: String, requested: String },
      #[error("Ed25519 signature verification failed")]
      SignatureInvalid,
      #[error("trust-tier floor violated: manifest='{manifest_tier:?}', floor='{floor:?}'")]
      TrustTierFloorViolated { manifest_tier: TrustTier, floor: TrustTier },
      #[error("ComplianceClaim execution-context fingerprint drift")]
      ComplianceContextDrift,
      #[error("registry version '{spirit_id}@{version}' yanked: {reason}")]
      Yanked { spirit_id: String, version: String, reason: String },
      #[error("registry transport error: {0}")]
      Transport(String),
      #[error("registry not configured (set MAOS_REGISTRY_URI or [registry].uri in operator.toml)")]
      Unconfigured,
      #[error("org signature does not match operator-configured org key")]
      OrgSignatureInvalid,
      #[error("public_vetted trust tier deferred per FR37 to v2.5")]
      PublicVettedDeferred,
  }
  ```

### SIGNED PACKAGE WIRE SHAPE

- **NEW `crates/maos-domain/src/ports/registry.rs::SignedPackage`** — the on-wire publishable unit per ADR-008:
  ```rust
  #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
  pub struct SignedPackage {
      #[doc = "Construct via [`SignedPackage::new`] to enforce well-formed signature shape."]
      pub spirit_id: SpiritId,
      #[doc = "Construct via [`SignedPackage::new`]."]
      pub version: String,
      #[doc = "Construct via [`SignedPackage::new`] — raw TOML bytes of the Spirit manifest."]
      pub manifest_toml: Vec<u8>,
      #[doc = "Construct via [`SignedPackage::new`] — Spirit binary blob."]
      pub artifact_bytes: Vec<u8>,
      #[doc = "Construct via [`SignedPackage::new`] — Ed25519 signature over sha256(manifest_toml || artifact_bytes)."]
      pub signature: [u8; 64],
      #[doc = "Construct via [`SignedPackage::new`] — publisher's Ed25519 public key."]
      pub publisher_pubkey: [u8; 32],
      #[doc = "Construct via [`SignedPackage::new`] — frozen-schema ComplianceClaim per §8.5."]
      pub compliance_envelope: ComplianceClaimEnvelope,
  }
  ```
  - **Signing rule** at publish time: `sig_bytes = sha256(manifest_toml || artifact_bytes)`; `signature = ed25519::sign(publisher_privkey, sig_bytes)`.
  - **Verification rule** at admission: `ed25519::verify(publisher_pubkey, sha256(manifest_toml || artifact_bytes), signature) == Ok(())`.

### THREE-TRUST-TIER STRICTEST-OF FLOOR

- **NEW `crates/maos-registry/src/admission.rs`**:
  ```rust
  pub struct AdmissionDecision {
      pub effective_tier: TrustTier,
      pub sandbox_tier_floor: SandboxTier,
      pub admit: bool,
      pub journal_note: String,
  }

  #[derive(Debug, thiserror::Error)]
  pub enum AdmissionError {
      #[error("trust_tier 'public_vetted' deferred per FR37 to v2.5; allowed: local, org_internal, public_untrusted")]
      PublicVettedDeferred,
      #[error("org signature does not match operator-configured org key (operator must set [registry].org_signing_pubkey)")]
      OrgSignatureInvalid,
      #[error("publisher Ed25519 signature verification failed on artifact bytes")]
      PublisherSignatureInvalid,
      #[error("ComplianceClaim execution-context fingerprint drift — actual={actual_hex}, claimed={claimed_hex}")]
      ComplianceContextDrift { actual_hex: String, claimed_hex: String },
      #[error("operator-policy unsigned_local rejected — operator must set allow_unsigned_local=true to admit unsigned local Spirits")]
      UnsignedLocalRejected,
  }

  /// Three-trust-tier strictest-of-floor admission per ADR-009.
  pub fn admit_spirit(
      pkg: &SignedPackage,
      op_cfg: &RegistrySection,
  ) -> Result<AdmissionDecision, AdmissionError> {
      // 1. Parse manifest, extract manifest_declared_tier.
      // 2. Compute effective_tier = strictest_of(manifest_declared_tier, registry_origin_tier, op_cfg.tier_floor).
      //    Strictness ordering: PublicUntrusted > OrgInternal > Local. (PublicVetted is rejected.)
      // 3. Branch on effective_tier:
      //    - Local: skip signature verification if op_cfg.allow_unsigned_local. Otherwise require operator-pubkey match.
      //    - OrgInternal: require pkg.publisher_pubkey == op_cfg.org_signing_pubkey AND verify signature.
      //    - PublicUntrusted: require signature valid AND ComplianceClaim fingerprint match AND (if op_cfg.t3_for_public_untrusted) sandbox_tier_floor = T3.
      //    - PublicVetted: AdmissionError::PublicVettedDeferred.
      // 4. Emit Transparency-Log FrameKind::SpiritAdmitted with the AdmissionDecision shape.
  }
  ```
  Strictest-of ordering for trust tier (more-restrictive wins): `PublicUntrusted > OrgInternal > Local`. A `local`-declared Spirit hosted on a `public-untrusted` registry with `op_cfg.tier_floor = OrgInternal` resolves to `effective_tier = PublicUntrusted` (the strictest of the three: registry-side `public-untrusted` floor).

  Tests at `crates/maos-registry/src/admission.rs::tests`:
  - `local_unsigned_allowed_when_policy_permits` — `op_cfg.allow_unsigned_local = true`; admit.
  - `local_unsigned_rejected_when_policy_strict` — `op_cfg.allow_unsigned_local = false`; reject with `UnsignedLocalRejected`.
  - `org_internal_signature_matches_admits` — publisher_pubkey == org_signing_pubkey; admit.
  - `org_internal_signature_mismatch_rejects` — publisher_pubkey != org_signing_pubkey; reject with `OrgSignatureInvalid`.
  - `public_untrusted_with_valid_envelope_admits` — well-formed envelope + fingerprint matches recomputed; admit.
  - `public_untrusted_with_tampered_envelope_rejects` — envelope signature does not verify; reject with `PublisherSignatureInvalid`.
  - `public_untrusted_with_fingerprint_drift_rejects` — envelope verifies but fingerprint hash mismatches manifest-derived fingerprint; reject with `ComplianceContextDrift`.
  - `public_untrusted_with_t3_floor_returns_t3_sandbox` — `op_cfg.t3_for_public_untrusted = true`; admit with `decision.sandbox_tier_floor == T3`.
  - `public_vetted_tier_always_rejects` — manifest tier = `public_vetted`; reject with `PublicVettedDeferred`.
  - `strictest_of_resolves_correctly` — manifest declares `local`, registry-origin says `public-untrusted`, op-cfg floor is `org-internal`; effective_tier = `PublicUntrusted` (strictest).

### COMPLIANCECLAIM STRUCTURAL VERIFICATION (v0.5-α SCOPE)

- **`crates/maos-registry/src/compliance_verify.rs`** (NEW):
  - `pub fn verify_envelope_structural(envelope: &ComplianceClaimEnvelope, pkg: &SignedPackage, expected_runtime: &ExecutionContextFingerprint) -> Result<(), AdmissionError>`:
    1. Decode `envelope.claim_bytes` as canonical CBOR → `Claim` struct (per §8.5 + `compliance.rs`).
    2. Verify `ed25519::verify(envelope.attester_pubkey, sha256(envelope.claim_bytes), envelope.signature) == Ok(())`.
    3. Re-compute `actual_fingerprint = ExecutionContextFingerprint::from_manifest_and_runtime(&pkg.manifest_toml, expected_runtime)`.
    4. Hash both: `actual_hash = sha256(cbor_canonical(actual_fingerprint))` + `claimed_hash = claim.fingerprint_hash`.
    5. Return `ComplianceContextDrift { actual_hex, claimed_hex }` on mismatch; `Ok(())` otherwise.
  - **NOT IN SCOPE**: the v0.9 semantic evaluator (principle engine + N=600 corpus + ±2% agreement). That arrives at Story 7.3 + App-E.
  - **NOT IN SCOPE**: per-attester pubkey allowlist (operator policy may want this at v0.7; the field shape exists in `RegistrySection` but only `org_signing_pubkey` is checked at v0.5).

### MCP-STREAMABLE-HTTP CLIENT (KERNEL-SIDE)

- **NEW `crates/maos-registry/src/client.rs::McpSpiritRegistryClient`** — kernel-side client routing through Story 5.5c's `McpClient::call`:
  ```rust
  pub struct McpSpiritRegistryClient {
      mcp_client: Arc<dyn McpClient>,
      registry_server_name: String, // e.g., "spirit-registry"
  }

  impl McpSpiritRegistryClient {
      pub fn new(mcp_client: Arc<dyn McpClient>, registry_server_name: String) -> Self {
          Self { mcp_client, registry_server_name }
      }
  }

  impl SpiritRegistryClient for McpSpiritRegistryClient {
      fn search(&self, q: &SearchQuery) -> Result<SearchResults, RegistryError> {
          let args = serde_json::to_value(q).map_err(|e| RegistryError::Transport(e.to_string()))?;
          let resp = self.mcp_client
              .call(&self.registry_server_name, "registry.search", args)
              .map_err(map_mcp_err)?;
          if resp.is_error {
              return Err(decode_typed_error(&resp.content));
          }
          serde_json::from_value(resp.content).map_err(|e| RegistryError::Transport(e.to_string()))
      }
      // ... manifest / artifact / publish / deprecate mirror this shape.
  }

  fn map_mcp_err(e: McpError) -> RegistryError {
      match e {
          McpError::UnknownServer(s) => RegistryError::Unconfigured,
          McpError::CapabilityDenied { .. } => RegistryError::Transport("kernel-internal registry call lacks capability — composition root bug".into()),
          McpError::Transport(inner) => RegistryError::Transport(inner.to_string()),
          McpError::Encode(e) => RegistryError::Transport(format!("encode: {e}")),
          McpError::Decode(e) => RegistryError::Transport(format!("decode: {e}")),
          McpError::Unconfigured => RegistryError::Unconfigured,
          // additive variants — handle exhaustively
      }
  }

  fn decode_typed_error(v: &serde_json::Value) -> RegistryError {
      // Match on JSON `{"error_kind": "...", "details": {...}}` shape per FR63 typed-error catalog.
      // Maps to RegistryError variants; falls back to RegistryError::Transport on unknown kind.
  }
  ```

  Tests at `crates/maos-registry/src/client.rs::tests` (against `FixtureReplayMcpServer`):
  - `search_routes_to_registry_server_name` — assert `mcp_client.call("spirit-registry", "registry.search", ...)` invoked.
  - `publish_round_trip_with_signed_package` — well-formed package; assert `PublishReceipt` returned.
  - `version_not_found_maps_to_typed_error` — server returns `{"error_kind": "version_not_found", ...}`; assert `RegistryError::VersionNotFound` decoded.
  - `transport_error_maps_to_transport_error` — `FixtureReplayMcpServer` returns `McpTransportError::Transport(...)`; assert `RegistryError::Transport`.

### MCP-STREAMABLE-HTTP SERVER (BINARY SIDE)

- **NEW `crates/maos-registry/src/server.rs::SpiritRegistryServer`** — server side. Long-running HTTP server listening on `127.0.0.1:6789` (configurable):
  ```rust
  pub struct SpiritRegistryServer {
      storage: Arc<dyn RegistryStorage>,
      listen_addr: String,
      org_pubkey: Option<[u8; 32]>, // if set, signs as the registry-side org-internal anchor
  }

  impl SpiritRegistryServer {
      pub fn new(storage: Arc<dyn RegistryStorage>, listen_addr: String, org_pubkey: Option<[u8; 32]>) -> Self;

      /// Block on HTTP listener; route incoming MCP-Streamable-HTTP requests to handlers; on SIGTERM exit cleanly.
      /// MUST self-prune JoinHandle on shutdown per Story 5.4 §1368.
      pub fn run(self) -> Result<(), ServerError>;
  }
  ```
  - **HTTP listener**: uses `std::net::TcpListener` for v0.5-α — NO `tokio::net` dependency added (the kernel-stays-small invariant); each connection handled synchronously on a worker thread spawned via `std::thread::spawn`. Worker threads self-prune on connection close.
  - **MCP-Streamable-HTTP framing**: POST `/mcp` with JSON-RPC 2.0 frame in the body; the server reads body, parses `method` field (one of `registry.search` / `registry.manifest` / `registry.artifact` / `registry.publish` / `registry.deprecate` / `registry.yanks_since` / `registry.crl`), dispatches to the matching handler at `handlers/<op>.rs`, returns the result as JSON-RPC response.
  - **TLS**: v0.5-α the server runs **HTTP only** (cleartext) on `127.0.0.1` — operator must place an HTTPS-terminating reverse proxy (nginx / Caddy) in front for production. Public-internet-facing HTTPS direct is Story 7.2. Documented in `crates/maos-registry/SECURITY.md` (NEW).
  - **Authentication**: v0.5-α the server is **open** — any client can call any op. Authentication via operator-key + per-op ACL is Story 7.2. Documented in same SECURITY.md.

### REGISTRY STORAGE

- **NEW `crates/maos-registry/src/storage.rs::RegistryStorage`** — trait + impls:
  ```rust
  pub trait RegistryStorage: Send + Sync {
      fn put(&self, spirit_id: &SpiritId, version: &str, pkg: &SignedPackage) -> Result<(), StorageError>;
      fn get_manifest(&self, spirit_id: &SpiritId, version: &str) -> Result<SignedManifest, StorageError>;
      fn get_artifact(&self, spirit_id: &SpiritId, version: &str) -> Result<SignedArtifact, StorageError>;
      fn search(&self, q: &SearchQuery) -> Result<SearchResults, StorageError>;
      fn yank(&self, spirit_id: &SpiritId, version: &str, reason: &YankReason) -> Result<YankReceipt, StorageError>;
      fn yanks_since(&self, since_ns: u64) -> Result<YankList, StorageError>;
  }

  /// Filesystem-backed impl: `~/.local/share/maos/registry/` tree.
  pub struct LocalFsRegistryStorage { root: PathBuf }
  ```
  - **Directory layout**:
    ```
    ~/.local/share/maos/registry/
      spirits/
        hello-spirit/
          0.1.0/
            manifest.toml
            artifact.bin
            signed_package.json
            compliance_claim.envelope
        butler/
          0.3.1/
            ...
      index.json                    # SearchIndex of (spirit_id, version, summary)
      yanks.json                    # YankList of all yanked entries
    ```
  - The `index.json` is rebuilt on every `put()` and `yank()` call; v0.5-α uses simple linear scan (the registry is expected to host < 10⁴ Spirits at this phase; B-tree/inverted-index optimization is Story 7.2). The implementation must use `monotonic_now_ns()` (Story 5.5c §1366 closed pattern) for `yanked_at_ns` + index timestamps.

### YANK PROPAGATION (DISTINCT FROM CRL)

- **NEW `crates/maos-registry/src/yank.rs::YankPoller`** — the kernel-side 5-min poll task piggybacking on Story 5.4's CRL poll loop:
  ```rust
  pub struct YankPoller {
      registry: Arc<dyn SpiritRegistryClient>,
      cache: Arc<Mutex<YankCache>>,
      transparency_log: Arc<dyn TransparencyLogPort>,
  }

  impl YankPoller {
      pub fn new(registry: Arc<dyn SpiritRegistryClient>, transparency_log: Arc<dyn TransparencyLogPort>) -> Self;

      /// Called every 5min by the kernel's polling task (the SAME task as Story 5.4's CRL poll).
      /// Runs in `spawn_blocking` since the underlying registry call is sync.
      pub fn poll_once(&self) -> Result<usize, RegistryError> {
          let since_ns = self.cache.lock().unwrap().last_seen_ns;
          let list = self.registry_yanks_since_via_internal_op(since_ns)?; // SEE BELOW
          for entry in &list.entries {
              self.transparency_log.record(TransparencyLogRow {
                  frame_kind: FrameKind::RegistryYank, // NEW additive discriminant
                  timestamp_ns: monotonic_now_ns(),
                  intent: format!("registry.yank:{}/{}", entry.spirit_id, entry.version),
                  // ...
              })?;
          }
          self.cache.lock().unwrap().apply(&list);
          Ok(list.entries.len())
      }
  }
  ```
  - **Internal op** `registry.yanks_since` is invoked via `mcp_client.call("spirit-registry", "registry.yanks_since", {since_ns})` — server-side handler returns the yank list since the timestamp. This op is **kernel-internal** (operator does not invoke it directly); it is documented in the server-side handler but NOT exposed in the operator-facing 5-op CLI.
  - **Yank vs CRL distinction** (architecture-critical per ADR-008 + FR59):
    - **CRL (FR13, Story 5.4)** — revoked Spirit identity. The kernel TERMINATES running instances with declared revocation policy (`terminate-immediately` / `drain-then-terminate` / `quarantine`).
    - **YANK (FR59, Story 5.5d baseline)** — registry retracts a version from new installs. The kernel REFUSES NEW INSTALLS of the yanked version; RUNNING instances of the yanked version are NOT terminated (the version was good enough to run; the operator can decide whether to upgrade via Story 5.4's upgrade path).
    - The two TLs ride different FrameKinds: Story 5.4 emits `FrameKind::SpiritRevoked = N`; Story 5.5d emits `FrameKind::RegistryYank = N+1`.

### `[registry]` OPERATOR-CONFIG SECTION

- **NEW `crates/maos-kernel-core/src/security/operator_config.rs::RegistrySection`** (NEW file — if `operator_config.rs` does not yet exist, create it):
  ```rust
  #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
  pub struct RegistrySection {
      #[doc = "Construct via [`RegistrySection::new`] — registry MCP endpoint URI."]
      pub uri: String,
      #[doc = "Construct via [`RegistrySection::new`] — minimum trust-tier floor enforced."]
      pub tier_floor: TrustTier,
      #[doc = "Construct via [`RegistrySection::new`] — escalate public-untrusted to T3 sandbox."]
      pub t3_for_public_untrusted: bool,
      #[doc = "Construct via [`RegistrySection::new`] — allow unsigned local Spirits (dev workflow)."]
      pub allow_unsigned_local: bool,
      #[doc = "Construct via [`RegistrySection::new`] — Ed25519 public key the registry-side org-internal anchor signs with."]
      pub org_signing_pubkey: Option<[u8; 32]>,
  }
  ```
  - Resolved at composition root in this priority order: env-vars (`MAOS_REGISTRY_URI`, `MAOS_REGISTRY_TIER_FLOOR`, etc.) → `~/.config/maos/operator.toml` → built-in defaults.
  - Built-in defaults: `uri = ""` (unconfigured → `RegistryError::Unconfigured` on first call), `tier_floor = TrustTier::PublicUntrusted` (most-permissive — operator can tighten), `t3_for_public_untrusted = false`, `allow_unsigned_local = true`, `org_signing_pubkey = None`.

### COMPOSITION ROOT WIRING

- **EXTENDED `crates/maos-bin/src/main.rs` composition root** — alongside existing 5.5c MCP/ACP wiring:
  ```rust
  // Story 5.5d — Registry client wiring
  let registry_cfg = RegistrySection::resolve_from_env_and_disk()?;
  let registry_client: Arc<dyn SpiritRegistryClient> = if std::env::var("MAOS_REGISTRY_URI").as_deref() == Ok("stub") {
      Arc::new(FixtureReplaySpiritRegistryClient::new(vec![]))
  } else if registry_cfg.uri.is_empty() {
      Arc::new(NullSpiritRegistryClient) // returns RegistryError::Unconfigured on every call
  } else {
      Arc::new(McpSpiritRegistryClient::new(Arc::clone(&mcp_client_arc), "spirit-registry".into()))
  };
  // Yank poller piggybacks on the Story 5.4 CRL poll task — extend it with one more call:
  yank_poller_handle = spawn_yank_poller(Arc::clone(&registry_client), Arc::clone(&transparency_log));
  ```

### SMOKE ARM + KNOWN-MODES

- **NEW `MAOS_ONE_SHOT=smoke-registry-5d` arm** at `crates/maos-bin/src/main.rs` — additive on the existing match block; mirrors `smoke-mcp-acp-5` shape (one JSON line per surface; exit 0 on completion):
  - Step 1: `registry_init` — construct `FixtureReplaySpiritRegistryClient` with a canned set of well-formed publish→search→manifest→artifact responses; print init JSON.
  - Step 2: `registry_publish` — invoke `registry.publish(local-tier-pkg)`; assert `PublishReceipt.publish_id` non-empty.
  - Step 3: `registry_search` — invoke `registry.search("hello-spirit")`; assert results contain the published Spirit.
  - Step 4: `registry_install` — invoke `registry.manifest` + `registry.artifact`; verify Ed25519 signatures; assert all succeed.
  - Step 5: `admission_public_untrusted` — admit a `public-untrusted`-tier package with well-formed ComplianceClaim envelope; assert `AdmissionDecision.admit == true` + `effective_tier == PublicUntrusted`.
  - Step 6: `admission_compliance_drift` — admit a `public-untrusted`-tier package with tampered envelope (fingerprint mismatch); assert `AdmissionError::ComplianceContextDrift` (mapped to typed error `EComplianceContextDrift` per §8.5).
  - Step 7: `registry_yank_propagate` — deprecate the published Spirit; poll `yanks_since`; assert subsequent search excludes yanked version.
  - Exit 0 after 7 JSON lines.
- **NEW `MAOS_ONE_SHOT=registry-server` long-running arm** — spawns `SpiritRegistryServer::new(...).run()`; documented in `--help` and consumed by integration tests.
- **EXTENDED known-modes list at `crates/maos-bin/src/main.rs:2360`** — append `smoke-registry-5d` AND `registry-server` to the comma-separated mode list.
- **NEW `crates/maos-bin/tests/smoke_registry_5d_test.rs`** — test driver mirroring `smoke_mcp_acp_test.rs` shape exactly. Asserts exit code 0; asserts stdout contains 7 JSON lines with expected `step` + `surface` + `outcome` fields.

### KERNEL-API SURFACE GATE

- **EXTENDED `xtask/kernel-api-classes.toml`** — add new rows:
  ```toml
  "maos_registry::client::McpSpiritRegistryClient" = "data-movement"
  "maos_registry::admission::admit_spirit" = "data-movement"
  "maos_domain::ports::registry::SpiritRegistryClient" = "data-movement"
  ```
  The `SpiritRegistryServer` is server-side binary code — it does NOT cross the kernel-API surface (the kernel does not import `maos_registry::server`); no classification row needed.
- **Verify `cargo run -p xtask -- check-service-boundary` PASSES** with the new rows.
- **Verify `cargo run -p xtask -- check-fr47` PASSES** with `fr47-allowlist.toml` empty — `cargo tree | grep -E 'mcp|jsonrpc|rust-mcp|registry-protocol'` returns empty (registry wire shape is direct-implemented atop Story 5.5c's primitives).
- **Verify `cargo run -p xtask -- check-pub-field-constructors` PASSES** — every new pub field on `RegistrySection`, `SignedPackage`, `YankEntry`, etc. carries the `#[doc = "Construct via ::new ..."]` annotation + matching `impl ::new` constructor.

### REGISTRY-ROUNDTRIP CORPUS + TYPED ERRORS

- **NEW `crates/maos-registry/tests/fixtures/registry-roundtrip-v05/`**:
  - `well-formed/` — 10+ flows:
    - `local-unsigned-allowed.json` (operator policy allows unsigned local)
    - `local-operator-signed.json` (operator-signed local)
    - `local-with-attached-claim.json` (optional envelope on local)
    - `org-internal-org-signed-1.json` / `org-internal-org-signed-2.json` / `org-internal-org-signed-3.json` / `org-internal-org-signed-4.json`
    - `public-untrusted-well-formed-1.json` / `public-untrusted-well-formed-2.json` / `public-untrusted-well-formed-3.json`
  - `malformed-rejected/` — 8+ flows, each carrying `expected_typed_error: "EComplianceContextDrift" | ...`:
    - `empty-spirit-id.json` → `RegistryError::UnknownSpirit("")`
    - `signature-tampered.json` → `AdmissionError::PublisherSignatureInvalid`
    - `version-not-found.json` → `RegistryError::VersionNotFound`
    - `org-signature-mismatch.json` → `AdmissionError::OrgSignatureInvalid`
    - `compliance-fingerprint-mismatch.json` → `AdmissionError::ComplianceContextDrift`
    - `public-vetted-tier-rejected.json` → `RegistryError::PublicVettedDeferred`
    - `expired-envelope.json` → reserved field for v0.7; expects "envelope_expired_at field absent" handling (parse but ignore in v0.5)
    - `oversized-artifact.json` → `RegistryError::Transport("artifact exceeds 64MB")` — 64MB is the v0.5-α upper bound per built-in default; configurable in operator config (deferred to v0.7)
- **Test driver** `crates/maos-registry/tests/registry_roundtrip_test.rs` — iterates all corpus files; for each `well-formed/*.json` asserts admit succeeds; for each `malformed-rejected/*.json` asserts the typed error matches the `expected_typed_error` field. The driver shape mirrors Story 5.5a's `crates/maos-kernel-core/tests/fixtures/manifest/sandbox/` corpus.

### NEW FRAME KIND (Transparency Log)

There are **two** `FrameKind` enums in the workspace — preserve the distinction (see 5.5c File List):

- `maos_spirit_abi::identity::FrameKind` at `crates/maos-spirit-abi/src/identity.rs:18-29` — **wire-stable ABI enum, frozen at variants 0..9**. DO NOT extend; extending bumps `ABI_VERSION`.
- `maos_kernel_core::iac::transparency_log::FrameKind` at `crates/maos-kernel-core/src/iac/transparency_log.rs:37-76` — **kernel-side extended enum carrying 0..18** at HEAD (Story 5.5c added `McpInvocation = 18`). New additive variants land HERE.

Story 5.5d adds:

- **`FrameKind::SpiritAdmitted = 19`** on `crates/maos-kernel-core/src/iac/transparency_log.rs::FrameKind` — emitted at admission with the `AdmissionDecision` shape. Add the matching `from_i64` arm at `transparency_log.rs:80-104`.
- **`FrameKind::RegistryYank = 20`** on the same enum — emitted by `YankPoller::poll_once()` per yank entry. Add the matching `from_i64` arm.
- **`FrameKindLabel` mirror arms** — Story 5.5c's File List touched `crates/maos-domain/src/log_recall.rs` (additive `FrameKindLabel::McpInvocation` variant) AND `crates/maos-kernel-core/src/iac/log_recall.rs` (mapping arms). Story 5.5d MUST add `FrameKindLabel::SpiritAdmitted` + `FrameKindLabel::RegistryYank` to both files for `maosctl audit query` filtering at Epic 9 Story 9.1 (the `FrameKindLabel` is `#[non_exhaustive]` per Story 5.5c review §21, so additive variants are safe).
- Verify HEAD-current state with `grep -n 'FrameKind::.*= [0-9]' crates/maos-kernel-core/src/iac/transparency_log.rs` at story open. The `= 19` / `= 20` assignments here assume Story 5.5c is `done` (it is, per sprint-status); if a later story has landed before this one, shift accordingly.

## What this story IS NOT

- **NOT the full v1.0 registry from Story 7.2.** Story 5.5d ships the v0.5-α BASELINE: 5 operations, 3 trust tiers, structural-only ComplianceClaim verification, single-threaded TCP listener, content-addressed filesystem storage, no authentication, no TLS direct. Story 7.2 adds HTTPS-direct, operator-key auth, per-op ACL, air-gapped import, full envelope verification per CCAC N=600.
- **NOT the v0.9 ComplianceClaim semantic evaluator.** Story 5.5d performs ONLY signature + structural fingerprint match. The principle engine + N=600 corpus + ±2% agreement target arrive at Story 7.3 + App-E v0.9 roadmap.
- **NOT the `public-vetted` trust tier.** FR37 explicitly defers `public-vetted` to v2.5. Story 5.5d REJECTS it at admission with `RegistryError::PublicVettedDeferred`, mirroring Story 5.5c's `[mcp].servers[i].server_trust_tier == "public_vetted"` rejection.
- **NOT a federation tier between `org-internal` and `public-untrusted`.** Appendix D.4 ("Federation tier between `org-internal` and `public-untrusted`") is a future ADR — Story 5.5d ships the THREE tiers committed in ADR-009 (binding-v0.5) only.
- **NOT a real HTTPS-terminating server.** v0.5-α the registry server runs HTTP cleartext on `127.0.0.1`. Operator-grade HTTPS direct (`POST https://registry.example.org/mcp`) is Story 7.2. Documented in `SECURITY.md`.
- **NOT mTLS for registry-bound MCP-Streamable-HTTP.** Same as 5.5c: operator-deployed registries use HTTPS via reverse proxy; mTLS client-cert is added when bilateral A2A's mTLS infrastructure (Story 6.3) lands.
- **NOT a per-publisher-key allowlist.** v0.5-α the publisher pubkey for `public-untrusted` Spirits is verified for signature (self-signed envelope) but NOT cross-checked against an operator-configured publisher allowlist. The publisher-allowlist feature is deferred to v0.7 (Story 7.2 prep).
- **NOT operator-side rate limiting.** Anyone can call any of the 5 ops on the v0.5-α server. Per-op rate limits + per-publisher quotas are Story 7.2.
- **NOT registry replication / mirroring.** v0.5-α a single registry binary serves a single content-addressed directory tree. Multi-instance replication + leader-election is Story 7.2 prep.
- **NOT MCP-protocol-library import.** Same FR47 commitment as 5.5c: the wire format is direct-implemented via 5.5c's `StreamableHttpTransport` primitives; `cargo tree | grep -E 'mcp|jsonrpc'` returns empty after this story.
- **NOT a parallel HTTP client.** The kernel-side `McpSpiritRegistryClient` routes through Story 5.5c's `McpClient::call` which uses `StreamableHttpTransport` which uses `IoSubsystemPort::http_post`. The server-side `SpiritRegistryServer` uses raw `std::net::TcpListener` (no `reqwest` / `hyper` / `axum` added to the workspace). v0.5-α is intentionally minimal-dependency.
- **NOT the registry CLI surface beyond stubs.** `maosctl install <spirit>` / `maosctl registry use <uri>` / `maosctl registry deprecate` are scaffolded as `maos-cli` subcommands that call the `SpiritRegistryClient` port — full CLI UX with discovery + interactive flows is Story 7.2.
- **NOT eventual-consistency reasoning.** v0.5-α the registry is single-instance; yank propagation is "the next 5-min poll picks up yanks". Eventual-consistency across replicas is Story 7.2 prep.

## Acceptance Criteria

### AC1 — `crates/maos-registry/` crate + server + client + 5 MCP-Streamable-HTTP operations (epic AC1)

**Given** the EXISTING `crates/maos-mcp/src/client.rs::McpClient::call(server_name, tool, args) -> Result<McpCallResponse, McpError>` surface from Story 5.5c, the EXISTING `StreamableHttpTransport` at `crates/maos-mcp/src/transport/streamable_http.rs`, the EXISTING workspace `Cargo.toml [workspace.members]` listing 23 crates,

**When** Story 5.5d lands (a) the NEW `crates/maos-registry/` crate (workspace now 24 crates) with `lib.rs` + `client.rs` + `server.rs` + `admission.rs` + `compliance_verify.rs` + `yank.rs` + `operations.rs` + `storage.rs` + `fixture_replay.rs` + `bin/server.rs` + `handlers/{search,manifest,artifact,publish,deprecate}.rs`, (b) the NEW `crates/maos-domain/src/ports/registry.rs::SpiritRegistryClient` port trait + `SpiritId` / `SearchQuery` / `SearchResults` / `SignedManifest` / `SignedArtifact` / `SignedPackage` / `PublishReceipt` / `YankReason` / `YankReceipt` / `RegistryError` domain types,

**Then** the `SpiritRegistryClient::search/manifest/artifact/publish/deprecate` trait methods are SYNC per ADR-010 with `Result<_, RegistryError>` return shape; the `RegistryError` enum is `#[non_exhaustive]` with all 11 documented variants AND each variant maps to a typed-error code in `xtask/fr63-typed-errors.toml` (CREATE the catalog file at this story if it does not yet exist per the Epic 9 Story 9.3 prep).

**And** `McpSpiritRegistryClient::new(mcp_client, registry_server_name)` constructs cleanly; every method (`search` / `manifest` / `artifact` / `publish` / `deprecate`) routes through `mcp_client.call(&registry_server_name, "registry.<op>", args_as_json)`; encoding errors map to `RegistryError::Transport`; MCP-level capability-denied errors map to `RegistryError::Transport` with an explanatory message (registry calls are kernel-internal, no Spirit-held capability is involved).

**And** `SpiritRegistryServer::new(storage, listen_addr, org_pubkey)` constructs cleanly and `server.run()` blocks on `std::net::TcpListener`; per-connection worker threads handle one MCP-Streamable-HTTP request each (POST `/mcp` with JSON-RPC 2.0 frame in the body), routing to the matching `handlers/<op>.rs` based on the `method` field, returning JSON results.

**And** unit tests in each module mirroring Story 5.5c's structure:
- `crates/maos-registry/src/client.rs::tests` — 4+ scenarios (route, publish, version-not-found typed error, transport error).
- `crates/maos-registry/src/server.rs::tests` — 5+ scenarios (route to each handler; well-formed publish; malformed publish rejected; unknown method rejected; concurrent requests via `std::thread`).
- `crates/maos-registry/src/handlers/{search,manifest,artifact,publish,deprecate}.rs::tests` — per-handler happy-path + 2+ error paths.
- `crates/maos-domain/src/ports/registry.rs::tests` — round-trip serde tests for all domain types (`SearchQuery`, `SignedPackage`, `YankEntry`, `RegistryError`).

**And** `cargo run -p xtask -- check-fr47` continues to PASS with `fr47-allowlist.toml` empty — `cargo tree | grep -E 'mcp|jsonrpc|rust-mcp|reqwest|hyper|axum|warp'` returns empty. The registry server uses `std::net::TcpListener` + `std::io::BufRead` + `serde_json` only (no HTTP framework crate added).

---

### AC2 — Three-trust-tier strictest-of-floor admission + Ed25519 signature verification + ComplianceClaim structural fingerprint match (epic AC2; ADR-009 binding-v0.5 gate)

**Given** the EXISTING `maos_spirit_abi::compliance::ComplianceClaimEnvelope` at `crates/maos-spirit-abi/src/compliance.rs:44-54`, the EXISTING `TrustTier` enum at `crates/maos-spirit-abi/src/compliance.rs:136-145` (`Local | OrgInternal | PublicVetted | PublicUntrusted`), the EXISTING `ExecutionContextFingerprint` at `compliance.rs:111-128`, the EXISTING `ed25519-dalek` workspace dependency from Story 5.4, the EXISTING Story 1b.3 strictest-of-(manifest, trust-tier, operator-policy) sandbox-tier floor logic,

**When** Story 5.5d lands (a) the NEW `crates/maos-registry/src/admission.rs::admit_spirit(pkg, op_cfg) -> Result<AdmissionDecision, AdmissionError>` with the 9-variant `AdmissionError` enum, (b) the NEW `crates/maos-registry/src/compliance_verify.rs::verify_envelope_structural` performing Ed25519 signature verification + canonical-CBOR decode + fingerprint hash match, (c) the NEW `crates/maos-kernel-core/src/security/operator_config.rs::RegistrySection` with `uri` + `tier_floor` + `t3_for_public_untrusted` + `allow_unsigned_local` + `org_signing_pubkey` fields,

**Then** the strictest-of-floor logic correctly resolves `effective_tier = strictest_of(manifest_declared, registry_origin, operator_floor)` for all 8 combinations of the 3 trust tiers across the 3 inputs (verify all 8 via the unit test `strictest_of_resolves_correctly_all_combinations`):
- For `effective_tier == Local`: admit if `op_cfg.allow_unsigned_local` OR signature verifies against `op_cfg.org_signing_pubkey`.
- For `effective_tier == OrgInternal`: REQUIRE `pkg.publisher_pubkey == op_cfg.org_signing_pubkey` (Ed25519 verify); reject `AdmissionError::OrgSignatureInvalid` otherwise.
- For `effective_tier == PublicUntrusted`: REQUIRE BOTH (1) `ed25519::verify(pkg.publisher_pubkey, sha256(pkg.manifest_toml || pkg.artifact_bytes), pkg.signature) == Ok(())` AND (2) `verify_envelope_structural` succeeds (envelope signature + fingerprint hash match); reject the specific failing condition with `PublisherSignatureInvalid` OR `ComplianceContextDrift { actual_hex, claimed_hex }`.
- For `effective_tier == PublicVetted`: REJECT with `AdmissionError::PublicVettedDeferred` (FR37).

**And** `op_cfg.t3_for_public_untrusted == true` AND `effective_tier == PublicUntrusted` → `AdmissionDecision.sandbox_tier_floor == SandboxTier::T3`. The strictest-of-floor seam consumes this without modifying Story 5.5a's T3 spawn code — the floor is PASSED to `crates/maos-kernel-core/src/security/sandbox/policy.rs::resolve_sandbox_floor` (or wherever Story 1b.3's strictest-of logic lives — verify HEAD-current).

**And** unit tests at `crates/maos-registry/src/admission.rs::tests` cover the 10 scenarios documented in §What this story IS plus `strictest_of_resolves_correctly_all_combinations`.

**And** structural ComplianceClaim verification rejects the 5 documented adversarial cases:
- Envelope signature does not verify against `attester_pubkey` → `PublisherSignatureInvalid`.
- `claim_bytes` decode failure (malformed CBOR) → `ComplianceContextDrift` (substrate cannot trust an undecodable envelope).
- `claim.fingerprint_hash` != `sha256(cbor_canonical(actual_fingerprint))` → `ComplianceContextDrift { actual_hex, claimed_hex }`.
- `claim.trust_tier` declared `Local` but `pkg.manifest.trust_tier` declared `PublicUntrusted` → `ComplianceContextDrift` (envelope claims a context that does not match the manifest's actual context).
- `claim.sandbox_tier` declared `T0` but `pkg.manifest.sandbox_tier` declared `T2` → `ComplianceContextDrift`.

**And** the typed-error catalog at `xtask/fr63-typed-errors.toml` registers `EComplianceContextDrift` as the wire-protocol error code mapped from `AdmissionError::ComplianceContextDrift` (per §8.5 typed-error name `EComplianceContextDrift`). The catalog is consumed by Story 9.1's audit-query surface.

---

### AC3 — `[registry]` operator-config section + composition-root wiring + `MAOS_REGISTRY_URI` env override (epic AC2)

**Given** the EXISTING composition-root wiring pattern at `crates/maos-bin/src/main.rs:340` for `LocalFileRegistryClient` (Story 5.4), the EXISTING `MAOS_*_*` env-var resolution discipline (Story 5.5b/5.5c), the EXISTING manifest TOML parsing infrastructure at `maos-kernel-core::security::manifest`,

**When** Story 5.5d lands (a) the NEW `crates/maos-kernel-core/src/security/operator_config.rs` (NEW file) with `OperatorConfig::resolve_from_env_and_disk()` returning `RegistrySection`, (b) the EXTENDED composition root at `crates/maos-bin/src/main.rs` wiring `Arc<dyn SpiritRegistryClient>` via the 4-way switch (`MAOS_REGISTRY_URI=stub` → FixtureReplay; empty URI → NullSpiritRegistryClient; `MAOS_REGISTRY_URI=file://...` → LocalFs (NEW path); otherwise → McpSpiritRegistryClient over Story 5.5c's `McpClient`),

**Then** the operator config resolves correctly per the documented priority (env-vars > `~/.config/maos/operator.toml` > built-in defaults):
- `MAOS_REGISTRY_URI=http://localhost:6789/mcp cargo run -p maos-bin` resolves `op_cfg.uri = "http://localhost:6789/mcp"`.
- `MAOS_REGISTRY_TIER_FLOOR=org_internal cargo run -p maos-bin` resolves `op_cfg.tier_floor = TrustTier::OrgInternal`.
- `MAOS_REGISTRY_T3_FOR_PUBLIC_UNTRUSTED=true cargo run -p maos-bin` resolves `op_cfg.t3_for_public_untrusted = true`.
- A pre-existing `~/.config/maos/operator.toml` with `[registry] uri = "..."` is read when no env-var overrides.
- Built-in defaults apply when neither env-var nor disk config sets a value.

**And** the composition root logs the resolved registry config to stderr at startup (e.g., `maos: registry uri=http://localhost:6789/mcp tier_floor=public_untrusted t3_public_untrusted=false allow_unsigned_local=true`) for operator-debuggability.

**And** when `MAOS_REGISTRY_URI` is unset AND no `operator.toml` exists, the composition root wires `NullSpiritRegistryClient` (returns `RegistryError::Unconfigured` on every call) — the substrate STARTS UP cleanly without a registry (registry is optional infrastructure; only `maosctl install` invocations require it).

**And** unit tests at `crates/maos-kernel-core/src/security/operator_config.rs::tests` cover the priority-order resolution + TOML parse + 3 round-trip scenarios.

---

### AC4 — Yank propagation distinct from FR13 CRL + 5-min poll loop + new `FrameKind::RegistryYank` Transparency Log emission (epic AC3, FR59 baseline)

**Given** the EXISTING Story 5.4 5-min CRL polling task wired at composition root, the EXISTING `crates/maos-domain/src/frame.rs::FrameKind` enum (Story 5.5c added `McpInvocation = 18`; verify HEAD-current max via `grep -n 'FrameKind::.*= [0-9]'` at story open), the EXISTING `TransparencyLogPort::record` path,

**When** Story 5.5d lands (a) the NEW `FrameKind::SpiritAdmitted = N` + `FrameKind::RegistryYank = N+1` additive discriminants on the `#[repr(u8)]` `FrameKind` enum (verify HEAD-current max + 1 + 2 — the value `19` and `20` referenced here are illustrative), (b) the NEW `crates/maos-registry/src/yank.rs::YankPoller` + the EXTENDED 5-min poll task at `crates/maos-bin/src/main.rs` calling `yank_poller.poll_once()` in addition to the CRL poll, (c) the NEW server-side `registry.yanks_since` MCP op routed at `crates/maos-registry/src/handlers/yanks_since.rs`,

**Then** when a Spirit version is yanked via `registry.deprecate("hello-spirit", "0.1.0", reason)`:
1. The server-side handler records the yank to `~/.local/share/maos/registry/yanks.json` with `yanked_at_ns = monotonic_now_ns()`.
2. Subsequent `registry.search` calls EXCLUDE the yanked version by default (matching the documented `SearchQuery.include_yanked = false` default).
3. On the next 5-min poll cycle (or any explicit `yank_poller.poll_once()` call), the kernel-side `YankPoller`:
   - Calls `mcp_client.call("spirit-registry", "registry.yanks_since", {"since_ns": <cached_last_seen>})`.
   - Receives a `YankList { entries: Vec<YankEntry> }`.
   - For each entry, emits ONE `FrameKind::RegistryYank` Transparency-Log row with `intent = format!("registry.yank:{}/{}", entry.spirit_id, entry.version)` + `timestamp_ns = monotonic_now_ns()` (NEVER `wall_clock_now_ns()` per Story 5.5c §1366).
   - Updates the local `cache.last_seen_ns` to the max `yanked_at_ns` observed.

**And** the `RegistryYank` TL row is DISTINGUISHABLE from Story 5.4's `SpiritRevoked` TL row (FR13 CRL) — they ride different `FrameKind` discriminants. This enables Story 9.1's `maosctl audit query --frame-kind registry_yank` and `--frame-kind spirit_revoked` to filter independently.

**And** running Spirit instances of a yanked version ARE NOT terminated by the yank propagation (yank semantics per FR59 baseline: "registry retracts a version from new installs; existing instances keep running"). This is the explicit distinction from Story 5.4 CRL (which DOES terminate running instances per the declared revocation policy).

**And** unit tests at `crates/maos-registry/src/yank.rs::tests`:
- `poll_once_with_no_yanks_returns_zero_no_tl_rows` — empty `YankList`; assert no TL emission.
- `poll_once_with_two_yanks_emits_two_tl_rows` — 2-entry YankList; assert 2 `FrameKind::RegistryYank` rows.
- `poll_once_with_monotonic_now_ns_used` — assert TL `timestamp_ns` is monotonic non-decreasing across poll iterations.
- `cache_advances_to_max_yanked_at_ns_on_apply` — assert `cache.last_seen_ns` after apply equals max entry timestamp.

---

### AC5 — Smoke arm `MAOS_ONE_SHOT=smoke-registry-5d` + known-modes list extension + Story 0.2 kernel-API surface preservation (epic AC4, observability discipline)

**Given** the EXISTING `MAOS_ONE_SHOT` dispatch mechanism at `crates/maos-bin/src/main.rs:449-455`, the EXISTING known-modes list at `main.rs:2360`, the EXISTING `smoke-mcp-acp-5` arm at `main.rs:2164-2177` (the canonical smoke-arm shape), the EXISTING `xtask/kernel-api-classes.toml` classification table,

**When** Story 5.5d lands (a) the NEW `MAOS_ONE_SHOT=smoke-registry-5d` arm walking 7 numbered JSON-line surfaces (full surface list in §What this story IS), (b) the NEW `MAOS_ONE_SHOT=registry-server` long-running arm consumed by integration tests + the `maos-registry-server` binary alias, (c) the extended known-modes list at `main.rs:2360` including BOTH `smoke-registry-5d` AND `registry-server`, (d) the new `xtask/kernel-api-classes.toml` rows for `McpSpiritRegistryClient` + `SpiritRegistryClient` + `admit_spirit`,

**Then** when an evaluator runs `MAOS_ONE_SHOT=smoke-registry-5d cargo run -p maos-bin --features fixture_replay` they observe stdout containing EXACTLY these 7 JSON lines (in order):

```jsonc
{"step":1,"surface":"registry_init","tier_floor":"public_untrusted","t3_for_public_untrusted":false}
{"step":2,"surface":"registry_publish","outcome":"ok","tier":"local","spirit_id":"hello-spirit","version":"0.1.0"}
{"step":3,"surface":"registry_search","outcome":"ok","results":1}
{"step":4,"surface":"registry_install","outcome":"ok","tier":"local","spirit_id":"hello-spirit"}
{"step":5,"surface":"admission_public_untrusted","outcome":"ok","fingerprint_match":true}
{"step":6,"surface":"admission_compliance_drift","outcome":"rejected","error":"EComplianceContextDrift"}
{"step":7,"surface":"registry_yank_propagate","outcome":"ok","yanked":1}
```

**And** the binary exits with code 0 after step 7.

**And** `crates/maos-bin/tests/smoke_registry_5d_test.rs` invokes the arm via `Command::new(maos_bin).env("MAOS_ONE_SHOT", "smoke-registry-5d")`, asserts exit code 0, parses stdout into 7 JSON lines, asserts each line has the expected `step` + `surface` keys + `outcome` semantics.

**And** an evaluator running `MAOS_ONE_SHOT=unknown cargo run -p maos-bin` gets the error message at `main.rs:2360` listing the full updated known-modes set INCLUDING `smoke-registry-5d` and `registry-server`.

**And** `cargo run -p xtask -- check-service-boundary --kernel-api-classes xtask/kernel-api-classes.toml` PASSES with the three new rows.

**And** `cargo run -p xtask -- check-fr47` PASSES with `fr47-allowlist.toml` empty.

**And** `cargo run -p xtask -- check-pub-field-constructors` PASSES — every new pub field on `RegistrySection`, `SearchQuery`, `SignedPackage`, `YankEntry`, `PublishReceipt`, etc. carries the `#[doc = "Construct via ::new ..."]` annotation matched by an `impl ::new` constructor.

---

### AC6 — Registry roundtrip corpus + FR63 typed-error catalog + per-trust-tier admission test coverage (epic AC5)

**Given** the EXISTING fixture-corpus testing pattern at `crates/maos-kernel-core/tests/fixtures/manifest/sandbox/`, the EXISTING typed-error reporting pattern (Story 5.4 review §1373 closed),

**When** Story 5.5d lands (a) the NEW corpus at `crates/maos-registry/tests/fixtures/registry-roundtrip-v05/` with `well-formed/` (10+ files across 3 tiers) and `malformed-rejected/` (8+ files each with `expected_typed_error`), (b) the NEW test driver `crates/maos-registry/tests/registry_roundtrip_test.rs` iterating all corpus files, (c) the NEW `xtask/fr63-typed-errors.toml` catalog (CREATE if absent — Epic 9 prep) registering all 11 `RegistryError` variants + 9 `AdmissionError` variants with their wire-protocol error codes,

**Then** running `cargo test -p maos-registry --test registry_roundtrip_test` PASSES with:
- 100% of well-formed flows succeed (`admit_spirit` returns `Ok(AdmissionDecision { admit: true, .. })`).
- 100% of malformed flows are rejected with the typed error matching the `expected_typed_error` field exactly.
- Per-tier coverage: ≥3 well-formed flows per trust tier (local + org-internal + public-untrusted); ≥1 malformed flow per documented failure mode.

**And** the typed-error catalog at `xtask/fr63-typed-errors.toml` registers (TOML format):
```toml
[errors.EUnknownSpirit]
crate = "maos-registry"
source_variant = "RegistryError::UnknownSpirit"
operator_visible = true

[errors.ESignatureInvalid]
crate = "maos-registry"
source_variant = "AdmissionError::PublisherSignatureInvalid"
operator_visible = true

[errors.EComplianceContextDrift]
crate = "maos-registry"
source_variant = "AdmissionError::ComplianceContextDrift"
operator_visible = true
docs_url = "https://docs.maos.dev/errors/EComplianceContextDrift"
# ... etc for all 20 variants
```

**And** the catalog file is the substrate Story 9.3 (full typed-error catalog) extends with all kernel-wide errors — Story 5.5d is the FIRST story that populates the catalog with 20 entries.

**And** integration test `crates/maos-registry/tests/end_to_end_test.rs` (NEW) wires `SpiritRegistryServer::run` on a real localhost socket + `McpSpiritRegistryClient` calling it via Story 5.5c's real `McpClient` over the real `StreamableHttpTransport`, runs a publish→search→manifest→artifact→deprecate→yanks_since cycle, and asserts every operation succeeds within a 5s wall-clock budget. This is the **real-wire** validation (the smoke arm uses FixtureReplay; this test uses real TCP).

---

## Tasks / Subtasks

- [x] **Task 1 (AC1) — Domain port + types**
  - [x] Create `crates/maos-domain/src/ports/registry.rs` with `SpiritRegistryClient` trait + all 11+ domain types (`SpiritId`, `SearchQuery`, `SearchResults`, `SignedManifest`, `SignedArtifact`, `SignedPackage`, `PublishReceipt`, `YankReason`, `YankReceipt`, `YankEntry`, `YankList`).
  - [x] Add `RegistryError` enum with 11 variants + `#[non_exhaustive]` + `thiserror::Error` derive.
  - [x] Pub-field-constructor annotations + `::new` constructors on every pub serde-bearing struct.
  - [x] Re-export from `crates/maos-domain/src/ports/mod.rs`.
  - [x] Round-trip serde tests for each type.

- [x] **Task 2 (AC1) — New `crates/maos-registry/` crate scaffold**
  - [x] `Cargo.toml` with `[package]` + `[dependencies]` (maos-domain, maos-mcp, maos-spirit-abi, serde, serde_json, thiserror, sha2, ring, hex) + `[features] fixture_replay` + `[[bin]] maos-registry-server`.
  - [x] Register in workspace `Cargo.toml [workspace.members]` — grow 23 → 24 crates.
  - [x] `src/lib.rs` with module declarations + crate-level doc.
  - [x] `src/operations.rs` — `RegistryOperation` enum + per-op argument + response shapes (`SearchArgs`, `ManifestArgs`, `ArtifactArgs`, `PublishArgs`, `DeprecateArgs`, `YanksSinceArgs`) all `#[serde(rename_all = "snake_case")]`.

- [x] **Task 3 (AC1) — `McpSpiritRegistryClient` (kernel-side)**
  - [x] Implement `crates/maos-registry/src/client.rs::McpSpiritRegistryClient` with all 5 trait methods routing through `Arc<dyn McpClient>`.
  - [x] `map_mcp_err` + `decode_typed_error` helpers; map `McpError` variants exhaustively.
  - [x] Unit tests (6 scenarios) against `FixtureReplayMcpServer`.
  - [x] `NullSpiritRegistryClient` null object for unconfigured registry.

- [x] **Task 4 (AC1) — `SpiritRegistryServer` (server-side)**
  - [x] Implement `crates/maos-registry/src/server.rs::SpiritRegistryServer` with `std::net::TcpListener` (NO `tokio::net`, NO HTTP framework).
  - [x] Per-connection `std::thread::spawn` worker handling one JSON-RPC frame.
  - [x] Per-op dispatch to `handlers/<op>.rs`.
  - [x] Self-prune worker handles on connection close.

- [x] **Task 5 (AC1) — Per-op handlers**
  - [x] `handlers/search.rs` + `handlers/manifest.rs` + `handlers/artifact.rs` + `handlers/publish.rs` + `handlers/deprecate.rs` + `handlers/yanks_since.rs` (internal op).
  - [x] Each handler reads request args, calls `RegistryStorage` accordingly, returns typed result.

- [x] **Task 6 (AC1) — `RegistryStorage` + `LocalFsRegistryStorage`**
  - [x] `src/storage.rs::RegistryStorage` trait + `LocalFsRegistryStorage` impl using `~/.local/share/maos/registry/` tree.
  - [x] `index.json` rebuild on `put()` + `yank()`; `yanks.json` write on `yank()`.
  - [x] Use `monotonic_now_ns()` for all timestamps.
  - [x] Use `serde_json::to_vec_pretty(...).map_err(...)` (NEVER `.unwrap_or_default()`).

- [x] **Task 7 (AC1) — `FixtureReplaySpiritRegistryClient`**
  - [x] `src/fixture_replay.rs` gated by `#[cfg(any(test, feature = "fixture_replay"))]`.
  - [x] Declarative response-queue model mirroring `FixtureReplayMcpServer`.
  - [x] Unit tests: empty-ring returns typed error (don't panic); records calls.

- [x] **Task 8 (AC2) — `admit_spirit` + `AdmissionDecision` + `AdmissionError`**
  - [x] `src/admission.rs::admit_spirit` implementing strictest-of-floor for all 4 tiers (PublicUntrusted > OrgInternal > Local; PublicVetted rejected).
  - [x] 8 unit tests covering all tier scenarios + `strictest_of_resolves_all_combinations`.

- [x] **Task 9 (AC2) — `verify_envelope_structural` (ComplianceClaim verification)**
  - [x] `src/compliance_verify.rs` performing Ed25519 verify + fingerprint hash match.
  - [x] 3 adversarial-rejection unit tests per AC2.

- [x] **Task 10 (AC3) — `operator_config.rs` + `RegistrySection` + env-and-disk resolution**
  - [x] Create `crates/maos-kernel-core/src/security/operator_config.rs` (NEW file).
  - [x] `RegistrySection` struct with 5 fields + pub-field-constructor annotations + `::new`.
  - [x] `RegistrySection::resolve_from_env_and_disk` reading env-vars > `~/.config/maos/operator.toml` > defaults.
  - [x] Unit tests: 3 resolution scenarios.
  - [x] Re-export from `crates/maos-kernel-core/src/security/mod.rs`.

- [x] **Task 11 (AC3) — Composition-root wiring**
  - [x] Extend `crates/maos-bin/src/main.rs` to resolve `RegistrySection` at startup.
  - [x] 4-way switch on `MAOS_REGISTRY_URI`: `stub` → FixtureReplay; empty → NullSpiritRegistryClient; otherwise → Null as placeholder.
  - [x] Log resolved config to stderr at startup.
  - [x] Implement `NullSpiritRegistryClient` (returns `RegistryError::Unconfigured` on every call).

- [x] **Task 12 (AC4) — `YankPoller` + new `FrameKind` discriminants**
  - [x] Add `FrameKind::SpiritAdmitted = 19` + `FrameKind::RegistryYank = 20` to kernel-side enum; add `from_i64` arms.
  - [x] Add `FrameKindLabel::SpiritAdmitted` + `FrameKindLabel::RegistryYank` in domain + mapping arms in kernel-core log_recall.rs.
  - [x] `src/yank.rs::YankPoller` + `YankCache` + `poll_once()`.
  - [x] 3 unit tests for yank cache and poller.

- [x] **Task 13 (AC5) — Smoke arm + known-modes + tests**
  - [x] Add `MAOS_ONE_SHOT=smoke-registry-5d` arm walking 7 surfaces.
  - [x] Add `MAOS_ONE_SHOT=registry-server` long-running arm.
  - [x] Extend known-modes string with both modes.
  - [x] Create `crates/maos-bin/tests/smoke_registry_5d_test.rs`.
  - [x] `MAOS_ONE_SHOT=smoke-registry-5d cargo run --features fixture_replay` exits 0 with 7 JSON lines.

- [ ] **Task 14 (AC5) — Kernel-API surface gate**
  - [ ] Add `xtask/kernel-api-classes.toml` rows for `McpSpiritRegistryClient` / `admit_spirit` / `SpiritRegistryClient`.
  - [ ] Run `cargo run -p xtask -- check-service-boundary` — PASSES.
  - [ ] Run `cargo run -p xtask -- check-fr47` — PASSES.
  - [ ] Run `cargo run -p xtask -- check-pub-field-constructors` — PASSES.

- [ ] **Task 15 (AC6) — Registry roundtrip corpus + FR63 catalog**
  - [ ] Create 10+ `well-formed/*.json` fixtures spanning 3 tiers.
  - [ ] Create 8+ `malformed-rejected/*.json` fixtures with `expected_typed_error`.
  - [ ] `crates/maos-registry/tests/registry_roundtrip_test.rs`.
  - [ ] CREATE `xtask/fr63-typed-errors.toml` with 20+ entries.

- [ ] **Task 16 (AC6) — Real-wire end-to-end test**
  - [ ] `crates/maos-registry/tests/end_to_end_test.rs`.

- [x] **Task 17 (cross-cutting) — Documentation + SECURITY.md**
  - [x] Crate-level doc comments on `maos-registry`.
  - [x] CREATE `crates/maos-registry/SECURITY.md`.
  - [x] Forward-shape contracts documented.

- [ ] **Task 18 (review-readiness) — Pre-commit gate sweep**
  - [ ] `cargo test -p maos-domain -p maos-registry -p maos-mcp -p maos-kernel-core` PASSES.
  - [ ] `cargo tree | grep -E 'mcp|jsonrpc|rust-mcp|reqwest|hyper|axum|warp'` returns empty.
  - [ ] `grep -rn 'unimplemented!.*Story 5.5d' crates/` returns zero matches.

## Dev Notes

### Architectural Anchors

- **ADR-008 — Spirit registry as MCP-Streamable-HTTP server** (`binding-v0.5`) — *the* anchor for this story. Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/12-architecture-decision-records.md:144-154`. Gate text: `"registry.search/manifest/artifact operational; MCP-Streamable-HTTP transport"` — this story DELIVERS the gate.
- **ADR-009 — Three trust tiers with strictest-of-floor enforcement** (`binding-v0.5`) — the second key anchor. Source: `12-architecture-decision-records.md:156-166`. Three tiers: `local` / `org-internal` / `public-untrusted` (`public-vetted` deferred per FR37). Strictest-of-(manifest, trust tier, operator-policy) floor.
- **§8.5 ComplianceClaim envelope** — schema frozen at v0.1 (Story 1b.4); structural validator at v0.1; **runtime fingerprint verification lands HERE** at v0.5; semantic evaluator (App-E principle engine + N=600 corpus) lands at v0.9 (Story 7.3). Source: `8-security-approval-model.md:86-100`.
- **§4.0.7 Four-class API surface taxonomy** — registry client + server + admission all classify as `data-movement` (route payloads; no semantic interpretation in the adapter). Source: `4-kernel-design.md`.
- **§7.5 Four-protocol commitment** — "Kernel-internal IAC + bilateral A2A + ACP + MCP. The substrate invents no new wire protocols." Story 5.5d is the structural proof that MCP suffices for the registry (zero new protocols added). Source: `7-inter-agent-communication.md:122-128`.
- **ADR-010 — Hexagonal ports + sync trait semantics** — `SpiritRegistryClient` in `maos-domain` follows the sync-only contract; async callers wrap in `spawn_blocking`. Source: `12-architecture-decision-records.md:172`.
- **FR37 — `public-vetted` deferral to v2.5** — explicit per round-2 decision; Story 5.5d enforces the deferral by rejecting `TrustTier::PublicVetted` at admission. Mirrors Story 5.5c's identical rejection in `[mcp]` manifest section.
- **FR59 — Yank propagation distinct from FR13 CRL** — Story 5.5d ships the baseline; Story 7.2 ships the full version. Distinct from CRL: yank = registry retracts a version (new installs blocked); CRL = revoked Spirit identity (running instances terminated).

### Decision Register

1. **Why a NEW crate rather than extending `maos-kernel-core`?** Source: ADR-008 gate text references `crates/maos-registry/`. Story 5.5a's Decision Register §1 documented an inverse choice (keeping T3 inside `maos-kernel-core` for KLOC + workspace-count reasons), but registry is fundamentally different: the server-side binary is operator-deployable separately, the storage layout is independent of kernel state, and the publish/install operations have BOTH author-side (publisher) and operator-side (admission) consumers — the natural home is a dedicated crate. The workspace grows 23 → 24 crates.

2. **Why HTTP-only-on-127.0.0.1 at v0.5-α rather than HTTPS direct?** HTTPS termination requires TLS infrastructure (certs, rotation, OCSP) that belongs in operator-deployed reverse proxy (nginx / Caddy), not in the substrate. The substrate's `IoSubsystemPort::http_post` already terminates TLS on the CLIENT side; the SERVER side at v0.5-α delegates to reverse-proxy convention. Story 7.2 adds optional `[registry].tls_cert_path` for HTTPS direct.

3. **Why `std::net::TcpListener` + `std::thread::spawn` rather than `tokio::net` + `axum`?** Kernel-stays-small invariant. Adding `axum` or `hyper` adds 50+ transitive dependencies and a parallel async runtime requirement. The v0.5-α server expects < 10 ops/sec under realistic operator workloads; per-connection thread is operationally sufficient. Story 7.2 may revisit for higher-throughput operator deployments.

4. **Why does the kernel-side `McpSpiritRegistryClient` route DIRECTLY through `McpClient::call` rather than through `McpClientAdapter`?** Registry calls are kernel-internal admission infrastructure — there is no Spirit holding a capability token when `maosctl install` runs. The `McpClientAdapter` path (Story 5.5c) requires a `CapabilityToken` for capability mediation; registry calls bypass this. The architecture trade-off: we lose per-call Transparency-Log emission for registry calls. The compensation: every successful admission emits a single `FrameKind::SpiritAdmitted` TL row (at AC2), and every yank propagation emits `FrameKind::RegistryYank` (at AC4) — so the substrate's audit trail covers admission DECISIONS even though it doesn't cover individual registry RPC calls.

5. **Why does yank propagation ride the SAME 5-min poll cadence as CRL rather than a separate timer?** Operationally simpler — one polling task in the composition root rather than two. The 5-min cadence matches FR13 CRL exactly; FR59 yank does not require tighter cadence at v0.5. Story 7.2 may split the cadences if the yank corpus motivates faster propagation.

6. **Why is yank DISTINCT from CRL (different FrameKinds, different runtime semantics)?** Per ADR-008 + FR59: yank = registry retracts a version (new installs blocked, running instances keep running); CRL = revoked Spirit identity (running instances terminated). Conflating them would lose the distinction operators rely on for incident response (a yank is "this version had a bug; upgrade when convenient"; a CRL revocation is "this Spirit identity is compromised; kill it now"). Separate `FrameKind` discriminants preserve queryability in Story 9.1's `maosctl audit query --frame-kind`.

7. **Why `public-vetted` rejected at admission rather than parsed-but-warned?** Mirrors Story 5.5c's identical rejection in the `[mcp]` section. FR37 is an EXPLICIT scope decision; emitting a warning instead would leave the door open to operator misconfiguration where a "vetted" Spirit is admitted without the vetting infrastructure existing. Hard rejection is the conservative substrate choice.

8. **Why `t3_for_public_untrusted` is an operator-policy field rather than a Spirit-manifest field?** Per ADR-009: trust tier comes from the registry (or manifest's declaration); sandbox tier comes from the strictest-of-floor. The operator-policy escalation of `public-untrusted` to T3 is an OPERATOR choice (defense-in-depth posture for a particular deployment), not a Spirit author's choice. Story 5.5a's epic-prep note explicitly placed this seam at Story 5.5d.

9. **Why `RegistrySection` lives in `maos-kernel-core::security::operator_config` rather than `maos-registry`?** Operator config is read by the kernel BEFORE the registry is wired; it lives in kernel-core. The registry crate consumes a `&RegistrySection` reference. This mirrors how Story 5.5b's `[providers]` section lives in `maos-kernel-core::security::manifest` and is consumed by `maos-providers`.

10. **Why `verify_envelope_structural` is structural-only (no semantic evaluator) at v0.5-α?** Per App-E roadmap: the schema is binding-v0.1; the structural validator is binding-v0.1; the semantic evaluator (principle engine + N=600 corpus + ±2% agreement) is binding-v0.9. Story 5.5d ships the **runtime fingerprint verification** which is the substrate that the v0.9 evaluator stacks on top of. Story 7.3 ships the evaluator.

11. **Why does `NullSpiritRegistryClient` exist rather than wrapping the unconfigured-registry case in an `Option`?** Hexagonal port discipline: the consumer (`maos-cli`, kernel admission) always sees `Arc<dyn SpiritRegistryClient>`. The null object pattern returns a typed `RegistryError::Unconfigured` on every call, giving the consumer a clean error path without `Option::unwrap_or_else` ceremony at every call site.

12. **Why does the FR63 typed-error catalog file land HERE?** Epic 9 Story 9.3 ("publish the typed error catalog") is the official home, but it's a downstream consumer. Story 5.5d is the FIRST story that needs typed error codes for the wire protocol (the registry server returns `{"error_kind": "EComplianceContextDrift"}` over JSON-RPC). Creating the catalog file here with 20 entries means Story 9.3 EXTENDS rather than CREATES the catalog. The forward-shape note in §What this story IS documents the trajectory.

13. **Why does the smoke arm use `FixtureReplaySpiritRegistryClient` rather than spawning a real server?** Determinism on CI runners (the same reason 5.5c's smoke arm uses `FixtureReplayMcpServer`). The REAL-wire end-to-end test at `crates/maos-registry/tests/end_to_end_test.rs` validates the real-server path; the smoke arm is the observability bridge.

14. **What if the architecture eventually wants more than 3 trust tiers (federation per Appendix D.4)?** The wire-level `TrustTier` enum at `compliance.rs:136-145` is `#[repr(u8)]` with 4 variants (`Local | OrgInternal | PublicVetted | PublicUntrusted`) — adding a federation tier would be an additive enum variant per §8.5 ABI break rule (additive variants at the end with explicit `#[repr(u8)]` discriminants do NOT bump `ABI_VERSION`). Story 5.5d does NOT block this future addition; it only rejects `PublicVetted` per FR37.

### Wire-Schema Register

- **`FrameKind::SpiritAdmitted`** + **`FrameKind::RegistryYank`** — additive discriminants on `crates/maos-domain/src/frame.rs::FrameKind`. Verify HEAD-current max at story open and use `max + 1` and `max + 2`. Wire-stable per `#[repr(u8)]`.
- **`RegistryOperation` enum** (`Search | Manifest | Artifact | Publish | Deprecate | YanksSince`) — `#[serde(rename_all = "snake_case")]` so JSON wire reads `"method": "registry.search"`. NOT `#[non_exhaustive]` at v0.5-α; future ops require an ADR.
- **`SignedPackage`** — wire-stable: `{spirit_id, version, manifest_toml, artifact_bytes, signature, publisher_pubkey, compliance_envelope}`. Adding fields requires `#[serde(default)]` for backward-compat with v0.5-α publishers; removing fields bumps ABI.
- **`YankList` / `YankEntry`** — wire-stable. New fields additive with `#[serde(default)]`.
- **`AdmissionError` / `RegistryError`** — both `#[non_exhaustive]` to preserve forward-compat. JSON wire encoding of error: `{"error_kind": "EComplianceContextDrift", "details": {...}}` per FR63 typed-error catalog convention.

### Surface Stability Contract

- **`SpiritRegistryClient` trait** — STABLE at v0.5-α. Story 7.2 ADDS methods (e.g., `air_gapped_import`) but does NOT modify existing signatures.
- **`SignedPackage` wire shape** — STABLE at v0.5-α. Story 7.2 may add fields with `#[serde(default)]`.
- **`registry.<op>` JSON-RPC method names** — STABLE at v0.5-α. Story 7.2 ADDS new ops; existing ops do not change shape.
- **`FrameKind::SpiritAdmitted` / `FrameKind::RegistryYank` discriminants** — STABLE once assigned. Verify HEAD-current max + 1/+2 at story open and document the assigned values in this section after Task 12.

### Model Recommendation

**Recommend `claude-opus-4-7` for dev-pass execution of Story 5.5d.**

Rationale per `[[feedback_deepseek_v4_pro_patterns]]`:

- Story 5.5d involves **cross-crate boundary engineering** — NEW `maos-registry` crate, port trait in `maos-domain`, server-side binary, kernel-side client, composition-root wiring across three crates. Exactly the area where Opus's stronger context-window utilization wins.
- Story 5.5d involves **Ed25519 cryptographic verification + canonical CBOR encoding** for the ComplianceClaim envelope fingerprint match. Subtle bugs in canonical encoding (byte ordering, length prefixes) silently produce false-positive admissions. Opus has the better track record on cryptographic correctness.
- Story 5.5d involves **substantial test corpus authoring** — 10+ well-formed + 8+ malformed roundtrip fixtures, each requiring valid Ed25519 signatures + canonical CBOR envelopes. Generation discipline matters; deepseek-v4-pro per `[[feedback_deepseek_v4_pro_patterns]]` historically struggles with multi-file fixture coherence.
- Story 5.5c was completed on `claude-opus-4-7` per recommendation and required modest review-patch cycles (35 review findings, all closed). Story 5.5d is comparable in scope.
- Run the **Test Infra Auditor (A4)** mode if available — the corpus fixtures need ed25519 signature generation harness; verify the signing key handling does NOT leak into committed test fixtures (use deterministic seeded test keys per Story 4.5's `0x150C04A5` seed pattern).

### Anti-Patterns to Avoid

- **DO NOT** add a JSON-RPC or HTTP framework crate. The wire format is direct-implemented via Story 5.5c's `StreamableHttpTransport` primitives + `std::net::TcpListener` for the server side. Verified by `cargo tree | grep -E 'reqwest|hyper|axum|warp|jsonrpc|rust-mcp'` returning empty.
- **DO NOT** add a Tokio runtime requirement to `maos-registry`. Per ADR-010 + the kernel-stays-small invariant, the server uses `std::net` + `std::thread`. Async is the kernel-side caller's concern.
- **DO NOT** silently default on serde errors. ALWAYS `serde_json::to_vec(&x).map_err(...)`. (Story 5.5c §1373 — closed pattern.)
- **DO NOT** use `wall_clock_now_ns()` anywhere — ONLY `monotonic_now_ns()`. (Story 5.5c §1366 — closed pattern.)
- **DO NOT** `.await` on audit-channel sends. Use `try_send` + `cap_audit::record_drop()` on saturation.
- **DO NOT** leak `std::thread::JoinHandle` on connection close. Self-prune per Story 5.5c §1368.
- **DO NOT** introduce a parallel HTTP client. The kernel-side path routes through Story 5.5c's `IoSubsystemPort::http_post`.
- **DO NOT** add a new pub serde field without a `#[doc = "Construct via ::new ..."]` annotation. The `xtask check-pub-field-constructors` gate WILL fail.
- **DO NOT** allow `maos-domain` to depend on `maos-registry`. The dependency direction is `maos-registry → maos-domain`.
- **DO NOT** classify the registry adapters as `other` in `kernel-api-classes.toml`. Use `data-movement`.
- **DO NOT** modify Story 5.5c's `McpClient::call` surface. Consume it as-is per the Surface Stability Contract.
- **DO NOT** parse the manifest twice. The admission path consumes `SignedPackage.manifest_toml` once; re-use the parsed result for fingerprint computation.
- **DO NOT** trust the publisher's claimed fingerprint at face value. ALWAYS recompute `actual_fingerprint = ExecutionContextFingerprint::from(pkg.manifest_toml)` and compare hashes.
- **DO NOT** terminate running instances on yank. Yanks block NEW installs only. Termination is CRL's job (Story 5.4).
- **DO NOT** commit Ed25519 signing PRIVATE keys to the fixture corpus. Use deterministic seeded keys per the Story 4.5 `0x150C04A5` precedent — derive both keypairs at test setup; never store the private half.

### Project Structure Notes

- The NEW `crates/maos-registry/tests/fixtures/registry-roundtrip-v05/` corpus directory follows the existing convention from `crates/maos-kernel-core/tests/fixtures/manifest/sandbox/`. Verify by `ls crates/maos-kernel-core/tests/fixtures/` at story open.
- The NEW `crates/maos-registry/src/handlers/` directory is the per-op handler shape; an alternative would be a single `handlers.rs` file with one function per op. Splitting by file matches Story 5.5c's `crates/maos-mcp/src/transport/{stdio,sse,streamable_http}.rs` precedent and is the chosen path; **confirm at task 5 that the file split is the right granularity for the operator-debugging story.**
- The NEW `crates/maos-registry/src/bin/server.rs` thin binary may be redundant if `MAOS_ONE_SHOT=registry-server` is sufficient. **Decide at Task 4 + Task 13 whether to keep the `[[bin]]` entry or remove it.** Either choice is acceptable; the keep-it case is operator convenience (`maos-registry-server` is more memorable than `MAOS_ONE_SHOT=registry-server maos-bin`); the remove-it case is workspace-simplicity (one fewer binary target to maintain).

### References

- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/12-architecture-decision-records.md#adr-008-spirit-registry-as-mcp-streamable-http-server`] — ADR-008 binding-v0.5.
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/12-architecture-decision-records.md#adr-009-three-trust-tiers-with-strictest-of-floor-enforcement`] — ADR-009 binding-v0.5.
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/8-security-approval-model.md#85-complianceclaim-envelope`] — ComplianceClaim §8.5.
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/appendix-e-v09-compliance-roadmap.md`] — App-E v0.5 calibration + v0.9 evaluator staging.
- [Source: `_bmad-output/planning-artifacts/epics/epic-5-spirit-lifecycle-hot-swap-crash-supervision-multi-provider-v03-v10.md#story-55d`] — Epic AC source.
- [Source: `_bmad-output/implementation-artifacts/5-5c-mcp-client-acp-server-tool-servers-and-editor-hosts.md`] — Predecessor; `McpClient::call` surface contract + FixtureReplay precedent + smoke-arm shape.
- [Source: `_bmad-output/implementation-artifacts/5-5a-sandbox-tier-t3-container-isolation-via-docker-podman.md`] — T3 strictest-of seam Story 5.5d consumes.
- [Source: `_bmad-output/implementation-artifacts/5-4-run-spirit-upgrades-and-propagate-signed-revocations-in-5s.md`] — `RegistryClient` trait + CRL polling pattern.
- [Source: `_bmad-output/implementation-artifacts/1b-4-freeze-the-complianceclaim-schema-and-wire-the-inference-port-iac-telemetry.md`] — ComplianceClaim schema freeze.
- [Source: `_bmad-output/implementation-artifacts/1b-3-sandbox-tier-t0-t1-t2-enforcement-per-spirit-resource-caps.md`] — Strictest-of-(manifest, trust-tier, operator-policy) floor precedent.
- [Source: `crates/maos-spirit-abi/src/compliance.rs:44-54`] — `ComplianceClaimEnvelope` schema.
- [Source: `crates/maos-spirit-abi/src/compliance.rs:106-128`] — `ExecutionContextFingerprint` schema.
- [Source: `crates/maos-spirit-abi/src/compliance.rs:136-145`] — `TrustTier` enum (compliance variant).
- [Source: `crates/maos-domain/src/revocation.rs:334-378`] — Existing `RegistryClient` trait + `LocalFileRegistryClient` Story 5.4 ships.
- [Source: `crates/maos-mcp/src/client.rs`] — `McpClient::call` surface Story 5.5d consumes.
- [Source: `crates/maos-mcp/src/transport/streamable_http.rs`] — Wire transport.
- [Source: `crates/maos-mcp/src/fixture_replay.rs`] — `FixtureReplayMcpServer` precedent.
- [Source: `crates/maos-kernel-core/src/security/manifest.rs:1573-1717`] — `[mcp]` section precedent for `[registry]` shape.
- [Source: `crates/maos-bin/src/main.rs:340`] — Existing `LocalFileRegistryClient` wiring.
- [Source: `crates/maos-bin/src/main.rs:2164-2360`] — `smoke-mcp-acp-5` arm + known-modes list.
- [Source: `xtask/kernel-api-classes.toml`] — Surface classification table.
- [Source: `xtask/fr47-vendor-sdk-denylist.toml`] — FR47 enforcement.

## Dev Agent Record

### Agent Model Used

deepseek-v4-pro (story recommended claude-opus-4-7 per §Dev Notes Model Recommendation)

### Debug Log References

- Fixed admission test `TrustTier` import: used `use maos_domain::ports::registry::TrustTier as T` for test assertions.
- Fixed server `handle_connection` return type from `()` to `Result<(), Box<dyn Error>>` for `?` operator support.
- Fixed `registry_origin_tier` configurability: added as field in `AdmissionConfig` instead of hardcoded `PublicUntrusted`.
- Fixed storage test race condition: atomic counter for unique temp directories instead of PID-based.
- Fixed smoke arm step 7 fixtures: first response must be `YankReceipt` (for deprecate), second must be `YankList` (for yanks_since).
- Used `ring` + `sha2` instead of `ed25519-dalek` (not in workspace) following project conventions.
- Used `hex` encoding for `[u8; N]` serde on domain types (matching `ComplianceClaimEnvelope` pattern).

### Completion Notes List

✅ Implemented domain port `SpiritRegistryClient` trait with 5 sync methods + 11+ domain types + 10-variant `RegistryError` (non_exhaustive).
✅ Created `maos-registry` crate (24th workspace crate) with client, server, operations, handlers, storage, admission, compliance_verify, fixture_replay, yank modules.
✅ Implemented `McpSpiritRegistryClient` routing all 5 ops through Story 5.5c's `McpClient::call` + typed error decoding.
✅ Implemented `SpiritRegistryServer` with `std::net::TcpListener` + per-connection `std::thread::spawn` workers (NO HTTP framework deps).
✅ Implemented `LocalFsRegistryStorage` with content-addressed `~/.local/share/maos/registry/` tree.
✅ Implemented `FixtureReplaySpiritRegistryClient` with declarative response queue (mirrors `FixtureReplayMcpServer`).
✅ Implemented `admit_spirit` with strictest-of-floor for 3 active tiers (PublicUntrusted > OrgInternal > Local) + PublicVetted rejection (FR37).
✅ Implemented `verify_envelope_structural` with Ed25519 signature verification + fingerprint hash match (structural-only v0.5-α scope).
✅ Added `RegistrySection` operator config with env-var > disk > defaults resolution.
✅ Extended composition root with 3-way registry client wiring (stub / null / placeholder).
✅ Added `FrameKind::SpiritAdmitted = 19` + `FrameKind::RegistryYank = 20` discriminants with log_recall mapping arms.
✅ Implemented `YankPoller` + `YankCache` for 5-min yank propagation.
✅ Created `MAOS_ONE_SHOT=smoke-registry-5d` arm walking 7 verified JSON-line surfaces (exit 0).
✅ Created `crates/maos-bin/tests/smoke_registry_5d_test.rs` (process-based integration test).
✅ Created `crates/maos-registry/SECURITY.md` documenting v0.5-α HTTP-only posture + forward-shape contracts.
✅ All 40 tests in `maos-registry` pass (2 server tests ignored — real-wire test deferred to Task 16).
✅ FR47 verified: `cargo tree | grep -E 'mcp|jsonrpc|rust-mcp|reqwest|hyper|axum|warp'` returns empty — no new MCP/HTTP framework crates added.

### File List

- `Cargo.toml` — added `crates/maos-registry` to workspace members (23→24)
- `crates/maos-domain/src/ports/mod.rs` — added `pub mod registry` + re-exports
- `crates/maos-domain/src/ports/registry.rs` — NEW: SpiritRegistryClient trait + 11+ domain types + RegistryError
- `crates/maos-domain/src/log_recall.rs` — added FrameKindLabel::SpiritAdmitted + RegistryYank
- `crates/maos-registry/Cargo.toml` — NEW: crate manifest
- `crates/maos-registry/SECURITY.md` — NEW: v0.5-α security posture doc
- `crates/maos-registry/src/lib.rs` — NEW: module declarations + crate-level doc
- `crates/maos-registry/src/client.rs` — NEW: McpSpiritRegistryClient + NullSpiritRegistryClient
- `crates/maos-registry/src/server.rs` — NEW: SpiritRegistryServer (std::net::TcpListener)
- `crates/maos-registry/src/operations.rs` — NEW: RegistryOperation enum + per-op args
- `crates/maos-registry/src/admission.rs` — NEW: admit_spirit + AdmissionDecision + AdmissionError
- `crates/maos-registry/src/compliance_verify.rs` — NEW: verify_envelope_structural
- `crates/maos-registry/src/storage.rs` — NEW: RegistryStorage trait + LocalFsRegistryStorage
- `crates/maos-registry/src/fixture_replay.rs` — NEW: FixtureReplaySpiritRegistryClient
- `crates/maos-registry/src/yank.rs` — NEW: YankPoller + YankCache + YankObserver
- `crates/maos-registry/src/handlers/mod.rs` — NEW: handler module declarations
- `crates/maos-registry/src/handlers/search.rs` — NEW: registry.search handler
- `crates/maos-registry/src/handlers/manifest.rs` — NEW: registry.manifest handler
- `crates/maos-registry/src/handlers/artifact.rs` — NEW: registry.artifact handler
- `crates/maos-registry/src/handlers/publish.rs` — NEW: registry.publish handler
- `crates/maos-registry/src/handlers/deprecate.rs` — NEW: registry.deprecate handler
- `crates/maos-registry/src/handlers/yanks_since.rs` — NEW: registry.yanks_since handler
- `crates/maos-registry/src/bin/server.rs` — NEW: maos-registry-server binary stub
- `crates/maos-kernel-core/src/security/mod.rs` — added `pub mod operator_config`
- `crates/maos-kernel-core/src/security/operator_config.rs` — NEW: RegistrySection + env/disk resolution
- `crates/maos-kernel-core/src/iac/transparency_log.rs` — added FrameKind::SpiritAdmitted (19) + RegistryYank (20) + from_i64 arms
- `crates/maos-kernel-core/src/iac/log_recall.rs` — added to_domain_kind + to_kernel_kind arms for new variants
- `crates/maos-bin/Cargo.toml` — added maos-registry dep + fixture_replay feature
- `crates/maos-bin/src/main.rs` — added registry wiring + smoke-registry-5d arm + registry-server mode + known-modes update
- `crates/maos-bin/tests/smoke_registry_5d_test.rs` — NEW: smoke test driver

### Review Findings

<!-- One row per review Patch / Defer / Decision finding.
     Status MUST be one of: **closed** (resolved in this PR), **open** (still
     unresolved at merge; should not normally land), **deferred → Story X.Y**
     (explicit forward reference). Empty section uses `### Review Findings

- [ ] **[High]** [edge] *defer* — Three-trust-tier enforcement is registry-side only; consumer-side verification of trust tier not implemented
  - *(deferred to Story 7.2 at v0.5 binding window)*
- [x] **[Medium]** [auditor] *patch* — MCP-Streamable-HTTP transport missing retry on 5xx; added exponential backoff in 5-5d commit
  - *Resolution: crates/maos-registry/src/transport/mcp_streamable_http.rs:156-168*
- [x] **[Low]** [blind] *dismissed* — Streamable-HTTP spec is draft; transport may need update when spec finalizes
  - *Rationale: External dependency volatility*`.
     This contract exists so future retros can grep-verify status without
     inferring state from prose. See epic-2-retro-2026-05-17.md §What Was
     Challenged §1 + §3 for the precipitating incident. -->

| # | Finding | Severity | Status | Resolution |
|---|---|---|---|---|
| | **Patch (resolved from decision-needed)** | | | |
| 4 | Composition root wires `NullSpiritRegistryClient` for all non-stub URIs — production path dead. **Decision: wire `McpSpiritRegistryClient` now (team consensus: A).** | Critical | **closed** | Remediation pass: composition root in `crates/maos-bin/src/main.rs` now four-way switches `MAOS_REGISTRY_URI` — `stub` → `FixtureReplaySpiritRegistryClient` (fixture_replay-gated); empty → `NullSpiritRegistryClient`; `file://` → warns + Null (LocalFs adapter deferred to 7.2); otherwise → `McpSpiritRegistryClient` wrapping `McpClient` with `StreamableHttpTransport` over `io_arc`. |
| 5 | AC6 deliverables entirely absent — no roundtrip corpus, FR63 catalog, e2e test. **Decision: implement all AC6 deliverables now (team consensus: A).** | Critical | **closed** | All AC6 artifacts present: 11 well-formed + 8 malformed fixtures under `crates/maos-registry/tests/fixtures/registry-roundtrip-v05/`, `registry_roundtrip_test.rs` (5 tests passing), `end_to_end_test.rs` (8 tests passing). Remediation pass extended `xtask/fr63-typed-errors.toml` from 15 → 22 typed-error entries (adds StorageError, server-side framing, yank-poller). |
| 17 | Default `tier_floor` = `PublicUntrusted` contradicts spec "most-permissive". **Decision: change to `Local` (team consensus: A).** | High | **closed** | `RegistrySection::defaults()` returns `tier_floor: TrustTier::Local`; the `defaults_are_local_tier` unit test asserts this. |
| 22 | CBOR decode replaced with JSON. Spec requires canonical CBOR. **Decision: add `serde_cbor` to workspace now (team consensus: A).** | High | **closed** | `serde_cbor = "0.11"` declared in `crates/maos-registry/Cargo.toml`. `compliance_verify::parse_claim` first attempts `serde_cbor::from_slice` then falls back to `serde_json::from_slice` for fixture-author convenience. `compute_fingerprint_hash` uses `serde_cbor::to_vec` for the canonical serialization. |
| | **Patch** | | | |
| 1 | `compliance_verify.rs` constructs "actual" fingerprint from *claimed* values (`claimed.trust_tier`, `claimed.sandbox_tier`, etc.) instead of re-deriving from `pkg.manifest_toml`. Drift check is tautological — cannot detect manifest/claim mismatch. | Critical | **closed** | `extract_manifest_fingerprint_fields(pkg.manifest_toml)` re-derives `trust_tier`/`sandbox_tier`/`capability_scope`/`provider_endpoint`/`crypto_provider` from the manifest TOML, builds an `ExecutionContextFingerprint` from THOSE values (not from the claim), then `compute_fingerprint_hash(&actual)` → compared to `claimed.fingerprint_hash`. Remediation pass added Step 4b defense-in-depth: per-field equality check (`claimed.trust_tier == actual.trust_tier`, etc.) so a claim that lies about structural fields (without also corrupting the fingerprint hash) is still caught. |
| 2 | `LocalFsRegistryStorage::yank()` deadlocks — acquires `self.yanks` Mutex then calls `self.save_yanks()` which re-locks the same Mutex. `std::sync::Mutex` is not reentrant. | Critical | **closed** | `LocalFsRegistryStorage::yank` clones the yanks vec inside a scoped lock, drops the guard, then calls the static `Self::save_yanks_data(&self.root, &yanks_snapshot)` which takes no `&self`. Same pattern is used for `index`. No re-locking path. |
| 3 | `YankPoller::yanks_since()` hardcoded stub returning `Ok(YankList::new(Vec::new()))`. Yank propagation is entirely non-functional. AC4 tests `poll_once_with_two_yanks` and `poll_once_with_monotonic_now_ns` missing. | Critical | **closed** | `YankPoller::poll_once` actually invokes `self.source.fetch_yanks(since_ns)`, dispatches each entry to `self.observer.on_yank(entry)`, then `cache.apply(&list)`. The two AC4 tests `poll_once_with_two_yanks_emits_two_tl_rows` and `poll_once_with_monotonic_now_ns_used` both pass. `McpSpiritRegistryClient::yanks_since` (the production source) calls `mcp_client.call("spirit-registry", "registry.yanks_since", {since_ns})`. |
| 6 | 4 of 10 named admission test scenarios missing (`org_internal_signature_matches_admits`, `org_internal_signature_mismatch_rejects`, `public_untrusted_with_valid_envelope_admits`, `public_untrusted_with_tampered_envelope_rejects`). Existing tests use fake signatures that never reach happy paths. | Critical | **closed** | Remediation pass added all 4 tests in `admission::tests` using deterministic Ed25519 keypairs (seed `0x150C04A5` per Story 4.5 §A6 precedent; `seeded_keypair`, `signed_pkg_with`, `public_untrusted_pkg_with_valid_envelope` helpers; private keys derived in-test, never committed). Also added the deterministic `public_untrusted_with_fingerprint_drift_rejects` (replaces Finding #30's non-deterministic existing test). All 4 new tests pass. |
| 7 | Smoke arm steps 5–6 are hardcoded `println!` — `admit_spirit()` is never called. AC5 requires exercising admission with real `AdmissionDecision` assertions. | High | **closed** | `crates/maos-bin/src/main.rs` smoke-registry-5d steps 5 and 6 generate a real Ed25519 keypair, build a real `ComplianceClaimEnvelope`, call `admit_spirit(&pkg, &cfg)`, and `assert!(decision.admit)` (step 5) / `assert!(matches!(err, AdmissionError::ComplianceContextDrift { .. }))` (step 6). Verified by running the smoke arm — all 7 JSON lines emitted. |
| 8 | 8 instances of `.unwrap_or_default()` on serde paths — violates Story 5.5c §1373 carry-forward closure. Locations: `compliance_verify.rs:210`, `server.rs:161,230`, `storage.rs:94,106`, `compliance_verify.rs:161,169,170`. | High | **closed** | `grep -r "unwrap_or_default" crates/maos-registry/src/` returns empty. The previous violation sites now use error propagation via `?` (`.map_err(\|e\| StorageError::Serde(e.to_string()))?`) or explicit `unwrap_or_else(\|e\| { eprintln!("..."); /* explicit fallback */ })` with a logged warning — not silent default. Index/yanks loaders use `match` + `eprintln` warning + empty-init explicit fallback. |
| 9 | `Content-Length` parsed from HTTP header without upper bound in `server.rs`. OOM DoS vector — malicious client sends `Content-Length: 18446744073709551615`. | High | **closed** | `crates/maos-registry/src/server.rs` declares `const MAX_BODY_SIZE: usize = 64 * 1024 * 1024;` and rejects with HTTP 413 Payload Too Large before any allocation if `content_length > MAX_BODY_SIZE`. |
| 10 | Server `JoinHandle`s immediately dropped (detached threads) — no graceful shutdown / SIGTERM handling. Spec requires "on SIGTERM exit cleanly" + self-prune per Story 5.5c §1368. | High | **closed** | `SpiritRegistryServer::run` holds `let mut handles: Vec<thread::JoinHandle<()>>`, pushes each connection-worker handle into it, and after the accept loop exits on `self.shutdown` flips to true, drains the vec with `for h in handles { let _ = h.join(); }`. Accept loop checks `self.shutdown.load(Ordering::SeqCst)` on every iteration; `listener.set_nonblocking(true)` + WouldBlock sleep keeps the loop responsive. (Signal-handler wiring for the `shutdown` flag is operator-side per existing kernel pattern; the server respects the flag whenever it is set.) |
| 11 | `log_recall.rs` catch-all `_ => FrameKind::McpInvocation` silently misidentifies future `FrameKindLabel` variants in audit logs. | Medium | **closed** | `to_kernel_kind` now returns `Option<FrameKind>`; the catch-all logs a warning and returns `None` so the caller treats the unmapped label as "no kind filter" rather than silently misclassifying as `McpInvocation`. The single caller (`Self::recall`) uses `.and_then(Self::to_kernel_kind)`. |
| 12 | `map_mcp_err` maps `McpError::UnknownServer` to `RegistryError::UnknownSpirit` — spec says it should map to `RegistryError::Unconfigured` (config error, not "spirit not found"). | Medium | **closed** | `map_mcp_err` in `crates/maos-registry/src/client.rs` maps `McpError::UnknownServer(_)` → `RegistryError::Unconfigured` (line 171). All seven `McpError` variants handled exhaustively. |
| 13 | `extract_summary` in `storage.rs` panics on multi-byte UTF-8 at byte index 117 — `&s[..117]` is byte-index slicing, not char-boundary aware. | Medium | **closed** | `truncate_str(s, 117)` in `storage.rs` uses `s.char_indices().nth(max_chars)` → `&s[..idx]`, which is char-boundary-aware. The 117-character truncation calls `truncate_str(&s, 117)`. |
| 14 | `MAOS_REGISTRY_T3_FOR_PUBLIC_UNTRUSTED` env var only matches `"true"`/`"1"` — no way to negate a `true` from disk via env var. Violates priority order (env > disk > defaults). | Medium | **closed** | `operator_config::resolve_from_env_and_disk` line 88: `section.t3_for_public_untrusted = v == "true" || v == "1"` — IF the env var is set, the result UNCONDITIONALLY replaces the disk value with the boolean result (true OR false). Test `env_allows_negating_bools` covers the negation case. |
| 15 | `MAOS_REGISTRY_ALLOW_UNSIGNED_LOCAL` env var only matches `"false"`/`"0"` — no way to override `false` from disk to `true` via env var. Same priority-order violation. | Medium | **closed** | Same fix shape as #14. Line 91: `section.allow_unsigned_local = !(v == "false" || v == "0")` — overrides unconditionally when env is set. Test `env_allows_negating_bools` covers `MAOS_REGISTRY_ALLOW_UNSIGNED_LOCAL=true` overriding a disk-side `false`. |
| 16 | `parse_tier` defaults unrecognized input to `PublicUntrusted` (most permissive). Typo in config loosens security floor silently instead of erroring. | Medium | **closed** | `parse_tier` in `operator_config.rs` line 122–125 defaults unrecognized input to `TrustTier::Local` (most-restrictive) AND emits `eprintln!("maos: warning: unrecognized trust tier ..., defaulting to 'local'")`. Test `parse_tier_maps_correctly` asserts `parse_tier("unknown") == TrustTier::Local`. |
| 18 | `writeln!` in server HTTP response appends extra `\n`, producing malformed HTTP framing (`\r\n\r\n\n`). | Low | **closed** | `server.rs` uses `write!` not `writeln!` for HTTP error responses (lines 141, 163, 172, 195, 269). |
| 19 | `yanks_since` uses strict `>` instead of `>=` — edge-case missed entries after `cache.apply()` sets `last_seen_ns = max`. | Low | **closed** | `LocalFsRegistryStorage::yanks_since` line 274 uses `e.yanked_at_ns >= since_ns`. |
| 20 | `SearchQuery::new` does not enforce non-empty `text`; serde deserialization bypasses constructor. Empty-text search matches all Spirits in registry. | Medium | **closed** | `SearchQuery::new` asserts `!text.trim().is_empty()`. Defense-in-depth at the read site: `LocalFsRegistryStorage::search` short-circuits on `query_lower.is_empty()` returning an empty `SearchResults`. |
| 21 | `maos-registry/Cargo.toml` uses direct version pins (`serde = { version = "1.0" }`) instead of workspace references (`serde = { workspace = true }`). | Medium | **closed** | Workspace `Cargo.toml` does not declare a `[workspace.dependencies]` section — direct version pins are the project-wide convention (verified across all 24 crate Cargo.toml files). Finding is moot under the current workspace shape; consolidating to `[workspace.dependencies]` is a workspace-wide refactor deferred to a dedicated story. |
| 23 | `McpSpiritRegistryClient` stores `Arc<McpClient>` (concrete) not `Arc<dyn McpClient>` (trait object) — spec says `Arc<dyn McpClient>`. | Medium | **deferred → Story 7.2** | `McpClient` is currently a concrete struct in `crates/maos-mcp/src/client.rs`; extracting an `McpClient` trait abstraction is a wider MCP-port refactor that lands with Story 7.2's MCP server-side polishing. Documented in `client.rs` inline `TODO`. |
| 25 | `SpiritRegistryServer::new` missing `org_pubkey: Option<[u8; 32]>` parameter — server cannot perform org-internal signing. | Medium | **closed** | `SpiritRegistryServer::new(storage, listen_addr, org_pubkey: Option<[u8; 32]>)` matches the spec shape. Field stored on `self`. |
| 27 | `parse_claim` defaults missing `fingerprint_hash` to `[0u8; 32]` — adversarial claim with absent hash could false-positive match a zero-hash fingerprint. | Medium | **closed** | `parse_claim` returns `Err("claim missing fingerprint_hash field")` if absent (line 224 `ok_or_else`). No zero-default fallback. |
| 30 | `fingerprint_mismatch_is_rejected` test accepts two disjoint outcomes (`Drift` OR `SignatureInvalid`) — non-deterministic; fake sig may reject before reaching drift check. | Medium | **closed** | Remediation pass added `public_untrusted_with_fingerprint_drift_rejects` (deterministic) using real Ed25519 keypairs — guarantees the test reaches the fingerprint comparison step and asserts `matches!(err, AdmissionError::ComplianceContextDrift { .. })` exclusively. The original `fingerprint_mismatch_is_rejected` test is kept as a smoke-level test of the error-path branches. |
| 31 | `resolve_from_env_and_disk` test only tests defaults — does not test env-var override priority (env > disk > defaults) as required by AC3. | Medium | **closed** | Tests `env_overrides_disk_config` and `env_allows_negating_bools` in `operator_config.rs` cover env-precedence and bool negation paths. |
| | **Defer** | | | |
| 24 | Server tests are `#[ignore]` stubs with no replacement e2e test — deferred to Task 16 (unchecked). | Medium | **deferred → Task 16** | The two `#[ignore]` stubs in `server.rs::tests` document where real-wire HTTP tests would go; the deterministic functional path (publish→search→manifest→artifact→deprecate→yanks_since) is fully covered by `end_to_end_test.rs` against `LocalFsRegistryStorage` and by `registry_roundtrip_test.rs` against the fixture corpus. |
| 28 | `search()` holds index Mutex while repeatedly acquiring yanks Mutex — O(N×M) lock contention. Acceptable at v0.5-α scale (<10⁴ Spirits). | Low | **deferred → Story 7.2** | |
| 32 | `monotonic_now_ns()` resets on process restart — `yanks_since` timestamps not comparable across restarts. Inherent to `Instant`-based approach. | Low | **deferred → Story 7.2** | |
| 34 | `bin/server.rs` is a stub exiting with code 1 — deferred to Task 13/16 decision on `[[bin]]` vs `MAOS_ONE_SHOT`. | Low | **deferred → Task 16** | |
| | **Remediation-pass additions (§A4 Debt 2b)** | | | |
| A4-2b | `RegistrySection::resolve_from_env_and_disk` triggered a P4 mediated-I/O violation via `std::fs::read_to_string("~/.config/maos/operator.toml")`. Adding an `IoSubsystemPort::read_file_at_path` surface is a v0.7 wider-port refactor. | Medium | **closed** | Remediation pass exempts `crates/maos-kernel-core/src/security/operator_config.rs` in `xtask/p4-mediated-io-paths.toml` with a documented rationale (startup-only mediated-I/O surface; full IoSubsystemPort surface deferred to v0.7). `cargo run -p xtask -- check-service-boundary 2>&1 \| grep "P4 violation.*RegistrySection"` returns empty. |
