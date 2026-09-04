---
baseline_commit: "**`53845402`**, and every line citation in this file is **HEAD-relative, verified with `git show HEAD:`** — not against the working tree. ⚠⚠ **THE WORKING TREE IS LIVE: a `14-2b` dev pass is landing INTO IT RIGHT NOW.** Measured 2026-09-02: `crates/maos-cohort/src/{lib,rotation,state}.rs` and `crates/maos-control/src/lib.rs` carry ~256 uncommitted insertions (`PeerConvergence`, `record_peer_convergence`, `peer_convergence`, `pin_generation`, `CONVERGENCE_*`, a REPLACED `Pull` arm). Two of those files appeared between two of my own commands, and one scout watched `self.grace` move `:982 → :1010` between two of its own greps. **NO LINE NUMBER IN `maos-cohort` OR `maos-control` IS STABLE UNTIL 14-2b COMMITS.** Measured `state.rs` drift HEAD→tree: `+0` below 89, `+55` to 118, `+65` to 185, `+66` to 384, `+146` above. `rotation.rs` `+28` after `:367`. **Re-derive against the landed 14-2b before acting.** Citations in `maos-a2a-core`, `maos-a2a-tcp`, `maos-bin` and `xtask` are UNAFFECTED — those files are untouched."
depends_on: "**`14-2b-cohort-convergence-observability`** — AC3's swap rule is evaluated against its per-peer convergence table, and there is no other source (measured: every per-peer map in the cohort/a2a crates is orthogonal to version). **IN FLIGHT, not committed.** · **`14-2c-plane-c-local-leaf-declaration`** — supplies the apply-time divergence detector this story acts on, the `A2ARouterCore` local-leaf GETTER, and the corrected recovery procedure. **NOT STARTED.**"
blocks: "Nothing. This is the last of the 3-way split and the close of `RELEASE-HOLDS` **(c.2)**."
split_from: "**`14-2b-self-identity-rotation`**, split 3 ways 2026-09-02 (Lunarpulse). Siblings: `14-2b-cohort-convergence-observability`, `14-2c-plane-c-local-leaf-declaration`. Pre-split forensics: `.archive/14-2b-pass-4-presplit-source.md`, `.archive/14-2b-pass-1-to-3.md`."
kernel_grant: "NONE and none needed — nothing here touches `crates/maos-kernel-core/src`. ⚠ `check-kernel-baseline` counts raw lines with no exclusions (`check_kernel_baseline.rs:109`, recursion at `:105`) and compares for EQUALITY (`:65`), so drift in EITHER direction reds; resolve the pin from `xtask/kernel-core-baseline.toml`, never restate it. ⚠ `check_cohort_mesh.rs:604-606` re-runs the same check, so a kernel drift reds TWO gates."
kloc_grant: "Measured at HEAD 2026-09-02 (`kloc-check --json`, EXIT 1 on two FOREIGN rows). ⚠ **RE-MEASURE AFTER 14-2b COMMITS — it is spending `maos-cohort` right now and this story's headroom is whatever it leaves.** At HEAD: **`maos-cohort` 5678/5713 = +35** · **`maos-bin` 16941/16971 = +30** · **`maos-a2a-core` 4849/4850 = +1** (ask ALREADY AUTHORIZED 2026-08-31 — a ceiling EDIT, not a re-ask) · **`maos-control` 418/1500 = +1082** · **`maos-a2a-tcp` 1439/1500 = +61** · `xtask` 41897/41932 = +35. ⚠ **THREE crates on this story's path are contended or exhausted**: `maos-cohort` is ALSO 14-2c's binding constraint (priced 55–70 against +35 by two validators) AND is being spent by 14-2b as you read this; `maos-bin` +30 must absorb the actuator; `maos-a2a-core` +1 must absorb the plane-C setter. **Expect measured grants on `maos-cohort` AND `maos-bin`; ask only after `cargo fmt --all` (`kloc.toml:60-65`), citing `kloc.toml:87` with the precedent at `:303`, NEVER `:269` (anti-masking).** ⚠ TWO FOREIGN reds, disown do not absorb: `maos-domain` −51 (D14, owner **14-7**), `_aggregate_hardfail` −7646 (D17, owner **14-6**). ⚠ ALL of `crates/*/tests/` is UNCHARGED, so every co-edit in §2 is budget-free."
model: "frontier-class allowlist {opus-4-6, opus-4-7, opus-4-8, gpt-5.5, gpt-5.6, glm-5.1, glm-5.2, glm-5.3, opus-5, equiv}. `FRONTIER_FAMILIES` (`xtask/src/check_dev_model_tier.rs:45-48`) is the machine-checked set; **`equiv` is prose, NOT a match token**. Keep the literal `allowlist {`."
review: "§A6 full-layer net (Blind + Edge Case + Acceptance + Test-Infra + non-author runtime), **non-degradable** — this is the epic's highest-risk story by its own sequencing note. **Security review is mandatory and named**: this story gives a signed manifest the power to change what certificate a live host presents, and `swap_serving_cert` validates NOTHING but chain-non-empty and key-matches-leaf (`transport.rs:530-561`). Every authorization control on that path is authored here."
---

# 14-2d — Self-identity rotation: the host follows its own signed fingerprint

Status: **blocked.**

> **Why `blocked` and not `ready-for-dev`, stated plainly:** this story's swap rule is evaluated
> against `14-2b`'s convergence table, which is **being written into this working tree right now and is
> not committed**, and it acts on `14-2c`'s divergence detector and local-leaf getter, which are **not
> started**. Its own citations are therefore stale at birth. The preflight work the sprint row demanded
> *before* dev — the port, both security hazards, the `G` co-edit — **is complete and is below**; what
> is missing is the substrate. **Unblock condition: `14-2b` and `14-2c` both `done`, then re-derive
> every `maos-cohort`/`maos-control` citation and re-measure the budget.**

> **The capability:** *when a signed manifest moves this host's own certificate fingerprint, the host
> follows it — swapping the leaf it actually serves, in place, inside the one instant when every peer
> will accept either generation — or refuses loudly and never swaps into a partition.*

---

## 1. The one-instant constraint, and why this story is hard

A peer `P` pins this host `A`. When a reissue moves `A`'s fingerprint `F_old → F_new`, `P` opens a
one-generation window and closes it **promote-and-retire** (`maos-a2a-core/src/tofu.rs:356-376`). So
`P`'s accepted set for `A` is:

| interval | `P` accepts from `A` | `A` must serve |
|---|---|---|
| before `T_P` | `{F_old}` | `F_old` |
| `[T_P, T_P + G_P]` | `{F_old, F_new}` | either |
| after `T_P + G_P` | `{F_new}` | `F_new` |

**`A` can only ever serve ONE of them.** `SwappableServingCert` is `RwLock<Arc<CertifiedKey>>`
(`transport.rs:1247-1258`) and `ResolvesServerCert::resolve` returns that single `Arc`
(`:1304-1313`); `swap_serving_cert` rotates **both halves at once** — serving resolver *and* dial
materials (`:530-561`). There is **no one-generation overlap available on the rotating host's own
side**, the way there is for every peer. That asymmetry is the entire design problem.

⇒ **`A` must switch at a single instant inside `⋂_P [T_P, T_P + G_P]`**, which is non-empty only when
`skew ≤ min_P G_P`.

⚠ **And `A` cannot act the moment a peer opens.** It learns that `P` applied version `N` only when `P`
reports `known_version ≥ N` on its **next** pull, one interval late: `R_P = T_P + I`. So the earliest
safe instant is `T_A = max(R_P) = max(T_P) + I`, and feasibility demands `T_A ≤ min(T_P) + G`:

> ### **`G ≥ skew + I`** — not `G ≥ skew`.
> And the falsifier is **`spread > G − I`**, not `spread > G`. ⚠ **`spread > G` is FALSE BY
> CONSTRUCTION** — `A` observes `spread(R_P) = spread(T_P + I) = spread(T_P) = skew`, so under
> `G ≥ skew` that predicate can never fire. **It is a null control against the dominant failure**, and
> writing it is the single defect that split this story. Do not restore it.

**There is no recovery if the instant is missed.** Verified three ways: `invalidate_if_boot_nonce_differs`
is reached only from `handle_intake_inner` (`router.rs:1358`) — i.e. *after* a frame is accepted, which
requires a successful handshake, which is precisely what has stopped; invalidation only **marks** a pin
(`tofu.rs:657-679`), it never re-learns; and the re-pin path `await_repin_consent` has **zero production
callers** and its only decision source is a `test_repin_hook` (`tofu.rs:613-620`). **A missed swap is a
permanent partition in both directions, unrecoverable in band, including by restart.**

## 2. 🔴 THE `G` RAISE IS FORCED, AND THERE IS NO EDIT THAT LEAVES BOTH TESTS GREEN

`cold_deployment_t_grace()` is **arity-zero** and resolves to **exactly 5 s** — `compute_t_grace(500, 0)`
= `max(2 × max(500,500), 5000)` (`maos-cohort/src/rotation.rs:183-188`,
`maos-a2a-core/src/chaos/rotation.rs:11-19`). With `I = 60 s` at the default, `G = 5 s` violates
`G ≥ skew + I` **always**. The ratified value is `G = cold_deployment_t_grace() + 2 × confirmation_interval()`.

**Two edits are conceivable and each reds a different Blocking leg:**

| Edit | Reds |
|---|---|
| Raise the **installed** grace at the call site | `crates/maos-bin/tests/cert_rotation_trigger_14_2a.rs:720-732` — it **recomputes** `cold_deployment_t_grace()` and budgets `acked_at + grace + 45 s = 50 s` against a **real daemon**; the close now lands at ~125 s. `TEST_FILES[0]`, leg `rotation-trigger-production-caller`, marker `ROTATION_WINDOW_CLOSED_OBSERVED=1`. |
| Change `cold_deployment_t_grace()` itself | `crates/maos-cohort/tests/cert_rotation_14_2a.rs:723-742` — `assert_eq!(cold_deployment_t_grace(), compute_t_grace(CONSTS))`, *"the grace is DERIVED through §7.2.1.a's shipped formula, never restated as a literal"*. `TEST_FILES[1]`, leg `reload-atomicity`. |

⇒ **RULING: compose `G` at the composition root (`main.rs:9796-9801`), leave the function untouched, and
CO-EDIT the trigger test.** Prior-pass *"it scales, so it stays green"* is **REFUTED**: the budget scales
with the **recomputed floor**, which does not move.
⚠ **CORRECTION — "no edit leaves both green" is FIXTURE-DEPENDENT, not universal.** Test 1's deadline is
`acked_at + cold(5 s) + 45 s = 50 s`, so **any installed `G` ≲ 45 s leaves both green**. It reds here only
because the fixture pins `t_stale_secs: 120` (`cert_rotation_trigger_14_2a.rs:144`) → `G = 126 s`. At the
signed minimum `t_stale_secs = 30`, `G = 35 s` and nothing reds. **Do not restate the universal.**

**Measured cost, not estimated.** The test is **5.22 s** today — *the 5.22 s IS the grace*. At `G ≈ 125 s`
it becomes **≈125.4 s test / ≈127 s wall**, a **+120 s step on one invocation**;
`check-cert-rotation-trigger` goes **25 s → ≈145 s** warm (40 invocations, strictly sequential). The CI
job (`.github/workflows/discipline.yml:402-416`) is **`timeout-minutes: 20`** — not an obvious breach, but
real erosion, and `Swatinem/rust-cache` keys on `Cargo.lock`, so lock churn puts a cold build and the new
+2 min on the same run. ⚠ **`network` is a DEFAULT feature of `maos-bin`**, so any developer's
`cargo test -p maos-bin` pays the +120 s too.

⚠ **One flake this FIXES:** `drive_a_signed_rotation` polls ≤30 s to observe the window *while open*; at
`G = 5 s` a stalled runner can miss it entirely. At 125 s that race closes.

## 3. 🔴 THE RATIFIED `G` HAS A HOLE NOBODY HAS NAMED: LIVE CADENCE vs FROZEN GRACE

`G` is **boot-frozen** — moved by value into `PeerCertRotation` (`rotation.rs:334`) behind a set-once
`OnceLock` (`state.rs:138`) and read at **exactly one site**, `rotation.rs:982`. But `I` is **live**:
`t_stale_secs` is inside the **signed canonical bytes** (`manifest.rs:246`), replaced wholesale on every
accepted reissue (`state.rs:514`), and the pull loop **re-reads `confirmation_interval()` every tick**
(`main.rs:10129`, `:10146-10147`).

> **So a signed reissue can raise `I` while no host's `G` can follow it.** `t_stale_secs ∈ [30, 3600]`
> (`manifest.rs:484-491`) ⇒ `I ∈ [15 s, 1800 s]` ⇒ `G ∈ [35 s, 3605 s]`. A manifest that moves
> `t_stale_secs` 120 → 3600 takes `I` to 1800 s against a `G` still frozen at 125 s: **`G ≥ skew + I` is
> silently violated and self-rotation becomes infeasible with no signal.** ⚠ And at the top of that range
> the *ratified* posture is a **one-hour accept-both window for a retired certificate** — a security
> posture nobody has ratified. **AC3 must rule on both.**

⚠ **W1 is RELOCATED HERE AND STILL NOT CLOSED**: feasibility turns on **each peer's own** `G_P`, and
`A` cannot observe it. Nothing on the wire carries it.

## 3b. ✅ RATIFIED 2026-09-02 (Lunarpulse), CORRECTED ON MEASUREMENT — `G = cold + I`, push-confirmed

**This SUPERSEDES the earlier `G = cold + 2 × I`.** Lunarpulse ratified *"the cheaper `G ≥ skew + RTT`
design, before anyone builds the expensive version."* ⚠ **The ratification is adopted; the validator's
arithmetic behind it was WRONG and is corrected here — read the correction before implementing.**

**`I` is a poll cadence, not a physical constant — and this host can drive a confirmation round itself.**
Measured at HEAD: `CohortDistributor::pull_from` (`distribution.rs:62-74`) makes the peer queue a pull
request, and `service_pending_pulls` → `push_to` (`:79-86`, `:46-57`) responds with
`Push { manifest_toml: self.state.signed_toml()? }` — its doc reads *"Send the exact signed artifact
previously verified by this member."* This host then verifies it under `PinnedAuthorityKeys` and reads the
version out of the **signed canonical bytes**.

That channel is **strictly better than the table on both axes**: it runs at **this host's** cadence, so
`max(R_P)` collapses from `max(T_P) + I` to `max(T_P) + RTT`; and it is **cryptographically
version-authenticated**, where `Pull.known_version` is a bare self-report (§4 Hazard 2).

🔴 **THE CORRECTION, AND IT IS THE WHOLE POINT.** The inequality is `G ≥ skew + C`, where `skew` is the
spread of peer apply instants and `C` is this host's confirmation latency. **The push round collapses `C`,
from `I` to RTT. It does NOT collapse `skew`, and nothing can.** Every host pulls **every** peer on each
tick (`main.rs:10119-10126` boot, `:10137-10148` renewal), so once any host holds version `N` the rest
apply within **one** interval: **`skew ≤ I`, set by the poll cadence.** A push round accelerates
*laggards*, but peers that already applied have `T_P` in the past and their windows are already running.
⇒ **`G ≥ I + RTT`, not `G ≥ RTT`.** RTT on this substrate is ≪ 5 s, so the shipped cold floor absorbs it.

> ### **RATIFIED VALUE: `G = cold_deployment_t_grace() + confirmation_interval()`** ≈ **65 s** at the
> default — derived through the shipped functions, never a literal, composed at the composition root.

**What the ratification buys, measured:** `G` roughly **halves, 125 s → ~65 s**; the CI step drops from
+120 s to **≈ +60 s**; and confirmation stops depending on a **self-reported** version, becoming
**cryptographically version-authenticated** — which is a security improvement, not just a cost one.
🔴 **What it does NOT buy — three claims that were relayed and are FALSE:** `cold_deployment_t_grace()`
does **not** stay 5 s; **both tests do NOT stay green** (the trigger test's budget is 50 s, so a ~65 s
close still reds it — **§2's co-edit stands in full**); and **§3's frozen-grace hole does NOT vanish**,
because `skew` still tracks the live `I`, so a signed reissue can still outgrow a boot-frozen `G` and
AC3's feasibility check remains load-bearing.
⚠ **Cost of the new wiring:** `push_to` has **no production caller** outside `service_pending_pulls`, so
the proactive push-with-ACK round is net-new. The ACK is real — `router.rs:1666-1680` ACKs on accept and
NACKs on reject, and `push_to` maps a NACK to `EDistributionFailed`.

## 4. Two security hazards. Both REAL. Both ruled.

**HAZARD 1 — `reachable` inferred from absence is a remotely-inducible premature swap.** An off-path
attacker who drops one peer's frames, compromising nothing, manufactures "unreachable"; a rule that then
swaps leaves that peer pinning `F_old` and refusing this host forever.
🔴 **RULING: NEVER infer. The codebase already forbids it in its own words** — `state.rs:669` (HEAD):
*"absence is never a trust decision."* Story 12.3 **P2 is ratified**: *"absence is a PROBE, and it must
dial the MISSING member … absence only exists as the return value of a probe"*, with the §A7 reflex
*"NEVER a bare `TransportFailed`, NEVER `PartitionTimeout`, NEVER a broadcast failure to a different
peer, **NEVER a silent gap**."*
⚠ **And do not read `halt_absence` either.** Measured at HEAD: it is **insert-only** — four sites, `state.rs:108`
declaration, `:181` init, `:671` insert, `:698` read. **No removal, no clear, no TTL, no timestamp.** One
transient dropped frame latches "absent" for the process lifetime.
⚠ The doctrine-compliant probe **exists and is unwired**: `classify_presence`
(`maos-cohort/src/halt_receipt.rs:228-241`), one dial, zero retry, **ZERO production callers**.
⚠ **Its PRESENT allowlist is now STALE**: `j1-crosshost-2c` typed `CODE_INTERNAL → PeerInternalFailure`
and `CODE_TIMEOUT → PeerIntakeTimeout` (`router.rs:1259-1275`), and neither is in the PRESENT arm
(`halt_receipt.rs:132-152`), so a peer that **provably answered** now classifies `Indeterminate`. Drift is
safe-direction, but any rule reading `Indeterminate` as "not reachable" inherits a false negative on every
NACK. **AC3 does not need reachability at all — see its ruling — which is the cheapest way to be right.**

**HAZARD 2 — a peer's version is a self-report, and lying LOW is undetectable in principle.**
`Pull { known_version, known_hash }` is authenticated as *from* that peer, never verified as *true*
(`control.rs:14-22`; both senders fill it from their own `self.state.version()`).
- **Lying HIGH is cryptographically bounded**: a pull makes the peer push its exact `signed_toml`, which
  this host verifies under `PinnedAuthorityKeys` and reads the version out of the **signed** bytes.
- **Lying LOW is not.** A genuinely-signed *older* manifest is byte-indistinguishable from "not yet
  applied". There is **no nonce, no freshness challenge, no signed *I-applied-N* attestation and no Merkle
  accumulator** anywhere in the tree; `CachedManifest` holds exactly one manifest, so there is no history
  oracle either.
🔴 **RULING, CORRECTED BY VALIDATION: the veto DISSOLVES, because AC3 is a DEADLINE and not a vote.**
A rule that waits for unanimity hands every member an unbounded veto; a rule that swaps by a deadline
gives a lying peer nothing to withhold. **The self-report is still not trusted — it is simply not
load-bearing.** What remains is declared: a peer that never applies is broken by the swap, and heals the
moment it does.
⚠ **CORRECTION — the "manifest-ADDED member can never confirm" amplifier cited the WRONG SIDE OF THE
WIRE.** `pull_peers` (`main.rs:10050-10053`) is **this host's** list — whom *it* pulls *from*.
Confirmation arrives on **inbound** `Pull` frames, governed by each **peer's** own boot config. So an
added member `M` **can** confirm, iff `M` was deployed with this host in `M`'s peer list. It is a
**deployment-ordering constraint, not an impossibility** — materially different, and it is the third time
in this lineage that a peer/self axis has been read backwards (see `feedback_measurement_wrong_axis`).
⚠ The other amplifier stands: `apply_reissue` never persists (no `fs::write` anywhere in
`crates/maos-cohort/src/`), so an honest restarted peer is observationally identical to a liar.

---

## Blocking conditions

1. **🔴 THE TREE IS MOVING AND THIS STORY IS BLOCKED ON IT.** `14-2b` is landing into `maos-cohort` and
   `maos-control` right now, and `14-2c` has not started. **Do not begin. Re-derive every citation and
   re-measure every ceiling against the landed result**, then flip this story out of `blocked`.

2. **🔴 A SIGNED MANIFEST GAINS THE POWER TO CHANGE WHAT CERTIFICATE A LIVE HOST PRESENTS, AND
   `swap_serving_cert` VALIDATES ALMOST NOTHING.** Measured (`transport.rs:530-561`): it checks **only**
   chain-non-empty and key-matches-leaf, via `SwappableServingCert::certified_key`. It does **NOT** check
   authorization, expiry/`not_before`/`not_after`, chain-to-root, subject/SAN, newer-than-outgoing, or
   **the manifest fingerprint**. A rollback to a retired leaf, or any self-consistent unrelated pair, is
   accepted. ⚠ **Nothing upstream checks either** — the only local-leaf-vs-manifest check in the tree is
   boot-only arm (c) (`main.rs:9987-10004`). **EVERY authorization control on this path is authored by
   this story.** This is the §A6 security-review anchor.

3. **⚠ ARM (c) FAILS OPEN AND READS THE WRONG ARTIFACT — both must be fixed before reuse.**
   `if let Some(signed_own)` (`main.rs:9989`) means a host **absent from the manifest silently skips the
   check**: defensible for a boot check, **wrong for a swap precondition**, where "I am not in the
   manifest" must be a refusal. And it loads from **disk** (`tcp_config.load_identity()`), not the live
   transport — after a swap those differ, so verbatim reuse compares a stale artifact.
   ⚠ **`signed_fingerprint` is a LOCAL CLOSURE** (`main.rs:9944-9950`) with **three** consumers —
   `:9956` (arm a), `:9973` (arm b), `:9989` (arm c). It cannot be `pub`. **Extract it to a free `fn`
   taking `&CohortManifest`; do NOT copy it**, or the three planes can disagree about what the manifest
   signs.

4. **⚠ THE ACTUATOR MUST DO THE DISK RE-READ ITSELF — nothing else can.** Measured: the bound
   `TcpA2ATransport` retains **no `TcpA2AConfig` and no paths** (full field list at `transport.rs:103-149`);
   there is **no file watcher** anywhere in the workspace, and the **only** `SignalKind` in the tree is
   `terminate()` at `main.rs:8800` — **no SIGHUP**. The operator surface is read-only by ratified design.
   ⇒ **Capture `bootstrap.tcp.clone()` at `main.rs:9765`**, one line beside the existing
   `Arc::clone(&bootstrap.state)`. ⚠ **`main.rs:10080` is NOT an option** — it is inside
   `build_cohort_a2a_daemon_runtime` (starts `:10024`) while the consumer `install_cert_rotation` is at
   `:9796` inside `run_cohort_a2a_daemon` (`:9627-9871`). **Different functions; a capture there cannot
   reach it.**

5. **⚠ THE SWAP MUST NOT RUN UNDER THE `cached` GUARD, AND `maos-cohort` HAS A LOCK-ORDER DOC THAT THIS
   STORY FALSIFIES.** `apply_reissue` holds `cached` from `state.rs:402` to `:540`; `load_identity()` is
   **blocking `std::fs` I/O** and the caller is an async intake path (`router.rs:1670`). An in-guard swap
   is blocking disk I/O on a tokio worker with the whole cohort consent path queued behind it.
   ⇒ **Invoke OUTSIDE the guard**, at the outer `CohortManifestGate::apply_reissue` (`state.rs:867-900`),
   after the inner call returns and the guard has dropped.
   🔴 **AND CO-EDIT `crates/maos-cohort/src/rotation.rs:129-138`.** That block is the project's lock order
   *"in one place"* and it says: *"`TcpA2ATransport::swap_lock` is **NOT** in this order because this story
   never takes it: the local leaf is `14-2b`'s."* **This story makes both halves false** — it takes
   `swap_lock`, and `14-2b` is a deleted key. ⚠ It is the **ninth** dangling reference to that key and the
   **only one in source**; an exact-key grep misses it because the comment uses the short form.

6. **⚠ THERE IS NO WAY TO READ THE INSTALLED GRACE, AND THE OBVIOUS WORKAROUND IS THE DEFECT.**
   `PeerCertRotation.grace` is private, `CohortManifestState` exposes no accessor, and neither
   `RotationWindow` (`rotation.rs:232-266`) nor `maos_control::RotationWindowRow` carries a grace field.
   Having the co-edited test re-derive `cold_deployment_t_grace() + 2 * state.confirmation_interval()?`
   costs zero production lines — **and is a SECOND SOURCE for `G`, exactly what
   `t_grace_matches_the_shipped_formula` exists to forbid.**
   🔴 **RULING: add the accessor — but know it does NOT close AC5's co-edit.** It is justified on its own
   terms, because **AC3's feasibility check needs the installed `G` in production** (§3). ⚠ **It is
   nevertheless UNREACHABLE from the co-edited test**, which drives a **real spawned daemon** and reads it
   over HTTP (`cert_rotation_trigger_14_2a.rs:383-750`, `operator_get(... "/v1/a2a/rotation-windows")`).
   A Rust accessor on `CohortManifestState` cannot be called from another process, and
   `maos_control::RotationWindowRow` has **no grace field**. ⇒ **Sourcing the installed grace in that test
   requires a `maos-control` SCHEMA DELTA that this story must budget and name.** Price it at T4.

7. **⚠ THE DEADLOCK IS REAL BUT ITS SCOPE IS NARROWER THAN FILED — state it correctly.** Calling
   `confirmation_interval()` **synchronously at `rotation.rs:982`** self-deadlocks: it re-takes
   `self.cached` (`state.rs:614-620`), the same non-reentrant `std::sync::Mutex` already held from
   `state.rs:402`. ⚠ **But that wiring does not exist today** — `PeerCertRotation` holds **no handle to
   `CohortManifestState` at all** (8 fields: `pins, core, audit, timer, grace, transition_lock, ledger,
   instances`), so the call is not even constructible. **Two reads that ARE safe and are what this story
   uses:** at the **composition root** (`main.rs:9796`, argument evaluation completes before
   `install_cert_rotation` takes any lock — `?` already converts `CohortError` there), and **inside the
   expiry closure** (the timer contract at `rotation.rs:207-210` forbids inline execution and
   `TokioGraceTimer` spawns).

8. **⚠ SCOPE ARMS — DECLARED, NOT DISCOVERED. A MIXED MESH BRICKS SILENTLY AND ASYMMETRICALLY.**
   The rotating host must be on the **cohort-a2a-daemon arm** (`maos run` coerces to
   `Arc<dyn A2ARouter>` at `main.rs:2484` — a **one-method** trait with no `as_any`, so `pins()`,
   `core()` and `swap_serving_cert()` are irrecoverable). ⚠ **And so must every peer**: the `maos run` arm
   binds with **no `CohortManifestGate` and no refresh loop** (`main.rs:2457-2465`), so such a peer
   (a) never pulls → never confirms → stalls the swap forever, (b) can never receive the manifest at all
   (`LegacyCohortManifestGate::apply_reissue` returns *"cohort manifest control is not configured"*,
   `cohort.rs:216-226`), and (c) **refuses this host after the swap** while this host can still reach it —
   **asymmetric, which is the shape that hides longest.** (d) And its `CODE_INTERNAL` NACK classifies
   `Indeterminate`, so the probe cannot tell it from a broken peer.

9. **⚠ `all gates green` IS NOT AN AVAILABLE DONE CRITERION.** `kloc-check` is **Blocking** in CI
   (`discipline.yml:154`, no `continue-on-error`) and **RED at HEAD on two FOREIGN keys** —
   `maos-domain` −51 (**14-7**) and `_aggregate_hardfail` −7646 (**14-6**); `kloc.toml:458` allows the
   aggregate to be recalculated **only at an epic retrospective**. `check-dev-record-completeness` is RED
   with exactly one violation, `deferred-work.md:860` STALE owner `14-1`, whose own text names the
   successor **14-6**. GREEN at HEAD and yours to keep: `check-kernel-baseline`, `check-cohort-mesh` (35
   legs), `check-cert-rotation-trigger` (4 legs, 39 enrolled). ⚠ Three more Blocking gates needle files
   this story edits: `check-j1-two-host-signed-run` (**both** `router.rs` and `maos-cohort/src/state.rs`),
   `check-j1-loopback-delegation` (`router.rs`), and `check-env-contract` (**blocking at all four phases**,
   scans `maos-bin/src` for unregistered `MAOS_*` reads — **add no env read**).

---

## Acceptance Criteria (6)

**AC1 — `G` is composed at the composition root, and the shipped formula is not touched.**
`G = cold_deployment_t_grace() + confirmation_interval()` (**RATIFIED §3b — the `2 ×` form is
SUPERSEDED**), evaluated at `main.rs:9796-9801` as the 4th
argument of `install_cert_rotation`, **derived through the shipped functions and never restated as a
literal**. Do **not** modify `cold_deployment_t_grace()` (§2 — a Blocking equality leg pins it), and do
**not** read `confirmation_interval()` at `rotation.rs:982` (Blocking 7 — self-deadlock).
Add the **installed-grace accessor** AC3 needs (Blocking 6).
🔴 **Proven-red:** revert the composition to the bare cold floor → AC5's feasibility leg reds.

**AC2 — The port: the decider names a fingerprint; the actuator owns the bytes.**
A new port trait in `maos-cohort`, modelled on the shipped `RotationGraceTimer` (`rotation.rs:190-211`) —
injected, `Arc<dyn …>`, installed as a **5th parameter** on `install_cert_rotation` (**10** call sites, not 7 — 3 in
`maos-bin/tests/cert_rotation_trigger_14_2a.rs`, 7 in `maos-cohort/tests/cert_rotation_14_2a.rs`, all
kloc-free; verified safe:
`check-cert-rotation-trigger` pins method **names** only, `check_cert_rotation_trigger.rs:87-89`, and its
own unit test feeds the probe a **one-argument** call, `check_cert_rotation_trigger_tests.rs:56` — **no
arity assertion in 945 lines**). Ratified signature shape:
```rust
fn reconcile_local_identity(&self, declared: &PeerCertFingerprint) -> LocalIdentityOutcome;
```
🔴 **Rulings, each with the alternative it kills.** **(a) Fingerprint, not bytes** — `maos-cohort` holds
**zero** certificate material and cannot name `CertificateDer`/`PrivateKeyDer` (no `rustls`, and
`maos-a2a-core` has **no `pub use rustls`**, so it cannot path through). A PEM-bytes port would force
`std::fs` into the decider. **(b) TYPED `&PeerCertFingerprint`, not `&str`** — a raw-string port bypasses
`PeerCertFingerprint::parse`, which lowercases (`identity.rs:73-85`) while `CohortMember.fingerprint` is a
raw `String` (`manifest.rs:141`); `rotation.rs:441-451` is emphatic that *"a fingerprint that did NOT go
through `parse` can silently never match."* **(c) A typed outcome, not `Result`** — a local refusal must
not reject a signed manifest every other member already accepted.
🔴 **THE PORT MUST FIRE ON `Confirmed` AS WELL AS `Applied`, AND ON THE BOOT RECONCILE — a one-shot at
apply time is a PERMANENT NO-OP.** At the instant this host applies version `N`, **zero peers can have
confirmed `N`** (§1: `R_P = T_P + I`), so a single evaluation there always refuses and never runs again.
The re-entry channel exists and is measured: redelivery at the same version returns
`ReissueOutcome::Confirmed` (`state.rs:453-457`) and **still flows through the outer gate**
(`state.rs:874`), and it recurs every interval because `pull_from` → `service_pending_pulls` → `push_to`
re-pushes the full `signed_toml` unconditionally (`distribution.rs:79-86`). **Wire the port to both
outcomes.** ⚠ **And to the boot reconcile** (`state.rs:243-265`): that path calls
`rotation.reload(&peers, …)`, and `peer_configs_for` **excludes self**, so a local-fingerprint move that
lands in the install window is reconciled by nothing at all — the same two-site defect `14-2c` names, on
this story's axis. ⚠ Latent third path: `issue_reissue` (`state.rs:547`) calls the **inherent**
`apply_reissue` directly and bypasses the outer gate entirely. Zero production callers today; **the outer
site is therefore the production chokepoint, not the only one. Do not claim otherwise.**
The actuator lives in **`crates/maos-bin/src/cert_rotation.rs`** — measured constructible: it is
`#[cfg(feature = "network")]` and `maos-a2a-tcp` is on the **same** feature, so **no new crate edge**. It
re-reads via `load_identity()`, hashes the leaf with `PeerCertFingerprint::from_cert_der`, **refuses
unless it equals the signed value**, then calls `swap_serving_cert`.
⚠ **Retain the outgoing chain/key** — `dial_materials.snapshot()` and `SwappableServingCert` are private,
so after the swap the previous materials are **unreadable**; without a retained copy there is no abort.

**AC3 — The swap decision is a DEADLINE, probe-free, and feasibility-checked.**
🔴 **RULING INVERTED BY VALIDATION — READ THIS BEFORE AC3.** An earlier draft said *"swap only when every
peer has confirmed; otherwise refuse."* **That is wrong in the dangerous direction, and the measurement is
decisive:** each peer's window close is **unconditional** — `schedule_close` runs `close_rotation_window`
after `grace` elapses regardless of what this host did (`rotation.rs:974-983` → `tofu.rs:356-376`,
promote-and-**retire**). So **refusing does not preserve the status quo, it guarantees the partition:**
this host keeps serving `F_old` while every peer that already applied retires it, in both directions, with
no in-band recovery (§1). **Swapping with a straggler outstanding breaks ONE peer and SELF-HEALS** the
moment that peer applies and opens its own window. Refusal is not fail-closed; it is **fail-partitioned**.
⇒ **Swap by a deadline** — at `min_P(T_P) + G − ε`, using confirmations to swap EARLY when they arrive,
never to withhold the swap indefinitely. Refuse **only** for a correctness reason (the leaf on disk does
not hash to the signed value; this host is absent from the manifest; the feasibility check below fails),
**never merely because a peer is quiet.**
🔴 **Never infer reachability from absence, and never read `halt_absence`** (§4 Hazard 1 — the codebase's
own words are *"absence is never a trust decision"*, and that table is insert-only with no TTL). **This AC
needs no probe and no reachability at all**: anything short of full confirmation is a **REFUSAL**.
🔴 **Confirm by PUSH, not by waiting** (§3b, ratified): after applying, push the signed manifest to every
peer and collect ACKs — `C` becomes RTT and the confirmation is authenticated by the peer's own signed
artifact, not by its self-report. **The `14-2b` table remains the fallback and the operator surface**, but
it is no longer the timing path.
🔴 **Feasibility check, in production, before swapping** (§3): refuse when
`cold_deployment_t_grace() + max(I_prev, I_now) > installed_grace`. ⚠ **`I_now` alone is blind in the
direction that bites** — it reduces to `I_now > I_boot` and fires only on GROWTH, while the wait that
matters is bounded by the peers' **in-flight** deadline, armed with the **previous** interval
(`main.rs:10147` re-arms only after the current sleep fires). A reissue that raises `t_stale` and a second
that lowers it back while moving this host's fingerprint would pass an `I_now` check with peers still
asleep on the old, larger deadline — i.e. when a signed reissue
has raised `I` past what this host's boot-frozen `G` can cover. **This is the only defence against the
live-cadence/frozen-grace hole, and it also disposes of the `[35 s, 3605 s]` range: at the top of it the
host simply refuses, loudly.**
🔴 **The falsifier is `spread > G − I`. `spread > G` is FALSE BY CONSTRUCTION** (§1) — do not write it.
Every refusal writes a named row. ⚠ `CohortAuditEvent::CertRotationRefused { peer, reason, version }`
already ships and is already written with `peer = local_host` on two paths, so **the self axis needs no
schema change**; the swap *itself* needs a new variant (reusing `CertRotationWindowOpened/Closed` would
make a local swap indistinguishable from a plane-B peer window). ⚠ **Nothing today records a decision to
WAIT** — the unconfirmed set and the evaluated deadline have no audit surface, and this AC creates one.
⚠ Fingerprints in rows are **8-hex** via the shipped `fingerprint_short` (`audit.rs:57-60`) or
`PeerCertFingerprint::short()` (`identity.rs:88-91`) — the I2 redactor eats any hex run ≥ 32
(`redaction.rs:140`, `:169-181`).

**AC4 — Plane C moves with the leaf IN A MANDATED ORDER, and the lock order is extended.**
🔴 **"Atomically" is NOT ACHIEVABLE and an earlier draft claimed it.** `swap_serving_cert` takes
`TcpA2ATransport::swap_lock` (`transport.rs:541-561`, field `:112`); plane C lives in `maos-a2a-core`
(`router.rs:173`). **Two locks, two crates, no shared lock exists** — `PeerCertRotation.transition_lock`
covers neither. A reader can always observe the transient, so the design decision is **which transient**.
⇒ **MANDATED ORDER: `swap_serving_cert` FIRST, then the plane-C write, under one actuator-owned mutex.**
Serve-`F_new`/declare-`F_old` means this host cannot originate a crossing — **loud, local, attributable,
and it clears on the next line**. The reverse transient, declare-`F_new`/serve-`F_old`, is `14-2c`'s
**undetectable lie**. A developer given "atomically" picks wrong half the time.
A host that serves
`F_new` while declaring `F_old` cannot originate a crossing; one that declares `F_new` while serving
`F_old` holds an **undetectable lie** (`14-2c` Blocking 2). This story owns the **setter** `14-2c`
deliberately declined to build.
⚠ **F3 is ratified and its cost is measured — do not re-derive it.** `std::sync::RwLock`: 4 lines changed
1-for-1 (`router.rs:173` decl, `:223` init, `:256` builder body, `:943` read) + 1 read binding + a 3–5 line
setter = **net +4…+6**. The consuming builder **signature does not change**, so `transport.rs:395` and
`team_identity_13_6a.rs:282` need **zero edits**. 🔴 **Use `.clone()`, never a held `RwLockReadGuard`** —
`prepare_outbound` is `async` with an `.await` at `router.rs:1018` and `#[async_trait]` requires a `Send`
future. `std::sync::Mutex` is already used fully-qualified in that file, so **no new `use` line**.
⚠ `arc_swap` is in `Cargo.lock` already but **not** a `maos-a2a-core` manifest dep; `RwLock` is the
`transport.rs` idiom (`SwappableServingCert`, `SwappableDialMaterials`). ⚠ `:943` is **per-outbound-frame
hot** — a clone costs two `String` allocations per frame; say so.
🔴 **Co-edit `rotation.rs:129-138`** (Blocking 5): add `swap_lock` to the ratified lock order and re-point
the dead `14-2b` reference.

**AC5 — Proven on a real mesh, with the co-edits executed and both falsifiers fired.**
A live multi-host test in which a signed reissue moves the **local** host's own fingerprint, the host
swaps inside the window, and **conversations do not drop**. Plus the negatives: a leaf on disk that does not hash to the signed value → **refusal**; a host absent
from the manifest → **refusal** (Blocking 3); the feasibility check failing → **refusal** (AC3). ⚠ **Do
NOT write "an unconfirmed peer → refusal"** — AC3's inverted ruling makes that the wrong assertion; the
straggler negative is *"a straggler does not withhold the swap past the deadline."*
🔴 **A RESTART-AFTER-SWAP LEG IS REQUIRED, and this story CREATES the hazard it must cover.**
`apply_reissue` never persists (no `fs::write` in `crates/maos-cohort/src/`) and boot re-reads the
on-disk manifest (`main.rs:9179-9194`). After a successful swap: disk leaf `F_new`, disk manifest still
naming `F_old` → arm (c) `Err` → **the daemon HARD-FAILS its next boot.** Before this story that was
latent; this story makes the operator's disk write a **required** step, so the procedure and the leg must
both exist. **Do not restate "the restart revert is unchanged" — it is not.**
⚠ **Write the drop oracle PER-HANDSHAKE, not per-session.** `dial_once` (`transport.rs:635-646`) opens a
**fresh `TcpStream::connect` + handshake per frame** — there is no connection pool, and rustls never
renegotiates, so established connections are untouched by design. A per-session oracle would prove
nothing.
⚠ **`client_config()` GOES STALE AND WILL PRODUCE A FALSE RED.** Production dials rebuild per call via
`scoped_client_config` (`transport.rs:609-620`), so the production path is correct — but
`self.client_config` (`:119`, built once at `:434`) is **never updated** by the swap, and
`client_config()` (`:565`) is public and used by tests to craft raw mTLS connections. **Any negative built
on it presents `F_old` after the swap.**
🔴 **Execute the §2 co-edits and state the new wall-clock cost**: `cert_rotation_trigger_14_2a.rs`
`:720` (source the **installed** grace, not the recomputed floor), `:725` (the 45 s slop must **scale**),
`:727-732` and `:702-703`/`:715` (message and doc text), `:742-750` (the early-fire floor must use the same
installed value or it stops detecting a wrong grace). Raise `discipline.yml:404` `timeout-minutes`
20 → 25–30. Fix the second-source restatement at `t_10_4b_rotation_real_timing.rs:715-716`, which would
otherwise publish `T_grace = 5000 ms` in a drill report while production installs 125 000 ms.
New files **`t_14_2d_*`** — verified safe against both prefix channels
(`check_rotation_real_timing.rs:354-355` scopes `t_10_4b_rotation_` / `t_14_2_`; `t_14_2d_` matches
neither, 7th char is `d`). Test lanes are kloc-free.

**AC6 — Gate named and registered, budget measured, and (c.2) actually closed.**
Name the gate and **register it in every mechanical place** — the pre-split story had none. Prefer a leg
on **`check-cohort-mesh`**: `struct Leg` at `check_cohort_mesh.rs:11-14`, container `let legs = [ … ];`
`:62-600` with an **inferred length and no `legs.len()` pin**, so a leg is a **one-line append**.
🔴 `run_leg` (`:29-34`) rejects a leg whose transcript lacks **either** `"running 1 test"` **or**
`"1 passed"` — **exactly one test per leg, via `-- --exact`**; and `build_journey_daemon()` (`:37-51`)
runs `cargo build -p maos-bin` first, so a compile break reds everything before any leg.
Then `cargo fmt --all`, measure the **formatted** tree, record **EXACT MEASURED / ZERO HEADROOM** per
crate, and only then ask for the `maos-cohort` and `maos-bin` grants (frontmatter). **Close
`RELEASE-HOLDS` (c.2)** — `swap_serving_cert` now has a production caller — and re-point **(c.7)**'s self
half. ⚠ **Do not claim (c.4)**: post-promotion irreversibility and the restart revert are unchanged.
**ZERO kernel-Δ.**

---

## Declared cut lines

- **W1 is NOT CLOSED, only relocated a second time.** Feasibility turns on **each peer's own** boot-frozen
  `G_P`, and nothing on the wire carries it. AC3's check defends only against *this* host's `G` being
  outgrown. **A peer booted with a smaller `G_P` is undetectable and will retire `F_old` early.**
- **The single-member veto stands** (§4 Hazard 2). Declared, audited, not engineered around.
- **The 12.3 probe stays unwired.** AC3 is designed to need no reachability, so `classify_presence` keeps
  its zero production callers — **and its stale PRESENT allowlist** (`CODE_INTERNAL`/`CODE_TIMEOUT` now
  falling to `Indeterminate`) is a **filed defect with no owner**. Name one.
- **A REFUSED SELF-SWAP HAS NO OPERATOR READ SURFACE OF ITS OWN.** `RotationWindowRow` is peer-keyed and
  `rotation_status()` projects `peer_configs_for`, which excludes self, so AC3's refusal is
  Transparency-Log-only — the exact failure this story exists to make loud is invisible on the operator
  HTTP surface. **`14-2c` AC3 adds the self-identity key that carries it; this story consumes that and
  must not re-invent it.** Named here so it is not discovered later.
- **`maos run` peers are out of scope and a mixed mesh is unsupported** (Blocking 8).
- **The Send-seam refusal keeps its uninformative message** — `router.rs:995-1007` hard-codes
  `claimed_team: None, declared: None`, and `main.rs:10416` conflates two causes into one
  `"team_identity_mismatch"`. Inherited from `14-2c` with this story as owner; **not fixed here.**

## Dev notes

**Placement.** Decision logic + port + audit variant in `maos-cohort`; actuator in
`maos-bin/src/cert_rotation.rs` (102 lines today, no `#[cfg(test)]` module, composition root
`main.rs:2757-2765`); plane-C setter in `maos-a2a-core`. **Extend `cert_rotation.rs`; do not add a file to
`crates/maos-bin/src/`** — `cohort_daemon_smoke_13_5c.rs:863`/`:937-941` assert
`SCANNED_SOURCE_FILES.len()` equals a live flat `read_dir` (**17 == 17**), and adding one is a **three-part**
edit (bump `; 17]`, add the `include_str!` tuple, contain zero `manifest_scopes`).

**Idioms to mirror.** `RotationGraceTimer` → `TokioGraceTimer` (`rotation.rs:190-211` →
`cert_rotation.rs:49-58`, **8 code lines**) is the exact injection shape; `schedule_close`
(`rotation.rs:974-983`) is the exact deferral shape — clone `Arc`s into a `Box<dyn FnOnce() + Send>` so
the work runs outside the guard. `PeerCertRotation::new` has **exactly one** call site
(`state.rs:214`), so a field there costs 3 lines and zero call-site churn — but it is peer-scoped by name
and doc, and buys no reach the 5th parameter lacks.

**`CohortError` has 51 variants, no `#[non_exhaustive]`, and ZERO exhaustive matches workspace-wide** —
a new variant costs ~4 lines and breaks nothing. ⚠ Do **not** reuse `EAuditAppendFailed` for a refusal:
`install_cert_rotation` already abuses it for its set-once refusal (`state.rs:220-225`), and repeating
that is the same defect one story later. `CohortAuditEvent` is matched exhaustively in **exactly one**
place (`audit.rs:179`) and costs **zero** lines in `maos-audit`/`maos-iac` (both at ZERO headroom).

**⚠ THREE exact-count source gates fence `router.rs`**, in `crates/maos-a2a-core/tests/` — **not**
`maos-a2a-tcp/tests/`, which has none — and there are three, not two:
`cohort_manifest_chokepoint_12_1.rs`, `cohort_consent_chokepoint_12_2.rs`,
`cohort_halt_receipt_chokepoint_12_3.rs`. The `== 2` is on the **qualified receiver**
`cohort_manifest_gate.consent_and_team(`; bare `consent_and_team(` is **3**. Do not conflate.

**⚠ FOUR STALE CITATIONS in the 14-2a artifact — do not inherit them.** `swap_serving_cert` is
`transport.rs:541` (cited `:518`); the accessors are `:526`/`:515` (cited `:502-505`);
`reconcile_transport_identity_with_manifest` is `main.rs:9934` (cited `:9855`); and `rotation.rs`'s own
doc cites `main.rs:9917`.

**⚠ A leftover git worktree doubles grep hits** — `.claude/worktrees/agent-a9a6d76f71ebb443b` holds a full
copy of `crates/`. Gitignored, so it does not affect `kloc-check`, but exclude `.claude` explicitly.

**Do not touch:** `maos-domain` (−51, 14-7), `maos-audit` / `maos-iac` (both ZERO), `maos-kernel-core`.

## Tasks / Subtasks

- [ ] **T0 — DO NOT START.** Confirm `14-2b` and `14-2c` are `done` and committed, then re-derive every
      `maos-cohort`/`maos-control` citation and re-run `kloc-check`. Flip out of `blocked` only then.
- [ ] T1 — AC2: the port trait + the 5th install parameter (7 test call-sites, all kloc-free).
- [ ] T2 — AC2: the actuator in `cert_rotation.rs` + the `bootstrap.tcp.clone()` capture at `main.rs:9765`;
      extract `signed_fingerprint` to a free `fn` (**export, do not copy** — three consumers).
- [ ] T3 — AC4: the plane-C setter (F3, `.clone()` not a guard) + the `rotation.rs:129-138` lock-order
      co-edit.
- [ ] T4 — AC1: compose `G` at `main.rs:9796-9801` + the installed-grace accessor + **price the
      `maos-control` schema delta the out-of-process co-edited test needs** (Blocking 6).
- [ ] T5 — AC3: the **deadline** rule (NOT a unanimity vote), the `max(I_prev, I_now)` feasibility check,
      the audit variants, `spread > G − I`.
- [ ] T6 — AC5: `t_14_2d_*` live mesh test, per-handshake drop oracle, the correctness negatives, **the
      restart-after-swap leg**, **the §2 co-edits executed**, `discipline.yml` timeout. Avoid
      `client_config()` — it goes stale after the swap.
- [ ] T7 — AC6: leg append (`-- --exact`), both proven-red vectors EXECUTED, `RELEASE-HOLDS` (c.2) closed.
- [ ] T8 — `cargo fmt --all`, measure, record EXACT MEASURED / ZERO HEADROOM, then ask for the
      `maos-cohort` + `maos-bin` grants. Disown the foreign reds with owners.

## Dev Agent Record

### Agent Model Used
_(record the exact model id from the frontmatter allowlist — `equiv` is prose and reds a Blocking gate)_

### Debug Log References

### Completion Notes List

### File List

### Change Log
