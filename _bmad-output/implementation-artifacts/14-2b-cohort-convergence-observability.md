---
baseline_commit: "`53845402` — HEAD of `main`, tree **DIRTY IN DOCS ONLY**; zero tracked source files modified (re-verified by five scouts and two validators, 2026-09-02)."
depends_on: "**`12-1`** (done, `8393b957`) — authored `CohortManifestControl` (`crates/maos-cohort/src/control.rs:14-22`), `CohortDistributor::pull_from` (`distribution.rs:62-74`) and the `pull_requests` receive arm (`crates/maos-cohort/src/state.rs:884`) this story completes. **`14-2a`** (done, `ee3422d0`) — authored the `RotationWindowSource` port shape (`crates/maos-control/src/lib.rs:77`, impl `crates/maos-bin/src/cert_rotation.rs:79`) that AC3 mirrors, and `GET /v1/a2a/rotation-windows` (`maos-control/src/lib.rs:206-253`) beside which AC3's route lands."
blocks: "**`14-2d-self-identity-rotation`** — its swap deadline is evaluated against this story's table, and cannot be written until the table exists. Does NOT block `14-2c`."
split_from: "**`14-2b-self-identity-rotation`**, split 3 ways 2026-09-02 (Lunarpulse, after two fresh-context validators returned 7 ship-blockers, of which SB-3 was that the scope could not be funded). Sibling rows: **`14-2c`** (the LIVE plane-C defect) and **`14-2d`** (the self-rotation swap + `G` + gate). Pre-split forensics: `_bmad-output/implementation-artifacts/.archive/14-2b-pass-4-presplit-source.md`; passes 1–3: `.archive/14-2b-pass-1-to-3.md`."
kernel_grant: "NONE and none needed. `check-kernel-baseline` **GREEN, EXIT 0, 24472 == 24472**, measured live 2026-09-02 by running the gate. Resolved from `xtask/kernel-core-baseline.toml:472`; never restate it as a literal. ⚠ The baseline is a **recursive** raw `.lines().count()` over `crates/maos-kernel-core/src` (`check_kernel_baseline.rs:105`) with **no exclusions — blanks and comments included**. A comment-only edit there reds it."
kloc_grant: "Measured live 2026-09-02 (`cargo run -q -p xtask -- kloc-check --json`, EXIT 1 on two foreign rows): **`maos-cohort` 5678 / 5713 = +35** · **`maos-control` 418 / 1500 = +1082** · **`maos-bin` 16941 / 16971 = +30** · **`xtask` 41897 / 41932 = +35**. ⚠ **`sprint-status.yaml:304`'s grant summary is WRONG on two rows** (it says cohort 5491 / xtask 41717; the live TOML and the live gate both say **5713** and **41932**). **All of `crates/*/tests/`, `xtask/tests/` and `xtask/src/tests/` are UNCHARGED** — tokei argv `-e target -e tests -e benches -e examples -e fuzz -e spirits` (`xtask/src/kloc_check.rs:167-191`); `-e tests` is a NAME match, not a path prefix. ⚠ **In-`src` `#[cfg(test)]` IS charged.** ⚠ **`maos-cohort`'s +35 is the binding constraint of this story** — AC1+AC2 must fit it or a MEASURED grant is asked at AC5, never before. `kloc.toml:60-65` forbids a grant on an estimate. ⚠ **Two FOREIGN reds, do not absorb:** `maos-domain 8695 > 8644` (D14, owner **14-7**) and `aggregate 154703 >= 147057` (D17, owner **14-6**). Review closure measured 2026-09-03 after formatting and Lunarpulse's apply-all authorization: **`maos-cohort` 5793 / 5793**, **`maos-bin` 16987 / 16987**, **`maos-a2a-core` 4853 / 4853** — exact measured, zero headroom. Final foreign reds only: `maos-domain 8695 > 8644` (14-7) and `aggregate 155093 >= 147057` (14-6)."
model: "frontier-class allowlist {opus-4-6, opus-4-7, opus-4-8, gpt-5.5, gpt-5.6, glm-5.1, glm-5.2, glm-5.3, opus-5, equiv}. `FRONTIER_FAMILIES` (`xtask/src/check_dev_model_tier.rs:45-48`) is the machine-checked set; **`equiv` is prose, NOT a match token** — recording it reds a Blocking gate. Keep the literal token `allowlist {` (`check_dev_model_used_populated.rs:302`)."
review: "§A6 full-layer net (Blind + Edge Case + Acceptance + Test-Infra + non-author runtime). **Test-Infra is load-bearing**: this story's whole deliverable is that a value stops being discarded, and the failure mode of such a story is a test that passes against an empty table. The non-author runtime layer MUST watch a planted mutation red the assertion live."
---

# 14-2b — Cohort convergence observability (retain the version peers already report)

Status: **done.**

> **The capability:** *an operator can see, on a live daemon, which cohort peers have converged on
> which signed manifest version — using a signal every peer already sends on every pull and this host
> currently throws away.*

---

## The finding this story exists to act on

A MAOS cohort host has, at HEAD, **no local observation of any kind** about whether a peer has
converged on a new signed manifest version. Not an approximate one, not a lower bound. This was
measured exhaustively (five scouts, 2026-09-02): every per-peer map in the cohort and A2A crates —
`pull_requests`, `halt_presence`, `halt_absence`, `outstanding_digest_reads`, `received_digest_summaries`,
`peers`, `pins`, `rotation_next` — is orthogonal to manifest version.

**And the signal is already on the wire.** `CohortManifestControl::Pull` carries the puller's exact
version and canonical hash:

```rust
pub enum CohortManifestControl {                  // crates/maos-cohort/src/control.rs:14-22
    Push { manifest_toml: String },
    Pull { known_version: u64, known_hash: String },
}
```

**Both senders populate them correctly** — `distribution.rs:66-67` (`pull_from`) and
`halt_receipt.rs:230-231`. The **sole** receive site discards both:

```rust
Ok(CohortManifestControl::Pull { .. }) => {       // crates/maos-cohort/src/state.rs:884
    self.pull_requests. … .push(verified_peer.clone());
    Ok(CohortReissueDisposition::PullRequested)
}
```

`verified_peer` — the **TLS-authenticated** identity — is in scope alongside both values.
**Exhaustive grep: `known_version` and `known_hash` have three references each (one declaration, two
senders) and ZERO READERS in the workspace.** The receive site does not even appear in the grep,
because `{ .. }` binds nothing.

⚠ **The value is authenticated, and this was checked.** The `Pull` arm is reached for every configured
peer: `is_reserved_cohort_intent` (`router.rs:761-764`) makes the reissue intent bypass the consent
block (`:1591`), landing at `router.rs:1666-1670`; and `verified_peer` derives from the peer binding
`handle_intake_verified` performs at `router.rs:1760-1777`, which requires
`frame.from.host_id == the TLS-verified peer`. (The bare `A2ARouter::handle_intake` at
`transport.rs:1381` does **not** bind identity — the TCP accept loop never uses it.)

⚠ **`pull_peers` is derived once at boot** (`main.rs:10050-10053`) from the operator TOML and never
recomputed, and each host pulls every peer on its own `confirmation_interval` cadence
(`main.rs:10137-10147`, re-armed at `:10146` as `now + confirmation_interval()`). So a member the
manifest ADDS is never pulled until a restart — **a real gap in this story's coverage, declared as a
cut line, not discovered later.**

---

## Blocking conditions

1. **⚠ A TABLE WITH NO PRODUCTION READER IS THIS REPO'S NAMED FAILURE MODE, AND IT IS WHY AC3 EXISTS.**
   `CohortManifest::peer_configs_for` (`manifest.rs:568`) was built by Story 12.1 *for exactly one
   purpose* and left with **zero production callers** until 14-2a found it dead two epics later. **A
   retention AC without a read AC repeats that defect precisely.** AC1 and AC3 ship together or the
   story does not ship.

2. **⚠ "A NEVER OVER-REPORTS" IS FALSE ACROSS A PEER RESTART, AND AC2 EXISTS FOR THIS.**
   `apply_reissue` writes only the in-memory cache (`state.rs:519-524`) — **there is no disk write**;
   boot re-reads `file.manifest_path` (`main.rs:9179`). A peer that applied version N in memory and
   then restarts **reverts to its on-disk version**, while this host's table still holds the `≥ N`
   high-water mark. ⚠ **The shipped restart detector does not cover this**:
   `invalidate_if_boot_nonce_differs` (`router.rs:1356-1358`) invalidates a **TOFU pin**, not a
   version record. AC2 must invalidate on its own terms.

3. **⚠ `maos-control` TAKES PLAIN SCALARS ON PURPOSE, AND ITS DEP LIST IS THE REASON.**
   `crates/maos-control/Cargo.toml` declares `maos-domain`, `maos-kernel-core`, `ring`, `serde_json` —
   and **none of `maos-cohort` / `maos-a2a-core` / `maos-a2a-tcp`**. `maos-control/src/lib.rs:32-35`
   states the rule: *"Plain owned scalars ON PURPOSE … the rotation read surface adds NO crate edge."*
   **Every field AC3 adds must be a `String`/`u64`/`bool`, never a typed cohort value**, and the
   crossing is done by a port trait implemented in `maos-bin` — the shipped `RotationWindowSource`
   shape (`maos-control/src/lib.rs:77`, impl `crates/maos-bin/src/cert_rotation.rs:79`). **Zero new
   crate edges.**

4. **⚠ A NEW `xtask` GATE IS UNAFFORDABLE AT +35.** 14-2a's `check_cert_rotation_trigger.rs` cost
   **945 raw lines**. Do not author a new gate. **Enroll into an existing leg-list gate** —
   `check-cohort-mesh` is the candidate (35 independent hard-fail legs, GREEN at HEAD, cohort-scoped).
   ⚠ **Verify its leg-add shape before writing the leg**, and do NOT enroll into
   `check-cert-rotation-trigger`: its `TEST_FILES` is a fixed 5-entry const whose
   `derive_rotation_tests_in` (`:722-754`) **hard-fails if any derived test is not leg-named**, so one
   new `#[test]` in those files reds a Blocking gate.

5. **⚠ FILE NAMING.** `check-rotation-real-timing` walks `crates/maos-a2a-tcp/tests/` for
   `t_10_4b_rotation_*` **or `t_14_2_`** (`check_rotation_real_timing.rs:354-355`) and enrollment is a
   **hard-fail** channel. `t_14_2b_*` does **not** match `t_14_2_` — verified — so that name is safe.
   **Never name a file `t_14_2_*`.**

6. **⚠ `all gates green` IS NOT AN AVAILABLE DONE CRITERION.** Measured 2026-09-02:
   `kloc-check` **1** (two foreign rows, above) · `check-multi-tenant-loom` **1** — ⚠ **and NOT for
   the reason prior records give.** It reds on `crates/maos-bin/tests/drain_once_audit_writer.rs:27`,
   *"cli_wrapper command 'worker-cli-fixture' not found"*: that binary is declared in
   `spirits/worker/Cargo.toml:12-13`, **`spirits/` is not a workspace member**, so `cargo test -p
   maos-bin` never builds it. `f4_pairing.rs:276` builds it explicitly as its own prerequisite;
   `drain_once_audit_writer.rs` does not. **A test-infra/build-ordering defect, foreign to this
   story.** · `check-dev-record-completeness` **1** (`deferred-work.md:860` stale owner `14-1`) —
   ⚠ **CORRECTED 2026-09-02 by the `14-2c` pass: it is NOT induced by the uncommitted
   `sprint-status.yaml`.** A live re-run gives exactly one violation, and the row's own text names its
   successor: *"Owner: 14-1 follow-up or **14-6** (instrument work)"*. **Owner is 14-6**; do not
   re-investigate. GREEN at HEAD: `check-kernel-baseline`, `check-cert-rotation-trigger`,
   `check-rotation-real-timing`, `check-cohort-mesh`, `check-scale-churn`, `check-epic-close-coherence`
   (which reports **14** epics, not the 15 an earlier note claimed). **Disown each red with a named
   owner; absorb none; do not restate an expected end position measurement contradicts.**

---

## Acceptance Criteria (5)

**AC1 — The version peers already report is retained, authenticated, per-peer.**
At `crates/maos-cohort/src/state.rs:884` the `Pull` arm binds `known_version` and `known_hash` instead
of `{ .. }`, and records them against `verified_peer` — with the observation instant from the shipped
`CohortClock` (`state.rs:25`), never `SystemTime::now()` — in a per-peer table beside `pull_requests`
(`state.rs:98`). The implementation initially required ZERO `maos-a2a-core` delta; review found that
retention must also carry the request-authenticated nonce to avoid re-sampling mutable pin state.
Lunarpulse authorized that measured interface delta. No new wire field, intent, or reserved name.
🔴 **Proven-red vector, stated as a full revert:** remove the record write → the AC4 observation test
reds. ⚠ **Do NOT state the mutation as "restore `{ .. }`"** — that alone leaves the bindings unresolved
at the insert and produces **E0425, a build failure, not a test red**.

**AC2 — A stale record is not a convergence claim.**
A retained record is invalidated — not merely aged — when it can no longer be trusted:
**(a)** on peer restart, and **(b)** past a staleness bound derived from `confirmation_interval()`
(`state.rs:614-620`), never a literal and never a new constant.
⚠ **(a) is the ship-blocker half** (Blocking 2): a peer that applied N in memory and restarted reverts
to its on-disk manifest while this host still holds `≥ N`. The shipped `boot_nonce` on `TofuPin`
(`tofu.rs:24`) is the available restart signal; `invalidate_if_boot_nonce_differs`
(`router.rs:1356-1358`) invalidates the **pin**, not a version record, so this AC must act on its own
terms. **A record that survives a peer restart is a false convergence claim, and the story's whole
value is that it is not one.**

**AC3 — The operator can read it on a live daemon.**
A read-only, authenticated operator surface reports, per peer: the last version that peer declared, its
canonical hash, when it was observed, and whether the record is currently valid per AC2. It lands
beside `GET /v1/a2a/rotation-windows` and follows the shipped port shape exactly — a trait declared in
`maos-control` and implemented in `maos-bin` over `CohortManifestState`, mirroring `RotationWindowSource`
(`maos-control/src/lib.rs:77` → `crates/maos-bin/src/cert_rotation.rs:79`). **Plain scalars only; zero
new crate edges** (Blocking 3). Reuse the route's existing auth gate (`maos-control/src/lib.rs:193-205`)
— **do not invent a second authentication path.**
⚠ **Distinguish "no record" from "converged"**: an empty result must not render as agreement. This is
the same defect class as `Some(Healthy(vec![]))` meaning *"control installed, nothing in flight"* on the
rotation route (`maos-control/src/lib.rs:79-80`).

**AC4 — Proven against real behaviour, and the negative cannot pass on an empty table.**
A live multi-host test in which peers at **different** versions pull, and the table and the AC3 surface
**distinguish them** — not merely "a record exists". Plus: a restarted peer's record is invalidated
(AC2a), and a stale record is invalidated (AC2b).
🔴 **The negative must be one an empty table cannot satisfy.** *"A peer that never pulls has no record"*
is **vacuous** — an empty table satisfies it under every mutation of AC1. Assert instead that two peers
at different versions are told apart.
Tests land in **NEW files named `t_14_2b_*`** (Blocking 5). Test lanes are kloc-free.

**AC5 — Enrolled in a gate that reds, budget measured, reds disowned.**
Enroll AC4's tests as legs of an existing leg-list gate (Blocking 4 — `check-cohort-mesh` candidate,
verify its leg-add shape first). **Do not author a new `xtask` gate** (+35 headroom vs 945 lines of
precedent) and **do not enroll into `check-cert-rotation-trigger`**. Execute and record **both**
proven-red vectors: reverting AC1's write reds the leg; an AC2 invalidation removed reds the
restart/staleness leg. Then `cargo fmt --all`, measure the **formatted** tree, record **EXACT MEASURED /
ZERO HEADROOM** per crate — and only then ask for any grant (`kloc.toml:60-65`). Disown every
pre-existing red with the named owner and the diagnosis in Blocking 6. **ZERO kernel-Δ @ 24472.**

---

## Declared cut lines

- **A member the manifest ADDS is never observed until a restart** — `pull_peers` is boot-derived
  (`main.rs:10050-10053`) and never recomputed. Declared, not fixed.
- **The observation is a LOWER BOUND, one pull cycle late.** P reports its pre-push version and only
  declares `≥ N` on its next tick (`R_P = T_P + I`). ⚠ **This lag is load-bearing for `14-2d` and must
  be carried into its `G` derivation** — it is the exact quantity that made the pre-split story's
  timing incoherent (SB-1). This story states the lag; it does not consume it.
- **A peer's version is a SELF-REPORT.** It is authenticated as *coming from* that peer, not verified
  as true. A peer lying **high** is bounded; a peer lying **low** can, once `14-2d` exists, **veto that
  host's rotation indefinitely**. This story only displays the value, so the exposure is informational —
  **`14-2d` inherits it as a security question and must rule on it.**
- **`maos run` cross-host peers are out of scope.** That arm binds with **no `CohortManifestGate` and no
  refresh loop** (`main.rs:2457-2465`), so such a peer never pulls and can never be observed. **Cohort
  daemon arm only.**

## Dev notes

**Placement.** Retention and invalidation stay in `maos-cohort`; request-generation transport stays in
`maos-a2a-core`; the read port stays in roomy `maos-control` with its production impl in `maos-bin`.
Review grants are exact measured / zero headroom and recorded below. The `pull_requests`
field/push/drain trio remains the retention idiom.

**Do not touch:** `maos-domain` (RED −51, owner 14-7); `maos-kernel-core` (a comment-only edit reds the
baseline). **Two exact-count source gates fence `router.rs`**: `cohort_manifest_chokepoint_12_1.rs:24-31`
and `cohort_consent_chokepoint_12_2.rs:30-37` both assert `consent_and_team(` count **== 2**.

**Adding a file to `crates/maos-bin/src/` reds a Blocking gate** — `cohort_daemon_smoke_13_5c.rs:861-863`
declares `SCANNED_SOURCE_FILES: [(&str, &str); 17]` and `:937-941` asserts the length equals a live
flat `read_dir` (**17 == 17** today), under **four** executors. The assert's own message says the fix:
list the file. Bump `17 → 18` in the same commit — a test-file edit, uncharged. **Prefer extending
`cert_rotation.rs`** and avoid the question entirely.

**One shipped defect found in passing, not this story's to fix:** `PeerCertFingerprint` deserialisation
is unvalidated and case-sensitive (`crates/maos-a2a-core/src/identity.rs:49-57` — **`maos-a2a-core`, not
`maos-cohort`**); `parse()` at `:75-88` is off the serde path.

## Tasks / Subtasks

- [x] T1 — Read the `pull_requests` trio (`state.rs:98`, `:884-892`, `:377-383`) and mirror its idiom.
- [x] T2 — AC1: bind at `state.rs:884`, record against `verified_peer` with the `CohortClock` instant.
- [x] T3 — AC2: restart invalidation (a) **first** — it is the ship-blocker half — then staleness (b),
      derived from `confirmation_interval()`, never a literal.
- [x] T4 — AC3: the `maos-control` port + `maos-bin` impl + route, scalars only, existing auth gate,
      empty ≠ converged.
- [x] T5 — AC4: multi-host test at **differing** versions; the non-vacuous negative; `t_14_2b_*` files.
- [x] T6 — AC5: leg enrollment (verify the shape first), **both proven-red vectors EXECUTED**.
- [x] T7 — `cargo fmt --all`, measure the formatted tree, record EXACT MEASURED / ZERO HEADROOM, then
      ask for any grant. Disown the pre-existing reds with owners and the Blocking-6 diagnoses.
      **Grant ASKED and then AUTHORIZED (Lunarpulse, 2026-09-02); applied at EXACT MEASURED / ZERO
      HEADROOM. `kloc-check` now reds ONLY the two foreign rows.**

### Review Findings

- [x] [Review][Patch] **[HIGH] Carry the authenticated request generation across concurrent restart invalidation** [`crates/maos-cohort/src/state.rs:460`] — fixed by threading the request-authenticated nonce through `CohortManifestGate`.
- [x] [Review][Patch] **[MEDIUM] Omit removed members from convergence observations** [`crates/maos-cohort/src/state.rs:1030`] — fixed by rejecting refreshes and filtering retained rows against the current signed cohort.
- [x] [Review][Patch] **[HIGH] Fail closed for generationless startup observations** [`crates/maos-cohort/src/state.rs:460`] — fixed; zero-generation pulls are served but never retained.
- [x] [Review][Patch] **[HIGH] Persist each observation's exact stale deadline** [`crates/maos-cohort/src/state.rs:493`] — fixed with immutable `expires_at_secs` captured from the signed lease at observation time.
- [x] [Review][Patch] **[HIGH] Exercise the production AC3 adapter and HTTP reader in the enrolled live gate** [`crates/maos-bin/tests/t_14_2b_cohort_convergence.rs:437`] — fixed with `CohortPeerVersions` plus authenticated live HTTP assertions.
- [x] [Review][Patch] **[HIGH] Drive restart invalidation through the live router** [`crates/maos-bin/tests/t_14_2b_cohort_convergence.rs:586`] — fixed with a changed-nonce mTLS sender and live-router NACK.
- [x] [Review][Patch] **[MEDIUM] Hide convergence outside cohort-daemon mode** [`crates/maos-bin/src/cert_rotation.rs:129`] — fixed by refusing the source until the rotation observer is installed.
- [x] [Review][Patch] **[MEDIUM] Move the sprint proposal into the configured planning-artifact root** [`_bmad-output/planning-artifacts/sprint-change-proposal-2026-09-01.md:1`] — fixed.
- [x] [Review][Patch] **[MEDIUM] Reconcile the Epic 14 count with its split-story inventory** [`_bmad-output/planning-artifacts/epics/index.md:143`] — fixed at 17 stories.
- [x] [Review][Patch] **[MEDIUM] Keep backlog model-pin claims in future tense** [`_bmad-output/planning-artifacts/epics/epic-14-scale-closers-ecosystem-readiness-v2-2.md:48`] — fixed.

## Dev Agent Record

### Agent Model Used

`opus-5` — frontier-class allowlist {opus-4-6, opus-4-7, opus-4-8, gpt-5.5, gpt-5.6, glm-5.1,
glm-5.2, glm-5.3, opus-5}. Exact provider id `anthropic/claude-opus-5`.

### Debug Log References

**Implementation measurements were taken live 2026-09-02; review measurements were taken live
2026-09-03. Both trees were `cargo fmt --all` formatted first. None is an estimate.**

**Proven-red vectors — ALL THREE EXECUTED, all EXIT 101.**

| Vector | Mutation (applied, run, reverted) | Leg | Result |
|---|---|---|---|
| AC1 write, FULL revert | `state.rs` Pull arm back to `Ok(CohortManifestControl::Pull { .. }) => {` **and** the `record_peer_convergence` call deleted — the full revert, because restoring `{ .. }` alone is E0425, a build failure and not a test red | `convergence-versions-told-apart` | **RED**, `exactly the two peers that pulled are retained … : []` — the empty table, exactly |
| AC2a invalidation | `let restarted = false;` | `convergence-record-invalidation` | **RED**, `left: "observed"` / `right: "restarted"` on `host_b` at `pin_generation: 2` |
| AC2b invalidation | staleness branch condition → `false` | `convergence-record-invalidation` | **RED**, `left: "observed"` / `right: "stale"` on `host_c` past the derived bound |

**Gate measurements at completion.**

| Gate | Exit | Note |
|---|---|---|
| `check-cohort-mesh` | **0** | `PASSED (37 independent hard-fail legs)` — **35 → 37**, the two new convergence legs |
| `check-kernel-baseline` | **0** | `24472 lines, pinned 24472` — **ZERO kernel-Δ**, as required |
| `check-cert-rotation-trigger` | **0** | `oracle green (4 legs, 39 enrolled rotation tests)` — the gate Blocking 4 forbade enrolling into stayed green |
| `check-rotation-real-timing` | **0** | `t_14_2b_*` does **not** match its `t_14_2_` enrollment glob — verified by running it, not by reading it |
| `check-scale-churn` | **0** | — |
| `check-epic-close-coherence` | **0** | — |
| `kloc-check` | **1** | Final review measurements fit their exact grants; only the two foreign rows remain |
| `check-multi-tenant-loom` | **1** | **DISOWNED, foreign.** Exactly the Blocking-6 diagnosis: `drain_once_audit_writer.rs:27` cannot find `worker-cli-fixture` because `spirits/` is not a workspace member and that test, unlike `f4_pairing.rs:276`, does not build it as its own prerequisite. Test-infra/build-ordering defect. |
| `check-dev-record-completeness` | **1** | **DISOWNED, foreign.** One violation, `deferred-work.md:860`; the row's own text names **14-6** ("instrument work"). Not re-investigated, per Blocking 6. |

**One PRE-EXISTING test-isolation defect found in passing, disowned with evidence.**
`cargo test -p maos-a2a-tcp` reds `t_14_2a_a_promoted_generation_makes_the_retired_leaf_a_queryable_refusal`
with `captured tracing: []`. It is **not this story's**: it passes ALONE and passes under
`--test-threads=1`, and it fails only when the file's three tracing-capture tests race for the
process-global subscriber. Its owning Blocking gate `check-cert-rotation-trigger` runs each as its own
single-test leg and is GREEN. Owner: **14-2a test infra.** Nothing in this story's diff is on the
post-grace journal path.

### Completion Notes List

**AC1 — SATISFIED.** `crates/maos-cohort/src/state.rs` binds `known_version` and `known_hash` at the
sole receive site and records them against the TLS-verified `verified_peer` with the injected
`CohortClock` instant, in `peer_convergence: Mutex<BTreeMap<String, PeerConvergence>>` beside
`pull_requests`. The request-authenticated `boot_nonce` is carried through `CohortManifestGate`;
retention never re-samples mutable pin state after admission. `BTreeMap` keeps the operator-facing
list deterministic without sorting on every read.

**AC2 — SATISFIED.** The restart signal is the exact `A2AJsonRpcRequest.boot_nonce` authenticated for
the Pull being handled. `A2ARouterCore` passes that nonce through the gate, so an old-generation Pull
cannot race a concurrent invalidate/re-pin and inherit the new generation. A zero nonce remains the
wire's “generation unavailable” sentinel: the Pull is served but never retained as a convergence
claim. Every retained row stores an immutable `expires_at_secs`, computed once from the signed
`t_stale_secs` active when the observation arrived; a later longer lease cannot revive it. Read-time
restart comparison still covers both invalidated-pin `None` and a later re-pin, and restart outranks
staleness as the actionable state.

The read projection also intersects retained rows with the current signed member set. Applying a
manifest that removes a peer hides its old row immediately, and a later reserved Pull from that
boot-static peer cannot refresh it back into the table. The live test proves the inclusive expiry
boundary, restart precedence, lease non-extension, removal, and non-refresh.

**AC3 — SATISFIED, zero new crate edges.** `CohortConvergenceSource` + `PeerVersionRow` +
`PeerVersionStatus` in `maos-control` (`String`/`u64`/`bool` only; `Cargo.toml` **unchanged**),
implemented by `CohortPeerVersions` in `crates/maos-bin/src/cert_rotation.rs`. The production adapter
returns `None` until the daemon's rotation observer is installed, so a loaded manifest outside the
cohort-daemon composition cannot render a false healthy-empty result. `GET /v1/cohort/peer-versions`
reuses the existing bearer gate; the live test exercises unauthorized, authorized, and post-restart
responses through `OperatorHttpServer`. No state is **404**, unreadable state is **503**, and an
available observer with no peer observations is **200 `{"peer_versions":[]}`**.

**AC4 — SATISFIED, and the negative is the non-vacuous one.**
`crates/maos-bin/tests/t_14_2b_cohort_convergence.rs` stands **three** hosts on real mTLS over real
TCP with the shipped `CohortDistributor` and the shipped gate: `host_a` at v3, `host_b` at v1, `host_c`
at v2, both members pulling. The assertion is that the table and the surface **tell v1 and v2 apart by
version AND by canonical hash** — the vacuous *"a peer that never pulls has no record"* is not used
anywhere. The invalidation test asserts that a restart invalidates **exactly one of two live records**
while the sibling stays valid, so an empty table has neither a row to invalidate nor a row to leave
standing. `install_cert_rotation` is exercised because it is what makes the pin generation reachable —
the same call `main.rs` makes on the cohort-daemon arm, not scaffolding.

**AC5 — leg enrollment, proven-red vectors, review repairs, and measured grants DONE.**
Enrolled as two `Leg` literals in the existing `check-cohort-mesh` (leg-add shape verified first:
`struct Leg { name, args }`, a local array, `run_leg` requires `running 1 test` + `1 passed`, and —
definitively — it has **no** derived-enrollment hard-fail, unlike `check_cert_rotation_trigger.rs`'s
`derive_rotation_tests_in`). Gate went 35 → **37** legs, EXIT 0. No new `xtask` gate authored.

**EXACT MEASURED / ZERO HEADROOM — `cargo fmt --all` first, because fmt is what CI measures.**

| Crate | Pre-story budget | **MEASURED (formatted)** | Δ vs budget | Disposition |
|---|---|---|---|---|
| `maos-cohort` | 5713 | **5793** | −80 | **GRANTED in two measured steps: 5752 implementation, 5793 review; ZERO HEADROOM** |
| `maos-bin` | 16971 | **16987** | −16 | **GRANTED in two measured steps: 16984 implementation, 16987 review; ZERO HEADROOM** |
| `maos-a2a-core` | 4850 | **4853** | −3 | **REVIEW GRANT: exact request generation threading; ZERO HEADROOM** |
| `maos-control` | 1500 | **615** | +885 | FITS, no ask |
| `xtask` | 41932 | **41925** | +7 | FITS, no ask |
| `maos-kernel-core` | pinned 24472 | **24472** | 0 | **ZERO kernel-Δ** |
| `maos-domain` | 8644 | **8695** | −51 | **DISOWNED — owner 14-7** |
| aggregate | 147057 | **155093** | over | **DISOWNED — owner 14-6** |

The 2026-09-03 review grants were authorized by Lunarpulse's “apply every patch” decision only after
`cargo fmt --all` and live measurement: `maos-cohort` 5752 → 5793 (+41), `maos-bin` 16984 → 16987
(+3), and `maos-a2a-core` 4850 → 4853 (+3). Each is recorded in `xtask/kloc.toml` at EXACT MEASURED /
ZERO HEADROOM. Final `kloc-check` reports only the pre-existing `maos-domain` and aggregate rows.

### File List

| File | Change |
|---|---|
| `crates/maos-cohort/src/state.rs` | AC1 + AC2 and review repairs — retained generation/deadline, restart/stale evaluation, current-member filtering |
| `crates/maos-cohort/src/rotation.rs` | AC2a — `PeerCertRotation::pin_generation`, the shipped restart signal read at evaluation time |
| `crates/maos-cohort/src/lib.rs` | Re-exports `PeerConvergence` and the three `CONVERGENCE_*` constants |
| `crates/maos-a2a-core/src/cohort.rs` | Carries the exact authenticated request generation through `CohortManifestGate` |
| `crates/maos-a2a-core/src/router.rs` | Passes `request.boot_nonce` to the reissue gate; migrates the test gate impl |
| `crates/maos-a2a-core/tests/cohort_consent_router_12_2.rs` | Migrates the cohort gate test impl |
| `crates/maos-control/src/lib.rs` | AC3 port, scalars, authenticated route, and route tests |
| `crates/maos-bin/src/cert_rotation.rs` | AC3 production adapter; hides convergence until observer installation |
| `crates/maos-bin/src/main.rs` | AC3 composition-root wiring from the same `bootstrap.state` |
| `crates/maos-bin/Cargo.toml` | Test-only dependencies for the live production-surface convergence suite |
| `crates/maos-bin/tests/t_14_2b_cohort_convergence.rs` | Live three-host mTLS, router restart, production adapter/HTTP, expiry, and membership tests |
| `crates/maos-bin/tests/team_identity_13_6a.rs` | Migrates the cohort gate test impl |
| `xtask/src/check_cohort_mesh.rs` | Enrolls both convergence tests against their production-owning package |
| `xtask/kloc.toml` | Exact measured implementation and review grants |
| `_bmad-output/planning-artifacts/sprint-change-proposal-2026-09-01.md` | Moved from the nested misplaced planning root |
| `_bmad-output/planning-artifacts/epics/index.md` | Reconciles Epic 14 to 17 stories |
| `_bmad-output/planning-artifacts/epics/epic-14-scale-closers-ecosystem-readiness-v2-2.md` | Corrects backlog model-pin claims to future tense |
| `_bmad-output/implementation-artifacts/sprint-status.yaml` | Status and reconciled Epic 14 inventory |

### Change Log

| Date | Change |
|---|---|
| 2026-09-03 | **CODE REVIEW COMPLETE — 10/10 findings fixed.** Exact request-generation threading closes the concurrent restart/re-pin race; zero-generation observations fail closed; signed expiry deadlines are immutable; current membership bounds both reads and refreshes; the enrolled live suite now exercises the production adapter, authenticated HTTP route, and live router restart NACK. Planning records were reconciled. Formatted measurements: `maos-a2a-core` 4853, `maos-bin` 16987, `maos-cohort` 5793; exact review grants applied with zero headroom. `check-cohort-mesh` passes 37/37; `kloc-check` retains only the two pre-existing owned reds. |
| 2026-09-02 | **IMPLEMENTED (anthropic/claude-opus-5), T1–T6 complete.** The version every cohort peer already puts on the wire is retained per authenticated peer, invalidated exactly on peer restart (shipped `TofuPin.boot_nonce`, compared at read, invalidated-pin-as-`None`) and past a bound derived from `confirmation_interval()`, and readable on a live daemon at `GET /v1/cohort/peer-versions` through a scalar-only port with ZERO new crate edges. `known_version`/`known_hash` went from three references and **zero readers** to a production reader. Proven with two live three-host mTLS tests whose assertions an empty table cannot satisfy, enrolled as `check-cohort-mesh` legs (35 → **37**, EXIT 0). **All three proven-red vectors executed, EXIT 101.** ZERO kernel-Δ @ 24472; ZERO `maos-a2a-core` delta. |
| 2026-09-02 | **T7 COMPLETE — measured kloc grant ASKED, AUTHORIZED and APPLIED at EXACT MEASURED / ZERO HEADROOM.** `cargo fmt --all` first, then measured: `maos-cohort` 5713 → **5752** (+74 charged) and `maos-bin` 16971 → **16984** (+43 charged), both recorded in `xtask/kloc.toml` with named grant rows — **deliberately NOT the `measured + 35` operating slack 14-2a took**, so this story leaves zero pre-approved capacity behind it. `maos-control` 615/1500 and `xtask` 41925/41932 needed no ask. `kloc-check` now reds ONLY `maos-domain 8695 > 8644` (**owner 14-7**) and `aggregate 155045 >= 147057` (**owner 14-6**, already red at HEAD at 154703; this story's +342 does not change its ownership and was NOT absorbed). `check-multi-tenant-loom` (1) and `check-dev-record-completeness` (1, `deferred-work.md:860` STALE owner `14-1` → **14-6**) re-measured live and disowned with the Blocking-6 diagnoses. |
