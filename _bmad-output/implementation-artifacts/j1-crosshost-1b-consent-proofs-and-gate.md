---
baseline_commit: 0769869d
depends_on: j1-crosshost-1a-frame-borne-delegation (done, `6827dc87` — the wire must exist before it can be proven to refuse)
also_after: j1-crosshost-2a-signable-heterogeneous-worker (done, `0769869d`). **Not a dependency — a re-baseline.** 2a is not upstream of anything 1b builds, but it landed *after* this story reached `ready-for-dev` and it modified the same gate, the same workflow job, the same demo and three of the same ceilings. Every number and line reference below was re-derived at `0769869d` on 2026-08-16. See the "what 2a moved" block.
blocks: j1-crosshost-2b-cross-host-delegation-mechanism
split_from: j1-crosshost-1-loopback-developer-remote-delegation (SCP 2026-07-16 §4.1; split ratified by Lunarpulse 2026-08-14 at preflight)
kernel_grant: "NONE — `check-kernel-baseline` GREEN at 24472. **But do not cite `abi-diff` as evidence:** it scopes to `crates/maos-spirit-abi/Cargo.toml` ONLY (`xtask/src/abi_diff.rs:8`), so it is structurally blind to `maos-kernel-core`'s re-exported surface — that is open **FLAG-E4**. This story expects zero ABI movement anyway (`1a` owns `Mailbox::install_a2a_router`); the honest statement is 'no ABI change was made', not 'the gates confirm none'."
kloc_grant: "**NO NEW GRANT — and the walls moved since this story was written. Re-measure before you write a line.** Measured at `0769869d`: `maos-bin` **16260 / 16260 — GREEN, ZERO headroom** (D15's 16219 was superseded by 2a's review grant, `kloc.toml:264`); `maos-cli` **4642 / 4642 — ZERO** (2a, `kloc.toml:263`, not this story's crate but do not reach for it); `maos-a2a-core` **4654 / 4654 — ZERO** (D10); `xtask` **38386 / 38609 — +223 headroom**, NOT the +534 this story was drafted against. This story still adds **+0** to `maos-bin/src`; all production code lands in `crates/maos-bin/tests/` (kloc-free) and `xtask/`. **The `_aggregate_hardfail` grant is REFUSED deliberately** (filed **D17**; the breach is now **147942 / 147057 = +885**, up from +492). Your job is to keep the numbers true, not to request them — see AC3."
model: frontier-class {opus-4-8, gpt-5.5, glm-5.2, opus-5, equiv}
review: §A6 full-layer net (Blind + Edge + Acceptance + Test-Infra + runtime) — NON-DEGRADABLE (consent/A2A surface)
---

# j1-crosshost-1b — ADR-012 consent refusal proofs + `check-j1-loopback-delegation` legs

Status: **done** — §A6 review CLOSED 2026-08-16 (reviewer `zai/glm-5.2` ≠ dev `anthropic/claude-opus-5`; 4 layers Blind · Edge · Acceptance · Test-Infra + runtime, per the NON-DEGRADABLE net). 26 raw findings → 14 after dedup → 13 patches applied + verified, 1 dismissed with grounds; the sole decision resolved by party-mode consensus 8/8. Named measured `xtask` grant 38609→38655 per `kloc.toml:87`. Artifact: `### Review Findings` below. D19 remains OPEN — compliance asserted by this record, not by a gate.

`j1-crosshost-1a` is `done` (`6827dc87`). The wire this story judges exists, and the gate it adds
legs to is registered, `Blocking`, and **PASSING at HEAD** with two legs.

**Kernel-Δ: ZERO expected on both axes.** `check-kernel-baseline` is GREEN at 24472 — that one is
measured evidence. The ABI axis is **not** evidenced by a gate: `abi-diff` and
`check-abi-ratification` scope to `crates/maos-spirit-abi` only and are blind to `maos-kernel-core`'s
re-exported surface (open **FLAG-E4**, see F11). This story makes no ABI change; report that as a
fact about the diff, not as a gate result. Any kernel seam is FLAG-Winston, never an implicit re-pin.

> **What this story is.** `1a` built a wire that carries a canonical
> `development-task:write-workspace` intent and delivers. This story proves the wire **refuses** —
> with the *right* error codes, kept distinct — and adds the legs that keep it refusing.
>
> **The split hazard, and how it was closed.** This story judges machinery `1a` shipped — the shape
> that forced the 13.6 → 13.6a/13.6e and 12.4 → 12.4a/12.4b corrections. `1a` therefore landed the
> gate **skeleton** itself, so this story adds an exam to a grader that already blocks rather than
> building both. That is confirmed true at HEAD (AC2 preamble).

---

## ⚠ What `j1-crosshost-2a` moved under this story (re-baselined 2026-08-16 at `0769869d`)

**Read this before anything else.** This story reached `ready-for-dev` at `5a921c0c`. Story `2a`
then shipped at `0769869d` and it is not adjacent work — it extended *this gate*, *this workflow
job*, *this demo* and three of *these ceilings*. Ten of its effects change instructions below, and
two of them would have caused a silent failure. They are folded into the ACs; this table is so you
can see the delta in one place.

| Thing | This story was written against `5a921c0c` | TRUE at `0769869d` | Where it bites |
|---|---|---|---|
| the gate | 219 code / 353 raw, **2 legs** | **362 code / 576 raw, 5 legs** | every line number in "Gate anatomy" |
| `ENROLLED_TEST_TARGETS` + `WORKFLOW` const | did not exist | `:78` / `:72`, leg at `:469`, **job-scoped** | **AC2.5 — the leg you were told to build EXISTS. Your delta is one string.** |
| proven-red fixture trees | one `lay_green` | **TWO** (`1a:95`, `2a:105`), nine identical `write_file`s | **AC2.4 — a new governed file must be laid in BOTH or its file's baseline reds and every vector in it goes vacuous** |
| `xtask` headroom | +534 (38075/38609) | **+223** (38386/38609) | AC3.1's argument inverts — see below |
| `maos-bin` | 16178 ceiling / 16219 measured, RED +41 | **16260 / 16260 — GREEN, ZERO** | AC3.2, Dev Notes table |
| `maos-cli` | not mentioned | **4642 / 4642 — ZERO** | AC3.6 — a second wall, not yours, don't reach for it |
| `_aggregate_hardfail` | 147549 / 147057 (+492) | **147942 / 147057 (+885)** | AC3.3 table, AC3.5 |
| keys red at HEAD | "four" | **three** (aggregate · `maos-domain` · `maos-kernel-core`) | AC3.5 |
| `demo_j1.rs` | 938 code, `run_delegation_gate:763` | **1106 code, `:793`**; `unlanded_beats:815`; the ABSENT beat `:817-821` | AC2.11 |
| `demo_j1_tests.rs` beats assert | `:218`, `== 9` | **`:254`, still `== 9`** | AC2.12 |
| J1 job legs step | `discipline.yml:1819-1821` | **`:1820-1823`**; the `-p maos-bin` line is **`:1823`** | AC2.5 |
| `maos-bin/tests/` census | "24 of 28 dead in CI" | **24 of 30 dead** (6 enrolled) | AC2.5, AC2.6 |

**Two things 2a did NOT change, verified by reading, so the ACs that rest on them stand unaltered:**
`crates/maos-a2a-core/` is untouched by `0769869d` — `send_admits:633`, `accept_admits:648`,
`consent_decision:534`, `handle_intake_verified:1494`, `map_a2a_error_to_iac_bus:1671` all still
hold — and `leg_loopback_from_host_unverified` is **byte-identical**, still the null control AC2.2a
exists to repair.

---

## ⚠ Read this block before the ACs — the draft this replaces was wrong in thirteen places

Line numbers were re-derived at `5a921c0c` by reading the file, then **re-derived again at
`0769869d`** after `2a` landed. The original draft was written against `af788c3e`, `246660f9`
reformatted the tree, and `0769869d` grew the gate by 143 code lines — so **inherit no line number
from memory, from an older story, or from an earlier revision of this one.**

### The five findings that change what you build

**F1 — The gate is a STATIC SOURCE-TEXT ORACLE. It runs no tests.**
There is no `Command`, no `cargo`, no `--test` *invocation*, no test execution anywhere in
`xtask/src/check_j1_loopback_delegation.rs` (**362 tokei CODE lines at `0769869d`**, was 219). Its
**five** legs read **nine** committed files (consts at **`:49-87`**) and match structural needles
against them. The behavioural `cargo test` commands live in **CI**, at
**`.github/workflows/discipline.yml:1815-1823`** — the `-p maos-bin` delegation line is **`:1823`**.
*Consequence:* "add consent legs to the gate" means **add source-structural checks**, plus wire the
behavioural test file into the gate's CI job. It does **not** mean make the gate shell out.
*Refinement from `2a`:* one governed file is now the workflow itself (`WORKFLOW`, `:72`) — the gate
reads YAML as text. That is still static and still safe; it is what makes AC2.5's falsifier fire.

**F2 — There is NO vacuous-green guard in this gate — and no shared one to reach for.**
The previous draft told you to copy `check_vetting_attestation.rs:224-235`. That guard is in a
*different* gate. This gate's entire aggregation is `let oracle_green = findings.is_empty();`
(**`:529`**, was `:306`) — a leg that reads nothing and pushes nothing is indistinguishable from a
leg that passed. There is no per-leg `ran`/`passed`/`failed` record to guard.
**The evidence for this was RE-TAKEN 2026-08-16, and the way it was originally taken was wrong.**
The "16 of 68 gates carry a vacuity guard" figure came from `grep -l 'vacuous\|vacuity'` — a text
match that counts **doc comments**. Re-run at `0769869d` it returns **19 of 68**, and one of the
three new hits is *this gate*, which acquired the word (not the mechanism) when `2a` wrote
"vacuum"/"vacuous" into its leg documentation. **A census that counts prose is a claim standing in
for a control** — do not quote either number as evidence.
**What IS evidence, verified by reading `xtask/src/gate_common.rs` function by function
(233 CODE lines):** it exports `phase_disposition` `:39`, `is_blocking_at` `:52`,
`read_disposition` `:61`, `dev_enforced_red_blocks` `:97`, the `EvidenceState`/`EvidenceVerdict`
projection `:127-213`, `validate_dates` `:362` and `emit_command` `:389`. **There is no `ran`, no
`checks`, and no per-leg outcome record anywhere in it.** That is the finding. AC2.2 lands the
~20-line primitive there and makes this gate its first consumer; migrating other gates is 14-6's,
not yours.

**F3 — SHIP-BLOCKER: a `cargo`-invoking leg silently destroys the proven-red harness.**
`xtask/tests/j1_crosshost_1a_proven_red.rs` runs the xtask **binary**
(`env!("CARGO_BIN_EXE_xtask")`) with `.current_dir(dir.path())` where `dir` is a **tempdir
containing NINE stub source files and no `Cargo.toml`**. `check_vetting_attestation::invoke_leg`
(`:59-66`) builds `Command::new("cargo")` and **never sets `current_dir`**, so it would inherit that
tempdir. If you copy that template: `baseline_fixture_tree_is_green` (**`:157`**) goes red, and all
planted vectors keep "passing" **for the wrong reason** — the gate is red no matter what is planted.
The suite would still report green in CI while proving nothing. **Every new leg stays root-relative
and source-static, honouring `run_with_root(json, root)` (**`:519`**).**

**F3b — NEW, SAME CLASS, INTRODUCED BY `2a`: there are now TWO fixture trees, and a missing
governed file reds BOTH.**
*(Found at the 2026-08-16 re-baseline round-table. This is the one instruction below that would have
failed silently.)*
`read()` (`:111-128`) pushes a `Finding` when a governed file cannot be opened — so **any file the
gate reads that the fixture tree does not lay makes the gate red on the fixture tree**, which is
precisely F3's failure mode arriving through a different door. At `5a921c0c` there was one
`lay_green`. At `0769869d` there are **two, duplicated verbatim**:
`xtask/tests/j1_crosshost_1a_proven_red.rs:95` and `xtask/tests/j1_crosshost_2a_proven_red.rs:105`,
each laying the same nine files (`TOPOLOGY`, `DELEGATION_RS`, `MAILBOX_RS`, `MAIN_RS`,
`ORCHESTRATOR_RS`, `A2A_ROUTER_RS`, `WORKER_CLI_RS`, `BIN_LIB_RS`, `WORKFLOW`).
**Your consent leg reads a TENTH file** (`crates/maos-bin/tests/consent_refusal_1b.rs` — AC2.1), so
**both** `lay_green` functions and **both** const blocks must learn it. Update one and the other
file's `baseline_fixture_tree_is_green` reds, at which point every planted vector in that file
passes for the wrong reason and CI still reports green. Both files are under `xtask/tests/` and cost
**zero kloc**, so there is no budget reason to skip either — only the reason that the duplication is
invisible until you look. See AC2.4 and Trap 22.

**F4 — The story's headline assertion is UNREACHABLE through the production `DelegationLeg`.**
`DelegationLeg::install(mailbox, intent)` (`crates/maos-bin/src/delegation.rs:102-131`) builds *both*
endpoints from the **same single intent**:
`LoopbackEndpoint::sender_of(TO_HOST, 7451, intent)` and `acceptor_of(FROM_HOST, 7452, intent)`.
So **any** intent mismatch is caught by `send_admits` (`router.rs:749`) *before the frame reaches the
wire*, yielding `A2AError::IntentDenied{direction: Send}` — **not** `-32001` / `IntentDeniedAtPeer`.
To produce `-32001` the intent must be **in the destination's `send_allowlist` but absent from the
source's `accept_allowlist`**. `LoopbackEndpoint`'s four fields are all `pub`, so you build that
asymmetry directly — no production change needed. See AC1.2.

**F5 — The two deny codes are CONFLATED above the router, so the non-conflation assertion must be
made at the router seam.**
`map_a2a_error_to_iac_bus` (`crates/maos-a2a-core/src/router.rs:1671-1783`) loses the vocabulary,
though **not uniformly — be precise about which half** (measured 2026-08-15 for D18):
- **The `-32001` half survives, partially.** `IntentDenied{Send}` (`:1673-1683`) and
  `IntentDeniedAtPeer` (`:1684-1690`) both produce `CrossHostIntentDenied`, but with
  `direction: Send` vs `Accept`, so the two seams *are* still tellable apart by field. The defect
  there is that `IntentDeniedAtPeer` stuffs the NACK **message** into a field named `intent`.
- **The `-32009` half is destroyed.** Both `ConsentUnclassified` variants collapse into stringly
  `IacBusError::CrossHostRouteFailure` (`:1773-1782`), discarding the typed `UnclassifiedReason`
  (`Absent`/`NonCanonical`/`Oversized`) and the direction outright.
`DelegationLeg::delegate` (`delegation.rs:149-171`) then stringifies even that into a `String`.
*Consequence, unchanged and load-bearing:* a typed `-32001`-vs-`-32009` assertion is **impossible
above `A2ARouterCore`** — one side keeps a variant, the other becomes a sentence, so there is nothing
to compare. Assert at the router seam. The lossy mapping is itself a finding — this story **files it as D18** with an
owner and a deadline (AC1.5b), rather than addressing it to `j1-crosshost-2`, which has no story
file to receive it. It does not fix it.

### Eight corrections you would otherwise carry forward as facts

**F6 — Envelope expiry is NOT a gap. The previous AC1.7 rested on a dead premise.**
`ConsentEnvelope::with_fine_grained_intent` (`crates/maos-domain/src/frame.rs:447-465`) does build
`timestamp_ns: 0, valid_until_ns: None` — but `prepare_outbound` **stamps a TTL** on any envelope
carrying `None` (`router.rs:866-871`, Story 8.9/G10: *"expiry was dead code on every real frame"*),
and `handle_intake_inner` **enforces it** (`router.rs:1208-1222`) with a **third** code,
`CODE_CONSENT_EXPIRED` (`-32003`). There is nothing to "either set or record". What there IS: a
third code that must also stay distinct. See AC1.4.

**F7 — Both `smoke-*` precedents are `#[cfg(feature = "network")]` binary-only arms with ZERO CI
invocation.** *(All four anchors re-derived at `0769869d` — `2a` grew `main.rs`, so every number in
the original text shifted by ~+106.)* `smoke_a2a_consent_vocab_8_7` is **`main.rs:11069`**
(dispatch **`:8277`**); `smoke_a2a_fail_closed_8_8` is **`:11264`** (dispatch **`:8283`**). Neither is
`cargo test`-reachable and no workflow invokes either. **Copying "the smoke-8-7 shape" as a smoke
reproduces a null control.** Copy its *assertions* into a real integration test.
Also: the previous draft's "positive control at `main.rs:11121-11146`" pointed **inside the deny
arm**. The real positive control is the `// ── (1) fine-grained ADMITTED frame` block at
**`:11175-11195`**. And `smoke-8-8`'s `-32001`-is-a-failure assertion — *"got -32001
(classified-denied) — conflation defect"* — is at **`:11403`**.

**F8 — The budget block was stale by a full grant, and then it went stale AGAIN.**
Ceiling is **38609** (`kloc.toml:203`, raised by `j1-demo-one-command-scene` — the **eighth**
consecutive `xtask` re-base). The original correction here read "measured 38075, headroom +534",
which killed the "budget is the reason for the split" argument. **Re-measured at `0769869d`:
38386 / 38609 = +223.** `2a` spent 311 of that slack (the three new gate legs, the argv-group
seam and the demo controls).
*The direction of the argument inverts.* "+534, ~2.4× the whole gate module" is now **+223 against a
362-line gate module — 0.6×**. There is still enough for this story's estimated 110-145 `xtask/src`
lines, but the margin is one careless in-`src` test module wide. AC2.3's "prefer `xtask/tests/`" stops
being advice and becomes the plan. Conversely the story understated the reds: at `5a921c0c` it was
four breaches, and at `0769869d` it is **three** — `maos-bin` went green under `2a`'s grant while the
aggregate grew from +492 to **+885** (AC3).

**F9 — `xtask/src/lib.rs` is not a seventh enrollment surface.** It exports 8 modules and this gate
is correctly absent: the proven-red suite drives the **binary**, not the library. There are **six**
real surfaces, all correct at HEAD. Adding a `pub mod` would grow the xtask public surface for no
control.

**F10 — `deferred-work.md` contains ZERO rows naming this story.** A case-insensitive sweep of all
807 lines finds no `j1-crosshost-1`, `-1a`, `-1b`, or `j1-demo` owner. `check-dev-record-completeness`
is GREEN (`violation_count: 0` over 24 owner assertions). The stale-owner-on-close hazard is
**unfounded here** — do not plan around it.

**F11 — The ABI gates cannot see the surface this lane moves (FLAG-E4).** `abi-diff` and
`check-abi-ratification` scope to `crates/maos-spirit-abi` **only** (`xtask/src/abi_diff.rs:8`
`MANIFEST`). `1a` recorded this after discovering *"**None can see the new `Mailbox` method**"*.
`maos-kernel-core/src/iac.rs:13` is `pub use maos_iac::*`, so kernel-core's public API can grow at a
flat `src_lines` with every gate green. **Their passing is not evidence.** This story makes no ABI
change; say that, and do not launder it through a blind gate.

**F12 — `cargo test -p maos-bin` is RED at HEAD under default parallel flags (D16).** A suite-wide
test-isolation defect: `std::env::set_var("MAOS_HOME", …)` is process-global and locking is
inconsistent across `cross_team_consent_13_3.rs`, `cross_team_crossing_13_6b.rs` and
`cross_wall_log_read_13_6d.rs`. `cross_wall_recall_live_path_uses_verified_state_and_home_team`
fails 5/5 parallel, passes 3/3 with `--test-threads=1`. **This will bite you the moment you run the
crate's suite.** Run your file scoped (`--test consent_refusal_1b`) and do not "fix" the flake — D16
routes the whole-suite decision to 14-0, and `--test-threads=1` is recorded as a masking workaround,
never a resolution.

**F13 — Line numbers and two structural claims corrected.** `accept_admits` is `router.rs:648-665`
(not `:650`) — **re-verified unchanged at `0769869d`**. The `story_10_5_proven_red` idiom is
**`discipline.yml:2620`** (was `:2598-2599`; not `:2567`, which is a Windows-sandbox comment).
`cargo test -p maos-journey-test --locked` is **`:1922`** (was `:1901`, never `:1869`).
The "documented at
`main.rs:10860-10864`" citation for allowlist keying is wrong — that is `smoke-6-3`; the real
documentation is `crates/maos-a2a/src/pairing.rs:9-17` and `crates/maos-bin/src/delegation.rs:58-65`.
Two claims about *shape*, not just position, were also wrong:
- The demo suite asserts **`beats.len() == 9`** — still 9 at `0769869d`, but now at
  `xtask/src/tests/demo_j1_tests.rs:254` (was `:218`), not 7 and not `:199`.
- `1a`'s boundary leg is **~43 lines (`:300-342` at `0769869d`, was `:266-293`) that push a
  `Finding` and RED the gate when the boundary moves** — not "a 4-line leg that merely records". Do
  not describe it as passive; it has teeth. It is **byte-identical at `0769869d`** — `2a` did not
  touch it — so AC2.2a's repair applies exactly as written.

### Four findings added at the 2026-08-16 re-baseline

**F14 — AC2.5's gate leg ALREADY EXISTS. Your delta is one string, and that is the whole problem.**
`2a` built `leg_completion_vectors_enrolled` (`:469-514`) against a new `WORKFLOW` const (`:72`),
and its own review (`2a`-P8) scoped it to the `check-j1-loopback-delegation` **job block** because a
workflow-wide scan stayed green on a `--test` line that lived in a *non-blocking* job. The mechanism
is correct and better than what this story specified. But the enrolled set is a hand-maintained
const — `ENROLLED_TEST_TARGETS: &[&str] = &["worker_completion_2a", "worker_manifests_2a"]` (`:78`).
**A test file that nobody adds to that list is dead in CI and the gate is green** — which is
`smoke_cli_wrapper_8_12`'s exact failure at a new address. See AC2.5 for the resolved shape.

**F15 — `D18` reads RESOLVED, not OPEN, and its deadline was re-pinned to a different story.**
This story instructs the dev three times (AC1.5b, AC4.3, T3) to *"confirm D18 still reads OPEN; do
not close it."* It was **RESOLVED 2026-08-15** — the zero-headroom paradox died on measurement (the
`maos-a2a-core` cost is ≈ zero net lines; the new variant rides `maos-domain` under **D14**, owner
14-7), and the deadline was re-pinned from *"before `j1-crosshost-2` writes its first line"* to
**"before `j1-crosshost-2b` writes its first line."** Following this story as originally written, a
dev would either re-open a resolved decision or file a contradiction against the register.
**Also note what did NOT change:** D18 is resolved as a *decision*, not as *code* — the conflation
in `map_a2a_error_to_iac_bus` is still live at HEAD, so AC1's "assert at the router seam" is
unaffected. Only the bookkeeping instruction was wrong.

**F16 — `j1-crosshost-2` no longer exists, and this story's proudest sentence now applies to itself.**
AC1.5 and AC4.2 tell the dev to write two non-coverage statements into the **`j1-crosshost-2`
sprint-status row**, and the story argues at length that D18 must not be *"deferred against
`j1-crosshost-2`, which has a sprint-status row and no story file — you cannot defer into a document
that does not exist."* Rung 2 was split (ratified 2026-08-15) into **2a** (`done`), **2b** and
**2c**. The tracker's J1 keys at `0769869d` are `j1-crosshost-1a`, `-1b`, `-2a`, `-2b`, `-2c`.
**There is no `j1-crosshost-2` row.** The residual was true when written and expired when the world
moved — a distinct failure from "a residual stale at birth" (13.6), because checking harder at
authoring time would not have caught it. Retarget to **2b**: its story file exists, it is where
cross-host peer authentication first becomes operator-visible, and D18's own re-pin already names it.

**F17 — D19's deadline, read literally, blocks this story; read correctly, it prices it.**
D19 (seven blocking gates blind to `j1-*` filenames) was filed **2026-08-16 by `2a`'s round-table**,
with the deadline *"before the next `j1-*` story leaves `ready-for-dev`"* — and `j1-crosshost-1b` is
the only `ready-for-dev` `j1-*` story, filed one day after it got there. Owner **14-0** does not yet
exist as a story. Holding the lane's only unblocked story for an instrument story nobody has written
is not a control, it is an outage; but waving it through is the fourth consecutive disclosure, and
disclosure stopped being a disposition at D19's filing. **Resolved at AC4.1.**

### What is already true — verify, do not rebuild

*(Re-verified by execution at `0769869d`, 2026-08-16.)*

| Claim | State at HEAD |
|---|---|
| Gate registered, `Blocking`, hard-blocks regardless of `CURRENT_PHASE` | TRUE — `dev_enforced_red_blocks(BindingClass::Blocking, true)`; `gate_common.rs:97` returns `true` unconditionally |
| Gate PASSES at HEAD | TRUE — exit 0, `"passed":true,"oracle_green":true,"binding":"Blocking"`, **five** legs: `frame-borne-route-intact` · `loopback-from-host-unverified` · `completion-oracle-per-adapter` · `worker-cli-under-library` · `completion-vectors-enrolled` |
| Proven-red runs as its OWN CI steps | TRUE — **two** now: `1a` at `discipline.yml:1817` (11 tests) and `2a` at `:1819` (15 tests), both `--test-threads=1` |
| `pub mod delegation` is reachable from `crates/maos-bin/tests/` | TRUE — `lib.rs` `#[cfg(feature="network")] pub mod delegation`, and `default = ["network"]` (`Cargo.toml:14-15`). AC1's whole premise; verified, not assumed — this is the trap `2a` hit with `worker_cli` |
| `LoopbackEndpoint`'s four fields are `pub` | TRUE — `crates/maos-a2a/src/pairing.rs:36-40`; `sender_of:45`, `acceptor_of:56`, `paired_loopback_router:87`. AC1.2's asymmetric build is legal |
| `MAX_CANONICAL_INTENT_LEN` = 128 | TRUE — `maos-domain/src/invariants/i8.rs:44`, enforced `:87` |
| All enrollment surfaces correct | TRUE — six surfaces (AC2.8). `gate-registry.toml:100` flat list and `:280` `[[ship_gate]]` `name =`; `env_contract.rs:420`/`:425`; `coverage-matrix.yaml:472-473` |
| `EXPECTED_GATES` = 37, `check_*.rs` = 68 | TRUE — `expected 37 / found 43 / missing []`. The ratio **asserts nothing**; see Trap 12 |
| A CI positive control already exists | TRUE — `journey_j1.rs:107` asserts `delegation_routed.intent`, run under `-p maos-journey-test` |
| `smoke_cli_wrapper_8_12` null control | **TRUE and STILL OPEN at `0769869d`** — zero CI invocation. `2a` enrolled two *other* files and left this one dead. AC2.6 stands |
| `ledger_leg_names()` has no derivation test | **TRUE and now three times worse** — it hand-lists **five** legs (`:91-100`) against five invoked, and `2a` added three of them by hand with no test. This gate has **no `#[cfg(test)]` module at all**. AC2.3 stands |
| `gate_common.rs` has no vacuity primitive | TRUE — verified by reading all seven exported fns; no `ran`, no `checks`, no per-leg record |

---

## Story

**As** Lunarpulse, relying on the founder loop's delegation path,
**I want** the loopback A2A wire to provably refuse a disallowed intent with the correct, unconflated
error code, guarded by a blocking gate whose legs cannot pass vacuously,
**so that** the `developer-remote` leg cannot silently regress into local delivery or into a
fail-open admit, and rung 2 inherits a `PROVEN_BLOCKING` foundation rather than a claim.

---

## Acceptance Criteria (4)

### AC1 — Refusal proofs, asserted where the codes are actually distinguishable

Lands as **one new integration test file**, `crates/maos-bin/tests/consent_refusal_1b.rs`.
*Why there:* it is the only location that can drive the **real production peer configuration** —
`maos_a2a::pairing::{paired_loopback_router, LoopbackEndpoint}` plus the
`maos_bin::delegation::{FROM_HOST, TO_HOST, RECIPIENT_SPIRIT, FROM_SPIRIT}` constants — instead of
hand-copied identity strings that drift. `crates/*/tests/` costs **zero kloc** (verified: the
measured set contains 0 files under `/tests/`), and `maos-bin`'s `network` feature is `default`, so
it compiles under a bare `cargo test`.

1. **Positive control, in the same test binary as the negatives.** An allowlisted
   `development-task:write-workspace` frame is admitted and its delivered `intent_class` asserted
   equal. Without a working positive in the same binary, the negatives can pass vacuously.
   *(The end-to-end production positive already exists and runs in CI —
   `crates/maos-journey-test/tests/journey_j1.rs:107` and
   `crates/maos-bin/tests/delegation_leg_1a.rs:75-87`. Do not rebuild either; this is the local
   control for this file.)*
2. **`-32001` / `IntentDeniedAtPeer`, via a deliberately ASYMMETRIC pairing.** Per F4 this is the
   only way to reach the code. Build endpoints directly (all `LoopbackEndpoint` fields are `pub`):
   the destination's `send_allowlist` carries the disallowed intent, the source's `accept_allowlist`
   does **not**. Assert `A2AError::IntentDeniedAtPeer { peer, message }` where `peer == TO_HOST`, and
   that `message` names **`founder-loop-host`** — the SOURCE host, produced at `router.rs:1331-1336`.
   That tail is the source-keying asymmetry made observable; naming it is the assertion.
   **Both `peer_id` strings must appear literally in the test.** An AC phrased *"Host B's
   accept_allowlist admits X"* would be wrong on loopback.
3. **`-32009` / `ConsentUnclassified` at BOTH seams, with the reason typed.** Send seam:
   `A2AError::ConsentUnclassified { direction: Send, reason }` (`router.rs:696-701`). Accept seam:
   the `-32009` NACK from `handle_intake` (`router.rs:1235-1244`), which must be driven directly
   because `prepare_outbound` denies first. Cover **every reachable** `UnclassifiedReason` — the
   architecture names four triggers and the enum ships three:
   `Absent` (no `consent_envelope` / no `intent_class`), `NonCanonical`, `Oversized`
   (`> MAX_CANONICAL_INTENT_LEN` = 128, `maos-domain/src/invariants/i8.rs:44`). Assert the NACK's
   `data.reason` deserializes to the expected variant — a numeric-only assertion makes the deny
   illegible, which is the defect `fail_closed_8_8.rs:128-135` exists to prevent.
4. **Non-conflation, asserted in BOTH directions, across all THREE codes.** The `-32001` leg must
   reject `-32009`, **and** the `-32009` leg must reject `-32001` (the both-ways shape of
   `fail_closed_8_8.rs:216-244` — `classified_not_allowlisted_is_minus_32001_not_minus_32009`; the `-32001`-is-a-failure arm of `main.rs:11403`). Per F6 a
   third code is live — `CODE_CONSENT_EXPIRED` (`-32003`) — so also assert an expired envelope
   yields `-32003` and **not** either deny code. Three codes, three meanings, no collapse.
5. **STATED NON-COVERAGE — two gaps, both written into the record and into `j1-crosshost-2`.**
   (a) **Rung 1 does not exercise peer authentication.** On the TCP path `handle_intake_verified`
   binds `frame.from.host_id` to the TLS-verified peer (`router.rs:1494-1521`). On loopback there is
   nothing to bind it to — `router.rs:1477-1479` says so outright (*"no wire identity to bind"*), and
   `LoopbackA2ARouter` calls `handle_intake` directly (`crates/maos-a2a/src/adapter.rs:82`, `:97`).
   **The field that selects which `accept_allowlist` applies is written by the sender and never
   verified: a frame picks its own judge.** Survivable in-process; **not** acceptable as the
   inherited claim that rung 1 "proves the wire so rung 2 only adds network." Every refusal proved
   above is one string assignment from selecting a different allowlist.
   (b) **NEW — the production error path conflates these codes** (F5). Refusals are proven at the
   router seam; a cross-host operator consuming `IacBusError` cannot today tell `-32001` from
   `-32009`, and cannot see an unclassified reason at all. One of those is policy working; the other
   is policy being unreadable. **Do not fix it here** — it is a `maos-a2a-core` surface at zero kloc
   headroom, and widening it is a scoped decision, not a side effect.
   **D18 is FILED and RESOLVED** (2026-08-15): the fix costs `maos-a2a-core` ≈ zero net lines (two
   5-line `format!` arms become typed constructions) with ~+6 in `maos-domain` riding on D14, so
   D10's zero-headroom wall never applied; owner John + Vex at 14-4, deadline *"before
   `j1-crosshost-2b` writes its first line"*.
   **Your instruction is: VERIFY IT STILL READS RESOLVED. Do NOT re-file it, do NOT re-open it, and
   do NOT expect it to read OPEN.** *(Corrected 2026-08-16 — F15. An earlier revision of this AC and
   of AC4.3/T3 told you to confirm D18 reads OPEN. That was true when written and stopped being true
   on 2026-08-15.)* Resolved-as-a-decision is not resolved-as-code: **the conflation is still live at
   HEAD**, which is exactly why AC1.4 asserts at the router seam and not above it. If you find the
   typed variant has landed in `maos-domain` before you start, AC1.4 still asserts at the router seam
   — do not widen the assertion to the `IacBusError` surface on the strength of a decision row.
   **On the deferral target:** the original text warned against filing "against rung 2" because
   `j1-crosshost-2` had a row and no story file. That warning is now moot in the worst way — **there
   is no `j1-crosshost-2` row either** (F16). Rung 2 is `2a` (done) / `2b` / `2c`. Everything this AC
   hands forward goes to **`j1-crosshost-2b`**, which has a story file. Scout the story you defer
   *into*, and check it still exists on the day you defer.

### AC2 — Gate legs on a static oracle, with the vacuity hole closed

1. **Add the consent leg(s) to `xtask/src/check_j1_loopback_delegation.rs` as source-structural
   checks.** Per F1/F3: no `Command`, no `cargo`, root-relative only, honouring
   `run_with_root(json, root)` (**`:519`**). The leg asserts the AC1 proofs **exist and are correctly
   shaped** — i.e. that `consent_refusal_1b.rs` still asserts `IntentDeniedAtPeer`, still asserts
   both `-32009` seams, and still contains the both-ways non-conflation assertion. Deleting or
   weakening an assertion must RED the gate.
   **This makes `crates/maos-bin/tests/consent_refusal_1b.rs` the gate's TENTH governed file. Read
   F3b before you add the const** — both proven-red fixture trees must lay it or one of them reds and
   its vectors go vacuous.
   **Use the composed idiom `structural(production_before_tests(src))` — the exact spelling at
   **`:171`** (and again at `:348`) — for every multi-token needle. Never bare `contains_live()`
   (**`:138-140`**).** Both halves are load-bearing and each was learned from a live escape:
   - `contains_live` matches raw line content and is layout-sensitive. That is what made `cargo fmt`
     and this gate **mutually exclusive** at `246660f9`, whose commit message ends: *"j1-crosshost-1b
     should reuse the same normalization for its refusal legs."*
   - `structural()` alone normalizes the **whole file**, so at `5a921c0c` the demo's review found the
     repair was itself bypassable: relocating the production fail-closed closure into a
     `#[cfg(test)]` module kept the `Blocking` leg **green**. `production_before_tests` (`:108`) is
     the fix, and it is now pinned by an **11th** planted vector.
   Your consent legs must not be satisfiable by an assertion that lives in a test module.
   **One honest caveat about the idiom on your own file:** `production_before_tests` (`:142-145`) is
   `src.split_once("\n#[cfg(test)]")`. An integration test under `crates/*/tests/` has no
   `#[cfg(test)]` attribute — the whole file is the test — so on `consent_refusal_1b.rs` the call is
   a **no-op**. Use it anyway (uniformity, and it is correct on every *source* file the leg touches),
   but do not believe it is protecting you there. What protects you there is AC2.5: the gate can only
   check that the assertion is *written*; only CI can check that it *runs*.
   **Add sites in this order, re-derived at `0769869d`:** const near `:49-87` → `fn leg_*` after the
   last leg (`:514`) → call in `run_with_root` (`:519-528`) → name in `ledger_leg_names()` (`:91-100`)
   → published boolean in the JSON (`:536-548`) → the `## Legs` module doc at the head of the file.
   **Seven sites now, not six** — `2a` added `ENROLLED_TEST_TARGETS` (`:78`); see AC2.5.
2a. **FIRST — repair `1a`'s boundary leg. It is a null control that can never fire.**
   *(Found by the `j1-crosshost-2` preflight, 2026-08-15. This is the vacuity class AC2.2 exists to
   close, sitting inside the very gate you are extending — fix it before adding legs beside it.)*
   *(Re-verified at `0769869d`: the leg is **byte-identical** — `2a` did not touch it — but it moved
   to **`:300-342`**. Everything below applies exactly as written.)*
   `leg_loopback_from_host_unverified` (**`:300-342`**) computes
   `unverified = contains("frame.from.host_id") && contains("pub async fn handle_intake_verified")`
   over **one shared file**, `crates/maos-a2a-core/src/router.rs`. **Both needles are permanent
   features of that file** — the loopback path will always need the self-asserted resolution at
   `:1087-1090`, and the verified entry point already exists at `:1494` for the TCP path. The leg
   therefore publishes `loopback_from_host_unverified: true` **forever, in every possible future**.
   It cannot change state, so it cannot fail, so it is decoration.
   *Sharpened 2026-08-16 by the `j1-crosshost-2a` preflight — the mechanism is worse than "both
   needles are permanent", in two ways you need before you rewrite it:*
   - **The "self-asserted" needle is satisfied by the VERIFIED path's own error message.**
     `contains_live` (**`:138-140`**) filters comment-prefixed lines but **not string literals**, and
     `crates/maos-a2a-core/src/router.rs:1514` (**re-verified unchanged at `0769869d`**) is the
     `format!` literal
     `"frame.from.host_id {} does not match TLS-verified peer {}"` — **inside
     `handle_intake_verified`'s own TLS-mismatch NACK**. So even deleting the loopback resolution at
     `:1087-1090` would leave the needle green, pinned by the code that proves verification *is*
     enforced.
   - **The leg cannot observe its own declared trigger.** "Rung 2 turns verification on" is a change
     in *which* router entry the composition root calls — in `maos-a2a-tcp` and `maos-bin` — **not a
     text change in `router.rs`, the only file this leg reads.** Any rewrite must therefore point the
     leg at the file where the trigger actually lives, or it reproduces the same defect at a new
     address.
   That makes three shipped documents wrong: this gate's own doc (`:30-32`), `1a`'s story record
   (`j1-crosshost-1a-frame-borne-delegation.md:288`) and the `j1-crosshost-2` sprint-status row all
   claim *"when rung 2 turns verification on, this leg flips."* It will not.
   **Fix it to observe the thing it names:** the leg must key on whether the **J1 delegation path**
   binds wire identity — i.e. on `crates/maos-bin/src/delegation.rs` / the composition root using
   `LoopbackA2ARouter` (which calls `handle_intake` directly, `crates/maos-a2a/src/adapter.rs:82`,
   `:97`) versus a verified transport. Then it genuinely flips when rung 2 lands, and a planted
   regression can red it. **Add a proven-red vector for the flipped state** — otherwise you have
   replaced one unfalsifiable leg with another. Correct the three documents in the same change.
2. **Land the vacuous-green guard as a SHARED PRIMITIVE in `gate_common.rs`, and be its first
   consumer** (F2). A leg that reads nothing must not be able to look like a leg that passed.
   **The reason this is not a local fix — stated as evidence, not as a census** *(corrected
   2026-08-16, F2)*: `xtask/src/gate_common.rs` (233 CODE lines) exports `phase_disposition`,
   `is_blocking_at`, `read_disposition`, `dev_enforced_red_blocks`, the `EvidenceState` /
   `EvidenceVerdict` projection, `validate_dates` and `emit_command` — **and no per-leg outcome
   record of any kind**. Verified by reading it. Several individual `check_*.rs` gates hand-roll
   their own guard (`check_vetting_attestation.rs:225-235` is the reference implementation); the
   shared home does not exist, which is why every one of them is bespoke. **Do NOT quote a
   "N of 68 gates" count as the justification** — that figure came from a text grep for
   `vacuous|vacuity`, it counts doc comments, and at `0769869d` it counts *this gate*, which
   acquired the word from `2a` and still has no mechanism.
   - Add the minimum to `xtask/src/gate_common.rs`: a per-leg outcome record (`ran`, plus a count of
     checks actually performed) and one assertion that hard-FAILs when any leg reports
     `!ran || checks == 0`. Semantics from `check_vetting_attestation.rs:225-235`. **~20 lines. Not a
     framework** — no plugin points, no trait hierarchy.
   - `check_j1_loopback_delegation` consumes it for **all FIVE** existing legs plus yours — `1a`'s
     two *and* `2a`'s three. `2a`'s legs shipped with no vacuity record either; you are the first
     consumer, so you are the one who makes them honest.
   - **The primitive gets its own falsifier** (proven-red): a leg rigged to check nothing must RED.
     A vacuity guard that can itself be vacuous is the defect it exists to prevent.
   - **Migrating the other 15 is explicitly OUT OF SCOPE** — that is 14-6's instrument work. This
     story lands the shared home and one consumer, nothing more.
   *Net effect on this story is negative lines:* a shared 20-line primitive replaces the bespoke
   guard that would otherwise be written inline here.
3. **Close the leg-omission null control — and note `2a` tripled the surface it guards.** Add a
   derivation test asserting `ledger_leg_names()` matches the legs `run_with_root` actually invokes.
   This gate is the **only** `ledger_leg_names()` owner with no such test —
   `check_reza_production_path.rs:1164`, `check_cross_region_consensus.rs:315`,
   `check_multi_tenant_loom.rs:1933` and `check_multi_region_slo.rs:752` all have one.
   **Measured at `0769869d`: the accessor now hand-lists FIVE names (`:91-100`) against five invoked
   legs, three of them added by `2a` by hand, and the file has NO `#[cfg(test)]` module at all.**
   A leg added and forgotten in the accessor still reds nothing, and the odds of that just went up
   150%.
   *Budget note — now load-bearing, not advisory:* an in-`src` `#[cfg(test)]` module is kloc-charged
   and CI-invisible (D11-E3; `xtask/src/tests/` measures **2367 charged CODE lines**). With `xtask`
   headroom at **+223**, put this in `xtask/tests/` unless you can state why it cannot go there. If
   it must be in-`src`, count it in AC3.1 and say so.
4. **Extend the proven-red vectors — in a NEW `xtask/tests/j1_crosshost_1b_proven_red.rs`, and
   repair BOTH existing fixture trees.** *(Rewritten 2026-08-16 — F3b. The original text named one
   file and one `lay_green` at one line. All three were correct at `5a921c0c` and none is correct
   now.)*
   - **Where the vectors live: a new per-story file.** `2a` set the precedent
     (`xtask/tests/j1_crosshost_2a_proven_red.rs`, 15 vectors, its own CI step at
     `discipline.yml:1818-1819`) and it is the right one — a story's falsifiers should red for that
     story's reasons. Register it as its own CI step beside the other two. `xtask/tests/` is
     kloc-free.
   - **The vectors:** a planted regression that **admits a disallowed intent**, and one that
     **collapses `-32001` into `-32009`**, must each RED. Plus AC2.2's own falsifier (a leg rigged to
     check nothing must RED) and AC2.5's enrollment vector.
   - **SHIP-BLOCKER, do this first:** your consent leg reads a **tenth** governed file
     (`crates/maos-bin/tests/consent_refusal_1b.rs`). `read()` (`:111-128`) pushes a `Finding` when a
     governed file is missing, so that file must be added to the const block **and** to `lay_green`
     in **all three** proven-red files: `j1_crosshost_1a_proven_red.rs:95`,
     `j1_crosshost_2a_proven_red.rs:105`, and your new one. Miss either existing file and its
     `baseline_fixture_tree_is_green` (`1a:157`) reds — at which point **every planted vector in that
     file passes for the wrong reason and CI still reports green.** That is F3's failure mode, and
     `2a`'s duplication of `lay_green` means there are now two ways to walk into it.
   - Keep `--test-threads=1` on every proven-red step (load-bearing — each vector sets
     `current_dir`).
5. **Enroll the new test file in CI — this line IS the control, and it is the load-bearing one.**
   *(Rewritten 2026-08-16 — F14. The leg this AC told you to build now EXISTS. What is left is
   smaller, and one decision.)*
   Add `--test consent_refusal_1b` to the J1 job's delegation-legs step — **`discipline.yml:1823`**
   (was `:1821`). `maos-bin` is never tested bare in CI; every invocation names explicit `--test`
   targets, so an unenrolled file in `crates/maos-bin/tests/` is dead. **Measured at `0769869d`: 24
   of the 30 files in `crates/maos-bin/tests/` are invoked by no job at all** — and
   `smoke_cli_wrapper_8_12` is still one of them (AC2.6).
   **Why this is not a footnote.** The gate is a static oracle: it has never observed a frame being
   refused and cannot — it greps source text. The behaviour is proven by the CI step, a *different
   mechanism in a different file*. **The enrollment line is the only thing connecting the linter to
   the judge.** Delete it and the gate still finds the right words in the test file and goes green
   while the test never runs.
   **(a) The enrollment leg already exists — do not build a second one.** `2a` landed
   `leg_completion_vectors_enrolled` (`:469-514`) against a new `WORKFLOW` const (`:72`), and its own
   review scoped it to the `check-j1-loopback-delegation` **job block** (a workflow-wide scan stayed
   green when the `--test` line merely existed *somewhere*, including in a non-blocking job). That
   mechanism is better than what this AC originally specified. Creating a second one collides —
   Trap 8.
   **(b) RATIFIED SHAPE — DERIVE the enrolled set, do not append to the const.** The leg is driven by
   `ENROLLED_TEST_TARGETS: &[&str] = &["worker_completion_2a", "worker_manifests_2a"]` (`:78`), a
   hand-maintained list. Appending `"consent_refusal_1b"` would make *your* vector fire and leave the
   next author one forgotten const line away from a dead test and a green gate — **which is
   `smoke_cli_wrapper_8_12`'s exact failure re-created at a new address**, inside the very gate built
   to prevent it. So: **replace the const with a derivation.** Walk `crates/maos-bin/tests/` for
   files matching the J1 delegation naming (`*_1a.rs` / `*_1b.rs` / `*_2a.rs`), and require every one
   of them to be named in the blocking job's block. The set then cannot go stale, and adding a J1
   test file without enrolling it reds the gate **by construction** rather than by remembering.
   Keep the job-scoping exactly as `2a` built it; only the source of the list changes.
   *Costs ~15 lines against the +223 `xtask` headroom (AC3.1); it removes a const, so net is near
   zero. If a directory walk is refused at review, the fallback is the const append **plus** an
   explicit Trap in this file naming the hazard — but the fallback is the weaker answer and this
   round-table did not choose it.*
   **(c) Proven-red vector: remove `--test consent_refusal_1b` from the workflow → the gate MUST
   red.** Not "should". *(The original numbering "vector #12" assumed one shared proven-red file;
   `1a`'s file holds 11 tests and `2a` opened its own. Number it inside your own file per AC2.4.)*
   **And one more, because the derivation in (b) needs its own falsifier:** add a vector that plants
   a `crates/maos-bin/tests/xyz_1b.rs` into the fixture tree **without** enrolling it, and assert the
   gate reds. A derivation that cannot be shown to notice a new file is a const list wearing a walk.
6. **Close the `smoke_cli_wrapper_8_12` null control — by exact test name.** Today
   `ci_local_split_refuses_a_granted_real_agent_without_the_live_flag` has **zero** CI invocation, so
   *"CI physically cannot spawn a paid agent"* holds by env-var omission rather than by an executed
   assertion. It is safe to run: it needs no secret and no network, spawns a 2-line `/bin/sh` fake
   `codex` it writes itself, and asserts the **refusal** path.
   **Enroll the single test, not the file** — following the **`discipline.yml:2599`** idiom (`cargo test -p maos-cli install_verify_local_release_artifact -- --exact`):
   `cargo test -p maos-bin --test smoke_cli_wrapper_8_12 ci_local_split_refuses_a_granted_real_agent_without_the_live_flag -- --exact`.
   Enrolling the whole file drags in `maos_run_cli_wrapper_worker_spawns_real_subprocess`, which
   needs `worker-cli-fixture` on `PATH` — not a `maos-bin` dev-dependency, so it would red on a fresh
   runner (the failure `discipline.yml` documents around the journey job — **and note `2a` already fixed this class for its own targets by pre-building the fixture at `:1840` (`cargo build -p worker --bin worker-cli-fixture`); reuse that, do not re-discover it**).
7. **Live-`codex` leg stays `MAOS_LIVE_AGENT=1`, local-only, NEVER CI.** Already registered at
   `crates/maos-bin/src/env_contract.rs:420`; no new env var is expected. If you add one, register it
   there or `check-env-contract` reds.
8. **Enrollment is VERIFY-only.** All six surfaces are correct at HEAD (see AC2 preamble table and
   Dev Notes). Touch one only if it is wrong. **Do not create a second gate module, dispatch arm, or
   registry entry** — it will collide.
9. **Keep the job hermetic and unconditional.** No `services.postgres` (the J1 job already satisfies
   `runs_gate`, so adding one fires `check-loom-substrate-drift`,
   `check_loom_substrate_drift.rs:701-706`); no `if: false` at job **or step** level
   (`check_epic_close_green.rs:83-90` trims leading whitespace, so it catches both).

10. **Expose per-leg outcomes, so a consent red is not reported under the wrong name.** The per-leg
    record AC2.2 introduces must be readable by callers, not collapsed to `Result<(), String>`.
    Today `demo_j1::run_delegation_gate()` (**`:793-813`**, was `:763-781`) matches on that one
    boolean and emits a single beat named `frame-borne-route-intact` — so after this story a red
    *consent* leg would print `FAIL frame-borne-route-intact`, naming the wrong failure in the
    narrated artifact. **This got worse under `2a`, which added three more legs behind the same
    single boolean: five legs now collapse into one beat.** Emit one beat per leg.
11. **Flip the ABSENT beat this story owns — the artifact must not print a false claim about its own
    work.** `xtask/src/demo_j1.rs:817-821` (was `:785-790`) declares
    `Beat::absent("disallowed-intent-refused-blocking", "a disallowed intent must be REFUSED (-32001
    CODE_INTENT_DENIED, distinct from -32009)", "j1-crosshost-1b")`. If the legs land and that beat
    still renders `ABSENT`, the demo states this work was never done.
    **Flip it in code:** remove the entry from `unlanded_beats()` (**`:815-838`**, four `Beat::absent`
    entries at `:817`/`:822`/`:827`/`:832`; extended into the vector at `:246`) and emit it as an
    **executed** beat from the gate-judging path.
    **Do NOT attempt the published-ledger route** — it is structurally unreachable: `ledger_gates()`
    (`xtask/src/evidence_ledger.rs:148-150`) derives from `check_loom_substrate_drift::contract_jobs()`
    (the four Postgres substrate gates), `expected_ledger_legs` (`:154-166`) has no J1 arm, and
    `validate_against` (`:1246-1249`) rejects an unknown gate up front. A hand-written J1 ledger
    would fail validation and suppress published-ledger application for the **entire** demo
    (`demo_j1.rs:817-826`). Extending `ledger_gates()`/`CONTRACTS` is **out of scope**.
12. **Do not break the demo's own suite, and do not rename `1a`'s leg.**
    **`xtask/src/tests/demo_j1_tests.rs:254`** (was `:218`) asserts `beats.len() == 9` — still 9 at
    `0769869d` — over `evaluate_beats`'s (`demo_j1.rs:559`) **event-derived** beats. A new *gate leg*
    does not trip it, but flipping your beat out of `unlanded_beats()` into the executed set **will**.
    Update that constant deliberately; `:15-24` asserts every `unlanded_beats()` entry is `ABSENT`
    and names an owner. The beat name `frame-borne-route-intact` is hard-coded at `demo_j1.rs:237`,
    `:796` and `:805` — renaming `1a`'s leg silently breaks the matcher. **Do not rename `2a`'s three
    leg names either** (`completion-oracle-per-adapter`, `worker-cli-under-library`,
    `completion-vectors-enrolled`) — they are published in `ledger_leg_names()` and consumed by AC2.2's
    per-leg record.

### AC3 — Budget, measured at HEAD and attributed honestly

1. **`xtask` needs no grant — but the margin is a quarter of what this story was drafted against.**
   **Measured at `0769869d`: 38386 / 38609 = +223 headroom** (was 38075 / +534 at `5a921c0c`; `2a`
   spent 311). *The original sentence here read "+534, that is ~2.4× the whole gate module." The
   argument inverts: **+223 against a 362-line gate module is 0.6×.*** `kloc.toml:64-65` still
   governs: *"Slack is operating capacity, NOT authorization."*
   **Estimated `xtask/src` cost of this story: 110-145 lines** — consent legs 60-90, the
   `gate_common` primitive ~20-25, per-leg beats in `demo_j1` 20-30, AC2.5(b)'s derivation ~15 minus
   the const it deletes. It fits inside +223. It does **not** fit if the `ledger_leg_names()`
   derivation test lands in-`src` as well, so it does not (AC2.3).
   Keep the proven-red harness in `xtask/tests/` (free) and prefer it for anything that fits.
   *Note also: `kloc.toml:203`'s own annotation still ends "Current headroom 534" — stale at
   `0769869d`. It is prose in a ceiling record, not a control. **Do not "fix" it by editing
   `kloc.toml`**: touching that file invites exactly the silent re-run the file forbids. Report it in
   the Dev Agent Record and leave the number to whoever next takes a measured `xtask` grant.*
2. **D15 is RESOLVED and then SUPERSEDED. The number in this story's original text is no longer the
   ceiling. Do not re-request; keep the CURRENT number true.**
   D15 ratified `maos-bin = 16219` (exact measured, zero headroom) on 2026-08-15. **`2a` then took a
   further review grant to `maos-bin = 16260`, ratified by Lunarpulse 2026-08-16 after measurement,
   live at `xtask/kloc.toml:264` — again EXACT MEASURED, ZERO HEADROOM.** The crate is **GREEN at
   `0769869d` at 16260 / 16260.** Full attribution chain, per commit: `af788c3e` 16027 → `6827dc87`
   (1a) **16211** → `296aa2ce` (j1-demo drain fix) **16219** → `0769869d` (2a: `worker_cli`
   relocation −202, then dev + §A6 review patches) **16260**.
   **What this means for you, concretely:**
   - Your additions to `maos-bin/src` must be **+0**. All new code lands in `crates/maos-bin/tests/`
     (kloc-excluded) and `xtask/` (**223** spare, not 534). **One production line in `maos-bin/src`
     reds CI** — that is the deliberate design, not an accident, and `2a` has now taken the posture
     twice.
   - If you genuinely need a `maos-bin/src` line for a **correctness or compliance repair**, the
     valve is `kloc.toml:87` (a ceiling *"must never block a correctness or compliance repair"*,
     cited by name by Story 13.5g). State the repair and take a named grant. **Silent growth is what
     the zero stops; declared repair is not blocked.**
   - Rationale is recorded in the `kloc.toml` annotation and in D15 — read it before arguing with
     the wall. Summary: the formula's 16544 would grant 325 lines to a crate that is 73% `main.rs`
     with no decomposition scheduled; tighter-than-formula is house style (`kloc.toml:203`); zero
     headroom is precedented and deliberate (`maos-a2a-core` 4654/4654).
3. **REFUSE the aggregate grant, deliberately, and say why in the record.** All three breaches below
   are pre-existing and owned elsewhere:

   *(Re-measured at `0769869d`. `maos-bin` has LEFT this table — it went green under `2a`'s grant.
   The aggregate breach nearly doubled.)*

   | Breach | Size | Owner |
   |---|---:|---|
   | `maos-kernel-core` 18933/18248 | +685 | **D13** — `spec-epic-5-review-finding-closure` (repair) / 14-6 (instrument) |
   | `maos-domain` 8694/8644 | +50 | **D14** — 14-7, via explicit AC expansion |
   | `_aggregate_hardfail` **147942/147057** | **+885** | **D17 (already filed)** — see below |

   **The aggregate grew +393 while the crate ceilings it sits over grew +57** (`maos-bin` +41,
   `maos-cli` +16 — both `2a`). That gap is the whole point of the key: it is measuring real code,
   not ceiling arithmetic. **D17's row still records the old +492 / 147549** — verify and report the
   current figure; do not edit the register row (that is 14-6's, and Binding rule 1 applies).

   **This story *could* take the aggregate grant, and must not.** `kloc.toml:61` permits recalculation
   *"at an epic retrospective, **or** under an explicitly authorized measured grant"* — two doors, and
   Stories 13.6d and 13.6e both went through the second one, as did the epic-orphaned
   `j1-demo-one-command-scene`. So neither "bridge stories have no retrospective" nor "the aggregate
   is unowned" is a reason. The reason is narrower and stronger:
   - **1b's contribution to the aggregate is ZERO** — every line it writes is kloc-excluded. 13.6e's
     grant is annotated *"authorized formula applies **because aggregate actually breached**"*: the
     story that caused it paid for it.
   - The +885 is **arithmetic downstream of D13 (+685), D14 (+50), D15 (+41) and `2a`'s two grants
     (+57), plus real measured growth**. Re-basing it here
     turns the CI signal that holds those three to account **green**, leaving only prose behind them.
     D13 forbids precisely this of 14-6 — *"may not erase this red with a grant it has no measured
     delta to justify"* — and 1b is further from the delta than 14-6 is.
   - **It would not stay fixed anyway — and `2a` just proved it with a second data point.**
     Re-basing a *crate* ceiling does not change the *measured* aggregate; measured stays 147942.
     D13/D14/D15 can all land in full and this key is still red. `2a` re-based two crates to exact
     zero and the aggregate went **up**, not down. It is independent by design — the only instrument
     that catches distributed growth no per-crate reserve can see.
4. **D17 is already FILED — verify, do not re-file.** `_aggregate_hardfail` is a **standing red with
   three named debtors**, it does **not** clear when they re-base, and it clears at an epic
   retrospective or under a grant taken by someone with a measured delta to justify it. Owner: the
   ceiling instrument (**14-6**) or the Epic-14 retrospective, deadline at the v2.2 close. Do
   **not** record it as "unowned" — that framing was disproved at the round-table.
5. **Close with `kloc-check` RED, attributed.** Measure before and after; state plainly which keys
   are red and whose they are. **At `0769869d` that is THREE keys, not four** — `over_budget` reads
   exactly `['aggregate 147942 >= 147057', 'maos-domain 8694 > 8644', 'maos-kernel-core 18933 >
   18248']`. `maos-bin` left the list under `2a`'s grant. Do not inherit the count; re-run it. This
   story cannot and should not make that gate green.
6. **Three crates now sit at EXACTLY zero headroom. Two of them are not yours; do not reach for
   either.**
   - `maos-a2a-core` **4654 / 4654** (`kloc.toml:407`; D10 forbids a third unscoped grant). One added
     **production** line hard-fails on contact. `crates/maos-a2a-core/tests/` remains free, but
     AC1.5(b) is explicit that the error-mapping fix is not taken here — it goes to **D18**, whose
     resolution routes the `maos-domain` side to 14-7 under D14.
   - `maos-bin` **16260 / 16260** — yours to keep true at +0 (AC3.2).
   - `maos-cli` **4642 / 4642** *(NEW at `0769869d`, `kloc.toml:263`, `2a`'s grant)*. Not a crate this
     story touches. It is listed so that "I'll put the helper in `maos-cli`" is a decision you make
     knowingly rather than a wall you discover.
   *If you genuinely need a production line in any of the three for a correctness or compliance
   repair, the release valve is `kloc.toml:87` — state the repair and take a NAMED MEASURED grant.
   It is permission to ask, not a machine carve-out: `kloc_check.rs` has no exemption token and the
   compare loop is unconditional.*
7. **Kernel axes: re-run the instruments, but report them honestly.** `check-kernel-baseline` GREEN
   at 24472 is real evidence. `abi-diff` / `check-abi-ratification` green is **not** — per F11 they
   scope to `crates/maos-spirit-abi` only and are structurally blind to `maos-kernel-core`'s
   re-exported surface (`iac.rs:13` is `pub use maos_iac::*`), which is open **FLAG-E4**. A flat
   `src_lines` does not imply a flat ABI. This story makes no ABI change; state that as a fact about
   the diff, not as a gate result. Any kernel seam is FLAG-Winston, never an implicit re-pin.

### AC4 — Close the lane honestly

1. **Set this story's `sprint-status.yaml` row to `done`, and PAY D19's price explicitly.**
   *(Resolved at the 2026-08-16 round-table — F17.)*
   **The gap, re-verified and now larger than this story recorded: SEVEN blocking gates skip this
   filename**, not five. Five are directory-walkers behind an ASCII-digit-prefix filter —
   `check_dev_model_tier.rs:103`, `check_dev_model_used_populated.rs:136`,
   `check_bare_review_findings.rs:35`, `check_dev_record_completeness.rs:245-247`,
   `check_review_findings_resolved.rs:57-60` — and two skip by different mechanisms:
   `check_epic_close_coherence.rs:215-217` (`head.parse().ok()?`, its comment naming `j1-crosshost-1`
   outright) and `check_epic_6_bridge.rs:820-828` plus its own two walkers at `:2563` and `:2608`.
   That is **D19**, owner **14-0**, filed 2026-08-16 by `2a`'s round-table.
   **D19's deadline reads *"before the next `j1-*` story leaves `ready-for-dev`"* — and this story is
   the only `ready-for-dev` `j1-*` story, so read literally D19 blocks it.** That reading is refused
   here, on the record and with a reason: D19 governs *story-file discipline enforcement*, 14-0 does
   not yet exist as a story, and stalling the lane's only unblocked story behind an unwritten
   instrument story is an outage, not a control. **What is NOT refused is the cost.** Disclosure was
   already the disposition for `1a`, `1b`, `j1-demo` and `2a`, and *"disclosed four times is not a
   disposition"* is why D19 exists. So this story pays it manually, in full:
   - Record in the Dev Agent Record, by name: the **dev model** (`vendor/model` + harness + date),
     the **reviewer model** (must differ from the dev model), **all four §A6 layers run** (Blind ·
     Edge Case · Acceptance · **Test-Infra**) plus the runtime-execution check, and the **in-repo
     path of the review artifact**. That is precisely the set the seven blind gates would have
     checked. A green CI does not assert any of it.
   - **Cite D19 as OPEN** and state that this story's compliance is asserted by a human, not by a
     gate. Do not close D19; do not widen a gate's scope here (Trap: that is 14-0's single-source
     decision, and patching one of seven walkers is the defect this project has already paid for
     twice).
2. **Write AC1.5's two non-coverage statements into the `j1-crosshost-2b` story file and its
   sprint-status row** as explicit inheritance, so rung 2's preflight cannot mistake a partial proof
   for a whole one. *(Retargeted 2026-08-16 — F16. The original text named `j1-crosshost-2`. Rung 2
   was split on 2026-08-15 into `2a` (done) / `2b` / `2c`, and **there is no `j1-crosshost-2` row at
   `0769869d`.** This story's own rule — "you cannot defer into a document that does not exist" —
   caught up with it.)* `2b` is the correct heir on the merits, not just by elimination: it is where
   a second host first authenticates a peer, so AC1.5(a)'s *"a frame picks its own judge"* becomes
   load-bearing there, and D18's re-pinned deadline already names it. **Verify the row and the file
   both exist on the day you write into them.**
3. **Verify D17 and D18 — both FILED, and they do NOT read the same way.**
   *(Corrected 2026-08-16 — F15. An earlier revision said "confirm both still read as OPEN". That
   was true for D17 and false for D18.)*
   - **D17** — the standing aggregate red (owner 14-6 / Epic-14 retro). **Expect OPEN.** Its row
     records 147549 / +492; measured at `0769869d` is **147942 / +885**. Report the drift; do not
     edit the row.
   - **D18** — the deny-code conflation (owner John + Vex, target 14-4). **Expect RESOLVED**, with
     the deadline re-pinned to *"before `j1-crosshost-2b` writes its first line"*. Resolved as a
     decision, **not** as code: the conflation is still live in `map_a2a_error_to_iac_bus` at HEAD,
     so AC1.4 still asserts at the router seam.
   Do not re-file and do not close either. Binding rule 1: shipping adjacent work does not close a
   row.
4. **Rung 2's evidence contract expects this rung to read `PROVEN_BLOCKING`**
   (`xtask/src/gate_common.rs:139-146`).
5. **Verify D11 and leave it OPEN.** Its row already reads *"36 entries vs 67 `check_*.rs` at
   `af788c3e`; 37 / 68 once the J1 loopback gate lands"* and the counts are **correct at HEAD**.
   Binding rule 1: shipping adjacent work does not close a row — D11's owners (Winston + Murat)
   settle substance at 14-6's preflight. **Do not cite the 37/68 ratio as evidence of anything**
   (Trap 12).
6. **Do not claim FR23a closure.** The coverage row carries `corpora: []` while FR23a's PRD floor is
   *"cross-Spirit consent 30 scenarios with 100% disallowed blocked"*. This story adds a handful of
   refusal legs on the J1 path — it does not deliver that corpus. Update the row's `notes`; do not
   mark the FR satisfied.

---

## Traps

1. **`cargo run -q -p xtask -- <cmd>`.** There is no `cargo xtask` alias.
2. **The gate runs no tests.** It is a static source oracle. Do not make it shell out (F1/F3).
3. **Never copy `check_vetting_attestation::invoke_leg`.** It shells `cargo` with no `current_dir`
   and would silently vacuum the proven-red suite (F3).
4. **`-32001` is unreachable through `DelegationLeg::install`** — one intent configures both
   allowlists, so the send seam denies first (F4).
5. **`-32001` / `-32009` / `-32003` must not be conflated** — and above `A2ARouterCore` they already
   are (F5). Assert at the router seam.
6. **The loopback allowlist is keyed by the SOURCE host** (`router.rs:1087-1090`, fallback
   `HostId("loopback")` when `None`, `lookup_peer` at `:1093` with no first-peer fallback).
7. **`task.assign` is not a legal consent intent.** The vocabulary is
   `development-task:write-workspace`, defined **once** at `spirits/orchestrator/src/lib.rs:73`.
8. **The gate already exists.** Creating a second module/dispatch/registry entry collides.
9. **Use `structural()`, not `contains_live()`, for multi-token needles** — or you re-create the
   `cargo fmt` vs gate mutual exclusion that `246660f9` fixed.
10. **kloc is tokei `code` lines, not raw.** `check_vetting_attestation.rs` is 235 code / 273 raw;
    this gate is 219 code / 353 raw. Budget in the right unit.
11. **`tests/` costs zero kloc in EVERY crate** (verified: 0 measured files under `/tests/`), but
    `xtask/src/tests/` is **charged** (it is under `src/`) — that is D11-E3.
12. **The 37/68 ratio asserts nothing.** `check_ship_gate_completeness` iterates `EXPECTED_GATES`
    one-directionally (`:182`, `:203`); at HEAD it reports `expected 37 / found 43 / missing []`.
    Six ship-gate jobs are absent from `EXPECTED_GATES` and it is silent. Quoting the ratio as
    evidence is a claim standing in for a control.
13. **`coverage-matrix` cannot fail.** `mode: warning` (`tests/coverage-matrix.yaml:3`) and
    `coverage_matrix.rs:65` fails only on `"hard"`; at HEAD it reports `passed:false` with **31
    violations** and exits **0**. So a misspelled gate name in the FR23a row reds nothing.
    `check-coverage-matrix-completeness` only checks for *empty* gate lists.
14. **A `maos-bin` test not named in a `--test` flag is dead in CI.** `maos-a2a-core` is the opposite
    — `discipline.yml:1536` runs it bare, so anything in `crates/maos-a2a-core/tests/` auto-runs.
15. **Do not enroll the whole `smoke_cli_wrapper_8_12` file** — enroll the one test by exact name
    (AC2.6).
16. **`journey_j1` and `delegation_leg_1a` already prove the positive path.** Do not rebuild them.
17. **Establish a clean baseline measurement before attributing kloc movement.** Four crates arrive
    red.
18. **`cargo test -p maos-bin` is RED at HEAD under default parallel flags** (D16, F12). Run your
    file scoped: `cargo test -p maos-bin --test consent_refusal_1b`. The `MAOS_HOME` isolation
    defect is not yours to fix.
19. **Two local `maos run … --once` on the same `XDG_DATA_HOME` within 60s: run 2 exits 1** with
    `EOrchestratorDispatchRawOutput` — FR21's dispatch gate is chronological, not causal (open
    **FLAG-E5**, owned by Story 6.2). Use a fresh data home between runs; do not "fix" it here.
20. **`abi-diff` green proves nothing about this surface** (F11 / FLAG-E4).
21. **Do not rename `1a`'s leg `frame-borne-route-intact`** — three hard-codings in `demo_j1.rs`
    (`:237`, `:796`, `:805`). Nor `2a`'s three leg names, published in `ledger_leg_names()`.

*(Traps 22-25 added at the 2026-08-16 `0769869d` re-baseline.)*

22. **`lay_green` exists TWICE and a missing governed file reds the fixture tree.** `1a:95` and
    `2a:105` lay the same nine files. Your tenth (`crates/maos-bin/tests/consent_refusal_1b.rs`) must
    be added to **both**, plus your own new file. Miss one and its
    `baseline_fixture_tree_is_green` reds, every planted vector in that file passes for the wrong
    reason, and CI reports green (F3b). This is F3's failure mode arriving through a door F3 did not
    know existed.
23. **The enrollment gate leg ALREADY EXISTS — do not build a second one.**
    `leg_completion_vectors_enrolled` (`:469`), `WORKFLOW` const (`:72`), job-scoped. Trap 8 applies:
    a second module/dispatch/registry entry collides. Your work is AC2.5(b): replace
    `ENROLLED_TEST_TARGETS` (`:78`) with a derivation.
24. **`production_before_tests` is a NO-OP on a `crates/*/tests/` file.** It splits at
    `"\n#[cfg(test)]"`, which an integration test does not contain. Use the composed idiom anyway for
    uniformity, but do not believe it is guarding your own test file — only AC2.5's CI enrollment
    proves that file runs at all.
25. **Do not edit `xtask/kloc.toml` to correct its stale "Current headroom 534" annotation.** Prose
    in a ceiling record is not a control, and touching that file invites the silent formula re-run it
    forbids (`kloc.toml:58-65`). Report it; leave the number to the next measured `xtask` grant.
26. **Story-file line references in THIS document expire.** Two stories (`2a`, `j1-demo`) have now
    moved the gate, the demo and the workflow underneath a `ready-for-dev` story. **Re-derive every
    line number by reading the file before you use it**, and if `git log` shows commits after
    `0769869d` touching `xtask/src/check_j1_loopback_delegation.rs`, `xtask/src/demo_j1.rs`,
    `.github/workflows/discipline.yml` or `xtask/kloc.toml`, re-run the whole "what 2a moved" table
    for yourself before starting.

---

## Tasks

- [x] **T1 (AC1.1-1.2)** — Create `crates/maos-bin/tests/consent_refusal_1b.rs`. Local positive
      control + the asymmetric-pairing `-32001` leg asserting `IntentDeniedAtPeer` and the
      `founder-loop-host` message tail. Name both `peer_id`s literally.
- [x] **T2 (AC1.3-1.4)** — `-32009` at send **and** accept seams, every reachable
      `UnclassifiedReason` (`Absent` / `NonCanonical` / `Oversized`), typed `data.reason` assertion;
      both-ways non-conflation; `-32003` expiry kept distinct.
- [x] **T0 (re-baseline) — DO THIS FIRST.** `git log --oneline 0769869d..HEAD` over
      `xtask/src/check_j1_loopback_delegation.rs`, `xtask/src/demo_j1.rs`,
      `.github/workflows/discipline.yml`, `xtask/kloc.toml` and `xtask/tests/j1_crosshost_*`. If
      anything landed after `0769869d`, re-derive the "what 2a moved" table yourself before writing
      a line. Then run `kloc-check --json`, `check-j1-loopback-delegation --json` and
      `check-kernel-baseline --json` and record the four numbers. **Measure in a `git archive`
      extraction if another session may be holding the tree.**
- [x] **T3 (AC1.5)** — Write both non-coverage statements (peer-auth unverified; production error
      mapping conflates) into the story record **and into `j1-crosshost-2b`** (file + row), not into
      `j1-crosshost-2` — that row no longer exists. Confirm `1a`'s `loopback-from-host-unverified`
      boundary leg still reports `true`. **D18: verify it reads RESOLVED** (deadline re-pinned to
      `2b`); do not re-file, do not re-open.
- [x] **T4 (AC2.5)** — Add `--test consent_refusal_1b` at **`discipline.yml:1823`**. **Do not build
      an enrollment leg — it exists** (`:469`). Instead replace `ENROLLED_TEST_TARGETS` (`:78`) with
      a derivation over `crates/maos-bin/tests/*_1a.rs|*_1b.rs|*_2a.rs`, keeping `2a`'s job-scoping.
      Two vectors: delete the flag → gate MUST red; plant an unenrolled `*_1b.rs` → gate MUST red.
      This is the only link between the static oracle and the behaviour it judges.
- [x] **T5 (AC2.1)** — Add the consent leg(s) to the gate, `structural(production_before_tests(…))`,
      root-relative, in the **seven**-site order. Registers `consent_refusal_1b.rs` as the gate's
      tenth governed file → T7 becomes mandatory. **Measure code lines on completion.**
- [x] **T6 (AC2.2-2.3)** — Land the per-leg `ran`/`checks` record + vacuous-green hard-FAIL as a
      **shared primitive in `gate_common.rs`** (~20 lines, no framework); make this gate its first
      consumer across **all five** existing legs — `1a`'s two **and `2a`'s three** — plus yours; give
      the primitive its own proven-red (a leg that checks nothing must RED). Do **not** migrate other
      gates — that is 14-6. Add the `ledger_leg_names()` derivation test **in `xtask/tests/`** (five
      hand-listed names, no test today).
- [x] **T7 (AC2.4) — SHIP-BLOCKER ORDERING.** Create `xtask/tests/j1_crosshost_1b_proven_red.rs`
      (2a's per-story precedent) with its own CI step. **Then add the tenth governed file to the
      const block AND `lay_green` in ALL THREE proven-red files** — `1a:95`, `2a:105`, yours — and
      confirm `baseline_fixture_tree_is_green` is green in each before trusting any planted vector.
- [x] **T8 (AC2.6)** — Enroll `ci_local_split_refuses_a_granted_real_agent_without_the_live_flag` by
      exact name; confirm it executes and that CI needs no secret.
- [x] **T9 (AC2.8-2.9)** — Verify the six enrollment surfaces; fix only what is wrong. Job stays
      hermetic, no `if:`.
- [x] **T10 (AC2.10-2.12)** — Per-leg beats in `demo_j1` (**five** legs collapse into one beat today,
      not two); flip `disallowed-intent-refused-blocking` out of `unlanded_beats()` (**`:815-838`**)
      in code, never via a ledger; update **`demo_j1_tests.rs:254`**'s `beats.len() == 9`
      deliberately; do not rename `1a`'s or `2a`'s leg names.
- [x] **T11 (AC3)** — Re-measure `kloc-check` before/after. **No grant to take: `maos-bin` is
      16260/16260 GREEN at `0769869d`** (D15's 16219 was superseded by `2a`'s review grant at
      `kloc.toml:264`) — verify it is still true and that your `maos-bin/src` delta is **+0**. Budget
      the `xtask` work against **+223**, not +534. **REFUSE the aggregate grant** (D17 filed; now
      +885). Close with `kloc-check` red, attributed to **three** keys. Re-run
      `check-kernel-baseline` (real evidence, 24472) and the ABI gates (report as blind, per F11).
- [x] **T12 (AC4)** — Sprint-status → `done`; write the two inheritances into **`j1-crosshost-2b`**
      (file + row) — `j1-crosshost-2` does not exist; confirm **D17 reads OPEN and D18 reads
      RESOLVED**; verify D11 reads 37/68 and stays **open**; **cite D19 as OPEN and pay its price
      manually per AC4.1** — dev model, differing reviewer model, all four §A6 layers named, artifact
      path in-repo; update the FR23a `notes` without claiming corpus closure.

### Review Findings

- **Reviewer**: `zai/glm-5.2` (≠ dev `anthropic/claude-opus-5`) · 2026-08-16 · §A6 4 layers (Blind · Edge · Acceptance · Test-Infra) + runtime check · 26 raw findings → 14 after dedup → **13 patch, 1 dismissed** (the sole decision resolved by party-mode consensus 8/8 → patch, per spec + long-term correctness).

- [x] [Review][Patch] *(resolved from Decision by round-table consensus 8/8 — Winston · Murat · Vex · Grumbal · Mary · Amelia · John · Paige, 2026-08-16; criterion: per spec + long-term correctness)* Document the literal-bait limit in the consent leg's doc block — `structural()` strips comment-prefixed lines only, so a bait `const BAIT: &str = "assert_eq!(…)"` or inline `/* … */` satisfies needles while the assertion no longer executes. RULING: accept-and-document (Trap 24's precedent applied a second time); needles 3-4 embed string literals, so quote-stripping breaks self-match and any heuristic filter is the `246660f9` false-alarm class; every accidental path (delete/weaken/`//`-comment/reformat/relocate) is caught once P1/P3 land — **this acceptance is VOID if P1/P3 do not land**. The durable fix (tokenizer/AST needles) is **14-6's** instrument work with this gate named customer #1. Fix: one honest paragraph beside the `production_before_tests` caveat in `leg_consent_refusal_proofs`' doc comment [xtask/src/check_j1_loopback_delegation.rs:676-694]
- [x] [Review][Patch] Consent leg cannot see test de-registration — REQUIRED needles live in function bodies; deleting all seven `#[tokio::test]` attributes leaves every needle green, `cargo test --test consent_refusal_1b` runs 0 tests and exits 0, gate + CI green. The `GOOD_CONSENT_REFUSAL` fixture itself proves the bypass (shapes in unannotated `fn`s pass). Fix: needle test registration (`#[tokio::test]` count ≥ 7 or per-test identity) + a deleting-one-attribute vector [xtask/src/check_j1_loopback_delegation.rs:714-733; xtask/tests/j1_crosshost_1b_proven_red.rs:119-141]
- [x] [Review][Patch] Boundary leg scans router.rs WITHOUT `production_before_tests` — `structural(&router_src)` at :409 includes the `#[cfg(test)]` module (router.rs:1786); relocating/copying the `let peer_host = match &frame.from.host_id` resolution into the test module keeps `self_asserted_resolution` true and the boundary-moved event unobserved — the exact relocation escape the composed idiom exists to close, inside the AC2.2a repair itself. Latent today (expression exists only in production, verified). Fix: `structural(production_before_tests(&router_src))` + relocation vector [xtask/src/check_j1_loopback_delegation.rs:408-409]
- [x] [Review][Patch] REQUIRED needle set incomplete vs AC1.1-1.4 — three deletions stay green: (a) the AC1.1 positive-control test `allowlisted_delegation_intent_is_admitted_and_its_intent_class_delivered` (no needle; deleting it leaves only negatives that can pass vacuously); (b) `assert_eq!(peer, TO_HOST)` (needle 1 only pins the destructure pattern — a `-32001` naming the WRONG peer passes); (c) the accept-seam typed binding `assert_eq!(nack_reason(&nack.error), expected)` (needles 7-10 pin the helper + variant names, not the comparison — the `-32009` reason goes back to numeric-only). Fix: three needles + three vectors [xtask/src/check_j1_loopback_delegation.rs:714-733 vs crates/maos-bin/tests/consent_refusal_1b.rs:212-236,271-274,342-346]
- [x] [Review][Patch] Enrollment matcher accepts non-executing / non-exact occurrences — `flat.contains("--test{target}")` over the whole job block is satisfied by the flag in a step `name:` or `echo` (the run line deleted), and by a longer token prefix (`--test foo_1b_extra` contains `--testfoo_1b`); conversely a valid `--test=` spelling is rejected. Fix: bind the match to a `run:` cargo line and token boundaries [xtask/src/check_j1_loopback_delegation.rs:649-673]
- [x] [Review][Patch] `disallowed-intent-refused-blocking` beat lifecycle has two holes — (a) `--skip-gate` runs omit the beat ENTIRELY (it left `unlanded_beats()` and is emitted only inside `run_delegation_gate()`, which the skip branch bypasses) — a silent-skip regression of the demo's honest-labeling contract; (b) no test asserts the beat is EMITTED executed — `the_refusal_beat_is_no_longer_declared_unlanded` only checks absence from `unlanded_beats()`, so deleting the `beats.push` at demo_j1.rs:846 leaves the suite green and the beat gone from every run. Fix: emit/ABSENT in the skip branch + an emission test [xtask/src/demo_j1.rs:235-249,844-854; xtask/src/tests/demo_j1_tests.rs:61-69]
- [x] [Review][Patch] `verified_composed` false-flips on a bare token — any production mention of `handle_intake_verified`/`maos_a2a_tcp` in delegation.rs (e.g. a preparatory `use` import during 2b) flips the boundary and reds the Blocking gate while `install` still composes `paired_loopback_router`. Fix: treat verified as composed only when loopback is NOT (`!loopback_composed && (…)`); the main.rs-level swap blind spot stays documented (main.rs already carries verified-path smoke text and cannot be a text key) [xtask/src/check_j1_loopback_delegation.rs:394-399]
- [x] [Review][Patch] Exact-name CI control dies green on rename — `cargo test … ci_local_split_… -- --exact` exits 0 when the filter matches zero tests, so a renamed/removed test silently re-kills the AC2.6 control. Fix: guard the step with `grep -q 'fn ci_local_split_refuses_a_granted_real_agent_without_the_live_flag' crates/maos-bin/tests/smoke_cli_wrapper_8_12.rs &&` before the cargo line [.github/workflows/discipline.yml:1872-1873]
- [x] [Review][Patch] Fixture trees omit the `_1a` derived targets — none of the three `lay_green` trees lays `delegation_leg_1a.rs`/`topology_delegation_1a.rs`, so deleting `"_1a.rs"` from `J1_TEST_SUFFIXES` keeps every vector green while the real gate silently stops enforcing 1a enrollment (a hand-list hiding inside the derivation). Fix: lay both `_1a` stubs + their `--test` invocations in all three trees (1b, 1a, 2a) [xtask/tests/j1_crosshost_1b_proven_red.rs:57-62; j1_crosshost_1a_proven_red.rs; j1_crosshost_2a_proven_red.rs]
- [x] [Review][Patch] Boundary-leg doc corrections incomplete or wrong — 1a's story record :285-289 (the second of AC2.2a's "three documents") is UNTOUCHED and still describes the four-line auto-flip leg; the 2b inheritance (:331-335) and the sprint-status INHERITED comment claim the leg "no longer needles `router.rs`" — false: door two reads router.rs and needles the resolution expression. Fix: correct 1a's record, the 2b section + sprint comment to name BOTH live inputs (and 2b's stale :266-293/:432-437 line refs while there) [_bmad-output/implementation-artifacts/j1-crosshost-1a-frame-borne-delegation.md:285-289; j1-crosshost-2b-cross-host-delegation-mechanism.md:331-335; sprint-status.yaml INHERITED comment]
- [x] [Review][Patch] 2b's G15 premise is disproved by production code — G15 calls the delegation envelope "a permanent, non-expiring bearer grant" (also :533-535, chain table :709 "expiry no-op", wired into AC4.1/T10), but `prepare_outbound` stamps `valid_until_ns = now + consent_ttl_secs` on every None-carrying envelope before the wire (router.rs:866-871) — a fact THIS diff's own test file asserts (consent_refusal_1b.rs:452-456). 2b would build a negative control for a non-gap. Fix: rewrite G15 + refs to the true residual (D1 TRANSITIONAL policy: transport-stamped TTL when the granter supplies none; explicit granter expiry is authoritative) [_bmad-output/implementation-artifacts/j1-crosshost-2b-cross-host-delegation-mechanism.md:246-252,533-535,709]
- [x] [Review][Patch] AC2.1's seventh site is missing — no named `consent_refusal_proofs` boolean in the gate's JSON (`leg_audits` carries the substance but a caller must reconstruct `leg_green` from two collections). Fix: publish the derived boolean [xtask/src/check_j1_loopback_delegation.rs:845-865]
- [x] [Review][Patch] Dev Agent Record accuracy — "1a 11/11, 2a 15/15, 1b 22/22 = **47** vectors" sums to **48**; the File List omits the changed `.memlog.md` and lists the wholly-NEW 2b story file as a section change. Fix the record [j1-crosshost-1b-consent-proofs-and-gate.md:1186,1288-1324]

- Dismissed (1): `LegAudit::checked()` not coupled to an evaluated condition — matches the AC2.2-specified reference semantics (`check_vetting_attestation.rs:225-235` is counter-based identically); the guard's threat model is legs that read nothing (caught), and exploiting the residual requires editing the oracle source itself.

---

## Dev Notes

### Measured at HEAD (`0769869d`, clean tree, 2026-08-16) — inherit no number from an older story

*(Re-measured after `j1-crosshost-2a` landed. The `5a921c0c` column is kept so the delta is visible,
because the drift is the finding.)*

| Instrument | Ceiling / pin | at `5a921c0c` | **at `0769869d`** | Verdict |
|---|---:|---:|---:|---|
| kloc `xtask` | 38609 (`kloc.toml:203`) | 38075 (+534) | **38386** | GREEN, **+223 headroom** |
| kloc `maos-bin` | **16260** (`kloc.toml:264`) | 16219 vs 16178, RED +41 | **16260** | **GREEN — ZERO headroom.** D15's 16219 superseded by `2a`'s grant |
| kloc `maos-cli` | **4642** (`kloc.toml:263`) | 4626 | **4642** | GREEN — **ZERO headroom** (`2a`). NEW wall, not yours |
| kloc `maos-a2a-core` | 4654 | 4654 | **4654** | GREEN — **zero headroom by design** (D10) |
| kloc `maos-kernel-core` | 18248 | 18933 | **18933** | RED +685 — D13, not yours |
| kloc `maos-domain` | 8644 | 8694 | **8694** | RED +50 — D14, not yours |
| kloc `_aggregate_hardfail` | 147057 | 147549 (+492) | **147942** | **RED +885 — standing red, named debtors (D17)** |
| `kloc-check` `over_budget` | — | four keys | **three keys** | `['aggregate 147942 >= 147057', 'maos-domain 8694 > 8644', 'maos-kernel-core 18933 > 18248']` |
| `check-kernel-baseline` | 24472 | 24472 | **24472** | GREEN |
| `abi-diff` / `check-abi-ratification` | — | 0 / 3 / 0 | unchanged | GREEN — **and blind (F11)** |
| `check-j1-loopback-delegation` | — | 2 legs | **5 legs**, `oracle_green: true` | GREEN, Blocking |
| `check-ship-gate-completeness` | — | 37/43 | **expected 37 / found 43 / missing []** | GREEN |
| `coverage-matrix` | — | 31 violations | unchanged | **exits 0 — cannot fail** |

Per-file at `0769869d`: `check_j1_loopback_delegation.rs` **362** code / 576 raw *(was 219/353)* ·
`gate_common.rs` **233** code · `demo_j1.rs` **1106** code / 1449 raw *(was 938/1207)* ·
`xtask/src/tests/` **2367** code across 15 files — **charged** (D11-E3).
`xtask/tests/j1_crosshost_1a_proven_red.rs` (11 tests) and `j1_crosshost_2a_proven_red.rs`
(15 tests) are **excluded** from the measurement.

### Gate anatomy — what is ACTUALLY there

`xtask/src/check_j1_loopback_delegation.rs`, a static source-text oracle:

**Re-derived at `0769869d` — every line below moved when `2a` added 143 code lines and three legs.**

```
consts :49-87   DELEGATION_INTENT :49 · DELEGATION_HOST :51 · TOPOLOGY :55 · DELEGATION_RS :56 ·
                MAILBOX_RS :57 · MAIN_RS :58 · ORCHESTRATOR_RS :59 · A2A_ROUTER_RS :63 ·
                WORKER_CLI_RS :65 · BIN_LIB_RS :66 · WORKFLOW :72  ← 2a, the gate reads YAML now
                ENROLLED_TEST_TARGETS :78  ← 2a, HAND-MAINTAINED. AC2.5(b) replaces it.
                RETIRED_SHARED_ORACLE :83 · CODEX_ORACLE :86 · CLAUDE_ORACLE :87
ledger_leg_names() :91-100  → FIVE names, hand-listed, NO derivation test (AC2.3)
struct Finding              → { check, detail }  ← still no ran/passed/failed (F2)
read() :111-128             → pushes a Finding when a governed file is MISSING  ← F3b's mechanism
contains_live() :138-140    → layout-SENSITIVE, single-token needles only; does NOT filter string
                              literals (that is why 1a's boundary leg is pinned by router.rs:1514)
production_before_tests() :142-145 → src.split_once("\n#[cfg(test)]"); NO-OP on a tests/ file
structural() :159-165       → strip comment lines, strip ALL whitespace
   :171 (and :348)  structural(production_before_tests(src))   ← THE COMPOSED IDIOM. USE THIS.
leg_frame_borne_route_intact()       :181-299
leg_loopback_from_host_unverified()  :300-342  ← BYTE-IDENTICAL to 5a921c0c. The null control
                                                 AC2.2a repairs. Returns `true` as a boundary
                                                 observation, PUSHES a Finding if it MOVES.
leg_completion_oracle_per_adapter()  :343-416  ← 2a
leg_worker_cli_under_library()       :417-468  ← 2a (note :449 forbids an in-src #[cfg(test)])
leg_completion_vectors_enrolled()    :469-514  ← 2a. THE ENROLLMENT LEG. It already exists.
run() :515-518 → run_with_root(json, Path::new("."))
run_with_root() :519-575 → five legs → `oracle_green = findings.is_empty()` :529
                         → dev_enforced_red_blocks(Blocking, true)
                         → JSON :536-548 → Ok(()) | Err(...)
```

Gates return `Result<(), String>`; there is no third exit code. **This gate has no `#[cfg(test)]`
module of its own** — the derivation test AC2.3 asks for is net-new, and belongs in `xtask/tests/`.

### Where the code goes

| Concern | File | Anchor |
|---|---|---|
| **NEW** consent proofs | `crates/maos-bin/tests/consent_refusal_1b.rs` | NEW — free, must be `--test`-enrolled |
| Production pairing helper (use it) | `crates/maos-a2a/src/pairing.rs` | `LoopbackEndpoint` (all fields `pub`), `paired_loopback_router` |
| Production identities (use them) | `crates/maos-bin/src/delegation.rs` | `RECIPIENT_SPIRIT` :53 · `FROM_SPIRIT` :57 · `TO_HOST` :60 · `FROM_HOST` :65 · `install` :102 · `delegate` :144 |
| Canonical intent (single definition) | `spirits/orchestrator/src/lib.rs` | `DELEGATION_CONSENT_INTENT` :73 |
| Test harness to copy | `crates/maos-bin/tests/delegation_leg_1a.rs` | `harness()` :31-47, `delegation_frame()` :49-69 |
| Allowlist enforcement (send) | `crates/maos-a2a-core/src/router.rs` | `send_admits` :633-646, call :749, deny :750-759 |
| Allowlist enforcement (accept) | `crates/maos-a2a-core/src/router.rs` | `accept_admits` :648-665, call :1313, NACK :1328-1337 |
| Source-keyed peer resolution | `crates/maos-a2a-core/src/router.rs` | :1087-1090, `lookup_peer` :1093 |
| Unclassified (send / accept) | `crates/maos-a2a-core/src/router.rs` | :696-701 / :1235-1244 |
| Expiry stamp / check | `crates/maos-a2a-core/src/router.rs` | :866-871 / :1208-1222 |
| Classification (private) | `crates/maos-a2a-core/src/router.rs` | `consent_decision` :534-552, `ConsentDecision` :119-126 |
| **Conflation to file, not fix** | `crates/maos-a2a-core/src/router.rs` | `map_a2a_error_to_iac_bus` :1671-1783 |
| Deny codes | `crates/maos-a2a-core/src/transport/json_rpc.rs` | `-32001` :30, `-32009` :62 |
| Error variants | `crates/maos-a2a-core/src/error.rs` | `IntentDenied` :70-75, `IntentDeniedAtPeer` :136-138, `ConsentUnclassified` :166-169 |
| Non-conflation precedent | `crates/maos-a2a-core/tests/fail_closed_8_8.rs` | `classified_not_allowlisted_is_minus_32001_not_minus_32009` **:216**, reason assert **:128-135**, `nack_round_trips_to_consent_unclassified_at_peer` **:287**, `exactly_128_bytes_is_classified` **:397** |
| Assertion source (not shape) | `crates/maos-bin/src/main.rs` | `smoke_a2a_consent_vocab_8_7` **:11069** (positive **:11175-11195**) · `smoke_a2a_fail_closed_8_8` **:11264** (`-32001`-is-failure **:11403**). **Both are `network`-gated binary arms with ZERO CI invocation — copy the assertions, never the shape (F7)** |
| Gate (EXTEND, not create) | `xtask/src/check_j1_loopback_delegation.rs` | see anatomy above — 5 legs, 10th governed file is yours |
| Vacuity primitive home (NEW) | `xtask/src/gate_common.rs` | 233 code; 7 exported fns, **no per-leg record** — AC2.2 |
| Proven-red **#1** (EXTEND `lay_green`) | `xtask/tests/j1_crosshost_1a_proven_red.rs` | `lay_green` **:95**, `GOOD_*` :42-:83 (**nine**), baseline **:157**, 11 tests |
| Proven-red **#2** (EXTEND `lay_green` TOO) | `xtask/tests/j1_crosshost_2a_proven_red.rs` | `lay_green` **:105**, consts :40-:48, `GOOD_*` :50-:93, 15 tests — **F3b: this file is why AC2.4 changed** |
| Proven-red **#3** (CREATE) | `xtask/tests/j1_crosshost_1b_proven_red.rs` | NEW, per-story precedent from `2a`; own CI step |
| Demo coupling | `xtask/src/demo_j1.rs` | `evaluate_beats` :559, `run_delegation_gate` **:793-813**, `unlanded_beats` **:815-838**, this story's ABSENT beat **:817-821** |
| Demo suite | `xtask/src/tests/demo_j1_tests.rs` | `beats.len() == 9` at **:254**; ABSENT-owner assertions :15-24 |
| CI job (add one `--test`) | `.github/workflows/discipline.yml` | job **:1804**, gate **:1814-1815**, proven-red `1a` **:1816-1817**, proven-red `2a` **:1818-1819**, delegation legs **:1820-1823** (the `-p maos-bin` line is **:1823**), `2a` completion legs **:1833-1842** |

### The six enrollment surfaces — all correct at HEAD, VERIFY only

*(Re-derived at `0769869d`.)*
(a) `xtask/src/main.rs` `mod` + `#[command(name=…)]` + dispatch arm ·
(b) `discipline.yml` **:1804** job, **:3198** `v1-0-ship-gate` needs, **:3257** + **:3286** echo
tables ·
(c) `xtask/gate-registry.toml` **:100** flat list, **:280** `[[ship_gate]] name =`
(`disposition = { v1_0 = "blocking", v1_5 = "blocking" }`) ·
(d) `xtask/src/check_ship_gate_completeness.rs` `EXPECTED_GATES` ·
(e) `tests/coverage-matrix.yaml:472-473` (FR23a row; already names the gate and this story) ·
(f) `crates/maos-bin/src/env_contract.rs:420` (`MAOS_LIVE_AGENT`), **:425** (`MAOS_HOST_GRANTS`) —
both still correct.
**(g) `xtask/src/lib.rs` is NOT a surface** (F9) — the registry declares only the GA ladder; the
dev-time binding lives in code at `check_j1_loopback_delegation.rs:308`.

### What this story does NOT do

- **No mechanism** — the wire, router install, pump, and env deletion are `1a`'s.
- **No fix to `map_a2a_error_to_iac_bus`** — filed for rung 2 (AC1.5b); `maos-a2a-core` is at zero
  headroom and D10 forbids a third unscoped grant.
- **No new protocol, no mTLS, no second host** — `j1-crosshost-2`.
- **No enforced egress** — stays `declared-not-enforced`.
- **No repair of the `maos-kernel-core` / `maos-domain` / aggregate reds** — D13 / D14 / unowned.
- **No FR23a corpus closure** — 30 scenarios is not this story.
- **No widening of digit-prefix gate scoping** — Epic 14's instrument work (D11).
- **No extension of `ledger_gates()` / `CONTRACTS`** to make a J1 evidence ledger publishable.

### References

- [Source: `_bmad-output/planning-artifacts/sprint-change-proposal-2026-07-16.md#4.1`] — the ratified card (its AC4 refusal shape is corrected here: smoke-8-8 was the wrong precedent).
- [Source: `_bmad-output/implementation-artifacts/j1-crosshost-1a-frame-borne-delegation.md`] — predecessor; `DelegationLeg`'s `intent` parameter was added explicitly for this story.
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/7-inter-agent-communication.md#7.2`] — normative: unconditional fail-closed on unclassified at BOTH seams; `-32009` distinct from `-32001`; no band-fallback opt-in; no auto-retry.
- [Source: `docs/adr/ADR-012-typed-intent-a2a-consent.md`] — consent is `(peer-identity, intent-class)`; the ADR names no allowlist keys.
- [Source: `_bmad-output/planning-artifacts/prd/functional-requirements.md#FR23a`] — 30 consent scenarios, 100% disallowed blocked (**not** closed by this story).
- [Source: `_bmad-output/planning-artifacts/prd/user-journeys.md#Journey-1`] — J1; note its "remote Host's allowlist" phrasing is wrong for loopback (source-keyed).
- [Source: `_bmad-output/planning-artifacts/epics/epic-14-preflight-decisions.md`] — D10 (`maos-a2a-core` zero headroom), D11 + D11-E1/E2/E3, D13, D14, **D15 (this story's grant)**.
- [Source: `xtask/kloc.toml#58-65`] — grants are measured, never estimated; slack is not authorization.
- [Source: `xtask/src/gate_common.rs#139-146`] — `PROVEN_BLOCKING` / `ABSENT` / `INDETERMINATE` spellings rung 2 consumes.

---

## Dev Agent Record

### Agent Model Used

_(record `vendor/model` + harness + date — required by policy even though **seven** gates skip this
filename; see AC4.1, which makes the full manual disclosure this story's stated price for D19 not
blocking it. The set to record: **dev model** · **reviewer model, which must differ** · **all four
§A6 layers named** · **the runtime-execution check** · **the in-repo path of the review artifact**.
A green CI asserts none of it.)_

**DEV PASS — recorded manually, because seven blocking gates cannot see this filename (D19).**

| Field | Value |
|---|---|
| **Dev model** | `anthropic/claude-opus-5` |
| **Harness** | Oh My Pi coding harness, `bmad-dev-story` skill (BMM 6.10.0) |
| **Date** | 2026-08-16 |
| **Baseline** | `0769869d` — pinned in this file's frontmatter and **verified equal to HEAD**; `git log 0769869d..HEAD` over `xtask/src/check_j1_loopback_delegation.rs`, `xtask/src/demo_j1.rs`, `.github/workflows/discipline.yml`, `xtask/kloc.toml` and `xtask/tests/j1_crosshost_*` returned EMPTY, so the re-baselined "what 2a moved" table stands unaltered (Trap 26 discharged by execution, not by assumption) |
| **Reviewer model** | **NOT YET RUN — MUST DIFFER from `anthropic/claude-opus-5`.** `2a` used `zai/glm-5.2`; any frontier-class model that is not the dev model satisfies the constraint |
| **§A6 layers required** | Blind Hunter · Edge Case Hunter · Acceptance Auditor · **Test-Infra Auditor** · **runtime-execution check** — all five, NON-DEGRADABLE (consent/A2A surface) |
| **Review artifact path** | `_bmad-output/implementation-artifacts/j1-crosshost-1b-consent-proofs-and-gate.md` → `### Review Findings` (in-repo, this file). A separate artifact path must be recorded here if the reviewer writes one elsewhere |
| **D19** | **CITED AS OPEN.** Owner **14-0**, which does not yet exist as a story. This story's compliance with story-file discipline is asserted **by a human reading this table**, not by any gate. D19 is NOT closed here and no gate's scope was widened (that is 14-0's single-source decision, and patching one of seven walkers is the defect this project has already paid for twice) |

**Why the row reads `review` and not `done`.** AC4.1 says "set this story's `sprint-status.yaml` row to `done`". That instruction assumes dev and review are one pass. They are not, and §A6 is non-degradable on this surface, so the dev pass sets **`review`** and the reviewer sets `done` — `1a`, `j1-demo` and `2a` all closed that way. Recording `done` before a differing-model four-layer review would be exactly the "disclosed instead of controlled" move D19 exists to stop.

> **§A6 is non-degradable here, and the Test-Infra layer is the one that matters.**
> `epic-10-process-agreements.md:26-30`: *"A degraded review is not a review. Main-session
> self-review does not satisfy §A6."* Four layers — Blind Hunter · Edge Case Hunter · Acceptance
> Auditor · **Test-Infra Auditor** — plus, for security surfaces, a **runtime-execution check
> confirming the code is actually exercised in CI, not merely compiled.**
> This is not boilerplate on this story. `1a`'s §A6 pass **omitted Test-Infra** (its record,
> `:16-17`); `j1-demo` ran it voluntarily and **it is what caught the gate bypass** that a
> `#[cfg(test)]` relocation could walk through. Given F1–F3 (a static oracle, a missing vacuity
> guard, and a harness that silently vacuums when a leg shells out), Test-Infra is the layer with
> the highest yield here. Run all four.

### Debug Log References

**Falsifiability evidence — every control was proven able to FAIL before being trusted.**

1. **AC1's refusal proofs (mutation testing).** The 7 tests passed on first run, which on its own
   proves nothing about a suite asserting behaviour that already exists. Four mutations were planted
   and reverted:
   - accept-allowlist made SYMMETRIC (source admits the disallowed intent) →
     `disallowed_intent_is_denied_at_peer_with_minus_32001_naming_the_source_host` and
     `classified_not_allowlisted_is_minus_32001_never_minus_32009` both RED;
   - expected `UnclassifiedReason::NonCanonical` swapped to `Absent` → both `-32009` seam tests RED.
   All four reverted; 7/7 green after.
2. **The gate's consent leg, proven RED before enrollment landed.** With
   `crates/maos-bin/tests/consent_refusal_1b.rs` present but `--test consent_refusal_1b` NOT yet in
   the workflow, the gate reported `passed:false` with exactly one finding, from
   `completion-vectors-enrolled`, naming the un-enrolled target. That is the derivation working
   before the fix, not after.
3. **A LAYOUT-SENSITIVITY BUG IN MY OWN NEEDLES, caught by my own reformat vector.** The first
   version of the leg needled `A2AError::IntentDeniedAtPeer{peer,message}` **with** the closing
   brace. `reformatting_the_refusal_proofs_does_not_flip_the_leg` failed, because `cargo fmt` adds a
   TRAILING COMMA when it breaks a pattern across lines: `{peer,message,}`. That is precisely the
   `246660f9` false-alarm class this gate was already repaired for once. **Fix: no needle in the leg
   carries a closing delimiter.** Had that vector not existed, a future `cargo fmt` would have RED a
   `Blocking` gate against code that was still correct.
4. **The vacuity primitive's own falsifier.** Emptying `crates/maos-bin/tests/` in the fixture tree
   makes the derived enrollment set empty, so the leg iterates nothing, evaluates NO check and pushes
   NO finding — under `oracle_green = findings.is_empty()` alone the gate would have been **GREEN**.
   `emptying_the_derived_enrollment_set_must_red_as_VACUOUS` asserts both the redness and the guard's
   own wording, and `every_leg_publishes_a_non_zero_check_count` is its positive control.
5. **Runtime execution, not compilation.** `cargo run -q -p xtask -- demo-j1` was executed
   end-to-end (46s, exit 0): six distinctly-named leg beats with real per-leg check counts
   (7 · 2 · 5 · 3 · 5 · 13) and `disallowed-intent-refused-blocking` rendering **PROVEN_BLOCKING**.
   `ci_local_split_refuses_a_granted_real_agent_without_the_live_flag` was executed by exact name
   (1 passed, 2 filtered) with no secret and no network.
6. **CI parity.** Every step of the `check-j1-loopback-delegation` job was run locally in job order,
   including the two new ones. All three YAML files re-parsed with `yaml.safe_load` after editing —
   which caught a real break: `run: cargo test -p xtask demo_j1:: …` is invalid YAML (colon-space in
   a plain scalar), fixed to `demo_j1`.

**Not fixed, deliberately:** `cargo test -p maos-bin` remains RED under default parallel flags
(D16 / F12, the `MAOS_HOME` process-global isolation defect across three unrelated 13.x files). The
new file was run scoped (`--test consent_refusal_1b`) and is itself isolation-free — it touches no
environment variable — so `--test-threads=1` on the CI line is defence for its neighbours, not for it.

### Completion Notes List

**AC1 — refusal proofs.** `crates/maos-bin/tests/consent_refusal_1b.rs`, 7 tests, **+0 kloc**.
Drives the REAL production configuration throughout — `maos_a2a::pairing::{paired_loopback_router,
LoopbackEndpoint}` and `maos_bin::delegation::{TO_HOST, FROM_HOST, RECIPIENT_SPIRIT, FROM_SPIRIT}` —
with frames built by the production `Orchestrator::assign_frame_remote`, so nothing drifts against a
hand-copied identity string.
- **AC1.1** local positive control in the same binary: the granted intent is admitted and its
  delivered `intent_class` asserted equal. `journey_j1.rs:107` and `delegation_leg_1a.rs:75-87` were
  NOT rebuilt.
- **AC1.2** `-32001` reached the only way it can be (F4): a hand-built **asymmetric** pairing where
  the destination's `send_allowlist` carries the disallowed intent and the source's
  `accept_allowlist` does not. Asserts `IntentDeniedAtPeer { peer, message }` with
  `peer == "developer-remote-host"` while the NACK **message names `founder-loop-host`** — the
  SOURCE host, produced at `router.rs:1331-1336`. Both `peer_id` strings are pinned literally.
- **AC1.3** `-32009` at BOTH seams. Send seam via `route_outbound` (`prepare_outbound` denies before
  the allowlist); accept seam driven through `handle_intake` DIRECTLY, because `prepare_outbound`
  would never let an unclassified frame reach intake. All reachable `UnclassifiedReason`s covered —
  `Absent` **twice** (no envelope, and an envelope with `intent_class: None`, which are different
  production bugs), `NonCanonical`, `Oversized` — with `data.reason` deserialized to the typed
  variant, never a numeric-only assertion.
- **AC1.4** non-conflation asserted in BOTH directions plus the third code: `-32001` never reported
  as `-32009`, `-32009` never as `-32001` (at both seams), and an expired envelope yields `-32003`
  and neither deny code.
- **AC1.5** both non-coverage statements written into `j1-crosshost-2b`'s **file** (new
  "Inherited from `j1-crosshost-1b`" section, after its Blocking conditions) **and** its
  sprint-status row. **D18 verified RESOLVED** (2026-08-15, owner John + Vex, target 14-4, deadline
  re-pinned to "before `j1-crosshost-2b` writes its first line") — not re-filed, not re-opened.
  Resolved-as-a-decision is not resolved-as-code: the conflation is live at HEAD, which is why AC1.4
  asserts at the router seam.

**AC2 — gate legs, with the vacuity hole closed.**
- **AC2.1** sixth leg `consent-refusal-proofs`, 13 structural needles via the composed idiom
  `structural(production_before_tests(src))`, root-relative, no `Command`, no `cargo`.
  `consent_refusal_1b.rs` is the gate's tenth governed file. Trap 24 confirmed by reading:
  `production_before_tests` is a genuine NO-OP on a `crates/*/tests/` file; what actually protects
  the needles there is that `structural` strips comment lines, so the module docs — which mention
  every code and variant name — satisfy zero needles.
- **AC2.2a — the null control REPAIRED, and it was worse than the story recorded.**
  `leg_loopback_from_host_unverified` could not fail: both needles are permanent features of
  `router.rs`, one of them pinned by `handle_intake_verified`'s **own TLS-mismatch message literal**
  at `:1514`, and the leg could not observe its declared trigger at all, because "rung 2 turns
  verification on" is a change of composition root, not a text change in `router.rs`. It now reads
  `crates/maos-bin/src/delegation.rs` (`paired_loopback_router` present, `maos_a2a_tcp` /
  `handle_intake_verified` absent) **and** `router.rs`'s peer-RESOLUTION EXPRESSION
  (`letpeer_host=match&frame.from.host_id{`, which the `:1514` literal cannot satisfy). Both doors
  are now plantable: `1a`'s retargeted vector covers the shared-intake door, `1b`'s new
  `boundary_leg_reds_when_the_composition_root_gains_a_verified_transport` the composition-root door.
  The three documents that claimed "when rung 2 turns verification on, this leg flips" are corrected
  (this gate's module doc, the `2b` inherited section, the `2b` sprint row).
- **AC2.2** shared primitive `LegAudit` + `vacuous_legs` in `xtask/src/gate_common.rs`. Fields are
  private, so no gate can mint a check count it did not perform — the same compile-time guarantee
  `EvidenceVerdict` gives the evidence projection. This gate is the first consumer across **all six**
  legs, including `2a`'s three, which shipped with no vacuity record. Published as `leg_audits` in
  the JSON. Other gates NOT migrated (14-6).
- **AC2.3** the `ledger_leg_names()` derivation test landed as
  `ledger_leg_names_reconciles_with_the_legs_actually_invoked` **inside
  `xtask/tests/j1_crosshost_1b_proven_red.rs`** rather than a new target — a new `xtask/tests/` file
  is one more thing to forget to enroll, which is the failure this story exists to close. The other
  half of the reconciliation is now a COMPILE error: `judge()` destructures
  `ledger_leg_names()` into exactly six audits, so adding a leg to one side and not the other does
  not build.
- **AC2.4** `xtask/tests/j1_crosshost_1b_proven_red.rs`, **22 vectors**, own CI step. F3b's
  ship-blocker paid FIRST: the tenth governed file **and** the three derived enrollment targets are
  laid in **all three** `lay_green` functions, and `baseline_fixture_tree_is_green` verified green in
  each (`1a` 11/11, `2a` 15/15, `1b` 22/22 = **48** vectors). `GOOD_DELEGATION` gained the pairing
  call in both existing files (the repaired leg 2 reads it), and both files' `GOOD_WORKFLOW` plus
  `2a`'s two inline workflow literals gained the third `--test` line — otherwise `2a`'s reformat
  vector would have red for the wrong reason.
- **AC2.5** `--test consent_refusal_1b` added to the J1 job's delegation-legs step. **AC2.5(b)
  ratified shape implemented: `ENROLLED_TEST_TARGETS` DELETED**, replaced by a derivation over
  `crates/maos-bin/tests/*_{1a,1b,2a}.rs` keeping `2a`'s job-scoping exactly. At HEAD it derives five
  targets, all enrolled. Two falsifiers: delete the flag → RED; plant an un-enrolled
  `crates/maos-bin/tests/xyz_1b.rs` → RED naming `--test xyz_1b`. An empty derived set is caught by
  AC2.2's guard, which is what stops a filesystem walk from going quietly decorative.
- **AC2.6** closed by EXACT NAME per the `discipline.yml:2599` idiom, reusing `2a`'s
  `worker-cli-fixture` pre-build lesson by NOT enrolling the whole file. Verified executing.
- **AC2.7** no new env var. **AC2.8** all six surfaces verified correct, none touched;
  `xtask/src/lib.rs` verified ABSENT (F9). **AC2.9** job stays hermetic, no `services`, no `if:`.
- **AC2.10/2.11** the demo emits **one beat per leg** plus the flipped
  `disallowed-intent-refused-blocking`, removed from `unlanded_beats()` **in code** (never via a
  ledger — `ledger_gates()` is structurally unreachable for J1). The flipped beat is a CONJUNCTION:
  assertions-present AND enrolled-in-CI, because a static oracle cannot claim a frame was refused.
- **AC2.12 — the story's premise was WRONG, and it was checked rather than obeyed.**
  `demo_j1_tests.rs`'s `beats.len() == 9` is asserted over `evaluate_beats`, which is
  **event-derived**; `unlanded_beats()` and `run_delegation_gate()` are extended onto `beats` in
  `run()` and are NOT part of it. Flipping the beat therefore does **not** trip the constant, and it
  is left at 9 **deliberately, after measuring** — the suite is green with it unchanged. What DID
  break was `the_two_host_rung_is_owned_by_crosshost_2`, because the `two-host-signed-run` beat
  cannot name an owner that no longer has a row (F16); it is retargeted to `j1-crosshost-2b` and a
  new assertion pins the refusal beat out of `unlanded_beats()`.

**AC3 — budget. NO GRANT TAKEN, and the estimate was wrong.**

| Key | Ceiling | Baseline `0769869d` | At close | Verdict |
|---|---:|---:|---:|---|
| `xtask` | 38609 | 38386 (+223) | **38604** | **GREEN — +5 headroom.** This story spent **218**, not the estimated 110-145 |
| `maos-bin` | 16260 | 16260 | **16260** | GREEN, **delta +0** as AC3.2 requires |
| `maos-cli` | 4642 | 4642 | **4642** | untouched |
| `maos-a2a-core` | 4654 | 4654 | **4654** | untouched — no production line, D10 honoured |
| `maos-domain` | 8644 | 8694 | **8694** | RED +50 — **D14**, not this story's |
| `maos-kernel-core` | 18248 | 18933 | **18933** | RED +685 — **D13**, not this story's |
| `_aggregate_hardfail` | 147057 | 147942 (+885) | **148196 → 148160** | **RED +1103 — grant REFUSED** |
| `check-kernel-baseline` | 24472 | 24472 | **24472** | GREEN — real measured evidence |

- **The overshoot is reported, not smoothed.** First implementation measured `xtask` at **38685 —
  RED, +76 over the ceiling**. `kloc.toml:60-65` forbids a grant on an estimate and the story
  forbids an `xtask` grant outright, so **71 code lines were trimmed back out** rather than asked
  for: the 13-needle table lost its paraphrase column (the needle names the assertion more precisely
  than a description of it, and the reasoning moved into comments, which are not charged), `judge()`
  lost ~20 lines of `audit_of` ceremony to a slice pattern that is now also the compile-time
  reconciliation, `Judgement` carries `Finding` instead of re-mapping it to tuples twice, and the
  demo narrations were shortened. Nothing was deleted that carried a control.
- **+5 headroom is a finding, not a comfort.** The next `xtask` story cannot land a single line
  without a measured grant. AC3.1's own argument inverted twice: "+534, ~2.4× the gate module" →
  "+223 vs a 362-line module, 0.6×" → in fact **218 spent against 223**.
- **AC3.3/3.4 aggregate REFUSED**, on the story's own grounds: this story's contribution to the
  measured aggregate is its authorized-and-in-budget `xtask` work, the +1103 is arithmetic downstream
  of D13/D14/`2a`'s grants plus real growth, and re-basing it would green the only instrument holding
  three named debtors to account. **D17's row still records 147549 / +492; measured is 148160 /
  +1103. Drift REPORTED, row NOT edited** (Binding rule 1).
- **AC3.7 ABI, stated honestly.** `abi-diff --base abi-baseline/v1-pre-bump.txt` and
  `check-abi-ratification` are GREEN (0 changes / 3 ratified / 0 uncovered) **and structurally blind**
  — `xtask/src/abi_diff.rs:8` scopes to `crates/maos-spirit-abi/Cargo.toml` only, so they cannot see
  `maos-kernel-core`'s re-exported surface (**FLAG-E4**). This story made **no ABI change**: that is
  a fact about the diff, not a gate result. No kernel seam touched, no implicit re-pin.
- **Trap 25 honoured**: `xtask/kloc.toml` was NOT edited. Its `:203` annotation still ends "Current
  headroom 534", which was already stale at `0769869d` and is now stale by 529. Left for whoever
  next takes a measured `xtask` grant.

**AC4 — closing the lane.** Sprint row → `review` (see the table above for why not `done`); the two
inheritances written into `2b`'s file and row; **D17 OPEN**, **D18 RESOLVED**, **D11 OPEN with its
counts verified correct at HEAD (`EXPECTED_GATES` = 37, `check_*.rs` = 68 — the ratio still asserts
nothing, Trap 12)**, **D19 OPEN and priced manually**; FR23a `notes` updated with `corpora` left
deliberately EMPTY and no claim of the 30-scenario floor.

---

### Three findings this story did not go looking for

**FOUND-1 — `check-env-contract` is RED at HEAD, and it is not this story's.**
`crates/maos-bin/src/main.rs:3254-3255` reads `MAOS_OPERATOR_BEARER_TOKEN` and
`MAOS_OPERATOR_HTTP_BIND`, neither registered in `env_contract.rs`; the gate reports
`passed: false` with those two violations. **Verified PRE-EXISTING** by `git show
0769869d:crates/maos-bin/src/main.rs` — both reads are there at the baseline, and this story touched
neither `main.rs` nor `env_contract.rs`. NOT fixed: registering them is a `maos-bin/src` edit against
a crate at EXACTLY zero headroom, which AC3.2 pins at +0. It is a blocking gate that is red at HEAD
and needs an owner.

**FOUND-2 — D11-E3's citation is half wrong, and the wrong half is the one AC2.3 leaned on.**
The story cites D11-E3 as "`xtask/src/tests/` measures 2367 charged CODE lines". **Measured: that
directory is PATH-EXCLUDED from the budget.** `xtask` measures **38604**, which equals the sum of
`xtask/src` tokei CODE lines *excluding* `xtask/src/tests/` exactly; including it gives 40981. The
`tests/` rule at `kloc.toml:2` frees any `tests/` path segment, not only `crates/*/tests/`. So D11-E3
is correct about `#[cfg(test)]` modules inside **ordinary** `src` files (charged) and wrong about
that directory. **The CI-invisible half is TRUE and worse than recorded:** no job runs
`cargo test -p xtask` unscoped — every invocation names a test or a `--test` target — so ~490 in-`src`
`xtask` tests, including the entire demo beat ledger, were **executed by no CI job at all**. This
story's own AC2.11 assertion would have landed inside that dead zone, so
`cargo test -p xtask demo_j1` is now enrolled in the J1 job (25 tests). Filed here against D11 for
14-6; the register row is NOT edited.

**FOUND-3 — a needle of mine was layout-sensitive, and only a vector caught it.** See Debug Log 3.
`cargo fmt` adds a trailing comma when it breaks a pattern, so `{peer,message}` would have stopped
matching still-correct code. Every needle in the new leg now ends before its closing delimiter. This
is the third appearance of the `246660f9` class in this gate; the second was `2a`'s.

### File List

**New**
- `crates/maos-bin/tests/consent_refusal_1b.rs` — the ADR-012 refusal proofs (AC1). Zero kloc.
- `xtask/tests/j1_crosshost_1b_proven_red.rs` — 22 proven-red vectors + the `ledger_leg_names()`
  derivation test (AC2.2-2.5). Zero kloc.

**Modified**
- `xtask/src/check_j1_loopback_delegation.rs` — sixth leg `consent-refusal-proofs`; AC2.2a repair of
  `loopback-from-host-unverified`; `ENROLLED_TEST_TARGETS` replaced by `derive_enrolled_targets`;
  `LegAudit` wired through all six legs; `Judgement`/`judge()` extracted so callers can report per-leg
  outcomes; `Finding` made public; `leg_audits` published in the JSON; module doc `## Legs` rewritten
  to cover all six legs and the vacuity rule.
- `xtask/src/gate_common.rs` — NEW shared vacuity primitive: `LegAudit` (private fields, `entered` /
  `checked` / `leg` / `checks` / `is_vacuous`) and `vacuous_legs`.
- `xtask/src/demo_j1.rs` — one beat per gate leg via `judge()`; `leg_narration`;
  `disallowed-intent-refused-blocking` flipped out of `unlanded_beats()` and emitted as executed;
  `two-host-signed-run` retargeted to `j1-crosshost-2b`; the printed non-claims block corrected for
  the rung-2 split and the repaired boundary leg.
- `xtask/src/tests/demo_j1_tests.rs` — owner assertion retargeted to `j1-crosshost-2b`; new
  assertion that the refusal beat is no longer declared unlanded.
- `xtask/tests/j1_crosshost_1a_proven_red.rs` — tenth governed file + three derived enrollment
  targets laid in `lay_green`; `GOOD_DELEGATION` carries the pairing call; `GOOD_WORKFLOW` carries the
  third `--test`; boundary vector retargeted to the shared-intake door.
- `xtask/tests/j1_crosshost_2a_proven_red.rs` — same `lay_green` / `GOOD_DELEGATION` /
  `GOOD_WORKFLOW` repairs, plus the third `--test` in both inline workflow literals so `2a`'s
  reformat vector stays green for the right reason.
- `.github/workflows/discipline.yml` — `check-j1-loopback-delegation` job gains three steps: the `1b`
  proven-red vectors, the demo beat-ledger assertions, and the `smoke_cli_wrapper_8_12` control by
  exact name; the delegation-legs step gains `--test consent_refusal_1b`.
- `tests/coverage-matrix.yaml` — FR23a `notes` updated; `corpora` deliberately still empty.
- `_bmad-output/implementation-artifacts/sprint-status.yaml` — `j1-crosshost-1b` → `review` with the
  full dev record; new inheritance comment above the `j1-crosshost-2b` row; `last_updated`.
- `_bmad-output/implementation-artifacts/j1-crosshost-2b-cross-host-delegation-mechanism.md` — *(§A6
  review P12 correction: this file is NEW in the change, not modified — the diff adds all 806 lines;
  it was authored by the 2026-08-15 rung-2 split preflight and this story's dev pass added the
  "Inherited from `j1-crosshost-1b`" section carrying AC1.5's two non-coverage statements.)*
- `_bmad-output/party-mode/memories/installed/.memlog.md` — *(§A6 review P12: omitted from the
  original File List)* round-table memory entries from the 2026-08-16 re-baseline appended by the
  party-mode session, not hand-edited.
- `_bmad-output/implementation-artifacts/j1-crosshost-1b-consent-proofs-and-gate.md` — this file:
  tasks, Dev Agent Record, File List, Change Log, Status.

**§A6 review patch addendum (2026-08-16, applied by the reviewer, `zai/glm-5.2`)** — 13 patches,
all verified green: gate (`check_j1_loopback_delegation.rs`) gains the `#[tokio::test]`
registration count + 4 needles (positive control ×2, `assert_eq!(peer,TO_HOST,`,
`assert_eq!(nack_reason(&nack.error),expected,`), production-scoped router scan, token-bounded
executable-`run:` enrollment matching, the `!loopback_composed` flip guard, the named
`consent_refusal_proofs` JSON boolean, and the literal-bait Known-limit doc; `demo_j1.rs` emits the
refusal beat ABSENT under `--skip-gate`; `demo_j1_tests.rs` gains the emission test; `discipline.yml`
gains the `grep -q` zero-match guard on the exact-name control; all three proven-red trees lay the
`_1a` targets and 5 new vectors (27 total in `1b`); `1a`'s story record, `2b`'s inherited section +
G15 + AC4.1/T10/chain-table, and the sprint INHERITED comment corrected. **Named measured grant
taken per `kloc.toml:87`: `xtask` 38609 → 38655** (exact measured, zero headroom) — the aggregate
grows by the same +51 and stays D17's standing red. Verification: gate PASS · `1a` 11/11 · `2a`
15/15 · `1b` 27/27 · `demo_j1` 26/26 · `consent_refusal_1b` 7/7 · `cargo fmt --all --check` clean ·
`kloc-check` red on exactly the three attributed keys.

---

## Open Questions for Lunarpulse

**None blocking. All closed — Q1/Q2 at the 2026-08-15 round-table, Q3/Q4 at the 2026-08-16
re-baseline.**

- **Q1 — the `maos-bin` grant: RESOLVED, then SUPERSEDED, and there is nothing left to ask.**
  D15 ratified 16219 (exact measured, zero headroom) on 2026-08-15 over the formula's 16544. `2a`
  then took 16219 → **16260** on 2026-08-16, same posture, ratified after measurement, live at
  `kloc.toml:264`. `maos-bin` is GREEN at `0769869d`. The dev does not request a grant — see AC3.2
  for what the zero constrains.
- **Q2 — rung 2 exists now, and it is three stories.** `j1-crosshost-2` was split (2026-08-15) into
  `2a` (**done**, `0769869d`), `2b` and `2c`; `2b` and `2c` have story files. So AC4.2's inheritance
  lands in a real document — **`2b`**, not `j1-crosshost-2`, which no longer has a tracker row at all
  (F16). D18's deadline was re-pinned to `2b` for the same reason.
- **Q3 — enumerate or derive the CI-enrolled test set? RATIFIED: DERIVE** (AC2.5(b)). `2a` shipped
  the enrollment leg this story was told to build, driven by a hand-maintained
  `ENROLLED_TEST_TARGETS` const. Appending one string would make this story's falsifier fire and
  leave the next author one forgotten const away from a dead test behind a green gate — the
  `smoke_cli_wrapper_8_12` failure re-created inside the gate built to prevent it. Derivation from
  the filesystem costs ~15 lines against +223 headroom and deletes the const. **Fallback if refused
  at review: const append PLUS a named Trap — recorded as the weaker answer, not chosen.**
- **Q4 — does D19 block this story? NO, and the refusal is on the record with its price** (AC4.1).
  D19's literal deadline (*"before the next `j1-*` story leaves `ready-for-dev`"*) names this story,
  because it was filed one day after this story got there and owner 14-0 does not yet exist. Stalling
  the lane's only unblocked story behind an unwritten instrument story is an outage, not a control.
  **What is not refused is the cost:** AC4.1 requires the dev to record manually, by name, exactly
  what the seven blind gates would have checked, and to cite D19 as OPEN.

Also settled by the round-table and written into the ACs: the aggregate breach is a standing red with
three named debtors and is **not** this story's to clear (**D17** filed); the per-leg gate surface is
in scope and carries the shared vacuity primitive; AC3 folded into AC2/AC4, leaving four ACs.

---

## Change Log

| Date | Change |
|---|---|
| 2026-08-14 | Created by splitting `j1-crosshost-1-loopback-developer-remote-delegation` (split ratified by Lunarpulse at preflight). Carries the judge: consent refusal proofs + the blocking gate + the deferred, measured `xtask` grant. |
| 2026-08-14 | **Round-table consensus applied.** Gate skeleton moved to `1a`; budget rationale dissolved on measurement; NEW stated non-coverage on peer authentication; intent widened to `development-task:write-workspace`; D11 verification added. |
| 2026-08-15 | **Re-grounded at `5a921c0c` by a five-scout preflight; thirteen premises disproved or corrected.** (F1) The gate is a **static source-text oracle** — it runs no tests; AC2 rewritten from "cargo legs" to source-structural legs + CI enrollment. (F2) **No vacuous-green guard exists** in this gate — AC2.2 changed from "extend" to "build", and it now supplies the per-leg record. (F3) **Ship-blocker:** copying `check_vetting_attestation::invoke_leg` would inherit the proven-red tempdir and silently vacuum every planted vector; new legs must stay root-relative. (F4) `-32001` is **unreachable through `DelegationLeg::install`** (one intent configures both allowlists, so the send seam denies first) — AC1.2 now builds an asymmetric pairing. (F5) `map_a2a_error_to_iac_bus` **conflates both deny pairs**, so the non-conflation assertion moves to the router seam and the lossy mapping is filed for rung 2 as new non-coverage AC1.5(b). (F6) **Envelope expiry is NOT a gap** — TTL is stamped at `router.rs:866-871` and enforced at `:1208-1222` via a third code `-32003`; the old AC1.7 is deleted and replaced by a three-code distinctness assertion. (F7) Both `smoke-*` precedents are **network-gated binary arms with zero CI invocation**; copying their *shape* reproduces a null control. (F8) Budget was stale by a full grant: `xtask` headroom is **+534**, not ~29; four crates are red, not one. (F9) `xtask/src/lib.rs` is **not** an enrollment surface — six, not seven. (F10) `deferred-work.md` names this story **nowhere**; the stale-owner hazard is unfounded. **NEW AC3** — `demo_j1` carries an `ABSENT` beat named `disallowed-intent-refused-blocking` **owned by this story**, and collapses all gate legs into one beat, so a consent red would be reported under the wrong name; the evidence-ledger route to flip it is structurally unreachable. **AC4** now carries the **D15 `maos-bin` +41 grant** (measured per-commit) and names the **unowned aggregate breach**. **AC5.5** forbids claiming FR23a corpus closure (`corpora: []` vs a 30-scenario floor). (F11) **`abi-diff` / `check-abi-ratification` are structurally blind to this surface** — they scope to `crates/maos-spirit-abi` only (`abi_diff.rs:8`), open **FLAG-E4** — so AC4.5 no longer cites them as evidence, and `kernel_grant` was rewritten from "verified flat" to "no change was made". (F12) **`cargo test -p maos-bin` is RED at HEAD under default parallel flags** (D16 `MAOS_HOME` isolation defect); added as a trap with the scoped-invocation workaround, plus FLAG-E5's 60s FR21 window for repeated local runs. (F13) Line/shape corrections re-derived throughout (`accept_admits` :648, proven-red idiom :2598, journey :1901, allowlist doc citation -> `pairing.rs`): the demo suite asserts `beats.len() == 9` at `:218` (not 7/`:199`), and `1a`'s boundary leg is **28 lines that RED when the boundary moves**, not "a 4-line record". **AC2.1 now mandates the composed idiom `structural(production_before_tests(src))`** (`:137`) — `structural()` alone was itself bypassable by relocating the production closure into a `#[cfg(test)]` module, fixed at `5a921c0c` and pinned by an 11th vector. **AC3.1 names the only viable flip mechanism** (remove from `unlanded_beats()` + emit an executed beat; the ledger path is unreachable). §A6 note added: `1a`'s pass omitted the Test-Infra layer, which is the layer that caught the demo's gate bypass. 3 ACs → 5. |
| 2026-08-15 | **Round-table consensus (Winston · Murat · Amelia · John · Mary · Sally · Paige · Vex · Dana · Grumbal); five decisions folded, 5 ACs → 4.** (1) **AC3 collapsed into AC2.10-2.12 and AC4** — the demo beat is a *correctness constraint on the close*, not a capability ("every fix a constraint, never a new AC"): if the legs land and `disallowed-intent-refused-blocking` still renders `ABSENT`, the narrated artifact prints a false claim about its own work. (2) **The vacuity guard becomes a SHARED PRIMITIVE in `gate_common.rs`** with this gate as first consumer — measured 16/68 gates carry one, `gate_common` carries none, all 16 hand-rolled; a 17th bespoke guard is how that number grew. ~20 lines, no framework, its own falsifier, and migrating the other 15 stays 14-6's. Net lines negative. (3) **NEW proven-red vector #12** — delete `--test consent_refusal_1b` from the workflow and the gate MUST red. Sally's question exposed why: the gate has *never observed a frame being refused* and cannot — it greps source text. The enrollment line is the only thing connecting the linter to the judge, and it had no falsifier. (4) **D15 grant set to `maos-bin = 16219` — exact measured, ZERO headroom**, over the formula's 16544: `main.rs` is 73% of the crate with no decomposition scheduled, tighter-than-formula is house style (the `j1-demo` grant records landing 4 lines tighter as the point), zero headroom is precedented and deliberate (`maos-a2a-core` 4654/4654 "hard-fail on contact"), and `kloc.toml:87` exempts correctness/compliance repairs so it traps no genuine fix. Dana's trap objection recorded and answered. (5) **The aggregate grant is REFUSED deliberately → D17.** Two claims from story-creation were disproved in the room: `kloc.toml:61` permits recalculation at a retrospective **or** under an authorized measured grant — two doors, both used by Stories 13.6d/13.6e and by the epic-orphaned `j1-demo` — so "bridge stories have no vehicle" was wrong; and the aggregate is not "unowned" but a standing red with three named debtors that would **not** clear even if all three re-based, because a ceiling move does not change the measured aggregate. 1b's contribution is **zero**, and granting it would green the CI signal holding D13's +685 to account — which D13 forbids of 14-6, and 1b is further from the delta. (6) **F5 conflation → D18** with an owner and the deadline *before rung 2 writes its first line*, not deferred "against `j1-crosshost-2`", which has a sprint-status row and **no story file**: you cannot defer into a document that does not exist. |
| 2026-08-16 | **RE-BASELINED at `0769869d` by a party-mode round-table (Winston · Amelia · Murat · John · Mary · Sally · Vex · Yui · Dana · Grumbal · Paige); still 4 ACs — every fix landed as a constraint on an existing one.** Story `j1-crosshost-2a` shipped *after* this story reached `ready-for-dev` and modified the same gate, the same CI job, the same demo and three of the same ceilings. Ten effects folded in; four new findings **F14-F17**; five traps **22-26**; two Open Questions **Q3/Q4** opened and closed in the room. **(1) NUMBERS — the explicit ask.** `xtask` headroom **+534 → +223** (AC3.1's "~2.4× the gate module" inverts to **0.6×**); `maos-bin` **16178/16219 RED +41 → 16260/16260 GREEN, ZERO** (D15's 16219 superseded by 2a's review grant); `maos-cli` **4642/4642 ZERO** added as a third wall; `_aggregate` **+492 → +885**; red keys **four → three**; gate **219 code / 2 legs → 362 code / 5 legs**; `demo_j1.rs` **938 → 1106**; and every line number in the anatomy block, the "where the code goes" table, the six enrollment surfaces and the `discipline.yml` references re-derived by reading. **(2) F14 — AC2.5's gate leg ALREADY EXISTS.** `2a` landed `leg_completion_vectors_enrolled` (`:469`) with a `WORKFLOW` const (`:72`), job-scoped by its own review. **Ratified Q3: DERIVE, do not append.** The leg is driven by a hand-maintained `ENROLLED_TEST_TARGETS` (`:78`); appending `"consent_refusal_1b"` would fire this story's falsifier and leave the next author one forgotten const from a dead test behind a green gate — `smoke_cli_wrapper_8_12`'s exact failure re-created inside the gate built to prevent it. AC2.5 now replaces the const with a filesystem derivation plus its own falsifier (plant an unenrolled `*_1b.rs` → gate MUST red). **(3) F3b — SHIP-BLOCKER CLASS, and the instruction that would have failed silently.** `lay_green` now exists **TWICE**, duplicated verbatim (`1a:95`, `2a:105`), each laying nine files. This story's consent leg registers a **tenth** governed file, and `read()` (`:111-128`) pushes a `Finding` on a missing one — so updating only the file named in the original AC2.4 reds the other's `baseline_fixture_tree_is_green`, at which point every planted vector in that file passes for the wrong reason while CI reports green. That is F3's failure mode arriving through a door F3 did not know existed. AC2.4 rewritten: new per-story `j1_crosshost_1b_proven_red.rs` (2a's precedent) **and** repair all three fixture trees; T7 re-ordered as a ship-blocker. **(4) F15 — D18 reads RESOLVED, not OPEN.** AC1.5b, AC4.3 and T3 all instructed the dev to *"confirm D18 still reads OPEN"*; it was resolved 2026-08-15 (the zero-headroom paradox died on measurement) and its deadline re-pinned to **`2b`**. Following the story as written, a dev would re-open a resolved decision. Corrected — with the distinction that matters: resolved as a *decision*, not as *code*, so AC1.4 still asserts at the router seam. **(5) F16 — `j1-crosshost-2` no longer exists, and this story's proudest sentence caught up with it.** AC1.5/AC4.2 deferred two non-coverage statements into the `j1-crosshost-2` sprint-status row while arguing at length that *"you cannot defer into a document that does not exist."* Rung 2 was split; the tracker's J1 keys are `1a`/`1b`/`2a`/`2b`/`2c`. Retargeted to **`2b`** on the merits (it is where a second host first authenticates a peer, so AC1.5(a)'s "a frame picks its own judge" becomes load-bearing there). New failure shape distinct from 13.6's "a residual stale at birth": **a residual that was true and expired** — checking harder at authoring would not have caught it. **(6) F17 / Q4 — D19's deadline literally names this story.** Filed by `2a`'s round-table one day after `1b` reached `ready-for-dev`, deadline *"before the next `j1-*` story leaves `ready-for-dev`"*, owner 14-0 not yet a story. Refused as a block (an outage is not a control) and **priced instead**: AC4.1 now requires the dev to record manually, by name, exactly what the seven blind gates would have checked — dev model, a differing reviewer model, all four §A6 layers incl. Test-Infra, the runtime-execution check, and the in-repo artifact path — and to cite D19 as OPEN. Gate-scope widening stays 14-0's single-source decision; patching one of seven walkers is the defect this project has paid for twice. **(7) F2's evidence was RE-TAKEN and the original method was wrong.** "16 of 68 gates carry a vacuity guard" came from `grep -l 'vacuous\|vacuity'` — a text match that counts doc comments; re-run it returns 19 of 68 and one new hit is *this gate*, which acquired the word from `2a` and still has no mechanism. **A census that counts prose is a claim standing in for a control.** AC2.2's justification restated as what was verified by reading: `gate_common.rs`'s seven exported fns contain no `ran`, no `checks`, no per-leg record. The substance survives and strengthens — `2a`'s three legs shipped with no vacuity record either, so this story is the first consumer across **five** legs, not two. **(8) AC2.3 got stronger for free.** `ledger_leg_names()` now hand-lists five names against five invoked legs, three added by `2a` by hand, in a file with **no `#[cfg(test)]` module at all** — the drift surface this AC guards tripled while nobody was looking. Budget note promoted from advice to plan: with +223 headroom the derivation test goes in `xtask/tests/`. |
| 2026-08-16 | **DEV PASS COMPLETE → `review`** (`anthropic/claude-opus-5`; baseline `0769869d`, verified equal to HEAD with an EMPTY `git log` over all five drift-prone paths, so Trap 26 is discharged by execution). **All 13 tasks and all 4 ACs satisfied.** DELIVERED: `crates/maos-bin/tests/consent_refusal_1b.rs` (7 tests, +0 kloc) proving `-32001` via a hand-built ASYMMETRIC pairing (the only route to the code) with the NACK asserted to name the SOURCE host, `-32009` at BOTH seams across every reachable `UnclassifiedReason` with the reason read back TYPED, non-conflation in BOTH directions, and `-32003` kept distinct; the gate's sixth leg `consent-refusal-proofs` (13 structural needles, tenth governed file); the AC2.2a REPAIR of `loopback-from-host-unverified`, which could not fail in any possible future and is now pointed at the composition root where rung 2's flip actually happens; the SHARED `LegAudit`/`vacuous_legs` primitive in `gate_common.rs` with this gate as first consumer across all SIX legs; AC2.5(b)'s DERIVATION replacing `ENROLLED_TEST_TARGETS`; 47 proven-red vectors green across three files with F3b's ship-blocker paid first; AC2.6's null control closed by exact name; one demo beat PER LEG and `disallowed-intent-refused-blocking` flipped to PROVEN_BLOCKING, verified by executing `xtask demo-j1`. **THREE PREMISES OF THIS STORY WERE MEASURED FALSE AND ARE CORRECTED IN THE RECORD, NOT WORKED AROUND:** (a) **AC2.12** — `beats.len() == 9` is asserted over `evaluate_beats`, which is event-derived and does NOT include `unlanded_beats()`, so flipping the beat cannot trip it; left at 9 after measuring, while the assertion that DID break (`two-host-signed-run`'s owner) was retargeted to `2b` per F16. (b) **D11-E3's citation** — `xtask/src/tests/` is PATH-EXCLUDED from kloc (38604 == the sum excluding it; the 2377 lines are free), so only `#[cfg(test)]` modules in ordinary `src` files are charged; the CI-invisible half is true and WORSE than recorded, because no job runs `cargo test -p xtask` unscoped and ~490 in-`src` tests including the whole demo beat ledger ran nowhere — now enrolled, because AC2.11's own assertion would otherwise have been a null control. (c) **AC3.1's estimate** — 110-145 `xtask/src` lines was wrong; the first implementation measured 38685, **RED +76**, and 71 lines were trimmed back under the wall rather than a forbidden grant requested. **CLOSING NUMBERS:** `xtask` **38604 / 38609 GREEN with only +5 headroom — the next `xtask` story cannot land one line without a measured grant**; `maos-bin` 16260/16260 GREEN at delta **+0**; `maos-a2a-core` and `maos-cli` untouched; aggregate **148160 / +1103, grant REFUSED** with D17's stale +492 reported and its row unedited; `kloc-check` closes RED on the same THREE keys as baseline, none of them this story's; `check-kernel-baseline` GREEN 24472; ABI gates green **and blind** (FLAG-E4) over a diff that makes no ABI change. **ALSO FOUND, NOT FIXED:** `check-env-contract` is RED at HEAD (`MAOS_OPERATOR_BEARER_TOKEN` / `MAOS_OPERATOR_HTTP_BIND`, `main.rs:3254-3255`), verified PRE-EXISTING at `0769869d`; unfixable here without a production line in a zero-headroom crate. **AND ONE FINDING AGAINST MY OWN WORK:** a needle in the new leg carried a closing brace, which `cargo fmt` breaks by inserting a trailing comma — the third appearance of the `246660f9` false-alarm class in this gate, caught only because the leg shipped with a reformat vector. No needle now carries a closing delimiter. D17 OPEN · D18 RESOLVED · D11 OPEN (37/68 verified) · D19 OPEN and paid manually per AC4.1. Row set to `review`, NOT `done`: §A6 is non-degradable here and the reviewer model must differ from `anthropic/claude-opus-5`. |
| 2026-08-16 | **§A6 REVIEW CLOSED → `done`** (`zai/glm-5.2`; 4 layers + runtime; 13 patches applied+verified, 1 dismissed, 1 decision resolved by 8/8 round-table; `xtask` 38609→38655 named measured grant per `kloc.toml:87`; gate PASS, 11/15/27/26/7 suites green, fmt clean, kloc red on exactly the three attributed keys). See `### Review Findings` and the File List addendum. |
