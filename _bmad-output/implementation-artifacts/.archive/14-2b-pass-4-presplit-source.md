---
baseline_commit: "`53845402` — HEAD of `main`, tree **DIRTY IN DOCS ONLY** (`sprint-status.yaml`, `.memlog.md`, `epics/epic-14-*.md` modified; `model-pin-currency-correction-input-2026-08-31.md` and `maos/` untracked). **Zero tracked source files are modified** — re-verified by all five scouts of this pass. ⚠ `53845402`, NOT `ee3422d0`: the 14-2a commit is `ee3422d0`; `53845402` lands three one-line model-pin edits on top with zero budget or gate impact. ⚠ **Do NOT inherit 14-2a's frontmatter line refs** — 14-2a moved `transport.rs` by +200 and `main.rs` by +107."
depends_on: "**`14-2a-production-mtls-rotation-trigger`** (done, `ee3422d0`) — the PEER half this story mirrors: `PeerCertRotation` (`crates/maos-cohort/src/rotation.rs:329`), its `reload` write entry and `status` read entry, the set-once `CohortManifestState::install_cert_rotation` (`crates/maos-cohort/src/state.rs:206`), the production install (`crates/maos-bin/src/main.rs:9796-9801`), `TokioGraceTimer` (`crates/maos-bin/src/cert_rotation.rs:51`), the read-only `GET /v1/a2a/rotation-windows` (`crates/maos-control/src/lib.rs:206-253`) and the Blocking gate `check-cert-rotation-trigger`. **`13-6a`** (done, `a414f922`) — authored BOTH halves of this story's live defect: `A2ARouterCore::with_local_leaf_fingerprint` (`crates/maos-a2a-core/src/router.rs:255`) and `CohortManifestState::team_declaration_at` (`crates/maos-cohort/src/state.rs:806`), plus the boot-only arm (c) at `crates/maos-bin/src/main.rs:9989-10004`. **`12-1`** (done, `8393b957`) — authored `apply_reissue` and `peer_configs_for` (`crates/maos-cohort/src/manifest.rs:568`), whose self-exclusion at `:579-582` is why the local host is invisible to every rotation consumer."
blocks: "Nothing in Epic 14 by dependency. Closes `RELEASE-HOLDS.md` clause **(c.2)** (`swap_serving_cert` has no production caller) and **(c.6)** (the `cert_post_grace_reject` label is not machine-readable). (c.1)/(c.3)/(c.5) survive untouched; **(c.5) is EXTENDED, not closed**; (c.4) is inherited and needs a split statement. ⚠ `RELEASE-HOLDS.md:95` says **\"Six boundaries\"** and the file carries **SEVEN** `c.N` labels — (c.7) at `:156` is mis-nested at the outer bullet level and self-assigns to 14-2a. Structural doc defect; fix the count, do not adopt the boundary."
kernel_grant: "NONE and none needed. `check-kernel-baseline` **GREEN, EXIT 0, 24472 == 24472** — measured live this pass by running the gate, not read from a note. Resolved from `xtask/kernel-core-baseline.toml:472` (`src_lines`); never restate it as a literal (Epic-13 retro C1). ⚠ **The baseline measures ONE directory by raw `.lines().count()`** (`xtask/src/check_kernel_baseline.rs:99-113`) — blanks and comments INCLUDED, no tokei, no exclusions. **A comment-only edit in `crates/maos-kernel-core/src` reds it.**"
kloc_grant: "⚠ **MEASURE; DO NOT INHERIT. `sprint-status.yaml:304`'s summary of 14-2a's grants is WRONG ON TWO ROWS** — it claims `maos-cohort 4900→5491` and `xtask 41131→41717`; the live TOML and the live gate both say **`maos-cohort = 5713`** (`xtask/kloc.toml:439`) and **`xtask = 41932`** (`:203`). Measured this pass by running `cargo run -q -p xtask -- kloc-check --json` (**EXIT 1**, see below): **`maos-a2a-core` 4849 / 4850 = +1 — THE D10 WALL** · `maos-bin` 16941 / 16971 = **+30** · `xtask` 41897 / 41932 = **+35** · `maos-cohort` 5678 / 5713 = **+35** · `maos-a2a-tcp` 1439 / 1500 = **+61** · `maos-control` 418 / 1500 = **+1082** · `maos-capability` 1046 / 2000 = +954. **ALL of `crates/*/tests/`, `xtask/tests/` AND `xtask/src/tests/` are UNCHARGED** — the tokei argv is `-e target -e tests -e benches -e examples -e fuzz -e spirits` (`xtask/src/kloc_check.rs:167-191`), and `-e tests` is a NAME match, not a path prefix. Every test, every proven-red vector and every gate unit test costs ZERO. ⚠ **In-`src` `#[cfg(test)]` IS charged** — tokei has no cfg awareness; the hand-written exclusion of `spill_test_faults.rs` at `:188` is the proof. ✅ **`maos-a2a-core` GRANT AUTHORIZED (Lunarpulse 2026-08-31)** under `kloc.toml:86-87` cited BY NAME, and this pass spends it ENTIRELY on plane C — the ratified observation mechanism costs that crate **ZERO**. ⚠ **NO OTHER GRANT IS PRE-AUTHORIZED.** `kloc.toml:60-65` forbids a grant on an estimate: write the code, `cargo fmt --all`, measure the FORMATTED tree, record EXACT MEASURED / ZERO HEADROOM per crate, then ask. ⚠ **`kloc-check` EXITS 1 AT HEAD on exactly two FOREIGN rows that MUST NOT be absorbed** — verbatim from the gate's `over_budget` array: `maos-domain 8695 > 8644` (**−51**, D14, owner **14-7**) and `aggregate 154703 >= 147057` (D17, owner **14-6**). Also inside the aggregate: **155 lines in no crate budget** (`spikes/story-11-0-wasm-host/` 144, `templates/spirit-rust/src/lib.rs` 11) — they infer as `(unknown:…)` and still sum at `kloc_check.rs:221` (⚠ prior notes cite `:245`; that line is the `over_budget` field of the `Report` literal, not the summation)."
model: "frontier-class allowlist {opus-4-6, opus-4-7, opus-4-8, gpt-5.5, gpt-5.6, glm-5.1, glm-5.2, glm-5.3, opus-5, equiv}. `FRONTIER_FAMILIES` (`xtask/src/check_dev_model_tier.rs:45-48`) is the machine-checked set; **`equiv` is prose, NOT a match token** — a dev who records `equiv` reds a Blocking gate. The literal token `allowlist {` is the boilerplate guard at `check_dev_model_used_populated.rs:302`; keep it."
review: "§A6 full-layer net (Blind + Edge Case + Acceptance + **Test-Infra** + non-author runtime execution) — **NON-DEGRADABLE**, with strictly more force than 14-2a. 14-2a made a signed wire frame mutate the *peer* trust set. This story makes a signed wire frame mutate **this process's own TLS serving identity and its own private-key material read from disk**. ⚠ **The Test-Infra layer is load-bearing and cannot be waived**: the entire existing rotation corpus is STRUCTURALLY BLIND to plane C (Blocking 4). Book the non-author runtime layer from the start; it MUST watch a planted mutation red the plane-C assertion live, not read a committed report."
---

# 14-2b — Self-identity rotation (the local leaf, and `swap_serving_cert`'s first production caller)

Status: **backlog — NEEDS-REWORK. Two independent fresh-context validators, 7 ship-blockers.
The OBSERVATION half survived contact; the DECISION half did not.**
⚠ **The mechanism's own numbers say it swaps AFTER the earliest peer's window has closed, with every
falsifier green** (SB-1). ⚠ **The budget cannot fund the scope** — the mirror story 14-2a took **+813
charged lines in `maos-cohort`**; this story has **+35** (SB-3). **Do not implement from this file.**
See **FIFTH-PASS VALIDATION** below. Binding rule this status obeys, stated here because prior passes
invoked it without recording it: **no `ready-for-dev` without a fresh-context validator pass** — not
the author's re-read, a validator that does not know what the table decided.
🔴 **OPEN FORK FOR THE ORCHESTRATOR: SPLIT.** The observation primitive, the live plane-C defect, and
the self-rotation swap are three separable deliverables sharing one unfundable budget. Three prior mechanisms were ratified and killed. This
pass did not begin with a design: it began by measuring **what convergence observation this host can
actually make**, per the instruction closing `sprint-status.yaml:305`. The answer changed the story.

---

## What changed this pass

Five parallel read-only scouts measured at `53845402`; every claim acted on below was re-verified by
hand. Three results reshaped the story, and one of them retires a fork that had been open since
2026-08-31.

**1. The candidate the sprint row told me to price first is DEAD, and it dies on measurement.**
The row reads: *"A SERVES the manifest via `service_pending_pulls` so A already knows which peers
pulled and at what version — a real local observation."* **Both halves are false.**
`take_pull_requests` (`crates/maos-cohort/src/state.rs:377-385`) drains with `std::mem::take` inside
one 10 ms tick; `push_to` (`crates/maos-cohort/src/distribution.rs:46-57`) never calls `version()` at
all — it sends `signed_toml()`, so the version **is not materialised at that site**; and the `usize`
return is discarded by the expression statement at `crates/maos-bin/src/main.rs:10134`. A knows which
peers pulled for about ten milliseconds, and never knows at what version.

**2. The observation A needs already arrives at A, and one `..` pattern throws it away.**
`CohortManifestControl::Pull` carries `known_version: u64` and `known_hash: String`
(`crates/maos-cohort/src/control.rs:18-21`). **Both senders populate them correctly**
(`distribution.rs:66-67`, `halt_receipt.rs:230-231`). The sole receive site is:

```rust
Ok(CohortManifestControl::Pull { .. }) => {                       // state.rs:884
    self.pull_requests. … .push(verified_peer.clone());
    Ok(CohortReissueDisposition::PullRequested)
}
```

`verified_peer` — the **authenticated** identity — is in scope alongside P's exact version and
canonical hash, and the pattern discards both. Exhaustive grep over `crates/`: `known_version` has
**three** references (one declaration, two senders) and `known_hash` has three. **There is no reader
in the workspace.** The receive site does not even appear in the grep, because `{ .. }` binds nothing.

**3. This INVERTS validator finding V1, which killed the previous mechanism.**
V1 retired the probe on the finding that `classify_presence` sends `known_version: self.state.version()`
— *A's own* version (`halt_receipt.rs:230`). That is true and it is **the wrong direction**. The signal
A needs travels **P → A**, on the pull P already makes on its own cadence, and it is exact,
authenticated, and already on the wire. A mechanism was retired on a measurement of the opposite
direction.

> **Why this keeps happening, and what changed.** Three mechanisms died — a probe that could not run
> where it was placed, a probe with no version bits, a delay that deleted its own observation. The
> pattern named at the last pass was *ruling faster than measuring*. This pass inverted the order, and
> the fourth mechanism is not a new invention: it is **reading a field that is already on the wire.**

---

## The mechanism — four parts, each measured before it was chosen

### Part 1 — Retain the observation that already arrives (NEW; zero `maos-a2a-core` delta)

Bind `known_version` / `known_hash` at `state.rs:884` and record them against `verified_peer` in a
per-peer table beside `pull_requests` (`state.rs:98`). **No new wire field, no new intent, no new
reserved name, no `maos-a2a-core` line** — the type exists and both senders already fill it. This is
the **first** convergence observation in the system.

**What it yields, stated conservatively.** P sends its version *before* receiving A's push, so A
learns P's pre-push version and sees `≥ N` only on P's **next** tick. The lag is one pull cycle **in
the conservative direction** — A under-reports convergence, never over-reports it. A lower bound that
errs toward waiting is exactly what a swap deadline needs.

### Part 2 — `G ≥ skew`, because observation alone cannot fix an empty intersection

Each peer's closer starts at **its own** processing instant: `schedule_close`
(`crates/maos-cohort/src/rotation.rs:974`) fires `self.timer.schedule(self.grace, …)` at `:982`. With
`G = 5 s` and 60 s propagation, peer 1's window shuts at t=5 before peer 2 opens at t=60 — **the
intersection is empty and no `T_A` exists.** A probe locates windows; only `G ≥ skew` makes them
overlap. Measured at HEAD, `G < skew` **unconditionally**: `G = 5 s` (`cold_deployment_t_grace()`,
`rotation.rs:183-188`, a zero-arity pure fn over two consts) versus `confirmation_interval() ∈
[15 s, 1800 s]` (`state.rs:614-620`, floor set by `T_STALE_MIN = 30`). The violation ratio is **3× at
the most favourable clamp and 360× at the worst**. No test anywhere couples the two quantities.

### Part 3 — `T_A` is set by OBSERVATION, and the refusal finally has an input

A swaps when every **reachable** peer has reported version `≥ N`. Murat's non-negotiable falsifier —
*"if skew > `G`, REFUSE, never pick a number"* — **has an input for the first time**: A no longer
*predicts* skew, it **measures the spread** between the first and last confirming report and refuses
when that spread exceeds `G`.

⚠ **This is also how W1 is disposed without a live read.** W1 correctly observed that `G` is frozen at
boot (`cert_rotation` is a set-once `OnceLock`, `state.rs:138`; `grace` is moved by value into
`PeerCertRotation`, `rotation.rs:334`, read at exactly one site) while `confirmation_interval()` reads
a **signed, mutable** `t_stale_secs` (`manifest.rs:219-220`, inside the canonical bytes at `:246`,
clamped `[30, 3600]` at `:485-491`, replaced per reissue at `state.rs:519-524`). Under the previous
design that was a 28× violation of the story's own inequality. **Under this one it is a detected
condition, not a silent one** — because A compares `G` against a measurement, not against a prediction
made at boot.
🔴 **AND THE OBVIOUS REPAIR IS FORBIDDEN — measured, so nobody re-derives it.** `grace` CANNOT be made
to read `confirmation_interval()` at its use site: `rotation.reload(…)` is called **inside the
`cached` guard** (`state.rs:487-495`), and `state.rs:475-477` states the rule in its own words —
*"INSIDE the `cached` guard … `RotationGraceTimer` implementations therefore MUST NOT call back into
this state."* `schedule_close` reads `self.grace` under that guard, and `confirmation_interval()`
re-locks `self.cached` (`state.rs:614-620`), which is a **non-reentrant `std::sync::Mutex`**. **A live
read there self-deadlocks.** This is validator finding V4 reappearing at a new site. Any value must be
**pushed in from outside the guard**, never pulled from inside it.

### Part 4 — Plane C first, then swap (F3, unchanged in shape, newly grounded in cost)

The fallible write happens first; the swap is the commit. Rationale unchanged and recorded because the
naive order is the opposite: a failed plane-C write **after** a successful swap IS the bricked state
this story exists to prevent.

---

## The defect is live at HEAD, and this pass traced it end to end

Not a latent risk — a shipping failure with a complete path, measured this pass:

1. Operator signs a reissue moving `members[local_host].fingerprint` from `F_old` to `F_new`.
2. `apply_reissue` accepts it. The reissue projects peers through
   `peer_configs_for(self.local_host)` (`state.rs:488`), which **excludes self**
   (`manifest.rs:579-582`) — so no rotation machinery ever sees a local row.
3. `PeerCertRotation` holds `core: Arc<A2ARouterCore>` and can only call `&self` methods. **There is
   no plane-C call it could make: the method does not exist.**
4. `router.rs:943` still passes the **boot-time** `local_leaf_fingerprint` = `F_old`.
5. `team_declaration_at` (`state.rs:817-822`): `presented` = `F_old`, `signed` = `F_new` →
   **`return None`**.
6. Every crossing send hits `(true, None)` at `router.rs:994` → **`A2AError::CohortTeamIdentityRefused`**.

**Result: the host is permanently unable to originate any cross-team crossing for the life of the
process, with NO audit row** (the error returns before any rupture sink), and the field is not
writable through the `Arc`. **A restart does not recover it** — the on-disk identity is still `F_old`,
so arm (c) at `main.rs:9995` **hard-fails the boot**. Live process: silently bricked Send seam.
Restarted process: refuses to start.

⚠ **The Accept seam self-heals and the Send seam cannot, purely because of where the value is
sourced.** Accept passes `peer_leaf_fingerprint` taken fresh from the live TLS handshake on every
connection (`router.rs:1802-1805`). Send passes a value frozen at bind. That asymmetry is the defect.

---

## Blocking conditions

1. **⚠ `T_grace` IS 5 s. PROPAGATION IS PULL-BASED AT 15–1800 s. `G < skew` UNCONDITIONALLY.**
   `confirmation_interval() = (t_stale_secs + 1) / 2` (`state.rs:614-620`) with
   `T_STALE_DEFAULT = 120` (`manifest.rs:109-111`) → **60 s default**, clamped `[15 s, 1800 s]` by
   `T_STALE_MIN/MAX = 30/3600` (`manifest.rs:92-94`). `G = compute_t_grace(500, 0) = 5 s`
   (`crates/maos-a2a-core/src/chaos/rotation.rs:11-19`, wired at `main.rs:9800`). **There is NO
   production push broadcast** — `issue_reissue` (`state.rs:547-578`) has zero production callers.
   Each peer's closer starts at its own processing instant. **Part 2 owns this and lands FIRST.**

2. **⚠ PLANE C — A THIRD PLANE 14-2a's MODEL DOES NOT CONTAIN, AND THE PEER PRECEDENT DOES NOT
   TRANSFER.** `A2ARouterCore.local_leaf_fingerprint` (`router.rs:173`) is a **plain private
   `Option<PeerCertFingerprint>`** with no setter and no interior mutability, written once by the
   **consuming** builder `with_local_leaf_fingerprint(mut self, …) -> Self` (`:255`) at
   `crates/maos-a2a-tcp/src/transport.rs:395`, and `Arc::new(core_inner)` follows 20 lines later at
   `:415`. **After that line the field is frozen for the process lifetime.** Read at exactly one site,
   `router.rs:943`.
   🔴 **DO NOT MODEL THE FIX ON `set_peer_cert_fingerprint`.** That method (`router.rs:603-611`,
   **9 code lines under 15 doc lines** — ⚠ the item starts at `:588`; prior notes cite `:598-611`,
   which begins mid-doc-block) takes `&self` and works through an `Arc` **only because `peers` is a
   `DashMap` supplying its own interior mutability**. Plane C gets nothing for free. The field must
   **gain** interior mutability. Per F3: `std::sync::RwLock`, no new dependency — `arc_swap` is not a
   `maos-a2a-core` dep and adding one to a crate at **+1 line** to save one uncontended acquisition
   per rotation is not a trade this budget can justify. `RwLock` is already the `transport.rs` idiom
   for exactly this (`SwappableServingCert`, `SwappableDialMaterials`).

3. **⚠ EVERY EXISTING ROTATION TEST IS STRUCTURALLY BLIND TO PLANE C.**
   `build_mesh_n_provisioned` hard-codes `gates: vec![None; names.len()]`
   (`crates/maos-a2a-tcp/tests/support/mod.rs:519`); a `None` gate installs `LegacyCohortManifestGate`,
   whose `consent_and_team` **discards the fingerprint argument outright**
   (`crates/maos-a2a-core/src/cohort.rs:234`, `_endpoint_fingerprint`). **Every green swap test in the
   tree exercises a path where `local_leaf_fingerprint` is never read.** A dev who reuses the drill
   gets a green run that proves nothing. ⚠ **No shipped helper does both**: `build_mesh_n_with_gates`
   (`support/mod.rs:537-546`) takes real gates but does not provision windows. **Extend
   `build_mesh_n_with_gates` with provisioning** — do not add a third mesh builder (`support/mod.rs:899`:
   *"AC2.1 forbids a second mesh"*). Test lanes are kloc-free.

4. **⚠ THE DRIVER CANNOT LIVE BESIDE THE THING IT DRIVES — AND FOUR PLACEMENTS COMPILE.**
   Measured edge truth table: `maos-cohort → maos-a2a-core` only. **`maos-cohort` CANNOT see
   `maos-a2a-tcp`**, and adding that edge would additionally cycle against `maos-a2a-tcp`'s existing
   **dev-dep** on `maos-cohort`. `maos-bin` is the ONLY crate whose normal `[dependencies]` contain
   BOTH `maos-cohort` and `maos-a2a-tcp` (both `optional`, both pulled by the default `network`
   feature). **Do not add the dep.** Structurally possible placements, none requiring a new edge:
   **(A)** `maos-bin` lib module (`crates/maos-bin/src/cert_rotation.rs` is the existing home) — but it
   can only fire where the transport handle lives; **(C)** a port trait declared in `maos-cohort` and
   implemented in `maos-bin`, **in byte terms** (`maos-cohort` has no `rustls` dep and must not grow
   one) — the shipped `RotationGraceTimer` shape, whose own doc states the doctrine: *"`maos-cohort`
   has no `tokio` dependency and must not grow one, so the timer is injected"*; **(D)** a port trait in
   `maos-a2a-core`, which **already depends on `rustls`** so it may name `CertificateDer`/`PrivateKeyDer`
   directly — but that crate is at **+1**. **(A) + (C) is the combination that costs zero new edges and
   zero `maos-a2a-core` lines beyond the authorised plane-C grant.**

5. **⚠ `TcpA2ATransport` STORES NO `TcpA2AConfig`. THIS IS WHY `c.2` SAYS THE MECHANISM "PROVABLY
   CANNOT" ROTATE THE LOCAL LEAF — AND IT IS ONE CLONE, NOT A REDESIGN.** `tcp_config` is **moved**
   into `bind_with_intake_sink` at `main.rs:10080` and dropped; the transport struct
   (`transport.rs:104-149`) retains no cert paths. A runtime re-read needs the config captured
   **before** that move. `TcpA2AConfig` is `Clone` (`config.rs:59`) and `load_identity(&self)`
   (`config.rs:102`) is `pub`, does two `std::fs` reads, caches nothing, and is **idempotently
   re-callable** — so it picks up a rotated file on every call. Exact precedent for the capture one
   line up: `main.rs:9765` already does `let rotation_state = Arc::clone(&bootstrap.state);`
   immediately before `bootstrap` moves.

6. **⚠ `swap_serving_cert` IS UNAUTHENTICATED; ALL AUTHORISATION LIVES IN THE NEW CALLER.**
   `SwappableServingCert::certified_key` validates exactly two things — chain non-empty, key matches
   leaf. It checks the leaf against **neither** the signed manifest fingerprint **nor** `posture`'s
   roots. The transport will install an unrelated well-formed certificate. The fail-closed property is
   entirely the caller's and cannot be moved into the transport (`maos-a2a-tcp` cannot see a cohort
   manifest). ⚠ **The 50× grep fence cited for this in prior passes is FICTION** (validator V7):
   `t11_t12_chaos_absence.rs:170-177` asserts `!src.contains("maos-kernel-core")` — a **different
   crate** — and has no loop; and `crates/maos-a2a-tcp/Cargo.toml` **already declares `maos-cohort` as
   a dev-dependency**. The architectural conclusion survives; the named enforcement does not.

7. **⚠ THE INSTALL MUST STAY IN `main.rs`.** `check-cert-rotation-trigger`'s production probe reads
   **exactly one file** — `COMPOSITION_ROOT = "crates/maos-bin/src/main.rs"`
   (`xtask/src/check_cert_rotation_trigger.rs:85`, `:198`) — and requires non-test **method** calls to
   `install_cert_rotation`, `.pins()` and `.core()` there (`main.rs:9796-9798`). Moving that region
   into a library reds a Blocking gate.
   🔴 **AND CHANGING WHAT IS *PASSED* AT `main.rs:9800` IS **NOT** FREE — THIS STORY ASSERTED IT WAS,
   AND MEASUREMENT REFUTES IT.** `crates/maos-bin/tests/cert_rotation_trigger_14_2a.rs:720`
   **recomputes** `let grace = maos_cohort::cold_deployment_t_grace();` (5 s) and asserts at `:723-729`
   that the window closes before `rotation.acked_at + grace + 45 s` — a **50 s budget against a REAL
   daemon** whose closer is armed from `main.rs:9800`. Raise the installed grace to 60 s and the close
   lands after the budget: **the leg reds.** It does **not** scale, because the test recomputes the
   cold-branch constant instead of reading the installed value. That test is
   `expected_tests: 1` on leg `rotation-trigger-production-caller` with marker
   `ROTATION_WINDOW_CLOSED_OBSERVED=1` (`check_cert_rotation_trigger.rs:259-265`).
   ⚠ **The prior pass ruled the opposite** (V14: *"it scales, so it stays green … priced, not a
   blocker"*). V14 is REFUTED. **AC2 must co-edit that test in the same commit** so it reads the
   installed grace, and must state the leg's new wall-clock cost.
   ⚠ **Do NOT modify `cold_deployment_t_grace()`**: it is zero-arity, and
   `crates/maos-cohort/tests/cert_rotation_14_2a.rs:723-732` **hard-asserts** its equality with
   `compute_t_grace(consts)` inside `TEST_FILES[1]` of that same Blocking gate.

8. **⚠ ADDING A FILE TO `crates/maos-bin/src/` REDS A BLOCKING GATE — AND LISTING IT IS THE DESIGNED
   RESPONSE.** `cohort_daemon_smoke_13_5c.rs:861-863` declares `SCANNED_SOURCE_FILES: [(&str, &str); 17]`
   and `:937-941` asserts its length equals a **live flat `read_dir`** of `crates/maos-bin/src`.
   Measured: **17 == 17**. Executed by **THREE** Blocking executors — `check-reza-production-path`
   (`check_reza_production_path.rs:147-160`), `check-multi-tenant-loom`'s whole-package leg
   (`check_multi_tenant_loom.rs:1859-1863`, `cargo test -p maos-bin`, no filter), and the raw CI step
   at `discipline.yml:1927`. **The assert's own message says what to do**: *"every maos-bin src/\*.rs
   file must be listed so new files cannot evade this negative."* Add the tuple, bump the length —
   a test-file edit, **kloc-uncharged**. ⚠ Record in passing, do not exploit: the `read_dir` is **flat
   and does not recurse**, so `src/foo/bar.rs` evades this Blocking negative entirely. Not this
   story's to fix; worth one line in the dev record.

9. **⚠ THE GATE THAT LOOKS RIGHT IS A NULL CONTROL, AND ITS ENROLLMENT IS A BIJECTION.**
   `check-cert-rotation-trigger` is `BindingClass::Blocking` (`gate-registry.toml:291-292`,
   `v1_5`/`v2_0`/`v2_2` = blocking) and GREEN at HEAD. Its AST probe keys on `install_cert_rotation`,
   so **a self-rotation that reuses that install stays green for free** — the leg cannot notice this
   story's deliverable. ⚠ **Do NOT widen `INSTALL_METHOD` to include `swap_serving_cert`** (validator
   V6): the probe reads `main.rs` only, and the placement plan does not put the swap there. **The
   production-caller proof is a RUNTIME leg.** Second trap: `TEST_FILES` is a fixed 5-entry const and
   `derive_rotation_tests_in` (`:722-754`) hard-fails if any derived test is not leg-named, **and
   errors if any listed file derives zero tests** — so adding one `#[test]` to any of those five files
   reds the gate. **Put this story's tests in NEW files named `t_14_2b_*`** (never `t_14_2_*`, which
   `check-rotation-real-timing` (`check_rotation_real_timing.rs:347-419`) would silently enroll).
   ✅ Reuse `MethodCallProbe` (`:145-169`) — it implements `visit_expr_method_call` (the only such
   visitor in the 72-gate suite) and `swap_serving_cert` is a method call at every site.
   ⚠ **It is declared `struct MethodCallProbe<'a>` with NO `pub` (`:145`) — it is PRIVATE.** Reuse from
   a *new* gate file requires making it `pub(crate)`, an edit to a Blocking gate's source. Reuse from
   *within* `check_cert_rotation_trigger.rs` is free. **AC6 must pick which, and the two readings have
   different costs** (see AC6).

10. **⚠ `all gates green` IS NOT AN AVAILABLE DONE CRITERION.** Measured live this pass over seven
    gates on this story's axes: **`check-cert-rotation-trigger` 0** · **`check-rotation-real-timing` 0**
    · **`check-cohort-mesh` 0** · **`check-scale-churn` 0** · **`check-epic-close-coherence` 0** ·
    **`check-multi-tenant-loom` EXIT 1 (FOREIGN)** · **`check-dev-record-completeness` EXIT 1 (FOREIGN)**.
    Plus `kloc-check` **EXIT 1** on two foreign rows (Blocking condition in the frontmatter).
    ⚠ **`check-multi-tenant-loom`'s RED IS NOT WHAT PRIOR PASSES RECORD.** It is not the
    `route_outbound` source-grep window, and 14-2a's claim to have repaired it is not the current
    state. Root cause, reproduced directly: `crates/maos-bin/tests/drain_once_audit_writer.rs:27` fails
    with *"cli_wrapper command 'worker-cli-fixture' not found"* — `worker-cli-fixture` is declared in
    `spirits/worker/Cargo.toml:12-13`, **`spirits/` is not a workspace member**, so `cargo test -p
    maos-bin` never builds it. `f4_pairing.rs:276` builds it explicitly as its own prerequisite;
    `drain_once_audit_writer.rs` does not, so it passes only when a shared `target/` happens to carry
    the artifact. **A test-infra/build-ordering defect, zero relation to cert rotation.** Disown it
    with that diagnosis; do not "fix" the rotation code chasing it.
    ⚠ **`check-dev-record-completeness`'s red may be INDUCED by the dirty tree** — `deferred-work.md:860`
    names owner `14-1` while the *uncommitted* `sprint-status.yaml` marks 14-1 `done`. **Re-run against
    a clean checkout before assigning an owner.**
    ⚠ **Scope honesty:** seven of **72** gate files were executed this pass. This is NOT a refutation
    of the prior pass's "five hermetic gates red"; it is a scoped measurement of this story's axes.
    ⚠ `check-epic-close-coherence` reports **14 epics**, not the 15 a prior pass recorded. It is GREEN
    either way; the number in the earlier note is wrong.

11. **⚠ SHARED-FILE COLLISION WITH A LIVE PARALLEL LANE.** The uncommitted correct-course in
    `sprint-status.yaml` opens `model-pin-currency-gate-and-retired-pin-sweep` (`:321`), which also
    lands in `xtask` and touches `main.rs` provider-construction literals. This story must edit
    `main.rs` (capture + install) and `xtask/src/main.rs` (gate registration). **Sequence with that
    lane explicitly; do not discover it at merge.**

---

## Measured grounding — FOURTH PASS (2026-09-02, at `53845402`, docs-dirty)

Five parallel read-only scouts; every acted-on claim hand re-verified. **Citation drift corrected in
place** — prior passes carried roughly thirty off-by-N references and this pass found ten more.

| Claim carried into this pass | Measured | Verdict |
|---|---|---|
| *"A already knows which peers pulled and at what version"* (`sprint-status.yaml:305`, raised and dropped twice) | `take_pull_requests` drains with `std::mem::take` in 10 ms; `push_to` sends `signed_toml()` and never calls `version()`; the `usize` return is discarded at `main.rs:10134` | **REFUTED ON BOTH HALVES** |
| V1: *"the probe carries ZERO version bits"* | True of the **A→P** probe. The **P→A** pull carries P's exact `known_version` + `known_hash` (`control.rs:18-21`), populated by both senders, discarded at the sole receive site by `{ .. }` | **DIRECTION INVERTED — a mechanism was retired on the wrong axis** |
| W1: *"one reissue moves the delay 60→1800 s while `G` stays 65 s"* | Mechanism CONFIRMED exactly (`OnceLock` `state.rs:138`; `grace` moved by value `rotation.rs:334`; single read `rotation.rs:982`; `t_stale_secs` signed at `manifest.rs:246`, clamped `[30,3600]`, replaced `state.rs:519-524`) | **STANDS against a predicted `G`; DISPOSED by Part 3, which measures instead of predicting** |
| *"the fix is to read `confirmation_interval()` at the grace site"* | `reload` runs **inside** the `cached` guard (`state.rs:487-495`); `state.rs:475-477` forbids calling back into the state; `confirmation_interval()` re-locks a non-reentrant `std::sync::Mutex` | **FORBIDDEN — self-deadlock. V4 at a new site** |
| W6: *"`lib.rs` is a pure `pub mod` manifest, every module `#[cfg(feature = "network")]`"* | 11 `pub mod`, zero `fn` — but **`topology` (`:53`) is unconditional** | **HALF REFUTED** |
| W6/AC1: *"arm (d) is arm (c)'s ONLY `main.rs`-private dependency"* | **FALSE.** Arm (d) already delegates to `maos_bin::cross_team_crossing::reconcile_home_team_with_manifest` (`cross_team_crossing.rs:892`). Arm (c)'s only private helper is the **8-line `signed_fingerprint` closure** (`main.rs:9944-9951`); everything else it touches is `pub` cross-crate. ⚠ **But that closure has THREE consumers** — arms (a) `:9956`, (b) `:9973`, (c) `:9989` — so the move must **export or share** it, never copy it (a copy is the "second comparison" AC3 forbids) | **REFUTED — arm (c) is freely separable** |
| W5: the `include_str!` falsifier is a null control | **CONFIRMED** — but ⚠ **this pass's own first arithmetic was WRONG and is corrected here.** Arm (c) spans `main.rs:9989-10004` and is **arm three of four inside `reconcile_transport_identity_with_manifest` (`main.rs:9934-10021`)** — a named function, not unclaimed space. Correct neighbours: `run_cohort_a2a_daemon` starts **`:9627`** (not 9651), `build_cohort_a2a_daemon_runtime` starts **`:10024`** (not 10039). **No assertion in the repo pins arm (c)'s span** — verified across all six `include_str!("../src/main.rs")` sites — and that conclusion survives the corrected bounds | **CONFIRMED** |
| W4: the `SCANNED_SOURCE_FILES` bijection | **CONFIRMED 17 == 17**, three Blocking executors — but listing the file is the designed response, and the `read_dir` is **flat**, so a subdirectory evades it | **CONFIRMED, cost re-priced from blocker to one-tuple edit** |
| V15: F5 rests on ADR-047 §5 | **§5 is "Explicit v1.0 scope" and never mentions revocation.** The exclusion is **§3:55** (*"No online CA / OCSP dependency … Rotation is seed rotation with re-derivation, not certificate renewal"*) and **§4:63-64** (*"No key server, no CA, no OCSP responder"* / *"No certificate-revocation-list fetch"*). `docs/release/v2.2-capacity-envelope.md:197` already cites **§3/§4** correctly | **REPAIRED — F5 proceeds on the corrected citation** |
| V11: F3 sizing *"+1 / effectively zero"* | The analogue is **9 code lines under 15 doc lines**, and it works only because `peers` is a `DashMap`. Plane C must **add** interior mutability, so it is strictly larger | **UNDERSTATED — confirmed, and the reason is now named** |
| `check-multi-tenant-loom` is red from 14-2's `route_outbound` move | **FALSE at HEAD.** It reds on `drain_once_audit_writer.rs:27` — `worker-cli-fixture` unbuilt because `spirits/` is not a workspace member | **NEW ROOT CAUSE — third distinct cause on this gate** |
| `check-epic-close-coherence` GREEN 15 == 15 | GREEN, **14 epics** | **GREEN; count corrected** |
| `sprint-status.yaml:304` grants (`cohort 5491`, `xtask 41717`) | Live TOML **and** live gate say **5713** / **41932** | **WRONG — do not budget off the sprint row** |
| `router.rs:598-611` = `set_peer_cert_fingerprint` | Item is **`:588-611`**; `pub fn` at **`:603`**; `:598` is mid-doc-comment | **DRIFT — corrected** |
| `router.rs:995` = the `team_declaration_at` consumer | `:995` is the `return Err(…)` inside the `(true, None)` arm. **The `local_leaf_fingerprint` read is `router.rs:943`** | **DRIFT — cite `:943` for the field, `:989-1010` for the refusal** |
| `manifest.rs:578-582` = the self-exclusion | `:578` is the `for` header; the exclusion is **`:579-582`** | **off-by-one — corrected** |
| `kloc_check.rs:245` = the aggregate summation | Summation is at **`:221`**; `:245` is the `over_budget` field of the `Report` literal | **DRIFT — corrected** |

---

## Fork rulings

### F1 — FOURTH RULING (2026-09-02): the observation exists; retain it. BINDING.

Supersedes all three prior F1 rulings (probe-at-`apply_reissue`, version-confirmed probe, bare delay).
**The mechanism is Parts 1–3 above.** What makes this ruling different in kind from its predecessors:
it names no new primitive. Every element already ships — the wire field, both senders, the guard site,
the composition root. The story's work is to **stop discarding a value**, raise a derived number, and
compare a measurement against it.

**Falsifiers, non-negotiable (Murat's, now with real inputs):**
- **(i)** A peer that is reachable and never reports `≥ N` by the deadline → **A REFUSES**, and the
  refusal names the recovery (sign a reversion to `F_old` per AC5, or finish provisioning the named
  hosts) — not merely the state.
- **(ii)** A peer that is unreachable → **A SWAPS and names the excluded host.** Unreachable ≠ not-ready:
  waiting is rational only for something that can arrive, and a down host is already broken on its own
  axis (stale TOML pin, refuses to boot).
- **(iii)** Observed spread > `G` → **A REFUSES.** Never pick a number.
- **Either falsifier greening is the bound being decoration.**

### F1b — RE-RULED WITH F1 (2026-09-02), as W2 demanded. BINDING.

W2 was right that the prior pass left F1b marked STANDS while deleting the observation every one of its
dispositions requires. F1b is not restored by fiat — it is restored because **Part 1 supplies the
observation it always needed**, and each disposition now maps to a measurable predicate:

| F1b disposition | Predicate, measured |
|---|---|
| **observed** | a version record exists for the peer in the Part-1 table |
| **confirmed** | that record's `known_version ≥ N` |
| **reachable** | a pull record within a derived freshness window (never a new constant) |
| **deadline `D`** | derived, single-sourced, **never a literal and never a new constant** |

⚠ **V2 is disposed, not inherited.** V2 killed the confirmed/unconfirmed axis because
`classify_probe_result` maps `PinInvalidated` and `PeerIdentityMismatch` both to `Present`
(`halt_receipt.rs:141-147`). That is a fact about **reusing that helper**, and this ruling does not use
it. The axis here is a `u64` version comparison, not a probe classification.

### F2 — COLLAPSED INTO F1. Retained for the record.

`G ≥ propagation SKEW`, not handshake RTT. Peer P accepts `{F_old, F_new}` only on `[T_P, T_P + G]`, so
`max(T_P) ≤ T_A ≤ min(T_P) + G`. Architecture **§7.2.1.a is AMENDED AT SOURCE** regardless of this
story's mechanism: its `T_grace` input is `p99_handshake_rtt` from `iac_handshake_duration_us`, a metric
with **no producer anywhere in the workspace**, and the formula was written for a centralised-CA model
with one fleet-wide `t_revoke`. MAOS has epidemic PULL and no CA. NFR-Rel-7 `[DELTA-2026-08-28]` is the
precedent; the amendment gets its own record and **does not grow this story's ACs**.
⚠ **The `t_revoke` = signature mapping stays OVERTURNED.** `RELEASE-HOLDS.md:156-163` (c.7) says it in
writing: *"A signature is therefore not certificate provisioning; signing first can promote trust in a
leaf the peer does not yet serve."* The `t_provision` half survives: **operator file placement is
`t_provision`.**

### F3 — plane C: option (a), grant AUTHORISED, sizing corrected upward

The field moves behind a `std::sync::RwLock` and gains a `&self` setter, called from the swap's caller.
**Grant authorised by Lunarpulse 2026-08-31** under `kloc.toml:86-87` cited BY NAME — a correctness
repair on the security path, third citation of that valve on this crate, and the defect it repairs is
**shipping today** (traced above).
⚠ **AUTHORISATION DISCHARGES THE ASK, NOT THE MEASUREMENT.** Write the code, `cargo fmt --all`, measure
the FORMATTED tree, record EXACT MEASURED / ZERO HEADROOM. *A pre-approved grant recorded from an
estimate is the same defect as an unapproved one, with a signature on it.*
🔴 **F3 does NOT ship on the existing harness** (Blocking 3). Real `CohortManifestGate`, or no ruling.

### F4 — option (a): the `RuptureReason` variant, ceiling UNRAISED

One refusal, one record. Precedent: `kloc.toml:269` — `j1-crosshost-2c` added
`RuptureReason::PeerIdentityUnverified` as **+1 with the ceiling deliberately UNRAISED**, keeping the
arithmetic legible (*"−51 = −50 (D14, not ours) + 1 (ours)"*). Same enum, same seam, same reason.
**Plus a SELF ROW** on `/v1/a2a/rotation-windows`, which is peer-keyed three times over (struct field
`peer` at `maos-control/src/lib.rs:39`, JSON key at `:240`, doc *"The cohort member whose certificate is
rotating"*), so a self-rotation is today **indistinguishable from nothing rotating**.
⚠ **The self row crosses FIVE crates, and `maos-control` constrains the type.**
`crates/maos-control/Cargo.toml:11-12` declares only `maos-domain` and `maos-kernel-core`, and
`lib.rs:32-35` states the rule: *"Plain owned scalars ON PURPOSE … the rotation read surface adds NO
crate edge."* **The new field must be a scalar (`String`/`bool`), never a typed fingerprint.** And
`rotation_status` (`state.rs:284-295`) derives its whole population from `peer_configs_for` at `:293`,
which excludes self — **a self row must be synthesised outside that projection.**

### F5 — amend FR23b at source, on the CORRECTED citation

FR23b (`prd/functional-requirements.md:61`) specifies *"revocation latency median ≤60s p99 ≤5min"* —
an **unservable clause**, because ADR-047 **§3:55** and **§4:63-64** architecturally exclude the
OCSP/CRL mechanism such a latency would measure. ✅ **V15 REPAIRED**: the prior citation (§5) is the
v1.0-scope section and contains no revocation text; the repo already cites §3/§4 correctly at
`docs/release/v2.2-capacity-envelope.md:197`. Recorded as its own PRD-delta line; **the story stays
6 ACs**. Paige owns writing down why two amendments fell out of one story's measurement.

### Specification closures (carried forward, still binding)

1. **Scope arm — DAEMON ONLY, declared not discovered.** Self-rotation is supported **only** on the
   `cohort-a2a-daemon` arm. `main.rs:9779-9787` states it: the `maos run` cross-host arm coerces its
   transport to `Arc<dyn A2ARouter>` at `:2484`, a one-method trait that is not `Any`-downcastable, so
   `pins()`, `core()` and `swap_serving_cert` are **irrecoverable** there.
2. **AC3's two attach points are NOT symmetric.** `state.rs:250` runs inside `install_cert_rotation`,
   at boot, before peers are reachable — it performs the **equality CHECK ONLY** and takes the boot
   disposition (fatal, matching arm (c)). `state.rs:487-514` is the runtime path.
3. **Plane-C ordering — PLANE C FIRST, and the swap is the commit.** Recorded because the naive order
   is the opposite.
4. **Plane-C setter mechanism — `std::sync::RwLock`, no new dependency** (rationale in Blocking 2).

---

## Acceptance Criteria (6)

> Every AC below names a site that exists at `53845402` and was hand-verified this pass. Where an AC
> forbids something, the forbidden thing was **measured to fail**, not assumed to.

**AC1 — The convergence observation is retained instead of discarded.**
At `crates/maos-cohort/src/state.rs:884` the `Pull` arm binds `known_version` and `known_hash` instead
of `{ .. }`, and records them against `verified_peer` (already in scope) in a per-peer table beside
`pull_requests` (`state.rs:98`), with a reader on `CohortManifestState`. **ZERO `maos-a2a-core` delta** —
`CohortManifestControl::Pull` already carries both fields (`control.rs:18-21`) and both senders already
populate them (`distribution.rs:66-67`, `halt_receipt.rs:230-231`); this AC adds **no wire field, no
intent, no reserved name**. Proven by a test in which a peer at a known version pulls and the table
reports it, plus a negative in which a peer that never pulls has no record.
🔴 **Proven-red vector — RESTATED; the first version was unreachable** (V1-4). Restoring `{ .. }`
*alone* leaves `known_version`/`known_hash` unresolved at the insert → **E0425, a build failure, not a
test red**, so "a green run with the binding reverted" cannot occur. The mutation is **"revert Part 1's
record write in full"** → the observation test reds.
⚠ **AC1's negative is VACUOUS as written**: *"a peer that never pulls has no record"* is satisfied by
an **empty table**, so it stays green under every mutation of Part 1. Replace it with a negative an
empty table cannot satisfy (e.g. two peers at different versions, asserting the table distinguishes
them).

**AC2 — `G` covers the skew, derived, at the composition root.**
At `crates/maos-bin/src/main.rs:9800`, the value passed as `grace` is raised so that
**`G ≥ skew + confirmation_interval`**, all terms derived and none a literal.
🔴 **`G ≥ skew` ALONE IS WRONG — this story asserted it and it is incoherent with AC4** (SB-1).
Derivation, from this story's own primitives: P applies reissue N at `T_P`; P's accept-both window is
`[T_P, T_P + G]`; **P's next pull is the first carrying `known_version ≥ N`, at `R_P = T_P + I`**
where `I = confirmation_interval()` (`main.rs:10146-10147` re-arms `next_confirmation = now + I`). AC4
swaps at `T_A = max(R_P) = max(T_P) + I`, and feasibility demands `T_A ≤ min(T_P) + G`, hence
**`G ≥ skew + I`**. Under `G ≥ skew` the deficit is one full `I` — 60 s by default, up to 1800 s —
and A swaps after the earliest peer has already promoted to `{F_new}` and begun refusing it.
⚠ **THE EXPRESSION IS STILL NOT WRITTEN, AND MUST BE BEFORE DEV.** Naming the inequality is not naming
the code. `skew` has no direct source expression: it is *observed* (AC1's table), not predicted, so a
boot-time `G` must use a bound for it. **The next pass must state the literal Rust expression, and say
whether the bound is the live `confirmation_interval()` or the `T_STALE_MAX` clamp — a 60 s vs 1800 s
fleet-wide dual-cert-acceptance posture decision that must not be left to a dev.**
⚠ **Do NOT modify `cold_deployment_t_grace()`** — zero-arity, and `cert_rotation_14_2a.rs:723-732`
hard-asserts its equality with `compute_t_grace(consts)` inside `TEST_FILES[1]` of a **Blocking** gate.
⚠ **Do NOT read `confirmation_interval()` at the grace use site** (`rotation.rs:982`) — it runs under
the `cached` guard and the mutex is non-reentrant; **it deadlocks.** Push the value in from outside.
⚠ **The `install_cert_rotation` call stays in `main.rs`** (Blocking 7). `G` is PEER-LOCAL
(`main.rs:9800`), so raising it is a fleet-wide deployment fact and must be stated as one.
**Lands FIRST** — the §7.2.1.a amendment gates it, so that record lands with this AC, not at the end.

**AC3 — Fail-closed by construction: a mismatch is an audited refusal, never a swap.**
The driver re-reads `own_cert_chain` / `own_private_key` from the retained paths (Blocking 5: capture a
`TcpA2AConfig` clone before the move at `main.rs:10080`) and **refuses unless the loaded leaf hashes to
the signed fingerprint**. The decision **reuses arm (c)** (`main.rs:9989-10004`) rather than
re-authoring it — a second comparison of a security invariant fails this AC.
✅ **The move is cheap and unpinned**: arm (c) has no `main.rs`-private dependency but an 8-line closure,
and **no assertion in the repo pins its span** (W5, re-measured). Follow the shipped recipe at
`xtask/src/check_j1_loopback_delegation.rs:710-757` — `lib.rs` exports `pub mod`, `main.rs` does not
re-declare it, no in-`src` `#[cfg(test)]` — and **add the new file to `SCANNED_SOURCE_FILES`, bumping
`17 → 18`** (Blocking 8; test-file edit, uncharged).
The refusal is audited through the existing `CohortAuditSink` under the existing `a2a:cert-rotation`
intent, **with `*_short` fields for BOTH expected and observed** — `CertRotationRefused` carries no
fingerprint today, and a full one in `reason` is eaten by the I2 redactor. **A refusal must not kill the
daemon**: the boot disposition (fatal) and the runtime disposition (audited refusal, keep serving)
**differ**, and both must be stated.

**AC4 — `T_A` is set by observation, and every refusal has an input.**
A swaps only when every **reachable** peer has reported `≥ N` per AC1's table, subject to F1b's four
predicates and three falsifiers. The inequality `max(T_P) ≤ T_A ≤ min(T_P) + G` ships as a comment at
the decision site. **If the observed spread exceeds `G − I`, REFUSE** — never pick a number.
🔴 **`spread > G` IS A NULL CONTROL AND THIS STORY SHIPPED IT AS A FALSIFIER** (SB-1). A observes
`spread(R_P) = spread(T_P + I) = spread(T_P) = skew`, and AC2 guarantees `G ≥ skew`, so `spread > G`
is **false by construction** — the falsifier can never fire against the very failure it exists to
catch. The evaluable predicate is `spread > G − I`.
⚠ **AND A IS COMPARING AGAINST THE WRONG `G`** (V1-7): feasibility turns on `min(T_P) + G_P`, i.e.
**each peer's own boot-frozen grace**, set from *that peer's* `OnceLock` at *its* boot. A can only see
A's `G`. W1's frozen-vs-live hazard is therefore RELOCATED, not disposed — from A's prediction to A's
inability to observe P's constant. **The next pass must rule on this; it is not closed.**
⚠ **The enqueue site is not the decision site** (V1-8): at `state.rs:487-514` the reissue has just been
applied and **zero peers can have reported `≥ N`**. That site can host a *pending-swap intent* only.
**Where the predicate is evaluated is unstated** — the 10 ms drain tick, or `state.rs:884` on the last
confirming pull. Decide it.
⚠ **The swap runs on the daemon service loop, not under `cached`** — enqueued at `state.rs:487-514`
(the `state.rs:528-539` nesting is the shipped precedent for a second lock under `cached`), drained
beside `main.rs:10134-10135`. ⚠ **Do NOT `?` at the drain** (validator V12): that site is inside a
`tokio::select!` in a `tokio::spawn`, and a `?` there terminates manifest refresh for the process
lifetime — which AC3 forbids. Handle the error at the drain.
⚠ **The shipped line beside it already has the property this AC forbids**: `main.rs:10134` is
`distributor.service_pending_pulls().await?;` — same `select!`, same `spawn`. **Do not "fix" it** (out
of scope, unpriced) and do not read it as licence. The rule binds new code.

**AC5 — Plane C moves with the leaf, and the operator can see it.**
After a successful swap, `team_declaration_at` compares against the leaf the host **now presents** —
i.e. the live defect traced above is closed, not merely documented. Plane C is written **before** the
swap (closure 3). Proven on a **real `CohortManifestGate`**, never `build_mesh_n_provisioned`
(Blocking 3). Plus, per F4: a `RuptureReason` variant (**ceiling UNRAISED**, arithmetic stated) and a
**SELF ROW** on `/v1/a2a/rotation-windows` — scalar-typed, synthesised outside `peer_configs_for`.
⚠ Record the **dialer-side asymmetry**: when a *server* refuses a dialer's leaf, TLS 1.3 gives the
dialer only a fatal alert, which `classify_handshake` maps to `Handshake`/`Io`, not `TofuPinMismatch` —
so **the rotating host's own journal has no row for half its refusals**, and `verifier.rs:195-198`
wraps every server-side rejection in `CertificateError::Other`, so the alert never says why.

**AC6 — A gate that reds when the production caller is removed, and an honest disposition.**
⚠ **Do NOT widen `INSTALL_METHOD`** (validator V6) — the AST probe reads `main.rs` only and the swap
does not live there. The AST leg stays a **second opinion**; **the production-caller proof is a RUNTIME
leg** driving a real signed reissue against a live daemon. Tests land in **NEW `t_14_2b_*` files**
(Blocking 9). Reuse `MethodCallProbe`. **Two proven-red vectors, both EXECUTED and recorded:** deleting
the swap call reds the runtime leg; a no-op swap reds the runtime leg while the AST probe stays green.
**Disown, with named owners and the diagnoses measured above, every pre-existing red** — including the
corrected root cause for `check-multi-tenant-loom` and the dirty-tree caveat on
`check-dev-record-completeness`. **Do not restate an expected end position that measurement
contradicts** (14-2a's AC6.6 error). ZERO kernel-Δ @ 24472.

---

## The one capability, and the declared cut lines

**The one capability:** *an operator rotates a live daemon's **own TLS serving identity** by placing new
key material on disk and signing a manifest that names its hash — with no restart, with the swap refused
unless disk and signature agree, and with the host's cross-team governance surviving the rotation.*

**Declared cut lines** (state them; do not discover them):
- **Bilateral non-member peers keep the restart-to-rotate posture** (c.1, unchanged).
- **A rotation to a leaf under a NEW CA root is out of scope** — `posture` does not rotate
  (`transport.rs:479-480`) and the failure names no cause. Reinforced by measurement:
  `verify_webpki_then_pin` (`verifier.rs:120-186`) runs **WebPKI first**, so a structurally invalid or
  expired `F_new` fails at step 1, indistinguishable from a pin refusal.
- **`client_config()` stays stale after a swap** — latent, zero call sites; recorded, not fixed.
- **Peers' durable `tcp.peer_pins` TOML still names the retired fingerprint.** A peer restart reverts to
  it and refuses this host's boot. **The failure surfaces on someone else's machine, at their restart,
  hours later.** This is c.4 inherited, and it is the sharpest un-owned consequence.
- **The flat `read_dir` hole in `SCANNED_SOURCE_FILES`** (Blocking 8) is recorded, not fixed.

---

## Dev notes

**Placement, decided by the dependency graph and the budget, not by taste.** Decision logic in
`maos-cohort` (`rotation.rs`, +35 headroom); actuator in `maos-bin`
(`crates/maos-bin/src/cert_rotation.rs` is the existing home, deliberately in the lib so
`crates/maos-bin/tests/` can drive it) behind an injected port **declared in byte terms** — `maos-cohort`
has no `rustls` dep and must not grow one. This is the shipped `RotationGraceTimer` shape
(`rotation.rs:206`), whose own doc states the doctrine. **No new crate edge.** The `TcpA2AConfig` capture
has exact precedent one line up at `main.rs:9765`.

**Do not touch:** `maos-domain` (RED −51, owner 14-7) except per F4; `maos-kernel-core` (a comment-only
edit reds the baseline). **Two exact-count source gates fence `router.rs`**:
`cohort_manifest_chokepoint_12_1.rs:24-31` and `cohort_consent_chokepoint_12_2.rs:30-37` both assert
`consent_and_team(` count **== 2**, and 12_1 asserts `.is_current(` count **== 0** — any third seam reds
both.

**Two shipped defects found in passing, neither this story's to fix, both worth a line in the dev
record:** 14-2a shipped **broken intra-doc links** (`rotation.rs:119`, `:340` reference
`[PeerCertRotation::open_windows]`; the method is `status` at `:388`). And `PeerCertFingerprint`
deserialisation remains **unvalidated and case-sensitive** (`identity.rs:49-57`) — rewriting pins is
exactly what a rotation forces operators to do, so an uppercase pin boots clean and then refuses 100% of
frames while naming no case problem.

---

## Tasks / Subtasks

- [x] **T0 — F1 RE-RULED FOURTH TIME on measurement (2026-09-02).** Five scouts; the sprint row's own
      priority candidate refuted; V1's direction inverted; F1b re-ruled **with** F1 as W2 demanded.
- [ ] **T1 — AC2 FIRST.** Raise `G` at `main.rs:9800`, both terms derived. Land the §7.2.1.a amendment
      **with this task**, not at the end — it gates the AC.
- [ ] T2 — AC1: bind `known_version`/`known_hash` at `state.rs:884`; per-peer table + reader; the
      proven-red vector.
- [ ] T3 — Capture the `TcpA2AConfig` clone at **`main.rs:9765`**, where `bootstrap.tcp` is still live
      one line before `bootstrap` moves at `:9769`. ⚠ **NOT at `main.rs:10080`**: that line is inside
      `build_cohort_a2a_daemon_runtime` (starts `:10024`), while the consumer `install_cert_rotation` is
      at `:9796` inside `run_cohort_a2a_daemon` (starts `:9627`) — **a capture in `:10080`'s scope cannot
      reach `:9796`** without widening `CohortDaemonRuntime`, which is unpriced.
- [ ] T4 — AC3: move arm (c) to the lib per the `check_j1_loopback_delegation` recipe; **bump
      `SCANNED_SOURCE_FILES` 17 → 18 in the same commit**; two dispositions (boot fatal / runtime refusal).
- [ ] T5 — AC4: the observation-driven deadline, F1b's four predicates, three falsifiers; outbox at
      `state.rs:487-514`, drain beside `main.rs:10134-10135`, **no `?` at the drain**.
- [ ] T6 — AC5: plane-C `RwLock` + `&self` setter; plane C **before** swap; real-gate harness (extend
      `build_mesh_n_with_gates` with provisioning); `RuptureReason` variant, ceiling unraised; scalar
      self row synthesised outside `peer_configs_for`.
- [ ] T7 — AC6: runtime leg, `MethodCallProbe`, new `t_14_2b_*` files, **both proven-red vectors executed**.
- [ ] T8 — `cargo fmt --all`, measure the FORMATTED tree, record EXACT MEASURED / ZERO HEADROOM per
      crate, **then** ask for grants.
- [ ] T9 — `RELEASE-HOLDS.md`: close c.2 + c.6, **fix the "Six boundaries" count (seven `c.N` labels
      exist)**, split c.4's statement, flip the stale `14-2b … (backlog)` at `:105`.
- [ ] T10 — Separate records: the §7.2.1.a `T_grace` amendment (with T1) and the FR23b revocation
      amendment on the **corrected ADR-047 §3/§4 citation**, each with its own delta line and a
      why-note.

---

## ⚠ FIFTH-PASS VALIDATION (two independent fresh-context validators, 2026-09-02) — NEEDS-REWORK

> Run under this story's own binding rule. **V1** audited mechanism correctness; **V2** audited
> dev-executability. Neither knew what the round-table decided. They converged independently on
> "not ready", and on the same two structural causes.

### Ship-blockers

| # | Finding | Measured |
|---|---|---|
| **SB-1** | **The timing is incoherent with itself, and the falsifier that should catch it cannot fire.** | `G ≥ skew` (AC2) vs `T_A = max(T_P) + I` (AC4) requires **`G ≥ skew + I`**; deficit one full `I`. And `spread(R_P) = spread(T_P) = skew ≤ G`, so `spread > G` is **false by construction**. Corrected in AC2/AC4 above; **the Rust expression is still unwritten.** *This is the pass's own defect: Part 1's one-cycle lag was measured and called "conservative", then spent directly out of Part 2's grace budget without being carried through.* |
| **SB-2** | **Raising `G` REDS a Blocking gate, and this story said it would not.** | `crates/maos-bin/tests/cert_rotation_trigger_14_2a.rs:720` **recomputes** `cold_deployment_t_grace()` (5 s) and budgets `acked_at + grace + 45 s` = **50 s** against a real daemon armed from `main.rs:9800`. It does **not** scale. Prior-pass V14 (*"it scales, so it stays green"*) is **REFUTED**. Folded into Blocking 7. |
| **SB-3** | **The budget cannot fund the scope.** | 14-2a — the peer half this story mirrors — landed **+3118 raw / +813 charged** in `maos-cohort`, **+1264 / +801** in `xtask`. This story has **+35 / +35 / +30 / +1**. `AC3`'s `*_short` fields alone touch **7 `CertRotationRefused` construction sites** (`state.rs:259`, `:504`; `rotation.rs:605`, `:820`, `:927`, `:1024`, `:1040`) ≈ 14 lines = 40 % of `maos-cohort`'s entire headroom on one sub-clause. `kloc.toml:60-65` forbids granting on an estimate, so T8 has the dev write hundreds of lines before learning if any is permitted. **This is a placement/scope decision, not a T8 detail.** |
| **SB-4** | **AC6 names no gate.** | No gate id, no `xtask/gate-registry.toml` row, no `BindingClass`. Blocking 11 says the story edits `xtask/src/main.rs` but nothing says register *what*. Given this project's documented gate-binding decay (E12-B1), an unregistered gate ships as decoration. Required shape at `gate-registry.toml:290-292` + the flat `gates` array at `:161`. |
| **SB-5** | **`MethodCallProbe` is private.** | `struct MethodCallProbe<'a>` (`check_cert_rotation_trigger.rs:145`), no `pub`. AC6's "reuse" is free *within* that file and requires a `pub(crate)` edit to a Blocking gate's source from a new file. **AC6 never picks the reading.** |
| **SB-6** | **`reachable` is inferred from ABSENCE, and the response is to SWAP.** | F1b defines reachable as *"a pull record within a derived freshness window"*; falsifier (ii) then makes A **swap** and name the excluded host. An off-path attacker who drops one peer's A-bound pulls — compromising nothing — manufactures "unreachable" and triggers a **remotely-inducible premature swap**, after which that peer refuses A (its `tcp.peer_pins` still names `F_old`, this story's own c.4). **Regresses ratified 12.3 doctrine P2: absence must be probed, not inferred.** |
| **SB-7** | **The peer-side scope arm is undeclared, and a mixed mesh silently bricks.** | Closure 1 declares daemon-only for the *rotating* host and says nothing about peers. The `maos run` cross-host arm binds via `TcpA2ATransport::bind(…, None, None)` (`main.rs:2457-2465`) — **no `CohortManifestGate`, no refresh loop**. Such a peer never pulls (→ unreachable → SB-6 → A swaps) and opens no rotation window (→ refuses `F_new` outright). **Declare "all cohort peers on the daemon arm" as a cut line, or AC4 is unsatisfiable.** |

### Majors folded into the ACs above

**V1-4** AC1's proven-red vector was unreachable and its negative vacuous — restated. **V1-5** *"never
over-reports"* is **false across a peer restart**: `apply_reissue` writes only `cached`
(`state.rs:519-524`), never disk, so a peer that applied N in memory and restarts reverts to its
on-disk version while A's table holds the `≥ N` high-water mark; `invalidate_if_boot_nonce_differs`
(`router.rs:1356-1358`) invalidates a *TOFU pin*, not this table. **The new table needs a staleness or
boot-nonce invalidation.** **V1-7** W1 relocated, not disposed (folded into AC4). **V1-8** enqueue ≠
decide (folded into AC4). **V2-F3** T3's capture site corrected to `main.rs:9765`. **V2-F8** the port
trait — name, signature, byte-vs-fingerprint return, installation path — **is still unspecified**, and
until it exists AC3's own anti-duplication clause is unsatisfiable. **V2-F10 / V1** *seven invented
constants* remain unnamed: `N`; the freshness-window and deadline-`D` source expressions; the
`RuptureReason` variant name; the `RotationWindowRow` field and JSON key; the `CertRotationRefused`
`*_short` field names.

### What both validators confirmed survives

The **(a)–(e) foundation is real**: `Pull` carries both fields, both senders populate them, the sole
receive site discards them, **no reader exists**, and `CohortManifestControl` lives in `maos-cohort` —
so the observation genuinely costs `maos-a2a-core` **zero**. The `Pull` arm **is** reached for every
configured peer (`is_reserved_cohort_intent` bypasses consent at `router.rs:1591`/`:761-764`), and
`verified_peer` **is** trustworthy on the daemon path (`handle_intake_verified` binds
`frame.from.host_id` to the TLS-verified peer at `router.rs:1760-1777`). The **live plane-C defect is
correctly traced end to end**. AC3's *"reuse arm (c)"* is exactly right. Plane C via `RwLock` is
implementable. The crate-edge truth table, Blocking 3/5/6/7/8, the `kloc-check` foreign-red disowning,
and every `RELEASE-HOLDS.md` claim all verified. Citation accuracy 34/37 in the scouted set — **by a
wide margin the best-measured of the four passes.**

### Corrections to this pass's own citations (V1 minors, applied)

`identity.rs` is in **`maos-a2a-core`**, not `maos-cohort` · `T_STALE_DEFAULT` is at **`manifest.rs:94`**,
not `:109-111` · `t_stale_secs` is **rejected**, not clamped (`ECohortStaleBoundViolation`) ·
`TokioGraceTimer` struct is at **`cert_rotation.rs:49`** (`:51` is the impl) · arm (c) closes at
**`:10005`** · `take_pull_requests` is **`:377-383`** · the `signed_fingerprint` closure is **7** lines
(`:9944-9950`) · `T_STALE_MIN/MAX` at **`:92-93`** · `kloc_check.rs` exclusion literal at **`:189`** ·
`check-kernel-baseline` **recurses** (`check_kernel_baseline.rs:105`) — the "one directory" phrasing is
wrong, the raw-count substance is right · `cohort_daemon_smoke_13_5c` has **four** executors, not three ·
`maos-control` also declares `ring` and `serde_json` · `verify_webpki_then_pin` is `:120-189` and
`reject` (`:195-201`) is shared by **both** dial- and listen-side arms, so AC5's dialer-asymmetry note
needs re-wording.
⚠ **V1-23 — F4 may be over-engineered**: the `peer` key **accommodates** a self row with `local_host`
as its value, so the scalar-field constraint guards a change the design may not need. Re-rule F4.
⚠ **V1-25 — arm (c) FAILS OPEN when the local host is absent from the manifest** (`if let
Some(signed_own)`, `main.rs:9989`). For a boot check that is defensible; **for a swap decision "no
signed fingerprint for me" must mean DO NOT SWAP.** Unstated, and inherited by the runtime driver.

### What the next pass must do

**Do not open with a design.** Three of four passes died on a mechanism ruled before it was measured;
this one died on a mechanism measured but **not carried through to the quantity it changed**. The next
pass starts by settling the two structural questions — **the split (SB-3)** and **the timing expression
(SB-1)** — and only then writes ACs.

---

## Appendix — the dead mechanisms, and why the record is kept

> A defect list deleted after the fix is a lesson nobody inherits. The full forensic file for passes
> one through three is preserved verbatim at
> `_bmad-output/implementation-artifacts/.archive/14-2b-pass-1-to-3.md`. Compressed here to what a
> future reader needs.

| # | Mechanism ratified | How it died | Pass |
|---|---|---|---|
| 1 | An in-mesh **probe** attached at `state.rs:487-514` | Not constructible **at the site AC1 named**: `apply_reissue` is a sync trait method holding a `std::sync::Mutex` guard; an async probe cannot await there. The real constraint turned out not to be asyncness but **crate placement** | 2026-08-31 → reopened |
| 2 | A **version-confirmed probe** over the reserved PULL intent | **V1**: `classify_presence` sends `known_version: self.state.version()` — **A's own** version. No wire path from P's version to A *in that direction* | 2026-09-01 |
| 3 | A bare **delay** of `confirmation_interval` after own-processing | **W2**: abandoning the probe deleted the only observation the design had, while every F1b disposition (deadline, quorum, confirmed-vs-unconfirmed) is built on observation. `AC3(c)`'s *"if skew > G, REFUSE"* had **no input** | 2026-09-01 |

**The pattern, named at pass three and acted on at pass four:** *ruling faster than measuring.* Three
designs were ratified by a round-table and then killed by a first contact with the code. The fourth was
not designed at all — it was **found**, by asking what observation exists before asking what mechanism
to build.

**The sharpest single lesson.** Mechanism 2 was retired on a true measurement of the **wrong
direction**. `known_version` really is A's own version when A probes P — and it is **P's** version when
P pulls from A, which is the direction that matters and the one nobody measured. A correct
measurement, applied to the wrong axis, killed the right mechanism for four weeks. When a scout returns
a refutation, check which way the arrow points.

**Validator findings retained as live constraints** (all hand-verified, all folded into the ACs and
Blocking conditions above): V4 (non-reentrant mutex — now Blocking 2/AC2), V6 (AST probe reads `main.rs`
only — AC6), V7 (the 50× fence is fiction — Blocking 6), V8 (self row crosses five crates — F4),
V11 (F3 sizing understated — Blocking 2), V12 (never `?` at the drain — AC4), V13 (no shipped helper
provisions *and* takes real gates — Blocking 3), V15 (F5's citation repaired — F5), W1 (frozen `G` vs
live interval — Part 3), W4 (the `SCANNED_SOURCE_FILES` bijection — Blocking 8), W5 (the null-control
falsifier — re-measured and confirmed).
**Disposed as refuted this pass:** V1 (wrong direction), V2 (applies only to reusing
`classify_probe_result`), V5/W6 (arm (c) is freely separable; `lib.rs` has an unconditional module),
W3 (four placements compile).

## Dev Agent Record

### Agent Model Used
_(record the exact model id from the frontmatter allowlist — `equiv` is prose and reds a Blocking gate)_

### Debug Log References

### Completion Notes List

### File List

### Change Log
