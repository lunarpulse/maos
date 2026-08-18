---
baseline_commit: "**A WORKING TREE, NOT A COMMIT.** HEAD is `87eb6c37` (`j1-crosshost-1b`), but `j1-crosshost-2b`'s entire delivery — 43 modified files, 7 new files, its own story file, and three `kloc.toml` ceiling grants — is **uncommitted** at authoring time. Every number and every `file:line` below was measured against that working tree. Original baseline was clean `5a921c0c`; **do not inherit any number from the original text** (see § RE-BASELINE LEDGER). Blocking condition 1 now requires 2b to be COMMITTED before 2c writes a line, or 2c's own budget deltas are unattributable."
depends_on: "`j1-crosshost-2b` — `done`, §A6 review closed 2026-08-17, **but uncommitted**. Three of this story's five ACs judge a mechanism 2b builds."
blocks: "**`j1-crosshost-2d-paid-two-host-run` — RATIFIED BY LUNARPULSE 2026-08-18 — the operator-lane row that performs the run this story builds the judge for.** It is ratified as a planned unit but is SCOPE TEXT ONLY until a story file is authored, so it cannot leave `backlog` yet. *(Round-table 2026-08-18: `blocks: NONE` was false in two ways — 2c blocks the successor, and 2c is NOT the lane closer.)* **What `2c` DONE means: the judge is built and proven-red. It does NOT mean a two-host signed run happened.** The capture at `_bmad-output/test-artifacts/j1-two-host-evidence/` is ABSENT, leg 9 `paid-run-capture` correctly refuses the claim, and the `two-host-signed-run` beat stays ABSENT — so closing `2c` without a successor would leave **an ABSENT beat whose owner is a `done` story**, which `demo_j1` renders against its owning story as work that is coming. The successor is **CODE-FREE by construction**: it owns the paid run, the capture, the boot-nonce rehearsal (blocking condition 3) and host B's separately-provisioned audit key (AC2.4). **If it acquires an AC that writes a line of Rust, it is the wrong row** — that would be a new mechanism story and a regression of this split."
split_from: j1-crosshost-2-cross-host-signed-run (three-way split RATIFIED by Lunarpulse 2026-08-15; that file is the shared preflight for 2a/2b/2c)
kernel_grant: "NONE, and the correct answer for the one kernel-core surface in scope is **do not touch it**. `check-kernel-baseline` GREEN at **24472 = 24472**, re-verified by executing `cargo run -p xtask -- check-kernel-baseline` against the working tree. The tempting edit is `spawn_and_bridge` (`crates/maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs:449-473`) to add `env_clear()` — **that would be a REGRESSION, not a hardening** (see F3). Every other seam this story needs lives in `maos-a2a-tcp`, `maos-a2a-core`, `maos-audit`, `maos-cli`, `xtask` or `tests/`. Do NOT cite `abi-diff` (FLAG-E4, `crates/maos-spirit-abi` only)."
kloc_grant: "**Budget is the FIRST thing to read and the LAST thing to ask for. Before the first line you must KNOW the zero-headroom state; the grants themselves are taken AFTER the code exists and is measured, never on an estimate (`kloc.toml:60-65`) — that ordering is project discipline, not a contradiction. TWO grants minimum, not one. The story's original 'all budget risk is in `maos-cli`' is FALSE.** Re-measured by running the gate against the working tree: **`maos-cli` 4642/4642 = ZERO** (AC1 lives here) and **`xtask` 38742/38742 = ZERO** (AC5's gate lives here; it was +223 when this story was written, and 1b then 2b consumed all of it). Also at literal zero: `maos-a2a-core` 4669/4669 and `maos-bin` 16738/16738. Still open: `maos-a2a-tcp` **1137/1500 = +363** (AC3), `maos-audit` **6643/6665 = +22** (AC2), `maos-loom-lite` +106, `maos-cohort` +87. `crates/*/tests/`, `xtask/tests/`, `xtask/src/tests/` and all of `spirits/` cost ZERO (`kloc_check.rs:167-193`, verified unchanged) — every test and proven-red vector is still free. Cite `kloc.toml:60-65` with the measurement attached. **`kloc.toml:87`'s 'it must never block a correctness or compliance repair' is PROSE, not a machine carve-out — but 2b just used it BY LINE NUMBER to take `maos-a2a-core` 4654→4669, so the precedent for citing it is fresh and ratified.** AC1 is a correctness repair on a signing path: strongest possible case. AC5's gate is NOT a correctness repair and needs a different argument; take the mandated free reduction FIRST (2b's precedent: it deleted two compiler-dead consts before asking)."
model: frontier-class {opus-4-8, gpt-5.5, glm-5.2, opus-5, equiv}
review: §A6 full-layer net (Blind + Edge + Acceptance + Test-Infra + runtime) — NON-DEGRADABLE (this story decides what a signed two-host artifact is allowed to assert). Reviewer model MUST differ from the dev model.
---

# j1-crosshost-2c — the two-host signed run

Status: **done** — §A6 net CLOSED 2026-08-18 (reviewer `zai/glm-5.3` ≠ dev
`anthropic/claude-opus-5`; 4 layers: Blind + Edge + Acceptance + Test-Infra; runtime layer
re-executed by the reviewer: kernel 24472==24472, story gate PASS 10 legs). 41 raw findings →
25 retained (1 decision resolved → patch, 20 patches applied + verified, 4 deferred), 3
dismissed. All five ACs verified MET in substance by the Acceptance Auditor. Post-patch
verification: proven-red 42/42, d19 governance 11/11, full `-p xtask` 761/0, focused suites
green (two_host_bundle 12, reconcile 9, credential_posture 12, signing 7, fault windows 5,
pin journal 4), `cargo fmt --check` clean, kloc reds only the standing D13/D14 keys; four
measured review grants recorded in `kloc.toml` (xtask 39960, maos-audit 6847, maos-cli
5095, maos-iac 6960). Original dev pass: 2026-08-17 (`anthropic/claude-opus-5`, baseline
`7aa07ee3`), all 15 tasks T0-T14 done, all 5 ACs delivered, D19 RESOLVED under vehicle 14-0.

> **Blocking conditions at close.** (1) **DISCHARGED** — `2b` was committed as
> `7aa07ee3` and `crates/`/`xtask/`/`.github/` were clean before T1; every R1 number
> re-verified exactly. (2) **DISCHARGED** — six named measured grants, each taken after
> the code existed and was `cargo fmt`-measured, with AC-level attribution and the
> mandated free reduction taken first. (3) **OPEN, operator action** — the manual
> boot-nonce pairing has not been rehearsed on a release build; that is a human step
> and runbook **Phase 7.0** now exists for it. Nothing in this story CLAIMS the paid
> run: the gate refuses the claim while the capture is absent. (4) **DISCHARGED** —
> D19 resolved, not disclosed, with a planted red as its acceptance test.

*Original preflight header: re-baselined 2026-08-17 against `j1-crosshost-2b`'s working
tree by five parallel scouts; four blocking conditions, all mechanically checkable.*

> **What this story is.** `2a` made one host able to tell the truth about its worker. `2b` made a
> second host actually do the work. `2c` is **the judge**: it breaks the wire on purpose, scans what
> was stored rather than what was sent, journals the refusals nobody was recording, and produces one
> signed artifact from two independent Transparency Logs that a third party can verify.
>
> **And its first job is still to stop a bug that would burn the paid run.** `sealed-export` prints one
> public key and signs with a different one whenever a region is configured — in **two** places, both
> confirmed live at the working tree (`subcommands.rs:2242` and `:3061`; `derive_region_pubkey` has
> **zero call sites** in that file). `demo-j1` already scrapes that printed key and feeds it to
> `verify-bundle`. Set `MAOS_REGION_HOME` and the existing Tier-2 leg fails **after** the agent has
> been billed. AC1 lands first for that reason.

---

## 📌 READ THIS FIRST — every open call is already decided

*Eleven calls were closed at the 2026-08-17 round-table, per spec + long-term correctness. **Three of
them REVERSE what an earlier reading of this file would tell you**, and they are marked ⛔. If you read
the ACs top-down without this table, you will implement two things backwards.*

| # | Call | **Ratified answer** | Trap |
|---|---|---|---|
| 1 | AC1.3 — stdout arm | **Print the pubkey to stderr**, identical line shape | Refusing breaks pipes; documenting leaves an unverifiable artifact |
| 2 | AC1.1 — the fix itself | Bind the resolved `Option<Region>` to a **local before the match consumes it** | The obvious edit **does not compile** — no `region` at `:2242`; `signed.region` is `Option<String>`, not `&Region` |
| 3 | AC2.1 — what the host field proves | **"Two keyed identities signed."** Never "two hosts" | Anything stronger is a claim standing in for a control |
| 4 | AC2.1 — verification | **Not complete until `tools/verify-audit-bundle/verify.py`** verifies the bundle | Our own `verify-bundle` is a self-check, not the stranger |
| 5 | AC2.2 — where the verb lives | **Reimplement the pattern natively in `maos-audit`** | `maos-audit` → `maos-loom-lite` is a **cycle**; hosting it in `maos-cli` makes reconciliation a feature of our binary |
| 6 | AC2.2 — hidden cost | Two roots ⇒ **two pubkeys** ⇒ new `maos-cli` surface | `maos-cli` is at **ZERO**, and `kloc_grant` funds AC1 only. Measure it early |
| 7 | ⛔ AC2.4 — per-host keys | **Independent per-host roots. NOT the region→team weld.** | The weld exists to make keys derivable from **one** root — the exact property this AC must disprove. One seed-holder could sign **both halves** |
| 8 | AC2.6 — the schema | **Wire it or delete it** | "Mark it documentation" is **refused** — it is a false specification, not documentation |
| 9 | AC3.2 — deny codes | Type **`CODE_INTERNAL` + `CODE_TIMEOUT` only**; take the `maos-a2a-core` grant | Scope wall: two codes, not nine. **Also fix the census comment** that mis-states its own scope |
| 10 | AC3.5 — `deferred-work.md:819` | **FIX it**, do not bound it. *Nothing is `Duplicate` until something is durable* | Bounding ships *"we fixed the durability lie except where we didn't"* |
| 11 | AC4.1 — read-path scan | **Assert BOTH** prefix rules and the ≥32-hex heuristic | Prefix-only blinds you to the class the write path handles **silently** |
| 12 | ⛔ AC5.1/5.2 — gate shape | **ONE always-`Blocking` gate.** No second job, no `AdvisorySubstrate` | Its substrate (operator + two hosts + funded key) can **never** exist in CI — that job would never fire once |
| 13 | ⛔ AC5.3 — beat owner | **Already done by 2b. VERIFY, do not edit** | `demo_j1.rs:907` already reads `"j1-crosshost-2c"`; `"j1-crosshost-2"` has zero hits |
| 14 | AC5.5 — the two manual steps | State as **properties of the system**, not as "did we rehearse" | Out-of-band trust anchor **and** separately hand-provisioned host-B key |
| 15 | D19 | **Option (a)** — one shared helper across all seven walkers | Acceptance is the **planted red**, not the helper. Owner 14-0, not you |

**Precedence rule for the rest of this file:** **R-findings outrank F-findings**, and anything marked
`RATIFIED` / `WITHDRAWN` / `SUPERSEDED` outranks the prose around it. The F-findings are kept for their
reasoning, not their conclusions.

---

## ⚠ RE-BASELINE LEDGER — what moved under this story after it was written

*A ready-for-dev story is not a frozen story. This is the fourth time in this lane that a story's
numbers were invalidated by its predecessors before it started (1a→1b, 2a→1b, 2a+1b→2b, now **2b→2c**).
Read this section before the findings. Everything in it is measured at the working tree described in
`baseline_commit`. **Where an R-finding contradicts an F-finding below, the R-finding wins.***

**R1 — BUDGET INVERTED. AC5 lost its entire budget, and this story now needs TWO grants.**
The original text's headline — *"All of this story's budget risk is now in `maos-cli`"* — is false.
Measured by executing `cargo run -p xtask -- kloc-check --json` against the working tree:

| key | ceiling | measured | headroom | vs. original text |
|---|---|---|---|---|
| `maos-cli` (AC1) | 4642 | 4642 | **ZERO** | confirmed |
| `xtask` (AC5) | 38742 | 38742 | **ZERO** | **was +223 — gone** |
| `maos-a2a-core` (stay out) | 4669 | 4669 | **ZERO** | ceiling moved +15 |
| `maos-bin` | 16738 | 16738 | **ZERO** | 2b's §A6 grant |
| `maos-a2a-tcp` (AC3) | 1500 | 1137 | +363 | was +415 |
| `maos-audit` (AC2) | 6665 | 6643 | +22 | confirmed |
| `maos-loom-lite` | 5383 | 5277 | +106 | new |
| `maos-cohort` | 4900 | 4813 | +87 | new |
| `_aggregate_hardfail` | 147057 | 148892 | **RED −1835** | **was −885** |
| `check-kernel-baseline` | 24472 | 24472 | GREEN | confirmed |

`kloc-check` closes RED on exactly three keys — `maos-domain` (−50, **D14**), `maos-kernel-core`
(−685, **D13**), `_aggregate_hardfail` (−1835, **D17**). All three are pre-existing and named; **none
of them are yours**. Do not repair them and do not let them mask a red you caused.

**R2 — D10's wall is passable, and `2b` just walked through it — cite that precedent, don't rediscover it.**
`2b` took `maos-a2a-core` 4654→4669 under **`kloc.toml:87` cited by line number** — *"it must never
block a correctness or compliance repair"* — for two security-path repairs that could not be routed
elsewhere. So the original text's Trap 14 (*"`maos-a2a-core` is 4654/4654, frozen by D10. One
production line hard-fails"*) is now **wrong in its number and wrong in its finality**. The wall is
`4669/4669` and it is passable *with a named measured grant taken after the code exists*. This
directly dissolves the original **Q2** dead-end, which assumed a `maos-a2a-core` fix had no budget.

**R3 — F10 IS INVERTED: the intake sink is INSTALLED, so two of AC3's three fault windows have a real
target for the first time.** The original text says *"at HEAD `build_cohort_a2a_daemon_runtime` never
installs an intake sink, so an accepted frame is validated, ACKed and dropped."* `2b` wired it:
`crates/maos-bin/src/main.rs:9800` now calls `TcpA2ATransport::bind_with_intake_sink`. A receive-side
executor exists. **The gap moved from "no target exists" to "the target exists and its failure is
untyped" — which is R4.** Bonus for AC3.4: the `CohortRuptureLogSink` bound to the primary TL sits two
lines above at `main.rs:9797-9798`, already in scope at the composition root.

**R4 — SHIP-BLOCKER FOR AC3: `CODE_INTERNAL` and `CODE_TIMEOUT` both render as `TransportFailed`, so
AC3's fault windows cannot distinguish the faults they inject.** `2b` filed this against this story
**in the source itself** (`crates/maos-a2a-core/src/router.rs:384-388`):
> *"Known rendering gap, filed for `j1-crosshost-2c`: the resulting `CODE_INTERNAL` NACK itself falls
> through `interpret_response`'s catch-all into `TransportFailed` at the sender — the same
> misattribution shape as H13, on a code this story newly emits. **2c's fault-injection preflight owns
> typing it.**"

The full path: emitted at `router.rs:1371-1375` (digest-reply push) and `:1549-1553` (delegation push)
→ catch-all `_ => Err(A2AError::TransportFailed(n.error.message))` at **`router.rs:1135`** →
`map_a2a_error_to_iac_bus`'s `TransportFailed` arm at **`:1814-1821`** → `CrossHostTransportFailure`.
**A dropped-receiver internal NACK is byte-identical at the sender to a genuine network partition.**
And note the sharper half: `interpret_response`'s own scope-wall comment (`router.rs:1124-1130`) lists
the six untyped fall-throughs as `PARSE_ERROR, INVALID_REQUEST, METHOD_NOT_FOUND, TIMEOUT,
FRAME_TOO_LARGE, INTERNAL` and asserts they are *"not newly reachable here"* — **but the same story
newly emits `INTERNAL` at two sites**, and `TIMEOUT` is the code AC3.1's own new timeouts will produce.
**Two of the six are exactly the two AC3 needs.** Type those two, record the census, touch nothing
else — the same binding scope wall `2b` used for H13.

**R5 — This story inherits THREE deferred items, not the two the sprint row names.**
`deferred-work.md`:
- **`:817`** — Ctrl-C/graceful shutdown waits forever on a never-exiting remote worker; `spawn_blocking`
  enters `run_cli_wrapper_manifest` and the shutdown await has no deadline or abort path. *"Fault-injection
  semantics owned by j1-crosshost-2c."* Seams: `crates/maos-bin/src/delegation.rs:601-633`,
  `crates/maos-bin/src/main.rs:9561-9571`.
- **`:818`** — Crash window between `journal(Written)` and worker spawn makes a delegated task durably
  look processed (every replay returns `Duplicate`, no execution record). *"Mechanism fix
  (reconciliation/recovery) is 2c's; should be recorded as a RELEASE-HOLDS claim boundary."* Seam:
  `delegation.rs:442-450`.
- **`:819`** — **the one the sprint row omits.** Digest-reply path ACKs a retry as `Duplicate` after a
  dropped-receiver NACK, because `observe_reply` precedes `push_to_intake_sink` and the retry
  short-circuits before the consumer. Owner `j1-crosshost-2c`. Seam: `router.rs:1353-1379`.
**`:819` is not a footnote — it is a live counterexample sitting inside AC3's own duplicate-safety
claim.** A sender that retries after an internal drop is told `Duplicate` and the frame is still gone.
AC3 may not assert duplicate-safe idempotency while this is open; it must either cover it or bound it.

**R6 — AC5.2 AS WRITTEN CANNOT BE BUILT. No gate in this repo mixes binding classes inside one job.**
AC5.2 asks for hermetic `Blocking` legs beside an `AdvisorySubstrate` paid-run leg in one gate.
`check-j1-loopback-delegation` calls `dev_enforced_red_blocks(BindingClass::Blocking, true)` **exactly
once**, in `run_with_root` (`check_j1_loopback_delegation.rs:1110`), governing all seven legs
uniformly; its registry disposition (`gate-registry.toml:281`) is uniform `blocking`/`blocking`; and it
publishes **no ledger at all** (`println!` under `--json` only). Every `AdvisorySubstrate` gate in the
repo (`check-trial-attestation`, `check-escape-detector`) is **its own dedicated job with a uniform
disposition**. There is no per-leg binding mechanism to hang the paid leg off. Do not invent per-leg
binding.
**⚠ R6's CONCLUSION IS SUPERSEDED by the 2026-08-17 round-table — the measurement above stands, the
"two jobs" answer does not.** A second `AdvisorySubstrate` job would need a substrate CI can never have
(an operator, two hosts, a funded API key), so it would take the ABSENT branch on every run for its
entire lifetime — *a gate whose substrate cannot exist is a monument, not a control*. **Resolution:
ONE always-`Blocking` hermetic gate**, validating the paid run's capture when present and refusing the
claim when absent. See AC5.1.

**R7 — AC5.3's re-pointing task is ALREADY DONE. Scope it down to "verify".**
`2b` re-pointed the beat owner: `xtask/src/demo_j1.rs:907` reads `"j1-crosshost-2c"`. The literal
string `"j1-crosshost-2"` has **zero hits repo-wide**, and the narration block the original text cited
at `:283` no longer exists — `2b` rewrote it (`:304-310`) to reference `two-host-delegation`. F5's
"stale at birth" owner finding is **discharged**. What survives from F5 is the part that still binds:
`Beat::absent` (`:106-115`) sets `executed: false` and `Beat::failed()` (`:118-120`) is
`self.executed && !self.state.is_proven()`, so **an unlanded beat still can never fail a run**, and the
in-process flip (`:264-282`) is still the only viable route.

**R8 — MONEY RISK: the only pairing path the paid run can use has never once been executed.**
`2b` shipped the boot-nonce gap as a **stated boundary**, not a fix (RELEASE-HOLDS.md row 8: *"peer
boot-nonce provisioning is manual with no automated channel"*). In a release build,
`main.rs:1865-1871` gates the `MAOS_TEST_BOOT_NONCE` override behind `cfg!(debug_assertions)`, so the
nonce is always random. The documented manual path — operator reads host A's nonce from its own
`cohort:daemon-started` TL row and hand-transcribes it into host B's static peer-pin config
(`crates/maos-a2a-tcp/src/config.rs:25-36`) — is **the only path a release-build paid run can use, and
`2b`'s harness never exercises it** (`two_host_delegation_2b.rs:298,467` use the debug shortcut).
**This is the J1 Tier-2 lesson verbatim: a live path is only proven by running it live.** Rehearse the
manual pairing on a release build BEFORE the agent is billed. See blocking condition 3.

**R9 — D19 is due at this story's CLOSURE, and disclosure is no longer an acceptable disposition.**
Deadline: *"Before the next `j1-*` story **leaves** `ready-for-dev`"* — that is 2c's transition to
`review`/`done`, not its entry, so it does not block starting. But the decision text is explicit:
*"The hole has been open across `1a` (done), `1b`, `j1-demo-one-command-scene` (done) and now `2a`;
each disclosed it in prose and none of them closed it, **which is why disclosure is no longer an
acceptable disposition**."* **AC5.6's "disclose D19" is therefore WRONG and is rewritten below.** The
decision's owners are Mary + John under vehicle **14-0**, not this story — 2c cannot resolve it
unilaterally. See blocking condition 4.

**R10 — ANCHOR DRIFT is arithmetic, not semantic. Apply these offsets.**
`2b` made one clean +88/−1 insertion in `transport.rs` (`bind_with_intake_sink` + a `boot_nonce == 0`
refusal guard), so **every** AC3 seam below it shifted by exactly **+87** and is otherwise byte-for-byte
what the original text described.

| file | offset | note |
|---|---|---|
| `crates/maos-a2a-tcp/src/transport.rs` | **+87** uniform | one insertion; all AC3 seams intact |
| `crates/maos-a2a-core/src/router.rs` | **+97** | `map_a2a_error_to_iac_bus` unchanged in body |
| `xtask/src/demo_j1.rs` | +8 … +336 (non-uniform) | see per-anchor table in Dev Notes |
| `crates/maos-cli/src/subcommands.rs` | site 1 **+0**, everything after **+36** | `2a`/`1b` committed churn, not `2b` |
| `crates/maos-iac/src/adapter/transparency_log.rs` | +18 … +89 | 2b's duplicate-`frame_id` handling |
| `crates/maos-audit/src/*`, `crates/maos-loom-lite/src/*`, `crates/maos-domain/src/*` | **+0** | byte-identical; every AC2 citation exact |

**R11 — `deliver_typed` changed its return type and fixed a REMOTE-TRIGGERABLE PANIC that AC2's own
determinism creates.** It now returns `LogBeforeDeliver<FrameRowWrite>` with typed `Written`/`Duplicate`
outcomes. Before this, a peer resending one deterministic `frame_id` drove a plain `INSERT` into a
`panic!` inside `insert_frame_row_with_correlation` — unreachable at `5a921c0c` only because nothing
ACKed-and-dropped frames ever reached the write path with a duplicate id, and **reachable the moment
`2b` wired the intake sink**. AC2.3 depends on deterministic ids (`seq ‖ run_nonce`); AC3 deliberately
injects retries. **Those two ACs together are the exact input that produced the panic.** Read the
`Written`/`Duplicate` typing before writing reconciliation logic.

**R12 — F13's join key is no longer a design claim; it is PROVEN by an executed CI test.**
`crates/maos-bin/tests/two_host_delegation_2b.rs:533-535` asserts `host_a_frame_id == host_b_frame_id`
and then checks `log_has_frame` on each host's own SQLite log. `2b`'s dev record states it plainly:
*"both Transparency Logs carry the same sixteen `frame_id` bytes — proven by … the first multi-process
daemon test that runs in CI (H7)."* AC2.3's join costs **zero** `maos-audit` lines, and `maos-audit`'s
+22 stays available for AC2.1's host field.

**R13 — Peer authentication is CLOSED, but not the way the tracker predicted.**
`2b` did **not** flip the `loopback-from-host-unverified` leg. It added a **7th gate leg**,
`cross-host-identity-proof` (`check_j1_loopback_delegation.rs:151`, impl `:527`), deriving the
cross-host identity claim from the **executed** two-daemon proof rather than from grep — because the
loopback arm still self-asserts and `true` is its honest permanent value. **`sprint-status.yaml:269`
still carries the disproved prediction** (*"the leg FLIPS and the gate reds with 'boundary MOVED'"*).
Correct that record as part of T12; do not act on it.

**R14 — Small corrections that would cost a dev a compile or a wrong assertion.**
- `env_clear` occurrences: **22**, not 23 — all in `crates/maos-cli/tests/*.rs`, production still **0**.
- `MAOS_AUDIT_KEY_SEED`'s sole occurrence is an error string at **`main.rs:8344`**, not `:8873`.
- `CodexCli::nonsecret_env` is at **`worker_cli.rs:655-663`**, not `:302-310`. It did **not** move to
  the new `worker_spawn.rs`; that file only calls it (`:567`).
- The overclaim negative cited at `subcommands.rs:3935` **does not exist there**; the two real ones are
  `:3973-3983` (egress) and `:3985-4012` (fs_jail).
- `EXPECTED_GATES` = **37**, `check_*.rs` = **68** — D11's counts still correct at the working tree.
- AC5.1's "five places" is really **four files**: `gate-registry.toml` carries both the `gates = [...]`
  array (`:5`) and the 35 `[[ship_gate]]` disposition blocks (`:156`+, J1's at `:279-281`).

**R15 — `2b` is `done` but UNCOMMITTED, and its own story file does not disclose that.**
Both records read `done` (`sprint-status.yaml:270`, `2b`'s story file `:14`) with §A6 closed, but HEAD
is `87eb6c37` and the entire delivery — including three `kloc.toml` ceiling grants this story's budget
depends on — is working-tree state. **A grant recorded in an uncommitted file is not a ratified
ceiling.** See blocking condition 1.

---

## The original findings — F1-F14, with re-baseline verdicts

*Kept because they still carry the reasoning. Line numbers corrected; superseded conclusions marked.*

**F1 — STANDS, both sites confirmed live; site 2's coordinates moved +36.**
`crates/maos-cli/src/subcommands.rs:2242` computes `derive_pubkey(&seed)` and prints it raw
(`:2243-2248`), while `sign_bundle` signs with `derive_region_signing_seed(seed, region)` whenever the
bundle carries a region (`crates/maos-audit/src/sealed_export.rs:253-261`). Region resolves at
`subcommands.rs:2205` via `resolve_region_home()` (`:3656-3660`), precedence `MAOS_REGION_HOME` →
`~/.config/maos/operator.toml [region].home_region` (`:3664-3676`). **`MAOS_REGION_HOME` is read at SIGN
time only** — `audit_verify_bundle` (`:2593-2675`) never calls `resolve_region_home()` and passes the
supplied `--pubkey` straight to `sealed_export::verify_bundle` (`:2661`). *Site 2 (trajectory export):*
region-pin `:3024-3031`, sign `:3033`, **raw print `:3061`**. *Blast radius:* `xtask/src/demo_j1.rs`
scrapes the pubkey from `sealed-export` stderr (`:1440-1445`, `pubkey_hex` `:1469-1474`) and feeds it
to `verify-bundle` (`:1446-1462`); it neither sets nor clears `MAOS_REGION_HOME`. *The helper already
exists:* `derive_region_pubkey` (`sealed_export.rs:41-43`) — **zero call sites in `subcommands.rs`.**

**F2 — STANDS VERBATIM. Every citation exact; `maos-loom-lite` is byte-identical.**
`crates/maos-loom-lite/src/replication/bundle.rs` carries `CrossRegionReplicationBundle` (`:67-81`,
additive `source_team` with `serde(default, skip_serializing_if)` `:73-79`, derive-pubkey-from-CLAIMED-
identity `:74-76`) and `ReAttestationReceipt` (`:104-112`). Verbs: `build_replication_bundle` `:306`,
`_v2` `:345`, `verify_replication_bundle` `:536`, `build_reattestation_receipt` `:982`,
`verify_reattestation_receipt` `:1011`. **Port this design.** *New constraint (R-B):* `maos-audit`
**cannot** depend on `maos-loom-lite` — `maos-loom-lite → maos-audit` already exists and the reverse
edge closes a cycle. Either reimplement the pattern natively inside `maos-audit` (zero new edges), or
call `maos-loom-lite` from `maos-cli`, which already depends on both. **Decide and say which.**

**F3 — STANDS, INVERTED AS BEFORE: the missing `env_clear` is LOAD-BEARING.** Production count is
**0** and all **22** occurrences are in `crates/maos-cli/tests/*.rs`. `spawn_and_bridge`
(`crates/maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs:449-473`) adds env only (`:467-469`).
That is deliberate and documented: `CodexCli::nonsecret_env` (`worker_cli.rs:655-663`) says the
credential *"is inherited host-side from the maos process env, NEVER set here (so MAOS never holds the
value)"*. Adding `env_clear()` breaks the paid worker path and breaches the 24472 pin.
**AC4's credential deliverable is a NEGATIVE TEST asserting the current posture.** The caveat that
matters: the 11 payload variants (`crates/maos-domain/src/frame.rs:63-80` — still 11) carry no
credential *by schema*, but `TaskAssignPayload.goal`/`.success_criteria` are free-form `String`
(`:93-96`) and redaction runs on the **TL write path, not the A2A wire**.

**F4 — STANDS. Two recorded "facts" remain false.** `MAOS_AUDIT_KEY_SEED` does not exist (sole
occurrence: an error string at `main.rs:8344`); the real mechanism is
`maos_domain::audit_key::load_audit_key_seed` (`crates/maos-domain/src/audit_key.rs:31`), env var
**`MAOS_AUDIT_KEY`** holding a **filesystem path** (`:92`). And `PROVEN_LIVE_SIGNED` **has** been
reached — 27 legs across four Reza ledgers on the operator lane; the true narrow statement is that
**CI** has never reached it (no operator key by ratified design, `evidence_ledger.rs:567-574`) and
**J1** has never reached it. The four ledger files are gitignored (`.gitignore:36`) — their absence in
a fresh clone proves nothing.

**F5 — HALF DISCHARGED BY 2b (see R7).** The dead owner string and dead printed name are **fixed**.
What survives: `Beat::absent` (`:106-115`) sets `executed: false`; `Beat::failed()` (`:118-120`) is
`self.executed && !self.state.is_proven()`, so an unlanded beat can never fail a run (`:348-359`). The
ledger flip route remains **dead twice**: `apply_published_ledgers` (`:920-979`) filters
`l.gate == DELEGATION_GATE` (`:935-938`) and that gate writes **no ledger file**; and `ledger_gates()`
(`evidence_ledger.rs:148-150`) **is** `check_loom_substrate_drift::contract_jobs()` — the four Postgres
gates (`check-cross-region-consensus`, `check-multi-region-slo`, `check-multi-tenant-loom`,
`check-reza-production-path`). **Use the in-process executed-leg route (`demo_j1.rs:264-282`).**
*Precision correction:* "demo-j1 has ZERO CI invocation" is imprecise — `cargo test -p xtask demo_j1`
IS enrolled at `discipline.yml:1838`. The operative claim holds: **no CI job ever executes the
`demo-j1` binary or observes its printed claim table.**

**F6 — STANDS, all seams intact at +87.** `TcpStream::connect` is unbounded (**`transport.rs:552-554`**,
bare `.await`, ~130s OS hang) and **`framed.send` is ALSO unbounded** (**`:571-574`**) — a peer that
accepts and stops reading hangs `route_outbound` forever with no OS backstop. Bound both.
`TcpTimeouts::production()` (`:66-72`, unchanged) is `handshake = h` (30s at both real sites),
`intake = 30s`, `idle = 60s`; `test_profile()` (`:75-81`, unchanged) is 250ms.
`A2AError::PartitionTimeout` **does** have a match arm (**`router.rs:1807-1813`**).
`route_outbound` **does** read `peer_cfg` (**`:860`, `:863`**) — just never `partition_timeout_secs`,
whose only production consumer is `LoopbackA2ARouter::route_outbound` (`crates/maos-a2a/src/adapter.rs:81`).

**F7 — STANDS at +87. The verifier provably cannot reach a TL; the listen side leaves ZERO trace.**
Three walls: **dependency** (`maos-a2a-tcp/Cargo.toml:16-33` has no `maos-iac`/`maos-cohort`/
`maos-kernel-core` in production deps, and adding `maos-kernel-core` reds
`t12a_kernel_zero_auto_retry_dep_absent`, `t11_t12_chaos_absence.rs:170-176`); **ownership**
(`TofuPinningVerifier` holds only pins/posture/direction/expected_peer/validation_time/sig_algs);
**signature** (`verify_server_cert`/`verify_client_cert` are synchronous rustls trait methods).
*And it does not need to.* **Dial side:** the mismatch is already typed at the caller
(`transport.rs:882`, variant constructed in `error.rs:66-67`). **Listen side:** replace the discarding
`_ => return` at **`transport.rs:666`** with an `Ok(Err(e))` arm — `core: Arc<A2ARouterCore>` is in
scope at `serve_connection` (**`:649`, param `:652`**), and the core carries an installed **synchronous**
`ConsentRuptureSink` (`crates/maos-a2a-core/src/cohort.rs:41-42`; installed `router.rs:399-402`; prod
impl wired `main.rs:9797-9798`). *The cited `tracing::warn!` is at **`:680`** and remains structurally
unreachable* — `resolve_verified_peer` (**`:766-775`**) consults the same
`find_active_pin_by_fingerprint` oracle the verifier already passed.

**F8 — STANDS. All three pin-mismatch tests are DIAL-side; the untested side is the weaker one.**
`t3_tofu_pin_mismatch_rejected` (`crates/maos-a2a-tcp/tests/t3_t6_security.rs:82`),
`t4b_pin_only_unpinned_leaf_rejected_at_pin` (`:157`), `t6_mitm_cert_swap_after_pin_rejected` (`:299`)
all drive `mira_dials_nash`. **No test exercises the listen side rejecting a client cert**, and that is
the side where `find_active_pin_by_fingerprint` accepts **any** active pin (`verifier.rs:178-187`);
per-peer scoping exists only on the dial side (`scoped_client_config` **`transport.rs:528-540`**).
*Under TLS 1.3 the dialer may not see it:* `connector.connect()` (**`:557-566`**) can complete before
the server evaluates the client cert, so the rejection arrives on the response read and maps to `Io`
(**`:581`**). **A listen-side negative must assert on the SERVER's journal, never the dialer's error class.**

**F9 — STANDS. TOFU is thinner than its docs claim.** `InMemoryTofuPinStore` is the **only**
`impl TofuPinStore` in the workspace (`crates/maos-a2a-core/src/tofu.rs:233`); the comments claiming a
persistence-backed impl (`tofu.rs:66-68`, `:129-131`) are **false**, so **pins are rebuilt from config
at every boot**; `pin_first_contact` is a boot-time config loader
(`crates/maos-a2a-tcp/src/config.rs:119-138`), never an observed cert. Context for AC5.5's honesty.

**F10 — SUPERSEDED BY R3.** The window inventory still holds and the naming correction still holds:
the ACK is `AckBody { delivered, receiver_logical_clock }` (**`router.rs:1556-1562`**) and means
**delivered**, not **executed**. The three honest windows: **(a) before the delivery ACK**, **(b) during
host-B worker execution**, **(c) on the reverse `TaskComplete` delivery**. **(b) and (c) now have a
real target (R3).** Levers, all kloc-free: `silent_endpoint()`
(`crates/maos-a2a-tcp/tests/t_12_3_cohort_halt_receipt.rs:279`, used `:515-522`), `drop(transport)` via
`ServeGuard::drop` (**`transport.rs:91-99`**, used `t_12_3…:452`, `:529`), `set_peer_endpoint` to a dead
address (`t_11_3_scale_churn.rs:124-126`), and `raw_client_stream`/`raw_client_connect`
(`crates/maos-a2a-tcp/tests/support/mod.rs:157-201`). **There is no `a2a-fault-inject` feature and you
should not add one.**

**F11 — STANDS; call sites shifted. `2c` owns the READ-path scan and nothing else on redaction.**
`static RULES` (`crates/maos-iac/src/adapter/redaction.rs:67-132`) has **16** prefix rules plus a 17th
non-prefix heuristic — a hex run ≥ `TOKEN_HEX_MIN_LEN = 32` (`:140`, `contains_hex_token` `:317`) —
which the filter scrubs but `detect_credential` (`:309-313`) deliberately does not report. It runs at
five call sites, **all pre-write**: **`transparency_log.rs:825`, `:1370`, `:1859`, `:2023`, `:2040`**
(plus the write-path guard `subcommands.rs:2434`). **CONFIRMED: no read-path scan exists anywhere** —
`query_with_redaction` (`maos-audit/src/lib.rs:337`) only surfaces metadata computed at write time.
`redaction.rs` itself is untouched by 2b. *Ownership:* `2a` claimed demo-j1's provider-aware write-path
scan and `ClaudeCli::ambient_auth_path`. **`2c`'s deliverable is the thing that exists nowhere: a scan
over STORED rows.**

**F12 — STANDS VERBATIM (`maos-audit` byte-identical).** `AuditBundle`
(`crates/maos-audit/src/sealed_export.rs:94-113`): `schema_version`, `entries`, `i12_digest_refs`,
`i11_distilled_content`, `freshness` (`:121-132`), `applied_redaction`, `redaction_policy`, `region`,
`signature_block` (`:134-139`). `AuditEntry` (`maos-audit/src/lib.rs:91-118`) carries **`boot_nonce`** —
but P11 showed one bundle sweeping **8 distinct boot nonces** under `--range 1d`. And `region` is
jurisdiction, not host: two hosts in one region derive the **same** key
(`derive_region_signing_seed`, `:27-36`). `attester_pubkey` is bundle-supplied, which R-RG1 forbids
trusting (`:84-90`). `verify-bundle` takes one bundle and one `--pubkey`
(`crates/maos-cli/src/cli.rs:440-446`, impl `subcommands.rs:2593-2675`).

**F13 — SUPERSEDED BY R12 (and its original conclusion was already corrected once).** `frame_id` is
the join key, it is selected FIRST (`maos-audit/src/lib.rs:194-196`:
`SELECT frame_id, timestamp_ns, spirit_pid, boot_nonce, capability_token, kind, intent,
payload_redacted FROM transparency_log`), and **both hosts provably carry the same 16 bytes**.
`correlation_id` is indeed dropped from that SELECT (its column is now `transparency_log.rs:286`, index
`:515-518`, filter `:1476-1478`) but it is **not** the join key. **Do not project it.**

**F14 — STANDS on the schema; half discharged on the beat.**
`schemas/audit-bundle.schema.json` is enforced by nothing (**zero** refs in `.github/workflows/` and
`xtask/src/`, both re-verified) and is **already three fields behind** the struct:
`additionalProperties: false` with `[schema_version, entries, i12_digest_refs, i11_distilled_content,
freshness, signature_block]`, omitting `region`, `applied_redaction`, `redaction_policy`. Adding `host`
makes it a **fourth** drift against a schema no machine reads. The demo-ledger half is superseded by R7.

---

## Blocking conditions

1. **`j1-crosshost-2b`'s delivery is COMMITTED.** It is `done` and reviewed but entirely uncommitted
   (R15), including the three `kloc.toml` grants this story's budget arithmetic rests on. A ceiling
   recorded in an uncommitted file is not a ratified ceiling, and 2c's own before/after deltas cannot
   be attributed against a moving tree. **Mechanically checkable:** `git status --short` is clean for
   `crates/`, `xtask/`, `.github/` before T1.
2. **Two named measured grants exist — `maos-cli` (AC1) and `xtask` (AC5) — taken AFTER the code is
   written and measured, never on an estimate** (R1, `kloc.toml:60-65`). Take the mandated free
   reduction first. AC1's grant is a correctness repair on a signing path and should cite
   `kloc.toml:87` with 2b's `maos-a2a-core` precedent (R2); AC5's is not a correctness repair and needs
   its own argument.
3. **The manual boot-nonce pairing is REHEARSED on a release build before the paid run is scheduled**
   (R8). This is the only pairing path a release build has, and nothing has ever executed it. Rehearse
   it, or the artifact records a pairing step that was never performed.
4. **D19 is RESOLVED — not disclosed — before this story leaves `ready-for-dev`** (R9). Owners are
   Mary + John under vehicle 14-0; 2c cannot resolve it unilaterally. **Operator escalation required.**
   **Round-table 2026-08-17 chose OPTION (a), unanimously, per spec + long-term correctness:** replace
   the digit-prefix filter with the sprint-status key set across **all seven** walkers via **one shared
   helper** (not seven edits — five walkers sharing one filter is the single-source defect this project
   has already paid for twice). **Option (b) was refused on grounds:** it would ratify that a category
   of story is *exempt* from dev-record, model-tier and review-findings discipline, converting a defect
   into a policy just as the defect was about to expire — and the lane it would exempt is the one
   running cross-host mTLS, signed artifacts and a paid agent, i.e. where review discipline matters
   most. **Acceptance test (binding): plant a `j1-*` story file with a missing dev record and watch a
   Blocking gate RED.** A shared helper without the planted red is a refactor, not a control.
   **Schedule 14-0 NOW** — the written deadline *"before the next `j1-*` story leaves ready-for-dev"*
   is **self-voiding**, because 2c is the closer and there is no next `j1-*` story (verified: the lane
   holds 1a/1b/2a/2b/2c and the demo, all `done` but this one). Either it binds here or it never binds.
   **Sequencing hazard the dev must pre-empt (Amelia):** if 14-0 lands option (a) while 2c is in
   flight, **2c's own story file becomes gate-visible mid-story** and must suddenly satisfy dev-record
   completeness, model tier and the §A6 marker. Populate every field from day one so the flip is a
   no-op instead of a 2am red.

---

## Story

**As** the founder who has to hand a third party evidence that the developer-remote loop is real,
**I want** the wire broken on purpose and the refusals recorded, the stored rows scanned rather than
the sent ones, and one signed artifact reconciled from two independent Transparency Logs whose signer
identity actually verifies —
**so that** "two hosts did this" is a claim a stranger can check, and every gap it does not cover is
named in the artifact rather than in a story file nobody reads.

---

## Acceptance Criteria (5)

### AC1 — Fix the signing-identity bug first, in both places, or nothing downstream verifies

1. **`sealed-export` prints the key that actually signed.** Print
   `derive_region_pubkey(&seed, &region)` (`sealed_export.rs:41-43`) when a region resolved, not
   `derive_pubkey(&seed)` (`subcommands.rs:2242`).
   **The obvious edit does NOT compile — read this before you write it.** At the print site there is no
   `region` binding: `resolve_region_home()`'s match arm (`:2205-2212`) is
   `Ok(Some(r)) => unsigned.with_region(&r)`, so `r` dies inside the arm. And `signed.region` is
   **`Option<String>`** (`sealed_export.rs:110-111`), while `derive_region_pubkey` takes **`&Region`** —
   so reaching for the bundle field gives you a type error, and parsing the String back into a `Region`
   is the clumsy answer. **Do this instead: bind the resolved `Option<Region>` to a local BEFORE the
   match consumes it**, use it for both `with_region(..)` and the pubkey derivation, and branch
   `Some(r) => derive_region_pubkey(&seed, r), None => derive_pubkey(&seed)`. Same shape at site 2. **Keep the stderr line's exact shape** — `"…({N} entries, pubkey {hex})"`
   (`:2243-2248`) is a de-facto ABI with three consumers: `pubkey_hex` (`demo_j1.rs:1469-1474`),
   `entry_count` (`:1477-1490`), and the pin at `demo_j1_tests.rs`.
2. **Fix the second site.** Trajectory export prints the raw pubkey at **`subcommands.rs:3061`** while
   signing with the derived seed at **`:3033`** (region-pin `:3024-3031`). Fixing only AC1.1 leaves the
   bug live on `maosctl audit export`.
3. **Cover the `--output`-less arm — RATIFIED: print to stderr, identical line shape.**
   `subcommands.rs:2250-2257` writes the bundle to stdout and emits **no pubkey line at all**, so a
   stdout-mode export is unverifiable. *(Round-table 2026-08-17 closed the three-way fork.)* Stdout
   carries the bundle, stderr carries the key — that is already the contract in `--output` mode.
   **Refusing** breaks anyone piping today; **documenting** leaves an unverifiable artifact you can
   produce by accident. Print it.
4. **Give `verify-bundle` a derivation path.** `audit_verify_bundle` (**`:2593-2675`**) never calls
   `resolve_region_home()` (**`:3656-3660`**) and passes `--pubkey` verbatim to `verify_bundle`
   (`:2661`). Accept a base seed plus a claimed region — mirroring `verify_replication_bundle`'s
   *derive-from-claimed-identity* rule (`bundle.rs:74-76`) — or state explicitly in `--help` that
   `--pubkey` must be the region-derived key.
5. **Two negative tests.** One asserting `printed_pubkey == signing_pubkey` with `MAOS_REGION_HOME`
   set — **which must RED before the fix** — and one asserting they match with it unset. Tests are
   kloc-free; **the fix is not, and `maos-cli` is at ZERO** (blocking condition 2).

### AC2 — Two-TL reconciliation, ported not invented

1. **Add a host discriminator to `AuditBundle` additively — it is an ANTI-FORGERY control, not a label.**
   `2b` proves the crossing by writing **the same `frame_id`** into both logs — now proven by an
   executed CI test (R12) — which is what makes reconciliation free, and which means the two halves are
   otherwise **indistinguishable**. `region` cannot separate them (same region ⇒ same signing key,
   `sealed_export.rs:27-36`); `boot_nonce` is per-boot and one export swept eight; `attester_pubkey` is
   bundle-supplied and R-RG1 forbids trusting it (`:84-90`). **Without this field one host can produce
   both halves of a "two-host" bundle.** Implement as `Option<String>` (or a `HostId` newtype) with
   `serde(default, skip_serializing_if = "Option::is_none")`, exactly as `source_team` was added
   (`bundle.rs:73-79`) and as `region` already behaves (`sealed_export.rs:537-563`). The 9.2b **HARD
   byte-identity replay must stay byte-identical** for bundles that omit the field — **assert it, do not
   assume it**. **The field must be bound by the signature**: add a negative where a host field altered
   post-signing fails verification.
   **BOUND THE CONTROL HONESTLY (round-table 2026-08-17).** The field defeats a forger who does **not**
   hold the other host's key. It does **NOT** prove physical separation, and under a shared base seed it
   proves nothing at all (see AC2.4). State the exact reach in the artifact: *the host field proves
   **two keyed identities signed**; it does not prove two machines, two processes, or two operators.*
   Anything stronger is a claim standing in for a control.
   **THE STRANGER'S PATH IS NOT OPTIONAL (round-table 2026-08-17).** The premise of this artifact is
   *"a claim a stranger can check"* — and no stranger has ever checked one, so that premise is currently
   a control whose consumer does not exist. Verifying our artifact with **our own** `verify-bundle` is a
   self-check and does not discharge it. **The run is not complete until
   `tools/verify-audit-bundle/verify.py` — the field-agnostic Python twin, which drops `signature_block`
   and sorts the rest (`verify.py:91-93`), so a new `host` field flows through untouched — verifies the
   produced bundle.** That twin is not our Rust path; it is the nearest thing to the stranger that
   exists, and it already exists. Its passing output is part of the capture.
2. **A two-bundle verb, ported — AND IT NEEDS A `maos-cli` SURFACE, which the budget note does not
   fund.** *(Uncosted item found at the 2026-08-17 readiness check.)* `verify_bundle` takes **one**
   bundle and **one** `&[u8; 32]` pubkey (`sealed_export.rs:283-285`), and the CLI mirrors that with a
   single required `--pubkey` (`cli.rs:440-446`). Under AC2.4's independent per-host roots there are
   **two** roots and therefore **two** pubkeys, so reconciliation cannot be expressed by the existing
   surface: it needs a new subcommand or a second key argument. **That lands in `maos-cli`, which is at
   ZERO headroom and which `kloc_grant` attributes to AC1 alone.** Measure AC2's CLI cost separately and
   fold it into the same `maos-cli` ask — do not discover it at T3.
   **A two-bundle verb, ported.** Port `verify_replication_bundle` / `build_reattestation_receipt` /
   `verify_reattestation_receipt` (`bundle.rs:536`, `:982`, `:1011`) rather than designing a new
   protocol; the receipt shape — *"source X's bundle landed at dest Y"* — is exactly the two-host claim.
   **Derive each side's pubkey from its CLAIMED identity; never read `attester_pubkey` out of the
   artifact.** **Dependency constraint (R-B): `maos-audit` CANNOT depend on `maos-loom-lite`** —
   `maos-loom-lite → maos-audit` exists and the reverse closes a cycle. Either reimplement the pattern
   natively in `maos-audit`, or call the verbs from `maos-cli` (which already depends on both).
   **State which, and why, in the Dev Agent Record.**
3. **Reconcile on `frame_id`. Do NOT project `correlation_id`.** It is `AuditEntry.frame_id_hex` in
   every bundle already (`maos-audit/src/lib.rs:194-196` selects it FIRST), and both hosts provably
   carry the same 16 bytes (R12). The join costs **zero** `maos-audit` lines. J1 ids are deterministic
   `seq ‖ run_nonce` (`delegation.rs:380-382`, `spirits/orchestrator/src/lib.rs:358`), so 2c can
   *compute* the expected id. **Read R11 first**: `deliver_typed` now returns typed `Written`/`Duplicate`,
   and determinism plus AC3's injected retries is the exact input that used to panic the write path.
4. **DO NOT weld a per-host key from the shared base seed. Two hosts must hold two INDEPENDENT roots,
   or the artifact's "two" is our word for it.** *(Reversed at the 2026-08-17 round-table; the
   previously-ratified "stage 3 of the region→team template" is WITHDRAWN as actively wrong.)*
   `derive_team_signing_seed(base_seed, region, team)` (`sealed_export.rs:72-82`) welds over
   `derive_region_signing_seed(base_seed, region)` (`:27-36`) — **every weld descends from ONE
   `base_seed`**, and the doc comment says so outright: *"an attacker who recovers the region signing
   seed can derive every team key in that region."* That template exists to make keys **derivable from
   one root** — the exact property this AC must disprove. A stage-3 host weld therefore means **one
   seed-holder can legitimately sign BOTH halves**: signature valid, host field inside it, artifact a
   perfect "two-host" bundle produced by one machine.
   **Required instead:** each host signs with its **own independently-held base seed** (its own
   `MAOS_AUDIT_KEY` path, `audit_key.rs:31`/`:92`), never derived from a shared root; reconciliation
   verifies each half against **that host's separately published pubkey**. Two signatures are evidence
   of two holders only under distinct roots.
   **Consequence you must handle, not discover:** `2b`'s two-process-one-box shape defaults to **one
   HOME and therefore one key file**, so the mechanism proof and the signed proof want *opposite*
   setups. Provisioning host B's audit key separately is a second manual, unwritten, never-executed
   operator step sitting beside the manual nonce (R8) — **both belong in the runbook (AC5.6) and both
   bound the claim (AC5.5)**. If the paid run cannot give the two hosts distinct roots, the artifact
   says **"two identities"**, never **"two hosts"**.
5. **Say what the bundle can and cannot discriminate**, in the artifact — `region` cannot separate two
   hosts in one region; `boot_nonce` is per-boot and one `--range 1d` export swept eight.
6. **The schema — RATIFIED: wire it or delete it. "Mark it documentation" is refused.**
   *(Round-table 2026-08-17.)* `schemas/audit-bundle.schema.json` declares `additionalProperties: false`
   and omits three fields the struct emits (F14). **That is not documentation — it is a false
   specification**, and marking it documentation writes down that a wrong contract is acceptable while
   nobody enforces it. Wiring is small: the hermetic gate (AC5.1) validates an emitted bundle against
   the schema, and the schema is corrected to match the struct **in the same change**.
   **Acceptance test: plant a bundle carrying an extra field and watch the gate RED** — otherwise you
   have built a validator that validates nothing. If wiring is refused, **delete the file**; do not
   leave a fourth drift behind.

### AC3 — Break the wire on purpose, record what happens, and be able to tell the faults apart

1. **Bound BOTH unbounded operations.** `TcpStream::connect` (**`transport.rs:552-554`**) and
   **`framed.send`** (**`:571-574`**). The second is the cheaper real partition — a peer that accepts
   and stops reading hangs `route_outbound` forever with no OS backstop (F6). Both land in
   `maos-a2a-tcp` (**+363**), never in `maos-a2a-core`.
2. **SHIP-BLOCKER — type `CODE_INTERNAL` and `CODE_TIMEOUT` at `interpret_response`, or AC3.3 cannot
   distinguish its own injected faults.** Both currently fall through the catch-all
   (**`router.rs:1135`**) into `A2AError::TransportFailed`, then into `CrossHostTransportFailure`
   (**`:1814-1821`**) — so a dropped-receiver internal NACK and a genuine partition are **byte-identical
   at the sender** (R4). `2b` filed this against this story in-source (`router.rs:384-388`).
   **Apply `2b`'s H13 scope wall verbatim:** repair only these two codes, record the census (currently
   10 of 16 typed), touch nothing else. This lands in `maos-a2a-core` at **ZERO headroom** — take it as
   a named `kloc.toml:87` correctness-repair grant, citing 2b's own 4654→4669 precedent (R2).
   Also correct the census comment at `router.rs:1124-1130`, which asserts `INTERNAL` is *"not newly
   reachable here"* while the same story newly emits it at `:1371` and `:1549`.
3. **Wire `partition_timeout_secs` to the TCP path, or delete the claim.** Its only production consumer
   is `LoopbackA2ARouter::route_outbound` (`crates/maos-a2a/src/adapter.rs:81`); `maos-a2a-tcp` reads it
   zero times, and the wire is bounded by hardcoded `TcpTimeouts::production()` instead. Do not restate
   "in-flight frames are NACKed after a configurable 30s partition timeout", and do not repeat the
   "zero match sites" claim — `A2AError::PartitionTimeout` has an arm at **`router.rs:1807-1813`**.
4. **Three fault windows, named correctly, now that (b) and (c) have a target (R3).** (a) before the
   delivery ACK, (b) during host-B worker execution, (c) on the reverse `TaskComplete` delivery. Do
   **not** write "after-completion-before-ACK" — the ACK means *delivered*, not *executed*
   (**`router.rs:1556-1562`**). Use the existing kloc-free levers (F10); **do not add an
   `a2a-fault-inject` feature.**
5. **Resolve the three inherited fault-semantics debts, or bound them in the artifact (R5).**
   `deferred-work.md:817` (shutdown waits forever on a never-exiting worker —
   `delegation.rs:601-633`, `main.rs:9561-9571`), `:818` (journal-before-spawn crash window —
   `delegation.rs:442-450`), and **`:819` (digest-reply retry ACKed as `Duplicate` after a
   dropped-receiver NACK — `router.rs:1353-1379`)**.
   **`:819` — RATIFIED: FIX IT, do not bound it** *(round-table 2026-08-17; the earlier
   "narrow the claim to the delegation path" compromise was WITHDRAWN)*. It is **the same durability
   lie `2b` just fixed one layer over, reached by a different door**: 2b's own comment says *"an ACK
   here is a lie about durability, and the sender has no other way to learn the frame is gone"* — and
   `:819` is a retry ACKed `Duplicate` while the frame is still gone. Bounding it means shipping *"we
   fixed the durability lie except on the path where we didn't."*
   **The invariant to implement: NOTHING IS `Duplicate` UNTIL SOMETHING IS DURABLE.** Do not record the
   dedup until `push_to_intake_sink` succeeds (`observe_reply` currently precedes it). Same crate, same
   grant as AC3.2, same scope wall. **Fallback condition (Murat, binding):** if the measured fix comes
   back materially larger than the ~10 lines this implies, come back to the room — and if a narrowing
   is taken instead, **the narrowing must be machine-checked**, never a sentence in this file.
   `:817` and `:818` may still be bounded rather than fixed; `:818` explicitly wants a RELEASE-HOLDS
   claim-boundary row.
6. **Journal the pin mismatch on BOTH sides, without touching the verifier.** It provably cannot reach
   a TL (F7). Dial side: the typed `HandshakeFailed{PinMismatch}` already surfaces at the composition
   root (**`transport.rs:882`**). Listen side: replace the discarding `_ => return` at
   **`transport.rs:666`** with an `Ok(Err(e))` arm using the already-installed synchronous
   `ConsentRuptureSink` — `core` is in scope at **`:649`/`:652`**, prod impl wired at
   **`main.rs:9797-9798`**. **Do not add `maos-kernel-core` to `maos-a2a-tcp`'s deps**
   (`t11_t12_chaos_absence.rs:170-176` greps for exactly that, inside the 50× loop).
7. **The listen-side negative asserts on the SERVER's journal.** All three existing pin tests are
   dial-side (F8), the listen side accepts any active pin, and under TLS 1.3 the dialer may see `Io`
   (**`transport.rs:581`**) rather than `PinMismatch`.
8. **Keep it out of the 50× loop.** Every test in `crates/maos-a2a-tcp/tests/` runs **51× per push**
   (50 from `discipline.yml:1538-1544` inside `timeout-minutes: 10`, plus once scoped). A test that
   waits out the 60s idle timeout or the ~130s unbounded connect cannot live there — that is what
   `TcpTimeouts::test_profile()` (250ms, `transport.rs:75-81`) exists for.

### AC4 — Scan what was stored, and assert the credential posture rather than changing it

1. **Build the read-path scan — it exists nowhere.** All 16 prefix rules plus the hex heuristic run
   pre-write only, at **`transparency_log.rs:825`, `:1370`, `:1859`, `:2023`, `:2040`** (F11). `2c` owns
   a scan over **stored rows**. Reuse `redaction::detect_credential` (`redaction.rs:309-313`) rather
   than re-deriving rules, and note it is prefix-only by design — the ≥32-hex-run heuristic (`:140`,
   `:317`) scrubs but does not report.
   **RATIFIED: the scan asserts BOTH, reported distinctly** *(round-table 2026-08-17)*. This is not a
   hedge. The write path handles **both** classes, so in a correctly-redacted stored row **neither**
   may appear — therefore **a hit on either is a redaction escape**. Asserting only the prefix half
   leaves the scan blind to exactly the class the write path handles *silently*, i.e. the class where a
   miss would never have been logged in the first place. Report prefix hits and hex-run hits as
   separate findings so the escape's class is visible.
2. **The credential-isolation deliverable is a NEGATIVE TEST asserting the current posture.**
   `env_clear` is absent **deliberately** — **22** occurrences, all in `crates/maos-cli/tests/*.rs`,
   production **0** — because the worker credential is inherited host-side so MAOS never holds it
   (`worker_cli.rs:655-663`), and `spawn_and_bridge` is kernel-core (F3). Assert that the **11** payload
   variants (`frame.rs:63-80`) carry no credential *by schema*, and state the caveat honestly: `goal`
   and `success_criteria` are free-form `String` (`:93-96`) and redaction runs on the TL write path,
   **not** the A2A wire. A negative that plants a token in `goal` tests content, not construction —
   **say which you are asserting.**
3. **Do not touch what `2a` owns.** demo-j1's provider-aware write-path scan and
   `ClaudeCli::ambient_auth_path` are `2a` AC2.5 and AC2.1 verbatim. Verify; do not re-implement.

### AC5 — The judge: a gate that binds, a beat that flips, and a claim bounded by what was proven

1. **ONE hermetic `Blocking` gate — RATIFIED 2026-08-17. The "two jobs" answer is WITHDRAWN, and so is
   any `AdvisorySubstrate` job.** The R6 measurement stands (no gate here mixes binding classes inside a
   job: `check-j1-loopback-delegation` calls `dev_enforced_red_blocks(Blocking, true)` **once**,
   `check_j1_loopback_delegation.rs:1110`, uniform `blocking`/`blocking` at `gate-registry.toml:281`);
   the *conclusion* drawn from it did not. **Ask what substrate the paid-run job needs: an operator, two
   hosts and a funded API key — which CI has never had and will never have.** That job would take the
   substrate-ABSENT branch on every run for its entire lifetime, printing WOULD-HAVE-BLOCKED into the
   void and blocking nothing, ever. **A gate whose substrate cannot exist is a monument, not a control**
   — and standing one up would hand every future reader an `AdvisorySubstrate` disposition to explain.
   **Land exactly one always-`Blocking` hermetic gate.** The paid run's evidence is a **capture
   artifact** on the T6 model: the same gate **validates it when present** and **refuses to let anything
   claim it when absent**. One job, one binding class, no never-firing second job, and it costs less in
   `xtask` — which is at zero (R1).
   **Register it in all five slots across four files:** `EXPECTED_GATES`
   (`check_ship_gate_completeness.rs:20-63`, 37 entries, hand-maintained — nothing forces a gate into
   it), `gates = [...]` (`gate-registry.toml:5`), a `[[ship_gate]]` disposition block
   (`gate-registry.toml:156`+), the job in `discipline.yml`, and the job name in the `v1-0-ship-gate`
   `needs:` array (`discipline.yml:3217-3260`). **No `services:` block** —
   `check_loom_substrate_drift`'s leg 2 (`check_loom_substrate_drift.rs:698-711`,
   `is_service_bearing_gate_job` `:551-572`) rejects an unregistered service-bearing gate job and is
   itself blocking and in ship-gate needs (`discipline.yml:3260`). Copy
   `check-live-bilateral-consent` (**`discipline.yml:2500-2513`**).
2. **Binding class, chosen honestly.** The mechanism legs a hermetic CI can run are
   `BindingClass::Blocking` from the day they land (`gate_common.rs:78-90`, `dev_enforced_red_blocks`
   `:97-102`) — 1a's stated precedent, *"Blocking from the day it lands, not advisory-now-blocking-later"*.
   **Every leg of this gate is `Blocking`. Do NOT reach for `AdvisorySubstrate` (`gate_common.rs:84-89`)
   — see AC5.1: the paid run is a validated capture, not a substrate-gated job.** The absent-capture
   case is expressed by the beat staying ABSENT (which is already the demo's honest model), never by a
   binding class that can never fire. **Give every leg a
   `LegAudit`** (`gate_common.rs:128-168`, `vacuous_legs` `:172-178`) — 1b shipped that primitive
   precisely so a gate cannot mint a check count it did not perform.
3. **Flip the beat by an executed leg. RE-POINT the owner to `j1-crosshost-2d-paid-two-host-run`
   (RF-0 — this instruction previously said "VERIFY, do not edit", which was TRUE when written and
   EXPIRED the moment the lane gained a successor).** `2b` had moved the owner off the dead
   `j1-crosshost-2` key onto `2c`, which was correct while `2c` owned the run; after the 2026-08-18
   split `2c` owns the judge and `2d` owns the run, so leaving it would close `2c` with an ABSENT beat
   whose owner is a `done` story. **Three sites move together** — `demo_j1.rs` (the beat),
   `check_j1_two_host_signed_run.rs` (the absent-branch leg that *enforces* attribution, plus its
   finding text), and `demo_j1_tests.rs` (the pin) — and the proven-red `GOOD_DEMO_J1` fixture with
   them, or the baseline reds. Mirror
   `demo_j1.rs:264-282` (find by name, set `state`/`detail`/`executed = true`/`owner = None`). The
   ledger route is structurally dead twice (F5) — do not attempt it. **RF-0 (2026-08-18) landed the
   re-point:** `unlanded_beats` names `j1-crosshost-2d-paid-two-host-run` (the pre-filed instruction
   to "verify, don't edit" described `2b`'s state and expired the moment the lane gained a
   successor — see the §A6 findings for the six record sites corrected with it).
4. **`PROVEN_LIVE_SIGNED` follows Reza's posture, and the vocabulary is real.** Use `MAOS_AUDIT_KEY`
   (a **path**, `audit_key.rs:92`), `release_verify::sign_sha256sums`, a `MAOS-EVIDENCE-V1` record bound
   to `MAOS_EVIDENCE_COMMIT`/`MAOS_EVIDENCE_NONCE`, verified by `verify_release_signature`
   (`evidence_ledger.rs:49`, `:647`; requires `outcome == "PASSED"` `:622`, commit `:628`, nonce `:634`)
   — **not** `MAOS_AUDIT_KEY_SEED`, which does not exist and will not compile (F4). Do **not** write
   "no leg has ever reached this state": 27 have, on the operator lane. The honest sentence is *"CI
   holds no operator key by ratified design (`evidence_ledger.rs:567-574`), so in CI this leg is
   `INDETERMINATE`; the operator lane produces the signed claim."*
5. **Bound the claim by what was actually proven.** Name, in the artifact and not only in this file:
   whether the two hosts were two processes or two machines — `2b`'s mechanism proof is **two real OS
   processes** (`two_host_delegation_2b.rs:291,311,457,471`, `CARGO_BIN_EXE_maos`), so the paid run must
   state its own shape; that peers are bare `IP:port` with **no DNS** (`transport.rs:521-522`, `:555`);
   that pins are rebuilt from config at every boot and no durable TOFU store exists (F9); and that the
   listen side accepts any active pin (F8).
   **TWO HUMAN-PERFORMED STEPS MUST BE NAMED AS PROPERTIES OF THE SYSTEM, not as questions about our
   diligence (round-table 2026-08-17).** *"Whether the manual pairing was rehearsed"* is a question about
   **us** and belongs in the blocking conditions. The artifact must instead state the **system**
   properties: **(i) the mTLS trust anchor between these two hosts was established out-of-band by a
   human operator, not by the protocol** (R8, RELEASE-HOLDS row 8 — boot-nonce provisioning is manual,
   with no automated channel); and **(ii) host B's audit signing key was provisioned separately by hand**
   (AC2.4 — without distinct roots, "two hosts" degrades to "two identities"). A reader who is told
   *"these two hosts authenticated"* will not guess that *"these two hosts were introduced."* Use `2a`'s established shape — a
   stated posture a capture **cannot overclaim**, with a negative refusing the overclaim direction
   (`CaptureDoc` `subcommands.rs:2315-2349`, `validate()` `:2357-2408`, negatives **`:3973-3983`** and
   **`:3985-4012`**).
6. **Close the lane's record — and RESOLVE D19, do not disclose it (R9).** Add the J1 rows to
   `_bmad-output/test-artifacts/traceability-matrix.md` (the lane has **zero** today; the file is 84
   lines and carries only a J3 GAP row). Extend `runbook-j1-tier-2-signed-live-run.md` (287 lines, 5
   phases, one host, codex only — **mentions claude zero times**) to the two-host + heterogeneous-adapter
   run **including the manual boot-nonce pairing step**. Correct the stale prediction at
   `sprint-status.yaml:269` (R13). **D19 requires a decision, not a paragraph** — its own text says
   disclosure is no longer acceptable, and its owners are Mary + John under 14-0; escalate rather than
   disclose (blocking condition 4). Populate the model/§A6 fields regardless.

---

## Traps

1. **Do not add `env_clear`** (F3). It is a regression and a kernel breach.
2. **Do not write code against `MAOS_AUDIT_KEY_SEED`** (F4). It does not exist; sole hit is an error
   string at `main.rs:8344`.
3. **Do not claim `PROVEN_LIVE_SIGNED` is unreached** (F4). 27 legs, operator lane. The ledger files are
   gitignored (`.gitignore:36`) — absence in a fresh clone proves nothing.
4. **Do not flip the beat via a published ledger** (F5). Dead twice.
5. **Do not re-fix the beat owner string.** RF-0 (2026-08-18) re-pointed it to
   `j1-crosshost-2d-paid-two-host-run`; the §A6 record sweep corrected the sites that still said
   "verified, not edited".
6. **Do not restate P3's false clauses** (F6): `PartitionTimeout` has a match arm
   (`router.rs:1807-1813`), and `route_outbound` **does** read `peer_cfg` (`:860`, `:863`) — just not
   `partition_timeout_secs`.
7. **Do not add `maos-kernel-core` to `crates/maos-a2a-tcp`** — `t12a_kernel_zero_auto_retry_dep_absent`
   (`t11_t12_chaos_absence.rs:170-176`) greps the manifest and reds inside the 50× loop.
8. **Do not put a slow test in `crates/maos-a2a-tcp/tests/`** — 51× per push, 10-minute cap (AC3.8).
9. **Do not add a `services:` block to a new job** (AC5.1).
10. **Do not touch `2a`'s redaction or ambient-auth work** (F11). Verify it; do not re-own it.
11. **Do not read `attester_pubkey` out of the artifact to verify it** (`sealed_export.rs:84-90`).
    Derive from the claimed identity.
12. **Do not change the `sealed-export` stderr line shape** — three consumers parse it (AC1.1).
13. **Any new gate leg must read via `root.join(rel)`, never a hardcoded path.** The proven-red harness
    runs with `current_dir(tempdir)`. **Verified-still-dangerous callees:**
    `gate_common::read_disposition` (hardcoded `Path::new("xtask/gate-registry.toml")` at **`:63`**),
    `check_ship_gate_completeness` (hardcoded `.github/workflows/discipline.yml` at **`:174`**),
    `evidence_ledger::REPORT_DIR` (const `:73`, hardcoded use `:1201-1202`), and anything shelling
    `cargo`/`tokei`. `check_j1_loopback_delegation.rs` is compliant — its `read()` helper uses
    `root.join(rel)` at `:171`; copy that.
14. **`maos-cli` ZERO and `xtask` ZERO** (R1) — two grants, measured after the code exists.
    `kloc.toml:87` is **prose**: permission to ask, not a carve-out. But 2b's `maos-a2a-core` grant is a
    live precedent for the ask (R2).
15. **`maos-a2a-core` is 4669/4669, not 4654/4654.** AC3.2 must land there and needs its own grant.
16. **`check-env-contract` cannot see anything this story writes.** It walks **only**
    `crates/maos-bin/src/` (`check_env_contract.rs:119-121`). `MAOS_AUDIT_KEY`, `MAOS_REGION_HOME`,
    `MAOS_EVIDENCE_*` and `CODEX_API_KEY` are all unregistered because they are read elsewhere. Do not
    read that as permission. *(It was also RED at `0769869d` for
    `MAOS_OPERATOR_BEARER_TOKEN`/`MAOS_OPERATOR_HTTP_BIND` — pre-existing, not yours.)*
17. **`cargo test -p maos-bin` is RED under default parallel flags** (D16). Run scoped, `--test-threads=1`.
18. **FR21's 60s window** bites repeated runs on one data home; `MAOS_HOME` outranks `XDG_DATA_HOME`.
    The advertised `MAOS_ORCHESTRATOR_DISPATCH_WINDOW_NS` escape hatch has **zero code readers**.
19. **There is no unscoped `cargo test -p maos-bin` in CI.** A test file that is not `--test`-enrolled
    is a suggestion. Enrol by exact name, as 1b did.
20. **`A2AProfile` is dead config and defaults to `Loopback`** (`config.rs:79-81`). Never derive a
    "cross-host" claim from it; derive it from the transport type or the endpoint.
21. **`demo-j1`'s BINARY has zero CI invocation** and `Beat::absent` never fails a run (F5). Its unit
    tests *are* enrolled (`discipline.yml:1838`) — do not confuse the two. The new gate is the only
    thing that will bind.
22. **Deterministic `frame_id` + injected retries used to panic the write path** (R11). Read
    `deliver_typed`'s `Written`/`Duplicate` typing before writing AC2.3 or AC3.4.

---

## Tasks

- [x] **T0 (blocking conditions)** — Confirm 2b is committed and `git status` is clean for `crates/`,
      `xtask/`, `.github/`; re-run `kloc-check` and `check-kernel-baseline` and record the numbers you
      actually get. **If they differ from R1, this file is stale again — say so and re-measure, do not
      proceed on these numbers.**
- [x] **T1 (AC1)** — Both P12 sites (`subcommands.rs:2242`, `:3061`), the stdout-mode arm, the
      verify-side derivation path, and the two negatives (one must RED before the fix). **First, own
      commit, before any paid run is scheduled.** Measure `maos-cli` before and after; take the grant.
- [x] **T2 (AC2.1, AC2.5)** — Additive host field on `AuditBundle` with the byte-identity assertion
      **and the post-signing-alteration negative**; state its discrimination scope in the exact ratified
      words (*two keyed identities signed*, never *two hosts*). Schema status is already ratified —
      wiring lives in T11, there is nothing left to decide.
- [x] **T3 (AC2.2, AC2.3)** — Port the two-bundle verb and receipt; **record the hosting decision
      (`maos-audit` native vs. `maos-cli` call site) and why — `maos-audit` cannot depend on
      `maos-loom-lite`.** Reconcile on `frame_id_hex`. **No `correlation_id` work.**
- [x] **T4 (AC2.4)** — **Independent per-host roots. Do NOT weld from the shared `base_seed`** (the
      region→team template guarantees the property this AC must disprove). Verify the two hosts do not
      share `~/.config/maos/audit-signing.key`; record host B's separate key provisioning as a runbook
      step and a claim boundary.
- [x] **T5 (AC3.1, AC3.3)** — Bound `connect` **and** `framed.send`; wire `partition_timeout_secs` to
      the TCP path or delete the claim.
- [x] **T6 (AC3.2) — SHIP-BLOCKER, do before T7** — Type `CODE_INTERNAL` and `CODE_TIMEOUT` at
      `interpret_response`; correct the census comment; take the `maos-a2a-core` grant. **Without this,
      T7's assertions cannot distinguish the faults T7 injects.**
- [x] **T7 (AC3.4, AC3.8)** — The three correctly-named fault windows using existing levers; keep them
      out of the 50× loop.
- [x] **T8 (AC3.5)** — **FIX `:819`** under AC3.2's grant: invariant *nothing is `Duplicate` until
      something is durable* (do not record the dedup until `push_to_intake_sink` succeeds). Measure it;
      if materially larger than ~10 lines, return to the room — a narrowing must be machine-checked.
      `:817` and `:818` may be bounded; `:818` wants a RELEASE-HOLDS claim-boundary row.
- [x] **T9 (AC3.6, AC3.7)** — Pin-mismatch journaling on both sides via the composition root and the
      `ConsentRuptureSink`; the listen-side negative asserting on the server's journal.
- [x] **T10 (AC4)** — The read-path stored-row scan; the credential-posture negative; verify (do not
      re-own) `2a`'s work.
- [x] **T11 (AC5.1, AC5.2, AC2.6)** — **ONE** always-`Blocking` hermetic gate (no `AdvisorySubstrate`
      job), registered in all five slots, no `services:` block, `LegAudit` on every leg, plus a
      proven-red file copying `xtask/tests/j1_crosshost_2b_proven_red.rs`'s idiom. **Wire the
      bundle-schema validation into this gate and correct the schema in the same change** (or delete the
      schema); acceptance = a planted extra field REDs it. Take the `xtask` grant **after** a free
      reduction.
- [x] **T12 (AC5.3, AC5.4)** — Executed-leg beat flip (RF-0 re-pointed the owner to
      `j1-crosshost-2d-paid-two-host-run`; the original T12 text said "verify, don't edit", which
      described `2b`'s state and was corrected by the §A6 review); Reza-posture signing with the
      real vocabulary; correct `sprint-status.yaml:269`.
- [x] **T13 (AC5.5, AC2.1)** — The bounded claim as a capture that cannot overclaim, with the
      overclaim-refusing negative. **State both human-performed steps as system properties** (out-of-band
      trust anchor; separately hand-provisioned host-B audit key), and **run
      `tools/verify-audit-bundle/verify.py` against the produced bundle** — the stranger's path — with
      its output in the capture.
- [x] **T14 (AC5.6)** — J1 traceability rows; two-host runbook incl. manual pairing; **D19 escalation
      (not disclosure)**; Dev Agent Record; budget attributed by key in a clean tree.

### Review Findings

_(§A6 net is non-degradable; reviewer model MUST differ from `anthropic/claude-opus-5`.)_

**PRE-LOADED BY THE 2026-08-18 ROUND-TABLES — filed here rather than edited silently so the
reviewer's baseline stays clean.**

> **SEQUENCING (ratified):** `2c`'s delivery is **UNCOMMITTED** — HEAD is `7aa07ee3`, which is `2b`.
> **Commit `2c`, with RF-0 and RF-1 in the same commit, BEFORE §A6 runs.** Blocking condition 1 says a
> grant recorded in an uncommitted file is not a ratified ceiling; that sentence was written about `2b`
> and applies to `2c` verbatim — `maos-cli`, `xtask` and `maos-a2a-core`'s ratified numbers all live in
> an uncommitted `kloc.toml` right now. This is the **second** time this lane has left a `done`/`review`
> story uncommitted, and the first time cost `2c` a full five-scout re-baseline. **Process rule for the
> Epic-14 retro (not a `2c` AC): a story does not enter `review` until it is committed.**

- **RF-1 (blocking, AC5.6 / AC5.1) — the judge does not publish its own admission criteria, and the
  runbook tells the operator to copy a file it never describes. This is AC1's failure shape at the
  capture layer, and it costs the paid run.**
  Leg 9 `paid-run-capture` requires **seven** fields in
  `_bmad-output/test-artifacts/j1-two-host-evidence/two-host-capture.json`: `host_a`, `host_b`,
  `shape`, `claim_scope`, `trust_anchor_established_out_of_band`,
  `host_b_audit_key_provisioned_separately`, `stranger_verification`. **That contract exists in exactly
  two places — the validator source, and a fixture buried inside
  `xtask/tests/j1_crosshost_2c_proven_red.rs`.** The runbook's §7.6 correctly documents the four
  `two_host_*` fields for the **Phase-4 `CaptureDoc`** (2a's `record-capture`, whose overclaim refusals
  are real machinery — do NOT fold that into a hand-written JSON), and then the next block says
  `cp two-host-capture.json …` **without ever stating what is in it**.
  **Failure mode, concretely:** the operator runs a paid agent across two hosts, follows the runbook to
  the letter, invents a capture, and leg 9 rejects it — *after* the agent is billed. That is precisely
  the defect AC1 landed first to prevent, reproduced one layer up.
  **Fix (code-free, and it belongs to `2c`, not `2d` — a judge that will not state its own admission
  criteria is not finished):** lift the existing fixture out of the proven-red test into a **committed
  example capture** under the evidence directory, have leg 9's own test validate that example so the
  two cannot drift, and point the runbook at it so the operator **fills in a template instead of
  inventing a shape**.
  *Also record, do not fix here:* one fact is currently carried under two names in two documents —
  `two_host_trust_anchor` (CaptureDoc) and `trust_anchor_established_out_of_band` (leg 9). Two names,
  two files, **no single source**, so they can disagree and nothing catches it. Merging the *mechanisms*
  is wrong (2a's refusals are compiled in); naming the duplication is not.

- **RF-2 (blocking, AC5.1) — FOUND BY RF-1's OWN VECTOR ON ITS FIRST RUN: the judge was
  UNSATISFIABLE. No capture could ever be admitted, and the paid run would have failed after the
  agent was billed.** RESOLVED.
  Leg 9 required two things at once: `claim_scope` must equal `CLAIM_SCOPE` **verbatim**, and the
  capture text must not contain `two operators` unless the literal `not two operators` appears. The
  ratified `CLAIM_SCOPE` reads *"…not two machines, two processes, or two operators"* — it contains
  `two operators`, and it does **not** contain `not two operators`. **So satisfying requirement one
  forced a violation of requirement two.** Every existing vector that wrote a capture asserted RED;
  **no test anywhere asserted green with a capture present**, so leg 9's entire PRESENT branch — the
  success path of the paid run — had never once been exercised.
  *The first fix was wrong and the harness caught that too:* rewriting `CLAIM_SCOPE` to negate each
  item explicitly satisfied the tripwire but **broke the tripwire**, because the negation check is
  **global over the whole document** — once any text carries `not two machines`, an operator could
  write "we used two machines" anywhere else and nothing fires. `an_overclaiming_capture_reds` went
  red and said so.
  **Fix as landed (line-neutral, ratified wording untouched):** `claim_scope` is already pinned
  byte-for-byte, so it needs no scanning — the overclaim scan now excludes it and covers exactly the
  operator-authored free text where an overclaim could actually appear.
  **Shape: a control whose two requirements contradict each other, in the leg that guards the money.**

- **RF-0 (blocking, AC5.3) — the `two-host-signed-run` beat owner must re-point to
  `j1-crosshost-2d-paid-two-host-run` before this story reaches `done`.**
  `xtask/src/demo_j1.rs:910` currently reads `"j1-crosshost-2c"`. That was correct when this story
  owned the run. Under the round-table split it no longer does: `2c` owns the judge, `2d` owns the run.
  **If `2c` closes with this string unchanged, the ledger carries an ABSENT beat whose owner is a
  `done` story** — the widowed-control shape this split exists to prevent, and precisely the defect
  `2b` fixed when it re-pointed the same string off `j1-crosshost-2` and onto `2c`.
  *Note the irony deliberately:* AC5.3 already tells the dev **"VERIFY the owner string, do not edit
  it"** — true when written, false the moment the lane gained a successor. **A residual that was TRUE
  and EXPIRED**, in the AC written to stop exactly that. Re-point it, and update AC5.3's instruction in
  the same change so the next reader is not told to verify a value that must move.
+
#### §A6 net 2026-08-18 (4 layers: Blind + Edge + Acceptance + Test-Infra; reviewer `zai/glm-5.3` ≠ dev `anthropic/claude-opus-5`)

_25 findings retained from 41 raw (cross-layer merges applied), 3 dismissed. Runtime layer re-executed at HEAD by the reviewer: `check-kernel-baseline` PASS 24472==24472; `check-j1-two-host-signed-run` PASS (10 legs, capture absent ⇒ claim refused)._

- [x] [Review][Patch] AC2.1 stranger-path substrate — the skip sentinel `two_host_reconcile_2c.rs:396` (`"No Ed25519 backend available"`) matches nothing `verify.py` can print (actual: `"Error: no Ed25519 library found"`, `tools/verify-audit-bundle/verify.py:67`); the graceful-skip branch is dead code and a backend-less machine fails the Blocking lane mislabeled as a signature mismatch. **DECIDED 2026-08-18 (Lunarpulse): require the backend, fail loud** — replace the dead skip with a fail carrying the install instruction, and add a CI step ensuring `cryptography` is present so the Blocking control has a guaranteed substrate. — sources: test-infra + acceptance — LOW
- [x] [Review][Patch] `scan-credentials` reports correctly-redacted digest rows as hex-run escapes and exits 1 — `crates/maos-iac/src/adapter/redaction.rs:382` `scan_stored_payload` has no `clause_sources` carve-out while the write path (`:269-289`) deliberately retains exact-32-hex frame refs; the scan's own doc (`:348`) states the now-false premise "a correctly-redacted row can contain neither"; runbook Phase 7.5 makes 0-escapes an abort condition, so the honest operator is blocked at this story's own gate; fix by mirroring the write-path carve-out + a `clause_sources` fixture in `credential_posture_2c.rs` [redaction.rs:382-401] — sources: blind+edge — **HIGH** — **LANDED**: JSON-aware carve-out + case-insensitive hex; 3 new fixtures (sanctioned refs clean, 48-hex still flagged, uppercase flagged).
- [x] [Review][Patch] Leg 9 validates field PRESENCE only — `xtask/src/check_j1_two_host_signed_run.rs:897-915`: `trust_anchor_established_out_of_band:false`, `host_b_audit_key_provisioned_separately:false`, empty `stranger_verification`, `host_a == host_b`, and capture↔bundle host mismatches all still mint `two_host_signed_run_claimed:true` (`:1148`); the halves themselves need only exist (`:940-949` — no signature or host cross-check); value-check the two booleans == true, require non-empty strings, refuse `host_a == host_b`, cross-check capture hosts against the bundle host stamps, add vectors [check_j1_two_host_signed_run.rs:897-949] — sources: blind+edge+test-infra — **HIGH** — **LANDED**: all value checks + capture↔bundle host cross-check; 4 new vectors (false booleans, empty attestation, host-twice, mismatch); template still admissible.
- [x] [Review][Patch] RF-0's record half did not land — six sites still assert the beat owner was "verified, not edited" while this commit moved it to `j1-crosshost-2d-paid-two-host-run`: `xtask/src/demo_j1.rs:927-929`, this file `:693-696` (AC5.3), `:741` (guardrail 5), `:825` (T12), `:1167-1170` (Dev Agent Record — a false completion claim), `traceability-matrix.md:119`; leg 9 only greps for the new literal so nothing machine-checks the stale records [demo_j1.rs:927-929] — sources: acceptance+blind — **HIGH** — **LANDED**: all six sites corrected (demo_j1 comment + module doc, AC5.3, guardrail 5, T12, Dev Agent Record, traceability).
- [x] [Review][Patch] `reconcile-hosts` escapes `SharedAttesterRoot` with one base seed under two claimed regions — `subcommands.rs:2969` → `resolve_verify_key:2764-2781` derives each half's key from that half's claimed region, so the same seed file for both halves yields distinct keys and `sealed_export.rs:456-458` passes; refuse byte-identical base seeds when both halves used `--seed`, add a same-seed/different-region test, and note the two-invocation `MAOS_REGION_HOME` residual against RELEASE-HOLDS row 9's "refuses two halves attested by one root" wording [subcommands.rs:2764-2781] — source: acceptance — **MEDIUM**
- [x] [Review][Patch] `verify_two_host_receipt` does not pin `claim_scope`/`schema_version` — `sealed_export.rs:571-580` rebuilds the signed payload from receipt-supplied values, so a receipt re-signed with a widened scope or bumped schema verifies; pin both against `TWO_HOST_CLAIM_SCOPE`/`TWO_HOST_RECEIPT_SCHEMA` + test [sealed_export.rs:565-593] — sources: blind+edge — **MEDIUM**
- [x] [Review][Patch] The gate path never verifies the capture transcript's signature — `verify_capture_signature` (`check_j1_two_host_signed_run.rs:1102-1119`) has no caller in `judge`/`run_with_root` (sole consumer is the demo lane), and it verifies `records.first()` only; unsigned capture + bundle files mint `two_host_signed_run_claimed:true` in the gate JSON; call it in leg 9's present branch and fail closed when the verification key is absent [check_j1_two_host_signed_run.rs:1102-1148] — source: blind — **MEDIUM**
- [x] [Review][Patch] Window-(a)'s typed `PartitionTimeout` arm is unreachable — `t_2c_fault_windows.rs:44-54`'s silent endpoint never completes TLS, so `transport.rs:573-582`'s handshake timeout wins and the fallback arm (`:156-159`) tolerates the degraded `TransportFailed` mapping; leg 5 (`:649-654`) only text-greps the source; the send-side typed mint (`transport.rs:591-600`) has zero behavioral coverage — add a fixture that completes TLS then stalls the read and assert the typed arm [t_2c_fault_windows.rs:140-165] — sources: blind+test-infra — **MEDIUM** — **LANDED PARTIALLY, deliberately**: a live typed-arm fixture is not kernel-deterministic on loopback (send-buffer autotune absorbs a sub-codec-cap body; SO_RCVBUF pins at listener and accept both proved insufficient on this kernel), so the fix that IS deterministic landed — the fallback arm now accepts ONLY the bounded handshake timeout, so the frozen-map degradation (`PartitionTimeout` collapsing into `TransportFailed("partition timeout …")`) reds the test. The live typed-arm residual is text-grep + unit-seam coverage; owned by the 2d preflight alongside the deferred read-phase typing.
- [x] [Review][Patch] Overclaim negation is document-global and the README misdescribes the tripwire — `check_j1_two_host_signed_run.rs:928-930`: one `not two machines` anywhere disarms every overclaim; `j1-two-host-evidence/README.md:49-51` claims "negated in place" and "whole capture text" (both wrong); no proven-red vector pins the negation-smuggle; hyphenated/underscore token forms evade; scope the negation to the occurrence's field/window, align the README, add the vector [check_j1_two_host_signed_run.rs:928-938] — sources: edge+test-infra+acceptance — **MEDIUM**
- [x] [Review][Patch] Tautological assertion — `two_host_bundle_2c.rs:168-172` compares `derive_region_pubkey(&shared, &region)` to itself, so AC2.5's headline claim rests on a vacuous green (8.13-P5 class); replace with the weld identity (`derive_region_pubkey(s,r) == derive_pubkey(derive_region_signing_seed(s,r))`) or defer to the real control (same-root refusal) [two_host_bundle_2c.rs:164-172] — source: test-infra — **MEDIUM**
- [x] [Review][Patch] Planted-red coverage is partial — the d19 suite exercises 4/7 converted walker sites (`check-dev-model-tier`, both `check-epic-6-bridge` legs unexercised end-to-end); the proven-red missing-governed-file vector covers 6/13 governed files; ~15 gate needles unmutated (leg 1 `ArgGroup`, leg 3 typed-refusal needles, leg 5 `A2AError::PartitionTimeout`, leg 6 cohort needle, leg 9 absent-branch owner, invalid-JSON arms) — "34/34" is a vector count, not a per-needle seal; add the missing vectors [d19_story_file_governance.rs; j1_crosshost_2c_proven_red.rs] — sources: acceptance+edge+test-infra — LOW
- [x] [Review][Patch] Leg-10 derivation/enrollment robustness — hardcoded four `TESTS_DIRS` (`check_j1_two_host_signed_run.rs:81-86`; a `_2c.rs` in any other crate is a dead test), silent `read_dir` skip (`:963-966`), and `--test`/`-p` matched as independent substrings over the whole job blob (`:1016-1018`; `-p maos-a2a-tcp` prefix-matches `-p maos-a2a-tcp2`) [check_j1_two_host_signed_run.rs:81-86,963-1018] — sources: blind+edge+test-infra — LOW
- [x] [Review][Patch] Leg-2 heuristic is first-match-only on `pub host: Option<String>` with a 200-char attribute window (`check_j1_two_host_signed_run.rs:338-342`); `BundleForSigning`'s host stamp is not independently pinned [check_j1_two_host_signed_run.rs:338-342] — source: edge — LOW
- [x] [Review][Patch] `journal_peer_identity_refusal`'s `Err` is discarded at all three production call sites (`transport.rs:711-717, 739-745, 986-995`) — the "fails loudly rather than silently" contract (`router.rs:500-513`) lives only in the API; emit `tracing::error!` on `Err` [transport.rs:711-995] — source: blind — LOW
- [x] [Review][Patch] Partition window silently clamped to idle — `transport.rs:942-943` honors `min(partition_timeout_secs, idle)` with no warning, so a ratified 300s window is silently 60s; emit a `tracing::warn` on clamp or validate ≤ idle at config load [transport.rs:942-943] — source: blind — LOW
- [x] [Review][Patch] Traceability row says 33 vectors; the enrolled file carries 34 (`traceability-matrix.md:118`) — source: acceptance — LOW
- [x] [Review][Patch] `scan-credentials --spirit` multi-pair dead end — the error says "disambiguate" (`subcommands.rs:2993-2998`) but the verb exposes only `spirit`/`range` (`cli.rs:512-519`); add `--boot` (RecordCapture precedent) or reword the remedy [subcommands.rs:2986-2998] — source: edge — LOW
- [x] [Review][Patch] Reconcile accepts blank host claims from hand-forged bundles — `sealed_export.rs:462-472` byte-compares raw values (`Some("")` passes `MissingHostClaim`); the CLI producer trims (`subcommands.rs:2263`), so this is reconcile-side hardening only [sealed_export.rs:462-472] — source: edge (core claim disproved at HEAD; residual kept) — LOW
- [x] [Review][Patch] Test-record nits — scraper-twin citation points at `demo_j1.rs:1469-1474` while the scraper is at `:1527-1532` (`signing_identity_2c.rs:116-117`); the d19 suite doc claims `--stories-dir`/`--sprint-status` flag isolation while the tests rely on cwd-clap defaults (`d19_story_file_governance.rs:18-20` vs `:105`) [signing_identity_2c.rs:116-117] — source: test-infra — LOW
- [x] [Review][Patch] Declared sprint keys with no story file escape all seven walkers — `gate_common.rs:54-68` never checks member existence, so `rm <story>.md` silently shrinks the governed set; fail closed on missing members [gate_common.rs:54-68] — source: edge — LOW
- [x] [Review][Patch] The read-path scan is blind to uppercase hex — `is_hex_byte` accepts only `a-f` (`redaction.rs:135-137`), so a 32+-char `A-F` secret neither scrubs (pre-existing) nor scans; accept `A-F` in `scan_stored_payload`'s run counting [redaction.rs:135-137,404-418] — source: edge — LOW
- [x] [Review][Defer] Read-phase `awaiting response` is bounded by `idle`, not the partition window (`transport.rs:602-603`) — the most common post-handshake partition shape renders as untyped `TransportFailed` with no frame id; out of ratified AC3.1 scope (connect/send only) — deferred, owner: 2d preflight / a2a lane
- [x] [Review][Defer] Intake-sink mutex held across the durable audit append (`router.rs:1490-1501`) — serializes pushes/replies behind audit-DB latency; the lock scope is load-bearing for the nothing-Duplicate-until-durable invariant, so narrowing needs a redesign + load evidence — deferred, perf follow-up
- [x] [Review][Defer] `CODE_INTERNAL`/`CODE_TIMEOUT` collapse into `CrossHostRouteFailure` with hand-duplicated display strings (`error.rs:200,206` vs `router.rs:1955-1964`) — two sources of truth for the same rendered text, no sync test; design acknowledged in-code — deferred
- [x] [Review][Defer] Write-path redaction blind to uppercase hex (`redaction.rs:135-137`) — pre-existing; only the read-path scan half is this story's (kept as patch above) — deferred, pre-existing

_Dismissed as noise/false-positive (3): "untrimmed `--host` flows into the bundle" (false at HEAD — `subcommands.rs:2263` trims; residual kept as the blank-host patch above); "undeclared `.md` invisible to walkers" (documented deliberate design — manifest membership over filename shape; the inverse direction is kept as the missing-member patch); "`live_lines` treats `/* */` blocks as live" (accepted limitation of text-grounded legs; behavior is pinned by the cargo suites and proven-red vectors)._

---

## Dev Notes

### Measured at the working tree (HEAD `87eb6c37` + `2b` uncommitted), 2026-08-17

Produced by executing `cargo run -p xtask -- kloc-check --json` and
`cargo run -p xtask -- check-kernel-baseline`. **Not estimates.**

| Instrument | Ceiling | Measured | Verdict |
|---|---|---|---|
| kloc `maos-cli` | 4642 | **4642** | **ZERO — AC1 needs a named grant** |
| kloc `xtask` | 38742 | **38742** | **ZERO — AC5 needs a named grant (was +223)** |
| kloc `maos-a2a-core` | 4669 | **4669** | **ZERO — AC3.2 needs a named grant** |
| kloc `maos-bin` | 16738 | **16738** | ZERO — stay out |
| kloc `maos-a2a-tcp` | 1500 | **1137** | +363 — AC3.1/AC3.6 fit |
| kloc `maos-audit` | 6665 | **6643** | +22 — AC2.1's host field fits |
| kloc `maos-loom-lite` | 5383 | **5277** | +106 |
| kloc `maos-cohort` | 4900 | **4813** | +87 |
| kloc `maos-domain` | 8644 | **8694** | RED −50 — **D14, not yours** |
| kloc `maos-kernel-core` | 18248 | **18933** | RED −685 — **D13, not yours** |
| kloc `_aggregate_hardfail` | 147057 | **148892** | RED −1835 — **D17, not yours** |
| `check-kernel-baseline` | 24472 | **24472** | GREEN |
| Zero-cost surfaces | — | `crates/*/tests/`, `xtask/tests/`, `xtask/src/tests/`, all `spirits/` | `kloc_check.rs:167-193` (verified) |

> **All of this story's test weight is free. All of its budget risk is in `maos-cli`, `xtask` and
> `maos-a2a-core` — three crates at literal zero.** Measure in a clean tree; two scouts on the original
> preflight reached a false conclusion by measuring a tree a predecessor was mutating.

### Where the code goes — re-derived at the working tree

| Concern | File | Anchor |
|---|---|---|
| P12 site 1 | `crates/maos-cli/src/subcommands.rs` | `:2242`, print `:2243-2248`, stdout arm `:2250-2257`, region `:2205` |
| P12 site 2 | `crates/maos-cli/src/subcommands.rs` | region `:3024-3031`, sign `:3033`, **raw print `:3061`** |
| verify-bundle | `crates/maos-cli/src/subcommands.rs` | `:2593-2675` (verify call `:2661`); CLI surface `crates/maos-cli/src/cli.rs:440-446` |
| region home | `crates/maos-cli/src/subcommands.rs` | `resolve_region_home` `:3656-3660`, operator.toml `:3664-3676` |
| Bundle type | `crates/maos-audit/src/sealed_export.rs` | `AuditBundle` `:94-113`, `SignatureBlock` `:134-139`, byte-identity precedent `:537-563`, R-RG1 `:84-90` |
| Key derivation | `crates/maos-audit/src/sealed_export.rs` | region `:27-36`, `derive_region_pubkey` `:41-43`, team `:72-82`, tripwire `:383-390`, negatives `:415-449` |
| Entry type / join key | `crates/maos-audit/src/lib.rs` | `AuditEntry` `:91-118`; `query` SELECT `:194-196` (`frame_id` FIRST) |
| Design to PORT | `crates/maos-loom-lite/src/replication/bundle.rs` | types `:67-81`, `:104-112`; verbs `:306`, `:345`, `:536`, `:982`, `:1011` |
| Join-key proof | `crates/maos-bin/tests/two_host_delegation_2b.rs` | `host_a_frame_id == host_b_frame_id` `:533-535` |
| Read-path scan | `crates/maos-iac/src/adapter/redaction.rs` | `RULES` `:67-132` (16), `detect_credential` `:309-313`, hex `:140`/`:317` |
| Write-path call sites | `crates/maos-iac/src/adapter/transparency_log.rs` | `:825`, `:1370`, `:1859`, `:2023`, `:2040` |
| Timeouts | `crates/maos-a2a-tcp/src/transport.rs` | `connect` `:552-554`, `framed.send` `:571-574`, `TcpTimeouts` `:56-82` |
| Pin journaling seams | `crates/maos-a2a-tcp/src/transport.rs` | dial `:882`; listen `_ => return` `:666`, `core` in scope `:649`/`:652`; unreachable warn `:680` |
| Error typing (AC3.2) | `crates/maos-a2a-core/src/router.rs` | catch-all `:1135`, census comment `:1124-1130`, emissions `:1371`, `:1549`, map `:1814-1821` |
| Rupture sink (sync) | `crates/maos-a2a-core/src/cohort.rs` | `append` `:41-42`; install `router.rs:399-402`; prod wire `main.rs:9797-9798` |
| Intake sink (2b) | `crates/maos-bin/src/main.rs` | `bind_with_intake_sink` `:9800` |
| Deterministic ids | `crates/maos-bin/src/delegation.rs` | `seq ‖ run_nonce` `:380-382`; `spirits/orchestrator/src/lib.rs:358` |
| Fault levers (free) | `crates/maos-a2a-tcp/tests/` | `silent_endpoint` `t_12_3…:279`, `drop` `:452`/`:529`, `set_peer_endpoint` `t_11_3…:124-126`, raw `support/mod.rs:157-201` |
| Beat + demo coupling | `xtask/src/demo_j1.rs` | beat `:904-908` (owner already `2c`), flip template `:264-282`, ledger filter `:935-938`, `Beat::absent` `:106-115`, `failed` `:118-120`, pubkey scrape `:1440-1445`, `pubkey_hex` `:1469-1474`, `entry_count` `:1477-1490` |
| Gate registration | `xtask/src/check_ship_gate_completeness.rs`, `xtask/gate-registry.toml`, `.github/workflows/discipline.yml` | `EXPECTED_GATES` `:20-63` (37); registry `:5` + `[[ship_gate]]` `:156`+ (J1 at `:279-281`); job template `:2500-2513`; needs `:3217-3260` |
| Gate primitives | `xtask/src/gate_common.rs` | `BindingClass` `:78-90`, `dev_enforced_red_blocks` `:97-102`, `LegAudit` `:128-168`, `vacuous_legs` `:172-178`, `EvidenceState` `:198-211`, `project` `:273-287` |
| Proven-red template | `xtask/tests/j1_crosshost_2b_proven_red.rs` | `lay_green` `:60-88`, `assert_red` `:107-118`, baseline `:120-121` |
| Overclaim precedent | `crates/maos-cli/src/subcommands.rs` | `CaptureDoc` `:2315-2349`, `validate()` `:2357-2408`, negatives `:3973-3983`, `:3985-4012` |

### Proven-red inventory at the working tree

| File | Lines | Vectors |
|---|---|---|
| `j1_crosshost_1a_proven_red.rs` | 456 | 11 (`lay_green :157-191`, `assert_red :218`, baseline `:242-243`) |
| `j1_crosshost_1b_proven_red.rs` | 868 | 27 |
| `j1_crosshost_2a_proven_red.rs` | 585 | 15 |
| `j1_crosshost_2b_proven_red.rs` | 242 | 7 |
| **`j1_crosshost_2c_proven_red.rs`** | — | **yours; all `lay_green` trees must stay synchronized** |

### Signing and evidence — the real vocabulary

| Concern | Mechanism | file:line |
|---|---|---|
| Key material | `MAOS_AUDIT_KEY` = a **path**; explicit → env → `~/.config/maos/audit-signing.key` | `crates/maos-domain/src/audit_key.rs:31`, `:92` |
| Operator key present on this box | 64 bytes, mode 0600 (stat only; contents never read) | `~/.config/maos/audit-signing.key` |
| Bundle signing | `sealed_export::sign_bundle`, region-welded seed | `crates/maos-audit/src/sealed_export.rs:244-261` |
| Evidence record signing | `release_verify::sign_sha256sums`, `MAOS-EVIDENCE-V1` | `tests/harness/evidence_record.rs:100`, `:35` |
| Build binding | `MAOS_EVIDENCE_COMMIT` / `MAOS_EVIDENCE_NONCE` | `evidence_record.rs:30-31`, `:72` |
| Verification | `verify_release_signature`; `outcome == "PASSED"` + commit + nonce | `xtask/src/evidence_ledger.rs:49`, `:622`, `:628`, `:634`, `:647` |
| CI has no operator key (by design) | `NotFound` → downgrade with written reason, no dev-key fallback | `evidence_ledger.rs:567-574` |
| Ledger gate set (J1 is NOT in it) | `ledger_gates()` = the four Postgres substrate gates | `evidence_ledger.rs:148-150` |

### References

- Shared preflight: `j1-crosshost-2-cross-host-signed-run.md` (§2 P1-P14 — P3, P7 and the
  `env_clear`/`MAOS_AUDIT_KEY_SEED`/`PROVEN_LIVE_SIGNED` claims are corrected here by F6, F7, F3, F4)
- Predecessor: `j1-crosshost-2b-cross-host-delegation-mechanism.md` — read its **RE-BASELINE LEDGER**
  (H1-H13), its **T0** (why D18 is not 2c's either), and **H13** (the scope-wall template AC3.2 copies)
- Claim boundaries: `RELEASE-HOLDS.md` row 8 (2b's stated boundaries, incl. manual nonce provisioning)
- Deferred items: `deferred-work.md:817`, `:818`, `:819` — all three owned here
- Runbook to extend: `_bmad-output/test-artifacts/runbook-j1-tier-2-signed-live-run.md` (287 lines,
  5 phases, one host, codex only, **claude zero times**)
- T6 evidence: `_bmad-output/test-artifacts/j1-tier2-evidence/{j1-tier2-capture.json, j1-tier2-bundle.json}`
- Decision register: `_bmad-output/planning-artifacts/epics/epic-14-preflight-decisions.md`
  (D10, D11, D13, D14, D16, D17, D18, **D19**)

---

## Dev Agent Record

### Agent Model Used

`anthropic/claude-opus-5` · harness: omp (Oh My Pi coding harness) · 2026-08-17.
Frontier-class, satisfying the story's `model:` requirement.
**§A6 net pre-booked** — full-layer (Blind + Edge + Acceptance + Test-Infra + runtime),
NON-DEGRADABLE. The reviewer model MUST differ from `anthropic/claude-opus-5`.

> This field is now **enforced** on this filename, not merely required by policy: D19
> was RESOLVED by this story (option (a), vehicle 14-0), so
> `j1-crosshost-2c-two-host-signed-run.md` is governed by five Blocking gates that
> previously could not see it. The sequencing hazard Amelia flagged fired exactly as
> predicted: `check-dev-model-tier` went RED on this file the moment the walker
> conversion landed, with `dev model \`_\` is not in the frontier allowlist`. It is
> green now because this section is populated — which is the point.

### Debug Log References

**T0 — the baseline moved again, in our favour.** Blocking condition 1 is DISCHARGED,
not waived: `2b`'s delivery was committed as `7aa07ee3` before this story began, and
`git status --short crates/ xtask/ .github/` was empty at T1. Every R1 number
re-verified EXACTLY by executing `cargo run -p xtask -- kloc-check --json` and
`check-kernel-baseline` against that tree — `maos-cli` 4642/4642, `xtask` 38742/38742,
`maos-a2a-core` 4669/4669, `maos-a2a-tcp` 1137/1500, `maos-audit` 6643/6665,
`_aggregate` 148892 (RED −1835), kernel baseline GREEN at 24472 = 24472. The story
file was NOT stale this time.

**Proven-red evidence, in the order it happened.**

- AC1: `signing_identity_2c.rs` written FIRST. 4 of 7 tests RED before the fix —
  `sealed_export_prints_the_region_key_it_actually_signed_with`,
  `trajectory_export_prints_the_region_key_it_actually_signed_with`,
  `stdout_mode_export_prints_its_pubkey_to_stderr_and_keeps_stdout_pure`,
  `verify_bundle_derives_the_region_key_from_a_base_seed`. Two more were passing
  VACUOUSLY (they were satisfied by clap's unknown-argument error) and were
  strengthened before the fix landed so they could not pass for the wrong reason.
- AC3.2: `fault_typing_2c.rs` failed to compile against HEAD — `no variant named
  PeerInternalFailure` / `PeerIntakeTimeout`. The repair made all 4 pass.
- AC5: 33 proven-red vectors, every one confirmed RED against a green fixture
  baseline; `baseline_fixture_tree_is_green` guards against vacuity.
- D19: 10 vectors, including the binding planted red.

**Two gate defects found by writing the gate's own proven-red, not by review.**

1. `live_lines` filtered every line starting with `#`, which is right for TOML/YAML
   comments and WRONG for Rust attributes: the `ArgGroup` check could never be
   satisfied because `#[command(group(...))]` was invisible. Fixed to treat `#[…]`
   as live code. Caught because the fixture wrote the attribute on one line while
   the real `cli.rs` happens to wrap it across three.
2. The `two-host-vectors-enrolled` derivation used a `_2c.rs` SUFFIX only, so it
   silently missed both `crates/maos-a2a-tcp/tests/t_2c_*.rs` files — recreating the
   hand-maintained-list failure inside the leg built to prevent it, by assuming one
   naming convention. Broadened to both conventions, with a proven-red vector for
   each.

**AC3.5 measurement, against Murat's binding fallback condition.** The story
authorized a return to the room if the `:819` fix came back materially larger than
~10 lines. Measured: `maos-a2a-core` +116 total for AC3.2 + AC3.5 + AC3.6 combined,
of which `:819` is the `observe_reply_guarded` trait method (+~12 in `cohort.rs`), the
router's guard closure (+~14), the `maos-cohort` impl (+1 line moved, `before_commit()?`
placed before the dedup publish) and the `maos-bin` forwarding (+1). **No narrowing was
taken, so no machine-checked narrowing was required.**

**One pre-existing RED closed in passing, reported rather than hidden.**
`check-dev-record-completeness` was already RED at HEAD `7aa07ee3` on
`deferred-work.md:817` and `:820` — both `2b` review text asserting an owner that
resolves to no sprint-status key. Isolated by stashing this story's `deferred-work.md`
edit and re-running the gate. Restated in the gate's own `ownerless and open`
vocabulary; the deferred items are unchanged and still open.

**Five existing gate unit-test suites had to be repaired, and the repair is the
point.** D19's fail-closed rule means a fixture directory of bare `.md` files governs
NOTHING, so 21 pre-existing tests across the five converted gates went red. They were
not weakened: each fixture helper now DECLARES its stories via a shared
`register_fixture_story`, exactly as a real story is declared.
`check_dev_model_tier::unknown_status_is_still_checked` was conceptually split,
because D19 separated two cases the digit filter had conflated — "declared with an
unrecognised status" (still checked) and "not declared at all" (not a story). Both
halves now have their own leg.

**Runtime verification, not just tests.** `maosctl` was driven as a real subprocess
for AC1/AC2/AC4 (`CARGO_BIN_EXE_maosctl`), real TLS listeners were bound and dialed
for AC3, and `python3 tools/verify-audit-bundle/verify.py` was executed against a
host-stamped bundle produced by the real binary — accepting it, and rejecting the same
bundle with its `host` field rewritten.

### Completion Notes List

**AC1 — the bug that would have burned the paid run.** Both `sealed-export` sites now
print the key that actually SIGNED. The obvious edit does not compile (there is no
`region` binding at the print site, and `signed.region` is `Option<String>` while
`derive_region_pubkey` takes `&Region`), so the resolved `Option<Region>` is bound to
a local BEFORE the match consumes it and drives both `with_region(..)` and the pubkey
derivation. The stderr line shape is unchanged, verified against the byte-identical
scraper from `demo_j1.rs` reproduced inside the test. The `--output`-less arm now
prints its key too (stdout stays pure bundle JSON — asserted by parsing it).
`verify-bundle --seed` derives from the bundle's CLAIMED region under an `ArgGroup`,
so exactly one key source is required and the refusal names both.

**AC2 — hosting decision, recorded as the AC demanded.** The two-bundle verb is
**reimplemented natively in `maos-audit`**, not called from `maos-cli`. Reasons, in
order: (i) `maos-audit → maos-loom-lite` closes a CYCLE, since
`maos-loom-lite → maos-audit` already exists; (ii) hosting reconciliation in
`maos-cli` would make two-TL reconciliation a feature of our BINARY rather than of the
artifact FORMAT — a stranger with the bundles and the published keys could not
reproduce it. Native hosting adds ZERO dependency edges, and a gate leg asserts
`maos-audit`'s manifest never gains the loom-lite edge.

The `host` field is an **anti-forgery control**, not a label, and it is bound by the
signature because it is declared on `BundleForSigning` too — a field only on
`AuditBundle` would be rewritable, since `verify_bundle` re-canonicalizes the former.
9.2b byte-identity is **asserted, not assumed**: the pre-2c committed golden sha256
still holds for a host-less bundle, and the JSON round-trip carries no `host` key.

**AC2.4 — the reversal implemented as ratified.** No weld from the shared `base_seed`.
`reconcile_two_host_bundles` REFUSES two halves attested by one root before anything
else succeeds, because the region→team template exists to make keys derivable from ONE
seed — the exact property this AC must disprove. `one_root_signing_both_halves_is_refused`
proves both halves verify individually and the reconciliation still refuses.

**AC2.1 — the stranger's path executed.** `tools/verify-audit-bundle/verify.py`
verifies a host-stamped bundle produced by the real binary, and rejects the same
bundle with `host` rewritten. That is the nearest thing to a stranger that exists, and
it had never been pointed at one of our artifacts.

**AC3 — the faults are now distinguishable, which is what made AC3.4 possible.**
`TcpStream::connect` and `framed.send` were BOTH unbounded; the second is the cheaper
real partition (a peer that accepts and stops reading hangs `route_outbound` forever
with no OS backstop). Both are bounded by the operator-configured
`partition_timeout_secs`, clamped by the injected `timeouts.idle` for two stated
reasons: `TcpTimeouts` is the test-injection seam that keeps this crate inside its
51×-per-push budget, and the response read is ALREADY bounded by `idle`, so a longer
write window could never be observed. A partition is minted as the typed
`A2AError::PartitionTimeout` carrying the frame id, at the one site that holds both.

**AC3.5 — fixed, not bounded.** The invariant is *nothing is `Duplicate` until
something is durable*. `:817` and `:818` are bounded as RELEASE-HOLDS rows 11 and 10,
with the reason each is a different KIND of problem: `:817` is a local `spawn_blocking`
join needing a kernel-core abort path (pinned byte-identical this story), and `:818` is
a CRASH window across two operations with no shared transaction, needing a boot-time
reconciliation pass — a recovery mechanism, not a fault-semantics repair.

**AC3.6 — one `maos-domain` line, and it is the honest one.**
`RuptureReason::PeerIdentityUnverified` was added because every existing variant
presupposes a KNOWN counterparty; recording a pin mismatch as an allowlist or posture
failure would be a claim standing in for a control. `maos-domain`'s ceiling was
deliberately NOT raised — that would erase D14's pre-existing red.

**AC5.1 — why ONE job.** The paid-run job's substrate is an operator, two hosts and a
funded API key. CI has never had them and never will, so an `AdvisorySubstrate` sibling
would take the ABSENT branch on every run for its entire lifetime. **A gate whose
substrate cannot exist is a monument, not a control.** The paid run's evidence is a
capture this gate validates when present and refuses to let anything CLAIM when absent;
`paid_run_capture_present: false` is GREEN and honest.

**AC2.6 — the schema is wired AND corrected in the same change.** It declared
`additionalProperties: false` while omitting three fields the struct emits, read by
zero machines — a false specification, not documentation. It now declares all ten
bundle fields and both missing entry fields, and the gate validates real bundles
against it with a dependency-free `additionalProperties`/`required` validator. Four
proven-red vector families guard it, including AC2.6's named acceptance: a planted
extra field REDs the gate.

**AC5.3 — the owner string was RE-POINTED by RF-0.** `unlanded_beats` reads
`"j1-crosshost-2d-paid-two-host-run"` (the dev pass initially recorded this as
"verified, not edited" — a claim about `2b`'s state that expired with the 2d
split; corrected by the §A6 review, together with five other record sites). The
beat flips by an executed leg (`apply_two_host_signed_run` runs the judge
in-process), never by a published ledger, which is structurally dead twice.

**D19 RESOLVED, not disclosed.** Option (a) under vehicle 14-0: ONE shared helper at
all SEVEN walk sites; six `j1-*` files are now governed by five Blocking gates; the
acceptance is a PLANTED RED, CI-enrolled. `check_dev_model_tier` needed one extra
repair the filter swap alone would not have delivered — its `ENFORCE_FROM_EPIC` scoping
skipped any story with no epic number, so `j1-*` would still have been exempt after
becoming visible. Full record in the decision register.

**Budget — six named measured grants, every one taken AFTER the code existed and was
`cargo fmt`-measured.** `maos-cli` 4642→5035 (+393; AC1 +52, AC2.2 +150, AC4+AC5.5
+191, measured separately as blocking condition 2 required), `maos-audit` 6665→6822,
`maos-a2a-core` 4669→4785 (`kloc.toml:87` correctness-repair, citing 2b's precedent),
`maos-iac` 6888→6927 (after the mandated free reduction — the compiler-confirmed-dead
`parse_frame_id_hex_field`), `maos-bin` 16738→16739, `xtask` 38742→39808.
`maos-a2a-tcp` +77 and `maos-cohort` +53 fit existing ceilings. `maos-domain` (+1) and
`_aggregate` (+1926) were deliberately NOT raised, because raising them would erase
D14's and D17's reds and let this story's growth hide inside someone else's overage;
the split is stated in `kloc.toml` instead. **kloc-check reds on exactly the same
three pre-existing keys as HEAD — no new key.** Kernel baseline GREEN at 24472 = 24472,
ZERO delta; the tempting `env_clear()` edit was NOT made, and a test asserts it stays
absent.

**What this story does NOT claim.** The paid two-host run has not happened: blocking
condition 3 (rehearse the manual boot-nonce pairing on a RELEASE build) is an operator
action, and Phase 7.0 of the runbook now exists for it. `PROVEN_LIVE_SIGNED` is
`INDETERMINATE` in CI because CI holds no operator key by ratified design — not because
no leg has ever reached that state (27 have, on the operator lane).

### File List

**New — production (1)**

- `xtask/src/check_j1_two_host_signed_run.rs`

**New — tests, all kloc-free (10)**

- `crates/maos-cli/tests/signing_identity_2c.rs`
- `crates/maos-cli/tests/two_host_reconcile_2c.rs`
- `crates/maos-cli/tests/credential_posture_2c.rs`
- `crates/maos-audit/tests/two_host_bundle_2c.rs`
- `crates/maos-a2a-core/tests/fault_typing_2c.rs`
- `crates/maos-a2a-core/tests/digest_reply_durability_2c.rs`
- `crates/maos-a2a-tcp/tests/t_2c_fault_windows.rs`
- `crates/maos-a2a-tcp/tests/t_2c_pin_journal.rs`
- `xtask/tests/j1_crosshost_2c_proven_red.rs`
- `xtask/tests/d19_story_file_governance.rs`

**Modified — production (18)**

- `crates/maos-cli/src/cli.rs`
- `crates/maos-cli/src/subcommands.rs`
- `crates/maos-audit/src/sealed_export.rs`
- `crates/maos-a2a-core/src/cohort.rs`
- `crates/maos-a2a-core/src/error.rs`
- `crates/maos-a2a-core/src/lib.rs`
- `crates/maos-a2a-core/src/router.rs`
- `crates/maos-a2a-core/Cargo.toml` (dev-dep `parking_lot`)
- `crates/maos-a2a-tcp/src/error.rs`
- `crates/maos-a2a-tcp/src/transport.rs`
- `crates/maos-a2a-tcp/Cargo.toml` (dev-dep `parking_lot`)
- `crates/maos-cohort/src/state.rs`
- `crates/maos-domain/src/frame.rs`
- `crates/maos-iac/src/adapter/redaction.rs`
- `crates/maos-iac/src/adapter/distillate.rs` (free reduction: dead fn deleted)
- `crates/maos-bin/src/main.rs`
- `Cargo.lock`
- `schemas/audit-bundle.schema.json`

**Modified — xtask gates + D19 (11)**

- `xtask/src/main.rs`
- `xtask/src/lib.rs`
- `xtask/src/gate_common.rs`
- `xtask/src/demo_j1.rs`
- `xtask/src/check_ship_gate_completeness.rs`
- `xtask/src/check_bare_review_findings.rs`
- `xtask/src/check_dev_model_tier.rs`
- `xtask/src/check_dev_model_used_populated.rs`
- `xtask/src/check_dev_record_completeness.rs`
- `xtask/src/check_review_findings_resolved.rs`
- `xtask/src/check_epic_6_bridge.rs`

**Modified — registration, config, records (10)**

- `xtask/gate-registry.toml`
- `xtask/kloc.toml`
- `.github/workflows/discipline.yml`
- `crates/maos-bin/tests/bounded_postures_2b.rs` (census 10 → 12 of 16)
- `crates/maos-bin/tests/consent_refusal_1b.rs` (**`cargo fmt` reflow only**, no
  behavioural change)
- `RELEASE-HOLDS.md` (claim boundaries 9-12)
- `_bmad-output/implementation-artifacts/deferred-work.md` (`:819` resolved; `:817`,
  `:818`, `:820` restated)
- `_bmad-output/implementation-artifacts/sprint-status.yaml` (2c → `review`; R13's
  disproved prediction corrected)
- `_bmad-output/planning-artifacts/epics/epic-14-preflight-decisions.md` (D19 resolution)
- `_bmad-output/test-artifacts/traceability-matrix.md` (J1 lane, 24 rows — was zero)
- `_bmad-output/test-artifacts/runbook-j1-tier-2-signed-live-run.md` (Phase 7)
- `_bmad-output/implementation-artifacts/j1-crosshost-2c-two-host-signed-run.md` (this file)

---

## Open Questions

**Q1 — RESOLVED, and now settled by construction.** `2b`'s mechanism proof runs **two real OS
processes** (`two_host_delegation_2b.rs:291,311,457,471`, `CARGO_BIN_EXE_maos`), each with its own
config, audit DB and mTLS identity. In MAOS's vocabulary a Host is a process with its own Transparency
Log, identity and cert, so "two hosts, one box" is architecturally honest — but a reader will hear "two
machines", so the **artifact** carries the distinction (AC5.5). The only remaining operator call is
whether to fund a two-machine paid run.

**Q2 — RESOLVED by `2b`, with a new obligation attached.** `2b` shipped the boot-nonce gap as a stated
boundary (RELEASE-HOLDS row 8), not a fix. So the paid run uses **manual out-of-band pairing**: the
operator reads host A's nonce from its `cohort:daemon-started` TL row and transcribes it into host B's
static peer-pin config (`crates/maos-a2a-tcp/src/config.rs:25-36`). The original dead-end — *"a fix
would land in `maos-a2a-core` at zero headroom"* — is **dissolved by R2**: that wall is passable with a
named `kloc.toml:87` grant, and `2b` just took one. **The obligation is R8: that manual path has never
been executed.** Rehearse it on a release build before the agent is billed (blocking condition 3).

**Q3 — NEW, and it is the only one this story cannot answer itself. How is D19 resolved?**
Its deadline is *before this story leaves `ready-for-dev`*, its text forbids disclosure as a
disposition, and its owners are Mary + John under vehicle 14-0. Option (a) replaces the digit-prefix
filter in all seven walkers with the sprint-status key set; option (b) ratifies bridge-lane story files
as outside story-file discipline and states the boundary in `RELEASE-HOLDS.md` §Claim boundaries.
**Neither is 2c's to choose.** Escalate at T0, not at T14 — if the answer is (a), seven walkers change
and this story's own file becomes gate-visible mid-flight.

---

## Change Log

| Date | Change |
|---|---|
| 2026-08-18 | **ROUND-TABLE AT `review` — what `done` is allowed to mean, and three dispositions that named no owner.** (Mary · Paige · John · Sally · Winston · Amelia · Murat; Grumbal and Dana walking on.) State at open: `2b` COMMITTED at `7aa07ee3` (blocking condition 1 DISCHARGED), the dev pass complete (15/15 tasks, gate `check-j1-two-host-signed-run` landed with 10 legs, D19 resolved option (a) and the walker conversion fired on this very file exactly as Amelia predicted), story at `review`, §A6 net NOT yet run. **(1) THE SPLIT — `2c` is NOT the lane closer, and `blocks: NONE` was false in two ways.** The capture at `_bmad-output/test-artifacts/j1-two-host-evidence/` is ABSENT: the paid run has not happened, leg 9 `paid-run-capture` correctly refuses the claim, and the `two-host-signed-run` beat stays ABSENT. So the story named *the two-host signed run* was one review away from `done` **without a two-host signed run ever having happened** — and closing it with no successor would leave **an ABSENT beat whose owner is a `done` story**, which `demo_j1` renders against its owning story as work that is coming. New shape: **a widowed control** (an ABSENT beat naming a closed owner). Dana blocked the hold-until-run option — the mechanism is proven-red TODAY and hostaging it to a calendar rots real work. Grumbal blocked the easy split — *every* story in this lane has handed the real thing to its successor. Resolved by the distinction that survives both: **`2c` DONE means the judge is built and proven-red; it does NOT mean the run happened.** Successor `j1-crosshost-2d-paid-two-host-run` owns the run, the capture, the boot-nonce rehearsal and host B's separate audit key — and is **CODE-FREE by construction** (Grumbal's condition, binding): if it acquires an AC that writes a line of Rust it is the wrong row. Precedent is `2b`'s own ratified Q4 beat split, not a new pattern. **(2) THREE DISPOSITIONS NAMING NO OWNER — ROUTED, and the finding CORRECTED against ourselves.** `deferred-work.md:817/:818/:820` closed as *"Ownerless and open: no story successor exists"*. The room first read this as a novel failure; **measurement disproved that** — `Ownerless and open` is a STANDING CONVENTION in that file, 9 uses across many epics, long predating this lane. What is genuinely wrong is narrower: these three are *new* debt created by the lane closer, so "no successor exists" is a statement of fact with no remedy attached. Routed to named lanes and schedulers (817 → Epic-14 kernel lifecycle, FLAG-Winston, needs an abort path in `run_cli_wrapper_manifest` against the 24472 pin; 818 → Epic-14 recovery/reconciliation, a boot pass over `Written`-without-execution rows; 820 → worker-grant hardening, and flagged **FAIL-OPEN/security-relevant** because it defaults an omitted `permitted_tier` to T3, highest privilege, against its own doc). **Routing a lane is NOT having a story** — stated in each entry so it cannot read as closure. **(3) RECORD CORRECTIONS.** `RELEASE-HOLDS.md` rows 9-12 are written in the past tense about an artifact that does not exist; row 11 verified TRUE at the working tree (`connect` and `framed.send` both bounded, `partition_timeout_secs` wired to `maos-a2a-tcp:943`), row 9 describes what the design permits and must not read as a report on a run. 5 ACs held. |
| 2026-08-17 | **DEV PASS — 15 tasks, 5 ACs, D19 resolved, six measured grants.** Model `anthropic/claude-opus-5`, harness omp, baseline `7aa07ee3` == HEAD (blocking condition 1 discharged, not waived; every R1 number re-verified exactly, so the file was NOT stale a fifth time). **AC1** — the P12 signing-identity bug repaired at BOTH sites plus the key-less stdout arm; 4 of 7 tests RED before the fix, and two more that were passing VACUOUSLY were strengthened first so they could not pass for the wrong reason. `verify-bundle --seed` derives from the CLAIMED region under an `ArgGroup`. **AC2** — `host` declared on BOTH bundle structs so it is bound by the signature (a field only on `AuditBundle` would be rewritable); pre-2c golden sha256 still holds, so byte-identity is ASSERTED not assumed; `reconcile_two_host_bundles` refuses a SHARED ROOT before anything else succeeds — the AC2.4 reversal implemented as ratified, no weld from `base_seed`; hosting decision recorded as native-in-`maos-audit` because the reverse edge closes a cycle AND because `maos-cli` hosting would make reconciliation a feature of our binary rather than of the format; **the stranger's path executed** — `verify.py` accepts a host-stamped bundle and rejects a rewritten `host`. **AC3** — `connect` AND `framed.send` bounded (the second had no OS backstop at all) with `partition_timeout_secs` finally reaching the TCP path and a typed `PartitionTimeout` carrying the frame id; `CODE_INTERNAL`/`CODE_TIMEOUT` typed, closing the gap `2b` filed against this story IN THE SOURCE — census 10→12 of 16, machine-checked, not a comment; three correctly-named fault windows in 0.25s inside the 51× loop; **`:819` FIXED, not bounded** (nothing is `Duplicate` until something is durable — the intake hand-off is now the reply's commit guard, measured well inside Murat's fallback threshold so no narrowing was taken); pin refusals journaled on BOTH sides with the listen-side negative asserting on the SERVER's journal, since under TLS 1.3 the dialer may only see `Io`. **AC4** — the read-path stored-row scan that existed NOWHERE, reporting both classes distinctly and never echoing the secret it finds; the credential posture ASSERTED rather than changed (`env_clear` stays absent — adding it would be a regression and a kernel breach). **AC5** — ONE always-`Blocking` hermetic gate, because a second `AdvisorySubstrate` job would need a substrate CI can never have and would fire never: a gate whose substrate cannot exist is a monument, not a control. Registered in all five slots, no `services:` block, `LegAudit` on all 10 legs, 33 proven-red vectors, schema WIRED and corrected in the same change with AC2.6's planted extra-field red as its acceptance. Beat flipped by an executed leg; owner string VERIFIED not edited. **TWO GATE DEFECTS found by writing the gate's own proven-red** — `live_lines` blinded every Rust attribute (so the `ArgGroup` leg was unsatisfiable), and the enrollment derivation missed both `t_2c_*` files, recreating the hand-maintained-list failure inside the leg built to prevent it. **D19 RESOLVED, not disclosed** (option (a), vehicle 14-0): one shared helper at all SEVEN walk sites, six `j1-*` files now governed by five Blocking gates that could not see them, fail-closed so a gate can never govern an empty set, and a PLANTED RED as the binding acceptance test. The sequencing hazard fired exactly as Amelia predicted — `check-dev-model-tier` went RED on THIS file the moment the conversion landed. 21 pre-existing gate unit tests were repaired by DECLARING their fixtures, never by weakening them. **One pre-existing RED closed in passing and reported, not hidden.** Budget: six grants, `maos-domain` and `_aggregate` deliberately NOT raised so D14/D17 stay visible; kloc-check reds on exactly the same three pre-existing keys as HEAD; kernel baseline GREEN at 24472, ZERO delta. |
| 2026-08-17 | **PREFLIGHT ROUND-TABLE — nine open calls closed unanimously per spec + long-term correctness, and the room reversed TWO of this file's own ratified answers.** (Mary · Paige · John · Sally · Winston · Amelia · Murat; Grumbal and Dana walking on.) **(AC2.4 REVERSED — the biggest find.)** Sally asked what stops one machine holding both keys. Measurement answered: `derive_team_signing_seed` welds over `derive_region_signing_seed`, and **every weld descends from ONE `base_seed`** — the doc comment says so outright (*"an attacker who recovers the region signing seed can derive every team key in that region"*). So the ratified "stage 3 of the region→team template" would have made **one seed-holder able to legitimately sign BOTH halves**: valid signature, host field inside it, a perfect "two-host" bundle from one machine. **The room picked the one primitive in the repo that guarantees the property the AC must disprove.** Reversed to **independent per-host roots**; and since `2b`'s two-process-one-box shape defaults to one HOME and therefore one key file, **the mechanism proof and the signed proof want opposite setups** — host B's key becomes a second hand-provisioned step. New shape: *a control reused from a primitive built for the opposite job.* **(AC2.1 bounded.)** The host field defeats a forger who does not hold the other key; it does **not** prove separation. Reach stated exactly: *two keyed identities signed* — never "two hosts". **(AC5.1 REVERSED — "two jobs" withdrawn.)** Murat asked what substrate the paid-run job needs: an operator, two hosts, a funded key — which CI will never have, so the `AdvisorySubstrate` job takes the ABSENT branch **for its entire lifetime**. *A gate whose substrate cannot exist is a monument, not a control.* Collapsed to **ONE always-`Blocking` hermetic gate** that validates the paid run's **capture** when present and refuses the claim when absent — cheaper in `xtask`, which is at zero. **(D19 → option (a), unanimous.)** One shared helper across all seven walkers; option (b) refused because it would convert a defect into a policy exactly as the defect expired, exempting the *most* security-critical lane from review discipline. Acceptance = **plant a `j1-*` file with a missing dev record, watch a Blocking gate RED**. Mary named the deadline **self-voiding** — *"before the next `j1-*` story"* when 2c is the closer and there is no next one (**shape #39: a deadline whose successor was cancelled**). Amelia added the sequencing hazard: option (a) landing mid-flight makes 2c's own file gate-visible. **(`:819` → FIX, not bound.)** Winston's narrowing compromise withdrawn: it is the **same durability lie `2b` fixed one layer over, reached by a different door**, and bounding it ships *"we fixed the durability lie except where we didn't."* Invariant: **nothing is `Duplicate` until something is durable.** **(AC3.2 grant ratified by Dana)** — the `kloc.toml:87` valve opened for H13 in 2b by line number; refusing the same defect class now would mean the valve was a mood. Two codes plus `:819`, measured after, scope wall held — **and fix the census comment at `router.rs:1124-1130`, which lists `INTERNAL` as "not newly reachable" while the same story newly emits it twice** (*a scope wall that mis-states its own scope*, tallied separately). **(AC2.6)** Paige refused the third option: a schema declaring `additionalProperties: false` while omitting three emitted fields is **a false specification, not documentation** — wire it or delete it. **(AC1.3)** stdout arm prints the pubkey to stderr, identical line shape. **(AC4.1)** the read-path scan asserts **both** classes — the write path handles both, so a hit on either is a redaction escape, and asserting only the prefix half blinds it to the class handled *silently*. **(Sally's closer, and the round's through-line.)** *"Has a stranger ever checked one?"* — no. So the artifact is a control whose consumer does not exist, and our own `verify-bundle` is a self-check. **The run is not complete until `tools/verify-audit-bundle/verify.py` verifies the produced bundle**; it is field-agnostic (`verify.py:91-93`) and already exists. Two human-performed steps promoted from "did we do our homework" to **properties of the system** in the artifact. 5 ACs held — John's rule unbroken, every resolution a constraint on an existing AC. "Claim standing in for a control" tally: **40**. |
| 2026-08-17 | **RE-BASELINED `5a921c0c` → `2b`'s working tree, and moved `backlog` → `ready-for-dev`.** Five parallel scouts re-derived every number and every anchor. **Fifteen re-baseline findings (R1-R15); the fourth time in this lane a story's numbers were invalidated before it started.** Headline: **(R1) BUDGET INVERTED — `xtask` went +223 → ZERO**, so this story needs **two** named grants (`maos-cli` for AC1, `xtask` for AC5), not the one its own text advertised; `_aggregate` red deepened −885 → **−1835**; three of the four crates 2c must write in sit at literal zero. **(R2) D10's wall is passable and `2b` just walked through it** — `maos-a2a-core` 4654→4669 under `kloc.toml:87` cited by line number — which dissolves Q2's dead-end. **(R3) F10 is INVERTED**: `main.rs:9800` now calls `bind_with_intake_sink`, so AC3's fault windows (b) and (c) have a real target for the first time. **(R4) SHIP-BLOCKER for AC3** — `CODE_INTERNAL` and `CODE_TIMEOUT` both fall through `interpret_response`'s catch-all (`router.rs:1135`) into `TransportFailed`, so a dropped-receiver NACK and a genuine partition are byte-identical at the sender; `2b` filed this against this story **in the source** (`router.rs:384-388`), and the two untyped codes are **exactly the two AC3 must distinguish**. Added as AC3.2 with 2b's H13 scope wall; also corrects the census comment, which claims `INTERNAL` is "not newly reachable" while the same story newly emits it twice. **(R5)** three inherited deferrals, not two — `deferred-work.md:819` (digest-reply retry ACKed `Duplicate` after a dropped-receiver NACK) is a **live counterexample inside AC3's own duplicate-safety claim**. **(R6) AC5.2 was unbuildable** — no gate in this repo mixes binding classes inside one job; resolved to **two jobs**. **(R7) AC5.3 is already done** by 2b (owner is `"j1-crosshost-2c"`; the literal `"j1-crosshost-2"` has zero hits) — scoped down to *verify*. **(R8) MONEY RISK** — the manual boot-nonce pairing is the only path a release build has and **has never been executed**; new blocking condition. **(R9)** D19's deadline is this story's **closure**, and disclosure is explicitly no longer an acceptable disposition — AC5.6 rewritten from "disclose" to "resolve", raised as **Q3** and a blocking condition, escalated to its real owners. **(R10)** anchor drift is arithmetic: `transport.rs` uniform **+87** from one clean 2b insertion, `router.rs` +97, `subcommands.rs` +36 after site 1, `maos-audit`/`maos-loom-lite`/`maos-domain` **+0**. **(R11)** `deliver_typed` now returns typed `Written`/`Duplicate` and fixed a **remote-triggerable panic** that AC2's determinism plus AC3's retries is the exact input for. **(R12)** the `frame_id` join key is no longer designed but **proven** by an executed CI test (`two_host_delegation_2b.rs:533-535`). **(R13)** peer auth is CLOSED via 2b's 7th gate leg `cross-host-identity-proof`, **not** by the leg flip `sprint-status.yaml:269` still predicts — that record is stale. **(R14)** small corrections that would cost a compile: `env_clear` 22 not 23, `MAOS_AUDIT_KEY_SEED` at `main.rs:8344`, `nonsecret_env` at `worker_cli.rs:655-663`, the overclaim negative not at `:3935`. **(R15)** `2b` is `done` but **uncommitted** and its own file does not say so — new blocking condition, because a grant in an uncommitted file is not a ratified ceiling. Also new: **`maos-audit` CANNOT depend on `maos-loom-lite`** (cycle), so AC2.2 must name its hosting decision. 5 ACs held (John's rule), 22 traps, 15 tasks; ZERO kernel-Δ re-verified GREEN at 24472 by execution. |
| 2026-08-16 | **Preflight round-table** (same room). **One AC corrected, one AC's meaning changed by a question nobody had asked.** **(F13 corrected.)** AC2.3 said to project `correlation_id` and budget it against `maos-audit`'s +22. `correlation_id` *is* dropped from that SELECT, but it is **not the join key**: `frame_id` is selected **first** and `deliver_typed` writes the *received* frame's id, so both bundles already carry it. Reconciliation costs **zero** `maos-audit` lines. **(AC2.1 reframed.)** Sally asked how you tell the two logs apart when the join key is identical on both sides. Nothing else in the artifact does — `region` cannot, `boot_nonce` is per-boot and T6 swept eight, `attester_pubkey` is bundle-supplied and R-RG1 forbids trusting it. **So without the host discriminator one host can produce both halves of a "two-host" bundle.** It is an anti-forgery control, not a label, and it must be **bound by the signature**. **(Q1 resolved)** Two processes prove the mechanism; the artifact — not the story — records whether the paid run was two processes or two machines. 5 ACs unchanged. |
| 2026-08-16 | **Created** at clean `5a921c0c` from a five-scout preflight, following the 2026-08-15 ratification of the `2a/2b/2c` split. Status **`backlog`** with three blocking conditions. **Fourteen premises disproved or corrected, three of which overturned statements then recorded as project fact.** Headline: **(F1) P12 is in TWO places** — both print a pubkey that is not the signing key whenever a region resolves, `verify-bundle` never derives, the `--output`-less arm prints no key at all, and `demo-j1` already scrapes that key and feeds it to `verify-bundle`, so the existing Tier-2 leg fails **after** the agent is billed; AC1 lands first for that reason. **(F2)** two-TL reconciliation is **not greenfield**. **(F3) INVERTED — the missing `env_clear` is LOAD-BEARING**, so the deliverable is a negative test asserting the posture. **(F4)** `MAOS_AUDIT_KEY_SEED` does not exist and `PROVEN_LIVE_SIGNED` **has** been reached (27 legs, operator lane). **(F5)** the beat cannot be flipped by a ledger — dead twice. **(F6)** "partition" is **two** unbounded operations. **(F7)** a rustls verifier provably cannot reach a TL and the cited warn is structurally unreachable, so the listen side leaves *zero* trace. **(F8)** all three pin tests are dial-side. **(F9)** no durable TOFU store exists. **(F10)** two of the three sketched fault windows had no target and the third was mis-named. **(F11)** `2c` owns only the read-path stored-row scan. **(F12)** `region` cannot discriminate two hosts in one region. **(F13)** `maos_audit::query` drops `correlation_id`. **(F14)** two null controls sit under this story. 5 ACs, 20 traps, 12 tasks; ZERO kernel-Δ where the correct action on the one kernel surface is **do not touch it**. |
