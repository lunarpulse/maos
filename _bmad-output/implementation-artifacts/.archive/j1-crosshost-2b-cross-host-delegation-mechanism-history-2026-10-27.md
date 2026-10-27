# j1-crosshost-2b cross-host delegation mechanism — historical sections archive 2026-10-27

Verbatim relocation of historical sections of `j1-crosshost-2b-cross-host-delegation-mechanism.md` (re-baseline ledger, re-verified G-findings, inherited-from-1b notes, Traps). Each section below holds the body that followed the same-named heading in the story file, copied **verbatim** and keyed by that heading. The story keeps the heading and a one-line pointer; frontmatter, Status, Blocking conditions, Story, Acceptance Criteria, Tasks, Review Findings, Dev Notes, the Dev Agent Record, Open Questions and Change Log stay in the story. Nothing was rewritten.

## ⚠ RE-BASELINE LEDGER — what moved under this story after it was written (story lines 50-285)

*A ready-for-dev story is not a frozen story. This is the third time in this lane that a story's own
numbers were invalidated by its predecessors before it started (1a→1b, 2a→1b, now 2a+1b→2b). Read
this section before the findings; it is the delta, and everything in it is measured at `87eb6c37`.*

**H1 — SHIP-BLOCKER: the boundary leg `1b` built for this story cannot observe the flip in the shape
AC2.1 mandates.** `leg_loopback_from_host_unverified` (`xtask/src/check_j1_loopback_delegation.rs:385-437`)
computes:

```rust
let loopback_composed  = flat.contains("paired_loopback_router(");                 // :397
let verified_composed  = !loopback_composed                                        // :403
    && (flat.contains("handle_intake_verified") || flat.contains("maos_a2a_tcp")); // :404
let unverified = loopback_composed && self_asserted_resolution && !verified_composed; // :419
```

The `!loopback_composed &&` guard is 1b's own §A6 review patch P6, added so a *preparatory*
`use maos_a2a_tcp` beside a still-loopback router cannot flip the boundary. But **AC2.1 tells this
story to make `DelegationLeg::install` CHOOSE its router** — keep the loopback branch, add a TCP
branch, in the same file. If `paired_loopback_router(` survives in `delegation.rs`, then
`loopback_composed == true` ⇒ `verified_composed == false` ⇒ `unverified == true` ⇒ **no
`boundary MOVED` finding fires, the Blocking gate stays GREEN, and it keeps publishing
`loopback_from_host_unverified: true` over a verified wire.** And 1b's falsifier
(`xtask/tests/j1_crosshost_1b_proven_red.rs:821-837`) only ever plants the *total-replacement* shape
— it `.replace(…)`s the loopback call out entirely. **This re-creates P2's "tripwire that can never
fire" inside the leg written to repair it.**

**H1b — RESOLVED at the 2026-08-16 round-table, and the resolution is NOT "re-key the needle". The
leg is MIS-specified, not under-specified.**
Re-keying the grep was the obvious fix and it is wrong. Ask the question the leg's own name asks:
after `2b` lands, run `maos run` with the **loopback** topology — does the frame still pick its own
judge? **Yes.** `LoopbackA2ARouter` is untouched (`crates/maos-a2a/src/adapter.rs:82`, `:97`, direct
`handle_intake`). `loopback_from_host_unverified` is a claim about the **loopback path**, and `2b`
does not verify the loopback path — it adds a *different* path beside it. **So the honest value stays
`true`, and AC2.3's ratified assertion (`true → false`) asserts something that must never happen.**
Worse, the leg's `boundary MOVED` finding is wrong in *both* directions: it fires spuriously if `2b`
replaces the loopback arm, and not at all if `2b` forks — and neither behaviour describes the design.
**And no grep can fix it**: once `install` chooses at runtime, source text cannot know which arm ran.
This is the same limit already recorded against this gate — *a linter that checks the judge is still
written down*. **Resolution (unanimous, per spec + long-term correctness): split the fact.** One leg
stays source-static and permanently, correctly `true` (the loopback arm self-asserts); the cross-host
claim — *does the J1 cross-host path bind wire identity* — is **derived from the executed two-daemon
proof (AC1.6), never from grep.** AC2.3 owns both halves; it lands as a constraint on AC2.3, not a
new AC.

**H2 — BUDGET: `xtask` went from +223 to ZERO, and the aggregate red deepened by 269.**
`xtask = 38655` measured against a `38655` ceiling (`xtask/kloc.toml:203`, 1b's §A6 grant).
`_aggregate` measured **148211** vs 147057 = **−1154** (this file previously said −885; D17's register
row still says −492). `kloc-check` exits 1 on **three** keys, not four — `maos-bin` is GREEN at
16260/16260. T6/T11/T12 all land in `xtask/src`. Take the free reduction (H12) before asking.

**H3 — AC3.4 became a NULL CONTROL when 2a landed, and the vocabulary doubled.** 2a hoisted
`if !completion.is_completed() { return Err(…) }` to `crates/maos-bin/src/main.rs:4035-4041` so it
covers **every** topology `[cli_wrapper]` entry, with a twin on the standalone path at `:4454`.
`journal_completion` is called at `main.rs:4044`, *after* that guard — so it is **unreachable on any
non-completion**. Threading `completion.label()` into the frame would provably always write
`"completed"`; `main.rs:4052`'s `"result": completion.label()` is likewise a constant today; and
`crates/maos-bin/tests/delegation_leg_1a.rs:234` would not go red on its assertion, only by compile
break if the signature changes. Separately, `WorkerCompletion::label()` moved to
`crates/maos-bin/src/worker_cli.rs:87-106` and now returns **six** values, not three — 2a added
`not_completed:turn_failed`, `not_completed:no_effect_evidence`, `not_completed:permission_denied`.
AC3.4 is re-scoped below; do not implement the old wording.

**H4 — G15 is DISPROVED. The delegation consent envelope is NOT a non-expiring bearer grant.**
`prepare_outbound` stamps the TTL on any envelope carrying `None`
(`crates/maos-a2a-core/src/router.rs:866-871`):

```rust
if let Some(env) = frame.consent_envelope.as_mut() {
    if env.valid_until_ns.is_none() {
        let ttl_ns = peer_cfg.consent_ttl_secs.saturating_mul(1_000_000_000);
        env.valid_until_ns = Some(self.consent_now_ns().saturating_add(ttl_ns));
    }
}
```

1b's §A6 finding P11 caught this and its correction was recorded in `sprint-status.yaml` but **never
written into this file**. `crates/maos-bin/tests/consent_refusal_1b.rs:452-456` asserts the behaviour.
**As previously written, AC4.1's second bullet and T10 would have built a negative control for a
gap that does not exist.** The real residual is narrower and is a *policy* question, not a missing
expiry: **Decision §D1 TRANSITIONAL** — the transport stamps a TTL when the granter supplies none,
and an explicit granter `valid_until_ns` is authoritative and left untouched. AC4.1 now states that.

**H5 — the inherited section's own description of the boundary leg is wrong.** This file says the
leg "no longer needles `router.rs`". 1b's review finding P9 rebutted that explicitly: the leg reads
**both** files — door one is `crates/maos-bin/src/delegation.rs`, door two is
`crates/maos-a2a-core/src/router.rs`'s peer-**resolution expression**
(`letpeer_host=match&frame.from.host_id{`, production half only, which
`handle_intake_verified`'s own TLS-mismatch message literal cannot satisfy). The correction landed in
`sprint-status.yaml` and not here. Corrected below.

**H6 — T1's premise is disproved twice, but the task survives in narrower form.**
(a) 2a **already performed this relocation once**, for `worker_cli`, citing the same doctrine —
`crates/maos-bin/src/lib.rs:18-25` now carries a second copy of the "in the library, not `main.rs`"
comment and names *this story's AC1.1* as the precedent. `lib.rs` exports eight modules
(`cross_team_consent`, `cross_team_crossing`, `cross_wall_log_read`, `delegation`,
`enterprise_identity`, `tenant_map`, `worker_cli` — all `#[cfg(feature = "network")]` — plus
ungated `topology`).
(b) **"A host-B proof written today has no legal home" is FALSE.** `crates/maos-bin/tests/worker_completion_2a.rs`
already drives the whole worker-spawn surface end-to-end **by subprocess**
(`Command::new(env!("CARGO_BIN_EXE_maos")).args(["run", <manifest>, "--once"])` at `:871`, `:1119`,
`:1160`), exercising both the frame-borne path (`main.rs:4016`) and the standalone path (`:4432`).
What remains true is *in-process item visibility*: `run_cli_wrapper_manifest` and its five helpers
are still bare private `main.rs` items, so typed `WorkerCompletion` assertions and port injection are
impossible from `tests/`. **T1 is justified by that, and only that — say so.**
(c) **Do NOT move the daemon region.** `crates/maos-bin/tests/enterprise_daemon_seam_13_5a.rs:52-171`
and `cross_team_crossing_13_6b.rs:320, 373, 478, 502` assert the *literal text* of
`if mode == "cohort-a2a-daemon"`, `async fn run_cohort_a2a_daemon(`,
`async fn build_cohort_a2a_daemon_runtime(` and `async fn emit_cross_team_share(` inside
`include_str!("../src/main.rs")`. Relocating any of them reds both files.

**H7 — the harness this file calls "the RIGHT substrate" has ZERO CI enrollment.**
`crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs` appears in **none** of the eleven files under
`.github/workflows/`, and there is no workspace-wide `cargo test` anywhere — every `-p maos-bin` and
`-p xtask` invocation is `--test`-scoped or name-filtered. Its header comment calls its leg
`Blocking`; that word is a comment, not a control. Its **shape** is still the right one to clone
(hermetic, no Postgres, no `#[ignore]`, real mTLS from `mint_pems`, ephemeral-port scrape,
per-fixture `MAOS_AUDIT_DB` isolation set on the **child `Command`** — which sidesteps D16 entirely
because nothing calls `std::env::set_var` in-process). Its **binding** must be built by AC4.3.
Related: **no multi-process daemon test runs in CI at HEAD** — all four real ones
(`cross_team_crossing_13_6b.rs:1642`, `:1838`, `:2355`, `:2811`) are `#[ignore]` + live Postgres. This
story's proof would be the first.

**H8 — CORRECTED 2026-08-16 at the round-table. This finding overstated its own case, and the
register's conclusion stands.**
*What this file asserted:* "`IntentDirection::Accept` has zero construction sites, so the `-32001`
pair's discriminator is dead and 'already distinguishable' is a claim standing in for a control."
*What is actually true, read line by line:*
- `IntentDirection::Accept` genuinely has **zero construction sites anywhere, production or test** —
  so `router.rs:1676` (the `IntentDirection::Accept => CrossHostIntentDirection::Accept` mapping
  *inside* the `IntentDenied` arm) **is dead**, surviving only because `match` must be exhaustive.
- **But `router.rs:1688` hardcodes `CrossHostIntentDirection::Accept`** on the `IntentDeniedAtPeer`
  arm. So arm one always yields `Send` and arm two always yields `Accept`: **the `-32001` pair IS
  distinguishable at the `IacBusError` layer.** The register's premise (a) is **correct**; only its
  stated mechanism is loose.
**A dead line inside a live discriminator is not a dead discriminator.** Filed against this story's
own record, not against the register.
*And the thing nobody had quoted:* `IacBusError::CrossHostRouteFailure(String)` carries its own doc
at `crates/maos-domain/src/iac_bus_types.rs:69-72` — ***"DEPRECATED — use the typed sub-variants
above instead. Retained for backward compatibility with existing test stubs."*** So D18's fix was
never "add a variant to a RED crate against a dead premise"; it is "stop constructing a variant the
codebase already marks deprecated", with `CrossHostIntentDenied` / `CrossHostPinMismatch` /
`CrossHostConsentExpired` showing the shape.
*Cost, re-measured:* arm sizes at HEAD are `IntentDenied{…}` `:1673-1683` = **11**,
`IntentDeniedAtPeer` `:1684-1690` = **7**, `ConsentUnclassified` `:1773-1777` = **5**,
`ConsentUnclassifiedAtPeer` `:1778-1782` = **5**. A typed replacement is ≈+2 in `maos-a2a-core`
(4654/4654) and ≈+8..14 in `maos-domain` (RED −50) — and `UnclassifiedReason` lives in
`maos-a2a-core`, which `maos-domain` may not depend on (ADR-010), so a fully typed reason needs a
domain-side enum. **This is why D18 is not this story's to repair — see T0.**

**H9 — `kind = TaskComplete` is a CONTAMINATED oracle.** There are **seven** producers that bypass
the typed payload, not five. The two this file missed are the decisive ones:
`transparency_log.rs:1302` (`insert_kernel_event_returning_id`) hardcodes `FrameKind::TaskComplete`
for **every kernel event**, across **nine production callers** (`cross_team_crossing.rs:935, 962`;
`main.rs:5505, 8812, 10443, 10541`; `memory/mod.rs:462, 492, 593`), and `transparency_log.rs:873`
(`insert_distillate_redaction_marker`) writes kind `TaskComplete` under intent
`"distillate.redacted"`. **AC3.3's two-log join must key on the 16 `frame_id` bytes, never on
`kind` alone.**

**H10 — the ship-blocker fix has NO precedent to mirror.** `ErrorCode::ConstraintViolation` appears
**zero** times repo-wide; `rusqlite::Error::SqliteFailure` appears once and it is a *construction*
(`crates/maos-audit/src/lib.rs:346`), not a match arm. AC3.2 must be written from scratch, against a
`panic!` that fires on the very next line (`transparency_log.rs:820`).

**H11 — a pre-written two-host topology already names this story as its owner, and nothing in this
file mentions it.** `spirits/topologies/bilateral-2-host-mira-nash.toml` (Story 10.4b) declares
`host` on **class** Spirits (mira/nash), which `validate_remote_topology_target`
(`crates/maos-bin/src/topology.rs:31-46`) **refuses** — remote routing is `[cli_wrapper]`-only. It
parses but cannot be loaded. 2a's reader found it and recorded it verbatim
(`crates/maos-bin/tests/worker_manifests_2a.rs:265-272`): *"a forward declaration of a two-host scene
that `j1-crosshost-2b` owns"*, and 2a's sprint row repeats it. **This is a real fork, stated as
Q3 below.**

**H12 — assorted, each verified:**
- **`J1_TEST_SUFFIXES` is `["_1a.rs", "_1b.rs", "_2a.rs"]`** (`check_j1_loopback_delegation.rs:110`).
  A `crates/maos-bin/tests/*_2b.rs` file is not derived, not required, **silently un-enrolled**.
- **Three synchronized `lay_green` fixture trees** now exist (`j1_crosshost_1a_proven_red.rs:149`,
  `…_2a_proven_red.rs:155`, `…_1b_proven_red.rs:164`), each laying the same 14 files, hand-duplicated
  with no shared module and no reconciliation test.
- **`judge()` destructures `let [leg1..leg6] = &mut audits[..] else { panic!(…) }`**
  (`check_j1_loopback_delegation.rs:890`) and `j1_crosshost_1b_proven_red.rs:736-745` hard-asserts the
  six leg names. A seventh leg is a **panic**, not a graceful red, until six sites are updated.
- **Every leg must record a `LegAudit`** or `vacuous_legs()` turns it into a Finding
  (`gate_common.rs:127-178`, consumed at `check_j1_loopback_delegation.rs:906-915`). A leg that
  early-returns on a missing subject is RED, not silent.
- **`CODEX_ORACLE` / `CLAUDE_ORACLE`** (`:124-125`) are compiler-confirmed `never used` — the §A6
  needle rewrite orphaned them, and `leg_completion_oracle_per_adapter` hardcodes the literals
  instead. Two free `xtask` lines, and a half-null control worth naming.
- **`check-env-contract` is RED at HEAD and ownerless** (`MAOS_OPERATOR_BEARER_TOKEN` /
  `MAOS_OPERATOR_HTTP_BIND`, `main.rs:3254-3255`), verified pre-existing at `0769869d` by both 2a and
  1b. This story is the first since 1b with a plausible reason to touch `env_contract.rs`
  (`MAOS_COHORT_DAEMON_CONFIG`, AC2.2). **Attribute it; do not absorb it.**
- **Line shifts.** The old frontmatter's "everything past `main.rs:48` reads +2" is false. Actual:
  `issue_enterprise_governed_capability` 201→**203**, `RunArgs` 439→**439**, `parse_sandbox_tier`
  687→**689**, `resolve_cli_binary` 704→**706**, `load_host_grant_allowlist` 827→**843**,
  `run_cli_wrapper_manifest` 918→**938** (body to **:1358**), `DelegationLeg::install` call
  3150→**3219**, `maos run` block 3790→**3859**, `MAOS_ONE_SHOT` 5250→**5344**, daemon arm
  8140→**8234**, `run_cohort_a2a_daemon` 9815→**9909**, `build_cohort_a2a_daemon_runtime`
  10098→**10192**, `CohortDaemonFileConfig` 9374→**9469**, `smoke_a2a_tcp_8_6` router line
  10678→**10773**.

**H13 — NEW, found at the 2026-08-16 round-table. This is the defect D18's deadline was reaching for,
and it is worse than D18: a permanent security failure that presents as a transient network fault.**
`A2ARouterCore::interpret_response` (`crates/maos-a2a-core/src/router.rs:892-1052`) is where a remote
NACK becomes a sender-side `A2AError`. It has **nine typed arms** — `CODE_INTENT_DENIED`,
`CODE_PIN_MISMATCH_NOT_PINNED`, `CODE_CONSENT_EXPIRED`, `CODE_PEER_IDENTITY_MISMATCH`,
`CODE_CONSENT_GRANTER_MISMATCH`, `CODE_CONSENT_UNCLASSIFIED`, `CODE_TEAM_IDENTITY_MISMATCH`,
`CODE_CROSSING_SOURCE_TEAM_UNBOUND`, `CODE_CROSS_TEAM_CROSSING_REFUSED` — and a catch-all at
**`:1049`**: `_ => Err(A2AError::TransportFailed(n.error.message))`.
**Sixteen codes are defined** (`crates/maos-a2a-core/src/json_rpc.rs:27-87`). Seven fall through, and
one of them is **`CODE_SPIRIT_RESTART_DETECTED`**.
*Why that is this story's problem and nobody else's:*
- Host B detects a boot-nonce mismatch, **permanently invalidates the pin**
  (`tofu.rs:351-373` sets `Invalidated::SpiritRestarted`), and NACKs (`router.rs:1123-1159`).
- Host A receives it as `A2AError::TransportFailed(String)` → `IacBusError::CrossHostTransportFailure`.
  **The operator is told the network broke, and the implied action — retry — is the one action that
  can never work**, because the pin is dead until a config is edited and a process restarted.
  `await_repin_consent` has zero production callers and its default hook returns `TimedOut`.
- **It is structurally unreachable in rung 1.** `router.rs:1123` is `if request.boot_nonce != 0`, and
  loopback stamps the zero sentinel (`crates/maos-a2a/src/adapter.rs:75-77`). The branch has never
  executed. **It becomes reachable the moment two processes with real nonces talk — this story.**
*Severity vs D18:* D18 makes a refusal **illegible**. H13 makes it **legible and wrong** —
misattribution, not stringiness. An operator can act on illegible. They will act wrongly on this.
*Cost:* **≈4 lines, zero new types.** `A2AError::PinInvalidated { peer, awaiting_repin }` already
exists (`crates/maos-a2a-core/src/error.rs:68`) and already maps typed to
`IacBusError::CrossHostPinMismatch` (`router.rs:1697-1701`) — and it is semantically exact, because
restart detection *is* pin invalidation and re-pin *is* the designed recovery. It lands in
`maos-a2a-core` at ZERO headroom as a **correctness repair on a security path**, which
`kloc.toml:87` says a ceiling must never block; cite that line by name.
*Scope wall, binding:* **`2b` repairs the one code it makes reachable and no others.** The remaining
six fall-throughs (`PARSE_ERROR`, `INVALID_REQUEST`, `METHOD_NOT_FOUND`, `TIMEOUT`,
`FRAME_TOO_LARGE`, `INTERNAL`) are not newly reachable here; record the **9-of-16 census** with a
named owner and stop. A nine-arm refactor inside a frozen crate is how this becomes a different story.
*New failure shape, filed:* **a defect that was unreachable, which the story that makes it reachable
inherits.** Rung 1 did not miss it — rung 1 could not reach it. This is the second instance in this
same story: the `install_intake_sink` call also opens the duplicate-`frame_id` kernel panic (G16).
**The story's deliverable is also its threat model.**

## ⚠ The findings, re-verified at `87eb6c37` — the ratified scope is still wrong in fifteen places (story lines 291-595)

*Each finding below carries its re-verification verdict. `git diff 5a921c0c..HEAD` over
`maos-a2a-tcp`, `maos-a2a-core` and `maos-iac` is empty, which is why most of these survive verbatim.*

### The six findings that change what you build

**G1 — CONFIRMED VERBATIM. The receiver is not missing. It ACKs the frame and drops it. That single
fact is the story.**
`TcpA2ATransport::bind` (`crates/maos-a2a-tcp/src/transport.rs:139`) → `bind_with_cohort_manifest_gate`
(`:166`) → `bind_with_cohort_wiring` (`:196`) → `…_and_digest` (`:229`) → `…_and_crossing` (`:268`),
which does `TcpListener::bind` (`:343`) and `tokio::spawn(accept_loop(…))` (`:355`) → `accept_loop`
(`:504`) → `serve_connection` (`:562`) → **`core.handle_intake_verified(req, &verified_peer,
Some(&peer_leaf_fingerprint))` at `:637-643`**. The full admission chain for a delegation `TaskAssign`
runs today: host binding (`router.rs:1504-1521`), peer lookup (`:1093`), TOFU (`:1105`), boot-nonce
restart check (`:1123-1159`), consent granter/expiry (`:1169-1223`), accept-allowlist (`:1313`),
Lamport (`:1451`).
Then `router.rs:1453-1458` is `if let Some(sink) = …` — and `intake_sink` is declared at
`router.rs:175` and initialized `None` at `router.rs:218`. **Zero `install_intake_sink` occurrences
exist anywhere in `crates/maos-a2a-tcp/src/`** (the only hits in that crate are under `tests/`:
`t_10_4b_live_bilateral.rs:412, 662`; `trust_binding_8_9.rs:523, 566, 651`). The frame is
acknowledged `delivered: true` (`:1460-1466`) and discarded.
*The seam is already public and already documented for the wrong reason:* `TcpA2ATransport::core()`
(`transport.rs:388`, `pub`, doc at `:387` says *"for tests that drive intake directly"*) and
`A2ARouterCore::install_intake_sink` (`crates/maos-a2a-core/src/router.rs:345`, doc at `:343-344`
says ***"test-only hook"***). **That doc is FALSE, and more strongly than first recorded**: the
production chain is `main.rs:3219` (inside `async fn main()`, unconditional) → `delegation.rs:103`
`paired_loopback_router` → `crates/maos-a2a/src/pairing.rs:87` → `pairing.rs:112`
`install_intake_sink` — **and the receiver is DRAINED in production too**, at `delegation.rs:173` and
again at `:190`. The inline `"// (5) Push to intake sink (test hook)."` at `router.rs:1453` is wrong
for the same reason. Twenty lines below, `install_rupture_sink` (`router.rs:352-355`, doc `:350-351`)
carries the **correct** wording — *"Live transports install this before exposing their listener"* —
and it is factually correct: the live transport installs it at `transport.rs:324`, i.e. **inside the
bind chain, before `TcpListener::bind`**. Fix both comments in this story.

**G1b — NEW, and it is the same fact twice more.**
(i) **A second intake-sink push site exists** at `router.rs:1279-1283`, on the digest-reply
`Accepted` branch — same `if let Some(sink)`, same ACK `delivered: true`, same drop. A mechanism that
fixes only `:1453` leaves it behind.
(ii) **Both push sites discard the send result**: `let _ = sink.send(frame.clone());`
(`router.rs:1456`, `:1281`). A production sink whose receiver has been dropped would silently discard
frames **while still ACKing `delivered: true`** — G1's exact shape, one layer in. AC1.2 must not
inherit the `let _`.

**G2 — CONFIRMED, but see H6: the ship-blocker is narrower than written.** `run_cli_wrapper_manifest`
is `fn` (**not `async`**) at `main.rs:938-947`, body to `:1358`, and it is a private item of the
binary crate — it now returns `Result<worker_cli::WorkerCompletion, …>`, because 2a made it return
the oracle verdict. So are five things it needs: `RunArgs` (`:439-445`), `parse_sandbox_tier`
(`:689`), `resolve_cli_binary` (`:706`), `load_host_grant_allowlist` (`:843`),
`issue_enterprise_governed_capability` (`:203`). So are `CohortDaemonBootstrap` (`:9488-9496`),
`run_cohort_a2a_daemon` (`:9909-10039`) and `build_cohort_a2a_daemon_runtime` (`:10192-10320`).
**Nothing under `crates/maos-bin/tests/` can NAME any of them** — a grep of every `maos_bin::` path
across all 26 test files returns exactly the eight `lib.rs` modules and nothing else. The doctrine is
in the file twice now: `lib.rs:27-30` (`topology`, 1a) and `lib.rs:18-25` (`worker_cli`, 2a, which
names *this story's AC1.1* as its reason). Do the relocation first, in its own commit (T1), and
measure it. **Relocate `run_cli_wrapper_manifest` + the five helpers only — NOT the daemon region
(H6c).**

**G3 — CONFIRMED, with one path correction. "Duplicate-delivery safety" (split item 9) is aimed at a
hazard that does not exist at this layer.** The retry loop is real — `transport.rs:783-806`,
`max_attempts` default **4** (`crates/maos-a2a-core/src/mtls.rs:22-30`, value at `:27`), and the same
`request` built once at `:769-772` is re-sent by reference at `:787`. But the guard is
`!self.retry_policy.is_retryable(&a2a)` (`:794`), and `is_retryable` returns true **only** for
`A2AError::HandshakeFailed { class: BadCertificate | CertExpired }` (`mtls.rs:73-83`). Those classes
are minted only by `classify_handshake`, whose call sites are the **pre-send** handshake arm
(`transport.rs:477`, inside the TLS match at `:470-479`) and `verifier.rs:197`. The request body is
sent at `transport.rs:484-487` — *after*. Post-send failures map to `TransportFailed`/`Io`
(**`crates/maos-a2a-tcp/src/error.rs:82`, `:86`** — this file previously cited `maos-a2a-core`, wrong
crate), both non-retryable. **The transport is at-most-once with an ambiguous outcome, not
at-least-once.** No second retrier exists: `route_outbound`'s only callers are
`crates/maos-iac/src/adapter/mailbox.rs:533` (one call per host in a fan-out `for`),
`main.rs:10425`/`:10520`, and `crates/maos-a2a/src/adapter.rs:111` — none retries. *And the receiver
is at-ZERO-once anyway* (G1). AC3.5 re-scopes this deliberately.

**G4 — REFRAMED. The boot-nonce gap is a DEPLOYMENT-PROVISIONING blocker, not a PROOF blocker.**
The mechanism is exactly as recorded: `boot_nonce` is `getrandom` per process
(`main.rs:2651-2664`) and the `MAOS_TEST_BOOT_NONCE` override is **`cfg!(debug_assertions)`-gated**
at `:2651`. Host B pins host A's nonce **statically, from a config file**
(`crates/maos-a2a-tcp/src/config.rs:25-36`, `PinnedFingerprint{peer_id, fingerprint, boot_nonce}`),
and `boot_nonce` has **no serde default** — deliberately, and the code says so: *"REQUIRED (review
patch P2): a `#[serde(default)] = 0` here was a footgun… Operators MUST pre-pair the nonce."*
`build_pin_store` is at `:119-137`. The outbound stamps the *live* nonce (`transport.rs:769-772`). At
intake, `invalidate_if_boot_nonce_differs` (`crates/maos-a2a-core/src/tofu.rs:351-373`) sees the
mismatch, sets `TofuPin.invalidated = Some(Invalidated::SpiritRestarted{…})`, and NACKs
`CODE_SPIRIT_RESTART_DETECTED` (`router.rs:1123-1159`, code at `:1142`). Recovery is worse than
"permanent": the invalidation lives in an in-memory `DashMap` so it dies with the process, but the
receiver **rebuilds its pins from the same stale config on restart**, so the NACK recurs
deterministically; `await_repin_consent` (`tofu.rs:319-340`) is the designed recovery, its default
hook returns `RePinDecision::TimedOut` (`tofu.rs:145`), and it has **zero production callers**.
**But two escape hatches make a release-build proof possible, and the previous framing —
"provable in debug CI, not as a production posture" — was too strong:**
1. **The `boot_nonce == 0` wire sentinel** (`router.rs:1123`: `if request.boot_nonce != 0 {`). If the
   *sender* stamps 0, restart detection is skipped entirely. **The loopback leg this story forks from
   already does this** (`crates/maos-a2a/src/adapter.rs:75-77`, commented *"Loopback uses the v0.5-α
   `boot_nonce = 0` sentinel"*) — i.e. rung 1 ships with NFR-Rel-6 restart detection **off**. For TCP
   the nonce is a `bind()` argument, not an operator field, so this is a one-line source choice at
   the construction site. **REJECT it explicitly** — shipping the first cross-host path with restart
   detection disabled is precisely the control-standing-in-for-a-control shape this lane keeps
   catching. Name the rejection in the story record so the next author does not rediscover it as a
   shortcut.
2. **The peer's nonce is readable out-of-band from its own Transparency Log.** The daemon writes a
   `cohort:daemon-started` row on every boot (`main.rs:10005-10011`) stamped with the live nonce, and
   `transparency_log` carries a `boot_nonce INTEGER NOT NULL` column. For a **same-machine
   two-process** harness — which is what CI is — boot A, read A's nonce from A's `MAOS_AUDIT_DB`,
   write B's `peer_pins`, boot B. **That is a genuine release-build pairing with no debug hook.**
**The honest boundary, which AC4.1 must state:** MAOS has **no automated peer-nonce provisioning
channel**, so a true cross-machine first handshake requires an operator to transcribe a random 63-bit
value that changes on every restart. That is a production-posture gap worth its own filing; it does
not stop this story from building a release-build two-process proof.

**G5 — CONFIRMED on shape, CORRECTED on binding (H7). The substrate the shared preflight points at is
the wrong one; the right one is better but is not a control yet.**
P1 of `j1-crosshost-2-…md` names `crates/maos-bin/tests/cross_team_crossing_13_6b.rs:1642`. Measured:
that test is **`#[ignore = "AdvisorySubstrate: requires MAOS_TEST_POSTGRES_TEAM_A/_B (live Postgres)"]`
at `:1641`**, it **panics** without those vars (`pg_conn_team`, `:1327-1336`, no skip path), and the
frame it carries is `FrameKind::TelemetryEvent` (`crates/maos-bin/src/cross_team_crossing.rs:859`)
with `event_type = "maos.cross-team-crossing.v1"`
(`crates/maos-a2a-core/src/cohort.rs:395`, stamped at `cross_team_crossing.rs:147`) routed by an
`event_type` classifier (`router.rs:313-318`) into `apply_crossing`. **It never touches a `Mailbox`**
— `grep -ri mailbox crates/maos-a2a-core/src/` returns zero hits.
**Clone `crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs` instead** — `boot_hermetic_daemon`
(`:436-441`), `cohort_daemon_boots_and_serves` (`:443-476`), `fixture()` (`:227-250`), `mint_pems`
(`:163-176`), signed manifest (`:107`) — **not `#[ignore]`, no Postgres**, and it boots a real `maos`
daemon with real mTLS. **But it is enrolled by NO CI job (H7)**, so cloning it is necessary and not
sufficient: AC4.3 supplies the binding. Concretely, what it does:
- `Command::new(env!("CARGO_BIN_EXE_maos"))` with `.current_dir(workspace_root)`, stdout+stderr piped;
- four env vars **on the child**, never `std::env::set_var`: `MAOS_AUDIT_DB=<fixture>/transparency.sqlite`,
  `MAOS_OLLAMA_URL=skip`, `MAOS_ONE_SHOT=cohort-a2a-daemon`, `MAOS_COHORT_DAEMON_CONFIG=<fixture>/daemon.toml`;
- readiness by scraping `"cohort-a2a-daemon listening on "` off stderr in a reader thread through an
  `mpsc` channel, then parsing the ephemeral port (`listen_addr = '127.0.0.1:0'`, never pre-bound);
  `LISTEN_TIMEOUT = 90s`;
- teardown by an explicit `reap(child)` = `kill()` + `wait()`. **There is NO `Drop` guard on the
  child — a panic before `reap` leaks the daemon.** Copy `struct RunningDaemon(Child)` with a killing
  `Drop` from `cross_team_crossing_13_6b.rs:1520-1533`.

**G6 — CONFIRMED, and 2a supplied the precedent. The topology file cannot carry a TLS endpoint.**
`TOPOLOGY_SPIRIT_KEYS` is a **strict allowlist** — `crates/maos-bin/src/topology.rs:57` is
`["manifest", "path", "host"]`, unchanged by 2a, and any unknown key is a hard error (`:73-79`). On
top of that: `xtask/src/check_j1_loopback_delegation.rs:229-244` reds unless the topology declares
**exactly one** line whose trimmed start is `host` and which contains `developer-remote-host` —
`BindingClass::Blocking`, hermetic; and `crates/maos-bin/tests/topology_delegation_1a.rs:228-238`
asserts `hosts == vec![delegation::TO_HOST]`. **Both read `spirits/topologies/j1-founder-loop.toml`
by name and nothing else.**
**2a already proved the escape works**: it added `spirits/topologies/j1-founder-loop-codex.toml` and
landed green, because neither control looks at the new file. Route transport config through the
existing daemon config path instead (`MAOS_COHORT_DAEMON_CONFIG` → `CohortDaemonFileConfig`,
`main.rs:9469-9479`, `deny_unknown_fields`). **Three directory-scoped controls now constrain any new
topology file** — see AC2.2.

### Nine corrections you would otherwise carry forward as facts

**G7 — CONFIRMED. `main.rs:10773` is a SMOKE, not a production composition path.** The shared
preflight's `kernel_grant` note cites the old `:10678` as evidence that `TcpA2ATransport` "is ALREADY
constructed as `Arc<dyn A2ARouter>` in shipped code". At HEAD `main.rs:10773` is
`let router: std::sync::Arc<dyn A2ARouter> = mira.clone();` inside `smoke_a2a_tcp_8_6`
(`:10617-10798`), reachable only via `MAOS_ONE_SHOT=smoke-a2a-tcp-8-6` (`:8266-8268`), and it mints
its own CA + two leaves into `temp_dir()` at runtime. **Do not model the composition root on it.**
*The underlying claim is still TRUE by a better route:* `impl maos_domain::ports::a2a::A2ARouter for
TcpA2ATransport` at `transport.rs:826-840` — host A can pass an `Arc<TcpA2ATransport>` to
`install_a2a_router` today with **zero new adapter code**. This makes the outbound axis *smaller*
than the sketch.

**G8 — CONFIRMED. The composition-root fork is narrower than "restructure `main.rs`".**
`main.rs:3219` is `let mut delegation_leg = maos_bin::delegation::DelegationLeg::install(` —
unconditional, and it is **≪ `:3859`** (the `maos run` block) **≪ `:5344`** (`MAOS_ONE_SHOT`
dispatch) **≪ `:8234`** (`cohort-a2a-daemon`), i.e. **5,015 lines before** the daemon arm. So a
host-B daemon has already burned its mailbox router slot on the loopback pair. But
`Mailbox::install_a2a_router` (`crates/maos-iac/src/adapter/mailbox.rs:242-244`) is a bare
`self.a2a_router.set(router).map_err(|_| ())` over the `OnceLock` at `mailbox.rs:131`, with
**exactly one production caller**, `delegation.rs:110`, which turns `Err(())` into a hard boot error
(`delegation.rs:111-115`). **One caller means the cheap correct shape is to make
`DelegationLeg::install` CHOOSE its router, not to move or duplicate the call site** — but see **H1**:
that shape is exactly what blinds the boundary leg, and AC2.3 must be landed with it.

**G9 — CONFIRMED (two cites transposed). The `TaskComplete` return hop cannot route cross-host even
with `host_id` set.** `completion_frame` (`crates/maos-bin/src/delegation.rs:258-290`) sets
`to[0].host_id: None` (`:268`), `from.host_id: None` (`:277`) **and `consent_envelope: None`
(`:285`)**. That third one is decisive: `prepare_outbound` rejects it at the sender with
`A2AError::ConsentUnclassified { direction: Send, reason: UnclassifiedReason::Absent }`
(`crates/maos-a2a-core/src/router.rs:696-700`, `direction` at `:698`), pinned by `router.rs:2333-2344`.
Flipping `host_id` alone produces a fail-closed refusal, not a return hop. A real return needs
**five** things: both `host_id`s; a `ConsentEnvelope::with_fine_grained_intent` whose granter equals
`frame.from` — the constructor is `crates/maos-domain/src/frame.rs:447-465`, the mint site is
`spirits/orchestrator/src/lib.rs:363-366`, and the enforcement is receiver-side at
`router.rs:1170-1204` (`ConsentGranterMismatch` when `envelope.granter` ≠ `frame.from` on either
`spirit_id` or `host_id`); a **new** `complete_frame_remote`-shaped builder (none exists —
`assign_frame_remote` at `lib.rs:332` is the only remote builder, and `spirits/` is **kloc-free**); a
**second** intent with its own `send_allowlist`/`accept_allowlist` entries in the *reverse* direction;
and a router on host B's mailbox, which G8 says is already consumed. **DEFERRED to `2c`** — AC3.6.

**G10 — CONFIRMED with one correction. Correlation is a wire problem, not a schema problem.**
The Transparency Log already has the column (`crates/maos-iac/src/adapter/transparency_log.rs:268`),
the migration (`:497`), a non-unique index (`:498-501`), the row field
(`TransparencyLogEntry.correlation_id` `:204`), the filter field (`FrameFilter.correlation_id` `:216`),
the query predicate (`:1387-1390`) and a multi-adapter join helper
(`reconcile_correlated_frames` `:1888-1916`). What does **not** exist:
1. **`IacFrame` has no correlation field at all** (`crates/maos-domain/src/frame.rs:26-51`; fields are
   `frame_id, timestamp_ns, logical_clock, from, to, kind, intent, payload, auto_marker,
   consent_envelope, intent_lineage`). Nothing correlation-shaped crosses the wire.
2. **No public writer takes correlation AND a caller-chosen `frame_id` AND sender/recipient.**
   `insert_frame_event_with_correlation` (`:605-633`) passes `frame_id: None` (`:622`) and
   `from_spirit_id`/`to_spirit_id` as `""` (`:625-626`). Meanwhile `deliver_typed` — the only path J1
   frames take — reaches `insert_frame_row_with_correlation(… None …)` (`:758-769`, `None` at `:765`),
   so **every frame-borne row is hard-coded NULL correlation.**
   *Correction:* the correlation writer is **not** callerless — `crates/maos-cohort/src/audit.rs:153`
   and `main.rs:9647` both use it, for `TelemetryEvent` rows with kernel-minted ids. It is unused
   **for J1 frames**, which is the claim that matters.
*And a free key already threaded through the system:* `TaskAssignmentRecord { task_id,
capability_token, ttl_deadline_ns, intent_class, originator_spirit_id }`
(`crates/maos-domain/src/ports/task.rs:11-23`) is consumed by `crash_detector.rs:142-170`,
`progress_watchdog.rs:73`, `silent_failure_detector.rs:73`, `revocation/applier.rs:129` and
`disposition.rs:18` — and has **zero production writers** (only `main.rs:6775`/`:6828`, both smokes).
Recorded, **not needed** — G16 makes it unnecessary for this story.

**G11 — SUPERSEDED BY H3 AND H9. The task-outcome vocabulary exists one layer too low, and it has
doubled.** `WorkerCompletion::label()` (`crates/maos-bin/src/worker_cli.rs:87-106`) now returns
**six** values — `"completed"`, `"not_completed:process_crash"`, `"not_completed:no_completion_marker"`,
`"not_completed:turn_failed"`, `"not_completed:no_effect_evidence"`, `"not_completed:permission_denied"`
— and `main.rs:4052` prints it as `"result"` in the `delegation_completed` JSON event, while the
*frame* says `"completed"` unconditionally because `journal_completion` hardcodes the literal at
`delegation.rs:243`. **But H3: only `Completed` can reach that call site at HEAD**, so the hardcode
is currently a redundancy, not a lie. `TaskCompletePayload` is `{ result: String }`
(`crates/maos-domain/src/frame.rs:189-191`) with **no `impl` block anywhere** (grep-verified) and a
stale `TODO(Story 3.2)` doc at `:187`. `TaskAssignPayload` (`:93-106`) is
`goal / scope / success_criteria / posture_preferences / prior_distillate_ref` — **no task id**, and
`success_criteria` is circular and dead, handed to this story **by name** by 2a. **Seven** producers
write a `FrameKind::TaskComplete` row and bypass the typed payload (H9). **Do not unify them**
(Trap 14).

**G12 — SUPERSEDED BY H8.** Retained as a pointer: D18's register entry records a fix cost that does
not re-measure, and this file's own counter-estimate (+5 / +14) was itself unsound. See **H8** and
**T0** for the numbers that hold.

**G13 — CONFIRMED. `check-a2a-sender-completeness` structurally cannot see the file this story edits.**
`xtask/src/check_a2a_sender_completeness.rs` scans `spirits/{mira,nash}/{src,tests}` (`:129-132`) plus
four *named* smoke fns inside `main.rs` (`MAOS_BIN_CROSS_HOST_FNS`, `:58-63` —
`smoke_a2a_loopback_6_3`, `smoke_a2a_consent_vocab_8_7`, `smoke_a2a_tcp_8_6`,
`smoke_a2a_fail_closed_8_8`), with `EXEMPT_BASELINE = 0` (`:55`) and
`FORBIDDEN = ["consent_envelope: None", "intent_class: None"]` (`:49`). It opens only `main.rs` and
scans only those four fn bodies. `delegation.rs:285` carries `consent_envelope: None` on the
**production** completion frame and is out of scope entirely. A completeness gate that excludes the
story's own file is a null control here — and a new host-B send path added to a *named* `main.rs` fn
**would** be caught, so name it deliberately.

**G14 — CONFIRMED, with (iii) still wrong in the source and (iv) unchanged.** (i) `router.rs:343-344`
"test-only hook" on `install_intake_sink` — false (G1). (ii) `transport.rs:387` "for tests that drive
intake directly" on `core()` — same. (iii) `delegation.rs:21-23` says `smoke_orchestrator_fanout_6_2`
"binds **every** handle to `_`"; in `main.rs` they are bound to
`_orchestrator`/`_worker_a`/`_worker_b`/`_worker_cli` — underscore-*prefixed*, **not** dropped at
statement end. The substantive claim (never drained) holds; the mechanism is wrong. (iv)
**`crates/maos-iac/src/adapter/mailbox.rs:653-662` — `TODO(F5)`: the raw-byte
`IacBusPort::enqueue_frame` (`:653-657`) and `broadcast_frame` (`:659-663`) ALWAYS journal
`FrameKind::TaskAssign` regardless of the real kind.** Any consumer that routes bytes rather than
typed frames inherits a lying TL row. **Use `deliver_typed`.**

**G15 — DISPROVED. See H4.** The delegation envelope is not a non-expiring bearer grant on the wire;
`prepare_outbound` stamps `valid_until_ns = consent_now_ns() + consent_ttl_secs·1e9` on any envelope
carrying `None` (`router.rs:866-871`), and the expiry check at `router.rs:1207-1222` (guard at
`:1208`) then fires on it. The residual is the **Decision §D1 TRANSITIONAL** policy — transport-stamped
TTL when the granter supplies none, explicit granter expiry authoritative — which AC4.1 states as a
posture, not as a gap.

**G16 — CONFIRMED VERBATIM, and it is still the finding that collapses three ACs into one test while
promoting the headline into a SHIP-BLOCKER.**
The original AC3.1/3.2/3.3 asked for a correlation token on `IacFrame`, a TL writer to carry it, and a
join. Measured, **the join key already crosses the wire and already lands in both logs and both
bundles**: `assign_frame_remote` overwrites frame-id bytes 8..16 with `run_nonce`
(`spirits/orchestrator/src/lib.rs:358`) and `journal_completion` builds the id as
`seq ‖ run_nonce` (`delegation.rs:240-242`) — **deterministic, no ULID entropy**; the id travels on
the frame; **`deliver_typed` writes `Some(frame.frame_id)`**, the *received* id
(`crates/maos-iac/src/adapter.rs:561-562`, and the DRR branch does the same at
`adapter/drr_scheduler.rs:288-289`); and **`maos_audit::query` selects `frame_id` as its FIRST
column** (`crates/maos-audit/src/lib.rs:193-196`), so it is `AuditEntry.frame_id_hex` in every bundle
(field `:94`, populated `:292`, `:473`). So the two-host join costs **zero** new fields and touches
neither `maos-domain` (RED −50) nor `maos-audit` (+22). *`correlation_id` is a real column and is NOT
the join key — do not wire it for this purpose; `maos_audit::query` drops it from the projection, so
it could not reach a bundle anyway.*
**And the same fact is a remote-triggerable kernel halt.** `frame_id` is
`BLOB NOT NULL PRIMARY KEY` (`crates/maos-iac/src/adapter/transparency_log.rs:259`), the value is
**peer-supplied**, it is deterministic, the INSERT at `:797-802` is plain (no `ON CONFLICT`, no
`INSERT OR IGNORE`, no dedup anywhere in `deliver_typed`), and a failed write is
`Err(e) => panic!("MAOS kernel panic — Transparency Log write failed…")` (`:819-825`, panic at
`:820`, inside `insert_frame_row_with_correlation` `:773`). **A peer that re-sends one frame halts
host B.** It is unreachable today *only* because the frame is ACKed and dropped — so **the single
`install_intake_sink` call this story exists to add is also what opens it.** G1 and this are one fact
read twice. See AC3.2; it is promoted into the same change as AC1.2.

### What is already true — verify, do not rebuild

| Claim | State at `87eb6c37` |
|---|---|
| A verified inbound frame reaches a single production entry point | TRUE — `transport.rs:637-643` → `router.rs:1494`; every other caller is under `crates/*/tests/` |
| The full admission chain runs for a delegation `TaskAssign` | TRUE — host binding, TOFU, boot-nonce, consent, allowlist, Lamport all execute (G1 trace) |
| `TcpA2ATransport` implements the port the mailbox wants | TRUE — `impl A2ARouter for TcpA2ATransport`, `transport.rs:826-840`; the trait has exactly one method. **Zero new adapter code for host A** |
| The intake sink seam is public **and production-drained** | TRUE — `TcpA2ATransport::core()` `transport.rs:388`, `install_intake_sink` `router.rs:345`, drained in prod at `delegation.rs:173`/`:190` |
| `install_a2a_router` is set-once with one production caller | TRUE — `mailbox.rs:242-244`, caller `delegation.rs:110`, `Err(())` ⇒ hard boot error `delegation.rs:111-115` |
| `delegation.rs:200-218` is the only production `TaskAssign` consumer | TRUE — the cohort daemon has **no `FrameKind` match anywhere** on its path |
| A `FrameKind` dispatch precedent already exists | **TRUE — this file previously said otherwise.** `SpiritMailboxHandle::try_recv` returns `(FrameKind, IacFrame)` (`mailbox.rs:634`) and `delegation.rs:205-218` already matches on it **with a fail-closed default arm**. Copy it; do not invent it. What is genuinely absent is a `FrameKind` dispatch at the *router intake* layer — zero `FrameKind::` occurrences in `crates/maos-a2a-tcp/src/`, and in `maos-a2a-core/src/` only a construction at `router.rs:406` |
| A2A intake writes **no** TL row | TRUE — zero `insert_frame_event`/`TransparencyLog` hits in `router.rs` or `transport.rs`. Two indirect seams, neither the frame: `emit_consent_rupture` (deny only, `router.rs:361-416`) and `apply_crossing` (a port call at `:1641`) |
| The outbound side DOES journal | TRUE — I2 log-before-deliver, `crates/maos-iac/src/adapter.rs:474-547`; pinned to **exactly one** row per delegation by `delegation_leg_1a.rs:123-136` |
| A manifest→peer-config projection exists | **HALF-BUILT AND DEAD** — `CohortManifest::peer_configs_for` (`crates/maos-cohort/src/manifest.rs:568-634`), all callers are its own in-`src` `#[cfg(test)]` module, endpoints are sentinels `tls://{host_id}:0` (`:622`) |
| Hostnames work | **FALSE** — `dial_addr` does `rest.parse::<SocketAddr>()` (`transport.rs:434`), SNI is `ServerName::IpAddress` (`:468`). Bare `IP:port`, no DNS. And `A2APeerConfig::validate` (`crates/maos-a2a-core/src/config.rs:99-136`) **accepts** a hostname — its comment literally reads "accept hostname, IPv4, or IPv6" — whose own doc example is `tls://host-b.internal:7443` (`:43-45`): it passes bind and fails at first dial |
| `A2AProfile` selects behaviour | **FALSE** — `{Loopback, CrossHost}` is written and unit-asserted only; every workspace occurrence is a struct-literal write, and `default_profile()` is `Loopback` (`config.rs:79-81`). Never derive a "cross-host" claim from it |
| The loopback pair occupies real sockets | FALSE — `LoopbackEndpoint::config` (`crates/maos-a2a/src/pairing.rs:65-79`) emits `tls://127.0.0.1:7451`/`7452` as **strings** (ports supplied at `delegation.rs:104-105`); no `TcpListener` |
| `check-kernel-baseline` | **GREEN, 24472 = pinned 24472** (re-run at HEAD) |
| `kloc-check` at HEAD | **RED on THREE keys** — `maos-domain` −50 (D14), `maos-kernel-core` −685 (D13), `_aggregate` −1154 (D17). **None are yours; `maos-bin` is GREEN** |
| `check-env-contract` at HEAD | **RED, pre-existing, ownerless** — `MAOS_OPERATOR_BEARER_TOKEN` / `MAOS_OPERATOR_HTTP_BIND`, `main.rs:3254-3255` (H12) |

## Inherited from `j1-crosshost-1b` — two things rung 1 does NOT prove (story lines 624-667)

*Written here by `j1-crosshost-1b`'s dev pass (2026-08-16) under its AC1.5 / AC4.2. **Corrected in
this re-baseline by 1b's own §A6 findings P9 and P11**, which landed in `sprint-status.yaml` and never
reached this file.*

**(a) Rung 1 does not exercise peer authentication — a frame picks its own judge.**
On the TCP path `handle_intake_verified` binds `frame.from.host_id` to the TLS-verified peer
(`crates/maos-a2a-core/src/router.rs:1494-1521`). On loopback there is nothing to bind it to:
`router.rs:1477-1479` says so outright, and `LoopbackA2ARouter` calls `handle_intake` **directly**
(`crates/maos-a2a/src/adapter.rs:82`, `:97`). The field that selects **which `accept_allowlist`
applies** is written by the sender and never verified — so every refusal `1b` proved is **one string
assignment away** from selecting a different allowlist. Survivable in-process; NOT acceptable as the
inherited claim that rung 1 "proves the wire so rung 2 only adds network."
**This story is where it becomes load-bearing**, because it is where a second host first
authenticates a peer. `1b` also repaired the leg that watches this boundary.
**[P9 CORRECTION, applied here 2026-08-16]** The earlier text said the leg "no longer needles
`router.rs`". **That is false.** The repaired leg reads **two** files:
*door one* — `crates/maos-bin/src/delegation.rs`, asking whether the **J1 composition root** composes
a loopback or a verified router; *door two* — `crates/maos-a2a-core/src/router.rs`, needling the peer
**resolution expression** `letpeer_host=match&frame.from.host_id{` over the production half only,
which `handle_intake_verified`'s own TLS-mismatch message literal at `:1514` cannot satisfy (that
literal is exactly what pinned the OLD leg green forever). Door two is the second way the flip can
arrive: if the shared intake body ever binds identity itself, the J1 path becomes verified without
the composition root changing a line.
When this story composes a verified transport, `loopback_from_host_unverified` is *meant* to flip and
the gate is *meant* to red with `boundary MOVED`. **That is intended** — the finding text says
*"update this leg, the AC1.5(a) non-coverage statement in j1-crosshost-2b, and the story records — do
not delete the leg"*, and
`xtask/tests/j1_crosshost_1b_proven_red.rs::boundary_leg_reds_when_the_composition_root_gains_a_verified_transport`
(`:821-837`) is the vector that proves the flip is observable. **But H1: that vector plants only the
total-replacement shape, and the leg cannot see the fork shape AC2.1 mandates.** AC2.3 owns both.

**(b) The production error path conflates the deny codes, so operator-visible refusals are not yet
legible.** `map_a2a_error_to_iac_bus` (`router.rs:1671-1784`) preserves the `-32001` half only by
field (`direction: Send` vs `Accept`) — **and H8 shows that discriminator is dead**, because
`IntentDirection::Accept` has zero construction sites anywhere. It **destroys** the `-32009` half
outright: both `ConsentUnclassified` variants collapse into a stringly
`IacBusError::CrossHostRouteFailure` (`:1773-1782`), discarding the typed `UnclassifiedReason` and
the direction; `DelegationLeg::delegate` (`crates/maos-bin/src/delegation.rs:149-171`) then
stringifies even that. **This is why `1b` asserts at the router seam and not above it** — above
`A2ARouterCore` one side keeps a variant and the other becomes a sentence, so there is nothing to
compare. A cross-host operator cannot tell "policy refused you" from "policy could not classify you".
`1b` did **not** fix it: it is **D18**, resolved as a *decision* on 2026-08-15 (owner John + Vex,
target 14-4) with its deadline re-pinned to this story. **Resolved-as-a-decision is not
resolved-as-code: the conflation is live at HEAD.** See T0.

## Traps (story lines 1077-1156)

1. ~~Coordinate with `2a` on two regions.~~ **MOOT — 2a is `done` and both hunks landed** (blocking
   condition 4). The line numbers in the old trap are stale.
2. **Do not copy `check_vetting_attestation::invoke_leg`** (`xtask/src/check_vetting_attestation.rs:52-65`).
   It builds `Command::new("cargo")` with no `current_dir` and inherits the proven-red tempdir. Still
   true at HEAD.
3. **Any new gate leg must read via `root.join(rel)`, never a hardcoded path.** The proven-red harness
   sets `current_dir(tempdir)`; a leg using `Path::new("…")` resolves against the tempdir. If the miss
   is a Finding, `baseline_fixture_tree_is_green` reds and the suite is unrunnable; if it is a skip,
   every vector for that leg passes **vacuously** — and now `vacuous_legs()` turns that into a Finding
   instead. Known-dangerous callee: **`gate_common::read_disposition` (`:61-74`) hardcodes
   `Path::new("xtask/gate-registry.toml")` at `:63`** with no `root` parameter; the J1 gate
   deliberately never calls it.
4. **Do not extend `spirits/topologies/j1-founder-loop.toml`** (G6). Two blocking controls pin it, and
   a third (`worker_manifests_2a.rs:352-358`) exact-list-asserts the founder-loop set (AC2.2).
5. **Do not add a `std::env::var*` read under `crates/maos-bin/src` without registering it** —
   `xtask/src/check_env_contract.rs:119-121` walks that tree and fails. Note the converse null
   control: it walks **only** `crates/maos-bin/src/`, so a var read from `maos-a2a-tcp`, `xtask` or
   any test is invisible to it. **And note the gate is ALREADY RED at HEAD** (H12) — registering is
   still the rule, and your close must attribute the pre-existing failure.
6. **`cargo test -p maos-bin` is RED under default parallel flags** (D16, `MAOS_HOME` is
   process-global). Run scoped and `--test-threads=1`. **The clean way out is the one 13_5c already
   uses: set env on the child `Command`, never `std::env::set_var`** — then the new file is not part
   of the problem. D16 belongs to 14-0.
7. **FR21's 60s wall-clock window bites two processes on one box.** `check_orchestrator_distillate_required`
   (`crates/maos-iac/src/adapter/orchestrator_dispatch.rs:63-146`, window
   `DEFAULT_ORCHESTRATOR_DISPATCH_WINDOW_NS = 60_000_000_000` at `:40`) filters
   `{kind: TaskComplete, since_ns, until_ns, limit: 1}` with `spirit_pid` and `correlation_id` left
   `None` — **no** pid, session, orchestrator or boot-nonce scoping; any `TaskComplete` row in the
   window refuses the next `TaskAssign`. It keys on the **TL file path**, so distinct data homes
   genuinely avoid it — but `MAOS_HOME` **outranks both `MAOS_AUDIT_DB` and `XDG_DATA_HOME`**
   (`crates/maos-audit/src/lib.rs:872-902`), so setting a lower-precedence var while `MAOS_HOME` is
   set does not isolate. The advertised escape hatch `MAOS_ORCHESTRATOR_DISPATCH_WINDOW_NS`
   (`orchestrator_dispatch.rs:37`) has **zero code readers workspace-wide** — it is a doc comment.
   Also note the fail-open `if now == 0 { return Ok(()) }` at `:88-90`.
8. **A duplicate `frame_id` journaled twice PANICS the kernel** (`transparency_log.rs:819-825`), and
   J1 frame ids are **deterministic** `seq ‖ run_nonce` (`spirits/orchestrator/src/lib.rs:358`,
   `delegation.rs:240-242`). In debug builds `MAOS_TEST_BOOT_NONCE` pins the nonce (`main.rs:2651`).
   **A harness that pins the nonce for reproducibility walks straight into this on the second run.**
   Use fresh data homes per run, and see AC3.2.
9. **Two source-inspection suites assert the exact text of the daemon region.**
   `crates/maos-bin/tests/enterprise_daemon_seam_13_5a.rs:52-171` requires seven exact substrings
   inside the `if mode == "cohort-a2a-daemon"` block plus `enterprise_posture_required: bool` and
   `enterprise_daemon_governance: Option<Arc<EnterpriseDaemonGovernance>>` in **both**
   `run_cohort_a2a_daemon`'s and `build_cohort_a2a_daemon_runtime`'s signatures, plus whole-file
   counts (`.issue_with_mediation(` exactly once, `seal_row_at_rest(` zero, `issue_under_principal(`
   zero, no `LoadedSpiritKind::Enterprise`). `cross_team_crossing_13_6b.rs:320, 373, 478, 502` uses the
   same technique with a **paren-balanced** argument-list check, and each unwire must plant **exactly
   one** problem — so adding a second occurrence of any needle inside those scopes also reds it.
   **This is why AC1.1 must not relocate the daemon region.**
10. **`A2AProfile` is dead config and its default lies.** `{Loopback, CrossHost}` is never read to
    select behaviour, and `default_profile()` is `Loopback` (`crates/maos-a2a-core/src/config.rs:79-81`)
    — so a peer TOML omitting `profile` on a TCP transport is silently `Loopback` and behaves
    identically. Never derive a "cross-host" claim from this field.
11. **`maos-a2a-core` is at 4654/4654 and frozen by D10.** One production line hard-fails
    `kloc-check`. If your change lands there, re-route it to `maos-a2a-tcp` (+415) or `maos-bin`, or
    take a named grant. **`maos-cli` (4642/4642) and `xtask` (38655/38655) are at zero too.**
12. **Every line put in `crates/maos-a2a-tcp/tests/` runs 50× per push** — the determinism loop at
    `discipline.yml:1537-1543` runs `cargo test -p maos-a2a-tcp` **unscoped** under
    `timeout-minutes: 10` (`:1524`), plus once more for each scoped gate. A test that waits out the
    60s prod idle timeout or the ~130s unbounded connect **cannot live in that crate**.
13. **The bridge never parses NDJSON.** `crates/maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs:387-398`
    handles `NdjsonOverStdio` and `Raw` identically (shared arm at `:388` → `read_newline_delimited`,
    `:330-340`, which returns raw bytes with no JSON parse) despite the manifest declaring
    `ndjson_over_stdio` (`crates/maos-manifest/src/manifest.rs:4034-4038`). Unchanged by this story;
    do not assume otherwise when reading worker output on host B.
14. **Do not unify the seven `TaskComplete` producers** (G11, H9). Two of them —
    `crates/maos-kernel-core/src/halt/resolver.rs:223` and
    `crates/maos-kernel-core/src/supervision/crash_detector.rs:164` — are **inside kernel-core**, and
    the pin counts physical `.rs` lines in every file under that directory
    (`check_kernel_baseline.rs:99-110`). Touching either breaks ZERO-Δ.
15. **`delegated_task` carries only a goal string.** `Option<&str>` at `main.rs:946`, consumed at
    `:1114-1117` (`match delegated_task { Some(task) => worker_cli.argv(task), None => Vec::new() }`);
    `None` means *no delegation*, not a default. Call sites are **`:4016` and `:4432`**. It carries no
    task id, no correlation, no consent, no lineage.
16. **`maos-iac` has +36 lines of headroom.** AC3.2's typed `Duplicate` lands there. Measure before
    writing, not after.
17. **NEW — `install_intake_sink` is already filed as racy with in-flight frames**
    (`deferred-work.md:309-314`, alongside "duplicate `peer_id` silently overwrites"). AC1.2 installs
    it *before* `TcpListener::bind` (`transport.rs:343`), which is the ordering that avoids the race —
    do it there, not after the listener is exposed.

