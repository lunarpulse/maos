---
baseline_commit: "**`c0a397ea`** (main; 17-3a `done`). This story, its evidence and the planning pass it rests on (epic §R6/§R7, ADR-060 §Amendment, PRD `DELTA-2026-09-27`, the tracker) were committed together after the fresh validation round (§Validation round) — that commit, not `c0a397ea`'s clean tree, is the dev pass's starting point; the code files are still byte-identical to `c0a397ea`, so `lean-prototype.diff` applies unchanged. Authored 2026-09-26 from five scouts (sandbox core + spike evidence, launch path, tests + CI, AC6 surface, governance) plus a MEASURED kernel prototype (E16-A7), then **re-shaped the same day by the preflight round-table (§Preflight round-table, RT-1…RT-10)** and re-measured with a second, lean prototype (`17-6-evidence/lean-*`). Measured here at HEAD: `cargo run -q --locked -p xtask -- check-kernel-baseline` → `PASSED (maos-kernel-core/src = 24920 lines, 98 files, pinned 24920)`; `kloc-check --json` → `passed: true, alarm: true, aggregate: 167819`, `maos-kernel-core` 19212 / ceiling 19213, `maos-bin` 23401/23401, `xtask` 44101/44101, `maos-bench` 1531/1791, `maos-spirit-sdk` 839/3000, `maos-wasm-host` 1057/1577, `maos-escape-detector` 279/376. **Cites are SYMBOL-first, line second**, re-measured at `c0a397ea`; they supersede the epic's cites where they differ (§Premise corrections)."
depends_on: "**`17-3a-wasm-recall-and-componentize-spike` (`done`)** — Q4b MEASURED (R17-44), which lifted this story's backlog gate. Evidence spent: `spikes/story-17-3a-wasm-recall/t2-ladder/report.txt`, `evidence/t6-ci-run9-t2-annotations.log` (run `36210072648`, both x64 images), `evidence/t6-ci-run10-t2-annotations.log` (run `36210776858`, per-syscall leave-one-out), `evidence/q4b-landlock-execute-loader.log`, `evidence/q4b-forbidden-syscall-selected-candidate-local.log`. **ADR-060 clause 4** (`docs/adr/ADR-060-spirit-forms-by-trust-tier.md:76-77`, ACCEPTED): `[cli_wrapper]` is *governed by its T3 admission control* — binding on AC5 (RT-1). **ADR-031** (binding via ADR-060 `:89`): the WASM-component runner runs inside the **T2 process boundary** — the first production T2 form is 17-3c's (epic §R7 split the old 17-3b in three). **ADR-060 §D-C Amendment** (RATIFIED 2026-09-26): the recall transport is an inherited socketpair descriptor, and building it is not kernel-Δ 0 — the premise of RT-4 / R17-46, now binding."
blocks: "**`17-2-worker-cgroups-applied`** (its AC1 needs a live T2 child — impossible until AC2+AC3 land) and **`17-3c-wasm-spirit-on-the-bus-under-t2`** (its runner runs under T2 through the route AC5 lands). After epic §R7 (R17-51) the hand-offs land on two stories, both FLAG-Winston: **17-3c** takes `memfd_create`, the `maos run` SIGSYS proof (RT-1, R17-47) and the kind-8 row for a production T2 kill (RT-5, R17-48); **`17-3d-wasm-log-recall`** takes `sendto`, `recvfrom` and the inherited-fd hand-off (a CLOEXEC clear in `pre_exec` — `unsafe`, so sandbox zone; RT-4, R17-46). `17-3b-wasm-form-admission-and-contract` (kernel-Δ 0) does not depend on this story. Epic-17 exit line 3 (`epic-17…:21`) is this story's AC1."
spec_alignment: "Rule 9 applies mid-epic. Creating this story disproved or tightened premises in AC1, AC3, AC4, AC5 and AC6 (§Premise corrections P1–P22); the preflight round-table then re-ruled five of the creation-time decisions for long-term correctness (RT-1…RT-10). **Filed now** (not deferred to the landing commit, because they re-shape another story's ACs): epic §R6 rows R17-46…R17-50 (routed by §R7 R17-51 after the 17-3b split) and the 17-3c/17-3d tracker rows. **Applied by the dev pass in the landing commit:** §Epic OLD→NEW edits to the 17-6 section itself. The ACs are the epic's six, tightened; AC5's `maos run` leg moves to 17-3c because an accepted ADR forbids the only form that could carry it here (RT-1) — none is added."
split_from: "Not a split. Created 2026-09-25 by operator decision (Lunarpulse, 17-3a RT-8; D2 = a for AC5; R17-37 re-ruled 2026-09-26 for AC6). **Sizing: WHOLE** — AC2/AC3 without AC4 are invisible (a refused spawn still skips), AC4 without AC2/AC3 is permanently red, and AC5 without AC2/AC3 routes children into a sandbox that cannot start them."
kernel_grant: "◆ **FLAG-Winston — original lean-prototype figure RATIFIED 2026-09-26 by the operator (Lunarpulse: confirmed):** `maos-kernel-core` tokei 19212 → 19283 (+71), `src_lines` 24920 → 25003 (+83), across `security/sandbox/linux.rs` +26, `security/sandbox/mod.rs` +4, `lifecycle/cli_wrapper/runtime.rs` +41, `lifecycle/cli_wrapper/admission.rs` 0 (`17-6-evidence/lean-prototype.diff`, `lean-measure.log`). **Landing figure recorded in the Dev Agent Record after T8: tokei 19212 → 19292 (+80), `src_lines` 24920 → 25015 (+95).** The extra +2/+3 is arm-measured `ppoll` (RT-6); +7/+9 is the 2026-09-28 operator-approved, argument-conditioned `prctl(PR_GET_AUXV)` grant required by ubuntu-26.04's uutils `cat` (Dev Agent Record §AC3, final CI run 36425906424). The creation-time +415 and 17-3a +25/−1 figures stay superseded. Kernel lines, pins and kloc grant (`RATIFIED_AT_EASING`) land together; the review guard was separately operator-ratified 2026-09-28: tokei 19292 → 19309 (+17), `src_lines` 25015 → 25032 (+17), final +97/+112 from the 19212/24920 baseline."
kloc_grant: "Measured on the lean prototype (tokei 14.0.0 code): **NEW crate `maos-exec-deps` = 243** (its own `kloc.toml` row, set to the exact formatted value at landing — `kloc-check` refuses a crate with no row: *missing KLOC budget for measured crate `maos-exec-deps`*); `maos-bin` **+7** (`worker_spawn.rs` 634 → 641), `xtask` **+3** (`check_skill_conformance.rs`), `maos-bench` **+3** (`j1.rs`). Aggregate 167819 → **168146** with those rows (hardfail 170884 — `kloc.toml:658`; the alarm at 158608 already fires and is not silenced). NOT prototyped (measure at landing): AC6's `maos-bin/src/admission.rs` refusal and the smoke-spirit `T0` fix (`main.rs:6785-6788`); the SDK self-check line(s); the fuzz target's xtask lines — `check_fuzz_targets.rs` `TARGETS` row + its `green_workspace()` fixture, `check_fuzz_floor.rs` `REQUIRED_TARGETS` + its test vectors (RT-3, validation V-3); the probe `--benign` mode (out-of-workspace fixtures, uncounted). **`maos-bin` and `xtask` carry ZERO headroom** — every line there is an operator-authorized raise at the exact formatted measured value citing the bare token `17-6` in the same commit (`d11_xtask_ceiling_ratchet.rs:164`) — AUTHORIZED by the operator 2026-09-26 with the kernel grant. `crates/*/tests/` is uncharged; inline `#[cfg(test)]` is charged. Review patch measurement: `maos-exec-deps` 233 → 357 tokei, exact current ceiling; kernel 19292 → 19309 under a separately ratified FLAG."
model: "frontier-class allowlist {opus-4-6, opus-4-7, opus-4-8, gpt-5.5, gpt-5.6, glm-5.1, glm-5.2, glm-5.3, opus-5, equiv}. `FRONTIER_FAMILIES` (`xtask/src/check_dev_model_tier.rs`) is the machine-checked set; `equiv` is prose, NOT a match token. Keep the literal `allowlist {` — it is a NEGATIVE marker: `check_dev_model_used_populated.rs:302` treats any line containing it as boilerplate and SKIPS it."
review: "§A6 full-layer net **BINDING and NON-DEGRADABLE** (◆). Layers: Blind + Edge Case + Acceptance + Test-Infra + **Security** (a containment control AND a parser of hostile input running in the unsandboxed daemon — the 16-6 precedent names the layer) + **non-author runtime re-execution in CI** (a non-author reads every recorded run id from public annotations and re-runs the local legs). E15-A6 binds every probe by name: *does this test read the tree/runtime, or only what it set itself?* — and RT-10 adds: *show the planted diff and a warning-free build, or the fault was not planted.*"
---

# 17-6 — T2 sandbox repair: a T2 child starts, a forbidden syscall dies, proven red in CI ◆ FLAG-Winston

Status: done

> **The capability:** *A process the kernel launches at `SandboxTier::T2` actually starts, actually runs Rust to
> `main`, and actually dies of `SIGSYS` the moment it calls `ptrace` — on every runner image the project ships for,
> read from the job output — and a T2 spawn the host refuses is a red test, never a skip. The one launch primitive
> (`spawn_and_bridge`) applies the sandbox admission grants it, and no manifest can claim a tier nothing applies.*

**Closes:** R17-25 (T2's Landlock ruleset denies every `execve`) · R17-26 (a Rust binary dies before `main` under the
T2 allow-list) · R17-44(a) (the seccomp order defect: the filter never installed anywhere) · R17-27 for the three
named T2 suites (each passes by skipping) · R17-24's missing launch line, **at the primitive** (AC5) · R17-37 (AC6) ·
the `17-6` half of E16-A2 (`sprint-status.yaml:453`) · epic-17 exit line 3.

**Does NOT close, and says so:** the skip-as-pass **class** in every other suite and the xtask gate semantics that
read `SKIP` lines (`20-3a-gate-honesty`, `deferred-work.md:1035`); T3 applied (`17-1` AC4); cgroups applied
(`17-2`); **a production T2 child** — the first is 17-3c's WASM runner, which also carries the `maos run` SIGSYS
proof, `memfd_create` and the kind-8 row for a production kill; the inherited-fd hand-off and `sendto`/`recvfrom` are
17-3d's (R17-46…R17-48, routed by epic §R7 R17-51);
T1's *UID separation* (no mechanism exists — T1 is a typed refusal, not an implementation).

## Story

As **the operator who runs untrusted spawned forms (WASM components next, through the one launch primitive)**,
I want **the T2 tier to be a sandbox that starts real programs and kills forbidden syscalls, routed by the launch
primitive from the spec admission grants, and proven red in CI on every shipped architecture**,
so that **17-2 and 17-3c build on a containment control that has been observed to contain — not on three suites
that have passed by skipping since Stories 11.1a/11.4b.**

## Preflight round-table (2026-09-26 — Winston · Murat · Amelia · John · Mary · Sally · Paige · Vex · Grumbal · Dana · Boundary · Yui · Level · Splinter)

Convened on the operator's directive: *critique the story and resolve the issues raised at creation for the
long-term correctness.* Every ruling is applied in this file; the ones that re-shaped the old 17-3b are filed as epic §R6
(R17-46…R17-50) and on the tracker in the same pass; epic §R7 then split that story into 17-3b/17-3c/17-3d (R17-51) and routed every hand-off below.

| # | Ruling | Why (one line) | Supersedes |
|---|---|---|---|
| RT-1 | **cli_wrapper stays exactly T3.** `resolve_cli_wrapper_tier`'s floor becomes `requested != T3` (T4 is refused typed before any probe; today it slips through to the probe/Liveness check). No T2 Worker, no probe sandboxing, **no error-code rename** — `ECliWrapperRequiresT3` stays true. AC5's `maos run` SIGSYS leg moves to **17-3c** (R17-47; the old 17-3b AC1 — epic §R7); 17-6 proves the route at the primitive. | ADR-060 clause 4 (ACCEPTED): *`[cli_wrapper]` … is governed by its T3 admission control*; ADR-031's T2 boundary belongs to the WASM runner. A ratified AC does not outrank an accepted ADR; admitting T2 Workers to manufacture a test path would re-decide a form taxonomy inside a repair story. | creation-draft D-17-6-C (A), D-17-6-I, Q2 (tracker history) |
| RT-2 | **`BridgeSpawnSpec` carries `sandbox: SandboxSpec`, not `tier`.** The bridge applies the spec admission granted (tier, scopes, caps, cgroup id) and never invents one; the child is held as a private `BridgeChild::{Plain, Sandboxed}` with one `SandboxedChild::child_mut` accessor. | The creation prototype built `resolved_caps: Default`, `declared_scopes: vec![]` *inside the kernel* — policy invented at the spawn site; 17-3c's runner needs an `FsRead` scope for its `.wasm` and 17-2 needs caps, so a tier-only field would force kernel edits on both. Three `take_*` accessors and triplicated if/else arms (and two deleted comments) collapse to one accessor. | D-17-6-B |
| RT-3 | **The ELF reader leaves the kernel: new leaf crate `crates/maos-exec-deps`** (`#![forbid(unsafe_code)]`, `libc` only; `resolve(program, PATH) -> ExecDeps { program, loader, libraries }`), hardened (non-blocking open + regular-file check, checked offsets, `DT_RPATH` honoured, `$ORIGIN` refused typed), with crafted-byte hostile-input tests and a **fuzz target on the house pattern** (`crates/maos-manifest/fuzz`, `crates/maos-domain/fuzz` → `fuzz-cadence.yml` matrix + `xtask/src/check_fuzz_targets.rs` row + report doc). The kernel keeps the rights decision. | It parses input any manifest author controls, inside the unsandboxed daemon. As private kernel code it could only be tested by charged inline tests; as a leaf crate it is tested and fuzzed for free. Measured: a FIFO named as the program is refused in 7 µs instead of hanging the daemon. Kernel 19212 → 19283 (+71) instead of +415. | creation-draft D-17-6-A, Q1 |
| RT-4 | **Each allow-list entry lands with the story that proves it necessary.** 17-6 lands `poll` (x86_64 row), `rt_sigaction` (every Rust binary) and `clone3` (`std::thread`; the benign probe mode spawns and joins one thread, so its drop is provable here). `memfd_create` goes to **17-3c** and `sendto`/`recvfrom` to **17-3d**, each with its leave-one-out (R17-46, routed by R17-51). | D-17-6-F's premise — *17-3b is kernel-Δ 0, so it cannot add them* — is false: the inherited recall fd needs a CLOEXEC clear inside `pre_exec` (`unsafe` → sandbox zone; `runtime.rs` is `#![forbid(unsafe_code)]`), so the recall story (now 17-3d) is FLAG-Winston regardless (P20). A line this story cannot prove red does not belong in this story (rule 11(c)). | D-17-6-F |
| RT-5 | **The kind-8 `sandbox.block` row for a production T2 kill → 17-3c** (R17-48; the old 17-3b AC1), emitted **at the primitive** (the bridge's exit classification), proven through `maos audit query` on the first production T2 kill. | With RT-1 there is no production T2 child in 17-6; the row's proof needs one. Murat's condition, accepted: an AC on 17-3c, not a note. | creation-draft Q3 |
| RT-6 | **aarch64 is measured, or T2 is refused there.** The T2 matrix is **four** images: `ubuntu-24.04`, `ubuntu-26.04`, `ubuntu-24.04-arm`, `ubuntu-26.04-arm` (labels verified in `actions/runner-images` README, 2026-09-26; free for public repos). The aarch64 allow-list entry (hypothesis `ppoll`) is measured by leave-one-out on arm in T8 before the kernel commit; if no arm leg can run, `spawn_sandboxed` refuses T2 on aarch64 with a typed `SandboxUnavailable`. | `aarch64-unknown-linux-gnu` is a shipped release target (`release.yml:46-48`); a T2 that SIGSEGVs every Rust child there is a broken control on a product we ship, not a label. | creation-draft D-17-6-G, Q4 |
| RT-7 | **"Declared capable" stays ambient `CI` (fail-closed default).** No incapable-declaration hatch until a job needs one; the panic text names rule 11(b) and says how a future job declares itself incapable. | Opt-in capability fails open the day someone forgets it; every GitHub-hosted Ubuntu image is capable (run #9). YAGNI on the hatch (Dana). | D-17-6-E (sharpened) |
| RT-8 | **The cookbook's `[sandbox]` + `[resources]` must PARSE, not just name `T0`.** Both examples also teach `[resources]` keys the parser refuses (`max_memory_mb`/`max_cpu_ms` — `RawResourceCaps` is `deny_unknown_fields`; the real fields are `cpu_max_pct`/`memory_max_mb`/`fd_max`). Fix those lines in both blocks (en + ko) and add one uncharged test that extracts both en fenced blocks and parses their `[sandbox]` and `[resources]` with the parsers admission gate 6 uses. `manifest_self_check` is **not** run on the blocks: it requires a `[posture]` section the hello-world block lacks (measured, validation V-2) — making the starter self-check green is 17-3b's page rewrite. The rest of the 138-line `manifest-fields` reference block and the deeper untruth — the hello-world page teaches third parties the `rust-inproc` form ADR-060 reserves for first-party code — go to **17-3b** (`17-3b-wasm-form-admission-and-contract`; it ships and must teach the third-party form; `deferred-work.md` row, R17-49). | *The reader copies the block, not the tier line* (Sally). Dana capped it at the two sections AC6 touches: *the rest of that page is a different story's lie.* | AC6 docs clause |
| RT-9 | **Resolver edges are acceptance, not review trivia**: static binary, symlinked program, PATH-relative program, FIFO/directory/script refused typed without blocking, missing soname, `$ORIGIN` refused, a **relative** `DT_RPATH`/`DT_RUNPATH` entry refused (ld.so resolves it against the child's CWD, which the parent cannot pin — validation V-7), RUNPATH-before-loader-dir search order; `#!` scripts cannot be T2 programs (documented). | Boundary: *every one of these is a daemon thread that either hangs or grants the wrong file.* | obligation (d) |
| RT-10 | **A planted fault is proven only with its diff and a warning-free build.** | Measured twice this session: the creation prototype read `SKIP ×3 … 3 passed` as PASS (P16); the room's own first plant compiled into an **unreachable pattern** and planted nothing (P22). | review rule |

**Operator confirmation (2026-09-26, Lunarpulse: *confirmed*).** Both items the room left open are ratified: the kernel grant as ruled (+71 / +83 + the arm-measured aarch64 entry, and the `maos-bin`/`xtask` raises), and the ADR-060 §D-C transport amendment, now written into `docs/adr/ADR-060-spirit-forms-by-trust-tier.md` §Amendment (17-3a's checklist ticked). ADR-061's egress amendment was not asked and stays pending.

**T8 CI authorization (2026-09-27, Lunarpulse):** throwaway-branch CI runs for T8 are **AUTHORIZED** — a branch pushed over SSH, a temporary workflow on that branch's own `push` trigger, the four images, every verdict a public annotation, captures committed as `*.log`, the branch deleted after capture (the 17-3a T6 recipe).

**Dissents recorded.** Dana: the fuzz target is scope (*"a matrix row, a report doc and an xtask row for a parser
nobody has attacked"*) — folded when Vex showed both existing hostile-input parsers ship one. John: RT-1 moves a
ratified leg; he conceded because the ADR is the higher decision and 17-3c (the old 17-3b's execution half, epic §R7) is the story that makes the leg real.
Grumbal: RT-3 *"moves the lines, it doesn't delete them"* — true for the aggregate (+327 vs +~430); the kernel
number is the one the operator holds at zero.

## Validation round (2026-09-27 — two fresh, non-author readers: executability and consistency)

Run on the operator's instruction *make 17-6 truly ready to dev*. The executability reader rebuilt
`lean-prototype.diff` in a scratch worktree and **re-measured every figure** (tokei +26/+4/+41/0 = +71; `src_lines`
25003; kloc kernel 19283, bin +7, xtask +3, bench +3, `maos-exec-deps` 243; aggregate 168146; the listed suites green
with zero `SKIP`) and spot-checked 30+ cites (correct at HEAD). De-duplicated across the two readers: **5 ship-blockers** (the two workspace gates merge into V-1 below; the room
raised the macOS finding to a blocker as V-5), **13 should-fixes and 5 nits** — every one applied above:

| # | Finding | Fixed in |
|---|---|---|
| V-1 | The 56th workspace member reds two Blocking gates the story never named: `check-workspace-count` (measured `actual 56 ≠ declared 55`) and `stability-matrix --check` (`STABILITY.md:27`). | §Kernel grant item 10; commands; T9, T11 |
| V-2 | AC6's *"`manifest_self_check` on the hello-world block — green"* cannot pass inside RT-8's cap: the SDK shape requires `[posture]` (measured: `missing field posture`). | RT-8, AC6 (gate-6 parsers only; test in `maos-manifest/tests/`) |
| V-3 | The fuzz target misses `fuzz-build`, `check_fuzz_floor.rs` `REQUIRED_TARGETS` (an unknown ledger target is a hard failure) and the `green_workspace()` fixture (measured: two xtask tests fail). | T2.3; kloc_grant |
| V-4 | T5 (*"whole block"*) and the OLD→NEW AC6 row (*"whole cookbook blocks load"*) contradicted RT-8, R17-49 and `deferred-work.md`. | T5; OLD→NEW `:170` |
| V-5 | *(consistency, raised to blocker by the room)* The T2 arm under `cfg(not(windows))` routes macOS T2 to an unmeasured Seatbelt path that also **drops the piped stdio and env** (`macos.rs:31-38`). | D-17-6-B, AC5 (T2 on Linux only; typed refusal elsewhere) |

Should-fixes: V-6 surface-pin item names non-surface keys (fixed: three re-hashed rows `:112`/`:667`/`:677`); V-7 a
relative runpath was resolved against the owner's directory, ld.so uses the CWD (fixed: refused); V-8 the bridge test's
probe path and CI rule (fixed); V-9 AC4 would have widened two helpers' skip sets (fixed: sets unchanged); V-10 the
aarch64 fallback was unexecutable under a four-image AC1 (fixed: x64-only step + refusal assertion + operator FLAG);
V-11 the new kernel-core and escape-detector T2 tests ran only on `ubuntu-latest` (fixed: added to the matrix); V-12
five, not three, `gate_manifest` bypasses — `smoke-spirit` admits `rust-inproc` at a defaulted T2 (fixed: made `T0`;
gate 6 checks the parsed tier); V-13 the CI Dev Note understated `check-escape-detector`'s change (fixed; its false docs
join AC4); plus stale epic anchors (fixed: re-anchored to the committed epic), missing OLD→NEW rows (`:161`, `:163`, exit
line 3's *"no skip path"* and `--locked`), and the kernel `admission.rs:256-258` doc bullet. Nits: `rt_sigaction` in
T8(b), `KO_COVERAGE_MIN=100`, supersession labels, cite ranges, the admission.rs line count.

## Premise corrections at HEAD `c0a397ea` (measured at story creation and at the round-table)

| # | Premise (17-6 section, `epic-17…:133-144`, and this story's creation draft) | Measured now |
|---|---|---|
| P1 | AC1: *`forbidden_syscall`'s probe … and `t2_benign_control` exiting 0* | Those are **skip labels** passed to `skip_if_perm_denied`, not test names. The fns are `forbidden_syscall_killed_by_t2_with_sigsys` (`t2_sandbox_kill.rs:74-110`) and `benign_process_survives_t2_under_same_spec` (`:115-134`); a third, `granted_fs_capability_works_ungranted_is_refused` (`:143-198`), also runs under T2. The benign control is **`/bin/true` (C)** — nothing about a *Rust* program reaching `main` (D-17-6-H). |
| P2 | AC1 runs *in CI … on every runner image*; exit line 3 lives in `epic-17-exit` *created by 17-1 AC1* | **`epic-17-exit` does not exist**; 17-1 is `backlog` and not a prerequisite. No committed workflow has a multi-image matrix (the only one is the spike's `ci-probe.yml.txt:11-21`). `wasm-host-tests` (`discipline.yml:2914-2928`) runs on `ubuntu-latest` (`:2915`) without `--nocapture` (`:2928`). → D-17-6-D. |
| P3 | AC2: *loader and libraries, or a static binary, whichever Q4b layer 1 selects* | Q4b selected **dynamic** (program + loader `Execute\|ReadFile`, libraries `ReadFile` — CI run #9). The spike resolved libraries with `ldd`; the 17-3a kernel prototype did not resolve them at all. → RT-3 / D-17-6-A. |
| P4 | AC3: *P19 names `poll`, `ppoll`, `rt_sigaction`* | Six syscalls were leave-one-out proven for the **runner and relay** (CI runs #9/#10): `poll`, `rt_sigaction` (drop → SIGSEGV 11), `clone3` (rc 101), `memfd_create` (rc 3), `sendto`/`recvfrom` (recall denied). **`ppoll` is NOT needed** on x86_64. A plain Rust binary needs only `poll` + `rt_sigaction` (`q4b-loo-hello-dynamic-local.log`). → RT-4. |
| P5 | AC3: *dropping an added line brings back the pre-`main` abort (`rc=134`)* | `rc=134` was the `LD_PRELOAD` emulation. The real filter gives **SIGSEGV (signal 11)** for `poll`/`rt_sigaction`. |
| P6 | AC3 omits the order defect | **Seccomp never installed anywhere** (R17-44a): `build_seccomp_filters` returns `Ok(vec![bpf, kill_bpf])` (`linux.rs:346`) while its comments claim the opposite (`:319-322`, `:339-345`); installing the second filter needs `prctl`/`seccomp`, which the allow-list refuses → `Io(PermissionDenied)` → every suite SKIPs (CI run #9 `p20`: `skip-count=3 … 3 passed`). |
| P7 | AC3: `libc::SYS_poll` goes into the allow-list | **`libc::SYS_poll` does not exist on aarch64** (libc 0.2.186), and `aarch64-unknown-linux-gnu` is built in the Blocking `release-dry-run` matrix (`discipline.yml:3808`). `poll` goes in the x86_64-only `LEGACY_X86_SYSCALLS` row — correctly named: aarch64 dropped it as a legacy call, like `pipe`/`dup2`/`stat` beside it. → RT-6. |
| P8 | Q4b's *+25/−1 across 5 files* is the grant | Under-measured: no library resolver, no stdio path through `SandboxedChild`, no admission, never compiled xtask or a test target, and it **missed a `BridgeSpawnSpec` site** (`xtask/src/check_skill_conformance.rs:69`). |
| P9 | AC4: cites `sandbox_enforcement_linux.rs:19-23`, `common/mod.rs:152-166` | The helpers span **`:19-28`** and **`:152-175`**. The escape-detector helper's doc (`:140-151`) claims *a capable-host enforcement regression … never reaches this helper* — P6 is exactly such a regression and it reached the helper. Rewritten. |
| P10 | AC4: *a refused spawn fails* | After P6 but before AC2, `execve` fails `EACCES` → also `PermissionDenied` → still skipped. **AC4 lands with (or before) AC2/AC3's reds.** `check_escape_detector.rs:115-122` parses the exact local `SKIP` text — keep it byte-stable. |
| P11 | AC5: *a T2 fixture launched by `maos run` … dies of `SIGSYS`* | **Unreachable at HEAD**: the only spawned form `maos run` reaches is `[cli_wrapper]`, refused below T3 at three sites (`lifecycle/cli_wrapper/admission.rs:275-279`, `:52-57`, `worker_spawn.rs:635-642`), and ADR-060 clause 4 keeps it there (P19). → RT-1. |
| P12 | AC5: *Admission refuses, typed, … `T1` … and `T4`* | On the cli_wrapper path T1 is refused (`ECliWrapperRequiresT3`); **T4 passes the `< T3` floor** and is refused later by the built-in grant, or — under an operator `T4` grant — by `probe_and_verify_shape` or the bare-`String` Liveness check. `parse_sandbox_tier` is `worker_spawn.rs:82-92` (T4 arm `:89`). On the class path `try_from_manifest_str` (`i9.rs:106-114`) already rejects `T4`, and the kernel refuses an effective T1/T4 (`security/mod.rs:419-426`). → RT-1's `!= T3`. |
| P13 | AC6: *gate 6 … already holds gate 5's `class_section.forms` (`:448`)* | `:448` is **gate 4's** binding (`:448-452`); gate 5 (`:454-459`) reads only `class_section.name`; `forms` is read nowhere in `admission.rs`. `class_section` is in scope at gate 6 (`:461-471`). |
| P14 | AC6: *inline test manifests … (counted at story creation)* | **Exactly 1** — `crates/maos-spirit-sdk/tests/spirit_test_smoke.rs:141` (`tier = "T99"`, already expects `InvalidValue { field: "sandbox.tier" }`). **But 8 `spirit_smoke` tests pin `assert_eq!(report.sandbox_tier, "T2")`**: `spirits/{architect:38, butler:87, mira:87, nash:77, observer:98, orchestrator:52, researcher:88, reviewer:42}/tests/spirit_smoke.rs`. 12 `[cli_wrapper]` manifests declare T3 and must not change. |
| P15 | AC5: `spawn_and_bridge` *keeps the piped stdio* through `spawn_sandboxed` | `spawn_sandboxed` keeps the caller's `Stdio` but returns `SandboxedChild` (private `child`, `mod.rs:89-96`; `Drop` kills, waits, removes the cgroup dir `:119-129`); `runtime.rs` is `#![forbid(unsafe_code)]`; `BridgeError` derives `Clone, Eq`. → RT-2. |
| P16 | *The second filter install fails `EPERM`* — still true? | **Yes — and the measurement nearly lied.** The creation prototype planted `vec![bpf, kill_bpf]`, read `3 passed` and reported *"this host permits the later install"*. The same run prints `maos: seccomp apply failed` ×3 and `SKIP …` ×3 above `3 passed` (`17-6-evidence/orchestrator-planted.log`). |
| P17 | `/bin/sh` runs cleanly under T2 | Passes 4/4, but dash prints `sh: 0: getcwd() failed: Operation not permitted` (`getcwd` is not on the allow-list; dash tolerates it) — `lean-green.log`. Not added. |
| P18 | A dropped `poll` reds *"with the pre-`main` abort"* | The red at `t2_sandbox_kill.rs:94` says *"must classify this exit as a SandboxViolation"* and **does not name the signal** (`orchestrator-planted.log`). Every T2 assertion prints the `ExitStatus` it saw. |
| P19 | The creation draft's D-17-6-C: *admit T2 on the cli_wrapper path* | **Contradicts an ACCEPTED ADR**: ADR-060 clause 4 (`:76-77`) governs `[cli_wrapper]` by T3 admission; ADR-031's T2 boundary is the WASM runner's (ADR-060 `:89`). → RT-1. |
| P20 | D-17-6-F: *17-3b (as it stood before epic §R7) is kernel-Δ 0 by construction, so `sendto`/`recvfrom` must land here* | **False.** Under T2 the recall transport is an inherited socketpair fd (R17-44) whose FD_CLOEXEC must be cleared in the child (`q4b-kernel-price-prototype.log` note; the spike did it in `pre_exec` — `t2_ladder.rs.txt`); `pre_exec` is `unsafe` and only the sandbox zone may hold it. The recall story (now 17-3d) needs kernel lines regardless. → RT-4, R17-46. |
| P21 | Creation draft: *the `object` crate fails `--locked`* | An artifact, not a verdict: **any** new dependency edge — including this story's own new path crate — changes `Cargo.lock` once (measured: +8 lines, no external version moves, `lean-measure.log`). The real reason to reject `object` for four ELF fields is a multi-format parser in the kernel graph. |
| P22 | The round-table's first planted fault (*route T2 bare*) | **Planted nothing**: the edit added `T2` to a later match arm; the compiler warned `unreachable pattern` and the T2 row still read `Signaled { signal: 31 }`. The corrected plant (delete the T2 arm) reads `Exited { code: 0 }` — the probe's `ptrace` returned (`lean-bridge.log`). → RT-10. |

## Decisions (rule 7 — one per fork; as ruled by the round-table)

| # | Fork | Ruling |
|---|---|---|
| D-17-6-A | How the kernel finds the loader and libraries (AC2) | **Leaf crate `crates/maos-exec-deps`** (RT-3): `pub fn resolve(program: &OsStr, path_env: Option<&OsStr>) -> Result<ExecDeps, Error>`; `ExecDeps { program, loader: Option<PathBuf>, libraries: Vec<PathBuf> }`, all canonical. Program: a path with a separator is canonicalized; a bare name is found on the child's `PATH` (the `Command`'s own `PATH` env, else the parent's) as a regular file with an exec bit. ELF: opened `O_NONBLOCK`, `fstat` must be a regular file, ELF64-LE only, `phentsize == 56`; `PT_INTERP` → loader; `PT_DYNAMIC` → `DT_NEEDED`/`DT_STRTAB`/`DT_STRSZ`/`DT_RPATH`(15)/`DT_RUNPATH`(29), string table located through the `PT_LOAD` vaddr→offset map with checked arithmetic; strings ≤ 4096 bytes, NUL-terminated, UTF-8. Search per owner: `DT_RUNPATH` (else `DT_RPATH`) — absolute entries only; a relative entry is refused like `$` substitution, because ld.so resolves it against the child's CWD (V-7; the lean diff still joins it to the owner's directory — change it) — then the loader's canonical directory; transitive, deduplicated. Any `$` substitution, missing soname, bad header or non-regular file → `Error` → the kernel refuses (`SpawnError::SandboxSetup`), never a broader grant. The kernel (`linux.rs`) grants program + loader `Execute\|ReadFile`, libraries `ReadFile`. Measured: `/bin/true`, `/bin/sh`, bare `true` resolve in ~60–140 µs; a FIFO → `not a regular file` in 7 µs; a directory → refused; `/usr/bin/ldd` (a `#!` script) → `not an ELF64 little-endian executable` (`lean-bridge.log`). Rejected: `ldd` (runs the loader on the target from the daemon); `object` (P21); `ReadFile` beneath the loader directory (not exact, no cheaper). |
| D-17-6-B | The bridge's spec and child (AC5) | **`BridgeSpawnSpec.sandbox: SandboxSpec`** (RT-2); private `enum BridgeChild { Plain(Child), #[cfg(target_os = "linux")] Sandboxed(SandboxedChild) }` with `fn get(&mut self) -> &mut Child`; private `fn spawn_child(&SandboxSpec, &mut Command, &str) -> Result<BridgeChild, BridgeError>`: `T2` → `spawn_sandboxed` under **`#[cfg(target_os = "linux")]`** mapped to new `BridgeError::SandboxRefused(String)`; on every other OS `T2` → `SandboxRefused("T2 not measured on <os>")` (V-5: macOS's `spawn_sandboxed` rebuilds the `Command` through `sandbox-exec` and drops the piped stdio and env, `security/sandbox/macos.rs:31-38`; RT-6's rule — measured, or refused); `T0 \| T3` → bare `spawn`; any other tier → `SandboxRefused`. `SandboxedChild::child_mut(&mut self) -> &mut Child` (cfg linux) — the guard keeps owning the child and its cgroup dir until the bridge drops it. Every existing comment in `spawn_and_bridge` survives (the creation prototype deleted the `StdinCommands`/J1-bench note — restore it). |
| D-17-6-C | Where the cli_wrapper tier floor sits (AC5) | **Exactly T3** (RT-1): `resolve_cli_wrapper_tier` → `if requested != SandboxTier::T3` with the comment citing ADR-060 clause 4; `ECliWrapperRequiresT3` unchanged (its name and catalog row stay true); `probe_and_verify_shape`'s floor (`:52-57`) and the Liveness re-assert (`worker_spawn.rs:635-642`) stay as defence in depth — the Liveness one becomes the typed `ECliWrapperRequiresT3` text instead of a bare `String`. |
| D-17-6-D | Where exit line 3 runs in CI (P2) | **`wasm-host-tests` becomes a four-image matrix** (`strategy: { fail-fast: false, matrix: { image: [ubuntu-24.04, ubuntu-26.04, ubuntu-24.04-arm, ubuntu-26.04-arm] } }`, `runs-on: ${{ matrix.image }}`, rust-cache key includes the image) with a dedicated step running exit line 3 verbatim (`cargo test --locked -p maos-wasm-host --test t2_sandbox_kill -- --nocapture`) that emits one `::notice` per test outcome (name, signal/rc) — job logs are 403 without a token, annotations are public. The same matrix also runs, after building **both** probe copies, `cargo test --locked -p maos-kernel-core --test sandbox_enforcement_linux --test bridge_t2_route_17_6 -- --nocapture` and `cargo test --locked -p maos-escape-detector -- --nocapture`, so every un-skipped T2 suite and the primitive's route run on the four images (V-11; `workspace-test-suite` on `ubuntu-latest` still runs them too). `epic-17-exit` stays 17-1 AC1's to create; it carries line 3 verbatim. |
| D-17-6-E | *A runner declared capable* (rule 11(b)) | **Ambient `CI`/`GITHUB_ACTIONS` ⇒ capable** (RT-7; precedent `journal_fsync_assertion.rs:66`): a refused T2 spawn PANICS with `T2 spawn refused on a runner declared capable (rule 11(b)): <kind>: <err>` — text that never begins with `SKIP `, so `check_escape_detector.rs`'s `observed_substrate_skip` cannot mistake it for an absent substrate; off-CI each helper keeps **exactly its current skip set** and byte-identical `SKIP` text (wasm-host and kernel helpers: `PermissionDenied`; escape-detector: `PermissionDenied`/`Unsupported`/`SandboxUnavailable`) — no helper gains a new skip condition (V-9). No new env var. |
| D-17-6-F | Which syscalls land here | **`poll` (x86_64 row), `rt_sigaction`, `clone3`; `ppoll` on aarch64**, measured by leave-one-out (RT-4, RT-6). The additional T8 grant approved by the operator 2026-09-28 is **only `prctl(PR_GET_AUXV)`**, checked by its first seccomp argument: ubuntu-26.04's uutils `cat` needs it; unrelated `prctl` operations remain refused. Never `socket`, `connect`, `ppoll` on x86_64. `memfd_create` → 17-3c; `sendto`/`recvfrom` → 17-3d (R17-46, R17-51). |
| D-17-6-G | aarch64 | Measured on `ubuntu-24.04-arm` + `ubuntu-26.04-arm` in T8 (leave-one-out, same ladder as x64) before the kernel commit; the measured entries live in an aarch64 arm of the per-arch list beside `LEGACY_X86_SYSCALLS`. If no arm leg can run: `spawn_sandboxed` returns `SandboxUnavailable { reason: "T2 allow-list not measured on aarch64" }` there (RT-6); the exit-line-3 step then runs on the two x64 images only, the arm legs run a dedicated step asserting that typed refusal, and the refusal's kernel lines go back to the operator as a FLAG before the kernel commit (V-10). |
| D-17-6-H | The benign control (P1) | The **same probe binary** gains `--benign`: spawn and join one thread (proves `clone3`), print `forbidden-syscall-probe: benign ok`, exit 0 — in **both** copies (`crates/maos-wasm-host/test-fixtures/…`, `crates/maos-escape-detector/test-fixtures/…`). New test `benign_rust_probe_survives_t2_under_same_spec` runs it under the identical spec. `/bin/true` stays as the C control. |

## Acceptance Criteria

Each AC cites the code it changes. *Proven red* = a planted fault turns the control red — in CI with the run id
recorded, or locally with command + output committed when CI cannot host it (rule 11(c)); the planted diff and a
warning-free build are recorded with it (RT-10).

- **AC1 (command) — epic exit line 3.** In CI, on **all four** images (D-17-6-D), the step running
  `cargo test --locked -p maos-wasm-host --test t2_sandbox_kill -- --nocapture` shows, in the job output and as
  public annotations: `forbidden_syscall_killed_by_t2_with_sigsys` — the probe killed by **signal 31 (`SIGSYS`)** on
  `ptrace`, `classify_exit` → `SandboxViolation { sandbox_tier: T2 }`; `benign_process_survives_t2_under_same_spec`
  (`/bin/true`) and `benign_rust_probe_survives_t2_under_same_spec` (D-17-6-H) — exit 0;
  `granted_fs_capability_works_ungranted_is_refused` — granted read succeeds, ungranted read refused **with
  `Permission denied` from `cat` naming the file** (the test asserts the child's stderr: Landlock on read, not a
  failed exec); **zero `SKIP` lines**. Every assertion message prints the observed `ExitStatus` (P18). Run ids and
  annotation captures are recorded in the Dev Agent Record. If T8(a) selects D-17-6-G's fallback, AC1 reads *both x64
  images*, and the arm legs assert the typed aarch64 refusal instead (V-10).
- **AC2 — a T2 child can start** (`prepare_landlock`, `linux.rs:163-202`; `spawn_sandboxed`, `:33-153`; new crate
  `crates/maos-exec-deps`). Parent-side, before `fork`, `spawn_sandboxed` calls `maos_exec_deps::resolve(
  command.get_program(), <the Command's PATH>)` and grants program + loader `Execute|ReadFile` and each library
  `ReadFile` (D-17-6-A); nothing is ever granted on a directory the resolver did not name, and never on `/`. A
  resolver error is a typed `SpawnError::SandboxSetup` refusal. The crate ships `crates/maos-exec-deps/tests/`
  (uncharged) covering RT-9's cases with crafted bytes and real system binaries, and a fuzz target on the house
  pattern (`crates/maos-exec-deps/fuzz`, `fuzz-cadence.yml` matrix row, `xtask/src/check_fuzz_targets.rs` row, a
  `docs/compliance/fuzz-exec-deps-report.md`) over a `Read + Seek` parse entry point. **Proven red:** without the
  exec-set grant the `t2_sandbox_kill` CI step reds on `EACCES` (a named panic per AC4, never a skip); run id
  recorded.
- **AC3 — a Rust program reaches `main`, and the kill filter installs** (`build_seccomp_filters`,
  `linux.rs`). (a) **Kill-first**: `Ok(vec![kill_bpf, bpf])`; every installed filter runs and the most restrictive
  result wins (`KILL_PROCESS` > `ERRNO` > `ALLOW`). The order matters because installation needs `prctl`/`seccomp`,
  which the allow-list otherwise refuses. (b) `rt_sigaction`, `clone3` in `basic_syscalls`; `poll` in the x86_64
  per-arch row; arm-measured `ppoll` on aarch64. The operator-approved 2026-09-28 T8 delta allows
  **`prctl(PR_GET_AUXV)` only when arg0 is `PR_GET_AUXV`**, needed by ubuntu-26.04's uutils `cat`; unrelated
  `prctl` operations remain denied. **No `socket`, `connect`, `memfd_create`, `sendto`, `recvfrom`; no `ppoll` on
  x86_64.** **Proven red:** (i) verbatim filter order causes `seccomp apply failed` and a named panic;
  (ii) dropping `poll`/`ppoll` or `rt_sigaction` kills the benign Rust probe (signal 11 on x64, 5 on arm);
  (iii) dropping `clone3` reds it with rc 101 while forbidden `ptrace` still causes SIGSYS;
  (iv) dropping the conditioned `prctl` rule kills the granted-read control on both 26.04 images.
  The final T8 run is `36425906424`; 24.04 uses GNU `cat`, so its `prctl` leave-one-out is not a valid red there.
- **AC4 — the three T2 suites cannot pass by skipping** (`t2_sandbox_kill.rs:32-41`,
  `sandbox_enforcement_linux.rs:19-28`, `maos-escape-detector/tests/common/mod.rs:152-175`). In CI every branch that
  would print `SKIP` panics with the named reason instead; off-CI each helper keeps exactly its current skip set and
  byte-identical `SKIP …` line (D-17-6-E — no helper gains a skip condition). Rewritten docs: the escape-detector
  helper (`:140-151`, P9), `sandbox_enforcement_linux.rs`'s NOTE (`:6-8`), `fuel_t2_matrix.rs:18-21`'s *self-skips*, and
  the gate's own repetitions of the claim P6 disproves — `xtask/src/check_escape_detector.rs:44-46` (*advisory on
  seccomp-blocked hosts*) and `:103-107` (*never reaches that helper*): **docs only**; the parsing code stays 20-3a's.
  Consequence, stated: `check-escape-detector` legs 8/9 become blocking in CI (a refusal panics, so no `SKIP` reaches
  `observed_substrate_skip`). `wasm-host-tests`
  (`discipline.yml:2914-2928`) runs the four-image matrix with `--nocapture` on the T2 step; its *"self-skips without
  CAP_SYS_ADMIN"* comment (`:2912-2913`) goes. **Proven red:** a CI run whose spawn is refused reads red (AC3(i)'s
  plant serves) — run id recorded.
- **AC5 — T2 is a production path at the launch primitive** (operator decision 2026-09-25, D2 = a; RT-1, RT-2).
  `BridgeSpawnSpec` (`runtime.rs:240-269`) gains `sandbox: SandboxSpec`; all five construction sites migrate —
  `worker_spawn.rs:784` builds it from `granted_tier` (`:491-493`, always T3 today) with `spirit_id: "worker"`;
  `runtime.rs:1149` (test), `cli_wrapper_bridge_8_12.rs:32`, `maos-bench/src/harness/j1.rs:219`,
  `xtask/src/check_skill_conformance.rs:69` pass `SandboxSpec::new_for_test(SandboxTier::T0)` (today's bare
  behaviour). `spawn_and_bridge` (`:453-551`) spawns through `spawn_child` (D-17-6-B — T2 on Linux only), keeping the ADR-023
  check (`:457-460`) before any spawn and the piped stdio. cli_wrapper admission floor: exactly T3 (D-17-6-C); its fn
  doc bullet (`lifecycle/cli_wrapper/admission.rs:256-258`, *"below the CliWrapper T3 floor"*) becomes *"any tier other
  than T3 (below: no containment; T4: no spawn path)"* (comment-only). False docs
  rewritten (rule 11(a)): `runtime.rs:19-26`, `:447-452`; `maos-host/src/lib.rs:7-20`, `:42-46`, `:84-87` (*"the
  kernel needs no new field"* becomes false); `maos-wasm-host/src/lib.rs:12-14`; `escape_detector_consumer.rs:13-17`
  and `producer_wired_e2e.rs:17-20` (the route exists; no production T2 child until 17-3c); `i9.rs:79` (T1 has no UID
  mechanism) and `i9.rs:85` (*"T4 — WASM-component sandbox"* → T4 is the reserved WASM *tool* sandbox; WASM-component
  Spirits run at T2 — PRD amended 2026-09-27, epic-17 R17-57). **Wiring (rule 11(a)):** the first production caller of the T2 arm is `17-3c-wasm-spirit-on-the-bus-under-t2`
  (named, not done — R17-47, R17-51). **Proven red** (new `crates/maos-kernel-core/tests/bridge_t2_route_17_6.rs`, `#![cfg(target_os = "linux")]`, resolving
  the probe at `env!("CARGO_MANIFEST_DIR")/../maos-wasm-host/test-fixtures/forbidden-syscall-probe/target/release/forbidden-syscall-probe`
  and asserting it exists, with D-17-6-E's CI rule on a refused T2 row — V-8):
  `spawn_and_bridge` on the probe at `T0` → `Exited { code: 0 }` (ptrace returned — the control), `T2` →
  `Signaled { signal: 31 }`, `T3` → `Exited { code: 0 }` (bare, 17-1's), `T1`/`T4` → `SandboxRefused`; planted:
  delete the T2 arm → the T2 row reads `Exited { code: 0 }` (measured, `lean-bridge.log`). A `T4` cli_wrapper
  request is refused by `resolve_cli_wrapper_tier` (row added to `admission_tier_grant_gate`,
  `cli_wrapper_bridge_8_12.rs:~279`).
- **AC6 — an in-process manifest claims no sandbox it does not get** (operator decision 2026-09-25, re-ruled
  2026-09-26, R17-37; RT-8). maos-bin admission gate 6 (`admission.rs:461-471`; `class_section` bound at gate 4,
  `:448-452`, P13) refuses, typed (new `AdmissionRefusal` variant, stable `code()` + `Display` naming tier and form;
  both matches — `code()` `:247-265`, `Display` `:276-308` — are exhaustive), a manifest whose `class.forms` contains
  `rust-inproc` and whose parsed `sandbox_cfg.tier` is not `T0` (a `[sandbox]` with no `tier` key defaults to `T2`,
  `crates/maos-manifest/src/manifest.rs:158-160`, and is therefore refused for `rust-inproc` — V-12); the doc at `:189-203` is rewritten for AC5+AC6 and the kernel test doc repeating it
  (`operator_posture_ceiling_16_6.rs:227-232`) reworded. The SDK self-check
  (`crates/maos-spirit-sdk/src/spirit_test/manifest.rs:130-139`) refuses the same pair as
  `InvalidValue { field: "sandbox.tier", … }` (keeps `spirit_test_smoke.rs:141` green). The nine first-party lines
  become `tier = "T0"` (`spirits/{architect,orchestrator,reviewer}/manifest.toml:27`, `butler:72`, `digest:29`,
  `mira:54`, `nash:50`, `observer:67`, `researcher:99`), the justifying comments (`observer:65-66`,
  `researcher:97-98`) rewritten, the 8 `spirit_smoke` pins flip `"T2"` → `"T0"` in the same commit. No schema,
  SDK-API or template change; no effective tier change (admission admits at `DEFAULT_FLOOR`,
  `crates/maos-kernel-core/src/capability/cap_policy/mod.rs:225-248`). The one bypass that admits an in-process Spirit
  at a defaulted T2 — the `smoke-spirit` diagnostic, `crates/maos-bin/src/main.rs:6785-6788`
  (`SandboxConfig::default()` on a `rust-inproc` class) — passes an explicit `T0` (V-12).
  **Cookbook (RT-8):** the fenced examples in `docs-site/docs/cookbook/hello-world-spirit.md` (block `:24-48`;
  `[sandbox]` `:38-39`, `[resources]` `:41-43`) and `manifest-fields.md` (block `:17-155`; `[sandbox]` `:34-35`,
  `[resources]` `:37-39`) + ko twins (`docs-site/i18n/ko/docusaurus-plugin-content-docs/current/cookbook/…`,
  `[resources]` at `:42-44` / `:38-40`) name `T0`–`T4`, say an in-process Spirit declares `T0`, and use the real
  `[resources]` fields (`cpu_max_pct`, `memory_max_mb`, `fd_max`) — **nothing else in either block changes** (RT-8).
  One uncharged test, `crates/maos-manifest/tests/cookbook_sections_17_6.rs`, extracts both en fenced blocks and parses
  their `[sandbox]` with `SandboxConfig::from_toml_str` and their `[resources]` with `ResourceCaps::from_toml_str`
  (`manifest.rs:93`, `:112` — the parsers gate 6 uses, `crates/maos-bin/src/admission.rs:461-464`) — green; it does
  **not** run `manifest_self_check` (the hello-world block has no `[posture]`, which the SDK shape requires — measured,
  V-2; the whole-page lesson is 17-3b's, R17-49). en + ko in the same commit (`gate:glossary-lock` counts locked terms incl.
  `Spirit`, `sandbox tier`). `cli-wrapper-spirit.md` stays 17-1 AC5's. **Proven red:** a `rust-inproc` fixture
  declaring `T2` is refused by `maos run` and the door (row in `maosctl_load_16_6.rs:513-538`, built with the
  `REVIEWER_MANIFEST` `.replace()` idiom at `:588-593`) and by `manifest_self_check`; the nine relabelled manifests
  and the four existing `T0` ones (`spirits/hello-spirit:30`, `crates/maos-spirit-hello:45`,
  `templates/spirit-rust:30`, `examples/example-spirit:30`) still load; restoring `max_memory_mb` in the extracted
  block reds the cookbook test through `ResourceCaps` (`deny_unknown_fields`; the SDK shape would ignore it).

## Kernel grant (E16-A7 — measured, not ranged; RULED, RT-3)

Measured on the lean prototype at `c0a397ea` (`17-6-evidence/lean-prototype.diff`, applies cleanly;
`lean-measure.log`): `cargo build --locked -p maos-kernel-core -p maos-bin -p maos-bench -p xtask` rc 0; `cargo test
--no-run` for those + `maos-wasm-host`, `maos-exec-deps`, `worker` rc 0; `t2_sandbox_kill` 3/3,
`sandbox_enforcement_linux` 4/4, `cli_wrapper_bridge_8_12` 9/9, `worker` `cli_wrapper_admission` 7/7, runtime lib
23/23, the escape-detector suite with its three `ESCAPE-*-MEASURED` markers — **zero `SKIP`** (`lean-green.log`).

| Kernel file | What | tokei code Δ |
|---|---|---|
| `security/sandbox/linux.rs` 302 → 328 | kill-first return + comment rewrite; `rt_sigaction`, `clone3`; `poll` in the x86 row; the exec-set call and its three-kind rule loop | **+26** |
| `security/sandbox/mod.rs` 168 → 172 | `SandboxedChild::child_mut` | **+4** |
| `lifecycle/cli_wrapper/runtime.rs` 854 → 895 | `sandbox: SandboxSpec`, `BridgeChild`, `spawn_child`, `SandboxRefused`, the `:1149` test site (charged, inline `#[cfg(test)]`) | **+41** |
| `lifecycle/cli_wrapper/admission.rs` 241 → 241 | `< T3` → `!= T3` + comment (4 lines changed, code count unchanged; plus the `:256-258` doc bullet at landing) | **0** |
| **Total** | kloc **19212 → 19283**; `check-kernel-baseline` **24920 → 25003** (+83 physical) | **+71** |

Plus the aarch64 entry T8 measures (≤ +3, RT-6). Not in the kernel figure: `maos-exec-deps` (243, own row), the
benign probe mode, the helpers, AC6, the CI YAML, the fuzz target, docs.

**Landing reconciliation (2026-09-28):** the table above is the 2026-09-26 lean-prototype
measurement, not the final grant. Arm `ppoll` adds +2 tokei/+3 physical lines; the operator-approved
`prctl(PR_GET_AUXV)` rule adds +7/+9. The pre-review landing figure was **19292 tokei (+80)**,
**25015 `src_lines` (+95)**; `linux.rs` +35, `mod.rs` +4, `runtime.rs` +41, `admission.rs` 0.
The original pin, `RATIFIED_AT_EASING` and Dev Agent Record record that stage.

**Review grant (operator-ratified 2026-09-28):** matching each opened Landlock exec-set
rule fd to the inode parsed by the resolver adds +17 tokei/+17 physical lines in
`linux.rs`. The current pin is **19309 tokei (+97)** and **25032 `src_lines` (+112)**;
the leaf resolver is **357 tokei**. The 19292/25015 and 233 figures below remain
historical dev-pass evidence, not the current ceilings.

**Re-pin artifacts — every one in the SAME commit as the kernel lines** (checklist `kernel-core-baseline.toml:728-763`):

1. `xtask/kernel-core-baseline.toml:505` — `src_lines` 24920 → measured; it **stays on line 505** (position assert
   `kernel_pin_content_hash_16_0.rs:846-851`).
2. Same file, `[kernel_src]` (`:624-…`) from `cargo run -p xtask -- check-kernel-baseline --emit-pin`: `set_hash`
   (`:626`) + the digests of exactly the four touched files — `security/sandbox/linux.rs` (`:703`),
   `security/sandbox/mod.rs` (`:705`), `lifecycle/cli_wrapper/runtime.rs` (`:663`),
   `lifecycle/cli_wrapper/admission.rs` (`:660`). `src_lines` is set by hand.
3. A HISTORY row **below** `:505` (precedents `:506`, `:536`, `:554`, `:611`).
4. `xtask/kloc.toml:247` `maos-kernel-core = 19213` → measured, citing `17-6` and the driver inline; fix the row's
   stale *"24796 -> 24918"*.
5. `xtask/tests/recovery_lane_ceiling_rule.rs:268` `RATIFIED_AT_EASING = 19_213` → the same value.
6. `xtask/tests/kernel_pin_content_hash_16_0.rs` — the four `24920` literals (`:802`, `:820`, `:829`, `:840`).
7. Surface pin (E16-A1; checklist `kernel-core-baseline.toml:750-755`): the gate keys **top-level items**, not fields,
   variants or methods (`xtask/src/check_service_boundary.rs:312-396`, identity `(kind, path, signature_hash)`). Add
   class rows (value `supervision`, the sandbox family's class) in `xtask/kernel-api-classes.toml` for
   `maos_kernel_core::lifecycle::cli_wrapper::runtime::{BridgeSpawnSpec, BridgeError, SpawnedBridge}` (no
   `cli_wrapper::runtime` rows exist today); regenerate `docs/ci-baselines/kernel-surface-v0.1-beta.json` from
   `cargo run -p xtask -- check-service-boundary --json` → `.current_surface` (379 rows before and after — measured,
   V-6). Exactly three rows re-hash: `BridgeError` (`:112`, + variant), `BridgeSpawnSpec` (`:667`, + pub field),
   `SpawnedBridge` (`:677`, a **private-field reshape** — name it so). `SandboxedChild::child_mut` changes no row; the
   `:1367` `use` row does not change. Enumerate the diff in the Dev Agent Record.
8. `xtask/kloc.toml`: NEW row `maos-exec-deps` (exact); raises for `maos-bin` (`:477`) and `xtask` (`:320`) citing
   `17-6`; `maos-bench` absorbs.
9. Prose pins no gate sees (rule 9): `epic-17…:141` (the 17-5 Δ line), `:213` (the §R4.1 pin row), `epic-18…:32`,
   `epic-20…:45` (bolded `**24920**`), the tracker rows `epic-17` / `17-6` — re-grep `24920` at landing.
10. **The workspace grows 55 → 56** (V-1, measured: `check-workspace-count --json` → `actual 56, declared 55`;
   `stability-matrix --check --json` → `in_sync: false`; both Blocking, in `aggregate`): update the
   `<!-- workspace-count-authoritative -->` paragraph in
   `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md:117` (count + a *→ 56 (Story 17-6,
   `maos-exec-deps`)* history entry) and regenerate `STABILITY.md` (`:27`) with `cargo run -p xtask -- stability-matrix`.

Commands, in order (`cargo fmt --all` first; tokei 14.0.0): `cargo run -q -p xtask -- kloc-check --json` ·
`cargo run -p xtask -- check-kernel-baseline --emit-pin` · `cargo run -p xtask -- check-kernel-baseline` ·
`cargo run -p xtask -- check-service-boundary` · `cargo run -p xtask -- check-dependency-closure` (the new edge
must stay clean) · `cargo run -p xtask -- check-fuzz-targets` · `cargo run -p xtask -- check-fuzz-floor` ·
`cargo run -p xtask -- check-workspace-count --json` · `cargo run -p xtask -- stability-matrix --check --json` ·
`cargo test -p xtask --test kernel_pin_content_hash_16_0 --test recovery_lane_ceiling_rule`.

## Review obligations (§A6 — each EXECUTED by a non-author, never read-only)

- (a) Re-run AC1 locally (this host installs kill-first seccomp — `Seccomp: 2`, measured): no `SKIP`, `SIGSYS` on the
  probe, exit 0 on both benign controls.
- (b) Fetch every recorded run's annotations for **all four** images and match the ids.
- (c) Plant each fault once locally, **diff + warning-free build recorded** (RT-10): verbatim order; no exec-set
  grant; drop `poll`; drop `rt_sigaction`; drop `clone3`; delete the bridge's T2 arm; helper with `CI` unset (prints
  the byte-identical `SKIP`) and set (panics with the named reason).
- (d) RT-9's resolver cases pass as tests in `crates/maos-exec-deps/tests/` — and one more each reviewer invents.
- (e) The ungranted `cat` child's stderr names the **file** (E15-A6).
- (f) `/bin/sh` and `/bin/cat` under the final allow-list on all four images; any extra syscall is a **finding
  returned to the operator** as a grant delta, never a silent line (P17: dash's `getcwd` EPERM is tolerated).
- (g) AC5 `Drop`/`on_unload`/`wait_and_finalize` with a `Sandboxed` child: reaped, cgroup dir removed (no leak
  under `maos.slice/`).
- (h) T3 Workers byte-for-byte unchanged (`smoke_cli_wrapper_8_12.rs`, `worker_supervision_16_3.rs`,
  `worker_crash_corpus_16_3.rs`, `worker_hang_corpus_16_3.rs` green).
- (i) `credential_posture_2c.rs:282/301` — no new `env_clear` literal in `runtime.rs`.
- (j) AC6: `maosctl load` and `maos run` refuse the `rust-inproc`+`T2` fixture with the new code; the **five** admission
  paths that bypass `gate_manifest` — `main.rs:3665` (shell hello-spirit), `:6785-6788` (`smoke-spirit`, made `T0` by
  AC6), `:7969` (hello-spirit boot), `:9823/:9845` (daemon control Spirit), `:13001-13007` (`smoke-abi`, T0) — listed
  with the tier each uses (V-12).
- (k) Kernel pin PASSED at the new value; four literals, `RATIFIED_AT_EASING`, surface-pin diff, HISTORY row present;
  kernel lines ≤ the ruled figure.
- (l) aarch64: the arm legs ran (or the typed aarch64 refusal is in, with its test); `release-dry-run` aarch64 green.
- (m) `cargo run -p xtask -- check-escape-detector --json` reads the live legs as substrate-present and green; compare
  with `17-6-evidence/orchestrator-escape-detector.log`.
- (n) Security: the resolver never follows a path it did not canonicalize; a program swapped between resolve and
  `execve` fails closed (Landlock rules bind the resolved inodes — show it: swap the file, expect `EACCES`).

## Declared cut lines (rule 10 — each names an owner, never a bucket)

- **Skip-as-pass outside the three helpers** (e.g. `detection_quality.rs:104-112`) and the gate code that parses
  `SKIP` (`check_escape_detector.rs:115-122`, `AdvisorySubstrate`) → **`20-3a-gate-honesty`** (`deferred-work.md:1035`).
- **The first production T2 child, its `maos run` SIGSYS proof, the runner's syscalls (`memfd_create`, `sendto`,
  `recvfrom`), the inherited-fd hand-off and the kind-8 row for a production T2 kill** →
  **`17-3c-wasm-spirit-on-the-bus-under-t2`** (the child, the SIGSYS proof, `memfd_create`, the kind-8 row) and
  **`17-3d-wasm-log-recall`** (`sendto`, `recvfrom`, the descriptor hand-off) — R17-46…R17-48, routed by R17-51.
- **The cookbook teaches third parties the first-party `rust-inproc` form** → **`17-3b-wasm-form-admission-and-contract`** (it ships and must teach
  the third-party form; `deferred-work.md` row, R17-49).
- **The admission probe for T3 Workers stays bare** (`admission.rs:74`) → **`17-1`** (AC4 makes T3 applied).
- **cgroup limits on T2 spawns** (`linux.rs:357-396`; `worker_spawn.rs` passes `spirit_id: "worker"`, so concurrent
  Workers would share one dir [INFERENCE]) → **`17-2-worker-cgroups-applied`** (*one cgroup layout*).
- **`i9.rs:83` *"T3 … rejected at v0.1-β"* (stale)** → **`17-1`** (rewrites the published isolation claims, AC5).

## Dev Notes

### Current state of every file this story edits (READ each fully before editing)

- **`crates/maos-kernel-core/src/security/sandbox/linux.rs`** (396 lines; `#![allow(unsafe_code)]` `:13`).
  `spawn_sandboxed` (`:33-153`) prepares Landlock (`tier ≥ T2`, `:44-48`) and seccomp (`:50-54`) parent-side,
  installs them in `pre_exec` (`:67-131`: `restrict_self` `:78` — `NotEnforced` → raw write + `ENOSYS` `:80-83`;
  seccomp loop `:95-102`; `setrlimit` `:106-127`), spawns (`:133`), then cgroup limits (`:136-146`).
  `prepare_landlock` (`:163-202`): `ABI::V1`, `BestEffort`, handles `AccessFs::from_all` (incl. `Execute`), adds only
  `FsRead`/`FsWrite` rules. `build_seccomp_filters` (`:208-347`): 52-entry allow-list + 7 x86 legacy, mismatch
  `Errno(EPERM)`; 15-entry `KillProcess` list; returns `vec![bpf, kill_bpf]` (`:346`). **Preserve:** allocation
  parent-side only (module doc `:1-12`); the `NotEnforced` fail-closed; the kill list; `setrlimit` after seccomp;
  the cgroup code (17-2's).
- **`crates/maos-kernel-core/src/security/sandbox/mod.rs`** — `SandboxSpec` (`:32-41`, `new_for_test` `:45-54`),
  `SpawnError` (`:58-75`), `SandboxedChild` (`:89-96`, `Drop` `:119-129`), dispatch (`:136-166`, T3 arm
  `:140-148`), `classify_exit` (`:169-202`). **Preserve:** the T3 arm; Windows/macOS arms compile.
- **`crates/maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs`** (1179 lines, `#![forbid(unsafe_code)]`).
  `BridgeError` (`:166-198`), `BridgeSpawnSpec` (`:240-269`), `SpawnedBridge` (`:297-307`), `spawn_and_bridge`
  (`:453-551`), `wait_and_finalize` (`:651-660`), `on_unload` (`:787-791`), `Drop` (`:795-804`), test literal
  (`:1114`). **Preserve:** ADR-023 check first; stdin EOF on `Signals`; reader threads; RAII kill+reap; every comment.
- **`crates/maos-kernel-core/src/lifecycle/cli_wrapper/admission.rs`** — `resolve_cli_wrapper_tier` (`:266-294`,
  floor `:272-279`, doc `:254-265`). Only the floor, its comment and the `:256-258` doc bullet change.
- **`crates/maos-bin/src/worker_spawn.rs`** — `granted_tier` (`:491-493`), Liveness re-assert (`:635-642`), spec
  literal (`:784-796`).
- **`crates/maos-bin/src/admission.rs`** (AC6) — `AdmissionRefusal` (`:204-245`), `code()` (`:247-265`), `Display`
  (`:276-315`), gates 4–6 (`:444-471`).
- **`crates/maos-spirit-sdk/src/spirit_test/manifest.rs`** (AC6) — tier check (`:130-139`); zero kernel dependency.
- **NEW `crates/maos-exec-deps/`** — start from `17-6-evidence/lean-prototype.diff` (`src/lib.rs`, 243 code
  lines, builds `--locked` after the one-time lock edge; **change its relative-runpath handling to refuse**, V-7);
  the new member moves the workspace count 55 → 56 (§Kernel grant item 10); refactor `Elf::read` to parse any `Read + Seek` for the
  fuzz target; add the workspace member (`Cargo.toml:56`) and the kernel's linux-only dependency
  (`crates/maos-kernel-core/Cargo.toml`, beside `landlock`/`seccompiler`).
- **Tests:** `crates/maos-wasm-host/tests/t2_sandbox_kill.rs` (probe path `:53-66` asserts the fixture exists —
  keep), `crates/maos-kernel-core/tests/sandbox_enforcement_linux.rs`,
  `crates/maos-escape-detector/tests/common/mod.rs` (+ `producer_wired_e2e.rs`, `detection_quality.rs`),
  `crates/maos-kernel-core/tests/cli_wrapper_bridge_8_12.rs`, `crates/maos-bin/tests/maosctl_load_16_6.rs`, the 8
  `spirit_smoke.rs`, `crates/maos-spirit-sdk/tests/spirit_test_smoke.rs`.
- **Fixtures:** both `forbidden-syscall-probe` copies (own `[workspace]`, `libc` only).
- **CI:** `discipline.yml` `wasm-host-tests` (`:2903-2928`), `fuzz-build` (`:2955-2971`); `fuzz-cadence.yml`
  `fuzz-run-t1` matrix (`:53-67`). `workspace-test-suite` (`cargo test --workspace`, `ubuntu-latest`) also picks up every
  new test (`bridge_t2_route_17_6.rs`, the `maos-exec-deps` tests, the Rust benign probe, the cookbook and AC6 tests);
  `check-escape-detector` legs 8/9 **become blocking in CI** because a refusal now panics without a `SKIP` line (V-13).

### Guardrails

- **Never widen to make it pass.** No rule on a directory the resolver did not name; no syscall beyond D-17-6-F;
  never a bare fallback when `spawn_sandboxed` refuses. An extra need found in review is a grant delta for the
  operator.
- **Never run the target** to learn its dependencies (no `ldd`, no `ld.so --list`, no `PT_INTERP` execution).
- **No allocation in `pre_exec`.** Paths, `PathFd`s and BPF are built before `fork`.
- **Reds before greens** (P10): land AC4 with AC2/AC3.
- **Evidence is an annotation that shows the signal** — a job with `SKIP` or no annotation proves nothing.
- **Do not touch T3** (`spawn_sandboxed`'s T3 arm, `spawn_t3`, the Worker path) — 17-1's.
- **Probe build path:** tests read `<fixture>/target/release/forbidden-syscall-probe`; a global `CARGO_TARGET_DIR`
  (`/mnt/build/cargo` here, 17-3a F9) redirects it — build with `env -u CARGO_TARGET_DIR … --target-dir target`.
- **`BridgeError` stays `Clone + Eq`.**
- **Kernel lines stop at the approved landing figure:** +80 tokei / +95 `src_lines`
  (lean +71/+83, arm-measured `ppoll` +2/+3, operator-approved `prctl(PR_GET_AUXV)` +7/+9).
- **One commit** for kernel lines + both pins + kloc rows + surface pin (E16-A1 order: class rows → surface pin →
  `src_lines` + HISTORY → `check-service-boundary` + `check-kernel-baseline`).

### Previous story intelligence (17-3a, `done` at `c0a397ea`)

- CI proof without a GitHub token: a throwaway branch pushed over SSH, a temporary workflow on
  `on: push: branches: [<branch>]`, every verdict a `::notice`/`::error` annotation (≤10 per step), read
  `GET /repos/lunarpulse/maos/check-runs/<job>/annotations` unauthenticated, captures committed as `*.log`, branch
  deleted. T8's reds and the arm leave-one-out ride that recipe.
- `ci-ladder.sh` as committed is missing `;;` after its `seccomp)` arm — do not reuse verbatim.
- SIGSYS under the final allow-list has never been observed in CI — AC1 is its first observation.
- Phrase sub-agent tasks as defensive sandbox hardening (17-3a's Q4b slice was refused mid-flight).
- Use a private `CARGO_TARGET_DIR` for worktree builds (F9).

### Git intelligence (`main`)

`c0a397ea` 17-3a close-out · `5fa8843b` 17-3a verdicts · `a388fe62` operator decisions (rule 11, ADR-068) ·
`92911f59` v0.1.0-alpha.1 · `0c661995` kernel re-pin 24922 → 24920 (copy its HISTORY-row shape). No crate code
changed since `0c661995`.

### Latest technical facts (fetched 2026-09-26)

- **landlock** locked `0.4.4` (latest `0.4.7`: ABI 7–9 additions only) — do not bump. **seccompiler** locked `0.4.0`
  (latest `0.5.0`) — do not bump; `apply_filter` = `prctl(PR_SET_NO_NEW_PRIVS)` + `seccomp(SET_MODE_FILTER)`.
- Several seccomp filters: all run, highest-precedence result wins; `KillProcess` → `SIGSYS` (31). Landlock V1
  checks `execve` and `open`, not `mmap(PROT_EXEC)` — libraries need only `ReadFile`. Landlock rules bind inodes,
  so a swap between resolve and exec fails closed.
- glibc ≥ 2.34 `pthread_create` uses `clone3` and falls back only on `ENOSYS`, not `EPERM` → rc 101. aarch64 has no
  `poll` syscall; glibc's `poll()` issues `ppoll`.
- Runner labels (`actions/runner-images` README): `ubuntu-24.04`, `ubuntu-26.04`, `ubuntu-24.04-arm`,
  `ubuntu-26.04-arm`. `ubuntu-latest` moves to 26.04 between 2026-10-19 and 2026-11-19 (#14748) — never use it here.

### Testing standards

Integration tests in `crates/*/tests/` (uncharged); deterministic; each asserts **why** (signal, errno, stderr). No
test re-pins wording. Planted-fault runs are throwaway — commands, diff and output go in the Dev Agent Record.

### Project Structure Notes

- Kernel code only under `security/sandbox/` (the sole `unsafe` zone) and `lifecycle/cli_wrapper/`; the parser
  lives in the leaf crate. `check-dependency-closure` (`CHECKED_CRATES` = kernel-core, domain) must stay green —
  `maos-exec-deps` depends on `libc` only.
- Fuzz targets follow `crates/maos-manifest/fuzz` (own `[workspace]`, `fuzz_targets/<bin>.rs`, corpus) and are
  registered in `xtask/src/check_fuzz_targets.rs:31-42` with a report doc.
- CI matrix shape: the spike's `ci-probe.yml.txt:11-21`; keep `timeout-minutes`.

### References

- **Evidence** `_bmad-output/implementation-artifacts/17-6-evidence/`: `lean-prototype.diff` (**the reference
  implementation** — applies cleanly to `c0a397ea`; it predates the validation round, so two hunks change at
  landing: the bridge's T2 arm becomes `cfg(target_os = "linux")` (V-5) and `maos-exec-deps` refuses a relative
  runpath (V-7) — neither changes a line count), `lean-prototype.numstat`, `lean-measure.log`, `lean-green.log`,
  `lean-bridge.log` (T0–T4 bridge table; resolver edge cases; the planted T2-arm deletion, and the first plant that
  planted nothing); superseded creation prototype: `prototype.diff`, `orchestrator-*.log`, `variant-iii.txt`.
- Epic `…/epics/epic-17-workers-and-third-party-form-w2.md` (as committed with this story) — exit block `:17-34`,
  Kernel-Δ `:38`, rules 1–11 `:44-46`, 17-6 `:159-170`, §R5 `:246-282`, §R6 `:283` (R17-46…R17-50), §R7 `:295`
  (R17-51…R17-59), Dependencies `:311`.
- ADRs: ADR-060 (`:64-95`, clause 4 `:76-77`), ADR-031 (T2 boundary), ADR-023 (argv prefix).
- 17-3a story: Q4 row `:261`, ratification checklist `:266`, P9/P19–P21 `:415-427`, F9 `:378`.
- `deferred-work.md:1035`; `epic-16-retro-2026-09-20.md:150` (E16-A7).

## Tasks / Subtasks

- [x] **T0 — Baseline.** Re-run `check-kernel-baseline` and `kloc-check --json` at the current HEAD; if either
  moved from the baseline above, re-measure against `lean-prototype.diff` before any kernel commit.
- [x] **T1 — AC4 first: the helpers.**
  - [x] T1.1 Three helpers per D-17-6-E; docs `:140-151`, `sandbox_enforcement_linux.rs:6-8`, `fuel_t2_matrix.rs:18-21`.
  - [x] T1.2 With `CI=true` at HEAD's `linux.rs` the suites PANIC (F1) — capture as the AC4 local red.
- [x] **T2 — `maos-exec-deps` (AC2).**
  - [x] T2.1 Crate from `lean-prototype.diff`; `Read + Seek` parse entry; workspace member; lock edge.
  - [x] T2.2 `tests/`: RT-9 cases (crafted bytes: truncated header, ELF32, big-endian, `phentsize != 56`, strtab
        outside `PT_LOAD`, string past `DT_STRSZ`, unterminated/over-long string, offset overflow, `$ORIGIN`
        runpath, **relative** runpath, missing soname; filesystem: FIFO, directory, `#!` script, symlinked program, PATH-relative,
        static binary, the dynamic probe).
  - [x] T2.3 Fuzz target on the house pattern (V-3): `crates/maos-exec-deps/fuzz` (own `[workspace]`, `[[bin]]`,
        `fuzz_targets/<bin>.rs`, `corpus/<bin>/` seeds, committed `Cargo.lock`); a `fuzz-cadence.yml` `fuzz-run-t1` row;
        a `fuzz-build` step (`discipline.yml:2955-2971`) and its summary line (`:4184`); the `TARGETS` row in
        `xtask/src/check_fuzz_targets.rs:31-42` **and** its `green_workspace()` test fixture (`:248-256`); the bin in
        `xtask/src/check_fuzz_floor.rs:40` `REQUIRED_TARGETS` and its test vectors (an unknown ledger target is a
        hard failure, `:113-117` — consequence stated: the 72 CPU-hour pre-GA floor then applies to this target too);
        `docs/compliance/fuzz-exec-deps-report.md`; `docs/runbooks/fuzz-cadence.md`. xtask lines measured at landing,
        inside the authorized exact raise. `check-fuzz-targets` and `check-fuzz-floor` green.
- [x] **T3 — Kernel (AC2, AC3, AC5)** — from `lean-prototype.diff`:
  - [x] T3.1 `linux.rs`: exec-set grant; kill-first + comments; `rt_sigaction`, `clone3`, `poll` (x86 row).
  - [x] T3.2 `mod.rs`: `child_mut`. `runtime.rs`: `sandbox: SandboxSpec`, `BridgeChild`, `spawn_child`,
        `SandboxRefused`; restore every comment. `admission.rs`: `!= T3`.
  - [x] T3.3 Migrate the five `BridgeSpawnSpec` sites; typed Liveness text (`worker_spawn.rs:635-642`).
  - [x] T3.4 Rewrite the false docs listed in AC5.
- [x] **T4 — Fixtures + suites (AC1, AC3, AC5).**
  - [x] T4.1 `forbidden-syscall-probe --benign` (thread spawn+join) in both copies; `benign_rust_probe_survives_t2_under_same_spec`.
  - [x] T4.2 `granted_fs_…` asserts the file in the child's stderr; every assertion prints `ExitStatus`.
  - [x] T4.3 `crates/maos-kernel-core/tests/bridge_t2_route_17_6.rs` (`#![cfg(target_os = "linux")]`; T0–T4 table; the
        wasm-host probe path, asserted to exist; D-17-6-E's CI rule on a refused T2 row); T4 row in
        `admission_tier_grant_gate`.
  - [x] T4.4 Local: `t2_sandbox_kill`, `sandbox_enforcement_linux`, `maos-escape-detector`, the new bridge test —
        zero `SKIP`; paste output.
- [x] **T5 — AC6.** `AdmissionRefusal` variant + gate-6 check + doc; SDK refusal; nine manifests → `T0` + two
  comments; eight `spirit_smoke` pins; kernel test doc; `smoke-spirit` → explicit `T0` (`main.rs:6785-6788`); the cookbook's
  `[sandbox]` + `[resources]` lines in both blocks, en + ko (RT-8 — nothing else in the blocks) + the extraction test
  `crates/maos-manifest/tests/cookbook_sections_17_6.rs`; `KO_COVERAGE_MIN=100 npm run gate:ko-coverage` +
  `npm run gate:glossary-lock` (in `docs-site/`); proven reds.
- [x] **T6 — CI (AC1, AC4).** `wasm-host-tests` → four-image matrix; exit-line-3 step with `--nocapture` and
  per-test annotations; build both probe copies, then `cargo test --locked -p maos-kernel-core --test
  sandbox_enforcement_linux --test bridge_t2_route_17_6 -- --nocapture` and `cargo test --locked -p
  maos-escape-detector -- --nocapture` (V-11); delete `:2912-2913`; keep `timeout-minutes`.
- [x] **T7 — deferred-work + routing.** Confirm the R17-49 row (cookbook form lesson → 17-3b) is present.
- [x] **T8 — CI proofs (throwaway branch, 17-3a recipe; AUTHORIZED by the operator 2026-09-27).** (a) Arm leave-one-out on `ubuntu-24.04-arm` +
  `ubuntu-26.04-arm` → the aarch64 entries (or the typed aarch64 refusal); (b) planted reds: verbatim order, no
  exec-set grant, drop `poll`, drop `rt_sigaction`, drop `clone3`, delete the bridge T2 arm — diff + warning-free build
  + run id each; if (a) fails, apply D-17-6-G's fallback and tell the operator before the kernel commit;
  (c) record captures; delete the branch.
- [x] **T9 — Re-pin + budgets (same commit as the kernel lines).** §Kernel grant 1–10; all commands green.
- [x] **T10 — Epic OLD→NEW + tracker (rule 9).** Apply §Epic OLD→NEW to the 17-6 section; tracker row `17-6` →
  `review` with measured figures; note the E16-A2 half closed.
- [x] **T11 — Full gates.** `cargo fmt --all`, `cargo clippy --workspace --all-targets --locked`,
  `cargo test --workspace --locked --no-fail-fast`, `check-escape-detector --json`, `check-dependency-closure`,
  `error-catalog-check`, `check-workspace-count`, `stability-matrix --check`, `check-fuzz-targets`, `check-fuzz-floor`,
  `templates-regen --check`.
- [x] **T12 — Clear the three named-prerequisite reds from the workspace run.** *(ADDED 2026-09-28 at the operator's
  request after T11. It is outside the original AC set, and reviewers should check it as its own change.)*
  T11's run failed 10 tests in three suites that E16-A2 lists as host-environment gaps.
  - [x] T12.1 `equiv_harness` (3): **code defect, fixed at origin.** `ensure_native_twin_binary` built the native twin
        on demand but ignored a global `CARGO_TARGET_DIR`, so the binary landed outside the directory it probes. The
        build now passes `--target-dir <twin_dir>/target`.
  - [x] T12.2 `abi_diff_integration` (5): **provisioning, no repo change.** Install `cargo-public-api` 0.51.0 (the
        `CARGO_PUBLIC_API_VERSION` CI pins).
  - [x] T12.3 `two_host_reconcile_2c` (2): **provisioning, no repo change.** Put a Python Ed25519 backend
        (`cryptography`) on `PATH`, as `workspace-test-suite` does (`discipline.yml:3989`).
  - [x] T12.4 Re-run the whole workspace: 0 failed.

### Review Findings

- [x] [Review][Patch] Refuse directory or non-ELF `PT_INTERP` before granting Landlock rights: a symlink to `/` can bind a root-wide rule, then be retargeted to a real loader before `execve`. [crates/maos-exec-deps/src/lib.rs:74]
- [x] [Review][Patch] Bound hostile ELF dynamic-entry, dependency and transitive-resolution work in the unsandboxed parent; repeated `DT_NEEDED` entries grow allocations and filesystem lookups without a limit. [crates/maos-exec-deps/src/lib.rs:195]
- [x] [Review][Patch] Refuse non-regular paths before opening them for ELF parsing; an attacker-controlled RUNPATH can cause the unsandboxed daemon to open side-effectful character devices. [crates/maos-exec-deps/src/lib.rs:145]
- [x] [Review][Patch] Carry inherited `DT_RPATH` into transitive dependency resolution; an executable whose library's dependency lives in its RPATH is refused even though the loader can start it. [crates/maos-exec-deps/src/lib.rs:79]
- [x] [Review][Patch] Print the classified T2 violation and the named-file `Permission denied` stderr in AC1's public CI verdicts, not only the exit status. [crates/maos-wasm-host/tests/t2_sandbox_kill.rs:81]
- [x] [Review][Patch] Test the `prctl(PR_GET_AUXV)` argument constraint directly under T2: the current plant is uncaught on both 24.04 images, and no test detects an unconditional `prctl` grant. [crates/maos-kernel-core/src/security/sandbox/linux.rs:309]
- [x] [Review][Patch] Make the relative-PATH fixture contain a matching executable so deleting the absolute-directory filter actually reds the resolver test. [crates/maos-exec-deps/tests/resolver_17_6.rs:400]
- [x] [Review][Patch] Test the defaulted-T2 admission refusal with a `rust-inproc` `[sandbox]` lacking a tier key, not just an explicit `tier = "T2"`. [crates/maos-bin/src/admission.rs:466]
- [x] [Review][Patch] Rebuild or version-check the release probe before the new `--benign` test; a pre-existing executable ignores that mode and creates a false local SIGSYS failure. [crates/maos-wasm-host/tests/t2_sandbox_kill.rs:66]
- [x] [Review][Patch] Reconcile AC3 and the story's kernel grant/frontmatter with the operator-approved `prctl(PR_GET_AUXV)` addition and the measured +80/+95 kernel pin. [_bmad-output/implementation-artifacts/17-6-t2-sandbox-repair-and-proven-red.md:164]

Review integration FLAG (operator-ratified 2026-09-28): guarding only `PT_INTERP`
at parse time left a path-swap race before `PathFd::new` attached the Landlock
rule. The repair compares each opened rule fd's regular-file device/inode with
the inode parsed by the resolver. The kernel grew **+17 physical / +17 tokei**
over the dev-pass figure, to 25032 `src_lines` (+112 from 24920) and 19309
tokei (+97 from 19212); both kernel pins and `RATIFIED_AT_EASING` moved together.
The leaf now measures 357 tokei (233 before review). Local resolver, sandbox,
bridge, admission, kernel-pin and ceiling tests passed; four-image review-patch
annotations require a new CI run (the recorded run predates this review).

## Epic OLD→NEW edits

**Filed by the round-table on 2026-09-26** (epic §R6): R17-46 (17-3b kernel-Δ 0 is false — FLAG-Winston for the
runner syscalls + inherited-fd hand-off), R17-47 (the `maos run` T2 SIGSYS proof moves to 17-3b AC1), R17-48 (kind-8
row for a production T2 kill → 17-3b AC1, at the primitive), R17-49 (cookbook teaches third parties the first-party
form → 17-3b), R17-50 (the T2 matrix is four images incl. arm). **§R7 (same day)** split the old 17-3b: R17-46's
`memfd_create`, R17-47 and R17-48 land on **17-3c**; R17-46's descriptor hand-off and `sendto`/`recvfrom` on **17-3d**;
R17-49 on the new **17-3b** (R17-51).

**Applied by the dev pass in the landing commit** (the 17-6 section):

Anchors are measured against the epic **as committed with this story** (after §R6/§R7); re-grep each at landing
(symbol-first — the quoted OLD text is authoritative, the line number is a hint).

| Anchor | OLD | NEW |
|---|---|---|
| `:24` exit line 3 (command + comment) | *created by 17-6 AC1 … a refused spawn FAILS — there is no skip path*; no `--locked` | `cargo test --locked -p maos-wasm-host --test t2_sandbox_kill -- --nocapture` (identical to the CI step); *run by `wasm-host-tests` (four-image matrix) from 17-6; a refused spawn FAILS in CI (rule 11(b)); off-CI it prints `SKIP`; `epic-17-exit` (17-1 AC1) carries it verbatim* |
| `:31` provenance line 3 | *runs it without `--nocapture`* | the 17-6 step + run ids |
| `:33` runner images | two images | four for T2 jobs (R17-50) |
| `:38` Kernel-Δ, 17-6 sentence | *measured by a prototype at story creation* | lean +71/+83 plus measured aarch64 `ppoll` and approved `prctl(PR_GET_AUXV)` = final +80/+95 across 4 kernel files; `maos-exec-deps` measured at landing |
| `:64` stories-table row (17-6) | *Q4b count + AC5 prototype count* | the ruled figure |
| `:161` 17-6 *Closes · Δ* | *the exact count 17-3a Q4b measures … plus AC5's production-path lines in … admission* | final +80/+95 across 4 files (E16-A7, RT-3, T8 approved delta) |
| `:163` 17-6 *Sequence* | *before 17-2 … and 17-3b (its runner runs under T2)* | before 17-2 and **17-3c** (its runner runs under T2); 17-3b runs in parallel (R17-51) |
| `:165` AC1 | skip labels | fn names + the Rust benign control; four images (or D-17-6-G's x64 fallback) |
| `:166` AC2 | *whichever Q4b layer 1 selects* | dynamic, resolved by `maos-exec-deps` |
| `:167` AC3 | *`poll`, `ppoll`, `rt_sigaction` … `rc=134`* | `poll` (x86), `rt_sigaction`, `clone3`; kill-first; signal 11 / rc 101; aarch64 measured |
| `:168` AC4 | `:19-23`, `:152-166` | `:19-28`, `:152-175`; ambient-CI capability; each helper's skip set unchanged |
| `:169` AC5 | `maos run` leg; admission refuses T1/T4 | the primitive + `SandboxSpec` (T2 on Linux only); cli_wrapper exactly T3 (ADR-060 cl. 4); `maos run` leg → 17-3c (R17-47, R17-51) |
| `:170` AC6 | *gate 5's `class_section.forms` (`:448`)*; *counted at story creation* | gate 4 binding; 1 inline + 8 `spirit_smoke` pins; the cookbook blocks' `[sandbox]` and `[resources]` parse (RT-8) — the rest of the blocks → 17-3b (R17-49) |
| `:141`, `:213`, `epic-18…:32`, `epic-20…:45` | `24920` | the new pin |

## 결과와 후속 작업 (쉬운 설명, 2026-09-28)

### 무엇을 만들었나

이전에는 제한된 실행 환경(T2)이 프로그램을 시작하지 못했고, 관련 시험 세 개는
실행 거부를 건너뛰면서 성공으로 표시할 수 있었다. 이제 실행 전에 프로그램,
실행을 돕는 로더, 필요한 라이브러리를 찾는다. 프로그램·로더에는 읽기와
실행, 라이브러리에는 읽기 권한만 주고, 금지된 시스템 호출은 실제로
종료시킨다. 실행 경로는 선언한 격리 설정을 그대로 적용한다. 프로세스
격리가 없는 `rust-inproc`은 T2라고 주장할 수 없다.
경로가 검사 후 바뀌어도 실제로 권한을 줄 파일이 검사한 파일과
같은지 확인하고, 손상된 실행 파일을 읽는 작업량에도 상한을 뒀다.

### 다음 이야기와 에픽에 미치는 영향

- **Epic 17 / 17-1 → 17-2:** 17-2는 이제 실제 T2 자식 프로세스로 자원 제한을
  확인할 수 있다. 다만 Worker의 프로세스 구성을 정하는 **17-1이 17-2보다
  먼저**다. 17-1의 T3 Worker 격리를 이번 T2 성공으로 대신 증명해서는 안 된다.
- **Epic 17 / 17-3b → 17-3c → 17-3d:** 17-3b는 WASM 형식·계약을
  준비한다. 17-3c는 이번에 마련한 T2 실행 경로에 실제 WASM Spirit을
  연결하고, `maos run`을 통한 종료·감사 기록을 입증해야 한다. 17-3d는 그
  뒤에 로그 회상용 파일 설명자 전달을 맡는다. **이번 이야기에는 실제
  서비스에서 실행되는 T2 WASM Spirit이 아직 없다.**
- **Epic 18·19·20:** 18-1 전에는 별도 이야기 17-5가 오래된 모델 지정을
  정리해야 한다. 19-3의 Worker 운영 검증에는 17-1이 필요하다. 20-1이
  필요로 하는 공개 WASM 형식·입장 규칙은 17-3b의 몫이며, 모든 시험의
  건너뛰기 문제는 20-3a가 다룬다. 이 이야기만으로 해당 에픽들의 완료
  조건이 충족되는 것은 아니다.

### 잘한 점

- 실제 자식 프로세스에서 허용된 읽기와 금지된 읽기, 정상 종료와
  `SIGSYS` 종료를 각각 관찰했다. 개발 단계의 CI 기록 `36425906424`에는
  네 운영 이미지의 결과가 있으며, 검토 수정 후 로컬에서는 resolver 19개,
  T2 5개, bridge 4개, 기본 T2 입장 거부 1개가 통과했다.
- 파일 권한을 디렉터리 전체로 넓히지 않고 검사한 파일의 식별자까지
  대조했다. 추가 커널 코드 17줄은 운영자 승인을 받은 뒤 핀과 예산에
  함께 반영했다. 공개 커널 API는 바뀌지 않았다.

### 더 잘할 수 있었던 점

- 처음부터 로더 이름 검증뿐 아니라 **검사와 권한 부여 사이의 경로 교체**를
  시험 설계에 넣었으면 재작업과 별도 커널 승인을 줄일 수 있었다. 17-1과
  17-3c는 경로·설정의 검사 시점과 사용 시점이 다른 곳을 설계 검토 때 찾는다.
- 상대 경로 시험은 실제로 선택될 후보가 있어야 하고, 시험용 실행 파일은
  빌드 결과가 최신인지 확인해야 한다. 다음 이야기에서는 회귀 시험마다
  「고장난 구현이면 이 시험이 실패하는가」를 먼저 점검한다.

### 잘못됐던 점과 다음 개선

- 기존 T2 시험은 실행 실패를 **통과로 잘못 표시**했고, 필터 설치 순서
  때문에 실제 보호도 적용되지 않았다. 현재 이름 붙은 세 시험은 CI에서
  건너뛰기를 실패로 취급하지만, **다른 시험 전체**의 같은 문제는
  20-3a에서 따로 막아야 한다.
- 첫 검토본은 루트 디렉터리로 바뀔 수 있는 로더 경로, 특수 장치 열기,
  제한 없는 의존성 탐색, 부모 실행 파일의 검색 경로 누락을 놓쳤다.
  이번에는 검사한 파일과 권한을 줄 파일을 묶고 작업량을 제한했다.
  17-3c의 실행 파일·아티팩트 처리에서도 같은 공격자 입력 검토를 반복한다.
- 개발 단계의 네 이미지 CI 기록은 **검토 수정 이전**의 결과다. 검토 뒤
  로컬 시험과 예산·커널 경계 검사는 통과했지만, 수정된 공개 알림을 네
  이미지에서 다시 관찰한 기록은 없다. 다음 CI 실행에서 분류된 T2 위반과
  거부된 파일 이름을 이미지별 공개 알림으로 확인하고 기록한다. 검토 후
  전체 작업공간 시험도 다시 실행했다고 주장하지 않는다.

### 다음 추천

**Epic 17의 `17-1-worker-egress-allowlist-and-scoped-credential`**부터 진행한다.
선행 17-3a는 완료됐고, 17-1은 Worker의 T3 격리·송신 규칙·자격 증명
경로를 정해 17-2의 자원 제한 작업을 열어 준다. 17-3b는 이 경로와
독립적으로 병행 가능하지만, 17-2를 바로 시작하면 17-1의 미정 프로세스
구성에 의존하게 된다. 시작 시 커널 기준 **25032줄 / 19309 tokei**를
다시 측정하고, T3를 실제 실행으로 증명하며, 새 T2 공개 알림도 확인한다.

## Dev Agent Record

### Agent Model Used

anthropic/claude-opus-5-5 (`opus-5` family), the orchestrator. It owned the kernel lines, the helpers, the suites,
the CI job, T8, the re-pin and every record, and ran two parallel sub-agents on the same model: `ExecDepsTestsAndFuzz`
(T2.2 resolver tests, T2.3 fuzz target + wiring) and `Ac6InprocTierRefusal` (T5). Both results were reviewed and
re-run by the orchestrator; one sub-agent edit was revised (the `AdmissionRefusal` doc, cut to two lines, restored to
an explanatory doc).

### Debug Log References

All in `_bmad-output/implementation-artifacts/17-6-evidence/`:

- `t1-ac4-local-red.log`: T1.2. At HEAD's `linux.rs`, off CI prints byte-identical `SKIP` lines (rc 0). With `CI=true`,
  all three suites fail with the named rule-11(b) panic (rc 101): wasm-host 0/3, kernel 2/4, escape-detector red.
- `t8-local-plants.log`: the local plants. Each entry has its diff and a build with no new warnings in the planted
  file: verbatim order, no exec-set grant, drop `poll`, drop `rt_sigaction`, drop `clone3`, T2 routed bare at the
  bridge, cli_wrapper floor back to `< T3`. Every plant is red.
- `t8-ci-run1-annotations.log`: run **36319846823**, 4 images, 6 plants each. 24.04 and 24.04-arm are all green. 26.04
  and 26.04-arm are red on the granted `cat`; that finding is described below.
- `t8-ci-run2-annotations.log`: run **36320219812**, the 26.04 diagnosis (image facts + strace).
- `t8-ci-run3-annotations.log`: run **36425906424** (FINAL), 4 images, 7 plants. The workflow and the plant harness
  are kept as `t8-run{1,2,3}-workflow.yml.txt` and `t8-plants.py.txt`. Branch `t8-17-6` was deleted after capture.
- `t4-final-local-green.log`: the final local run with `CI=true`. Zero `SKIP`; `SIGSYS` (31) on `ptrace`; the bridge
  table T0 `Exited{0}` / T1 refused / **T2 `Signaled{31}`** / T3 `Exited{0}` / T4 refused; escape-detector
  `ESCAPE-*-MEASURED` ×3; `maos-exec-deps` 13/13.

### Completion Notes List

- Ultimate context engine analysis completed — comprehensive developer guide created (2026-09-26, story creation).
- Preflight round-table 2026-09-26: RT-1…RT-10 applied; kernel figure re-measured on the lean prototype (+71).
- Operator confirmed 2026-09-26: kernel grant RATIFIED; ADR-060 §D-C amendment RATIFIED and written into the ADR.
- Validation round 2026-09-27 (two fresh non-author readers): 5 ship-blockers, 13 should-fixes, 5 nits — all applied; T8 CI runs authorized by the operator.
- **Review patch batch 2026-09-28 (operator-ratified kernel FLAG):** `PT_INTERP` must
  be absolute, non-root and a regular ELF, and the resolver holds an O_PATH
  inode before opening any bytes. The sandbox compares the inode behind every
  opened exec-set Landlock rule with the inode parsed before attachment; a
  directory or wrong-file path swap refuses the spawn instead of granting an
  unrelated tree/file. Bounded dynamic tables, sonames, transitive lookups and
  inherited RPATH close unsandboxed parser work and valid transitive loads.
  Kernel 19292 → **19309** tokei and 25015 → **25032** physical (both +17);
  leaf 233 → **357** tokei; aggregate **168381**. Four-image CI annotations
  for the patched probe/CI output have not yet been observed (prior run
  36425906424 covers the dev pass only).
- **Dev pass 2026-09-27/28, landing figures.**
  - Kernel: **+80 tokei, 19212 → 19292**, and **`src_lines` 24920 → 25015 (+95)**. Per file: `linux.rs` +35,
    `mod.rs` +4, `runtime.rs` +41, `admission.rs` 0. The operator ruled +71/+83. The remainder is two measured
    additions.
    - (a) The aarch64 `ppoll` row, +2/+3, within RT-6's ≤ +3. Measured by arm leave-one-out: dropping it kills the
      Rust benign probe with SIGTRAP (5) on both arm images.
    - (b) **A grant delta the operator approved on 2026-09-28 (FLAG-Winston):** `prctl` is allowed only when
      `arg0 == PR_GET_AUXV` (a seccomp argument condition), +7/+9. This was a T8 finding.
  - How (b) was found: on `ubuntu-26.04` and `26.04-arm` (kernel 7.0), `/bin/cat` is uutils rust-coreutils 0.8.0. It
    calls `prctl(PR_GET_AUXV)` (EPERM), then opens `/proc/self/auxv` (Landlock EACCES), and rustix panics. That killed
    the granted-read half of the fs test with SIGSEGV on x64 and SIGTRAP on arm (run 2 strace). It is not needed on
    24.04 (GNU cat 9.4).
  - Proven red: dropping the prctl rule reds the 26.04 images and is green on 24.04. That plant has nothing to catch
    on 24.04, which is why the two 24.04 jobs of run 3 read `failure` (plant-harness semantics). Every other plant is
    red there.
  - `maos-exec-deps` = **233** (lean 243, minus V-7's refusal rework and a chunked bounded string read).
  - `maos-bin` 23401 → **23423** (+22). `xtask` 44101 → **44123** (+22: fuzz wiring +19,
    `check_skill_conformance` +3). Both raises cite `17-6`.
  - `maos-bench` 1534, `maos-wasm-host` 1062, `maos-escape-detector` 284 (the probe fixtures count) and
    `maos-spirit-sdk` 846 all sit under their ceilings.
  - Aggregate: **168196**, `kloc-check` passed.
- **AC1** (run 36425906424, all four images, exit line 3 verbatim with `--locked`):
  - `forbidden_syscall_killed_by_t2_with_sigsys` shows `signal: 31 (SIGSYS)`.
  - `benign_process_survives_t2_under_same_spec` and `benign_rust_probe_survives_t2_under_same_spec` exit 0.
  - Granted `cat` exits 0; ungranted `cat` exits 1. Its stderr names the file with `Permission denied`, which the test
    asserts.
  - Zero `SKIP`; 4/4 on every image.
- **AC2:** the exec set is resolved parent-side by `maos_exec_deps::resolve`. Program and loader get
  `Execute|ReadFile`; libraries get `ReadFile`.
  - Refusals: relative or empty RUNPATH/RPATH entries (V-7), `$` substitution, `DT_NEEDED` with `/`, relative PATH
    entries (skipped), non-regular files (read with O_NONBLOCK), and non-ELF64-LE input.
  - The `Read + Seek` entry point is `parse_elf`.
  - Resolver tests pass 13/13, covering every RT-9 case.
  - The fuzz target `fuzz_exec_deps_parse_elf` built with `cargo +nightly fuzz` and ran twice: 16.1 M execs in 61 s and
    8.8 M execs in 46 s. No crash.
  - Proven red by the `no-exec-set-grant` plant: the CI step fails with the rule-11(b) panic and EACCES
    (`os error 13`) on every image.
- **AC3:**
  - Kill-first `vec![kill_bpf, bpf]`, with the comments rewritten.
  - Allow-list additions: `rt_sigaction`, `clone3`, `poll` (x86_64 row), `ppoll` (aarch64 row) and
    `prctl(PR_GET_AUXV)`. The per-arch const is renamed `ARCH_SYSCALLS`, since it now carries an aarch64 row.
  - Proven red on all four images:
    - verbatim order: `seccomp apply failed`, then the named panic;
    - drop `poll`/`ppoll` and `rt_sigaction`: signal 11 on x64, signal 5 on arm. AC3 predicted signal 11; that holds
      on x64 only.
    - drop `clone3`: rc 101 (the [INFERENCE] is now measured), with the forbidden probe still dying of SIGSYS.
- **AC4:**
  - The three helpers panic under `CI`/`GITHUB_ACTIONS`. Off CI their skip sets and `SKIP` text are byte-identical.
  - Docs rewritten: `sandbox_enforcement_linux.rs:6-8`, the escape-detector helper doc, `fuel_t2_matrix.rs:18-21`,
    and `check_escape_detector.rs` `:44-46` / `:105-107` (docs only, line-neutral).
  - `wasm-host-tests` is a four-image matrix with the exit-line-3 step (`--nocapture`, one annotation per test), plus
    the kernel, bridge and escape-detector steps (V-11). The *self-skips* comment is gone.
  - `check-escape-detector --json`: every leg `substrate_present: true`, green.
- **AC5:**
  - `BridgeSpawnSpec.sandbox: SandboxSpec` with a private `BridgeChild`, `spawn_child` and `SandboxRefused`. The T2
    arm is `cfg(target_os = "linux")`; elsewhere T2 falls to the typed refusal.
  - All five construction sites migrated. `worker_spawn.rs` builds the spec from `granted_tier` and raises the typed
    `ECliWrapperRequiresT3` on the Liveness path.
  - The cli_wrapper floor is `!= T3`, with a `T4`-under-a-T4-grant row in `admission_tier_grant_gate`. The plant
    `< T3` makes that row red: `Ok(SandboxTier(4))`.
  - False docs rewritten: `runtime.rs`, `maos-host`, `maos-wasm-host`, `escape_detector_consumer.rs`,
    `producer_wired_e2e.rs`, `i9.rs:79/:85`, and the kernel `admission.rs` doc bullet.
  - `bridge_t2_route_17_6.rs` passes 4/4 on the four images. The route-bare plant makes the T2 row `Exited{0}`
    everywhere.
- **AC6** (sub-agent, re-run by the orchestrator):
  - `AdmissionRefusal::RustInprocRequiresT0` (`rust_inproc_requires_t0`) at gate 6; a defaulted T2 is refused too.
  - The SDK refuses with `InvalidValue { field: "sandbox.tier" }`.
  - Nine manifests are relabelled `T0` and their two comments rewritten; the 8 `spirit_smoke` pins are flipped.
  - smoke-spirit uses an explicit `T0`, and the kernel test doc is reworded.
  - Cookbook: `[sandbox]` and `[resources]` updated, en + ko. `cookbook_sections_17_6.rs` passes. `gate:ko-coverage`
    reads 38/38 and `gate:glossary-lock` passes.
  - Proven reds, recorded in the sub-agent report:
    - `maos run` prints `rust-inproc requires sandbox tier T0, got T2`.
    - `maosctl load` returns `rust_inproc_requires_t0 … (HTTP 400)`; this now has a permanent row in
      `maosctl_load_16_6.rs`.
    - The SDK returns `InvalidValue{sandbox.tier, T2}`.
    - Restoring `max_memory_mb` fails with ``unknown field `max_memory_mb` ``.
  - Obligation (j) bypass tiers: shell hello-spirit T0 (embedded manifest), smoke-spirit explicit T0, hello-spirit
    boot T0, daemon control Spirit (its first-party manifests, now T0), smoke-abi T0.
- **Surface pin (E16-A1):**
  - Class rows added for `runtime::{BridgeSpawnSpec, BridgeError, SpawnedBridge}` (`supervision`).
  - Exactly three rows re-hash in `kernel-surface-v0.1-beta.json`, 379 rows before and after:
    - `BridgeError`: `13553a93…` → `a9317d2c…` (+ variant);
    - `BridgeSpawnSpec`: `542acf62…` → `f7abd467…` (+ pub field);
    - `SpawnedBridge`: `7fdd42b5…` → `57c22dc9…` (private-field reshape).
  - `check-service-boundary` passes with 0 violations.
- **Re-pin (one commit):**
  - `kernel-core-baseline.toml:505` = 25015, still on line 505. `set_hash` `cdd49863…` and the four digests.
  - HISTORY row below `:505`.
  - `kloc.toml` kernel row 19292, with the stale `24796 -> 24918` corrected to `24923`. `RATIFIED_AT_EASING = 19_292`.
  - The four `24920` literals → 25015.
  - Workspace 55 → 56: `4-kernel-design.md:117` and `STABILITY.md` regenerated.
  - Prose pins: epic-17 `:141`/`:213`, epic-18 `:32`, epic-20 `:45`.
- **Test repaired, not re-pinned:** `kernel_pin_content_hash_16_0.rs::a_line_neutral_kernel_edit_reds_the_gate…` used
  the LIVE `linux.rs` as the falsifier's "after" side. Growing `linux.rs` (396 → 435) broke its line-neutral premise.
  The vector now rebuilds BOTH sides from git (`kernel-pin-falsifier-15-4^` vs the tag), so it keeps proving the
  hash-only red whatever the live file's size.
- **T11 gates, local:**
  - `cargo fmt --all --check` passes. `cargo clippy --workspace --all-targets --locked` rc 0, and the one new lint
    (`unbuffered_bytes` in `maos-exec-deps`) is fixed.
  - All pass: `check-kernel-baseline` (25015 / 98), `check-service-boundary` (0), `check-dependency-closure`,
    `check-fuzz-targets`, `check-fuzz-floor` (advisory bootstrap), `check-workspace-count` (56), `stability-matrix
    --check`, `error-catalog-check` (47/47), `templates-regen --check`, `check-epic-close-coherence` (21 epics at
    25015), `check-exit-commands`, `check-escape-detector` (all legs green, substrate present), `kloc-check`
    (aggregate 168196).
  - `cargo test --workspace --locked --no-fail-fast`: **4508 passed / 10 failed / 121 ignored**. All 10 failures are
    the pre-existing host-environment gaps E16-A2 already lists, with the same messages:
    - `abi_diff_integration` (5): `no such command: public-api`;
    - `equiv_harness` (3): the native twin is not built under the global `CARGO_TARGET_DIR`;
    - `two_host_reconcile_2c` (2): no Python Ed25519.
  - `t2_sandbox_kill`, the fourth suite on that list, is now green.
- **T12 (ADDED 2026-09-28, operator request — review it as its own change).** The 10 environment-gap failures from
  T11 are cleared (`17-6-evidence/t12-prereq-suites.log`).
  - `equiv_harness` had a **real test-harness defect**, not a missing tool. Its on-demand
    `cargo build --release --manifest-path native-twin/Cargo.toml` honoured the global
    `CARGO_TARGET_DIR=/mnt/build/cargo`, so the binary landed in `/mnt/build/cargo/release/` while the harness asserts
    `native-twin/target/release/`. This is the same trap as 17-3a F9.
    - Fix: `.args(["--target-dir", &target_dir])` at `crates/maos-wasm-host/tests/equiv_harness.rs`
      `ensure_native_twin_binary` (uncharged test code).
    - Before: 3/20 failed with ``on-demand build reported success but did not produce …``. After: 20/20.
    - CI never saw it because CI sets no `CARGO_TARGET_DIR` and pre-builds the twin.
  - `abi_diff_integration` and `two_host_reconcile_2c` have **no repo defect**. Both already fail loud with a named
    prerequisite, and CI installs both. They went green once the dev host was provisioned the way
    `workspace-test-suite` provisions CI:
    - `cargo install cargo-public-api --version 0.51.0 --locked` → 5/5;
    - Python `cryptography` 50.0.1 in a venv (`~/.venvs/maos-test`), because PEP 668 refuses CI's
      `pip install --user` on this host, with the venv first on `PATH` → 10/10.
  - Full `cargo test --workspace --locked --no-fail-fast` with the venv on `PATH`: **4518 passed / 0 failed / 121
    ignored, 553 binaries, rc 0.**
  - Rule 10: `20-3a-gate-honesty` keeps the *class* (how an unprovisioned host reports these suites). T12 fixes one
    instance at origin and changes no gate or skip semantics.
- Epic OLD→NEW applied (every row in §Epic OLD→NEW edits, figures at landing). The R17-49 row
  (`deferred-work.md:1050`, owner 17-3b) is present.
- **For review:**
  - Obligations (a)–(n) are for the non-author net.
  - (f) is answered: `/bin/sh` is dash on all four images; `/bin/cat` is GNU on 24.04 and uutils on 26.04, which is
    what produced the prctl grant delta.
  - Seen, not fixed: `examples/example-spirit/Cargo.toml` omits the SDK `spirit_test` feature, so
    `cargo test -p example-spirit` alone fails E0432. This predates 17-6 and is outside its scope.

### File List

- `.github/workflows/discipline.yml`: `wasm-host-tests` becomes the four-image matrix with T2 steps; `fuzz-build` step
  and summary rows.
- `.github/workflows/fuzz-cadence.yml`
- `Cargo.toml`, `Cargo.lock`
- `STABILITY.md`
- `_bmad-output/implementation-artifacts/17-6-t2-sandbox-repair-and-proven-red.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `_bmad-output/implementation-artifacts/17-6-evidence/{t1-ac4-local-red.log, t8-local-plants.log,
  t8-ci-run1-annotations.log, t8-ci-run2-annotations.log, t8-ci-run3-annotations.log, t4-final-local-green.log,
  t8-plants.py.txt, t8-run1-workflow.yml.txt, t8-run2-workflow.yml.txt, t8-run3-workflow.yml.txt,
  t12-prereq-suites.log}` (new)
- `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md`
- `_bmad-output/planning-artifacts/epics/epic-17-workers-and-third-party-form-w2.md`
- `_bmad-output/planning-artifacts/epics/epic-18-spirits-that-think-w3.md`
- `_bmad-output/planning-artifacts/epics/epic-20-ship-it-w5.md`
- `crates/maos-exec-deps/{Cargo.toml, src/lib.rs, tests/resolver_17_6.rs}` (new)
- `crates/maos-exec-deps/fuzz/{Cargo.toml, Cargo.lock, fuzz_targets/fuzz_exec_deps_parse_elf.rs,
  corpus/fuzz_exec_deps_parse_elf/{truncated-elf, static-elf64, dynamic-runpath-elf64}}` (new)
- `crates/maos-kernel-core/Cargo.toml`
- `crates/maos-kernel-core/src/security/sandbox/linux.rs`
- `crates/maos-kernel-core/src/security/sandbox/mod.rs`
- `crates/maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs`
- `crates/maos-kernel-core/src/lifecycle/cli_wrapper/admission.rs`
- `crates/maos-kernel-core/tests/bridge_t2_route_17_6.rs` (new)
- `crates/maos-kernel-core/tests/cli_wrapper_bridge_8_12.rs`
- `crates/maos-kernel-core/tests/sandbox_enforcement_linux.rs`
- `crates/maos-kernel-core/tests/operator_posture_ceiling_16_6.rs`
- `crates/maos-bench/src/harness/j1.rs`
- `crates/maos-bin/src/{admission.rs, main.rs, worker_spawn.rs, escape_detector_consumer.rs}`
- `crates/maos-bin/tests/maosctl_load_16_6.rs`
- `crates/maos-domain/src/invariants/i9.rs`
- `crates/maos-host/src/lib.rs`
- `crates/maos-wasm-host/src/lib.rs`
- `crates/maos-wasm-host/tests/{t2_sandbox_kill.rs, fuel_t2_matrix.rs}`
- `crates/maos-wasm-host/tests/equiv_harness.rs`: T12.1, the `--target-dir` fix for the on-demand native-twin build
- `crates/maos-wasm-host/test-fixtures/forbidden-syscall-probe/src/main.rs`
- `crates/maos-escape-detector/test-fixtures/forbidden-syscall-probe/src/main.rs`
- `crates/maos-escape-detector/tests/{common/mod.rs, producer_wired_e2e.rs}`
- `crates/maos-manifest/tests/cookbook_sections_17_6.rs` (new)
- `crates/maos-spirit-sdk/src/spirit_test/manifest.rs`
- `crates/maos-spirit-sdk/tests/spirit_test_smoke.rs`
- `docs-site/docs/cookbook/{hello-world-spirit.md, manifest-fields.md}` and their ko twins under
  `docs-site/i18n/ko/docusaurus-plugin-content-docs/current/cookbook/`
- `docs/ci-baselines/kernel-surface-v0.1-beta.json`
- `docs/compliance/fuzz-exec-deps-report.md` (new)
- `docs/runbooks/fuzz-cadence.md`
- `spirits/{architect, butler, digest, mira, nash, observer, orchestrator, researcher, reviewer}/manifest.toml`
- `spirits/{architect, butler, mira, nash, observer, orchestrator, researcher, reviewer}/tests/spirit_smoke.rs`
- `xtask/kernel-api-classes.toml`
- `xtask/kernel-core-baseline.toml`
- `xtask/kloc.toml`
- `xtask/src/{check_escape_detector.rs, check_fuzz_floor.rs, check_fuzz_targets.rs, check_skill_conformance.rs}`
- `xtask/tests/{kernel_pin_content_hash_16_0.rs, recovery_lane_ceiling_rule.rs}`

- `crates/maos-exec-deps/src/lib.rs`, `crates/maos-exec-deps/tests/resolver_17_6.rs`
  (review: bounded parser, safe opens, inode IDs, transitive RPATH and edge tests).
- `crates/maos-kernel-core/src/security/sandbox/linux.rs` (review: rule-fd inode check).
- `crates/maos-wasm-host/test-fixtures/forbidden-syscall-probe/src/main.rs`,
  `crates/maos-wasm-host/tests/t2_sandbox_kill.rs`,
  `crates/maos-kernel-core/tests/bridge_t2_route_17_6.rs`,
  `crates/maos-bin/tests/maosctl_load_16_6.rs` (review probes and coverage).

### Change Log

- 2026-09-28 — Story 17-6 implemented, and the status moves to `review`.
  - T2 starts real programs: the exec-set Landlock grant comes from the new leaf crate `maos-exec-deps`, and seccomp
    installs kill-first. SIGSYS on `ptrace` was proven in CI on four images (run 36425906424).
  - The three T2 suites can no longer pass by skipping in CI.
  - `spawn_and_bridge` applies the admitted `SandboxSpec`, with T2 on Linux only. The cli_wrapper floor is exactly T3.
  - `rust-inproc` manifests must declare T0.
  - Kernel figure: +80 / +95 = ruled +71/+83, plus the aarch64 `ppoll` row, plus the operator-approved
    `prctl(PR_GET_AUXV)` grant delta. The re-pin to 25015 is in the same change set.
- 2026-09-28 — **T12 added at the operator's request.**
  - `equiv_harness`'s on-demand native-twin build now pins `--target-dir`, fixing a real harness defect under a global
    `CARGO_TARGET_DIR`.
  - The dev host was provisioned with `cargo-public-api` 0.51.0 and Python `cryptography`, as CI does.
  - The workspace test run goes from 4508 passed / 10 failed to **4518 passed / 0 failed**.
- 2026-09-28 — Full-layer code-review patch batch, all 10 findings closed.
  - Operator-ratified +17/+17 kernel inode-check guard; current kernel pin
    19309 tokei / 25032 physical, leaf budget 357. No public kernel surface
    changed; 98 files remain pinned.
  - Local resolver 19/19, T2 sandbox 5/5, bridge 4/4 and defaulted-tier
    admission 1/1; pin/ceiling contracts 34/34; KLOC, kernel-baseline and
    service-boundary gates passed. Patched four-image CI annotations have not
    been run; run 36425906424 precedes this review.
