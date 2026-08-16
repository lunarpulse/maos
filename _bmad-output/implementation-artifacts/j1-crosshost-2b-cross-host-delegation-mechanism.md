---
baseline_commit: "5a921c0c — BUT SEE THE BUDGET NOTE. `j1-crosshost-2a` was landing in the working tree while this story was scouted (`crates/maos-bin/src/{lib,main,worker_cli}.rs` modified, `crates/maos-bin/tests/worker_completion_2a.rs` created, 2026-08-16 05:45–05:53). Every `crates/maos-bin/src/main.rs` line number in this file was re-derived with `git show 5a921c0c:` against CLEAN HEAD. In a tree carrying 2a's relocation, everything past `main.rs:48` reads **+2**. Re-measure against 2a's merge commit, never against a dirty tree and never against this file."
depends_on: "`j1-crosshost-2a` (ready-for-dev, IN FLIGHT) **and** `j1-crosshost-1b` (ready-for-dev) must both reach `done`. 1b is a hard ordering constraint, not a preference — AC2.3 below verifies the leg 1b repairs, and that leg does not exist yet."
blocks: j1-crosshost-2c-two-host-signed-run
split_from: j1-crosshost-2-cross-host-signed-run (three-way split RATIFIED by Lunarpulse 2026-08-15; that file is the shared preflight for 2a/2b/2c)
kernel_grant: "NONE, and it is not at risk. `check-kernel-baseline` GREEN at **24472** (`xtask/kernel-core-baseline.toml:472`; **23679 is stale, do not inherit it**). Measured: `crates/maos-kernel-core/src/iac.rs:13` is a 66-line `pub use maos_iac::*;` shim — `Mailbox`, `IacBusAdapter` and `SpiritMailboxHandle` all physically live in `crates/maos-iac/`. `maos_domain::ports::a2a::A2ARouter` is **outbound-only** (one method, `route_outbound`), so an inbound pump needs no domain or kernel trait change. The two kernel-core files that touch this lane — `halt/resolver.rs:222` and `supervision/crash_detector.rs:163` — are named in Trap 14 as OUT of scope precisely because the pin counts physical `.rs` lines in every file under that directory (`xtask/src/check_kernel_baseline.rs:99-110`). Do NOT cite `abi-diff`: it scopes to `crates/maos-spirit-abi` only (`xtask/src/abi_diff.rs:8`), open **FLAG-E4**."
kloc_grant: "REQUIRED. **RE-MEASURED 2026-08-16 AFTER `j1-crosshost-2a` LANDED at `0769869d` — the pre-2a numbers below are superseded and 2a consumed more than it returned.** At `0769869d`: `maos-bin` **16260/16260 = ZERO HEADROOM** (2a re-based it twice — D15 16178→16219, then a review grant 16219→16260, both exact-measured), `maos-cli` **4642/4642 = ZERO** (2a review grant), `maos-audit` **6643/6665 = +22**, `maos-a2a-tcp` **1085/1500 = +415**, `maos-iac` **6852/6888 = +36**, `xtask` **38386/38609 = +223** (was +534), `maos-domain` **8694/8644 = RED −50** (D14), `_aggregate` **147942/147057 = RED −885** (was −492; D17). **The relocation in AC1.1 returns NOTHING** — it is `main.rs` → `lib.rs` inside one crate, so it is kloc-neutral; 2a's +204 refund was a different move (a test module into kloc-excluded `tests/`) and 2a has already spent it. **So 2b begins with ZERO headroom in the crate it must edit most.** Plan a named measured grant, or route production code into `maos-a2a-tcp` (+415) and keep `maos-bin` a thin call site. `crates/*/tests/`, `xtask/tests/`, `xtask/src/tests/` and all of `spirits/` cost ZERO."
model: frontier-class {opus-4-8, gpt-5.5, glm-5.2, opus-5, equiv}
review: §A6 full-layer net (Blind + Edge + Acceptance + Test-Infra + runtime) — NON-DEGRADABLE (this story is the first production code path that acts on a frame another machine sent)
---

# j1-crosshost-2b — cross-host delegation mechanism

Status: **backlog** — story authored, preflight complete, **not** `ready-for-dev`.

> `backlog` is deliberate and is this lane's convention, not a downgrade. The flow is
> `backlog → ready-for-dev → review → done`; **`blocked` is not a status in `sprint-status.yaml`** —
> `j1-crosshost-1b`'s file was corrected from `blocked` to `backlog` on 2026-08-14 for exactly this
> reason, and `j1-crosshost-2` carried its blockers in prose. The conditions that move this row are
> in **§ Blocking conditions** below and every one of them is mechanical.

> **What this story is.** Rung 1 proved a delegation frame can be *emitted* and routed on a loopback
> pair. `2a` proved one host can tell the truth about whether its worker did the work. `2b` is the
> first time a MAOS Host **acts on a frame another Host sent it**. Two processes, real mTLS, real
> TOFU, a real worker spawned on the far side, and a `TaskComplete` that comes back.
>
> **And the scope you were handed is wrong about where the work is.** The split's item 6 says "build
> a host B that receives `task.assign` over TCP". Measured: **the receiver already receives it.** A
> real daemon authenticates the peer, binds the wire identity, runs TOFU, checks the boot nonce,
> evaluates consent, advances the Lamport clock, and **ACKs `delivered: true`** — then drops the
> frame on the floor, because nothing ever installed an intake sink. That is the story: not a
> protocol, not a transport, **one missing `install_intake_sink` call and a consumer behind it.**

---

## ⚠ Read this block before the ACs — the ratified scope is wrong or imprecise in fifteen places

Every line number below was re-derived at **clean** `5a921c0c` by five parallel scouts. **Inherit no
line number from the shared preflight, from `sprint-status.yaml`, or from memory.** Numbers already
stale in circulating documents: the kernel pin is **24472** (not 23679); `main.rs:10678` is *not* a
production `Arc<dyn A2ARouter>` construction; and `kloc-check` is red on **four** keys at committed
HEAD, not three.

### The six findings that change what you build

**G1 — The receiver is not missing. It ACKs the frame and drops it. That single fact is the story.**
`TcpA2ATransport::bind` (`crates/maos-a2a-tcp/src/transport.rs:139`) → `bind_with_cohort_manifest_gate`
(`:166`) → `bind_with_cohort_wiring` (`:196`) → `…_and_digest` (`:229`) → `…_and_crossing` (`:268`),
which does `TcpListener::bind` (`:343`) and `tokio::spawn(accept_loop(…))` (`:355`) → `accept_loop`
(`:504`) → `serve_connection` (`:562`) → **`core.handle_intake_verified(req, &verified_peer,
Some(&peer_leaf_fingerprint))` at `:637-643`**. The full admission chain for a delegation `TaskAssign`
runs today: host binding (`router.rs:1504-1521`), peer lookup (`:1093`), TOFU (`:1105`), boot-nonce
restart check (`:1123-1159`), consent granter/expiry (`:1169-1223`), accept-allowlist (`:1313`),
Lamport (`:1451`).
Then `router.rs:1454-1458` is `if let Some(sink) = …` — and `intake_sink` is initialized `None` at
`router.rs:218`. **Zero `install_intake_sink` occurrences exist anywhere in `crates/maos-a2a-tcp/src/`.**
The frame is acknowledged `delivered: true` and discarded.
*The seam is already public and already documented for the wrong reason:* `TcpA2ATransport::core()`
(`transport.rs:388`, doc says *"for tests that drive intake directly"*) and
`A2ARouterCore::install_intake_sink` (`crates/maos-a2a-core/src/router.rs:345`, doc at `:342-343`
says ***"test-only hook"***). **That doc comment is FALSE**: `crates/maos-a2a/src/pairing.rs:112`
calls it and `pairing.rs:87` is reached from production `delegation.rs:103`. The inline
`"// (5) Push to intake sink (test hook)."` at `router.rs:1453` is wrong for the same reason. Twenty
lines below, `install_rupture_sink` (`router.rs:352-356`) carries the **correct** doc — *"Live
transports install this before exposing their listener"* — and that is the pattern to copy. Fix both
comments in this story; the next author will otherwise read the label and build a second mechanism.

**G2 — SHIP-BLOCKER, and it is the same shape `2a` just hit: the worker-spawn surface is
`main.rs`-private.** `run_cli_wrapper_manifest` is `fn` (**not `async`**) at `main.rs:918-931`, body
to `:1289`, and it is a private item of the binary crate. So are five things it needs: `RunArgs`
(`:439-445`), `parse_sandbox_tier` (`:687`), `resolve_cli_binary` (`:704`),
`load_host_grant_allowlist` (`:827`), `issue_enterprise_governed_capability` (`:201`). So are
`CohortDaemonBootstrap`, `run_cohort_a2a_daemon` (`:9815`) and `build_cohort_a2a_daemon_runtime`
(`:10098-10226`). **Nothing under `crates/maos-bin/tests/` can call any of them**, so a host-B proof
written today has no legal home — exactly `2a`'s F1, one function over. `2a` has already set the
precedent and the doctrine is in the file: `lib.rs:19-22` on `pub mod topology` (*"In the library,
not `main.rs`, so `crates/maos-bin/tests/` can execute it — an in-`src` test module is budget-charged
and CI-invisible"*). Do the relocation first, in its own commit (T1), and measure it.

**G3 — "Duplicate-delivery safety" (split item 9) is aimed at a hazard that does not exist at this
layer. Two scouts disproved it independently.** The retry loop is real —
`transport.rs:779-806`, `max_attempts` default **4** (`crates/maos-a2a-core/src/mtls.rs:22-30`), and
the same `request` built once at `:769-772` is re-sent by reference at `:787`. But the guard is
`!self.retry_policy.is_retryable(&a2a)` (`:794`), and `is_retryable` returns true **only** for
`A2AError::HandshakeFailed { class: BadCertificate | CertExpired }` (`mtls.rs:71-83`). Those classes
are minted only by `classify_handshake`, whose call sites are the **pre-send** handshake arm
(`transport.rs:477`) and `verifier.rs:197`. The request body is sent at `transport.rs:484-487` —
*after*. Post-send failures map to `TransportFailed`/`Io` (`error.rs:82`, `:86`), both
non-retryable. **The transport is at-most-once with an ambiguous outcome, not at-least-once.**
*And the receiver is at-ZERO-once anyway* (G1). AC3.5 re-scopes this deliberately: fix delivery
before designing dedup, or the story ships a duplicate guard over a path that delivers nothing.

**G4 — SHIP-BLOCKER-CLASS: the boot-nonce handshake makes a *release-build* two-host run
impossible, and `2c` would discover it with a paid agent attached.**
`boot_nonce` is `getrandom` per process (`main.rs:2582-2597`) and the `MAOS_TEST_BOOT_NONCE` override
is **`cfg!(debug_assertions)`-gated**. Host B pins host A's nonce **statically, from a config file**
(`crates/maos-a2a-tcp/src/config.rs:25-37`, `PinnedFingerprint{peer_id, fingerprint, boot_nonce}` —
and `boot_nonce` has **no serde default**; `build_pin_store` `:119-138`). The outbound stamps the
*live* nonce (`transport.rs:768-771`). At intake, `invalidate_if_boot_nonce_differs`
(`crates/maos-a2a-core/src/tofu.rs:351-372`) sees the mismatch, **permanently invalidates the pin**,
and NACKs `CODE_SPIRIT_RESTART_DETECTED` (`router.rs:1123-1159`).
The only two `MAOS_TEST_BOOT_NONCE` consumers in the repo are
`crates/maos-bin/tests/cross_team_crossing_13_6b.rs:1617` and `:2683` — i.e. **the one existing
two-daemon proof pre-pairs the nonce out of band, in a debug build.** So `2b` is provable in debug CI
and **is not provable as a production posture**. AC4.1 makes this a stated, machine-asserted boundary.
Do not let it be discovered at the paid run.

**G5 — The substrate the shared preflight points at is the wrong one, and the right one is better.**
P1 of `j1-crosshost-2-…md` names `crates/maos-bin/tests/cross_team_crossing_13_6b.rs:1642`. Measured:
that test is **`#[ignore = "AdvisorySubstrate: requires MAOS_TEST_POSTGRES_TEAM_A/_B (live Postgres)"]`
at `:1641`**, it **panics** without those vars (`pg_conn_team`, `:1327-1336`), and the frame it
carries is `FrameKind::TelemetryEvent` with `event_type = "maos.cross-team-crossing.v1"`
(`crates/maos-bin/src/cross_team_crossing.rs:859`) routed by an `event_type` classifier
(`router.rs:313-318`) into `apply_crossing`. **It never touches a `Mailbox`.** Re-targeting it drags
a live-Postgres dependency and an *advisory* binding into `2b`'s only proof.
**Use `crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs` instead**: `boot_hermetic_daemon`
(`:436-441`) + `cohort_daemon_boots_and_serves` (`:443-476`) — **not `#[ignore]`, no Postgres**,
boots a real `maos` daemon with real mTLS from a `fixture()` (`:227-250`) that mints PEMs
(`mint_pems`, `:163`) and signs a manifest (`:107`), and its header calls the leg `Blocking`
(`:433`). Two of those is the honest `2b` harness. **That choice alone decides whether `2b`'s proof
is blocking or advisory.**

**G6 — The topology file cannot carry a TLS endpoint, and two blocking controls pin it shut.**
`TOPOLOGY_SPIRIT_KEYS` is a **strict allowlist** — `crates/maos-bin/src/topology.rs:57` is
`["manifest", "path", "host"]`, and any unknown key is a hard error (`:72-80`, landed by 1a). On top
of that: `xtask/src/check_j1_loopback_delegation.rs:153-167` reds unless the topology declares
**exactly one** `host` line containing `developer-remote-host` — `BindingClass::Blocking`, hermetic,
`dev_enforced_red_blocks(…, true)` at `:308`, running at `discipline.yml:1815`; and
`crates/maos-bin/tests/topology_delegation_1a.rs:228-239` asserts `hosts == vec![delegation::TO_HOST]`.
**So `2b` must not express endpoints/ports/fingerprints in `j1-founder-loop.toml`, and must not add a
second host entry to it.** Add a *new* topology file plus the daemon-side config path that already
exists (`MAOS_COHORT_DAEMON_CONFIG` → `CohortDaemonFileConfig`, `main.rs:9374-9432`, `deny_unknown_fields`).

### Nine corrections you would otherwise carry forward as facts

**G7 — `main.rs:10678/10679` is a SMOKE, not a production composition path.** The shared preflight's
`kernel_grant` note cites it as evidence that `TcpA2ATransport` "is ALREADY constructed as
`Arc<dyn A2ARouter>` in shipped code". At HEAD `main.rs:10679` is
`let router: std::sync::Arc<dyn A2ARouter> = mira.clone();` inside `smoke_a2a_tcp_8_6`
(`:10523-10704`), reachable only via `MAOS_ONE_SHOT=smoke-a2a-tcp-8-6` (`:8172-8173`), and it mints
its own CA + two leaves into `temp_dir()` at runtime. **Do not model the composition root on it.**
*The underlying claim is still TRUE by a better route:* `impl maos_domain::ports::a2a::A2ARouter for
TcpA2ATransport` at `transport.rs:826-840` — host A can pass an `Arc<TcpA2ATransport>` to
`mailbox.install_a2a_router` today with **zero new adapter code**. This makes `2b` *smaller* than the
sketch on the outbound axis.

**G8 — The composition-root fork is narrower than "restructure `main.rs:3150`".**
`main.rs:3150` is `let mut delegation_leg = maos_bin::delegation::DelegationLeg::install(` —
unconditional, and it is **≪ `:3790`** (the `maos run` block) **≪ `:5250`** (`MAOS_ONE_SHOT`
dispatch) **≪ `:8140`** (`cohort-a2a-daemon`). So a host-B daemon has **already burned its mailbox
router slot on the loopback pair** 5,000 lines before it builds a transport. But
`Mailbox::install_a2a_router` (`crates/maos-iac/src/adapter/mailbox.rs:242-244`) is a bare
`OnceLock::set` (the `OnceLock` is `mailbox.rs:131`) with **exactly one production caller**,
`delegation.rs:110`, which turns `Err(())` into a hard boot error (`delegation.rs:109-115`).
**One caller means the cheap correct shape is to make `DelegationLeg::install` CHOOSE its router, not
to move or duplicate the call site.** Moving `:3150` past `:5250` puts it after the daemon arm
returns and is a dead end.

**G9 — The `TaskComplete` return hop cannot route cross-host even with `host_id` set.**
`completion_frame` (`crates/maos-bin/src/delegation.rs:258-290`) sets `to[0].host_id: None` (`:268`),
`from.host_id: None` (`:277`) **and `consent_envelope: None` (`:285`)**. That third one is decisive:
`prepare_outbound` rejects it at the sender with
`A2AError::ConsentUnclassified { direction: Send, reason: UnclassifiedReason::Absent }`
(`crates/maos-a2a-core/src/router.rs:697`, pinned by `router.rs:2333-2342`). Flipping `host_id` alone
produces a fail-closed refusal, not a return hop. A real return needs **five** things: both
`host_id`s; a `ConsentEnvelope::with_fine_grained_intent` whose granter equals `frame.from`
(the constraint `assign_frame_remote` encodes at `spirits/orchestrator/src/lib.rs:363-366`); a **new**
`complete_frame_remote`-shaped builder (none exists — `assign_frame_remote` at `lib.rs:332` is the
only remote builder, and `spirits/` is **kloc-free**); a **second** intent with its own
`send_allowlist`/`accept_allowlist` entries in the *reverse* direction; and a router on host B's
mailbox, which G8 says is already consumed. **Scope it deliberately or cut it** — see AC3.6.

**G10 — Correlation is a wire problem, not a schema problem, and it is TWO missing mechanisms.**
The Transparency Log already has the column (`crates/maos-iac/src/adapter/transparency_log.rs:268`),
the migration (`:497`), a non-unique index (`:498-501`), the row field
(`TransparencyLogEntry.correlation_id` `:204`), the filter field (`FrameFilter.correlation_id` `:216`),
the query predicate (`:1387-1390`) and even a multi-adapter join helper
(`reconcile_correlated_frames` `:1888-1915`). What does **not** exist:
1. **`IacFrame` has no correlation field at all** (`crates/maos-domain/src/frame.rs:26-51`). Nothing
   correlation-shaped crosses the wire.
2. **No public writer takes correlation AND a caller-chosen `frame_id` AND sender/recipient.**
   `insert_frame_event_with_correlation` (`:605-633`) passes `frame_id: None` (`:622`) and
   `from_spirit_id`/`to_spirit_id` as `""` (`:625-626`). Meanwhile `deliver_typed` — the only path J1
   frames take — routes to `insert_frame_row_with_correlation(… None …)` (`:758-769`), so **every
   frame-borne row is hard-coded NULL correlation.**
*And there is a free key already threaded through the system:* `TaskAssignmentRecord { task_id,
capability_token, ttl_deadline_ns, intent_class, originator_spirit_id }`
(`crates/maos-domain/src/ports/task.rs:12-23`) is consumed by `crash_detector.rs:150-171`,
`progress_watchdog.rs:73`, `silent_failure_detector.rs:73`, `revocation/applier.rs:129` and all three
FR50 emitters — and has **zero production writers** (only `main.rs:6683`/`:6736`, both smokes). The
type exists, the plumbing exists, nobody fills it. That is `2b`'s cheapest correlation key.

**G11 — The task-outcome vocabulary already exists one layer too low.**
`WorkerCompletion::label()` (`crates/maos-bin/src/worker_cli.rs:71-81`) returns
`"completed" | "not_completed:process_crash" | "not_completed:no_completion_marker"`, and
`main.rs:3969-3985` **prints it as `"result"` in the JSON event** — while the *frame* says
`"completed"` unconditionally, because `journal_completion` hardcodes the literal at
`delegation.rs:243`. `TaskCompletePayload` is `{ result: String }` (`crates/maos-domain/src/frame.rs:188-191`)
with **no `impl` block anywhere** and a stale `TODO(Story 3.2)` doc at `:187`. `TaskAssignPayload`
(`:92-106`) is `goal / scope / success_criteria / posture_preferences / prior_distillate_ref` — **no
task id**. Seven producers write a `FrameKind::TaskComplete` row and **five bypass the typed payload
entirely** (halt orphan `resolver.rs:218-232`, crash orphan `crash_detector.rs:141-171`, and FR50
nack/escalate/reassign at `crates/maos-iac/src/adapter.rs:239-301`). Six mutually incompatible
payload shapes under one `FrameKind`. **Do not unify all seven here** (Trap 14).

**G12 — D18's "≈ zero net lines" resolution does not survive re-measurement, and its stated evidence
is itself a null control.** The register (`epic-14-preflight-decisions.md`, D18) records the fix as
≈0 net in `maos-a2a-core` and +6 in `maos-domain`. Re-measured at HEAD:
- Current arms in `map_a2a_error_to_iac_bus` (`router.rs:1671-1784`): `IntentDeniedAtPeer` **7 lines**
  (`:1684-1690`), `ConsentUnclassified` **5** (`:1773-1777`), `ConsentUnclassifiedAtPeer` **5**
  (`:1778-1782`) = **17**. Typed replacements ≈ 8 + 7 + 7 = **22** ⇒ **net ≈ +5 in a crate with
  EXACTLY ZERO headroom (4654/4654, D10)**. It hard-fails on contact.
- New variants land in `crates/maos-domain/src/iac_bus_types.rs` (whole file 156 lines; `IacBusError`
  `:14-109`, 17 variants) ⇒ ≈ **+14**, into a crate already **RED at −50** (D14).
- And D18's premise (a) — *"the `-32001` pair is ALREADY distinguishable via `direction: Send` vs
  `Accept`"* — **is a null control.** `IntentDirection::Accept` has **zero production construction
  sites** inside an `A2AError::IntentDenied`; every non-test construction is `Send`
  (`maos-a2a/src/adapter.rs:228`; `router.rs:698, 751, 788, 806, 826, 1014`; `main.rs:10925, 11331,
  11375, 11405`). The receiver-side denial never becomes a local `A2AError` — it becomes a NACK,
  re-materialized as `IntentDeniedAtPeer`. **`router.rs:1676` is unreachable in production**, so the
  arm that would make `direction` a real discriminator is dead. `map_a2a_error_to_iac_bus` is called
  only outbound (`maos-a2a/src/adapter.rs:113`, `transport.rs:832-838`) — which is *why* it is dead.
**Consequence:** D18 needs two named grants or a net-neutral construction. AC3.7 states the choice;
do not re-litigate the zero-headroom wall from the register's number.

**G13 — `check-a2a-sender-completeness` structurally cannot see the file this story edits.**
`xtask/src/check_a2a_sender_completeness.rs` scans `spirits/{mira,nash}/{src,tests}` (`:129-132`) plus
four *named* smoke fns inside `main.rs` (`MAOS_BIN_CROSS_HOST_FNS`, `:58`), with `EXEMPT_BASELINE = 0`
(`:55`) and `FORBIDDEN = ["consent_envelope: None", "intent_class: None"]` (`:50`).
`delegation.rs:285` carries `consent_envelope: None` on the **production** completion frame and is out
of the gate's scope entirely. A completeness gate that excludes the story's own file is a null control
for `2b` — and a new host-B send path added to a *named* `main.rs` fn **would** be caught, so name it
deliberately.

**G14 — Two doc comments and one code comment will mislead the dev; a fourth is a live wire-format
lie.** (i) `router.rs:343-344` "test-only hook" on `install_intake_sink` — false (G1).
(ii) `transport.rs:387` "for tests that drive intake directly" on `core()` — same. (iii)
`delegation.rs:21-23` says `smoke_orchestrator_fanout_6_2` "binds **every** handle to `_`"; at
`main.rs:9133-9144` they are bound to `_orchestrator`/`_worker_a`/`_worker_b`/`_worker_cli` —
underscore-*prefixed*, **not** dropped at statement end. The substantive claim (never drained) holds;
the mechanism is wrong. (iv) **`crates/maos-iac/src/adapter/mailbox.rs:653-662` — `TODO(F5)`: the
raw-byte `IacBusPort::enqueue_frame`/`broadcast_frame` ALWAYS journal `FrameKind::TaskAssign`
regardless of the real kind.** Any `2b` consumer that routes bytes rather than typed frames inherits a
lying TL row. Use `deliver_typed`.

**G15 — The delegation consent envelope is a permanent, non-expiring bearer grant, and it is
currently unowned.** `ConsentEnvelope::with_fine_grained_intent` at
`spirits/orchestrator/src/lib.rs:328-330` mints `timestamp_ns = 0, valid_until_ns = None`, and the
expiry check at `router.rs:1207-1222` only fires when `valid_until_ns.is_some()`. On a loopback pair
that is inert. **On a real wire it is a credential with no expiry.** The doc defers it to "the
`j1-crosshost-2` surface" — a story key that no longer exists post-split. `2b` is the first story
where it is on a real wire: state it as a bounded gap with a named owner (AC4.1), or close it.

**G16 — Added by the 2026-08-16 round-table, and it collapses three of this story's own ACs into one
test — while turning the story's headline finding into a SHIP-BLOCKER.**
The original AC3.1/3.2/3.3 asked for a correlation token on `IacFrame`, a TL writer to carry it, and a
join. Measured, **the join key already crosses the wire and already lands in both logs and both
bundles**:
`assign_frame_remote` overwrites frame-id bytes 8..16 with `run_nonce`
(`spirits/orchestrator/src/lib.rs:357`) and `journal_completion` builds the id as
`seq ‖ run_nonce` (`delegation.rs:240-242`) — **deterministic, no ULID entropy**; the id travels on
the frame; **`deliver_typed` writes `Some(frame.frame_id)`**, the *received* id
(`crates/maos-iac/src/adapter.rs:562`); and **`maos_audit::query` selects `frame_id` as its FIRST
column** (`crates/maos-audit/src/lib.rs:194-196`), so it is `AuditEntry.frame_id_hex` in every bundle.
So the two-host join costs **zero** new fields and touches neither `maos-domain` (RED −50) nor
`maos-audit` (+22). *`correlation_id` is a real column and is NOT the join key — do not wire it for
this purpose; `maos_audit::query` drops it, so it could not reach a bundle anyway.*
**And the same fact is a remote-triggerable kernel halt.** `frame_id` is
`BLOB NOT NULL PRIMARY KEY` (`crates/maos-iac/src/adapter/transparency_log.rs:259`), the value is
**peer-supplied**, it is deterministic, and a failed write is
`Err(e) => panic!("MAOS kernel panic — Transparency Log write failed…")` (`:819-825`). **A peer that
re-sends one frame halts host B.** It is unreachable today *only* because the frame is ACKed and
dropped — so **the single `install_intake_sink` call this story exists to add is also what opens it.**
G1 and this are one fact read twice. See AC3.2; it is promoted into the same change as AC1.2.

### What is already true — verify, do not rebuild

| Claim | State at clean HEAD |
|---|---|
| A verified inbound frame reaches a single production entry point | TRUE — `transport.rs:637-643` → `router.rs:1494`; every other caller is under `crates/*/tests/` |
| The full admission chain runs for a delegation `TaskAssign` | TRUE — host binding, TOFU, boot-nonce, consent, allowlist, Lamport all execute (G1 trace) |
| `TcpA2ATransport` implements the port the mailbox wants | TRUE — `impl A2ARouter for TcpA2ATransport`, `transport.rs:826-840`. **Zero new adapter code for host A** |
| The intake sink seam is public | TRUE — `TcpA2ATransport::core()` `transport.rs:388`, `install_intake_sink` `router.rs:345` |
| `install_a2a_router` is set-once with one production caller | TRUE — `mailbox.rs:242-244`, caller `delegation.rs:110`, `Err(())` ⇒ hard boot error `delegation.rs:109-115` |
| `delegation.rs:200-218` is the only production `TaskAssign` consumer | TRUE — the cohort daemon has **no `FrameKind` match anywhere** on its path |
| A2A intake writes **no** TL row | TRUE — zero `insert_frame_event`/`TransparencyLog` hits in `router.rs` or `transport.rs`. Two narrow exceptions, neither the frame: `emit_consent_rupture` (deny only, `router.rs:357-415`) and `apply_crossing` (Loom-lite, not TL) |
| The outbound side DOES journal | TRUE — I2 log-before-deliver, `crates/maos-iac/src/adapter.rs:474-547`; pinned to **exactly one** row by `delegation_leg_1a.rs:123-135` |
| A manifest→peer-config projection exists | **HALF-BUILT AND DEAD** — `CohortManifest::peer_configs_for` (`crates/maos-cohort/src/manifest.rs:568-634`), **zero production callers**, endpoints are sentinels `tls://{host_id}:0` (`:620-622`) |
| Hostnames work | **FALSE** — `dial_addr` does `rest.parse::<SocketAddr>()` (`transport.rs:434`), SNI is `ServerName::IpAddress` (`:468`). Bare `IP:port`, no DNS. And `A2APeerConfig::validate` **accepts** a hostname (`config.rs:115-136`) whose own doc example is `tls://host-b.internal:7443` — it passes bind and fails at first dial |
| The loopback pair occupies real sockets | FALSE — `LoopbackEndpoint::config` (`pairing.rs:65-79`) emits `tls://127.0.0.1:7451`/`7452` as **strings**; no `TcpListener`. No port conflict, but `delegation.rs:60`/`:65` read as real endpoints to the next author |
| `check-kernel-baseline` | GREEN, 24472 = pinned 24472 |
| `kloc-check` at committed HEAD | RED on **four** keys (`maos-bin` +41, `maos-kernel-core` +685, `maos-domain` +50, `_aggregate` +492). Three are D13/D14/D17 — **not yours** |

---

## Blocking conditions — every one is mechanical

1. **`j1-crosshost-2a` reaches `done`.** It is in flight in the working tree. `2b` re-measures its
   budget against 2a's merge commit and inherits 2a's `pub mod` precedent for G2.
2. **`j1-crosshost-1b` reaches `done`** with rung-1 evidence reading `PROVEN_BLOCKING`. Hard: AC2.3
   verifies the boundary leg **1b rewrites**, and the rewritten leg does not exist yet.
3. **D18 has a budget decision** (G12). Its deadline is literally *"before `j1-crosshost-2b` writes
   its first line"*, and the register's ≈0-line premise does not re-measure. Either two named grants
   land, or the arms are constructed net-neutral, or D18 slips a third time **by decision, in writing**.
4. **Agreement with `2a` on two shared regions** (Trap 1): the standalone `[cli_wrapper]` block at
   `main.rs:4355-4367`, and the `completion_tl_ref` → `last_stdout_tl_ref` rename whose emitted JSON
   key (`main.rs:1282`) is the natural input to cross-host correlation.


---

## Inherited from `j1-crosshost-1b` — two things rung 1 does NOT prove

*Written here by `j1-crosshost-1b`'s dev pass (2026-08-16) under its AC1.5 / AC4.2, so this story's
preflight cannot mistake a partial proof for a whole one. `1b` proved the loopback wire **refuses** a
disallowed intent with `-32001`, `-32009` (both seams, every reachable reason) and `-32003`, each
kept distinct, in `crates/maos-bin/tests/consent_refusal_1b.rs`, enrolled at
`.github/workflows/discipline.yml`'s `check-j1-loopback-delegation` job and judged by that gate's
`consent-refusal-proofs` leg. What follows is what those proofs do **not** cover. Retargeted here
because rung 2 was split on 2026-08-15 and there is no `j1-crosshost-2` row to defer into.*

**(a) Rung 1 does not exercise peer authentication — a frame picks its own judge.**
On the TCP path `handle_intake_verified` binds `frame.from.host_id` to the TLS-verified peer
(`crates/maos-a2a-core/src/router.rs:1494-1521`). On loopback there is nothing to bind it to:
`router.rs:1477-1479` says so outright, and `LoopbackA2ARouter` calls `handle_intake` **directly**
(`crates/maos-a2a/src/adapter.rs:82`, `:97`). The field that selects **which `accept_allowlist`
applies** is written by the sender and never verified — so every refusal `1b` proved is **one string
assignment away** from selecting a different allowlist. Survivable in-process; NOT acceptable as the
inherited claim that rung 1 "proves the wire so rung 2 only adds network."
**This story is where it becomes load-bearing**, because `2b` is where a second host first
authenticates a peer. `1b` also repaired the leg that watches this boundary: it no longer needles
`router.rs` (where the trigger does not live and where
`handle_intake_verified`'s own TLS-mismatch message literal pinned it green forever) but the **J1
composition root** — `crates/maos-bin/src/delegation.rs`. When `2b` composes a verified transport
there, `loopback-from-host-unverified` flips and the gate reds with `boundary MOVED`. That is
**intended**: AC2.3's verification target is that leg, and
`xtask/tests/j1_crosshost_1b_proven_red.rs::boundary_leg_reds_when_the_composition_root_gains_a_verified_transport`
is the vector that proves the flip is observable. `2b` must update the leg, this section, and the
story records in the same change — never delete the leg.

**(b) The production error path conflates the deny codes, so operator-visible refusals are not yet
legible.** `map_a2a_error_to_iac_bus` (`router.rs:1671-1783`) preserves the `-32001` half only by
field (`direction: Send` vs `Accept`) and **destroys** the `-32009` half: both `ConsentUnclassified`
variants collapse into a stringly `IacBusError::CrossHostRouteFailure` (`:1773-1782`), discarding the
typed `UnclassifiedReason` and the direction; `DelegationLeg::delegate`
(`crates/maos-bin/src/delegation.rs:149-171`) then stringifies even that. **This is why `1b` asserts
at the router seam and not above it** — above `A2ARouterCore` one side keeps a variant and the other
becomes a sentence, so there is nothing to compare. A cross-host operator cannot tell "policy refused
you" from "policy could not classify you". `1b` did **not** fix it: it is **D18**, RESOLVED as a
decision 2026-08-15 (owner John + Vex, target 14-4) with its deadline re-pinned to *"before
`j1-crosshost-2b` writes its first line"* — i.e. **this story**. Resolved-as-a-decision is not
resolved-as-code: **the conflation is live at HEAD.** See blocking condition 3.
---

## Story

**As** the founder running the J1 developer-remote loop,
**I want** a second MAOS Host that actually executes the task I delegated — receiving the frame over
mTLS, spawning its own worker, journaling what it did, and answering with a typed outcome I can join
to my own log —
**so that** "developer-remote" stops being a host id in a config file and becomes a machine that did
the work, with both sides' evidence able to be reconciled by `2c`.

---

## Acceptance Criteria (4)

### AC1 — The receiver stops dropping the frame

*Scope note, decided at preflight: this AC is the story. Everything the split described as "build a
receiver" is already built and running (G1); what is missing is a sink, a consumer, and a journal
call. Do not re-implement admission, TOFU, consent or the Lamport clock — they execute today.*

1. **The worker-spawn surface moves under the library, and its callers follow.** Mirror `2a`'s
   relocation exactly (`lib.rs:19-22` doctrine, `2a` AC1.1 precedent): make
   `run_cli_wrapper_manifest` and the five private items it depends on — `RunArgs` (`main.rs:439-445`),
   `parse_sandbox_tier` (`:687`), `resolve_cli_binary` (`:704`), `load_host_grant_allowlist` (`:827`),
   `issue_enterprise_governed_capability` (`:201`) — reachable from `crates/maos-bin/tests/`. **Land
   this first, in its own commit** (T1), and record the measured `maos-bin` delta before and after.
   Without it, AC1.6's proof has no legal home (G2). *This is a move, not a rewrite: if the diff
   changes behaviour, you have left T1.*
2. **A production TCP transport installs an intake sink, and the two lying doc comments are
   corrected.** Add the `install_intake_sink` call to the daemon's `bind_*` path (or a `bind_with_*`
   sibling), using the public seam `TcpA2ATransport::core()` (`transport.rs:388`). In the same change,
   correct `router.rs:342-344` and `transport.rs:387` — copy the wording `install_rupture_sink`
   already carries at `router.rs:352-356` (*"Live transports install this before exposing their
   listener"*). A comment that says "test-only" over the sole production delivery path is how the next
   author builds a second mechanism (G1, G14).
3. **A drain task dispatches on `FrameKind` — the first such dispatch on any inbound path.** Today
   every intake branch keys on a consent-intent string (`consent_match_key`, `router.rs:496`) or a
   payload shape (`is_crossing_frame`, `router.rs:313-318`). The drain must handle
   `FrameKind::TaskAssign` and must **fail closed and loudly** on a kind it does not expect — a silent
   `_ => {}` reproduces G1 one layer up.
4. **Host B journals the inbound frame.** There are zero TL rows for an inbound frame today. Write it
   through `IacBusAdapter::deliver_typed` — the same shape `DelegationLeg::delegate` already uses on
   host A — **not** by adding a TL dependency to `maos-a2a-core` and **not** through the raw-byte
   `enqueue_frame`/`broadcast_frame` path, which mislabels every row `TaskAssign` (G14.iv).
5. **The worker is spawned without parking a reactor thread.** `run_cli_wrapper_manifest` is
   synchronous and blocks for the whole worker lifetime (`spawn_and_bridge` `main.rs:1179`,
   `pump_to_journal` `:1192`, `wait_and_finalize` `:1201`) inside
   `#[tokio::main(flavor = "multi_thread")]` (`main.rs:2361`). Use `spawn_blocking` or an equivalent;
   a direct call parks a worker thread for the duration.
6. **A hermetic two-daemon proof, on the RIGHT substrate.** Build on
   `crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs` — `boot_hermetic_daemon` (`:436-441`),
   `fixture()` (`:227-250`), `mint_pems` (`:163`) — **not** on `cross_team_crossing_13_6b.rs:1642`,
   which is `#[ignore]`, needs live Postgres, and carries a `TelemetryEvent` that never touches a
   Mailbox (G5). Two real `maos` processes, real mTLS, a `TaskAssign` delivered, a worker spawned on
   host B, a TL row on host B. **Distinct `MAOS_HOME` per process** (Trap 6). Assert the *absence*
   case too: with the sink uninstalled the frame must still be ACKed and the test must red — that is
   the falsifier for AC1.2.

### AC2 — The composition-root fork, without breaking the loopback leg it forks from

1. **`DelegationLeg::install` chooses its router; `main.rs:3150` does not move.** The `OnceLock`
   (`mailbox.rs:131`) is set exactly once by exactly one caller (`delegation.rs:110`), so the fork
   belongs inside `install`, not at the call site (G8). Moving `:3150` past the `MAOS_ONE_SHOT`
   dispatch at `:5250` puts it after the daemon arm returns — a dead end, do not attempt it.
   Host A passes `Arc<TcpA2ATransport>` directly: the `A2ARouter` impl already exists
   (`transport.rs:826-840`), so this is a construction change with **zero new adapter code** (G7).
2. **`TcpA2AConfig` reaches the `maos run` path WITHOUT touching the topology key allowlist.**
   `TOPOLOGY_SPIRIT_KEYS` is `["manifest", "path", "host"]` and rejects unknown keys
   (`topology.rs:57`, `:72-80`); two blocking controls pin the file to exactly one
   `developer-remote-host` entry (G6). **Add a new topology file** and route transport config through
   the existing daemon config path (`MAOS_COHORT_DAEMON_CONFIG` → `CohortDaemonFileConfig`,
   `main.rs:9374-9432`) or an equivalent operator file. Do not extend `j1-founder-loop.toml` and do
   not widen the allowlist in this story.
   *Mandatory fields, measured:* `TcpA2AConfig` (`crates/maos-a2a-tcp/src/config.rs:61`) needs
   `listen_addr` (`:64`), `own_cert_chain` (`:66`), `own_private_key` (`:68`), `peer_pins` (`:70`);
   `A2APeerConfig` (`crates/maos-a2a-core/src/config.rs:39`) needs `peer_id` (`:41`), `endpoint`
   (`:46`, `tls://IP:port` — **no DNS**), `cert_fingerprint` (`:52`). `PinnedFingerprint.boot_nonce`
   has **no serde default** — see AC4.1.
3. **Re-target 1b's boundary leg and PROVE the flip.** 1b's AC2.2a rewrites
   `leg_loopback_from_host_unverified` (`check_j1_loopback_delegation.rs:266-293`) to key on whether
   the **J1 delegation path binds wire identity**, rather than on two permanent strings in
   `router.rs`. `2b` is the story that flips it. Assert the transition
   `loopback_from_host_unverified: true → false` as a gate leg **with a planted-red vector**, and
   update `ledger_leg_names()` (`:64-66`) and the gate's `## Legs` module doc if the name changes.
   **If 1b has not landed the rewrite, this AC is not satisfiable — that is why 1b is a blocking
   condition, not a courtesy.**
4. **Leg 1 of the same gate must stay green on its own terms, and the change must be visible to it.**
   `leg_frame_borne_route_intact` (`check_j1_loopback_delegation.rs:153-167` and the `main.rs` /
   `delegation.rs` needles) governs five files `2b` will edit. Read it before you write; if a needle
   must move, move it deliberately and add the planted-red vector — **never delete a leg to make a
   change fit** (the gate's own text: *"do not delete the leg"*).

### AC3 — Prove the crossing with the key that already exists, and make the word mean something

*Scope note, decided at the 2026-08-16 round-table: **this AC lost three items to measurement.** The
original AC3.1/3.2/3.3 asked for a correlation token on `IacFrame`, a new TL writer to carry it, and a
join across the two. All three are unnecessary — **the join key already crosses the wire and already
lands in both logs and both bundles.** They are recorded here as G16 rather than deleted silently,
because the reasoning is what stops a future author from re-adding them.*

1. **Prove the crossing with `frame_id`. Do NOT add a correlation field to `IacFrame`.**
   Measured end-to-end at HEAD:
   - Host A mints the id and it is **deterministic**, not random:
     `assign_frame_remote` overwrites bytes 8..16 with `run_nonce`
     (`spirits/orchestrator/src/lib.rs:357`), and `journal_completion` builds it as
     `seq.to_le_bytes() ‖ run_nonce.to_le_bytes()` (`crates/maos-bin/src/delegation.rs:240-242`).
   - It travels on the frame and survives intake unchanged.
   - **`deliver_typed` writes `Some(frame.frame_id)`** — the *received* id, not a fresh one
     (`crates/maos-iac/src/adapter.rs:562`). So the instant AC1.4 journals the inbound frame through
     `deliver_typed`, host B's row carries **host A's `frame_id`**.
   - **`maos_audit::query` selects `frame_id` as its FIRST column**
     (`crates/maos-audit/src/lib.rs:194-196`), so it is already `AuditEntry.frame_id_hex` in every
     bundle — which is what `2c` reconciles on.
   **Consequence:** the two-host join needs **zero** new fields, **zero** `maos-domain` lines (a crate
   RED at −50), and **zero** `maos-audit` lines. Write the test that joins the two logs on
   `frame_id`; that is the whole deliverable. `correlation_id` is a real TL column with a real index
   and a real filter, and it is **not** the join key — do not spend budget wiring it for this purpose
   (and note `maos_audit::query` drops it, so it would not reach a bundle anyway).
2. **SHIP-BLOCKER, promoted from AC3.5 — the same fact is a remote-triggerable kernel halt, and AC1.2
   is what makes it reachable.** `frame_id` is `BLOB NOT NULL PRIMARY KEY`
   (`crates/maos-iac/src/adapter/transparency_log.rs:259`), the value is **supplied by the peer**, it
   is deterministic `(seq ‖ run_nonce)` with no ULID entropy, and a failed write is
   `Err(e) => panic!("MAOS kernel panic — Transparency Log write failed…")` (`:819-825`).
   **A peer that re-sends one frame halts host B.** It is unreachable today only because the frame is
   ACKed and dropped (G1) — so **the single call this story exists to add is also what opens it.**
   Fix it in the same change as AC1.2, not later:
   - Match **only** the UNIQUE/constraint-violation arm — `rusqlite::Error::SqliteFailure(e, _)` with
     `e.code == ErrorCode::ConstraintViolation` — and return a typed `Duplicate` outcome. The
     vocabulary to mirror is `DigestReplyObservation::{Accepted, Duplicate, Unauthorized}`
     (`crates/maos-a2a-core/src/cohort.rs:286-295`) — its *shape*, not its in-memory implementation.
   - **Every other write error must keep panicking.** That `panic!` is the I2 log-before-deliver
     guarantee; converting it wholesale trades a denial-of-service for silent audit loss, which is
     worse. State this constraint in the code comment, or the next reader deletes the panic.
   - Negative test: deliver the same frame twice to host B; assert a typed `Duplicate` and a **live
     process**. The falsifier is that the test reds at HEAD-plus-AC1.2 without the fix.
3. **A test joins the two logs on `frame_id` and asserts both halves.** Host A's emit row and host B's
   intake row, same 16 bytes, two distinct data homes (Trap 7). `reconcile_correlated_frames`
   (`transparency_log.rs:1888-1915`) exists for the correlated case — for `frame_id` a direct
   two-store query is simpler and is what `2c` will reconcile on. **Do not build a second join
   helper.**
4. **`journal_completion` stops hardcoding the word.** `delegation.rs:243` writes
   `"completed".to_string()` while the richer typed outcome is in hand at the call site —
   `WorkerCompletion::label()` (`worker_cli.rs:71-81`) already yields three values and `main.rs:3980`
   already prints one of them as `"result"` in the JSON event (G11). Thread the real outcome into the
   frame. **`crates/maos-bin/tests/delegation_leg_1a.rs:234` (`assert_eq!(p.result, "completed")`)
   goes red on contact** — update it deliberately, in the same commit, and say so.
5. **Do NOT build a general dedup store.** The sketched at-least-once hazard does not exist: retries
   are admitted only for pre-send handshake classes (G3), so the transport is at-most-once. The real
   replay hazard is a *peer* re-sending, and it is handled by AC3.2 at the storage layer, where the
   uniqueness constraint already lives. A second state store — in particular the
   `DigestReadPort`-style four `Mutex<HashMap>`s capped at 256 and wiped on restart
   (`crates/maos-cohort/src/state.rs:111-117`) — would be a weaker guard in front of a stronger one.
6. **The `TaskComplete` return hop is DEFERRED to `2c`, and `2b` must not claim a round trip.**
   *(Q1 resolved at the round-table.)* G9 enumerates five required parts, and `consent_envelope: None`
   (`delegation.rs:285`) means a partial attempt does not degrade — it **fails closed** at the sender
   with `ConsentUnclassified{Send, Absent}` (`router.rs:697`). A half-built return hop is a refusal,
   not a weaker return hop, so there is no useful middle. `2b` therefore proves: host B journals its
   outcome locally, and host A and host B's logs are joined on `frame_id` (AC3.1, AC3.3).
   **Language constraint, binding on every artifact this story produces:** `2b` may claim *"the
   crossing is proven in two logs"*. It may **not** claim *"round trip"*, *"the task completed back on
   host A"*, or anything implying a frame returned. The return hop is a **second delegation in the
   opposite direction** — second intent, second allowlist pair, second consent envelope — and naming
   it as a partial feature is how the next reader inherits a claim nobody built.
7. **D18 lands, slips by decision, or is made net-neutral — pick one in writing.** Its deadline is
   this story's first line. Re-measured (G12) the fix is ≈ **+5** in `maos-a2a-core` (**0 headroom,
   D10**) and ≈ **+14** in `maos-domain` (**RED −50, D14**), not the ≈0 the register records. Also
   correct the register's premise (a): `IntentDirection::Accept` is **unreachable in production**, so
   "already distinguishable" is a claim standing in for a control.

### AC4 — Bound the claim honestly, and do not hand `2c` a surprise

1. **State the production gaps this mechanism has, as machine-asserted boundaries, not prose.**
   Three, all measured, all discovered here:
   - **Boot-nonce (G4).** As built, a release-mode two-host run NACKs its first frame
     (`CODE_SPIRIT_RESTART_DETECTED`) and **permanently invalidates the pin**, because host B pins a
     nonce statically while host A generates one per process and the test override is
     `debug_assertions`-gated. Either fix the pairing or state the boundary and assert it with a
     negative test. **`2c` runs a paid agent against this.**
   - **Non-expiring consent (G15).** The delegation envelope is minted `valid_until_ns = None`
     (`spirits/orchestrator/src/lib.rs:328-330`); the expiry check is a no-op for it
     (`router.rs:1207-1222`). Inert on loopback, a bearer grant with no expiry on a real wire. Name an
     owner.
   - **No DNS.** Peers are bare `IP:port` (`transport.rs:434`, `:468`), while `A2APeerConfig::validate`
     accepts a hostname whose own doc example is `tls://host-b.internal:7443` (`config.rs:43-45`,
     `:115-136`) — it passes bind and fails at first dial. Say what `2b` proved and what it did not.
   Use the precedent `2a` established: a stated posture a capture cannot overclaim, with a negative
   test refusing the overclaim direction (`CaptureDoc::validate`,
   `crates/maos-cli/src/subcommands.rs:2285-2360`, negative at `:3935`).
2. **Budget, measured against 2a's merge, attributed by key.** Re-measure `maos-bin` after T1 and
   after 2a lands; record before/after in the Dev Notes. Take **no** grant unless still over, and then
   only with the measurement attached (`kloc.toml:60-65`). Do **not** absorb the standing reds —
   `maos-kernel-core` +685 (D13), `maos-domain` +50 (D14), `_aggregate` +492 (D17). `kloc-check` will
   exit 1 at close through no fault of this story. **The measurement must be taken in a clean tree**
   (`git archive <commit> | tar -x -C <tmp>`): two of this story's own scouts drew a false conclusion
   about the D15 ceiling by measuring a working tree that 2a was mutating.
3. **CI enrollment on a job that already blocks, with no `services:` block.** Extend
   `check-j1-loopback-delegation` (`discipline.yml:1804-1821`) — it is `BindingClass::Blocking`, in
   `gate-registry.toml`, and a `needs:` of the ship gate (`:3177`). **Do not add a `services:` block
   to any new job**: `check_loom_substrate_drift`'s leg 2 rejects an unregistered service-bearing gate
   job and is itself blocking and in ship-gate needs (`:3193`). The shape to copy is
   `check-live-bilateral-consent` (`:2437-2449`) — two real TCP endpoints, zero external services.
   Remember there is **no unscoped `cargo test -p maos-bin` anywhere in CI**: a new test file that is
   not `--test`-enrolled is a suggestion, not a control.
4. **Close the record honestly.** Disclose the seven blind story-file gates (**D19**,
   `epic-14-preflight-decisions.md`; its deadline is *before the next `j1-*` story leaves
   `ready-for-dev`*, which is this one) and populate the model/§A6 fields anyway. Report "no ABI change
   was made" as a fact about your diff, never as an `abi-diff` result (FLAG-E4). If a `demo-j1` beat
   moves, move it in code and say the ledger did not enforce it.

---

## Traps

1. **Coordinate with `2a` on exactly two regions.** `main.rs:4355-4367` (2a's standalone
   `is_completed()` change rewrites the same 12 lines this story must fork) and the
   `completion_tl_ref` → `last_stdout_tl_ref` rename, whose emitted JSON key (`main.rs:1282`) is the
   natural input to cross-host correlation. Everything else in 2a is textual-merge-only. **Sequence
   2a → 2b as ratified; do not parallelize those two hunks.**
2. **Do not copy `check_vetting_attestation::invoke_leg`.** It builds `Command::new("cargo")` with no
   `current_dir` and inherits the proven-red tempdir. Still true at HEAD.
3. **Any new gate leg must read via `root.join(rel)`, never a hardcoded path.** The proven-red harness
   sets `current_dir(tempdir)`; a leg using `Path::new("…")` resolves against the tempdir. If the miss
   is a Finding, `baseline_fixture_tree_is_green` reds and the suite is unrunnable; if it is a skip,
   every vector for that leg passes **vacuously**. Known-dangerous callees: `gate_common::read_disposition`
   (`:63-65`), `check_ship_gate_completeness` (`:174`), `evidence_ledger::REPORT_DIR` (`:73`).
4. **Do not extend `spirits/topologies/j1-founder-loop.toml`** (G6). Two blocking controls pin it.
5. **Do not add a `std::env::var*` read under `crates/maos-bin/src` without registering it** —
   `check_env_contract.rs:119-160` walks that tree and fails. Note the converse null control: it walks
   **only** `crates/maos-bin/src/`, so a var read from `maos-a2a-tcp`, `xtask` or any test is invisible
   to it. Registering is still the rule.
6. **`cargo test -p maos-bin` is RED under default parallel flags** (D16, `MAOS_HOME` is
   process-global). Run scoped and `--test-threads=1`. D16 belongs to 14-0.
7. **FR21's 60s wall-clock window bites two processes on one box.** `check_orchestrator_distillate_required`
   (`crates/maos-iac/src/adapter/orchestrator_dispatch.rs:63-146`, window
   `DEFAULT_ORCHESTRATOR_DISPATCH_WINDOW_NS = 60_000_000_000` at `:40`) has **no** pid, session,
   orchestrator or boot-nonce scoping — any `TaskComplete` row in the window refuses the next
   `TaskAssign`. It keys on the **TL file path**, so distinct data homes genuinely avoid it — but
   `MAOS_HOME` **outranks** `XDG_DATA_HOME` (`crates/maos-audit/src/lib.rs:872-902`), so setting only
   the latter does not isolate. The advertised escape hatch `MAOS_ORCHESTRATOR_DISPATCH_WINDOW_NS`
   (`orchestrator_dispatch.rs:36-39`) has **zero code readers workspace-wide** — it is prose.
8. **A duplicate `frame_id` journaled twice PANICS the kernel** (`transparency_log.rs:819-825`), and
   J1 frame ids are **deterministic** `seq ‖ boot_nonce` (`spirits/orchestrator/src/lib.rs:358`,
   `delegation.rs:241-243`). In debug builds `MAOS_TEST_BOOT_NONCE` pins the nonce (`main.rs:2585`).
   **A harness that pins the nonce for reproducibility — which G4 says you need — walks straight into
   this on the second run.** Use fresh data homes per run, and see AC3.5.
9. **Two source-inspection suites assert the exact text of the daemon region you are about to edit.**
   `crates/maos-bin/tests/enterprise_daemon_seam_13_5a.rs:56-105` asserts the
   `if mode == "cohort-a2a-daemon"` block and `build_cohort_a2a_daemon_runtime`'s **signature** contain
   seven exact substrings; `cross_team_crossing_13_6b.rs:320, 373, 478, 502` use the same technique.
   Both will red.
10. **`A2AProfile` is dead config and its default lies.** `{Loopback, CrossHost}` is never read to
    select behaviour, and `default_profile()` is `Loopback` (`config.rs:79-81`) — so a peer TOML
    omitting `profile` on a TCP transport is silently `Loopback` and behaves identically. Never derive
    a "cross-host" claim from this field.
11. **`maos-a2a-core` is at 4654/4654 and frozen by D10.** One production line hard-fails
    `kloc-check`. If your change lands there, re-route it to `maos-a2a-tcp` (+415) or `maos-bin`, or
    take a named grant.
12. **Every line put in `crates/maos-a2a-tcp/tests/` runs 51× per push** — 50× from the determinism
    loop (`discipline.yml:1538-1544`, `timeout-minutes: 10` at `:1524`) plus once from the scoped
    gate. A test that waits out the 60s prod idle timeout or the ~130s unbounded connect **cannot live
    in that crate**.
13. **The bridge never parses NDJSON.** `runtime.rs:388-390` handles `NdjsonOverStdio` and `Raw`
    identically despite the manifest declaring `ndjson_over_stdio`. Unchanged by this story; do not
    assume otherwise when reading worker output on host B.
14. **Do not unify the seven `TaskComplete` producers** (G11). Two of them —
    `crates/maos-kernel-core/src/halt/resolver.rs:222` and
    `crates/maos-kernel-core/src/supervision/crash_detector.rs:163` — are **inside kernel-core**, and
    the pin counts physical `.rs` lines in every file under that directory
    (`check_kernel_baseline.rs:99-110`). Touching either breaks ZERO-Δ.
15. **`delegated_task` carries only a goal string.** `Option<&str>` at `main.rs:929`, consumed at
    `:1074-1077`; `None` means *no delegation*, not a default. It carries no task id, no correlation,
    no consent, no lineage — AC3.1 requires widening it or replacing it with a typed struct, and
    **both** call sites (`:3947`, `:4357`) change.
16. **`maos-iac` has +36 lines of headroom.** AC3.2's writer and AC3.5's typed `Duplicate` both land
    there. Measure before writing, not after.

---

## Tasks

- [ ] **T1 (AC1.1)** — Relocate `run_cli_wrapper_manifest` + its five private dependencies under the
      library, mirroring `2a`'s `pub mod` precedent. **Own commit.** Record `kloc-check` `maos-bin`
      before and after, measured in a clean tree. **Expect ~zero delta and do not plan around a
      refund:** unlike `2a`'s move — which relocated a budget-charged, CI-invisible `#[cfg(test)]`
      module into kloc-excluded `tests/` and returned 204 lines — this is `main.rs` → `lib.rs` inside
      the **same crate**, so it is kloc-neutral. It buys testability, nothing else.
- [ ] **T2 (AC1.2, AC1.3)** — Install the intake sink on the production daemon path; correct the two
      false doc comments; add the `FrameKind` drain with a fail-closed default arm.
- [ ] **T3 (AC1.4, AC1.5)** — Journal the inbound frame via `deliver_typed`; spawn the worker off the
      reactor.
- [ ] **T4 (AC1.6)** — Two-daemon hermetic proof on the `cohort_daemon_smoke_13_5c.rs` substrate,
      distinct `MAOS_HOME` per process, including the sink-uninstalled falsifier.
- [ ] **T5 (AC2.1, AC2.2)** — Router fork inside `DelegationLeg::install`; new topology file; transport
      config through the daemon config path. Do not widen `TOPOLOGY_SPIRIT_KEYS`.
- [ ] **T6 (AC2.3, AC2.4)** — Re-target 1b's boundary leg, assert the `true → false` flip, add the
      planted-red vector; re-read leg 1's needles and move them deliberately if required.
- [ ] **T7 (AC3.1, AC3.3)** — The two-host join test on `frame_id`, two distinct data homes. **No new
      frame field, no new TL writer, no `correlation_id` wiring** — the key already crosses the wire
      and is already projected into both bundles (G16).
- [ ] **T8 (AC3.2)** — **SHIP-BLOCKER, same commit as T2.** Convert ONLY the constraint-violation arm
      of the TL write into a typed `Duplicate`; every other write error keeps panicking (I2). Negative
      test: deliver the same frame twice to host B, assert `Duplicate` **and a live process**.
- [ ] **T9 (AC3.4, AC3.6, AC3.7)** — Thread the real `WorkerCompletion` outcome into the frame and
      update `delegation_leg_1a.rs:234`. Record the return-hop deferral and the no-round-trip language
      constraint. D18: land, slip by decision, or construct net-neutral — and correct the register's
      premise (a).
- [ ] **T10 (AC4.1)** — The three bounded gaps (boot-nonce, non-expiring consent, no DNS) as negative
      tests in the `CaptureDoc::validate` overclaim-refusing shape.
- [ ] **T11 (AC4.3)** — CI enrollment on `check-j1-loopback-delegation`, no `services:` block, plus a
      proven-red file copying `xtask/tests/j1_crosshost_1a_proven_red.rs` (`lay_green` `:72`,
      `assert_red`'s three-part assertion `:106-124`, baseline control `:130-139`).
- [ ] **T12 (AC4.2, AC4.4)** — Re-measure and attribute budget in a clean tree; Dev Agent Record;
      disclose D19; correct `demo_j1.rs:797-801`'s stale owner string if `2c` has not yet.

### Review Findings

_(populate at review; §A6 net is non-degradable — Blind Hunter · Edge Case Hunter · Acceptance
Auditor · Test-Infra Auditor · runtime execution.)_

---

## Dev Notes

### Measured at CLEAN HEAD `5a921c0c` — `git archive`, not the working tree

| Instrument | Ceiling | Measured | Verdict |
|---|---|---|---|
| kloc `maos-bin` | **16260** (post-2a, committed) | **16260** | **ZERO headroom.** D15 16178→16219, then 2a's review grant →16260. Both exact-measured |
| kloc `maos-a2a-core` | 4654 | **4654** | **ZERO headroom — the D10 wall** |
| kloc `maos-a2a-tcp` | 1500 | **1085** | **+415** — the only uncontested capacity in the lane, no grant history |
| kloc `maos-iac` | 6888 | **6852** | +36 |
| kloc `maos-domain` | 8644 | **8694** | RED −50 — D14, not yours |
| kloc `maos-kernel-core` | 18248 | **18933** | RED −685 — D13, not yours |
| kloc `xtask` | 38609 | **38386** | +223 (2a spent 311) |
| kloc `_aggregate_hardfail` | 147057 | **147942** | RED −885 — D17, standing, not yours |
| `check-kernel-baseline` | 24472 | **24472** | GREEN |
| Zero-cost surfaces | — | `crates/*/tests/`, `xtask/tests/`, `xtask/src/tests/`, **all of `spirits/`** | `kloc_check.rs:167-193` |

> **Why "clean tree" is in the AC.** Two scouts on this very preflight measured `maos-bin` at 16017
> and 16132 and concluded the D15 ceiling record was a broken instrument. Both had measured a working
> tree that `2a` was actively mutating. Re-measured via `git archive 5a921c0c`, `maos-bin` is
> **16219 — exactly the ceiling**. The instrument is fine; the measurement was contaminated.

### The receive path, hop by hop — read this before touching the transport

| # | Hop | file:line |
|---|---|---|
| 1 | `TcpListener::bind` + `tokio::spawn(accept_loop)` | `transport.rs:343`, `:355` |
| 2 | `accept_loop` → `listener.accept()` | `transport.rs:504`, `:516` |
| 3 | `serve_connection` (server TLS accept, `timeouts.handshake`) | `transport.rs:562`, `:577-580` |
| 4 | `resolve_verified_peer` (re-derives mTLS identity) | `transport.rs:679-688` |
| 5 | **`core.handle_intake_verified(...)`** | `transport.rs:637-643` → `router.rs:1494` |
| 6 | host binding / TLS-peer match | `router.rs:1504-1521` |
| 7 | `handle_intake_inner` → peer lookup → TOFU | `router.rs:1070`, `:1093`, `:1105` |
| 8 | **boot-nonce restart check** (G4) | `router.rs:1123-1159`, `tofu.rs:351-372` |
| 9 | consent granter / expiry (expiry no-op when `valid_until_ns` is `None`) | `router.rs:1169-1223`, `:1207-1222` |
| 10 | accept-allowlist | `router.rs:1313` |
| 11 | Lamport advance | `router.rs:1451` |
| 12 | **`if let Some(sink)` — `intake_sink` is `None`, frame DROPPED** | `router.rs:1454-1458`, init `:218` |
| 13 | ACK `delivered: true` | `router.rs:1459-1465` |

### Tests and gates that constrain the change

| file:line | What | Exposure |
|---|---|---|
| `check_j1_loopback_delegation.rs:383-429` | the boundary leg 1b rewrites | AC2.3 flips it |
| `topology_delegation_1a.rs:228-239` | `hosts == vec![TO_HOST]` | Same constraint as above |
| `delegation_leg_1a.rs:123-135` | **exactly one** `TaskAssign` TL row per delegation | Reds if host A journals twice |
| `delegation_leg_1a.rs:234` | `p.result == "completed"` | **Breaks by design** — AC3.4 |
| `enterprise_daemon_seam_13_5a.rs:56-105` | seven exact substrings in the daemon block + builder signature | **Breaks** — you are editing that region |
| `cross_team_crossing_13_6b.rs:320, 373, 478, 502` | same source-inspection technique | **Breaks** |
| `router.rs:2333-2342` | `consent_envelope: None` ⇒ `ConsentUnclassified{Send, Absent}` | Pins G9 |
| `cohort_daemon_smoke_13_5c.rs:443-476` | hermetic two-daemon boot, **not** `#[ignore]` | **Clone this — it is the harness** (G5) |
| `mailbox_a2a_router_installer_1a.rs` | the set-once installer contract | On the critical path for AC2.1 |
| `t12a_kernel_zero_auto_retry_dep_absent` (`t11_t12_chaos_absence.rs:170-176`) | greps `maos-a2a-tcp/Cargo.toml` for `maos-kernel-core` | Reds if you add that dep to reach a TL |

### Where the code goes

| Concern | File | Anchor |
|---|---|---|
| **Enabling move** | `crates/maos-bin/src/lib.rs` | mirror `2a`; doctrine at `:19-22` |
| Worker spawn (to relocate) | `crates/maos-bin/src/main.rs` | `run_cli_wrapper_manifest` `:918-1289` + `:439`, `:687`, `:704`, `:827`, `:201` |
| Sink install seam | `crates/maos-a2a-tcp/src/transport.rs` | `core()` `:388`; bind chain `:139/:166/:196/:229/:268` |
| Sink declaration + the false doc | `crates/maos-a2a-core/src/router.rs` | `intake_sink` `:175`/`:218`, push `:1454-1458`, `install_intake_sink` `:345`, doc `:342-344`, **correct pattern** `:352-356` |
| Router fork | `crates/maos-bin/src/delegation.rs` | `DelegationLeg::install` `:102-131`, installer call `:110` |
| Outbound port impl (reuse) | `crates/maos-a2a-tcp/src/transport.rs` | `impl A2ARouter for TcpA2ATransport` `:826-840` |
| Daemon builder | `crates/maos-bin/src/main.rs` | `build_cohort_a2a_daemon_runtime` `:10098-10226`, bind at `:10147-10165` |
| Daemon config path | `crates/maos-bin/src/main.rs` | `CohortDaemonFileConfig` `:9374-9385`, loader `:9432`, env `:9413` |
| Correlation writer | `crates/maos-iac/src/adapter/transparency_log.rs` | `:605-633`, `:758-769`, join `:1888-1915` |
| Task id type (unused) | `crates/maos-domain/src/ports/task.rs` | `TaskAssignmentRecord` `:12-23` |
| Outcome labels | `crates/maos-bin/src/worker_cli.rs` | `WorkerCompletion::label()` `:71-81` |
| Return-hop builder (none exists) | `spirits/orchestrator/src/lib.rs` | `assign_frame_remote` `:332`; **`spirits/` is kloc-free** |
| Gate to EXTEND | `xtask/src/check_j1_loopback_delegation.rs` | files `:55-60`, legs `:64-66`, idiom `:137`, aggregate `:306`, binding `:308` |
| Proven-red template | `xtask/tests/j1_crosshost_1a_proven_red.rs` | `lay_green` `:72`, `assert_red` `:106-124`, baseline `:130-139` |
| Harness to clone | `crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs` | `fixture()` `:227-250`, `mint_pems` `:163`, `boot_hermetic_daemon` `:436-441` |
| CI job | `.github/workflows/discipline.yml` | job `:1804`, gate `:1815`, proven-red `:1817`, legs `:1819-1821` |

### References

- Shared preflight: `_bmad-output/implementation-artifacts/j1-crosshost-2-cross-host-signed-run.md`
  (§2 P1-P14 — **note P1, P4 and the `main.rs:10678` claim are corrected here by G5, G3 and G7**)
- Predecessors: `j1-crosshost-1a-frame-borne-delegation.md` (done, `6827dc87`),
  `j1-crosshost-1b-consent-proofs-and-gate.md`, `j1-crosshost-2a-signable-heterogeneous-worker.md`
- Successor: `j1-crosshost-2c-two-host-signed-run.md` — inherits AC3's correlation and AC4.1's gaps
- Decision register: `_bmad-output/planning-artifacts/epics/epic-14-preflight-decisions.md`
  (D10, D13, D14, D15, D16, D17, **D18 — its measurement is corrected by G12**, D19)

---

## Dev Agent Record

### Agent Model Used

_(record `vendor/model` + harness + date. **Required by policy even though seven story-file gates
skip this filename — D19.** A green CI does not mean the §A6 net ran.)_

### Debug Log References

### Completion Notes List

### File List

---

## Open Questions

**Q1 — Does the `TaskComplete` frame travel back in `2b`, or does `2c` carry it?** AC3.6 permits
either but forbids silence. G9 measured five parts, of which `consent_envelope: None`
(`delegation.rs:285`) is the one that makes a partial attempt fail closed rather than degrade.
Recommendation: **defer the return frame to `2c`** and prove the crossing in `2b` by correlation-joined
TL rows on both hosts. Rationale: a return hop is a *second* delegation in the opposite direction —
second intent, second allowlist pair, second consent envelope — and bundling it makes `2b` a two-way
protocol story rather than a mechanism story. Operator's call; the AC accepts either as long as it is
written down.

**Q2 — Is the debug-only boot-nonce pairing an acceptable posture for `2b` to ship on?** G4 measured
that a release build cannot complete a two-host handshake without out-of-band nonce pairing, and the
only existing precedent (`cross_team_crossing_13_6b.rs:1617`) is a `debug_assertions`-gated test
override. Options: (a) ship `2b` as a debug-CI-provable mechanism with the boundary asserted, and give
the production pairing to `2c` or a named successor; (b) fix the pairing here — which means either a
first-contact nonce learn on the listen side or a nonce-agnostic pin, both of which touch
`maos-a2a-core` at zero headroom. Recommendation: **(a)**, because `2c`'s paid run is where the gap
becomes expensive and it should arrive there named rather than discovered. Operator's call.

---

## Change Log

| Date | Change |
|---|---|
| 2026-08-16 | **Preflight round-table** (Winston · Murat · Amelia · John · Mary · Paige · Sally; Vex on the security read). **Three ACs collapsed into one test, and the story's headline finding was promoted to a SHIP-BLOCKER — both from the same measurement.** **(G16)** AC3.1/3.2/3.3 asked for a correlation token on `IacFrame`, a TL writer to carry it, and a join. All unnecessary: `deliver_typed` writes `Some(frame.frame_id)` — the **received** id (`crates/maos-iac/src/adapter.rs:562`) — and `maos_audit::query` selects **`frame_id` as its FIRST column** (`crates/maos-audit/src/lib.rs:194-196`), so the moment AC1.4 journals the inbound frame both logs share a key and both bundles already carry it. Cost avoided: a field in `maos-domain` (**RED −50**) and lines in `maos-audit` (**+22**). `correlation_id` is a real column and is **not** the join key. **(AC3.2, promoted)** The same fact is a **remote-triggerable kernel halt**: `frame_id` is `BLOB NOT NULL PRIMARY KEY` (`transparency_log.rs:259`), the value is **peer-supplied**, it is **deterministic** `seq ‖ run_nonce` with no ULID entropy (`spirits/orchestrator/src/lib.rs:357`, `delegation.rs:240-242`), and a failed write is `panic!` (`:819-825`). A peer that re-sends one frame halts host B — unreachable today **only** because the frame is ACKed and dropped, so **the single `install_intake_sink` call this story exists to add is what opens it.** Fix lands in the same commit as AC1.2 and must match **only** the constraint-violation arm; every other write error keeps panicking, because that `panic!` is the I2 log-before-deliver guarantee and trading it wholesale swaps a DoS for silent audit loss. **(Q1 resolved)** The `TaskComplete` return hop is **deferred to `2c`** — `consent_envelope: None` makes a partial attempt fail *closed*, so there is no useful middle — with a binding language constraint: `2b` may claim *"the crossing is proven in two logs"*, never *"round trip"*. **(T1 clarified)** unlike `2a`'s relocation, which returned 204 lines, this move is `main.rs` → `lib.rs` **inside the same crate** and is kloc-neutral; it buys testability, not budget. 4 ACs unchanged; AC3 shrank from 7 items to 6 and got sharper. |
| 2026-08-16 | **Created** at clean `5a921c0c` from a five-scout preflight (composition root & transport · host-B consumption · correlation/dedup/outcome vocabulary · faults & pin journaling · gates/CI/budget), following the 2026-08-15 ratification of the `2a/2b/2c` split. Status **`backlog`** with four mechanical blocking conditions, per this lane's convention (`blocked` is not a status). **Fifteen premises disproved or corrected.** Headline: **(G1) the receiver is not missing — it authenticates, runs TOFU and consent, advances Lamport, ACKs `delivered: true`, and DROPS the frame**, because `intake_sink` is `None` (`router.rs:218`) and no `bind*` ever installs one; the whole story is one `install_intake_sink` call plus a consumer behind it, and the seam is already public (`transport.rs:388`, `router.rs:345`) under a doc comment that falsely calls it "test-only". **(G2)** the worker-spawn surface `run_cli_wrapper_manifest` and five helpers are `main.rs`-private — the same ship-blocker shape `2a` hit, so the proof has no legal home until they move. **(G3)** "duplicate-delivery safety" targets a hazard that does not exist: `is_retryable` admits only pre-send handshake classes, so the transport is **at-most-once**, and the receiver is at-zero-once anyway. **(G4) SHIP-BLOCKER-CLASS** — the boot-nonce handshake makes a *release-build* two-host run impossible (host B pins statically, host A regenerates per process, the override is `debug_assertions`-gated, and the mismatch **permanently invalidates the pin**); `2c` would meet this with a paid agent attached. **(G5)** the substrate the shared preflight names is `#[ignore]` + live-Postgres + `TelemetryEvent` and never touches a Mailbox — use `cohort_daemon_smoke_13_5c.rs` instead, which decides whether the proof is blocking or advisory. **(G6)** the topology key allowlist is strict and two blocking controls pin the file to one `developer-remote-host` entry, so transport config cannot go there. Corrections: **(G7)** `main.rs:10678` is a smoke, not a production composition path — but `impl A2ARouter for TcpA2ATransport` (`transport.rs:826`) makes host A's side zero-code anyway; **(G8)** the fork belongs inside `DelegationLeg::install` (one caller), not at `main.rs:3150`; **(G9)** the return hop needs five parts and `consent_envelope: None` makes a partial attempt fail closed; **(G10)** correlation is two missing mechanisms (no field on `IacFrame`, no writer taking correlation + `frame_id` + sender) and `TaskAssignmentRecord.task_id` already exists with zero writers; **(G11)** the outcome vocabulary exists in `WorkerCompletion::label()` and is thrown away at `delegation.rs:243`; **(G12) D18's ≈0-line resolution does not re-measure** — it is ≈+5 into a zero-headroom crate and ≈+14 into a red one, and its premise (a) is a null control because `IntentDirection::Accept` is unreachable in production; **(G13)** `check-a2a-sender-completeness` structurally excludes `delegation.rs`; **(G14)** four misleading comments including a live wire-format lie at `mailbox.rs:653-662`; **(G15)** the delegation consent envelope is a non-expiring bearer grant, unowned since the split. 4 ACs, 16 traps, 12 tasks; ZERO kernel-Δ @24472, measured and argued rather than intended. Budget re-measured in a clean `git archive` tree after two scouts drew a false "the D15 ceiling is a broken instrument" conclusion from a tree `2a` was mutating: `maos-bin` is **16219 exactly** and the record reproduces. |
