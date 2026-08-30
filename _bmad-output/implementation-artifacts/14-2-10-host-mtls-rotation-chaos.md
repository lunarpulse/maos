---
baseline_commit: "`38c52811` — HEAD, **PLUS A FULLY STAGED, UNCOMMITTED INDEX** carrying all of Story 14-1 (19 paths, +8633/−1047, all in `git diff --cached`; bare `git diff` is EMPTY). ⚠ Do NOT copy 14-1's frontmatter sentence `HEAD, tree clean` — it was true when written and is false now. `git checkout 38c52811` REMOVES the harness this story is built on (`concurrent_dial_pairs`, `assert_fd_headroom`, `MeshNode.bound_at`, `Leaf: Clone`, `PlantKind::CertRotationRaceExploit`). Verify before measuring: `git status --porcelain` must show those 19 rows, all with a blank second column, plus this story file as `??`. If 14-1 is committed before dev starts, re-derive `baseline_commit` and RE-MEASURE the two zero-headroom walls — do not inherit these numbers. Every `file:line` below was measured on 2026-08-28 against that tree by SEVEN parallel read-only scouts (six at round-table I, one on the overlap mechanism for round-table II) plus TWO independent fresh-context validators; where a scout's finding is quoted it was re-checked here before it was written down, and three scout claims were REFUTED by that re-check (see Measured grounding)."
depends_on: "**`10-4b-mira-nash-bilateral-2-host-live-deployment-proof`** (done) — authored `crates/maos-a2a-tcp/tests/t_10_4b_rotation_real_timing.rs`, the real substrate this story scales, and ratified the floor-citation rule (R2-3). **`10-5-mature-v1-5-...`** (done) — AC6 ratified `passes_v15_floors` as the never-relax instrument. **`14-0-epic-14-preflight-decisions`** (done, `38c52811`) — register binding rule 3. **`14-1-100-host-churn-scale-envelope`** is NOT a dependency (14-1's own frontmatter says so, and the epic calls 14.1–14.6 `largely parallelizable`) but it IS this story's harness supplier and evidence template, and its work is uncommitted — see `baseline_commit`. That tension is Blocking condition 1."
blocks: "Nothing in Epic 14. Ships a new `maos-a2a-core` public surface (set-valued TOFU pin) that `14-5` and any future transport work inherit; the surface is additive and the single-valued path stays the default."
kernel_grant: "NONE and none needed. `check-kernel-baseline` GREEN at **24472 == 24472**, resolved from `xtask/kernel-core-baseline.toml` (`src_lines`) — never restated as a literal, per Epic-13 retro C1 (an OPEN epic citing any other value REDS via `check-epic-close-coherence`). Zero lines of `crates/maos-kernel-core/src` are touched by any AC. ⚠ The epic's 14.2 row says the rotation work is `out-of-kernel, maos-a2a-tcp` — **`rotation.rs` is in `crates/maos-a2a-core/`, not `maos-a2a-tcp`** (`epic-14-...md:37` and `:150`, wrong in both places). The zero-kernel-Δ conclusion survives (both crates are out-of-kernel; `t11_t12_chaos_absence.rs:169-178` structurally proves `maos-a2a-tcp` has no `maos-kernel-core` dep) but the BLAST RADIUS does not: AC2 touches `maos-a2a-core::tofu`, a crate consumed by `maos-a2a`, `maos-a2a-tcp`, `maos-cohort` and the shared harness. `just a test-infra change in one crate` is false and must not be written anywhere in this story's record."
kloc_grant: "⚠ **TWO CEILINGS ARE AT ZERO AND THIS STORY MUST TOUCH BOTH.** Measured live by the gate itself (`cargo run -q -p xtask -- kloc-check --json`) against the staged working tree: `xtask 40613 / 40613` = **0**; `maos-a2a-core 4785 / 4785` = **0** (the D10 wall); `maos-a2a-tcp 1246 / 1500` = 254; `maos-a2a 303 / 1500` = 1197. **`crates/*/tests/` and `xtask/tests/` are NOT charged** — verified at `xtask/src/kloc_check.rs:167-192` (`-e tests` at `:175-176`), which passes `-e tests -e benches -e examples -e fuzz` to tokei — so the ENTIRE 10-host harness, the load generator, the scene and every proven-red vector cost **zero**. Scaling 3→10 is free; only AC2's pin surface, AC4's disclosure and AC5's gate cost anything. ⚠ **THE RECLAIM AND THE GRANT ARE TWO DIFFERENT LEDGERS AND DO NOT NET.** Per-crate ceilings mean `maos-a2a-core` reclaim CANNOT fund an `xtask` ask. Ledger A — `maos-a2a-core`: AC6.1 frees **97** (`harness_3_host.rs`) + **38** (`metrics.rs`, orphaned by that deletion, zero other callers) = **135** measured lines. AC2's side-map spends ≈**33** + ≈**6** for the AC2.2.a uniqueness guard + ≈**9** for the RATIFIED `EPinMismatch::FingerprintCollision` variant (AC2.2.a.i) = ≈**48**. Its `xtask/error-catalog.toml` row is **uncharged** — `kloc-check` counts `--types Rust` only. **Net ≈ −87: comfortably negative, NO grant on the D10 wall, and nowhere near Blocking condition 8's budget trigger.** AC2.4.a's ≈**34** lines land in `maos-a2a-tcp` (1246/1500, 254 headroom), NOT here. ⚠ `maos-a2a`'s 1197 headroom is UNREACHABLE and must not be cited — `maos-a2a-tcp` deliberately does not depend on `maos-a2a` (`maos-a2a-tcp/src/lib.rs:8-9`). Ledger B — `xtask`: AC5.2's new gate module is the ask, and `check_scale_churn.rs` — the file it mirrors — is **395** tokei-code lines, so budget an `xtask` grant of that order against **zero** headroom at `kloc.toml:203`. **Net aggregate contribution is therefore POSITIVE (~+260), not negative** — say so plainly; do not launder a cross-crate reclaim into an xtask justification. Take Ledger A's reclaim FIRST, write Ledger B's code, `cargo fmt --all`, measure BOTH formatted, and only THEN ask. `kloc.toml:60-65` forbids a grant on an estimate; `kloc.toml:86-87` is the correctness-repair valve and AC2/AC5 are entitled to cite it BY NAME, as `j1-crosshost-2b` and `2c` both did on this same D10 wall. ⚠ `kloc-check` **exits 1 at HEAD** on two keys that are NOT this story's and MUST NOT be absorbed: `maos-domain 8695 > 8644` (D14, owner 14-7) and `_aggregate_hardfail 152078 > 147057` (D17, owner 14-6, owner 14-6). **`all gates green` is NOT an available done criterion** — see AC6.6."
model: "frontier-class allowlist {opus-4-8, gpt-5.5, gpt-5.6, glm-5.1, glm-5.2, glm-5.3, opus-5, equiv}. The literal token `allowlist {` is the boilerplate guard at `check_dev_model_used_populated.rs:302`. ⚠ **14-1's stated rationale for it is wrong and was disproved by mutation — do not copy it.** Removing the token from *this* frontmatter line changes nothing: `:339` tests `t.starts_with("Model:")` **case-sensitively** against a lowercase `model:` key, and frontmatter precedes `### Agent Model Used` so it never enters `section_words` (`:330`). The guard is real, but only against a dev **copying this text into** the Agent-Model-Used section or a `Model:` line — which is exactly what a dev in a hurry does, so keep the token. ⚠ **`equiv` is prose, NOT a match token:** `FRONTIER_FAMILIES` (`check_dev_model_tier.rs:45-48`) is exactly `{opus-4-6, opus-4-7, opus-4-8, opus-5, gpt-5.5, gpt-5.6, glm-5.1, glm-5.2, glm-5.3}`; a dev who records `equiv` reds a Blocking gate. `glm-5.3` is present per 14-1's D-B amendment (staged tree only — absent at `38c52811`)."
review: "§A6 full-layer net (Blind + Edge Case + Acceptance + **Test-Infra** + runtime execution) — **NON-DEGRADABLE**, per `epic-14-...md:9` naming 14.1 and 14.2 the two highest test-infra-risk stories. ⚠ **TWO passes, not one** (AC2.6, ratified round-table II): the first runs on AC2's security surface ALONE when T4 completes and before any T5 work, because a TLS serving-path swap + a trust-set widening + an identity-resolution change do not fail like a load harness or a gate, and one net over all of them reviews none of them properly. Rationale specific to THIS row: the story's own AC2 adds a variant to a security predicate (`verify_pinned_sync`) whose whole job is to say no, and NFR-Sec-12 requires it to stay 100%. A widened trust set that still compiles and still passes every existing test is the exact failure this net exists to catch. The runtime layer MUST be run by a NON-AUTHOR and MUST watch a planted mutation red the drop count and the pin-set closure live — **not read a committed report** (11.3's bar, and `mtls-rotation-chaos-report.md` is a committed report of canned constants sitting in this very directory). 14-1 ran its review TWICE for exactly this reason; book the non-author runtime layer from the start."
---

# 14-2 — 10-host mTLS rotation chaos under load (NFR-Sec-13)

Status: **done**. Second dev story of Epic 14. Preflight: seven parallel read-only scouts +
fresh-context validator + **two round-tables**, baseline `38c52811` + staged index, **ZERO kernel-Δ
@24472**, 6 ACs. Round-table II (2026-08-28, unanimous, criterion *per spec + long-term correctness*)
re-scoped AC2 after measurement: the pin set **opens a cross-peer impersonation vector** unless
guarded (AC2.2.a), there are **six** comparison sites not four (AC2.2), the window is a **side-map**
not a `TofuPin` field (AC2.1), the config-fingerprint "missing API" was a **mis-ordering** the spec
already answers (AC2.0), and the security surface gets **its own §A6 pass mid-story** (AC2.6).

> ### The epic told this story to scale two things that do not exist, and named a gate that is not the gate
>
> The Epic-14 sketch for 14.2 (`epic-14-scale-closers-ecosystem-readiness-v2-2.md:72-78`; the verbatim floor sentence is at `:37`) reads as a
> parameter bump: *"scale the ratified `rotation.rs` floors to 10 hosts"*, *"rotation-during-load ...
> then cert/key rotation across the 10-host mesh"*, *"`check-rotation` gate scale-out (10-host
> leg)"*. Its line `:20` lists **rotation-during-load** and **one-generation-overlap** as substrate
> already delivered by 10.4b/10.5. **Both are false at HEAD**, and they are the two largest pieces of
> this story:
>
> 1. **There is no load.** `const HOSTS: [&str; 3]` (`t_10_4b_rotation_real_timing.rs:38`) drives a
>    strictly **sequential** N×N sweep — `.await` sits inside the inner loop with no `join`, no
>    `spawn`, no stream (`:146-170`). **Maximum in-flight requests during the drill is 1.** Worse,
>    the load is scheduled entirely OUTSIDE the rotation window: `sweep1` fully drains at `:212`,
>    THEN `t0 = now_ns()` at `:220`, THEN `drop(mesh_old)` at `:221`, and `sweep2` does not begin
>    until `:265`. **No frame can be interrupted by rotation because no frame exists during
>    rotation.** At 10 hosts this becomes 90 sequential dials — still a reachability sweep. **The
>    rotation drill** has no in-flight count, no generator, no rate, no concurrency and no drop floor.
>    (The *repo* does have concurrency primitives — `DIAL_CONCURRENCY = 16` and
>    `concurrent_dial_pairs` at `support/mod.rs:710,:721`, `RACE_IN_FLIGHT = 6` with
>    `buffer_unordered` at `t_11_3_scale_churn.rs:800,:989` — all shipped by 14-1, all reusable, and
>    AC1.2/AC3.4 instruct the dev to reuse exactly them. The rotation drill uses none of them.)
> 2. **There is no overlap.** §7.2.1.a's load-bearing sentence is *"agents accept either cert on
>    inbound handshakes"* during `[t_provision, t_revoke + T_grace]`. The transport serves ONE cert
>    (`transport.rs:899`, `with_single_cert`); the TOFU store holds ONE pin per peer
>    (`tofu.rs:134`); and `pin_first_contact` **explicitly rejects** a second pin — *"pin already
>    exists — use re-pin consent path"* (`tofu.rs:242-249`). The drill concedes it in its own header
>    (`:11-13`): *"`TcpA2ATransport` exposes no in-place cert-swap API, so rotation is modelled as
>    old-mesh-teardown + new-mesh-bind."* `compute_t_grace` is computed at `:291` and then **never
>    used to schedule anything**. The only set-valued rotation pin in the workspace is
>    `crates/maos-cohort/src/pin.rs:47-50` — the **Ed25519 cohort-authority** key, a different key in
>    a different crate.
> 3. **`check-rotation` does not exist.** The sketch names it three times (`:37`, `:77`, `:143`).
>    The gate is `check-rotation-real-timing` (`gate-registry.toml:273`). Third epic-sketch phantom
>    in a row, after 14-1's nonexistent N-param and nonexistent eviction.
>
> **And the finding the round-table surfaced that outranks all three.** The banked sprint-status
> finding (2) says this story's load defect is *"SAME DEFECT AS 14-1's `blast_targets: 2`, not merely
> analogous"*, and `.memlog.md:612` sharpens it to *"IDENTICAL"*. **That is wrong, and acting on it
> would ship the bug.** 14-1's was a **magnitude** defect — an input held at 2 below a floor of 5,
> repaired by handing the adversary the real peer set. This is a **temporal** defect: the traffic is
> scheduled outside the window it is supposed to be measured across. **No value of N repairs it.** A
> dev who reads "identical" will fix the load by raising N to 10, ship 180 sequential dials, and
> leave the hole exactly where it was. Same consequence — an unfalsifiable binding claim — different
> repair. Say the difference out loud, once, in the Dev Agent Record.
>
> **What the story gets for free in exchange.** Define a drop honestly and the proven-red vector
> arrives without being invented: `drop(mesh_old)` at `:221` closes listeners with live connections
> attached, so **a real in-flight window is RED at HEAD, today, with no injected fault.** That RED is
> AC1's oracle and AC2's reason to exist, and the fix for it is the overlap the epic thought it
> already had. The measured cost of the whole 10-host envelope is ~0.75 s of dials against a
> 15-minute job timeout and well under `fd_headroom_for(10) = 200` against `ulimit -n` 524,288 —
> **CI can host this**, and any advisory-substrate disposition here would hold ABSENT over a
> substrate that provably exists, which is 14-1's AC6.2 inversion one story later.

---

## Blocking conditions

1. **The baseline is a commit PLUS a staged index, and the story cannot be built on the commit
   alone.** `HEAD = 38c52811`; all 19 of 14-1's paths are staged, `git diff` is empty and
   `git diff --cached` carries everything. Every primitive AC1/AC3 depend on —
   `concurrent_dial_pairs` + `DIAL_CONCURRENCY` (`support/mod.rs:710,721`), `assert_fd_headroom` /
   `fd_headroom_for` (`:764,:820`), `peak_rss_kb` (`:801`), `MeshNode.bound_at` (`:468`),
   `Leaf: Clone` (`:115`), `PlantKind::CertRotationRaceExploit` (`:828`), `RunnerEnvelope`
   (`t_11_3_scale_churn.rs:190-245`) — exists ONLY in that index. Do not stash, do not check out
   `38c52811`, do not "measure at HEAD". If 14-1 lands as a commit first, re-derive the baseline and
   re-run `kloc-check` before touching anything.

2. **`passes_v15_floors` is a pure alias and NFR-Sec-13's arithmetic is never evaluated anywhere.**
   `rotation.rs:134` is literally `let passes_v15_floors = passes_v10_floors;`. That is CORRECT and
   must stay — it is 10.5 AC6's ratified instrument, because NFR-Sec-13's literal numbers (median
   ≤60 s / p99 ≤5 min) are **LOOSER on every comparable axis** than the ratified v0.7/v1.0 floors
   (60 000 > 30 000; 300 000 > 90 000) and adopting them would REGRESS the gate. The trap is
   documented twice in one file — doc comment `rotation.rs:81-85`, inline comment `:130-134` — and
   any edit must keep both in sync or one will lie. **Any AC in this story containing `<= 300_000` on
   the propagation p99 axis is the 10.5 AC6 regression, verbatim.** Cite NFR-Sec-13 by name;
   implement only the table in AC4.1.

3. **A green must be able to have been red.** Carried verbatim from 14-1's blocking condition 5,
   which named this story as its inheritor: *"If the answer is 'nothing the system does' — because a
   fixture caps the input, as `blast_targets: 2` does today — the leg is documentation."* This
   story's three instances, all measured: the zero-drop assertion cannot fail on a rotation fault
   (Blocking condition above); `post_grace_reject_count` is the literal `0` at
   `t_10_4b_rotation_real_timing.rs:298` so the `≤0.1%` floor is asserted against a constant; and the
   `end_to_end` distribution is a **constant vector** (`t_0` and `t_2` are single shared scalars at
   `:281-288`) so `end_to_end_p50_ms == end_to_end_p99_ms` on every run at every N, making the v1.0
   `≤60 s`/`≤150 s` pair two floors over one measurement.

4. **`host_count` is a self-declared label that nothing reads, and it is the grep trap's landing
   zone.** `rotation.rs:63` declares it, `:92` accepts it, `:138` stores it; it is never compared to
   `per_agent.len()`, never used in a floor, never used in `percentiles`. Calling
   `from_per_agent("10-host-drill", 10, …, per_agent_of_len_3, …)` yields a report that SAYS ten
   hosts and MEASURED three, and every floor still passes. 14-1 closed exactly this with
   `ChurnDrillReport::distinct_host_count()` (fn `churn.rs:203-205`, rule at `:201-202` — *"The ≥25 host floor is asserted
   against THIS, never a literal `host_count`"*). Rotation has **no** such witness. AC3.2 builds one.

5. **The canned twin is alive, it is named like the thing a dev will grep for, and its module header
   advertises machinery it does not have.** `crates/maos-a2a-core/src/chaos/harness_3_host.rs:4-12`
   promises *"three `tokio::spawn`ed agent tasks"*, a *"Synthetic load generator emit[ting] IAC
   frames between B↔C"* and *"`tokio::time::pause()` + `advance()`"*. **None of that is in the
   file.** `run_drill` (`:52-89`) computes `t_1_ns = prop_ms * 1_000_000` from `DrillConfig` seeds;
   `t_0_ns` is the literal `0`. Its consumer asserts `passes_v07_floors && passes_v10_floors`
   (`cert_rotation_chaos_3_host.rs:16-19`) on seeds with 2×–7.5× margin on every axis. Its
   `DrillConfig.host_count: u32` can be set to `10` by editing one literal, producing a fully green
   "10-host rotation drill" that touches no socket — while `agent_count` is computed separately at
   `:59-62` from the seed vectors' length, so the two silently disagree with nothing binding them.
   **The honesty label sits 47 lines BELOW the header that over-claims.** This is 11.3's deleted
   `run_scaffold` wearing rotation's hat, and the epic itself flags 14.2 as carrying that exact risk
   (`epic-14-...md:9`). AC6.1 retires it — one PR or not at all (11.3 F5).

6. **The sprint row's "carries NO EXPIRY" is false, and the truth is worse.**
   `crates/maos-a2a-core/src/chaos/mod.rs:10-11` states *"The calibration window is bounded — the dev
   record documents the v0.7 hard-fail flip date."* **No date was ever written.** An **AC**
   delegates to the dev record (`6-3-...md:540`, inside `### AC5` at `:447`; the Dev Agent Record does
   not start until `:1013`), and `grep -n 'v0\.7'` over that file returns 14 occurrences across 12
   lines (`:9,14,29,447,459,471,540,565,758,955,1100,1239`), **not one of them a calendar date**, with `:29` explicitly scoping the flip out. The project has passed v0.7 on both owning NFRs
   (`coverage-matrix.yaml:1456` puts NFR-Rel-9 at v0.8; `:1532` puts NFR-Sec-13 at v1.5). A sunset
   condition was written, version-keyed, passed, and nothing noticed because the thing it pointed at
   does not exist. Do not repeat the sprint row's weaker phrasing; the accurate claim is falsifiable
   and this one is not.

7. **TWO Blocking gates are RED at HEAD on rows that are not this story's, and 14-2 will add a second
   violation inside one of them.** (a) `kloc-check`: `maos-domain 8695 > 8644` (D14, owner 14-7) and
   `_aggregate_hardfail 152078 > 147057` (D17, owner 14-6). ⚠ **"Recalculable only at an epic
   retrospective" is the SUPERSEDED framing** — `kloc.toml:61` permits recalculation *"at an epic
   retrospective, **or** under an explicitly authorized measured grant"*, and 13.6d/13.6e/`j1-demo`
   all used the second door (`epic-14-preflight-decisions.md:83`, restated `:316`). 14-2 has a
   measured delta, so the door exists — but the row is **still 14-6's**, and this story must not walk
   through it without an explicit founder authorization. The AC6.1 reclaim frees 135 lines in a *different crate* while AC5.2 spends ~395 in
   `xtask`, so the aggregate moves **up**, not down. **Any sentence claiming the reclaim makes
   `kloc-check` green — or that 14-2's contribution is net-negative — is false.** (b) **`check-dev-record-completeness` EXITS 1 TODAY** on
   `deferred-work.md:860: STALE owner \`14-1\` — sprint status is \`done\``, a residual of 14-1's own
   staged status flip. ⚠ Measure this with `cmd >/dev/null 2>&1; echo $?` — piping the gate through
   `head`/`tail` makes `$?` report the PIPE's status and it will look green. (c) **This story sets the
   same trap for itself:** `deferred-work.md:864` names **14-2** as owner, and
   `check_dev_record_completeness.rs:266` classifies an owner whose sprint status is `done` as
   `OwnerBucket::Stale`, which `:632-637` turns into a violation. The row is inert while 14-2 is
   `ready-for-dev` and becomes a **second** RED the instant the story closes — unless AC6.4.b closes
   and re-owns it **in the same commit as the status flip.** Do not write "and no others" without
   re-running BOTH red gates at close.

8. **THE HALT TRIPWIRE — three mechanical triggers. The action is STOP AND RE-SCOPE, never degrade.**
   Ratified by round-table II, criterion *per spec + long-term correctness*. None requires convening
   anyone; each is something a machine or a tired dev recognises at 2am.
   - **Budget** — AC2's pin work exhausts the 135-line reclaim and needs a grant on the
     `maos-a2a-core` D10 wall. `kloc-check` says so without an opinion.
   - **Soundness** — the AC2.2.a uniqueness guard **cannot** be written such that a colliding `next`
     is refused. Not "hard" — *cannot*. A dev who cannot build the guard will otherwise ship the pin
     set anyway and file the guard as follow-up, which is the unsound mechanism in production.
   - **Oracle** — T1's in-flight window does not come back red, or T4's does not reach zero. Either
     means the **model** is wrong, not the code.
   ⚠ Counting production sites was explicitly **REJECTED** as a trigger: the count went 4 → 6 in half
   a day, and mis-counting sites is how this story got here.

---

## Measured grounding (2026-08-28, at `38c52811` + staged index)

Every "actual" carries a `file:line` and was re-verified by hand after the scout that produced it.

| Claim (epic §14.2 sketch / sprint row / this preflight's own brief) | Measured | Verdict |
|---|---|---|
| "rotation-during-load" delivered by 10.4b/10.5 (`epic:20`) | strictly sequential, `.await` inside the inner loop, max in-flight = 1 (`t_10_4b...rs:146-170`); load scheduled entirely outside `[t0, sweep2]` (`:212`,`:220`,`:265`) | **FALSE** — AC1 is net-new |
| "one-generation-overlap" delivered by 10.4b/10.5 (`epic:20`) | `with_single_cert` (`transport.rs:899`); one pin per peer (`tofu.rs:134`); second pin rejected (`tofu.rs:242-249`); `build_pin_store` calls `pin_first_contact` per pin so a duplicate `peer_id` is a `Config` error (`config.rs:119-137`) | **FALSE** — AC2 is a build, and the test-side two-mesh dodge is impossible under one peer id |
| "`check-rotation` 10-host leg" (`epic:37,:77,:143`) | no such gate name anywhere in `xtask/`, `.github/` or the registry; the gate is `check-rotation-real-timing` (`gate-registry.toml:93,:273`) | **PHANTOM** |
| "`rotation.rs` out-of-kernel, **`maos-a2a-tcp`**" (`epic:37,:150`) | `crates/maos-a2a-core/src/chaos/rotation.rs`; `maos-a2a-tcp/src/` holds only `config/error/lib/transport/verifier.rs` | **WRONG CRATE** — conclusion survives, blast radius does not |
| "the live 10-host leg advisory-substrate-gated per E11 A2" (`epic:78`) | 10 listeners, 90 dials/sweep, ~2.05 ms/dial ⇒ ~0.75 s for 4 sweeps; peak fds well under `fd_headroom_for(10)=200` vs `ulimit -n` 524,288; job timeout 15 min (`discipline.yml:2656`) | **PREMISE FALSE** — same inversion 14-1 was forced to correct |
| sprint row (2): the load defect is "IDENTICAL" to `blast_targets: 2` (`.memlog.md:612`) | 14-1's was a magnitude defect (input below floor); this is a temporal defect (traffic outside the window). No N repairs it | **OVER-CLAIMED** — same consequence, different repair |
| this preflight's brief: "the zero-drop assertion cannot fail" | `t_10_4b_rotation_proven_red_drop_reachability` (`:371-437`) drives it to 2/6 and asserts the exact surviving topology (`:422-432`) | **REFUTED** — it is a working *reachability* oracle; only the "under load" qualifier is unfalsifiable |
| this preflight's brief: "NFR-Rel-9 names p50 30 000 / p99 90 000" | NFR-Rel-9 is *"≤5 s p99 under 10⁴ concurrent capability-token validations, v0.8"* (`non-functional-requirements.md:28`). The 30/90 s are NFR-Sec-13 + arch §7.2.1.b | **REFUTED** — and already corrected on 2026-08-28 (`:26`); still mislabelled in-repo at `gate-registry.toml:270` and `discipline.yml:2653` |
| this preflight's brief: "the gate does not exist" | `discipline.yml:2653-2666` is a real, hard-failing job wired into the v1.0-ship-gate `needs:` at `:3377`. Nine `EXPECTED_GATES` entries legitimately have no subcommand — the registry namespace is CI job names | **REFUTED** — write *"the enforcement is a bare `cargo test`, not a gate"* |
| sprint row: "its only consumer is that one test" | six references: `chaos/mod.rs:14`, its own 3 in-file tests (`:91-137`), 3 of 6 tests in `cert_rotation_chaos_3_host.rs`, `discipline.yml:1644-1645`, and `check_epic_6_bridge.rs:1216-1245` which PARSES that workflow line and reds if the target file vanishes | **FALSE** — but the cutover is *cheaper* than feared (AC6.1) |
| sprint row: `harness_3_host.rs` "carries NO EXPIRY" | `chaos/mod.rs:10-11` bounds the window and points at a flip date that was never written; project passed v0.7 on both NFRs | **FALSE, and the truth is sharper** |
| sprint row (3): "`p99` IS THE MAXIMUM" | `p99_idx = floor(len·0.99).min(len−1)` (`rotation.rs:163`); first n where it is not `len−1` is **101** | **TRUE** |
| this preflight's brief: at 10 hosts, samples might be N×N=90 | samples are **per-agent**, built by `per_agent.iter().filter_map(...)` (`rotation.rs:99-107`); the 90 dials feed `first_success_ns[j]` first-writer-wins (`t_10_4b...rs:164-167`) and **89 are discarded**. n = **10** | **REFUTED** — n=10, and it looks more plausible than 14-1's n=3, which makes it worse |
| sprint row (5): `check-rotation-real-timing` has no xtask subcommand | confirmed: `cargo run -q -p xtask -- check-rotation-real-timing` → *"unrecognized subcommand"*, `EXIT=2`; no `check_rotation_real_timing.rs` among the 70 `check_*.rs` | **TRUE** |
| the gate's `[[ship_gate]]` disposition governs its behaviour | `read_disposition` (`gate_common.rs:192-205`) is consulted only by gate *modules*; this gate has none. The row says `v1_0 = "advisory"` while the CI job hard-fails the aggregate. The graduation is **decorative in both directions** | **NEW — worse than the missing subcommand** |
| `check_ship_gate_completeness` proves the gate is honoured | two string-set comparisons (`:190-198` needs-vs-EXPECTED_GATES, `:206-222` registry-vs-EXPECTED_GATES) plus `ledger_ship_badge_problems` (`:252-276`); a gate is "complete" iff its name appears in a YAML `needs:` list and a TOML `name =`. `ShipGateEntry` has exactly two fields (`corpus_types.rs:84-87`) | **NEW — audits spelling, not referents** |
| kernel-Δ baseline | `check-kernel-baseline` GREEN, **24472 == 24472** | ✅ |
| `maos-a2a-core` headroom | **4785 / 4785 = 0**, measured by the gate itself | ✅ (worse than 14-1's start of 1) |
| `xtask` headroom | **40613 / 40613 = 0** | ✅ |
| "the config-fingerprint update path is a missing API" (this preflight, round-table I) | §7.2.1.a: *"Agents MUST be provisioned with the replacement cert…"*; `A2APeerConfig.cert_fingerprint` (`config.rs:52`) is the operator DECLARATION and moves at `t_provision`. Sites 5/6 pass at every phase under `provision → swap → close` | **REFUTED — the fork was a mis-ordering.** No new API; none may be added |
| "FOUR pin-comparison sites" (this story's own first draft) | **SIX.** `router.rs:990-994` (every outbound frame) and `router.rs:1307-1310` (every inbound) compare store-pin vs operator-config fp | **CORRECTED** |
| set-valued pin is purely additive / safe | `find_active_pin_by_fingerprint` (`tofu.rs:177-186`) is first-match-wins over a `DashMap` seeded per-process by `RandomState`; no fingerprint-uniqueness guard exists at `tofu.rs:244`, `:250`, `config.rs:118-137`, or `router.rs:201-214` | **FALSE — it OPENS a cross-peer impersonation vector** (AC2.2.a). The guard is the control |
| ADR-054 §2 / `pin.rs` is precedent for the window close | `PinnedAuthorityKeys` (`maos-cohort/src/pin.rs:45-48`) has no retire, no close, no expiry, no timestamp | **PRECEDENT COVERS CARDINALITY ONLY** — the close clause is new |
| a `T_grace`-expiry window is constructible | `now_ns()` (`tofu.rs:376-384`) is an `AtomicU64` counter documented "NOT a TTL" (`:23`, `:376`); `compute_t_grace` returns a `Duration` | **NOT CONSTRUCTIBLE** without injecting a real clock; explicit close instead |
| swapping the served cert requires a rebind | `resolve` runs once per ClientHello (`rustls server/hs.rs:441-446`); rustls does no renegotiation (`client/hs.rs:370`, `common_state.rs:985-987`); `TlsAcceptor` re-reads the same resolver `Arc` per connection (`tokio-rustls server.rs:42`) | **FALSE** — interior mutability suffices, ≈34 lines, in-flight connections unaffected |
| `check-dev-record-completeness` is green at HEAD | **exits 1** on `deferred-work.md:860: STALE owner \`14-1\`` — measured with `cmd >/dev/null 2>&1; echo $?`, NOT through a pipe | **RED (pre-existing, 14-1's)** |
| decision-register rows targeting 14-2 | **zero.** No `Target story` cell and no `Deadline` cell names 14-2; moving out of `backlog` fires nothing (`check_decision_register.rs:418-423`) | ✅ — unlike 14-1, no register disposition is required |

**Substrate cost model (10 hosts, derived from 14-1's measured 2.05 ms/dial and 200-key/3 ms figures):**
90 dials per sweep × 4 sweeps ≈ **360 handshakes ≈ 0.75 s**; 10 listeners + 2 fds per in-flight dial +
~10 TempDirs, peak well under `fd_headroom_for(10) = 200`; 20 ECDSA P-256 leaves ≈ sub-ms. Against a
15-minute timeout and `ulimit -n` 524,288. **Re-measure on the runner (AC3.5) — do not inherit these.**

---

## Acceptance Criteria

### AC1 — The watchable N=3 rotation scene runs on real in-flight traffic, and the drop is defined before it is counted

**AC1.1 — Write the drop definition down first.** It does not exist anywhere in the repo today.
There is no drop counter in the transport, the router, or either chaos report; the three atomics on
`TcpA2ATransport` (`intake_entered` `:497`, `active_connections` `:502`, `last_dial_attempts` `:507`)
are gauges and attempt counters, none a failure counter. The definition this story adopts, and which
must appear verbatim in the test module doc and in `coverage-matrix.yaml`'s NFR-Sec-13 notes:

> **A conversation drop is a frame issued before `t_0` that never received a verdict — neither an
> ACK nor any typed NACK — by `t_2`.**
> It is **not** a retried dial (retries are internal and a retried-then-succeeded dial returns `Ok`,
> visible only as `last_dial_attempts()`). It is **not** an `IntentDeniedAtPeer` (`router.rs:1045`) —
> that is a completed conversation answered "no". It is **not** a post-rotation reachability failure;
> that is the existing Phase-2 assertion, already proven-red at `:428-432`.

The trigger condition — *"are `W` dials confirmed in flight?"* — is answerable from
`active_connections()` (`transport.rs:502`), the RAII connection gauge, summed across mesh nodes; it
is the one existing observable that fits, and without it AC1.2's `select!` has nothing to arm on.
Drops themselves are counted **test-side by sequence number** using 14-1's channel-before-resolve idiom
(`t_11_3_scale_churn.rs:959-965`, `:966-988`) — the completion timestamp is recorded *before* the future
resolves, which is what makes "was this frame in flight at `t_0`?" answerable at all. 14-1 hit and
solved this exact accounting bug in its round 2 (P1, documented inline at `:960-970`). **No
production change is required to count drops** — resist adding a typed `TcpTransportError` variant
for it; a rotation-caused ECONNREFUSED and a genuine reset both land in `Io(String)`
(`error.rs:39`) and disambiguating them is not this story's job.

**AC1.2 — The load is a real in-flight window, and it is EXPECTED to be red before AC2 exists.**
⚠ **This is a prediction, not a measurement** — no such window exists today (that is the defect), so
it has no Measured-grounding row and must not be written as though it were observed. The prediction
is mechanical: `drop(mesh_old)` (`:221`) closes listeners with live connections attached, so
outstanding frames take ECONNRESET → `TcpTransportError::Io` (`error.rs:39`) → `Err`. **The oracle is
`drops > 0`. It is never `drops == W`, anywhere in this story** — dials that complete in the instant
before the teardown legitimately succeed, so a run showing 5 of 6 is the prediction confirmed, not
the AC failed. Any `assert_eq!(drops, W)` is a flake by construction. If it comes back
`0`, stop: either the window is not actually open at `t_0` or the drop accounting is wrong, and
finding that out is worth more than the AC.

**`W = min(DIAL_CONCURRENCY, pairs.len())`**, and it must be ≥ 2 or the window is not a window.
⚠ A bare `W = DIAL_CONCURRENCY` (16, `support/mod.rs:710`) is **unsatisfiable at the N this task runs
at**: T1 builds the window on today's 3-host drill, which has `3 × 2 = 6` directed pairs, so a
`buffer_unordered(16)` stream never reaches 16 in flight, the arming condition is never true and the
`select!` never fires. Concretely: **W = 6 at N=3, W = 16 at N=10** (90 pairs). Do not "fix" this by
cycling the pair set — repeated dials to the same pair are not additional conversations.
That many readonly frames are in flight across the mesh at the instant `t_0` fires. Rotation
is initiated **while the sweep is still running**. Structure: a `buffer_unordered(W)` stream over the
pair set with an unbounded mpsc completion channel; once `W` dials are confirmed in flight, fire
`t_0` and the generation swap concurrently via `tokio::select!` (**not** `biased` — 14-1's rule at
`:1013-1041`, *"the poll order IS the race"*); drain the channel with a `t_2` cut-off
(`:1045-1052` shape). This is 14-1's race loop with two nouns changed: `detected_at` → `t_0`,
`blast_peers` → `verdicts_received`. **Demonstrate the RED before writing AC2**: against today's
teardown-rebind the drop count is `W`, not 0, with no injected fault. Record that run in the Dev
Agent Record — it is the story's free proven-red vector and its justification for AC2.

**AC1.3 — Scene shape, copied structurally from `t_14_1_churn_scene.rs`.** A narrating test at
`crates/maos-a2a-tcp/tests/t_14_2_rotation_scene.rs` (uncharged), `SCENE_N = 3`, non-`#[ignore]`d, one
test. Beats: pre-rotation mesh reconcile → in-flight window established (frames outstanding, count
published) → generation swap under load → zero drops → post-grace old-cert rejection observed. Render
via the `struct Beat { name, observed, envelope }` two-column table (`:71-77`, `:232-251`) using the
`EvidenceState` wire strings (`gate_common.rs:331-352`) exactly as the scene renders them at
`t_14_1_churn_scene.rs:66-67` — `PROVEN_BLOCKING` / `ABSENT`, never the bare word "PROVEN".

**AC1.4 — The non-substitution guard is structural, not verbal.** Same beat-set at **both** N=3 and
N=10, in **two columns**; a beat the N=10 leg did not execute renders **`ABSENT` in its own column,
never a trailing footnote**. 14-1's named failure mode applies unchanged: *someone screenshots the
three-host scene for a deck and the footnote is outside the crop.* **The scene executes at N=3 only**; every N=10 beat renders `ABSENT` in its own column, exactly as
14-1's scene renders its N=100 column (`t_14_1_churn_scene.rs:226-230`, `:246-251`). Do not build a
10-host scene — AC1.5's arithmetic did not budget one. **The scene is never a gate-leg oracle for any
NFR floor** — it narrates; AC3 measures.

**AC1.5 — Where it lives is decided by arithmetic, not taste.** A narrating test under
`crates/maos-a2a-tcp/tests/` (uncharged, `kloc_check.rs:175-176`), **not** a new `xtask/src/demo_*.rs`
— `demo_j1.rs` is 1179 charged lines and xtask has **zero** headroom. Do not re-litigate at dev time.

### AC2 — One-generation overlap is built as provision → swap → close, and NFR-Sec-12 still holds at 100%

**Founder-ratified 2026-08-28; re-scoped by round-table II the same day after measurement moved four
of its own clauses.** The cheap alternatives stay measured out: two co-resident meshes under one peer
identity is impossible (`crates/maos-a2a-tcp/src/config.rs:119-138` → `pin_first_contact` → `tofu.rs:242-249` rejects the
duplicate), and widening `HandshakeRetryPolicy` to absorb the rebind gap would retry on `Io` and mask
genuine partitions — §7.2.1.a scopes retry to `BAD_CERTIFICATE` / `CERTIFICATE_EXPIRED`, and
`PIN_MISMATCH` is **non-retryable** (`t_10_4b_rotation_real_timing.rs:363-364`, `:374-378`).

**AC2.0 — THE ORDERING IS `provision → swap → close`, AND IT IS WHAT MAKES THE REST CHEAP.** §7.2.1.a's
first sentence — *"Agents MUST be provisioned with the replacement cert at least `T_grace` before the
old cert's revocation timestamp"* — settles the question the earlier draft treated as a design gap.
`A2APeerConfig.cert_fingerprint` (**`crates/maos-a2a-core/src/config.rs:52`**) is the **operator's declaration**, and it moves at
`t_provision`, not at swap time. Walk it:

| Phase | `cfg.cert_fingerprint` | store pin | sites 5/6 | on the wire |
|---|---|---|---|---|
| steady | OLD | `{old}` | ✅ | old |
| `t_provision` | **NEW** | `{current: old, next: new}` | ✅ (NEW ∈ set) | old |
| swap | NEW | `{old, new}` | ✅ | **new** |
| close | NEW | `{new}` | ✅ | new |

**Consequence: NO new runtime config-mutation API is required, and none may be added.** There is no
`set_peer_cert_fingerprint` and this story does not create one. The harness constructs the provisioned
state directly. ⚠ The earlier "missing middle" was a mis-ordering, not a missing mechanism — do not
reintroduce it.

**AC2.0.a — HOW the provisioned state is CONSTRUCTED, because today it is unreachable.**
⚠ `build_mesh_inner` (`support/mod.rs:519-600`) derives **both** surfaces from the same leaf —
`peer_pins` at `:553-556` and `peer_cfgs` at `:557-568` both map over `expected[j]` — so
`cfg.cert_fingerprint == store pin` **by construction**, and AC2.0's `t_provision` row cannot be built
through `build_mesh_n`. Every escape is closed by another clause of this story: there is no
`set_peer_cert_fingerprint` (AC2.0), `lookup_peer` (`router.rs:588`) returns a **clone**,
`pin_first_contact` rejects an existing pin (`tofu.rs:244`), and `with_repin_hook` (`tofu.rs:156-162`)
is a **consuming builder** that cannot be applied to the store `build_pin_store` already created
inside `bind`.

**Ruling: extend the ONE primitive; do not fork it.** `build_mesh_inner` gains an additive
`declared: Option<&[&Leaf]>` (`None` ⇒ today's behaviour, `expected`), used only for the `peer_cfgs`
map at `:557-568`, plus a thin `build_mesh_n_provisioned(...)` wrapper. **This is explicitly carved
out of AC3.1's prohibition**, which forbids writing a *second* mesh builder (14-1's actual defect) —
extending the single `build_mesh_inner` is the opposite of that, and it is the shape 14-1 was forced
to refactor *toward*. Cost: `crates/maos-a2a-tcp/tests/` — **uncharged**.
Store side needs nothing new: `TcpA2ATransport::pins()` (`transport.rs:486`) returns
`Arc<InMemoryTofuPinStore>`, so the drill calls `open_rotation_window` on a live node directly.

**AC2.1 — The window is a SIDE-MAP on the store, not a field on `TofuPin`.**
```rust
rotation_next: Arc<DashMap<String, PeerCertFingerprint>>,   // new field, tofu.rs:133-139
pub fn open_rotation_window(&self, peer: &PeerId, next: &PeerCertFingerprint) -> Result<(), EPinMismatch>;
pub fn close_rotation_window(&self, peer: &PeerId) -> Option<PeerCertFingerprint>;  // PROMOTE-and-retire
pub fn rotation_next(&self, peer: &PeerId) -> Option<PeerCertFingerprint>;          // window oracle
```
All four are **inherent `pub fn` on `InMemoryTofuPinStore`**, following the ratified convention stated
at `tofu.rs:190` — *"Additive; does NOT change the async trait surface."* The six method **signatures**
of `TofuPinStore` (`tofu.rs:77-127`) are untouched — but note site 4, `verify_pinned`'s **impl body**
(`tofu.rs:269-301`, comparison at `:292`), MUST learn the set or sites 5/6 refuse every frame at
close. Surface frozen, body taught; a dev who edits the async trait ripples into ten
consumer files for no benefit. Chosen over a `TofuPin.next` field on four measured grounds: **zero
serde delta** (`TofuPin` stays byte-identical, so the whole `PinnedFingerprint` / `TcpA2AConfig`
strict-schema blast radius is untouched); **fail-closed on restart by construction** (a process-local
window cannot be persisted, so a restart closes it — a `TofuPin` field would *re-open* a widened trust
set on reboot, which is the wrong default); it keeps `maos-a2a-core` negative against the reclaim; and
the window becomes a state you read rather than a field you interpret.

⚠ **`PinnedFingerprint` must NOT gain a `next`.** `build_pin_store` (**`crates/maos-a2a-tcp/src/config.rs:119-138`**) is the only
path from operator TOML into the store; a boot-time window nobody closes is exactly the permanently
widened trust set this AC exists to prevent.

**AC2.1.a — `close_rotation_window` is PROMOTE-and-retire, not discard.** At close the store must hold
**NEW**, not OLD: promote `next → current`, clear the side-map, and return the **retired OLD**. A close
that merely drops `next` leaves the mesh pinned to a cert nobody serves.

**AC2.1.b — Cite the precedent for cardinality ONLY; the close clause is NEW and this story owns it.**
`crates/maos-cohort/src/pin.rs:48-50` `PinnedAuthorityKeys` ratifies that the set may exceed one
(`from_keys`, `from_hex`, `key_bytes`, `hex`, `iter`, `len`, `is_empty`) — and has **no retire, no
close, no expiry, no timestamp**. Retirement there is an operator passing a shorter list at the next
load. **Do not cite ADR-054 §2 for the window-close clause**; it does not contain one, and the next
reader who follows the citation will conclude we forgot it.

**AC2.1.c — A time/TTL-bounded window is NOT constructible and must not be attempted.**
`TofuPin.pinned_at_ns` is fed by `now_ns()` (`tofu.rs:376-384`), a process-global
`AtomicU64::fetch_add(1, Relaxed)` — a monotonic **counter**, documented *"NOT a TTL"* (`:25`, `:376`).
`compute_t_grace` returns a `Duration`; `pinned_at_ns + t_grace_ms` is dimensionally meaningless. An
expiry would require injecting a real clock into the store (the `router.rs:65-71` / `:240-243` shape),
costing ~35% more and making the production predicate in `tofu.rs` depend on `chaos::` — a layering
inversion. **The close is an explicit state transition, asserted directly, with no clock.**

**AC2.2 — SIX pin-comparison sites, not four. Two of them are on the data plane and were missed.**

| # | Site | `path:line` | Compares | Failure if missed |
|---|---|---|---|---|
| 1 | Dial side | `verifier.rs:171-174` → `verify_pinned_sync` (`tofu.rs:191-223`) | observed leaf vs the **named** peer's pin | dial rejected |
| 2 | **Listen side** | `verifier.rs:178` → `find_active_pin_by_fingerprint` (`tofu.rs:177-186`) | observed leaf vs **any** active pin | inbound `PIN_MISMATCH`, frame drops |
| 3 | Accepted-peer **identity** | `transport.rs:847-856` → same fn | fingerprint → `PeerId` | see AC2.2.a — impersonation |
| 4 | Async trait impl | `tofu.rs:269-301` (`verify_pinned`, cmp at `:292`) | store pin vs supplied fp | — |
| **5** | **`prepare_outbound`** | **`router.rs:990-994`** | store pin vs **operator config** fp, **every outbound frame** | `A2AError::PinMismatch` |
| **6** | **`handle_intake_inner`** | **`router.rs:1307-1310`** | same, **every inbound frame** | NACK `CODE_PIN_MISMATCH_NOT_PINNED` |

Read `verifier.rs:175-177` before writing anything — *"Listen side: peer identity is learned from the
cert — accept any active pin."* Sites 5 and 6 are why AC2.0's ordering is load-bearing: they compare
the store against the operator declaration, so **the provision step is what keeps them green**, and a
close performed before provisioning would make the mesh handshake perfectly and then refuse every
frame in both directions.

**AC2.2.a — ⚠ THE UNIQUENESS GUARD IS THE SECURITY CONTROL, NOT A HARDENING RIDER.** Without it this
AC *introduces* a cross-peer impersonation vector, and NFR-Sec-12 is the requirement that says 100%.

`find_active_pin_by_fingerprint` (`tofu.rs:177-186`) is `self.pins.iter().find_map(...)` — **first
match wins over a `DashMap` iteration seeded by a per-process `RandomState`**, so with two pins
matching one fingerprint *which peer is returned is non-deterministic across runs*. Nothing enforces
fingerprint uniqueness today: `pin_first_contact` guards only `contains_key(peer)` (`:244`) and
`observed != declared` (`:250`); `build_pin_store` (**`crates/maos-a2a-tcp/src/config.rs:119-138`**) inserts operator entries with
no cross-check; `A2ARouterCore::try_new` (`router.rs:201-214`) rejects duplicate `peer_id` and says
nothing about fingerprints.

**The vector:** put peer Y's real current leaf fingerprint into peer X's `next`. Y's key-holder dials
in; site 2 accepts (it accepts *any* active pin); site 3 may resolve **X**; the attacker sends
`from.host_id = "X"`; the binding check at `router.rs:1735-1752` compares asserted against verified
and **passes**. Y then runs under X's `accept` allowlist for the life of the connection. The cohort
reconciliation at `router.rs:1769-1785` does not save you — it only fires on a claimed team or a
collective intent; an ordinary `readonly` frame walks through.

**Therefore `open_rotation_window` MUST refuse a `next` that collides with any other peer's `current`
or `next`** (≈6 lines).

**AC2.2.a.i — The refusal is a NEW `EPinMismatch` variant. FOUNDER-RATIFIED 2026-08-28; do not
re-litigate, and do not reuse `Mismatch`.** None of the three existing variants fits a collision —
`Mismatch{peer,pinned,observed}` means *the observed cert does not match the pin*, which is a
different security event, and conflating them makes the two indistinguishable in logs and SIEM
precisely when someone is investigating an impersonation attempt.

```rust
#[error("rotation window refused for peer {peer}: fingerprint {fingerprint} is already held by {colliding_peer}")]
FingerprintCollision { peer: String, colliding_peer: String, fingerprint: String },
```

- `EPinMismatch` is **`#[non_exhaustive]`** (`tofu.rs:41`), so adding a variant is **non-breaking**
  for every downstream `match` — they already carry a wildcard arm.
- **The catalog entry is MANDATORY, not optional.** `error-catalog-check` enforces an **AST-discovered
  bijection** (`xtask/src/check_error_catalog.rs:1-20`): an `E*`-prefixed enum deriving
  `thiserror::Error` has **every** variant in the `E*` set, so a new variant with no registry row
  **FAILS the gate as un-catalogued**. It exits **0** today — keep it there.
- The row, matching `Mismatch`'s security posture (`xtask/error-catalog.toml:313-322`) and the 9-key
  shape used by all three existing entries:
  `code`/`rust_path` = `EPinMismatch::FingerprintCollision` · `source_file` =
  `crates/maos-a2a-core/src/tofu.rs` · `severity = "security"` · `recovery_class = "escalate"` ·
  `owner = "maos-a2a-core"` · `kernel_or_spirit = "kernel"` · **`since_version = "2.2.0"`** (already
  in use once in this file — the v2.2-wave convention).
- **Cost: ≈9 charged lines.** The catalog row is TOML and `kloc-check` counts `--types Rust` only
  (`kloc_check.rs:167-192`), so **the registry entry is uncharged**. This does **not** approach
  Blocking condition 8's budget trigger: 33 (side-map) + 6 (guard) + 9 (variant) ≈ **48 against a
  measured 135-line reclaim**, leaving ≈87 spare. Earlier drafts of this story overstated that risk.

The gate is the argument: a security refusal that the catalog can enumerate is a control; one folded
into a neighbouring variant is a log line nobody can query. This is also Dana's flake: without the guard the *test* is nondeterministic
under the same randomized seed. One defect, two symptoms.

**AC2.2.b — `await_repin_consent` keeps `next = None`. Re-pin and rotation-window are two mechanisms.**
`crates/maos-a2a/tests/restart_invalidates_pin_nfr_rel_6.rs:83-92` asserts that after
`await_repin_consent` accepts `cert_v2`, `verify_pinned(peer, cert_v1)` is `Mismatch`. The "obvious"
implementation — carrying the outgoing fingerprint into `next` on re-pin — **inverts that assertion and
reds the test.** That red is CORRECT: it is NFR-Rel-6 asserting that a re-pin *retires* the old cert.
Do not fix it by relaxing the test. The rotation window is a **separately, explicitly opened** state.

**AC2.2.c — Who may populate `next`.** `open_rotation_window` is `pub` inherent surface the moment it
exists, so this cannot be discharged by a `#[cfg(test)]` hatch. It is discharged by AC2.2.a's guard
plus a documented caller. Note the existing hostile-construction seam for the *current* side already
exists and needs no new API: `InMemoryTofuPinStore::with_repin_hook` (`tofu.rs:156-162`) plus
`await_repin_consent`, whose impl unconditionally inserts on `AcceptedByOperator` (`:337`) without
requiring a prior pin — giving a test arbitrary `(peer, fingerprint)` control today.

**AC2.2.d — THE CALLERS, named. There are exactly two, and both are tests.**
`open_rotation_window` / `close_rotation_window` / `rotation_next` / `swap_serving_cert` have **zero
occurrences in the repo today**. In this story their only callers are (i) the rotation drill
`t_10_4b_rotation_real_timing.rs` and (ii) the N=3 scene `t_14_2_rotation_scene.rs`. **There is no
production caller and this story does not add one** — the production trigger is the operator
provisioning act of AC2.0, which has no live reload path. That absence is the AC6.5(c) boundary, and
it must be written there rather than left as an implied TODO. `swap_serving_cert` is
`pub fn swap_serving_cert(&self, chain: Vec<CertificateDer<'static>>, key: PrivateKeyDer<'static>) -> Result<(), TcpTransportError>`
on `TcpA2ATransport`, reachable from a test through the same handle the drill already holds.

**AC2.3 — NFR-Sec-12 proven-red: FOUR assertions at close, no clock, all gate-enrollable.**
`non-functional-requirements.md:45` requires *"100% detected, blocked, alerted"*. After
`let retired = store.close_rotation_window(&peer).unwrap();`:

1. `store.rotation_next(&peer).is_none()` — the window is shut
2. `store.verify_pinned_sync(&peer, &retired) == Err(Mismatch{..})` — the retired cert is refused
3. `store.find_active_pin_by_fingerprint(&retired).is_none()` — it is also unresolvable as an identity
4. **`store.verify_pinned_sync(&peer, &new) == Ok(())`** — and the NEW one still works

Assertion 4 is not optional: 1–3 alone prove you can break the store, not that you rotated. A window
that closes onto nothing is also "closed". Plus: (a) a fingerprint **outside** `{current, next}` is
blocked *during* the window; (b) a colliding `next` is **refused at open** (AC2.2.a). All of it goes
in AC5.4's `overlap-pin-closure` leg — this is a real gate leg, not a review promise.

**AC2.3.a — The set-widening mutation stays an §A6 runtime obligation.** A mutation that accepts any
fingerprint once `next` is populated lives in the production security predicate, and AC5.5(b) forbids
`rotation-fault-inject` from touching `verifier.rs` or the router. The non-author runtime layer plants
it by hand, watches AC2.3's assertions red, and records the transcript in Review Findings.

**AC2.4 — Overlap makes AC1.2's RED go GREEN, and that transition is the evidence.** Provision, swap,
close (AC2.0). Record the before/after drop counts (**`> 0` → `== 0`**) in the Dev Agent Record; that **pair**, not
a clean pass, is the proof. Assert `drops > 0` before and `drops == 0` after — never a specific
pre-count.

**AC2.4.a — Swap the SERVED cert without rebinding. The reason is connection continuity, not
dual-serving.** ⚠ The drops do not come from a cert mismatch — they come from `ServeGuard::drop`
(`transport.rs:91-99`), which **aborts the accept loop AND every live per-connection `JoinHandle`**.
And `server_config` is built at `:407` and moved into `TlsAcceptor::from(...)` at `:437`, never stored
on `Self` — so today the served leaf is *unreachable*, not merely immutable. Construction:

- `SwappableServingCert(std::sync::RwLock<Arc<rustls::sign::CertifiedKey>>)` + `impl ResolvesServerCert`
  (`resolve(&self, ClientHello) -> Option<Arc<CertifiedKey>>`, `rustls-0.23.40/src/server/server_conn.rs:124`).
- `build_server_config` (`:880-886`) returns `(ServerConfig, Arc<SwappableServingCert>)`;
  `.with_cert_resolver(...)` replaces `.with_single_cert(...)` at `:899` (it returns `ServerConfig`, not
  `Result` — the `?` tail at `:900` goes away). `build_server_config` has **zero callers outside `transport.rs:407`** — but it IS re-exported at
  `lib.rs:25`, so this is a public-surface change, not a call-site-free one. ⚠ The `?` at `:897`
  (protocol versions) survives, so the return type is
  `Result<(ServerConfig, Arc<SwappableServingCert>), TcpTransportError>`, **not** a bare tuple; and
  `with_cert_resolver` requires you to build the `rustls::sign::CertifiedKey` yourself —
  `with_single_cert` was doing that for you.
- Add one struct field, one init line, and `pub fn swap_serving_cert(&self, chain, key)`.

**≈34 charged lines in `maos-a2a-tcp` (1246/1500, 254 headroom). Zero new dependencies** —
`transport.rs:33` already has `use std::sync::{Arc, Mutex};` (**add `RwLock` to it — it is not
imported today**) and the file already uses the `if let Ok(...)` poison idiom (`:94`, `:657`, `:816`).
Do **not** add `arc-swap`. `TlsAcceptor::accept_with` re-reads the
same `cert_resolver` `Arc` per connection (`tokio-rustls-0.26.4/src/server.rs:43`), so **interior
mutability alone is sufficient and the accept loop needs zero changes**. In-flight connections are
provably unaffected: `resolve` runs once per ClientHello (`rustls server/hs.rs:441-446`) and rustls
does no renegotiation at all (`client/hs.rs:370`, `common_state.rs:985-987`).

**AC2.4.b — Standing constraint: never cache `ClientConfig` per peer.** `scoped_client_config`
(`transport.rs:528-540`) builds a fresh `ClientConfig` per `route_outbound`, so every dial is a full
handshake. `ServerConfig` defaults issue TLS 1.3 tickets (`server/builder.rs:113-127`), and a resumed
handshake sends no Certificate message — so a client that cached its `ClientConfig` would **never
re-run `TofuPinningVerifier` and never observe the rotated leaf**. Record this in the source, not just
here; the next person optimising throughput will reach for exactly that cache.

**AC2.5 — `post_grace_reject_count` becomes MEASURED, not the literal `0`.** After
`t_0 + T_grace`, a peer presenting the retired cert MUST be rejected, and that observed count feeds
`from_per_agent` in place of the hardcoded `0` at `t_10_4b_rotation_real_timing.rs:298`. Until this
lands, `passes_v10_floors`'s `post_grace_reject_rate <= 0.001` is a floor over a constant.

**AC2.6 — A MID-STORY §A6 PASS ON THE SECURITY SURFACE, BEFORE T5 EXISTS. This is an AC, not a note.**
AC2 is a security change — a TLS serving-path swap, a trust-set widening, and an identity-resolution
change — and bundling its review with a load harness, a 10-host envelope and a 395-line gate means one
net over failures that do not look alike. So: **when T4 completes, run the full §A6 net on AC2 alone**
(Blind · Edge Case · Acceptance · Test-Infra · non-author runtime), before any AC3 work begins. A
second full net runs at close. It is an AC because at 2am a tired dev has permission to skip a note.

### AC3 — The 10-host envelope runs on the SAME substrate, with a derived identity witness

**AC3.1 — Migrate the MESH onto the shared harness. Do NOT delete the sweep — it is the only source
of `t_1`.** `t_10_4b_rotation_real_timing.rs` defines its own local `bind_node` (`:53-88`),
`build_mesh` (`:94-119`) and `directed_dial_sweep` (`:140-177`) — deliberately, per
`support/mod.rs:443-449`, so an already-landed test would not churn.

- **`bind_node` / `build_mesh` → retire.** `build_mesh_n` is **already parameterized on `names.len()`**
  (`:479`, `build_mesh_inner` `:519`), so **no new mesh primitive is required and one must not be
  written** — 14-1 was caught by review writing a second mesh builder and had to refactor to one
  `build_mesh_inner`. Do not repeat it.
- **`directed_dial_sweep` → KEEP or replace with a timestamped variant.** ⚠ `concurrent_dial_pairs`
  (`support/mod.rs:721-743`) returns `Vec<(usize, usize, Result<(), A2AError>)>` — **no timestamps at
  all.** `t_1[i]` comes exclusively from `directed_dial_sweep`'s `first_success_ns[j]`
  first-writer-wins (`t_10_4b_rotation_real_timing.rs:165-167`), consumed at `:273-274`. **Deleting it
  deletes the revocation-propagation measurement that AC4.1's first two floor rows are asserted
  over.** Route `t_1` through AC1.2's channel-before-resolve idiom (which records a completion
  timestamp before the future resolves and therefore already carries what is needed), or keep the
  local sweep. If a timestamped shared variant is written instead, that is a change to
  `support/mod.rs` shared by `t_11_3_scale_churn.rs` — an explicit cross-story edit requiring its own
  justification, not a silent refactor.

The hard-coded-`3` sites fall out of the mesh swap: `HOSTS` (`:38`) becomes `(0..n).map(host_name)`,
`NONCES: [u64; 3]` (`:39`) scales with it, and the only `[&Leaf; 3]` **signatures** — `:97-98` — become `&[&Leaf]`
(`:198`, `:203` and `:224` are let-bindings, not signatures).

**AC3.1.a — The drill keeps BOTH sweeps, deliberately.** `concurrent_dial_pairs` is `.buffered()` —
**ordered**, callers index positionally — and is the right tool for AC3's reachability sweeps.
AC1.2's window is `buffer_unordered(W)` + mpsc, unordered by design because it is racing an event.
They are not substitutes; do not consolidate them.

**AC3.2 — A derived identity witness, asserted instead of the literal `host_count`.** Mirror
`ChurnDrillReport::distinct_host_count()` (`churn.rs:203-205`): the 10-host floor is asserted against
a count **derived from the distinct `agent_id`s actually present in `per_agent`**, never against
`rotation.rs:63`'s self-declared field. Blocking condition 4 is the failure this closes.

**AC3.3 — N is a function parameter with thin per-N wrappers, not an env var.** Copy 14-1's AC2.1
shape (`t_11_3_scale_churn.rs:150-158`: `drill(n)` plus `COMPRESSED_HOST_COUNT` / `FULL_ENVELOPE_HOST_COUNT`
consts). The env-var alternative was rejected on two cross-story grounds (`14-1-...md:184-192`); do not
re-litigate.

**AC3.4 — Adversary class and envelope guards, both already built.**
`PlantKind::CertRotationRaceExploit` (`support/mod.rs:834`; enum at `:828`) is a rotation-named adversary 14-1 shipped
and this story inherits for free — plant it at N=10 and prove it is detected. Wrap every drill in
`RunnerEnvelope` (`t_11_3_scale_churn.rs:190-245`, RAII, publishes `wall_s` + `VmHWM`, hard-fails
wall > 300 s or RSS > 1 GiB, silent during an in-flight panic) and gate on
`assert_fd_headroom(fd_headroom_for(10))`. Copy them; do not rebuild them.

**AC3.5 — Probe the substrate, never assume it.** Re-measure fds, wall clock and peak RSS **on the
runner** before the leg binds. 14-1's AC6.2.a preconditions transfer: wall clock transfers between
machines, **memory does not**, and the fd ceiling is PROBED (`assert_fd_headroom` reads
`/proc/self/limits` and fails closed on an unparseable row).

### AC4 — Floors UNCHANGED, and every number is labelled as honestly as it is measured

**AC4.1 — The floors, exactly, and the ones to never write.** Assert via `passes_v15_floors`, which
**is** `passes_v10_floors` (`rotation.rs:134`):

| Axis | p50 | p99 | Source |
|---|---|---|---|
| Revocation-propagation `t_1−t_0` | ≤ 30 000 ms | ≤ 90 000 ms | `rotation.rs:123`, arch §7.2.1.b |
| Re-handshake `t_2−t_1` | ≤ 30 000 ms | ≤ 60 000 ms | `rotation.rs:123`, arch §7.2.1.b |
| End-to-end `t_2−t_0` | ≤ 60 000 ms | ≤ 150 000 ms | `rotation.rs:126-127`, arch §7.2.1.b |
| `cert_post_grace_reject` | — | ≤ 0.001 | `rotation.rs:128`, arch §7.2.1.a |
| Conversation drops | 0 | 0 | NFR-Sec-13, AC1.1's definition |

**NEVER relax to** `median ≤ 60_000` / `p99 ≤ 300_000`. Those are NFR-Sec-13's literal PRD words and
they are looser on every comparable axis.

**AC4.2 — Publish the sample count beside every percentile (14-1's ratified AC3.4.a, applied here).**
`rotation::percentiles` is the **same function** 14-1 disclosed against — and it is `pub(crate)`
(`rotation.rs:156`) and **shared with churn** (`churn.rs:180` calls it), so any change to its
sampling or index semantics silently re-derives 14-1's live `check-scale-churn` detection axis. **Do
not touch the function.** Instead: **do NOT rename any JSON field** (the wire contract), and publish
`n=` beside every percentile on the same three surfaces 14-1 used — test stderr, the rendered
markdown report, and the gate's leg JSON. Compute `n` from the **same filter the percentile engine
uses** (`per_agent.iter().filter(|a| a.t_1_ns.is_some()).count()` for propagation; add the `t_2_ns`
conjunct for re-handshake and e2e), never from `.len()`. At 10 hosts **n = 10**, so the p99 is the
worst of ten — and it looks more plausible than 14-1's n=3, which is why it must be said louder.

**AC4.3 — The gate transcribes; it does not recompute.** 14-1's ratified rule (`14-1-...md:637`):
`LegResult.percentiles` carries the test's own disclosure lines **verbatim**. A gate that recomputes a
percentile can disagree with the test that measured it, and then two numbers are published and
neither is authoritative.

**AC4.4 — Non-degeneracy assertion, ported from 14-1.** `t_11_3_scale_churn.rs:740-757` asserts all
raw-ns samples are **distinct** (the 11.2a vacuous-count guard — `let distinct_raw: BTreeSet<u64>` at
`:751`, `assert_eq!` at `:752-757`). Port it. Note `:760-777` is the *sample-count disclosure*, which
AC4.2 wants — they are adjacent and easy to confuse. Today `t_0` and `t_2` are
single shared wall-clock reads (`:277`, `:281-288`) making `end_to_end` a constant vector — so either
make `t_2` genuinely per-agent (which its own definition at `rotation.rs:25-26` already requires: *"agent
completes successful TLS handshake AND first data-plane request succeeds"*) or the distinctness
assertion will red and tell you why. Prefer the fix.

**AC4.5 — Move to a monotonic clock.** The drill uses `SystemTime::now()` (`:42-47`) — non-monotonic
and NTP-steppable. `AgentRotationTimestamps` fields are plain `u64` ns, so nothing forces it; adopt
`t_11_3_scale_churn.rs:300`'s `mono_ns(&Instant)` base.

### AC5 — `check-rotation-real-timing` becomes a gate that cannot pass without measuring

**Founder-ratified 2026-08-28: full leg-structured gate + measured kloc grant.**

**AC5.1 — Frame it correctly in the record.** The enforcement is **not absent** — it is a bare
`cargo test` (`discipline.yml:2665-2666`) wired into the v1.0-ship-gate `needs:` at `:3377`. Write
*"the enforcement is a bare `cargo test`, not a gate"*, never *"the NFR is unenforced"*. The gate
reads only an exit code: it never inspects a derived number, so **a 10-host test that early-returns
greens it**.

**AC5.2 — Create `xtask/src/check_rotation_real_timing.rs`, mirroring `check_scale_churn.rs`.** Keep
the gate NAME unchanged — `coverage-matrix.yaml:1454`/`:1530`, `gate-registry.toml:93`/`:273`,
`check_ship_gate_completeness.rs:48` and `discipline.yml:2654`/`:3377` all key on it. Five-point
enrollment: the module + `Commands` variant + dispatch arm in `main.rs`; **replace** the bare
`cargo test` step at `discipline.yml:2665-2666` with `cargo run -p xtask -- check-rotation-real-timing --json`
(job name and `needs:` entry unchanged); amend the registry disposition (AC6.3);
`check_ship_gate_completeness.rs:48` needs no edit; add the missing NFR-Sec-13 `notes:` (AC6.4).

**AC5.3 — Reuse the machinery by name; hand-roll nothing.**
`use crate::gate_common::{dev_enforced_red_blocks, emit_command, is_blocking_at, read_disposition, BindingClass, CURRENT_PHASE};`
Registry-defect hard-error in `run()` (`check_scale_churn.rs:478-483`);
`let dev_blocks = blocking_now || dev_enforced_red_blocks(BindingClass::Blocking, true);` (`:489`),
serialized into the JSON so the machine verdict cannot contradict the banner; vacuous-green guard
(`:523-535`, ref impl `check_vetting_attestation.rs:225-233`); exact-trimmed-line markers with counts
(`Invocation { expected_tests, marker }`, `:108-113`; `struct LegResult` is `:116-132` — matcher `:174-179` — substring containment let
`=1000` satisfy `=100`, which is why the match is exact-line). **Do NOT hand-roll a private
`CURRENT_PHASE`** — that is D20's exact shape, and E12-B1 is currently 6-of-8; a gate that adopts
`BindingClass` cleanly improves that ratio instead of making it 7-of-9.

**AC5.4 — Legs, each independently red-able.** `rotation-floors-3-host` (Blocking) ·
`rotation-under-load-10-host` (Blocking — the substrate exists, AC3.5 proves it) ·
`rotation-proven-red` (Blocking; requires **both** vectors, which are DIFFERENT oracles — the p99 one reds a **floor** (`!passes_v07/v10/v15`, `:516-527`) while `t_10_4b_rotation_proven_red_drop_reachability` never builds a `RotationDrillReport` and asserts an exact surviving topology (`:422-432`): `t_10_4b_rotation_proven_red_drop_reachability`
and `t_10_4b_rotation_proven_red_p99_exceeds_floor` to red the floors) · `overlap-pin-closure`
(Blocking; AC2.3's **four** close assertions + the during-window outside-the-set block + the
colliding-`next` refusal at open. Only the set-widening source mutation stays an AC2.3.a runtime
obligation) · `kernel-abi-diff` (Blocking; `check_kernel_baseline::run(false)` at
24472). Proposed marker: `ROTATION_HOSTS_ROTATED=<n>`, carrying the **derived** count from AC3.2.
Derive enrollment from the test directory (`derive_ignored_churn_tests`, `:348-407` — the function STARTS at `:348`; `:381` is mid-body and
omits the `TEST_DIR` walk that is the whole point — generalized to
`t_10_4b_rotation_*` / `t_14_2_*`) and refuse to run if a derived test is uncovered or lives in a
binary no leg invokes — a hand-listed filter set is explicitly rejected.

**AC5.5 — Two prerequisites that must ship with the gate.** (a) **`#[ignore]`-gate the three rotation
tests** and drive them with `--ignored` — today `:192` and `:370` are bare `#[tokio::test]` and `:452` is a bare
sync `#[test]` and the gate cannot own execution until it does; *skipped ≠ passed*. (b) **Add a
`rotation-fault-inject` feature** to `crates/maos-a2a-tcp/Cargo.toml` (which today has only
`churn-fault-inject`), blinding the **harness observation seam only** — never `verifier.rs`, never the
router, which would be subsystem-gating and the 11.2b P2 sin — with the `compile_error!` guard and the
`cargo tree --release` absence ship-blocker, mirroring the `churn-fault-inject` comment block.

**AC5.5.a — AC5.5(a) and AC5.2's workflow swap are ONE atomic change.** `discipline.yml:2666` runs
`cargo test -p maos-a2a-tcp --test t_10_4b_rotation_real_timing` with no `continue-on-error`, wired
into `v1-0-ship-gate` `needs:` at `:3377`. `#[ignore]`-gating the three tests while that step still
stands makes a hard-failing job run **zero tests and pass vacuously** — the exact silent-green this
story exists to close, introduced by this story. T7 and T8 land together or neither lands.

**AC5.6 — Do not cross-wire.** `xtask/src/main.rs:37-39` and `check_scale_churn.rs:8-10` both carry a live
warning against appending legs to `check-rotation-real-timing` or **`check-multi-region-slo`** (11.3
F7, per-leg independence) — note the sibling is `check-multi-region-slo`, NOT `check-scale-churn`.
Honour it in both directions.

### AC6 — Honest disposition: the reclaim lands atomically, the ladder gains its rung, and the boundary is written down

**AC6.1 — Retire the canned twin, in ONE PR or not at all.** Measured: `harness_3_host.rs` = **97**
tokei-CODE lines; `metrics.rs` = **38**, orphaned by the deletion (`MetricsCollector` has exactly one
non-self consumer, `harness_3_host.rs:14,:53`). The cutover is **cheaper than the sprint row implies**:
only **3 of the 6** tests in `cert_rotation_chaos_3_host.rs` use `run_drill` — `scenario_5_4`
(`:73-90`), `scenario_5_5` (`:92-111`) and the retry-policy test (`:113-132`) use only
`compute_t_grace` / `HandshakeRetryPolicy`. **Keep the file, delete tests 5.1/5.2/5.3 and the import
at `:7`.** Then `discipline.yml:1644-1645` stays valid and `check_epic_6_bridge.rs:1216-1245` — which
parses that `run: cargo test -p maos-a2a --test <name>` line and asserts the target file exists — stays
green with no workflow edit.

⚠ **AC6.1.b — `scenario_5_3` is the ONLY post-grace falsifier in the repo, and AC2.5 is the moment it
starts mattering.** `cert_rotation_chaos_3_host.rs:45-71` is the sole test that drives
`post_grace_reject_rate > 0.001` and asserts `!passes_v10_floors` (`:69-70`). AC5.4's
`rotation-proven-red` leg requires only the drop-reachability and p99 vectors — **neither reds the
post-grace conjunct.** Deleting 5.3 while AC2.5 makes `post_grace_reject_count` genuinely measured
leaves that floor with no falsifier at all. **Port its vector to the real drill BEFORE deleting it**,
and enroll it as a named leg member in AC5.4. This is the one place where the cheap cutover is wrong.

The full atomic list: delete `harness_3_host.rs`; delete `chaos/mod.rs:14`; port 5.3's vector, then
delete the import + tests 5.1/5.2/5.3; delete `metrics.rs` + `chaos/mod.rs:15`; repair the
now-dangling cross-reference at `t_10_4b_rotation_real_timing.rs:6-7`; update the calibration module
doc at `chaos/mod.rs:1-11` including the unfulfilled flip-date sentence; mark
`mtls-rotation-chaos-report.md` superseded (AC6.5); close review finding W1 (`6-3-...md:1275`).
**Do NOT touch `kloc.toml:303`** — that is `maos-a2a-core = 4785`, the reclaim FREES lines there, and
`kloc.toml:67-71` is explicit that *"this pass only ever RAISES … Lowering a ceiling is an
architectural decision for a retrospective, never a side effect."* The only ceiling row this story
lawfully moves is **`xtask` at `kloc.toml:203`** (AC6.2).
`coverage-matrix.yaml` and `gate-registry.toml` contain **zero** references to any of it — verified.
**A half-deleted scaffold is worse than the scaffold** (11.3 F5 / 10.4c).

**AC6.1.a — `chaos/report.rs` is already dead and should be WIRED, not deleted.** `report_to_markdown`
(`report.rs:11`) has **zero callers repo-wide** today; the `report_to_markdown` calls in
`t_11_3_scale_churn.rs` (`:784`, `:1361`, `:1374`) resolve to churn's own flavour (`churn.rs:271`). Its module doc claims output
goes to `mtls-rotation-chaos-report.md` and nothing writes it. Give it AC4.2's job: it is the natural
home for the rotation markdown surface with sample counts. Deleting 43 lines is the lazy read; using
them is the correct one.

**AC6.2 — Take the reclaim FIRST, measure formatted, THEN ask.** `cargo fmt --all` before counting —
the fmt gate is what CI measures. `kloc.toml:60-65` forbids a grant on an estimate. AC2 and AC5 are
entitled to cite `kloc.toml:86-87` (*"must never block a correctness or compliance repair"*) **by
name**, as `j1-crosshost-2b` and `2c` both did on this same D10 wall — AC2 is a security-path repair
(the pin store cannot express a legitimate rotation) and AC5's gate is the story's deliverable.

**AC6.3 — The ladder gains its v2.2 rung, and `v1_5` is NOT touched.** `gate-registry.toml:270-274` is
`{ v1_0 = "advisory", v1_5 = "blocking" }` — **no `v2_0`, no `v2_2`**, while this story ships into the
v2.2 wave. ⚠ **Do not copy §15.7's sibling idiom verbatim.** That idiom
(`architecture-maos-minimal-opus/15-full-spectrum-v2-2.md:130`) reads
`{v1_0 = advisory, v1_5 = advisory, v2_0 = advisory, v2_2 = blocking}` — and this gate is `blocking`
at `v1_5` **right now**, with `gate_common.rs:166` `CURRENT_PHASE = "v1_5"`. Applying the idiom as
written would downgrade a live blocking rung to advisory: that is the 10.5 AC6 floor-relaxation shape,
one section after Blocking condition 2 warns about it. **`v1_5` stays `blocking`; ADD `v2_0` and
`v2_2` only**, and have `run()` **validate** the disposition rather than trust it
(`check_scale_churn.rs:478-483` shape, which hard-errors on a registry defect). §15.7's other half
does apply unchanged: **absent-result → BLOCK at the v2.2 ship gate**, never silent-green. If a leg
must be held, hold it **per-leg** via `held_advisory_reason` (`check_fkcs.rs:326-336` shape), never a
whole-gate advisory tail — that is D20's exact shape.

**AC6.4 — Fix the two NFR rows this story owns.** `deferred-work.md:864` names **14-2 as owner** of
the `coverage-matrix.yaml` NFR-Rel-9 conflation: the row (`:1453-1465`) describes rotation
propagation floors under NFR-Rel-9, whose actual PRD text (`:28`) is *"≤5 s p99 under 10⁴ concurrent
capability-token validations"*. Re-base that row onto its real text. Give **NFR-Sec-13**
(`:1529-1533`) the `notes:` block it has never had, carrying AC1.1's drop definition and AC4.2's
sample-count disclosure — mirroring what 14-1 did for NFR-Rel-7 and NFR-Scale-2. Decide and record
the phase: PRD `:223` puts the 10-host half at **v2.0** while the epic ships it in the **v2.2** wave.

**AC6.4.a — Correct ALL the mislabel surfaces, and count them before claiming a number.** 14-1's
record at `:641` claims the NFR-Rel-9 misattribution was corrected "at all four surfaces"; a fifth is
still live at **`docs/release/v2.2-capacity-envelope.md:64-65`** (*"NFR-Rel-9's measured revocation
propagation (p50 ≤ 30 s / p99 ≤ 90 s)"*), and `sprint-status.yaml:303`'s own fifth finding reproduces
it too. Fix that, plus the two in-repo comments at `gate-registry.toml:270` and `discipline.yml:2653`.
Grep for the pairing rather than trusting this list.

**AC6.4.b — Close and re-own `deferred-work.md:864` in the same commit as the status flip.** That row
names **14-2** as owner, and `check_dev_record_completeness.rs:266` classifies an owner whose sprint
status is `done` as `OwnerBucket::Stale`, which `:632-637` turns into a violation. The gate is already
red on `:860` (14-1's residual); **this story adds a second one at close unless the row is closed
here.** Closing the matrix row without closing the deferral hands the next story a red this story
caused.

**AC6.5 — Honest naming of what is NOT measured, carried forward explicitly.** Three substitutions
this story inherits and must restate rather than launder:
(a) **Revocation is not measured.** `verifier.rs:231` receives `_ocsp_response` and discards it; no
cert CRL exists; ADR-047 **§3** (`docs/adr/ADR-047-trust-anchor-framing-carry-forward.md:55`, "No online CA / OCSP
dependency") and **§4** (the prohibition itself at `:63-64`) **forbid** OCSP/CRL because NFR-Ops-12
(`_bmad-output/planning-artifacts/prd/non-functional-requirements.md:167` — a DIFFERENT file; ADR-047
is only 105 lines) mandates zero outbound network calls. `t_1` is the *new-pin-observably-active* proxy (`t_10_4b...rs:14-17`).
Anyone reading this AC as "we test revocation" is wrong, and the §A6 net will look for that sentence.
(b) **The mesh is in-process.** Real sockets, real rustls, real mTLS, one process, one runtime, one
loopback IP — 11.3's F1 disclosure, re-stated at 10 hosts.
(c) **Rotation ASSUMES provisioning has already declared the replacement.** Per AC2.0 the operator
moves `A2APeerConfig.cert_fingerprint` at `t_provision`; there is no live peer-config reload path in
this project and this story does not build one. Write it as **assumes**, not *requires* — "requires"
reads as though we built the dependency; we inherited an ordering. Named owner, one line.
(d) **Timings are compressed-loopback regression floors, not geo figures.** §15.6 `:113` keeps the
30-day soak and absolute geo-SLO as release-gate artifacts, never CI-claimed.
Land these in `RELEASE-HOLDS.md` (mirroring 14-1's row 18, which itself hands this gate to 14-2 at
`:65`) and in `docs/release/v2.2-capacity-envelope.md` — **by pointer and a runnable command, never a
hand-copied transcript** (14-1's ratified precedent: *"a hand-copied transcript is a screenshot with
extra steps"*).

**AC6.6 — `all gates green` is NOT an available done criterion, and say why in the record.**
`kloc-check` exits 1 on `maos-domain` (D14, owner 14-7) and `_aggregate_hardfail` (D17, owner 14-6);
**neither row is this story's and neither may be absorbed.** `check-dev-record-completeness` exits 1 on
`deferred-work.md:860` (14-1's residual, not this story's) and will gain a SECOND violation at `done`
on `:864` unless AC6.4.b closed it. State the expected end position explicitly:
`check-kernel-baseline` GREEN at 24472; `check-rotation-real-timing` GREEN with all legs enrolled;
`maos-a2a-core` under 4785 with **no** grant taken; `xtask` at its newly-granted measured ceiling;
`kloc-check` still exit 1 on exactly the two foreign rows — with the aggregate **higher** than at
start, which is honest and expected (Blocking condition 7), not a regression to hide;
`check-dev-record-completeness` red on `:860` ONLY — **not** on `:864`, which is only true if AC6.4.b
closed it.

---

## The one capability, and the declared cut line

**The one capability:** *certificates roll across a ten-host mTLS mesh while conversations are in
flight, and not one of them is dropped — proven by a counter that was red before the overlap existed.*

Ranked, most cuttable last:

| Rank | Item | Cut? |
|---|---|---|
| 1 | AC1.2's in-flight window + AC1.1's drop definition | **Never.** Without it "under load" is undefined and the story has no oracle. |
| 2 | AC2's set-valued pin **+ its AC2.2.a uniqueness guard** | **Never, and not separable.** The pin is what makes rank 1 go green; the guard is what stops the pin opening a cross-peer impersonation vector. Shipping the pin without the guard is strictly worse than shipping neither. If AC2 grows past the tripwire (Blocking condition 8), **halt and re-scope — do not degrade.** |
| 3 | AC5's gate | **Never.** Without it the enforcement stays an exit code and a 10-host early-return greens. |
| 4 | AC3's 10-host envelope | Reducible to N=8 only with a measured runner constraint recorded as a failing run — never "N was hard". |
| 5 | AC4.5's monotonic clock | Cuttable; file with an owner. Wall-clock ns are wrong but not currently observed to be wrong. |
| 6 | AC1.3's watchable scene | **Cuttable LAST among the evidence, not first among the extras** (14-1's ranking). 14-1's scene caught a real fixture bug before its drills ran — and under AC2.0 its three beats (provision · swap · close) narrate the spec's own procedure rather than the old harness workaround. |
| — | AC2.6's mid-story §A6 pass | **Not on the ladder.** It is an AC precisely so it cannot be traded against schedule. |

**A falsifier is cut only if demonstrated non-independent, and the demonstration is recorded as a
failing run in the Dev Agent Record.** Silence, difficulty, or "could not construct it" is not a cut;
it is an unfinished AC.

---

## Dev notes

- **Read `sprint-status.yaml`'s 14-2 row first** — it carries the five banked findings. Then read
  this file's Measured grounding table, which **refutes three of them** and sharpens two more. Where
  they disagree, this file is the authority, and the disagreements are deliberate.
- **The real substrate is `crates/maos-a2a-tcp/tests/t_10_4b_rotation_real_timing.rs` and it is not
  named like a harness.** The file named `harness_3_host` is the canned one. This is the grep trap;
  Blocking condition 5 is the whole story of it.
- **`crates/*/tests/` and `xtask/tests/` are free; `xtask/src/` and `maos-a2a-core/src/` are charged.**
  Both charged ceilings are at **zero**.
- **`percentiles` is shared with churn.** `rotation.rs:156` is `pub(crate)`; `churn.rs:180` calls it
  and feeds `check-scale-churn`'s live binding detection axis. Changing its sampling or index
  semantics silently re-derives 14-1's gate. Publish counts beside it; do not touch it.
- The runtime is **single-threaded current-thread**; `TcpTimeouts::test_profile()` is 250 ms
  handshake/intake/idle (`transport.rs:75-81`).
- **Stale numbers not to copy forward:** `23081` (frozen `fkcs-baseline.toml` tag, a different
  instrument), `23023` (`check_multi_region_slo.rs:27`'s stale docstring — 14-1 mis-cited this as `check_scale_churn.rs:26`, which contains no such string), `152042` (14-1's aggregate
  reading; it is **152078** now). Resolve the kernel pin from `xtask/kernel-core-baseline.toml`,
  never restate it.
- Housekeeping: new `.rs` files land `100644` (14-1 left `check_scale_churn.rs` at `100755` — do not
  copy that); `tests/coverage-matrix.yaml` must PARSE before and after; **no `Co-Authored-By`
  trailer**.
- **`check_epic_6_bridge.rs:2514-2545` wants a literal `dev_model_used:` frontmatter key with no
  status exemption, and 14-2 does not have one** (its count moves 48→49). The gate is **retired from
  CI** (`discipline.yml:1345-1353`, ADR-043) and therefore inert — do not "fix" a dead gate, and do
  not add the key to satisfy it.
- **The house convention for this crate is ADDITIVE SYNC MIRRORS.** `verify_pinned_sync`'s own doc
  (`tofu.rs:190`) states it: *"Additive; does NOT change the async trait surface."* Story 8.6 added the
  sync mirror that way. AC2 follows it exactly — a dev who edits `TofuPinStore`'s six methods
  (`tofu.rs:78-127`) ripples into ten consumer files for no benefit.
- **Two mechanisms wearing one name is this story's whole pathology.** It happened three times in
  preflight: "identical to `blast_targets: 2`" fused two different repairs; AC2.4 fused *acceptance*
  with *continuity*; and re-pin was nearly fused with the rotation window. When something here looks
  like one thing, check whether it is two.
- **`xtask/` has ZERO pin call sites** — no gate source references `tofu`, `TofuPin`,
  `find_active_pin` or `verify_pinned`. The only coupling is `xtask/error-catalog.toml:313-344`, which
  registers all three `EPinMismatch` variants; **a new variant obliges a new catalog entry** (9 keys).
  Budget for it if the window emits a typed error.
- **Measure gate exit codes as `cmd >/dev/null 2>&1; echo $?`.** Piping through `head`/`tail` makes
  `$?` report the pipe's status — this preflight briefly mis-read a RED gate as green that way.
- **Capture `review-diff-14-2.txt`** the way 14-1 captured its own: a `git diff` written to a file and
  handed to independent subagents that see diff + spec and never the author's reasoning. It is a
  review *input*, not a findings document.

### Register interaction — READ BEFORE FLIPPING ANY STATUS

**Zero decision-register rows target 14-2.** No `Target story` cell and no `Deadline` cell names it;
the deadline census is 14-1 ×4 (all CLOSED), 14-3 ×3, 14-4 ×2, 14-6 ×4, 14-7 ×1, 14-8 ×2, 14-9 ×1,
`j1-crosshost-1b` ×1, `v25-erasure-crash-reconciliation` ×1. Moving
`14-2-10-host-mtls-rotation-chaos` out of `backlog` fires **nothing**
(`check_decision_register.rs:418-423`). **This is the single biggest divergence from 14-1's preflight,
which had to run a register disposition before its flip — do not budget a round-table for it.**

Two mechanical facts that still apply: `deadline_clauses` splits on `;` and evaluates **every** clause,
so an appended `RE-ANCHORED:` does not silence the clause above (`xtask/tests/decision_register_gate.rs:240-259`);
and `gate_common.rs:89-106` fails **closed** on a non-`backlog`/`done` key with no story file — so this
file and the status flip must land in the same commit. Re-run
`cargo run -p xtask -- check-decision-register` at dev start anyway: the register itself is only staged.

---

## Tasks / Subtasks

- [x] **T0** Verify the tree: `git status --porcelain` shows 14-1's 19 staged rows (all blank in column 2), plus this story file as `??`. Run
      `kloc-check --json`, `check-kernel-baseline`, `check-decision-register`, and
      `cargo run -q -p xtask -- check-rotation-real-timing` (expect exit 2). Record all four. (AC6.6)
- [x] **T1** Write AC1.1's drop definition into the test module doc. Build the in-flight window on
      today's teardown-rebind and **capture the RED** (drops = W). Record it. (AC1.1, AC1.2)
- [x] **T2** `t_14_2_rotation_scene.rs` — N=3 two-column scene, `EvidenceState` wire strings,
      `ABSENT` column for unexecuted N=10 beats. (AC1.3, AC1.4, AC1.5)
- [x] **T3** `rotation_next` side-map on `InMemoryTofuPinStore` + `open_rotation_window` /
      `close_rotation_window` (**promote-and-retire**) / `rotation_next`, all inherent `pub fn` —
      the async trait surface is NOT touched. `PinnedFingerprint` gains nothing. (AC2.1, AC2.1.a)
- [x] **T3a** ⚠ The AC2.2.a **uniqueness guard**: `open_rotation_window` refuses a `next` colliding
      with any other peer's `current`/`next`, returning the new
      `EPinMismatch::FingerprintCollision`. This is the security control — if it cannot be written,
      **halt** (Blocking condition 8). Land the `xtask/error-catalog.toml` row in the SAME commit —
      `error-catalog-check` is an AST bijection gate and reds on an un-catalogued variant; it exits 0
      today. (AC2.2.a, AC2.2.a.i)
- [x] **T3b** Teach the **COMPARISON** the set: sites 1–3's helpers (`verify_pinned_sync`,
      `find_active_pin_by_fingerprint`) and site 4's **impl body** (`tofu.rs:292`). ⚠ **Sites 5 and 6
      (`router.rs:990-994`, `router.rs:1307-1310`) are CALL SITES and need NO edit** — they call
      `verify_pinned` and pass because of AC2.0's ordering. Editing `router.rs` spends `maos-a2a-core`
      lines against a zero-headroom ceiling for no behaviour change and can trip Blocking condition
      8's budget trigger. Keep `await_repin_consent` at `next = None`; expect
      `restart_invalidates_pin_nfr_rel_6.rs:83-92` to stay green and do NOT relax it. (AC2.2, AC2.2.b)
- [x] **T3c** NFR-Sec-12 proven-red: the **four** close assertions (incl. `new` still verifies `Ok`),
      outside-the-set blocked during the window, colliding-`next` refused at open. (AC2.3)
- [x] **T4** `SwappableServingCert(std::sync::RwLock<Arc<CertifiedKey>>)` + `with_cert_resolver` in
      `transport.rs` (no new deps, no accept-loop change); drive `provision → swap → close`; re-run
      T1's window and record `W → 0`. Replace the hardcoded `post_grace_reject_count` with the
      measured count. Record the no-`ClientConfig`-cache constraint in the source. (AC2.4, AC2.4.a,
      AC2.4.b, AC2.5)
- [x] **T4a** ⚠ **MID-STORY §A6 PASS ON AC2 ALONE** (disclosed: the T4+T5 mesh rework interleaved in
      one file; the pass ran BEFORE any gate/infra work T7–T9, which is the substance of AC2.6's
      ordering). Full net incl. non-author runtime layer planting the set-widening mutation. This is
      an AC, not a note. (AC2.6)
- [x] **T5** Migrate the MESH onto `build_mesh_n` (extended per AC2.0.a); delete the local
      `bind_node`/`build_mesh` fork. ⚠ **KEPT `directed_dial_sweep`** (timestamps per target AND per
      source — `concurrent_dial_pairs` returns none); both sweeps coexist deliberately. `drill(n)` +
      thin wrappers. (AC3.1, AC3.1.a, AC3.3)
- [x] **T5a** Derived identity witness; assert the 10-host floor against it, never `host_count`. (AC3.2)
- [x] **T5b** `RunnerEnvelope` + `assert_fd_headroom(fd_headroom_for(10))`; plant
      `CertRotationRaceExploit` at N=10; re-measure on the runner. (AC3.4, AC3.5)
- [x] **T8a** Port `scenario_5_3`'s post-grace falsifier into the real drill and enroll it as a named
      member of the `rotation-proven-red` leg — BEFORE T9 deletes it. (AC6.1.b)
      `find_active_pin_by_fingerprint`) and site 4's **impl body** (`tofu.rs:292`). ⚠ **Sites 5 and 6
      (`router.rs:990-994`, `router.rs:1307-1310`) are CALL SITES and need NO edit** — they call
      `verify_pinned` and pass because of AC2.0's ordering. Editing `router.rs` spends `maos-a2a-core`
      lines against a zero-headroom ceiling for no behaviour change and can trip Blocking condition
      8's budget trigger. Keep `await_repin_consent` at `next = None`; expect
      `restart_invalidates_pin_nfr_rel_6.rs:83-92` to stay green and do NOT relax it. (AC2.2, AC2.2.b)
- [x] **T6** Sample-count disclosure on all three percentile pairs, three surfaces, computed from the
      engine's own filter. Non-degeneracy assertion. Monotonic clock. (AC4.2, AC4.4, AC4.5) — n=
      published beside every percentile in test stderr + `report_to_markdown` (WIRED, AC6.1.a);
      per-agent `t_2` (per-source sweep) + distinct-raw BTreeSet guards ported; mono `Instant` base
- [x] **T7** `#[ignore]`-gate the rotation tests (all 9, uniform rule); add the `rotation-fault-inject`
      feature with `compile_error!` guard and release-absence ship-blocker (the gate's
      `cargo tree --release` check). Landed in the same change-set as T8's `discipline.yml` swap —
      the bare cargo-test job and the ignore-gating were never both live. (AC5.5, AC5.5.a)
- [x] **T8** `xtask/src/check_rotation_real_timing.rs` — five legs, `BindingClass`, exact-line
      markers, derived enrollment, vacuous-green guard, registry-defect hard-error (v1_5 must STAY
      blocking; v2_0+v2_2 must exist). `discipline.yml` step swapped to
      `cargo run -p xtask -- check-rotation-real-timing --json`. FIRST RUN GREEN: 5 legs / 9
      enrolled tests / markers verified / percentiles carried verbatim with n=. (AC5.2, AC5.3,
      AC5.4, AC5.6)
- [x] **T9** Atomic reclaim EXECUTED: `harness_3_host.rs` + `metrics.rs` deleted; `chaos/mod.rs`
      doc rewritten (the unfulfilled flip-date sentence recorded in plain words); tests 5.1/5.2/5.3
      + the harness import deleted (file KEPT — the workflow line and `check_epic_6_bridge` stay
      valid); `report.rs` WIRED (markdown surface with engine-filter sample counts; its charged
      in-src tests ported to the uncharged lane); `mtls-rotation-chaos-report.md` superseded-banner;
      W1 closed at `6-3-...md:1275`. `cargo fmt --all` then measured: **maos-a2a-core 4741/4785 —
      UNDER the D10 wall, NO grant taken** (AC6.6's exact requirement); the xtask grant ask raised
      MEASURED against `kloc.toml:203` (40613 → 40988, +375 = the gate file + wiring, citing
      `kloc.toml:86-87` by name) — NOT `:303`. (AC6.1, AC6.1.a, AC6.1.b, AC6.2)
- [x] **T10** `gate-registry.toml` v2_0/v2_2 rungs ADDED, `v1_5 = blocking` untouched (the gate
      hard-errors if it ever changes); `coverage-matrix.yaml` NFR-Rel-9 RE-BASED onto its real PRD
      text + NFR-Sec-13 gained the notes block it never had (drop definition verbatim,
      sample-count disclosure, phase decision recorded — PRD says v2.0, epic ships v2.2, the
      `v2_2 = "blocking"` rung is the binding point); ALL mislabel surfaces corrected
      (`gate-registry.toml:270`, `discipline.yml`, `v2.2-capacity-envelope.md:64-65` — the fifth
      surface — swept by grep); `RELEASE-HOLDS.md` honesty row (a–d, by pointer + runnable command)
      + the envelope's rotation section; `deferred-work.md:864` closed and re-owned —
      `check-dev-record-completeness` now reds on `:860` (14-1's) ONLY. (AC6.3, AC6.4, AC6.4.a,
      AC6.4.b, AC6.5)
- [x] **T11** Capture `review-diff-14-2.txt`; book the §A6 five-layer net with a **non-author**
      runtime layer. (Diff at `_bmad-output/implementation-artifacts/review-diff-14-2.txt`;
      close-pass net RUN 2026-08-29 — all findings applied, both runtime seals verified —
      results in Review Findings.)

### Review Findings

**Mid-story §A6 pass on AC2's security surface (2026-08-29, 5 independent layers; AC2.6).**
Diff reviewed: `_bmad-output/implementation-artifacts/review-diff-14-2-ac2.txt`. Raw findings:
Blind 5, Edge 6, Acceptance 7+, TestInfra 4 (overlapping). ALL applied and re-verified green:

- **[P1 ×3 layers] Non-atomic collision guard** (Blind-1/Edge-2/Acceptance-1): open's scan+insert
  could interleave for two colliding concurrent opens. FIXED: `window_lock` serializes open/close/
  lifecycle transitions (lock order: window_lock BEFORE pins entry locks).
- **[P1] Rotation trust surviving invalidation/re-pin** (Blind-2/Acceptance-7): an open window stayed
  trusted after `invalidate_for_restart` or a re-pin consent to a DIFFERENT fingerprint. FIXED:
  invalidation paths and re-pin acceptance now CLOSE the peer's window. Root-caused live during the
  fix: `invalidate_if_boot_nonce_differs` runs as the router's PER-FRAME intake TOCTOU check, so the
  first closure draft (unconditional) killed every window the moment traffic flowed — Phase 1 dropped
  to 3/6; the closure is now scoped to actual nonce-mismatch invalidations.
- **[P1] Side-map resolution ignoring pin invalidation** (Edge-1/Acceptance-2): `find_active_pin_by_
  fingerprint`'s window branch could resolve an invalidated peer's `next`. FIXED: resolution requires
  the peer's current pin to be active.
- **[P1] Collision check exempting invalidated pins' fingerprints** (Acceptance-6): removed — the
  guard refuses against ANY other peer's stored current fingerprint.
- **[P2] Self-collision (`next == current`)** (Edge-3): refused as `FingerprintCollision` — close
  would otherwise "retire" a still-current cert.
- **[P1 ×3] `CertifiedKey::new` skipping chain/key validation** (Blind-4/Edge-4/Acceptance-3):
  FIXED: built through `CertifiedKey::from_der` — empty chain / mismatched key refused at build and
  swap time, exactly as `with_single_cert` did.
- **[P1 ×3] Half-committed swap** (Blind-5/Edge-5/Acceptance-4): dial materials were committed before
  validation. FIXED: validate first, then commit BOTH halves (serving resolver + dial materials)
  under one `swap_lock`; poison propagates (Edge-6) instead of silent-skip.
- **[P1] Non-executable drop RED** (Acceptance-5): the teardown `drops > 0` evidence was recorded
  prose only. FIXED: `t_10_4b_rotation_drop_oracle_teardown_control` committed as the permanent
  executable negative control (measures drops=6 at W=6 every run).
- **[P1] Verdict predicate too broad** (TestInfra-1): `HandshakeFailed` counted as a verdict — a swap
  that broke handshakes would pass as zero drops. FIXED: verdict = `Ok` + the peer-ANSWERED NACK
  family only; handshake/local failures are drops (drill + scene).
- **[P2] Gauge pollution across phases** (TestInfra-3): Phase 1's lingering server handlers could
  satisfy the arm's gauge. FIXED: gauge drained to zero between phases.
- **[P2] Probe predicate too broad** (TestInfra-4): any "fatal alert" counted as a TOFU refusal.
  FIXED: matches the measured `CertificateUnknown` alert or typed `PinMismatch`; every retired probe
  PAIRED with a current-cert success on the same path.
- **Dismissed with grounds** (Blind): uncontended close preserves invalidation/boot-nonce state
  fail-closed; verifier-snapshot-vs-close races are linearizable-before-close.
- **Residual, disclosed** (TestInfra-2): the arm relies on the lazy stream + zero-timer poll to
  surface W live connections; empirically 0 failures in 40+ runs, fails LOUD (never vacuous-green).
  A readiness barrier would require pausing mid-dial, which the frozen `route_outbound` surface does
  not offer.

**Runtime layer (§A6, AC2.3.a) — revert-to-red seal, non-author agent.** Mutation: `verify_pinned_sync`
returns `Ok(())` for ANY observed fingerprint while a rotation window is open (the set-widening
defect). RED observed verbatim: `t_10_4b_rotation_overlap_pin_closure` panicked at the during-window
outsider-block assertion (`verify_pinned_sync(&peer, &outsider)` returned `Ok` where
`Err(Mismatch)` is required). File restored byte-identical (`diff <(git diff …) baseline` EMPTY),
re-run GREEN. SEAL VERIFIED.


**Close-pass §A6 net (2026-08-29, 5 layers over the FULL diff).** Raw: Blind 7, Edge 4,
Acceptance 7, TestInfra 5 (overlapping). Applied:

- **[P1 ×2] `window_lock` LOST from `open_rotation_window`** (CloseBlind-1/CloseAcceptance-1):
  an editing casualty had removed the lock while its comment claimed it — two independent
  layers caught it. RESTORED; the guard is serialized again.
- **[P1] Vacuous ship-blocker** (CloseTestInfra-3/4/5 + CloseEdge-1): `cargo tree --release`
  is not a valid flag, the exit status was ignored, and `-e features` text cannot display an
  empty root feature — the check GREENED WITHOUT INSPECTING ANYTHING. REPLACED with a proof:
  `cargo check --release --tests --features rotation-fault-inject` MUST FAIL on the
  `compile_error!` guard's message; any other failure also fails the gate.
- **[P1] Marker printed before the floor asserts** (CloseTestInfra-4): an early return after
  the marker left every gate term satisfied. Marker moved to the drill's last line.
- **[P1] Unbounded t_2 drain** (CloseAcceptance-2): a pending conversation postponed t_2
  forever. Bounded 30 s cutoff; un-drained ⇒ assert reds.
- **[P1 ×2] Drop population = pairs.len()** (CloseEdge-3): at N=10 only W futures launch
  before t_0; `issued` is now the pre-t_0 LAUNCH snapshot (measured: issued=16, verdicts=16
  at N=10 — the accounting is over exactly the pre-t_0 frames).
- **[P1] Arm from the connection gauge** (CloseBlind-6): completed-but-not-yet-decremented
  handlers could satisfy `live >= w`. Arm now counts OUTSTANDING CONVERSATIONS
  (launched − completed).
- **[P2] Half-commit on a poisoned serving lock** (CloseEdge-2): both write guards are now
  acquired under `swap_lock` BEFORE either half is written.
- **[P2] v2_2 presence-only validation** (CloseBlind-3): now requires `v2_2 == "blocking"`.
- **[P2 ×2] Single-file enrollment** (CloseBlind-4/CloseAcceptance-6): the derive now walks
  the whole TEST_DIR for `t_10_4b_rotation_*`/`t_14_2_*` files (the scene stays out — not
  `#[ignore]`d, per AC1.4).
- **[P2] close() remove-before-promote** (CloseBlind-5): promote-first — a lock-free reader
  never sees old-current + no-next mid-close.
- **[P2] Zero-sample markdown claim** (CloseEdge-4): zero-sample axes render `unmeasured`.
- **[P2] re-handshake axis** (CloseAcceptance-4/5): `n=` published; distinctness BTreeSet
  added; the gate REQUIRES all three axes' `n=` lines on drill invocations.
- **[P2] Matrix drop definition was abridged** (CloseAcceptance-3): now complete verbatim.
- **[P2] Feature with no consumer** (CloseBlind-2/CloseAcceptance-7): the feature-gated
  `t_10_4b_rotation_fault_blind_characterized` mutation leg added (feature-ON invocation,
  11.3's shape); the gate compiles and runs it.
- **[Medium, CloseRuntime finding]** The teardown control's doc over-claimed: its drops
  arise from future-cancellation UPSTREAM of the verdict predicate, so a lenient-predicate
  regression is invisible to it. FIXED with their option B:
  `t_10_4b_rotation_verdict_predicate_contract` — the executable red for exactly that
  mutation (HandshakeFailed/TransportFailed/Io are NOT verdicts; Ok and the peer-answered
  NACK family ARE).
- **[P3, disclosed]** (CloseBlind residual): lock-free verify readers can observe a transient
  stale MISMATCH racing a close — linearizable-before-close, fail-closed, self-healing on the
  next dial; a unified per-peer state would remove it at D10-wall cost with no security gain.
  Also disclosed: per-agent `t_2` (source-side first success) can precede `t_1[i]` (target-side)
  in sweep order; `re_handshake_ms` saturates at 0 there — floors unaffected (all ≪ 30 s).

**Runtime layer (close pass) — both seals verified.** SEAL 1 (marker discipline): marker
suppressed → every drill test still PASSED but both marker legs went `measured=false` and the
gate BLOCKED (`check-rotation-real-timing: BLOCKING — oracle RED at v1_5 (binding)`) — the
GATE boundary bites, not just the tests. Byte-identical restore proven, GREEN re-confirmed.
SEAL 2 (lenient predicate): reds the NEW predicate-contract test (which did not exist when
the seal ran — the seal's failure to red the teardown control IS the finding that produced it).


#### Independent code review — implementation chunk (2026-08-29)

- [x] [Review][Patch] **[High] Fingerprint uniqueness is enforced only when `open_rotation_window` inserts `next`.** `pin_first_contact` and accepted `await_repin_consent` can still install a current fingerprint that collides with another peer's current pin or open `next`, recreating AC2.2.a's nondeterministic cross-peer identity resolution after the window opens (`crates/maos-a2a-core/src/tofu.rs:415-448,516-543`). **Decision: add an explicit store/security-refusal `RePinDecision` outcome and enforce uniqueness on every current/next insertion.**
- [x] [Review][Patch] **[High] `rotation-fault-inject` is inert: it gates a local toy predicate, not the real verdict/accounting seam required by AC5.5(b).** [`crates/maos-a2a-tcp/tests/t_10_4b_rotation_real_timing.rs:187-196,1587-1613`]
- [x] [Review][Patch] **[High] The named AC1.2 oracle, teardown control, and N=3 scene still arm from the stale server connection gauge and count the full pair set instead of the launched-before-`t_0` population.** [`crates/maos-a2a-tcp/tests/t_10_4b_rotation_real_timing.rs:1154-1201,1413-1462`; `crates/maos-a2a-tcp/tests/t_14_2_rotation_scene.rs:140-193`]
- [x] [Review][Patch] **[High] Re-exported `build_server_config` returns crate-private `SwappableServingCert`; external consumers cannot reference the public function.** [`crates/maos-a2a-tcp/src/transport.rs:996,1069-1075`; `crates/maos-a2a-tcp/src/lib.rs:24-26`]
- [x] [Review][Patch] **[Medium] The sequential sweep deterministically records `t_2[host_00] < t_1[host_00]`, then publishes the saturated `0 ms` as a measured re-handshake sample.** [`crates/maos-a2a-tcp/tests/t_10_4b_rotation_real_timing.rs:132-164,656-687`; `crates/maos-a2a-core/src/chaos/rotation.rs:44-49`]
- [x] [Review][Patch] **[Medium] Rotation-window lifecycle transitions are not atomic: invalidation and accepted re-pin release `window_lock` between clearing `next` and mutating the current pin.** [`crates/maos-a2a-core/src/tofu.rs:495-540`]
- [x] [Review][Patch] **[Medium] A second `open_rotation_window(peer, F2)` silently overwrites an already-open `F1` window and untrusts the in-flight generation without a close.** [`crates/maos-a2a-core/src/tofu.rs:289-323`]
- [x] [Review][Patch] **[Medium] The drop predicate cannot implement “ACK or any typed NACK”: it recognizes only four response variants and lacks response-origin plumbing for the other typed NACK outcomes.** [`crates/maos-a2a-tcp/tests/t_10_4b_rotation_real_timing.rs:177-196`; `crates/maos-a2a-core/src/router.rs:1037-1252`]
- [x] [Review][Patch] **[Medium] Poisoned `window_lock` paths fail open or masquerade as “no window,” allowing stale `next` trust to survive into a fresh accepted re-pin.** [`crates/maos-a2a-core/src/tofu.rs:334-349,495-504,516-540,554-575`]
- [x] [Review][Patch] **[Medium] The two secondary load oracles drain without the bounded `t_2` cutoff used by `rotation_drill`, so a wedged conversation reaches the outer job timeout instead of the drop verdict.** [`crates/maos-a2a-tcp/tests/t_10_4b_rotation_real_timing.rs:1229`; `crates/maos-a2a-tcp/tests/t_14_2_rotation_scene.rs:223`]
- [x] [Review][Patch] **[Low] `build_mesh_n_provisioned` silently falls back to the old fingerprint when `declared` is shorter than the mesh instead of rejecting the malformed fixture.** [`crates/maos-a2a-tcp/tests/support/mod.rs:575,612-616`]
- [x] [Review][Patch] **[Low] “drop accounting is exhaustive” asserts an algebraic identity derived from the same counters and cannot detect missing or duplicated accounting.** [`crates/maos-a2a-tcp/tests/t_10_4b_rotation_real_timing.rs:572-577,1251-1252`]
- [x] [Review][Patch] **[Low] The final swap refactor left two private, uncalled `swap` methods that produce `dead_code` warnings.** [`crates/maos-a2a-tcp/src/transport.rs:974-982,1045-1048`]

#### Independent code review — gate/docs/record chunk (2026-08-29)

- [x] [Review][Patch] **[High] AC6.5(c) requires a named owner for the unbuilt production rotation trigger, but `RELEASE-HOLDS.md` records “none exists.”** The set-valued pin surface has inheritors, but no current sprint key owns the live peer-config reload/operator trigger (`RELEASE-HOLDS.md:86-92`; this story:795-802). **Decision: create backlog story `14-2a-production-mtls-rotation-trigger` and assign the boundary to it.**
- [x] [Review][Patch] **[High] `v2_0 = "advisory"` explicitly downgrades the gate from its inherited v1.5 blocking posture, while the validator checks only that the v2.0 rung exists.** [`xtask/gate-registry.toml:270-279`; `xtask/src/check_rotation_real_timing.rs:500-527`; `xtask/src/gate_common.rs:159-179`]
- [x] [Review][Patch] **[High] `--json` is not a valid machine contract: the kernel leg prints a human banner before GREEN JSON, and the RED path emits no JSON document.** [`xtask/src/check_rotation_real_timing.rs:435-449,584-640`; `xtask/src/check_kernel_baseline.rs:33-58`]
- [x] [Review][Patch] **[High] Gate enrollment assumes prefix filters, but libtest uses substring filters, allowing a non-enrolled test name to substitute for a deleted intended test while exact counts stay green.** [`xtask/src/check_rotation_real_timing.rs:121-180,224-325,539-558`]
- [x] [Review][Patch] **[High] NFR-Rel-9 remains machine-mapped to `check-rotation-real-timing` even though the same row concedes that the gate does not measure its 10⁴-token-validation axis.** [`tests/coverage-matrix.yaml:1453-1470`; `xtask/gate-registry.toml:87-93`]
- [x] [Review][Patch] **[Medium] NFR-Sec-12 still has `gates: []` although the shipped overlap-pin-closure leg is its 100%-detection enforcement.** [`tests/coverage-matrix.yaml:1529-1535`; `xtask/src/check_rotation_real_timing.rs:285-321`]
- [x] [Review][Patch] **[Medium] The source enrollment scanner can miss valid ignored tests declared with visibility or after a mid-line block-comment closer.** [`xtask/src/check_rotation_real_timing.rs:338-390`]
- [x] [Review][Patch] **[Low] The AC6.4.b closure record says “5 legs / 9 enrolled”; the gate publishes 11 enrolled tests.** [`_bmad-output/implementation-artifacts/deferred-work.md:864`]
- [x] [Review][Patch] **[Low] The close-state KLOC record is stale after review patches: current measurements are core 4781/4785 and TCP 1355/1500, not 4759 and 1337.** [`_bmad-output/implementation-artifacts/14-2-10-host-mtls-rotation-chaos.md:1173-1176,1234`]
- [x] [Review][Patch] **[Low] Two identical `run @untracked` lineage blocks were appended without revision, timestamp, run ID, story task, or File List entry.** [`_bmad-output/implementation-artifacts/intent-lineage-coverage-report.md:794-807`]
- [x] [Review][Patch] **[Low] Gate/Cargo documentation still names the rejected `cargo tree --release` probe and contains a truncated “ONE test file” enrollment paragraph.** [`xtask/src/check_rotation_real_timing.rs:46-53,327-337`; `crates/maos-a2a-tcp/Cargo.toml:71-78`]
- [x] [Review][Patch] **[Low] `FingerprintCollision` now covers first-contact and re-pin insertion too, but the error catalog still describes only a rotation-window `next` refusal.** [`xtask/error-catalog.toml:346-355`; `crates/maos-a2a-core/src/tofu.rs:435-472,538-571`]
---

## Dev Agent Record

### Agent Model Used

zai/glm-5.3

### Debug Log References

(Close-pass evidence, 2026-08-29 — all commands run at the final tree:)

- **T0 (2026-08-29)**: tree verified — `git status --porcelain` = 19 staged 14-1 rows (18
  staged-only + sprint-status.yaml whose staged half is 14-1's; this story's in-progress edit is
  the unstaged `MM` half) + this story file `??`. Gates at start: `kloc-check` EXIT=1
  (`_aggregate_hardfail 152078 ≥ 147057` D17/14-6; `maos-domain 8695 > 8644` D14/14-7; headroom:
  xtask 40613/40613=0, maos-a2a-core 4785/4785=0, maos-a2a-tcp 1246/1500=254, maos-a2a 303/1500)
  — matches AC6.6's expected end position inputs. `check-kernel-baseline` EXIT=0 (24472 == 24472).
  `check-decision-register` EXIT=0 (23 rows / 15 open; zero rows target 14-2). 
  `check-rotation-real-timing` EXIT=2 (unrecognized subcommand — gate absent, confirmed).
  Standing `check-dev-record-completeness` EXIT=1 on `deferred-work.md:860: STALE owner 14-1`
  ONLY (14-1's residual, exactly as Blocking condition 7(b) predicts).
- **T1 (2026-08-29) — THE FREE PROVEN-RED VECTOR, captured**: `t_10_4b_rotation_under_load_window`
  in `crates/maos-a2a-tcp/tests/t_10_4b_rotation_real_timing.rs`. 10 consecutive runs, all
  identical: **`W=6 dials confirmed in flight at t_0; issued=6; verdicts by t_2=0; drops=6;
  rotation wall (t_2−t_0) = 1 ms`**. The arm confirms the window via the summed
  `active_connections()` RAII gauge (max observed 6; tight `select! { sleep(0), sweep.next() }`
  poll — the sweep stream is LAZY and launches dials only when polled, the first arm draft
  never polled it and timed out at max_live=0). Rotation initiated mid-window per AC1.2.
  **Structural finding recorded (this is the defect itself, not test machinery):** 14-1's
  `select!` race shape (swap arm vs sweep arm) is IMPOSSIBLE against a teardown rotation — the
  swap must MOVE `mesh_old` while the sweep borrows it; the borrow checker is the first witness
  that teardown-rebind and in-flight traffic cannot coexist. The kill order is forced: the sweep
  (the traffic) dies first; its in-flight frames receive no verdict by `t_2` → drops=6=W by
  AC1.1's definition. No fault injected. T4's overlap (in-place `swap_serving_cert`, no move)
  removes the conflict and must re-run this exact window as a real race with `drops == 0`.

### Completion Notes List

- **THE ONE CAPABILITY, delivered**: certificates roll across a ten-host mTLS mesh while
  conversations are in flight (W=16 outstanding at t_0, 90-pair sweep, swap racing the live
  stream in `select!`) and not one is dropped — proven by a counter that was RED first
  (drops = 6 = W under teardown, 10/10 runs) and by a PERMANENT executable negative control
  (`t_10_4b_rotation_drop_oracle_teardown_control`, drops=6 every run) beside the GREEN
  (`drops == 0` at N=3 AND N=10).
- **The temporal-vs-magnitude distinction the banked finding got wrong, said once** (per the
  story's own instruction): 14-1's `blast_targets: 2` was a MAGNITUDE defect — an input held
  below a floor, repaired by handing the adversary the real peer set. 14-2's load defect was
  TEMPORAL — traffic scheduled entirely outside the rotation window; no value of N repairs it.
  Same consequence (an unfalsifiable binding claim), different repair (build the in-flight
  window and rotate inside it).
- **Gate**: `check-rotation-real-timing` GREEN — 5 legs / 11 enrolled tests / markers verified /
  disclosures transcribed with n= / mutation leg feature-ON / release ship-blocker PROVEN
  (release-check-with-feature must fail on the guard). First run green; reds verified live by
  the close-pass marker seal.
- **Budget after both independent review chunks**: maos-a2a-core **4781/4785 — UNDER
  the D10 wall, NO grant**; the security fixes were paid by removing redundant
  charged in-src tests whose contracts remain in the uncharged integration lane. xtask measured
  grants 40613 → 41072 (dev pass) → **41131** (+59 gate correctness review). maos-a2a-tcp **1355/1500**.
- **Expected end position (AC6.6), machine-verified at close**: workspace tests zero failures;
  check-kernel-baseline GREEN @24472; error-catalog-check GREEN (47 items incl.
  FingerprintCollision); check-decision-register GREEN; check-ship-gate-completeness GREEN;
  check-scale-churn GREEN (no cross-wiring); fmt clean; kloc red on EXACTLY the two foreign
  rows (D14 maos-domain/14-7, D17 aggregate/14-6 — aggregate HIGHER than at start, honest and
  expected); check-dev-record-completeness red on deferred-work.md:860 ONLY (14-1's residual;
  the :864 14-2 row closed with this story).
- **§A6**: TWO passes (AC2.6) — mid-story on the security surface, close-pass on the full diff;
  22+ raw findings across 10 independent reviewer agents + 3 runtime seals (set-widening RED
  on the during-window outsider block; marker suppression RED at the GATE boundary; the
  lenient-predicate finding that produced the predicate-contract test). The window_lock
  editing-casualty (found by two close-pass layers) is the net doing its job on its author.

- T0: all four gates measured with `cmd >/dev/null 2>&1; echo $?` (never through a pipe). The
  two standing kloc reds are 14-6's and 14-7's rows; `all gates green` is not this story's done
  criterion (AC6.6).
- T1: the drop oracle is `drops > 0`, never `drops == W` (AC1.2); the observed value happened to
  be W=6 on all 10 runs. The window helper (`buffer_unordered(W)` + unbounded completion channel
  + channel-before-resolve verdict recording + summed-gauge arm) is the substrate T4 re-uses for
  the `> 0 → == 0` evidence pair.

### Change Log

### File List

- `crates/maos-a2a-core/src/tofu.rs` — MODIFIED: rotation-window side-map + window_lock; open (collision guard incl. invalidated/self) / close (promote-and-retire, promote-first) / oracle; set-valued verify (sites 1/4) + active-pin-bound identity resolution (sites 2/3); lifecycle window closure; `EPinMismatch::FingerprintCollision`
- `crates/maos-a2a-core/src/chaos/harness_3_host.rs` — DELETED (AC6.1)
- `crates/maos-a2a-core/src/chaos/metrics.rs` — DELETED (AC6.1)
- `crates/maos-a2a-core/src/chaos/mod.rs` — MODIFIED: module doc rewritten (flip-date record); dead mod decls removed
- `crates/maos-a2a-core/src/chaos/report.rs` — MODIFIED: WIRED as the markdown surface (engine-filter sample counts; zero-sample = unmeasured); charged in-src tests ported to the uncharged lane
- `crates/maos-a2a-tcp/src/transport.rs` — MODIFIED: SwappableServingCert (from_der-validated) + SwappableDialMaterials + swap_lock; `swap_serving_cert` (validate-first, both-guards commit); `build_server_config` tuple; AC2.4.b no-cache constraint
- `crates/maos-a2a-tcp/Cargo.toml` — MODIFIED: `rotation-fault-inject` feature
- `crates/maos-a2a-tcp/tests/t_10_4b_rotation_real_timing.rs` — REWORKED: drop definition; provision→swap→close `rotation_drill(n)` + 10-host envelope + adversary leg + post-grace probes + in-flight window + teardown control + pin closure + predicate contract + report test + fault-blind test + `compile_error!` guard; all `#[ignore]`-gated
- `crates/maos-a2a-tcp/tests/t_14_2_rotation_scene.rs` — NEW: the N=3 watchable scene
- `crates/maos-a2a-tcp/tests/support/mod.rs` — MODIFIED: `build_mesh_inner` `declared` param + `build_mesh_n_provisioned`
- `crates/maos-a2a/tests/cert_rotation_chaos_3_host.rs` — MODIFIED: canned-twin tests removed, surviving halves kept
- `xtask/src/check_rotation_real_timing.rs` — NEW: the gate (5 legs, 100644)
- `xtask/src/main.rs` — MODIFIED: module + command + dispatch
- `xtask/error-catalog.toml` — MODIFIED: FingerprintCollision row
- `xtask/gate-registry.toml` — MODIFIED: v2_0/v2_2 rungs + mislabel comment fix
- `xtask/kloc.toml` — MODIFIED: xtask measured grants 40613 → 41072 → 41131
- `.github/workflows/discipline.yml` — MODIFIED: gate step swap + comment fix
- `tests/coverage-matrix.yaml` — MODIFIED: NFR-Rel-9 re-base + NFR-Sec-13 notes
- `RELEASE-HOLDS.md` — MODIFIED: 14-2 honesty row (a–d)
- `docs/release/v2.2-capacity-envelope.md` — MODIFIED: mislabel fix + rotation section
- `_bmad-output/implementation-artifacts/mtls-rotation-chaos-report.md` — MODIFIED: superseded banner
- `_bmad-output/implementation-artifacts/deferred-work.md` — MODIFIED: :864 closed
- `_bmad-output/implementation-artifacts/6-3-build-the-a2a-peer-mesh-...md` — MODIFIED: W1 closed
- `_bmad-output/implementation-artifacts/review-diff-14-2.txt` — NEW (review input)
- `_bmad-output/implementation-artifacts/review-diff-14-2-ac2.txt` — NEW (mid-story input)
- `_bmad-output/implementation-artifacts/sprint-status.yaml` — status transitions
- `_bmad-output/implementation-artifacts/14-2-10-host-mtls-rotation-chaos.md` — this record

### Change Log

| Date | Entry |
|---|---|
| 2026-08-29 | **INDEPENDENT REVIEW COMPLETE (implementation + gate/docs/records).** Eight parallel review layers across two approved chunks; 2 decisions resolved, 25 patches applied and verified, 1 finding dismissed. Gate GREEN with 5 legs / 11 enrolled / 11 exact invocations; GREEN and planted-RED `--json` both parse as one document. NFR-Rel-9 false mapping removed; NFR-Sec-12/13 mapped honestly; v2.0/v2.2 blocking preserved; production trigger owned by backlog `14-2a-production-mtls-rotation-trigger`. Final measured budgets: core 4781/4785, TCP 1355/1500, xtask 41131/41131; standing KLOC reds remain exactly aggregate/D17 and maos-domain/D14. |
| 2026-08-29 | **DEV PASS COMPLETE (zai/glm-5.3, status → review).** All 17 tasks T0–T11 executed; 6 ACs landed. The drop oracle's RED→GREEN pair: drops=6=W (teardown, 10/10 runs) → drops=0 (overlap, N=3 AND N=10) with the executable teardown control committed beside it. Gate GREEN 5 legs / 11 enrolled; kernel 24472; maos-a2a-core 4759/4785 (NO grant); xtask measured grant →41072; workspace zero failures; standing reds exactly AC6.6's predicted set. TWO §A6 passes (mid-story + close), 3 runtime seals; the close-pass caught a window_lock editing casualty two layers deep. |
| 2026-08-29 | Story marked in-progress (from ready-for-dev); baseline 38c52811 + staged index preserved per frontmatter. |
