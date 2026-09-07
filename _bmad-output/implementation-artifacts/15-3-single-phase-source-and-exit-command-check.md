---
baseline_commit: "**`55ac884f`** (\"15-2-kloc-ceiling-rebase\"). Every number below was RE-MEASURED at this commit, not inherited: `cargo run -p xtask -- kloc-check --json` → `passed:true, alarm:false, aggregate 155593`, `xtask 42048/42353`, `maos-bin 17109/20109`; `cargo test -p xtask` → 830 passed / 0 failed / 1 ignored; `cargo fmt --all --check` clean; local `tokei 14.0.0` == the CI pin (`discipline.yml:11`). ⚠ **15-2 moved the ground this story stands on.** The epic's 15-3 section still says *\"maos-bin 17109/17109\"* — that was true before 15-2 and is false now: the ceiling is **20109**, headroom **3000**. The epic's own 15-2 table at `epic-15…:86` already carries the right pair; only the AC3 parenthetical was left behind. ⚠ **Do not cite `xtask/kloc.toml` by line number** — this story does not move that file's structure, but `kloc.toml:219`'s own comment already lies about its own value (§9, H11), and quote-form is house style since 15-2 F2."
depends_on: "**15-2 only, and it is `done`.** 15-2 supplied the `maos-bin` 3000-line headroom the verb table spends and the `xtask` 305-line headroom the new gate draws on. Nothing else blocks: the phase consolidation touches only `xtask/src`, and `check-exit-commands` reads planning artifacts. ⚠ **This story does NOT depend on 15-1.** 15-1 makes the FIRST exit line green; this story creates the SECOND. They are independent and 15-1 is scheduled after."
blocks: "**Rule 1 for the entire recovery lane, and four named downstream ACs.** Until `maos` can be asked what verbs it has, confidence rule 1 (*verb-first*) is a promise, not a control — `epic-15-21-preflight-r2-2026-09-05.md:35` calls the `maos --help` verb table *\"the single highest-leverage edit in the lane\"* and `:105` orders it second. Named consumers: **18-2 AC1** (`maos eval halt --class butler`, `epic-18…:23,:66,:87`), **20-2 AC2** (the homebrew formula's `maos --version` test, `epic-20…:81,:101,:113`), **21-1 AC1** (`epic-21…:93`), and Epic 16's `maos init` / `maos uninstall` / shell door. Exit-block line 2 of Epic 15 cannot run until this story lands."
spec_alignment: "⚠ **THE EPIC IS AMENDED IN THE SAME COMMIT — eleven places.** Operator directive in force (`feedback_pay_effort_early_deferring_drifts`, rule 9): *no drift in spec; the story and the epic say the same thing, or the epic is amended in the same commit; and never let a story be merely BETTER than its epic.* Five adversarial scouts disproved premises in this story's own section. OLD→NEW list in §11. The largest: **AC4's stated justification for retiring `check-cna-registration` is factually inverted** (§1), **AC2's minimal form installs the decay it exists to prevent** (§2), **AC3 names the wrong CI list** (§5), **AC3's own tokeniser clause is a null control** (§6), and **AC4's \"xtask net ≤0\" is arithmetically false** (§9). `epic-19`, `epic-20` and `xtask/kloc.toml` also carry edits (AC7)."
split_from: "Not a split. Authored from `epics/epic-15-foundations-w0.md:124-131` (the `### 15-3-…` section) under Round 3. ⚠ **SIZING FLAGGED FOR THE OPERATOR, NOT DECIDED HERE:** the epic prices this story as *\"rule 1 mechanical\"* and §E prices the gate at *\"+≤60\"*; measured against the four closest anchors the gate alone is **380–520** production lines (§9), and the story additionally carries a 27-file rename, a two-`main()` verb table, and a gate retirement. It is delivered WHOLE. ⚠ **RULED at the 2026-09-07 round-table, on two independent arguments that agree:** (1) *capability* — rule 1 becomes a control only when the verb table and the gate ship together; split them and you ship a `maos --help` nobody reads plus a phase cleanup nobody notices, and rule 1 stays a promise; (2) *budget* — AC2's dedup (−104) is what funds AC5, so the obvious seam (AC5 → `15-3b`) is the seam the arithmetic forbids. The seam everyone can see is the one the budget rules out. **Dissent recorded (Dana):** three weeks minimum, and every day of it is a day rule 1 is not a control. The operator may still split it; the room's answer is that the split costs more than it saves."
kernel_grant: "**NONE and none needed. ZERO kernel-Δ @24472, by construction.** Nothing here touches `crates/maos-kernel-core/src`. Files written: `xtask/src/*` (8 gate modules + 2 new), `crates/maos-bin/src/main.rs`, `crates/maos-bin/tests/`, `.github/workflows/discipline.yml`, `xtask/gate-registry.toml`, `tests/coverage-matrix.yaml`, `tests/phase-config.toml`, 12 xtask fixtures, `docs/adr/`, and the planning artifacts of §11. `check-kernel-baseline` is GREEN at HEAD (`24472 == 24472`) and this story cannot move it. T14 asserts `maos-kernel-core/src` is byte-unchanged in the diff."
kloc_grant: "**Draws on 15-2's existing `xtask` and `maos-bin` grants; asks for no new ceiling — but the margin is real and must be measured, not assumed.** `xtask` 42048/42353 → **305 free**; `maos-bin` 17109/20109 → **3000 free**. ⚠ **A GRANT IS A GLOBAL, NOT A RESERVATION** (`feedback_grant_is_a_global`): `kloc.toml`'s xtask ledger books *\"15-3 +34 at top\"* alongside 15-1, 19 (+150), 21-2 (+150–300) and 21-4 (+150–350) — **450–800 of asks against 271 remaining after 15-3's booked +34.** The row is already over-subscribed, so 15-3's actual net is load-bearing for four later stories, and T0 re-measures before a line is written. **Measured projection (§9): net +95 … +235.** The `+34` booking is FALSE in both directions and AC7 corrects it. ⚠⚠ **THE FULL CONSOLIDATION IS WHAT FUNDS THE GATE.** AC2's *minimal* form (delete 7 consts) frees **−4 to −7** and does not pay for AC5; the full dedup of §2 frees **−104**. The story is affordable only in the version that is also correct. **Tests are free**: `kloc_check`'s tokei arg vector (`-e tests`) passes a bare `-e tests` to tokei, a depth-agnostic glob — RUN at HEAD, 546 Rust reports, **0** under any `tests/` dir. Every new test line goes in `xtask/tests/` or `crates/maos-bin/tests/`; a test written in-`src` is charged AND, per `crates/maos-bin/src/topology.rs`'s module doc, never executed by CI."
model: "frontier-class allowlist {opus-4-6, opus-4-7, opus-4-8, gpt-5.5, gpt-5.6, glm-5.1, glm-5.2, glm-5.3, opus-5, equiv}. `FRONTIER_FAMILIES` (`xtask/src/check_dev_model_tier.rs:45-48`) is the machine-checked set; `equiv` is prose, NOT a match token. Keep the literal `allowlist {` (`check_dev_model_used_populated.rs:302`)."
review: "§A6 full-layer net (Blind + Edge Case + Acceptance + Test-Infra + non-author runtime). **Test-Infra is load-bearing and this story is why.** Both of its new controls have a documented path to shipping as null controls, and one of them cannot be run by CI at all unless the story enrolls it by name: **no CI job runs `cargo test -p maos-bin` unscoped** — all ten invocations are `--test <name>`-scoped (`discipline.yml:1924,1925,1927,1928,1951,1969,1970,1985,2048,2809`), so **26 of the 38 files in `crates/maos-bin/tests/` execute nowhere**, and `discipline.yml:1902-1903` says so in its own comment. The same is true of `xtask`: `discipline.yml:1931` records that *no job runs `cargo test -p xtask` unscoped*. ⚠ **The reviewer must re-run the gate and both proven-red vectors, and must check the `discipline.yml` enrollment lines exist** — a green suite proves nothing about whether CI would ever have run them."
---

# 15-3 — Single phase source, `maos --help`, and the exit-command check

Status: **done.**

> **The capability:** *Every phase decision in this repository reads one constant, `maos` can be
> asked what verbs it has, and a machine — not a reviewer — checks that every epic's exit command
> is made of verbs that exist or are owed by a named, open story.*

**Closes:** the epic's S4 · confidence **rule 1** becomes mechanical rather than aspirational ·
decision **D20** (both un-migrated gates, not one) · retires `check-cna-registration` **without
dropping a control**.

---

## What this story actually is

Confidence rule 1 says *"exit verbs exist at HEAD or are AC1 in this epic."* At HEAD that rule is
enforced by nobody. Three separate things make it unenforceable, and this story removes all three:

1. **`maos` cannot be asked what verbs it has.** Measured, not read — `MAOS_HOME=<scratch> timeout
   45 ./target/debug/maos --help` → **EXIT 124**, killed by SIGTERM, **0 bytes on stdout**, 529
   lines on stderr, opens the SQLite TL on disk, spawns 5 watchdogs and 34 threads, **panics four
   tokio workers** (`monotonic_now_ns() called before init_monotonic_base()`,
   `maos-capability/src/cap_tokens/mod.rs:69`), and writes two files. It never listens and never
   returns. The epic says `--help` "boots the daemon"; that understates it — **a CI gate that
   shells `maos --help` without a timeout hangs the runner.**
2. **The phase a gate binds at has three sources, not one**, and the epic's oracle can see only the
   first (§2).
3. **Nothing reads the exit blocks.** `grep -rn "check-exit-commands" xtask .github` is empty.

Everything below is what five adversarial scouts DISPROVED in this story's own epic section.

---

## 1. 🔴 AC4's JUSTIFICATION IS INVERTED. THE GATE IS LIVE, AND RETIRING IT DROPS TWO CONTROLS.

The epic says: *"`check-cna-registration` (absent-PASS at `check_cna_registration.rs:54-60`;
`docs/compliance/cna-registration.md` **absent at HEAD → vacuous today**) retired…"*

Measured:

```
$ git ls-files docs/compliance/cna-registration.md
docs/compliance/cna-registration.md                 # tracked; 92 lines; added 3806d9df (Epic 10-3)
$ cargo run -q -p xtask -- check-cna-registration
check-cna-registration: PASS (CNA doc + SECURITY.md valid)     # exit 0 — the LIVE arm
```

The absent-PASS arm at `:54-60` is **not the arm taken**. The gate runs its content-validating path
and asserts three properties (`check_cna_registration.rs:64-85`):

| | Property | Code | Survives retirement? |
|---|---|---|---|
| (a) | the CNA doc exists and is non-empty | `:64-71` | **No** — and low value; the doc is static evidence |
| (b) | `SECURITY.md` carries no `<TO-BE-PUBLISHED>` GPG placeholder | `:75-80`, const `:23` | **NO — and this is a live control** |
| (c) | `SECURITY.md`'s supported-versions table has a real `1.0.x` **row** | `:81-85`, `has_version_table_row` `:38-50` | **NO — and this is a live control** |

`has_version_table_row` is not a `contains()`: it was hardened at Epic 10-3 review against the
`11.0.x` substring false-pass. `SECURITY.md:56` satisfies it today.

**Nothing else enforces (b) or (c).** The only other reader of `SECURITY.md` in the whole repo is
`check_security_md.rs`, and its `REQUIRED_SECTIONS` (`:16-21`) asserts **only four H2 headers** —
its own doc comment says it is *"intentionally header-text-based (not regex-rich) so that prose
evolution within sections does not break CI."* And the CNA doc itself names the regression the gate
exists to catch, at `cna-registration.md:91`: *"`SECURITY.md` regresses (placeholder returns,
version table drops `1.0.x`)."*

Retiring the gate as the epic describes is **exactly** the "a claim standing in for a control"
failure the Epic-12 retro named. **Ruling F1.**

---

## 2. 🔴 AC2's MINIMAL FORM INSTALLS THE DECAY IT EXISTS TO PREVENT

There are not 8 phase constants. There are **16 constants and 12 function copies**, and the six that
matter are **stale**:

```
$ grep -rn "^const .*PHASE\|^pub const .*PHASE" xtask/src/*.rs        # 16 hits
gate_common.rs:161   pub const PHASE_ORDER = ["v1_0","v1_5","v2_0","v2_2"]   <- canonical
gate_common.rs:166   pub const CURRENT_PHASE = "v1_5"                        <- canonical
check_cohort_mesh.rs:8  CURRENT_PHASE   :9   PHASE_ORDER  (4 elems — matches)
check_enterprise_identity.rs:15 / :14   PHASE_ORDER = ["v1_0","v1_5","v2_0"]   STALE
check_enterprise_pdp.rs:51      / :48   STALE
check_escape_detector.rs:62     / :59   STALE
check_fkcs.rs:16                / :17   STALE
check_trial_attestation.rs:23   / :24   STALE
check_wasm_form_equiv.rs:50     / :46   STALE
```

Six gates additionally carry **byte-identical private copies of the shared functions** —
`phase_disposition` at `check_fkcs.rs:825`, `check_wasm_form_equiv.rs:71`,
`check_enterprise_pdp.rs:73`, `check_enterprise_identity.rs:34`, `check_escape_detector.rs:84`,
`phase_disposition_with_order` at `check_trial_attestation.rs:444`; `is_blocking_at` at
`check_fkcs.rs:838`, `:83`, `:83`, `:44`, `:94`.

`phase_disposition` opens with `PHASE_ORDER.iter().position(|p| *p == phase)?`. **Today the
divergence is inert**, because each gate's own `CURRENT_PHASE = "v1_5"` is inside its own local
order. **AC2 as written deletes the const and leaves the order** — so the shared const can now walk
past the end of a local list. Proven mechanically by a scout compiling verbatim copies of
`gate_common.rs:161`, `check_escape_detector.rs:59` and the real disposition at
`gate-registry.toml:397`:

```
phase=v1_5   shared->blocking=false  local->blocking=false
phase=v2_0   shared->blocking=true   local->blocking=true
phase=v2_2   shared->blocking=true   local->blocking=false   <<< DIVERGENCE
```

At `v2_2` the local order returns `None` → `is_blocking_at` → `false`. **Six gates go silently
non-blocking at the phase they were written to block.** This is `project_gate_binding_decay`
reproduced by the story written to close it. The consolidation must take `PHASE_ORDER`,
`phase_disposition` and `is_blocking_at` **with** the const, or it is a regression.

**And there is a third phase source the epic's oracle cannot see at all:**

```
check_third_party_trial.rs:91-93   std::env::var("MAOS_SHIP_PHASE").unwrap_or_else(|_| "v1_5")
discipline.yml:2433                MAOS_SHIP_PHASE: ${{ vars.MAOS_SHIP_PHASE || 'v1_5' }}
```

A **GitHub repository variable** can move one gate's phase with no code change. It is not in
`env_contract.rs`, and it never could be: `check_env_contract.rs:120` scans only `maos_bin_dir/src`,
so an xtask `MAOS_*` var is out of scope by construction. That is why it survived eight epics.

**AC1's command — `grep -rho "const CURRENT_PHASE" xtask/src | wc -l` == 1 — is a null control for
its own story title.** A dev can print `1` and leave every claim above false. AC1 is rewritten.

---

## 3. 🔴 AC1 NAMES ONE OF D20's TWO GATES, AND D20 IS OPEN AGAINST A DIFFERENT STORY

`grep -rln BindingClass xtask/src/*.rs` → 18 files. Of the eight `CURRENT_PHASE` holders, exactly
**two** never adopted it: `check_escape_detector.rs` and `check_cohort_mesh.rs`. Decision **D20**
(`epic-14-preflight-decisions.md:86`, **OPEN**, target `20-3-gate-honesty-pass`) names both, in
those words. The epic's AC1 names only the escape detector — and `check_cohort_mesh` is the
*original* hand-rolled Story-12.1 carve-out that `gate_common.rs:151-153` was written to replace.

**AC1's tail looked like a ship-blocker and is not.** Measured:

```
$ ./target/debug/xtask check-escape-detector --json
oracle_green:false   passed:true   exit 0
reds: producer-wired-proven-red, detection-quality-live      # both seccomp legs
```

The gate is RED at HEAD and passes *only* because it is advisory. Applying the majority whole-gate
pattern `dev_enforced_red_blocks(BindingClass::Blocking, true)` would red CI on any host without
usable seccomp. But that is the wrong adoption, and `gate_common.rs:216-221` says so in the
`AdvisorySubstrate` doc comment itself:

> *"Requires a substrate CI cannot always provision (Postgres, multi-region geo, a measurement
> engagement, **a seccomp-capable kernel**). Hard-fails when the substrate IS present and the oracle
> is RED; when the substrate is ABSENT, the caller emits a WOULD-HAVE-BLOCKED banner and passes
> advisory — **never silent-green**."*

The shared module was written with this gate in mind. The correct adoption is **mixed per-leg**, and
that is **effort, not scope**. Ruling **F3**.

---

## 4. 🔴 `maos` HAS TWO `main()`s WITH DIFFERENT VERB SETS, AND THE TABLE AC3 CALLS "NEW" ALREADY EXISTS

```
main.rs:1319  #[cfg(not(feature = "network"))] fn main()   {init, run, backup, audit, install, --version|-V}
main.rs:1645  #[cfg(feature = "network")]      fn main()   {init, run, shell, audit, traceback}
```

Overlap is **three**. `backup`/`install`/`--version` are air-gap-only; `shell`/`traceback` are
network-only. And `print_air_gap_usage()` (`main.rs:1345-1360`) is already a complete usage table
with exactly the `match … _ =>` shape AC3 asks for — 300 lines above the arm that is broken. Epic 20
AC1 (`epic-20…:80`) runs an air-gap matrix leg, so **both builds ship**: "the `maos --help` verb
table" is ambiguous until the story says which.

Three more measured facts the epic missed:

- **`--version` works in air-gap and hangs in network.** `main.rs:1332-1335` → exit 0,
  `maos 0.1.0-alpha`; the network build falls into `Some(_)` at `main.rs:1738` → daemon boot →
  EXIT 124. **Epic 20 AC1 asserts `target/debug/maos --version` prints `maos 0.1.0-alpha` and cites
  it as passing** — true only for the air-gap build. `epic-20…:81`, `:101`, `:113` all say *"15-3's
  `maos --help/--version` verb table"*, while `epic-15`'s AC3 says only `--help`. **The two epic
  files disagree; `--version` is in scope** (F5).
- **A third verb space exists that is not argv at all**: `MAOS_ONE_SHOT`, dispatched
  `main.rs:4969`, **29 modes** enumerated at `main.rs:7993` (`registry-server`, `uninstall`,
  `cohort-a2a-daemon`, `acp-server`, …). Epic 16 and Epic 20 exit lines use it.
- **A new `crates/maos-bin/src/*.rs` file reds a Blocking test.**
  `cohort_daemon_smoke_13_5c.rs:862` declares `SCANNED_SOURCE_FILES: [(&str,&str); 17]` and
  `:937-941` asserts it equals the `src/*.rs` count — which is **17**. 14-2b hit this and dodged it
  by extending an existing file (recorded in `kloc.toml`'s maos-bin row).

Citation repairs: `main.rs:1653` is a **comment**; the real dispatcher `match` is
**`main.rs:1677-1746`** and the fallthrough is `Some(_)` at `:1738`. `parse_run_args` spans
**`worker_spawn.rs:44-85`**, not `:44-72`.

**`clap` costs nothing** — `cargo tree -p maos-bin -i clap --edges normal` shows **v4.6.1 already in
maos-bin's normal closure** via maos-cli / maos-shell / maos-corpus-gen / maos-spirit-cli, single
version, `deny.toml:55 multiple-versions = "deny"` unaffected. The choice is on design merit only.

**"whose completeness a maos-bin test asserts against the dispatcher's match arms" is mechanically
impossible.** A Rust test cannot enumerate match arms at runtime. Ruling **F6** picks the only shape
in which divergence is unrepresentable rather than merely tested.

---

## 5. 🔴 AC3 NAMES THE WRONG CI LIST — ENROLLING AS WRITTEN REDS CI ON THE FIRST RUN

AC3 says the gate is enrolled in *"discipline job + **`aggregate.needs`** + …"*. There are two jobs:

| Job | line | `needs:` key | entries |
|---|---|---|---|
| `v1-0-ship-gate` | `discipline.yml:3384` | **`:3386`** | 46 |
| `aggregate` | `:3510` | `:3512` | 123 — and it reaches the ship gate transitively via `- v1-0-ship-gate` at `:3626` |

`check_ship_gate_completeness::extract_ship_gate_needs` parses **only** the `v1-0-ship-gate` job. The most recent Blocking gate added — `check-cert-rotation-trigger`,
`ee3422d0` — went into `v1-0-ship-gate.needs` at `:3432` and **never touched `aggregate.needs`**.
Enrolling as the epic writes it, while also adding the name to `EXPECTED_GATES`, **reds
`check-ship-gate-completeness` immediately.** That gate is green today (40/40).

Enrollment is **self-policing in one direction only**: `EXPECTED_GATES` → `needs` and
`EXPECTED_GATES` → `[[ship_gate]]` both red if missing. Everything else fails **open** — a job not
in `EXPECTED_GATES`, an orphan `[[ship_gate]]` row, a missing coverage-matrix row, all pass silently.
And a missing `v1_5` disposition key reads as non-blocking with nothing to notice.

---

## 6. 🔴 AC3's TOKENISER CLAUSE IS A NULL CONTROL, IN ITS OWN TEXT

AC3 says the tokeniser skips *"env-var prefixes"*. Epic 20's own exit block contains:

```
MAOS_ONE_SHOT=registry-server maos &        # epic-20 L5
MAOS_ONE_SHOT=uninstall maos                # epic-20 L8 region
```

Strip the prefix and the token is a bare `maos` — which is a **legal verb-less invocation** (shell
mode, `main.rs:1739-1746`, exit 0). **The gate resolves it green while the actual command is one of
29 undeclared env modes.** The tokeniser clause written to make the gate parse is the clause that
makes it vacuous.

Four more holes in the same clause, all measured:

| Token | Where | AC3's rule |
|---|---|---|
| `break` | epic-20 L6 | **not in the skip list** |
| `mktemp` (`REG_ROOT=$(mktemp -d)`) | epic-20 L5 | survives only if `VAR=$(…)` is skipped wholesale |
| **whole-line** `#` comments (5 of them) | epic-19 L2–L6 | AC3 says only *"trailing `#` comments"* |
| `target/debug/maos run …` | epic-17 L3 | path-qualified `maos` — no rule |
| `cargo build -p maos-bin -p maos-wasm-host --features … --bin maos --bin maos-wasm-runner` | epic-17 L3 | AC3's cargo rule is only `cargo test -p X --test Y` |
| `cargo test --workspace --no-fail-fast` | epic-15 L1 | neither `-p` nor `--test` |

And the premise the clause rests on — *"a block that is valid `bash -n` never fails the gate for
shell syntax alone"* — is true and **is the wrong oracle.** Epic 16 L3 contains `<halt_id>`, which
bash parses as **two redirections**. Run verbatim against a stub it produces
`ARGV: [halt] [resolve] [hello-spirit] [--kind] [provided-context] [--text] [idiomatic]` **and
creates a file literally named `--spirit`** — the required flag is swallowed by `> --spirit`.
`bash -n` is clean throughout.

Finally, an implementation-ordering trap: epic-20 L7 is
`MAOS_SPIRIT_SIGNING_KEY=$(od … | tr …) maos-spirit publish …`. The `$( )` contains a **pipe**;
splitting on separators before matching command substitution re-heads the command (reproduced: a
naive pass reports head `tr`; the real head is `maos-spirit`).

---

## 7. 🔴 THE GATE CANNOT SEE ITS OWN CLI, AND ITS CORPUS FILTER HAS A THIRD FILE

**(a) xtask cannot enumerate its own subcommands in-process.** `struct Cli` / `enum Commands` are
**private items of the binary** (`xtask/src/main.rs`'s private `struct Cli` / `enum Commands`); `xtask/src/lib.rs` is 23 lines and
exposes nine modules, none of them the CLI. A gate module living in the lib — which it must, so
`xtask/tests/` can drive it (§8) — **cannot call `Commands::command()`.** Surface sizes:
`xtask --help` = **92** declared variants + clap's injected `help` = 93; `maosctl --help` = **23**
`Subcommand` variants (`maos-cli/src/cli.rs:39-134`, exact) **+ `help` = 24**. `maos_cli::cli::Cli`
*is* reachable as a library (`maos-cli/Cargo.toml:14-16`, `maos-cli/src/lib.rs`'s `pub mod cli`) — but **xtask does not depend
on maos-cli**, and adding the edge pulls rusqlite / tokio-postgres / ed25519-dalek into xtask's
build.

**(b) `epic-15` maps to THREE files, and status does not filter them.**

```
$ ls _bmad-output/planning-artifacts/epics/epic-15-*.md
epic-15-20-preflight-2026-09-04.md      # no exit block
epic-15-21-preflight-r2-2026-09-05.md   # no exit block
epic-15-foundations-w0.md               # the real one
```

All three parse to epic 15, which is `in-progress`. The glob catches 27 files of which **7** carry
an exit block (epics 15–21; epics 0–14 predate the format and are all `done`). The natural fix —
*skip a file with no exit block* — makes the gate **silently pass if a real epic loses its block**.
`epic-list-revised-12-epic-consensus-structure.md` matches the glob with no derivable epic number
and needs an explicit classification, not a `continue`. Every fence is a **bare ` ``` ` with no info
string**, so a `` ```bash `` locator finds nothing.

---

## 8. 🔴 BOTH NEW CONTROLS CAN SHIP UNRUN, AND ONE OF THEM CANNOT BE RUN BY CI AT ALL

**No CI job runs `cargo test -p maos-bin` unscoped.** Ten invocations in `discipline.yml`
(`:1924,1925,1927,1928,1951,1969,1970,1985,2048,2809`), every one `--test <name>`-scoped. **26 of
the 38 files in `crates/maos-bin/tests/` execute nowhere.** The workflow says it itself at
`:1902-1903`; `discipline.yml:1931` records the same for xtask. A verb-table test dropped into that
directory runs on the developer's laptop and nowhere else.

⚠ **`cargo test -p maos-bin` is RED at HEAD, and it is environmental, not D16.**

```
$ cargo test -p maos-bin
error: crates/maos-bin/tests/drain_once_audit_writer.rs:27
  "maos run: cli_wrapper command 'worker-cli-fixture' not found
   (checked daemon-sibling, deps/ parent, and $PATH)"            EXIT 101

$ cargo build -p worker --bin worker-cli-fixture   &&  cargo test -p maos-bin
41 binaries, 271 passed, 0 failed, 12 ignored                    EXIT 0
```

`worker-cli-fixture` is a `[[bin]]` of the separate `worker` package
(`spirits/worker/Cargo.toml:11-13`) and is untracked. This is a **rediscovery** — the identical trap
is j1-crosshost-2a review finding P1 (`tracker-notes-archive-2026-09-04.md:511`), fixed **per-job**
at `discipline.yml:2119-2126` rather than systemically, so it recurs on every fresh runner and every
local run that omits the pre-build. **The dev agent will otherwise misdiagnose a red suite as their
own regression.**

**Test isolation is still four ad-hoc copies, not one helper.** `cross_team_consent_13_3.rs:40`
`RestoreMaosHome` + `:583` `LIVE_LOCK`; `cross_wall_log_read_13_6d.rs:15-17` `env_lock()` + its own
`RestoreMaosHome`; `cross_team_crossing_13_6b.rs:1323` `LIVE_LOCK` + `:2761` its own restore + `:63`
`SHARE_ENV_LOCK`; `host_grants_2b.rs:32`. `grep -rn "mod common\|test_support\|env_guard"
crates/maos-bin/tests/*.rs` → **zero**. D16 is recorded **CLOSED** promising *"the lock AND the
whole-package leg both ship here"*; the whole-package leg did not ship, and
`check-decision-register` is green over it. **A new test must not add a fifth copy** — the verb-table
test must touch no process-global env, so it needs none.

---

## 9. 🔴 "xtask NET LINES ≤ 0" IS FALSE — AND THE FULL CONSOLIDATION IS WHAT MAKES IT AFFORDABLE

15-2 §7 already filed this AC as false-as-written and got `+34` at the top of a **low** estimate.
Re-derived here against four anchors, all tokei `code`, prod-only (tests split out):

```
check_decision_register.rs      604   (603 prod; tests in xtask/tests/, uncharged)
check_epic_close_coherence.rs   454   (285 prod; 169 inline, charged)
check_ship_gate_completeness.rs 411   (262 prod; 149 inline, charged)
check_cna_registration.rs       209   (128 prod;  81 inline, charged)
```

`check-exit-commands` does strictly more than any anchor — file roster + status filter + fence
extraction + a real shell tokeniser (**no `shlex` in xtask**; `grep -rn shlex xtask/` = 0) + five
resolvers. **Defensible estimate: 380–520 prod code.** The epic's §E *"+≤60"* is not survivable.

| | lines |
|---|---|
| **Frees** — `check_cna_registration.rs` module | −209 |
| **Frees** — its `main.rs` wiring (mod + variant + arm) + `EXPECTED_GATES` | −7 |
| **Frees** — the **full** phase dedup of §2 (7 `CURRENT_PHASE` + 7 `PHASE_ORDER` + 90 lines of duplicated fn bodies) | **−104** |
| **Frees** — `check_third_party_trial::current_phase()` | −3 |
| **Costs** — `use crate::gate_common::…` in the 3 gates that lack one | +3 |
| **Costs** — re-homing (b)+(c) into `check_security_md.rs` (F1) | +25 |
| **Costs** — `check_exit_commands.rs` + `main.rs` wiring + `EXPECTED_GATES` | +390 … +530 |
| **NET** | **+95 … +235** |

Against **305** of headroom that 15-1, 19 (+150), 21-2 (+150–300) and 21-4 (+150–350) also draw on.
It fits — **and only because AC2 is done properly.** AC2's minimal form frees **−4 to −7** and the
story does not fit at all. `maos-bin`: verb table + test ≈ +80–150 against **3000**. Comfortable.

⚠ If the measured gate exceeds the projection, the lawful move is a **measured grant** under the
CEILING RULE's own door, taken with the measurement attached — **not** compressing the gate until it
fits. Compressing a gate to hit a ceiling is how gates become null controls.

---

## Decisions ratified in this story (rule 7 — one decision per fork)

| # | Fork | **Ruling** | Killed alternative |
|---|---|---|---|
| **F1** | What to do about the two live `SECURITY.md` controls | **Re-home (b) and (c) into `check_security_md.rs` (≈ +25 charged lines) BEFORE deleting the 209-line module.** It already reads the same file (`:31`). Retirement then frees a net −199 and **drops nothing**. `docs/compliance/cna-registration.md` is **KEPT** (NFR-Ops-4 evidence, linked from `SECURITY.md:41,:77`); its `:9`, `:17`, `:77` are edited. | Retiring on the epic's false premise; or declaring the loss as a boundary in `RELEASE-HOLDS.md` — trading a live control for prose, in the story whose thesis is that prose is not a control |
| **F2** | The invariant-lock review `gate-registry.toml:2-3` demands | **Mint the retirement ADR in this story.** `invariant-lock` **never fires on `gate-registry.toml`** — `xtask/invariants/lock.toml` maps `I1..In → docs/invariants/*.md` only. The sentence is a prose convention with one discharging precedent: **ADR-043** (Story 8.16). Copy its pattern exactly — ratified ADR, an **item-by-item successor table**, an explicit *"no control goes dark"* claim, and a `# RETIRED <date> by Story 15-3 per ADR-nnn` comment at each removal site. Take the **next free number after 15-5's block** (15-5 mints 060–064 and lands *after* this story): **ADR-065**. | Treating the TOML comment as satisfied by prose; or waiting for 15-5, which would invert the epic's own ordering |
| **F3** | How the escape detector "uses `BindingClass`" | **Mixed per-leg, not whole-gate.** Legs 1–7 → `Blocking`; the two seccomp legs → `AdvisorySubstrate` with a **real substrate probe** — today `invoke_cargo_test_marker` (`:193-233`) collapses *"seccomp unavailable"* and *"the code regressed"* into one `green=false`, and **that collapse is the defect**. Template: `check_multi_region_slo`'s `RawLeg`/`skipped`/`errored` (`RawLeg { class, substrate_present }`, `skipped()`, `errored()`). **`check_cohort_mesh` is done too** — its hand-rolled Story-12.1 carve-out is replaced by `BindingClass::Blocking`, which is what `gate_common.rs:151-153` says the module exists for. Both halves of **D20**. | Whole-gate `Blocking` (reds CI on every seccomp-less host — measured); or leaving D20 to 20-3 while doing 90% of its work here |
| **F4** | `check_cohort_mesh.rs:53-58`'s `assert_eq!(CURRENT_PHASE, "v1_5", "Story 12.1 must not advance global phase")` | **Delete it.** Today it compares a local const to its own literal — a tautology. Repointed at `gate_common::CURRENT_PHASE` it becomes a **live runtime panic** that fires the moment the project legitimately advances the shared phase, i.e. a tripwire against a sanctioned operation, in the crate that consolidates the source. The sibling `assert!(PHASE_ORDER.contains(&"v2_2"))` becomes trivially true and goes with it. | Mechanically repointing it (AC2's literal instruction here is the wrong move) |
| **F5** | `--help` only, or `--help` **and** `--version` | **Both, and in both builds.** `epic-20…:81,:101,:113` all specify *"`maos --help/--version`"* and 20-2 AC2's homebrew formula test (`maos.rb:76`) runs `maos --version` on the **network** build, where it hangs today. `epic-15`'s AC3 is amended to match rather than epic-20 weakened (§11). | Shipping `--help` only and leaving 20-2 to discover it |
| **F6** | How the verb table's completeness is asserted | **ONE `const VERBS: &[Verb]` that BOTH `--help` renders from AND both dispatchers index** (lookup-then-dispatch). Divergence becomes **unrepresentable** rather than merely tested; the test then only checks formatting, which is honest. Ship-blocker check: `grep -c 'Some("' ` inside each `main()` must go to **0**. Per-row `builds: &[Build]` handles the two-`main()` split (§4). **Placement is F13, which reverses this row's first answer.** | (b) a source-text regex over 640 KB of `main.rs` — **already wrong at HEAD**, ten `Some("…")` literals live in unit-test modules (`:3123,:4634,:7121,:7134,:7141,:12438-12440,:13823,:13838`), and a regex that matches nothing **passes**; (c) a second xtask gate, doubling the enrollment cost |
| **F7** | What the gate does with a token it does not recognise | **FAIL LOUD.** Three buckets: `Resolvable` (must resolve), `DeclaredNonCommand` (a **closed** list), `Unrecognised` → finding. Five in-repo precedents and **zero** for silent-skip-as-policy: `check_decision_register`'s `UNQUERYABLE` clause arm and `:182-187` (*"Refusing to report green over a register this gate could not parse"*), `check_third_party_trial`'s `unknown stratum` arm, `evidence_ledger`'s `unknown ledger gate` arm, `gate_common::governed_story_keys` (*"Refusing to walk an empty set: a gate that governs nothing passes for the wrong reason (D19)"*). An open skip means one typo in a prefix constant silently empties the corpus. | An open `if !starts_with("maos") { continue; }` — the exact shape of §6's null control |
| **F8** | The chicken-and-egg: 8 tokens are owed by stories that have not run | **RESOLVE-OR-BE-NAMED.** An unresolvable token passes **iff** its own epic's provenance names it as created by `<story-key> AC<n>`, that key exists in `sprint-status.yaml`, and its status is **not `done`**. The data already exists for all 8 and every key resolves. This is a **stronger** gate than the AC as written — it is what makes rule 1 machine-checked — and it lets the gate land **Blocking** in 15-3 despite 15-4 coming after it. | Enrolling advisory now and Blocking at 20-3 — the decay pattern this project has already paid for four times; or blocking 15-3 behind 15-4 and inverting the epic's ordering |
| **F9** | How the gate resolves each binary's verbs | **Subprocess `--help` with a hard timeout, uniformly, with the binaries built in the gate's own job** (precedent: `check_cohort_mesh.rs:37-50` `build_journey_daemon()` runs `cargo build -p maos-bin`). Rule 1 means *the binary answers*, not *the source lists it*. xtask resolves its own surface via `std::env::current_exe()` — exact, no dep, no new lib export. **A timeout is mandatory**, not defensive: `maos --help` hangs at HEAD and the gate must red on a hang, not inherit it. | Adding a `maos-cli` dependency to xtask (pulls rusqlite/tokio-postgres/ed25519-dalek into xtask's build); syn-parsing `cli.rs`/`main.rs` (proves the source, not the binary); moving ~950 lines of `enum Commands` into the lib |
| **F10** | The corpus roster | **A POSITIVE roster, not a glob-and-skip.** Every non-`done` `epic-N` key must have **exactly one** file carrying `Hermetic exit command`; zero or two is a **finding**. True at HEAD (7 epics ↔ 7 files) and it handles the three `epic-15-*` files, the `epic-14-preflight-decisions.md` match, and `epic-list-revised-*` without a single silent `continue`. | Skipping files with no exit block — which passes silently the day a real epic loses its block (§7b) |
| **F11** | Where the tests live | **`xtask/tests/check_exit_commands_gate.rs`** (integration, nameable as `--test <name>`) driving a **pure `audit()` exported from `xtask/src/lib.rs`** — the content-as-argument split, whose reference is `xtask/tests/decision_register_gate.rs` module doc: *"a vector that exercised a copy of the parsing logic would prove nothing."* The maos-bin verb test goes in `crates/maos-bin/tests/` **and is enrolled by name in `discipline.yml`**. Both homes are uncharged. | `xtask/src/tests/` (uncharged but only reachable via a substring filter); in-`src` `#[cfg(test)]` (charged **and** never executed — `crates/maos-bin/src/topology.rs`'s module doc) |
| **F13** | Where `const VERBS` and the dispatch live | **A new `crates/maos-bin/src/verbs.rs`, and `SCANNED_SOURCE_FILES` goes 17 → 18 deliberately.** The first answer was "keep it in `main.rs` so the 17-file assertion does not red" — that is **routing around a tripwire whose stated purpose is to be rung**: `cohort_daemon_smoke_13_5c.rs` says *"every maos-bin src/\*.rs file must be listed so new files cannot evade this negative."* It is a doorbell, not a wall; the sanctioned move is to add the entry and decide whether the negative applies to `verbs.rs`. And the destination it was protecting is **14,120 lines / 640 KB**, already 73% of the crate. The table **and** `verbs::dispatch(&args)` move together; both `main()`s call it, so `main.rs` gets smaller for the first time in four epics. | keeping it in `main.rs` (the 14-2b dodge, which extended a *domain* file — this would have extended the composition root) |
| **F14** | Is resolve-or-be-named a control or a receipt? | **STRENGTHENED: the owning story must actually claim the token.** As first written, any token passed if a provenance bullet named a story key that exists and is not `done` — so `maos summon-dragon` + *"created by 21-2 AC4"* passes, and the gate checks that a **string exists in a YAML file**. The token must additionally appear in the named story's own file or in that epic's AC text. Falsifier: plant a provenance line naming a story whose file never mentions the token → must red. Measured in **T0**, before any code: all 8 owed tokens must still pass (7 are named in an AC by verb; `maos-registry-server` and `maos-spirit` are named in `epic-20` AC1's text) or the strengthening is unlandable, not merely strict. | the receipt form — an owner and a deadline with nothing binding them to the thing being owed |
| **F15** | An exit block is a *template*, not a command | **The gate tokenises a DECLARED template, and a placeholder must be declared.** `epic-16` L3 carries `maosctl halt resolve <halt_id> --spirit …` whose own comment says *"the harness substitutes `<halt_id>` from `halt list`"* — so the epic already knows the block is not literally runnable, while rule 1's planning-time leg says a block is saved *"only after it was dry-parsed at HEAD."* Both are true and they describe different artifacts. bash reads `<halt_id>` as **two redirections**: run verbatim it creates a file named `--spirit` and **swallows the required flag** — `bash -n` clean throughout, which is why the R2 preflight caught it in prose (`epic-15-21-preflight-r2:36`) and nobody fixed it. So: `<placeholder>` is a first-class token class the gate recognises, and **`epic-16` L3 is repaired in this commit** to the real shape (`--spirit <s> --kind provided-context --text …`). ⚠ **Consequence worth stating plainly: this gate is not green from birth.** Its first act on the live corpus is to catch a real, semantically-broken command that a human preflight had already written down and dropped — the filed-and-never-applied pattern 15-2 named. The transcript of that catch goes in the story. | treating `bash -n` as sufficient (it is exactly the oracle that blesses this defect); or silently skipping `<…>` (§6's null control again) |
| **F16** | The third occupant of the name | **Eleven gate modules emit a JSON output field literally named `"current_phase"` carrying the SHIP-ladder value** (`check_escape_detector`, `check_fkcs`, `check_cert_rotation_trigger`, `check_enterprise_identity`, `check_enterprise_pdp`, `check_rotation_real_timing`, `check_scale_churn`, `check_wasm_form_equiv`, +3 — 22 sites). So the collision is **three-way**, not two, and the machine-readable one is the one that lies. **Zero in-repo consumers** (`grep -rn current_phase .github/workflows` = 0; `corpus_types.rs`'s two fields are the *delivered* serde, different struct) → renamed to **`ship_phase`**, in place, net 0 lines, in files AC2 already opens. An unconsumed output field is the cheapest moment this rename will ever have; the day something reads it, it is a contract and costs a deprecation. Considered and refuted in the same breath: renaming the Rust symbol `CURRENT_PHASE` instead — **95 sites in `xtask/src` and 30+ documents**, including `14-0-epic-14-preflight-decisions.md`, whose ratified **D20** text quotes *"decouple blocking-disposition from `CURRENT_PHASE`"*. Renaming it strands the decision this story closes. | renaming one of three and calling the story "single phase source"; or leaving the JSON field because nothing reads it yet |
| **F12** | The `[[ship_gate]]` disposition for a W0 foundations gate | **`{ v1_0 = "advisory", v1_5 = "blocking", v2_0 = "blocking", v2_2 = "blocking" }`** — the 14-2a precedent, and `v1_5 = "blocking"` is the key that binds at today's `CURRENT_PHASE`. Architecture `15-full-spectrum-v2-2.md:130` prescribes `{…v1_5 = advisory…, v2_2 = blocking}` for *"all new gates"*; that clause governs gates whose **subject is a v2.2 capability** — its own tail (*"absent-result → BLOCK at the v2.2 ship gate"*) only parses for evidence-bearing v2.2 gates. **The reading and the precedent are both recorded** rather than one being picked silently. | Copying `:130` literally and shipping a rule-1 gate that does not bind until v2.2 — a gate that cannot red is the thing this story exists to stop |

---

## Acceptance Criteria (7)

**AC1 (command) — the phase source is single, and the oracle counts FILES, not identifiers.**
The epic's original one-line grep was a null control for this story's own title (§2). Its first
replacement was worse: `grep -rEo "fn (is_blocking_at|phase_disposition…)"` counts **identifiers**,
and `gate_common.rs` contains a *test function* named `phase_disposition_inherits_the_nearest_earlier_phase`
— so the target number was right only by coincidence and moves the day anyone renames a test. (The
"was 15" figure was also wrong; the identifier count at HEAD is **14**.) The oracle is therefore a
**file** count, which cannot be moved by a test name and which **names its offenders** instead of
printing a number a human has to interpret:

```
# (1) no file but gate_common.rs may define a phase constant or a phase function
grep -rlE "^(pub )?const (CURRENT_PHASE|PHASE_ORDER)|^(pub )?fn (is_blocking_at|phase_disposition)" \
  xtask/src/*.rs | grep -v gate_common.rs | wc -l          # 0   (7 at HEAD, all named on failure)

# (2) the third source — a GitHub repository variable — is gone
grep -rlE "MAOS_SHIP_PHASE" xtask/src .github/workflows | wc -l     # 0   (2 at HEAD)

# (3) the name means ONE thing in machine-readable output
grep -rn '"current_phase"' xtask/src/*.rs | wc -l                    # 0   (22 at HEAD, 11 modules)
```

🔴 **Proven in both directions, and the transcript goes in the story.** A green that could not have
been red is not evidence: re-introduce a local `const CURRENT_PHASE` in one gate → oracle (1) prints
`1` **and the offending path**; revert. `cargo test -p xtask` stays green throughout, and
`check-ship-gate-completeness` still reports 40/40 (41 after AC5).

**AC2 — the consolidation is complete, so it removes the decay instead of arming it.**
(a) All seven per-gate `const CURRENT_PHASE`, all seven `const PHASE_ORDER`, all five
`fn phase_disposition`, `phase_disposition_with_order` (`check_trial_attestation.rs:444`) and all
`fn is_blocking_at` copies are deleted in favour of `gate_common`'s. **Deleting the const without
the order is forbidden** and §2's three-line divergence table is reproduced in the story as the
reason. (b) `check_third_party_trial::current_phase()` (`:91-93`) and the
`MAOS_SHIP_PHASE` env plumbing (`discipline.yml:2433`) are retired into the shared const; if the
repo variable must survive as an override it becomes **one** `gate_common` accessor, not a second
source. (c) **Two consumers of the duplicates must be repaired in the same commit or the build
breaks**: `xtask/tests/fkcs_oracle.rs:1-4` imports `is_blocking_at` and `phase_disposition` from
`xtask::check_fkcs` (both `pub` at `:825,:838`), and `check_trial_attestation.rs:9` imports
`is_blocking_at` from `check_fkcs` rather than `gate_common`; `check_wasm_form_equiv.rs:361` has the
same in-file `use super::{…}`. (d) `check_cohort_mesh.rs:53-58`'s two asserts are **deleted** per F4,
with the reason in the commit. (f) **F16 — the third occupant of the name.** Eleven gate modules
emit a JSON output field named `"current_phase"` carrying the **ship-ladder** value (22 sites);
it is renamed **`ship_phase`** in place — net 0 lines, zero in-repo consumers, and the only moment
this rename is free. The Rust symbol `CURRENT_PHASE` is deliberately **not** renamed: 95 sites in
`xtask/src` and 30+ documents depend on it, including the ratified **D20** text this story closes.
Three names, three meanings, none shared. (e) `check-escape-detector` and `check-cohort-mesh` adopt
`BindingClass` per F3 — mixed per-leg, with a real seccomp substrate probe — and the escape
detector's `--json` reports `passed:false` when the substrate IS present and the oracle is RED, and
a WOULD-HAVE-BLOCKED banner with `passed:true` when it is absent. Proven-red both ways.

**AC3 — `tests/phase-config.toml` is renamed `delivered_phase`, across all 27 files.**
The two ladders are genuinely disjoint and never converted (`gate_common` uses `v1_5`;
phase-config uses `v0.1-alpha`), so this closes a documentation conflation, not a live bug — and the
blast radius is **~27 files, not the 2 the epic names**: `tests/phase-config.toml:5`,
`tests/coverage-matrix.yaml:2`, **two non-`Option` serde fields** (`corpus_types.rs:34`
`CoverageMatrixFile`, `:52` `PhaseConfig` — no `#[serde(default)]`, so a missed file is a hard parse
error), **5 fixture `phase-config.toml`**, **7 fixture `coverage-matrix.yaml`** (including
`clean-staleness` and `violation-staleness`, which have no phase-config but are parsed by
`CoverageMatrixFile`), **10 inline literals** (`xtask/src/tests/coverage_matrix_tests.rs:36,55,210`,
`xtask/src/tests/corpus_staleness_tests.rs:21,111`,
`xtask/tests/coverage_matrix_completeness_tests.rs:20,58,92`), plus `coverage_matrix.rs:98,99,101,
105-106,145` and `corpus_staleness.rs:102`. The orphan fixture
`xtask/tests/fixtures/clean-coverage-matrix-fr-complete/` (**zero consumers**) is either renamed
with the rest or deleted — stated, not left to rot. ⚠ **Do not `sed -i` repo-wide**:
`.claude/worktrees/agent-a9a6d76f71ebb443b/` holds full copies and will produce a dirty untracked
diff. The epic's *"145 coverage-matrix violations at v1.5"* reproduces exactly (measured:
`violations: 145, deferred: 16, mode: warning`, exit 0) and is recorded, not acted on.

**AC4 — `maos --help` and `maos --version` answer, from a table the dispatcher itself indexes.**
`maos --help`, `maos -h`, `maos help` and `maos --version` each exit **0**, print to **stdout**, and
return in under a second — measured against the HEAD behaviour they replace (`--help` → **EXIT 124**,
0 bytes stdout, 4 panicked workers, 2 files written). An unknown verb exits **non-zero** with the
usage table, so `maos eval …` reds cleanly instead of hanging (`epic-18…:23` depends on this).
Per F6 there is **one `const VERBS`** in `main.rs`, carrying per-row `builds`, from which **both**
`main()`s render help **and** dispatch; `grep -c 'Some("' ` inside each `main()` is **0**;
`print_air_gap_usage` (`main.rs:1345-1360`) is re-rendered from it rather than kept as a second
table. The check is inserted **between `main.rs:1645` and the unconditional banner `eprintln!` at
`:1646`**, or the banner moves behind dispatch — a `--help` that emits a scaffold banner on stderr
before its usage breaks any consumer that diffs output. The table **and** `verbs::dispatch(&args)` live in a new
`crates/maos-bin/src/verbs.rs`, and `SCANNED_SOURCE_FILES` goes **17 → 18 deliberately** (F13) —
that assertion is a doorbell, not a wall, and its own message says so; routing around it into a
14,120-line `main.rs` is the 14-2b dodge one file further in. The
`MAOS_ONE_SHOT` mode list (`main.rs:7993`, 29 modes) is exposed the same way, because AC5 must
resolve it. Completeness is asserted by `crates/maos-bin/tests/verb_table_15_3.rs` **which is
enrolled by name in `discipline.yml`** — set **equality both directions**, a non-empty assertion, an
exact expected count, and both builds (`--no-default-features` too). Proven-red twice: a verb in the
dispatcher but not the table, and a table row with no dispatch.

**AC5 — `cargo run -p xtask -- check-exit-commands` exits 0 over all seven exit blocks, and could
have exited 1.**
Corpus by **positive roster** (F10): every non-`done` `epic-N` key has **exactly one** file carrying
`Hermetic exit command`; zero or two is a finding — 7 epics ↔ 7 files at HEAD. Reuse
`xtask::sprint_status::load_sprint_status` (`xtask::sprint_status::load_sprint_status`, the single-sourced parser that
strips trailing `# …`), the `epic-N-` walk of `check_epic_close_coherence::epic_doc_pins`, the
fence tracker of `gen_abi_docs`'s `FenceTracker`, and the quote-aware split of
`check_decision_register::cells`. Tokens are classified into **three buckets** (F7) — an
unrecognised shape is a finding, never a skip — and the skip list is **closed** and covers what §6
found missing: `break`, `mktemp`, **whole-line** `#` comments, `target/debug/maos`,
`cargo build …`, and `cargo test --workspace --no-fail-fast`. **`MAOS_ONE_SHOT=<mode>` is resolved
against the mode table, not stripped** — a bare `maos` under an unrecognised one-shot mode is a
failure (§6). Command substitution and quotes are matched **before** separator splitting (epic-20 L7).
Verbs resolve by subprocess `--help` with a hard timeout (F9). An unresolvable token passes **only**
under resolve-or-be-named (F8) **as strengthened by F14** — the owning story's own file or AC text
must name the token, or the rule is a receipt rather than a control. `<placeholder>` is a
first-class token class (F15), and **`epic-16` L3 is repaired in this commit**: `<halt_id>` is two
bash **redirections**, so the block as written creates a file named `--spirit` and drops the flag
the command requires. The gate carries `blocks_read` and `tokens_resolved` counters and
**hard-fails at zero** — `findings.is_empty()` cannot distinguish "all resolved" from "the glob
matched nothing" (`gate_common::vacuous_legs`). Enrolled Blocking at **eight** sites:
module, `main.rs` mod + `Commands` variant + dispatch arm, a `discipline.yml` job with
`timeout-minutes`, **`v1-0-ship-gate.needs` (`:3386`) — NOT `aggregate.needs` (§5)**,
`gate-registry.toml` `gates` + `[[ship_gate]]` with F12's disposition,
`check_ship_gate_completeness.rs:20` `EXPECTED_GATES` (40 → 41), and a `tests/coverage-matrix.yaml`
row. Proven-red in `xtask/tests/check_exit_commands_gate.rs` against a **GREEN control block**
(`decision_register_gate`'s `GREEN_ROW`), with at minimum: a planted absent verb; an unrecognised token;
a `MAOS_ONE_SHOT=<bogus>` prefix; an unresolvable token whose provenance names a **`done`** story;
and an epic with **two** exit-block files.

**AC6 — `check-cna-registration` is retired without a control going dark.**
Order matters: **(1)** port `GPG_PLACEHOLDER`, `V1_TABLE_TOKEN` and `has_version_table_row`
(`:38-50`) into `check_security_md.rs` with their own proven-red tests (F1); **(2)** mint the
retirement ADR on the ADR-043 pattern (F2) — successor table, explicit *"no control goes dark"*,
`# RETIRED … per ADR-nnn` at each site; **(3)** delete, `EXPECTED_GATES` **last** (deleting the const
first leaves an orphan CI job invoking a removed subcommand → clap exit 2). Complete site list, **13
code/config lines across 5 files** (the epic says six *sites*; six *files* is right, six sites is
not): `check_cna_registration.rs` (whole); `main.rs:95` comment (it also covers export-control and
fuzz-targets — rewrite, don't delete), `:96`, **`:948-953`** (6 lines), `:1360`;
`gate-registry.toml:72`, `:215-217`; `check_ship_gate_completeness.rs:34` (leave the `:32` comment,
it covers four gates); `discipline.yml:2593-2605`, `:3401`, `:3453`, `:3490`;
`tests/coverage-matrix.yaml:1277` **and `:1282`** (prose in the `NFR-Ops-4` notes). Not a site:
`coverage-matrix.yaml:844`. `docs/compliance/cna-registration.md` is **kept**, `:9`/`:17`/`:77`
edited. ⚠ Removing the `gates` array entry while leaving a coverage-matrix row fails **open** today
(`mode: warning`) and becomes a red when Epic 20 AC5 flips the mode to `error` — do both.

**AC7 — the register and the ledgers are corrected rather than left holding this story's work.**
(a) **D20** (`epic-14-preflight-decisions.md:86`, OPEN, target `20-3-gate-honesty-pass`) is
re-pointed to `15-3` and **CLOSED** — both gates, per F3 — with `check-decision-register` exiting 0
and the open count down by one; the `:91` re-homing map is updated. (b) `xtask/kloc.toml`'s xtask
ledger comment is corrected: the booked *"15-3 +34 at top"* is replaced by this story's **measured**
figure, and the row's provenance line — which says *"Final ceiling = 41897 measured + 35"* under a
value of **42353** — is repaired to record 15-2's re-base. (c) The epic OLD→NEW edits of §11 land in
the same commit. (d) **Filed, not fixed, with the reason named**: the `-e tests` glob is the
load-bearing premise of every ceiling in this repo and **nothing asserts it** — a tokei major bump
would silently add ~11k charged lines in xtask alone; the version pin (`kloc_check`'s `TOKEI_VERSION`, validated
`:293`) is a pin, not a property test. Filed to `21-4` with D13(b). Likewise the **systemic**
`worker-cli-fixture` pre-build (§8), fixed per-job twice now, filed to `20-3`.

---

## Declared cut lines

**Kept out because it is SCOPE, with the reason named — not because it is effort:**

- **A shared test-isolation helper for `crates/maos-bin/tests/`.** D16's unshipped half. This story
  adds a test that touches **no** process-global env, so it neither suffers nor worsens the defect;
  building the shared guard means editing four existing files' locking, which is a suite-wide
  refactor with its own review surface. The residual is real and is **named in AC7(d)**.
- **Enrolling the other 25 unrun `crates/maos-bin/tests/` files.** This story enrolls **its own**.
  A whole-package leg is D16's promised deliverable and belongs with the story that owns test
  isolation, not with a phase-consolidation story.
- **A property test for the `-e tests` glob.** Cheap in isolation, but it is an assertion about the
  *instrument*, and 15-2 already established that instrument work lands with 21-4's unification.
  Filed there.
- **The 33 coverage-matrix violations already RED at HEAD** (advisory via `mode: warning`; 27 are
  `references unknown gate`, two of the "unknown" gates are real, three rows carry phases absent
  from `phase_order`). Unfiled before this preflight, **now filed** — but Epic 20 AC5 owns the mode
  flip and the cleanup goes with it.
- **`/tmp/maos/crl/` escaping `MAOS_HOME`** (~470 stderr lines per `maos` run, observed in §"What
  this story actually is"). A real defect, entirely unrelated to this story's subject. Filed.

**Explicitly NOT cut, having been classified as EFFORT under the operator's directive:** the full
phase dedup (§2 — and it is what funds the story, §9), the `--version` verb (F5), both halves of
D20 (F3), the `SECURITY.md` control re-home (F1), the retirement ADR (F2), and the `discipline.yml`
enrollment of the new maos-bin test (§8) — which is the difference between a control and a file.

---

## Dev notes

**Build `worker-cli-fixture` before you trust any maos-bin test result.**
`cargo build -p worker --bin worker-cli-fixture` — without it `cargo test -p maos-bin` is EXIT 101
at HEAD on a clean checkout and you will misattribute it (§8).

**The loop.** `cargo fmt --all` → `cargo run -p xtask -- kloc-check --json` from the **repo root**
(the workspace root is derived lexically from the config path, `kloc_check`'s lexical workspace-root derivation; a wrong CWD
measures a different tree) → read `per_crate`. `check-fmt` is Blocking (`discipline.yml:136-139`),
so multi-line literals must use the `\`-continuation style already stable at `kloc_check`'s `\`-continuation literals
and `gate_common`'s `\`-continuation string literals.

**Order of operations inside the commit.** AC2 first (it frees the lines AC5 spends and its
divergence fix is the correctness core) → AC4 (the verb table, which AC5's oracle needs) → AC5 →
AC6 with `EXPECTED_GATES` last → AC3's rename → AC7's prose. Measure `xtask` after AC2 and again at
the end; the delta between them is what the gate actually cost.

**There is no shared `Report`.** `gate_common` has no base struct — every gate defines its own
`Report`/`Finding` and serialises via `serde_json::json!`. Shared items you *should* use:
`emit_command` (in JSON mode workflow commands go to **stderr** so stdout stays parseable),
`BindingClass`, `dev_enforced_red_blocks`, `read_disposition`, `LegAudit`/`vacuous_legs`.
⚠ **All by SYMBOL, never by line (R6, story 14-2c).** This story's first draft carried five
`gate_common.rs` line citations inherited from a scout report and never re-run — `emit_command`
was cited at `:465-480` and is at **597**, off by 132; `read_disposition` `:191`→**193**;
`vacuous_legs` `:288-295`→**304**; `governed_story_keys` `:71-80`/`:55`→**54**. The four the author
greped personally (`:161`, `:166`, `:210`, `:229`) were correct. The rule is not "check harder" —
it is **cite the symbol**, because a line citation is a claim standing in for a control, which is
this story's own §1 diagnosis. There is **no global `--json`**;
each subcommand declares its own `#[arg(long)] json: bool` (`xtask/src/main.rs`'s `CheckRotationRealTiming` declaration (⚠ **xtask's** `main.rs`, not maos-bin's — this Dev-Notes section discusses both)).

**The canonical new-gate CI job** (`discipline.yml:402-416`, from 14-2a — copy this one, not the
pre-`timeout-minutes` CNA block you are deleting):

```yaml
  check-exit-commands:
    runs-on: ubuntu-latest
    timeout-minutes: 20
    steps:
      - uses: actions/checkout@v5
      - uses: dtolnay/rust-toolchain@v1
        with: { toolchain: stable }
      - uses: Swatinem/rust-cache@v2
        with: { key: "${{ hashFiles('**/Cargo.lock') }}" }
      - name: Build the binaries the gate resolves against
        run: cargo build -p maos-bin --bin maos && cargo build -p maos-cli
      - name: Run check-exit-commands
        run: cargo run -p xtask -- check-exit-commands --json
      - name: Run the 15-3 AC5 proven-red vectors
        run: cargo test -p xtask --test check_exit_commands_gate
      - name: Run the maos verb-table completeness control (AC4)
        run: cargo build -p worker --bin worker-cli-fixture &&
             cargo test -p maos-bin --test verb_table_15_3 -- --test-threads=1
```

`--test-threads=1` matches all ten existing `-p maos-bin` CI invocations. The comment at
`discipline.yml:376-377` is the doctrine for putting the vectors in the gate's own job: *"a gate
whose reds are never executed is the failure mode this story exists to close."*

**The lib export AC5 needs.** `xtask/src/lib.rs` (23 lines, 9 modules) gains `pub mod
check_exit_commands;` for the reason `xtask/src/lib.rs`'s module doc already records for `decision_register` — so
`xtask/tests/` can drive the **real** `audit()`, not a copy.

**Do not reinvent.** No xtask module parses markdown *fenced* blocks for content
(`check_export_control.rs` does HTML-comment fences); `FenceTracker` (`gen_abi_docs`'s `FenceTracker`) is
the only fence state machine and is currently `struct`-private. There is **no shell tokeniser and no
`shlex`** in xtask — that part is genuinely new, and it is where §6's holes live.

---

## Tasks / Subtasks

- [x] **T0 — Re-measure before writing a line.** `git log --oneline -1`; if HEAD moved off
      `55ac884f`, `cargo fmt --all`, then `cargo run -p xtask -- kloc-check --json` and re-derive §9
      in full. A grant is a global, and the xtask row is already over-subscribed by four other
      stories.
- [x] T1 — `cargo build -p worker --bin worker-cli-fixture`; confirm `cargo test -p maos-bin` is
      GREEN (271 passed) **before** touching anything, so later reds are attributable.
- [x] T2 — AC2(a): delete all 7 `CURRENT_PHASE`, all 7 `PHASE_ORDER`, all 6 `phase_disposition*`
      and all 5 `is_blocking_at` copies. Reproduce §2's divergence table first, as the proof the
      partial fix is a regression.
- [x] T3 — AC2(c): repair `xtask/tests/fkcs_oracle.rs:1-4`, `check_trial_attestation.rs:9`,
      `check_wasm_form_equiv.rs:361`. The build breaks without this.
- [x] T4 — AC2(d): delete `check_cohort_mesh.rs:53-58`'s two asserts (F4), reason in the commit.
- [x] T5 — AC2(b): retire `check_third_party_trial::current_phase()` and `discipline.yml:2433`.
- [x] T5b — AC2(f)/F16: rename the 11 modules' JSON output field `"current_phase"` → `"ship_phase"`
      (22 sites, in place, net 0 lines). Do **not** rename the Rust symbol — 95 code sites and 30+
      documents, including D20's ratified text, depend on it.
- [x] T6 — AC2(e): `BindingClass` per F3 for both gates, with a real seccomp substrate probe.
      Proven-red both ways (substrate present + RED → fail; absent → banner, never silent-green).
- [x] T7 — **Measure `xtask` here.** This is the freed budget; record it before spending.
- [x] T8 — AC1: the four oracle commands green; plant a local const, watch them red, revert. Paste
      both transcripts.
- [x] T9 — AC4/F13: `const VERBS` **and `verbs::dispatch`** in a NEW `crates/maos-bin/src/verbs.rs`;
      add its entry to `cohort_daemon_smoke_13_5c.rs` `SCANNED_SOURCE_FILES` (17 → 18) and state in
      the commit whether that test's negative applies to it. Per-row `builds`; both `main()`s render **and**
      dispatch from it; `print_air_gap_usage` re-rendered from it; banner ordering (`:1645`/`:1646`);
      `--help`/`-h`/`help`/`--version` exit 0 to stdout; unknown verb exits non-zero;
      `grep -c 'Some("' ` inside each `main()` == 0. Expose the `MAOS_ONE_SHOT` mode list.
- [x] T10 — AC4: `crates/maos-bin/tests/verb_table_15_3.rs` — equality both directions, non-empty,
      exact count, both feature sets, no process-global env. **Enroll it by name in
      `discipline.yml`** (T14). Proven-red twice.
- [x] T11 — AC5: `check_exit_commands.rs` with the `audit()`/`run()` split + `pub mod` in `lib.rs`.
      Positive roster (F10), three-bucket tokeniser (F7), closed skip list covering §6's six holes,
      `MAOS_ONE_SHOT` resolution, `$( )`-before-separators, subprocess `--help` **with a timeout**
      (F9), resolve-or-be-named (F8), `blocks_read`/`tokens_resolved` hard-fail at zero.
- [x] T12 — AC5: `xtask/tests/check_exit_commands_gate.rs` — GREEN control block first, then the
      five named red vectors. Every red measured against the control.
- [x] T12b — AC5/F15: repair `epic-16` L3 to the real `maosctl halt resolve` shape
      (`--spirit <s> --kind provided-context --text …`) and declare `<placeholder>` as a token class.
      **Capture the gate's pre-repair red on that line and paste it into the story** — it is the
      story's only proof the gate is not green from birth.
- [x] T13 — AC5: run the gate over the live corpus. Expect all 8 owed tokens to pass under F8 and
      **name them in the story's transcript** — that list is the lane's real rule-1 ledger.
- [x] T14 — AC5: enroll at all eight sites. **`v1-0-ship-gate.needs` (`:3386`), not
      `aggregate.needs`.** `check-ship-gate-completeness` → 41/41.
- [x] T15 — AC6(1): port the two `SECURITY.md` controls into `check_security_md.rs` with proven-red
      tests. Verify by reverting each and watching the named test fail.
- [x] T16 — AC6(2): mint the retirement ADR on the ADR-043 pattern; `docs/adr/index.md` updated.
- [x] T17 — AC6(3): delete at all 13 lines / 5 files, `EXPECTED_GATES` **last**; edit
      `cna-registration.md:9,:17,:77`; remove both `coverage-matrix.yaml:1277` and `:1282`.
- [x] T18 — AC3: the `delivered_phase` rename across all ~27 files including both serde fields and
      12 fixtures; decide the orphan fixture explicitly. **No repo-wide `sed -i`** (worktree).
- [x] T19 — AC7(a): re-point and CLOSE D20; `check-decision-register` exit 0, open count −1;
      `:91` map updated.
- [x] T20 — AC7(b): correct the `kloc.toml` xtask ledger and its stale provenance line with this
      story's measured figure.
- [x] T21 — AC7(c): the epic OLD→NEW edits of §11, in this commit — now **thirteen**, with
      `epic-16` L3 (F15) and the F13/F14/F16 rulings folded into the epic's 15-3 section.
- [x] T22 — AC7(d): file the three residuals with their reasons (`-e tests` property test → 21-4;
      systemic `worker-cli-fixture` pre-build → 20-3; `/tmp/maos/crl/`).
- [x] T23 — Full sweep: `cargo fmt --all --check`; `cargo test -p xtask`; `cargo test -p maos-bin`;
      `kloc-check`; `check-kernel-baseline` (24472, byte-unchanged); `check-ship-gate-completeness`;
      `check-decision-register`; `check-epic-close-coherence`; `check-exit-commands`. Record the
      final `xtask` and `maos-bin` figures against §9's projection and say whether it held.

### Review Findings

- [x] [Review][Patch] Drive the exit-block roster from authoritative sprint-status keys [xtask/src/check_exit_commands.rs:682]
- [x] [Review][Patch] Resolve complete nested and secondary CLI surfaces, failing unreadable help [xtask/src/check_exit_commands.rs:910]
- [x] [Review][Patch] Bind owed tokens to their exact provenance clause and token boundaries [xtask/src/check_exit_commands.rs:593]
- [x] [Review][Patch] Validate ship-phase overrides against the ordered shared phase ladder [xtask/src/check_third_party_trial.rs:103]
- [x] [Review][Patch] Require both escape-detector sub-runs to prove substrate absence [xtask/src/check_escape_detector.rs:503]
- [x] [Review][Patch] Make the one-shot mode table drive or mechanically match runtime dispatch [crates/maos-bin/src/main.rs:7927]
- [x] [Review][Patch] Run the verb-table control with `--no-default-features` [.github/workflows/discipline.yml:442]
- [x] [Review][Patch] Enroll the re-homed SECURITY.md falsifiers in CI [.github/workflows/discipline.yml:534]
- [x] [Review][Patch] Enforce the sub-second help and version response bound [crates/maos-bin/tests/verb_table_15_3.rs:222]
- [x] [Review][Patch] Validate declarations for quoted placeholders [xtask/src/check_exit_commands.rs:662]
- [x] [Review][Patch] Reject unterminated quotes and substitutions during tokenization [xtask/src/check_exit_commands.rs:263]
- [x] [Review][Patch] Reject exit blocks without a closing fence [xtask/src/check_exit_commands.rs:541]
- [x] [Review][Patch] Inspect child commands launched through `env` [xtask/src/check_exit_commands.rs:880]
- [x] [Review][Patch] Bind owed ownership to the exact exit line and full command path [xtask/src/check_exit_commands.rs:764]
- [x] [Review][Patch] Keep present but unimplemented secondary binary surfaces owed, never commandless-resolved [xtask/src/check_exit_commands.rs:1182]

---

## 11. Epic OLD→NEW edits (rule 9 — land in this commit)

| # | File | OLD | NEW |
|---|---|---|---|
| 1 | `epic-15…` 15-3 AC4 | "`docs/compliance/cna-registration.md` absent at HEAD → vacuous today" | the gate is **live** (`PASS (CNA doc + SECURITY.md valid)`); retirement re-homes two `SECURITY.md` controls first (F1) |
| 2 | `epic-15…` 15-3 AC4 | "retired at all **six sites**" | six **files**, **13 code/config lines**, enumerated; `EXPECTED_GATES` last |
| 3 | `epic-15…` 15-3 AC4 | "xtask net lines ≤0 … (arithmetic in 15-2's xtask row)" | measured **+95 … +235** against 305 headroom (§9); the full AC2 dedup is what funds it |
| 4 | `epic-15…` 15-3 AC4 | "invariant-lock review per `gate-registry.toml:2-3`" | `invariant-lock` never fires on that file; the ADR-043 pattern + a new ADR (F2) |
| 5 | `epic-15…` 15-3 AC3 | "`aggregate.needs`" | **`v1-0-ship-gate.needs` (`discipline.yml:3386`)** |
| 6 | `epic-15…` 15-3 AC3 | "maos-bin 17109/17109" | **17109/20109, headroom 3000** (15-2 re-based it) |
| 7 | `epic-15…` 15-3 AC3 | "a new `maos --help` verb table" | `maos --help` **and `--version`**, in **both** `main()`s, from one `const VERBS` (F5, F6) |
| 8 | `epic-15…` 15-3 AC3 | "whose completeness a maos-bin test asserts against the dispatcher's match arms" | mechanically impossible; F6's lookup-then-dispatch table, test enrolled by name in `discipline.yml` |
| 9 | `epic-15…` 15-3 AC3 | the skip list; "env-var prefixes" | closed list + the six missing shapes; **`MAOS_ONE_SHOT` resolved, not stripped** (§6); resolve-or-be-named (F8) |
| 10 | `epic-15…` 15-3 AC1/AC2 | "8 `CURRENT_PHASE`… the escape detector uses `BindingClass`" | 16 consts + 12 fn copies + `MAOS_SHIP_PHASE`; **both** D20 gates; AC1's oracle widened (§2, §3) |
| 11 | `epic-15…` 15-3 AC1 | the one-line `const CURRENT_PHASE` grep; then an identifier grep | a **file**-count oracle (7 → 0) that names its offenders — an identifier count is moved by a *test function's name* (`gate_common.rs` has `fn phase_disposition_inherits_the_nearest_earlier_phase`) |
| 12 | `epic-15…` 15-3 AC2 | two things are called "current phase" | **three** — 11 gate modules emit a JSON field `"current_phase"` carrying the ship value (22 sites, 0 consumers) → `ship_phase` (F16) |
| 13 | `epic-16…` L3 | `maosctl halt resolve <halt_id> --spirit hello-spirit …` | the real shape (`--spirit <s> --kind provided-context --text …`); `<halt_id>` is two bash **redirections** (F15). ⚠ **Deliberately NOT repaired at spec time** — 15-3 T12b repairs it *and captures the gate's pre-repair red*, which is the story's only proof the gate is not green from birth |
| 14 | `epic-19…:22` · `epic-20…:88,:94` | epic-19's provenance section is formatted unlike the other six; epic-20 prices the CNA module at `wc -l` **260** | normalise epic-19 to the `- Line N —` form so F8's allowlist is derivable; epic-20 → tokei **209** (the same unit error 15-2 filed twice) |

---

## Dev Agent Record

### Agent Model Used

`opus-5` (frontier-class allowlist {opus-4-6, opus-4-7, opus-4-8, gpt-5.5, gpt-5.6, glm-5.1,
glm-5.2, glm-5.3, opus-5, equiv}), with four parallel sub-agents on disjoint file sets for the
`maos-bin` verb table (T9/T10), the `delivered_phase` rename (T18), and the `SECURITY.md`
control port (T15).

### Debug Log References

**T0 — baseline re-measured, HEAD unmoved.** `git log --oneline -1` = `55ac884f`, matching
`baseline_commit`. `kloc-check --json` → `passed:true, alarm:false, aggregate 155593`,
`xtask 42048/42353`, `maos-bin 17109/20109`; `cargo fmt --all --check` clean; local
`tokei 14.0.0` == the CI pin. §9's baseline held; no re-derivation was needed.

**§2's divergence table, reproduced mechanically before a line was deleted** (the proof that
the partial consolidation is a regression). Verbatim copies of `gate_common`'s `PHASE_ORDER`,
`check_escape_detector`'s local order, and the REAL disposition at `gate-registry.toml`
(`{v1_0=advisory, v1_5=advisory, v2_0=blocking}`), compiled with `rustc`:

```
phase=v1_0   shared->blocking=false  local->blocking=false
phase=v1_5   shared->blocking=false  local->blocking=false
phase=v2_0   shared->blocking=true   local->blocking=true
phase=v2_2   shared->blocking=true   local->blocking=false   <<< DIVERGENCE
diverging phases: 1
```

**AC1 oracles — GREEN, and proven RED in both directions.**

```
# (1) files defining a phase constant or phase function   0   (7 at HEAD)
# (2) files carrying the retired ship-phase env token      0   (2 at HEAD)
# (3) machine-readable `"current_phase"` output sites      0   (17 at HEAD)
```

Falsifier for (1): a local `const CURRENT_PHASE` re-introduced into `check_fkcs.rs` →
oracle printed `1` **and named the offender** (`xtask/src/check_fkcs.rs`); reverted, back to 0.
This is exactly why the oracle counts FILES, not identifiers.

⚠ **Oracle (2) initially read `2`, not `0`, because of this story's own comments.** The
replacement comments in `check_third_party_trial.rs` and `discipline.yml` *named* the retired
env var, and a file-count grep cannot tell a comment from a source. Both were reworded to
refer to it indirectly; the retired name is recorded in ADR-065 instead. A worked example of
the story's own thesis: an unfoolable oracle beats nuanced prose.

**AC2(e)/D20 — proven BOTH ways on a real host.** This host's kernel HAS seccomp filtering
(`/proc/sys/kernel/seccomp/actions_avail` lists eight actions, `Seccomp_filters: 1`) yet a
real sandboxed spawn fails: `maos: seccomp apply failed` → `SKIP …: sandbox spawn refused by
host (PermissionDenied)`. **A kernel-config probe alone would therefore have marked the
substrate PRESENT and hard-failed a host that genuinely cannot sandbox** — which is why
`substrate_present` is decided by the harness's real spawn attempt and the kernel probe is
reported as context.

- Substrate genuinely absent → `passed:true, advisory:true, substrate_absent:true`, exit 0,
  and the two red legs labelled `status:"substrate-absent"` rather than anonymous reds.
- Substrate probe neutralised so the same legs read as present → `passed:false`,
  `blocking_legs:["producer-wired-proven-red","detection-quality-live"]`, **exit 1**.

Before this story BOTH states produced `passed:true`. That difference is D20.

**AC5/T12b — the gate's pre-repair red on `epic-16` L3, captured before repairing it:**

```
[placeholder-hazard] epic-16 epic-16-one-daemon-one-door-j0-w1.md:18
    `<halt_id>` is declared but written UNQUOTED — bash reads it as two redirections,
    swallowing the following flag and creating a file named after it. Quote it.
```

And the defect reproduced against a stub, which is stronger than §6 recorded: `bash -n` was
CLEAN, the run created a file named `--spirit`, **and stdout was redirected into it**. That
file's contents are the proof the flag was swallowed:

```
ARGV: [halt] [resolve] [hello-spirit] [--kind] [provided-context] [--text] [idiomatic = clippy-clean, no unwrap]
```

`--spirit` is gone and `hello-spirit` slid into the positional slot the halt id should
occupy. After quoting: the real `maosctl` reports `halt resolved <halt_id>
(provided_context)` and no stray file is created.

**T13 — the lane's rule-1 owed ledger (6 tokens, all lawful under F8+F14):**

| token | epic | owner | status | claimed in |
|---|---|---|---|---|
| `release-dry-run` | 15 | `15-4-release-repair-and-first-signed-tag` | backlog | epic-15 AC text |
| `uninstall` | 16 | `16-4-maos-uninstall-and-keyring` | backlog | epic-16 AC text |
| `eval` | 18 | `18-2-eval-halt-verb-and-corpus-numbers` | backlog | epic-18 AC text |
| `release-dry-run` | 20 | `15-4-release-repair-and-first-signed-tag` | backlog | epic-20 AC text |
| `maos-registry-server` | 20 | `15-4-release-repair-and-first-signed-tag` | backlog | epic-20 AC text |
| `maos-spirit` | 20 | `15-4-release-repair-and-first-signed-tag` | backlog | epic-20 AC text |

**AC4 measured, both builds** (`--help` stdout is byte-identical across builds):

```
maos --help     exit=0  stdout=1345  stderr=0  75ms      (HEAD: exit 124, 0 bytes, 529 stderr)
maos -h         exit=0  stdout=1345  stderr=0  74ms
maos help       exit=0  stdout=1345  stderr=0  75ms
maos --version  exit=0  stdout=16    stderr=0  77ms      (HEAD: exit 124 on the network build)
maos eval halt --class butler -> exit=1, usage table on stdout   (epic-18 AC1's dependency)
```

Ship-blocker: `Some("` inside the air-gap `main()` (39 lines) = **0**; inside the network
`main()` (6614 lines) = **0**. The 11 whole-file occurrences that remain are in test modules,
explicitly out of AC4's scope. `SCANNED_SOURCE_FILES` = **18 == 18** `src/*.rs` files.

**Two regressions found and fixed at source, not suppressed.** Retiring the ship-phase env
var broke `trial_gate_rejects_missing_producer_signed_attestation_at_v2_0` and
`trial_gate_refuses_public_dev_producer_key_at_v2_0` — six tests set that variable to reach
the v2.0 branch. Deleting the coverage was refused, and so was re-creating an ambient source.
The phase is now an explicit `--ship-phase` CLI argument defaulting to
`gate_common::CURRENT_PHASE`: it cannot be set by a GitHub repository variable without
editing a reviewed file, so AC1's oracle stays at 0 while the v2.0 vectors keep running.
`story_10_2_proven_red` → 28 passed.

### Completion Notes List

**All 7 ACs satisfied; 26/26 tasks complete. Every gate in T23's sweep is green and the
kernel is byte-unchanged.**

`cargo fmt --all --check` clean · `cargo test -p xtask` **850 passed / 0 failed** ·
`cargo test -p maos-bin` **278 passed / 0 failed** (271 at T1 + 7 new verb-table tests) ·
`kloc-check` PASSED (aggregate 156840) · `check-kernel-baseline` PASSED (24472 == 24472, and
`git diff HEAD -- crates/maos-kernel-core/src` is **empty** — ZERO kernel-Δ as declared) ·
`check-ship-gate-completeness` 40/40 · `check-decision-register` 23 rows / **12 open**
(was 13) · `check-epic-close-coherence` PASSED (21 epics) · `check-exit-commands` PASS
(7 epics, 7 blocks, 31 tokens resolved, 6 owed, 0 findings) · `check-security-md` PASS.

**Post-review false-green repair.** Owed ownership now requires the command's exact
`Line N`/`Line Nb` provenance entry and its complete ordered command path, so the leaf
`install` in `maosctl spirit install` cannot inherit the owner of an earlier
`maosctl install`. Present binaries whose `--help` fails are recorded as unavailable,
not resolved: the live `maos-registry-server --bind … --root …` line is therefore owed
to open Story 20-1. The focused gate suite is **32 passed / 0 failed**; the live gate is
7 epics / 7 blocks / 31 resolved / 6 owed / 0 findings.

⚠ **THE §9 PROJECTION WAS WRONG AND THE MEASURED FIGURE GOVERNS.** §9 projected net
**+95 … +235**; the delivered net is **+742**, taken as an operator-authorized EXACT-MEASURED
grant (`xtask` 42353 → **43095**, ZERO headroom, per the 14-2a/14-2b/14-2c and D13(a)
precedent). §9's own instruction was followed to the letter: *the lawful move is a measured
grant with the measurement attached, not compressing the gate until it fits.* Three misses,
each measured per file:

| item | §9 projected | measured |
|---|---|---|
| `check_exit_commands.rs` | +390 … +530 | **+1143** (2.2× the upper bound) |
| full AC2 phase dedup | −104 | **−85** |
| `SECURITY.md` re-home (F1) | +25 | **+134** (≈140 of it inline tests, which tokei charges) |
| F3 per-leg binding + substrate probe | not priced | **+55** |
| CNA retirement | −216 | **−216** ✓ |

`maos-bin` 17109 → **17309** (+200) against 3000 of headroom — comfortable, as projected.
The aggregate (156840) stayed under both `_aggregate_alarm` (158608) and
`_aggregate_hardfail` (170884), so the only breach was the single per-crate ceiling.

**Six premises disproved by measurement during dev, all corrected in the epic in the same
commit (rule 9) rather than silently adapted:**

1. **F16's site count.** `"current_phase"` is **17 sites**, not 22. The ELEVEN-module list
   was exact.
2. **`MAOS_ONE_SHOT` mode count.** **48 modes**, not 29 — `main.rs:7993` enumerated only a
   subset.
3. **§6's `MAOS_ONE_SHOT` exit-line claim is false for the live corpus.** `MAOS_ONE_SHOT`
   appears in **no exit block** — only in prose provenance (`epic-20:30` describes what the
   registry server *is today*, and that is not the exit line). The resolver was still built,
   because AC5 mandates it and an unrecognised env prefix must fail loud; its red vector is
   necessarily planted, which AC5 already required.
4. **AC5's eighth enrollment site does not exist.** `tests/coverage-matrix.yaml` maps
   {FR, NFR} → {corpora, gates} per NFR-Meta-3; this gate serves no FR/NFR, and measured at
   HEAD **zero** of its four sibling process gates (`check-decision-register`,
   `check-epic-close-coherence`, `check-dev-record-completeness`,
   `check-bare-review-findings`) carries a row. Attaching one to an unrelated requirement
   would be the very "claim standing in for a control" failure §1 diagnoses. Enrolled at
   **seven** real sites; the matrix violation count is unchanged at 33, and the gate does not
   validate the registry→matrix direction, so nothing fails open that was not already.
5. **AC3's inline-literal count.** **8**, not 10 — the figure contradicted its own citation
   list (3+2+3). Fixture counts 5/7 were confirmed correct; the orphan fixture
   `clean-coverage-matrix-fr-complete/` was DELETED with evidence of zero consumers
   (4 phase-config + 6 coverage-matrix fixtures remain).
6. **The owed-token count.** **6**, not 8 — `init`, `shell`, `run` and `audit` all resolve
   once AC4's verb table exists, which is what AC4 is for.

**One further instrument fact worth recording:** tokei charges Rust `///`/`//!` doc comments
as CODE (`check_decision_register.rs` = 604 code / 12 comments over 120 doc lines), so §9's
anchors already included theirs and the comparison was apples-to-apples. This is *why*
AC7(d)'s filed residual — that nothing asserts the `-e tests` glob — matters.

**Declared reductions, each named rather than silent:** property (a) of the retired CNA gate
(the CNA doc's non-emptiness) is dropped, recorded in ADR-065; the coverage-matrix row is not
created, for the reason in (4) above. Everything else in the 7 ACs shipped whole.

### File List

**New**

- `xtask/src/check_exit_commands.rs` — the AC5 gate (pure `audit()` + `run()` split, F11)
- `xtask/tests/check_exit_commands_gate.rs` — 32 control/proven-red vectors over the real `audit()`
- `crates/maos-bin/src/verbs.rs` — the single `const VERBS` + `dispatch` (F6/F13)
- `crates/maos-bin/tests/verb_table_15_3.rs` — 7 completeness tests, both feature sets
- `docs/adr/ADR-065-retire-check-cna-registration-security-md-controls-re-homed.md`

**Deleted**

- `xtask/src/check_cna_registration.rs` (retired per ADR-065, controls re-homed first)
- `xtask/tests/fixtures/clean-coverage-matrix-fr-complete/` (4 files; orphan, zero consumers)

**Modified — xtask**

- `xtask/src/check_cohort_mesh.rs`, `check_enterprise_identity.rs`, `check_enterprise_pdp.rs`,
  `check_escape_detector.rs`, `check_fkcs.rs`, `check_trial_attestation.rs`,
  `check_wasm_form_equiv.rs` — AC2(a)/(c)/(d)/(e)
- `xtask/src/check_third_party_trial.rs` — AC2(b), `--ship-phase` argument
- `xtask/src/check_cert_rotation_trigger.rs`, `check_rotation_real_timing.rs`,
  `check_scale_churn.rs`, `check_vetting_attestation.rs` — AC2(f)/F16 rename only
- `xtask/src/check_security_md.rs` — AC6(1), the two re-homed controls + 6 proven-red tests
- `xtask/src/check_ship_gate_completeness.rs` — `EXPECTED_GATES` +new, −CNA (CNA last)
- `xtask/src/corpus_types.rs`, `coverage_matrix.rs`, `corpus_staleness.rs` — AC3
- `xtask/src/lib.rs` — `pub mod check_exit_commands`
- `xtask/src/main.rs` — gate wiring, `--ship-phase`, CNA removal
- `xtask/src/tests/coverage_matrix_tests.rs`, `xtask/src/tests/corpus_staleness_tests.rs`,
  `xtask/tests/coverage_matrix_completeness_tests.rs` — AC3 literals
- `xtask/tests/fkcs_oracle.rs` — AC2(c) repointed to `gate_common`
- `xtask/tests/story_10_2_proven_red.rs` — 6 tests moved to `--ship-phase`
- `xtask/tests/fixtures/**` (12 data files) — AC3 rename
- `xtask/gate-registry.toml` — gate enrolled (F12 disposition), CNA retired
- `xtask/kloc.toml` — AC7(b) measured grant + ledger correction

**Modified — maos-bin**

- `crates/maos-bin/src/main.rs` — both `main()`s render and dispatch from `VERBS`
- `crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs` — `SCANNED_SOURCE_FILES` 17 → 18

**Modified — config, docs, planning**

- `.github/workflows/discipline.yml` — new gate job, ship-gate enrollment, CNA job removed,
  ship-phase env retired
- `tests/coverage-matrix.yaml`, `tests/phase-config.toml` — AC3 + NFR-Ops-4 row
- `docs/adr/index.md`, `docs/compliance/cna-registration.md`
- `_bmad-output/planning-artifacts/epics/epic-14-preflight-decisions.md` — D20 CLOSED + ruling
- `_bmad-output/planning-artifacts/epics/epic-15-foundations-w0.md` — §11 edits + 6 corrections
- `_bmad-output/planning-artifacts/epics/epic-16-one-daemon-one-door-j0-w1.md` — L3 repaired
- `_bmad-output/planning-artifacts/epics/epic-19-founder-loop-w4.md`,
  `epic-20-ship-it-w5.md` — §11 row 14
- `_bmad-output/implementation-artifacts/sprint-status.yaml` — status + AC7(d) residuals

### Change Log

| Date | Change |
|---|---|
| 2026-09-07 | AC2 — phase source consolidated to one: 7 `CURRENT_PHASE` + 7 `PHASE_ORDER` + 6 `phase_disposition*` + 5 `is_blocking_at` copies deleted; the ship-phase env source retired; the JSON field renamed `ship_phase` at 17 sites in 11 modules; `check_cohort_mesh`'s two tautological asserts deleted (F4). §2's v2_2 divergence reproduced mechanically first. |
| 2026-09-07 | AC2(e)/D20 — `BindingClass` adopted by both never-adopting gates: `check-cohort-mesh` whole-gate `Blocking`; `check-escape-detector` mixed per leg with a REAL substrate probe. Proven both ways (substrate absent → advisory banner, exit 0; substrate present + RED → `passed:false`, exit 1). |
| 2026-09-07 | AC1 — file-count oracles at 0/0/0 (7/2/17 at HEAD), proven red and reverted. |
| 2026-09-07 | AC4 — `const VERBS` + `verbs::dispatch` in new `crates/maos-bin/src/verbs.rs`; both `main()`s render and dispatch from it; `--help`/`-h`/`help`/`--version` exit 0 to stdout in ~75ms (was EXIT 124 with 0 bytes); unknown verb exits 1 with the table; `Some("` == 0 in both `main()` bodies; `SCANNED_SOURCE_FILES` 17 → 18 deliberately. |
| 2026-09-07 | AC5 — `check-exit-commands` created and enrolled Blocking; positive roster (F10), three-bucket tokeniser (F7), command-substitution-before-splitting, `MAOS_ONE_SHOT` resolved not stripped, resolve-or-be-named strengthened by F14, counters hard-failing at zero. PASS over 7 epics / 7 blocks / 31 tokens / 6 owed. 32 control/proven-red vectors. |
| 2026-09-07 | AC5/F15 — `epic-16` L3 repaired (placeholder quoted) after capturing the gate's pre-repair red; the bash-redirection defect reproduced against a stub. |
| 2026-09-07 | AC6 — `check-cna-registration` retired per new **ADR-065** after re-homing its two live `SECURITY.md` controls into `check-security-md` with 6 proven-red tests; `EXPECTED_GATES` last; CNA evidence doc kept and repointed. |
| 2026-09-07 | AC3 — `delivered_phase` rename across both serde fields, 2 tracked data files, 12 fixtures and 8 inline literals; orphan fixture deleted with evidence. Behaviour-neutral (byte-identical gate output). |
| 2026-09-07 | AC7 — **D20 re-pointed to 15-3 and CLOSED** on both gates with a full ruling (open count 13 → 12); `kloc.toml` xtask ledger corrected and the operator-authorized measured grant recorded (42353 → 43095, +742, EXACT MEASURED, ZERO headroom); §11's epic edits landed with 6 further measured corrections; 3 residuals filed to 20-3 / 21-4. |
| 2026-09-07 | Regression repair — the 6 v2.0-path tests that drove the retired env var moved to an explicit `--ship-phase` argument, preserving coverage without re-creating an ambient phase source. |
| 2026-09-07 | §A6 review repair — all 13 patch findings closed: sprint-status-driven roster; nested and secondary CLI help surfaces; exact provenance/token ownership; ordered ship-phase validation; conjunctive substrate-absence evidence; table-gated one-shot dispatch; both-build CI coverage; SECURITY.md falsifier enrollment; 900 ms response tests; quoted-placeholder declarations; malformed shell rejection; closing-fence enforcement; and `env` child inspection. Verified with 32 exit-command vectors, both maos feature-set controls, 11 SECURITY.md vectors, v2.0/v2.2 + invalid-phase trial vectors, the live exit-command gate (7/7 blocks, 31 resolved, 6 owed, 0 findings), and the live escape-detector gate. |
| 2026-09-07 | Post-review false-green repair — provenance ownership is keyed by exact exit-line label plus complete ordered command path; unreadable present binaries stay unavailable and must resolve as owed or finding. Added the nested `install` collision and `maos-registry-server` stub falsifiers; 32/32 focused vectors and the live 7-epic gate pass with 31 resolved, 6 explicitly owed, 0 findings. |
