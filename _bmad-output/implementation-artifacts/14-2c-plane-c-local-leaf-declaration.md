---
baseline_commit: "**`4657cace`** (`14-2b-cohort-convergence-observability`) — ⚠ **RE-ANCHORED 2026-09-03: `14-2b` COMMITTED MID-SESSION, so Blocking 0 is DISCHARGED and every measurement in this story is now citable to a SHA.** (Was `53845402` + an uncommitted tree.) ⚠⚠ **RE-MEASURED 2026-09-03 (round-table). The 2026-09-02 drift table published here has itself DRIFTED and IS DELETED — it was measured mid-dev-pass and the dev pass kept going.** Measured then: `apply_reissue` `:388`->`:534`; measured now: **`:559`**. Then `:503-510`->`:649-656`; now the `CertRotationRefused` arm is at **`:675`**. Then `:246-266`->`:312-332`; now `install_cert_rotation` opens at **`:272`** and its refusal row is at **`:325`**. **A CORRECTION TO A STALE CITATION IS ITSELF A CITATION AND GOES STALE THE SAME WAY — which is worse than the original, because it reads like the fix.** ⇒ **RULING R6: NEVER cite a line number in this story again. Cite the SYMBOL and grep for it.** The three sites this story touches are `fn apply_reissue`, `fn install_cert_rotation`, and `CohortAuditEvent::CertRotationRefused` (TWO occurrences in `state.rs`, one per site — that pair IS AC1's two sites). Names do not drift. `14-2b`'s dev pass (11 files, +589 insertions, `maos-cohort`/`maos-a2a-core`/`maos-control`/`maos-bin`/`xtask`) — SUPERSEDED 2026-09-04: that dev pass IS the `4657cace` commit named at the head of this field (the sentence was written pre-commit and is retained as history; the re-anchor at the top governs); `cargo fmt --all --check` EXIT 0, so the measurements below are real, but they are real against an editor rather than a SHA. See Blocking 0."
depends_on: "**NOTHING.** This story is unblocked and does not depend on `14-2b` or `14-2d`. It reads state that already exists and writes no trust plane."
blocks: "**`14-2d-self-identity-rotation`** — 14-2d writes plane C, and it must not write it into a host that cannot observe or report its own identity. This story builds that observation."
split_from: "**`14-2b-self-identity-rotation`**, split 3 ways 2026-09-02 (Lunarpulse). Siblings: **`14-2b-cohort-convergence-observability`** (`ready-for-dev`) and **`14-2d-self-identity-rotation`** (`backlog`). Pre-split forensics: `.archive/14-2b-pass-4-presplit-source.md` and `.archive/14-2b-pass-1-to-3.md`."
kernel_grant: "NONE and none needed. Nothing here touches `crates/maos-kernel-core/src`. ⚠ `check-kernel-baseline` is a **recursive** raw `.lines().count()` with **no exclusions — blanks and comments included** (`check_kernel_baseline.rs:109`, reached by the recursion at `:105`); the comparison is `actual == pinned` (`:65`), so **drift in EITHER direction reds** — resolve the pin from `xtask/kernel-core-baseline.toml`, never restate it as a literal. ⚠ A new audit *kind* would also be ZERO kernel-Δ (kinds 30/31 ship as raw ints with no `FrameKind` variant, `maos-audit/src/lib.rs:694-698`) — but see `kloc_grant`: `maos-audit` has ZERO headroom, so that route is closed on budget, not on kernel."
kloc_grant: "⚠⚠ **RE-MEASURED LIVE 2026-09-03** (`cargo run -q -p xtask -- kloc-check --json`, fmt-clean tree, `14-2b` landed). **EVERY NUMBER IN THE 2026-09-02 GRANT SECTION IS EXPIRED. FOUR CRATES ARE AT ZERO OR EFFECTIVELY ZERO, NOT ONE.** `maos-cohort` **5793 / 5793 = ZERO** (was 5678/5713 = +35) · `maos-a2a-core` **4853 / 4853 = ZERO** (was 4849/4850 = +1) · `maos-bin` **16987 / 16987 = ZERO** (was +30) · `xtask` **41925 / 41932 = +7** (was +35) **against a ~14-charged-line `check-cohort-mesh` leg — measured from `14-2b`'s own append: 2 legs, 38 raw lines, `xtask` 41897->41925 = +28 charged, so ONE leg = +14. AC5 DOES NOT FIT EITHER.** · `maos-control` **615 / 1500 = +885** — the ONLY crate with room, and the one crate where the price of this exact shape is now MEASURED rather than estimated: `14-2b` paid **+197 charged** for a trait + a route + three in-`src` doubles + three tests. · `maos-audit` 6847/6847 ZERO · `maos-iac` 6960/6960 ZERO. 🔴 **R1 — THE `maos-a2a-core` 'ALREADY AUTHORIZED' CLAIM IS DEAD. `14-2b` SPENT IT.** The 2026-08-31 authorization was discharged by a DIFFERENT story's ceiling edit, `kloc.toml` `maos-a2a-core = 4850 -> 4853`, measured on the `peer_boot_nonce` trait parameter. This story asserted the grant was held for it in FOUR places; nobody was holding it. **A GRANT IS A GLOBAL, NOT A RESERVATION — a pre-authorized ceiling is consumed by whichever story measures first.** ⇒ AC5 owes a **FRESH MEASURED ASK on all four crates**, not a ceiling edit. **FUNDING RATIFIED 2026-09-03 (Lunarpulse): GRANT ALL FOUR, MEASURED — 6 ACs stay whole.** Discipline unchanged and non-negotiable: `kloc.toml:60-65`, code exists and is `cargo fmt --all`-formatted BEFORE the ask, EXACT MEASURED / ZERO HEADROOM per crate, cited under `kloc.toml:84-87` with the `kloc.toml:303` and the two-day-old `14-2a`/`14-2b` precedents. **NEVER cite `kloc.toml:269`** (anti-masking; carries no ship-red sanction). ⚠ **All of `crates/*/tests/`, `xtask/tests/` and `xtask/src/tests/` are UNCHARGED** (`kloc_check.rs:167-191`; `-e tests` is a NAME match, not a path prefix); **in-`src` `#[cfg(test)]` IS charged** — which is why `14-2b`'s `maos-control` cost was +197 and not +60. ⚠ **TWO FOREIGN reds, RE-MEASURED, do not absorb, do not repair:** `maos-domain 8695 > 8644` (D14, owner **14-7**) and `_aggregate_hardfail` **155093 >= 147057 = −8036** (D17, owner **14-6**) — **the aggregate red DEEPENED from −7646 by `14-2b`'s +390; state the new number and the three-way split, never the stale one.** **GRANTED EXACT-MEASURED 2026-09-03 after `cargo fmt --all`: `maos-cohort` 5793→5841 (+48), `maos-a2a-core` 4853→4856 (+3), `maos-bin` 16987→17107 (+120), `xtask` 41932→41939 (+7 ceiling; story code 41925→41939, +14), all ZERO HEADROOM. `maos-control` moved 615→803 (+188) inside its existing ceiling. Foreign reds remain: `maos-domain` 8695>8644 (−51, owner 14-7); aggregate 155466≥147057 (−8409, owner 14-6), split as D17 −7646 + `14-2b` −390 + this story +373.**"
model: "frontier-class allowlist {opus-4-6, opus-4-7, opus-4-8, gpt-5.5, gpt-5.6, glm-5.1, glm-5.2, glm-5.3, opus-5, equiv}. `FRONTIER_FAMILIES` (`xtask/src/check_dev_model_tier.rs:45-48`) is the machine-checked set; **`equiv` is prose, NOT a match token**. Keep the literal `allowlist {` (`check_dev_model_used_populated.rs:302`)."
review: "§A6 full-layer net (Blind + Edge Case + Acceptance + Test-Infra + non-author runtime). **Test-Infra is load-bearing and this story proves why:** the closest existing test to its subject, `emitter_with_a_stale_local_leaf_cannot_originate_a_crossing` (`crates/maos-bin/tests/team_identity_13_6a.rs:516-537`), is a **null control for the runtime defect** — it injects the divergence at construction on a bare `A2ARouterCore` with no transport, never runs a reissue, and would stay green under every mutation this story could make. The non-author runtime layer MUST watch a planted mutation red a live assertion."
---

# 14-2c — The host cannot see its own identity move

Status: **done.** All six ACs and Tasks T0–T8 are complete; §A6 review closed
2026-09-04 (4 layers + non-author runtime, 3 mutations red, 9 patches applied and
verified, gate green at 39 legs). Plane C remains read-only; Story 14-2d is
unblocked and owns production self-rotation.

> **The capability:** *a cohort host detects, journals and reports the moment a signed manifest moves
> its OWN certificate fingerprint — instead of being cut off from the entire mesh seconds later, in
> both directions, with no local signal and an error message that names the wrong cause.*

---

## The finding this story exists to act on

### 1. The defect is a TOTAL PARTITION, and every attempt to weaken that claim failed

A signed `cohort:manifest-reissue` that moves `members[local_host].fingerprint` from `F_old` to
`F_new` is accepted by every host including this one. On **this** host nothing happens at all:

| # | Step | Measured at |
|---|---|---|
| 1 | `apply_reissue` accepts it. Validation is signature/authority, `cohort_id`, schema, monotonicity — **nothing else**. `parse_and_validate` takes only `(src, pinned)`; it has **no `local_host` parameter**, so a self-check is structurally impossible there. | `maos-cohort/src/state.rs:388-538`, `manifest.rs:357-360` |
| 2 | The rotation projection is `peer_configs_for(self.local_host)`, which **excludes self** — *"Exclude self — a host never dials itself in a full-pairwise mesh."* No local row is ever constructed. | `state.rs:488`, `manifest.rs:577-582` |
| 3 | `PeerCertRotation` has **no method that could write plane C**. (⚠ It has **eight** fields, not the five prior notes list.) | `rotation.rs:329-346`, `impl` at `:348` |
| 4 | `router.rs:943` still passes the **boot-time** `local_leaf_fingerprint` = `F_old`. | `maos-a2a-core/src/router.rs:943` |
| 5 | `team_declaration_at` returns `None` the instant `presented != signed`. | `state.rs:817-822` |
| 6 | Every crossing **send** hits `(true, None)` → `A2AError::CohortTeamIdentityRefused`. | `router.rs:993-1012` |

**The peers do not stand still, and that is the real severity.** On each peer `P` this host is an
ordinary member, so `P` opens a one-generation window `{F_old, F_new}` and closes it
**promote-and-retire** — `entry.fingerprint = next` **and** `rotation_next.remove(peer)`
(`maos-a2a-core/src/tofu.rs:356-376`). The TOFU pin store is what the **TLS certificate verifier**
consults, on **both** sides: `verify_pinned_sync` when dialling and `find_active_pin_by_fingerprint`
when listening (`maos-a2a-tcp/src/verifier.rs:173`, `:178`), with WebPKI first, so this is not
fail-open. Both lookup paths then reject `F_old` (`tofu.rs:457-497`, `:250-280`).

> **Once a peer's grace elapses, a host still serving `F_old` has no active pin on that peer and is
> refused in BOTH directions with `PIN_MISMATCH`.** `cold_deployment_t_grace()` is **5 s** —
> `compute_t_grace(500, 0)` = `max(2 × max(500,500), 5000)` (`maos-a2a-core/src/chaos/rotation.rs:11-19`,
> `maos-cohort/src/rotation.rs:183-188`) — and a real `tokio::time::sleep` arms it in production
> (`maos-bin/src/cert_rotation.rs:51-58`, from `main.rs:9796-9801`), via `rotate_pinned_peer`
> (`rotation.rs:645`) → `schedule_close` (`:687`) → `self.timer.schedule(self.grace, …)` (`:981-982`).

⚠ **Timing, stated precisely — "seconds after the reissue" is the wrong model.** The partition arrives
**5 s after EACH PEER'S OWN apply**, and peers apply on independent `confirmation_interval()` phases
(`main.rs:10136-10147`), 60 s at the shipped default. So it can begin as little as 5 s after the *first*
peer picks the manifest up, completes up to ~65 s later, and **the operator cannot see which peer went
first**.

⚠ **Four candidate mitigations were checked and none exists.** `await_repin_consent` has **zero**
production callers (`router.rs:1230` says so in a comment). There is **no TOFU auto-pin** — an unpinned
leaf is refused, never learned. Pull failures never retry faster. The plane-A/plane-B checks at
`router.rs:1017`/`:1334` compare A against B and **pass** throughout, so they neither shrink nor extend
the window.

### 2. 🔴 THE DETECTOR IS SEVERED BY THE EVENT IT DETECTS. This is the story's governing constraint.

**This inverts the obvious design and it must be read before any AC.** A host learns of a new manifest
in exactly one way, and that way runs over the transport the move is about to cut:

- **There is no production reissue injector at all.** `CohortManifestState::issue_reissue`
  (`state.rs:547`) has **ZERO production callers** — its only other reference is inside `#[cfg(test)]`.
  `CohortDistributor::push_to` is called from exactly one place, `service_pending_pulls`
  (`distribution.rs:83`). The **only** production ingress is: a host boots with the new TOML at
  `file.manifest_path` (`main.rs:9179-9192`), and every other host learns it **by polling**.
- The poll is `pull_from(peer)` for every peer, re-armed unconditionally every
  `confirmation_interval()` = **60 s** (`main.rs:10136-10147`; `state.rs:614-620`). A failed pull only
  `eprintln!`s.
- Each peer accepts `F_old` for exactly **5 s** after its own apply.

> **So this host's chance to receive the very manifest that moves its identity is a 5-second window per
> peer, sampled once every 60 seconds — and the whole pull/service/push exchange must complete inside
> it. Miss it and the host never applies the reissue, so an apply-time detector NEVER FIRES, and a
> surface that reports only `cached.manifest` says "agree" forever.**

**AC1 is therefore BEST-EFFORT BY CONSTRUCTION, and the story does not pretend otherwise.** The
always-available diagnosis lives in AC3, which must carry a signal that does **not** require receiving
the manifest — and which reaches the operator because the operator surface is **loopback HTTP on this
host** and survives the partition intact.

### 3. ⚠ THE RECOVERY PATH IS N+1 ARTIFACTS ACROSS N HOSTS, AND ONE ORDERING OF IT WORKS

`RELEASE-HOLDS.md:156-163` **(c.7)** tells the operator to deploy *the peer's* next certificate before
signing. It says nothing about restarts and nothing about self. Measured, for the local host:

- **`apply_reissue` NEVER PERSISTS THE MANIFEST.** There is no `fs::write`, `File::create` or
  `write_all` anywhere in `crates/maos-cohort/src/` — the applied artifact lives only in
  `cached.signed_toml` (`state.rs:519-524`). **Boot always re-reads the on-disk TOML.**
- ⇒ **A "recovery restart" bricks the host** if the operator deployed the new PEM but not the new
  manifest: disk PEM `F_new` vs disk manifest `F_old` → arm (c) **fails the boot**.
- ⇒ **And if neither was deployed, the boot is GREEN and the host is permanently partitioned** —
  arm (c) compares `F_old` against `F_old` and passes, while every peer pins `F_new`. **A silent,
  boot-green, permanent partition is the worst case and it is nowhere in the prior record.**
- ⇒ **Every OTHER host's TOML must move too.** Arms (a) and (b) of the same reconciler
  (`main.rs:9954-9987`) check each peer's `tcp.peer_pins` and `file.peers` against the signed manifest,
  so a member whose TOML still says `F_old` cannot boot. And a member updated and restarted **early**
  pins `F_new` from boot (`maos-a2a-tcp/src/config.rs:116-125`) and refuses this host **with no rotation
  window at all — no 5-second grace whatsoever.**

**The one ordering that works:** write the `F_new` PEM to disk on this host **without restarting** (the
running process holds its identity by value from bind, `transport.rs:393-396`, so nothing changes);
distribute the new signed manifest TOML to **every** member; let it propagate; accept a partition
window; then restart this host, at which point disk PEM and disk manifest both say `F_new` and arm (c)
passes. **An outage is unavoidable. Nothing in the system says any of this today, and writing it down is
half of this story's value.**

### 4. ⚠ THREE CLAIMS IN THIS STORY'S OWN SPRINT ROW ARE FALSE. Do not inherit them.

1. **"with NO AUDIT ROW" is FALSE.** `crossing_outcome_label` maps `CohortTeamIdentityRefused` →
   `"team_identity_mismatch"` (`main.rs:10416`), and **both** production crossing emitters write a
   transparency-log kernel event **unconditionally** carrying that `status` and the error `Display` as
   `detail`: `emit_cross_team_share` → `collective.host.cross-team-share` (`main.rs:10281`) and
   `emit_collective_erase_reconciliation` → `collective.host.cross-team-erase` (`main.rs:10384`); a JSON
   blob also goes to stdout (`main.rs:10287`). **The true gap is smaller: no `ConsentRupture`, no
   `RuptureReason`, no `tracing` at the router seam — and one `&'static str` conflates the
   local-divergence Send refusal (`router.rs:995`) with the peer-side NACK decode (`router.rs:1181`).**
2. **"ALSO OWNS: `RuptureReason` variant" is FALSE ON TWO COUNTS.** (a) The enum already has **six** —
   `PeerIdentityUnverified` shipped with `j1-crosshost-2c` (`maos-domain/src/frame.rs:380-405`). (b)
   **There is NO rupture producer reachable from `prepare_outbound` at all**; all three production
   emitters are on the accept/inbound or TCP-dial-failure side. A variant with no producer is a
   definition, not a control. **See Blocking 4 for the (c.6) ruling.**
3. **"`maos-control` takes PLAIN SCALARS ONLY and its dep list is the reason (`lib.rs:32-35`)" is a
   MISREAD**, repeated in the `14-2b` story file. That doc forbids importing a **foreign type** — it is
   about **crate edges**, confirmed by `crates/maos-control/Cargo.toml`. It is **not** evidence on the
   shape of AC3 in either direction.


### 5. ⚠ RE-MEASURED 2026-09-03 (round-table). THE PREFLIGHT WENT STALE IN TWENTY-FOUR HOURS BECAUSE THE STORY BESIDE IT SHIPPED.

`14-2b`'s dev pass landed in this working tree between this story reaching `ready-for-dev` and its first
review. It is **11 files, +589 insertions, uncommitted**, and it moved three ceilings, two of the three
crates this story writes in, the `check-cohort-mesh` leg count, and the aggregate red. **Nothing about the
story's mechanism changed. Everything about its budget did.** Four rulings follow from that, and one
general shape:

| # | Ruling | Was | Is |
|---|---|---|---|
| **R1** | The `maos-a2a-core` ask is **not** already authorized | ceiling EDIT, ask discharged | `14-2b` spent it (4850→4853); **fresh measured ask** |
| **R2** | T0 is not a fit check | *"does 55–70 fit in +35?"* | four crates at ~zero; T0 output is **an ask** |
| **R3** | AC3's third source is **forced** | a design preference to argue | `convergence_observer_ready()` is the same `OnceLock`; consolidation 404s in the boot window |
| **R4** | AC3(b) narrows to the error half | build per-peer pull health | `14-2b`'s `state` field already gives observed/total; add **only** the last pull error |
| **R6** | Cite symbols, never lines | a published drift table | the drift table drifted; grep `fn apply_reissue` / `fn install_cert_rotation` / `CertRotationRefused` |

🔴 **THE SHAPE, and it is new: A GRANT IS A GLOBAL, NOT A RESERVATION.** This story recorded an operator
authorization on `maos-a2a-core` in four places and treated it as money in an account. It was a ceiling,
and ceilings are consumed by whoever measures first. `14-2b` measured first, legitimately, with a correct
grant of its own. **An authorization names a crate, not a claimant — never carry one across a story
boundary without re-measuring the ceiling it was granted against.**

⚠ **AND THE COROLLARY THAT COST MORE THAN THE GRANT: A CORRECTION TO A STALE CITATION IS ITSELF A
CITATION.** The frontmatter's drift table was measured mid-dev-pass and published as the fix; the dev pass
kept going and every offset in it is now wrong. This is worse than the original staleness because it reads
like the repair. It joins `14-2d`'s *"a residual stale at birth"* as the same family, one turn further in.

✅ **What the re-measurement CONFIRMED, so nobody re-derives it:** the tree is `cargo fmt --all --check`
clean (EXIT 0), so the numbers are real; `A2ARouterCore.local_leaf_fingerprint` **still has no getter**
(private at `router.rs:173`, builder at `:255`, single read at `:943`) so **AC2 stands unchanged**; AC1's
**two** sites both survive `14-2b` (`CohortAuditEvent::CertRotationRefused` still appears exactly twice in
`state.rs`, once per site, each after its own `rotation.reload(...)?`); and `check-cohort-mesh` still has
**no `legs.len()` pin**, so a leg is still a one-line append.


### 6. ✅ T0 OUTPUT — CEILINGS RE-DERIVED AND THE SCOPE LINE-PRICED (2026-09-03, at `4657cace`)

**(a) The four ceilings, re-derived at the commit.** `cargo run -q -p xtask -- kloc-check --json`, tree
`cargo fmt --all --check` EXIT 0:

| crate | measured | ceiling | headroom |
|---|---:|---:|---:|
| `maos-cohort` | 5793 | 5793 | **0** |
| `maos-a2a-core` | 4853 | 4853 | **0** |
| `maos-bin` | 16987 | 16987 | **0** |
| `xtask` | 41925 | 41932 | **+7** |
| `maos-control` | 615 | 1500 | **+885** |
| `_aggregate` | 155093 | 147057 | **−8036** |

**(b) 🔴 THE CALIBRATION CONSTANT NOBODY HAD MEASURED: DOC COMMENTS AND BLANKS ARE NOT CHARGED.** The
ceiling is tokei `code`. Measured against `14-2b`'s own commit, raw→charged: `maos-cohort` 230→**115**,
`maos-a2a-core` 8→**4** (both **0.50**), `maos-control` 269→**197** (**0.73**). **In this codebase's
doc-density, roughly half of what you type is free.** Any estimate quoted in raw lines is ~2× too high.

**(c) The price model, and its validation.** Every component below is measured from a *shipped* analogue,
not estimated. Applied to `14-2b`'s own AC3 the model predicts `maos-control` **200** against an actual
**197** (1.5% error) and `maos-bin` **48** against an actual **46** (4%). **It is a prediction the T8
measurement can falsify — that is the point of pricing it.**

| AC | component | shipped analogue (charged) | predicted |
|---|---|---|---:|
| AC1 | audit variant decl | `CertRotationRefused` 3 fields = **5** → 5-field shape | 7 |
| AC1 | match arm w/ two `fingerprint_short` | `CertRotationDeclarationMoved` arm = **15** | 15 |
| AC1 | the comparison helper | `signed_fingerprint` closure = **5**, + parse/guard/signature | 12–18 |
| AC1 | two call sites (call + append only) | shipped `Err` arms = **12** and **19** | 12 |
| AC2 | `A2ARouterCore` getter | `lookup_peer` = **8** | 4–8 |
| AC2 | `PeerCertRotation::core()` | `pin_generation` = **6** | 3–6 |
| AC3 | `maos-control` types + trait + route + threading + doubles/tests | `14-2b` = 17+42+5+136 | **200** |
| AC3 | `maos-bin` source impl + wiring | `14-2b` = 33+15 | **48** |
| AC3 | `maos-cohort` self-identity read | `peer_convergence` projection | 15–25 |
| AC5 | one `check-cohort-mesh` leg | `14-2b`'s two legs = **28** charged | **14** |
| AC4 | tests in `crates/maos-bin/tests/` | uncharged (`kloc_check.rs:167-191`) | **0** |

**(d) 🔴 A PLACEMENT RULING T0 FORCED, AND IT MOVES ~15 LINES OFF THE TIGHTEST CRATE.** AC3's last-pull-error
belongs in **`maos-bin`**, not `maos-cohort`. The pull loop and its discarded error are already there — and
it is **TWO sites, not one**: `main.rs:10135` and `main.rs:10154` (same "two sites" shape as AC1; wire both
or the signal is missing on one path). Routing it through `maos-cohort` would open a new write path into
cohort state for a value that is **not cohort state** — it is transport health. Long-term correctness and
the budget agree for once.

**(e) THE VERDICT, per crate.**

| crate | predicted | range | headroom | ask |
|---|---:|---|---:|---|
| `maos-cohort` | **~74** | 64–88 | 0 | **grant** |
| `maos-a2a-core` | **~6** | 4–8 | 0 | **grant** |
| `maos-bin` | **~63** | 58–68 | 0 | **grant** |
| `xtask` | **14** | 14 | +7 | **grant** |
| `maos-control` | **~200** | 190–210 | +885 | ✅ **FITS — NO GRANT** |

⚠ **THE VALIDATORS' 55–70 FIGURE FOR `maos-cohort` WAS ITSELF LOW.** Priced properly it is **~74 (64–88)**,
and that is *after* R4 moved the last-pull-error to `maos-bin`. Without R4 it would be ~90. **State the
higher number in the ask; do not anchor on 55–70.**
⚠ **Aggregate at story close ≈ −8393** (155093 + ~357). **DO NOT RAISE `_aggregate_hardfail`** — the
`j1-crosshort-2c` precedent is to state the split, not absorb it: **−357 is this story's, fully attributed
to four named per-crate grants**; the remainder is pre-existing with named owners (D14 → **14-7**,
D17 → **14-6**).

**(f) What T0 did NOT change.** Six ACs stand. The mechanism is untouched. **T0 was a pricing exercise, and
its only deliverable is that the T8 ask is now a prediction with an error bar instead of a discovery.**


### 7. 🔴 AC3'S JUSTIFYING WINDOW DOES NOT CONTAIN THE DATA AC3 MUST REPORT (found post-T0, 2026-09-03)

AC3 defends its own-source design like this: *"the route short-circuits to 404 whenever the rotation source
is unset, and the operator HTTP server binds ~7,900 lines before `install_cert_rotation`, so that window is
real. Give the self-identity key its own source."* **The window is real. It is also empty.** Measured:

| # | site | line | what exists |
|---|---|---|---|
| 1 | `OperatorHttpServer::bind` — sources captured **by value** | `main.rs:2782` (in `async fn main`) | `bootstrap.state` only |
| 2 | `return run_cohort_a2a_daemon(...)` | `main.rs:7884` | `main()` never resumes |
| 3 | `build_cohort_a2a_daemon_runtime` → transport binds | `main.rs:9781` | **router core / plane C first exists here** |
| 4 | `install_cert_rotation(pins, core, …)` | `main.rs:9811` | the `OnceLock` is set |

🔴 **Plane C — the fingerprint this host is SERVING, which is AC3(a)'s whole subject — does not exist until
step 3, and the operator server was bound at step 1 with its source list already consumed.** So an
own-source that merely avoids gating on `cert_rotation.get()` buys **30 lines** of extra coverage
(`:9781`→`:9811`), **not 7,900**. And every scenario this story names — §2's partition, §3's boot-brick,
§3's boot-green permanent partition — occurs with `install_cert_rotation` long since run, i.e. **outside the
window entirely**. The window is a boot transient measured in milliseconds.

⚠ **THIS IS THE WRONG-AXIS ERROR AGAIN, ONE LEVEL UP.** The measurement was correct
(`convergence_observer_ready()` *is* the same `OnceLock`). The **inference** was wrong: *therefore a separate
source renders in the boot window* is false, because the data is absent there too. **Check that the window
you are defending contains the value you intend to report.**

**WHAT SURVIVES, AND WHAT MUST CHANGE:**
✅ **R3's conclusion stands — a third source is still correct — but for the TYPE reason, not the timing
reason.** Self-identity is a different fact from peer convergence and `PeerVersionStatus` cannot express a
three-valued self verdict. **Re-ground R3 on that; delete the timing argument.**
⚠️ **WITHDRAWN 2026-09-03 by §8 — the prescription below was superseded within the hour by measuring the
carrier question. `bootstrap.state` is live on BOTH sides of the `:7884` `return`, so `serving` is reached
through the state exactly as `14-2b` reaches `pin_generation`. NO late-bound handle, NO signature change.
Retained struck-through because the reasoning that produced it is what led to the simpler answer.**
~~🔴 AC3 must adopt a LATE-BOUND ROUTER HANDLE or it cannot report `serving` at all.~~ The source is
constructed at `:2774` and handed to `bind` at `:2782`; the router core arrives at `:9781`. It therefore
needs an interior `OnceLock<Arc<A2ARouterCore>>` (or equivalent) that `run_cohort_a2a_daemon` fills at
`:9781`. ⚠ **This does NOT reopen Blocking 2** — that ruling forbids a *setter on plane C*; this is the
source's own handle to the router, a different object. Say so in the dev record or the next reader will
think the two rulings conflict.
🔴 **AND THE THREE-VALUED VERDICT NOW EARNS ITS KEEP FOR A SECOND REASON:** before the handle is filled the
honest answer is **`unconfirmable`**, not `agree` and not 404. AC3 already requires that vocabulary; this
makes it load-bearing at boot as well as under a missed reissue.
⚠ **Re-price AC3 after the re-grounding.** The late-bound handle is small (~6–10 charged) but it lands in
`maos-bin`, already at zero — fold it into that crate's ask, do not discover it at T8.

**COST OF THE FINDING:** one preflight pass on AC3's rationale and its `serving` plumbing. **AC1, AC2, AC4,
AC5 and AC6 are untouched**, as are §6's prices for them.


### 8. ✅ AC3 REWORK — RESOLVED BY MEASURING THE CARRIER, AND IT CAME OUT CHEAPER (2026-09-03)

§7 said AC3 needed a late-bound `OnceLock<Arc<A2ARouterCore>>` filled at `main.rs:9781`. **That was wrong
too, and one measurement retired it:** `run_cohort_a2a_daemon` takes `bootstrap: Option<CohortDaemonBootstrap>`
(`main.rs:9642-9645`), and the operator sources are built from `Arc::clone(&bootstrap.state)` at
`:2758`/`:2774`. **The same `Arc<CohortManifestState>` is live on both sides of the `:7884` `return`.**
Nothing needs to be handed across it.

**The three questions the rework had to answer, and the measured answer to each:**

| question | answer | cost |
|---|---|---|
| How does the route read the **signed** row? | `state.manifest()` + `state.local_host()` — **both already `pub`**; the exact find-and-parse is shipped at `main.rs:9960-9964` | **0 `maos-cohort`** |
| How does it read the **serving** leaf? | a two-level accessor pair mirroring `14-2b`'s shipped `pin_generation` pair (`rotation.rs:389` + `state.rs:549`) | **~8 `maos-cohort`**, and it **deletes AC2's `core()`** |
| How does the **pull error** reach it? | an `Arc<Mutex<…>>` field on `CohortDaemonBootstrap` — a **`maos-bin`** type, built at `:2038`, cloned at `:2774`, written at `:10135` **and** `:10154` | **~14 `maos-bin`**, R4's placement intact |

🔴 **AND THE OWN-SOURCE REQUIREMENT SURVIVES ON A BETTER REASON.** Not *"it renders in a boot window"* (§7
killed that) and not only *"it is a different type"* (R3's fallback), but: **the two routes have different
404 predicates.** `14-2b`'s 404s when the pin source is absent, because its whole table is then uncomputable.
This one must 404 only when there is **no cohort state at all**, because its `declared` half is computable
immediately. **One trait cannot carry two 404 predicates.** That is mechanical, checkable, and it is what
makes the third source correct.

**RE-PRICED (supersedes §6(e) for AC2/AC3; AC1/AC4/AC5/AC6 unchanged):**

| crate | §6 (T0) | **§8 (reworked)** | Δ | headroom |
|---|---:|---:|---:|---:|
| `maos-cohort` | ~74 | **~57** (50–68) | **−17** | 0 |
| `maos-a2a-core` | ~6 | **~6** (4–8) | — | 0 |
| `maos-bin` | ~63 | **~53** (48–60) | **−10** | 0 |
| `xtask` | 14 | **14** | — | +7 |
| `maos-control` | ~200 | **~205** (4th `bind` param) | +5 | +885 ✅ fits |

**The rework removed ~27 charged lines from the two tightest crates and deleted two mechanisms** (the
`core()` accessor and the late-bound handle). ⚠ **Still four grants — every one of those crates is at zero.**

**THE LESSON, and it is the same one twice in one day, one level apart:** §7 found that *the window AC3
defended did not contain the value it reports*. §8 found that *the fix §7 prescribed was unnecessary because
the value was already reachable by another path*. **Both were failures to trace the data to its carrier
before designing around its absence. Trace the carrier first.**

---

## Blocking conditions
0. **✅ DISCHARGED 2026-09-03 — `14-2b` IS COMMITTED AT `4657cace`. Every number in this story is now
   anchored to a SHA rather than an editor.** *(Original condition, retained because the rule stands for the
   next story:)* **`14-2b` MUST BE COMMITTED BEFORE T0 RUNS.**
   ⚠ *When this condition was written* `sprint-status.yaml` recorded `14-2b: done` while `git status` showed
   **11 modified files and +589 uncommitted insertions** across `maos-cohort`, `maos-a2a-core`,
   `maos-control`, `maos-bin` and `xtask`. **Every budget number in this story's frontmatter was measured
   against that working tree**, so all four grant asks are currently citable to an editor rather than a SHA,
   and any review pass on `14-2b` moves all four ceilings again. `14-2d` already ratified this exact rule for
   itself (*"14-2b and 14-2c both done+committed, then re-derive every citation and re-measure"*); it applies
   here for the same reason. ⇒ **Commit `14-2b`, then re-run `kloc-check` and re-derive the four numbers as
   the FIRST action of T0.** Status stays `ready-for-dev` rather than `blocked` because the precondition is a
   single operator commit, not missing work — **but a dev who starts before it is measuring sand.**

1. **🔴 READ §2 FIRST. THE APPLY-TIME DETECTOR CANNOT BE THE STORY'S ONLY SURFACE.** A dev who
   implements AC1, watches AC4 go green, and ships has built a control that in the field mostly never
   runs — the exact "claim standing in for a control" this project names as its signature failure. **AC1
   and AC3's unreachability signal ship together or the story does not ship.**

2. **⚠ DO NOT WRITE PLANE C. THE OPEN QUESTION IS SETTLED, AND IT IS SETTLED AGAINST THE WRITE.**
   Measurement settles it: writing `local_leaf_fingerprint = F_new` alone is **STRICTLY WORSE**. Nothing
   at runtime re-checks the declaration against the leaf actually served — arm (c) runs **once, at boot**
   (`main.rs:9989-10004`) — so a host declaring `F_new` while serving `F_old` holds an **undetectable
   lie**: its own Send-seam check starts passing, and every peer still refuses its handshake. That turns
   a loud, local, attributable refusal into a silent remote one. **The write belongs to `14-2d`.**
   ⚠ **State the one case that cuts the other way, rather than leaving it unstated:** inside each peer's
   open window the peer accepts **both** fingerprints, so for ~5 s a plane-C write would keep crossings
   working where refusing them does not. **The window is 5 s and the exposure is permanent — the ruling
   survives, and saying so is what makes it a ruling rather than an assertion.**
   ⇒ **This story needs a GETTER, not a setter. No `RwLock`, no interior mutability, no
   `set_local_leaf_fingerprint`.**

3. **🔴 BUDGET — SUPERSEDED IN FULL 2026-09-03. THE OLD TEXT SAID ONE CRATE AT +35 WITH THE
   `maos-a2a-core` ASK ALREADY PAID. BOTH HALVES ARE NOW FALSE.**
   **FOUR crates are at zero or effectively zero** (re-measured live, fmt-clean tree, `14-2b` landed):
   `maos-cohort` **5793/5793 = 0** · `maos-a2a-core` **4853/4853 = 0** · `maos-bin` **16987/16987 = 0** ·
   `xtask` **41925/41932 = +7 against a +14 leg**. Only `maos-control` has room (**615/1500 = +885**).
   🔴 **R1 — "THE `maos-a2a-core` ASK IS ALREADY AUTHORIZED" IS DEAD, AND THE WAY IT DIED IS THE LESSON.**
   `14-2b`'s dev pass edited `kloc.toml` `maos-a2a-core = 4850 -> 4853` on its own measured grant, for the
   `peer_boot_nonce` trait parameter. That **discharged the 2026-08-31 authorization this story was counting
   on.** This story asserted the grant was still held in FOUR places — frontmatter, this condition, AC5 and
   the dev notes — and **nobody was holding it.**
   ⇒ **NEW SHAPE FOR THE LEDGER: A GRANT IS A GLOBAL, NOT A RESERVATION.** A pre-authorized ceiling is
   consumed by whichever story measures first; an authorization names a crate, not a claimant. **Never carry
   a grant forward across a story boundary without re-measuring the ceiling it was granted against.** This is
   the third member of the family this epic keeps finding — *a residual stale at birth* (14-2d), *a
   correction stale at birth* (R6 below), and now *an authorization spent by someone else*.
   ⚠ **THE PRICE IS NO LONGER AN ESTIMATE FOR ONE OF THE FOUR.** `14-2b` shipped the exact AC3 shape
   (a `maos-control` trait + route + three in-`src` doubles + three tests) and it cost **+197 charged** —
   the in-`src` `#[cfg(test)]` block is charged, which is what makes it +197 and not +60. Use that number,
   not a guess. `maos-cohort` remains priced 55–70 by two independent validators, now against **zero**.
   🔴 **FUNDING RATIFIED 2026-09-03 (Lunarpulse): GRANT ALL FOUR, MEASURED. SIX ACs STAY WHOLE.**
   The three rejected alternatives are recorded because each was defensible: *defer AC3 to 14-2d* (rejected —
   Blocking 1 says AC1 alone is a control that mostly never fires in the field, so the story would ship its
   own named failure mode); *merge 14-2c into 14-2d* (rejected — rebuilds the ~16-task story the 3-way split
   existed to kill); *reclaim before asking* (rejected — 14-2a already attempted reclaim on `maos-a2a-core`
   and it could not fund a story, and an unbounded search does not belong in front of a live total-partition
   defect).
   **Discipline is UNCHANGED and non-negotiable** (`kloc.toml:60-65`): the code exists and is
   `cargo fmt --all`-formatted **before** the ask, the ask is **EXACT MEASURED / ZERO HEADROOM** per crate,
   and it cites `kloc.toml:84-87` with the `kloc.toml:303` precedent plus the two-day-old `14-2a`/`14-2b`
   rows on these same crates. **NEVER cite `kloc.toml:269`** — anti-masking, which carries no sanction for
   proceeding while red. **Pre-authorization discharges the ASK, not the MEASUREMENT.**

4. **⚠ RELEASE-HOLDS (c.6) IS NOT CLOSABLE BY THIS STORY.** It has two obstacles and only one is code:
   the frame `journal_peer_identity_refusal` builds has no detail field, **and** *"the seam cannot
   distinguish 'retired generation after the grace' from 'unknown leaf', because the store keeps no
   retired-generation history by design."* The second is architectural; a seventh `RuptureReason` cannot
   truthfully assert a distinction the seam cannot make. ⇒ **AC6 CORRECTS (c.6) and re-points its owner.
   It does not close it.**

5. **⚠ THE EXISTING TEST FOR THIS EXACT SUBJECT IS A NULL CONTROL AND WILL LOOK LIKE COVERAGE.**
   `emitter_with_a_stale_local_leaf_cannot_originate_a_crossing`
   (`crates/maos-bin/tests/team_identity_13_6a.rs:516-537`) proves the **static** refusal by *injecting*
   the divergence at construction on a bare `A2ARouterCore` with **no transport**. It never runs a
   reissue, never crosses a socket, and stays green under every mutation this story could make.
   Related, measured: **ZERO tests anywhere exercise a reissue that moves the LOCAL host's fingerprint.**
   `crates/maos-cohort/tests/cert_rotation_14_2a.rs:205-214` hard-pins the local row to
   `fingerprint(0xf0)` across **26** reissue applications; the only *deliberate* local-row mutation there is
   **removal**, at `:524`. (One incidental move exists elsewhere — see AC4.)

6. **⚠ THE HELPER-MATRIX TRAP IS REAL BUT ON THE WRONG AXIS FOR THIS STORY.** The filed trap
   (`build_mesh_n_provisioned` hard-codes `vec![None; ..]`) is true and is **three** sites —
   `crates/maos-a2a-tcp/tests/support/mod.rs` `:487`, `:519`, `:917` — but four corrections change what
   it costs. **(a)** *"the field is never read"* is **false**; `router.rs:943` evaluates on every
   non-reserved send regardless of gate. **(b)** The real neutraliser is `crossing`, not the gate:
   `(false, _) => None` at `router.rs:1011`. `t_12_2` and `t_12_5` **do** install real
   `CohortManifestState` gates and still observe nothing, because both dial `IntentClass::Readonly`.
   **(c)** 🔴 **`declared` is the WRONG AXIS** — plane C derives from the **serving** chain
   (`transport.rs:393-396`); `declared` rewrites only **peer** configs (`support/mod.rs:615-618`, over
   `j != i`). **This is the same wrong-axis error that cost the pre-split story four weeks.**
   **(d)** A crossing frame over the real transport additionally dies at `router.rs:919-929`
   (`IntentDenied`) **21 lines before the gate**, because `build_mesh_inner` hard-codes allowlists to
   `["readonly"]` (`support/mod.rs:594-598`) — which is why the crossing intents appear **zero times
   anywhere under `crates/maos-a2a-tcp/`** — its own suite has never sent one. ⚠ **Do NOT widen that to
   "never over the real transport anywhere": `crates/maos-bin/tests/cross_team_crossing_13_6b.rs:1641`
   drives `live_crossing_runs_through_two_daemon_processes` between two real daemons (`#[ignore]`d
   behind live Postgres).** The narrow claim is the true one. **AC4 is
   scoped so it needs none of this.**

7. **⚠ NO CYCLE EXISTS AND NONE WOULD BE CREATED.** `crates/maos-a2a-tcp/Cargo.toml:40` already declares
   `maos-cohort` as a **dev-dependency**; `t_12_2`/`t_12_3`/`t_12_4a`/`t_12_5` all name
   `CohortManifestState`. `maos-cohort` has **no `[dev-dependencies]` section at all**. And
   `crates/maos-bin/tests/` sees all three layers with **no `--features` flag** (`default = ["network"]`);
   precedent: `tenant_map_13_1.rs` uses `CohortAuditEvent::`, `team_identity_13_6a.rs` uses
   `A2ARouterCore`.

8. **⚠ `all gates green` IS NOT AN AVAILABLE DONE CRITERION.** `kloc-check` is **RED on two
   FOREIGN keys** — `maos-domain` **−51** (D14, owner **14-7**) and `_aggregate_hardfail` **−8036,
   RE-MEASURED 2026-09-03** (155093 >= 147057; it DEEPENED from −7646 by `14-2b`'s +390, so the split is
   now THREE terms: −51 not ours, −7646 D17 not ours, −390 `14-2b` not ours — state it, never the stale
   figure) (D17, owner **14-6**)
   (D17, owner **14-6**) — and is **Blocking** in CI (`.github/workflows/discipline.yml:154`, no
   `continue-on-error`, *"BLOCKING since 2026-07-25"*); it predates `gate_common::BindingClass` and does
   not use it. `kloc.toml:458` allows the aggregate to be recalculated **only at an epic retrospective**
   — this story does not rebase it. ⚠ The aggregate compares `>=` while per-crate compares `>`.
   ⚠ **`check-dev-record-completeness` is RED with the diagnosis now exact** — one violation,
   `deferred-work.md:860: STALE owner \`14-1\` — sprint status is \`done\``. It is **NOT** induced by the
   uncommitted `sprint-status.yaml` (the `14-2b` story hedged that; a live re-run refutes it). The row's
   own text names the successor — *"Owner: 14-1 follow-up or **14-6** (instrument work)"* — and the work
   is `xtask/src/check_scale_churn.rs` percentile publication. **Disown to 14-6; do not repair here.**
   ⚠ **Measured GREEN at HEAD, keep them so:** `check-kernel-baseline` **0** (*24472, pinned 24472* —
   **ZERO headroom**, resolved from `xtask/kernel-core-baseline.toml`, never restated as a literal),
   `check-cohort-mesh` **0** (35 legs), `check-cert-rotation-trigger` **0** (4 legs, 39 enrolled tests).
   ⚠ `check_cohort_mesh.rs:604-606` calls `check_kernel_baseline::check()` itself, so a kernel drift reds
   **two** gates. ⚠ **Three more Blocking gates needle the exact files this story edits**, listed by no
   prior note: `check-j1-two-host-signed-run` greps **both** `router.rs` (`:69`, `:629`, `:691`) **and
   `maos-cohort/src/state.rs`** (`:73`, `:716`); `check-j1-loopback-delegation` greps `router.rs`
   (`:84`, `:460-493`); `check-env-contract` is **blocking at all four phases** and scans
   `crates/maos-bin/src` for unregistered `MAOS_*` reads (`check_env_contract.rs:119-121`) — **add no env
   read.** **Disown every red with a named owner; absorb none.**

9. **⚠ THE 8-HEX ANSWER ALREADY SHIPS — DO NOT WRITE A THIRD ONE.** `redaction.rs:140` sets
   `TOKEN_HEX_MIN_LEN = 32` and `:169-181` rewrites **any** hex run ≥ 32 as
   `<REDACTED:type=capability_token,…>`, so a 64-hex fingerprint never lands readable. Two helpers exist:
   `PeerCertFingerprint::short()` (`maos-a2a-core/src/identity.rs:88-91`) and `fingerprint_short`
   (`maos-cohort/src/audit.rs:57-60`). **Reuse one.**

10. **🔴 THE SPRINT ROW'S RATIFIED F3 IS RETIRED BY THIS STORY — SAY SO, OR THE DEV GETS OPPOSITE
    INSTRUCTIONS FROM TWO ARTIFACTS.** `sprint-status.yaml:306` records *"RATIFIED F3: `std::sync::RwLock`,
    no new dependency"* for plane C. That ratification belongs to the **write**, which Blocking 2 moves
    to `14-2d`. **Under the detect-only scope F3 does not apply here.** Its measured cost is preserved
    for `14-2d`: 4 lines changed 1-for-1 + a read binding + a 3–5-line setter (net +4…+6), the consuming
    builder signature unchanged so **zero call-site edits**, and ⚠ a `.clone()` — never a held
    `RwLockReadGuard` — because `prepare_outbound` is `async` with an `.await` at `router.rs:1018` and
    `#[async_trait]` requires a `Send` future. **Re-point F3 to `14-2d` in the dev record.**

11. **⚠ A NEW AUDIT *KIND* IS THE OBVIOUS-LOOKING WRONG ANSWER.** It costs two lines in
    `maos-audit/src/lib.rs:666-703`/`:707-…` (symmetry required by the comment at `:723-726`) and
    `maos-audit` is at **6847/6847 = ZERO**; `maos-iac` is likewise 6960/6960. **Use a
    `CohortAuditEvent` variant in `maos-cohort`** — measured, it is matched exhaustively in **exactly one
    place**, `maos-cohort/src/audit.rs:179`, so it costs **zero** lines in `maos-audit`/`maos-iac`.

12. **✅ RESOLVED 2026-09-03 by the AC3 rework (§8). Retained because the RULE it produced still binds:
    *check that the window you are defending contains the value you intend to report.*** *(Original:)*
    **AC3 IS NOT READY. READ §7 BEFORE TOUCHING IT.** Its stated justification (*"the operator server
    binds ~7,900 lines before `install_cert_rotation`, so give the key its own source"*) is defended by a
    window that **does not contain plane C** — the router core first exists at `main.rs:9781`, 5,100 lines
    after `main()` has already `return`ed into `run_cohort_a2a_daemon` at `:7884` with the operator server's
    source list captured by value at `:2782`. An own-source buys **30 lines**, not 7,900, and none of the
    story's scenarios land in that window. **A third source is still right — for the TYPE reason — but AC3
    needs a late-bound router handle and a re-grounded rationale before anyone builds to it.** Do not
    implement AC3 from the current text.


---

## Acceptance Criteria (6)

**AC1 — The host detects the move at apply, at BOTH sites, and the row is honest about when it can exist.**
Compare the incoming `members[local_host].fingerprint` against the identity this process is actually
serving; on divergence **append a named `CohortAuditEvent` and continue** — the manifest still applies.
⚠ **The precedent is twenty lines away:** the `Err(error)` arm at `state.rs:503-510` writes
`CertRotationRefused` when a reissue REMOVES this host, recorded as *"the manifest applies and the
operator gets a named row rather than a silent no-op."* **Identical logic. Do not refuse the reissue** —
a host that rejects a signed monotonic manifest forks itself off the cohort *and* stays partitioned.
🔴 **THERE ARE TWO SITES, NOT ONE, AND THE CODE ITSELF SAYS WRITING ONLY ONE IS THE DEFECT.**
`install_cert_rotation` carries its **own** reconcile block at `state.rs:246-266`, and the shipped
precedent is written at **both** `:259` and `:504` under this comment (`state.rs:251-254`): *"The SAME
named row the reissue path writes for the identical failure. Silently skipping here would make 'this
node cannot derive a peer set for itself' observable on one path and invisible on the other."*
The install-site window is real and is exactly §2's: `state.rs:227-241` warns that *"a signed push or a
pull response landing in that interval would advance the cached manifest with `cert_rotation` still
unset — and redelivery at the SAME version returns `Confirmed`, which does no work, so the divergence
would persist silently until some LATER version arrived"* (the same-version early return is
`state.rs:453-457`). **Factor the comparison into ONE helper and call it from both sites.**
🔴 **Five rules the implementation must state, all measured:**
**(a) ORDERING** — append **after `rotation.reload(...)` returns `Ok`, before `*cached` is replaced**.
`state.rs:490-495` calls `reload` with `?`; a row appended before it journals a divergence for a
manifest this process never committed.
**(b) `None` ⇒ NO ROW. This is a RULING, not a choice, and a Blocking gate enforces it.**
`local_leaf_fingerprint` is `Option<…>` defaulted `None` (`router.rs:173`, `:223`), `Some` only because
`transport.rs:393-396` guards on `if let Some(own_leaf)`. **Every `maos-cohort` test builds the core via
`A2ARouterCore::try_new(...)` with no `.with_local_leaf_fingerprint(...)`
(`crates/maos-cohort/tests/cert_rotation_14_2a.rs:229-250`), so plane C is `None` in that whole harness.**
Treating `None` as divergence would spray rows through its **26** reissue applications — and that file is
`TEST_FILES[1]` of the **BLOCKING** `check-cert-rotation-trigger` (`check_cert_rotation_trigger.rs:92-118`).
Rule `None` ⇒ **no row**, matching `router.rs:943`'s fail-closed posture, and require AC4's harness to
call the public builder `with_local_leaf_fingerprint` (`router.rs:255`) to get a `Some`.
**(e) PRECONDITION, with an owner** — AC1 infers *"the manifest moved the leaf I serve"* from plane C,
which equals the served leaf **only** because it is set once at bind from `load_identity()`'s chain and
`swap_serving_cert` has zero production callers. **`14-2d` makes plane C mutable and owns keeping that
equality true.** Name it; do not leave it implicit.
**(c) NO LOCAL ROW ≠ DIVERGENCE** — `peer_configs_for` returns `EHostNotMember` (`manifest.rs:568-573`)
and the shipped `Err` arm already covers removal. ⚠ Note the boot reconciler **fails open** in the same
case (`if let Some(signed_own)`, `main.rs:9989`) — name it; `14-2d` inherits it.
**(d) COMPARE PARSED VALUES, NOT STRINGS** — `PeerCertFingerprint::parse` lowercases
(`identity.rs:73-85`) while the manifest field is a raw `String` (`manifest.rs:141`); a `.wire()`-vs-raw
compare fires spuriously on uppercase hex. The precedent parses (`main.rs:9945-9951`).
Fingerprints in the row are **8-hex** via a shipped helper (Blocking 9).

**AC2 — The running identity is readable, and read live rather than snapshotted.**
`A2ARouterCore.local_leaf_fingerprint` is private with a builder setter and **no getter anywhere in the
workspace** (`router.rs:173`/`:255`, single read at `:943`). Add the missing **read accessor** — the
symmetric sibling of the shipped `lookup_peer`.
🔴 **REACH IT AS A TWO-LEVEL ACCESSOR PAIR, MIRRORING A PAIR `14-2b` SHIPPED ONE STORY AGO — not through a
generic `core()`.** `PeerCertRotation.core` is private and `state.rs` is a **sibling module**, so it cannot
be read directly; the shipped answer to exactly that problem is `pin_generation`, which exists at BOTH
levels: `PeerCertRotation::pin_generation` (`rotation.rs:389`, reads its private `pins`) wrapped by
`CohortManifestState::pin_generation` (`state.rs:549`, `self.cert_rotation.get().and_then(…)`). **Build the
identical pair:** `PeerCertRotation::local_leaf_fingerprint()` (3 lines over its private `core`) wrapped by
`pub fn CohortManifestState::local_leaf_fingerprint()` (5 lines over the `OnceLock`). **This is what AC3
consumes, and it removes the need for a `core()` accessor entirely** — do not add one, and do not export
`PeerCertRotation`'s router to callers that only need one field.
⚠ **Price and reject `pub(crate) core` in writing too:** it is **line-neutral** and would work, but
`14-2b` chose an accessor for the identical problem eight days ago and every other `PeerCertRotation` field
is private. **Reject on idiom-consistency, and say that `14-2b` is the precedent.**
🔴 **Reject the cheaper alternative, and say why in the dev record.** A 5th parameter on
`install_cert_rotation` costs less and would not red `check-cert-rotation-trigger` (it pins
`INSTALL_METHOD` and `PLANE_METHODS` only — **no arity assertion**, `check_cert_rotation_trigger.rs:87-89`).
**It is still wrong: it snapshots at install time, and `14-2d` makes that value mutable — a residual
stale at birth.** Read the live value.
⚠ **Price and reject the second alternative too, rather than omitting it:** making the field `pub` at
`router.rs:173` is **line-neutral** and would dissolve this AC's budget question entirely — but **all 13
`A2ARouterCore` fields are private**, so it breaks the crate's idiom and exports a security-path field
with no accessor discipline. **Reject it on those grounds, in writing.**
✅ Feasibility verified end to end: `maos-cohort → maos-a2a-core` is a real `[dependencies]` edge with no
reverse edge; production passes the **right object** (`main.rs:9796` hands `runtime.transport.core()` —
the same `Arc` the transport set plane C on at `transport.rs:394-395`); and **no lock discipline is
violated** — `state.rs:475-477` forbids `RotationGraceTimer` impls from calling back into the state, but
a getter touches no `CohortManifestState` lock and plane C has no lock of its own.

**AC3 — The operator can diagnose this on a live daemon EVEN WHILE PARTITIONED.**
🔴 **REWORKED 2026-09-03 after §7. Read §7 and §8 before this AC — the previous text's justification was
wrong, and its prescribed fix (a late-bound router handle) is ALSO superseded by a simpler design found by
measuring the carrier question. This is the third pass on AC3 and it is now grounded on mechanics, not
on a window.**

The operator surface is **loopback HTTP on this host** and survives the partition intact, which is why it —
not AC1 — is the always-available diagnosis. Report, under the shipped bearer gate
(`maos-control/src/lib.rs:193-205`, which runs **before** path dispatch — **do not invent a second
authentication path**) and **READ-ONLY** for the reason at `lib.rs:66-76`.

**(a) THE FOUR FIELDS, AND WHERE EACH ONE COMES FROM. All four paths are measured and none is new plumbing.**

| field | source | available from |
|---|---|---|
| `declared` — the signed member row's fingerprint | `state.manifest()?` + `state.local_host()`, **both already `pub`** (`state.rs:363`, `:427`) | **immediately** (`:2758`) |
| `serving` — plane C, the leaf this process actually presents | `state.local_leaf_fingerprint()` → `PeerCertRotation` → `A2ARouterCore` (AC2) | `main.rs:9811` |
| `verdict` — `agree` / `diverged` / **`unconfirmable`** | derived from the two above | always renders |
| `peers_observed` / `peers_total` | **`state.peer_convergence()` — the SAME call `14-2b`'s route makes**, counted by `is_valid()` | `main.rs:9811` |

🔴 **THE `declared` HALF COSTS ZERO `maos-cohort` LINES, AND THAT IS THE FINDING THAT SIMPLIFIED THIS AC.**
`CohortManifest.members[].fingerprint` is readable through the existing public `manifest()` accessor, and the
**exact** shape is already shipped in `maos-bin` as the `signed_fingerprint` closure
(`main.rs:9960-9964`: `.members.iter().find(|m| m.host_id == host_id).and_then(|m| PeerCertFingerprint::parse(&m.fingerprint))`).
**Copy it; do not add a `maos-cohort` accessor for it.** ⚠ 14-2d's preflight already ruled that closure
should become a free fn (three consumers) — **that extraction is 14-2d's, not this story's. Copy, and file
the duplication with 14-2d as owner rather than pre-empting it.**

**(b) 🔴 THE ROUTE MUST NEVER `404` WHEN COHORT STATE EXISTS — AND THAT, NOT A BOOT WINDOW, IS WHY IT NEEDS
ITS OWN SOURCE.** `14-2b`'s peer-versions route 404s whenever `convergence_observer_ready()` is false, because
without the pin source its whole table is uncomputable. **This route is different: its `declared` half is
computable the instant cohort state exists.** So its `None`/404 predicate is *"this process has no cohort
state at all"* — never *"the rotation control is not installed yet"*. Same 404 vocabulary, different
predicate, which is a thing one trait cannot express in two ways. ⇒ **a third source, and the reason is
mechanical.**
✅ **NO LATE-BOUND HANDLE IS NEEDED. §7's prescription is WITHDRAWN.** Measured: `run_cohort_a2a_daemon`
takes `bootstrap: Option<CohortDaemonBootstrap>` (`main.rs:9642-9645`) and the operator sources are built
from `Arc::clone(&bootstrap.state)` at `:2758`/`:2774` — **the same `Arc<CohortManifestState>` is live on both
sides of the `:7884` `return`.** So `serving` is reached through the state, exactly as `14-2b` reaches
`pin_generation`, and no `OnceLock<Arc<A2ARouterCore>>`, no signature change and no new parameter is
required. **Blocking 2 is untouched — nothing here is a setter.**

**(c) 🔴 `unconfirmable` IS NOW LOAD-BEARING IN TWO INDEPENDENT WAYS, AND BOTH MUST BE TESTED.**
**(i)** Before `install_cert_rotation` (`main.rs:9811`) the serving side is unreachable, so the honest answer
is `unconfirmable` — **not `agree`, and not 404.** **(ii)** §2's dominant case: a host that never received the
reissue holds a stale `cached.manifest` and a two-valued verdict would report `agree` forever.
⚠ **Absent ≠ agreeing** — same defect class as `Some(Healthy(vec![]))` meaning *"installed, nothing in
flight"* (`lib.rs:79-80`).

**(d) 🔴 THE LAST PULL ERROR — THE ONLY SIGNAL THAT DOES NOT REQUIRE RECEIVING THE MANIFEST (R4).**
`14-2b`'s table already yields the *count* (`peers_observed`/`peers_total` above, read from the same
`peer_convergence()` call — **one mechanism, two consumers, never two mechanisms**). What it does not carry is
**why**: `main.rs:10135` and `main.rs:10154` both `eprintln!` the pull failure and discard it.
⚠ **TWO SITES, NOT ONE** — the initial pull-on-connect and the renewal loop. **Wire both or the signal is
missing on whichever path failed.** *"Serving `F_old`; 0 of 8 peers observed; last error `PIN_MISMATCH`"* is
the complete diagnosis and the only one available in the case §2 shows is dominant.
✅ **CARRIER, MEASURED: a `pull_health: Arc<Mutex<BTreeMap<String, String>>>` FIELD ON
`CohortDaemonBootstrap`** (`main.rs:9145`, a **`maos-bin`** type). It is constructed with the bootstrap at
`:2038`, cloned into the source at `:2774`, and the original travels into the daemon at `:7884` and is in
scope at both pull sites. ⚠ Clone the `Arc` **before** the `tokio::spawn(async move …)` at `:10131`.
**This keeps R4's placement ruling intact — the value is TRANSPORT health, not cohort state — and it is now
proven reachable rather than asserted.** Do NOT route it through `maos-cohort`.

**(e) Shape and reuse.** Mirror `14-2b` exactly (`maos-control/src/lib.rs:131-141` → `maos-bin`
`cert_rotation.rs`'s `CohortPeerVersions`): trait + row type + status enum in `maos-control`, the mapping in
`maos-bin` (*"it carries NO cohort type across the `maos-control` boundary"*), **never
`unwrap_or_default()`** on a poisoned read — an empty list must not be producible by a failure.
⚠ **`maos-control` may take another plain scalar**; the constraint it actually carries is *no new crate edge*
(§4.3), and that must hold. Fingerprints in the body are **8-hex** via a shipped helper (Blocking 9).
⚠ **A new trait means a fourth parameter on `OperatorHttpServer::bind`** — `14-2b` already widened it to
three (`main.rs:2782-2788`); one more is the same one-line-per-call-site edit, and the in-`src` `#[cfg(test)]`
doubles are per-trait, so **the existing three `RotationWindowSource` doubles are NOT touched** (the
superseded warning about them was wrong on this too).

**AC4 — Proven on real behaviour, through the real surface, with a non-vacuous negative.**
A test that **runs a real reissue moving the local host's own fingerprint** and asserts the AC1 row
appears and the AC3 surface reports `diverged`, while a reissue leaving the local row unchanged produces
**neither**.
⚠ **CORRECTED — "zero tests move the local row" is FALSE as an absolute, and the exception is a
regression surface AC1 will now traverse.** `crates/maos-bin/tests/team_identity_13_6a.rs:1131` applies a
reissue that drops `HOST_A`, and because that harness derives fingerprints by declaration **index**
(`:167`), the applier's own row moves `0xa1 → 0xa0` **incidentally and unasserted**. The substance of
Blocking 5 holds — no test *observes* a local move — but AC1 will start producing a row there. **Run that
file and account for the change.**
🔴 **Land it in `crates/maos-bin/tests/` and assert through the HTTP RESPONSE BODY.** Measured: that
crate sees `maos-control`, `maos-cohort` and `maos-a2a-core` with **no `--features` flag** (Blocking 7).
If AC4 calls the `maos-bin` impl method directly, **deleting AC3's `json!` key at `lib.rs:250` stays
green** — the unwired-mechanism failure this AC exists to prevent.
🔴 **The negative must be one an unwired mechanism cannot pass.** *"A host with a matching fingerprint
reports no divergence"* is **vacuous**. Assert instead that the **same harness, same host, two reissues**
are told apart. Add a negative for the rollback path: a reissue whose `reload` **errs** must leave **no**
divergence row (AC1a).
🔴 **Do not cite `team_identity_13_6a.rs:516` as coverage** (Blocking 5).
⚠ **RE-MEASURED 2026-09-03 — `14-2b` ALREADY BUILT THIS HARNESS SHAPE, IN THIS CRATE.**
`crates/maos-bin/tests/t_14_2b_cohort_convergence.rs` is a live three-host mTLS sweep asserting through the
operator surface, enrolled as two `check-cohort-mesh` legs. **Copy its construction rather than deriving
one** — and note it moved there from `crates/maos-a2a-tcp/tests/` during the dev pass, which is the same
placement lesson this AC states independently.
New files named **`t_14_2c_*`** — verified safe: `check_rotation_real_timing.rs:354-355` scopes on
`starts_with("t_10_4b_rotation_")` or `starts_with("t_14_2_")`, and `t_14_2c_` matches neither (7th char
is `c`); `check_cert_rotation_trigger.rs:92` is a fixed 5-entry `TEST_FILES` array, so a new file cannot
red that gate either. **Never name a file `t_14_2_*`.** Test lanes are kloc-free.

**AC5 — Enrolled in a gate that reds, budget measured after `fmt`, reds disowned by owner.**
Enroll AC4 as legs of **`check-cohort-mesh`**, whose shape is **measured**: `struct Leg` at
`xtask/src/check_cohort_mesh.rs:11-14`; the container is `let legs = [ … ];` at `:62-600`, an **array
literal with an INFERRED length**, and **no test pins `legs.len()`** (no `check_cohort_mesh_tests.rs`
exists) — so **adding a leg is a one-line append**. ⚠ **RE-MEASURED 2026-09-03: the gate now reports 37,
not 35** — `14-2b` appended two legs of its own. **Copy their shape verbatim** rather than the older
idioms below: `-p maos-bin --test t_14_2b_cohort_convergence <one_test> -- --ignored --exact`, with the
enrolment rationale written as a comment beside the leg. The `+1` over the entry count is the
`check_kernel_baseline::check()` sub-check.
🔴 **AND THE LEG IS NOT FREE — `xtask` IS THE FOURTH ZERO-HEADROOM CRATE.** `xtask` measures
**41925 / 41932 = +7**, and `14-2b`'s append PRICES a leg exactly: 2 legs, 38 raw lines, `xtask`
41897 → 41925 = **+28 charged, i.e. +14 per leg** (comments are uncharged by tokei `code`). **One leg
does not fit in +7.** `xtask` therefore joins the measured ask in Blocking 3 — this was invisible on
2026-09-02, when the crate had +35.
🔴 **Two constraints that silently defeat a leg.** `run_leg` (`:29-34`) rejects any leg whose transcript
lacks **either** `"running 1 test"` **or** `"1 passed"` — **every leg must select EXACTLY ONE test via
`-- --exact`.** And `build_journey_daemon()` (`:37-51`) runs `cargo build -p maos-bin` first, so a
`maos-bin` compile break reds the gate before any leg runs.
**Do NOT enroll into `check-cert-rotation-trigger`** — its leg count **is** pinned
(`xtask/src/tests/check_cert_rotation_trigger_tests.rs:243`). **Do not author a new `xtask` gate**:
`check_cert_rotation_trigger.rs` cost 945 raw lines against `xtask`'s +7.
Execute and record **both** proven-red vectors: removing AC1's comparison reds the divergence leg;
removing AC3's self key reds the surface leg **through the HTTP body**. Then `cargo fmt --all`, measure
the **formatted** tree, record **EXACT MEASURED / ZERO HEADROOM** per crate, and **only then** present
**ONE measured ask covering ALL FOUR crates** — `maos-cohort`, `maos-a2a-core`, `maos-bin` and `xtask` —
each as arithmetic, each with its own driver named (Blocking 3). ⚠ **There is NO pre-paid ceiling edit
here any more: `14-2b` spent the `maos-a2a-core` authorization (R1).** Funding was ratified 2026-09-03,
but **the ratification is of the ASK, not of a number** — the numbers come from the formatted tree after
the code exists, never before. **ZERO kernel-Δ** (`check-kernel-baseline` measured 0 with ZERO headroom;
resolve the pin from `xtask/kernel-core-baseline.toml`, never restate it as a literal).
⚠ **Do not mirror the NEAREST leg idiom blindly** (mirror `14-2b`'s two, above): 8 of the enrolled
`maos-cohort` legs use `--lib`,
i.e. in-`src` `#[cfg(test)]`, which **is charged**. AC4's `t_14_2c_*` integration-test direction is the
budget-free one — `-p <crate> --test <bin> <one_test> -- --exact` is an existing idiom (29 `--test` legs).
⚠ And adding a `#[test]` to `cert_rotation_14_2a.rs` instead of a new file **would** pull it into
`check-cert-rotation-trigger`'s 39 derived invocations.

**AC6 — Write down the procedure that does not exist, and repair the ledger the split broke.**
**(a)** **ADD a NEW self-scoped boundary** to `RELEASE-HOLDS.md` — do **not** characterise (c.7) as
unsatisfiable. Measured, (c.7) (`RELEASE-HOLDS.md:156-163`) is already peer-scoped (*"the **peer's** next
certificate"*), is a **top-level bullet at column 0** (a sibling of (a)–(d), not nested under (c)), and
already carves the local host out to a named owner — **whose key no longer exists**, which is (c) below.
The new boundary records: the **N+1-artifact recovery ordering**,
the **unavoidable outage**, the **boot-brick** variant (PEM without manifest) and the **boot-green
permanent partition** variant (neither deployed). This is the operational half of the story's value and
it exists nowhere today.
**(b)** **(c.6) is CORRECTED, NOT CLOSED** (Blocking 4), and its owner re-pointed.
**(c)** Re-point every reference to the deleted key `14-2b-self-identity-rotation`. 🔴 **DO NOT INHERIT
THE COUNT — RE-DERIVE IT (R6).** This story said **eight**; `14-2d`'s preflight then found a **ninth, and
the only one in SOURCE**: `crates/maos-cohort/src/rotation.rs`'s ratified lock-order comment, which asserts
`swap_lock` sits outside the order *"because this story never takes it: the local leaf is `14-2b`'s"* —
both halves now false, and invisible to an exact-key grep because it uses the short form. `14-2b`'s dev pass
has since added further doc comments to `state.rs` and `rotation.rs`. **Grep for BOTH the hyphenated and
the dotted forms, in source and in docs, at T7 — and report the number you measured, not the number you
were given.** The known set at time of writing: **five are exact-key
hits, all verified live:** `RELEASE-HOLDS.md:104`, `:155`, `:163`,
`docs/release/v2.2-capacity-envelope.md:210`, `tests/coverage-matrix.yaml:521`. ⚠ **And an exact-key grep
under-counts the ledger — three more, found only by looking:** `epics/index.md:149` names
*"14.2b self-identity rotation"* in **DOTTED** form (invisible to the grep) and omits 14.2c/14.2d
entirely; `sprint-status.yaml:304` still routes `swap_serving_cert` *"→ `14-2b`"*; and
`epic-14-…-v2-2.md:46` still says **"9 large stories"** while `index.md:148` says **17** and
`index.md:143` says **"E14=9"**. `check-epic-close-coherence` **PASSES today** (14 epics, pin 24472) — but
this is precisely the class that redded it once, at `a22f0c01`. (c.6) → this story; (c.2) and (c.7)'s self
half → `14-2d`.
**(d)** State plainly that plane C is **not written** here and why writing it alone would be worse
(Blocking 2).

---

## Declared cut lines

- **`14-2b`'s convergence route 404s in the pre-`install_cert_rotation` boot window** —
  `convergence_observer_ready()` is `self.cert_rotation.get().is_some()`, so the peer-version table is
  unavailable for the same ~7,900 lines that make AC3's own-source ruling necessary. Honest for `14-2b`
  (restart validity is uncomputable without the pin source) but it means **that route does not cover the
  partition case at boot and nobody may claim it does.** **Owner: `14-2b`.** Named limitation, not absorbed.

- **Plane C is not written and `swap_serving_cert` still has no production caller** — `RELEASE-HOLDS`
  (c.2) stays open, owner `14-2d`.
- **The Send-seam refusal keeps its uninformative message.** `router.rs:995-1007` hard-codes
  `claimed_team: None, declared: None`, so the `Display` renders *"host X claims team None but the
  signed manifest declares None"* — zero diagnostic content — and `main.rs:10416` collapses the
  local-divergence refusal and the peer-side NACK decode into one `"team_identity_mismatch"`. **Declared,
  not fixed:** with a ~5 s partition the operator loses the mesh long before a crossing is attempted, so
  AC1's apply-time row is the diagnosis that actually arrives. **File it with `14-2d` as owner** — the
  code comment at `router.rs:993-995` already names both causes and then discards the distinction.
- **`maos run` cross-host peers are out of scope** — that arm binds with no `CohortManifestGate` and no
  refresh loop (`main.rs:2457-2465`). **Cohort daemon arm only.**
- **A pre-existing, unfiled ABI defect found in passing, NOT this story's to fix:** `wit/spirit.wit:133-139`
  declares only **five** `rupture-reason` variants, so `frame_bridge.rs:196`'s `_ => RecipientUnloaded`
  backstop silently rewrites `PeerIdentityUnverified` into *"the recipient unloaded"* at the WASM
  boundary — the exact substitution that variant's own doc says it exists to prevent. No gate can see it
  (`abi-diff` scopes `maos-spirit-abi` only and fails only on removals). **Report it with an owner; do
  not absorb it.**

## Dev notes

**Placement.** ⚠ **BUDGET RE-MEASURED 2026-09-03 — the numbers below replace the 2026-09-02 set entirely
(Blocking 3 / §5).** Detection + the audit variant in `maos-cohort` (**5793/5793 = ZERO**, priced 55–70);
the read accessor in `maos-a2a-core` (**4853/4853 = ZERO — the "already authorized" ask was SPENT by
`14-2b`, so this is a REQUEST, not a ceiling edit**); the port + row in `maos-control` (**615/1500 =
+885**, the only roomy crate, and `14-2b` measured this exact shape at **+197 charged**) with its impl in
`maos-bin/src/cert_rotation.rs` (**16987/16987 = ZERO** — `14-2b` extended this same file by +55 raw,
composition root `main.rs:2757-2765`); and the gate leg in `xtask` (**41925/41932 = +7 against a +14
leg**). **Four measured asks at T8, funding ratified 2026-09-03.** **Extend `cert_rotation.rs`; do not add a file to
`crates/maos-bin/src/`** — `cohort_daemon_smoke_13_5c.rs:861-863`/`:937-941` assert
`SCANNED_SOURCE_FILES.len()` equals a live flat `read_dir` (**17 == 17** today) under **two** executors —
`xtask/src/check_reza_production_path.rs:147-160` and the whole-binary run at `discipline.yml:1927`.

**Idioms to mirror, not re-invent.** `RotationWindowSource` → `CohortRotationWindows`
(`maos-control/src/lib.rs:66-83` → `maos-bin/src/cert_rotation.rs:79-102`) is the exact port shape.
Poison handling in `maos-a2a-core` is in-file `.unwrap()` (`router.rs:740`); in `maos-a2a-tcp` it is
fail-closed `if let Ok(...)`. `PeerCertFingerprint` is `{ algo: String, hex: String }`
(`identity.rs:51`), `Clone` but **not `Copy`**.

**⚠ THREE exact-count source gates fence `router.rs`, and they are NOT where prior notes say.** They
live in **`crates/maos-a2a-core/tests/`** — *not* `maos-a2a-tcp/tests/`, which has no chokepoint file at
all — and there are **three**, not two: `cohort_manifest_chokepoint_12_1.rs`,
`cohort_consent_chokepoint_12_2.rs` and **`cohort_halt_receipt_chokepoint_12_3.rs`** (missed by every
prior note). All three `include_str!("../src/router.rs")` through a `strip_line_comments` helper and
assert exact counts: `.is_current(` == 0, `cohort_consent_decision(` == 2,
`cohort_manifest_gate.consent_decision(` == 1, `cohort_manifest_gate.consent_and_team(` == 2,
`.observe_receipt(` == 1, `HaltRegistry` == 0, `KernelHaltResolver` == 0. **All match live today.**
🔴 **The trap:** the `== 2` is on the **qualified receiver** `cohort_manifest_gate.consent_and_team(`.
Bare `consent_and_team(` in `router.rs` is **3** (`:940`, `:1802`, and a `#[cfg(test)]` impl at `:2115`).
**Do not conflate them.** A read accessor touches none of these — verify, do not assume.

**⚠ THREE counting conventions are in play for one file, and citing one for another is how a budget
argument goes wrong.** `crates/maos-a2a-core/src/router.rs` is **2619** raw `wc -l`, **1785** tokei
`code` (**this is what the ceiling measures**), and **1892** under the chokepoint tests'
`strip_line_comments`. `crates/maos-cohort/src/state.rs` is 1983 raw / 1656 tokei code.

**⚠ A LEFTOVER GIT WORKTREE WILL DOUBLE YOUR GREP HITS.** `git worktree list` shows a detached
worktree at `.claude/worktrees/agent-a9a6d76f71ebb443b` @ `ee3422d0` holding a full duplicate of
`crates/`. It is gitignored so it does **not** affect `kloc-check` (verified), but any `grep -rn` that
does not exclude it returns every hit twice. Exclude `.claude` explicitly.

**⚠ Adding a file to `crates/maos-bin/src/` is a THREE-part edit, not one** — bump
`SCANNED_SOURCE_FILES: [(&str, &str); 17]` → `; 18]` (`cohort_daemon_smoke_13_5c.rs:863`), add the
`include_str!` tuple, and ensure the new file contains **zero** occurrences of `manifest_scopes`
(asserted `:948-952`; only `enterprise_pdp_runtime.rs` is whitelisted at `:944`). Live count is **17 ==
17** today. The same file also pins `main.rs`: `CapabilityRegistryPort::record_invocation` == 3 (`:853`)
and `.collective_{method}(` == 1 (`:812`). **Extending `cert_rotation.rs` avoids all of it.**

**Do not touch:** `maos-domain` (−51, 14-7), `maos-audit` / `maos-iac` (both at zero),
`maos-kernel-core` (**24472 pinned with ZERO headroom**; a comment-only edit reds the baseline, because
`check_kernel_baseline.rs:109` counts raw lines with no exclusions and `:65` compares for equality).

## Tasks / Subtasks

- [x] **T0 — DONE 2026-09-03. (a) `14-2b` COMMITTED at `4657cace`; (b) ALL FOUR CEILINGS RE-DERIVED and
      AC1/AC2/AC3 LINE-PRICED. Results in §6 below — READ THEM BEFORE T2.** ⚠ **T0's SHAPE CHANGED 2026-09-03 (R2).** It is **no longer a fit check** — four crates
      are already known to be at or effectively at zero (`maos-cohort` 0, `maos-a2a-core` 0, `maos-bin` 0,
      `xtask` +7 vs a +14 leg), so the arithmetic is decided before the editor opens and the only honest
      T0 output is **a four-crate measured ask** (funding ratified 2026-09-03; the numbers are not).
      Line-price AC1/AC2/AC3 anyway — not to decide whether to proceed, but so the ask at T8 is a
      prediction the measurement can falsify. Use `14-2b`'s **measured** `maos-control` price (+197 charged
      for trait + route + three doubles + three tests) instead of an estimate.
- [x] T1 — Read `apply_reissue` (`state.rs:388-538`) end to end **and `install_cert_rotation`'s reconcile
      block (`state.rs:246-266`)** — the `CertRotationRefused` precedent AC1 mirrors is written at
      **BOTH** `:259` and `:504`, and `state.rs:251-254` says writing only one is the defect.
- [x] T2 — AC2: the `A2ARouterCore` read accessor + the `maos-cohort` reach through
      `PeerCertRotation.core`. Nothing else compiles without it. Price and reject `pub`, in writing.
- [x] T3 — AC1: **one helper, called from BOTH sites**; `None` ⇒ **no row**; parsed-value compare;
      after `reload` Ok / before `*cached`; 8-hex via a shipped helper; **apply-and-record, never refuse**.
- [x] T4 — AC3 (**REWORKED — read §7 and §8 first; the pre-2026-09-03 text is superseded**): a third
      `maos-control` source whose 404 predicate is *"no cohort state"*, NOT *"no rotation control"*;
      `declared` from the already-`pub` `manifest()`/`local_host()` (copy `main.rs:9960-9964`, **0
      `maos-cohort` lines**); `serving` via AC2's two-level accessor pair; the **three-valued** verdict with
      `unconfirmable` load-bearing in BOTH ways (pre-install, and never-received); `peers_observed`/
      `peers_total` read from the SAME `peer_convergence()` call `14-2b`'s route makes; and the last pull
      error carried on a `CohortDaemonBootstrap` field written at **BOTH** `main.rs:10135` and `:10154`.
      Existing bearer gate, read-only. **No late-bound handle, no `core()` accessor, no new crate edge.**
- [x] T5 — AC4: `t_14_2c_*` in **`crates/maos-bin/tests/`**, asserting **through the HTTP response body**;
      the two-reissue negative; the `reload`-errs negative; run `team_identity_13_6a.rs` and account for
      its incidental local move.
- [x] T6 — AC5: append one `check-cohort-mesh` leg, copying `14-2b`'s two verbatim
      (`-p maos-bin --test <file> <one_test> -- --ignored --exact`); the gate is at **37** legs, not 35.
      **Both proven-red vectors EXECUTED.** The leg costs ~14 charged `xtask` lines against +7 — it is part
      of the T8 ask, not free.
- [x] T7 — AC6: **ADD** a self-scoped RELEASE-HOLDS boundary + the N+1-artifact recovery ordering and both
      failure variants; (c.6) corrected not closed; **eight** dangling references re-pointed (five
      exact-key + the dotted `index.md:149` + `sprint-status.yaml:304` + the 9-vs-17 story count).
- [x] T8 — `cargo fmt --all`, measure the formatted tree, record EXACT MEASURED / ZERO HEADROOM, and
      present **ONE ask covering FOUR crates** — `maos-cohort`, `maos-a2a-core`, `maos-bin`, `xtask` — each
      with its own driver. ⚠ **There is no pre-paid ceiling edit: `14-2b` spent the `maos-a2a-core`
      authorization (R1).** Disown the foreign reds with owners and with the **re-measured** numbers:
      `maos-domain` −51 → **14-7**; aggregate **−8036** → **14-6** (state the three-way split: −51 not
      ours, −7646 D17 not ours, −390 `14-2b` not ours); `deferred-work.md:860` → **14-6**.

## Dev Agent Record

### Agent Model Used
gpt-5.6

### Implementation Plan

- AC2 reads plane C through the live `A2ARouterCore` → `PeerCertRotation` →
  `CohortManifestState` accessor chain. Rejected a public router field and a
  `pub(crate) core` escape hatch because every router/rotation field is private
  and Story 14-2b's `pin_generation` pair is the established narrow-accessor
  precedent. Rejected a fifth `install_cert_rotation` parameter because it
  snapshots the value Story 14-2d will make mutable.
- AC1 uses one parsed-value comparison helper after successful peer reload at
  both install and reissue sites. Missing plane C or local membership yields no
  divergence row; a mismatch appends `LocalLeafDeclarationMoved` without
  refusing the signed manifest.
- AC3 uses a dedicated `CohortSelfIdentitySource` because its 404 predicate is
  cohort-state absence, not rotation-control absence. The source reads signed
  and serving fingerprints live, gives direct divergence priority, reports
  `agree` only after every peer has a valid convergence observation, and
  otherwise reports `unconfirmable`. Per-peer pull failures remain in the
  `CohortDaemonBootstrap` transport-health carrier and clear on a successful
  pull at both production pull sites.

### Debug Log References
- Proven-red AC1 mutation: replacing the parsed fingerprint comparison with
  `false` failed `t_14_2c_local_leaf_reissue_is_diagnosable` at the expected
  audit-row assertion (`0 != 1`).
- Proven-red AC3 mutation: renaming the `self_identity` response key failed the
  same real HTTP test at the parsed body contract.
- `cargo test -p maos-bin --test t_14_2c_local_leaf_declaration -- --include-ignored`
  — 6 passed.
- `cargo run -q -p xtask -- check-cohort-mesh` — 38 independent hard-fail legs
  passed.
- `cargo test --workspace -- --test-threads=1` — 4,120 passed across 495
  suites; 118 intentionally ignored.
- `cargo fmt --all -- --check` and `check-kernel-baseline` passed; kernel
  baseline remained 24,472 lines.
- `kloc-check --json` reports only the two pre-existing owned reds:
  `maos-domain` 8,695 > 8,644 (14-7) and aggregate 155,466 >= 147,057 (14-6).
### Completion Notes List
- Added live serving-leaf accessors and one parsed-value divergence helper,
  invoked after successful reload at both install and reissue sites.
- Added the `LocalLeafDeclarationMoved` audit row without refusing a valid
  signed manifest; `None`, missing local membership, equal parsed values, and
  failed reloads emit no false row.
- Added authenticated `GET /v1/cohort/self-identity` reporting declared and
  serving fingerprints, three-valued verdict, peer observation counts, and
  per-peer pull failures from both production pull paths.
- Added real HTTP/reissue coverage, both mutation controls, and one blocking
  `check-cohort-mesh` leg.
- Added the self-scoped N+1 recovery boundary, including unavoidable outage,
  boot-brick, and boot-green permanent-partition cases. Re-pointed eight live
  stale ledger references; historical split provenance remains intact.
- Applied exact measured, zero-headroom grants: `maos-cohort` 5,793→5,841,
  `maos-a2a-core` 4,853→4,856, `maos-bin` 16,987→17,107, and `xtask`
  41,932→41,939. `maos-control` moved 615→803 inside its existing ceiling.
### File List
- `RELEASE-HOLDS.md`
- `_bmad-output/implementation-artifacts/14-2c-plane-c-local-leaf-declaration.md`
- `_bmad-output/implementation-artifacts/deferred-work.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `_bmad-output/planning-artifacts/epics/epic-14-scale-closers-ecosystem-readiness-v2-2.md`
- `crates/maos-a2a-core/src/router.rs`
- `crates/maos-bin/src/cert_rotation.rs`
- `crates/maos-bin/src/main.rs`
- `crates/maos-bin/tests/t_14_2b_cohort_convergence.rs`
- `crates/maos-bin/tests/t_14_2c_local_leaf_declaration.rs`
- `crates/maos-cohort/src/audit.rs`
- `crates/maos-cohort/src/rotation.rs`
- `crates/maos-cohort/src/state.rs`
- `crates/maos-cohort/tests/cert_rotation_14_2a.rs`
- `crates/maos-control/src/lib.rs`
- `docs/release/v2.2-capacity-envelope.md`
- `tests/coverage-matrix.yaml`
- `xtask/kloc.toml`
- `xtask/src/check_cohort_mesh.rs`
### Change Log

- 2026-09-03 — Implemented plane-C local-leaf declaration detection,
  authenticated self-identity diagnostics, behavioral/mutation coverage,
  blocking gate enrollment, recovery ledger repair, and exact measured grants;
  moved story to review.
- 2026-09-04 — §A6 review closed: 9 patches (change-gated audit row, in-transaction
  append, agree-requires-healthy-pulls verdict, live-pull harness leg + gate enrollment,
  (c.8) recovery ordering repair, 14-2a re-points, pre-install production test,
  frontmatter supersession) applied and verified; measured grants +16/+2/+14.

### Review Findings

§A6 review 2026-09-04 (non-author, glm-5.3): Blind Hunter + Edge Case Hunter +
Acceptance Auditor + Test Infrastructure Auditor (dev model `gpt-5.6`, so the
Test-Infra layer was mandatory), plus the non-author runtime layer executed
live: baseline `cargo test -p maos-bin --test t_14_2c_local_leaf_declaration
-- --include-ignored` = 6/6 green; three planted mutations each RED a live
assertion — (1) `state.rs` AC1 comparison inverted (`!=`→`==`): 4 tests red
including the gate leg; (2) `cert_rotation.rs` `diverged` guard inverted: the
source and gate-leg tests red; (3) `maos-control` HTTP body key `"verdict"`
renamed: gate leg red at the body assertion (`t_14_2c…:439`). Tree verified
byte-identical after restore. One incidental append to
`intent-lineage-coverage-report.md` (corpus-runner side effect) was reverted.

- [x] [Review][Patch] **`LocalLeafDeclarationMoved` must fire only when the local declaration CHANGES** [crates/maos-cohort/src/state.rs, `fn local_leaf_declaration_event`] — decided 2026-09-04 (Lunarpulse): emit-on-local-change. `local_leaf_declaration_event` currently emits on `serving != declared` at each successful apply, so a v3 that moves only another member still journals a "moved" row for version 3. Fix: compare the candidate's local fingerprint against the CACHED manifest's local fingerprint and emit only on a change (still gated on divergence vs the serving leaf, still after `reload` Ok / before `*cached`).
- [x] [Review][Patch] **Behavioral-evidence gap: `agree` branch and the pull-error lifecycle are unproven (AC3(c)(i)/AC3(d))** [crates/maos-bin/tests/t_14_2c_local_leaf_declaration.rs + xtask/src/check_cohort_mesh.rs] — decided 2026-09-04 (Lunarpulse): live-pull harness leg. No test asserts `verdict == "agree"` against the production adapter (`record_peer_convergence` is private with `expires_at_secs` staleness, so `peers_observed == peers_total` requires a real observed pull), and both `update_cohort_pull_health` sites in `main.rs` are unreachable from integration tests (delivered tests inject the map). Fix: copy the `t_14_2b_cohort_convergence.rs` construction — in-process distributor + transport, real pulls — then assert through the HTTP body: healthy mesh → `agree`; severed peer → pull error appears and the verdict degrades; restored peer → error clears. Enroll as an ignored `--exact` `check-cohort-mesh` leg.
- [x] [Review][Patch] **Local-move audit append sits outside the reload commit boundary** [crates/maos-cohort/src/state.rs:692-694] — with a fallible `CohortAuditSink` (legitimate deployment per `rotation.rs`'s own contract), a failed append returns `Err` after `reload` already moved the trust planes and wrote `MemberReissueAccepted`, but before `*cached` is replaced: cached manifest, live planes, and the durable accepted row then disagree — exactly the "planes and manifest agree or neither moves" invariant `reload` documents and enforces for its own commit event. Production sink always returns `Ok` (panics instead), so unreachable in production today; still a broken stated invariant plus a torn state under injected sinks. Fix: pass the pre-computed event into `reload` so it is appended inside the same transaction/rollback boundary (preserves AC1(a) ordering).
- [x] [Review][Patch] **`agree` verdict renders alongside current pull errors** [crates/maos-bin/src/cert_rotation.rs:211-219] — in §2's dominant case (host never received the reissue) `declared == serving` and convergence records stay `CONVERGENCE_OBSERVED` until `expires_at_secs` (~2× confirmation interval), so the endpoint reports `verdict:"agree"` together with `last_pull_errors:{…:"PIN_MISMATCH"}` for up to ~2 minutes of a total partition — a false all-clear in the story's target scenario. Fix: require `last_pull_errors.is_empty()` in the `agree` guard (falls to `unconfirmable`, which is the honest verdict while pulls fail).
- [x] [Review][Patch] **(c.8) recovery procedure omits the per-member static pin updates** [RELEASE-HOLDS.md, clause (c.8)] — boot arms (a)/(b) (`reconcile_transport_identity_with_manifest`, `main.rs:9980-10014`) make ANY member restart a boot-error unless that member's `tcp.peer_pins` and `file.peers` entries for this host also move to `F_new`; the shipped procedure distributes only the signed manifest TOML, so the first member restart bricks. Story §3 measured this ("a member whose TOML still says `F_old` cannot boot") but the shipped boundary text does not carry it. Fix: extend the artifact list and ordering in (c.8).
- [x] [Review][Patch] **(c.8) "let it propagate" is a dead step** [RELEASE-HOLDS.md, clause (c.8)] — an on-disk TOML is applied only at boot (`apply_reissue` never persists; every running member serves its cached v1; polling pulls cannot start propagation because nobody serves v2). The procedure must restart at least one non-target member (seed) to load and serve the new manifest before any propagation can happen; host-a restarts last. Fix: same (c.8) rewrite.
- [x] [Review][Patch] **AC6(c) incomplete: three live ownership statements still route to the deleted `14-2b-self-identity-rotation` key** [`_bmad-output/implementation-artifacts/14-2a-production-mtls-rotation-trigger.md:22`, `:509`, `:1070`] — these are current ownership claims ("moved to", "is owned by", "named as owner of the half"), not `split_from` provenance; T7's re-derived count missed them (line `:597` is historical narrative and is fine). Re-point to `14-2d-self-identity-rotation`.
- [x] [Review][Patch] **Pre-install `unconfirmable` is untested against the production adapter (AC3(c)(i) "BOTH MUST BE TESTED")** [crates/maos-bin/tests/t_14_2c_local_leaf_declaration.rs:280-297] — the only `serving: None` coverage is the `maos-control` stub double; `Fixture::boot` always installs rotation, so a regression that makes the real `CohortSelfIdentity` 404/`agree`/absent before `install_cert_rotation` stays green. Fix: one cheap test — loaded state, no rotation install, real `CohortSelfIdentity` through the HTTP route → 200 + `unconfirmable` + `serving: null`.
- [x] [Review][Patch] **`baseline_commit` frontmatter carries a stale self-contradiction** [`_bmad-output/implementation-artifacts/14-2c-plane-c-local-leaf-declaration.md:2`] — the field opens "`14-2b` COMMITTED at `4657cace` … Blocking 0 is DISCHARGED" and ends "`14-2b`'s dev pass … is LIVE AND UNCOMMITTED in this tree"; under this repo's R6 ruling a stale correction reads like the fix. Delete or mark the trailing sentence as superseded history.
- [x] [Review][Defer] **`manifest()`/`peer_convergence()` read under two separate locks** [crates/maos-bin/src/cert_rotation.rs:176-187] — deferred, pre-existing pattern composed by this change: a reissue committing between the two reads can render one request with an old `declared` and fresh convergence (transient false `agree`, self-corrects on the next poll); a single-snapshot read needs a new `maos-cohort` API (budget + API surface). Owner: `14-2d`, which owns the self-identity surface.

Dismissed (2): the gate leg "bypasses live A2A wiring" claim — AC4's normative text (real reissue, HTTP-body assertions, non-vacuous negatives) is met and the mutation evidence proves the enrolled leg's assertions are live; the residual daemon-composition gap is the pull-health Decision item above. The "non-author runtime layer missing" finding — discharged live during this review (evidence in the preamble above).

**Review closure 2026-09-04 (same session, all nine patches applied and verified):**
`cargo fmt --all` then `cargo test -p maos-cohort` (all targets 0 failed),
`-p maos-control` 0 failed, and the story file 9/9 including both ignored legs.
`check-cohort-mesh` PASSED with **39** independent hard-fail legs (the new
`self-identity-agreement-pull-health` leg enrolled alongside
`local-leaf-declaration-diagnosis`). Post-patch measured grants, formatted
before measurement, ZERO HEADROOM: `maos-cohort` 5841 -> 5857 (+16),
`maos-bin` 17107 -> 17109 (+2), `xtask` 41939 -> 41953 (+14, one leg);
`maos-a2a-core` unchanged at 4856. Foreign reds unchanged and disowned:
`maos-domain` 8695 > 8644 (D14, owner 14-7); aggregate 155498 >= 147057
(D17 + 14-2b, owner 14-6). `graphify update .` run after the code changes.
