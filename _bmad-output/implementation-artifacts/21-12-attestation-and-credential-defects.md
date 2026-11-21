---
story_id: '21.12'
story_key: 21-12-attestation-and-credential-defects
created: 2026-11-20
status: in-progress
context_commit: de16fa0a
baseline_commit: de16fa0a
security_sensitive: true
kernel_grant: "Forecast only; no prototype or grant requested. Kernel +55…+165 tokei with the recommended D-row options (method and per-file deltas below). ADR-069 kfree applies inside the envelope: measure at landing with check-kernel-baseline and kloc-check --json, re-pin with scripts/repin.py and write the HISTORY row in the same commit. No forecast reaches the 22,000 alarm."
kloc_grant: "Read-the-files forecast, not measured growth or permission: maos-kernel-core +55…+165; maos-iac +30…+80; maos-bin +10…+40; maos-domain 0 (D-21-12-02 A) or +5…+15 (B); maos-providers 0…+10; maos-mcp 0…+5; maos-audit 0. Re-measure after the D-rows are ruled."
review: "DIFFICULT and security-sensitive: credential handling on the kernel IO path, erasure convergence across three stores, and a signed attestation. Security review per D-E20-6 (SCP 2026-11-12 :82). Follow docs/testing/proof-contract.md. Story creation anthropic/claude-opus-5-5 (DraftAttestationCredentials). No independent review or execution claimed."
---

# Story 21.12: Attestation and credential defects — fix three before GA, not re-label

Status: in-progress

**Readiness:** `backlog` (operator rulings recorded). The operator approved D-21-12-01…04 as A/A/B/A via the preflight form on 2026-11-21 UTC. This records the proof and acceptance criteria decisions only; it does not move the story to `ready-for-dev` or authorize implementation, landing or shipping.

**GA-required, size M.** Authority: epic `planning-artifacts/epics/epic-21-multi-host-on-live-substrate-w6.md:20,76,301-312`; SCP `planning-artifacts/sprint-change-proposal-2026-11-12.md:35` (ruling B5 "fix it before GA"), `:266-281` (§5.B5 plan), `:490-491` (K7, K8), `:82` (D-E20-6); `ga3` and `kfree`, `planning-artifacts/decisions-2026-11-12-dedrift.md:28,33`; tracker `implementation-artifacts/sprint-status.yaml:424` (key) and `:505-506` (`v25-erasure-crash-reconciliation`, `v25-private-tier-erase-atomicity`); deferred rows `implementation-artifacts/deferred-work.md:616-617`, `:719`, `:720`, `:734`, `:1654`. Paths under `planning-artifacts/` and `implementation-artifacts/` are relative to `_bmad-output/`.

## Story

As **an operator who ships `v1.0.0` with signed erasure receipts and paid provider keys**,
I want **every regional teardown receipt to claim only what was done, provider credentials to reach only the pinned endpoint, and an interrupted forget to finish on its own**,
so that **the receipts, the endpoint pin and the forget cascade are true controls at GA (ruling B5), not labels**.

**Exact scope — three clauses, each a positive defensive property:**
1. **Receipt (`deferred-work.md:719`).** Already fixed by `20-3a-gate-honesty` AC9. This story re-runs the witnesses and records its own evidence. No code change.
2. **Credential (`:1654`).** The kernel IO adapter never follows an HTTP redirect, so a request and its headers go only to the URL the caller pinned, and a 3xx comes back as a typed, credential-free refusal.
3. **Cascade (`:616`/`:617`).** A forget that stops after any partial step completes from a durable intent record (the forward-completion journal of K8). This makes the cascade converge; it does not make it atomic.

Out of scope:
- Real key revocation or seed destruction for the region signing key. This stays a labelled claim boundary (`regional_teardown.rs:150-155`; ADR-059 amendment 2026-10-15, `docs/adr/ADR-059-operator-authority-collective-erasure.md:138-140`).
- `:720`: a crash between a durable mutation and its audit append, including the operator collective erase (`maos-loom-lite/src/store.rs`, `maos-bin/src/main.rs` collective path). It stays parked under `v25-erasure-crash-reconciliation`, per K8 and epic AC4.
- `:734` / ADR-059 residual 9: the general non-atomicity of the private tier inside `forget_principal_unix`. It stays parked under `v25-private-tier-erase-atomicity`.
- The duplicate-fallback N+1 row (`deferred-work.md:1655`, F2). It is ownerless and is not part of B5.
- `RegistryHttp` (`maos-registry/src/http.rs`), its own `IoSubsystemPort` over raw TCP, and `maos-notify-push`, which already builds its agent with `.redirects(0)` (`maos-notify-push/src/lib.rs:92-99`).
- Any red-before exploitation: no test or plant shows a credential reaching a second host (D-21-12-01).

## Inventory at base — source measurements, not execution

All observations were read at HEAD `de16fa0a` in `/home/lunarpulse/dev_ws-course28` with `git show`/`grep`/`sed`. The working tree has uncommitted changes from other lanes in `crates/maos-bin/src/admission.rs` (+5 lines above `:1128`), `maos-bin/tests/manifest_model_pin_s1.rs`, `xtask/src/release_verify.rs` and `xtask/tests/ci_nightly_w3.rs`. Cites below are at HEAD; working-copy offsets are noted where they differ. The ureq behavior was read from the locked source `ureq-2.12.1` (`Cargo.lock:5794-5795`) in the local cargo registry. No Cargo, build, test, gate, formatter, tokei or prototype was run.

| Surface | State at base | Preserve / required change |
|---|---|---|
| **AC1 receipt** | `decommission_region_key` (`crates/maos-audit/src/erasure/regional_teardown.rs:156-166`) returns `completed: false` (`:163`). Its doc (`:150-155`) states that the seed is retained and can still derive the key. The builder fails closed (`RegionalTeardownError::IncompletePhase`, `:195-199`). Unit witnesses: `ac14_fail_closed_when_key_decommission_incomplete` (`:438-453`) and `retained_seed_still_derives_region_key_after_decommission_attempt` (`:455-465`). Binary witnesses: `regional_uninstall_signs_key_coverage_gap` (`crates/maos-bin/tests/erasure_uninstall_13_5b.rs:367`) and `regional_uninstall_emits_no_success_receipt` (`:419`). Synthetic verifier tests keep the crypto path covered (`regional_teardown.rs:467-…`; 20-3a review patch, `20-3a-gate-honesty.md:217`). | No code change. Preserve every test named here. |
| **AC1 evidence on record** | `20-3a-gate-honesty.md:136` lists "Proven red: re-plant `completed: true` → the new assertion reds" as **AC text**, and T10 is checked (`:168`). **No recorded command or output for that plant exists**: none of the 8 logs or the `t11-harness-residuals/` directory under `20-3a-evidence/` mentions `decommission` or `ac14_fail_closed`, and the Completion Notes plant line (`:227`) names only pentest/trial/verifier mutations. | The epic's "cites the planted-red evidence of the 20-3a closure" is **unsupported**. AC1 records its own run instead (see the corrections below). |
| **AC2 adapter** | `IoSubsystemAdapter` is a unit struct, `#[derive(Debug, Clone)]` (`crates/maos-kernel-core/src/io/mod.rs:56-63`). `http_get` calls the free function `ureq::get(url).call()` (`:95-97`) and returns the body for every status below 400 (`:95-98`). `http_post` maps any non-2xx exchange to `IoError::Transport("… status {s}")` (`:101-111`). `http_post_exchange` uses `ureq::post(url)` plus `req.set(k, v)` for each header (`:123-126`) and returns every status as data (`:127-141`); it withholds `BadHeader` text (`:130-135`). No `.redirects(` call exists under `io/`. | Hold one `ureq::Agent` built with `.redirects(0)` and send every request through it. `ureq::Agent` is `Debug + Clone` (`ureq-2.12.1/src/agent.rs:111-112`), so both derives stay. Every constructor goes through `IoSubsystemAdapter::new()`: `maos-bin/src/main.rs:2151`, kernel tests, `maos-journey-test/tests/mcp_conformance_18_8.rs:43,65`, `maos-bin/tests/manifest_model_pin_s1.rs:147`, xtask fixtures `xtask/tests/fixtures/p1-*/src/main.rs`. No struct literal exists, so the field addition is source-compatible. |
| **ureq 2.12.1 redirect semantics** | The default agent follows up to 5 redirects (`agent.rs:262`). 301/302/303 turn a POST into a GET (`unit.rs:190-196`). The retained headers drop only `content-length`, `cookie` and `authorization` (`unit.rs:219-225`). With `redirects == 0`, the 3xx is returned as the response (`unit.rs:165-167`). `Error::Status` is built only for status ≥ 400 (`request.rs:169-172`). | A 3xx therefore comes back as `Ok(response)`. `http_post_exchange` returns it as data. **`http_get` would return a 3xx body as success**, so it needs an explicit non-2xx refusal (AC2). |
| **AC2 callers** | Anthropic (`maos-providers/src/anthropic.rs:81-90`, `x-api-key` at `:86`), Gemini (`gemini.rs:84-91`, `x-goog-api-key` at `:89`), OpenAI (`openai.rs:107`, `Authorization: Bearer` at `:112`) and Ollama (`ollama.rs:57`) all call `crate::provider::post_json` (`provider.rs:108-125`), which maps every non-2xx to `ProviderError::ProviderRejected { status, body: rejection_excerpt(..) }`. The excerpt withholds any body that echoes a credential skeleton (`provider.rs:127-148`). MCP streamable HTTP maps non-2xx to `McpTransportError::HttpStatus { status, .. }` (`maos-mcp/src/transport/streamable_http.rs:103-116`). MCP SSE is the only production `http_get` caller (`maos-mcp/src/transport/sse.rs:27-31`). | Under D-21-12-02 A, no driver changes: a 3xx on POST already becomes a typed refusal that names the status and withholds credential echoes. The SSE path receives the new `http_get` refusal as `McpTransportError::Transport`. |
| **AC2 endpoint pin** | `check_provider_pins` (`crates/maos-bin/src/admission.rs:1128-1166` at HEAD) checks the configured endpoint against the registered one (`pin_ok` `:1151-1154`, `url_ok` `:1155-1158`, refusal `:1159-1163`). The working copy is +5 lines (`:1156-1168`). | Unchanged. With redirects off, the pinned URL is the only host that receives a request. |
| **AC2 test harness** | `crates/maos-kernel-core/tests/io_redaction_and_status_20_4.rs` (`serve_once` `:21-50`; 2 tests) and `io_exchange_18_8.rs` (`serve_once` `:17`, 6 tests, including a GET at `:118-127`) run one-shot loopback servers. | New AC2 tests extend `io_redaction_and_status_20_4.rs` with the same one-listener harness. |
| **AC3 cascade** | `forget_with_reason` (`crates/maos-kernel-core/src/memory/mod.rs:448-617`) runs under `forget_serialization` (`:456-459`). Legal-hold placement (`:470-524`) and the prior-hold suspension (`:528-551`) are the same function. The `#[cfg(test)]` hook fires before any mutation (`:552-555`; builder `:434-438`). Then come the scrub (`scrub_forget_frames`, `:557-566`), `private.forget_principal` (`:569`), `principal_index.forget` (`:570`), the `principal.forget` payload (`:574-581`) and its insert (`:582-588`), and the receipt (`:589-615`). There is no intent record and no completion path: a failure after `:569` returns `Err` with the index rows still present and no frame. | Insert a durable intent before the scrub, and route the live path and the completion path through one idempotent step sequence. |
| **AC3 idempotence** | Scrub: re-runnable, and a retry recovers outstanding re-distillation requests and already-scrubbed ids (`deferred-work.md:617`; ADR-044 `:84-86`); `ErasurePending` is returned while held co-principals or an unsettled sweep remain (`maos-iac/src/adapter/transparency_log.rs:2047-2075`). Private: `forget_principal` takes `io_lock` and scans the map plus disk without the index (`memory/private.rs:1131-1144` onward), so a second run deletes the remainder and returns its count. Index: `DELETE … WHERE principal_id = ?1` (`memory/principal.rs:212-224`), so a second run returns 0. | Each step can be run again. The completion path must count only what it deleted, and say so. |
| **AC3 scrub exclusion** | The principal sweep skips frames whose intent is `principal.forget`, `principal.forget.held`, `distillate.redacted` or `principal.uninstall.%` (`transparency_log.rs:1999-2003`). | **Precondition:** a new `principal.forget.intent` frame carries the principal id, so the same scrub would erase it unless it is added to this exclusion list (maos-iac). |
| **AC3 precedent for "intent without outcome"** | `outstanding_redistill_requests` (`transparency_log.rs:2096-…`) selects request frames with no matching outcome frame through dedicated indexes (`:568-575`). Consumers: `maos-bin/src/hook_pump.rs:793`, `maos-iac/src/adapter/erasure_scan_work.rs:93,180`. | Reuse this pattern for "an intent frame with no later `principal.forget` frame for the same principal". Do not invent a second journal. |
| **AC3 callers** | One-shot forget (`maos-bin/src/main.rs:7595-7603`), the uninstall cascade (`main.rs:11563`), the operator door (`maos-bin/src/operator_door.rs:2452`) and the port `forget` (`memory/mod.rs:912-913`). The daemon assembles `memory` at `main.rs:2596-2615`. | Completion must run before any of these acts (D-21-12-03). |
| **AC3 fault injection** | `memory/spill_test_faults.rs` is included only under `#[cfg(any(test, debug_assertions))]` (`memory/mod.rs:21-22`), is reachable from integration tests (`tests/principal_identity_20_3b.rs:504-507`) and is excluded from the kloc count (`xtask/src/kloc_check.rs:306-309`). The `#[cfg(test)]` hook pattern (`mod.rs:87-88,434-438`) is reachable only from in-file unit tests, which do count as kernel code. Restart pattern: reopen the stores on the same directories (`tests/private_forget_restart_13_5i.rs:46,64`); file-backed TL: `TransparencyLogAdapter::open_with_global_legal_holds` (`tests/forget_cascade_18_6.rs:24`). | Add a `FailurePoint` for "after private delete" rather than a second `#[cfg(test)]` hook. |
| **Docs that state the boundary** | ADR-044 `:84-86` and `:134`, :199-202 ("cross-store atomicity remains `v25-erasure-crash-reconciliation`"); ADR-059 `:155` (residual 9); `docs/release/v1.0.0-rc.2.md:173` lists F1 as an open P3 row. | Reword ADR-044 to "converges by forward completion (21-12); not atomic", keep the `:720` and `:734` remainders, and hand the GA-notes line to the tag lane. |

Unqualified kernel file names are under `crates/maos-kernel-core/src/`.

## Decisions — settled authority and operator rulings

### Already settled; do not ask again

| Decision | Binding disposition |
|---|---|
| B5 (SCP 2026-11-12 `:35`) | Fix all three before GA; no re-labelling. |
| K8 (`:491`) | `:719` is a witness re-run only. `:616`/`:617` are re-owned to this key and fixed by a forward-completion journal. `:1654` (cited as `:1653`) is fixed with `redirects(0)` and a driver-result assertion. `:720` and `:734` stay parked, which supersedes `:720`'s "scope them together or neither" for this story. |
| `ga3` (`decisions-2026-11-12-dedrift.md:28`), K7 (`:490`) | GA precondition; the key `21-12-attestation-and-credential-defects` is final. |
| `kfree` (`decisions-2026-11-12-dedrift.md:33`; ADR-069 amendment) | Kernel growth inside the envelope is self-service and measured at landing. No prototype, grant or STOP exception. |
| D-E20-6 (SCP `:82`) | GPT-family security review stands while GLM quota is out, disclosed on the record. Hold the landing for GLM only if the story adds a new trust boundary; this one adds none. |
| SCP D-row alternative (`:281`) | Removing the claim is not proposed. `:1654` has no alternative. |

### D-rows — operator rulings (2026-11-21 UTC preflight)

| ID | Material choice and bounded options | Recommendation and evidence | Operator ruling |
|---|---|---|---|
| **D-21-12-01** | **Proof form.** SCP §5.B5 (`:268`, `:278`) and the epic (`:307-309`) require falsifiable evidence for each clause. **A** positive properties plus a benign control-removal plant per AC: the existing receipt predicate, a local redirect refusal using no credential, and interrupted forget completion on restart. **C** positive properties only, with no plants, as a recorded `proof-contract` exception. Vulnerability reproduction and credential delivery are out of scope. | **A.** This satisfies `proof-contract.md:28` and K8's driver-result assertion without reproducing a credential leak. C drops the falsifiability the contract requires. | **A approved — 2026-11-21 UTC.** Benign evidence only; no credentials or attack tests. |
| **D-21-12-02** | **Typed result of a refused redirect.** **A** no new type: POST callers keep `ProviderRejected { status: 3xx, body: rejection_excerpt }` (`provider.rs:118-124`) and MCP `HttpStatus` (`streamable_http.rs:110-116`); `http_get` refuses every non-2xx with `IoError::Transport("ureq GET {url}: status {s}")`, the same form as `http_post` (`io/mod.rs:107-110`). **B** a new `IoError::RedirectRefused { status }` in `maos-domain`, returned by the adapter for every 3xx on GET and POST and mapped by each driver. **C** A, plus the adapter strips any 3xx body so no redirect page text reaches an excerpt. | **A.** Every POST path already produces a typed refusal that names the status and withholds credential echoes (`provider.rs:127-148`); the adapter never reads `Location`, so no redirect target is echoed. A adds no domain or driver surface. B touches four drivers and the domain error type for a case A already types. C is redundant with the excerpt rules. | **A approved — 2026-11-21 UTC.** Preserve existing typed refusals; no new error type. |
| **D-21-12-03** | **When an unfinished forget is completed.** **A** at daemon boot only, after `memory` is assembled (`main.rs:2596-2615`) and before the door serves. **B** at boot **and** at the start of every `forget_with_reason` call, under `forget_serialization` and before the hold check, so the one-shot forget (`main.rs:7595`) and the uninstall cascade (`:11563`) complete stale intents too. **C** only at the start of `forget_with_reason`. | **B.** The one-shot paths do not start the daemon, so A misses them. C leaves a crashed forget pending until someone forgets again. Under B, completion shares the existing guard, so no concurrent hold or forget can interleave (`mod.rs:456-459`; 16-5 race test at `:1130-1142`). | **B approved — 2026-11-21 UTC.** Run completion under the guard at boot and at the start of every forget. |
| **D-21-12-04** | **An outstanding intent meets a later legal-hold request.** **A** complete the erasure first: the intent predates the hold, so the hold is recorded and its record says "PRINCIPAL NOT PRESENT — NOTHING TO ERASE; HOLD RECORDED", the honest status already used for a hold over an erased principal (`mod.rs:470-474,514-518`). **B** suspend: leave the partial state, journal `principal.forget.held` with `partial: true` and return a receipt saying the principal is partially erased and held. **C** refuse the hold while an intent is open, with a typed error, until completion runs. | **A**, which follows from D-03 B: completion runs under the same guard before the hold check, so the existing rule "the hold waits for the in-progress forget" (16-5, `mod.rs:1142`) carries across a restart. B keeps data in a state no receipt describes. C makes litigation holds depend on reconciler health. | **A approved — 2026-11-21 UTC.** Complete the earlier recorded erasure before recording the later hold. |

No kernel permission, surface-hash permission or STOP exception is requested. A final forecast above 22,000 would need an **ENVELOPE ALARM** D-row, and one above 23,000 a new operator decision (ADR-069). Neither is forecast.

## Acceptance Criteria

Numbering follows the epic (`:307-310`). Each "per D-21-12-0x" binds to the operator's ruling on that row.

1. **AC1 — the regional teardown receipt never claims a decommission that did not happen (`:719`; witness re-run, no code change).**
   - Running `cargo test -p maos-audit --lib erasure::regional_teardown` and `cargo test -p maos-bin --test erasure_uninstall_13_5b` passes `ac14_fail_closed_when_key_decommission_incomplete`, `retained_seed_still_derives_region_key_after_decommission_attempt`, `regional_uninstall_signs_key_coverage_gap` and `regional_uninstall_emits_no_success_receipt`.
   - Positive properties these tests show: production `decommission_region_key` returns `completed: false`; the builder refuses a receipt with an incomplete phase; a region-pinned uninstall signs the `region_signing_key` coverage gap and ends `failed`; no successful regional receipt is emitted.
   - The proof required by D-21-12-01 is recorded under `21-12-attestation-and-credential-defects-evidence/ac1/`. That record replaces the 20-3a plant evidence the epic cites, which was never recorded.
   - The remainder (no real key revocation; the retained seed derives the key) stays a labelled claim boundary. This AC does not change it.
2. **AC2 — no HTTP request from the kernel IO adapter follows a redirect (`:1654`).**
   - `IoSubsystemAdapter` holds one `ureq::Agent` built with `.redirects(0)`, and `http_get`, `http_post` and `http_post_exchange` all go through it. No free-function `ureq::get(`/`ureq::post(`/`ureq::request(` call is left under `crates/maos-kernel-core/src/io/`.
   - Against one loopback listener that answers `302` with a same-origin `Location`, using a **non-secret sentinel** header value:
     - (a) `http_post_exchange` returns `Ok(HttpExchange { status: 302, .. })`, and the listener sees exactly one request.
     - (b) `http_post` returns `IoError::Transport`; a provider driver over the real adapter (`post_json`) returns `ProviderRejected`. Both name status 302 and their error text contains neither the sentinel value nor the `Location` value. MCP streamable HTTP retains `HttpStatus`. No new error type is introduced (D-21-12-02 A).
     - (c) `http_get` returns `IoError::Transport` naming status 302 and never returns a 3xx body as success. The listener sees exactly one request.
   - The 2xx, 4xx and oversized-body behavior of `io_exchange_18_8.rs` and `io_redaction_and_status_20_4.rs` is unchanged.
   - The endpoint pin (`admission.rs:1151-1163`) is unchanged, and the provider documentation states that kernel provider and MCP requests never follow redirects.
3. **AC3 — an interrupted forget converges to complete (`:616`/`:617`, forward-completion journal).**
   - Before the scrub, `forget_with_reason` appends a durable `principal.forget.intent` frame carrying the principal id and the reason. The redaction set is **not** in the intent; the re-runnable scrub recomputes it (see the corrections below). The intent is appended after the hold checks, so a suspended forget writes none.
   - `principal.forget.intent` is added to the principal-sweep exclusion list (`transparency_log.rs:2002`), so the scrub does not erase it.
   - A maos-iac query returns every intent that has no later `principal.forget` frame for the same principal, following the `outstanding_redistill_requests` pattern (`:2096`).
   - Completion runs under `forget_serialization` at daemon boot after `memory` is assembled and before the door serves, and at the start of every `forget_with_reason` call before checking holds (D-21-12-03 B). For each outstanding intent it runs the same idempotent sequence (scrub → `forget_principal` → `principal_index.forget`) and appends `principal.forget` with `"completed_from_intent": <intent frame id>` and the counts deleted **by that run**. It claims nothing about what the failed attempt deleted (ADR-059 residual 9).
   - If the scrub returns `ErasurePending`, the intent stays outstanding, is reported, and is not retried in a loop within one call.
   - Under the same guard, an earlier recorded erasure completes before a later legal hold for that principal is recorded. The hold uses the existing status "PRINCIPAL NOT PRESENT — NOTHING TO ERASE; HOLD RECORDED" (D-21-12-04 A).
   - **Operator proof:** after a forget interrupted at the `ForgetAfterPrivateDelete` failure point and a restart of the stores on the same directories and audit DB, the principal index lists zero rows for the principal, exactly one `principal.forget` frame references the intent, a second restart appends nothing, and a bystander principal's entries and index rows are untouched.
   - ADR-044 `:84-86`, `:134` and `:199-202` describe the cascade as "converges by forward completion (21-12); not atomic", and the `:720`/`:734` remainders are kept and named.
4. **AC4 — owners and records (K8).** `deferred-work.md:616-617` and `:1654` close with this story's evidence. `:719` gets a dated re-run line. `:720` and `:734` stay with their `v25-*` owners. The orchestrator makes these edits with the tracker row; this story file does not.

## Tasks / Subtasks

- [ ] **T0 — apply rulings.** Record the operator dispositions for D-21-12-01…04 here, and refresh the forecast below if a non-recommended option is chosen. Do not start implementation before the rulings exist.
- [ ] **T1 — AC1 re-run.** Run the four named tests and the D-01 proof, and record the commands, outputs and wall-clock under `…-evidence/ac1/`. No source edit.
- [ ] **T2 — AC2 adapter (kernel).**
  - Add the `agent: ureq::Agent` field to `IoSubsystemAdapter`; `new()` builds it with `ureq::AgentBuilder::new().redirects(0).build()` (precedent `maos-notify-push/src/lib.rs:95-99`).
  - Route the three methods through `self.agent`.
  - Add the `http_get` non-2xx refusal per D-02, and update the module doc (`io/mod.rs:3-24`).
  - Keep the `io_call_journal` records and the `BadHeader` withholding unchanged.
- [ ] **T3 — AC2 tests.** In `io_redaction_and_status_20_4.rs`, add the three single-listener tests (a)-(c) and one driver-level test over `post_json` (Anthropic, as in `tests/anthropic_error_body_live_leg.rs:83`). Use a non-secret sentinel header value, never a credential or credential-shaped value. No attack tests.
- [ ] **T4 — AC3 journal (maos-iac).** Add the intent kind to the exclusion list at `transparency_log.rs:2002`. Add `outstanding_forget_intents()` with an index in the `:568-575` style.
- [ ] **T5 — AC3 cascade (kernel).**
  - Extract the scrub → private → index → journal sequence of `forget_with_reason` (`mod.rs:557-588`) into one helper used by the live path and by completion.
  - Append the intent before the scrub. Add `complete_outstanding_forgets()` per D-03 and D-04.
  - Add `FailurePoint::ForgetAfterPrivateDelete` to `spill_test_faults.rs`, checked right after `:569` behind `#[cfg(any(test, debug_assertions))]`.
- [ ] **T6 — AC3 wiring (maos-bin).** Call completion at boot after `main.rs:2596-2615` and before the door serves, and surface outstanding `ErasurePending` intents in the boot log or report. Under D-03 B, the call inside `forget_with_reason` covers the one-shot and uninstall paths.
- [ ] **T7 — AC3 tests.** New `crates/maos-kernel-core/tests/forget_forward_completion_21_12.rs` (names below), using the file-backed TL and the reopen pattern.
- [ ] **T8 — docs.** ADR-044 wording (AC3), provider/IO doc sentence (AC2). The GA-notes line for F1 goes to the tag lane (`docs/release/v1.0.0-rc.2.md:173` precedent); it is not edited here.
- [ ] **T9 — landing, orchestrator-owned.**
  - Measure with `check-kernel-baseline --json` and `kloc-check --json`.
  - Re-pin the kernel exact bytes, file set and hashes with `scripts/repin.py`, and write the HISTORY row (driver, files, before → after) **in the same commit**. Refresh any ADR-066 surface hash this change causes, under `kfree`.
  - Non-kernel ceilings are retrospective rows.
  - Run difficult-tier review with the security layer per D-E20-6.
  - Only the orchestrator updates the tracker, the epic and deferred work (AC4).

## Proof plan — existing anchors and named new tests, no execution claimed

### Existing anchors to preserve (must stay green)

| Boundary | Target / anchor |
|---|---|
| Receipt | `maos-audit` `erasure::regional_teardown` unit tests (`regional_teardown.rs:355-618`); `maos-bin/tests/erasure_uninstall_13_5b.rs:367,419`. |
| IO status and redaction | `io_exchange_18_8.rs` (6 tests); `io_redaction_and_status_20_4.rs` (`:55`, `:82`); `anthropic_error_body_live_leg.rs`; `maos-journey-test/tests/mcp_conformance_18_8.rs`. |
| Forget cascade | `forget_cascade_18_6.rs`; `private_forget_restart_13_5i.rs:124,146,168`; `principal_identity_20_3b.rs:504-560`; `memory/mod.rs` legal-hold race unit test (`:1130-1142`). |

### New tests (names fixed here; bodies follow the approved rulings)

`crates/maos-kernel-core/tests/io_redaction_and_status_20_4.rs` (AC2):
- `a_redirect_on_post_exchange_is_returned_not_followed`
- `a_redirect_on_post_is_a_typed_refusal_without_header_or_location_text`
- `a_redirect_on_get_is_refused_not_returned_as_success`
- `a_provider_driver_reports_a_redirect_as_rejected_with_its_status`

`crates/maos-kernel-core/tests/forget_forward_completion_21_12.rs` (AC3):
- `forget_interrupted_after_private_delete_completes_after_restart`
- `completion_is_idempotent_across_a_second_restart`
- `completion_frame_counts_only_what_it_deleted_and_names_its_intent`
- `intent_frame_survives_the_principal_scrub`
- `suspended_forget_writes_no_intent`
- `pending_scrub_leaves_the_intent_outstanding_and_reported`
- `hold_after_interrupted_forget_follows_the_ruled_order` (D-04)
- `bystander_principal_is_untouched_by_completion`

Per D-21-12-01 A, record three benign control-removal plants on `ubuntu-24.04`, one per AC, under `21-12-attestation-and-credential-defects-evidence/`: the receipt predicate, same-origin loopback redirect refusal with no credentials, and interrupted forget completion on restart. No credential delivery, vulnerability reproduction or attack tests. The landing owes one final-tree matrix run (`proof-contract.md:12-17`). No fixture, cassette, Postgres service or hosted job is introduced or claimed.

## Kernel and kloc forecast — method, not a prototype

**Method:** read the routines this story touches and price each addition: `io/mod.rs` (whole file, 147 lines), `memory/mod.rs` `forget_with_reason` and its builder and hook (`:434-617`), `memory/private.rs` `forget_principal`, `memory/principal.rs` `forget`, `spill_test_faults.rs`, the TL sweep and the redistill query, and the ureq 2.12.1 redirect loop. No source prototype, compiler output, gate output or tokei run was produced. All figures are `[INFERENCE]` and assume the recommended options.

Recorded base: kernel exact pin **26,061 physical**, **20,428 tokei / 90 files** (`xtask/kernel-core-baseline.toml:505-507`, 27-1 HISTORY row). Kernel ceiling **20,429** (`xtask/kloc.toml:272`). These are recorded values, not a new measurement.

| Kernel area read | Net tokei | Basis / option dependency |
|---|---|---|
| `io/mod.rs`: agent field, `new()` builder, three call sites switched | +6…+10 | The unit struct gains one field; the call sites change 1:1. |
| `io/mod.rs`: `http_get` non-2xx refusal (D-02 A) | +4…+8 | Mirrors `http_post`'s match (`:107-110`). B adds a match arm instead, about the same. |
| `memory/mod.rs`: intent append before the scrub | +10…+20 | A JSON payload plus `insert_kernel_event_returning_id`, like `:574-588`. |
| `memory/mod.rs`: extract the shared step sequence | +10…+30 | Net of moving `:557-588` into a helper. |
| `memory/mod.rs`: `complete_outstanding_forgets` (query, loop, `completed_from_intent`, `ErasurePending` handling) | +20…+80 | D-03 B adds about 3…8 for the in-guard call. D-04 A needs no extra code beyond the ordering. |
| Failure-point call after `:569` | +2…+4 | `spill_test_faults.rs` itself is outside the kloc count (`kloc_check.rs:306-309`) but inside the physical pin. |
| Doc comments | 0 | Comments are not tokei code; physical lines rise. |
| **Total (recommended options)** | **+55…+165** (rounded) | Not a grant. |

Projected from 20,428: **20,483…20,593**, below the 22,000 alarm and the 23,000 envelope. 21-6 forecasts +185…+440 and 21-9 +87…+250 from neighboring bases, and both also touch `memory/mod.rs`. All three together reach at most ~21,283, still below the alarm. This is not a reservation; refresh it against the actual landing base. The kernel file count stays at 90 unless T5 splits completion into a new file (91, captured by the re-pin).

Other production crates (tests and docs excluded; current usage not measured): maos-iac +30…+80 (exclusion entry, intent query, index; ceiling 9,053, `kloc.toml:416`); maos-bin +10…+40 (boot call and report; ceiling 39,699, `:527`); maos-domain 0 under D-02 A, +5…+15 under B (ceiling 10,483, `:435`); maos-providers 0 under A, +5…+10 under B (ceiling 2,026, `:437`); maos-mcp 0…+5 (ceiling 1,274, `:440`); maos-audit 0.

## Cross-story dependencies and claim limits

- **Parallel:** 21-6 and 21-9 touch `memory/mod.rs` (21-6 adds quota credits inside `forget_with_reason`, so its "credit each affected owner" and this story's completion helper meet at `:569-570`). Rebase onto whichever lands first. If 21-6 lands first, the completion run must credit only what it deleted. `later-registry-quarantine-closure` and 21-7 do not touch these files.
- **21-10:** this story flips no invariant claim. 21-10 may cite AC3 for the erasure invariant.
- **Claims:** AC3 is convergence, not atomicity. Between the failure and the completion run, the stores are inconsistent, and the window lasts until the next boot or forget (D-03). The receipt of the failed call is an error; the completion frame is the record. AC2 covers the kernel IO adapter only (`RegistryHttp` is out of scope above). AC1 makes no key-revocation claim.

## Re-measured cite corrections — parent-owned source updates

Only this story file is written. These OLD → NEW corrections, measured at `de16fa0a`, are a handoff to the owners of those files, not edits made here:

| Source premise | OLD → NEW |
|---|---|
| Epic `:76,:305,:308,:310,:311`; tracker `sprint-status.yaml:424`; SCP `:278`, K8 `:491` | `deferred-work.md:1653` → **`:1654`** (the redirect row). `:616`, `:617`, `:719`, `:720` and `:734` hold. |
| Epic AC2 `:308`, SCP `:278` "the drivers differ: Gemini → `ProviderRejected` (`gemini.rs:95-100`), Anthropic via `http_post` → transport error (`io/mod.rs:110`; `anthropic.rs:81-94`)" | **Stale.** Both drivers (and OpenAI and Ollama) call `provider::post_json` (`anthropic.rs:81-90`, `gemini.rs:84-91`; `provider.rs:108-125`), which maps any non-2xx to `ProviderRejected`. `http_post`'s transport mapping (`io/mod.rs:107-110`) is on no driver path. The per-driver decision becomes D-21-12-02. |
| Epic AC2 `:308` | Not mentioned: with `redirects(0)`, `http_get` (`io/mod.rs:91-99`) would return a 3xx body as success; its only production caller is MCP SSE (`sse.rs:27-31`). Added to AC2 (c). |
| Deferred `:1654` cites | `io/mod.rs:96-118` → GET `:95-98`, exchange `:114-142`; `anthropic.rs:87` → `:86`; `gemini.rs:89` holds; `admission.rs:996-1005` → `:1151-1163` (HEAD; `:1156-1168` in the current working copy). |
| Deferred `:1654` fix sketch "a 3xx surfaces as `Error::Status`" | **Incorrect for ureq 2.12.1:** `Error::Status` is built only for status ≥ 400 (`request.rs:169-172`), and with redirects 0 the 3xx is returned (`unit.rs:165-167`). SCP `:278` already corrected this; the deferred row did not. |
| Epic AC1 `:307`, SCP `:272` "cites the planted-red evidence of the 20-3a closure" | **Unsupported.** `20-3a-gate-honesty.md:136` states the plant as AC text only. No recorded command or output exists in `20-3a-evidence/` or in the Completion Notes (`:227`). AC1 records its own proof. |
| Epic AC1 `:307` cites | `regional_teardown.rs:156`, `:163`, `:439` and `erasure_uninstall_13_5b.rs:367,419` hold. Add `retained_seed_still_derives_region_key_after_decommission_attempt` (`:456`), which the deferred row names and the epic omits. |
| Epic AC3 `:309` journal `:574-590`; SCP `:280` `:577-585`, hook `:435-438` | Payload `:574-581`, insert `:582-588`; the hook builder is `:434-438` (cfg at `:434`), fired at `:552-555`. `:448`, `:565`, `:569` and `:570` hold. |
| Epic AC3 `:309` / SCP `:280` "intent carries the principal and the redaction set" | **Inconsistent with the stated order:** the redaction set is computed by `scrub_forget_frames` (`transparency_log.rs:2047`), which runs after the intent. The intent carries the principal and the reason; the idempotent scrub recomputes the set (`deferred-work.md:617`). |
| Epic AC3 `:309` "new `#[cfg(test)]` fault hook (pattern `with_forget_after_hold_check_hook`)" | Use the existing `spill_test_faults` `FailurePoint` pattern instead (`memory/mod.rs:21-22`; `kloc_check.rs:306-309`). It is reachable from a restart integration test, and the `#[cfg(test)]` hook is not. This is reuse of an existing convention, not a new one. |
| Epic AC3 (not mentioned) | The new intent frame must be added to the principal-sweep exclusion (`transparency_log.rs:1999-2003`), or the scrub erases it. |
| Epic AC4 `:310` "`:1653` is ownerless and open at HEAD" | `:1654` already names this key as owner ("re-owned 2026-11-13"); it is open, not ownerless. |

## Dev Agent Record

### Agent Model Used

Creation: **anthropic/claude-opus-5-5** (`DraftAttestationCredentials`). No implementer or independent-review attribution is claimed.

### Completion Notes List

- Created this key's story file in `/home/lunarpulse/dev_ws-course28` at HEAD `de16fa0a`, with read-only `git show`/`grep`/`sed`. Only this file was written. No epic, tracker, deferred-work, ADR or source edits, and no staging, commit or push.
- Read `bmad-create-story`, the 21-6 exemplar, epic §21-12 and its header rows, SCP 2026-11-12 §5.B5/K7/K8/D-E20-6, the `kfree` and `ga3` rulings, the four deferred rows and their neighbors, `20-3a-gate-honesty.md` and its evidence directory, `docs/testing/proof-contract.md`, the IO adapter and every `IoSubsystemAdapter`/`http_get` caller, the four provider drivers and `post_json`, both MCP transports, the endpoint-pin check, the ureq 2.12.1 agent/unit/request source, the forget cascade and its stores, the TL sweep exclusion and the redistill query, the fault-injection module, and ADR-044/ADR-059. `_bmad/scripts/resolve_customization.py`, `_bmad/bmm/config.yaml` and `_bmad/custom/` are absent in this checkout, so skill defaults were applied directly. The sprint-status update step is not performed: the tracker is parent-owned.
- No builds, Cargo, tests, gates, formatters, tokei, remote jobs or prototypes were run. No green result, red-before result, exact kernel delta or exploit reproduction is claimed. `intent-lineage-coverage-report.md` was not touched.
- 2026-11-21 UTC: Recorded operator preflight approval of D-21-12-01…04 (A/A/B/A). The story stays `backlog`; the task checklist and scope are unchanged. Approval records the decisions only; no implementation, checks, landing or shipping are authorized or claimed.

### File List

- `_bmad-output/implementation-artifacts/21-12-attestation-and-credential-defects.md` — new, measured story only.

## §A6 independent review artifact — 2026-11-21 (GLM-5.3-flash, static)

**Review scope.** GLM-5.3-flash is a non-Claude model family, independent of the Claude-authored implementation. The review was read-only and static: the reviewer ran no builds or tests. The D-E20-6 B security review requirement is satisfied for this static pass only. The orchestrator ran the suites listed below locally afterwards; there are no hosted receipts.

**Implementation on branch.** The kernel IO adapter uses a `ureq::Agent` with `redirects(0)`. `http_get` refuses non-2xx responses, and POST exchange returns any 3xx as status only, before reading the body. GDPR forget forward completion uses a `principal.forget.intent` journal frame; `complete_outstanding_forgets` runs at boot, and the next forget also attempts recovery. ADR-044 now describes the cascade as convergent by forward completion, not atomic.

| Finding | Fixed after static review |
|---|---|
| H1 | The journal redacted a principal ID consisting of 32 consecutive hex characters, and recovery used the redaction marker. The intent now stores the ID as grouped hex in `principal_hex`; completion closes the specific intent via `completed_from_intent`. |
| H2 | A legal hold is refused, with nothing recorded, while an earlier erasure for the same principal remains unfinished. |
| M4 | Malformed journal rows are isolated individually so they do not block recovery for other principals. |
| M5 | A live retry reuses its open intent and preserves the original reason; completion is idempotent by intent ID. |
| M6 | POST exchange returns 3xx as status only, before reading the response body. |
| M7 | Door replay reruns recovery when a later open intent exists and matches retained kinds exactly. Daemon-level replay coverage remains open. |

**Recorded deferrals — open, not closed under the standing no-more-review-rounds policy.**

- **M3:** The partial indexes incur an O(history) scan under the log mutex.
- **M8:** The exempt intent frame retains the free-form reason; boot diagnostics go to stderr; recovered regeneration requests are not dispatched at boot.
- **M7:** No daemon-level replay test.
- **Proof not run:** The three D-21-12-01 control-removal plants (receipt predicate, redirect refusal, interrupted forget) and the AC1 witness re-run were not run.

**Local results — orchestrator-run.**

| Suite or target | Result |
|---|---|
| `forget_forward_completion_21_12` | Green |
| `io_redaction_and_status_20_4` | Green |
| `forget_cascade_18_6` | 22/22 |
| `maos-kernel-core --lib` | Green |
| `maos-iac` | Green |
| `maos-audit` `gdpr_cascade_corpus_test` | 2/2 |
| `maos-bin` `forget_floors_18_6` | 1/1 |
| `maos-journey-test` `journey_forget_18_6` | 2/2 after rebuilding fixtures |
| `maos-providers` | Green |

**Kernel accounting.** Kernel production measured 20,517 lines after formatting. The orchestrator raised the ceiling from 20,429 by adjudication under the story’s +55…+165 kernel forecast. The kernel pin was refreshed via `scripts/repin.py`, and the kernel-surface baseline hash for `IoSubsystemAdapter` was refreshed. The ADR-066 surface refresh was not ratified.

No run IDs, implementation commit IDs, or gate output were supplied for this record.

## Change Log

- 2026-11-20: created at `de16fa0a`. Recorded the three clauses as positive defensive properties (receipt re-run, no-redirect IO adapter, forward-completion forget), their base inventory and real preconditions (the `http_get` 3xx-as-success gap, the intent-scrub exclusion, the fault-injection seam), the testable AC1–AC4 contract with named tests, four open D-rows, the kernel and kloc forecasts with their method, and the parent-owned cite corrections (including one unsupported evidence claim). Status: backlog pending rulings.
- 2026-11-21: Recorded operator preflight approval of D-21-12-01…04 (A/A/B/A) and aligned readiness and acceptance criteria with those rulings. Status remains `backlog`.
- 2026-11-21: Implementation landed on branch with local evidence only. Recorded the independent static review, fixes, open deferrals and outstanding proof; no hosted receipts. Status: `in-progress`.
