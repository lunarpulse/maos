---
story_id: '17.3d'
story_key: 17-3d-wasm-log-recall
created: 2026-10-17
context_commit: 39cd7966
baseline_commit: 39cd7966a354917b2a9cf805faa2462cb05c9677
security_sensitive: true
---

# Story 17.3d: `log.recall` for WASM Spirits, mediated in-band on the runner's stdio

Status: done

<!-- FLAG-Winston charter ratified 2026-10-18 (17-3d-flag-winston-decision.md, D-17-3d-FW-01…10: FW-07 A, FW-08 scaffold, FW-09 A), then re-ruled the same day at the party-mode preflight (D-17-3d-PF-R1…R4: in-band stdio transport, dormant seam deleted, quota window → 17-9, cross-incarnation recall a non-goal). Every ruling is materialized into this one contract (17-1 FW-05 precedent). -->

## Story

As a third-party Spirit author shipping a `wasm-component` (TypeScript) Spirit,
I want a versioned `recall` host import that returns the Transparency Log frames my Spirit emitted in this daemon run and fetches their payloads, scoped and mediated by the daemon exactly as the native form is,
so that my Spirit can ground its own distillation in its own history without ever holding a path, a pid, a descriptor or any authority it could forge — and every recall, fetch and refusal is an attributed, token-bearing audit row.

**Contract:** epic-17 §17-3d (AC1–AC5) as amended by **§R13 (R17-141…R17-156)** and **§R14 (R17-157…R17-164)**, R8/R17-66, ADR-060 §Amendment *in-band* (`docs/adr/ADR-060-spirit-forms-by-trust-tier.md:230-268`, which supersedes the 2026-09-26 transport at `:185-228`), ADR-013 (`docs/adr/ADR-013-log-recall-kernel-mediated-spirit-abi-capability.md:11`, incarnation note `:25-27`). ◆ FLAG-Winston and security-sensitive: the non-author §A6 review net (Blind + Edge + Acceptance + Test-Infra + runtime execution) is non-degradable. The charter `17-3d-flag-winston-decision.md` was ratified 2026-10-18 and re-ruled at the preflight the same day: kernel = the `CapabilitiesRequired` literal plus a deletion-only removal of the dormant scheduler recall seam, **no ceiling raise**, exact lower figures at landing; out-of-kernel rows exact at landing as measured requests; FW-07 **A** (FR29's participant/consent half → `17-8-participant-recall-and-consent-envelopes`); FW-08 — the TS scaffold carries the recall demo and grants; FW-09 **A** — one mediation path for both forms; recall travels in-band on the runner's ADR-032 stdio (no descriptor, no new syscall, no `unsafe`).

## Current-tree corrections — read before implementing

Every epic cite was re-measured at `39cd7966` by four read-only scouts and one isolated prototype, then re-checked by four preflight reviewers. OLD→NEW rows: epic §R13 and §R14.

| Epic premise | Current fact and consequence |
|---|---|
| AC1: fetch "another Spirit's frame id" | `maos run … --once` feeds exactly one inbound frame: the **operator seed**, built at `crates/maos-bin/src/spirit_session.rs:102-128` and delivered with `deliver_typed(frame, 0, None)` at `:356-361` — its TL emitter pid is **0**. No second Spirit exists in that run. The deterministic foreign vector is `fetch(seed.frame-id)` → `ScopeViolation` (`crates/maos-iac/src/adapter/log_recall.rs:481-487`). Called "another emitter's frame" in this story. Under 17-8's participant scope the seed becomes fetchable (R17-162). |
| "Recalls its own frames" (FR29 *participant*) | The adapter is **emitter-only** (`log_recall.rs:5-12`, `:313-314`, `:481-487`); ADR-013 defines participant as sender/receiver/addressed-by-role. 17-3d delivers form parity with the native path; the participant half is owned by `17-8-participant-recall-and-consent-envelopes` (D-17-3d-FW-07 A). |
| Binding the scheduler `spirit_pid` makes recall self-scoped (R17-23) | **Not across boots.** The TL is persistent (`transparency_log.rs:511-535`, opened on disk at `crates/maos-bin/src/main.rs:2081-2098`); the pid allocator restarts at 1 in every process (`crates/maos-kernel-core/src/scheduler/scheduler_loop.rs:29-33`); `FrameFilter` has no boot field (`transparency_log.rs:213-236`) and fetch compares only the pid (`log_recall.rs:481`). Today a Spirit at pid N recalls and fetches **another Spirit's** rows written at pid N in an earlier run — the native Researcher included (`spirits/researcher/src/lib.rs:663-670`). Fixed at origin by AC3.4; recalling an earlier incarnation is a ratified non-goal (ADR-013 note). |
| Check `Scope::LogRecall` against the grant at `cap_policy/mod.rs:185-188` | `:185-188` is the approval-class arm. The real gate is operator denies → unknown pid → declared scope (`crates/maos-kernel-core/src/capability/cap_policy/mod.rs:135-172`), reached through `issue_with_mediation` (`capability/mod.rs:180-210`). **No manifest syntax grants `LogRecall` or `LogFetch`** (`crates/maos-manifest/src/manifest.rs:602-608`, `:676-688`, `:796-838`); `LogFetch` is a distinct scope (`crates/maos-domain/src/invariants/i1.rs:82-85`). AC3.5 adds the declaration. |
| The recall rows "appear" in `maos audit query --spirit` | The adapter writes **tokenless** kind-7 rows (`log_recall.rs:370-377`, `:453-460`), which the FR4 `--spirit` view rejects (`crates/maos-audit/src/lib.rs:605-610`, `:901-904`; classifier `fr4_classifier.rs:516-518`; pinned by `crates/maos-audit/tests/fr4_classifier_16_2.rs:100-111`). A failed fetch writes **no** outcome row. The plain view prints `call_type` and `intent`, never payload (`lib.rs:880-916`). AC1 fixes the exact rows; AC3.3 makes the call rows token-bearing. |
| `recall-error::scope-violation` exists | It does not. The domain error's Display is `E_SCOPE_VIOLATION …` (`crates/maos-domain/src/log_recall.rs:340-345`). The WIT variant is new (AC2.2). |
| Recall pages are correct | **Not with the caller's own audit rows present.** `query_page` applies the SQL `LIMIT` (`log_recall.rs:320-325`) and only then drops kind-7 rows and intent mismatches in memory (`:338-344`). Every recall/fetch writes a kind-7 row at the caller's pid, so a Spirit that recalls often stops seeing its frames. The keyset cursor is already in SQL (`transparency_log.rs:1506-1511`). Fixed at origin by AC3.4. |
| Recall returns "frames the Spirit emitted" | It returns **every non-kind-7 row at the pid** (`log_recall.rs:338-344`) and `fetch` checks only the pid (`:481`): 17-3c's `spirit.frame.authorization` rows (kind 28, `spirit_session.rs:920-929`), this story's refusal receipts (kind 28, payload with `owner_pid`), the SSO issuer's `identity.asserted` rows (kind 30, `worker_spawn.rs:374-398`) — and kind 30 is unknown to `FrameKind::from_i64` (`transparency_log.rs:134-168`), so readers relabel it `TaskAssign` (`:1564-1572`). AC3.4's disclosure allow-list closes it (PF-02). |
| The ADR-060 §Amendment (2026-09-26) transport: inherited descriptor, `sendto`/`recvfrom`, CLOEXEC clear | **Superseded 2026-10-18** (ADR-060 `:230-268`). Since 17-3c the kernel bridge is byte-transparent: maos-bin owns the runner's stdin writer (`crates/maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs:836-841`) and reads every stdout record (`poll_frame`, `:849-860`; session loop `spirit_session.rs:330-457`), so recall rides the ADR-032 stdio in-band at zero kernel lines. The T2 allow-list is unchanged. The creation prototype measured the superseded transport and stays as evidence only (§Measured prototype). |
| AC4: lossless frames let recall honor consent | Lossless frames reach the guest (`crates/maos-wasm-host/src/frame_bridge.rs:546-578`), but the TL never persists the envelope (`crates/maos-iac/src/adapter.rs:496-499`, `:584-595`; `transparency_log.rs:194-209`, `:291-310`), the adapter's check is a comment (`log_recall.rs:489-497`), and guest-emitted frames are restamped with `consent_envelope = None` (`spirit_session.rs:948-960`). Under emitter scope **no frame a WASM Spirit can recall carries an envelope**: AC4 as written has no positive vector. Ruled (D-17-3d-FW-07 A): the safety half here, the positive vector in `17-8-participant-recall-and-consent-envelopes`. |
| Q5 already proves `@2.0` guests on `@2.1` hosts | Q5 measured `@1.0/@1.1/@1.2` on Wasmtime 46.0.3 (`17-3a-wasm-recall-and-componentize-spike.md:262`, `:338-350`). HEAD locks Wasmtime **48.0.6** (`Cargo.lock:6115-6118`). The runner's guard checks only the `frames` major (`crates/maos-wasm-host/src/runner.rs:325-338`). Fresh proof: AC2.4. |
| `wit_corpus.rs` is an xtask test; the `[toolchain]` table must be added | `crates/maos-wasm-host/tests/wit_corpus.rs:221-261` (20 records today, `:179-185`); `[toolchain]` already exists (`tests/fixtures/wasm/equiv-fixtures.provenance.toml:14-18`) — the re-issue updates it. |
| Port the 17-3a WIT/host | Spike WIT uses a closed 30-case label enum (`spikes/story-17-3a-wasm-recall/wit-1.1/spirit.wit:218-252`), the mapper collapses unknown labels to `TelemetryEvent` (`host/src/spike_runner.rs.txt:230-243`), the launcher binds the **OS** pid (`t2-ladder/relay_fd.py:60-64`), framing is u32+JSON with a partial-header-as-EOF bug (`host/src/protocol.rs.txt:41-46`). Reuse shapes, not code; the spike's descriptor transport is superseded. |
| 17-3c pins (25293 …) | HEAD: kernel physical **26311** (`xtask/kernel-core-baseline.toml:505`), kernel tokei **20545** (`xtask/kloc.toml:262`), `maos-manifest` 4474 (`:408`), `maos-iac` 7100 (`:401`), `maos-audit` 7351 (`:404`), `maos-domain` 9615 (`:415`), `maos-bin` 25917 (`:503`), `maos-frame-codec` 249 (`:662`), `maos-wasm-host` 1577 (`:688`), `xtask` 44930 (`:336`), `_aggregate_hardfail` 176234 (`:698`). `spirits/` is outside kloc (`kloc.toml:2`). |
| Exit line 2b is owned | Its command existed (epic `:39` after §R14) but the provenance block had no `- Line 2b` bullet, so `check-exit-commands` could not attribute an owed token to 17-3d (`xtask/src/check_exit_commands.rs:777-837`). Added at creation (epic `:48`, §R13 R17-142). |
| Kernel lines are the transport | Kernel lines are now: the `log` field in the `CapabilitiesRequired` literal (`scheduler_loop.rs:330`, +1, measured in variant B; the struct has no `Default` and is not `#[non_exhaustive]`, `manifest.rs:602-608`, so 10 literals change, one in kernel production) and the deletion of the dormant `log_recall` seam (`scheduler_loop.rs:110,127-129` → `hook_dispatch.rs:60,79,117-120,167-169` → `kernel_ctx.rs:27,48,85-88`; production fills it at `main.rs:2730`, nothing reads it). |

## Acceptance Criteria

### AC1 — command: exit line 2b, exact rows

**Given** a `wasm-host` release build (`cargo build --locked --release -p maos-bin -p maos-wasm-host --features maos-bin/wasm-host --bin maos --bin maos-wasm-runner`) and the TS example componentized in place (`npm ci && npm run build && npm run componentize` in `examples/example-spirit-ts`), run inside the delegated envelope exit line 2 already uses (`.github/workflows/discipline.yml:1957-1974`),
**when** exit line 2 (`target/release/maos run examples/example-spirit-ts/manifest.toml --once`) and then exit line 2b (`maos audit query --spirit example-spirit-ts --format plain`) run against the same audit store,
**then**:

1. Line 2b exits 0 (no `Fr4SchemaViolation`). For the TS Spirit's `(boot_nonce, spirit_pid)` in the latest boot, the recall-operation rows are exactly these three, as an **ordered subsequence** before that turn's 17-3c rows (`spirit.frame.authorization`, `standard`), which stay unchanged. The governed issuer's own `identity.asserted` rows (kind 30, written when SSO is configured, `crates/maos-bin/src/worker_spawn.rs:374-398`) may interleave:

   | # | `call_type` | `intent` | Writer | Token |
   |---|---|---|---|---|
   | R1 | `capability.invocation` | `log.recall` | `LogRecallAdapter`, attributed path (AC3.3) | minted at the Spirit's pid for `Scope::LogRecall` |
   | R2 | `capability.invocation` | `log.fetch` | same | minted for `Scope::LogFetch` |
   | R3 | `governance.event` | `log.fetch.refused:scope-violation` | the recall mediation (AC3.3) | none (kind 28, non-call) |

2. The scaffold's `handleFrame(seed)` produces them: `recall({ limit: 16 })` succeeds, then `fetch(seed.frameId)` returns the `scope-violation` error (the seed's emitter is pid 0), then the frame is echoed as today. The scaffold treats a successful `fetch` and `scope-violation` both as normal outcomes and rethrows any other error, so it stays correct when 17-8 makes received frames fetchable (PF-09).
3. The exit job asserts at the TS pid: R1→R2→R3 as an ordered subsequence, the literal `log.fetch.refused:scope-violation`, no other `log.recall` / `log.fetch` / `log.*.refused:*` / `recall.channel.closed:*` row, and exit 0. The line-2b step is **proven red once** (rule 11): the scaffold's `fetch` removed on a `ci/17-3d-*` branch → R2/R3 missing → job red; restored → green. The multi-turn suite asserts the same ordering with SSO configured.
4. "Recalls its own frames" is proven by content, not by the exit line: the multi-turn suite (T8 `recall_17_3d`) shows turn 2's `recall` returning turn 1's kernel-minted frame id, kind and intent, and `fetch` of that id returning its stored payload.

### AC2 — the contract grows additively (`maos:spirit@2.1.0`)

1. `wit/spirit.wit` becomes `package maos:spirit@2.1.0;`. `interface frames` keeps every type byte-for-byte. A new `interface recall` (shape in §WIT `@2.1.0`) is imported by `world spirit` (`import recall;`). The frame-kind label in recall records is an open `string` (R17-43): the exact `serde` name of `maos_domain::log_recall::FrameKindLabel` (e.g. `TaskAssign`) — no second spelling table; an unknown filter label is `invalid-request`, never "no filter" and never a fallback kind.
2. `recall-error` is final for 2.x: `scope-violation`, `frame-not-found`, `invalid-request(string)`, `limit-exceeded(u32)`, `capability-denied`, `consent-withheld`, `reply-too-large(u64)`, `unavailable`. A closed variant cannot grow without breaking both directions (17-3a Q5), so `consent-withheld` is reserved now (FW-07) and `reply-too-large` carries the reply byte bound (PF-03). Errors never carry an owner pid, a token, a path or a storage message.
3. Ratification: one `[[ratification]]` in `xtask/abi-ratifications.toml`, `proposal_id = "17-3d-AC2-wit-spirit-2.1.0"`, `status = "ratified"`, `adr_ref = "ADR-013,ADR-031,ADR-060"`, `covered_changes` = `"maos:spirit@2.1.0"` plus the new interface/records/variant names. The version-binding test requires **exactly one** covering entry (`crates/maos-wasm-host/tests/wit_corpus.rs:221-261`). Entry-shape precedent: `xtask/abi-ratifications.toml:85-104`.
4. Compatibility (Q5, fresh at HEAD): a `@2.0.0` guest built **before** the root WIT edit from a frozen copy of today's WIT (`crates/maos-wasm-host/guests/compat-echo-2-0/wit/spirit.wit` = byte copy of `wit/spirit.wit` at `39cd7966`) runs on the `@2.1` runner — spawned like production, through `spawn_sandboxed` under a T2 spec with `FsRead` on the component (pattern `crates/maos-wasm-host/tests/t2_sandbox_kill.rs:308-388`): exit 0, frame round trip byte-identical to the `@2.0` runner, and no recall record on its stdout. Built at test time like the 17-3c fixtures (no new committed blob, no fifth provenance row). The reverse (a `@2.1` recall guest on the pre-change runner fails at instantiation, typed exit) is recorded once as dev evidence, not a permanent test.
5. R8 (R17-66): the 4 provenance-pinned components (`tests/fixtures/wasm/{echo_spirit,equiv_identity_spirit,equiv_divergent_spirit,equiv_cosmetic_spirit}_component.wasm`) are rebuilt against `@2.1.0` with the `17-3b` recipe and their `[toolchain]`, `wasm_sha256` and `source_sha256` re-pinned (`check-equiv-fixture-provenance` green); `wit_corpus.rs` record and field-count rows updated (20 → 20 + the new records); root/template/example WIT stay byte-identical (`wit_copies_17_3b.rs:3-14`); `templates-regen --check` green.

### AC3 — transport, identity, grant and correctness at origin (ADR-060 §Amendment in-band)

1. **In-band records on the ADR-032 stdio (PF-01).** Recall adds no descriptor, socket, thread or syscall.
   - **Runner side.** `HostState` owns the runner's stdin reader and stdout writer (today locals of `run`, `crates/maos-wasm-host/src/runner.rs:262-265`) as **one persistent** `BufReader<Stdin>` / `BufWriter<Stdout>` — a second buffer on stdin would prefetch and drop bytes between the import and the frame loop — so the frame loop (`:356-414`) and the recall import share them through the `Store`. A guest call writes **one** request record (`maos_frame_codec::write_frame`; body = one CBOR data item tagged `RECALL_REQUEST_TAG` with `ciborium::tag::Required`), flushes, then reads **exactly one** stdin record, which must be a reply tagged `RECALL_REPLY_TAG`. Recall is callable only inside `handle-frame`; before Ready and during `on-shutdown` (stdin already closed, `:298`) it returns `unavailable` without I/O. The guest cannot forge a record: its WASI stdout is redirected to stderr (`crates/maos-wasm-host/src/host_state.rs:28`), and frames reach stdout only after the export returns (`runner.rs:397-413`). Measured by the in-band prototype (§In-band prototype).
   - **maos-bin side.** In the stdout arm (`spirit_session.rs:436-457`), a body whose first data item is a CBOR tag (major type 6) is a recall request and is dispatched **before** the frame counter and `decode_cbor::<IacFrame>`; untagged bodies keep today's path byte-for-byte. The reply is sent through the same writer channel that carries inbound frames (`:332-341`), so it is ordered on stdin; inbound frames are written only between exports (`:358`), so during an export the reply is the only stdin record. A tagged body outside an active export, an unknown tag, an undecodable request, or more than `MAX_RECALLS_PER_TURN` requests in one turn (named constant beside `MAX_FRAMES_PER_TURN`, `:43`) writes one kind-28 `recall.channel.closed:protocol-violation` receipt (payload names the reason) and ends the session with `SessionError::Protocol`, like every other stdout protocol violation.
   - **Bounds.** Mediation runs inline in the session loop; the whole call is bounded by the active-turn deadline (`:347-356`, `:385`), checked when the loop resumes; a single mediation step is bounded by Transparency-Log liveness exactly like 17-3c's inline authorization write (`:920-929`; I2 halts the kernel on a failed log write). No socket timeout, cancel flag or join exists to get wrong.
   - **Portability.** The recall client and `recall_wire` are std-only and platform-neutral; their wire tests run on every CI OS. The WASM session itself still requires T2 (`spirit_session.rs:255-257`), which is Linux-only.
2. **Kernel (FW-01 as re-ruled; PF-10).** No sandbox or security change: `SandboxSpec`, `spawn_sandboxed`, the T2 allow-list and the `pre_exec` block are untouched (their `kernel-core-baseline.toml` digests stay byte-identical). Kernel edits are exactly: the `log` field in the `CapabilitiesRequired` literal (`scheduler/scheduler_loop.rs:330`) and the deletion of the dormant recall seam — `KernelCtx.log_recall` and `with_log_recall` (`scheduler/kernel_ctx.rs:27,48,85-88`), `HookDispatcher`'s field, default, builder and propagation (`scheduler/hook_dispatch.rs:60,79,117-120,167-169`) and `SpiritSchedulerAdapter::new`'s `log_recall` parameter and use (`scheduler_loop.rs:110,127-129`). Every `SpiritSchedulerAdapter::new` caller migrates (production `crates/maos-bin/src/main.rs:2722-2736`; maos-bin tests, kernel tests and the two kernel benches), and the `kernel_ctx_round_trip.rs:76` assertion is deleted with the field. Any other kernel file returns to the operator as a charter request.
3. **Identity and mediation.** The session already holds `admitted.pid`, the admitted `spirit_id` and the TL boot nonce; the wire carries no pid and no identity. Every request is decoded → validated → minted through `issue_enterprise_governed_capability` (`crates/maos-bin/src/worker_spawn.rs:333-400`: SSO principal → enterprise PDP → `issue_with_mediation` incl. quota and operator denies; all in-process) for `Scope::LogRecall` (recall) or `Scope::LogFetch` (fetch) at the bound pid → passed to the adapter's **attributed** entry point, which writes the kind-7 call row with that token id (`capability_token_column`, `transparency_log.rs:283-287`) **before** any data moves → result → token revoked. A refused mint (policy, operator deny or exhausted quota) returns `capability-denied`, writes only the `log.<op>.refused:capability-denied` receipt and never calls the adapter. **Every operation that returns `Err(recall-error)` writes exactly one kind-28 receipt** `log.<op>.refused:<code>` (codes = the WIT case names) at the bound pid with the admitted `spirit_id` as sender (pattern `spirit_session.rs:920-929`); successful results — including an empty page or an empty stored payload — write none. The adapter has exactly one invocation-row writer per operation (no duplicate mediation row, no leftover tokenless row). These rules govern recall invocation/refusal rows only; the issuer's `identity.asserted` provenance rows are unchanged. Native and WASM share one error mapping (§Error mapping); `LogRecallError` gains `CapabilityDenied` so a mint refusal is never encoded as `Storage` (PF-07).
4. **Incarnation scope, disclosure and correct pages, at origin (rule 10(a); PF-02…PF-06).**
   - `FrameFilter` gains `boot_nonce`, a raw-kind allow-list and an exact-intent predicate, all applied in SQL **before** `LIMIT` (`transparency_log.rs:1466-1535`); the adapter's local `recall`/`fetch` (port and attributed paths) restrict to the adapter's own boot nonce; a same-pid row from another boot is omitted by `recall` and is `ScopeViolation` for `fetch`.
   - **Disclosure allow-list (one function, used by recall and fetch):** a row is Spirit-disclosable only if its **raw** SQLite kind is a bus frame (0–6: `TaskAssign` … `Retract`) or `McpInvocation` (18, the Researcher's walk, `spirits/researcher/src/lib.rs:440-441`). Every other kind — call rows (7), sandbox blocks (8), governance events (28, including refusal receipts and `spirit.frame.authorization`), `identity.asserted` (30) and any unknown discriminant — is never returned and never decoded through the `TaskAssign` fallback on this path; `fetch` of such a row returns `ScopeViolation`, indistinguishable from a foreign frame. A filter naming a valid but non-disclosable label returns an empty page.
   - **Metadata-only recall:** the recall query projects `length(payload_redacted)` instead of the payload, so a page never materializes payloads (`:1473-1477` loads them today); `fetch` checks the stored length before reading the blob.
   - **Representable bounds:** `since-ns`, `until-ns` and the cursor timestamp above `i64::MAX` are `invalid-request` before minting (the SQL binds use `as i64`, `:1498-1511`); a reversed representable interval is an empty page.
   - **Index:** an idempotent `idx_tlog_boot_pid_ts` on `(boot_nonce, spirit_pid, timestamp_ns, frame_id)` is created at open beside `idx_tlog_correlation` (`:532-535`).
   - `query_page`'s cross-wall caller (`crates/maos-bin/src/cross_wall_log_read.rs:42-68`) keeps today's boot semantics (no local boot applied to a remote artifact) and inherits the disclosure allow-list. Proven for the native Researcher port and for WASM.
5. **Grant syntax (schema 5, no bump — 17-3c precedent `xtask/abi-ratifications.toml:106-124`).** `[capabilities.required.log]` with `recall = <bool>` and `fetch = <bool>` (both default `false`, unknown keys refused, dropped below schema 5 like `iac`, `manifest.rs:660-674`); `capabilities_required_to_scopes` pushes `Scope::LogRecall` / `Scope::LogFetch` independently. Ratified entry `17-3d-log-capability` covering `LogCapabilities`, `RawLogCapabilities`, `CapabilitiesRequired`, `RawCapabilitiesRequired`. `POST_V1_SCHEMA_SECTIONS` (`manifest.rs:337`) and `xtask/src/check_manifest_schema_version.rs:37-46` gain `capabilities.required.log`. The TS template and example declare both.
6. **Reply budget (PF-03).** Wire ids and payloads are CBOR **byte strings** (a `serialize_bytes`/visitor newtype in `recall_wire`; no new dependency), because `codec` charges 1 KiB of its 16 MiB expansion budget per CBOR node (`crates/maos-wasm-host/src/codec.rs:29-34`, `:48-50`) and serde writes `Vec<u8>`/`[u8; 16]` as arrays (measured: a 1 MiB payload as `Vec<u8>` fails `container expansion budget exceeded`; as a byte string it round-trips). Every reply must pass both `MAX_FRAME_BYTES` (`crates/maos-frame-codec/src/lib.rs:7`) and the expansion budget: a recall page is the longest prefix of matching entries that fits — searched downward from the full page, not bisected, because the encoded size need not be monotonic — with `next-cursor` at its last entry (entries are never skipped); a `fetch` whose reply cannot fit, or a first page entry that alone cannot fit, returns `reply-too-large(<bound>)` with its receipt, never a truncated payload and never a broken channel.
7. **Lifecycle (runner side).** EOF while a request is outstanding returns `unavailable` and sets a closed latch (no further recall I/O); the export finishes and the frame loop then sees EOF and exits as today (exit 0). An unexpected, untagged, oversize or undecodable reply returns `unavailable`, latches the client broken (every later call returns `unavailable` without I/O), lets the export finish — its frames and TurnComplete are still written — and then makes the runner exit with a typed `recall protocol desync` error without reading further stdin. A `@2.0` guest never touches the channel. Tokens are minted per call and revoked after it; the bridge finalizer's revoke-all for the pid still runs (`runtime.rs:1067`, `spirit_session.rs:478-482`).
8. **Deployment docs.** `docs-site/docs/deploy/topology.md` (WASM T2 section, `:112`, `:135-147`) and its KO twin state that recall adds no descriptor, socket or syscall to T2: it rides the runner's stdio and every call is minted and audited by the daemon. No deployment condition is introduced.

### AC4 — FR29's consent clause: the safety half (D-17-3d-FW-07 A)

1. **Safety vector (17-3d).** In the multi-turn suite, an inbound frame carrying a `ConsentEnvelope` (granter = a peer, `valid_until_ns` both past and future) is delivered to the WASM Spirit, which sees the envelope losslessly; `recall` never returns it and `fetch(its id)` returns `scope-violation` with its receipt. A guest-emitted frame carrying a guest-asserted envelope is stored with none (`spirit_session.rs:948-960`) and confers nothing on recall. Labeled honestly in the test and docs: **withheld by emitter scope; no consent evaluation exists yet**. 17-8 replaces this vector when received frames become participant-visible (R17-162).
2. **Reserved surface.** `recall-error::consent-withheld` ships in `@2.1.0` so `17-8-participant-recall-and-consent-envelopes` adds evaluation without a WIT version change.
3. **Routed.** Participant recall (receiver/addressed-by-role, ADR-013) with persisted consent envelopes and their evaluation, for **both** forms, is owned by `17-8-participant-recall-and-consent-envelopes` (epic §17-8, R17-156); it is not built here, and this story claims no FR29 participant/consent completion.

### AC5 — no unbacked latency claim

No recall latency number appears in this story's code, tests, docs, commit messages or evidence summaries; the 17-3a figures (p50 0.71–0.75 ms, `_bmad-output/implementation-artifacts/17-3a-wasm-recall-and-componentize-spike.md:261`) measured a superseded transport, are historical and are never cited as a property. `20-7-stable-runner-perf-gates` measures the production route (host import → stdio record → recall mediation → adapter) on its 30-day audit fixture, reported separately from CLI audit-query latency; that leg is filed on 20-7 AC4 by R17-154 (`_bmad-output/planning-artifacts/epics/epic-20-ship-it-w5.md:188-189`).

### Delivery gate — measured authorization and the re-pin set

Kernel edits stay inside the re-ruled FW-01 locations (`scheduler/scheduler_loop.rs`; deletion-only in `scheduler/hook_dispatch.rs` and `scheduler/kernel_ctx.rs`); any other kernel file returns as a charter request. Measure the complete formatted implementation (`cargo fmt --all`; tokei 14.0.0; `cargo run -p xtask -- kloc-check --json`). The kernel has **no raise** (≤ 26311 `src_lines`, ≤ 20545 tokei) and re-pins the exact lower figure; any out-of-kernel figure above its pin returns to the operator **before** landing (FW-03). Re-pin in one commit, in this order:

1. `xtask/kernel-api-classes.toml` — remove rows for deleted public symbols (`KernelCtx::with_log_recall`, `HookDispatcher::with_log_recall`, if classified; scheduler rows near `:58`). No new public kernel symbol is expected; `CapabilitiesRequired` stays `universal-arithmetic` (`:225`, `:235`). If `LogCapabilities` surfaces through `security/manifest.rs`'s `pub use maos_manifest::*` in the walker output, classify it like `CapabilitiesRequired`.
2. `docs/ci-baselines/kernel-surface-v0.1-beta.json` — regenerate and enumerate the actual removed/added pairs (`KernelCtx` and `HookDispatcher` entries near `:870-883`, `SpiritSchedulerAdapter::new`). `SandboxSpec` (`:941-943`) does not change. Re-export entries (`:1921-1923`, `:1751-1753`) hash the `pub use` item and stay unchanged unless those statements change (`xtask/src/check_service_boundary.rs:393-396`, `:531-535`, `:564-575`).
3. `xtask/kernel-core-baseline.toml` — `src_lines` stays on line 505 (today 26311; exact lower), per-file digests (`scheduler_loop.rs`, `hook_dispatch.rs`, `kernel_ctx.rs`; `linux.rs`, `sandbox/mod.rs`, `security/mod.rs` unchanged), `set_hash`, HISTORY.
4. `xtask/tests/kernel_pin_content_hash_16_0.rs` — the four live `26311` literals (`:820`, `:838`, `:847`, `:858`); `PINNED_ENTRIES` (`:34`) unchanged (no file added or removed). Never touch the historical falsifier constants (`:24-34`) or the synthetic `24474` vector (`:754`).
5. `xtask/kloc.toml` — kernel `:262` (exact lower), `maos-manifest` `:408`, `maos-iac` `:401`, `maos-bin` `:503`, `maos-wasm-host` `:688`, `maos-domain` `:415` (`CapabilityDenied`), `xtask` `:336` (schema inventory row), `maos-audit` `:404` only if touched, `_aggregate_hardfail` `:698` (measured + 1).
6. `xtask/tests/recovery_lane_ceiling_rule.rs:268` — `RATIFIED_AT_EASING` moves with the kernel kloc pin.
7. `xtask/abi-ratifications.toml` — `17-3d-AC2-wit-spirit-2.1.0` and `17-3d-log-capability`.

Then run `check-kernel-baseline`, `check-service-boundary`, `check-empty-kernel`, `check-manifest-schema-version`, `check-equiv-fixture-provenance`, `templates-regen --check`, `kloc-check --json`, `check-exit-commands` (with `target/debug/{maos,maosctl}` built — it reads only `target/debug`, `check_exit_commands.rs:1549-1615`; this host sets `CARGO_TARGET_DIR=/mnt/build/cargo`). No new `Arc`/`OnceLock`/`Mutex` kernel holder and no `unsafe` anywhere (kernel, runner or maos-bin) is expected; if a holder appears it needs `#[maos_attrs::i9_exempt]` + register entry + the ≥ 2-signature rule (17-7's sole-founder waiver D-17-7-FW-10 does **not** carry over).

## Tasks / Subtasks

- [x] **T0 — Preflight at the dev HEAD** (all ACs)
  - [x] Re-read this file's cites at the dev HEAD; any drift → note in Debug Log, re-measure the pins in §Delivery gate.
  - [x] Build the frozen `@2.0` guest (`guests/compat-echo-2-0`, WIT copied from `39cd7966`) and record its round trip on the **unchanged** runner **before** editing `wit/spirit.wit` (AC2.4 baseline).
- [x] **T1 — Kernel: literal and seam deletion** (AC3.2; FW-01, FW-02 re-ruled; PF-10)
  - [x] `scheduler/scheduler_loop.rs:330`: the new `log` field in the `CapabilitiesRequired` literal.
  - [x] Delete `KernelCtx.log_recall` + `with_log_recall` (`kernel_ctx.rs:27,48,85-88`), `HookDispatcher`'s `log_recall` field/default/builder/propagation (`hook_dispatch.rs:60,79,117-120,167-169`) and `SpiritSchedulerAdapter::new`'s parameter and use (`scheduler_loop.rs:110,127-129`).
  - [x] Migrate every `SpiritSchedulerAdapter::new` caller (`main.rs:2722-2736`; `crates/maos-bin/tests/{shell_host_16_2,two_host_delegation_2b,worker_crash_corpus_16_3,worker_hang_corpus_16_3,worker_supervision_16_3,spawned_scope_17_3c}.rs`, `worker_terminal_facts_17_7/{unload_scenes,supervisor_scenes}.rs`; kernel tests incl. `tests/common/upgrade.rs`; `benches/{hot_swap_latency,revocation_propagation_p99}.rs`; re-run the search — the list is the creation-time grep) and delete `kernel_ctx_round_trip.rs:76`.
  - [x] Touch no file under `security/`; no `unsafe` added.
- [x] **T2 — Grant declaration** (AC3.5; FW-05)
  - [x] `crates/maos-manifest/src/manifest.rs`: `LogCapabilities { recall, fetch }` (`Debug, Clone, Default, PartialEq, Eq`), `CapabilitiesRequired.log`, `RawLogCapabilities` (`deny_unknown_fields`, `#[serde(default)]` fields), validation `:731-789`, degradation `:660-674`, conversion `:796-838`, `POST_V1_SCHEMA_SECTIONS` `:337`; re-export from `crates/maos-manifest/src/lib.rs`. Variant B is the measured shape of the grant (`17-3d-evidence/prototype-variant-b-increment.diff`).
  - [x] The other 9 `CapabilitiesRequired` literals: `crates/maos-manifest/src/manifest.rs:769`, `crates/maos-bin/src/admission.rs:587`, `crates/maos-bin/src/main.rs:6885`, `:13178`, kernel tests `abi_stability_admit.rs:49`, `operator_posture_ceiling_16_6.rs:85`, `:244`, `provider_switched_journal.rs:35`, `sandbox_admission.rs:59`.
  - [x] `xtask/src/check_manifest_schema_version.rs:37-46` inventory row; `crates/maos-kernel-core/tests/manifest_field_coverage.rs` roster (`:33-77`) + 3 fixtures per field under `crates/maos-kernel-core/tests/fixtures/manifest/capabilities/{well-formed,malformed-rejected,edge-case}/` (`log_recall.toml`, `log_fetch.toml`; fixtures resolve relative to the kernel crate, `manifest_field_coverage.rs:5-8`, `:82-86`; exact count `:238` 15 → 21); parser/degradation assertions `:194-240`.
  - [x] Ratification entry `17-3d-log-capability`.
- [x] **T3 — Adapter at origin** (AC3.3, AC3.4; FW-04, FW-06; PF-02, PF-04…PF-07)
  - [x] `transparency_log.rs`: `FrameFilter` (`:213-236`) gains `boot_nonce: Option<u64>`, `disclosable_kinds_only: bool` (raw `kind IN (0,1,2,3,4,5,6,18)`), `intent: Option<String>`; `query_frames` (`:1466-1535`) binds them as parameterized predicates before `LIMIT`; a metadata-only query variant projects `length(payload_redacted)`; the `idx_tlog_boot_pid_ts` index at open (`:532-535` pattern). Exhaustive `FrameFilter {` literals fail to compile — migrate each; `..Default::default()` sites are untouched.
  - [x] `log_recall.rs`: one `is_spirit_disclosable(raw_kind)` predicate (SQL for recall; the raw column for fetch — never the decoded `FrameKind`); local `recall`/`fetch` use the adapter's boot nonce, the allow-list and the metadata projection; `fetch` refuses a foreign-boot, foreign-pid or non-disclosable row with `ScopeViolation` and checks the stored length before loading the payload; timestamps above `i64::MAX` → `InvalidCursor`/`invalid-request` before any row is written; `query_page` takes the boot as a parameter (`None` for the cross-wall reader). Add `recall_attributed`/`fetch_attributed` taking the caller's `TokenId`; the trait methods delegate to the same body with `None`, so each operation keeps **one** invocation-row writer. Replace the stale "Story 6.3" consent comment (`:489-497`) with a one-line pointer to `17-8-participant-recall-and-consent-envelopes`.
  - [x] `crates/maos-domain/src/log_recall.rs:331-358`: `LogRecallError::CapabilityDenied` (Display without pid/token); migrate exhaustive matches.
  - [x] FR4 writer inventory: the scanner (`crates/maos-audit/tests/fr4_classifier_16_2.rs:706-741`, roots `:766-770`) inventories only kind-7 writers whose token argument is the literal `None` and rejects stale `WRITER_SHAPES` entries (`:852-868`). Reconcile `crates/maos-audit/src/fr4_classifier.rs:516-518` with the post-refactor sites: remove obsolete `recall`/`fetch` entries rather than keeping their names; extend the scanner and its scanner-control tests so the shared optional-token recall/fetch writer is recognized and stays `Call`. Token-bearing rows are `Call` through the classifier's default path (`fr4_classifier.rs:552-556`). Kind-28 refusal writers are not inventoried by this doorbell — prove their emission and classification with dedicated receipt tests. Never classify a genuine recall call as non-call.
  - [x] Tests (in-file or `crates/maos-iac/tests/`): two adapters on one SQLite file with different boot nonces, same pid → no cross-boot recall, fetch = `ScopeViolation`; 2,000 caller kind-7 rows + 2,000 kind-28 receipts + kind-30 rows + 3 frames → `recall(limit 16)` returns exactly the 3 and a correct cursor walk; fetch of a receipt id, an authorization row id and a kind-30 id → `ScopeViolation`, no payload; an unknown raw kind is never returned as `TaskAssign`; `since = u64::MAX` → invalid; a 1,024-entry page over 1 MiB payloads materializes no payload (assert on the projection, not on memory); intent predicate in SQL; attributed rows carry the 16-byte id + 16 zero bytes. Update the existing `log_recall.rs:549-1246` and `maos-kernel-core/tests/log_recall_scope.rs` / `capability_invocation_audit.rs` / `spirits/researcher/tests/recall_walker.rs` expectations only where the boot or disclosure predicate changes them.
- [x] **T4 — WIT `@2.1.0` and R8** (AC2)
  - [x] Root `wit/spirit.wit`: header comment, `package maos:spirit@2.1.0;`, `interface recall`, `import recall;`; byte-copy to `templates/spirit-ts/wit/spirit.wit` and `examples/example-spirit-ts/wit/spirit.wit`.
  - [x] `xtask/abi-ratifications.toml` entry `17-3d-AC2-wit-spirit-2.1.0`; `crates/maos-wasm-host/tests/wit_corpus.rs` record/field rows (`:179-210`).
  - [x] Re-issue the 4 pinned components (`cargo build --locked --release --target wasm32-wasip2 --manifest-path <guest>/Cargo.toml --target-dir <private>` for `guests/echo-spirit`, `guests/equiv-fixture/{wasm-guest,divergent-guest,cosmetic-guest}`; recipe `17-3b-evidence/t3-reissue.log:14-25`), update `equiv-fixtures.provenance.toml` hashes + `[toolchain]`; equivalence suites green.
  - [x] `guests/compat-echo-2-0` (frozen `@2.0.0` WIT) + test: runs on the `@2.1` runner, byte-identical frames, no recall record (AC2.4).
- [x] **T5 — Runner recall client and host import** (AC2, AC3.1, AC3.6, AC3.7; PF-01, PF-03)
  - [x] `crates/maos-wasm-host/src/recall_wire.rs` (new, safe, std-only): `RECALL_REQUEST_TAG`/`RECALL_REPLY_TAG` (private CBOR tag numbers in the first-come-first-served range, documented), request/reply enums wrapped in `ciborium::tag::Required`, byte-string newtypes for ids/payloads, `encode`/`decode` through `crate::codec` (bounded, canonical-by-construction — `BTreeMap`/ordered structs only), `is_tagged(body)` (major type 6 check), and the reply-budget helper (longest fitting page prefix, searched downward; `reply-too-large(bound)`). A partial header is a protocol error, not EOF. No JSON, no new dependency. The generated recall `Host` methods return the WIT `Result` directly; register with `HasSelf<HostState>` (prototype shape, `17-3d-evidence/inband/prototype-inband.diff`).
  - [x] `host_state.rs`: `HostState` owns the stdin reader/stdout writer and a recall client with a permanent broken latch and an `in_export` flag set by the runner around `call_handle_frame`; `impl recall::Host` maps replies to WIT (no `peer_spirit_pid`, no `capability_token`, label = serde name).
  - [x] `runner.rs`: move the reader/writer into `HostState` (`:262-265`) and read/write through `store.data_mut()` in the frame loop (`:356-414`); register the import with `add_to_linker` before `Spirit::instantiate` (`:234-249`); extend the import walk (`:325-338`) to accept `maos:spirit/recall@2.x`; exit with a typed desync error after an export whose recall latched broken (AC3.7). No new flag, no `unsafe`, no `libc`; fuel/limits unchanged.
- [x] **T6 — Mediation in maos-bin** (AC1, AC3.1, AC3.3; FW-06, FW-09; PF-01, PF-07, PF-08)
  - [x] `crates/maos-bin/src/recall_mediation.rs`, gated by `feature = "network"` (`issue_enterprise_governed_capability` and the enterprise runtimes exist only under `network`, `crates/maos-bin/src/lib.rs:34-42`, `:94-95`; the no-network build runs only the air-gap stub, `main.rs:1350-1380`): one `MediatedLogRecall` that validates, mints, calls the attributed adapter path, revokes and writes receipts — the only mediation implementation. It is also the `LogRecallPort` handed to the Researcher (`main.rs:5349-5352`, already `Arc<dyn LogRecallPort>`; FW-09 A); `spirits/researcher/manifest.toml` raises `manifest_schema_version` 4 → 5 (`:31`; log grants are dropped below 5) and declares `[capabilities.required.log] recall = true`, `fetch = true` — verify no other declared section changes meaning at schema 5 (it declares no `iac` section, `:43-77`), replace any comment saying log capabilities are not manifest-declarable, and test admission of the checked-in manifest with both scopes surviving degradation.
  - [x] `spirit_session.rs` (gated like the session, `lib.rs:60-61`): `SessionDeps` (`:56-65`) carries the mediated recall; the stdout arm (`:436-457`) dispatches tagged bodies before the frame counter; replies go through `tx` (`:332-341`); `MAX_RECALLS_PER_TURN` beside `MAX_FRAMES_PER_TURN` (`:43`); protocol violations per AC3.1. `main.rs:5596-5621` passes the composition-root adapter (`:2389-2415`) — never a second adapter.
  - [x] Researcher walk (PF-08): `on_idle` (`spirits/researcher/src/lib.rs:439-449`) re-walks and re-fetches its whole `McpInvocation` history every tick; with per-call minting that is quadratic issuance against a pid quota that never resets (`crates/maos-kernel-core/src/capability/mod.rs:188-190`; window owner `17-9-capability-issuance-quota-window`). Keep the last cursor and the already-fetched frames in the Researcher so each tick recalls and fetches only rows written since the previous tick; `recall_all`'s page cap (`:1010`, `:1023-1049`) stays. Prove with the existing walker tests plus one counting the mints across N ticks (O(new rows), not O(history)).
- [x] **T7 — TS scaffold** (AC1, AC2; FW-08; PF-09)
  - [x] `templates/spirit-ts/` then regenerate `examples/example-spirit-ts/` (never build inside `templates/`): `src/index.ts` imports the recall functions from `maos:spirit/recall@2.1.0`, calls `recall({ limit: 16 })` and `fetch(frame.frameId)` in `handleFrame`, treats success and `scope-violation` as normal, rethrows any other error, with comments teaching the scope (emitter-only today; received frames become fetchable with 17-8); `manifest.toml` adds `[capabilities.required.log] recall = true` / `fetch = true`; bindings via `npm run build` (`jco guest-types … --strict`); Vitest mocks the host import module; READMEs (skipped by `templates-regen`) updated by hand. Verify the exact JS error shape from the generated `.d.ts` — do not assume it.
  - [x] Migrate `crates/maos-wasm-host/tests/ts_component_17_3b.rs::typescript_component_round_trips_a_fully_populated_frame` (runner started with component/fuel only, `:143-175`; emitted frame required, `:177-190`; run by `.github/workflows/discipline.yml:1180-1192`, which `wasm-host-tests` depends on): the test answers the scaffold's two tagged requests on the runner's stdin through `recall_wire` (an empty page, then `scope-violation`), asserts both arrived in order, and keeps the fully populated, byte-identical round trip. Never suppress `unavailable`, skip the test or swap in the simpler CLI seed.
- [x] **T8 — Integration tests and proven-red** (AC1.4, AC3, AC4.1)
  - [x] `crates/maos-wasm-host/guests/recall-probe` (Rust, `@2.1.0`): scripted recall/fetch per inbound frame, results reported in its emitted frame.
  - [x] `crates/maos-bin/tests/recall_17_3d.rs` (`#![cfg(all(target_os = "linux", feature = "network", feature = "wasm-host"))]`, real release runner under T2, fixture build per `spawned_scope_17_3c.rs:36-85`): own-frame recall across two turns; seed fetch → `scope-violation` + R2/R3; R1→R2→R3 ordering with SSO configured (interleaved `identity.asserted` allowed, never recalled); empty page → no receipt; ungranted manifest → `capability-denied`, receipt only, no call row; operator deny → `capability-denied`; quota exhausted (test seam on the issuer's budget) → `capability-denied` + receipt, no tokenless fallback; cross-boot same pid → nothing; disclosure: the guest recalls and fetches its own receipt and authorization-row ids → never returned / `scope-violation`; reply budget: a stored payload whose reply exceeds the bound → `reply-too-large`, later calls still work; consent safety vector (AC4.1); hostile records (a tagged body between exports, an unknown tag, junk after the tag, `MAX_RECALLS_PER_TURN + 1` calls) → one `recall.channel.closed:protocol-violation` receipt and a typed session end; FR4 `audit query --spirit` exits 0 and lists R1–R3; the checked-in Researcher manifest admits both log scopes, its recall rows are token-bearing and its FR4 view exits 0.
  - [x] `recall_wire` unit tests (every OS): tag discrimination (an `IacFrame` body is never classified as recall), byte-string encoding cost, page prefix at the exact budget boundary, `reply-too-large` at bound + 1, partial header, oversize body.
  - [x] Proven red, archived (`17-3d-evidence/`): disclosure predicate removed → receipt-fetch test red; boot predicate removed → cross-boot test red; SQL kind restriction reverted to the in-memory drop → pagination test red; attributed token dropped → FR4 test red; receipt writer removed → R3 test red; byte-string newtype replaced by `Vec<u8>` → reply-budget test red; tag dispatch moved after the frame decode → hostile-record test red. Every positive/red/restored bundle records the source commit, `HEAD^{tree}`, the SHA-256 of the exact dirty diff, `rustc --version`, the command, the complete output and the CI run id/image (`deferred-work.md:1096`).
- [x] **T9 — CI** (AC1.3; FW-10 re-ruled)
  - [x] `wasm-host-tests` (`.github/workflows/discipline.yml:3374-3534`): `recall_17_3d` in the delegated envelope with exact pass-count and name guards (no SKIP, no zero-test pass — 17-7 log guards); the migrated `ts_component_17_3b`; `t2_sandbox_kill` unchanged (the T2 table did not change).
  - [x] `epic-17-exit` (`.github/workflows/discipline.yml:1821-1984`): line 2b step after line 2 (`:1957-1974`) with the AC1.3 assertions, same `MAOS_HOME`/audit store as line 2; delete the "line 2b (17-3d) is still owed" comment (`:1817-1819`).
- [x] **T10 — Docs, ADRs, registers** (AC3.8, AC4.3, AC5)
  - [x] `docs-site/docs/deploy/topology.md` + KO (recall adds no descriptor, socket or syscall); `cookbook/wasm-component-spirit.md` + KO (grant, import, scope, errors, export-only availability); `cookbook/manifest-fields.md` + KO and `manifest/v5.md` + KO (`[capabilities.required.log]`, schema-5 degradation) — EN/KO TOML blocks byte-identical (`crates/maos-bin/tests/cookbook_manifests_17_3b.rs:8-14`, `:76-85`).
  - [x] ADR-060 "Implementation note — Story 17-3d" (the landed tag numbers, grant syntax, disclosure allow-list, reply budget; the 2026-10-18 in-band amendment at `:230-268` is already ratified); ADR-032 note (tagged recall records share the runner's stdio; frame records unchanged); ADR-013 implementation note: local recall is emitter-scoped (the incarnation bound is ratified at `:25-27`); participant recall and consent evaluation are owned by `17-8-participant-recall-and-consent-envelopes`.
  - [x] No `deferred-work.md` row is needed for FR29's participant/consent half (tracked story 17-8) or the quota window (tracked story 17-9). Anything else the dev defers is filed with a same-line resolvable `Owner:` story key (`xtask/src/check_dev_record_completeness.rs:207-225`).
- [x] **T11 — Governance re-pins** (§Delivery gate; FW-02, FW-03)
  - [x] Measure, return any over-pin out-of-kernel figure before landing, re-pin the seven artifacts in order, run the listed gates; record figures in Completion Notes.

### Review Findings

**2026-10-19 — non-author code review (`openai-codex/gpt-6.1-sol`).** Full baseline-to-working-tree review against `39cd7966a354917b2a9cf805faa2462cb05c9677`: Blind Hunter, Edge Case Hunter and Acceptance Auditor in parallel; test-infrastructure audit and runtime execution by the parent. The configured Claude-author rule omits a separate Test Infrastructure subagent. The first Blind Hunter launch failed on agent-name ownership; its uniquely named retry completed. All three review layers returned findings.

- [x] [Review][Patch][High] Use an insertion-stable checkpoint for incremental Researcher recall [spirits/researcher/src/lib.rs:453-468] — Fixed with native `LogRecallFilter.order_by_insertion`, preserved by every `recall_all` page. SQL resolves the cursor frame id to its rowid and selects later appends without using wall time as the watermark. Boot/emitter/disclosure restrictions and the WASM timestamp-cursor contract remain unchanged. Permanent Researcher and adapter regressions cover backward timestamps, terminal-page polling, page walks, scope/disclosure and one fetch per new row. Sources: blind + edge + auditor.
- [x] [Review][Patch][Medium] Execute recall wire tests on every CI OS [.github/workflows/discipline.yml:3895-3911,4773-4789] — Fixed with Windows and macOS `--lib recall_wire` steps requiring six passes, zero failures/ignored tests and all six exact names. Portable recall/codec changes now trigger `macos-scope`. The actual shell step passed locally; zero-suite and missing-name controls exited 1. Native Windows/macOS execution remains the responsibility of their CI runners and is not claimed as locally exercised.

**Initial review verification:** `recall_17_3d` 16/16 under actual T2 with `MAOS_17_2_CGROUP_MODE=undelegated`; `recall_mediation_17_3d` 14/14; `researcher_log_grant_17_3d` 2/2; Researcher `incremental_recall` 3/3; adapter `log_recall_17_3d` 8/8; `recall_wire` 6/6. The undelegated lane reports its RLIMIT_AS fallback; no delegated-cgroup or non-Linux execution is claimed.

**Independent runtime smoke:** actual release runner and recall-probe over real pipes: Ready → Recall → empty Page → Fetch(seed) → ScopeViolation → guest report → TurnComplete → EOF, exit 0. A separate throwaway harness exercised the real Researcher hook and adapter with a newly inserted row's stored timestamp moved below the retained cursor: 2 claims produced 1 finding; after a later normal-timestamp insertion, 3 claims produced 2 findings; the rolled-back row was never fetched. This simulates a backward wall-clock adjustment without changing the host clock. Throwaway sources/binaries were removed after execution.

**Triage:** 0 decision-needed, 2 patches applied at the operator's request, 0 defer, 1 dismissed. The pipelined-request backpressure candidate was dismissed after reachability review: ordinary WASM imports synchronously consume each reply, and guest WASI stdout is redirected to stderr; reproducing pipelining requires replacing the trusted runner or assuming its prior compromise.

**Post-fix verification:** `cargo test --locked -p maos-domain -p maos-iac -p researcher` passed 486 tests; the feature-enabled `recall_17_3d`, `recall_mediation_17_3d` and `researcher_log_grant_17_3d` rerun passed 16 + 14 + 2; the macOS-scope/cgroup workflow suites passed 27 + 1; the extracted Windows/macOS wire step passed all 6 cases. Researcher `incremental_recall` is now 4/4 and adapter `log_recall_17_3d` is 9/9. The independent clock smoke now observes 2 claims → 2 findings and 3 claims → 3 findings; the rolled-back row is fetched exactly once. The attempted permanent-test pre-fix run timed out waiting for Cargo locks; the before/after runtime smoke, rather than an unobserved permanent-test red, is the reproduction evidence.

**Review re-pins (FW-03):** Lunarpulse approved the exact formatted measurement on 2026-10-19: domain 9628 → 9632 (+4), IAC 7290 → 7298 (+8), bin 26339 → 26340 (+1), aggregate measured 177609 / strict hardfail 177610. Kernel unchanged at 26291 physical / 20527 tokei. `cargo fmt --all -- --check`, `kloc-check --json`, `check-kernel-baseline`, `check-service-boundary` and `check-empty-kernel` passed. ADR-013 documents native append-order polling; WIT and T2 are unchanged.

**Closure consistency:** `check-dev-record-completeness` passed over 171 done-status stories; `check-epic-close-coherence` passed over 21 epics against pin 26291; `check-manifest-schema-version` passed at schema 5. Story and sprint status are synchronized to `done`; prior sprint-row annotations remain as explicitly historical evidence.

## Dev Notes

### Recall after this story

1. Admission → scheduler pid `P`, `SandboxSpec` (`crates/maos-bin/src/admission.rs:655-723`); `[capabilities.required.log]` → `Scope::LogRecall`/`LogFetch` in the pid-keyed policy table (`security/mod.rs:335-357`).
2. `run_admitted_session`: spawn and bridge exactly as 17-3c (`spirit_session.rs:281-296`, `task_args` empty); the session holds `(P, spirit_id, boot)`.
3. Seed delivered between exports; the runner calls `handle-frame`.
4. Guest `recall(filter)` → runner writes one tagged request record on stdout and flushes → kernel bridge passes it through → session stdout arm sees the tag → validate → mint `LogRecall` at `P` → adapter writes kind-7 `log.recall` with the token id → SQL page (boot = current, disclosable kinds only, metadata only, limit applied last) → page trimmed to the reply budget → tagged reply via `tx` on stdin → token revoked.
5. Guest `fetch(id)` → same with `LogFetch`; foreign pid, foreign boot or non-disclosable kind → `ScopeViolation` → reply `scope-violation` + receipt `log.fetch.refused:scope-violation`; payload over the reply budget → `reply-too-large(bound)` + receipt.
6. Export returns → frames → empty TurnComplete; the session revokes the turn's leases as today. Runner exit → the bridge finalizer revokes all tokens for `P` (`runtime.rs:1067`, `spirit_session.rs:478-482`).

### WIT `@2.1.0` — `interface recall`

```wit
interface recall {
    use frames.{frame-origin};

    // Opaque keyset cursor returned by the host.
    record recall-cursor { last-timestamp-ns: u64, last-frame-id: list<u8> }   // [u8; 16]

    // No pid, Spirit name, recipient or team: the host binds the caller (R17-23).
    // Callable only while the host is running `handle-frame`; otherwise `unavailable`.
    record recall-filter {
        kind: option<string>,          // FrameKindLabel serde name; unknown → invalid-request; non-disclosable → empty page
        since-ns: option<u64>,         // ≤ i64::MAX, else invalid-request
        until-ns: option<u64>,         // ≤ i64::MAX, else invalid-request
        limit: u32,                    // 0..=1024 (LogRecallFilter::MAX_LIMIT); larger → limit-exceeded(1024)
        cursor: option<recall-cursor>,
        intent-filter: option<string>,
    }

    record recall-entry {
        frame-id: list<u8>,
        timestamp-ns: u64,
        kind: string,
        intent: string,
        payload-available: bool,
    }

    // A page may hold fewer than `limit` entries when the reply budget is reached;
    // `next-cursor` then resumes after the last returned entry.
    record recall-page { entries: list<recall-entry>, next-cursor: option<recall-cursor> }

    record fetch-response {
        frame-id: list<u8>,
        timestamp-ns: u64,
        kind: string,
        intent: string,
        payload: list<u8>,             // stored redacted bytes, never truncated
        origin: frame-origin,
    }

    variant recall-error {
        scope-violation,
        frame-not-found,
        invalid-request(string),
        limit-exceeded(u32),
        capability-denied,
        consent-withheld,
        reply-too-large(u64),          // the reply byte bound
        unavailable,
    }

    recall: func(filter: recall-filter) -> result<recall-page, recall-error>;
    fetch: func(frame-id: list<u8>) -> result<fetch-response, recall-error>;
}
```

Excluded on purpose: `peer_spirit_pid` (always the caller under emitter scope; pids are not guest vocabulary), `capability_token` (`LogFetchResponse`, `crates/maos-domain/src/log_recall.rs:250-287` — never to a guest), owner pid in errors. `use frames.{frame-origin}` ties `origin` to the frames enum the bridge already refuses unknown values for. If the guest-visible shape must differ, change it here and in the ratification entry together.

### Wire protocol (runner ↔ maos-bin, in-band)

- Records are ADR-032 records (`maos_frame_codec`, Content-Length, `MAX_FRAME_BYTES`, `crates/maos-frame-codec/src/lib.rs:7-13`, `:96`). Frame bodies stay untagged CBOR maps; empty bodies stay transport controls (`runner.rs:267-270`). A recall body is **one** CBOR data item with a private tag (`RECALL_REQUEST_TAG` runner → daemon, `RECALL_REPLY_TAG` daemon → runner); maos-bin dispatches on major type 6 before any frame decode.
- One request, one reply, strictly alternating, only during an export; no request ids are needed because maos-bin writes nothing else to stdin during an export and the runner reads exactly one record per call.
- Request: `Recall { filter }` | `Fetch { frame_id: bytes(16) }`. Validation before minting: `limit ≤ 1024`, frame id exactly 16 bytes, kind label parses as `FrameKindLabel`, cursor id 16 bytes, timestamps ≤ `i64::MAX`; failures → `invalid-request` / `limit-exceeded` + receipt, no mint, no call row.
- Reply: `Page`, `Fetched`, or `Error(code[, detail])`; `Storage(_)` maps to `unavailable` (storage text stays in the receipt payload, never on the wire). Ids and payloads are CBOR byte strings; the reply passes both codec budgets (AC3.6).
- Receipt payload (operator-only, never disclosable — AC3.4; FR4 plain shows only the intent): `{"op","code","frame_id_hex"?,"owner_pid"?,"boot_nonce"}` — keys in a fixed order.

### Error mapping (native and WASM share it)

| Mediation / domain outcome | `LogRecallError` (native port) | WIT `recall-error` |
|---|---|---|
| Mint refused (policy, operator deny, quota) | `CapabilityDenied` (new) | `capability-denied` |
| Foreign pid, foreign boot, non-disclosable kind | `ScopeViolation { .. }` | `scope-violation` |
| No such row | `FrameNotFound { .. }` | `frame-not-found` |
| Bad cursor, label, id length, timestamp | `InvalidCursor(..)` | `invalid-request(detail)` |
| `limit` > 1024 | `LimitExceeded { .. }` | `limit-exceeded(1024)` |
| Reply over the wire budget | — (WASM only) | `reply-too-large(bound)` |
| Storage failure | `Storage(..)` | `unavailable` |
| Consent (17-8) | added by 17-8 | `consent-withheld` |

### File implementation map

| Area | Files |
|---|---|
| Kernel (FW-01 re-ruled) | `scheduler/scheduler_loop.rs`; deletion-only `scheduler/hook_dispatch.rs`, `scheduler/kernel_ctx.rs`; caller migrations in kernel tests/benches |
| maos-manifest | `src/manifest.rs`, `src/lib.rs` |
| maos-domain | `src/log_recall.rs` (`CapabilityDenied`) |
| maos-iac | `src/adapter/transparency_log.rs`, `src/adapter/log_recall.rs` |
| maos-audit | `src/fr4_classifier.rs` (writer inventory only) |
| maos-wasm-host | `src/recall_wire.rs` (new), `src/host_state.rs`, `src/runner.rs`, `src/lib.rs`; `guests/recall-probe/`, `guests/compat-echo-2-0/` (new — guest sources count toward this crate's kloc row: `xtask/src/kloc_check.rs:299-317` excludes `tests`, not `guests`); `tests/wit_corpus.rs`, `tests/ts_component_17_3b.rs`; `tests/fixtures/wasm/*` (repo root) |
| maos-bin | `src/recall_mediation.rs` (new), `src/spirit_session.rs`, `src/main.rs`, `src/admission.rs`; `tests/recall_17_3d.rs` (new); scheduler-constructor test migrations |
| Researcher | `spirits/researcher/{manifest.toml,src/lib.rs,tests/*}` |
| WIT / TS | `wit/spirit.wit`; `templates/spirit-ts/{wit/spirit.wit,src/index.ts,manifest.toml,README.md,tests/*}`; same under `examples/example-spirit-ts/` + `src/bindings/*` |
| Governance | `xtask/abi-ratifications.toml`, `xtask/src/check_manifest_schema_version.rs`, the §Delivery gate seven |
| CI / docs | `.github/workflows/discipline.yml`; `docs-site` pages + KO twins; `docs/adr/ADR-060…`, `ADR-032…`, `ADR-013…` |

`maos-frame-codec` is not expected to change; if it must, it is a measured request (FW-03).

### Guardrails

- Never add a syscall to T2, never pass a descriptor, never add `unsafe` or `libc` for recall; the transport is the runner's stdio (ADR-060 `:230-268`).
- Never take identity from the wire or from the runner's OS pid — only `admitted.pid` + boot.
- Never a second `LogRecallAdapter` — the composition-root one (`main.rs:2389-2415`) carries the cross-wall providers.
- Never hand-roll grant checks or seed the policy table from maos-bin (`admission.rs:40-47`); mediation is `issue_enterprise_governed_capability`.
- Never leave a tokenless kind-7 recall row on a Spirit-facing path, and never reclassify a call as non-call to green FR4.
- Never disclose a row by its decoded kind; the allow-list reads the raw discriminant, and nothing outside it reaches a guest or the Researcher.
- Never put recall code in the kernel; the only kernel edits are the literal and the seam deletion.
- Never expose `capability_token`, owner pids or storage errors to the guest; never map an unknown label to a default kind; never truncate a payload.
- Never write to the runner's stdin during an export except the one reply; never decode a tagged body as a frame.
- Never commit generated TS output or `node_modules`; never build inside `templates/`.
- Never cite a recall latency number (AC5).

### Proven-red controls (rule 11(c))

| Control | Plant | Expected red |
|---|---|---|
| Disclosure allow-list | remove the raw-kind predicate | receipt/authorization-row fetch test red |
| Boot scope | drop the boot predicate | cross-boot test red |
| Page correctness | revert to the in-memory kind-7 drop | pagination test red |
| Token attribution | pass `None` on the attributed path | FR4 `--spirit` exits non-zero |
| Refusal receipt | delete the receipt write | R3 assertion red |
| Reply budget | `Vec<u8>` instead of the byte-string newtype | reply-budget test red |
| Tag dispatch | decode frames before the tag check | hostile-record test red |
| Exit line 2b | remove the scaffold's `fetch` (`ci/17-3d-*`) | `epic-17-exit` red |

### Tests and commands

```bash
cargo test --locked -p maos-manifest
cargo test --locked -p maos-kernel-core --test manifest_field_coverage --test kernel_ctx_round_trip
cargo test --locked -p maos-domain
cargo test --locked -p maos-iac
cargo test --locked -p maos-audit --test fr4_classifier_16_2
cargo test --locked -p maos-wasm-host --test wit_corpus --test wit_copies_17_3b --test t2_sandbox_kill --test ts_component_17_3b -- --nocapture
cargo test --locked -p maos-wasm-host --lib recall_wire
cargo test --locked -p maos-bin --features network,wasm-host --test recall_17_3d -- --nocapture --test-threads=1
MAOS_17_2_CGROUP_MODE=undelegated cargo test --locked -p maos-bin --features network,wasm-host --test spawned_scope_17_3c -- --nocapture --test-threads=1   # 22 must still pass; the suite refuses to run without the mode (`delegated` in CI's envelope)
cargo test --locked -p researcher   # boot + disclosure predicates, mediated port, incremental walk (FW-09 A, PF-08)
cargo run -p xtask -- check-equiv-fixture-provenance && cargo run -p xtask -- templates-regen --check
cargo run -p xtask -- check-manifest-schema-version && cargo run -p xtask -- kloc-check --json
```

`spawned_scope_17_3c` and `recall_17_3d` compile to zero tests without `network,wasm-host` on Linux (`spawned_scope_17_3c.rs:1`) — a zero-test run is not a pass. The T2 suites print `SKIP` off CI and must fail in CI on a refused spawn (17-6).

### Previous-story intelligence

- **17-3a (spike):** inherited-fd and path-socket transports measured under T2 (path sockets need `socket`/`connect`, which escape); the in-bridge alternative was rejected on 2026-09-04 only for its then kernel cost (`preflight-r2/epic-17.md:183`), which 17-3c removed; identity is per-channel. Historical artifacts under `spikes/story-17-3a-wasm-recall/` — the descriptor transport is superseded; reuse no code.
- **17-3b:** `@2.0.0` lossless frames; WIT edits need the ratified version entry + 4-component re-issue; `componentize-js` 0.23.0 direct (not jco's nested), jco 1.35.0, Node `^22.20 || ^24.12 || >=25`; root/template/example WIT byte-identical.
- **17-3c:** admitted spawned session; maos-bin owns the stdin writer and stdout loop (the basis of the in-band design); governed `iac.send` minting and token-bearing rows (it had to build `iac.send` because the FR4 view rejected tokenless frames — the same trap AC3.3 closes); release runner mandatory (debug `Component::new` ~13× slower); CPU 10 % needs a delegated cgroup; compile watchdog 360 s; isolated target dirs (a shared target corrupted provenance); `membarrier` was found only on the ARM CI images. D-17-3c-ES-04: a FLAG-Winston is a location charter, not a numerical grant.
- **17-6:** T2 is a production path; its allow-list is not touched by this story; kill filter first, then allow list (`linux.rs:404-412`).
- **17-7 / 17-2 / 17-1:** creation = isolated `git archive` prototype + non-author validators; delivery-gate order; `ci/17-<n>-<purpose>` branches only under an explicit ruling; delegated-envelope log guards reject SKIP/zero/ignored; deletion-only charter extensions (17-1 R17-107, 17-2 FW-01).
- Git: `0d73cac3` (17-3c), `a6ef4974` (17-3b), `4f677bc1` (17-6), `daf3b876` (ARM `membarrier`), `6537454e` (17-7), `39cd7966` (20-3a, HEAD).

### Measured prototype — E16-A7 creation precondition (transport superseded)

Isolated copy (`git archive 39cd7966` → a throwaway directory with its own `CARGO_TARGET_DIR`, offline/locked; both removed after measurement), never the production tree. Instruments: physical = `lines().count()` over kernel `src/**/*.rs` (`xtask/src/check_kernel_baseline.rs:472-484`); tokei 14.0.0 with the `xtask/src/kloc_check.rs:296-320` exclusions. The base reproduced the kernel physical/tokei, `maos-domain` and `maos-exec-deps` pins; `maos-manifest` measured 4466 against its 4474 ceiling (8 lines of headroom). Evidence: `17-3d-evidence/{measurements.json, prototype-kernel.diff (A), prototype-variant-b-increment.diff, prototype-variant-b.diff, smoke.log, variant-b-smoke.log}`; diff SHA-256 A `d3722863…a3f7`, B `43632fc3…ae57`.

| Instrument | Base `39cd7966` | A (transport, superseded) | B (A + grant) | Still applicable |
|---|---:|---:|---:|---|
| Kernel physical | 26311 | 26328 | 26329 | +1 (the `scheduler_loop.rs` literal, B − A) |
| Kernel tokei | 20545 | 20557 | 20558 | +1 |
| `maos-manifest` tokei | 4466 (pin 4474) | 4466 | 4493 | +27 (**+19 over the pin**) |
| `maos-domain` / `maos-exec-deps` | 9615 / 358 | = | = | `maos-domain` now changes (`CapabilityDenied`), measured at landing |

The descriptor transport (A: `linux.rs` +12/+9, `sandbox/mod.rs` +4/+2, `security/mod.rs` +1/+1) was ruled out on 2026-10-18; its smoke (`sendto`/`recvfrom`/CLOEXEC leave-one-out) is historical evidence only. The seam deletion (≈ −21 physical, [INFERENCE] from the preflight count) is measured at landing.

### In-band prototype — 2026-10-18 (the ratified transport)

Built at the operator's request after the preflight, in an isolated copy (`git archive 39cd7966`, own target directory, offline/locked; base `5f24e89f`, prototype `d3d08fa8`, trees `7bee47d2`/`fe9bf230`; `rustc 1.98.1`), never the production tree; the copy and its target directories were removed after measurement. It implements root WIT `@2.1.0`, `recall_wire.rs` (tags, byte-string newtype, reply-budget helper), `HostState`-owned stdio with the recall import and its latches, the runner integration, a Rust `recall-probe` guest, and a harness that plays the daemon through the **real kernel bridge** (`spawn_and_bridge`, `take_frame_writer`, `poll_frame`). Evidence: `17-3d-evidence/inband/{prototype-inband.diff, smoke.log, measurements.json}`; diff SHA-256 `14e292fbe7300eb101b56468e63622cfd665d6408620cf4e4347c1b3bf8480c5`. The parent re-ran the harness: 6 passed, 0 failed.

| Scenario | Observed |
|---|---|
| Happy path, **T2** through the kernel bridge | the guest reports `recall=ok:2;fetch=err:scope-violation`; two tagged requests, fetch id = seed id; TurnComplete; exit 0; live `/proc/<child>/status` shows `Seccomp: 2` |
| Same, T0 plain spawn | identical result |
| Hostile reply (untagged body) | recall and fetch both `unavailable`; one request, no second; the guest's output and TurnComplete are written; runner exit 6 `recall protocol desync`, stdin not read again |
| EOF while a request is outstanding | both `unavailable`; TurnComplete; exit 0, no trap |
| `@2.0` echo component on the `@2.1` runner, T2 | byte-identical frame round trip; zero tagged records; exit 0 |
| Reply budget | 1 MiB payload as byte string → 1,048,694-byte reply, round-trips; as `Vec<u8>` → `container expansion budget exceeded`; realistic 1,024-entry page → 104,482 bytes, untrimmed; oversized metadata → trimmed to 858 entries with the cursor at the last, 859 rejected |
| `t2_sandbox_kill` (allow-list untouched) | 5 pass; `ptrace` → SIGSYS 31, classified T2 |
| maos-bin stub dispatch (tagged → `unavailable`) | `cargo check --features network,wasm-host` clean; `spawned_scope_17_3c` 22/22 with `MAOS_17_2_CGROUP_MODE=undelegated` — the frame path is unchanged |

| Instrument | Base | Prototype |
|---|---:|---:|
| Kernel `src_lines` / tokei | 26311 / 20545 | 26311 / 20545 — no kernel file touched; `check-kernel-baseline` passes |
| `maos-wasm-host` tokei (pin 1577) | 1525 | 1811 (+286, of which 50 are the probe guest) |
| `maos-bin` tokei (pin 25917) | 25917 | 25931 (+14, the stub dispatch only; mediation not built) |
| Aggregate (`_aggregate_hardfail` 176234) | 176233 | 176533 |
| `unsafe` added / workspace dependencies changed | — | 0 / none |

Not run: the delegated-cgroup lane (the host's cgroup mount is read-only), separately injected wrong-tag and undecodable replies, a recall call outside an export, the WIT corpus and equivalence suites; they remain T5/T8 obligations. The `maos-wasm-host` row already exceeds its pin by 234 before mediation, tests and the compat guest, and `maos-bin` and the aggregate sit at zero headroom: each returns as a measured request before landing (FW-03). The kernel figure for the full story is the measured +1 literal plus the seam deletion.

### Latest technical information

- Wasmtime: lock `48.0.6` (`Cargo.lock:6115-6118`; manifest requirement `48.0.4`, `crates/maos-wasm-host/Cargo.toml:48-49`) already carries the 2026-10-14 advisories' fixes for the 48 line (GHSA-q554-3cjc-rx7p handle tables, GHSA-m9f7-9w4p-gv38 symlink escape, others — patched 48.0.6/49.0.3). Latest stable 49.0.3; no upgrade in this story.
- Wasmtime component `Linker` resolves semver-compatible names both ways ([docs](https://docs.wasmtime.dev/api/wasmtime/component/struct.Linker.html#names-and-semver)): a `@2.0.0` import resolves to a `@2.1.0` definition. A `@2.1` guest importing `recall` on a host without it fails at instantiation (missing import) — "hosts ship first". Type-only growth inside an existing interface would still fail type-checking, which is why `recall` is a new interface. The runner is the only component host in the tree.
- `ciborium` 0.2.2 (`Cargo.lock:690-691`; maos-wasm-host `Cargo.toml:52-53`) provides `tag::Required<V, const TAG: u64>`, which refuses an absent or different tag during decode.
- componentize-js latest 0.23.0 (pinned); jco latest 1.39.0 (pinned 1.35.0 — no bump in this story).

### Project Structure Notes

- `recall_mediation.rs` sits beside `spirit_session.rs` (gated by `network`, because the native Researcher uses it too); `recall_wire.rs` lives in maos-wasm-host's safe library because maos-bin already depends on it under `wasm-host` (`crates/maos-bin/Cargo.toml:30`, `:60`) and the runner owns the other end.
- Tests follow `<topic>_17_3d.rs`. Evidence under `_bmad-output/implementation-artifacts/17-3d-evidence/`.
- `schemas/` holds no Spirit-manifest JSON schema; do not invent one.

### Non-author validation — 2026-10-17

Three fresh read-only validators (citations, design soundness, governance) re-read the draft, the charter and the epic edits against `39cd7966`. They found 11 blockers, 13 should-fix items and 5 nits; every finding was applied. Dispositions about the descriptor transport (socket timeouts, service-thread teardown, the CLOEXEC tier capture, the `ts_component_17_3b` T2 responder) are superseded by the 2026-10-18 in-band ruling (§Preflight); the rest stand:

| Finding | Disposition |
|---|---|
| FW-09 A's "scheduler injection" cannot take a `dyn` port (`kernel_ctx.rs:27`) | The seam is dormant; FW-09 A is the Researcher port only. Preflight ruling R2 deletes the seam. |
| Researcher is schema 4, so its log grants would be dropped | Raise it to 5 and test admission of the checked-in manifest (T6). |
| Mediation needs `network`-only modules | `recall_mediation` gated by `network`. |
| The TS scaffold change breaks `ts_component_17_3b` | Migrated: the test answers the tagged requests on stdin (T7). |
| `identity.asserted` rows interleave with R1–R3 under SSO | Ordered subsequence, not adjacency (AC1.1, AC1.3); never disclosable (AC3.4). |
| "Every data-less call" would receipt an empty page | Receipts only for `Err(recall-error)` (AC3.3). |
| FR4 inventory instruction contradicted the scanner | Reconcile and extend the scanner; kind-28 receipts proven by dedicated tests (T3). |
| A ruling alone could flip contradictory options to ready | Every ruling is materialized into this one contract before the flip. |
| 20-7 had no recall leg | Filed on 20-7 AC4 by R17-154 (AC5). |

Citation fixes at creation: `wasm_sha256` (not `blob_sha256`), the kernel-surface re-export entries do not re-hash, the finalizer callback site, workflow job ranges, the manifest-coverage fixture path, full story filenames, and the "every pin" overstatement.

### Preflight — long-term correctness (2026-10-18)

Party-mode preflight at the operator's request (*resolve the issues raised in the story creation for the long-term correctness*) builds on, rather than repeats, the creation prototype and three-validator review. Four independent read-only reviewers checked recall scope and audit rows, transport and lifecycle, mediation and form parity, and planning consistency at `39cd7966` (production source unchanged since creation); every claim below was re-read against the tree. Epic §R14 adds **R17-157…R17-164**. The operator ruled four questions outside the charter (D-17-3d-PF-R1…R4, recorded in `17-3d-flag-winston-decision.md`).

| ID | Problem / disposition | Required development oracle |
|---|---|---|
| **PF-01** | **Ruling R1.** The ratified descriptor transport needed a raw-fd adoption that safe Rust cannot express without `unsafe`/`libc`, Linux-only. Since 17-3c the kernel bridge is byte-transparent, so recall rides the ADR-032 stdio in-band: tagged records, dispatched in maos-bin, no descriptor, syscall, socket, thread or `unsafe` (AC3.1, AC3.7; ADR-060 `:230-268`). **Prototyped 2026-10-18** under real T2 (§In-band prototype). | Tag dispatch, hostile records, desync exit, export-only availability (T8); frames byte-identical (`spawned_scope_17_3c` 22 pass — 22/22 in the prototype). |
| **PF-02** | Recall dropped only kind 7 and fetch checked only the pid, so a guest could recall and fetch its own refusal receipts (`owner_pid`), authorization rows and `identity.asserted` rows — the last relabelled `TaskAssign` by the reader fallback. AC3.4's raw-kind allow-list {0–6, 18}, in SQL before `LIMIT`, shared by fetch and the cross-wall reader. | Mixed-row pagination; receipt/authorization/kind-30 ids → `scope-violation`; unknown raw kind never `TaskAssign` (T3, T8). |
| **PF-03** | `codec` charges 1 KiB per CBOR node; serde arrays make a 16 KiB payload exhaust the budget, and a failed reply would have broken the channel for good. Byte strings, both budgets, longest-fitting page prefix, `reply-too-large(u64)` (AC2.2, AC3.6). | Boundary tests at the exact budget; oversize fetch then a successful call (T8). |
| **PF-04** | Recall materialized and cloned every payload of `limit+1` rows. Metadata-only projection; fetch checks length first (AC3.4). | Projection test (T3). |
| **PF-05** | `u64` timestamps above `i64::MAX` wrapped negative in SQL. Rejected before minting (AC3.4). | `since = u64::MAX` → invalid (T3). |
| **PF-06** | No index serves `(boot, pid, keyset)`. Idempotent `idx_tlog_boot_pid_ts` (AC3.4). | Index present after open; existing stores upgrade idempotently (T3). |
| **PF-07** | The native port had no denial variant, inviting `Storage("denied")`. `LogRecallError::CapabilityDenied` + one mapping table (AC3.3, §Error mapping). | Native ungranted/denied/exhausted → `CapabilityDenied` (T6, T8). |
| **PF-08** | Per-call minting charges a per-pid issuance quota that never resets (`capability/mod.rs:188-190`), and the Researcher re-walks its whole history every tick — quadratic issuance until every capability is denied. Incremental walk here; **ruling R3:** the window itself → new story `17-9-capability-issuance-quota-window`. | Mint count O(new rows) across N ticks; exhaustion → typed `capability-denied` + receipt (T6, T8). |
| **PF-09** | 17-8 makes received frames fetchable, so the seed fetch will succeed: the scaffold accepts success or `scope-violation`, and §17-8 owns re-stating exit line 2b's rows and AC4.1 (R17-162). | Scaffold Vitest covers both outcomes (T7). |
| **PF-10** | **Ruling R2.** `KernelCtx.log_recall` is an unmediated recall handle production fills and nothing reads: deleted (deletion-only FW-01 extension; no raise) (AC3.2). | Kernel builds; no `log_recall` symbol in `KernelCtx`/`HookDispatcher`; kernel-surface regenerated (T1, T11). |
| **PF-11** | **Ruling R4.** Cross-incarnation recall had no owner: a non-goal by design, written into ADR-013 (`:25-27`); persistence across restarts is memory tiers and distillates. | None (decision). |
| **PF-12** | Planning drift: the epic still said 17-3d closes FR29 for non-Rust and has no later-epic consumer; §17-8 AC4 let creation drop header-only elision; AC5 cited `epic-20…:188` only. Fixed in the epic (§17-3d, §17-8, Dependencies) and here. | `check-epic-close-coherence`, `check-exit-commands` green. |

**Verification boundary:** this is a corrected development contract, not an implemented runtime. No production code changed; the ADR-060 in-band amendment and the ADR-013 incarnation note are ratified documentation. The in-band prototype proved the transport under real T2 with a stub daemon; mediation, the adapter changes, the TS toolchain build, proven-red CI runs and the full §A6 net remain development obligations.

### References

- Epic: `_bmad-output/planning-artifacts/epics/epic-17-workers-and-third-party-form-w2.md` — §17-3d, exit block and provenance (Line 2b), confidence rules incl. 10/11, §17-8, §17-9, §R5–§R8 rows R17-23, R17-42, R17-43, R17-46, R17-51, R17-66, §R13, §R14, Dependencies. `epic-20-ship-it-w5.md:188-189` (R17-154 recall leg on 20-7 AC4); `dependency-dag.md:217`; `epics/index.md`.
- PRD FR29 `_bmad-output/planning-artifacts/prd/functional-requirements.md:75`; NFR-Perf-5 `non-functional-requirements.md:13`; inventory `epics/requirements-inventory.md:557`.
- ADRs: `docs/adr/ADR-060-spirit-forms-by-trust-tier.md:100-110`, `:185-228` (superseded transport), `:230-268` (in-band); `ADR-013-log-recall-kernel-mediated-spirit-abi-capability.md:11`, `:25-27`; `ADR-012-typed-intent-a2a-consent.md:12-16`; `ADR-031-wasm-component-model-spirit-form.md:51`; `ADR-032-spirit-wire-protocol-bytes-on-wire.md:12`; `ADR-058-cross-wall-provenance-and-consented-recall.md:30-38`.
- Stories (`_bmad-output/implementation-artifacts/`): `17-3a-wasm-recall-and-componentize-spike.md:254-266`, `:338-350`, `:457`; `17-3b-wasm-form-admission-and-contract.md:35-36`, `:86-96`, `:247`; `17-3c-wasm-spirit-on-the-bus-under-t2.md:43-69`, `:99-108`, `:249-285`; `17-3c-early-start-flag-winston-decision.md:11-15`; `17-6-t2-sandbox-repair-and-proven-red.md:164-179`; `17-7-t3-worker-retained-lifecycle-and-terminal-facts.md:106-118`, `:358-375`, `:408-425`; `17-7-flag-winston-decision.md:46-59`.
- `_bmad-output/planning-artifacts/review-2026-09-04/preflight-r2/epic-17.md:129`, `:183` (the 2026-09-04 in-bridge rejection and its kernel-cost premise).
- No `project-context.md` matched the activation glob.

## Dev Agent Record

### Story context creation

Created 2026-10-17 by `anthropic/claude-opus-5-5` (bmad-create-story) with four read-only scouts, one isolated prototype agent and three non-author validators (§Non-author validation). Preflight 2026-10-18 by the same model with four read-only reviewers and four operator rulings (§Preflight). Creation evidence: `17-3d-evidence/`. The prototype and its probe are throwaway sizing instruments, not shipped code or tests.

### Agent Model Used

`anthropic/claude-opus-5-5` (bmad-dev-story, integrator), with seven same-model slice agents: KernelGrant (T1, T2), AdapterOrigin (T3), WasmHost (T0 baseline, T4, T5, recall-probe, `recall_wire` vectors), Docs (T10), Mediation (T6), TsScaffold (T7), RecallIntegration (T8 suite and the seven proven-red bundles). The integrator wrote T9, the line-2b checker and T11, and ran the full regression and CI route.

### Debug Log References

- **T0 drift check.** Dev HEAD = `context_commit` `39cd7966`; `git diff --quiet 39cd7966 -- crates wit spirits templates examples xtask .github` was clean, so no cite drifted and the §Delivery gate pins held. Frozen `@2.0` echo round trip on the base runner: `17-3d-evidence/t0-compat-baseline.log`. Reverse direction, a `@2.1` recall-probe on the base runner, exits 3 `InvalidComponent` at instantiation: `17-3d-evidence/t4-reverse-compat.log`.
- **Host limits.** cgroup2 is mounted read-only and there is no user systemd bus, so the delegated lane, exit lines 2/2b and the four-image runs went through the ratified `ci/17-3d-*` route (FW-10). Local T2 suites ran under `MAOS_17_2_CGROUP_MODE=undelegated`. Node 24.22.0 was provisioned outside the repo, because system Node 20 is below `engines`.
- **Kernel `--lib scheduler` filter.** 6 pass / 15 fail, the identical names at a `39cd7966` worktree (control_block tests need the monotonic base). Unfiltered `--lib` is 258/258 on both base and HEAD.
- **Full-workspace regression (`cargo test --workspace --no-fail-fast`).** Two regressions this story caused were fixed at origin. (1) `cohort_daemon_smoke_13_5c` lists every `maos-bin/src/*.rs`, so `recall_mediation.rs` was added to the manifest-scope negative. (2) `cross_team_consent_13_3` pins exactly one production `recall_cross_wall` caller: `MediatedLogRecall` no longer delegates cross-wall recall and keeps the trait's fail-closed default. The governance tests went green after the T11 re-pins. The `d11_xtask_ceiling_ratchet` reads only the `xtask` row's own inline grant, so that grant now cites `17-3d-wasm-log-recall` inline. Every remaining failure also fails, by exact test name, at a `39cd7966` worktree. They are environment-gated suites: T3/podman, delegated cgroups, Python Ed25519 twin, real-seccomp detector, `maos-shell` init, OTel air-gap canary.
- **Test side effect, reverted.** `intent_lineage_corpus_v0` appends a run block to `intent-lineage-coverage-report.md` on every workspace run. The append was reverted and that file is unchanged.
- **CI round 1** (run 39175953217 @ `8ccfb10b`). Exit lines 2 and 2b were green on both x64 images. `recall_17_3d` was red on all four images: the test hard-coded this host's target directory, and libtest summaries of re-executed children reached the log, so `check-log` saw more than one summary. Fixed by deriving the target dir from `current_exe()` and relaying child output with a `child| ` prefix. The shipped and throwaway name guards also now use `check-log --required-test`, because with `--nocapture` a `test … ok` line is not guaranteed to be contiguous.
- **CI round 2** (run 39177376846 @ `ce767ca3`). `recall_17_3d` was 16/16 on all four images. The throwaway job's `spawned_scope_17_3c` gate was 19/22 because that job lacked the shipped job's prerequisites (forbidden-syscall probes, `wasm-tools` 1.259.0). With them, run 39179015200 @ `344f956d` gave 22/22 on all four images.

### Completion Notes List

- **AC1 (command, exit line 2b): proven green / red / green in CI** (FW-10; public annotations archived in `17-3d-evidence/ci/`).
  - Positive: run 39177376846 @ `ce767ca3` (tree `fcd5cd6d`), `exit-line-2-2b` on ubuntu-24.04 and ubuntu-26.04. Line 2 rc=0 and `LINE-2B OK R1 log.recall -> R2 log.fetch -> R3 log.fetch.refused:scope-violation`.
  - Plant (scaffold `fetch` removed, `line2b-plant.diff` sha256 `649695e2…f9b4`): run 39177485729 @ `a450dbfd`. Line 2 still rc=0, line 2b red on both images (`recall-operation rows [('capability.invocation', 'log.recall')]`).
  - Restored (tree identical to the positive): run 39177591554 @ `7e6fd955`, green on both images.
  - The branch `ci/17-3d-verify` was deleted after capture.
  - AC1.4 own-frame content is proven by `own_frame_second_turn_recalls_kernel_minted_id_kind_intent_and_fetches_stored_payload`.
  - The real FR4 plain renderer output is checked against `scripts/epic_17_exit_line2b.py` (`17-3d-evidence/t8-fr4-plain-seed.txt`).
- **AC2 (`maos:spirit@2.1.0`).**
  - Root, template and example WIT are byte-identical; `interface frames` is unchanged; `interface recall` matches §WIT exactly, with `recall-error` final for 2.x.
  - Ratifications `17-3d-AC2-wit-spirit-2.1.0` and `17-3d-log-capability` landed; `wit_corpus` is 22/22.
  - The 4 pinned components were re-issued with private target dirs (`t4-reissue.log`; toolchain unchanged); `check-equiv-fixture-provenance` is green.
  - The compat guest runs under T2 on the `@2.1` runner: byte-identical frames, no tagged record (`recall_runner_17_3d`, 11/11 on four CI images).
- **AC3.1 transport.**
  - `recall_wire` uses tags 60000 (request) / 60001 (reply), byte-string `WireBytes`, `fit_page` (longest prefix searched downward), `encode_fetched_reply`, and `REPLY_BYTE_BOUND` = `MAX_FRAME_BYTES`.
  - The runner owns one persistent stdio pair in `HostState`, with closed and broken latches. Recall is available only inside `handle-frame`. A broken channel writes the export's frames and TurnComplete, then exits 6 `recall protocol desync`.
  - The session dispatches tagged bodies before the frame counter. `MAX_RECALLS_PER_TURN = 64`. Protocol violations write exactly one `recall.channel.closed:protocol-violation` receipt and end with a typed `SessionError::Protocol`.
  - No descriptor, socket, thread, syscall, `unsafe`, `libc` or dependency was added.
- **AC3.2 kernel.** Only `scheduler/{scheduler_loop,hook_dispatch,kernel_ctx}.rs` changed: the `log` literal field plus a deletion-only seam removal; every `SpiritSchedulerAdapter::new` caller was migrated. Physical 26311 → **26291**, tokei 20545 → **20527**, so no raise. `security/` and the T2 table are untouched; `t2_sandbox_kill` is 5/5.
- **AC3.3 mediation.**
  - `MediatedLogRecall` (`maos-bin/src/recall_mediation.rs`, `network`-gated) is the only mediation path, for both the WASM `serve_wire` and the Researcher's native port.
  - Per call: validate → `issue_enterprise_governed_capability` at the bound pid → attributed adapter call (token column) → revoke.
  - Every `Err` writes one kind-28 `log.<op>.refused:<code>` receipt; successes write none. `LogRecallError::CapabilityDenied` is typed.
- **AC3.4 at origin.** Covered by the AdapterOrigin vectors in `crates/maos-iac/tests/log_recall_17_3d.rs` (9) plus the T8 disclosure and cross-boot vectors:
  - SQL predicates bound before `LIMIT`: boot, the raw-kind allow-list {0–6, 18} and exact intent.
  - Metadata-only recall projection, and `fetch` checks the stored length first (`fetch_attributed_bounded` → `BoundedFetch::PayloadTooLarge`).
  - `idx_tlog_boot_pid_ts`, and `i64::MAX` bounds checked before minting.
- **AC3.5 grant.** `[capabilities.required.log] recall/fetch` on schema 5, dropped below 5, unknown keys refused. The coverage fixtures run 15 → 21. `check-manifest-schema-version` is green. The Researcher manifest moved to schema 5 with both grants, and its eleven prior scopes are preserved.
- **AC3.6 / AC3.7.** Covered by the wire vectors (6), runner lifecycle vectors (11) and the T8 reply-budget vector.
- **AC3.8 / AC4 / AC5.**
  - Docs and ADR notes landed (EN/KO, TOML blocks byte-identical; `cookbook_manifests_17_3b` 4/4; KO coverage 41/41).
  - The AC4.1 safety vector is labelled "withheld by emitter scope; no consent evaluation exists yet", and `consent-withheld` is reserved.
  - No latency number appears anywhere.
- **PF-08 Researcher.** The incremental walk keeps its cursor and fetched frames. `on_idle_recall_and_fetch_scale_with_new_rows_across_ticks` proves calls are O(new rows).
- **Proven-red (rule 11(c)).** Seven local bundles `17-3d-evidence/t8-proven-red/{a…g}-*.log`, each a real assertion RED then GREEN after exact restoration; the dirty-diff hash was restored (`b5f3f48c…1f45`). The CI exit-line plant is recorded above.
- **Test totals at HEAD.**
  - Local: `recall_17_3d` 16/16; `spawned_scope_17_3c` 22/22; `recall_mediation_17_3d` 14/14; `researcher_log_grant_17_3d` 2/2; `recall_runner_17_3d` 11/11; `recall_wire` 6/6; `ts_component_17_3b` 1/1 (`--ignored`, Node 24 component); researcher 42/42; maos-iac 132/132; maos-domain 310/310; `fr4_classifier_16_2` 17/17; maos-manifest 198/198; xtask full suite green.
  - CI (run 39179015200, four images): `recall_17_3d` 16/16, `spawned_scope_17_3c` 22/22, `recall_runner_17_3d` 11/11, `t2_sandbox_kill` 5/5, `wit_corpus` 22/22.
- **T11 governance.** These are Lunarpulse-approved FW-03 measured requests (2026-10-18), re-pinned in order after `cargo fmt --all`, tokei 14.0.0, `kloc-check --json` passed, aggregate 177596.
  - (1) `kernel-api-classes.toml`: `KernelCtx`/`HookDispatcher` class rows (`supervision`, inherited). The private-field reshape re-hashes both structs; no public symbol was removed in substance.
  - (2) The kernel-surface pin was regenerated; exactly 2 rows re-hash (those two structs, 399 items both sides).
  - (3) `kernel-core-baseline.toml` `src_lines` 26311 → 26291, three digests and `set_hash` updated, HISTORY added.
  - (4) `kernel_pin_content_hash_16_0.rs`: four live literals updated.
  - (5) `kloc.toml` (approved figures; maos-bin and aggregate landed lower than approved because the cross-wall delegation was removed):

    | Row | Before | After | Δ |
    |---|---:|---:|---:|
    | kernel | 20545 | 20527 | −18 |
    | maos-manifest | 4474 | 4493 | +19 |
    | maos-iac | 7100 | 7290 | +190 |
    | maos-domain | 9615 | 9628 | +13 |
    | maos-bin | 25917 | 26339 | +422 (approved 26348) |
    | maos-wasm-host | 1577 | 2253 | +676 |
    | `_aggregate_hardfail` | 176234 | 177597 | measured 177596 + 1; approved 177606 |

    xtask 44930 -> 44931 (+1, schema-inventory row).
  - (6) `RATIFIED_AT_EASING` 20_545 → 20_527.
  - (7) Both ratification entries.
  - Gates green: `check-kernel-baseline`, `check-service-boundary`, `check-empty-kernel`, `check-manifest-schema-version`, `check-equiv-fixture-provenance`, `templates-regen --check`, `kloc-check --json`, `check-exit-commands` (PASS, 15 tokens resolved, line 2b no longer owed), `cargo fmt --all -- --check`.
- **Deviations from the written contract.** None changes an AC.
  - (a) The receipt payload carries an optional `detail` (storage text for `unavailable`, the invalid-request detail) between `owner_pid` and `boot_nonce`, because AC3.3 keeps storage text in the receipt.
  - (b) `frame_id_hex` uses the journal's colon-separated byte-pair form (`researcher::encode_frame_id_hex`). The TL secret redactor scrubs contiguous 32-hex strings.
  - (c) Channel receipts are `{op:"channel",code:"protocol-violation",reason,boot_nonce}`.
  - (d) The Spirit-facing `MediatedLogRecall` has no cross-wall path (trait fail-closed default), which keeps Story 13.6d's single production caller.
  - (e) `recall_17_3d`'s probe manifest declares a 1 GiB memory cap for the undelegated lane's RLIMIT_AS fallback, because the recall component needs more address space than echo. It is still a real T2 child.
  - (f) The hostile-record vectors use a T2-contained `/bin/sh` ADR-032 child through the existing launch-plan seam; no production seam was added.
  - (g) `wasm-host-tests` now fetches standalone guest dependencies before the maos-wasm-host suites, which build `recall-probe` and the compat guest offline.
- **Not done here (owned elsewhere, no `deferred-work.md` row needed).** Participant recall and consent evaluation → `17-8-participant-recall-and-consent-envelopes`; issuance quota window → `17-9-capability-issuance-quota-window`; production recall latency → `20-7-stable-runner-perf-gates` AC4. The non-author §A6 review net remains the next gate (FLAG-Winston, security-sensitive).

### File List

New:
- `crates/maos-bin/src/recall_mediation.rs`
- `crates/maos-bin/tests/{recall_17_3d,recall_mediation_17_3d,researcher_log_grant_17_3d}.rs`
- `crates/maos-iac/tests/log_recall_17_3d.rs`
- `crates/maos-manifest/tests/log_capability_17_3d.rs`
- `crates/maos-kernel-core/tests/fixtures/manifest/capabilities/{well-formed,malformed-rejected,edge-case}/{log_recall,log_fetch}.toml`
- `crates/maos-wasm-host/src/recall_wire.rs`
- `crates/maos-wasm-host/tests/recall_runner_17_3d.rs`
- `crates/maos-wasm-host/guests/recall-probe/{Cargo.toml,Cargo.lock,src/lib.rs}`
- `crates/maos-wasm-host/guests/compat-echo-2-0/{Cargo.toml,Cargo.lock,src/lib.rs,wit/spirit.wit}`
- `spirits/researcher/tests/incremental_recall.rs`
- `scripts/epic_17_exit_line2b.py`
- `_bmad-output/implementation-artifacts/17-3d-evidence/{t0-compat-baseline.log,t4-reissue.log,t4-reverse-compat.log,t7-ts-scaffold.log,t8-recall-17-3d.log,t8-final-summary.log,t8-fr4-plain-seed.txt}`
- `_bmad-output/implementation-artifacts/17-3d-evidence/t8-proven-red/{a-disclosure,b-boot-scope,c-post-limit-kind-drop,d-token-attribution,e-refusal-receipt,f-vec-byte-sequence,g-tag-dispatch-order}.log`
- `_bmad-output/implementation-artifacts/17-3d-evidence/ci/{commits.txt,run-c1.json,run-c1b.json,run-c2.json,run-c3.json,run-c4.json,ci-17-3d-verify.yml.txt,ci-17-3d-verify-c4.yml.txt,line2b-plant.diff,line2b-plant-index.ts.txt}`

Modified:
- Kernel: `crates/maos-kernel-core/src/scheduler/{scheduler_loop,hook_dispatch,kernel_ctx}.rs`
- Kernel tests and benches: `crates/maos-kernel-core/tests/{abi_stability_admit,crash_detector_in_process_panic,hot_swap_same_major_lifecycle,kernel_ctx_round_trip,loaded_spirit_exit_16_6,manifest_field_coverage,on_revocation_three_actions,operator_posture_ceiling_16_6,progress_watchdog_smoke,provider_switched_journal,revocation_applier_pipeline,sandbox_admission,scheduler_five_verb_lifecycle,silent_failure_detector_smoke}.rs`, `crates/maos-kernel-core/tests/common/upgrade.rs`, `crates/maos-kernel-core/benches/{hot_swap_latency,revocation_propagation_p99}.rs`
- `crates/maos-manifest/src/{manifest,lib}.rs`
- `crates/maos-domain/src/log_recall.rs`
- `crates/maos-iac/src/adapter/{transparency_log,log_recall}.rs`
- `crates/maos-audit/src/fr4_classifier.rs`, `crates/maos-audit/tests/fr4_classifier_16_2.rs`
- `crates/maos-bin/src/{admission,cross_wall_log_read,lib,main,spirit_session}.rs`
- maos-bin tests: `crates/maos-bin/tests/{cohort_daemon_smoke_13_5c,cross_wall_log_read_13_6d,shell_host_16_2,spawned_scope_17_3c,two_host_delegation_2b,worker_crash_corpus_16_3,worker_hang_corpus_16_3,worker_supervision_16_3}.rs`, `crates/maos-bin/tests/worker_terminal_facts_17_7/{supervisor_scenes,unload_scenes}.rs`
- `crates/maos-wasm-host/src/{frame_bridge,host_state,lib,runner,wit_guest}.rs`
- maos-wasm-host tests: `crates/maos-wasm-host/tests/{frame_bridge_roundtrip,ts_component_17_3b,wit_corpus}.rs`
- Guests: `crates/maos-wasm-host/guests/echo-spirit/src/lib.rs`, `crates/maos-wasm-host/guests/equiv-fixture/{logic,wasm-guest}/src/lib.rs`
- Fixtures: `tests/fixtures/wasm/{echo_spirit,equiv_identity_spirit,equiv_divergent_spirit,equiv_cosmetic_spirit}_component.wasm`, `tests/fixtures/wasm/equiv-fixtures.provenance.toml`
- WIT: `wit/spirit.wit`, `templates/spirit-ts/wit/spirit.wit`, `examples/example-spirit-ts/wit/spirit.wit`
- TS template: `templates/spirit-ts/{manifest.toml,README.md,src/index.ts,tests/spirit.test.ts}`
- TS example: `examples/example-spirit-ts/{manifest.toml,README.md,src/index.ts,tests/spirit.test.ts}`
- Researcher: `spirits/researcher/{Cargo.toml,manifest.toml,src/lib.rs}`, `spirits/researcher/tests/{distillation_i11,five_metric_live_8_11,five_metric_self_verify,recall_walker,scalar_tap}.rs`; root `Cargo.lock` adds the existing SQLite crate as a test-only dependency for the clock regression.
- CI: `.github/workflows/discipline.yml`
- Docs: `docs-site/docs/{deploy/topology,cookbook/wasm-component-spirit,cookbook/manifest-fields,manifest/v5}.md`, their KO twins under `docs-site/i18n/ko/docusaurus-plugin-content-docs/current/`
- ADRs: `docs/adr/{ADR-013-log-recall-kernel-mediated-spirit-abi-capability,ADR-032-spirit-wire-protocol-bytes-on-wire,ADR-060-spirit-forms-by-trust-tier}.md`
- Governance: `docs/ci-baselines/kernel-surface-v0.1-beta.json`, `xtask/{abi-ratifications.toml,kernel-api-classes.toml,kernel-core-baseline.toml,kloc.toml}`, `xtask/src/check_manifest_schema_version.rs`, `xtask/tests/{kernel_pin_content_hash_16_0,recovery_lane_ceiling_rule}.rs`
- Story records: `_bmad-output/implementation-artifacts/{17-3d-wasm-log-recall.md,sprint-status.yaml}`

### Change Log

- 2026-10-17 — Story created (backlog, charter proposed): four scouts, prototype A/B, three non-author validators (all findings applied), epic §R13 R17-141…R17-155, decision record `17-3d-flag-winston-decision.md`.
- 2026-10-18 — Charter ratified by the operator (D-17-3d-FW-01…10; FW-07 A, FW-08 scaffold, FW-09 A); options materialized into one contract; `17-8-participant-recall-and-consent-envelopes` added (tracker, epic §17-8, DAG, index, inventory); epic R17-156; Status → ready-for-dev.
- 2026-10-18 — Party-mode preflight (long-term correctness): four reviewers, PF-01…PF-12; operator rulings R1 in-band stdio transport (ADR-060 amendment), R2 dormant seam deleted, R3 quota window → `17-9-capability-issuance-quota-window`, R4 cross-incarnation recall a non-goal (ADR-013 note); charter re-ruled (kernel no raise); epic §R14 R17-157…R17-164. Status stays ready-for-dev.
- 2026-10-18 — In-band transport prototyped in an isolated copy at the operator's request (`17-3d-evidence/inband/`): real kernel bridge under real T2, 6/6 scenarios, `t2_sandbox_kill` 5/5, `spawned_scope_17_3c` 22/22 with a stub dispatch; kernel unchanged, `unsafe` 0. Refinements folded into AC3.1, AC3.6, AC3.7, T5, the file map and the commands; ADR-060's EOF wording clarified; §In-band prototype added.
- 2026-10-19 — Dev pass (bmad-dev-story, `anthropic/claude-opus-5-5`): T0–T11 implemented.
  - Kernel −20 physical / −18 tokei (no raise). Out-of-kernel rows were re-pinned exactly under the operator's FW-03 approval (2026-10-18).
  - `recall_17_3d` 16/16 and `spawned_scope_17_3c` 22/22 on four CI images (run 39179015200). Exit line 2b went green / red (scaffold `fetch` removed) / green on both x64 images (runs 39177376846 / 39177485729 / 39177591554). Seven local proven-red bundles archived.
  - Status → review. The non-author §A6 net is next.
- 2026-10-19 — Non-author code review (`openai-codex/gpt-6.1-sol`): three parallel review layers plus parent test-infrastructure/runtime audit; 2 patches applied, 1 candidate dismissed, no deferrals. Fixed the Researcher's backward-clock lost-row regression with native append-order paging and added guarded wire-suite execution on Windows/macOS. Exact +13 out-of-kernel lines approved by Lunarpulse and re-pinned; kernel unchanged. Post-fix tests and runtime/CI-shell smoke passed; native non-Linux execution is not claimed. Status → done.
