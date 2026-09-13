---
baseline_commit: "`53845402` — HEAD of `main`, tree **DIRTY** in docs only (`sprint-status.yaml`, `epics/epic-14-*.md` modified; `_bmad-output/planning-artifacts/model-pin-currency-correction-input-2026-08-31.md` and `maos/` untracked). ⚠ **`53845402`, NOT `ee3422d0`.** The 14-2a commit is `ee3422d0`; `53845402` (\"maos hello sprit model fix\") lands three one-line model-pin edits on top of it (`crates/maos-bin/src/main.rs`, `crates/maos-spirit-hello/src/lib.rs`, `spirits/hello-spirit/manifest.toml`) and has **zero** budget or gate impact. Every `file:line` below was measured on 2026-08-31 by SIX parallel read-only scouts and every claim this story acts on was re-verified by hand. ⚠ **Do NOT inherit 14-2a's frontmatter line refs.** 14-2a moved `transport.rs` by +200 and `main.rs` by +107; its own cited pairs `transport.rs:368-370`, `transport.rs:518-538`, `router.rs:1641-1645`, `main.rs:2663`, `main.rs:10000`, `state.rs:260` are **all stale at HEAD** (see Measured grounding). ⚠ **One scout's git worktree was created at `38c52811` — four commits behind, missing 14-2a's entire 9,391-line diff.** It was re-pointed before measuring. Any future scout dispatched with `isolation: worktree` MUST verify `git log -1` before trusting a number."
depends_on: "**`14-2a-production-mtls-rotation-trigger`** (done, `ee3422d0`) — built the peer half this story is the mirror of: `PeerCertRotation` (`crates/maos-cohort/src/rotation.rs:329`), its `reload` write entry (`:473`) and `status` read entry (`:388`), the set-once `CohortManifestState::install_cert_rotation` (`crates/maos-cohort/src/state.rs:206`), the production install (`crates/maos-bin/src/main.rs:9796-9801`), the `TokioGraceTimer` (`crates/maos-bin/src/cert_rotation.rs:49`), the read-only `GET /v1/a2a/rotation-windows` (`crates/maos-control/src/lib.rs:206-253`) and the Blocking gate `check-cert-rotation-trigger` (`xtask/src/check_cert_rotation_trigger.rs`, 4 legs / 39 derived invocations). **`13-6a`** (done, `a414f922`) — authored BOTH halves of this story's live defect: `A2ARouterCore::with_local_leaf_fingerprint` (`crates/maos-a2a-core/src/router.rs:255`) and `CohortManifestState::team_declaration_at` (`crates/maos-cohort/src/state.rs:806`), and wrote the boot-only fingerprint-equality check `reconcile_transport_identity_with_manifest` (`crates/maos-bin/src/main.rs:9934`) whose arm (c) IS this story's ratified repair, already shipping. **`12-1`** (done, `8393b957`) — authored `CohortManifestState::apply_reissue` and `CohortManifest::peer_configs_for` (`crates/maos-cohort/src/manifest.rs:568`), whose self-exclusion at `:578-582` is why the local host is invisible to 14-2a."
blocks: "Nothing in Epic 14 by dependency. It closes the SECOND half of `RELEASE-HOLDS.md` clause (c) — specifically **(c.2)** (`swap_serving_cert` has no production caller) and **(c.6)** (the `cert_post_grace_reject` label is not machine-readable, owner named as this story). (c.1), (c.3) and (c.5) survive untouched; **(c.5) is EXTENDED, not closed**; (c.4) is inherited and needs a split statement (the leaf half is restart-STABLE where the pin half is not). ⚠ **`RELEASE-HOLDS.md` says \"Six boundaries\" and carries SEVEN `c.N` labels** — (c.7) PROVISION BEFORE REISSUE is mis-nested at the outer bullet level and self-assigns to 14-2a. Structural doc defect; fix the count in this story's doc pass, do not adopt the boundary."
kernel_grant: "NONE and none needed. `check-kernel-baseline` GREEN at **24472 == 24472** (measured 2026-08-31, EXIT=0), resolved from `xtask/kernel-core-baseline.toml:472` (`src_lines`) — never restated as a literal, per Epic-13 retro C1. Zero lines of `crates/maos-kernel-core/src` are touched by any AC. ⚠ **`check-kernel-baseline`'s scope is ONE directory measured by RAW `.lines().count()`** (`xtask/src/check_kernel_baseline.rs:30-31`, `:100-113`) — blanks and comments INCLUDED, no tokei, no exclusions. **A comment-only edit in `maos-kernel-core` reds it.** `xtask/kernel-crates.toml` (one member) is a DIFFERENT instrument and scopes `check-loom`, not the baseline."
kloc_grant: "⚠ **MEASURE BEFORE YOU BUDGET — the sprint-status summary of 14-2a's grants is WRONG ON TWO ROWS.** `sprint-status.yaml:304` claims `maos-cohort 4900→5491` and `xtask 41131→41717`; the actual TOML at HEAD is **`maos-cohort = 5713`** (`xtask/kloc.toml:439`, its own comment: *\"Final ceiling = 5678 measured + 35\"*) and **`xtask = 41932`** (`:203`, *\"Final ceiling = 41897 measured + 35\"*). The summary was written before 14-2a's independent-review deltas landed and was never corrected. Measured live by the gate itself (`cargo run -q -p xtask -- kloc-check --json`) at `ee3422d0`, confirmed two independent ways (tokei replayed with the gate's exact argv, and the gate): **`maos-a2a-core` 4849 / 4850 = +1 — THE D10 WALL, EFFECTIVELY ZERO** · `maos-bin` 16941 / 16971 = **+30** · `xtask` 41897 / 41932 = **+35** · `maos-cohort` 5678 / 5713 = **+35** · `maos-a2a-tcp` 1439 / 1500 = **+61** · `maos-control` 418 / 1500 = **+1082** (the roomy crate on this path) · `maos-capability` 1046 / 2000 = +954. **`crates/*/tests/`, `xtask/tests/` AND `xtask/src/tests/` are ALL uncharged** (`xtask/src/kloc_check.rs:166-193`; tokei argv `-e target -e tests -e benches -e examples -e fuzz -e spirits`) — every test, every proven-red vector and every gate unit test costs ZERO. ⚠ **In-`src` `#[cfg(test)]` IS charged** — tokei has no cfg awareness, which is why `spill_test_faults.rs` needed a hand-written exclusion (`:180-186`). ✅ **`maos-a2a-core` GRANT AUTHORIZED (Lunarpulse 2026-08-31) and now spent ENTIRELY on plane C** — the ratified probe costs that crate ZERO (F1 re-ruling), so the D10 headroom is not contested. ⚠ **NO OTHER GRANT IS PRE-AUTHORIZED.** 14-2a's grants were pre-authorized by Lunarpulse on 2026-08-30 for 14-2a; this story starts from zero asks. `kloc.toml:60-65` forbids a grant on an estimate: write the code, `cargo fmt --all`, measure the formatted tree, record EXACT MEASURED / ZERO HEADROOM per crate, then ask. `kloc.toml:86-87` (*\"must never block a correctness or compliance repair\"*) is the valve and this story is entitled to cite it BY NAME — but citing it is not the same as taking it silently. ⚠ **`kloc-check` EXITS 1 AT HEAD on two FOREIGN keys that MUST NOT be absorbed:** `maos-domain 8695 > 8644` (**measured −51, the register says +50** — D14, owner **14-7**; no Epic-14 commit has touched `crates/maos-domain/src`) and `_aggregate 154703 >= 147057` (D17, owner **14-6**, recalculable only at an epic retrospective). Also inside the aggregate: **155 lines in NO crate budget** — `spikes/story-11-0-wasm-host/` (144) and `templates/spirit-rust/src/lib.rs` (11) infer as `(unknown:…)` and still sum (`kloc_check.rs:245`)."
model: "frontier-class allowlist {opus-4-6, opus-4-7, opus-4-8, gpt-5.5, gpt-5.6, glm-5.1, glm-5.2, glm-5.3, opus-5, equiv}. `FRONTIER_FAMILIES` (`xtask/src/check_dev_model_tier.rs:45-48`) is the machine-checked set; **`equiv` is prose, NOT a match token** — a dev who records `equiv` reds a Blocking gate. The literal token `allowlist {` is the boilerplate guard at `check_dev_model_used_populated.rs:302`; keep it."
review: "§A6 full-layer net (Blind + Edge Case + Acceptance + **Test-Infra** + non-author runtime execution) — **NON-DEGRADABLE**, and with strictly more force than 14-2a. 14-2a made a signed wire frame mutate the *peer* trust set. This story makes a signed wire frame mutate **this process's own TLS serving identity and its own private-key material read from disk** — the local half 14-2a was split away from precisely so that one §A6 net would not cover both. ⚠ **The Test-Infra layer is load-bearing and cannot be waived**: the entire existing rotation corpus is STRUCTURALLY BLIND to the field this story must move (Blocking condition 4). Book the non-author runtime layer from the start; it MUST watch a planted mutation red the plane-C assertion live, not read a committed report."
---

# 14-2b — Self-identity rotation (the local leaf, and `swap_serving_cert`'s first production caller)

Status: **backlog — THE REPAIR FAILED. Second validator: 3 ship-blockers, 2 of them CREATED by the
repair itself.** ⚠ **Abandoning the probe deleted the only OBSERVATION the design had, and every F1b
disposition is built on observation** — yet F1b was left marked STANDS/BINDING. **F1b must be
re-ruled with F1.** Do not implement from this file. F3, F4, F5 stand. See **SECOND VALIDATOR** below.
Prior: Preflight complete 2026-08-31 (six parallel read-only
scouts at `53845402`; every acted-on claim hand re-verified; **more than twenty** claims corrected or
refuted — including four premises of this preflight's own scout briefs). **All five forks CLOSED at the round-table (2026-08-31)** — F1 and F2 collapsed into ONE
derivation, and the panel OVERTURNED half the repair ratified at `sprint-status.yaml:305`, which
did not survive measurement. See **Ratified fork rulings** below; they are binding and supersede the
sprint row's repair text.

> **What the sprint row says this story is:** *"rotating THIS host's own leaf, and therefore
> `swap_serving_cert`'s FIRST production caller."* That is true, and it is the smaller half of the
> truth.

---

## The check already exists. The swap is missing. And a signed frame can already brick this host.

The ratified repair reads: *"retain the two paths; on a reissue where THIS host's member fingerprint
moved, re-read `own_cert_chain`/`own_private_key` from disk, hash the leaf, and REFUSE unless it
equals the fingerprint the signed manifest declares."* **Measured, that check is already shipping
code**, and has been since Story 13.6a. `crates/maos-bin/src/main.rs:9987-10004`, arm (c) of
`reconcile_transport_identity_with_manifest`, verbatim:

```rust
// (c) This host's own leaf must equal its own signed member fingerprint —
// the local half of the same binding.
if let Some(signed_own) = signed_fingerprint(state.local_host().as_str()) {
    let (own_chain, _own_key) = tcp_config.load_identity()?;
    let own_leaf = own_chain.first().ok_or("cohort daemon identity chain is empty")?;
    let own_fingerprint = PeerCertFingerprint::from_cert_der(own_leaf.as_ref());
    if own_fingerprint != signed_own {
        return Err(format!(
            "the daemon's own certificate does not match the signed manifest fingerprint \
             for {} (loaded {own_fingerprint:?}, manifest {signed_own:?}) — the identity \
             on disk is stale relative to the signed manifest", …
```

It re-reads from disk, hashes the leaf, and refuses on inequality. It is **boot-only because its
caller is boot-only** — one production call, `main.rs:10056`, immediately before the transport binds
— not because the check is coupled to boot. Its signature is
`(&Arc<CohortManifestState>, &TcpA2AConfig, &[A2APeerConfig])`, every element of which a reissue
handler can supply. **This story is a runtime re-run of an existing function plus a swap, not new
logic.** A dev who writes the fingerprint comparison from scratch has written the second copy of a
security check, which is the failure mode this repo names *two mechanisms wearing one name*.

**And the absence is not inert.** `apply_reissue` (Story 12.1) can move the signed manifest under a
running daemon. `CohortManifest::peer_configs_for` **excludes self by construction** —
`crates/maos-cohort/src/manifest.rs:578-582`, *"Exclude self — a host never dials itself in a
full-pairwise mesh."* And `crates/maos-cohort/src/rotation.rs`, all 1,109 lines of 14-2a's driver,
contains **zero** occurrences of `local_host`, `own_`, `local_leaf` or `is_local` (grep returns
nothing). So a reissue that rotates **this host's own** fingerprint is not refused. It is
**unobserved** — no swap, no audit row, no `DIVERGED` window, and nothing on
`GET /v1/a2a/rotation-windows`, because the local host can never be projected into that set.

Two consequences follow, both live at HEAD today:

1. **The Send seam bricks, permanently, until restart.** `team_declaration_at`
   (`crates/maos-cohort/src/state.rs:806-826`) compares the **stale** bind-time
   `A2ARouterCore.local_leaf_fingerprint` against the **fresh** cached manifest on every send. The
   instant the reissue commits, `presented`(old) ≠ `signed`(new) → `None` → `router.rs:995` returns
   `A2AError::CohortTeamIdentityRefused`, surfaced as *"cohort team identity refused (Send): host X
   claims team None but the signed manifest declares None"*. The host silently loses the ability to
   originate every cross-team crossing. `router.rs:991-1000` states the case in its own words:
   a member *"whose local leaf no longer equals the signed `CohortMember.fingerprint` **after a
   rotation** — cannot originate a crossing at all."*
2. **The next restart does not boot.** Arm (c) above hard-errors at `main.rs:9996`.

**Attribution, stated because this repo punishes getting it wrong in both directions.** This is
**NOT 14-2a's defect.** `git log -S` places `with_local_leaf_fingerprint` and `team_declaration_at`
at **`a414f922` (13-6a)** and `apply_reissue` at **`8393b957` (12-1)**. The brick has been reachable
since 13.6a. 14-2a neither created it nor concealed it — it built the runtime half of arms (a) and
(b), declared arm (c) out of scope at its AC5.5, and named this key as the owner. What is new is
that 14-2a's own module doc now claims to be *"the RUNTIME half of that boot check"*
(`rotation.rs:16-20`) while covering two of the boot check's three certificate arms. **That
asymmetry is this story.**

---

## Blocking conditions

1. **⚠ THE RATIFIED REPAIR IS HALF-REFUTED, AND THE REFUTED HALF IS THE ORDERING RULE.**
   `sprint-status.yaml:305` maps *"operator file placement becomes §7.2.1.a's `t_provision`, the
   signature becomes `t_revoke`."* The `t_provision` half survives. The `t_revoke` half does not,
   on three independent measurements. (a) **It collapses a two-event procedure into one.** The
   spec's entire safety property (architecture §7.2.1.a, `7-inter-agent-communication.md:74`) is
   that agents *"MUST be provisioned with the replacement cert at least `T_grace` before the old
   cert's revocation timestamp"*, with accept-both **already in force at `t_provision`**. If the
   signature is simultaneously what widens each peer's accept set AND what triggers this host's
   swap, the separation is **zero by construction** — the spec's own inequality is violated.
   (b) **The spec's `t_revoke` is ONE instant; the signature is N instants** (Blocking condition 2).
   (c) **`RELEASE-HOLDS.md:156-163` (c.7) already says the opposite in writing**: *"The operator MUST
   deploy the peer's next certificate before signing/reissuing … **A signature is therefore not
   certificate provisioning**; signing first can promote trust in a leaf the peer does not yet
   serve."* Naming the signature `t_revoke` **and swapping on it** is the exact failure c.7 warns
   about, turned on the local leaf. **F1 must close before any AC is written.**

2. **⚠ `T_grace` IS 5 s. PROPAGATION IS PULL-BASED AT 60 s. NOTHING IN THE REPO RECONCILES THEM.**
   `confirmation_interval() = (t_stale_secs + 1) / 2` (`crates/maos-cohort/src/state.rs:614-620`)
   with `T_STALE_DEFAULT = 120` (`crates/maos-cohort/src/manifest.rs:94`) → **60 s default**, clamp
   15 s … 1800 s. The production loop pulls on that cadence (`crates/maos-bin/src/main.rs:10137-10148`).
   **There is NO production push broadcast** — `CohortDistributor::push_to`
   (`crates/maos-cohort/src/distribution.rs:46-57`) is reached only when answering a peer's pull,
   and `issue_reissue` (`state.rs:547-578`) has **zero production callers** (sole hit is a unit test
   at `state.rs:1527`). Meanwhile `T_grace = compute_t_grace(500, 0) = 5 s`, the §7.2.1.a cold
   floor (`crates/maos-a2a-core/src/chaos/rotation.rs:11-19`, wired at `main.rs:9800`), and each
   peer's closer starts at **its own** processing instant (`rotation.rs:974-982`) — there is no
   fleet-wide `t_revoke`. **The accept-both window is 12× shorter than the interval over which peers
   learn to open it.** An early-pulling peer has promoted and narrowed to `{F_new}` before a
   late-pulling peer has heard of `F_new`. And **pin mismatch is not retryable**:
   `HandshakeRetryPolicy::is_retryable` (`crates/maos-a2a-core/src/mtls.rs:73-83`) fires only on
   `BadCertificate | CertExpired`, exactly the two classes §7.2.1.a's retry clause names — a
   `TofuPinMismatch` is absorbed by nothing and fails hard at `transport.rs:785-786`. **This single
   number decides whether the story is shippable. F2 owns it.**

3. **⚠ PLANE C — A THIRD PLANE 14-2a's MODEL DOES NOT CONTAIN, AND IT IS A SHIP-BLOCKER.**
   14-2a moved two planes (router declaration + TOFU pin). `A2ARouterCore.local_leaf_fingerprint`
   (`crates/maos-a2a-core/src/router.rs:173`) is a **third**, and `swap_serving_cert` structurally
   cannot touch it: it mutates only `self.dial_materials` and `self.serving_cert`
   (`transport.rs:558-559`); `core` is an `Arc<A2ARouterCore>` (`transport.rs:105`);
   `A2ARouterCore` has **no `Clone` impl**; the field is a private plain
   `Option<PeerCertFingerprint>` with **no setter and no interior mutability**, written once by the
   **consuming** builder `with_local_leaf_fingerprint(mut self, …) -> Self` (`router.rs:255-258`,
   called at `transport.rs:393-396`). Contrast `set_peer_cert_fingerprint` (`router.rs:603-611`),
   which works only because `peers` is a `DashMap`. **Post-bind, plane C is frozen for the process
   lifetime.** A swap without a plane-C write ships a silent, permanent governance outage
   (the opener); a plane-C write is a `maos-a2a-core` delta against a crate with **+1 line of
   headroom**. F3 owns the shape.

4. **⚠ EVERY EXISTING ROTATION TEST IS STRUCTURALLY BLIND TO PLANE C. THE HARNESS CANNOT BE REUSED.**
   `t_14_2_rotation_scene.rs:104` builds through `build_mesh_n_provisioned`, which hard-codes
   `gates: vec![None; names.len()]` (`crates/maos-a2a-tcp/tests/support/mod.rs:519`);
   `t_10_4b_rotation_real_timing.rs` and `bind_endpoint` (`support/mod.rs:325-348`) go through
   `TcpA2ATransport::bind` → `bind_with_cohort_manifest_gate(.., None)` (`transport.rs:175`). A
   `None` gate installs `LegacyCohortManifestGate`, whose `consent_and_team` **discards the
   fingerprint argument outright** (`crates/maos-a2a-core/src/cohort.rs:234`, `_endpoint_fingerprint`)
   and returns `(Defer, None)`. **Every green swap test in the tree exercises a path where
   `local_leaf_fingerprint` is never read.** A dev who reuses the drill gets a green run that proves
   nothing about this story's deliverable. The Test-Infra §A6 layer is non-waivable for this reason.

5. **⚠ THE EXISTING DRILLS PROVE EXACTLY ONE ORDERING, AND IT IS THE ONE PRODUCTION CANNOT REPRODUCE.**
   Both `t_14_2_rotation_scene.rs` (`:104` → `:108-117` → `:208-214` → `:264-276`) and
   `t_10_4b_rotation_real_timing.rs` (`:399-408` → `:413-422` → `:547-553` → `:626-637`) open
   **every window on every node in a single synchronous in-process `for` loop with no network**,
   *then* swap, *then* close. What they prove is the set-overlap invariant: with `{old, new}`
   accepted everywhere **before** any swap, an in-place swap under load drops zero conversations.
   What they do **not** prove, and must not be inherited as proven: **no propagation delay is
   modelled** (step 2 is atomic across all N; production is N independent 60 s ticks); **no
   `T_grace` is ever waited** (`t_10_4b:716` computes `t_grace_ms`, stuffs it in a report struct,
   and **nothing sleeps on it**); and **there is no negative control for "swap before the peer
   opened its window"** — the exact production failure mode is absent from the repo. The drill also
   uses `compute_t_grace(500, 7)`, not the production `(500, 0)`.

6. **⚠ `swap_serving_cert` IS UNAUTHENTICATED, AND ALL AUTHORIZATION MUST LIVE IN THE NEW CALLER.**
   `SwappableServingCert::certified_key` (`transport.rs:1274-1290`) validates exactly two things:
   chain non-empty, and key matches leaf. It checks the leaf against **neither** the signed manifest
   fingerprint **nor** `posture`'s roots. The transport will happily install an unrelated,
   well-formed certificate. The fail-closed property is entirely the caller's, and it **cannot** be
   moved into the transport: `maos-a2a-tcp` cannot see a cohort manifest and must not gain the dep
   (`crates/maos-a2a-tcp/tests/t11_t12_chaos_absence.rs:170-177` greps that manifest in a 50× loop).

7. **⚠ THE DRIVER CANNOT LIVE BESIDE THE THING IT DRIVES — BLOCKING CONDITION 9 OF 14-2a IS LIVE
   AGAIN.** `sprint-status.yaml:304` records that conclusion as *"stale under the split"* for 14-2a,
   which was true: 14-2a's reload touched only `maos-a2a-core` types. **It is true again here, and
   the sprint row will mislead a dev who reads it.** This story must read
   `TcpA2AConfig::own_cert_chain`/`own_private_key` (`crates/maos-a2a-tcp/src/config.rs:66,:68`) and
   call `TcpA2ATransport::swap_serving_cert` (`transport.rs:541`) — both `maos-a2a-tcp`. Measured at
   HEAD: `crates/maos-cohort/Cargo.toml` declares exactly `maos-a2a-core`, `maos-domain`,
   `maos-iac`, `maos-spirit-abi`; the arrow runs tcp → cohort (dev-dep) and never back;
   **`maos-bin` is the ONLY crate in the workspace depending on `maos-a2a-tcp`**
   (`crates/maos-bin/Cargo.toml:69`, optional, feature-gated at `:20`). **Do not add the dep.** The
   actuator lands in `maos-bin` (`crates/maos-bin/src/cert_rotation.rs` is the existing home,
   deliberately in `lib.rs` so `crates/maos-bin/tests/` can drive it) behind an injected seam,
   exactly as `RotationGraceTimer` (`rotation.rs:206`) already is for exactly this problem.

8. **✅ RESOLVED 2026-09-01 — the lock edge is AVOIDED, not managed.** *(Kept because the reasoning
   is load-bearing and a dev who re-derives it will re-introduce the edge.)* The swap moves to the
   daemon service loop via an outbox (F1 re-ruling), so no `cached` guard is held when `swap_lock` is
   taken and `rotation.rs:134-136` survives verbatim. **The original hazard, for the record:**
   `rotation.rs:130-138` fixes the order as `CohortManifestState.cached → window_lock → pins entry →
   peers entry → ledger` and states `TcpA2ATransport::swap_lock` is deliberately **absent** *"because
   this story never takes it."* The attach point (`state.rs:487-514`) runs **inside the `cached`
   guard**, so a swap driven from there introduces `cached → swap_lock`. The same doc forbids
   `RotationGraceTimer` implementations from calling back into the state, so deferring to the timer
   is not a free escape either. Whatever F1 resolves to must state the new lock edge explicitly.

9. **⚠ THE GATE THAT LOOKS RIGHT IS A NULL CONTROL FOR THIS STORY, AND ITS ENROLLMENT IS A BIJECTION
   THAT REDS ON A NEW TEST.** `check-cert-rotation-trigger` is `BindingClass::Blocking`, GREEN at
   HEAD (4 legs / 39 derived invocations). Its AST probe asserts one non-test method call named
   `install_cert_rotation` plus one each of `pins`/`core`, scoped to `crates/maos-bin/src/main.rs`
   (`check_cert_rotation_trigger.rs:85-89`, `:197-224`). **A self-rotation install that reuses
   `install_cert_rotation` stays green there for free** — the leg cannot notice this story's
   deliverable. Worse, `TEST_FILES` (`:92-118`) is a **fixed 5-entry const** and
   `derive_rotation_tests_in` (`:722`) collects every `#[test]`/`#[tokio::test]` in them, then
   hard-fails if any derived test is **not** named by a leg (`:845-857`). **Adding one `#[test]` to
   any of those five files reds a Blocking gate.** Put this story's tests in NEW files and extend
   `TEST_FILES` deliberately. Second trap: `check-rotation-real-timing`
   (`check_rotation_real_timing.rs:347-419`) walks `crates/maos-a2a-tcp/tests/` for
   `t_10_4b_rotation_*` **or `t_14_2_`** — a file named `t_14_2_selfid.rs` would silently enroll into
   the *other* gate and red it. `t_14_2a_*` and `t_14_2b_*` do not match. Third: **`xtask` has +35
   lines**; `check_cert_rotation_trigger.rs` cost 945 charged lines. Reuse `MethodCallProbe`
   (`:144-168`) — the **only** `visit_expr_method_call` in the 72-gate suite, and `swap_serving_cert`
   is a method call at every site.

10. **⚠ `all gates green` IS NOT AN AVAILABLE DONE CRITERION, AND THE RED SET IS FIVE, NOT TWO.**
    Measured at HEAD with `--json` so exit codes match CI: `kloc-check` **1** (aggregate + maos-domain,
    both foreign), `check-empty-kernel` **1** (4 I9 persistent-struct violations + 2 undocumented
    `#[i9_exempt]`, kernel byte-identical), `check-service-boundary` **1** (2 unclassified new public
    kernel symbols + 4 P3), `check-env-contract` **1** (`main.rs:2669-2670`
    `MAOS_OPERATOR_BEARER_TOKEN` / `MAOS_OPERATOR_HTTP_BIND` unregistered — **pre-existing**;
    `git grep` at `38c52811` hits `main.rs:2665`, 14-2a moved the lines and did not create the read),
    `check-dev-record-completeness` **1** (`deferred-work.md:860` stale owner `14-1` — **named by no
    story**). Plus `check-escape-detector` exits **0 with a RED oracle** (D20's exact shape). **None
    of these jobs carry `continue-on-error`.** This story must disown each with a named owner in the
    Dev Agent Record and must not absorb any of them — and must not repeat 14-2a's AC6.6 error of
    stating an expected end position that measurement contradicts.

11. **⚠ SHARED-FILE COLLISION WITH A LIVE PARALLEL LANE.** The uncommitted correct-course in
    `sprint-status.yaml` opens a **model-pin currency hotfix lane** (three rows, 2026-09-01) whose
    own note reserves write ordering: *"the gate story holds `xtask/src/main.rs` +
    `crates/maos-bin/src/main.rs` first."* This story must edit **both** (`main.rs` for the capture
    and install; `xtask/src/main.rs` for any gate registration). Sequence with that lane explicitly;
    do not discover it at merge.

---

## Measured grounding (2026-08-31, at `53845402`, docs-dirty)

Every "measured" carries a `file:line` and was re-verified by hand after the scout that produced it.

| Claim (sprint row / `RELEASE-HOLDS.md` / 14-2a's record / this preflight's brief) | Measured | Verdict |
|---|---|---|
| sprint row: *"retain the two paths … re-read from disk, hash the leaf, REFUSE unless it equals the fingerprint the signed manifest declares"* is the repair to BUILD | That check is **already shipping**: `main.rs:9987-10004`, arm (c) of `reconcile_transport_identity_with_manifest` (Story 13.6a). Boot-only because its **caller** is (`main.rs:10056`), not because it is coupled to boot | **REDUNDANT — re-site, do not re-author** |
| sprint row: *"operator file placement becomes `t_provision`, the signature becomes `t_revoke`"* | Refuted 3 ways: zero separation by construction vs §7.2.1.a `:74`; `t_revoke` is N instants not one; `RELEASE-HOLDS.md:156-163` (c.7) says *"A signature is therefore not certificate provisioning"* | **REFUTED — F1** |
| sprint row: *"`TcpA2AConfig` … is taken BY VALUE into `bind`, consumed at `transport.rs:368-370`, and DROPPED, so nothing retains the paths"* | Line refs FALSE (`load_identity()` is at **`transport.rs:377`**; `:365-374` is the `boot_nonce` loop). There are **SIX** `bind*` entry points, only `bind_with_intake_sink` (`:337`) has a body. And retention is **ARM-SPECIFIC**: the `maos run` arm passes `bootstrap.tcp.clone()` (`main.rs:2458`) so the paths **survive**; only the daemon arm drops them (`:10042` destructure → `:10080` move) | **PARTLY FALSE — arm-specific, and the lines are wrong** |
| sprint row / c.2: *"nothing retains the identity"* | The **materials** are retained twice, for the process lifetime: `serving_cert: Arc<SwappableServingCert>` (`transport.rs:114`) and `dial_materials` (`:128`), both holding chain **+ private key**. What is not retained is the **provenance** (the paths) and any **predecessor** generation | **REFINED — the key is in memory; the paths are not** |
| the ratified repair is sufficient | Silent on **plane C** (Blocking 3) and on the **5 s vs 60 s inversion** (Blocking 2), either of which alone is a ship-blocker | **NECESSARY BUT NOT SUFFICIENT** |
| this preflight's brief: *"ADR-047 §3/§4 may forbid a probe-based confirmation"* | FALSE. `ADR-047:47-64` scopes to the **Transparency-Log HKDF signing seed** and forbids **external** authorities (CA/OCSP/CRL). An in-mesh A2A frame to a cohort peer is ordinary substrate traffic. **A confirmation probe IS available to F1/F2** | **REFUTED — my brief was wrong** |
| 14-2a: *"revocation forbidden by ADR-047 §3/§4"* | Conclusion holds, **pointer off by two sections**. §3 is *"TL-anchored trust root"*, §4 *"Air-gap-compatible anchoring and rotation"*, both about the TL seed; §3 itself disclaims certs (*"Rotation is seed rotation with re-derivation, not certificate renewal"*). The operative exclusion is **§5's deferred list** + NFR-Ops-12. Copied verbatim into `tests/coverage-matrix.yaml:518`; that file is `mode: warning` at `:3` and cannot fail | **TRUE-BUT-MISCITED — nothing catches it** |
| 14-2a: *"FR23b is the ROTATION THIRD ONLY (revocation forbidden)"* | FR23b's own text (`prd/functional-requirements.md:61`) **specifies** *"revocation latency median ≤60s p99 ≤5min"*, repeated by NFR-Sec-13 (`non-functional-requirements.md:46`). The requirement **commits to a figure the architecture has since ruled out** — not a deferred third, an **unservable clause**, same shape as the NFR-Rel-7 defect `[DELTA-2026-08-28]` amended at source | **NEW — F5** |
| `RELEASE-HOLDS.md` clause (c) has *"Six boundaries"* | **SEVEN `c.N` labels exist.** (c.7) PROVISION BEFORE REISSUE is written at the OUTER bullet level, not nested, and self-assigns to 14-2a | **NEW — doc defect, fix the count** |
| c.6 (`cert_post_grace_reject` label) is 14-2a's | `RELEASE-HOLDS.md` names **`14-2b-self-identity-rotation`** as owner, *"which already touches this seam."* My brief did not mention it | **NEW — this story owns c.6 too** |
| c.6 is fixable by adding a detail field | `journal_peer_identity_refusal` (`router.rs:502-546`) takes `detail: &str` and **never persists it** — `ConsentRupturePayload` carries only `rupture_id`, `original_frame_id`, `original_kind`, `accepted`, `rejected:[{address, reason}]`, `ruptured_at_ns`. The class lives only in `RuptureReason`, which is in **`maos-domain` — RED at −51, owner 14-7, and 14-2a says do not touch** | **CONSTRAINED — F4** |
| a `CertRotationRefused` audit shape exists to reuse | It exists (`crates/maos-cohort/src/audit.rs:117-121`) but is **`{peer, reason, version}` — the only one of four variants with NO fingerprint fields**. And `rotation.rs:605` formats a full `wire()` fingerprint into `reason`, where the I2 redactor eats it with **nothing left to correlate** | **PARTLY — a live legibility hole in 14-2a** |
| the I2 redactor leaves an 8-hex correlation prefix | Confirmed (`redaction.rs:139-183`, `TOKEN_HEX_MIN_LEN = 32`, applied unconditionally at `transparency_log.rs:825`). Survivors: the `sha256:` prefix, a **16-bit FNV** stamp (~1/65536 collisions — NOT a discriminator), and the deliberate `*_short` field. ⚠ **`is_hex_byte` is `a-f` ONLY (`:135-137`) — UPPERCASE hex escapes redaction entirely** | **TRUE + NEW input-dependence** |
| adding an audit kind is a kernel delta | It is not. Kinds **30 (`identity.asserted`)** and **31 (`run.capture`)** are raw-INSERT literal-kind rows written outside the kernel; next free is **32**. ⚠ `check_governance_categories.rs:19-42` enumerates via `FrameKind::from_i64` and **caps at 29**, so 30/31/32 are invisible to the "every kind classified" assertion | **TRUE — technique available, gate is blind** |
| `check-epic-close-coherence` will need an index bump (14-2a's Blocking 8) | **GREEN at HEAD**, 15 == 15. 14-2a bumped `index.md:148` 14→15 and `:149` already names **14.2b**. A `backlog → ready-for-dev → done` flip is invisible to all five rules | **RESOLVED — but rule 5 binds any epic-14 doc edit against pin 24472, and a `14-2c` split would need 15→16 in the SAME commit** |
| 14-2a: *"`check-epic-close-coherence` was ALREADY GREEN at 15==15"* | At `a22f0c01` it was **RED** at 14 vs 15; it is green **because 14-2a repaired it**. The word "already" hides its own fix | **MISLEADING** |
| 14-2a's grants: `maos-cohort 4900→5491`, `xtask 41131→41717` | **BOTH WRONG.** Actual `kloc.toml:439` = **5713**, `:203` = **41932** (each *"measured + 35"*). Written before final review deltas, never corrected | **FALSE — do not budget off sprint-status** |
| `maos-a2a-core` has room for a plane-C setter | **4849 / 4850 = +1.** The D10 wall is effectively ZERO | **BLOCKING for F3** |
| `check-scale-churn` and `check-multi-tenant-loom` repaired to green by 14-2a | **BOTH VERIFIED GREEN.** ⚠ But `check-multi-tenant-loom` took three runs: its `maos-bin-whole-package-parallel-isolation` leg needs the `worker-cli-fixture` binary from the separate `spirits/worker` package, which **its own CI job does not pre-build** (`discipline.yml:3074-3124` vs `:1968`, `:2120`), and it carries an **ephemeral-port race** (`45299`, no literal in-tree) | **TRUE, with two flake sources** |
| "the 71-gate suite" | 71 at `a22f0c01`, **72** at HEAD (`check_cert_rotation_trigger.rs`). `visit_expr_method_call` exists in **exactly one** file — that one (`:151`) | **TRUE-then, STALE-now** |
| `swap_serving_cert` has zero production callers | **CONFIRMED.** Three call sites, all tests, all passing in-memory rcgen leaves — **not one reads a PEM file** (`t_14_2_rotation_scene.rs:211`, `t_10_4b_rotation_real_timing.rs:550,:1252`) | ✅ |
| `open_rotation_window` has meaning for the local leaf | **NONE.** Keyed on `PeerId` into a store populated exclusively from `tcp_config.peer_pins` (`config.rs:119-138`). The local host is never a key | ✅ **peer-pin-only, confirmed** |
| kernel-Δ baseline | `check-kernel-baseline` GREEN, **24472 == 24472**, EXIT=0; independently `find … \| wc -l` = 24472 | ✅ |
| D14's maos-domain overage is +50 (register `epic-14-preflight-decisions.md:80`) | Measured **+51** (8695 vs 8644), and **no Epic-14 commit has touched `crates/maos-domain/src`** | **REGISTER OFF BY ONE** |
| `client_config()` goes stale after a swap | True but **latent, not live** — `transport.rs:435-441` is built once from `own_chain` and `swap_serving_cert` never touches it, but the accessor has **zero call sites** in the workspace | **LATENT — record, do not fix** |
| `posture` / `validation_time` rotate | They do not (`transport.rs:479-480`). **A rotation to a leaf under a NEW CA root cannot work** — the swap succeeds and every peer handshake then fails at a fixed `ChainToRoots` verifier, with no error naming the cause | **NEW — a coverage boundary to declare** |

---

## ⚠ SECOND VALIDATOR (fresh context, 2026-09-01) — THE REPAIR FAILED

> Run under the binding rule this story adopted. It audited the **repair**, not the original, and
> found the repair introduced new defects while leaving superseded rulings live. Three ship-blockers;
> **two were created by the repair.** All three hand-verified before being recorded.

| # | Ship-blocker | Measured |
|---|---|---|
| **W1** | **`G` is FROZEN at boot while the delay is read LIVE — the repair violates its own inequality.** | `install_cert_rotation` writes a **set-once `OnceLock`** (`crates/maos-cohort/src/state.rs:138`), so `grace` is fixed for the process lifetime. But `confirmation_interval()` derives from `cached.manifest.t_stale_secs` (`state.rs:614-620`) — a **signed, per-manifest, MUTABLE** field (`manifest.rs:219-220`, inside the signed bytes, clamp `[30, 3600]`) that a reissue replaces at `state.rs:519-524`. **One signed reissue moves the delay 60 s → 1800 s while `G` stays 65 s — a 28× violation of `G ≥ skew`**, producing exactly the TOO-LATE permanent brick F1b describes. Even constant, the lower bound has ZERO margin. |
| **W2** | **Every F1b ruling is still BINDING and every one requires the ABANDONED probe.** | F1b's deadline is *"`G` after the first OBSERVED window-open … because the probe observed that window"*; its quorum is *"all REACHABLE members"*; its dispositions turn on **confirmed vs unconfirmed**. **With no probe there is no observation, no reachability class, and nothing to name in an audit.** The daemon loop's only remote contact is `distributor.pull_from` (`main.rs:10120`, `:10139`) whose failures are **`eprintln!`-only** (`:10121-10125`, `:10140-10144`) — not returned, not audited, unusable as an oracle. AC3(c)'s *"if skew > `G`, REFUSE"* **has no input.** This is V2 reproduced one ruling later. |
| **W3** | **AC1 is unimplementable: arm (c) cannot be called from `state.rs:487-514`.** | That site is in **`maos-cohort`**, which cannot see `maos-a2a-tcp` — and arm (c) needs `TcpA2AConfig::load_identity()` (`config.rs:102`). **This story's own Blocking 7 forbids adding the edge.** AC3(c) also assigns the SAME line range to an outbox enqueue: two ACs, two mechanisms, one site, and neither says where the disk read lives. Secondary: if the equality decision did run there, `load_identity()` is two blocking `std::fs` reads **inside the `cached` `std::sync::Mutex` guard**. |

**Two more the repair never noticed, one of them a null control I introduced:**

- **W4 — an unnamed Blocking bijection over `crates/maos-bin/src/*.rs`.** `cohort_daemon_smoke_13_5c.rs:861-863` declares `SCANNED_SOURCE_FILES: [(&str, &str); 17]` and asserts at `:936-941` that its length equals a **live `read_dir`** of `crates/maos-bin/src`. Measured: **17 == 17**. **Any new `.rs` file in `maos-bin/src` reds it** — and it runs inside `check-multi-tenant-loom`'s Blocking leg (`check_multi_tenant_loom.rs:1860`), the same gate that went red under 14-2a. This story's plan adds files.
- **W5 — the `include_str!` "BLOCKING FALSIFIER" is a NULL CONTROL.** Cross-mapped: **no assertion in any of the five suites lands inside arm (c)'s span** (`main.rs:9987-10004`). `team_identity_13_6a.rs:806-824` — the suite belonging to the story that *authored* arm (c) — asserts on `:10059` and `:10087`, both **after** the region. So the falsifier reports clean while W4's real trip goes unchecked. **A control that cannot fire, introduced by the repair, in a story whose whole subject is controls that cannot fire.**
- **W6 — `lib.rs` is a pure `pub mod` manifest with ZERO `fn`**, every module `#[cfg(feature = "network")]`. "Move arm (c) into `lib.rs`" does not compile as written; the working destination is `cert_rotation.rs` — which is inside W4's bijection. And V5's stated reason was backwards: arm (d) already delegates to a library fn (`main.rs:10014-10018`), so arm (c) IS separable, for none of the reasons given.

**Also corrected/left-live (would-cause-rework):** `state` is not in scope at `main.rs:9800` — it is **`rotation_state`** (`:9765`), wrong in both places the expression appears · the RE-RULED table still names the probe as what sets `T_A`, **unmarked** · V7 and V16 were disposed in the findings table and left **live** in Blocking 6/10 and the Dev-notes "Do not touch" list, which is what a dev actually reads · the 30 s→60 s correction was applied to one of three sites and left on the wrong field at another · `main.rs:10146` is off by one (`:10147`) · V12's mechanism is **false** — the `JoinHandle` IS awaited via `shutdown()` (`main.rs:9914-9919`), though its consequence stands · *"every peer has pulled at least once"* is not established: `pull_peers` is derived once at boot (`main.rs:10050`) and never updated, so a member added by the same reissue is never pulled.

### Why this keeps failing, stated plainly

Three mechanisms have now been ratified and killed: a probe that could not run where it was placed; a
probe that carried no version bits; and a delay that deleted the observation its own dispositions
require. **The pattern is not bad luck — it is ruling faster than measuring.** The next pass does not
begin with a design. It begins by measuring **what observation, if any, this host can actually make
about its peers' convergence** — and the design follows from the answer.

**One candidate nobody has priced, raised and dropped twice:** A *serves* the manifest to peers that
pull from it. `service_pending_pulls` (`main.rs:10134`) means **A already knows which peers pulled and
what version it handed them** — a real local observation with zero new wire fields. It is incomplete
(a peer may pull from another member instead), so it is a *lower bound* on convergence, not a proof.
**Price it before designing anything on top of it.**

---

## F1 — THIRD AND FINAL RULING (2026-09-01): the probe is abandoned; a delay does the job

> The room ratified a probe twice. The first could not run where it was placed; the second carried no
> version bits. **The third answer is that no probe was ever needed** — `G ≥ skew` must be paid
> regardless, and the pull cadence supplies both quantities from a value the daemon loop already holds.

**The mechanism, and every part of it already exists:**

| Part | Where |
|---|---|
| **A waits `confirmation_interval` after processing its own reissue** — by then every peer has pulled at least once | The daemon service loop **already calls `refresh_state.confirmation_interval()?`** at `crates/maos-bin/src/main.rs:10129` and `:10146`, in async context, **outside any `cached` guard**, and already sleeps on a deadline via `sleep_until(next_confirmation)` |
| **`G ≥ confirmation_interval`** | Same value, same site (below) |
| No probe, no new intent, no new wire field, no new constant | — |

**⚠ V3 + V4 CLOSED BY ONE LINE, AT THE COMPOSITION ROOT — read this before touching `T_grace`.**
Do **NOT** modify `cold_deployment_t_grace()`: it takes no arguments, and
`crates/maos-cohort/tests/cert_rotation_14_2a.rs:723-732` **hard-asserts** it equals
`compute_t_grace(consts)` — inside `TEST_FILES[1]` of the **Blocking** `check-cert-rotation-trigger`.
Change what is **passed** instead. `install_cert_rotation` already takes `grace` as an argument
(`main.rs:9796-9801`), so at `main.rs:9800`:

```
grace = cold_deployment_t_grace() + state.confirmation_interval()?
```

Both terms derived, no literal, no signature change, the gate test stays green — **and V4's
self-deadlock never arises, because this site holds no `cached` guard.** V4 was a placement defect,
not a design defect.

**⚠ V5 — PARTIAL MOVE, and the trap is the one this epic already paid for.** Move **arm (c) ONLY**
into `crates/maos-bin/src/lib.rs` — the smallest unit that makes it callable from the library.
Arm (d) (`reconcile_home_team_with_manifest`, 13.6b's team axis — arm (c)'s **only** `main.rs`-private
dependency) and the `cohort-a2a-daemon` region **stay**, on the precedent `worker_spawn`'s own doc
records: *"The `cohort-a2a-daemon` region did NOT move (Trap 9: two suites assert its literal text via
`include_str!`)."* A partial move is a sanctioned outcome here, not a compromise — `lib.rs` already
carries five modules relocated for exactly this doctrine (`worker_cli`, `worker_spawn`, `delegation`,
`enterprise_pdp_runtime`, `topology`), each documented in place.
🔴 **BLOCKING FALSIFIER — check BEFORE the move, not after.** **Five** suites
`include_str!("../src/main.rs")` and assert on its literal text:
`crates/maos-bin/tests/{cross_team_consent_13_3.rs:373, cohort_daemon_smoke_13_5c.rs:800,:864,
enterprise_daemon_seam_13_5a.rs:19, team_identity_13_6a.rs:803, erasure_uninstall_13_5b.rs:702}`.
Any assertion landing inside the moved region is re-pointed **in the same commit**. **This is exactly
how `check-multi-tenant-loom` went red under 14-2a** — code moved out from under a source-grep oracle
— and it is not being re-run.

**⚠ NEW BINDING PROCESS RULE (table-adopted, filed by John as an epic-retro action).**
**No `ready-for-dev` without a fresh-context validator pass.** Not the author's re-read — a validator
that does not know what the table decided. This story shipped that gap twice; the second time a
validator found seven ship-blockers, one of which (V1) would have hit a dev agent on day one.

---

## ⚠ VALIDATOR FINDINGS (fresh context, 2026-09-01) — SEVEN SHIP-BLOCKERS, ALL HAND-VERIFIED

> Run per the create-story checklist's independent-validator step, against a story that had already
> survived two round-tables and a re-scout. It found that **the ratified mechanism does not exist**.
> Recorded in full, because a defect list deleted after the fix is a lesson nobody inherits.

| # | Finding | Measured |
|---|---|---|
| **V1** | **AC3(b) IS DEAD — the probe carries ZERO version bits.** | `classify_presence` sends `known_version: self.state.version()?` — **A's OWN version** (`crates/maos-cohort/src/halt_receipt.rs:229-232`). The receiver destructures `Ok(CohortManifestControl::Pull { .. })` and **never reads the body**, pushing only `verified_peer` into `pull_requests` (`state.rs:884-892`). The ACK carries `delivered` + `receiver_logical_clock`, no version (`router.rs:1672-1678`). **No wire path exists from P's manifest version to A.** The story's own second grounding table already recorded the disposition as *discarded* and never reconciled it against AC3(b). |
| **V2** | **F1b's three dispositions collapse to two.** | `classify_probe_result` (`halt_receipt.rs:132-151`) maps `PinInvalidated` and `PeerIdentityMismatch` to **`Present`** — "the peer answered, so it is UP". Up-but-hasn't-applied is byte-identical to up-and-applied. There is no Confirmed/Unconfirmed axis, so **Murat's non-negotiable falsifier (i) has no observable trigger.** |
| **V3** | **The `G` derivation is unconstructible as written, and changing it reds a Blocking gate.** | `cold_deployment_t_grace()` (`rotation.rs:183-188`) takes **no arguments** — a pure fn over two consts — so `confirmation_interval()` cannot enter it without a signature change; and `compute_t_grace` (`chaos/rotation.rs:11-19`) has **no additive term**. Worse: `crates/maos-cohort/tests/cert_rotation_14_2a.rs:723-732` **hard-asserts** `cold_deployment_t_grace() == compute_t_grace(consts)`, and that file is `TEST_FILES[1]` of the Blocking `check-cert-rotation-trigger`. |
| **V4** | **AC3(c) self-deadlocks.** | `confirmation_interval()` re-locks `self.cached` (`state.rs:614-620`); AC3(c) placed the skew decision at `state.rs:487-514`, **inside that guard**. `std::sync::Mutex` is non-reentrant. Blocking 8 was retired as *"the lock edge is AVOIDED"* without noticing the skew input itself takes the lock. |
| **V5** | **AC1's "reuse arm (c)" is impossible as written.** | `reconcile_transport_identity_with_manifest` is **private, in the binary** (`crates/maos-bin/src/main.rs:9934`); the actuator's declared home `cert_rotation` is a **library** module (`crates/maos-bin/src/lib.rs:7`). A lib module cannot call a private fn in `main.rs`. Arm (c) must **move to `lib.rs`** and the boot caller at `:10056` be re-pointed — which the story never says. |
| **V6** | **AC6 contradicts the placement.** | `probe_production_install` reads exactly one file, `COMPOSITION_ROOT = "crates/maos-bin/src/main.rs"` (`xtask/src/check_cert_rotation_trigger.rs:85`, `:198`). Widening `INSTALL_METHOD` to include `swap_serving_cert` demands that call **in `main.rs`**, where Dev notes do not put it. |
| **V7** | **A cited fence is fiction, and the boundary it guards is already crossed.** | `t11_t12_chaos_absence.rs:170-177` asserts `!src.contains("maos-kernel-core")` — a **different crate** — and has **no loop**. `crates/maos-a2a-tcp/Cargo.toml:40` **already declares `maos-cohort`** as a dev-dependency. Inherited verbatim from 14-2a without re-verification — **the exact failure this story's own frontmatter warns about.** The architectural conclusion (add no real `[dependencies]` edge) survives; the named enforcement does not. |

| **V8** | **AC5's SELF ROW is unbudgeted and crosses a crate the plan never names.** | `/v1/a2a/rotation-windows` renders in **`maos-control`** (`crates/maos-control/src/lib.rs:236-249`) from rows keyed `"peer"`, projected from `peer_configs_for`, which self-excludes. A self row is a new field/variant crossing **maos-cohort → maos-control**. **`maos-control` appears nowhere in the placement plan** (it has +1082, so it fits — but it must be named). |
| **V9** | **INTERNAL CONTRADICTION — the F1+F2 ruling still argues for a NARROW `G`.** | Marked SUPERSEDED in place rather than deleted; the third ruling governs. |
| **V10** | **Closure #4 / F1b's contingency was cleared for the REJECTED mechanism.** | `dial_once` is **private** (`transport.rs:635`) and the ruled driver sees only `Result<(), A2AError>` after `to_a2a_error()` (`error.rs:78-103`) has collapsed the classes. **Moot under the third ruling — there is no probe** — but the closure text must not be read as clearing a hazard for the delay form. |
| **V11** | **F3 sizing was understated.** | Not "+1 / effectively zero": the analogue `set_peer_cert_fingerprint` (`router.rs:598-611`) is 9 code lines under 14 doc lines. Realistic **+15…25** on `maos-a2a-core`. The authorized grant covers it; the grounding row still verdicting *"BLOCKING for F3"* is stale and reconciled here. |
| **V12** | **The drain site's shipped idiom is `?`, which silently kills the manifest-refresh loop.** | `main.rs:10133-10135` is inside `tokio::select!` in a `tokio::spawn` whose `JoinHandle` is never awaited. A `?` on a self-rotation drain **terminates manifest refresh for the process lifetime, invisibly** — and AC2 requires that a refusal must not kill the daemon. **Handle the error at the drain; never `?` it.** |
| **V13** | **AC4 forbids the only provisioning helper and never names the real-gate one.** | `build_mesh_n_with_gates` (`support/mod.rs:537-546`) takes real gates but does **not** provision windows; `build_mesh_n_provisioned` provisions but hard-codes `None` gates (`:519`). **No shipped helper does both**, and `support/mod.rs:899` warns *"AC2.1 forbids a second mesh."* AC4 must name which helper is extended and how. |
| **V14** | **Raising `G` roughly doubles a Blocking gate leg's wall clock.** | `crates/maos-bin/tests/cert_rotation_trigger_14_2a.rs:717-750` polls until `acked_at + grace + 45 s`. It scales, so it stays **green**, but runs ~65-110 s per invocation instead of ~5-50 s. Priced, not a blocker. |
| **V15** | **F5's premise is unsupported at the cited section.** | ADR-047 §5's deferred list (`:80-87`) **never mentions revocation**; the only revocation text is `:64`, inside §4, about the TL signing seed. This story corrected 14-2a's "off by two sections" pointer and **landed on a section that also does not contain the exclusion.** F5 is a PRD amendment resting on this — **re-ground the citation or F5 does not proceed.** |
| **V16** | **Blocking 10's disownment reason for `deferred-work.md:860` is wrong.** | The row reads *"Owner: `14-1` follow-up **or `14-6`**"* — and `14-6` **is** a named story. It is owned, not orphaned. The genuinely stale part is the row's premise (*"40610/40613"* vs `kloc.toml:203` = **41932**). |

**Also: ~30 off-by-N `file:line` citations** (validator §17), several load-bearing — including
`transport.rs:71` cited as `intake: 30 s` when `:70` is `intake` and `:71` is `idle: 60 s`, **and the
read leg uses `idle`**, so the raw-TLS probe's rejection price (~30 s/peer) was itself wrong; the real
bound is 60 s. ✅ **ALL CORRECTED IN PLACE 2026-09-01** against HEAD, after hand-spot-checking the corrections
themselves (`transport.rs:558-559`, `:70`/`:71`, `:687`, `:1033-1035`, `main.rs:9996` all re-verified
directly). The frontmatter model allowlist was also missing `opus-4-6`/`opus-4-7` from
`FRONTIER_FAMILIES` — added.

### The way out, and it needs no probe at all

The probe was never load-bearing. `G ≥ skew` has to be paid regardless, and **the pull cadence
supplies both quantities**: A waits `confirmation_interval` after processing its own reissue — by
which point every peer has pulled at least once — and `G ≥ confirmation_interval`. Same number, twice,
from a value already derivable. **The probe was an optimization on a cost that must be paid anyway**,
and two round-tables spent themselves designing it. F1 needs a third pass to ratify the delay form
and to dispose of V3/V4 (where `G` is computed, and by whom, without re-entering `cached`).

---

## Measured grounding — SECOND PASS (2026-09-01, probe seam, at `53845402`)

Dispatched because the round-table's F1 ruling met a path nobody had measured. Both blockers died;
the third finding is the one that mattered.

| Claim (this preflight's own reopening note / the F1 ruling / scout briefs) | Measured | Verdict |
|---|---|---|
| "an async probe cannot run relative to `apply_reissue`" | The `MutexGuard` is local to `state.rs:388-540` and is dropped before control returns to `router.rs:1671`. Two async continuation points already exist on the caller's stack; the real constraint was never asyncness, it was **crate placement** (`maos-a2a-core` holds no concrete transport) | **REFUTED — Blocker 1 dead** |
| "the swap must add a `cached → swap_lock` edge" (Blocking 8) | Sync-enqueue → async-drain ships **twice** (`pull_requests` 12.1; `pending_digest_replies` 12.4a), both drained at `main.rs:10134-10135`; and a second `Mutex` taken under `cached` is the shipped nesting **15 lines below the attach point** (`state.rs:528-539`) | **AVOIDED, not managed — lock order survives verbatim** |
| "a probe needs a THIRD reserved intent in `maos-a2a-core` (+1)" | Wrong twice. 12.3 already probes over the **existing** reserved intent via a `Pull` body (`halt_receipt.rs:224-239`); and `Unclassified` fires on the consent envelope's `intent_class` being Absent/Oversized/NonCanonical (`router.rs:704-721`), **not** on an unregistered name — a new intent is operator allowlist config, ZERO `maos-a2a-core` code | **REFUTED TWICE — Blocker 2 dead** |
| "a successful mTLS handshake with the new leaf IS the proof" | **FALSE, and the repo measured it twice before this preflight**: `cohort.rs:50-53` (production doc) and `t_10_4b_rotation_real_timing.rs:316-322` — under TLS 1.3 the client leaves handshaking before the server verifies the client cert, so `connect().await` returns `Ok` either way | **REFUTED — a bare handshake carries zero bits** |
| "the probe cannot test the new leaf pre-swap" (scout S7) | **FALSE.** `scoped_client_config` (`transport.rs:608-624`) reads the stored materials but delegates to the **public free fn** `build_client_config(chain, key, pins, posture, expected_peer, validation_time)` (`transport.rs:1350-1370`), which takes them **as arguments**. A probe config from `F_new` is constructible without swapping | **REFUTED (S8 over S7)** |
| F1b's reachable/unreachable split is unimplementable (S7, via `classify_probe_result`'s `Indeterminate`) | Applies to **reusing that helper**, not to the ruled mechanism. The three outcomes separate at three distinct sites in `dial_once`: TCP failure `:646`/`:640-645`, decoded response `:682-696`, `Io("… CertificateUnknown")` on the read leg `:689-692` | **F1b STANDS — contingency cleared** |
| the probe is the mechanism (F1, 2026-08-31) | ⚠ **THE INTERSECTION IS EMPTY.** At `G` = 5 s with 60 s skew: peer 1 holds `[0,5]`, peer 2 holds `[55,60]`. **No `T_A` exists in both.** A probe locates windows; it cannot make them overlap. **Only `G ≥ skew` can** | **EMPHASIS INVERTED — `G` is the fix, the probe is a refinement** |
| the raw-TLS leaf probe is free | Two measured costs: `TcpTimeouts::production` sets `intake: 30 s` (`transport.rs:70`) so the positive proof is ~30 s/peer holding a connection slot; and on every peer that has not yet processed the reissue it **manufactures** a refusal row (`transport.rs:1033-1035`) self-labelled *"cert_post_grace_reject candidate (or unknown-leaf refusal — this seam cannot distinguish them)"* — polluting the exact signal AC5 repairs | **PRICED AND REJECTED** |
| a refusal alert says WHY | It does not. `verifier.rs:195-198` wraps every server-side rejection in `CertificateError::Other` → `CertificateUnknown` (`rustls error.rs:655`). One undifferentiated alert, and the classification is **string matching on an `Io` variant** | **NEW — rider on AC3** |
| the reissue disposition is consumed downstream | It is **discarded** — `Ok(_)` at `router.rs:1672`; `CohortReissueDisposition` has no production reader. Routing self-rotation through the return value would mean plumbing it through `handle_intake_inner`'s response type | **NEW — another reason the outbox is cheaper** |
| P's verifier "is exactly the pin check" | Incomplete: `verify_webpki_then_pin` (`verifier.rs:120-186`) runs **WebPKI first** (validity/structure, chain-to-root under `ChainToRoots`), pin second. A structurally invalid or expired `F_new` fails at step 1 and is indistinguishable from a pin refusal | **REFINED — reinforces the new-CA-root cut line** |

---

## Ratified fork rulings (round-table 2026-08-31 — BINDING)

> Panel: Winston · Amelia · Murat · John · Mary · Paige · Sally + Vex · Grumbal · Dana.
> Criterion: **spec fidelity + long-term correctness.** The panel exercised its standing authority to
> OVERRIDE the ratified repair at `sprint-status.yaml:305`.

### F1 + F2 — COLLAPSED INTO ONE. They were never two forks.

**The derivation, and it belongs in the code as a comment, not in a story nobody re-reads.**
Peer `P` opens its accept-both window at its own pull tick `T_P` and holds `{F_old, F_new}` until
`T_P + G`. Before `T_P` it accepts only `F_old`, so this host must NOT have swapped. After `T_P + G`
it accepts only `F_new`, so this host MUST have. Therefore:

```
    max(T_P)  ≤  T_A  ≤  min(T_P) + G        ⇒        G  ≥  max(T_P) − min(T_P)
```

**`G` must cover the propagation SKEW, not the handshake RTT.** F1 asked what sets `T_A`; F2 asked
how large `G` must be; both fall out of one inequality. Measured, it is violated by **12×** at the
60 s default and by **360×** at the 1800 s clamp ceiling.

- **`T_A` is set by an IN-MESH PROBE, not a timer.** ADR-047 permits it — §3/§4 scope to the
  Transparency-Log HKDF seed and forbid **external** authorities (CA/OCSP/CRL); an A2A frame to a
  cohort peer is ordinary substrate traffic. The probe keeps `G` NARROW, which is the
  security-correct direction (Vex): the window opens when it is actually needed rather than being
  held twelve times wider to cover a guess.
  ⚠ **SUPERSEDED 2026-09-01 (validator finding V9).** The probe is abandoned and `G` IS widened to
  `confirmation_interval` — the "narrow `G`" argument did not survive the empty-intersection finding.
  Retained only so the reasoning chain is legible; **the third ruling governs.**
- **§7.2.1.a is AMENDED AT SOURCE regardless of the probe.** `T_grace = max(2 × p99_handshake_rtt, 5 s)`
  is derived from handshake time, which is the right input for the centralised-CA model the clause
  was written for — one `revoke()` at one CA is one fleet-wide `t_revoke`. MAOS has epidemic PULL and
  no CA, so the formula's input does not measure the quantity that actually varies. Precedent and
  shape: the NFR-Rel-7 **`[DELTA-2026-08-28]`** amendment by this same room — relax nothing, publish
  the breach as a result. **A wrong derivation left in the architecture because one story routed
  around it is a landmine, not a spec** (Winston). This is a PRD/architecture-delta act with its OWN
  record; it does **not** grow this story's ACs (John).
- The `t_provision` half of the ratified repair SURVIVES. **The `t_revoke` = signature half is
  OVERTURNED.**

### F1 — REOPENED then RE-RULED 2026-09-01 (both blockers disproved; the emphasis was backwards)

**Why it reopened:** the 2026-08-31 ruling made an in-mesh probe the mechanism, and the reissue path
is synchronous under a `std::sync::Mutex`. Two focused scouts then killed both objections and found a
third thing that matters more than either.

**Blocker 1 — DISPROVED. The outbox is a shipped idiom, twice.** The `MutexGuard` is local to
`state.rs:388-540` and is already dropped when control returns to `router.rs:1671`. More decisively,
this codebase already enqueues synchronously from the router path and drains asynchronously in the
daemon loop, in two places: `pull_requests` (12.1 — field `state.rs:98`, enqueue `state.rs:884-892`,
drain `take_pull_requests` `state.rs:377-383`, serviced `distribution.rs:79-86`) and
`pending_digest_replies` (12.4a — field `state.rs:116`, drain `state.rs:1033-1042`,
serviced `digest.rs:244-248`), **both drained at `crates/maos-bin/src/main.rs:10134-10135`** in the
10 ms tick of the spawned service loop (`main.rs:10116-10149`). And taking a second `Mutex` *while
holding `cached`* is already the shipped nesting **fifteen lines below the attach point**
(`state.rs:528-539`). A `self_rotation_requests` outbox is byte-for-byte that pattern.
⇒ **Blocking condition 8's `cached → swap_lock` edge is AVOIDED, not managed**: the swap runs on the
daemon loop with no `cached` guard held, so `rotation.rs:134-136`'s lock order survives verbatim.

**Blocker 2 — DISPROVED on two independent grounds.** (i) Story 12.3 already ships an in-mesh
liveness probe over the **existing** reserved intent using a `CohortManifestControl::Pull` body
(`crates/maos-cohort/src/halt_receipt.rs:224-239`, documented `:220-227`: *"The probe is a reserved
manifest-PULL — it bypasses both consent seams."*). (ii) Independently, `Unclassified` has **nothing
to do with intent registration** — `consent_decision` (`router.rs:704-721`) returns it only when the
consent envelope's `intent_class` is Absent, Oversized or NonCanonical. Any canonical
`namespace:verb` is Classified and passes the −32009 gate; it is then judged by operator allowlists.
**A new intent costs ZERO code in `maos-a2a-core` — it is operator allowlist config.** Note also that
three cohort intents are deliberately non-reserved and fully consent-evaluated
(`cohort.rs:19,:30,:34`); reserved status is not a prerequisite for a working intent.

**⚠ THE FINDING THAT INVERTED THE RULING — the intersection is EMPTY, so no probe can succeed.**
Applying this story's own ratified inequality with the measured numbers: peer 1 processes at `t=0`
and holds `{F_old, F_new}` on `[0, 5]`; peer 2 processes at `t=55` and holds it on `[55, 60]`.
**There is no `T_A` in both windows.** Not tight — empty. A probe can *locate* the windows; it cannot
make them overlap. **Only raising `G` can.** The 2026-08-31 ruling made the probe primary and the
`G` amendment a side-note; that is backwards, and a full round-table ratified the inequality without
anyone solving it.

**RE-RULED:**

| | |
|---|---|
| **PRIMARY FIX — lands first** | **`G` is derived to cover the propagation skew.** Without it every downstream mechanism selects from an empty set. This is the §7.2.1.a amendment, and AC3 cannot be implemented before it. |
| **`T_A` is set by** | the **12.3 reserved-PULL version-confirmation** (`halt_receipt.rs:224-239`). *"P has applied manifest version N"* implies *"P opened its window for A"*, because applying N is what calls `open_rotation_window`. Same fact as leaf-acceptance, no handshake, **no new intent, ZERO `maos-a2a-core` delta**. |
| **PRICED AND REJECTED — do not rebuild it** | The **raw-TLS leaf probe**. It IS constructible — `scoped_client_config` (`transport.rs:608-624`) delegates to the public free fn `build_client_config(chain, key, pins, posture, expected_peer, validation_time)` (`transport.rs:1350-1370`), which takes materials **as arguments**, so a probe config can be built from `F_new` without swapping. It also distinguishes all three outcomes cleanly (see F1b). **Rejected on two measured costs:** (1) the positive proof requires the read leg and `TcpTimeouts::production` sets `intake: 30 s` (`transport.rs:70`) — ~60 s per peer, holding a connection slot on each; (2) **on every peer that has not yet processed the reissue it manufactures a peer-identity refusal row** (`transport.rs:1033-1035`) whose own label reads *"cert_post_grace_reject candidate (or unknown-leaf refusal — this seam cannot distinguish them)"* — i.e. it sprays ambiguous candidates into **the exact signal AC5 exists to make machine-readable**. Shipping the repair and its pollution in one commit (Vex). |
| **Also disproved** | The naive form — *"a successful handshake IS the proof"* — is **false, and the repo had already measured it twice**: `cohort.rs:50-53` (production doc) and `t_10_4b_rotation_real_timing.rs:316-322` (*"a raw `TlsConnector::connect().is_err()` probe is NOT an oracle here"*). Under TLS 1.3 the client leaves handshaking before the server has verified the client certificate, so `connect().await` returns `Ok` either way. A bare handshake carries **zero bits**. |
| **NEW FALSIFIER (Murat)** | **skew > `G` ⇒ the mechanism MUST REFUSE to swap, not pick a number.** |

**The price, recorded rather than implied (Vex's standing objection, heard and overruled by the
arithmetic):** `G ≥ skew` means the accept-both window is held for the propagation interval rather
than five seconds — a materially longer period in which every peer accepts two generations for this
host. The room chose it because the alternative is an empty solution set, not because the widening
is comfortable.

**Derivation for AC3 (orchestrator closure — uses only existing quantities, invents no constant):**
`confirmation_interval()` (`state.rs:614-620`) **bounds** the skew: a peer pulls at most one interval
after any other. So
```
    G  =  confirmation_interval()  +  max(2 × p99_handshake_rtt, 5 s)
```
— the first term covers propagation skew, the second is §7.2.1.a's original overlap need, unchanged.
Single-sourced through `cold_deployment_t_grace()` (`rotation.rs:183`), never restated as a literal.
⚠ **`G` is PEER-LOCAL** — each host passes its own into `install_cert_rotation` (`main.rs:9800`), so
raising it is a fleet-wide deployment fact, not a per-rotation choice. Say so in the amendment.

### F1b — THE PROBE'S UNREACHABLE-PEER BOUND (ruled 2026-08-31; **STANDS** after the 2026-09-01 re-measure)

> ⚠ **A contingency against this ruling was raised and then cleared.** S7 reported that
> `classify_probe_result` (`halt_receipt.rs:115-120`, `:148-152`) yields THREE states — `Present` /
> `Absent` / **`Indeterminate`** — which would make the binary reachable/unreachable split
> unimplementable. That applies to **reusing that helper as-is**, not to the ruled mechanism: the
> three outcomes separate cleanly at three *different* sites in `dial_once` (`transport.rs:632-696`)
> — TCP failure at `:646` / `:640-645` (unreachable), a decoded response at `:682-696` (confirmed),
> `Io("… CertificateUnknown")` on the read leg at `:689-692` (refused). The version-confirmation probe
> distinguishes what the bound needs. **The split survives; `Indeterminate` must be mapped
> explicitly rather than inherited.**

**The discovery that settled it (Sally), and it inverted the room's security instinct:** the two
failure directions are NOT symmetric.

| A swaps... | Consequence |
|---|---|
| **TOO EARLY** (before `P` opened its window) | `P` rejects `A` until `P` pulls the reissue, opens `{F_old, F_new}`, and accepts. **Bounded by one confirmation interval and SELF-HEALING — no operator.** |
| **TOO LATE** (after `P` promoted) | `P` accepts only `F_new`; `A` serves `F_old`; **no event remains to fire on either side.** Permanent until restart — and restart hits arm (c) and **the daemon does not boot.** |

Therefore a blocking fail-closed is the fail-OPEN direction wearing a coat (Vex, on the record):
peers promote on their own timers whether or not `A` moves, so blocking past the first promotion does
not prevent the outage — **it guarantees the unrecoverable one.**

| | Ruling |
|---|---|
| **Deadline `D`** | **`G` after the FIRST OBSERVED window-open.** Derived from the run, single-sourced, **never a literal and never a new constant** (Murat: *"a new constant is a number somebody fits later"*). `A` can measure it because the probe observed that window. |
| **Quorum** | **All REACHABLE cohort members.** Unreachable ≠ not-ready — waiting is rational only for something that can arrive, and an unreachable host is already broken on its own axis (its on-disk `tcp.peer_pins` still names the retired fingerprint, so it refuses to boot against the new manifest regardless; the "failure surfaces on someone else's machine" cut line). |
| **Reachable but UNCONFIRMED at `D`** | **HARD REFUSE.** This is the hazard the probe exists for — a peer that is up, reachable, and has not opened its window. Audited; expected and observed as `*_short`; every unconfirmed peer NAMED. |
| **Only UNREACHABLE outstanding at `D`** | **SWAP.** Audited, excluded peers named, and the exclusion published on the operator read surface (the self row, below). |
| **Refusal text** | Must name the RECOVERY, not just the state: *sign a reversion to `F_old`* (AC5.3 — rollback is a new rotation back) *or complete provisioning on the named hosts*. An operator reading "rotation refused" at 2am learns nothing (Sally). |
| **Falsifiers — Murat, NON-NEGOTIABLE** | (i) a **reachable** peer that never opens its window → `A` MUST refuse; (ii) an **unreachable** peer → `A` MUST swap and name it. **If either greens, the bound is decoration.** |

### F3 — plane C: option (a), and the grant is AUTHORIZED

Option (c) was costed and **collapses into (a)**: `router.rs:943` is the reader and it reads the
cached field; live material lives on the transport in `maos-a2a-tcp`, which `maos-a2a-core` cannot
see, and threading a closure in costs MORE lines than a setter (Amelia). So: the field moves behind
a lock and gains a `&self` setter, called from the swap's caller.

⚠ **`maos-a2a-core` grant AUTHORIZED by Lunarpulse 2026-08-31**, on Vex's ground: this is not growth,
it is a **correctness repair on the security path** under `kloc.toml:86-87` cited BY NAME, and the
defect it repairs — a permanent silent governance outage described by `router.rs:991-1000` in its own
words — is shipping today. Third citation of that valve on this crate.
⚠ **AUTHORIZATION DISCHARGES THE ASK, NOT THE MEASUREMENT.** Write the code, `cargo fmt --all`,
measure the FORMATTED tree, record EXACT MEASURED / ZERO HEADROOM. *A pre-approved grant recorded
from an estimate is the same defect as an unapproved one, with a signature on it.*
🔴 **F3 does NOT ship on the existing harness** (Murat, blocking): `build_mesh_n_provisioned`
hard-codes `gates: vec![None; ..]` → `LegacyCohortManifestGate` → the fingerprint is discarded at
`cohort.rs:234`. Every green swap test in the tree runs a path where plane C is never read. **Real
`CohortManifestGate`, or no ruling.**

### F4 — option (a): the `RuptureReason` variant, ceiling UNRAISED

One refusal, one record. Splitting the class onto a cohort audit row to protect a budget row builds a
SIEM correlation problem to defend an arithmetic aesthetic (Vex). Winston's "do not absorb a foreign
red" objection was **withdrawn on the precedent**: `kloc.toml:269` — `j1-crosshost-2c` added
`RuptureReason::PeerIdentityUnverified` as **+1 with the ceiling deliberately UNRAISED**, keeping the
arithmetic legible (*"−51 = −50 (D14, not ours) + 1 (ours)"*). Same enum, same seam, same reason.
**Plus a SELF ROW on the operator read surface** — `/v1/a2a/rotation-windows` is peer-keyed and the
local host cannot appear in it by construction, so after a self-rotation an operator has no way to
see that this host's leaf moved (Sally). Recorded against AC5; not a new fork.

### F5 — amend FR23b at source, now, with its own record

FR23b's text specifies revocation latency (median ≤60 s, p99 ≤5 min) that ADR-047 §5 rules out
architecturally. That is an **unservable clause**, not a deferred third, and 14-2a described it as a
deferral — which reads as *"we'll get to it."* The panel is already sitting and NFR-Rel-7 was amended
by this same room on this same criterion in one pass, so amending now costs a session already being
paid for rather than a whole story. **Recorded as its own PRD-delta line; the story stays 6 ACs**
(John). Paige owns writing down WHY two amendments fell out of one story's measurement, or the next
reader will assume the story was overreaching.

---

### Specification closures (orchestrator, 2026-09-01 — not forks, and not open)

These five were unspecified after the round-table and would each have stalled a dev. They are
decided here; none of them is a design fork.

1. **Scope arm — DAEMON ONLY, declared not discovered.** Self-rotation is supported **only** on the
   `cohort-a2a-daemon` arm (`crates/maos-bin/src/main.rs:10078-10079`, moved to `CohortDaemonRuntime` at `:10152-10153`), the sole site where a
   concrete `Arc<TcpA2ATransport>` survives on `CohortDaemonRuntime.transport`. On the `maos run`
   cross-host arm the handle is type-erased to `Arc<dyn A2ARouter>` in the same expression
   (`main.rs:2484`) — a one-method trait, not `Any`-downcastable — so `swap_serving_cert` is
   **irrecoverable** there. 14-2a declared this boundary explicitly; this story inherits it verbatim
   rather than rediscovering it at 2am.
2. **AC1's two attach points are NOT symmetric, and `state.rs:250` takes the boot disposition.**
   `:250` runs inside `install_cert_rotation`, at boot, before peers are reachable — a probe there is
   meaningless. **`:250` performs the equality CHECK ONLY and takes the boot disposition (fatal,
   matching arm (c)); `:487-514` is the runtime path.** The boot-interval hole 14-2a closed for peers
   is closed for self by the check, not by a swap.
3. **Plane-C ordering — PLANE C FIRST, and the swap is the commit.** A failed plane-C write after a
   successful `swap_serving_cert` **is** the bricked state this story exists to prevent, so the
   fallible write happens first: set the local leaf fingerprint, then swap. If the swap then fails,
   plane C is ahead of the serving cert — which is the *recoverable* direction (the Send seam refuses
   until a retry or a reversion), not the silent one. Rationale is recorded because the naive order is
   the opposite.
4. **"Reachable" is a property of the probe, not a new knob.** It is whatever the probe's own
   transport-layer outcome says, using the SHIPPED `HandshakeRetryPolicy`
   (`crates/maos-a2a-core/src/mtls.rs:73-83`) and `TcpTimeouts::production` already wired at
   `main.rs`. **No new timeout constant, no new env var** — a new knob here is a number somebody fits
   later. ✅ **CONTINGENCY CLEARED 2026-09-01.** The three outcomes separate at three distinct sites
   in `dial_once` (`transport.rs:632-696`), so the reachable/unreachable split IS implementable and
   F1b stands. The TLS-1.3 asymmetry is real but does not bite here: P's rejection of A's *client*
   leaf never reaches `classify_handshake` — it arrives as `Io("… CertificateUnknown")` on the read
   leg at `:686`. ⚠ Two riders for whoever implements it: that classification is **string matching on
   an `Io` variant**, and `t_10_4b_rotation_real_timing.rs:355-358` already carries the §A6 warning
   that a bare `"fatal alert"` substring would count *any* TLS failure as a refusal. And
   `verifier.rs:195-198` wraps **every** server-side rejection in `CertificateError::Other`
   (→ `CertificateUnknown`, `rustls error.rs:655`), so the alert says "will not accept" and never
   says why.
5. **Plane-C setter mechanism — `std::sync::RwLock`, no new dependency.** `arc_swap` is not a
   `maos-a2a-core` dependency and adding one to the crate at **+1 line** to save a lock acquisition on
   a path that runs once per rotation is not a trade this budget can justify. `RwLock` is already the
   idiom in `transport.rs` for exactly this (`SwappableServingCert`, `SwappableDialMaterials`).

---

## Acceptance Criteria (6)

> Written against the ratified rulings above. AC3 and AC4 carry the F1+F2 derivation and the F1b
> bound; both are BINDING, and neither may be restated as a literal.

**AC1 — The reissue observes the local member, and the observation is not a second mechanism.**
A signed `cohort:manifest-reissue` whose `members[local_host].fingerprint` differs from the committed
manifest's is **observed** at `state.rs:487-514` — the same guard, after every auth/cohort-id/
monotonicity check, before `*cached` is replaced — and at `state.rs:250` (the `install_cert_rotation`
immediate reconcile), or the boot-interval hole 14-2a closed for peers reopens for self. The
fingerprint-equality decision **reuses** arm (c) of `reconcile_transport_identity_with_manifest`
rather than re-authoring it. A dev who adds a second comparison fails this AC.
⚠ **Reuse requires a MOVE (validator V5).** Arm (c) is **private, in the binary** (`main.rs:9934`)
and the actuator is a **library** module — a lib module cannot call it. Move **arm (c) only** into
`crates/maos-bin/src/lib.rs` and re-point the boot caller at `main.rs:10056`; leave arm (d)
(`reconcile_home_team_with_manifest` — arm (c)'s only `main.rs`-private dependency) and the
`cohort-a2a-daemon` region in place. **Check all five `include_str!("../src/main.rs")` suites BEFORE
the move** (F1 third ruling); any assertion inside the moved region is re-pointed in the same commit.

**AC2 — Fail-closed by construction: a mismatch is an audited refusal, never a swap.**
On observation, the driver re-reads `own_cert_chain`/`own_private_key` from the retained paths and
refuses unless the loaded leaf hashes to the signed fingerprint. The refusal is audited through the
existing `CohortAuditSink` under the existing `a2a:cert-rotation` intent — **with `*_short` fields
for BOTH expected and observed**, because `CertRotationRefused` today carries no fingerprint at all
and a full fingerprint in `reason` is eaten by the redactor. Refusal must not kill the daemon: the
boot disposition (fatal) and the runtime disposition (audited refusal, keep serving) **differ**, and
the story must state both.

**AC3 — `G` covers the skew, a DELAY orders the swap, and the swap runs off the guard.**
No probe. Three parts, in this order.
(a) **`G` covers propagation skew, changed at the composition root — NOT in the function.**
At `main.rs:9800`, pass `grace = cold_deployment_t_grace() + state.confirmation_interval()?`.
⚠ **Do NOT modify `cold_deployment_t_grace()`** — it takes no arguments and
`cert_rotation_14_2a.rs:723-732` hard-asserts its equality with `compute_t_grace(consts)` inside
`TEST_FILES[1]` of the Blocking `check-cert-rotation-trigger`. Both terms derived, never a literal.
`G` is PEER-LOCAL, so raising it is a fleet-wide deployment fact.
(b) **`T_A` is a DELAY, not a probe.** A swaps `confirmation_interval` after processing its own
reissue — by which point every peer has pulled at least once. The daemon loop already holds that
value (`main.rs:10129`, `:10146`) and already sleeps on a deadline (`sleep_until`). No new intent, no
new wire field, no new constant, **zero `maos-a2a-core` delta**.
(c) **The swap runs on the daemon service loop, not under `cached`** — outbox enqueued at
`state.rs:487-514` (the `state.rs:528-539` nesting is the shipped precedent), drained beside
`main.rs:10134-10135`. ⚠ **Do NOT `?` at the drain (validator V12)** — that site is inside a
`tokio::select!` in a `tokio::spawn` whose `JoinHandle` is never awaited, so a `?` silently
terminates manifest refresh for the process lifetime, and AC2 forbids a refusal killing the daemon.
The inequality `max(T_P) ≤ T_A ≤ min(T_P) + G` ships as a comment at the decision site.
**If skew > `G`, REFUSE — never pick a number** (Murat's falsifier).

**AC4 — Plane C moves with the leaf, or the story does not ship.**
After a successful swap, `team_declaration_at` compares against the leaf the host **now presents**.
Proven by a test built on a **real `CohortManifestGate`** — not `build_mesh_n_provisioned`, which
installs `LegacyCohortManifestGate` and discards the fingerprint (Blocking 4).
⚠ **No shipped helper does both (validator V13).** `build_mesh_n_with_gates`
(`support/mod.rs:537-546`) takes real gates but does **not** provision windows;
`build_mesh_n_provisioned` provisions but hard-codes `None` gates (`:519`). **Extend
`build_mesh_n_with_gates` with provisioning** — do not add a third mesh builder
(`support/mod.rs:899`: *"AC2.1 forbids a second mesh"*). Test lanes are kloc-free.

**AC5 — c.6: the post-grace refusal is machine-readable, and self-rotation is observable.**
Per F4: a `RuptureReason` variant (ceiling UNRAISED, arithmetic stated) plus a SELF ROW on
`/v1/a2a/rotation-windows`. ⚠ **The self row crosses maos-cohort → maos-control (validator V8)** —
rows render in `crates/maos-control/src/lib.rs:236-249`, keyed `"peer"`, projected from a
self-excluding source. **`maos-control` is therefore ON the placement plan** (418/1500, +1082 — it
fits, but it must be named rather than discovered).
A refusal caused by a retired local leaf is distinguishable in the Transparency Log from every other
`PeerIdentityUnverified` row. ⚠ Record the **dialer-side asymmetry**: when a *server* refuses a
dialer's leaf, TLS 1.3 gives the dialer only a fatal alert, which `classify_handshake` maps to
`Handshake`/`Io`, not `TofuPinMismatch` — so `transport.rs:768` never fires and **the rotating host's
own journal has no row for half its refusals**.

**AC6 — A gate that reds when the production caller is removed, and an honest disposition.**
⚠ **Do NOT widen `INSTALL_METHOD` to include `swap_serving_cert` (validator V6).**
`probe_production_install` reads exactly one file — `COMPOSITION_ROOT = "crates/maos-bin/src/main.rs"`
(`check_cert_rotation_trigger.rs:85`, `:198`) — so widening it would demand the swap call **in
`main.rs`**, where the placement plan does not put it. The AST leg stays as-is (a **second opinion**,
and per 14-2a's own doctrine a spelling check); **the production-caller proof is the RUNTIME leg**
driving a real reissue against a live daemon. Tests land in **NEW files** (Blocking 9's bijection), named
`t_14_2b_*` (never `t_14_2_*`). Proven-red vectors: deleting the swap call reds the runtime leg;
a no-op swap reds it while the AST probe stays green. Disown, with named owners, the **five**
pre-existing reds of Blocking 10. ZERO kernel-Δ @24472.

---

## The one capability, and the declared cut line

**The one capability:** *an operator rotates a live daemon's **own TLS serving identity** by placing
new key material on disk and signing a manifest that names its hash — with no restart, with the swap
refused unless disk and signature agree, and with the host's cross-team governance surviving the
rotation.*

**Declared cut lines** (state them; do not discover them):
- **Bilateral non-member peers keep the restart-to-rotate posture** (c.1, unchanged).
- **A rotation to a leaf under a NEW CA root is out of scope** — `posture` does not rotate
  (`transport.rs:479-480`) and the failure names no cause.
- **`client_config()` stays stale after a swap** — latent, zero call sites; recorded, not fixed.
- **Peers' durable `tcp.peer_pins` TOML still names the retired fingerprint.** A peer restart reverts
  to it and refuses this host's boot. **The failure surfaces on someone else's machine, at their
  restart, hours later.** This is c.4 inherited, and it is the sharpest un-owned consequence.

---

## Dev notes

**Placement, decided by the budget and the dependency graph, not by taste.**
Decision logic in `maos-cohort` (`rotation.rs`, +35); actuator in `maos-bin`
(`crates/maos-bin/src/cert_rotation.rs`, +30) behind a `SelfIdentitySwap`-shaped injected seam
slotting beside `timer: Arc<dyn RotationGraceTimer>` in `install_cert_rotation`'s signature
(`state.rs:206`) — **no new crate edge**. The path capture is one line with exact precedent:
`main.rs:9765` already does `let rotation_state = Arc::clone(&bootstrap.state);` immediately before
`bootstrap` moves at `:9769`; `bootstrap.tcp` (`main.rs:9132`) is in scope on that same line.
`TcpA2AConfig` is `Clone` (`config.rs:59`) and `load_identity(&self)` (`:102`) is `pub` and
idempotently re-callable; `load_certs` (`:142`) and `load_private_key` (`:170`) are also `pub` if only
the two `PathBuf`s are retained.

**Do not touch:** `maos-domain` (RED −51, owner 14-7) except per F4; `maos-kernel-core` (a
comment-only edit reds the baseline); `crates/maos-a2a-tcp/Cargo.toml` (grep-asserted clean in a 50×
loop). **Two exact-count source gates fence `router.rs`**:
`cohort_manifest_chokepoint_12_1.rs:24-31` and `cohort_consent_chokepoint_12_2.rs:30-37` both assert
`consent_and_team(` count **== 2**, and 12_1 asserts `.is_current(` count **== 0** — any third seam
reds both.

**Two shipped defects found in passing, neither this story's to fix, both worth a one-line note:**
14-2a shipped **broken intra-doc links** (`rotation.rs:119`, `:340` reference
`[PeerCertRotation::open_windows]`; the method is `status` at `:388`) — a `cargo doc` warning and a
name not to copy. And `PeerCertFingerprint` deserialization remains **unvalidated and case-sensitive**
(`identity.rs:49-57`); rewriting pins is exactly what a rotation forces operators to do, so an
uppercase pin boots clean and then refuses 100% of frames while naming no case problem.

---

## Tasks / Subtasks

- [x] **T0 — Close F1–F5 at a round-table.** DONE 2026-08-31; F1b/F3/F4/F5 binding; grant authorized.
- [x] **T0b — F1 REOPENED TWICE, RE-RULED THREE TIMES 2026-09-01.** Two-scout re-measure killed both
      reopening blockers; the empty-intersection finding inverted the emphasis; then a fresh-context
      validator killed the ratified probe outright (V1: it carries no version bits). **Final: no
      probe — a delay.** V1–V16 disposed in the findings table.
- [ ] T1 — Capture the identity paths on the daemon arm (`main.rs:9765` idiom).
- [ ] T2 — Self-member observation at `state.rs:487-514` **and** `state.rs:250`.
- [ ] T3 — Re-site arm (c) as the shared decision; two dispositions (boot fatal / runtime refusal).
- [ ] T4a — **`G` first, at the COMPOSITION ROOT** (`main.rs:9800`), never in
      `cold_deployment_t_grace()` — a Blocking gate test asserts that function's value.
- [ ] T4b — `self_rotation_requests` outbox at `state.rs:487-514`; drain beside `main.rs:10134-10135`
      (capture `transport.clone()` before the `tokio::spawn` at `main.rs:10116`).
- [ ] T4c — `T_A` = **delay** of `confirmation_interval` after own-processing (NO probe); F1b
      dispositions. Reuse the loop's existing `sleep_until` idiom.
- [ ] T4d — **V5 move**: arm (c) → `lib.rs`, boot caller re-pointed. **Check the five
      `include_str!("../src/main.rs")` suites FIRST**; re-point any assertion in the moved region.
- [ ] T5 — Plane-C write path (per F3) + the real-gate test harness (Blocking 4).
- [ ] T6 — Audit: expected/observed `*_short` on refusal; c.6 token per F4.
- [ ] T7 — Gate: **do NOT widen `INSTALL_METHOD`** (V6 — the AST probe scans `main.rs` only); the
      production-caller proof is the **RUNTIME leg**. New `t_14_2b_*` files (never `t_14_2_*`), extend
      `TEST_FILES` deliberately, two proven-red vectors.
- [ ] T8 — `cargo fmt --all`, measure formatted, record EXACT MEASURED per crate, then ask for grants.
- [ ] T9 — `RELEASE-HOLDS.md`: close c.2 + c.6, fix the "Six boundaries" count, split c.4's statement,
      **and flip `RELEASE-HOLDS.md:105`'s stale `14-2b … (backlog)`** (validator §17; T9 had missed it).
- [ ] T9b — Re-ground F5's ADR-047 citation (validator V15) or F5 does not proceed.
- [ ] T10 — SEPARATE RECORDS (not this story's ACs) — ⚠ **the §7.2.1.a amendment GATES AC3(a)**, so it
      lands with T4a, not at the end: §7.2.1.a `T_grace` amendment + FR23b revocation amendment, each with its own PRD/architecture-delta line and Paige's why-note.

## Dev Agent Record

### Agent Model Used
_(record the exact model id from the frontmatter allowlist — `equiv` is prose and reds a Blocking gate)_

### Debug Log References

### Completion Notes List

### File List

### Change Log
