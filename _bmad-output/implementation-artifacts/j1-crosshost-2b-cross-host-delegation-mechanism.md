---
baseline_commit: "**87eb6c37**, measured in a CLEAN tree (`git status --porcelain` empty, verified before every measurement below). Supersedes the `5a921c0c` pin this file was authored against. `j1-crosshost-2a` (`0769869d`) and `j1-crosshost-1b` (`87eb6c37`) BOTH landed after authoring and BOTH moved things under this story — ceilings, gate legs, `main.rs` line numbers, the demo beat ledger, and two of this file's own findings. Every number below was re-derived at `87eb6c37` by six parallel scouts on 2026-08-16. **Inherit no line number from the `5a921c0c` edition of this file.** Useful invariant: `git diff 5a921c0c..HEAD -- crates/maos-a2a-tcp crates/maos-a2a-core crates/maos-iac` is **EMPTY** — every cite into those three crates is as valid as when it was written; the churn is in `crates/maos-bin/src/{main,worker_cli,lib}.rs`, `crates/maos-cli/src/subcommands.rs`, `xtask/`, `.github/workflows/discipline.yml` and `spirits/`."
depends_on: "**BOTH SATISFIED.** `j1-crosshost-2a` = `done` (`0769869d`, §A6 closed, reviewer `zai/glm-5.2` ≠ dev `anthropic/claude-opus-5`). `j1-crosshost-1b` = `done` (`87eb6c37`, same reviewer split) **with rung-1 evidence verified `PROVEN_BLOCKING` by execution, not by record**: `cargo run -q -p xtask -- demo-j1` renders `disallowed-intent-refused-blocking  PROVEN_BLOCKING` and all six gate legs `PROVEN_BLOCKING`; gate JSON at HEAD is `passed:true, oracle_green:true, binding:\"Blocking\", findings:[]` with leg check-counts 7/2/5/3/5/18."
blocks: j1-crosshost-2c-two-host-signed-run
split_from: j1-crosshost-2-cross-host-signed-run (three-way split RATIFIED by Lunarpulse 2026-08-15; that file is the shared preflight for 2a/2b/2c)
kernel_grant: "NONE, and it is not at risk. `check-kernel-baseline` **PASSED at 24472 = pinned 24472**, re-run at HEAD (**23679 is stale, do not inherit it**). Measured: `crates/maos-kernel-core/src/iac.rs:13` is a 66-line `pub use maos_iac::*;` shim — `Mailbox`, `IacBusAdapter` and `SpiritMailboxHandle` all physically live in `crates/maos-iac/`. `maos_domain::ports::a2a::A2ARouter` (`crates/maos-domain/src/ports/a2a.rs:22-36`) has **exactly one method**, `route_outbound` (`:35`) — it is outbound-only, so an inbound pump needs no domain or kernel trait change. The two kernel-core files that touch this lane — `halt/resolver.rs:223` and `supervision/crash_detector.rs:164` — are named in Trap 14 as OUT of scope precisely because the pin counts physical `.rs` lines in every file under that directory (`xtask/src/check_kernel_baseline.rs:99-110`). Do NOT cite `abi-diff`: it scopes to `crates/maos-spirit-abi` only (`xtask/src/abi_diff.rs:8`), open **FLAG-E4**."
kloc_grant: "**REQUIRED, AND THE WALL MOVED AGAINST THIS STORY TWICE SINCE AUTHORING.** Re-measured at `87eb6c37` with the gate's own measurer: `maos-bin` **16260/16260 = ZERO** (unchanged), `maos-cli` **4642/4642 = ZERO**, `maos-a2a-core` **4654/4654 = ZERO** (the D10 wall), **`xtask` 38655/38655 = ZERO — was +223; `j1-crosshost-1b`'s §A6 review took a measured grant 38609→38655 and its own record says *“THE NEXT xtask STORY CANNOT LAND ONE LINE WITHOUT A MEASURED GRANT.”* That is this story.** Remaining capacity: `maos-a2a-tcp` **1085/1500 = +415** (still the only uncontested room in the lane), `maos-iac` **6852/6888 = +36**, `maos-audit` **6643/6665 = +22**. Standing reds, none of them yours: `maos-domain` **8694/8644 = −50** (D14), `maos-kernel-core` **18933/18248 = −685** (D13), `_aggregate_hardfail` **148211/147057 = −1154** (D17 — its register row still records the stale +492; 1b reported the drift and deliberately did not edit the row). **`kloc-check` is RED on THREE keys at HEAD, not four — `maos-bin` is now GREEN.** T1's relocation returns NOTHING (`main.rs` → `lib.rs` inside one crate is kloc-neutral; 2a's +204 refund came from moving a test module into kloc-excluded `tests/` and 2a has already spent it). `crates/*/tests/`, `xtask/tests/`, `xtask/src/tests/` and all of `spirits/` cost ZERO. **Plan TWO named measured grants (`maos-bin`, `xtask`) or route production code into `maos-a2a-tcp`.** One free reduction exists before asking: `CODEX_ORACLE`/`CLAUDE_ORACLE` (`xtask/src/check_j1_loopback_delegation.rs:124-125`) are compiler-confirmed `never used`."
model: frontier-class {opus-4-8, gpt-5.5, glm-5.2, opus-5, equiv}
review: §A6 full-layer net (Blind + Edge + Acceptance + Test-Infra + runtime) — NON-DEGRADABLE (this story is the first production code path that acts on a frame another machine sent)
---

# j1-crosshost-2b — cross-host delegation mechanism

Status: **done** — §A6 review CLOSED 2026-08-17 (reviewer `zai/glm-5.3` ≠ dev `anthropic/claude-opus-5`;
4-layer net Blind + Edge + Acceptance + Test-Infra + runtime, all layers delivered). 34 raw findings →
2 decisions (both ratified by Lunarpulse and implemented: D1 fail-closed remote PDP/SSO denial, D2
bounded intake channel + NACK), 15 patches ALL APPLIED + VERIFIED, 6 defers recorded in
`deferred-work.md`, 5 dismissed with grounds. One genuine regression was found and fixed (the 13_5a
dispatch-window displacement — see Review Findings P1), and the dev's workspace verification row was
corrected: the tallied "507 passed / 0 failed" was a fail-fast partial run. Post-review verification:
`cargo test --workspace --all-targets --no-fail-fast` = **3835 passed, 1 failed**, the 1 failure and
both bench panics all verified pre-existing at `87eb6c37`; `check-j1-loopback-delegation` PASS on
**seven** legs; `check-kernel-baseline` PASS at 24472 = pinned (ZERO Δ across dev pass AND review);
`demo-j1` renders `two-host-delegation` **PROVEN_BLOCKING**; `cargo fmt --all --check` clean; kloc
GREEN on every touched key after one measured review grant (`maos-bin` 16640 → 16738, +98 EXACT
MEASURED, annotation in `kloc.toml`).

> **All four blocking conditions are now closed.** Conditions 1, 2 and 4 cleared when `2a` and `1b`
> landed; **condition 3 (D18) was resolved at the 2026-08-16 preflight round-table, unanimously, per
> spec + long-term correctness**: `2b` does **not** owe D18's repair, and the deadline attached to it
> was pointing at the wrong defect. See **T0**, **H8** and **H13**. The story is unblocked.

> **What this story is.** Rung 1 proved a delegation frame can be *emitted* and routed on a loopback
> pair. `2a` proved one host can tell the truth about whether its worker did the work. `2b` is the
> first time a MAOS Host **acts on a frame another Host sent it**. Two processes, real mTLS, real
> TOFU, a real worker spawned on the far side, and a journal on both ends you can join.
>
> **And the scope you were handed is wrong about where the work is.** The split's item 6 says "build
> a host B that receives `task.assign` over TCP". Measured, and re-verified at HEAD: **the receiver
> already receives it.** A real daemon authenticates the peer, binds the wire identity, runs TOFU,
> checks the boot nonce, evaluates consent, advances the Lamport clock, and **ACKs `delivered: true`**
> — then drops the frame on the floor, because nothing ever installed an intake sink. That is the
> story: not a protocol, not a transport, **one missing `install_intake_sink` call and a consumer
> behind it.**

---

## ⚠ RE-BASELINE LEDGER — what moved under this story after it was written

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

---

## ⚠ The findings, re-verified at `87eb6c37` — the ratified scope is still wrong in fifteen places

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

---

## Blocking conditions — status at `87eb6c37`

1. ~~**`j1-crosshost-2a` reaches `done`.**~~ **SATISFIED** — `0769869d`, §A6 closed 2026-08-16.
2. ~~**`j1-crosshost-1b` reaches `done`** with rung-1 evidence reading `PROVEN_BLOCKING`.~~
   **SATISFIED and verified by execution** — see `depends_on`. The leg AC2.3 verifies now exists.
3. **D18 has a budget decision — STILL OPEN, and it is the only one.** A decision exists in the
   register (RESOLVED 2026-08-15, owner 14-4, deadline re-pinned to *"before `j1-crosshost-2b` writes
   its first line"*), but its stated budget premise does not survive re-measurement and neither did
   this file's counter-estimate (**H8**). Neither 2a nor 1b touched it; `maos-a2a-core` is
   byte-unchanged at 4654/4654 and `maos-domain` at 8694/8644. **Carried as T0** rather than holding
   the story: three of the original four conditions are mechanically clear, and T0 blocks only the
   production lines it actually governs.
4. ~~**Agreement with `2a` on two shared regions.**~~ **SATISFIED — both are landed, by 2a.** The
   standalone `[cli_wrapper]` block is now `main.rs:4421-4462` (33 lines, not the 12 at `:4355-4367`)
   and 2a closed its false-success surface. `completion_tl_ref` → `last_stdout_tl_ref` is **done**:
   declaration `main.rs:1304` with a comment stating it is *"assigned on EVERY stdout row and
   causally unrelated to the oracle's verdict"*, emitted at `:1349`. Note the paired deferral:
   `deferred-work.md` names **this story** as owner of the fact that a sealed capture cites
   `last_stdout_tl_ref` as a completion witness while the oracle verdict itself is `println!`-only
   and never journaled.

---

## Inherited from `j1-crosshost-1b` — two things rung 1 does NOT prove

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

---

## Story

**As** the founder running the J1 developer-remote loop,
**I want** a second MAOS Host that actually executes the task I delegated — receiving the frame over
mTLS, spawning its own worker, journaling what it did, and leaving evidence I can join to my own log —
**so that** "developer-remote" stops being a host id in a config file and becomes a machine that did
the work, with both sides' evidence able to be reconciled by `2c`.

---

## Acceptance Criteria (4)

### AC1 — The receiver stops dropping the frame

*Scope note: this AC is the story. Everything the split described as "build a receiver" is already
built and running (G1); what is missing is a sink, a consumer, and a journal call. Do not
re-implement admission, TOFU, consent or the Lamport clock — they execute today.*

1. **The worker-spawn surface moves under the library, and its callers follow.** Mirror the
   relocation `2a` performed for `worker_cli` (`lib.rs:18-25` doctrine, `2a` AC1.1 precedent): make
   `run_cli_wrapper_manifest` (`main.rs:938-1358`) and the five private items it depends on —
   `RunArgs` (`:439-445`), `parse_sandbox_tier` (`:689`), `resolve_cli_binary` (`:706`),
   `load_host_grant_allowlist` (`:843`), `issue_enterprise_governed_capability` (`:203`) — nameable
   from `crates/maos-bin/tests/`. **Land this first, in its own commit** (T1), and record the
   measured `maos-bin` delta before and after.
   **Justify it correctly (H6):** the old rationale — "a host-B proof has no legal home" — is FALSE;
   `worker_completion_2a.rs` already drives this surface end-to-end by subprocess. The surviving
   rationale is **in-process item visibility**: typed `WorkerCompletion` assertions and port injection
   are impossible from `tests/` today. Say that in the commit message.
   **Do NOT relocate the daemon region** (`if mode == "cohort-a2a-daemon"`, `run_cohort_a2a_daemon`,
   `build_cohort_a2a_daemon_runtime`, `emit_cross_team_share`): two suites assert its literal text
   inside `include_str!("../src/main.rs")` and both red on contact (H6c, Trap 9).
   *This is a move, not a rewrite: if the diff changes behaviour, you have left T1.*
2. **A production TCP transport installs an intake sink; the two lying doc comments are corrected;
   and the sink's send result is not discarded.** Add the install to the daemon's bind path using the
   public seam `TcpA2ATransport::core()` (`transport.rs:388`). **The insertion point is measured and
   the pattern already exists**: `transport.rs:322-325` is
   `let core = Arc::new(core_inner); if let Some(sink) = rupture_sink { core.install_rupture_sink(sink).await; }`
   — immediately before `TcpListener::bind` at `:343` and the `accept_loop` spawn at `:355`. Thread an
   `intake_sink` the same way the chain already threads its other five optional seams (`bind` `:139` →
   `bind_with_cohort_manifest_gate` `:166` → `bind_with_cohort_wiring` `:196` → `…_and_digest` `:229`
   → `…_and_crossing` `:268`, each adding one `Option<Arc<dyn …>>`); a sixth `bind_with_*` sibling is
   the idiomatic shape. In the same change:
   - correct `router.rs:343-344` and `transport.rs:387` — copy the wording `install_rupture_sink`
     already carries at `router.rs:350-351` (*"Live transports install this before exposing their
     listener"*), and fix the inline `// (5) … (test hook).` at `router.rs:1453`;
   - **handle BOTH push sites** — `router.rs:1453-1458` and the digest-reply branch at `:1279-1283`
     (G1b.i);
   - **do not inherit `let _ = sink.send(…)`** (`:1456`, `:1281`). A dropped receiver must not
     produce an ACK of `delivered: true` (G1b.ii). Decide the failure mode explicitly and state it.
3. **A drain consumer dispatches on `FrameKind`, copying the precedent rather than inventing it.**
   `delegation.rs:200-218` already matches `Some((FrameKind::TaskAssign, delivered))` off
   `SpiritMailboxHandle::try_recv` (`mailbox.rs:634`) with a **fail-closed default arm** for both the
   wrong kind and the empty case. Host B's drain must handle `FrameKind::TaskAssign` and must **fail
   closed and loudly** on anything else — a silent `_ => {}` reproduces G1 one layer up. What is
   genuinely new is that this is the first `FrameKind` dispatch fed by a *router intake* rather than a
   local delivery.
4. **Host B journals the inbound frame.** There are zero TL rows for an inbound frame today. Write it
   through `IacBusAdapter::deliver_typed` — the same shape `DelegationLeg::delegate` uses on host A —
   **not** by adding a TL dependency to `maos-a2a-core` (Trap 11) and **not** through the raw-byte
   `enqueue_frame`/`broadcast_frame` path, which mislabels every row `TaskAssign` (G14.iv).
5. **The worker is spawned without parking a reactor thread.** `run_cli_wrapper_manifest` is
   synchronous and blocks for the whole worker lifetime: `spawn_and_bridge` (`main.rs:1236`, the free
   fn imported from `maos_kernel_core::lifecycle::cli_wrapper` at `main.rs:950-953`), then the two
   blocking **methods on the returned bridge** — `bridge.pump_to_journal(…)` (`:1249`) and
   `bridge.wait_and_finalize(…)` (`:1258`) — all inside
   `#[tokio::main(flavor = "multi_thread")]` (`main.rs:2430`). Use `spawn_blocking` or equivalent; a
   direct call parks a worker thread for the duration.
6. **A hermetic two-daemon proof, on the right substrate, with a teardown guard.** Clone
   `crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs` — `boot_hermetic_daemon` (`:436-441`),
   `fixture()` (`:227-250`), `mint_pems` (`:163-176`) — **not** `cross_team_crossing_13_6b.rs:1642`,
   which is `#[ignore]`, needs live Postgres, and carries a `TelemetryEvent` that never touches a
   Mailbox (G5). Two real `maos` processes, real mTLS, a `TaskAssign` delivered, a worker spawned on
   host B, a TL row on host B. **Isolate via per-child `MAOS_AUDIT_DB` on the `Command`, never
   `std::env::set_var`** — that is how 13_5c sidesteps D16 entirely (Trap 6). **Add the `Drop` guard
   13_5c lacks**: copy `struct RunningDaemon(Child)` from `cross_team_crossing_13_6b.rs:1520-1533`, or
   a panic leaks a daemon. Assert the *absence* case too: with the sink uninstalled the frame must
   still be ACKed and the test must red — that is the falsifier for AC1.2. **Note what this earns:
   no multi-process daemon test runs in CI at HEAD (H7); this is the first.**

### AC2 — The composition-root fork, and a boundary leg that can actually see it

1. **`DelegationLeg::install` chooses its router; `main.rs:3219` does not move.** The `OnceLock`
   (`mailbox.rs:131`) is set exactly once by exactly one caller (`delegation.rs:110`), so the fork
   belongs inside `install`, not at the call site (G8). Moving `:3219` past the `MAOS_ONE_SHOT`
   dispatch at `:5344` puts it after the daemon arm returns — a dead end, do not attempt it.
   Host A passes `Arc<TcpA2ATransport>` directly: the `A2ARouter` impl already exists
   (`transport.rs:826-840`), so this is a construction change with **zero new adapter code** (G7).
   **Read AC2.3 before writing this — the two are coupled by H1.**
2. **`TcpA2AConfig` reaches the `maos run` path WITHOUT touching the topology key allowlist.**
   `TOPOLOGY_SPIRIT_KEYS` is `["manifest", "path", "host"]` and rejects unknown keys
   (`topology.rs:57`, `:73-79`); two blocking controls pin `j1-founder-loop.toml` to exactly one
   `developer-remote-host` entry (G6). **Add a new topology file** — `2a` proved this works — and
   route transport config through `MAOS_COHORT_DAEMON_CONFIG` → `CohortDaemonFileConfig`
   (`main.rs:9469-9479`).
   *Three directory-scoped controls constrain the new file, all discovered by reading 2a's landing:*
   - `crates/maos-bin/tests/topology_delegation_1a.rs:197-224` iterates **every** `.toml` in
     `spirits/topologies/` under the strict parser; the file must declare `[topology]` and use only
     allowlisted keys.
   - `crates/maos-bin/tests/worker_manifests_2a.rs:274-355` asserts every named member exists on disk
     and — for any stem starting `j1-founder-loop` — runs the real `validate_remote_topology_target`
     and requires exactly one host-bearing `[cli_wrapper]` target.
   - **The trap:** `worker_manifests_2a.rs:352-358` **exact-list-asserts**
     `["j1-founder-loop", "j1-founder-loop-codex"]`. A third `j1-founder-loop*` file reds it — fix
     that list deliberately, in the same commit. Naming the file outside that prefix exempts it from
     the routability control, which is the **weaker** posture; prefer the prefix and edit the list.
   *Mandatory config fields, measured:* `TcpA2AConfig` (`crates/maos-a2a-tcp/src/config.rs:61`) needs
   `listen_addr` (`:64`), `own_cert_chain` (`:66`), `own_private_key` (`:68`), `peer_pins` (`:70`);
   `A2APeerConfig` (`crates/maos-a2a-core/src/config.rs:39`) needs `peer_id` (`:41`), `endpoint`
   (`:46`, `tls://IP:port` — **no DNS**), `cert_fingerprint` (`:52`). Both are
   `deny_unknown_fields`. `PinnedFingerprint.boot_nonce` has **no serde default** — see AC4.1.
   **`CohortDaemonFileConfig` fields are `tcp / peers / manifest_path / authority_keys / local_host /
   control_spirit / digest_summary` — and it CANNOT carry `own_boot_nonce`**: 13.5c removed it and a
   config carrying it fails to boot (`cohort_daemon_smoke_13_5c.rs:642-662`). The daemon inherits the
   primary root's nonce.
   **(Q3 RESOLVED 2026-08-16, unanimously — strip, then widen; there is no outage.)**
   `spirits/topologies/bilateral-2-host-mira-nash.toml` declares `host` on **class** Spirits
   (mira/nash), which `validate_remote_topology_target` (`crates/maos-bin/src/topology.rs:31-46`)
   refuses — so it parses and cannot load, and 2a recorded it as *"a forward declaration of a two-host
   scene that `j1-crosshost-2b` owns"* (`worker_manifests_2a.rs:265-272`). **Measured, it is neither
   a forward declaration nor this story's feature: it is `1a`'s own `priority_weight` defect, missed
   in `1a`'s own file.** `1a` stripped `priority_weight` out of these topologies because *"the parser
   never read them — the keys claimed a scheduling behavior that never happened"*, and in the same
   pass gave `host` a meaning while leaving it on two class Spirits where the parser rejects it. The
   file's own header says the bilateral join is carried by operator-supplied `A2APeerConfig`, **not**
   by manifest fields (ADR-003), and `j4-mira-nash.toml` is the same pair with no `host` keys and
   loads fine — so those two keys are decorative. Therefore:
   - **Strip the two `host =` lines** (2 lines), on `1a`'s ratified precedent: a key is consumed or
     deleted. Do **not** extend remote routing to class Spirits — that is a different mechanism, it
     touches `topology.rs` in a zero-headroom crate, and it belongs to the Mira/Nash scene.
   - **Then widen the loadability control** past the `j1-founder-loop*` stem filter in
     `worker_manifests_2a.rs:274-355`, so **every** shipped topology is asserted routable-or-hostless.
     It lands **green on day one**, because the only unloadable file just became loadable — and the
     next decorative key reds at its origin commit.
   - `docs/release/v1.5-topology-support.md:21` is untouched: it cites the file as the deployment
     manifest and cites the *proof* elsewhere (the `maos-a2a-tcp` tests and
     `spirits/mira/tests/halt_bilateral.rs`). Removing keys that were never read changes no claim.
   - Filed, not owned: `j4-mira-nash.toml` and `bilateral-2-host-mira-nash.toml` are the same two
     Spirits in two files. Not this story's question.
3. **SHIP-BLOCKER — SPLIT the boundary fact. Do NOT re-key the needle, and do NOT assert the flip.**
   *(Rewritten 2026-08-16 at the round-table, unanimously, per spec + long-term correctness. The
   previously ratified text told the dev to re-key the grep and assert `true → false`; both are
   wrong — see **H1b**.)*
   As shipped, `leg_loopback_from_host_unverified` (`check_j1_loopback_delegation.rs:385-437`) gates
   `verified_composed` on `!loopback_composed` (`:403`), so the dual-mode composition root AC2.1
   mandates leaves the leg publishing `loopback_from_host_unverified: true` over a verified wire with
   the gate GREEN (**H1**). **But the flip is not the fix**: after this story the loopback arm still
   exists and still self-asserts, so `true` is the *honest* value and a `true → false` assertion
   asserts something that must never happen. And no grep can decide a runtime fork. Deliver instead:
   - **(a) Keep the static leg, and correct what it claims.** It reports a permanent property of the
     **loopback rehearsal path** — that arm does not bind wire identity — and it stays `true`. Fix its
     doc block, its `boundary MOVED` finding text, and the AC1.5(a) non-coverage prose so none of
     them promise a flip. Keep **door two** intact (`router.rs`'s
     `letpeer_host=match&frame.from.host_id{`, production half only): if the shared intake body ever
     binds identity itself, that *is* a real static change and the leg should still see it (H5).
   - **(b) Publish the cross-host fact, and DERIVE it from execution.** *Does the J1 cross-host path
     bind wire identity?* — answered by AC1.6's two-daemon proof, not by source text. Route it the way
     `1b` routed `disallowed-intent-refused-blocking`: a fact the gate publishes only because a named
     test ran, so the linter and the judge stay connected by the CI enrollment line and nothing else.
   - **(c) A proven-red vector for the FORK shape.** Plant both `paired_loopback_router(` and the TCP
     branch and assert the leg does **not** silently claim a moved boundary — 1b's existing vector
     (`j1_crosshost_1b_proven_red.rs:821-837`) only plants total replacement and must stay green on
     its own terms.
   - **never delete the leg** (the gate's own instruction).
   *Six sites move together if the leg is renamed or a seventh is added:* `ledger_leg_names()`
   (`:129-140`), `judge()`'s `let [leg1..leg6] = &mut audits[..] else { panic!(…) }` (`:890` — a
   **panic**, not a graceful red), `j1_crosshost_1b_proven_red.rs:736-745`'s hard-coded six names,
   `leg_narration()` (`demo_j1.rs:812`, which silently falls through to a catch-all on a rename), the
   `--skip-gate` ABSENT loop (`demo_j1.rs:235-255`), and the published JSON booleans. **Any new leg
   must call `audit.entered()` then `audit.checked()` per condition**, or `vacuous_legs()`
   (`gate_common.rs:172-178`) makes it a Finding.
4. **Leg 1 must stay green on its own terms, and the change must be visible to it.**
   `leg_frame_borne_route_intact` (`check_j1_loopback_delegation.rs:222-344`) now governs **ten**
   files plus one directory (`:79-96`, `:109-115`), several of which this story edits. Its six checks
   include: exactly one `host` line containing `developer-remote-host` in `j1-founder-loop.toml`
   (`:232-243` — note `starts_with("host")` is a prefix match, so a `hostname` key also counts);
   `main.rs` uses `assign_frame_remote` and does **not** read `MAOS_WORKER_TASK`; `delegation.rs`
   keeps `host_id = None`; `mailbox.rs` keeps its absent-router fail-closed skeleton; the orchestrator
   defines `development-task:write-workspace` and never builds a consent `A2AIntent::new("task.assign")`
   in production. Read it before you write; if a needle must move, move it deliberately and add the
   planted-red vector — **never delete a leg to make a change fit.**

### AC3 — Prove the crossing with the key that already exists, and make the word mean something

*Scope note: **this AC lost three items to measurement at the 2026-08-16 round-table and one more to
this re-baseline.** The original AC3.1/3.2/3.3 asked for a correlation token on `IacFrame`, a TL
writer to carry it, and a join — all unnecessary (G16). AC3.4 has now been re-scoped because `2a`
closed the surface it was aiming at, from the other direction (H3). They are recorded rather than
deleted, because the reasoning is what stops a future author from re-adding them.*

1. **Prove the crossing with `frame_id`. Do NOT add a correlation field to `IacFrame`.**
   Measured end-to-end at HEAD (G16): host A mints a **deterministic** id
   (`spirits/orchestrator/src/lib.rs:358`, `delegation.rs:240-242`); it travels on the frame and
   survives intake unchanged; **`deliver_typed` writes `Some(frame.frame_id)`** — the *received* id
   (`crates/maos-iac/src/adapter.rs:561-562`; the DRR branch too, `drr_scheduler.rs:288-289`); and
   **`maos_audit::query` selects `frame_id` as its FIRST column**
   (`crates/maos-audit/src/lib.rs:193-196`), so it is already `AuditEntry.frame_id_hex` in every
   bundle — which is what `2c` reconciles on. **Consequence:** the two-host join needs **zero** new
   fields, **zero** `maos-domain` lines (RED −50), **zero** `maos-audit` lines. Write the test that
   joins the two logs on `frame_id`; that is the whole deliverable. `correlation_id` is **not** the
   join key — do not spend budget wiring it, and note `maos_audit::query` drops it from the
   projection anyway.
2. **SHIP-BLOCKER — the same fact is a remote-triggerable kernel halt, and AC1.2 is what makes it
   reachable.** `frame_id` is `BLOB NOT NULL PRIMARY KEY` (`transparency_log.rs:259`), the value is
   **peer-supplied**, it is deterministic `(seq ‖ run_nonce)` with no ULID entropy, the INSERT at
   `:797-802` is plain with no `ON CONFLICT` and no dedup anywhere in `deliver_typed`, and a failed
   write is `panic!` (`:819-825`). **A peer that re-sends one frame halts host B.** Fix it in the
   same change as AC1.2, not later:
   - Match **only** the duplicate-primary-key arm, and **discriminate on the EXTENDED code, not
     `ErrorCode::ConstraintViolation`.** Measured against the pinned dependency
     (`rusqlite 0.31.0` / `libsqlite3-sys 0.28.0`, `crates/maos-iac/Cargo.toml:26`):
     `rusqlite::ErrorCode` is a re-export of the ffi enum (`rusqlite/src/lib.rs:80`), and
     `ErrorCode::ConstraintViolation` is mapped from the **primary** code `SQLITE_CONSTRAINT`
     (`libsqlite3-sys/src/error.rs:85`) — which covers `NOT NULL`, `CHECK` and `FOREIGN KEY`
     violations too. **`transparency_log` declares `NOT NULL` on ten of its twelve columns**
     (`transparency_log.rs:258-270`; only `capability_token` and `correlation_id` are nullable), so
     matching `ConstraintViolation` alone would silently convert a
     genuine NOT-NULL defect into a `Duplicate` — the exact inversion of this AC's intent. Match
     `rusqlite::Error::SqliteFailure(e, _)` with
     `e.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_PRIMARYKEY` (and, if you also guard the
     unique index, `SQLITE_CONSTRAINT_UNIQUE`), and let every other extended code fall through to the
     panic. **There is NO existing arm to copy (H10):** `ConstraintViolation` appears zero times
     repo-wide and `SqliteFailure` appears once, as a construction. The vocabulary to mirror is
     `DigestReplyObservation::{Accepted, Duplicate, Unauthorized}`
     (`crates/maos-a2a-core/src/cohort.rs:285-296`) — its *shape*, not its in-memory implementation.
     **Add the negative for this too:** a planted `NOT NULL` violation must still panic, or the
     discrimination is untested and the next author widens the arm.
   - **Every other write error must keep panicking.** That `panic!` is the I2 log-before-deliver
     guarantee; converting it wholesale trades a denial-of-service for silent audit loss, which is
     worse. State this constraint in the code comment, or the next reader deletes the panic.
   - Negative test: deliver the same frame twice to host B; assert a typed `Duplicate` and a **live
     process**. The falsifier is that the test reds at HEAD-plus-AC1.2 without the fix.
   - Budget note: this lands in `maos-iac` (**+36**). Measure before writing.
3. **A test joins the two logs on `frame_id` — on the BYTES, never on `kind`.** Host A's emit row and
   host B's intake row, same 16 bytes, two distinct data homes (Trap 7). **`kind = TaskComplete` is a
   contaminated oracle (H9)**: `insert_kernel_event_returning_id` (`transparency_log.rs:1302`) stamps
   it on *every* kernel event across nine production callers, and `insert_distillate_redaction_marker`
   (`:873`) stamps it under intent `"distillate.redacted"`. `reconcile_correlated_frames`
   (`:1888-1916`) exists for the correlated case — for `frame_id` a direct two-store query is simpler
   and is what `2c` will reconcile on. **Do not build a second join helper.**
4. **RE-SCOPED (H3) — make the outcome vocabulary reachable where it can be true, and journal the
   verdict.** The old wording ("`journal_completion` stops hardcoding the word") is a **null control
   at HEAD**: 2a's `if !completion.is_completed() { return Err(…) }` (`main.rs:4035-4041`) sits
   *above* `journal_completion` (`:4044`), so `"completed"` is the only reachable value and
   `delegation_leg_1a.rs:234` would not go red on its assertion. Deliver instead:
   - **(a) Host B journals its worker's REAL outcome.** On host B a remote worker's failure must
     become evidence, not an aborted process — so host B's own journal row carries
     `WorkerCompletion::label()` from the **six**-value set (`worker_cli.rs:87-106`), not a literal.
     This is where the vocabulary is genuinely reachable.
   - **(b) Journal the verdict, which is `println!`-only today.** `deferred-work.md` names **this
     story** as owner: the sealed capture cites `last_stdout_tl_ref` (documented in-code as *not* a
     completion witness) while the oracle verdict itself is never journaled
     (`main.rs:1324-1337`, `xtask/src/demo_j1.rs:1241`).
   - **(c) `success_criteria` is circular and dead** (`TaskAssignPayload`, `frame.rs:93-106`), handed
     to this story **by name** by 2a's F15. Either give it meaning in the host-B path or re-file it
     with a named owner — silence is not a disposition.
   - **(d) Host A's hard-abort is NOT to be softened here.** Journaling a failed `TaskComplete` on
     host A would close an in-flight delegation FR20 says must not close, and would trip FR21's 60s
     window (Trap 7) for the next run. That semantic belongs to Story 6.2's owners
     (`delegation.rs:150-163` says so). **State the boundary; do not cross it.**
   - If any signature change breaks `delegation_leg_1a.rs:210`/`:234`, update it deliberately in the
     same commit and say so — it will be a **compile** break, not an assertion break.
5. **Do NOT build a general dedup store.** The sketched at-least-once hazard does not exist: retries
   are admitted only for pre-send handshake classes and no second retrier exists (G3), so the
   transport is at-most-once. The real replay hazard is a *peer* re-sending, and it is handled by
   AC3.2 at the storage layer, where the uniqueness constraint already lives. A second state store —
   in particular the `DigestReadPort`-style four `Mutex<HashMap>`s capped at 256 and wiped on restart
   (`crates/maos-cohort/src/state.rs:111-117`) — would be a weaker guard in front of a stronger one.
6. **The `TaskComplete` return hop is DEFERRED to `2c`, and this story must not claim a round trip.**
   G9 enumerates five required parts, and `consent_envelope: None` (`delegation.rs:285`) means a
   partial attempt does not degrade — it **fails closed** at the sender with
   `ConsentUnclassified{Send, Absent}` (`router.rs:696-700`). A half-built return hop is a refusal,
   not a weaker return hop, so there is no useful middle. This story proves: host B journals its
   outcome locally, and host A and host B's logs are joined on `frame_id` (AC3.1, AC3.3).
   **Language constraint, binding on every artifact this story produces:** it may claim *"the
   crossing is proven in two logs"*. It may **not** claim *"round trip"*, *"the task completed back on
   host A"*, or anything implying a frame returned — nor *"signed"*, which is `2c`'s. The return hop
   is a **second delegation in the opposite direction** — second intent, second allowlist pair, second
   consent envelope — and naming it as a partial feature is how the next reader inherits a claim
   nobody built.

### AC4 — Bound the claim honestly, and do not hand `2c` a surprise

1. **State the production gaps this mechanism has, as machine-asserted boundaries, not prose.**
   Three, all re-measured at HEAD — **the middle one changed** (H4):
   - **Boot-nonce provisioning (G4, reframed).** MAOS has **no automated peer-nonce provisioning
     channel**: host A generates a fresh 63-bit nonce per process (`main.rs:2651-2664`), host B pins
     it statically with no serde default (`config.rs:25-36`), and a mismatch NACKs
     `CODE_SPIRIT_RESTART_DETECTED` and invalidates the pin with no live recovery
     (`await_repin_consent` has zero production callers). A same-machine release-build pairing IS
     possible by reading A's nonce from its own TL `cohort:daemon-started` row; a cross-machine one
     requires an operator to transcribe a value that changes every restart. **Assert the boundary
     with a negative test, and explicitly REJECT the `boot_nonce = 0` sentinel escape** — the loopback
     leg already uses it (`crates/maos-a2a/src/adapter.rs:75-77`), and reaching for it here would ship
     the first cross-host path with NFR-Rel-6 restart detection off. **`2c` runs a paid agent against
     this.**
     **MANDATORY REPAIR, ratified 2026-08-16 (H13) — this is the one arm this story owes.** The NACK
     above does not survive the trip home: `interpret_response` (`router.rs:892-1052`) types nine of
     the sixteen defined codes and `CODE_SPIRIT_RESTART_DETECTED` is not among them, so it falls
     through the catch-all at `:1049` into `A2AError::TransportFailed` →
     `IacBusError::CrossHostTransportFailure`. **A permanent pin invalidation is reported to the
     operator as a transient network fault, and the action it implies — retry — is the one action
     that can never work.** Add the typed arm:
     `CODE_SPIRIT_RESTART_DETECTED => Err(A2AError::PinInvalidated { peer: peer.as_str().to_string(), awaiting_repin: true })`
     — ≈4 lines, **zero new types**, reusing the variant at `crates/maos-a2a-core/src/error.rs:68`
     that already maps typed to `CrossHostPinMismatch` (`router.rs:1697-1701`), and semantically
     exact because restart detection *is* pin invalidation and re-pin *is* the designed recovery.
     It lands in `maos-a2a-core` at ZERO headroom as a **correctness repair on a security path** —
     **cite `kloc.toml:87` by name in the annotation.** Negative test: NACK `-32004` from host B and
     assert the sender surfaces a pin failure, never a transport failure.
     **SCOPE WALL, binding:** repair **only** the code this story makes reachable. The other six
     fall-throughs (`PARSE_ERROR`, `INVALID_REQUEST`, `METHOD_NOT_FOUND`, `TIMEOUT`,
     `FRAME_TOO_LARGE`, `INTERNAL`) are not newly reachable here — **record the 9-of-16 census with a
     named owner and stop.** A nine-arm refactor inside a frozen crate is a different story.
   - **Consent expiry is a TRANSITIONAL POLICY, not a missing expiry (H4).** The old "non-expiring
     bearer grant" claim is **disproved**: `prepare_outbound` stamps
     `valid_until_ns = now + consent_ttl_secs` on any `None`-carrying envelope
     (`router.rs:866-871`), and the expiry check then fires (`:1207-1222`). State the real posture —
     **Decision §D1 TRANSITIONAL: the transport supplies a TTL when the granter does not, and an
     explicit granter `valid_until_ns` is authoritative and left untouched** — and name whoever owns
     moving from transport-stamped to granter-authoritative. **Do not build a negative control for a
     non-gap.**
   - **No DNS.** Peers are bare `IP:port` (`transport.rs:434`, `:468`), while `A2APeerConfig::validate`
     accepts a hostname whose own doc example is `tls://host-b.internal:7443`
     (`crates/maos-a2a-core/src/config.rs:43-45`, `:99-136`) — it passes bind and fails at first dial.
     Say what this story proved and what it did not.
   Use the precedent `2a` established: a stated posture a capture cannot overclaim, with a negative
   test refusing the overclaim direction (`CaptureDoc::validate`,
   `crates/maos-cli/src/subcommands.rs`, and its negative vector).
   **Consider `RELEASE-HOLDS.md` §Claim boundaries as the home.** It has **no J1 or cross-host entry
   at all** — 2a's *"claude is signable, never claude works"* and 1b's two non-coverages live only in
   story files, the tracker, and the demo's printed non-claims block, none of which is a control.
   D19 option (b) names that section as the sanctioned home and it is unused.
2. **Budget, measured at HEAD, attributed by key.** Re-measure in a clean tree
   (`git archive <commit> | tar -x -C <tmp>`) — two scouts on the original preflight drew a false
   conclusion from a tree `2a` was mutating. **Plan TWO grants**: `maos-bin` (16260/16260) and
   **`xtask` (38655/38655 — this is new, H2)**. Take the free `xtask` reduction first (delete
   `CODEX_ORACLE`/`CLAUDE_ORACLE`, `check_j1_loopback_delegation.rs:124-125`), route production code
   into `maos-a2a-tcp` (+415) where it fits, and put every vector in `crates/*/tests/`,
   `xtask/tests/` or `xtask/src/tests/` (all kloc-free). Take **no** grant unless still over, and then
   only with the measurement attached (`kloc.toml:60-65`). Do **not** absorb the standing reds —
   `maos-kernel-core` −685 (D13), `maos-domain` −50 (D14), `_aggregate` −1154 (D17, whose register row
   still says −492). `kloc-check` will exit 1 at close through no fault of this story. **Also
   attribute, do not absorb, the pre-existing `check-env-contract` red** (H12) — and if AC2.2's
   `MAOS_COHORT_DAEMON_CONFIG` work touches `env_contract.rs`, say explicitly whether you repaired it.
3. **CI enrollment on a job that already blocks, with no `services:` block — and the derivation must
   be able to see your file.** Extend `check-j1-loopback-delegation` (`discipline.yml:1804-1882` —
   **eight steps now, not three**): it is `BindingClass::Blocking`, registered at
   `xtask/gate-registry.toml:100` and `:280-281` (`v1_0 = "blocking"`, `v1_5 = "blocking"`), and a
   `needs:` of `v1-0-ship-gate` (`discipline.yml:3234`). Required, in order:
   - **Add `"_2b.rs"` to `J1_TEST_SUFFIXES` (`check_j1_loopback_delegation.rs:110`) BEFORE writing
     `crates/maos-bin/tests/*_2b.rs`** — otherwise `derive_enrolled_targets` (`:591-619`) never sees
     it and the file is a suggestion, not a control (H12).
   - **Lay the new target in ALL THREE `lay_green` fixture trees** (`j1_crosshost_1a_proven_red.rs:149`,
     `…_2a_proven_red.rs:155`, `…_1b_proven_red.rs:164`) or the untouched trees'
     `baseline_fixture_tree_is_green` reds and every vector in them passes vacuously. 1b paid this
     exact ship-blocker once (F3b) and its review paid it again.
   - **Hand-add the `cargo test -p xtask --test j1_crosshost_2b_proven_red` step** — nothing derives
     `xtask/tests/` enrollment, and **no CI job runs `-p maos-bin` or `-p xtask` unscoped** (verified
     across all eleven workflow files).
   - **Also enroll `cohort_daemon_smoke_13_5c.rs` if you rely on it as a control** — it has zero CI
     invocation today (H7), so its "Blocking" header is a comment.
   - **Do not add a `services:` block to any job**: `check_loom_substrate_drift`'s leg 2 rejects an
     unregistered service-bearing gate job (`xtask/src/check_loom_substrate_drift.rs:551-571`,
     `:702-706`), is blocking unconditionally, and is itself in the ship-gate needs
     (`discipline.yml:3253`). The shape to copy is `check-live-bilateral-consent`
     (`discipline.yml:2494-2506`) — two real `127.0.0.1:0` mTLS endpoints, zero external services.
   - **Do not put tests in `crates/maos-a2a-tcp/tests/`** unless they are fast: `a2a-tcp-tests-8-6`
     (`discipline.yml:1522`, `timeout-minutes: 10` at `:1524`) runs `cargo test -p maos-a2a-tcp`
     **unscoped** 50× in a loop at `:1537-1543`.
4. **Close the record honestly, and fix the demo beat this story cannot deliver.**
   - **The `two-host-signed-run` beat names this story as owner and a CI-enrolled test pins it.**
     `demo_j1.rs:881-885` declares it *"two real hosts over mTLS/TOFU, heterogeneous worker, **one
     reconciled signed bundle**"* with owner `"j1-crosshost-2b"`, and
     `xtask/src/tests/demo_j1_tests.rs:48-55` (`the_two_host_rung_is_owned_by_crosshost_2b`) asserts
     it, run by `cargo test -p xtask demo_j1` at `discipline.yml:1830`. **This story delivers no
     signing, no reconciled bundle and no round trip** (AC3.6).
     **RATIFIED 2026-08-16 (Q4, unanimous): SPLIT the beat.** Add `two-host-delegation` — *two real
     hosts over mTLS/TOFU, a frame crossed, a worker ran on the far side, both logs carry the same
     sixteen bytes* — owned by and flipped by **this** story; re-own `two-host-signed-run` to
     **`j1-crosshost-2c`** and leave it ABSENT. Re-owning wholesale was rejected: it leaves a
     mechanism story with nothing to show, so the narrated artifact renders ABSENT against a story
     that already shipped — and this lane's whole discipline is that the demo tells the truth about
     its own work. **Cost ≈5 lines in `xtask/src` at ZERO headroom, funded by the free reduction in
     AC4.2** (delete `CODEX_ORACLE`/`CLAUDE_ORACLE`, `check_j1_loopback_delegation.rs:124-125`) —
     take the reduction in the same commit so the ceiling never enters the argument. Then move
     `demo_j1.rs:306-311`'s printed non-claim ("Rung 2b binds it to a TLS-verified identity") and
     `demo_j1.rs:28`'s module doc (still says `j1-crosshost-2`) with it, rename the test fn, and
     **say so in the record — `2c`'s preflight still expects the beat at `:797-801` naming
     `"j1-crosshost-2"`, which is already stale twice over.** Note `demo_j1.rs` now emits ABSENT beats
     on the `--skip-gate` path too (`:235-255`), one per `ledger_leg_names()` plus the conjunction; a
     beat added outside `run_delegation_gate()` does not get that for free. Current ledger state:
     **17 executed `PROVEN_BLOCKING` + 3 ABSENT.**
   - **Update `tests/coverage-matrix.yaml` FR23a**, which names two inheritances to this story
     (loopback peer auth; D18 conflation) and must record which you discharged. **Do not cite it as
     evidence** — the row itself discloses that the file cannot fail (`mode: warning`).
   - **Disclose D19** (`epic-14-preflight-decisions.md`): all **seven** digit-prefix story-file
     walkers verified intact at HEAD, so no gate reads this filename; its deadline — *before the next
     `j1-*` story leaves `ready-for-dev`* — was already passed by `2a`. Populate the model/§A6 fields
     anyway. Report "no ABI change was made" as a fact about your diff, never as an `abi-diff` result
     (FLAG-E4).

---

## Traps

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

---

## Tasks

- [x] **T0 (blocking condition 3) — CLOSED at the 2026-08-16 round-table, unanimous, per spec +
      long-term correctness. The dev inherits a record to write, not a decision to make.**
      **`2b` does NOT owe D18's repair.** Measured: `map_a2a_error_to_iac_bus` has exactly two call
      sites and both are outbound (`crates/maos-a2a/src/adapter.rs:113`, `transport.rs:834`); host B
      never calls it; D18's shape is byte-identical before and after this story. The deadline
      *"before `j1-crosshost-2b` writes its first line"* was pointing at the right story and the
      **wrong defect** — the thing that genuinely becomes reachable here is **H13**, and AC4.1 owns
      its ≈4-line repair. What `2b` owes instead:
      (a) **Do not extend the flattened error.** No new operator-visible deny may be built on
      `CrossHostRouteFailure(String)` — which `crates/maos-domain/src/iac_bus_types.rs:69-72` already
      marks ***DEPRECATED, "use the typed sub-variants above instead."***
      (b) **Correct the register's mechanism, not its conclusion.** D18's premise (a) is **upheld**:
      `router.rs:1688` hardcodes `CrossHostIntentDirection::Accept`, so the `-32001` pair *is*
      distinguishable. Only `router.rs:1676` — the mapping *inside* the `IntentDenied` arm — is dead,
      because `IntentDirection::Accept` has zero construction sites anywhere. **A dead line inside a
      live discriminator is not a dead discriminator**; this story's own earlier text said otherwise
      and is corrected in H8.
      (c) **Leave D18 with 14-4 and the `maos-domain` line with D14/14-7**, per the register.
- [x] **T1 (AC1.1)** — Relocate `run_cli_wrapper_manifest` + its five private dependencies under the
      library, mirroring `2a`'s `worker_cli` precedent. **Own commit.** Record `kloc-check` `maos-bin`
      before and after, measured in a clean tree. **Expect ~zero delta and do not plan around a
      refund** — `main.rs` → `lib.rs` inside one crate is kloc-neutral. Justify by *in-process item
      visibility*, not by "no legal home" (H6). **Do not move the daemon region** (Trap 9).
- [x] **T2 (AC1.2, AC1.3)** — Install the intake sink at `transport.rs:322-325` (beside
      `install_rupture_sink`, before `TcpListener::bind`); thread it through the bind chain; correct
      the three false comments; handle **both** push sites; replace `let _ = sink.send(…)` with a
      decided failure mode; add the `FrameKind` drain copying `delegation.rs:200-218`'s fail-closed
      shape.
- [x] **T3 (AC1.4, AC1.5)** — Journal the inbound frame via `deliver_typed`; spawn the worker off the
      reactor.
- [x] **T4 (AC1.6)** — Two-daemon hermetic proof cloned from `cohort_daemon_smoke_13_5c.rs`,
      per-child `MAOS_AUDIT_DB`, a `RunningDaemon(Child)` Drop guard, including the sink-uninstalled
      falsifier.
- [x] **T5 (AC2.1, AC2.2)** — Router fork inside `DelegationLeg::install`; new topology file (respect
      the three directory-scoped controls and edit `worker_manifests_2a.rs:352-358`'s exact list);
      transport config through `MAOS_COHORT_DAEMON_CONFIG` (no `own_boot_nonce`). Do not widen
      `TOPOLOGY_SPIRIT_KEYS`. **Plus Q3, ratified:** strip the two decorative `host =` lines from
      `spirits/topologies/bilateral-2-host-mira-nash.toml` (1a's `priority_weight` precedent — a key
      is consumed or deleted), **then** widen `worker_manifests_2a.rs:274-355` past the
      `j1-founder-loop*` stem so every shipped topology is asserted routable-or-hostless. Order
      matters: strip first and the widened control lands **green**. Do **not** extend remote routing
      to class Spirits.
- [x] **T6 (AC2.3, AC2.4)** — **SHIP-BLOCKER, land with T5. Re-scoped 2026-08-16 — do NOT re-key the
      needle and do NOT assert a flip** (H1b). Split the fact: (a) keep the static leg and correct
      what it *claims* — `loopback_from_host_unverified` stays permanently, correctly `true`, so fix
      its doc block, its `boundary MOVED` finding text and the AC1.5(a) prose to stop promising a
      transition; (b) publish the cross-host fact **derived from AC1.6's executed two-daemon proof**,
      the way 1b routed `disallowed-intent-refused-blocking`; (c) add the fork-shape proven-red vector
      and keep 1b's replacement-shape vector green on its own terms. Keep door two. Update the six
      coupled sites if the name or count changes; give any new leg its `LegAudit` calls. Re-read leg
      1's ten governed files and move needles deliberately.
- [x] **T7 (AC3.1, AC3.3)** — The two-host join test on `frame_id` **bytes** (never on `kind` — H9),
      two distinct data homes. **No new frame field, no new TL writer, no `correlation_id` wiring.**
- [x] **T8 (AC3.2)** — **SHIP-BLOCKER, same commit as T2.** Convert ONLY the duplicate-primary-key
      arm of the TL write into a typed `Duplicate`, discriminating on the **extended** code
      (`SQLITE_CONSTRAINT_PRIMARYKEY`), **not** on `ErrorCode::ConstraintViolation` — which also
      covers the ten `NOT NULL` columns in the same table. Every other write error keeps panicking
      (I2). **Write it from scratch — there is no `ConstraintViolation` arm anywhere to copy (H10);
      pinned deps are `rusqlite 0.31.0` / `libsqlite3-sys 0.28.0`.** Two negatives: (a) deliver the
      same frame twice to host B, assert `Duplicate` **and a live process**; (b) plant a `NOT NULL`
      violation and assert it still panics.
- [x] **T9 (AC3.4, AC3.6)** — Host B journals the real six-value `WorkerCompletion::label()`; journal
      the verdict that is `println!`-only today; dispose of `success_criteria`; state the host-A
      hard-abort boundary rather than crossing it; record the return-hop deferral and the
      no-round-trip / no-signing language constraint.
- [x] **T10 (AC4.1)** — The three bounded postures — boot-nonce provisioning (with the `boot_nonce = 0`
      sentinel **explicitly rejected**), the §D1 transitional consent-TTL policy (**not** a
      non-expiring grant — H4), and no-DNS — as negative tests in the overclaim-refusing shape.
      Consider `RELEASE-HOLDS.md` §Claim boundaries as the home; it has no J1 entry today.
- [x] **T10b (AC4.1, H13)** — **The one arm this story owes.** Add
      `CODE_SPIRIT_RESTART_DETECTED => Err(A2AError::PinInvalidated { peer, awaiting_repin: true })`
      to `interpret_response` (`crates/maos-a2a-core/src/router.rs:892-1052`), ≈4 lines, **zero new
      types** — today it falls through the catch-all at `:1049` and a permanent pin invalidation is
      reported as a transient transport fault. Cite **`kloc.toml:87`** by name (correctness repair on
      a security path, zero-headroom crate). Negative: NACK `-32004` from host B, assert the sender
      surfaces a pin failure and never a transport failure. **Record the 9-of-16 census with a named
      owner and repair nothing else** — the other six fall-throughs are not newly reachable here.
- [x] **T11 (AC4.3)** — CI enrollment: add `"_2b.rs"` to `J1_TEST_SUFFIXES` **first**; lay the target
      in all **three** `lay_green` trees; hand-add the `xtask/tests/` proven-red step; consider
      enrolling `cohort_daemon_smoke_13_5c.rs`; no `services:` block; copy
      `check-live-bilateral-consent`'s shape.
- [x] **T12 (AC4.2, AC4.4)** — Re-measure and attribute budget in a clean tree (**two grants
      expected: `maos-bin` and `xtask`**; take the free `CODEX_ORACLE`/`CLAUDE_ORACLE` reduction
      first); attribute the pre-existing `check-env-contract` red; **SPLIT the demo beat as ratified**
      — add `two-host-delegation` owned and flipped by this story, re-own `two-host-signed-run` to
      `j1-crosshost-2c` and leave it ABSENT — moving `demo_j1.rs:306-311`, `demo_j1.rs:28` and the
      pinning test `demo_j1_tests.rs:48-55` (including its fn name) with it, and **say in the record
      that `2c`'s preflight is invalidated at `demo_j1.rs:797-801`**; update `coverage-matrix.yaml`
      FR23a; Dev Agent Record; disclose D19.

### Review Findings

_§A6 review executed 2026-08-17 (reviewer `zai/glm-5.3` ≠ dev `anthropic/claude-opus-5`; 4 parallel
layers + runtime execution). 34 raw findings → 2 decision, 15 patch, 6 defer, 5 dismissed._

**Runtime evidence (executed by the reviewer, not asserted):** `two_host_delegation_2b` 3/3 serial
(13.79s) · `bounded_postures_2b` 6/6 · `host_grants_2b` 4/4 · `j1_crosshost_2b_proven_red` 6/6 ·
`j1_crosshost_{1a,2a,1b}_proven_red` 53/53 · `demo_j1` bins 26/26 · `check-j1-loopback-delegation`
PASS (7 legs) · `check-kernel-baseline` PASS 24472=pinned · `check-enterprise-identity` PASS 10 legs ·
`check-composition-root-completeness` PASS · `cargo fmt --all --check` clean · `kloc-check` granted
keys GREEN (maos-bin 16640/16640, xtask 38742/38742, maos-a2a-core 4669/4669, maos-a2a-tcp
1121/1500, maos-iac 6888/6888; standing reds D13/D14/D17 pre-existing) · `demo-j1`:
`two-host-delegation` PROVEN_BLOCKING, `two-host-signed-run` ABSENT owner `j1-crosshost-2c`.
**`cargo test --workspace --all-targets` is NOT green as recorded**: the dev-tallied "507 passed /
0 failed" is a fail-fast partial run (it aborts at the pre-existing `maos-bench` bench panic before
`--bin maos` ever runs); the reviewer's `--no-fail-fast` run shows **3828 passed / 2 failed / 4
failing targets** — see P1/P2 below and the three pre-existing failures (maos-bench bench,
maos-kernel-core `hot_swap_latency` bench, `cross_team_consent_13_3` intra-file parallel flake —
the last verified failing byte-identically at `87eb6c37` in a clean worktree while passing in
isolation).

**Decision-needed**

- [x] [Review][Decision] Remote-requested PDP/SSO Deny still spawns under host-grant authority —
      `crates/maos-bin/src/worker_spawn.rs:595-617`: `issue_enterprise_governed_capability` failure
      — including `PolicyVerdict::Deny` and missing `MAOS_SSO_ASSERTION` — falls into the catch-all
      that eprints and proceeds with the spawn "under AC5 host-grant authority" (2a's ratified FORK
      B for the LOCAL `maos run` path). 2b threads the enterprise pair into `HostBWorkerContext`
      precisely so a REMOTE-requested mint "runs the same SSO/PDP governance a local maos run
      applies" — but on that path a Deny is still advisory: host B spawns the remote-requested
      worker anyway, making the receiving Host the weaker endpoint exactly at the trust boundary the
      threading exists to guard. Pre-existing semantics, newly remote-reachable. Choose: block on
      Deny for the host-B path (fail-closed journal-and-refuse) vs keep the ratified 2a posture and
      record the boundary.
- [x] [Review][Decision] Unbounded intake channel + one-worker-at-a-time drain = authenticated-peer
      memory exhaustion — `crates/maos-bin/src/main.rs` (`unbounded_channel` for host B's sink) +
      `crates/maos-bin/src/delegation.rs:650-685` (`serve_host_b_intake` awaits each worker's full
      lifetime serially). An authenticated + consented peer can enqueue unbounded frames while one
      slow worker runs; no capacity, admission budget, timeout, or backpressure reaches the TCP NACK
      path. Choose: bounded channel + `try_send`-on-full → NACK (a real protocol change: the peer
      sees `CODE_INTERNAL`, which the sender-side `interpret_response` catch-all currently renders as
      `TransportFailed`) vs documented posture + named owner.

**Patch**

- [x] [Review][Patch] 13_5a unit-test regression: enterprise tokens pushed past the 2,000-char
      dispatch window [crates/maos-bin/src/main.rs:13429-13437, dispatch at :8261+] — the 2b
      insertions (init_monotonic_base comment block + threaded args) moved
      `enterprise_posture_required,`/`enterprise_daemon_governance,` to offsets 2333/2378 from the
      `if mode == "cohort-a2a-daemon"` marker. PASSES at baseline `87eb6c37`, FAILS in this tree
      (`cargo test -p maos-bin --bin maos story_13_5a_enterprise_governance_reaches`). The twin
      `tests/enterprise_daemon_seam_13_5a.rs` was updated; the unit-test twin inside `main.rs` was
      missed. HIGH.
- [x] [Review][Patch] Workspace verification row is a fail-fast partial tally
      [j1-crosshost-2b…md, Verification table row 1] — "507 passed / 0 failed" was tallied from a
      run that aborts at the pre-existing maos-bench bench panic before `--bin maos` executes; true
      `--no-fail-fast` state is 3828/2 with 4 failing targets (P1 + three pre-existing). Correct the
      row, attribute the three pre-existing failures, re-run after P1. HIGH.
- [x] [Review][Patch] `outcome_frame_id` bit-63 flip is not a namespace; collision silently drops
      evidence [crates/maos-bin/src/delegation.rs:493-510,532-541] — frame ids are peer-supplied
      primary keys, so a peer can send a second frame whose id equals a first frame's derived
      outcome id; `journal_inbound_outcome` ignores the `FrameRowWrite` result and returns the id as
      if persisted. Reject inbound ids with the high bit set (reserve the outcome half) AND error
      loudly when the outcome write returns `Duplicate`. HIGH (blind+edge).
- [x] [Review][Patch] Mandated NOT-NULL-still-panics negative control absent (AC3.2)
      [crates/maos-iac/src/adapter/transparency_log.rs:887-892] — the extended-code discrimination
      has no planted `NOT NULL` violation proving non-primary-key constraint failures still halt;
      that is the AC's exact inversion scenario (broad `ErrorCode::ConstraintViolation` matching
      would silently convert audit loss into `Duplicate` and keep the replay test green). HIGH
      (auditor+test-infra).
- [x] [Review][Patch] `frame_lineage_cache` poisoned by replay before the duplicate verdict
      [crates/maos-iac/src/adapter.rs:553-558] — the cache insert runs BEFORE the write returns
      `Duplicate`; at baseline a duplicate panicked so the poison was unreachable, but now the
      process survives and `retract()` consumes the attacker-controlled replay lineage instead of
      the persisted original. Move the insert into the `Written` branch. HIGH (blind+edge).
- [x] [Review][Patch] `CohortRuptureLogSink` (explicit-id production caller) now silently
      suppresses on duplicate [crates/maos-cohort/src/digest.rs:320-343;
      crates/maos-iac/src/adapter/mailbox.rs:665-675] — rupture ids (peer-frame-id half +
      process-local counter) can collide post-restart, suppressing a durable rupture record where
      pre-2b code halted; and `IacBusPort::deliver` flattens `Duplicate` into `Ok` for every legacy
      caller. Preserve fail-loud at the rupture-sink caller and state the `deliver()` semantics
      change in its doc. MEDIUM (auditor+blind).
- [x] [Review][Patch] Corrected doc cites a nonexistent method [crates/maos-a2a-core/src/router.rs:353]
      — names `bind_with_cohort_wiring_and_crossing_and_intake`; the shipped method is
      `bind_with_intake_sink`. The story that fixed two lying docs added a third wrong reference.
      MEDIUM.
- [x] [Review][Patch] `boot_nonce = 0` sentinel not explicitly rejected (AC4.1)
      [crates/maos-bin/tests/bounded_postures_2b.rs:120-140] — the control proves only that the TCP
      source stamps `self.own_boot_nonce`; a configured zero parses and binds, shipping the
      cross-host path with restart detection off — the exact escape AC4.1 forbids. Add nonzero
      validation at config-load/bind + negative test. MEDIUM (auditor+test-infra).
- [x] [Review][Patch] Two-daemon proof doesn't pin the operator-selected worker
      [crates/maos-bin/tests/two_host_delegation_2b.rs:490-533] — asserts the served event and a
      label from the six-value set, but never the fixture command, its deterministic output marker,
      or the `CliSubprocessOutput` row; a different host-granted worker with a valid label keeps
      AC1.6 green. MEDIUM (test-infra).
- [x] [Review][Patch] Beat-split test doesn't assert `two-host-signed-run` ABSENT (AC4.4)
      [xtask/src/tests/demo_j1_tests.rs:48-62] — verifies owners + emission of
      `two-host-delegation`, not the mandated ABSENT state for the 2c-owned beat (kloc-free home).
      MEDIUM (test-infra).
- [x] [Review][Patch] `accept_inbound` journals before verifying it is the addressed recipient
      [crates/maos-bin/src/delegation.rs:425-475] — checks `FrameKind` only; a TLS-authenticated
      peer can address another spirit/host and host B journals the row (evidence for a frame not
      addressed to it) before the loud failure. Add a `to`-recipient guard before the journal call.
      MEDIUM (blind).
- [x] [Review][Patch] Gate leg 7 satisfiable by inert text [xtask/src/check_j1_loopback_delegation.rs:533-605]
      — the needles match tokens anywhere in the file; the leg's own green fixture accepts inert
      `let outcome = HostBOutcome::Ran` bindings and empty test bodies, and the only destructive
      vector removes a token. Strengthen the needles to require assertion context or add the
      inert-binding proven-red vector (gate change needs a measured xtask grant; the vector itself
      is kloc-free). MEDIUM (test-infra).
- [x] [Review][Patch] Shutdown race in `serve_host_b_intake` [crates/maos-bin/src/delegation.rs:650-685]
      — `tokio::select!` without `biased;` may pick a queued frame over cancellation, starting
      another remote worker after shutdown began. LOW (edge).
- [x] [Review][Patch] Sink-uninstalled control can pass against a dead daemon
      [crates/maos-bin/tests/two_host_delegation_2b.rs:535-559] — no `try_wait` liveness assert on
      host B; a crashed daemon produces no served event and passes the negative control. LOW (edge).
- [x] [Review][Patch] `builtin_allowlist_grants_the_fixture_only` depends on ambient
      `MAOS_HOST_GRANTS` [crates/maos-bin/tests/host_grants_2b.rs:27-38] — claims a hermetic fixture
      but does not isolate the env var. LOW (test-infra).

**Deferred**

- [x] [Review][Defer] Ctrl-C/graceful-shutdown can wait forever on a never-exiting remote worker
      [crates/maos-bin/src/delegation.rs:601-633; main.rs:9561-9571] — deferred, pre-existing shape
      newly reachable; fault-injection semantics owned by `j1-crosshost-2c`.
- [x] [Review][Defer] Crash window between journal(`Written`) and worker spawn makes a delegated
      task durably look processed [crates/maos-bin/src/delegation.rs:442-450] — deferred; mechanism
      fix (reconciliation/recovery) is `2c`'s; record as a RELEASE-HOLDS claim boundary.
- [x] [Review][Defer] Digest-reply path ACKs a retry as `Duplicate` after a dropped-receiver NACK
      [crates/maos-a2a-core/src/router.rs:1353-1379] — deferred; 12.4a seam, owner `2c`.
- [x] [Review][Defer] `parse_host_grants_toml` defaults omitted `permitted_tier` to T3
      [crates/maos-bin/src/worker_spawn.rs:170-202] — deferred, pre-existing moved code; owner:
      worker-grant hardening lane.
- [x] [Review][Defer] Host grant keyed to manifest claims, not an attested executable
      [crates/maos-bin/src/worker_spawn.rs:384-427] — deferred, pre-existing 2a design (no
      digest/signature/resolved-path binding).
- [x] [Review][Defer] `revoke_cli_subprocess_exit` errors discarded; failed revocation leaves spawn
      token valid to TTL [crates/maos-bin/src/worker_spawn.rs:662-667] — deferred, pre-existing
      moved code.

**Dismissed (5, with grounds):** `insert_frame_event` result binding (false positive — the writer
panics internally on failure; the discarded value is an I2 typestate, not a `Result`); six remaining
`interpret_response` fall-through codes (explicit AC4.1 scope wall, 9-of-16 census recorded); ACK
before receiver-side execution ×2 (designed at-most-once semantics with the failure mode decided
and documented in `push_to_intake_sink`; reconciliation is `2c`'s — NOTE for `2c`'s preflight: the
new dropped-receiver `CODE_INTERNAL` NACK itself renders as `TransportFailed` at the sender via the
catch-all, the same misattribution shape as H13 on a code this story newly emits); "install chooses
its router" placement (satisfied structurally — the fork lives in `install_with_router`, main passes
a typed choice, call site immobile, matching the AC's own 12.3/13.6b no-caller-churn precedent).

**§A6 CLOSURE (2026-08-17).** All 17 patches (15 + 2 decision-derived) applied and verified by
execution: focused suites (`two_host_delegation_2b` 3/3 serial 10.79s, `bounded_postures_2b` 8/8 with
the two new live bind-refusal negatives, `host_grants_2b` 4/4 env-isolated, `j1_crosshost_2b_proven_red`
7/7 with the inert-binding vector, 1a/2a/1b trees 53/53 after the fixture-shape update,
`transparency_duplicate_discrimination_2b` 3/3, `demo_j1` bins 26/26), all four gates PASS, fmt clean,
kernel baseline 24472 = pinned, kloc green post-grant. Decisions D1/D2 implemented as ratified.
New production surface from the review: `GovernedMintError` (typed PDP/SSO vs kernel-policy
discrimination), bounded intake channels (`mpsc::Sender` end to end; host-B capacity 64 with the
NACK backpressure), high-bit outcome-id namespace reservation, pre-journal recipient guard,
`biased;` shutdown, zero-nonce bind rejection, lineage-cache Written-branch gating, rupture-sink
fail-loud. One measured grant recorded (`maos-bin` +98).

---

## Dev Notes

### Measured at CLEAN `87eb6c37`

| Instrument | Ceiling | Measured | Verdict |
|---|---|---|---|
| kloc `maos-bin` | 16260 | **16260** | **ZERO headroom.** D15 16178→16219, then 2a's review grant →16260 |
| kloc `maos-cli` | 4642 | **4642** | **ZERO** — 2a's review grant |
| kloc `maos-a2a-core` | 4654 | **4654** | **ZERO — the D10 wall** |
| kloc **`xtask`** | **38655** | **38655** | **ZERO — NEW.** Was +223; 1b's §A6 grant 38609→38655 consumed it |
| kloc `maos-a2a-tcp` | 1500 | **1085** | **+415** — the only uncontested capacity in the lane |
| kloc `maos-iac` | 6888 | **6852** | +36 |
| kloc `maos-audit` | 6665 | **6643** | +22 |
| kloc `maos-domain` | 8644 | **8694** | RED −50 — D14, not yours |
| kloc `maos-kernel-core` | 18248 | **18933** | RED −685 — D13, not yours |
| kloc `_aggregate_hardfail` | 147057 | **148211** | RED **−1154** — D17, standing, not yours (its row still says −492) |
| `check-kernel-baseline` | 24472 | **24472** | **GREEN** |
| `check-env-contract` | — | **RED** | Pre-existing, ownerless (H12) |
| Zero-cost surfaces | — | `crates/*/tests/`, `xtask/tests/`, `xtask/src/tests/`, **all of `spirits/`** | `kloc_check.rs:167-193` |

> **Why "clean tree" is in the AC.** Two scouts on the original preflight measured `maos-bin` at 16017
> and 16132 and concluded the D15 ceiling record was a broken instrument. Both had measured a working
> tree that `2a` was actively mutating. The instrument is fine; the measurement was contaminated.

### The receive path, hop by hop — read this before touching the transport

| # | Hop | file:line |
|---|---|---|
| 1 | `TcpListener::bind` + `tokio::spawn(accept_loop)` | `transport.rs:343`, `:355` |
| 2 | `accept_loop` → `listener.accept()` | `transport.rs:504`, `:516` |
| 3 | `serve_connection` (server TLS accept, `timeouts.handshake`) | `transport.rs:562`, `:577-580` |
| 4 | `resolve_verified_peer` (re-derives mTLS identity) | call `:590`, def `:679-688` |
| 5 | **`core.handle_intake_verified(...)`** | `transport.rs:637-643` → `router.rs:1494` |
| 6 | host binding / TLS-peer match | `router.rs:1504-1521` |
| 7 | `handle_intake_inner` → peer lookup → TOFU | `router.rs:1070`, `:1093`, `:1105` |
| 8 | **boot-nonce restart check** (G4) | `router.rs:1123-1159`, `tofu.rs:351-373` |
| 9 | consent granter / TTL stamp on send / expiry on accept (H4) | `router.rs:1170-1204`, `:866-871`, `:1207-1222` |
| 10 | accept-allowlist | `router.rs:1313` |
| 11 | Lamport advance | `router.rs:1451` |
| 12 | **`if let Some(sink)` — `intake_sink` is `None`, frame DROPPED** | `router.rs:1453-1458`, decl `:175`, init `:218` |
| 13 | ACK `delivered: true` | `router.rs:1460-1466` |
| — | *second sink push, same shape* (G1b.i) | `router.rs:1279-1283` |

### Tests and gates that constrain the change

| file:line | What | Exposure |
|---|---|---|
| `check_j1_loopback_delegation.rs:385-437` | the boundary leg | **AC2.3 must re-key it — H1** |
| `check_j1_loopback_delegation.rs:110` | `J1_TEST_SUFFIXES` without `_2b.rs` | **Silently un-enrolls your test file** |
| `check_j1_loopback_delegation.rs:890` | `let [leg1..leg6]` slice pattern | **Panics** if the leg count changes |
| `j1_crosshost_{1a,2a,1b}_proven_red.rs:{149,155,164}` | three synchronized `lay_green` trees | All three or the untouched ones vacuum |
| `j1_crosshost_1b_proven_red.rs:736-745` | hard-coded six leg names | Reds on a rename |
| `j1_crosshost_1b_proven_red.rs:821-837` | the flip vector — **replacement shape only** | Add the fork-shape twin |
| `topology_delegation_1a.rs:197-224` | every `.toml` in `spirits/topologies/` parses strictly | Constrains the new topology file |
| `topology_delegation_1a.rs:228-238` | `hosts == vec![TO_HOST]` in `j1-founder-loop.toml` | Do not add a second host line |
| `worker_manifests_2a.rs:274-355` | routability of every `j1-founder-loop*` file | Constrains the new topology file |
| `worker_manifests_2a.rs:352-358` | **exact list** `["j1-founder-loop","j1-founder-loop-codex"]` | **Reds on a third `j1-founder-loop*`** |
| `delegation_leg_1a.rs:123-136` | **exactly one** `TaskAssign` TL row per delegation | Reds if host A journals twice |
| `delegation_leg_1a.rs:210`, `:234` | `journal_completion(…3 args…)`, `p.result == "completed"` | **Compile** break if AC3.4 changes the signature |
| `enterprise_daemon_seam_13_5a.rs:52-171` | literal text + signatures of the daemon region | **Breaks if you relocate it** |
| `cross_team_crossing_13_6b.rs:320, 373, 478, 502` | same technique, paren-balanced | **Breaks if you relocate it** |
| `cross_team_crossing_13_6b.rs:1520-1533` | `RunningDaemon(Child)` killing `Drop` | **Copy this — 13_5c lacks it** |
| `router.rs:2333-2344` | `consent_envelope: None` ⇒ `ConsentUnclassified{Send, Absent}` | Pins G9 |
| `cohort_daemon_smoke_13_5c.rs:443-476` | hermetic daemon boot, **not** `#[ignore]`, **zero CI enrollment** | **Clone the shape; supply the binding** (H7) |
| `cohort_daemon_smoke_13_5c.rs:642-662` | a config carrying `own_boot_nonce` fails to boot | Constrains AC2.2 |
| `consent_refusal_1b.rs:452-456` | the transport-stamped consent TTL | **Pins H4 — G15 is disproved** |
| `mailbox_a2a_router_installer_1a.rs` | the set-once installer contract | On the critical path for AC2.1 |
| `t11_t12_chaos_absence.rs:170-177` (`maos-a2a-tcp/tests/`) | greps its own `Cargo.toml` for the `maos-kernel-core` substring | Reds if you add that dep to reach a TL |
| `demo_j1_tests.rs:48-55` | asserts `two-host-signed-run`.owner == `"j1-crosshost-2b"` | **AC4.4 must change this deliberately** |

### Where the code goes

| Concern | File | Anchor |
|---|---|---|
| **Enabling move** | `crates/maos-bin/src/lib.rs` | mirror `2a`; doctrine at `:18-25` and `:27-30` |
| Worker spawn (to relocate) | `crates/maos-bin/src/main.rs` | `run_cli_wrapper_manifest` `:938-1358` + `:439`, `:689`, `:706`, `:843`, `:203` |
| **Sink install point** | `crates/maos-a2a-tcp/src/transport.rs` | **`:322-325`** (beside `install_rupture_sink`, before `bind` `:343`); `core()` `:388`; chain `:139/:166/:196/:229/:268` |
| Sink declaration + the false doc | `crates/maos-a2a-core/src/router.rs` | `intake_sink` `:175`/`:218`, pushes `:1453-1458` **and `:1279-1283`**, `install_intake_sink` `:345`, doc `:343-344`, **correct pattern** `:350-355` |
| Router fork | `crates/maos-bin/src/delegation.rs` | `DelegationLeg::install` `:102-131`, installer call `:110`, drain precedent `:200-218` |
| Outbound port impl (reuse) | `crates/maos-a2a-tcp/src/transport.rs` | `impl A2ARouter for TcpA2ATransport` `:826-840` |
| Daemon builder | `crates/maos-bin/src/main.rs` | `build_cohort_a2a_daemon_runtime` `:10192-10320`, bind at `:10241-10256` |
| Daemon config path | `crates/maos-bin/src/main.rs` | `CohortDaemonFileConfig` `:9469-9479`, env read `:9507`, loader `:9526` |
| Typed `Duplicate` | `crates/maos-iac/src/adapter/transparency_log.rs` | `insert_frame_row_with_correlation` `:773`, INSERT `:797-802`, panic `:819-825` |
| Outcome labels | `crates/maos-bin/src/worker_cli.rs` | `WorkerCompletion` `:72-78`, `is_completed()` `:82-84`, `label()` `:87-106` (**six** values) |
| Return-hop builder (none exists) | `spirits/orchestrator/src/lib.rs` | `assign_frame_remote` `:332`; **`spirits/` is kloc-free** |
| Gate to EXTEND | `xtask/src/check_j1_loopback_delegation.rs` | files `:79-96`, suffixes `:110`, legs `:129-140`, boundary `:385-437`, vacuity `:906-915`, binding `:935` |
| Proven-red template | `xtask/tests/j1_crosshost_2a_proven_red.rs` | `write_file` `:147`, `lay_green` `:155`, `run_gate` `:196`, `assert_red` `:214`, baseline `:239` |
| Harness to clone | `crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs` | `fixture()` `:227-250`, `mint_pems` `:163-176`, `boot_hermetic_daemon` `:436-441`, `reap` `:379-382` |
| CI job | `.github/workflows/discipline.yml` | job `:1804-1882`, gate `:1815`, proven-red steps `:1817/:1819/:1821`, demo beats `:1831`, delegation legs `:1844`, ship-gate needs `:3234` |

### References

- Shared preflight: `_bmad-output/implementation-artifacts/j1-crosshost-2-cross-host-signed-run.md`
  (§2 P1-P14 — **P1, P4 and the `main.rs:10678` claim are corrected here by G5, G3 and G7**)
- Predecessors: `j1-crosshost-1a-frame-borne-delegation.md` (done, `6827dc87`),
  `j1-crosshost-1b-consent-proofs-and-gate.md` (done, `87eb6c37`),
  `j1-crosshost-2a-signable-heterogeneous-worker.md` (done, `0769869d`)
- Successor: `j1-crosshost-2c-two-host-signed-run.md` — inherits AC3's join key and AC4.1's postures.
  **Its preflight still expects the demo beat at `demo_j1.rs:797-801` naming `"j1-crosshost-2"`;
  AC4.4 invalidates that and must say so.**
- Deferrals naming **this story** by name: `deferred-work.md` (verdict-not-journaled),
  `worker_manifests_2a.rs:265-272` (`bilateral-2-host-mira-nash.toml`), 2a's F15 (`success_criteria`)
- Decision register: `_bmad-output/planning-artifacts/epics/epic-14-preflight-decisions.md`
  (D10, D13, D14, D15, D16, D17, **D18 — its measurement is corrected by H8**, D19)

---

## Dev Agent Record

### Agent Model Used

`anthropic/claude-opus-5` via the Oh My Pi harness (`bmad-dev-story` skill), dev pass 2026-08-17,
baseline `87eb6c37` (== HEAD at pass start; `git status --porcelain` carried only
`_bmad-output/**` story-tracking edits, all kloc-free). **§A6 review is still owed and is
NON-DEGRADABLE** — Blind Hunter · Edge Case Hunter · Acceptance Auditor · Test-Infra Auditor +
runtime execution, and the reviewer model MUST differ from `anthropic/claude-opus-5`. A green CI does
not mean the §A6 net ran (D19 verified still open at `87eb6c37`; seven digit-prefix story-file
walkers skip this filename, so no gate reads it).

### Debug Log References

### Completion Notes List

**T0 (blocking condition 3, D18) — record written, no code. Every claim re-verified by execution at
`87eb6c37`, not inherited.**
- **`2b` does not owe D18's repair.** `map_a2a_error_to_iac_bus` has exactly **two** call sites
  repo-wide outside tests, and **both are outbound**: `crates/maos-a2a/src/adapter.rs:113`
  (`LoopbackA2ARouter::route_outbound`) and `crates/maos-a2a-tcp/src/transport.rs:834`
  (`TcpA2ATransport::route_outbound`). Host B's intake path (`handle_intake_verified` →
  `handle_intake_inner`) never calls it, so D18's shape is byte-identical before and after this
  story. The deadline *"before `j1-crosshost-2b` writes its first line"* named the right story and
  the **wrong defect**; the defect this story genuinely makes reachable is **H13**, repaired under
  T10b.
- **(a) The flattened error was not extended.** No new operator-visible deny in this diff is built
  on `IacBusError::CrossHostRouteFailure(String)`, which
  `crates/maos-domain/src/iac_bus_types.rs:69-72` already marks *DEPRECATED — "use the typed
  sub-variants above instead."* T10b's new arm returns the **typed** `A2AError::PinInvalidated`,
  which `router.rs:1697-1701` already maps to the typed `IacBusError::CrossHostPinMismatch`.
- **(b) The register's mechanism is corrected; its conclusion stands.** D18 premise (a) is
  **UPHELD**: `router.rs:1688` hardcodes `CrossHostIntentDirection::Accept` on the
  `IntentDeniedAtPeer` arm while `:1675` yields `Send` on the `IntentDenied` arm, so the `-32001`
  pair **is** distinguishable at the `IacBusError` layer. Only `router.rs:1676` — the
  `IntentDirection::Accept => …Accept` mapping *inside* the `IntentDenied` arm — is dead:
  `IntentDirection::Accept` has **zero construction sites anywhere** (production or test); its sole
  repo-wide occurrence is that `match` **pattern**, which survives only because `match` must be
  exhaustive. **A dead line inside a live discriminator is not a dead discriminator.** This story's
  own earlier text (G12) claimed otherwise and is corrected by H8.
- **(c) Ownership unchanged.** D18 stays with **14-4**; the `maos-domain` line stays with **D14 /
  14-7**. Nothing was absorbed here.

**T1 (AC1.1) — the worker-spawn surface is under the library, and the move paid for itself.**
New `crates/maos-bin/src/worker_spawn.rs` (`pub mod` in `lib.rs`) holds `run_cli_wrapper_manifest`
plus `RunArgs`/`parse_run_args`, `parse_sandbox_tier`, `resolve_cli_binary`,
`load_host_grant_allowlist` (+ its `builtin_fixture_grant`/`parse_host_grants_toml`) and
`issue_enterprise_governed_capability` (+ `principal_attributes_for_pdp`). `enterprise_pdp_runtime`
followed it to `lib.rs` because the governed mint takes `&EnterprisePdpRuntime`; the FILE stayed at
`src/enterprise_pdp_runtime.rs`, which `cohort_daemon_smoke_13_5c.rs:886` reads by `include_str!`.
`main.rs` CONSUMES both (`use maos_bin::…`) and re-declares neither — the shape the
`worker-cli-under-library` leg demands.
- **Justified by in-process item visibility, not "no legal home" (H6).** `worker_completion_2a.rs:871`
  already drives this surface end-to-end by subprocess; what was impossible was a typed
  `WorkerCompletion` assertion or a port injection from `tests/`. `two_host_delegation_2b.rs`'s
  replay negative calls `handle_one_inbound` in-process, which is that gap closed.
- **The daemon region did NOT move** (Trap 9): `enterprise_daemon_seam_13_5a.rs` and
  `cross_team_crossing_13_6b.rs` assert its literal text inside `include_str!("../src/main.rs")`.
- **Measured, clean tree:** `maos-bin` 16260 → **16219 = +41 REFUNDED**, because the in-`src`
  `host_grant_tests` module moved to `crates/maos-bin/tests/host_grants_2b.rs`. That is a deliberate
  deviation from T1's "do not plan around a refund": it was not planned around, it was measured, and
  it is the doctrine the `worker-cli-under-library` leg enforces on `worker_cli.rs` — an in-`src` test
  module is budget-charged AND CI-invisible. Those four tests now RUN (`_2b.rs` is derived by
  `J1_TEST_SUFFIXES`). The relocation itself is kloc-neutral, exactly as predicted.
- **Two controls followed the code, deliberately, preserving their invariant.** The mint wrapper's
  `.issue_with_mediation(` call moved with it, so "exactly ONE direct kernel mint in the composition
  root" is now counted over the PAIR (`main.rs` + `worker_spawn.rs`) in
  `enterprise_daemon_seam_13_5a.rs` and in `check_enterprise_identity`'s `issuance-bypass-absence`
  leg. Counting `main.rs` alone would score 0 on a pure relocation while a SECOND mint added in the
  relocated file stayed invisible — strictly weaker. `cohort_daemon_smoke_13_5c.rs`'s
  `SCANNED_SOURCE_FILES` went 15 → 16.

**T2 (AC1.2, AC1.3) — the receiver stops dropping the frame.** `bind_with_intake_sink` is the sixth
`bind_with_*` sibling; it installs the sink beside `install_rupture_sink`, **before**
`TcpListener::bind` (Trap 17 — that ordering is what avoids the filed in-flight race). Three false
doc comments corrected: `install_intake_sink`'s "test-only hook", `core()`'s "for tests that drive
intake directly", and the inline `// (5) … (test hook)`. **Both** push sites are handled — the
delegation site and the digest-reply `Accepted` branch (G1b.i) — through one
`push_to_intake_sink`. **`let _ = sink.send(…)` is gone (G1b.ii); the decided failure mode is
stated:** no sink installed ⇒ `Ok` (unchanged pre-2b behaviour for deployments with no J1 consumer);
sink installed and send accepted ⇒ `Ok`; **sink installed and receiver DROPPED ⇒ NACK
`CODE_INTERNAL`, never an ACK** — an ACK there is a lie about durability. The drain
(`DelegationLeg::accept_inbound`) dispatches on `FrameKind` copying `delegate`'s fail-closed shape
and refuses any non-`TaskAssign` loudly; there is no `_ => {}`.

**T3 (AC1.4, AC1.5) — journaled, and off the reactor.** The inbound frame is journaled through
`IacBusAdapter::deliver_typed` (never a raw TL insert, never the raw-byte
`enqueue_frame`/`broadcast_frame` that mislabels every row `TaskAssign` — G14.iv), so the row carries
the RECEIVED `frame_id`. The worker runs under `tokio::task::spawn_blocking`:
`run_cli_wrapper_manifest` is synchronous and blocks for the worker's whole lifetime
(`spawn_and_bridge`, then the blocking `pump_to_journal` and `wait_and_finalize`), and a direct call
would park a `multi_thread` worker thread on the daemon that also serves the accept loop, the cohort
pull ticker and the digest replier.

**T4 (AC1.6) — `crates/maos-bin/tests/two_host_delegation_2b.rs`, 3 tests, none `#[ignore]`d, no
Postgres. This is the first multi-process daemon test that runs in CI (H7).** Cloned from
`cohort_daemon_smoke_13_5c.rs` (NOT `cross_team_crossing_13_6b.rs:1642`), per-child `MAOS_AUDIT_DB` on
the `Command` (never `std::env::set_var`, so D16 does not apply), two distinct data homes, ephemeral
`127.0.0.1:0` with the port scraped from the readiness line, and the `RunningDaemon(Child)` killing
`Drop` guard that 13_5c lacks. Includes the AC1.2 falsifier: with `worker_manifest` absent no sink is
installed, the frame is still ACKed, host B never serves it and its log does not contain the id — the
pre-2b receiver, which is what makes the positive non-vacuous.

**T5 (AC2.1, AC2.2) — the composition-root fork.** `DelegationLeg::install_with_router` chooses
between `LoopbackRehearsal` and `CrossHostVerified(Arc<dyn A2ARouter>)`; `install` keeps its exact
1a/1b shape and delegates. `main.rs`'s call site did not move (moving it past the `MAOS_ONE_SHOT`
dispatch is a dead end — the daemon arm returns first). Host A passes `Arc<TcpA2ATransport>` directly:
**zero new adapter code** (G7). `delegate` now returns a typed `DelegationOutcome`, and the two arms
end in different places — on `SentCrossHost` host A **does not also run the worker** (that would
double-execute the delegated task and let host A journal a completion for work it did not do) and
journals no `TaskComplete`, leaving the delegation in flight as FR20 requires.
Transport config reaches `maos run` through `MAOS_COHORT_DAEMON_CONFIG` → `CohortDaemonFileConfig`
with a new `#[serde(default)] worker_manifest`; `TOPOLOGY_SPIRIT_KEYS` was NOT widened and
`own_boot_nonce` stays absent. New `spirits/topologies/j1-founder-loop-crosshost.toml`, with
`worker_manifests_2a.rs`'s exact list edited deliberately to enrol it under the `j1-founder-loop*`
prefix (naming it outside the prefix would have EXEMPTED it from the routability control — the weaker
posture).
**Q3 landed in the ratified order and landed green:** the two decorative `host =` lines are stripped
from `bilateral-2-host-mira-nash.toml` on 1a's `priority_weight` precedent, and only THEN was the
loadability control widened past the stem filter (`validate_topology_is_loadable`). It passed on day
one because the only unloadable file had just become loadable; the next decorative key now reds at its
origin commit. Class-Spirit remote routing stays unbuilt.

**T6 (AC2.3, AC2.4) — SHIP-BLOCKER closed by SPLITTING the fact, not by re-keying the needle.**
- **(a)** Leg 2 `loopback-from-host-unverified` keeps both doors (including `router.rs`'s peer
  resolution expression) and its needles are UNCHANGED, but its doc, its finding text and the gate's
  PASS line no longer promise a flip. It reports a **permanent** property of the loopback rehearsal
  arm and stays correctly `true`: `LoopbackA2ARouter` is untouched and still calls `handle_intake`
  directly, so after this story the loopback frame still picks its own judge. `boundary MOVED` was
  renamed and re-scoped to what it can actually mean — a static door changed shape.
- **(b)** NEW **seventh** leg `cross-host-identity-proof` publishes the cross-host fact **derived from
  execution**: it needles the two-daemon proof's assertion skeletons and its registration count, and
  `completion-vectors-enrolled` (whose target set is DERIVED from `crates/maos-bin/tests/`, now
  including `_2b.rs`) refuses to let the file go un-run. Linter and judge are joined by the CI
  enrollment line and nothing else. New JSON boolean `cross_host_identity_proven`.
- **(c)** `xtask/tests/j1_crosshost_2b_proven_red.rs` plants the **FORK** shape — both
  `paired_loopback_router(` and the TCP branch — and asserts the gate does NOT claim a moved boundary
  and stays green with the boundary still `true`. 1b's replacement-shape vector stays green on its own
  terms. Plus 5 more vectors (needle deletion, missing subject, sub-threshold registrations,
  un-enrolled target, and the baseline-green guard).
- All six coupled sites moved together: `ledger_leg_names()`, `judge()`'s slice pattern (`[leg1..leg7]`
  and its panic message), 1b's hard-coded name list, `leg_narration`, the `--skip-gate` ABSENT loop,
  and the published JSON. The new leg calls `entered()`/`checked()` per condition, so `vacuous_legs()`
  cannot silently pass it. **The leg was not deleted.**

**T7 (AC3.1, AC3.3) — the join, on the BYTES.** The two-daemon proof joins host A's emit row and host
B's intake row on the same sixteen `frame_id` bytes, read from the two SQLite logs directly. **Never
on `kind`** (H9: `insert_kernel_event_returning_id` stamps `TaskComplete` on every kernel event across
nine production callers). **Zero** new frame fields, **zero** `maos-domain` lines, **zero**
`maos-audit` lines, no `correlation_id` wiring and no second join helper.

**T8 (AC3.2) — SHIP-BLOCKER: the remote-triggerable kernel halt, closed in the same change as T2.**
`insert_frame_row_with_correlation` now returns `FrameRowWrite { Written, Duplicate }` and matches the
duplicate arm on the **EXTENDED** code (`SQLITE_CONSTRAINT_PRIMARYKEY` / `_UNIQUE`), never on
`ErrorCode::ConstraintViolation` — which is mapped from the primary `SQLITE_CONSTRAINT` and would
silently reclassify a NOT-NULL defect (ten of twelve columns are `NOT NULL`) as "already journaled",
the exact inversion of the intent. Written from scratch (H10: no arm existed to copy). **Every other
write error still panics**, and the reason is in the code comment. Only the peer-supplied-id writer
(`insert_frame_event_with_id`) returns the typed outcome; kernel-minted-id writers keep halting via
`halt_on_duplicate`. `deliver_typed` propagates it and does NOT re-deliver on `Duplicate`, so a replay
cannot re-spawn a worker — at-most-once end to end, which is what the transport already is (G3). **No
general dedup store was built** (AC3.5). Landed inside `maos-iac`'s +36: measured **6888/6888, GREEN,
no grant**.

**T9 (AC3.4, AC3.6) — the vocabulary where it is reachable, and the verdict on disk.**
(a) `journal_inbound_outcome` writes host B's REAL `WorkerCompletion::label()` from the six-value set,
never a literal; its row id is derived by flipping the head's high bit and keeping the `run_nonce`
tail, because reusing the inbound id would collide with the row just written.
(b) The oracle verdict is **journaled** — a `TelemetryEvent` row under intent
`worker.completion-verdict`, carrying `last_stdout_tl_ref` NEXT TO the verdict with the distinction
written into the payload. `deferred-work.md` names this story as owner; it was `println!`-only, and
stdout is not evidence.
(c) `success_criteria` is **disposed of, not silently dropped**: surfaced on
`InboundDelegation::TaskAssign` with the reason it is not used as a contract — it is circular (the
Orchestrator writes criteria nothing evaluates), so treating it as an oracle here would invent one the
sender never agreed to. Host B's verdict comes from `WorkerCli::parse_completion`, which IS a
contract.
(d) Host A's hard-abort is **not softened**; the boundary is stated in code and belongs to Story 6.2.
(e) `delegation_leg_1a.rs:86`/`:119` were a **compile** break, updated deliberately to assert
`DelegationOutcome::RehearsedLocally` — and that assertion now also pins that the rung-1 control did
not silently start exercising rung 2's path.

**T10 (AC4.1) — `crates/maos-bin/tests/bounded_postures_2b.rs`, 6 tests.** Boot-nonce provisioning
(missing `boot_nonce` fails to deserialize; the `boot_nonce = 0` sentinel is **explicitly rejected**
for the cross-host path, asserted against the loopback source that DOES stamp it and the TCP source
that does not); the §D1 TRANSITIONAL consent-TTL posture with the half nothing else pinned (an
explicit granter `valid_until_ns` survives `prepare_outbound` untouched) and **no negative control for
a non-gap** (H4 — G15 is disproved); and no-DNS (a hostname passes `validate` and fails at first
dial). `RELEASE-HOLDS.md` §Claim boundaries gains its **first J1 entry** (D19 option (b)'s sanctioned
home, previously unused).

**T10b (AC4.1, H13) — the one arm this story owed.**
`CODE_SPIRIT_RESTART_DETECTED => Err(A2AError::PinInvalidated { peer, awaiting_repin: true })`. It
fell through `interpret_response`'s catch-all into `A2AError::TransportFailed` →
`CrossHostTransportFailure`, so a **permanent** pin invalidation was reported as a **transient**
network fault and the action it implies — retry — is the one action that can never work. Zero new
types. **`kloc.toml:87` is cited by name in the grant** (correctness repair on a security path).
**Scope wall held:** the census is corrected by measurement — the story's "9 of 16" was the
PRE-repair count (9 typed / 7 fall-throughs); this story's arm makes it **10 typed of 16 defined**,
with exactly six fall-throughs (`PARSE_ERROR`, `INVALID_REQUEST`, `METHOD_NOT_FOUND`, `TIMEOUT`,
`FRAME_TOO_LARGE`, `INTERNAL`) machine-asserted so they cannot drift. Those six are **not newly
reachable here and were not repaired**; a nine-arm refactor inside a frozen crate is a different
story. Owner: the same lane that owns D18 (**14-4**).

**T11 (AC4.3) — CI enrollment, in the mandated order.** `"_2b.rs"` was added to `J1_TEST_SUFFIXES`
**before** any `*_2b.rs` file was written. All THREE derived `_2b` targets
(`two_host_delegation_2b`, `host_grants_2b`, `bounded_postures_2b`) plus
`j1_crosshost_2b_proven_red` are invoked inside the existing Blocking
`check-j1-loopback-delegation` job, and `cohort_daemon_smoke_13_5c` is enrolled too — it had ZERO CI
invocation across all eleven workflow files, so its header's "Blocking" was a comment. All three
`lay_green` fixture trees lay the new targets and their fixture workflows invoke them, or the
untouched trees' `baseline_fixture_tree_is_green` reds and every vector in them passes vacuously (1b
paid that ship-blocker twice). **No `services:` block was added** — `check_loom_substrate_drift`'s
leg 2 rejects an unregistered service-bearing gate job; the shape copied is
`check-live-bilateral-consent`. `completion-vectors-enrolled` now evaluates **8** derived targets.

**T12 (AC4.2, AC4.4) — budget, beat, record.**
*Budget, measured in a clean tree at every step, code-before-ask, never an estimate.* **Free
reductions taken FIRST:** the dead `CODEX_ORACLE`/`CLAUDE_ORACLE` consts (−4 `xtask`) and the
`host_grant_tests` relocation (+41 `maos-bin`). Then **three** named grants, each EXACT MEASURED with
ZERO headroom (the D15 discipline):

| key | before | after | Δ | note |
|---|---|---|---|---|
| `maos-bin` | 16260 | **16640** | +380 | net of the +41 refund |
| `xtask` | 38655 | **38742** | +87 | net of the −4 reduction |
| `maos-a2a-core` | 4654 | **4669** | +15 | under `kloc.toml:87`, cited by name |
| `maos-iac` | 6888 | **6888** | 0 | **GREEN, no grant** — trimmed to fit the +36 |
| `maos-a2a-tcp` | 1500 | 1121 | — | +379 still free; the transport work went here |
| `maos-cli` | 4642 | 4642 | 0 | untouched |

*Standing reds ATTRIBUTED, NOT ABSORBED:* `maos-kernel-core` **18933/18248 = −685** (D13) and
`maos-domain` **8694/8644 = −50** (D14) are byte-unchanged by this story. `_aggregate` is
**148765/147057 = −1708**; it was **−1154** at `87eb6c37`, so **+554 of it is this story's** and every
one of those lines is covered by a named per-crate grant above. The aggregate ceiling itself is D17's
and is **not re-based here** — `kloc.toml:60-65` reserves that for an epic retrospective or an
explicitly authorized grant, and this is neither.
*Also attributed, not absorbed:* **`check-env-contract` is RED at HEAD and still RED, with
byte-identical findings** (`MAOS_OPERATOR_BEARER_TOKEN` / `MAOS_OPERATOR_HTTP_BIND`) — verified by
running the gate in a clean `git archive` worktree of `87eb6c37` and diffing the reported variables.
**This story did NOT repair it and did not touch `env_contract.rs`**; the AC2.2 work reused the
already-registered `MAOS_COHORT_DAEMON_CONFIG` and `MAOS_HOST_GRANTS` stayed inside the walked
`crates/maos-bin/src/` tree. Ownerless, as `1b` and `2a` both recorded.
*Two more pre-existing reds, both verified identical at `87eb6c37` in the same clean worktree:*
`check-service-boundary` (P3 `Arc`/`Vec` field violations in `SecurityManagerAdapter` /
`VerifiedImageLock`) and the `maos-bench --bench audit_query_latency` harness panic.
*Demo beat SPLIT as ratified (Q4).* `two-host-delegation` — *two real hosts over mTLS/TOFU, a frame
crossed, a worker ran on the far side, both logs carry the same sixteen bytes* — is owned and flipped
by this story and **renders `PROVEN_BLOCKING` by execution**, derived solely from the seventh gate leg
`cross-host-identity-proof` (deriving it from leg 2 would have re-created the tripwire-that-cannot-fire
this story exists to repair). `two-host-signed-run` is re-owned to **`j1-crosshost-2c`** and stays
ABSENT. `demo_j1.rs:28`'s stale module doc, the printed non-claim block, the seventh leg's narration
and the `--skip-gate` ABSENT counterpart all moved with it; the pinning test is renamed
`two_host_beats_are_owned_by_their_crosshost_stories` and asserts BOTH owners. Ledger: **19 executed
`PROVEN_BLOCKING` + 3 ABSENT** (was 17 + 3).
**`j1-crosshost-2c`'s preflight is INVALIDATED** where it still expects the beat at
`demo_j1.rs:797-801` naming `"j1-crosshost-2"`.
*`coverage-matrix.yaml` FR23a* records both inheritances: loopback peer authentication is **not**
discharged for the loopback arm (permanent there) while the cross-host path now binds wire identity;
D18 is **not** this story's repair, and the adjacent H13 defect is. Not cited as evidence anywhere —
the row itself discloses `mode: warning`, so the file cannot fail.
*D19 disclosed:* all seven digit-prefix story-file walkers remain intact, so no gate reads this
filename; the model/§A6 fields above are populated anyway. **No ABI change was made** — stated as a
fact about this diff, never as an `abi-diff` result (FLAG-E4; `abi_diff.rs` scopes to
`crates/maos-spirit-abi` only).

**TWO DEFECTS FOUND BY THIS STORY'S OWN PROOF, and they are opposite verdicts.**
1. **DISPROVED — no production defect.** The two-daemon proof first failed with
   `manifest_version_absent`, which looked like host A's cross-host `bind` missing a
   `CohortManifestGate`. Measured: `crates/maos-cohort/src/state.rs:548-556` returns
   `CohortConsentVerdict::Defer` for any peer outside the roster — *"a mixed-deployment bilateral
   path, not a cohort denial"* — and intake treats `Defer` as a PASS for every non-crossing intent.
   `reconcile_transport_identity_with_manifest` says the same in its own comment. The J1 peers must
   NOT be cohort members (ADR-003, which `bilateral-2-host-mira-nash.toml`'s header already states);
   the fixture had over-declared them. **No production change was made.**
2. **REAL, and it is the third instance of this story's own failure shape.**
   `init_monotonic_base()` was never called on the `cohort-a2a-daemon` arm, so the first inbound
   frame host B ever journaled panicked in `Mailbox::deliver`'s Phase 2 at
   `monotonic_now_ns()`. Every other `MAOS_ONE_SHOT` arm and the `maos run` path initialize it inside
   their own branch; the daemon arm did not, **because before AC1.2 installed a production intake sink
   the daemon never delivered a frame through the Mailbox at all** — it authenticated, ACKed
   `delivered: true`, and dropped. Fixed as that arm's first statement (idempotent). This is the same
   shape as G16 and H13: **a defect that was unreachable, inherited by the story that makes it
   reachable.** The story's own framing — *"the story's deliverable is also its threat model"* — held
   three times.

**Language constraint honoured (AC3.6).** Every artifact this story produced claims *"the crossing is
proven in two logs"* and nothing more. No artifact claims *round trip*, *the task completed back on
host A*, *signed*, or *reconciled bundle*. The return hop is a second delegation in the opposite
direction — second intent, second allowlist pair, second consent envelope — and is `2c`'s.

**Q5 recorded and NOT routed here:** `j4-mira-nash.toml` and `bilateral-2-host-mira-nash.toml` are the
same two Spirits in two files, now differing only in `[topology].name`. Someone should decide whether
both should exist.

### File List

**New — production**
- `crates/maos-bin/src/worker_spawn.rs`
- `spirits/topologies/j1-founder-loop-crosshost.toml`

**New — controls / vectors (all kloc-free)**
- `crates/maos-bin/tests/two_host_delegation_2b.rs`
- `crates/maos-bin/tests/bounded_postures_2b.rs`
- `crates/maos-bin/tests/host_grants_2b.rs`
- `xtask/tests/j1_crosshost_2b_proven_red.rs`

**Modified — production**
- `crates/maos-a2a-core/src/router.rs`
- `crates/maos-a2a-tcp/src/transport.rs`
- `crates/maos-bin/src/delegation.rs`
- `crates/maos-bin/src/lib.rs`
- `crates/maos-bin/src/main.rs`
- `crates/maos-iac/src/adapter.rs`
- `crates/maos-iac/src/adapter/drr_scheduler.rs`
- `crates/maos-iac/src/adapter/mailbox.rs`
- `crates/maos-iac/src/adapter/transparency_log.rs`
- `spirits/topologies/bilateral-2-host-mira-nash.toml`

**Modified — gates, controls, enrollment, budget, records**
- `.github/workflows/discipline.yml`
- `xtask/kloc.toml`
- `xtask/src/check_enterprise_identity.rs`
- `xtask/src/check_j1_loopback_delegation.rs`
- `xtask/src/demo_j1.rs`
- `xtask/src/tests/demo_j1_tests.rs`
- `xtask/tests/j1_crosshost_1a_proven_red.rs`
- `xtask/tests/j1_crosshost_1b_proven_red.rs`
- `xtask/tests/j1_crosshost_2a_proven_red.rs`
- `crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs`
- `crates/maos-bin/tests/delegation_leg_1a.rs`
- `crates/maos-bin/tests/enterprise_daemon_seam_13_5a.rs`
- `crates/maos-bin/tests/worker_manifests_2a.rs`
- `RELEASE-HOLDS.md`
- `tests/coverage-matrix.yaml`

### Verification executed (not asserted)

| What | Result |
|---|---|
| `cargo test --workspace --all-targets` | **CORRECTED BY §A6 REVIEW (P2): the dev-pass tally "507 passed, 0 failed" was a FAIL-FAST PARTIAL** — the run aborts at the pre-existing `maos-bench` bench panic before `--bin maos` executes, which masked one real regression (Review P1, fixed). True post-review state (`--no-fail-fast`): **3835 passed / 1 failed / 3 failing targets — all three pre-existing** (maos-bench bench, kernel-core `hot_swap_latency` bench, `cross_team_consent_13_3` intra-file parallel flake; the last verified failing byte-identically at `87eb6c37` in a clean worktree while passing in isolation). Never tally a fail-fast run as green. |
| `cargo test -p maos-bin --test two_host_delegation_2b -- --test-threads=1` | **3 passed** (two real daemons, real mTLS, 10.6s) |
| `cargo test -p maos-bin --test bounded_postures_2b` | **6 passed** |
| `cargo test -p maos-bin --test host_grants_2b` | **4 passed** (previously CI-invisible) |
| `cargo test -p xtask --test j1_crosshost_2b_proven_red` | **6 passed** (incl. the fork-shape vector) |
| `cargo test -p xtask --test j1_crosshost_{1a,2a,1b}_proven_red` | **53 passed** |
| `cargo test -p xtask --bins demo_j1` | **26 passed** |
| `check-j1-loopback-delegation` | **PASS** — 7 legs, 0 findings |
| `check-kernel-baseline` | **PASS — 24472 = pinned 24472, ZERO Δ** |
| `check-enterprise-identity` | **PASS** — 10 legs |
| `check-composition-root-completeness` | **PASS** |
| `cargo fmt --all --check` | **clean** |
| `cargo run -p xtask -- demo-j1 --skip-build` | `two-host-delegation` **PROVEN_BLOCKING** (judged by `cross-host-identity-proof`, 10 checks); `two-host-signed-run` **ABSENT**, owner `j1-crosshost-2c` |
| `kloc-check` | 3 granted keys GREEN; RED only on D13/D14/D17, all attributed above |
| `check-env-contract`, `check-service-boundary`, `maos-bench` bench | RED, **byte-identically RED at `87eb6c37`** in a clean `git archive` worktree — pre-existing, attributed, not absorbed |

---

## Open Questions

**Q1 — RESOLVED 2026-08-16 (round-table).** The `TaskComplete` return hop is **deferred to `2c`**;
`consent_envelope: None` makes a partial attempt fail *closed*, so there is no useful middle. AC3.6
carries the binding language constraint.

**Q2 — RESOLVED 2026-08-16 by this re-baseline.** *Was: is the debug-only boot-nonce pairing an
acceptable posture to ship on?* The premise was too strong. A **release-build, same-machine**
two-process pairing is achievable with no debug hook by reading host A's nonce from its own
Transparency Log (G4 hatch 2), and the `boot_nonce = 0` sentinel (hatch 1) is available but is
**rejected** because it ships the first cross-host path with restart detection off. The residual is a
*provisioning* gap — no automated peer-nonce channel — which AC4.1 states as a posture and hands
forward, rather than a proof gap that would block this story.

**Q3 — RESOLVED 2026-08-16 (round-table, unanimous). Neither "unblock" nor "inherit": the premise was
wrong.** `bilateral-2-host-mira-nash.toml`'s `host` keys on class Spirits are not a forward
declaration — they are **1a's own `priority_weight` defect, missed in 1a's own file**: a key claiming
behaviour the parser refuses, in the very pass that stripped `priority_weight` for exactly that
reason. The file's header says the bilateral join rides on operator-supplied `A2APeerConfig`, not on
manifest fields (ADR-003), and `j4-mira-nash.toml` is the same pair without `host` and loads fine —
so the keys are decorative. **Strip them (2 lines), then widen the loadability control** past the
`j1-founder-loop*` stem; in that order it lands green and Dana's outage objection never materialises.
Class-Spirit remote routing stays unbuilt and unassigned to this story. See AC2.2 and T5.

**Q4 — RESOLVED 2026-08-16 (round-table, unanimous): SPLIT the beat.** `two-host-delegation` is owned
and flipped by this story; `two-host-signed-run` is re-owned to `j1-crosshost-2c` and stays ABSENT.
Re-owning wholesale was rejected — it leaves a mechanism story with nothing to show, so the narrated
artifact would render ABSENT against a story that had already shipped. ≈5 `xtask/src` lines, funded by
deleting the two dead gate constants in the same commit. See AC4.4 and T12.

**Q5 — NEW, OPEN, and it is nobody's yet.** `j4-mira-nash.toml` and `bilateral-2-host-mira-nash.toml`
are the same two Spirits in two files, differing only in `[topology].name` and the (now-stripped)
`host` keys. Raised by Yui at the round-table and deliberately **not** routed to this story. Someone
should decide whether both should exist; `2b` records it and moves on.

---

## Change Log

| Date | Change |
|---|---|
| 2026-08-17 | **§A6 REVIEW CLOSED → `done`** (reviewer `zai/glm-5.3`, 4 layers + runtime; 34 raw → 2 decisions ratified + 15 patches applied + verified, 6 deferred, 5 dismissed). Headline finds: (1) a REAL regression the dev's fail-fast workspace tally masked — the 2b dispatch threading pushed `enterprise_posture_required,` past the 13_5a unit test's 2,000-char `include_str!` window (its twin in `tests/` was updated, the `main.rs` twin was missed); window now 2,600 with provenance. (2) The "507 passed / 0 failed" verification row was a partial tally — corrected to the `--no-fail-fast` truth (3835/1, all failures pre-existing). (3) Two security holes in the new receiver closed by ratified decision: PDP/SSO denial on a remote-requested spawn now REFUSES (typed `GovernedMintError`; local path keeps 2a's FORK B), and host B's intake channel is BOUNDED (64) with `try_send` → NACK backpressure. (4) Peer-crafted frame-id collisions can no longer silently drop outcome evidence (high-bit namespace + checked write verdict), and a replay can no longer poison the retraction lineage cache. One measured review grant: `maos-bin` 16640→16738 (+98). ||
| 2026-08-17 | **DEV PASS COMPLETE → `review`** (`anthropic/claude-opus-5`, baseline `87eb6c37`, all 14 tasks). **The story was right about where the work was: the receiver was never missing.** The whole receiving mechanism was one `install_intake_sink` call (the new `bind_with_intake_sink`, installed beside `install_rupture_sink` BEFORE `TcpListener::bind`) plus a consumer behind it (`DelegationLeg::accept_inbound` → `handle_one_inbound` → `serve_host_b_intake`). Two real `maos` processes now cross a `task.assign` over real mTLS with real TOFU and a real per-process boot nonce, host B spawns its own worker off the reactor, and **both Transparency Logs carry the same sixteen `frame_id` bytes** — proven by `crates/maos-bin/tests/two_host_delegation_2b.rs`, the **first multi-process daemon test that runs in CI** (H7). **The ship-blocker (H1) was closed by SPLITTING the fact, not by re-keying the needle:** leg 2 keeps both doors and its needles, but now states the truth — the loopback rehearsal arm self-asserts its peer PERMANENTLY, so `loopback_from_host_unverified` stays correctly `true` and a `true → false` assertion would assert something that must never happen — while a NEW seventh leg `cross-host-identity-proof` publishes the cross-host claim **derived from the executed two-daemon proof**, joined to it by the CI enrollment line and nothing else. A fork-shape proven-red vector (`j1_crosshost_2b_proven_red.rs`) pins that the runtime fork does NOT silently claim a moved boundary, which is the twin 1b's replacement-shape vector was missing. **The second ship-blocker (AC3.2) shipped in the same change as the sink that made it reachable:** a peer re-sending one frame used to halt the receiving Host through the plain `INSERT` + `panic!`, and now returns a typed `FrameRowWrite::Duplicate`, discriminated on the SQLite **extended** code so a NOT-NULL defect can never be reclassified as "already journaled" — every other write error still panics, because that panic IS the I2 guarantee. **Three defects were inherited, not introduced** — the story's own framing ("the story's deliverable is also its threat model") held three times: G16's duplicate-`frame_id` halt; H13's `CODE_SPIRIT_RESTART_DETECTED` fall-through, which reported a PERMANENT pin invalidation as a transient network fault and now returns the typed `A2AError::PinInvalidated` (`kloc.toml:87` cited by name); and a THIRD found by the proof itself — `init_monotonic_base()` was never called on the `cohort-a2a-daemon` arm, because before this story the daemon had never delivered a frame through the Mailbox at all. **One "blocker" was DISPROVED by measurement rather than fixed:** the `manifest_version_absent` refusal was a fixture over-declaration, not a missing cohort gate — `crates/maos-cohort/src/state.rs:548-556` defers for any peer outside the roster ("a mixed-deployment bilateral path, not a cohort denial"), and per ADR-003 the J1 peers must NOT be manifest members. **Q3 landed in the ratified order and landed GREEN on day one:** strip the two decorative `host =` keys, THEN widen the loadability control past the `j1-founder-loop*` stem. **Q4's beat split executes:** `two-host-delegation` renders `PROVEN_BLOCKING`, `two-host-signed-run` is re-owned to `2c` and stays ABSENT (ledger 19 + 3). **Budget: two free reductions taken FIRST** (the dead `CODEX_ORACLE`/`CLAUDE_ORACLE` consts; the in-`src` `host_grant_tests` relocation, which refunded 41 `maos-bin` lines AND made four CI-invisible tests run), then **three EXACT-MEASURED zero-headroom grants** — `maos-bin` 16260→16640, `xtask` 38655→38742, `maos-a2a-core` 4654→4669 — with `maos-iac` trimmed to land **GREEN at 6888/6888, no grant**. The standing D13/D14/D17 reds and the pre-existing `check-env-contract` / `check-service-boundary` / `maos-bench` reds are **attributed, verified byte-identical at `87eb6c37` in a clean `git archive` worktree, and NOT absorbed**. Every artifact claims *"the crossing is proven in two logs"* and nothing more: **no round trip, no signing, no reconciled bundle** — those are `2c`'s, whose preflight is INVALIDATED where it still expects the demo beat naming `"j1-crosshost-2"`. `check-kernel-baseline` PASS at **24472 = pinned, ZERO Δ**. §A6 is OWED and NON-DEGRADABLE. |
| 2026-08-17 | **PREFLIGHT ROUND-TABLE — the last two open forks closed unanimously per spec + long-term correctness, and the room disproved TWO of this file's own re-baseline findings in the process.** (Winston · Murat · Mary · Amelia · John · Paige · Sally; Vex, Grumbal, Dana and Yui walking on.) **(H13, NEW — the biggest find, and it dissolved a fight neither side could win.)** Vex and Dana deadlocked over whether D18's deadline was mis-pinned to this story. Measurement ended it by producing a third thing: `interpret_response` (`router.rs:892-1052`) types **nine of the sixteen** defined NACK codes and catch-alls the rest into `A2AError::TransportFailed` at `:1049` — and **`CODE_SPIRIT_RESTART_DETECTED` is not among the nine.** So host B's boot-nonce refusal, which **permanently** invalidates the pin, reaches the operator as a **transient network fault**, and the action it implies — retry — is the one action that can never work. It is **structurally unreachable in rung 1** (loopback stamps the `boot_nonce = 0` sentinel, so `router.rs:1123` never fires) and **becomes reachable exactly at this story.** Severity exceeds D18's: D18 makes a refusal illegible; this makes it *legible and wrong*. Fix is **≈4 lines, zero new types** — the existing `A2AError::PinInvalidated` already maps typed to `CrossHostPinMismatch` — landing as a `kloc.toml:87` correctness repair, with a **binding scope wall**: repair only the code this story makes reachable, record the 9-of-16 census, touch nothing else. New failure shape filed: **a defect that was unreachable, which the story that makes it reachable inherits** — the second instance in this same story, after `install_intake_sink` opening the duplicate-`frame_id` panic. *The story's deliverable is also its threat model.* **(T0 CLOSED.)** `2b` does **not** owe D18's repair — `map_a2a_error_to_iac_bus` is outbound-only and host B never calls it, so D18's shape is byte-identical before and after. It owes: don't extend the flattened error (which `iac_bus_types.rs:69-72` already marks **DEPRECATED**), correct the register's *mechanism*, leave the repair with 14-4/14-7. **(H8 CORRECTED against ourselves.)** This file claimed the `-32001` discriminator was dead. `router.rs:1688` **hardcodes** `CrossHostIntentDirection::Accept`, so the pair **is** distinguishable and the register's premise (a) is **upheld**; only `:1676` is dead. *A dead line inside a live discriminator is not a dead discriminator.* **(H1b — AC2.3 was MIS-specified, not under-specified.)** Sally's question ended the re-key design: after `2b`, running the loopback topology still lets the frame pick its own judge, so `loopback_from_host_unverified` stays **correctly `true`** and the ratified `true → false` assertion asserts something that must never happen — and no grep can judge a runtime fork anyway. Resolution: **split the fact** — keep the static loopback claim (permanently true, correct its prose), and derive the cross-host claim from AC1.6's **executed** two-daemon proof. **(Q3 RESOLVED — strip, then widen; no outage exists.)** `bilateral-2-host-mira-nash.toml`'s `host`-on-class-Spirit keys are **1a's own `priority_weight` defect in 1a's own file** — decorative, since ADR-003 puts the bilateral join in `A2APeerConfig` and `j4-mira-nash.toml` proves the pair loads without them. Strip 2 lines, *then* widen the loadability control past the `j1-founder-loop*` stem: it lands **green on day one**. Class-Spirit remote routing stays unbuilt. **(Q4 RESOLVED — SPLIT the demo beat.)** `two-host-delegation` for this story, `two-host-signed-run` re-owned to `2c`, funded by deleting the two compiler-dead gate constants. **(Q5 opened, unowned:** two topology files for the same Spirit pair.**)** All resolutions land as constraints on existing ACs — **still 4 ACs**, John's rule unbroken. |

| Date | Change |
|---|---|
| 2026-08-16 | **RE-BASELINED `5a921c0c` → `87eb6c37` and moved to `ready-for-dev`.** Both hard blocking conditions cleared (`2a` done at `0769869d`; `1b` done at `87eb6c37` with rung-1 evidence verified `PROVEN_BLOCKING` **by executing `demo-j1`**, not by reading a record). Six parallel scouts re-derived every claim at HEAD. **Load-bearing invariant found: `git diff 5a921c0c..HEAD` over `maos-a2a-tcp`, `maos-a2a-core` and `maos-iac` is EMPTY**, so G1/G3/G9/G10/G16 survive verbatim and the churn is confined to `maos-bin`, `maos-cli`, `xtask`, the workflow and `spirits/`. **Twelve re-baseline findings (H1-H12).** Headline: **(H1) SHIP-BLOCKER — the boundary leg `1b` built for this story cannot observe the flip in the shape AC2.1 mandates.** `verified_composed` is gated on `!loopback_composed` (`check_j1_loopback_delegation.rs:403`, 1b's own review patch P6), so a router *fork* inside `DelegationLeg::install` — which AC2.1 requires — leaves the Blocking gate GREEN while it publishes `loopback_from_host_unverified: true` over a verified wire; 1b's falsifier plants only the total-replacement shape. This re-creates P2's "tripwire that can never fire" **inside the leg written to repair it**. AC2.3 now owns the re-key and the fork-shape vector. **(H2) BUDGET: `xtask` went +223 → ZERO** (1b's §A6 grant 38609→38655, whose own record says the next `xtask` story cannot land one line without a grant — that is this story); `_aggregate` red deepened −885 → **−1154**; `kloc-check` is red on **three** keys, not four, because `maos-bin` is now GREEN. **(H3) AC3.4 became a NULL CONTROL**: 2a hoisted `if !completion.is_completed() { return Err(…) }` above `journal_completion`, so `"completed"` is the only reachable value and the old wording would thread a provable constant; the vocabulary also **doubled to six values**. Re-scoped to host-B journaling + the un-journaled verdict + `success_criteria`, with host A's hard-abort explicitly out of bounds (FR20/FR21). **(H4) G15 is DISPROVED** — `prepare_outbound` stamps the consent TTL (`router.rs:866-871`); 1b's review finding P11 caught it and the correction never reached this file, so AC4.1's second bullet and T10 would have built **a negative control for a non-gap**. Replaced with the real §D1 TRANSITIONAL posture. **(H5)** the inherited section's "the leg no longer needles `router.rs`" is false (1b review P9): the leg reads **both** doors. **(H6)** T1's premise is disproved twice — 2a already performed this relocation for `worker_cli`, and `worker_completion_2a.rs` already drives the worker-spawn surface end-to-end by subprocess — so T1 survives only as *in-process item visibility*, and the daemon region must NOT move. **(H7)** the "right substrate" `cohort_daemon_smoke_13_5c.rs` has **zero CI enrollment** across all eleven workflow files; its `Blocking` header is a comment. No multi-process daemon test runs in CI at HEAD. **(H8)** D18's premise (a) is a **total** null control — `IntentDirection::Accept` has zero construction sites anywhere, production or test — and both the register's ≈0 and this file's ≈+5 are unsound; the de-conflation is a net-0 substitution. Carried as **T0**. **(H9)** `kind = TaskComplete` is a **contaminated** oracle (7 bypass producers; `insert_kernel_event_returning_id` stamps it on every kernel event across 9 callers), so the two-log join must key on `frame_id` bytes. **(H10)** the ship-blocker fix has **no** precedent — `ConstraintViolation` appears zero times repo-wide. **(H11)** `bilateral-2-host-mira-nash.toml` already names this story as owner and nothing in this file mentioned it — now **Q3**. **(H12)** `J1_TEST_SUFFIXES` lacks `_2b.rs`; three synchronized `lay_green` trees; `judge()` panics on a leg-count change; every leg needs a `LegAudit`; two dead gate constants (a free `xtask` reduction); `check-env-contract` is RED and ownerless; ~90-line `main.rs` shifts. Also NEW: **a second intake-sink push site** (`router.rs:1279-1283`) and **`let _ = sink.send(…)` at both** (G1b); the **measured insertion point** for AC1.2 (`transport.rs:322-325`, beside `install_rupture_sink`, before `bind`); the `FrameKind` dispatch **precedent** at `delegation.rs:200-218` (this file previously claimed none existed); and the **`two-host-signed-run` demo beat is mis-owned** to this story with a CI-enrolled test pinning it (**Q4**). 4 ACs, 17 traps, 13 tasks; ZERO kernel-Δ re-verified GREEN at 24472. |
| 2026-08-16 | **Preflight round-table** (Winston · Murat · Amelia · John · Mary · Paige · Sally; Vex on the security read). **Three ACs collapsed into one test, and the story's headline finding was promoted to a SHIP-BLOCKER — both from the same measurement. (G16)** AC3.1/3.2/3.3 asked for a correlation token on `IacFrame`, a TL writer to carry it, and a join. All unnecessary: `deliver_typed` writes `Some(frame.frame_id)` — the **received** id — and `maos_audit::query` selects **`frame_id` as its FIRST column**, so the moment AC1.4 journals the inbound frame both logs share a key and both bundles already carry it. Cost avoided: a field in `maos-domain` (RED) and lines in `maos-audit`. **(AC3.2, promoted)** The same fact is a **remote-triggerable kernel halt**: `frame_id` is `BLOB NOT NULL PRIMARY KEY`, peer-supplied, deterministic, and a failed write is `panic!`. A peer that re-sends one frame halts host B — unreachable today **only** because the frame is ACKed and dropped, so **the single `install_intake_sink` call this story exists to add is what opens it.** **(Q1 resolved)** the return hop is deferred to `2c`, with a binding no-round-trip language constraint. |
| 2026-08-16 | **Created** at clean `5a921c0c` from a five-scout preflight, following the 2026-08-15 ratification of the `2a/2b/2c` split. **Fifteen premises disproved or corrected (G1-G15).** Headline: **(G1) the receiver is not missing — it authenticates, runs TOFU and consent, advances Lamport, ACKs `delivered: true`, and DROPS the frame**, because `intake_sink` is `None` and no `bind*` ever installs one; the whole story is one `install_intake_sink` call plus a consumer behind it, and the seam is already public under a doc comment that falsely calls it "test-only". **(G2)** the worker-spawn surface is `main.rs`-private. **(G3)** "duplicate-delivery safety" targets a hazard that does not exist — the transport is **at-most-once**. **(G4)** the boot-nonce handshake blocks a release-build two-host run. **(G5)** the substrate the shared preflight names is `#[ignore]` + live-Postgres + `TelemetryEvent` and never touches a Mailbox. **(G6)** the topology key allowlist is strict and two blocking controls pin the file. **(G7)** `main.rs:10678` is a smoke, not a composition path — but `impl A2ARouter for TcpA2ATransport` makes host A's side zero-code anyway. **(G8)** the fork belongs inside `DelegationLeg::install`. **(G9)** the return hop needs five parts and fails closed partially built. **(G10)** correlation is two missing mechanisms. **(G11)** the outcome vocabulary is thrown away at `delegation.rs:243`. **(G12)** D18's ≈0-line resolution does not re-measure. **(G13)** `check-a2a-sender-completeness` structurally excludes `delegation.rs`. **(G14)** four misleading comments including a live wire-format lie. **(G15)** the delegation consent envelope is a non-expiring bearer grant. *(G12 and G15 are superseded and disproved respectively by the 2026-08-16 re-baseline above.)* |
