# Story 8.13: Cross-Host Live Pair — Spirit→TCP Binding + Mobile Push (J4 end-to-end)

Status: done

<!-- Review complete 2026-06-08. P1–P4 resolved + verified green (see §Review Findings). P5 (cross-host consent-denial→ConsentRupture is hand-inserted) DISCLOSED as a PRE-EXISTING SYSTEMIC gap (no ConsentRupture emission exists anywhere in maos-a2a-core/maos-a2a-tcp; 8.5 loopback fakes it too) and carried to NEW story 8-13-1-genuine-cross-host-consent-denial-rupture-over-tcp via correct-course. 8.13's own new work (live TCP + real HTTP push + AC1 anti-tautology oracle) is genuine and green. The hand-inserted ConsentRupture row remains in the smoke until 8-13-1 lands — openly tracked, not silently shipped. -->
<!-- HONEST DISCLOSURE: smoke-mira-nash-tcp-8-13 still hand-inserts the ConsentRupture row ([main.rs:~6172]); its replacement-by-real-path is owned by 8-13-1. -->


<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->
<!-- Forks RESOLVED by party-mode preflight 2026-06-08 (Winston, Amelia, Murat, John — unanimous 4/4): FORK A → Option A (single-process, two distinct TcpA2ATransport over real loopback TCP); Option B (two `maos run` OS daemons) → its own follow-up story. FORK B → generic HTTP POST (ntfy-style), provider SDK banned from Tier-1. FORK C → keep `smoke-mira-nash-8-5` + add the TCP arm. Tier-2 → SPLIT (2a real-phone push human-signed; 2b two-OS-process when Option B lands); two-OS run NOT bundled into 8.13. NEW must-fix surfaced (Amelia + John, independently): `frame.from.host_id` MUST be derived from the transport's bound identity (== TLS peer), not hand-stamped as the loopback path did, or 8.9 `handle_intake_verified` rejects at intake and it misreads as a Nash bug — folded into AC1/AC2. See "Forks — RESOLVED" + the consensus rulings below. -->

## Story

As a v1.5 operator deploying Mira (Host A) and Nash (Host B),
I want Mira's **real** diagnosis to ride the **live mTLS wire** to Nash and a halt to reach my **phone**,
so that the J4 incident journey runs across two real endpoints end-to-end — not loopback with a test-double push.

> **Phase 2 (Epic-8 Completion Delivery); depends on 8.9 (TLS trust-binding) + 8.11 (daemon run surface).** This is the "two halves never meet" closure: Story **8.5** built the full J4 journey logic (diagnose → halt → mobile push → advisory → Nash → consent denial → three-tap → digest) but on an **in-process `LoopbackA2ARouter`** with a **test-double push**; Story **8.6** built the **live `maos-a2a-tcp` wire** but carries a **hand-built literal frame** with no real cognition. 8.13 composes 8.5 cognition + 8.6 transport + 8.9 secure identity, and ships the **first real mobile-push transport** (`maos-notify-push`, HTTP). **Zero kernel KLOC** — the A2A transport, halt machinery, and notification dispatch all already exist; this is Spirit + adapter + composition wiring. `maos-kernel-core/src` reverts to **byte-identical** (the 8.11/8.12 charter deltas do not recur here).

## Acceptance Criteria

1. **AC1 — Real Mira cognition on the live wire (replaces the hand-built literal).** The frame that rides the `maos-a2a-tcp` wire to Nash carries **`Mira::advisory(&Mira::diagnose(&signal))`** output, not the `smoke-a2a-tcp-8-6` hand-built `TaskAssignPayload` literal ([main.rs:5697-5722]):
   - Mira runs `diagnose()` ([spirits/mira/src/lib.rs:227]) on a fixture `AnomalySignal`, then `advisory()` ([:347]) → a `DiagnosticAdvisory`; the advisory is serialized (serde JSON) into the cross-Host frame payload **using the same wire contract the loopback journey already uses** (`smoke-mira-nash-8-5` serializes the advisory into the `TaskAssign.goal` field; Nash recovers it via `Nash::from_wire(&goal)` → `Nash::architect(...)` — [nash/src/lib.rs:145,157]). **Do not invent a second wire shape.**
   - Dispatch is through the **daemon-facing `A2ARouter` port** via `build_a2a_tcp_daemon_router(...)` ([main.rs:5551-5568]) → `TcpA2ATransport::route_outbound` ([maos-a2a-tcp/src/transport.rs:598]) — the SAME port the kernel Mailbox uses for cross-Host dispatch. No raw socket writes.
   - The fine-grained consent intent `ADVISORY_FINE_GRAINED_INTENT = "diagnosis-handoff:read-only-evidence"` ([mira/src/lib.rs:71]) is carried **end-to-end** and survives the **fail-closed** wire (Story 8.8 — an unclassified or band-only frame is denied `-32009`; no downgrade). `prepare_outbound` stamps `consent_envelope.valid_until_ns` from `peer_cfg.consent_ttl_secs` ([maos-a2a-core/src/router.rs:466-483]) — the real expiry path, not dead code.
   - **`from.host_id` must be transport-derived, NOT hand-stamped (party-mode must-fix — Amelia + John, write red first).** On the loopback path the 8.5 journey hand-stamped `frame.from.host_id` because binding was unchecked. Over real TCP the frame round-trips through 8.9 `handle_intake_verified` ([maos-a2a-core/src/router.rs:798]), which re-derives the peer from `peer_certificates()` and **rejects** any frame whose `from.host_id ≠ TLS-verified peer** (`-32007`). So the frame's `from.host_id` MUST be **derived from the sending transport's bound identity** and asserted equal to the TLS peer. If this is skipped, the TCP arm fails consent **at intake** — and it will *misread as a Nash/cognition bug* when it is a frame-stamping bug. This assertion doubles as the AC2 positive-binding check.
   - **Assertion (anti-tautology):** Nash's resulting `ArchitectureProposal` must be **derived from Mira's real finding** (its `proposed_fix`/`confidence` track the diagnosed `severity`), not a constant the test wrote — prove the cognition crossed the wire, not just bytes.

2. **AC2 — Runs over the 8.9-hardened TLS-verified peer binding (no bypass).** The inbound frame is admitted through `serve_connection` → `resolve_verified_peer` (re-derives identity from `peer_certificates()` — [transport.rs:431-476,512-520]) → `A2ARouterCore::handle_intake_verified` ([maos-a2a-core/src/router.rs:798]), **not** the bare `handle_intake`:
   - **Positive:** `frame.from.host_id == TLS-verified peer` → delivered; Nash observes the advisory (`last_intake_observed()` returns the boot_nonce + advanced Lamport clock, as `smoke-a2a-tcp-8-6` asserts — [main.rs:5730-5734]).
   - **Negative (confused-deputy regression guard, G8):** a frame whose `from.host_id` ≠ the TLS-verified peer is rejected with `CODE_PEER_IDENTITY_MISMATCH (-32007)` and `binding_passed == false` (no `intake_entered` increment). This must remain true with **real** Mira cognition in the payload (the 8.9 guarantee is payload-agnostic — prove it still holds on the live J4 path).
   - The smoke must route through the transport's verified path; a test that calls `handle_intake` directly (bypassing `handle_intake_verified`) is a **theater regression** and fails AC2.

3. **AC3 — Real mobile-push HTTP transport (`maos-notify-push`, generic POST) replacing the test-double.** NEW crate `crates/maos-notify-push` providing a real `NotificationChannel` whose `surface() == NotificationSurface::MobilePush` ([maos-domain/src/notification.rs:27-66]) and whose `dispatch()` performs a **real generic HTTP POST** of the serialized `NotificationEvent::Halt { payload }` (both `NotificationEvent` and `EpistemicHaltPayload` derive serde — [notification.rs:27], [frame.rs:197]):
   - **Generic HTTP POST, NOT a provider SDK (party-mode RESOLVED — unanimous).** A ntfy-style transport (operator-config **URL + token + JSON body**, targetable by ntfy / Pushover-message-API / Gotify / webhook with no code change) — **not** an FCM/Pushover SDK. HTTP push is the **v1.0 architecture mechanism** ([7-inter-agent-communication.md:120] "HTTP push at v1.0; native push at v1.5+"); native/SDK push is the v1.5+ deferral. A provider SDK pulls a vendor dep + auth dance (FCM = OAuth2 service-account token minting) + often an async runtime into a **synchronous** dispatch path — a poor fit and banned from Tier-1 (Murat: an SDK is unmockable hermetically without becoming in-process theater). Use **`ureq` v2** — the workspace-blessed, **synchronous** HTTP client ([maos-kernel-core/Cargo.toml:39]) matching the **synchronous** `NotificationChannel::dispatch` signature ([maos-director-surface/src/notification.rs:23-30]); **do NOT add `reqwest`/`hyper`** (async runtime bridge into a sync site = abi-diff/dep/kloc risk for zero AC value).
   - **Condition 1 — config provenance (8.7–8.9 never-trust-self-declared + 8.2 redaction):** URL, token, and body shape come from **host-side operator config** (env/config surface constructed in `maos-bin`), **never the Spirit manifest**; the token is **redacted** in `Debug`/tracing and any journaled frame (Story-8.2 redaction discipline — a secret never lands in a log line). Document the seam; full operator-managed provisioning is Epic 9 (same boundary 8.12's host-grant drew).
   - **Condition 2 — bounded timeout + typed error (mandatory):** the POST uses an **explicit** short `ureq` timeout (do not inherit the default) so a dead/slow endpoint **cannot hang the synchronous halt dispatch**; a push failure returns `Err(NotificationError::…)` (a typed error, never a swallow). The dispatcher already **isolates per-channel errors** (increments `report.errors`, still fans out to the terminal channel — [notification.rs:48-82]); halt delivery to other surfaces must never be blocked by a push outage.
   - This **replaces the `unimplemented!()` `MobilePushChannel` stub** ([maos-director-surface/src/notification.rs:270-285]). Implement the real channel in `maos-notify-push` and delete/retire the stub (or delegate) — **no `unimplemented!()` may remain on the live mobile-push path** (record the choice in Completion Notes).

4. **AC4 — Integrated J4 smoke over the LIVE TCP transport (the "two halves meet" gate).** NEW `MAOS_ONE_SHOT=smoke-mira-nash-tcp-8-13` arm (added to the dispatch table at [main.rs:~3947-3971]) = the **full `smoke_mira_nash_8_5` journey logic** ([main.rs:5122-5527]) — halt fires on Mira → real mobile push; advisory → Nash; deliberate consent denial → `ConsentRupture` in the TL; director three-tap resolution; morning digest cites the real `source_log_ref` — but with the two substitutions that make it real:
   - **`LoopbackA2ARouter` → two GENUINELY DISTINCT `TcpA2ATransport` endpoints** (the `smoke-a2a-tcp-8-6` setup: Nash server binds `127.0.0.1:0`, Mira client dials the readback addr, real TLS handshake + TOFU pins + `ca_roots` — [main.rs:5640-5728]). **Real-socket caveat (Winston + John — block review if violated):** the two endpoints must have **distinct identities and distinct TLS pins** — not one transport echoing to itself; the bytes must **leave through a kernel socket**, not an in-memory shortcut. If the wire is an in-process call, Option A collapses into "8.5 with extra steps" and the story proves nothing. Make this an explicit assertion (the 8.9 peer-binding is exercised: Host A's frame arrives at Host B and is validated against B's view of A's cert).
   - **`MobilePushCapture` → the real `maos-notify-push` transport** (AC3), pointed at a **real local mock HTTP server** spawned in-test (see AC6 anti-theater).
   - **Assertions:** advisory delivered over real TLS (`local_addr()` is a bound socket; boot_nonce + Lamport observed on the wire — proves the handshake ran); the **mock HTTP server received a well-formed POST** carrying the Halt payload; the full 8.5 journey (denial → `ConsentRupture`; three-tap → resolution journaled; digest cites real ref) completes against **real adapters**. This closes the gap where "8.5 logic + 8.6 wire never meet."

5. **AC5 — Process topology = Option A (party-mode RESOLVED — unanimous 4/4).** Deliver the **single integrated smoke process running two distinct `TcpA2ATransport` endpoints over real loopback TCP** (the 8.6 precedent — both halves genuinely meet on a real wire, hermetic). This fully satisfies AC1–AC4. Rationale (Winston/Murat/John): the integration risk lives on the **wire**, not the OS-process boundary; Option A exercises the *actual* TCP, mTLS/TOFU handshake, and `handle_intake_verified` peer-binding, and the user experiences J4 as "the wire + the phone," never as "two PIDs."
   - **Option B (two real `maos run` OS daemons) is OUT of scope — its own follow-up story.** It requires `classify_spirit(mira/nash)`, **composition-root TCP-transport wiring keyed on a manifest `[a2a]`/`[peers]` section** (does not exist today — [main.rs:534-1439]), AND a **Mira serving-loop trigger** (Observer-fed `AnomalySignal`, out of scope). Amelia: "3+ stories disguised as one"; the synthetic-trigger path is a phantom-debug day-killer; the new manifest section touches frozen-manifest `deny_unknown_fields` territory (7.5a). **Action (Winston, to John):** put the follow-up story **"Standalone-loadable Mira/Nash + two-daemon A2A run (Observer-triggered)"** on the board now so the Option-B value is sequenced, not lost (see Cross-Impact #1).
   - **Honest-disclosure (8.12 founder-class precedent — mandatory):** record in Completion Notes that "Mira/Nash remain non-standalone under `maos run`; the integrated J4 journey runs through the `smoke-mira-nash-tcp-8-13` arm." No silent amputation; the two-daemon run is captured as the **Tier-2b** gate item (AC6), not faked here.

6. **AC6 — Anti-theater + two-gate "presentable" (Murat + John, party-mode RESOLVED).** Tier-1 (CI default, unsigned, fully hermetic) carries the topology + push-path proof and **runs every CI pass** (bundling it behind a signed gate would leave the headline "two halves meet" AC unguarded — Amelia). **The Tier-1 hermetic assertion set — a green gate must be INCAPABLE of passing if any is violated (Murat's lie-test):**
   1. **Mock-server receipt (positive, socket-proven).** A real `TcpListener` mock receives a real HTTP `POST` from `maos-notify-push`; assert on the *received request* — method `POST`, expected path, and the J4 Halt/advisory payload present in the body (round-trips back via serde). In-process capture (`MobilePushCapture` short-circuit, [main.rs:5153-5175]) is **banned** in the integrated smoke (it may stay in the 8.5 loopback unit test). *Lie if absent:* "push works" with the HTTP layer never exercised.
   2. **Push guard trip-test (negative, mandatory).** Point the transport at a closed/refusing port (or a 500 server) and assert `dispatch` **surfaces** the failure (typed `Err`), does NOT swallow. *Lie if absent:* a fail-open push is indistinguishable from a working one on green CI.
   3. **Confused-deputy trip-test (negative — AC2's teeth).** Drive a frame across the live `TcpA2ATransport` where `frame.from.host_id ≠` the TLS-verified peer; assert `handle_intake_verified` rejects with **`-32007`** and does not enter intake. This is the *only* assertion proving the wire re-derived identity from the TLS session vs. trusting the frame's self-claim. *Lie if absent:* the "real TLS" arm could transport bytes while trusting attacker-claimed identity — the exact confused-deputy 8.9 closed.
   4. **No-egress assertion (hermeticity proof).** Zero connections off-loopback: only `127.0.0.1`/`::1`, **no DNS resolution of a provider hostname**, no real-provider endpoint reachable in config (Tier-1 push base URL points at `127.0.0.1:<mock-port>`). The guard must itself have a **trip-test** proving it fires on a real external endpoint (a guard with no failure-mode test is decoration — 8.12 scar). *Lie if absent:* a misconfigured base URL silently hits the real provider and the gate "passes" because the phone happened to be online.
   5. **XDG_DATA_HOME isolation (8.11 carry-forward).** Any arm touching the journal isolates `XDG_DATA_HOME` (even single-process — the smoke writes journal; cross-CI contamination otherwise).
   - **Real-TLS proof:** delivery asserted to have crossed `TcpA2ATransport` (real handshake) via `local_addr()` bound socket + boot_nonce/Lamport readback — not an in-process route (overlaps AC4 real-socket caveat).
   - **Tier-2 SPLIT into two signed gates (party-mode RESOLVED — different failure domains, different signers; do NOT bundle):**
     - **Tier-2a — real-device push (OPEN; this story's release-gate item).** One real push via a real provider (ntfy / Pushover / FCM) → **human-confirmed** receipt on a real phone. Signs exactly one claim: *a real push reached a real phone.* **Owner = a deployer/operator who can physically hold the phone and attest** (a named human, not an automated stamp), with **Winston co-signing transport correctness.** Reproducible from the same generic-POST operator config a real operator would write; the checklist records provider + redacted endpoint + observed-buzz. Recorded at `_bmad-output/test-artifacts/release-gate-8-13-tier-2-mobile-push.md`.
     - **Tier-2b — two-OS-process A2A run (deferred to the AC5 Option-B follow-up story).** Proves the real process-boundary daemon dispatch path; gated by *that* story when it lands (it carries the composition-root A2A wiring + Observer trigger). **NOT an 8.13 gate item** — you cannot gate a release on a capability the codebase does not yet have (Winston).

7. **AC7 — Zero-kernel-KLOC + workspace discipline.**
   - **`maos-kernel-core/src` BYTE-IDENTICAL.** This story reverts to the 8.5 zero-KLOC posture — the A2A transport (8.6), halt machinery (8.5), and notification dispatch (3.3/8.5) all exist. Assert `git diff --stat crates/maos-kernel-core/src` is **empty**; the post-8.12 baseline (16263) is unchanged. **No** LLM/inference/provider/orchestration type enters kernel-core. If any task tempts a kernel edit, that is a **scope error** — surface it.
   - **Workspace 42 → 43** (NEW `crates/maos-notify-push`). Bump root `Cargo.toml [workspace] members` **AND** the `<!-- workspace-count-authoritative -->` sentinel at [4-kernel-design.md:115] (42 → 43) **AND** the `check-workspace-count` floor — **in lockstep, same commit** (the 8.1/8.6/8.11 sentinel discipline).
   - **`maos-notify-push` dependency boundary:** depends only on `maos-director-surface` (the `NotificationChannel` trait) + `maos-domain` (event types) + `ureq`. **No kernel-core, no `maos-a2a`** dependency. Avoids a cycle (the trait lives in `maos-director-surface`).
   - **Do NOT edit `maos-a2a`** (over its 1500-LOC ceiling per [[project_story_8_5_spec_landed]]) **or `maos-kernel-core`.** `abi-diff --base abi-baseline/v1-pre-bump.txt` **Added-only** (frozen `maos-spirit-abi` byte-untouched — the bare `abi-diff` "breaking" is the no-base false-positive, Story-8.3 lesson). `kloc-check` aggregate-RED (20 KLOC) is **pre-existing + neutral** — the new adapter crate crosses no new per-crate ceiling; **do not bump the ceiling**. `cargo fmt -p <crate>` is **banned** (7.5a whole-crate collateral).

## Tasks / Subtasks

> **Sequencing (build the new adapter in isolation BEFORE wiring the integrated smoke — the 8.12 lesson: prove the piece, then compose):** T1 → T2 → T3 → T4 → T5.

- [x] **T1 — `maos-notify-push` crate, in isolation (AC3, AC7)**
  - [x] NEW `crates/maos-notify-push/{Cargo.toml,src/lib.rs}`: a `MobilePushHttp` struct holding the operator-config endpoint URL + (redacted) auth token; impl `NotificationChannel` (`surface()=MobilePush`, sync `dispatch()` → `ureq` POST of `serde_json::to_vec(&NotificationEvent::Halt{..})` with a **bounded timeout**).
  - [x] Config provenance: URL+token from a host-side `PushConfig` (constructed in `maos-bin` from env/operator config, NEVER the manifest); token redacted in `Debug`/tracing (Story-8.2 discipline).
  - [x] Retire the `unimplemented!()` `MobilePushChannel` stub ([maos-director-surface/src/notification.rs:270-285]) — implement-real-and-delete, or delegate. No `unimplemented!()` on the live path.
  - [x] Red→green unit test against a **real local mock HTTP server** (`std::net::TcpListener` reading one request): asserts the POST body deserializes back to the dispatched Halt payload; a second test asserts a **bounded-timeout failure** returns `Err` (not a hang) and does not panic.
  - [x] Workspace 42→43: members + sentinel ([4-kernel-design.md:115]) + `check-workspace-count` floor, same commit; `cargo build -p maos-notify-push` green.

- [x] **T2 — Compose real Mira cognition onto the TCP wire (AC1, AC2)**
  - [x] In the new smoke (T4) — or a shared helper — replace the 8.6 hand-built literal: `let diag = mira.diagnose(&signal); let adv = mira.advisory(&diag);` serialize `adv` into the `TaskAssign.goal` (the 8.5 wire contract); route via `build_a2a_tcp_daemon_router(...).route_outbound(...)`.
  - [x] **WRITE RED FIRST (party-mode must-fix):** `frame.from.host_id` derived from the sending transport's bound identity, asserted `== TLS-verified peer` — NOT hand-stamped as the loopback path did (else 8.9 `handle_intake_verified` rejects at intake; would misread as a Nash bug).
  - [x] Assert the fine-grained intent + consent-expiry (`prepare_outbound` stamping) survive the fail-closed wire (8.8); Nash recovers via `from_wire` → `architect`; the `ArchitectureProposal` tracks Mira's real `severity` (anti-tautology).
  - [x] AC2 positive + negative (confused-deputy `-32007`, AC6 assertion #3) over the real transport with the real payload; assert the path goes through `handle_intake_verified` (no `handle_intake` bypass).

- [x] **T3 — Wire the real push into the halt dispatch (AC3, AC4)**
  - [x] Register `MobilePushHttp` (pointed at the mock server) in the journey's `NotificationDispatcher` in place of `MobilePushCapture`; assert `dispatch_halt` ([halt_ui.rs:54-65]) → `report.delivered == 1` AND the mock server received the POST.
  - [x] Assert per-channel error isolation: with a deliberately-dead push endpoint, the terminal channel still delivers (`report.errors` increments, journey continues).

- [x] **T4 — Integrated `smoke-mira-nash-tcp-8-13` (AC4, AC5, AC6) — LAST of the build**
  - [x] NEW one-shot arm = full `smoke_mira_nash_8_5` journey logic with `LoopbackA2ARouter`→two `TcpA2ATransport` (8.6 two-endpoint setup) + `MobilePushCapture`→`maos-notify-push`→mock server. Add to the dispatch table.
  - [x] Preserve every 8.5 journey step: halt+push, advisory→Nash, deliberate denial→`ConsentRupture`, three-tap resolution, digest-cites-real-ref. Assert advisory crossed real TLS (boot_nonce+Lamport), POST received, journey completes against real adapters.
  - [x] **Keep the 8.5 loopback `smoke-mira-nash-8-5` green** (hermetic-fast regression) — the new arm is the integrated journey, not a replacement (unless party-mode rules otherwise — AC5 fork). `smoke-a2a-tcp-8-6` stays green.
  - [x] AC5 topology: deliver Option A; if Option B (two `maos run` daemons) deferred, log the honest-disclosure note + capture for Tier-2.

- [x] **T5 — Two-gate + discipline (AC6, AC7)**
  - [x] Tier-1 hermetic assertion set (AC6 #1–#5): mock-server receipt; push guard trip-test; confused-deputy `-32007` trip-test; no-egress assertion **+ its own trip-test**; `XDG_DATA_HOME` isolation. Real-TLS proof (bound `local_addr` + boot_nonce/Lamport).
  - [x] **Tier-2a** release-gate authored at `_bmad-output/test-artifacts/release-gate-8-13-tier-2-mobile-push.md` (OPEN — named human operator signs real-phone receipt + Winston co-signs transport; records provider + redacted endpoint + observed-buzz). **Tier-2b** (two-OS-process) explicitly deferred to the AC5 Option-B follow-up story — note it, do not gate 8.13 on it.
  - [x] Kernel byte-identity assertion (`git diff --stat crates/maos-kernel-core/src` empty); `abi-diff --base` Added-only; workspace-count green at 43; no `cargo fmt -p`; `maos-a2a`/`maos-kernel-core` untouched.

### Review Findings (code review 2026-06-08)

Layers: Blind Hunter · Edge Case Hunter · Acceptance Auditor · Test Infrastructure Auditor (dev model `openai/gpt-5.5` → TIA added per policy). All 4 layers completed; no layer failures.

**Decision findings — RESOLVED by party-mode consensus 2026-06-08 (Winston, Amelia, Murat, John — unanimous 4/4, per spec + long-term correctness). Both → PATCH. Handling strategy (2nd consensus, unanimous): SPLIT — mechanical/safe in-session, substantive red-first carried out; P5 reframed as `correct-course` (John).**

**Resolution status (verified 2026-06-08): P1–P4 applied in working tree, all green; P5 carried to a NEW `correct-course` story.**

- [x] [Review][Patch] **D1 / P4 — receiver-side wire-content oracle — DONE & VERIFIED.** Implemented in the smoke via the pre-existing `A2ARouterCore::install_intake_sink` hook (`nash.core().install_intake_sink(intake_tx)`, [main.rs:6078-6079]): the smoke pulls the frame Nash actually RECEIVED off the wire (`intake_rx.try_recv()` → `received_goal`, [:6124-6135]) and feeds its RAW goal bytes through `Nash::from_wire(&received_goal)` ([:6140]); severity/proposal now derive from the RECEIVED advisory, not the test-local copy. Raw-bytes-as-received invariant honored (captured pre-parse; never re-serialized). **Zero edits to `maos-a2a-tcp`/`maos-a2a-core`/`spirits/nash`/kernel** (git status confirms; even cleaner than the planned `last_intake_payload()` accessor). Smoke exits 0.
- [ ] [Review][CorrectCourse] **D2 / P5 — drive a REAL consent denial → ConsentRupture over the wire — CARRIED TO NEW STORY (`correct-course`).** Still self-fulfilling at [main.rs:6163-6196]: the smoke hand-inserts the advisory (`ConsentRequest`) + `ConsentRupture` rows then asserts they exist. Per John's ruling this is NOT a simple patch — it presumes the production path emits the rupture on a classified-policy-deny over TCP, which is unproven (we hand-inserted *because* it was unproven). **Acceptance bar (Murat): consent decision may be fixtured; the rupture must be EARNED** — delete the hand-insert, drive a real classified-but-denied frame over TCP, assert on the production-written row. **Escalation: if intake rejects WITHOUT emitting a ConsentRupture on classified-policy-deny over TCP, that is a genuine production gap.** → spun into its own `correct-course` story so the red phase runs honestly and a production gap routes properly. Source: testinfra.
- [x] [Review][Patch] **P1 — connect-phase timeout unbounded — DONE & VERIFIED.** Added `.timeout_connect(self.config.timeout)` to the `AgentBuilder` ([lib.rs:93]) with ureq-2.12.1 precedence comment; backed by red-first test `dispatch_to_blackhole_endpoint_is_bounded_not_hung` ([lib.rs:235]) that exercises the connect-STALL the prior refused-only negative missed. Green.
- [x] [Review][Patch] **P2 — halt POST follows up to 5 redirects — DONE & VERIFIED.** Added `.redirects(0)` ([lib.rs:94]); backed by `dispatch_does_not_follow_redirects` ([lib.rs:258]). Green.
- [x] [Review][Patch] **P3 — `assert_loopback_url` IPv6 `[::1]` branch broken — DONE & VERIFIED.** Bracketed-IPv6 authority now peeled off before the `:`/`/` split ([main.rs:5841-5851]); smoke adds positive (`[::1]` accepted) + negative (`[2001:db8::1]` rejected) trip-tests ([main.rs:5919-5923]). Green.
- [x] [Review][Defer] Mock push server is single-shot / thread never joined / panic surfaces only as `recv_timeout` disconnect [`crates/maos-bin/src/main.rs:83`, `crates/maos-notify-push/src/lib.rs:134`] — deferred, test robustness only (one POST expected; failure still fails the test, just with a generic cause).
- [x] [Review][Defer] Subprocess smoke has no outer timeout — a hang blocks rather than fails [`crates/maos-bin/tests/smoke_mira_nash_tcp_8_13.rs:15`] — deferred, internal ops are individually bounded (push 2s, recv 2s, handshake 30s); a future unbounded await would hang CI.

## Dev Notes

### What this story is (and is NOT)

This is the **integration spine for J4**, not a new reference Spirit and not kernel work. Mira, Nash, the halt machinery, the A2A TCP transport, and the notification dispatcher **all already exist and must be reused, not rebuilt.** The only genuinely new code is: (1) a small **`maos-notify-push` HTTP adapter** (the real mobile-push transport, replacing an `unimplemented!()` stub), and (2) an **integrated smoke** that wires 8.5's journey logic onto 8.6's live wire with the real push. Everything else is composition.

| Substrate (REAL — reuse) | Location |
| --- | --- |
| `TcpA2ATransport` (impls `A2ARouter` + `A2ATransport`; `bind`/`route_outbound`/`local_addr`/`last_intake_observed`) | `crates/maos-a2a-tcp/src/transport.rs:140,598,640` |
| **Daemon-facing TCP router builder** (the seam to reuse) | `crates/maos-bin/src/main.rs:5551-5568` `build_a2a_tcp_daemon_router` |
| 8.6 two-endpoint live-TCP setup (Nash server :0, Mira client dials readback, TOFU+ca_roots) | `crates/maos-bin/src/main.rs:5576-5747` `smoke_a2a_tcp_8_6` (the **hand-built literal** to replace: `:5697-5722`) |
| 8.9 hardened intake (`handle_intake_verified`, peer re-derive from `peer_certificates()`) | `maos-a2a-core/src/router.rs:798`; `maos-a2a-tcp/src/transport.rs:431-476,512-520` |
| `prepare_outbound` consent-expiry stamping (the real `valid_until_ns` path) | `maos-a2a-core/src/router.rs:417-491` (stamp at `:466-483`) |
| `Mira::diagnose` / `advisory` / `halt_payload` | `spirits/mira/src/lib.rs:227,347,285` |
| Mira fine-grained intent const | `spirits/mira/src/lib.rs:71` `ADVISORY_FINE_GRAINED_INTENT` |
| `Nash::from_wire` / `architect` (the serde wire contract — no crate coupling) | `spirits/nash/src/lib.rs:145,157` |
| **Full J4 journey logic** (halt→push, advisory→Nash, denial→ConsentRupture, three-tap, digest) | `crates/maos-bin/src/main.rs:5122-5527` `smoke_mira_nash_8_5` |
| `NotificationChannel` trait (**sync** `dispatch`) + `NotificationDispatcher` (fan-out, per-channel error isolation) | `crates/maos-director-surface/src/notification.rs:23-30,48-82` |
| `dispatch_halt` (Halt → `NotificationLevel::Immediate`) | `crates/maos-director-surface/src/halt_ui.rs:54-65` |
| `NotificationEvent::Halt` / `Surface::MobilePush` / `Level` (all serde) | `crates/maos-domain/src/notification.rs:27-66` |
| `EpistemicHaltPayload` (serde — the POST body) | `crates/maos-domain/src/frame.rs:197` |
| **`MobilePushChannel` stub (`unimplemented!()`) — REPLACE** | `crates/maos-director-surface/src/notification.rs:270-285` |
| `MobilePushCapture` test-double — RETIRE from the integrated smoke | `crates/maos-bin/src/main.rs:5153-5175` |
| HTTP client (workspace-blessed, **sync**) | `ureq` v2 — `crates/maos-kernel-core/Cargo.toml:39` |
| `classify_spirit` (Mira/Nash **ABSENT** — not `maos run`-loadable) | `crates/maos-bin/src/main.rs:197-205` |
| Smoke dispatch table (`mode == "smoke-…"`) | `crates/maos-bin/src/main.rs:~3937-3971` |
| Workspace-count authoritative sentinel | `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md:115` |
| HTTP push = the v1.0 mobile mechanism | `architecture-maos-minimal-opus/7-inter-agent-communication.md:120` |

> **There is NO daemon A2A-transport wiring today.** `maos run`'s composition root ([main.rs:534-1439]) constructs no A2A router; `build_a2a_tcp_daemon_router` is used **only inside `smoke_a2a_tcp_8_6`**. Option A (default) reuses that builder in the integrated smoke without touching the composition root. Option B (two daemons) would have to add composition-root wiring — that is the AC5 fork.

### The "two halves never meet" gap (read carefully)

- **8.5** (`smoke-mira-nash-8-5`, [main.rs:5122-5527]) = the **full journey** (real halt, real three-tap, real digest, advisory→Nash) but over **`LoopbackA2ARouter`** (in-process, [main.rs:5336]) with **`MobilePushCapture`** (in-process capture, [main.rs:5153-5175]). Real logic, simulated transport + push.
- **8.6** (`smoke-a2a-tcp-8-6`, [main.rs:5576-5747]) = the **real TLS wire** (two `TcpA2ATransport`, real handshake, TOFU pins) but carries a **hand-built literal `TaskAssignPayload`** ([main.rs:5697-5722]) — no diagnose, no halt, no push, no journey.
- **8.13** = 8.5 journey logic running on 8.6's transport with a real push. The single most important deliverable is the **integrated smoke** (AC4) that proves cognition + wire + push are simultaneously real.

### Why kernel byte-identity holds (and the trap if it doesn't)

8.11 and 8.12 were charter-amended kernel deltas (daemon spine; CliWrapper bridge). 8.13 needs **none of that**: the transport is 8.6, the halt is 8.5, the dispatcher is 3.3/8.5. The new push transport is an **adapter** (`maos-notify-push`) registered from `maos-bin`; the kernel receives nothing new. If a task seems to require editing `maos-kernel-core/src`, stop — it is almost certainly composition that belongs in `maos-bin` or the new adapter crate (the Winston trip-wire from 8.11/8.12: kernel-core never wires transports or reads manifests to decide topology).

### Files being modified (UPDATE) — preserve existing behavior

- `crates/maos-bin/src/main.rs` — NEW `smoke-mira-nash-tcp-8-13` one-shot arm + dispatch entry; reuse `build_a2a_tcp_daemon_router` + the 8.5 journey logic. **Preserve:** `smoke-mira-nash-8-5` and `smoke-a2a-tcp-8-6` byte-stable (both stay green); the 8.11/8.12 single-Spirit load/serving/`--once` paths untouched. (AC5 Option B only: `classify_spirit` + composition-root A2A wiring — flag first.)
- `crates/maos-director-surface/src/notification.rs` — retire/replace the `unimplemented!()` `MobilePushChannel` stub ([:270-285]). Keep the `NotificationChannel` trait + `NotificationDispatcher` byte-stable (don't change the trait shape — `maos-notify-push` implements the existing one).
- Root `Cargo.toml` — add `crates/maos-notify-push` to `members` (42→43).
- `architecture-maos-minimal-opus/4-kernel-design.md:115` — sentinel 42→43 (lockstep with members + gate floor).

### Project Structure Notes

- Workspace **42 → 43**. `maos-cli` and `maos-mcp` already exist as members (scaffolded for 8.14a/b, not yet "done") and are counted in the 42 — do not double-count; the delta is exactly `+maos-notify-push`.
- `maos-notify-push` is an **adapter crate** (Spirit/transport layer), not kernel — keep it dependency-light (`maos-director-surface` + `maos-domain` + `ureq`). No `maos-a2a`/`maos-kernel-core` dep (cycle-free; the `NotificationChannel` trait lives in `maos-director-surface`).
- `kloc-check` aggregate ceiling (20 KLOC) is already breached pre-story (decomposition-in-flight RED — 8.11/8.12 documented it). The new adapter crosses **no new per-crate ceiling**; same neutral-RED posture; **do not bump the ceiling**.
- No new discipline gate required (reuse `kloc-check`/`abi-diff`/`check-workspace-count`).

### Forks — RESOLVED (party-mode preflight 2026-06-08: Winston, Amelia, Murat, John — unanimous 4/4)

- **FORK A (AC5 — process topology) → Option A (RATIFIED).** Single integrated smoke, two **distinct** `TcpA2ATransport` over real loopback TCP (8.6 precedent; both halves genuinely meet on a real wire; hermetic). **Option B (two `maos run` OS daemons) → its own follow-up story** — it pulls in `classify_spirit(mira/nash)` + composition-root A2A `[a2a]`/`[peers]` wiring (frozen-manifest territory, 7.5a) + an Observer-fed Mira trigger (out of scope). Winston: "the integration risk lives on the wire, not the process boundary." Amelia: "3+ stories disguised as one." John: "the user never sees the second process — they see the phone." **Real-socket caveat (block review if violated):** distinct identities + distinct TLS pins; bytes leave through a kernel socket, not an in-memory shortcut (folded into AC4/AC5).
- **FORK B (AC3 — push provider) → generic HTTP POST (RATIFIED).** ntfy-style operator-config URL + token + JSON body via sync `ureq`; **provider SDK banned from Tier-1** (Murat: unmockable hermetically without in-process theater; Winston: SDK auth dance is a poor fit for a sync dispatch; §7:120 defers native push to v1.5+). Conditions folded into AC3: config from host not manifest, token redacted, explicit bounded timeout + typed error. Retires the §6.5 `unimplemented!()` stub.
- **FORK C (AC4 — loopback smoke fate) → keep both (RATIFIED).** Keep `smoke-mira-nash-8-5` (fast, flake-immune, brackets *journey logic*) AND add `smoke-mira-nash-tcp-8-13` (brackets *transport*). Differential diagnosis for free: 8.5-red+8.13-red → logic bug; 8.5-green+8.13-red → transport/binding/push bug. 8.5 staying green is also the pure-refactor guard that 8.13 didn't change the journey.
- **Tier-2 → SPLIT (RATIFIED).** Tier-2a = real-phone push (human-signed + Winston co-sign) = this story's OPEN gate item. Tier-2b = two-OS-process run = deferred to the Option-B follow-up story; **NOT** an 8.13 gate. Tier-1 (hermetic, every-CI) carries the topology proof and must not be bundled behind a signed gate (folded into AC6).

### Lessons from prior Epic-8 stories (apply)

- **8.12:** anti-theater must be **spawn/socket-or-fail** — a "push" that never touches a socket is theater; point the real HTTP transport at a real mock server and assert receipt. Every guard needs a trip-test. Every subprocess test isolates `XDG_DATA_HOME` (8.11 journal-corruption).
- **8.12 founder-class:** honest-disclosure when a Spirit is not standalone-loadable under `maos run` — say so, route through the smoke, don't fake a daemon (the AC5 Option-A disclosure pattern).
- **8.5:** the bilateral pair is **zero kernel KLOC** over real adapters as dev-deps — the same posture 8.13 returns to. Loopback peer lookup keys `HostId == peer_id`.
- **8.7–8.9:** never trust a self-declared field — the push **endpoint/token come from host config, not the manifest**; the 8.9 `handle_intake_verified` binding must not be bypassed.
- **8.2:** the redaction trap — secrets (the push auth token) must never land in a log/cite; scrub pre-write.
- **8.6:** `maos-a2a` is over its LOC ceiling — **do not edit it**; land A2A work in `maos-a2a-core`/`maos-a2a-tcp` (here: no A2A edit needed at all).
- **7.5a:** `cargo fmt -p <crate>` is **banned** (whole-crate collateral). **Epic-7 scar:** never flip a gate green while red; never `#[ignore]` a load-bearing test.

### Latest tech / library notes

- **`ureq` v2 (sync) is the right fit.** `NotificationChannel::dispatch` is **synchronous** ([notification.rs:23-30]) — a blocking `ureq` POST matches the trait with no async bridging. Set `ureq::AgentBuilder::timeout(...)` (or per-request timeout) so a dead endpoint can't hang halt dispatch. The dispatcher's per-channel error isolation ([notification.rs:48-82]) means a push `Err` won't block the terminal surface.
- **`NotificationEvent` + `EpistemicHaltPayload` both derive serde** ([notification.rs:27], [frame.rs:197]) — `serde_json::to_vec(&NotificationEvent::Halt{payload})` is the POST body; the mock server deserializes it back to assert fidelity.
- **Mock HTTP server for hermetic Tier-1:** a one-shot `std::net::TcpListener` that reads the HTTP request and captures the body is sufficient (no test HTTP-server dep needed). Bind `127.0.0.1:0`, hand the readback URL to `MobilePushHttp` — the exact pattern the 8.6 TCP smoke uses for the wire.
- **No new heavy deps:** do not add `reqwest`/`hyper`/`axum`. `ureq` (client) + raw `TcpListener` (mock) keep the dep graph and the kloc honest.

### Testing standards

- Per-crate `cargo test`; the integrated journey is a subprocess smoke under `crates/maos-bin/tests/smoke_*` (`Command::new(env!("CARGO_BIN_EXE_maos-bin"))`, CWD = workspace root, JSON-on-stdout, **isolated `XDG_DATA_HOME`** — no exceptions).
- `maos-notify-push` unit tests prove the POST reaches a real socket and the body round-trips; a bounded-timeout test proves no-hang on a dead endpoint.
- Keep `smoke-mira-nash-8-5` and `smoke-a2a-tcp-8-6` green (regression). The 8.9 trust-binding tests (`maos-a2a-tcp/tests/trust_binding_8_9.rs`) must stay green — AC2's negative path is the same `-32007` guarantee on the live J4 payload.
- Report the integrated journey as a runnable, observable demo (`MAOS_ONE_SHOT=smoke-mira-nash-tcp-8-13 ./target/debug/maos-bin`) — frame validation around the end-to-end run, per [[feedback_lunarpulse_observability_preference]].

### References

- [Source: epic-8-…md:451-463] — Story 8.13 AC sketch (AC1 real cognition on wire; AC2 8.9 binding; AC3 `maos-notify-push` HTTP; AC4 integrated J4 smoke); [:177-186] 8.5 J4 ACs (halt→mobile push, Nash via typed-intent consent, three-tap); [:383,389] DAG + per-journey gate (J4 = 8.9 + 8.11 + 8.13).
- [Source: _bmad-output/implementation-artifacts/8-12-…md] — predecessor pattern: anti-theater (socket-or-fail), two-gate Tier-1/Tier-2, guard trip-test, host-config-not-manifest seam, honest-disclosure for non-loadable spirits, kernel-discipline checklist, no `cargo fmt -p`.
- [Source: _bmad-output/implementation-artifacts/8-5-…md, 8-6-…md, 8-9-…md] — Mira/Nash journey + corpus (8.5); live TCP/mTLS transport + `build_a2a_tcp_daemon_router` (8.6); `handle_intake_verified` trust-binding + `prepare_outbound` consent expiry (8.9).
- [Source: crates/maos-a2a-tcp/src/transport.rs:140,431-476,512-520,598,640; maos-a2a-core/src/router.rs:417-491,798] — bind, serve_connection, resolve_verified_peer, route_outbound, handle_intake_verified, prepare_outbound.
- [Source: spirits/mira/src/lib.rs:71,227,285,347; spirits/nash/src/lib.rs:145,157; spirits/mira/tests/halt_bilateral.rs] — diagnose/advisory/halt_payload, fine-grained intent, from_wire/architect, the AC4 halt+push integration test (currently test-double push).
- [Source: crates/maos-bin/src/main.rs:197-205,3937-3971,5122-5527,5551-5568,5576-5747,5153-5175] — classify_spirit, dispatch table, smoke_mira_nash_8_5, build_a2a_tcp_daemon_router, smoke_a2a_tcp_8_6 (+ the hand-built literal), MobilePushCapture.
- [Source: crates/maos-director-surface/src/notification.rs:23-30,48-82,270-285; halt_ui.rs:54-65; maos-domain/src/notification.rs:27-66; frame.rs:197] — NotificationChannel trait, dispatcher fan-out/error-isolation, MobilePushChannel stub, dispatch_halt, event/surface/payload serde.
- [Source: architecture-maos-minimal-opus/7-inter-agent-communication.md:120; 13-phased-roadmap.md:15,33; 4-kernel-design.md:115] — HTTP push = v1.0 mechanism; v1.5 mobile-friendly approval surface; workspace-count sentinel.
- [Source: crates/maos-kernel-core/Cargo.toml:39] — `ureq` v2 (workspace HTTP client).

## Cross-Impact — what else these decisions touch

> Flag-list for the team; items marked **(action)** need a follow-up or confirming decision.

1. **NEW follow-up story owed — "Standalone-loadable Mira/Nash + two-daemon A2A run (Observer-triggered)"** (AC5 Option B, party-mode RESOLVED out of 8.13). Pulls in `classify_spirit(mira/nash)`, a daemon `[a2a]`/`[peers]` composition-root config surface, and an Observer-fed Mira trigger; carries the **Tier-2b** gate. **(action: John — put it on the board NOW so the Option-B value is sequenced, not lost; Winston owns the wire + config surface.)**
2. **`maos-notify-push` operator config (URL/token)** — 8.13 lands the minimal host-side seam; **operator-facing provisioning is Epic 9** (same boundary 8.12's host-grant drew). **(action: confirm the 8.13 seam vs. Epic-9 surface boundary.)**
3. **`MobilePushChannel` §6.5 stub** — retiring the `unimplemented!()` partially closes the Story-6.5 gateway gap for the MobilePush surface specifically; the broader ADR-029 gateway sub-modules (Telegram/Slack/Discord/Signal/Email) remain stubbed. **(action: note that 8.13 closes MobilePush-HTTP only, not the full gateway set.)**
4. **Story 8.15 (journey-acceptance harness)** — the J4 slice flips green when 8.13 lands; its hermetic harness can reuse the real-HTTP-to-mock-server + real-TCP-loopback pattern this story establishes.
5. **NFR — J4 latency budget (Observer colocation <10ms P95, §13.1)** is **not** an AC here (no Observer in scope; 8.5 owns the colocation benchmark). If the team wants a wire-latency number on the live transport, add it as a Tier-2 reported measurement (do not hard-gate on shared CI — 8.12 polarity).

## Dev Agent Record

### Agent Model Used

openai/gpt-5.5

### Debug Log References

- `cargo test -p maos-notify-push` — PASS (3 tests: real socket POST, closed-endpoint typed error, token debug redaction).
- `cargo build -p maos-notify-push` — PASS.
- `cargo build -p maos-bin` — PASS.
- `MAOS_ONE_SHOT=smoke-mira-nash-tcp-8-13 ./target/debug/maos-bin` — PASS.
- `cargo test -p maos-bin --test smoke_mira_nash_tcp_8_13` — PASS, with subprocess `XDG_DATA_HOME` isolation.
- `MAOS_ONE_SHOT=smoke-mira-nash-8-5 ./target/debug/maos-bin` — PASS.
- `MAOS_ONE_SHOT=smoke-a2a-tcp-8-6 ./target/debug/maos-bin` — PASS.
- `cargo run -p xtask -- check-workspace-count` — PASS (actual=43, declared=43).
- `git diff --stat -- crates/maos-kernel-core/src` — PASS (empty output).
- `cargo run -p xtask -- abi-diff --base abi-baseline/v1-pre-bump.txt` — PASS.
- `cargo run -p xtask -- kloc-check` — expected pre-existing aggregate RED; new `maos-notify-push` is within its 250 LOC per-crate budget (186/250).
- `graphify update .` — PASS.

### Completion Notes List

- Implemented `maos-notify-push::MobilePushHttp`, a synchronous generic HTTP POST `NotificationChannel` using `ureq` with explicit bounded timeout, host-side `PushConfig`, and redacted token debug output.
- Replaced the live mobile-push panic path by changing `MobilePushChannel` from `unimplemented!()` to a typed unavailable error; the real mobile path is now the `maos-notify-push` adapter wired by `maos-bin`.
- Added `smoke-mira-nash-tcp-8-13`: Mira runs real `diagnose()` and `advisory()`, the advisory JSON rides the `TaskAssign.goal` wire contract through two distinct `TcpA2ATransport` endpoints over loopback TCP/mTLS, Nash derives a severity-tracking proposal, and the confused-deputy `-32007` path is asserted with the real payload.
- Integrated real HTTP mobile push into the J4 halt dispatch using a socket-backed mock server, plus closed-endpoint/per-channel error isolation and no-egress trip-test assertions.
- Preserved the Option A topology: one process, two distinct TCP transports and distinct TLS pins. Honest disclosure: Mira/Nash remain non-standalone under `maos run`; the integrated J4 journey runs through `smoke-mira-nash-tcp-8-13`. Two-OS-process A2A remains Tier-2b/follow-up scope.
- Authored the Tier-2a real-phone push release-gate artifact as OPEN for named human operator plus Winston co-sign.

### File List

- `Cargo.lock`
- `Cargo.toml`
- `_bmad-output/implementation-artifacts/8-13-cross-host-live-pair-spirit-tcp-binding-and-mobile-push.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md`
- `_bmad-output/test-artifacts/release-gate-8-13-tier-2-mobile-push.md`
- `crates/maos-bin/Cargo.toml`
- `crates/maos-bin/src/main.rs`
- `crates/maos-bin/tests/smoke_mira_nash_tcp_8_13.rs`
- `crates/maos-director-surface/src/notification.rs`
- `crates/maos-notify-push/Cargo.toml`
- `crates/maos-notify-push/src/lib.rs`
- `xtask/kloc.toml`

### Change Log

- 2026-06-08 — Story 8.13 implemented: new generic HTTP mobile-push adapter, live TCP J4 smoke, Tier-1 hermetic assertions, Tier-2a release-gate artifact, workspace count 43, status → review.

## Story Context Validation — RESOLVED (party-mode preflight 2026-06-08: Winston, Amelia, Murat, John — unanimous 4/4)

1. **FORK A (process topology)** → **Option A** (single-process, two distinct `TcpA2ATransport` over real loopback TCP). Option B (two `maos run` OS daemons) → its own follow-up story (Cross-Impact #1). Real-socket caveat is review-blocking (AC4/AC5).
2. **FORK B (push provider)** → **generic HTTP POST** (sync `ureq`); provider SDK banned from Tier-1. Tier-2a names a real provider (ntfy / Pushover / FCM) at sign time (AC3/AC6).
3. **FORK C (loopback smoke fate)** → **keep both** (`smoke-mira-nash-8-5` + new TCP arm) for differential diagnosis (AC4).
4. **Tier-2** → **SPLIT.** Tier-2a (real-phone push) signed by **a named human operator who holds the phone + Winston co-signs transport**; Tier-2b (two-OS-process) deferred to the Option-B follow-up story, not an 8.13 gate (AC6).

**Residual (surface at review, do not block dev):** the new must-fix — `frame.from.host_id` transport-derived vs hand-stamped (AC1/AC2, write red first) — verify the 8.5 journey logic's frame construction is adapted, not copied verbatim, onto the TCP path.
