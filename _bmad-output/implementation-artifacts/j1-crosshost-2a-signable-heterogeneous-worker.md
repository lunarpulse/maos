---
baseline_commit: 5a921c0c
depends_on: "NONE. This story is unblocked by the 2026-08-15 split ratification — it is one-host worker hardening and cannot surface a cross-host deny, so it does NOT wait on `j1-crosshost-1b`."
blocks: j1-crosshost-2b-cross-host-delegation-mechanism
split_from: j1-crosshost-2-cross-host-signed-run (three-way split RATIFIED by Lunarpulse 2026-08-15; that file is the shared preflight for 2a/2b/2c)
kernel_grant: "NONE, and the story is scoped to keep it that way. `check-kernel-baseline` GREEN at **24472** (`xtask/kernel-core-baseline.toml:472` — re-pinned 2026-08-13 for the Epic-5 review closure; **23679 is stale, do not inherit it**). One option considered and DEFERRED specifically because it would breach: a working-tree effect oracle needs a `cwd` on `BridgeSpawnSpec` and a `.current_dir()` in `spawn_and_bridge` (`crates/maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs:240-269`, `:449-473`) — ~4 kernel-core lines, FLAG-Winston. See AC1 preamble. Do NOT cite `abi-diff` as evidence of anything here: it scopes to `crates/maos-spirit-abi` only (`xtask/src/abi_diff.rs:8`), open **FLAG-E4**."
kloc_grant: "**NONE REQUIRED — and do not ask for one until you have run the relocation in T1.** `maos-bin` is at 16219/16219 (zero headroom, D15, ratified 2026-08-15). The relocation in AC1.1 returns **exactly +204** to that crate by moving an in-`src` test module that is both budget-charged and CI-invisible — `tokei` on `worker_cli.rs:399-658` reports **204 CODE** / 31 comments / 25 blanks, re-measured at the 2026-08-16 preflight. Not an estimate; do not re-approximate it in either direction. Measure again after T1 to confirm the crate total, and only if you are still over do you ask, WITH the measurement attached (`kloc.toml:60-65`). `xtask` has **+534**. `crates/*/tests/`, `xtask/tests/` and the **entire `spirits/` tree** cost ZERO."
model: frontier-class {opus-4-8, gpt-5.5, glm-5.2, opus-5, equiv}
review: §A6 full-layer net (Blind + Edge + Acceptance + Test-Infra + runtime) — NON-DEGRADABLE (this story decides what the word "completed" means in a signed artifact)
---

# j1-crosshost-2a — a signable heterogeneous worker

Status: **done**

> **What this story is.** Before anything crosses a host boundary, one host has to be able to tell
> the truth about whether its worker did the work. Today it cannot. A live `claude -p` refused a
> write, exited 0, and the completion oracle scored it `completed: true`. That verdict is not a
> report — it is the **admission condition for signing** (`xtask/src/demo_j1.rs:998-1001`:
> `if !completed { return Err("the live worker did not complete — nothing to sign") }`). Everything
> `2b` and `2c` build sits on top of it.
>
> **The split hazard, and how it is closed.** `2a` is mechanism *and* judge in one story — the shape
> that forced the 1→1a/1b and 13.6→13.6a/13.6e corrections. It is bounded instead by scope: `2a`
> touches exactly one host and zero network, so its whole claim is falsifiable by a hermetic test
> with a fake binary on `PATH`. The proven-red harness that does it **already exists**
> (`crates/maos-bin/tests/smoke_cli_wrapper_8_12.rs:150-208`) and needs three edits, not a design.

---

## ⚠ Read this block before the ACs — the ratified scope is wrong or imprecise in twenty-four places

Every line number below was re-derived at `5a921c0c` by five parallel scouts reading the files, one
of which extracted flag surfaces from the installed `claude` and `codex` binaries. **Inherit no line
number from the shared preflight, from `sprint-status.yaml`, or from memory.** Two numbers that are
already stale in circulating documents: the kernel pin is **24472**, not 23679; and `kloc-check` is
red on **four** keys at committed HEAD, not three (the D15 grant is uncommitted in the working tree).

*Preflight round-table 2026-08-16 added **F6a** and **F21-F24**, closed **five either/or forks** inside
the ACs (AC1.5, AC1.6, AC2.2, AC2.5, AC4.1), and closed **Q1 and Q2**. Two of the additions are
ship-blockers in the story's own machinery: **F6a** (AC3.1 and AC4.1 contradict each other and would
desynchronize a signed artifact) and **F23** (AC1.9's falsifier cannot fire). If you are reading a
copy of this story that has an "either/or" in an AC, you have an old copy.*

### The six findings that change what you build

**F1 — SHIP-BLOCKER: `worker_cli` is bin-private, so AC1's proven-red vector has no legal home.**
`crates/maos-bin/src/main.rs:44-45` declares `#[cfg(feature = "network")] mod worker_cli;` and
`crates/maos-bin/src/lib.rs` does **not** re-export it. Nothing under `crates/maos-bin/tests/` can
name `ClaudeCli`, `CodexCli`, `parse_completion`, `select_worker_cli`, `live_agent_gate` or
`refuse_ambient_auth`. The story as ratified asks for a vector that cannot be written.
*The repo already states the fix as doctrine and this module is the violation.* `lib.rs:19-22`, on
`pub mod topology`:
> `[topology]` manifest parsing. In the library, not `main.rs`, so `crates/maos-bin/tests/` can
> execute it — an in-`src` test module is budget-charged and CI-invisible.
Both halves are true of `worker_cli` today: its `#[cfg(test)] mod tests` (`:399-658`, **204 tokei
CODE lines exactly**) is charged to `maos-bin`'s ceiling, and **no CI job runs `-p maos-bin` unscoped** — all
four invocations use `--test <name>`, so those 204 assertions never execute in CI. Mirror
`delegation`'s treatment exactly (feature-gated `pub mod`, `lib.rs:9-13`) — that is the proven
pattern in this very file. **Do this first, in its own commit** (T1), so the measurement is
attributable.
*Two viability facts, measured at the 2026-08-16 preflight so the dev does not discover them mid-move:*
`crates/maos-bin/Cargo.toml:15` is `default = ["network"]`, so a feature-gated `pub mod` is visible to
`crates/maos-bin/tests/` with no extra flags; and the test module's **only** import is `use super::*`,
naming **no** private item — `final_stdout_message_oracle` (`:211`) and `final_nonempty_line` (`:203`)
are the two private items in the file and neither appears anywhere in `:399-658`. The relocation
therefore compiles without making anything `pub` to satisfy it. If your version of the change needs a
new `pub`, you have changed the tests; stop and re-read this line.

**F2 — `completion_tl_ref` is NOT produced by the oracle, so "no citable ref for a refusal" is
unsatisfiable as written.**
`main.rs:1258` assigns `completion_tl_ref = Some(row.frame_id)` on **every** stdout row, inside the
read-back loop, and `parse_completion` is not called until `:1270`. The two values are emitted side
by side in one `json!` (`:1277` and `:1280-1281`) with no causal relationship. Changing the oracle
changes `completed` and nothing else.
*Consequence:* `xtask/src/demo_j1.rs:623-628` scores its P4 beat `state_of(completed &&
!tl_ref.is_empty())` — conjoining two causally unrelated facts, so the `tl_ref` half is a **null
control** that is true whenever the worker printed anything at all. AC1.6 fixes the field, not the
phrasing.

**F3 — There is a SECOND false-success surface, and the oracle fix does not close it.**
`is_completed()` is branched on at **exactly one site**: `main.rs:3961`, guarded by
`if delegated_task.is_some()` (`:3960`). The standalone `[cli_wrapper]` path
(`main.rs:4357-4366`) calls `run_cli_wrapper_manifest(..., None)?;` and **discards the returned
`WorkerCompletion` entirely**, then `return Ok(())`. So `maos run spirits/worker/manifest.toml
--once` already exits 0 today when the oracle says `completed: false`. The `None` is deliberate (1a
AC3.5, comment at `:4354-4356`: no delegation, so no frame to close) — but the *verdict* being
dropped is a separate thing from the *task* being absent, and no comment covers it.

**F4 — A structured oracle is architecturally blocked as a drop-in, and the two adapters are
asymmetric — one uniform oracle AC would be false for one of them.**
`parse_completion(&self, stdout, stderr, exit)` (`worker_cli.rs:104-109`) cannot know whether
`--output-format json` was in the argv: the adapter never sees `argv_prefix`, and all three `argv()`
impls are byte-identical `vec![task.to_string()]` (`:249-251`, `:298-300`, `:346-348`). Flags live
only in the manifest, hashed into the cap-token (`main.rs:1132` → `runtime.rs:453-456`). An adapter
that *assumes* JSON while the manifest omits the flag sees prose, fails to parse, and turns a real
success into a **false negative** — the inverse of the bug you are fixing.
Measured on the installed binaries (`claude 2.1.233`, `codex-cli 0.144.4`):
- **codex `exec --json`** emits a `ThreadEvent` JSONL stream with `item.completed` of type
  **`file_change`** (`add`/`delete`/`update`) and an explicit `turn.failed` terminal. That is a real,
  adapter-native **effect** signal.
- **claude `--output-format json`** emits a result object whose refusal case is
  `subtype: "success"`, `is_error: false`. Only `permission_denials.length > 0` distinguishes it —
  and a model that simply **declines without attempting a tool call** leaves that array empty and is
  indistinguishable from success. claude's JSON detects *this* defect; it is not a completion oracle.
*Consequence:* AC1.2 specifies **two different oracles**, and AC1.3 lands the seam that lets an
adapter demand its flags. Do not write "the adapters share a structured oracle."

**F5 — The manifests are not missing. They are committed on another branch, and porting them
verbatim breaks two ways.**
`git log --all --diff-filter=A -- '*manifest-codex.toml'` → **`60d4080a`** ("Save J1 live worktree
files", 2026-07-19) on branch **`maos-j1-live`**, which is not an ancestor of HEAD. Both files are
recoverable byte-for-byte via `git show 60d4080a:<path>`. **The verb is port, not author** — and
authoring fresh risks a *different* argv than the one the signed T6 bundle attests.
But the T6 artifact is **stale at birth against Story 1a**, twice:
1. `j1-founder-loop-codex.toml@60d4080a` carries `priority_weight` at `:13,:17,:21,:29`. 1a's
   `crates/maos-bin/src/topology.rs:72-79` now **rejects unknown keys** — so a verbatim commit reds
   `crosshost_1a_every_shipped_topology_parses_under_strict_keys`
   (`crates/maos-bin/tests/topology_delegation_1a.rs:196-222`), which runs in a **blocking** CI job
   at `.github/workflows/discipline.yml:1821`.
2. It has **no `host` key**. Without it `main.rs:3894-3896` sets `delegated_task = None`, the
   frame-borne emit is skipped, `topology_worker_admit` prints `frame_borne: false`, `task_args` is
   empty (`main.rs:1074-1077`), and the completion enforcement at `:3960-3967` is **bypassed
   entirely** — codex would be spawned with no task at all.
Also note `spirits/topologies/j1-founder-loop.toml` **does exist and is tracked**; only the `-codex`
variants are absent. The shared preflight and `sprint-status.yaml:268` conflate the two.

**F6 — "claude has no `--sandbox workspace-write` counterpart" is false and inverted. But the
follow-on claim "MAOS passes NO sandbox flag" is true only of the TRACKED TREE — the signed T6 run
DID pass one, and it was hash-bound.** *(Corrected at the 2026-08-16 preflight round-table; the
original wording of this finding overstated the gap and would have led AC4 to confess something the
project actually did right. Read F6a below before AC4.)*
Every occurrence of `--sandbox workspace-write` in the tracked tree is a **doc comment**
(`worker_cli.rs:288`, `xtask/src/check_j1_loopback_delegation.rs:47`,
`spirits/orchestrator/src/lib.rs:61`). `CodexCli::argv` returns the bare task. The only shipped
`[cli_wrapper]` manifest is the fixture's (`spirits/worker/manifest.toml:8-9`,
`argv_prefix = ["--maos-worker"]`), the in-tree codex smoke manifest uses `argv_prefix = ["exec"]`
with no `--sandbox` (`smoke_cli_wrapper_8_12.rs:174`), and **no test asserts a sandbox flag is
present in argv**. The FS-jail posture is currently prose for both adapters.
Measured, claude's counterpart is *different-shaped and stronger*: `claude 2.1.233` carries `bwrap`
×42 / `seccomp` ×84 / `unshare` ×22 with `sandbox.enabled`, `filesystem.denyRead`/`denyWrite`,
`credentials.files`/`envVars`, a **fail-closed hard gate** ("exit with an error at startup if
`sandbox.enabled` is true but the sandbox cannot start"), `--bare` (auth strictly
`ANTHROPIC_API_KEY`/`apiKeyHelper`, OAuth and keychain never read — **no codex counterpart**), and
`network.allowedDomains` enforced through a socat proxy — which is precisely what MAOS journals
today as `"egress": "declared-not-enforced"` (`main.rs:997-1000`). All of it rides in `argv_prefix`,
which is an unconstrained `Vec<String>` (`crates/maos-manifest/src/manifest.rs:3983`) and is
TOCTOU-hashed into the cap-token. **AC4 ratifies a posture; it does not confess a gap.**

**F6a — SHIP-BLOCKER between two of this story's own ACs: AC3.1 and AC4.1 contradict each other, and
following both silently desynchronizes the committed manifest from the signed T6 bundle.**
The manifest you are told to port already carries the flag. `git show
60d4080a:spirits/worker/manifest-codex.toml:25`:
```toml
argv_prefix = ["exec", "--sandbox", "workspace-write"]
```
And `_bmad-output/test-artifacts/j1-tier2-evidence/j1-tier2-capture.json` — the **Ed25519-signed**
capture — records `command_metadata` as
`codex exec --sandbox workspace-write "<c2 task>"; CODEX_API_KEY injected host-side (value redacted)`.
So the T6 signed run had a **real FS jail**, supplied by MAOS through the manifest and bound into
`argv_prefix_hash` (`main.rs:1132` → `runtime.rs:453-460`). AC3.1 says *port verbatim, because
authoring fresh risks a different argv than the signed bundle attests.* AC4.1 as originally written
said *land `-s workspace-write`* — the **short** spelling. `-s` and `--sandbox` are identical to
codex and **different bytes** to the hash. Writing the short form would mutate an attested
`argv_prefix` as a side effect of an AC about honesty.
*Resolution, ratified at preflight:* **port the long form verbatim and never retype it.** AC4.1's verb
changes from *land* to **assert**. And the honest posture claim is stronger than the one the story
originally reached for: **the FS jail is the ADAPTER's, DECLARED by MAOS in a hashed manifest,
ENFORCED by the adapter, not by MAOS.** Three checkable clauses, all true today, none of them an
apology. Write that; do not write "MAOS has no sandbox posture."

### Fourteen corrections you would otherwise carry forward as facts

**F7 — The worst hardcode is inside the SIGNED capture, and no one had named it.**
`xtask/src/demo_j1.rs:1044` writes the literal
`"maos run <codex topology> --live; CODEX_API_KEY injected host-side (value redacted)"` into
`command_metadata`, which is journaled and sealed into the bundle (`:1064-1085`). The moment
`:1007` widens to accept claude, **a claude run signs a bundle asserting it was codex.** This is the
same defect class the story exists to fix, one layer deeper.

**F8 — `demo_j1.rs:1007` must be WIDENED, never deleted.** Its own comment (`:1005-1006`) says why:
*"The topology is OPERATOR-authored, so nothing else stops the hermetic fixture from reaching the
signing path and earning a Tier-2 label."* Deleting the check lets `worker-cli-fixture` earn
`PROVEN_LIVE_SIGNED`. "De-hardcode" is the wrong verb for this line; it is an anti-overclaim control.

**F9 — The fixture cannot host the vector, and it is not scriptable.**
`FixtureCli::parse_completion` (`worker_cli.rs:253-273`) is **marker-based** — it already returns
`NoCompletionMarker` for a refusal line, because it does not use `final_stdout_message_oracle`.
A fixture-emitted refusal proves nothing about the defect. And
`spirits/worker/src/bin/worker-cli-fixture.rs` is 44 lines with one env knob
(`MAOS_FIXTURE_SHAPE_VERSION`, `:22`); it always prints its three canned lines and always exits 0.
The canned lines are double-pinned (`spirits/worker/tests/fixtures_pin.rs:12`, `:38-58`).
**The vector must run under `ClaudeCli`/`CodexCli`, which needs a fake binary named `claude`/`codex`
(basename dispatch, `worker_cli.rs:363-376`) plus `MAOS_LIVE_AGENT=1`.**

**F10 — The redaction claim a signed run makes is unexecuted for any non-codex worker.**
`demo_j1.rs:1027` and `:1088` read `CODEX_API_KEY` and guard both scans with `if !secret.is_empty()`.
For a claude run that variable is unset, so **both scans are silent no-ops** — while
`CaptureDoc::validate` (`crates/maos-cli/src/subcommands.rs:2321-2322`, `:2364-2370`) accepts the
capture on a **string equality check** that the field reads `"verified"`. Implementing
`ClaudeCli::ambient_auth_path` does not touch this. The shared preflight files the fix under `2c`
while the claim it justifies is signed in `2a` — AC2.5 closes that seam.

**F11 — The `kloc.toml:87` correctness-repair valve is prose, not a mechanism.** The text exists
verbatim at `kloc.toml:85-88` (*"it must never block a correctness or compliance repair"*) inside a
comment block explaining why an **older** policy was replaced, and is restated at `:264`.
`xtask/src/kloc_check.rs` contains no `exempt`/`waiv`/`correctness`/`compliance` token anywhere; the
compare loop (`:229-235`) is unconditional `if *loc > budget`, and `passed = over_budget.is_empty()`.
It is permission to **ask** for a grant plus a `kloc.toml` edit, not a machine carve-out. D15's
zero-headroom ratification leaned on it; this story is the first to test it, and AC1.1 is why the
test does not have to be run.

**F12 — Seven gates are blind to this filename, not five.** Five walk
`_bmad-output/implementation-artifacts/` with a digit-prefix filter:
`check_bare_review_findings.rs:35`, `check_dev_model_tier.rs:103`,
`check_dev_model_used_populated.rs:136`, `check_dev_record_completeness.rs:245-247`,
`check_review_findings_resolved.rs:57-60`. Two more skip by a different mechanism:
`check_epic_close_coherence.rs:215-217` (`head.parse().ok()?`, and its comment names
`j1-crosshost-1` explicitly) and `check_epic_6_bridge.rs:817-825` (hardcoded `"6-2"`/`"6-3"`
prefixes). All five directory-walkers are BLOCKING CI jobs. **Net effect: this story can ship with
no dev record, no `dev_model_used`, no §A6 marker and no review-findings closure, and zero gates will
notice.** A green CI does not mean the review net ran.

**F13 — `ClaudeCli`'s missing clean-home invariant is ASSERTED, not forgotten.**
`worker_cli.rs:489` is `assert_eq!(ClaudeCli.ambient_auth_path(home), None);`, inside a test named
`codex_ambient_auth_json_is_refused_but_fixture_is_immune` whose comment reads *"only codex names the
footgun."* The repo carries a green, executable claim that claude has no credential footgun. AC2.1
**inverts** that line; it does not sit beside it. (`worker_cli.rs:449` similarly asserts
`ClaudeCli.nonsecret_env().is_empty()`.)

**F14 — `home` is the operator's REAL `$HOME`, the control is live on day one, and the tests must
not read it.** `main.rs:1050` is `std::env::var_os("HOME")` — not `MAOS_HOME`, not `XDG_DATA_HOME`.
Nothing sets `HOME` for the child on any live path (`demo_j1.rs:956-967` sets `MAOS_HOME`,
`XDG_DATA_HOME`, `MAOS_LIVE_AGENT` — not `HOME`). On the development box
`~/.claude/.credentials.json` **exists** (508 bytes, mode 0600), so implementing AC2.1 will refuse
every live claude run until the operator moves it. Plan for that; do not discover it at the paid run.
Correspondingly, `smoke_cli_wrapper_8_12.rs` isolates `XDG_DATA_HOME`/`MAOS_HOME` but **never
`HOME`** (`:58`, `:117`, `:188-189`), so any new `maos run`-level test **must** set
`.env("HOME", tmp)` or it passes or fails depending on whose laptop runs it.

**F15 — The frame already carries the field that would make completion checkable, and it is filled
with a tautology.** `main.rs:3905-3906` builds the `task.assign` payload with `success_criteria` =
*"the worker reports completion through its adapter's parse_completion oracle"* — a criterion
asserting that the oracle is the oracle. `TaskAssignPayload.success_criteria` is drained
(`main.rs:9172`) and **never used to judge anything**. Not in scope to build here (it belongs with
2b's task-outcome vocabulary), but do not add a *new* field for the same purpose.

**F16 — No gate anywhere owns the worker/CLI surface.** Zero hits across all 68
`xtask/src/check_*.rs` for `worker_cli`, `cli_wrapper`, `FixtureCli`, `select_worker_cli`.
`check-j1-loopback-delegation` is a **static source-text oracle** over six named files
(`check_j1_loopback_delegation.rs:55-60`) — none of them `worker_cli.rs` — and it runs **no tests**.
It can host a *structural* leg; it structurally cannot judge completion-oracle behaviour. **Extend
it (AC1.7); do not build a new gate.** A fifth `--test` line in a proven blocking job beats a sixth
green box.

**F17 — 24 of 28 `crates/maos-bin/tests/` targets are DEAD in CI.** Only `delegation_leg_1a`,
`topology_delegation_1a` (`discipline.yml:1821`), `jetbrains_acp_server` (`:2576`) and
`erasure_uninstall_13_5b` (`:2943`) are invoked. `smoke_cli_wrapper_8_12` — the harness you are
cloning — is **dead**. There is no workspace-wide `cargo test` anywhere;
`journey-nightly.yml:85` is `cargo build --all-targets` (compiles, never runs). **A new test file
that is not added to `discipline.yml:1821` is a suggestion, not a control.**

**F18 — The basename allowlist is a naming constraint, not a structural block, and the repo's own
test demonstrates the bypass.** `smoke_cli_wrapper_8_12.rs:150-168` writes `$HOME/bin/codex`
containing `#!/bin/sh\nexit 0`, chmods 0755, prepends the dir to `PATH`, and the run resolves and
admits it. `resolve_cli_binary` (`main.rs:704-740`) searches exe-sibling → parent → `$PATH`, first
hit wins, with **no realpath, symlink, hash or signature check**. A shim named `codex` that execs
`bwrap … /usr/bin/codex.real "$@"` resolves, selects `CodexCli`, and satisfies host grant
`attested_image = "codex"` — because that field is `config.command.clone()` captured at
`main.rs:970`, **before** resolution, and matched by plain string equality. **Do not write
"external jails are structurally blocked" into any artifact.**

**F19 — `attested_image` is attested by nothing, and the granted tier never reaches the spawn.**
`attested_image` = the raw manifest string (`main.rs:970`); `signing_key_id` = manifest
`[author].name` (`:971-976`) — both self-declared. A real T3 container substrate exists
(`crates/maos-kernel-core/src/security/sandbox/t3/*`) but its only binary caller is the
`smoke-t3-sandbox-5` smoke mode; `spawn_and_bridge` is a bare `Command::new` with **no cwd, no
`env_clear`, no namespace, no rlimit, no process group and no timeout** (`runtime.rs:449-473`,
`:646-655`). Nothing after the host-grant lookup enforces a sandbox. This is context for AC4's
honesty, not a defect for `2a` to fix.

**F20 — For `j1-crosshost-1b`, not for you: 1a's boundary leg is defective in a sharper way than
recorded.** `check_j1_loopback_delegation.rs:266-293` requires `contains_live(&src,
"frame.from.host_id")`. `contains_live` (`:104-106`) filters comment lines but **not string
literals** — and `crates/maos-a2a-core/src/router.rs:1514` is
`"frame.from.host_id {} does not match TLS-verified peer {}"`, a `format!` literal **inside
`handle_intake_verified`'s own TLS-mismatch NACK**. The "self-asserted" needle is satisfied by the
verified path's error message. Worse, the leg's declared flip trigger is a change in *which* router
entry the composition root calls — in `maos-a2a-tcp`, **not** in `router.rs`, the only file the leg
reads. **The leg cannot observe its own trigger.** Record this in the handoff; 1b owns the repair
(its AC2.2a). `2a` must not restate the false claim anywhere.

### Four more, added by the 2026-08-16 preflight round-table

**F21 — `--bare` is not credential hygiene. It is a REPRODUCIBILITY precondition, and without it
AC3's own title is false for claude.** Filed under AC2.3, it is load-bearing for **AC3**. Verbatim
from `claude 2.1.233 --help`:
> `--bare` — Minimal mode: skip hooks, LSP, plugin sync, attribution, auto-memory, background
> prefetches, keychain reads, and **CLAUDE.md auto-discovery**. Sets `CLAUDE_CODE_SIMPLE=1`. Anthropic
> auth is strictly `ANTHROPIC_API_KEY` or `apiKeyHelper` via `--settings` (OAuth and keychain are
> never read).

`spawn_and_bridge` sets **no `cwd`** (`runtime.rs:449-473`, F19), so a claude worker inherits `maos`'s
working directory — and this repository has a tracked `CLAUDE.md`, with another in the operator's
`~/.claude/`. Without `--bare`, a signed claude run's behaviour is a function of hooks, plugins,
auto-memory and project instruction files that appear in **no manifest, no `argv_prefix_hash`, and
possibly no git**. "Reproducible from the repo" would be a false claim. `--bare` is therefore
required by AC3.2 as well as AC2.3, and it must arrive through AC1.3's `required_argv_flags` so a
manifest that omits it refuses the run rather than producing an unreproducible artifact.

**F22 — There is no host-side credential injection. The doc comment says there is.**
`WorkerCli::nonsecret_env` (`worker_cli.rs:98-99`) reads *"Credentials are injected separately,
host-side — NEVER returned here."* Grep `crates/maos-bin/src` for `CODEX_API_KEY`,
`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`: **zero setters.** The operator exports the key into `maos`'s
own environment and the child inherits it, because the spawn is a bare `Command::new` with **no
`env_clear`**. MAOS neither injects the credential nor knows which one was used — while the signed
capture's `command_metadata` says *"injected host-side"*, which reads as though MAOS chose it.
*Consequence for AC2:* `refuse_ambient_auth` refuses an ambient credential **file** while the same
process hands the child every ambient credential **variable** by inheritance. The path check is a
*filename* control on one door of a room with no walls. **The `env_clear` repair is `runtime.rs` —
kernel-core, FLAG-Winston, out of scope for a ZERO-Δ story.** `2a` owes the honest sentence, at the
impl site, and nothing more: *the credential control is "no ambient file"; the environment channel is
inherited by design and unattested.* Do not soften it, and do not let the `nonsecret_env` doc comment
stand unqualified beside it.

**F23 — SHIP-BLOCKER in AC1.9's own machinery: the falsifier cannot fire.** AC1.9 requires *"delete
the `--test worker_completion_2a` line from the workflow and the gate MUST red."* But
`check_j1_loopback_delegation.rs` governs six source files (`:55-60`) and
**`.github/workflows/discipline.yml` is not among them** — its only mention of the workflow is a doc
comment at `:119`. Delete the enrollment line and the gate stays green, because the gate has no eyes
on the file the vector edits. A proven-red vector for a control that is not watching is a **falsifier
standing in for a falsifier**.
*Repair, and it is a copy not an invention:* add the workflow to the governed files and add an
enrollment leg. Two existing idioms — `check_loom_substrate_drift.rs:66`
(`const WORKFLOW: &str = ".github/workflows/discipline.yml"`) and
`check_epic_6_bridge.rs:619` (`discipline_yml_has_step(...)`). Cost: `lay_green` in
`xtask/tests/j1_crosshost_2a_proven_red.rs` must lay a workflow file into the fixture tree. Pay it.

**F24 — codex's JSONL is line-shaped; claude's `--output-format json` is a single object that may
arrive in pieces.** Every stdout line becomes its own `CliSubprocessOutput` TL row, and the read-back
loop rebuilds `wc_stdout` as `Vec<String>` (`main.rs:1247-1263`). A per-line `serde_json` parse is
correct for codex and **wrong for claude**: if the CLI pretty-prints the result object, no single
line parses, `parse_completion` sees nothing valid, and a genuine success becomes a false negative —
F4's inversion arriving through a door F4 did not check. **Join `stdout` before parsing on the claude
side**; the joined form parses whether the object is one line or twenty. One line of implementation,
and it is cheap to be right about.

### What is already true — verify, do not rebuild

| Claim | State at HEAD |
|---|---|
| `ClaudeCli` is real and live-reachable | TRUE — `worker_cli.rs:339-358`, one construction site `:373`, one production dispatch `select_worker_cli` ← `main.rs:1033` |
| Adapter selection is manifest-declared, fail-closed | TRUE — basename match `:363-376`, `None` ⇒ caller errors `main.rs:1033-1038` |
| Heterogeneity needs no adapter code change | TRUE — two hosts can run different adapters under one protocol today |
| `refuse_ambient_auth` is correctly ordered | TRUE — `main.rs:1049`, **before** the liveness probe (`:1106`) and the spawn (`:1179`); no paid call precedes refusal |
| `MAOS_LIVE_AGENT` is never set in CI | TRUE by absence — `grep -rn MAOS_LIVE_AGENT .github/` = 0 hits across 11 workflows. **Nothing machine-checks it**; the two setters are `demo_j1.rs:964` and `:517` |
| Topology files ARE validated by a blocking CI leg | TRUE — `topology_delegation_1a.rs:196-222` `read_dir`s the whole directory, run at `discipline.yml:1821`, no `continue-on-error` |
| `spirits/*/manifest.toml` is validated by anything | **FALSE — zero readers.** This is where the null-control risk actually lives (AC3.3) |
| Redaction protects a leaked token on the write path | TRUE — `crates/maos-iac/src/adapter/redaction.rs:73-75` matches `sk-ant-`, covering `sk-ant-oat01-` OAuth. The value is scrubbed; what MAOS cannot do is **prove** it |
| `check-kernel-baseline` | GREEN, 24472 = pinned 24472 |
| `kloc-check` at committed HEAD | RED on **four** keys (`maos-bin` +41, `maos-kernel-core` +685, `maos-domain` +50, `_aggregate` +492). Three of the four are D13/D14/D17 — **not yours** |

---

## Story

**As** the founder running the J1 developer-remote loop,
**I want** one host whose worker adapter can be trusted to say whether the work was actually done —
with its credential posture asserted rather than defaulted, its isolation posture ratified rather
than assumed, and its run reproducible from the repo instead of from an operator's laptop —
**so that** the cross-host rungs above it have something real to sign, and a refusal can never again
be journaled as a completion.

---

## Acceptance Criteria (4)

### AC1 — The completion oracle stops certifying refusals

*Scope note, decided at preflight: a working-tree effect oracle (snapshot before/after, require a
non-empty diff) was measured and **deferred**. It needs a `cwd` on `BridgeSpawnSpec` and a
`.current_dir()` in `spawn_and_bridge` — a kernel-core Δ requiring FLAG-Winston — and it inverts on
read-only tasks, which legitimately produce no diff. The per-adapter structured oracle below is
`maos-bin`-only and ZERO kernel-Δ. Do not build a refusal-phrase denylist: prose matching is model-,
locale- and version-dependent, and it is exactly the "claim standing in for a control" shape this
project's retrospectives keep catching.*

1. **`worker_cli` moves under the library, and its in-`src` test module moves out.** Add
   `#[cfg(feature = "network")] pub mod worker_cli;` to `crates/maos-bin/src/lib.rs`, mirroring
   `delegation` (`lib.rs:9-13`) and carrying the same rationale comment `topology` already has
   (`lib.rs:19-22`). Change `main.rs:44-45` to consume it from the library. **Delete**
   `worker_cli.rs:399-658` and relocate every test verbatim into
   `crates/maos-bin/tests/worker_completion_2a.rs`. Land this **first, in its own commit**, and
   record the measured `maos-bin` delta before and after — the whole budget position of this story
   rests on it (F1, F11).
2. **Two oracles, not one, and each derived from its adapter's own machine-readable contract.**
   `CodexCli` and `ClaudeCli` must stop sharing `final_stdout_message_oracle`.
   - `CodexCli` consumes `codex exec --json` JSONL: require a terminal `turn.completed` (**not**
     `turn.failed`), and for a write-class task at least one `item.completed` of type `file_change`.
   - `ClaudeCli` consumes `claude --output-format json`: require `subtype == "success"`,
     `is_error == false`, **and `permission_denials` empty**.
   The story MUST record, in the code and in the Dev Notes, that these are **not equivalent**: codex
   proves effect natively, claude proves only that no tool permission was denied, so a bare model
   refusal with no tool attempt remains undetected on the claude side (F4). Do not paper over the
   asymmetry with shared wording.
3. **The enabling seam: an adapter can demand its argv flags, and the run refuses without them.**
   Add a `WorkerCli` method (e.g. `fn required_argv_flags(&self) -> &[&str]`) and validate it at the
   composition root where `config.argv_prefix` is in scope (`main.rs:1132`/`:1162`), refusing the run
   with a named error when the manifest omits them. Without this, an adapter that assumes JSON while
   the manifest ships prose converts a real success into a false negative (F4). The refusal must be
   covered by a negative test.
4. **A planted refusal reds, end-to-end, hermetically, with no live agent.** Clone
   `smoke_cli_wrapper_8_12.rs:150-208` into the new test file: fake binary on a prepended `PATH`,
   inline `MAOS_HOST_GRANTS` TOML, inline `[cli_wrapper]` manifest, `maos run … --once`. Three
   required changes: set `MAOS_LIVE_AGENT=1`; make the fake print a refusal-shaped final line and
   `exit 0`; and **set `.env("HOME", tmp)`** (F14). It **must go through the topology path**, because
   the standalone path discards the verdict (F3). Assert a non-zero exit **and** that stderr names
   the completion failure — never exit code alone.
5. **The second false-success surface is CLOSED — the fork is decided, do not re-open it.**
   `main.rs:4357-4366` discards the `WorkerCompletion`; consult `is_completed()` there and fail the
   run. *Why closed rather than bounded (ratified at preflight):* the manifest this story ports
   carries its own VERIFY-BEFORE-THE-SIGNED-RUN box, and step 1 is *"Pin the invocation that actually
   WRITES to `$DEMO` standalone first."* The runbook sends the operator down the standalone path to
   sanity-check the worker — so a standalone path that exits 0 on a refusal is **the pre-flight check
   certifying the exact defect this story exists to catch**. A comment explaining why that is
   tolerable would be a comment explaining why the story did not finish. Regression risk measured and
   nil: `smoke_cli_wrapper_8_12.rs:53-141` runs the fixture, whose marker oracle returns `Completed`,
   so exit stays 0. Keep the `None` at `:4363` (1a AC3.5 — no frame to drain); the absent *task* and
   the dropped *verdict* are different things.
6. **`completion_tl_ref` is RENAMED to `last_stdout_tl_ref` — do NOT gate it on `Completed`.** The
   fork in the original draft had a wrong branch and it is now removed. The field's own in-code
   comment (`main.rs:1254-1255`) is **already honest**: *"the last stdout `CliSubprocessOutput`
   frame_id — the Worker-produced TL reference a digest cites."* Only the **name** and the demo's
   conjunction lie. Gating emission on `Completed` would be actively harmful: the run you most need a
   citable TL reference for is the one that **failed**, and gating deletes the evidence pointer at
   precisely the moment someone asks what the worker actually printed. Rename, keep it unconditional,
   and fix `demo_j1.rs:623-628` so the beat stops conjoining it with `completed` as though they were
   one fact. Update the beat's narration to match what the oracle now does. Cost is bounded and known:
   Trap 9's three narrator-fixture spans in `xtask/src/tests/demo_j1_tests.rs` pin the field names —
   land them in the same commit.
7. **A structural gate leg, in the gate that already blocks — and the gate gains eyes on the
   workflow.** Extend `xtask/src/check_j1_loopback_delegation.rs`: add `crates/maos-bin/src/worker_cli.rs`,
   `crates/maos-bin/src/lib.rs` **and `.github/workflows/discipline.yml`** to the governed-files list
   (`:55-60`), add the leg names to `ledger_leg_names()` (`:64-66`), and assert three
   source-structural facts using the mandated composed idiom
   `structural(production_before_tests(src))` (`:137`): (i) neither `ClaudeCli` nor `CodexCli`
   delegates to `final_stdout_message_oracle`; (ii) `lib.rs` carries `pub mod worker_cli` — the
   regression that would silently re-orphan every test from AC1.1; **(iii) the
   `check-j1-loopback-delegation` job carries `--test worker_completion_2a`** — the **enrollment
   leg**, without which AC1.9's falsifier cannot fire at all (F23). The workflow read is a copy, not
   an invention: `check_loom_substrate_drift.rs:66` and `check_epic_6_bridge.rs:619` are the two
   in-repo idioms. Legs stay **root-relative and source-static**: a `cargo`-invoking leg inherits the
   proven-red tempdir and vacuums every vector (1b's F3, still true) — reading a workflow *file* is
   static and safe; *invoking* `cargo` is not.
8. **CI enrollment, on the existing blocking job.** Append to `.github/workflows/discipline.yml:1821`:
   `cargo test -p maos-bin --test worker_completion_2a -- --test-threads=1`. That job is already
   `BindingClass::Blocking`, already in `gate-registry.toml` and already a `needs` of the aggregate
   (`:3177`). Do not create a new job (F16, F17).
9. **The proven-red harness carries its own falsifier.** New file
   `xtask/tests/j1_crosshost_2a_proven_red.rs` (kloc-free), copying
   `j1_crosshost_1a_proven_red.rs` exactly: `lay_green` baseline, the
   `baseline_fixture_tree_is_green` anti-vacuity control (`:130-139`), and `assert_red`'s three-part
   assertion — `!passed`, non-zero exit, **and** the finding text names the regression (`:106-124`).
   Include 1b's vector-#12 shape: **delete the `--test worker_completion_2a` line from the workflow
   and the gate MUST red**, or the enrollment line has no falsifier. **This vector only works because
   AC1.7(iii) gave the gate the workflow to read — at HEAD it would pass vacuously (F23). `lay_green`
   must therefore lay a `.github/workflows/discipline.yml` into the fixture tree containing the
   enrollment line; that is the one new cost this repair carries, and it is the cost of the vector
   being real.** Enroll as its own CI step beside `discipline.yml:1817`.
10. **Do not break the fixture path.** `FixtureCli` keeps its marker oracle unchanged.
    `crates/maos-journey-test/tests/journey_j1.rs:151-158` and
    `crates/maos-bin/tests/drain_once_audit_writer.rs:14-56` both go red transitively if any new
    requirement reaches the fixture — the fixture writes no files (F9).

### AC2 — The credential posture is asserted, not defaulted

1. **`ClaudeCli::ambient_auth_path` is implemented, and `worker_cli.rs:489` is INVERTED.** Return
   `home.join(".claude").join(".credentials.json")` (note the leading dot on the filename, unlike
   codex's `auth.json`). The existing assertion that claude has no footgun is a false claim with a
   green test behind it (F13) — replace that line, do not add a second test beside it. Keep the
   `FixtureCli` immunity control and the positive-before-negative ordering of the template
   (`worker_cli.rs:479-504`): absence permits, then plant, then refuse.
2. **Do NOT widen `ambient_auth_path`'s return type. Keep the single `PathBuf` and state the residual
   at the impl site.** *(Q2 resolved at the 2026-08-16 preflight — the fork is closed, not delegated.)*
   claude's credential surface also includes `~/.claude.json`, the OS keychain, `ANTHROPIC_API_KEY`,
   `ANTHROPIC_AUTH_TOKEN`, `CLAUDE_CODE_OAUTH_TOKEN`, an `apiKeyHelper` command named in settings,
   enterprise managed settings, and Bedrock/Vertex ambient cloud credentials — **none reachable by a
   path check**. Widening to a `Vec<PathBuf>` buys exactly one of those (`~/.claude.json`) while
   *implying* the keychain and the environment were covered. That is this project's signature failure
   mode wearing a type signature. The real closure is structural and lives in AC2.3.
   Record two things at the impl site, in the code:
   - A file-existence check is a **filename** control. `~/.codex/auth.json.bk` exists on the
     development box: the codex invariant was satisfied by **renaming**, not by removing.
   - **The environment channel is inherited by design and unattested (F22).** There is no host-side
     credential injection despite `nonsecret_env`'s doc comment saying so — no setter for
     `CODEX_API_KEY`/`ANTHROPIC_API_KEY`/`OPENAI_API_KEY` exists anywhere in `crates/maos-bin/src`,
     and the spawn has no `env_clear`. Say plainly: *the credential control is "no ambient file"; the
     environment channel is inherited and unattested.* The `env_clear` repair is `runtime.rs` —
     kernel-core, FLAG-Winston, **explicitly out of scope here**. Do not restate the
     "injected host-side" phrasing anywhere in this story's artifacts without this qualification.
3. **`--bare` in the manifest's `argv_prefix` is the structural control — and it is required by AC3
   too, not only here.** On `claude 2.1.233` (help text verbatim), `--bare` makes auth *"strictly
   `ANTHROPIC_API_KEY` or `apiKeyHelper` via `--settings` (OAuth and keychain are never read)"* — it
   **deletes** two of the eight surfaces in AC2.2 rather than mitigating them, and `--settings` is
   itself argv-hashed, so MAOS owns the helper channel. In `argv_prefix` it becomes part of
   `argv_prefix_hash` (`main.rs:1132`) and is re-derived and asserted pre-spawn
   (`runtime.rs:453-460`) — a cap-token-bound guarantee instead of a TOCTOU stat. Route it through
   AC1.3's `required_argv_flags` so a manifest that omits it refuses the run.
   **It is also a reproducibility precondition (F21):** the same flag skips hooks, LSP, plugin sync,
   auto-memory and **CLAUDE.md auto-discovery**, and the child inherits `maos`'s cwd in a repository
   that has a tracked `CLAUDE.md`. Without `--bare`, AC3's "reproducible from the repo" is false for
   claude. Cite F21 at the `required_argv_flags` impl so the next reader cannot delete the flag as
   mere credential hygiene.
4. **Close the two fail-open branches on the live path.** `main.rs:1049-1050`: an unset `HOME`
   currently skips the clean-home check entirely — on the live path it must refuse, not permit.
   `main.rs:834-843`: an unreadable or unparseable `MAOS_HOST_GRANTS` currently `eprintln!`s and
   continues with built-in grants; state the disposition deliberately (it is safe today because real
   CLIs then fail closed, but it is a silent downgrade of an operator-intent file).
5. **A signed run must not certify a redaction check that did not execute — do BOTH halves; they are
   independent failures.** *(Fork closed at preflight: the original "either/or" let a dev fix one and
   leave the other live.)* `demo_j1.rs:1027` and `:1088` scan for `CODEX_API_KEY` only and are no-ops
   when it is unset, while `CaptureDoc::validate` accepts the literal `"verified"` (F10).
   - **(a) Provider-aware.** Derive the secret name from the selected adapter, so a claude run scans
     for the variable a claude run actually uses. Fixing only this still fails open.
   - **(b) An unexecuted scan must be structurally unable to emit `"verified"`.** The
     `if !secret.is_empty()` guard (`:1028`, `:1090`) means "operator forgot to export the variable"
     and "scan ran and passed" produce **byte-identical signed evidence**. Refuse the signing when no
     scan executed. Do **not** introduce a `"not-scanned"` value — that just teaches
     `CaptureDoc::validate` a new string to accept, which is how the field became decorative in the
     first place.
   Do not defer this to `2c`: `2c` owns the *read-path* TL scan, and the claim being signed is `2a`'s.
6. **`demo_j1.rs:907` calls the adapter instead of re-deriving the path.** It hardcodes
   `base.join(".codex/auth.json")`, duplicating `CodexCli::ambient_auth_path` — so the demo's own
   preflight is blind to claude for the same reason the production path was.
7. **The negative test isolates `HOME`.** Mandatory (F14): plant `tmp/.claude/.credentials.json`,
   set `.env("HOME", tmp)`, assert refusal and that stderr names the planted path. The paired
   proven-red is: delete `main.rs:1049-1063` and this test must red. Today, deleting it reds exactly
   one unit test that calls the function directly and never exercises the production wiring.

### AC3 — The run is reproducible from the repo, and nothing signs a claim about a run it did not make

1. **PORT the topology from `60d4080a`, then repair it — do not author it fresh.**
   `git show 60d4080a:spirits/topologies/j1-founder-loop-codex.toml`. Strip the four
   `priority_weight` keys (1a's strict parser rejects them and a blocking CI leg iterates the whole
   directory) and **add `host = "developer-remote-host"`**, without which the delegation is skipped
   and codex is spawned with no task (F5). Same for
   `git show 60d4080a:spirits/worker/manifest-codex.toml`; its header comment still references
   `MAOS_WORKER_TASK`, which 1a deleted.
2. **Author `spirits/worker/manifest-claude.toml` with a ratified argv posture.** `-p` alone cannot
   execute a coding task: `claude 2.1.233` carries the literal deny strings
   *"Current permission mode (${mode}) requires approval for this ${e} command"* and *"Claude
   requested permissions to use ${e}, but you haven't granted it yet"*, and with `-p` there is no TTY
   to approve — the tool call is denied, the model explains itself in prose, and the process exits 0.
   **That is the exact mechanism of the ship-blocker.** The manifest must carry an explicit
   permission posture (`--permission-mode` or `--allowedTools`), the `--output-format json` AC1.2
   depends on, and the `--bare` of AC2.3 — **which this AC depends on in its own right, not merely by
   reference to AC2.** Without `--bare`, a claude worker auto-discovers hooks, plugins, auto-memory
   and this repository's tracked `CLAUDE.md` (the child inherits `maos`'s cwd), none of which appear
   in the manifest or the argv hash, and "reproducible from the repo" becomes false (F21).
   `[author] name` must match a host grant or admission fails. All of `spirits/` is kloc-free — these
   files cost nothing.
3. **Give the worker manifests a reader, or committing them is decoration.** Nothing in the repo
   parses `spirits/*/manifest.toml`. Add a sibling to `topology_delegation_1a.rs:196-222` that
   `read_dir`s them, drives each through `CliWrapperConfig::from_toml_str`, and asserts
   `select_worker_cli(command)` resolves to a supported adapter — which simultaneously proves
   `manifest-claude.toml` is admissible. **It must be added to `discipline.yml:1821` or it is dead**
   (F17).
4. **`demo_j1.rs:1007` is widened to an allowlist, never removed** (F8). `{codex, claude}`, with
   `worker-cli-fixture` still refused. Widen the `tier2-live-agent-signed` owner string at `:795` in
   the same change, and the `--live-codex`/`--codex-topology` flag names and
   `MAOS_DEMO_J1_CODEX_TOPOLOGY` (`:889`) if you rename them — but a rename is optional; the
   allowlist is not.
5. **`demo_j1.rs:1044` stops asserting a run it did not observe.** The `command_metadata` literal is
   sealed into the signed bundle (F7). Derive it from the adapter and manifest actually used, or the
   first heterogeneous run signs a bundle saying it was codex.
6. **Nothing in this story's artifacts restates a disproved claim.** Specifically: not "the manifests
   do not exist" (F5), not "claude has no sandbox counterpart" (F6), not "external jails are
   structurally blocked" (F18), not the 1a boundary-leg claim (F20), and not "the T6 bundle is one
   signed run" — it swept eight `boot_nonce` values and renders 64% of entries `unknown`.

### AC4 — Ratify the isolation posture as a machine-checked claim, and close honestly

1. **ASSERT the per-adapter sandbox posture — do not "land" it, and do not retype the codex flag.**
   *(Corrected at preflight; the original wording contradicted AC3.1 — see F6a.)* The ported
   `manifest-codex.toml@60d4080a:25` **already carries** `argv_prefix = ["exec", "--sandbox",
   "workspace-write"]`, and that exact long-form spelling is what the **signed** T6 capture attests in
   `command_metadata`. `-s` and `--sandbox` are the same flag to codex and **different bytes** to
   `argv_prefix_hash`, so retyping it silently desynchronizes the committed manifest from the signed
   bundle. **Port the long form verbatim.** Author the equivalent settings posture in
   `manifest-claude.toml` (`--settings` accepts an inline JSON string, so claude's posture is
   argv-hashed exactly like codex's flag — the two adapters are symmetric on this axis). Then add the
   assertion — in AC3.3's manifest reader — that each real-adapter manifest carries a non-empty
   isolation declaration. An argv posture nothing asserts is the same doc comment in a different file.
   **The claim you are ratifying, in these words:** *the FS jail is the adapter's, declared by MAOS in
   a hashed manifest, enforced by the adapter, not by MAOS.* Three checkable clauses, all true at
   HEAD. Do not write "MAOS has no sandbox posture" — the T6 signed run had a real one.
2. **Use the egress precedent to state whatever gap remains.** `CaptureDoc::validate`
   (`crates/maos-cli/src/subcommands.rs:2285-2360`) requires `egress` to equal **exactly**
   `"declared-not-enforced"`, requires a non-empty `egress_followup`, and has a negative test
   refusing the **overclaim** direction (`:3935`
   `capture_validation_refuses_egress_enforced_overclaim`). Mirror that shape for the FS-jail claim:
   a stated posture that a capture cannot overclaim, with a follow-up field and a negative test — not
   a sentence in a story file.
3. **Record the measured asymmetry accurately, in the direction the evidence points.** claude's
   counterpart is a settings document; codex's is a flag; both are argv-declarable and both are
   TOCTOU-hashed. claude additionally offers enforced egress allowlisting, a fail-closed startup
   gate, and `--bare` — controls codex does not have. If any residual weakens the heterogeneous
   claim, name the residual; do not name the adapter.
4. **Budget, measured at HEAD, attributed honestly.** Re-measure `maos-bin` after AC1.1's
   relocation and record before/after in the Dev Notes. Take **no** grant unless still over, and then
   only with the measurement attached (`kloc.toml:60-65`). Do **not** absorb the four standing reds:
   `maos-kernel-core` +685 (D13), `maos-domain` +50 (D14), `_aggregate` +492 (D17), and — until the
   D15 grant is committed — `maos-bin` +41. `kloc-check` will exit 1 at close through no fault of
   this story; attribute it by key.
5. **Close the evidence lane without overclaiming it.** `demo-j1` has **zero CI invocation**;
   `ledger_gates()` derives from the four Postgres substrate gates, so the J1 lane sits structurally
   outside evidence-ledger enforcement; `Beat::absent` sets `executed: false` and an unlanded beat
   never fails a run. If a beat moves, move it in code via the beat list, and say in the Dev Notes
   that the ledger did not enforce it.
6. **Disclose the seven blind gates in the Dev Agent Record** (F12) and populate the model/§A6
   fields anyway. The §A6 net is non-degradable here: this story decides what the word "completed"
   means in a signed artifact, and 1a's pass omitted Test-Infra — the layer that catches exactly this
   class of defect. **Disclosure is now the interim state, not the disposition:** the hole stayed open
   across `1a`, `1b`, `j1-demo-one-command-scene` and this story, each disclosing it in prose and none
   closing it, so it is filed as **D19** (`epic-14-preflight-decisions.md`) with a named vehicle
   (**14-0** decomposes) and a mechanical deadline (before the next `j1-*` story leaves
   `ready-for-dev`). `2a` still discloses; `2a` does **not** fix five gates. Cite D19 in the Dev Agent
   Record so the disclosure points at an owner instead of at itself.

---

## Traps

1. **Do not copy `check_vetting_attestation::invoke_leg`.** It builds `Command::new("cargo")` with no
   `current_dir` and would inherit the proven-red tempdir (no `Cargo.toml`), turning every planted
   vector into a vacuous pass while CI reports green. Still true at HEAD.
2. **Do not put the vector under `FixtureCli`.** Its marker oracle is immune (F9).
3. **Do not run the vector through the standalone path.** The verdict is discarded there (F3).
4. **Do not let a test read the real `$HOME`.** Machine-dependent pass/fail (F14).
5. **Do not add a `std::env::var*` read under `crates/maos-bin/src` without registering it** —
   `check-env-contract` (`xtask/src/check_env_contract.rs:119-160`) walks that tree and fails.
   Reads under `spirits/` are outside its walk (which is why `MAOS_FIXTURE_SHAPE_VERSION` is
   unregistered).
6. **`cargo test -p maos-bin` is RED under default parallel flags** (D16, `MAOS_HOME` is
   process-global). Measured: `cross_wall_recall_live_path_uses_verified_state_and_home_team` fails
   parallel, passes with `--test-threads=1`; the whole suite is 204 passed / 0 failed at
   `--test-threads=1` in ~81s. Run scoped and single-threaded. D16 belongs to 14-0.
7. **The kernel pin is 24472.** Any document or memory saying 23679 is stale by one re-pin.
8. **`abi-diff` green is not evidence for this lane** — it scopes to `crates/maos-spirit-abi` only
   (FLAG-E4). Report "no ABI change was made" as a fact about your diff, never as a gate result.
9. **Renaming `worker_completion` JSON fields breaks the demo narrator.**
   `xtask/src/tests/demo_j1_tests.rs:140-144`, `:184-200`, `:229-260` synthesize that event and
   constrain its field names and shape; `:250-260` asserts that two `worker_completion` rows is a
   failure.
10. **`label_is_nonsecret_and_never_leaks_message` (`worker_cli.rs:647-657`) will red on a stricter
    oracle, and its intent is orthogonal.** It asserts the *label* carries no message text. Repair it
    by changing the fixture input, not by weakening the oracle.
11. **`only_the_fixture_speaks_the_bridge_handshake` (`:508-527`) reds if you touch
    `probe_strategy`.** The structured-output work must not migrate into the probe.
12. **The bridge never parses NDJSON.** `runtime.rs:388-390` handles `NdjsonOverStdio` and `Raw`
    identically despite the manifest declaring `ndjson_over_stdio`. The adapter must parse.
13. **The evidence chain is schemaless at both ends.** The `CliSubprocessOutput` payload is an ad-hoc
    `serde_json::json!` at `runtime.rs:595-602`, re-parsed with `.get("line")`/`.get("stream")` string
    lookups at `main.rs:1247-1263`. A field rename in either place yields empty stdout, which today
    reports `NoCompletionMarker` — fail-closed by luck, not by design. Do not make it fail-open.
14. **Redaction markers are JSON-safe** (`<REDACTED:type=…,len=…,hash=…>`, no quotes or backslashes)
    and a UUID's longest hex run is 12 against `TOKEN_HEX_MIN_LEN = 32` — so a claude JSON result line
    survives redaction intact. Verify this rather than assume it if you change the payload shape.
15. **`journal_completion` hardcodes the string.** `delegation.rs:243` writes
    `"completed".to_string()`; `delegation_leg_1a.rs:234` pins it. The oracle gates *whether* the
    frame is emitted, never *what it says*. Task-outcome vocabulary is `2b`'s (P5) — do not start it
    here.
16. **The demo's live take is entirely untested.** `xtask/src/tests/demo_j1_tests.rs` has **zero**
    codex references, consistent with `demo-j1` having zero CI invocation. Changes to
    `live_codex_take` are unguarded.
17. **`spirits/worker/tests/fixtures_pin.rs`** double-pins the fixture's canned lines by SHA-256. You
    should not need to touch them; if you do, the pin must move deliberately.
18. **Five story-file discipline gates skip this filename and two more skip it by another mechanism**
    (F12). Dev record, model tier and review-findings closure are on the honor system.
19. **`cargo test -p worker --locked` is a live blocking CI job** (`discipline.yml:815`) covering
    `spirits/worker/tests/`. It is a kloc-free, already-enrolled surface — use it if fixture work is
    needed, rather than inventing a job.
20. **Do not fix `success_criteria` here** (F15). It is circular and dead, but the typed task-outcome
    work belongs to `2b`. Note it; leave it.
21. **`maos-a2a-core` is at 4654/4654 and frozen by D10.** Nothing in this story should touch it. If
    you find yourself there, you have left `2a`'s scope.
22. **Never retype an `argv_prefix` you are porting.** It is TOCTOU-hashed into the cap-token, and the
    T6 capture attests the exact string. `-s` vs `--sandbox` is a semantic no-op to codex and a hash
    change to MAOS (F6a). `git show 60d4080a:<path> > <path>` — do not hand-transcribe, and diff the
    result against the `git show` output before committing.
23. **Parse claude's JSON from a JOINED stdout, not line by line** (F24). codex's `--json` is JSONL,
    one object per TL row; claude's `--output-format json` is one object that may be pretty-printed
    across many rows. A per-line parse turns a genuine claude success into a false negative.
24. **Do not "fix" `nonsecret_env`'s doc comment by making it true.** It claims host-side credential
    injection that does not exist (F22). Implementing injection means touching the spawn
    (`env_clear`, `runtime.rs`) — kernel-core, FLAG-Winston, out of scope. Qualify the comment; do not
    satisfy it.
25. **A `redaction_result` value other than `"verified"` is not a fix.** `CaptureDoc::validate` accepts
    on string equality; adding `"not-scanned"` teaches it a second accepted string and re-creates the
    defect one identifier over (AC2.5b). Refuse the signing instead.

---

## Tasks

- [x] **T1 (AC1.1)** — Move `worker_cli` under the library; relocate its `#[cfg(test)] mod tests`
      verbatim to `crates/maos-bin/tests/worker_completion_2a.rs`. **Own commit.** Record
      `kloc-check` `maos-bin` before and after.
- [x] **T2 (AC1.2, AC1.3)** — Add `required_argv_flags` to `WorkerCli` + composition-root validation
      with a named refusal; implement the codex JSONL oracle and the claude result-object oracle as
      separate impls; repair the six in-file tests that constrain the change (see Dev Notes table).
- [x] **T3 (AC1.4)** — Clone the fake-binary harness into the new test file; `MAOS_LIVE_AGENT=1`,
      `HOME`-isolated, topology path, refusal transcript, exit 0 → run must fail with a named stderr.
- [x] **T4 (AC1.5, AC1.6)** — Close or bound the standalone discard; fix `completion_tl_ref`'s
      meaning and `demo_j1.rs:623-628`'s conjunction and narration.
- [x] **T5 (AC2.1, AC2.2, AC2.7)** — Implement `ClaudeCli::ambient_auth_path`; invert
      `worker_cli.rs:489`; state the uncoverable surface at the impl site; add the `HOME`-isolated
      integration negative.
- [x] **T6 (AC2.3, AC2.4)** — `--bare` through `required_argv_flags`; close the unset-`HOME`
      fail-open; state the `MAOS_HOST_GRANTS` degrade-to-builtin disposition.
- [x] **T7 (AC2.5, AC2.6, AC3.4, AC3.5)** — Make the redaction scan provider-aware (or make
      `redaction_result` unable to read `"verified"` unscanned); route `demo_j1.rs:907` through the
      adapter; widen `:1007` to an allowlist; derive `:1044`.
- [x] **T8 (AC3.1, AC3.2)** — Port both files from `60d4080a`; strip `priority_weight`; add `host`;
      author `manifest-claude.toml` with the ratified argv posture.
- [x] **T9 (AC3.3, AC4.1)** — Manifest reader test asserting parse + adapter resolution + a non-empty
      isolation declaration; enroll at `discipline.yml:1821`.
- [x] **T10 (AC1.7, AC1.8, AC1.9)** — Structural gate legs **including the workflow-enrollment leg**
      (add `.github/workflows/discipline.yml` to the gate's governed files — at HEAD the gate cannot
      see it, so AC1.9's falsifier is vacuous without this, F23); CI enrollment; proven-red file with
      the baseline-green control and the enrollment-line falsifier, whose `lay_green` must lay a
      workflow into the fixture tree.
- [x] **T11 (AC4.2, AC4.3)** — Capture-level FS-jail posture mirroring the egress precedent, with the
      overclaim-refusing negative test.
- [x] **T12 (AC4.4, AC4.5, AC4.6)** — Re-measure and attribute budget; move any beat in code and say
      the ledger did not enforce it; populate the Dev Agent Record and disclose the seven blind gates.

### Review Findings

_(§A6 net executed 2026-08-16: Blind Hunter + Edge Case Hunter + Acceptance Auditor + Test-Infra
Auditor + runtime execution, under `zai/glm-5.2` — a different model than the dev pass, as the Dev
Agent Record requires. 25 raw findings → 18 unified after dedup → 11 patch, 3 defer, 4 dismissed.)_

- [x] [Review][Patch] The enrolled CI step cannot pass on a fresh runner — `worker_completion_2a`'s fixture-dependent test resolves `worker-cli-fixture` at runtime (exe-sibling/`$PATH`), but no step of the `check-j1-loopback-delegation` job builds package `worker`'s bin. REPRODUCED: on a clean target dir the suite is 39/40 with `command 'worker-cli-fixture' not found`; after `cargo build -p worker --bins` it is 40/40. Fix: build the fixture bin in the job before the test step. [.github/workflows/discipline.yml (check-j1-loopback-delegation job); crates/maos-bin/tests/worker_completion_2a.rs:1019] — HIGH (blocking job red on push)
- [x] [Review][Patch] `refuse_missing_argv_flags` validates isolated tokens, not flag/value bindings, and the production seam refuses no bypass flag — `["--output-format","text","--session-id","json"]` and `--sandbox read-only` + a stray `workspace-write` token all pass; `bypassPermissions`/`dontAsk`/`--dangerously-bypass-approvals-and-sandbox`/duplicate `--sandbox` are admitted on the live path (the `BYPASS_FLAGS` refusal exists only for COMMITTED manifests, worker_manifests_2a.rs:172-178), and the claude bypass flags also hide the `permission_denials` signal the oracle depends on. Fix: pair-binding for (flag,value) requirements + a forbidden-token refusal at the same seam, lifted from the test's `BYPASS_FLAGS`. [crates/maos-bin/src/worker_cli.rs:285-304; crates/maos-bin/src/main.rs:1172] — HIGH
- [x] [Review][Patch] `fs_jail: "adapter-enforced-maos-declared"` is sealed UNCONDITIONALLY, and claude's isolation is never required on the signing path — `ClaudeCli::required_argv_flags` omits `--settings`, the demo preflight calls only `refuse_missing_argv_flags`, and the committed-manifest check asserts the `--settings` TOKEN without inspecting its payload (`{}` passes). An operator-supplied claude topology with no settings doc signs an AC4 claim that did not hold. Fix: refuse to seal the positive posture unless the resolved argv actually declares the isolation (and the settings payload parses as JSON naming a sandbox); the e2e fake-claude fixtures gain `--settings`. [xtask/src/demo_j1.rs:1235-1249; crates/maos-bin/src/worker_cli.rs:638-640; crates/maos-bin/tests/worker_manifests_2a.rs:156-216] — HIGH
- [x] [Review][Patch] A hostless topology `[cli_wrapper]` entry still exits 0 on an oracle refusal — `delegated_task = None` when `entry.host` is absent (main.rs:3948-3991) and the `!is_completed()` refusal at :4014-4036 is gated on `delegated_task.is_some()`. Same defect class AC1.5 closed for the standalone path; the runbook's pre-flight shape. Fix: consult `is_completed()` regardless of delegation (TaskComplete/journal_completion stays delegated-only). [crates/maos-bin/src/main.rs:4014-4036] — HIGH
- [x] [Review][Patch] Multi-worker topologies sign for the FIRST `[cli_wrapper]` only — `resolve_topology_worker` returns the first wrapper (demo_j1.rs:920-985) and the post-run parse reads the first `worker_completion` event (:1150-1153), while production runs EVERY wrapper (main.rs:3989+): later workers run with their credential variable unscanned and their isolation unasserted while the capture describes wrapper #1. Fix: the signing preflight refuses a topology declaring more than one `[cli_wrapper]` member (the demo attests ONE worker by construction). [xtask/src/demo_j1.rs:920-985,1150-1153] — HIGH
- [x] [Review][Patch] The signed-bundle secret scan runs AFTER `sealed-export` wrote the bundle — a detected secret leaves a signed artifact on disk (persisting under `--keep-home`; the ephemeral default is cleaned only by the `EphemeralHome` Drop guard at exit). Fix: delete the bundle in the refusal branch before returning. [xtask/src/demo_j1.rs:1287-1293] — MEDIUM
- [x] [Review][Patch] Empty-string `HOME` passes the unset-`HOME` refusal — `var_os("HOME") == Some("")` reaches `Path::new("").join(…)`, scanning a RELATIVE `.codex/auth.json`/`.claude/.credentials.json` while the child may apply its own empty-HOME fallback, so the clean-home claim is unverifiable yet satisfied. Fix: treat empty `HOME` as unset at both refusal sites. [crates/maos-bin/src/main.rs:1075; xtask/src/demo_j1.rs:1041-1046] — MEDIUM
- [x] [Review][Patch] The two new gate legs are weaker than their ACs — the per-adapter leg checks substring PRESENCE of the oracle names (the proven-red green fixture passes with empty `fn codex_jsonl_oracle()` stubs, so an unwired/false-completion impl keeps the leg green), and the enrollment leg accepts `--test <target>` ANYWHERE in the workflow, not scoped to the blocking `check-j1-loopback-delegation` job. Fix: needle on the call forms (`codex_jsonl_oracle(stdout, exit)` etc.) and scope the enrollment search to the job block. [xtask/src/check_j1_loopback_delegation.rs:342-465] — MEDIUM
- [x] [Review][Patch] Claude's explicit permission posture is not machine-checked — AC3.2 requires `--permission-mode` or `--allowedTools`; nothing asserts presence (deleting it from `manifest-claude.toml` keeps every test green). Fix: the manifest reader asserts a permission-posture token for claude manifests. [crates/maos-bin/tests/worker_manifests_2a.rs:79-136] — LOW
- [x] [Review][Patch] The manifest reader does not descend subdirectories — `spirits/<name>/profiles/manifest-*.toml` escapes every manifest assertion while the helper claims to return every `[cli_wrapper]` manifest under `spirits/`. Fix: recursive walk. [crates/maos-bin/tests/worker_manifests_2a.rs:33-72] — LOW
- [x] [Review][Patch] The new signing controls have ZERO regression coverage — `resolve_topology_worker`, `SIGNABLE_WORKER_CLIS`, `credential_env_var` selection and the derived `command_metadata` are referenced by no test (demo_j1_tests.rs exercises `evaluate_beats` only), so reverting any of them leaves the enrolled suite green (Trap 16 acknowledged this for `live_codex_take`; the 2a controls deepened it). Fix: unit tests over the pure functions in xtask/src/tests/demo_j1_tests.rs. [xtask/src/demo_j1.rs:920-985,1029-1085,1227-1238; xtask/src/tests/demo_j1_tests.rs] — MEDIUM
- [x] [Review][Defer] Clean-home TOCTOU window between `refuse_ambient_auth` and the spawn (10s liveness probe in between) [crates/maos-bin/src/main.rs:1084→1221] — deferred: real closure is spawn-time enforcement on the kernel lane (F22 `env_clear` adjacency, FLAG-Winston)
- [x] [Review][Defer] The sealed capture's completion claim cites `last_stdout_tl_ref`, documented in-code as NOT a completion witness, and the oracle verdict itself is println-only [crates/maos-bin/src/main.rs:1324-1337; xtask/src/demo_j1.rs:1241] — deferred, owner j1-crosshost-2b (typed task-outcome vocabulary, Trap 15)
- [x] [Review][Defer] `record-capture` accepts caller-asserted control strings (`fs_jail`, `redaction_result`, free-form `audit_refs`) with no run evidence [crates/maos-cli/src/subcommands.rs CaptureDoc::validate] — deferred, pre-existing `egress`-precedent shape inherited by the new fields

Dismissed (4, with grounds): claude bare-decline completion reaching the signing gate — AC1.2/F4
documented residual, asserted as an executable asymmetry claim, and inside the ratified claim
boundary ("claude is signable, never claude works"); PATH/shim basename trust — F18/F19 disclosed,
the capture validator's own doc carries the honest sentence; AC1.1's own-commit deviation —
AGENTS.md "commit only when asked" policy, measurement recorded at the relocation boundary instead;
the Dev Record's "all 4 ACs satisfied" close-out line — superseded by these findings (6 ACs move to
Partially Met pending patches: AC1.3, AC1.7, AC2.5, AC3.4, AC3.5, AC4.1-4.3).

---

## Dev Notes

### Measured at HEAD (`5a921c0c`) — inherit no number from an older story

| Instrument | Ceiling / pin | Measured | Verdict |
|---|---|---|---|
| kloc `maos-bin` | 16219 (working tree, D15) / 16178 (committed) | **16219** | 0 headroom; **−41 vs committed HEAD** |
| kloc `xtask` | 38609 | **38075** | GREEN, **+534** |
| kloc `maos-a2a-tcp` | 1500 | 1085 | GREEN, +415 (not this story's) |
| kloc `maos-a2a-core` | 4654 | 4654 | GREEN, **zero headroom, D10-frozen** |
| kloc `maos-domain` | 8644 | 8694 | RED +50 — D14, not yours |
| kloc `maos-kernel-core` | 18248 | 18933 | RED +685 — D13, not yours |
| kloc `_aggregate_hardfail` | 147057 | 147549 | RED +492 — D17, standing, not yours |
| `check-kernel-baseline` | 24472 | **24472** | GREEN |
| Recoverable budget | — | **+204 exactly** to `maos-bin` via AC1.1 | `tokei` on `worker_cli.rs:399-658` = 204 CODE / 31 comments / 25 blanks; the story funds itself |
| Zero-cost surfaces | — | `crates/*/tests/`, `xtask/tests/`, `xtask/src/tests/`, **all of `spirits/`** | `kloc_check.rs:167-190` |

### The production chain, hop by hop — read this before touching the oracle

| # | Hop | file:line |
|---|---|---|
| 1 | `select_worker_cli(&resolved)` — sole production dispatch | `main.rs:1033` |
| 2 | `live_agent_gate` | `main.rs:1042-1044` |
| 3 | `refuse_ambient_auth` (**only when `live_agent`**; `HOME`-based) | `main.rs:1049-1063` |
| 4 | probe: fixture ⇒ bridge handshake; codex/claude ⇒ `--version`, 10s | `main.rs:1090-1117` |
| 5 | `argv_prefix_hash` + cap-token issue (**failure is non-fatal**) | `main.rs:1132-1156` |
| 6 | TOCTOU re-derive + assert of the hash | `runtime.rs:453-460` |
| 7 | `Command::new` — no cwd, no `env_clear`, no timeout, no namespace | `runtime.rs:462-473` |
| 8 | TL read-back of `CliSubprocessOutput` (kind 21) rows | `main.rs:1237-1269` |
| 9 | `completion_tl_ref` assigned on **every** stdout row | `main.rs:1258` |
| 10 | `parse_completion` | `main.rs:1270` |
| 11 | `worker_completion` JSON `println!` (**no TL row for the verdict**) | `main.rs:1271-1283` |
| 12 | topology: `if !is_completed() { Err }` → `journal_completion` → `TaskComplete` | `main.rs:3960-3968` |
| 13 | standalone: verdict **discarded** | `main.rs:4357-4366` |
| 14 | signing gate: `if !completed { "nothing to sign" }` | `demo_j1.rs:998-1001` |

### Tests that constrain the change

| file:line | Test | Exposure |
|---|---|---|
| `worker_cli.rs:600-611` | `codex_completes_on_final_stdout_message` | **Breaks** — asserts prose ⇒ `Completed` |
| `worker_cli.rs:635-645` | `claude_shares_the_final_stdout_oracle` | **Breaks, and its name becomes a lie** — rename |
| `worker_cli.rs:647-657` | `label_is_nonsecret_and_never_leaks_message` | **Breaks**; intent is orthogonal — fix the input (Trap 10) |
| `worker_cli.rs:430-435` | `argv_appends_task_as_trailing_arg` | Breaks if flags move into `argv()` |
| `worker_cli.rs:437-455` | `only_codex_declares_noninteractive_env_and_no_secret_leaks` | Breaks if claude gains env |
| `worker_cli.rs:479-504` | `codex_ambient_auth_json_is_refused_but_fixture_is_immune` | **`:489` must be INVERTED** (AC2.1) |
| `worker_cli.rs:508-527` | `only_the_fixture_speaks_the_bridge_handshake` | Breaks if `probe_strategy` moves (Trap 11) |
| `worker_cli.rs:613-633` | `codex_exit0_but_empty_stdout…`, `codex_nonzero_exit…` | **Survive — load-bearing negatives, keep passing** |
| `worker_cli.rs:555-598` | the three `fixture_*` oracle tests | **Survive** — fixture oracle unchanged |
| `journey_j1.rs:151-181` | `completed == true`, `delegation_completed.result`, drain ≥1 | Reds if any new requirement reaches the fixture |
| `drain_once_audit_writer.rs:14-56` | topology `--once` exits 0 | Same transitive exposure |
| `smoke_cli_wrapper_8_12.rs:150-208` | `ci_local_split_refuses_a_granted_real_agent…` | **Clone this — it is the harness** |
| `smoke_cli_wrapper_8_12.rs:53-141` | standalone + founder-loop smokes | On the critical path once AC1.5 lands |
| `demo_j1_tests.rs:140-260` | narrator fixtures | Constrain `worker_completion` field names (Trap 9) |
| `delegation_leg_1a.rs:234` | `p.result == "completed"` | Pins the hardcoded string (Trap 15) |

### Where the code goes

| Concern | File | Anchor |
|---|---|---|
| **Enabling move** | `crates/maos-bin/src/lib.rs` | mirror `delegation` `:9-13`, rationale `:19-22` |
| **Relocated tests (free)** | `crates/maos-bin/tests/worker_completion_2a.rs` | NEW — must be `--test`-enrolled |
| The oracle | `crates/maos-bin/src/worker_cli.rs` | `final_stdout_message_oracle` `:211-228`; impls `:312-319`, `:350-357` |
| Trait surface | `crates/maos-bin/src/worker_cli.rs` | `WorkerCli` `:86-135` |
| argv validation seam | `crates/maos-bin/src/main.rs` | `config.argv_prefix` in scope at `:1132`, `:1162` |
| Ambient auth | `crates/maos-bin/src/worker_cli.rs` | `CodexCli::ambient_auth_path` `:321-329`; template test `:479-504` |
| Harness to clone | `crates/maos-bin/tests/smoke_cli_wrapper_8_12.rs` | `:20-52` isolation, `:150-208` fake-binary vector |
| Newest isolation idiom | `crates/maos-bin/tests/drain_once_audit_writer.rs` | `:15-24` (`TempDir` + `XDG_DATA_HOME` + `MAOS_HOME`) |
| Manifest reader template | `crates/maos-bin/tests/topology_delegation_1a.rs` | `:196-222` `read_dir` + strict parse + `checked >= 3` |
| Manifests / topologies (free) | `spirits/worker/`, `spirits/topologies/` | port from `60d4080a` |
| Gate to EXTEND | `xtask/src/check_j1_loopback_delegation.rs` | files `:55-60`, legs `:64-66`, idiom `:137`, aggregate `:306` |
| Proven-red template | `xtask/tests/j1_crosshost_1a_proven_red.rs` | `lay_green` `:72`, `assert_red` `:106-124`, baseline `:130-139` |
| Gap-statement precedent | `crates/maos-cli/src/subcommands.rs` | `CaptureDoc::validate` `:2285-2360`, negative `:3935` |
| Demo coupling | `xtask/src/demo_j1.rs` | `:362`, `:623-628`, `:795`, `:907`, `:1007-1012`, `:1027`, `:1044`, `:1088` |
| CI job (add two lines) | `.github/workflows/discipline.yml` | job `:1804`, gate `:1815`, proven-red `:1817`, legs `:1821` |

### Installed adapter surfaces (measured 2026-08-16, do not re-derive from memory)

- `claude 2.1.233` — `--output-format text|json|stream-json` (`--print` only), `--json-schema`,
  `--permission-mode acceptEdits|auto|bypassPermissions|manual|dontAsk|plan`, `--allowedTools` /
  `--disallowedTools` / `--tools` (`""` disables all), `--add-dir`, `--settings <file-or-json>`,
  `--setting-sources`, `--strict-mcp-config`, `--safe-mode`, `--bare`, `--max-budget-usd`,
  `--session-id`. Result object: `{type, subtype, is_error, permission_denials[], result,
  num_turns, duration_ms, total_cost_usd, …}`. **Refusal ⇒ `subtype:"success"`, `is_error:false`.**
- `codex-cli 0.144.4` — `codex exec --json` (JSONL), `-o/--output-last-message <FILE>`,
  `--output-schema`, `-s/--sandbox read-only|workspace-write|danger-full-access`, `-C/--cd`,
  `--add-dir`, `--ephemeral`, `--ignore-user-config`; top-level `-a/--ask-for-approval`. Also a
  `codex sandbox` subcommand. Event vocabulary: `thread.started | turn.started | turn.completed |
  turn.failed | item.*`; item types include **`file_change`** with `add|delete|update`.
- Honest limit, flagged by the scout: codex's process **exit code on `turn.failed` was not
  verified**. The airtight case is "codex exits 0 on a *completed* turn that produced no file" —
  build the oracle on that, not on an assumed exit mapping.

### References

- Shared preflight: `_bmad-output/implementation-artifacts/j1-crosshost-2-cross-host-signed-run.md`
  (§1 adapter viability, §2 P1-P14, §4 the ratified split, §5 blockers)
- Decision register: `_bmad-output/planning-artifacts/epics/epic-14-preflight-decisions.md`
  (D10, D13, D14, D15, D16, D17, D18) — **verify these still read as recorded; do not re-file**
- Predecessors: `j1-crosshost-1a-frame-borne-delegation.md` (done, `6827dc87`),
  `j1-crosshost-1b-consent-proofs-and-gate.md` (ready-for-dev), `j1-demo-one-command-scene`
- Runbook: `_bmad-output/test-artifacts/runbook-j1-tier-2-signed-live-run.md` — abort conditions
  `:252-259`, host grants `:131-138`, bubblewrap note `:74`. **It mentions claude zero times.**
- T6 evidence: `_bmad-output/test-artifacts/j1-tier2-evidence/{j1-tier2-capture.json,
  j1-tier2-bundle.json}`

---

## Dev Agent Record

### Agent Model Used

`anthropic/claude-opus-5` — omp harness (`bmad-dev-story`), 2026-08-16, frontier-class per the
story's `model` field. **Recorded by policy, not by enforcement:** five story-file discipline gates
walk `_bmad-output/implementation-artifacts/` behind a digit-prefix filter and skip this filename
outright (`check_bare_review_findings.rs:35`, `check_dev_model_tier.rs:103`,
`check_dev_model_used_populated.rs:136`, `check_dev_record_completeness.rs:245-247`,
`check_review_findings_resolved.rs:57-60`), and two more skip it by a different mechanism
(`check_epic_close_coherence.rs:215-217`, `check_epic_6_bridge.rs:817-825`). All five
directory-walkers are BLOCKING CI jobs. **Net effect: this story could ship with no dev record, no
`dev_model_used`, no §A6 marker and no review-findings closure, and zero gates would notice.** A
green CI does not mean the review net ran. Filed as **D19** in
`_bmad-output/planning-artifacts/epics/epic-14-preflight-decisions.md:54` with **14-0** as the named
vehicle and a mechanical deadline (before the next `j1-*` story leaves `ready-for-dev`). `2a`
discloses; `2a` does not fix five gates (AC4.6).

§A6 review net for this story is **NON-DEGRADABLE** and has NOT yet run: Blind Hunter · Edge Case
Hunter · Acceptance Auditor · **Test-Infra Auditor** · runtime execution. 1a's pass omitted
Test-Infra, which is the layer that catches exactly this story's defect class. Run `code-review`
with a DIFFERENT model than the one above.

### Debug Log References

**Adapter contracts were derived from ground truth, not from memory.** Both structured-output shapes
were verified against the installed binaries before the oracles were written:

- **codex** — `codex exec --json` was EXECUTED unauthenticated under an isolated `CODEX_HOME` and
  emitted `{"type":"thread.started","thread_id":"…"}`, confirming `ThreadEvent` is externally tagged
  on `type`. The item shape was then confirmed against
  `codex-rs/exec/src/exec_events.rs @ rust-v0.144.4`: `ThreadItemDetails` is
  `#[serde(tag = "type", rename_all = "snake_case")]` FLATTENED into `ThreadItem`, and
  `FileChangeItem` carries `changes: Vec<FileUpdateChange>` plus
  `status: PatchApplyStatus{in_progress|completed|failed}`. The oracle requires
  `status == "completed"` because `item.completed` is a LIFECYCLE terminal, not a success.
  `codex app-server generate-json-schema` was also dumped; its v2 bundle is camelCase and does NOT
  describe the exec JSONL stream — do not derive the exec oracle from it.
- **claude** — the `2.1.233` binary's own result emitter was read out of the shipped bundle:
  `{type:"result",subtype:"success",…,is_error:!1,…,result:"",…,permission_denials:[],session_id,uuid}`.
  This is the measured refusal shape: `subtype: "success"`, `is_error: false`.

**Proven-red evidence, observed not asserted.** After the oracles landed and BEFORE the tests were
repaired, exactly the three tests the story's Dev Notes table predicted went red, and for the
predicted reason: `codex_completes_on_final_stdout_message` and
`claude_shares_the_final_stdout_oracle` both returned `NotCompleted(NoCompletionMarker)` where they
had asserted `Completed`, and `label_is_nonsecret_and_never_leaks_message` returned
`"not_completed:no_completion_marker"`. That is the defect being removed, demonstrated: prose on
stdout plus exit 0 no longer certifies completion. The three the table predicted would SURVIVE did
(`argv_appends_task_as_trailing_arg`, `only_codex_declares_noninteractive_env_and_no_secret_leaks`,
`only_the_fixture_speaks_the_bridge_handshake`) — flags stayed out of `argv()` and out of
`probe_strategy` (Traps 10, 11).

**Runtime execution.** `cargo run -p xtask -- demo-j1` was run end to end: 11 executed beats all
`PROVEN_BLOCKING`, 4 `ABSENT` with named owners, exit 0. The renamed field renders as
`last worker stdout TL ref 01a0097aabea997a… (evidence pointer, not the verdict)` and the gate beat
reports all five legs.

**Pre-existing red, confirmed NOT ours.** `check-env-contract` fails on two unregistered
`maos-bin/src` `MAOS_*` reads (`MAOS_OPERATOR_BEARER_TOKEN`, `MAOS_OPERATOR_HTTP_BIND`). Verified by
`git stash` against the working-tree baseline: identical two violations, only the line numbers
shift. This story adds ZERO new `std::env::var*` reads under `crates/maos-bin/src` (Trap 5).

### Completion Notes List

**AC1 — the completion oracle stops certifying refusals.**

- **AC1.1** `worker_cli` moved to `crates/maos-bin/src/lib.rs` as
  `#[cfg(feature = "network")] pub mod worker_cli;`, mirroring `delegation` and carrying `topology`'s
  rationale comment. `main.rs` now CONSUMES it (`use maos_bin::worker_cli;`) rather than declaring a
  second copy. The 204-CODE-line in-`src` `#[cfg(test)] mod tests` was relocated verbatim (bodies
  unchanged; dedented and re-rooted on `use maos_bin::worker_cli::*;`, exactly as 1a's
  `topology_delegation_1a.rs` relocation did). **Measured `maos-bin`: 16219 → 16017, exactly −202.**
  The predicted −204 minus the 2 CODE lines added to `lib.rs`; fully accounted, not approximated.
  *Deviation, stated:* the story asked for its own commit for attributability. `AGENTS.md` says
  "Commit only when explicitly requested by the user", so nothing was committed. The measurement was
  taken at the relocation boundary instead and is recorded above, which is what the own-commit
  requirement existed to produce.
- **AC1.2** `final_stdout_message_oracle` is DELETED. `CodexCli` now uses `codex_jsonl_oracle`
  (per-line JSONL: requires the last terminal event to be `turn.completed`, plus at least one
  `item.completed`/`file_change` with a non-empty `changes` array and `status: "completed"`).
  `ClaudeCli` uses `claude_result_object_oracle` (JOINED stdout, then requires
  `type == "result"`, `subtype == "success"`, `is_error == false`, and `permission_denials` PRESENT
  and EMPTY). Three new typed non-completions carry the verdicts: `TurnFailed`, `NoEffectEvidence`,
  `PermissionDenied`.
  **The asymmetry is recorded in the code AND as an executable claim.**
  `the_two_oracles_are_not_equivalent_and_the_asymmetry_is_asserted` drives the SAME logical event —
  "the model declined without attempting a tool call" — through both adapters and asserts codex
  catches it (`NoEffectEvidence`) while claude reports it as a COMPLETION. That residual is claude's,
  is named at the oracle, and is not closed here. Requiring `file_change` is sound rather than an
  over-constraint on read-only work only because `CodexCli::required_argv_flags` makes a missing
  `--sandbox workspace-write` a refusal: every admitted codex run is write-class by construction.
- **AC1.3** `WorkerCli::required_argv_flags` + `refuse_missing_argv_flags`, enforced at
  `main.rs` where `config.argv_prefix` is in scope, before the hash is bound and before the spawn.
  codex requires `["exec","--json","--sandbox","workspace-write"]`; claude requires
  `["--print","--output-format","json","--bare"]`; the fixture requires nothing (its marker oracle
  needs no flags, and giving it a requirement would red the hermetic journey suites). Covered by a
  unit negative AND an end-to-end negative.
- **AC1.4** Four end-to-end vectors in `worker_completion_2a.rs::planted_refusal`, all through the
  REAL `maos run` binary, all hermetic, all with `MAOS_LIVE_AGENT=1`, a fake `codex`/`claude` shell
  script on a prepended `PATH`, an inline `MAOS_HOST_GRANTS` and `.env("HOME", tmp)`. All go through
  the TOPOLOGY path with `host` set. Each asserts non-zero exit AND that stderr NAMES the typed
  failure — never exit code alone.
- **AC1.5** CLOSED, not bounded. `main.rs`'s standalone `[cli_wrapper]` path now consults
  `is_completed()` and errors with `"standalone cli_wrapper worker did not complete (<label>)"`. The
  `None` delegated-task argument is untouched (1a AC3.5). Regression risk was nil as predicted:
  `smoke_cli_wrapper_8_12` (3), `drain_once_audit_writer` (1) and `journey_j1` (3) all stay green
  because the fixture's marker oracle returns `Completed`.
- **AC1.6** `completion_tl_ref` → `last_stdout_tl_ref`, still UNCONDITIONAL. `demo_j1.rs`'s P4 beat
  no longer scores `completed && !tl_ref.is_empty()`; it scores the oracle's verdict alone and
  reports the ref as an evidence pointer. Trap 9's narrator fixtures were repaired in the same pass,
  and `completion_requires_the_oracle_and_a_tl_ref` was INVERTED into
  `completion_comes_from_the_oracle_not_from_a_tl_ref`, which now proves both directions: a
  completion with a null ref PASSES, and a non-completion with a perfectly good ref FAILS. The old
  test asserted the null control.
- **AC1.7** Three legs added to `check-j1-loopback-delegation`, all root-relative and source-static
  (no `cargo` invocation — Trap 1): `completion-oracle-per-adapter`, `worker-cli-under-library`,
  `completion-vectors-enrolled`. `worker_cli.rs`, `lib.rs` and **`.github/workflows/discipline.yml`**
  joined the governed-files list; the workflow read copies `check_loom_substrate_drift.rs:66` and
  `check_epic_6_bridge.rs:619`. Gate GREEN at HEAD with all five legs.
- **AC1.8** Two `cargo test` steps appended to the existing `check-j1-loopback-delegation` job
  (already `Blocking`, already in `gate-registry.toml`, already a `needs` of the aggregate), both
  `--test-threads=1` per D16. No new job.
- **AC1.9** `xtask/tests/j1_crosshost_2a_proven_red.rs`: 13 vectors including the
  `baseline_fixture_tree_is_green` anti-vacuity control and `assert_red`'s three-part assertion.
  `lay_green` lays a `.github/workflows/discipline.yml` into the fixture tree — the one new cost F23
  named, paid so the enrollment falsifier can fire at all. Two extra controls the AC did not ask for
  but the gate needed: a `#[cfg(test)]` mention of the retired oracle must NOT red (false-alarm
  guard), and reformatting the enrollment step across line continuations must NOT flip the leg
  (the `mailbox.rs`-needle defect class from `6827dc87`). **`j1_crosshost_1a_proven_red.rs::lay_green`
  was extended with the three newly-governed files** — without that, every 1a vector would have gone
  red for the wrong reason (a missing governed file is a finding, never a skip). 1a: 11 pass.
- **AC1.10** Fixture path untouched and PROVEN so:
  `the_hermetic_fixture_path_is_untouched_by_the_credential_controls` runs the real fixture manifest
  with `MAOS_LIVE_AGENT=1` and BOTH providers' credential files planted, and asserts exit 0 with
  `"completed":true`.

**AC2 — the credential posture is asserted, not defaulted.**

- **AC2.1** `ClaudeCli::ambient_auth_path` returns `~/.claude/.credentials.json` (leading dot on the
  filename). `worker_cli.rs`'s `assert_eq!(ClaudeCli.ambient_auth_path(home), None)` was INVERTED in
  place, not accompanied — the test is renamed
  `every_real_adapter_names_its_ambient_auth_footgun_but_the_fixture_is_immune` and keeps the
  positive-before-negative ordering and the fixture immunity control.
- **AC2.2** Return type NOT widened. Both residuals are stated at the impl site in code: a file check
  is a FILENAME control (`~/.codex/auth.json.bk` exists on the dev box — the codex invariant was
  satisfied by renaming), and **the environment channel is inherited by design and unattested**. The
  `nonsecret_env` doc comment was QUALIFIED, not made true (Trap 24): it now states plainly that
  there is no host-side injection, that no setter for any provider credential exists in
  `crates/maos-bin/src`, that the spawn has no `env_clear`, and that the `env_clear` repair is
  kernel-core and out of scope.
- **AC2.3** `--bare` routed through `required_argv_flags`, and F21 is cited at the impl: it is a
  REPRODUCIBILITY precondition (skips hooks/LSP/plugin-sync/auto-memory/**`CLAUDE.md`
  auto-discovery**, and the child inherits `maos`'s cwd in a repo that ships a tracked `CLAUDE.md`),
  not merely credential hygiene. Both the trait doc and the free function say so, so the next reader
  cannot delete it as redundant.
- **AC2.4** Unset `HOME` on the live path now REFUSES ("an unverifiable credential control is not a
  satisfied one"), proven by `an_unset_home_refuses_the_live_run_instead_of_skipping_the_check`. The
  `MAOS_HOST_GRANTS` degrade-to-builtin branch is documented as a deliberate disposition on
  `load_host_grant_allowlist`: safe today because the built-in set grants only the fixture so every
  real CLI then fails closed, but it IS a silent downgrade of an operator-intent file, and the safety
  argument is stated as the reason it is tolerable rather than an argument that it is correct.
- **AC2.5 BOTH halves.** (a) `WorkerCli::credential_env_var` (codex → `CODEX_API_KEY`, claude →
  `ANTHROPIC_API_KEY`, fixture → `None`) and the demo derives the scanned variable from the adapter.
  (b) The `if !secret.is_empty()` guards are GONE from both scans. The preflight now REFUSES the
  signing when the variable is unset, naming it, so "the operator forgot to export the variable" and
  "the scan ran and passed" can no longer produce byte-identical signed evidence. No `"not-scanned"`
  value was introduced (Trap 25).
- **AC2.6** The demo's clean-home preflight calls `refuse_ambient_auth` through the resolved adapter.
  It also refuses when `HOME` is unset, for the same reason the production path does.
- **AC2.7** `a_planted_claude_credentials_file_refuses_the_live_run` — `HOME`-isolated, asserts the
  refusal NAMES the planted path, and additionally asserts no `cli_wrapper_loaded` event, proving the
  refusal precedes the spawn.

**AC3 — reproducible from the repo, and nothing signs a claim about a run it did not make.**

- **AC3.1** Both files PORTED with `git show 60d4080a:<path> > <path>` and `diff`-confirmed
  byte-identical before repair — never hand-transcribed (Trap 22). Repairs: four `priority_weight`
  keys stripped, `host = "developer-remote-host"` added to the worker entry, and the header's three
  stale claims corrected (`MAOS_WORKER_TASK`, the retired oracle, and the pre-flight step that told
  the operator to confirm "a final message is on stdout"). `--json` was APPENDED; the long-form
  `--sandbox workspace-write` bytes were never retyped.
- **AC3.2** `spirits/worker/manifest-claude.toml` authored with `--print --output-format json --bare
  --permission-mode acceptEdits --settings '<inline JSON>'`. Every flag is justified in place,
  including why `-p` alone cannot execute a coding task (the measured deny strings, no TTY under
  `--print`, exit 0 — the ship-blocker's mechanism) and why `bypassPermissions`/`dontAsk` are
  forbidden (they would also hide the denial signal the oracle depends on). `[author] name =
  "Anthropic"` must match a host grant.
- **AC3.3** `crates/maos-bin/tests/worker_manifests_2a.rs` — the reader `spirits/*/manifest*.toml`
  never had. Drives each through `CliWrapperConfig::from_toml_str` the same way the composition root
  does (re-serialized `[cli_wrapper]` section), asserts adapter resolution, `[author] name`
  non-empty, `[sandbox] tier = "T3"`, and `refuse_missing_argv_flags`. Enrolled in
  `discipline.yml`, so it is a control rather than a suggestion (F17).
- **AC3.4** `demo_j1.rs:1007`'s literal check WIDENED to `SIGNABLE_WORKER_CLIS = ["codex","claude"]`
  and never deleted (F8) — the fixture is still refused. The check now fires TWICE: once in the
  preflight against the adapter the topology DECLARES (before money is spent) and once post-run
  against the identity that actually RAN, plus a new mismatch check between the two. The
  `tier2-live-agent-signed` owner string was widened. The `--live-codex`/`--codex-topology` flag
  names were left alone (rename optional); the owner string now says the adapter comes from the
  topology, not the flag name.
- **AC3.5** `command_metadata` is DERIVED: the manifest path actually used, the topology path, the
  adapter identity, the real `argv_prefix`, and the credential variable — with "injected host-side"
  REPLACED by "inherited from the operator's environment — MAOS neither injects nor holds it (value
  redacted, scanned)", per F22. A claude run can no longer sign a bundle asserting it was codex.
- **AC3.6** No disproved claim is restated anywhere in this story's artifacts. The manifests exist
  and were ported; claude's sandbox counterpart is real and argv-hashed; external jails are NOT
  structurally blocked (basename dispatch has no realpath/hash/signature check, and that sentence is
  in the capture validator's own doc); the 1a boundary-leg claim is not repeated.

**AC4 — the isolation posture as a machine-checked claim, closed honestly.**

- **AC4.1** ASSERTED, not landed. The codex flag was ported, never retyped, and
  `crosshost_2a_codex_manifest_keeps_the_attested_long_form_sandbox_spelling` pins the long form and
  its value order, refusing `-s`. `crosshost_2a_real_adapter_manifests_declare_an_isolation_posture`
  asserts every real-adapter manifest carries a non-empty isolation declaration in its HASHED
  `argv_prefix` (codex `--sandbox`, claude `--settings`) and that none carries a bypass flag. The
  fixture is exempt by construction and the test asserts that exemption is fixture-only. The ratified
  claim is written in these words, in the code: **the FS jail is the ADAPTER's, DECLARED by MAOS in a
  hashed manifest, ENFORCED by the adapter, not by MAOS.**
- **AC4.2** `CaptureDoc` gains `fs_jail` (exact-match `adapter-enforced-maos-declared`) and a
  required non-empty `fs_jail_followup`, mirroring the `egress` precedent field for field, with
  `capture_validation_refuses_maos_enforced_fs_jail_overclaim` (both the overclaim `"maos-enforced"`
  and the under-claim `"none"` are refused) and
  `capture_validation_requires_an_fs_jail_followup`. `demo-j1` emits both fields and narrates the
  posture. Not a sentence in a story file.
- **AC4.3** The asymmetry is recorded in the direction the evidence points: claude's counterpart is a
  settings document, codex's is a flag, BOTH are argv-declarable and argv-hashed, so the two adapters
  are symmetric on this axis. claude additionally offers enforced egress allowlisting, a fail-closed
  startup gate and `--bare` — controls codex does not have. The residual that weakens the claim is
  named as MAOS's own (no namespace, no rlimit, no process group, basename dispatch), never as an
  adapter's.
- **AC4.4 Budget, measured, attributed by key.**

  | Key | Baseline | After | Ceiling | Verdict |
  |---|---|---|---|---|
  | `maos-bin` | 16219 (0 headroom) | **16159** | 16219 | GREEN, **+60** — AC1.1 returned 202, the story spent 142 |
  | `xtask` | 38075 | **38319** | 38609 | GREEN, +290 (spent 244) |
  | `maos-cli` | 4601 | **4642** | 4642 | GREEN under a **measured grant**, zero headroom |
  | `maos-kernel-core` | 18933 | 18933 | 18248 | RED +685 — **D13, not this story. ZERO Δ.** |
  | `maos-domain` | 8694 | 8694 | 8644 | RED +50 — **D14, not this story. ZERO Δ.** |
  | `_aggregate_hardfail` | 147549 | 147774 | 147057 | RED +717 — **+492 is D17 (standing); +225 is this story's measured net** (−60 +244 +41) |
  | `check-kernel-baseline` | 24472 | **24472** | 24472 | GREEN — **ZERO kernel-core Δ** |

  `maos-cli` breached by exactly **+16** (4642 vs 4626), entirely from AC4.2's capture surface in
  `subcommands.rs`. Per `kloc.toml:63-65` that needs a named grant or a decomposition; it was raised
  to the operator WITH the measurement attached and **Lunarpulse ratified a measured grant
  4626 → 4642 (exact measured, zero headroom, mirroring D15)**, recorded in `kloc.toml:263`. The
  fail-closed diagnostic was deliberately NOT compressed to fit a ceiling. `kloc-check` still exits 1
  at close on the three standing keys, exactly as AC4.4 anticipated. **F11 confirmed as measured:**
  the `kloc.toml:87` "must never block a correctness repair" valve is prose —
  `xtask/src/kloc_check.rs` contains no `exempt`/`waiv`/`correctness`/`compliance` token and its
  compare loop is an unconditional `if *loc > budget`. It is permission to ASK plus a `kloc.toml`
  edit, which is precisely the path taken.
- **AC4.5** No beat MOVED. The P4 beat's ASSERTION changed (de-conjoined) and its narration was
  updated in the beat list in code; the beat name is unchanged and nothing was added or removed.
  **The evidence ledger did not enforce any of this:** `demo-j1` has ZERO CI invocation,
  `ledger_gates()` derives from the four Postgres substrate gates so the J1 lane sits structurally
  outside evidence-ledger enforcement, and `Beat::absent` sets `executed: false` so an unlanded beat
  never fails a run. The demo was therefore RUN by hand as the verification (see Debug Log).
- **AC4.6** The seven blind gates are disclosed above under Agent Model Used, with D19 and 14-0
  named as owner and vehicle.

**Residuals handed OUT, with owners — none of them silently absorbed.**

- F22's `env_clear` repair — kernel-core `runtime.rs`, FLAG-Winston. `2a` owes the honest sentence at
  the impl site and nothing more; it is written.
- D19's seven blind story-file gates — **14-0** decomposes.
- claude's bare-decline residual (no tool attempt ⇒ `permission_denials` empty ⇒ indistinguishable
  from success) — asserted as an executable measured fact, not hidden. Closing it needs a working-tree
  effect oracle, which needs a `cwd` on `BridgeSpawnSpec` and a `.current_dir()` in
  `spawn_and_bridge`: kernel-core Δ, FLAG-Winston, DEFERRED at preflight.
- `bilateral-2-host-mira-nash.toml` declares `host` on CLASS Spirits, which
  `validate_remote_topology_target` REFUSES — it parses but cannot be loaded today. Discovered by
  AC3.3's new reader. It is a forward declaration owned by `j1-crosshost-2b`, is not on this story's
  path, and is recorded in the reader's doc comment rather than asserted away.
- `success_criteria` remains circular and dead (F15) — `2b`'s typed task-outcome work. Not started.
- `check-env-contract`'s two unregistered `MAOS_*` reads — pre-existing, verified by `git stash`.
- **What `2a` may claim at close:** *claude is signable* — an honest oracle, an asserted credential
  posture, committed and machine-read manifests, derived capture metadata. NEVER *claude works*. No
  paid run happened and none is required (Q1). `abi-diff` is not cited as evidence for anything here;
  no ABI change was made, which is a fact about this diff and not a gate result (Trap 8).

### File List

**Modified**

- `crates/maos-bin/src/lib.rs` — `pub mod worker_cli` with its rationale (AC1.1)
- `crates/maos-bin/src/main.rs` — consume `maos_bin::worker_cli`; `refuse_missing_argv_flags` at the
  composition root; unset-`HOME` refusal; `MAOS_HOST_GRANTS` disposition; `last_stdout_tl_ref`;
  standalone completion enforcement
- `crates/maos-bin/src/worker_cli.rs` — three typed non-completions; `required_argv_flags`;
  `credential_env_var`; `refuse_missing_argv_flags`; `codex_jsonl_oracle`;
  `claude_result_object_oracle`; `final_stdout_message_oracle` DELETED;
  `ClaudeCli::ambient_auth_path`; `nonsecret_env` qualified; in-`src` tests removed
- `crates/maos-cli/src/subcommands.rs` — `fs_jail`/`fs_jail_followup` capture fields, exact-match
  validation, fixture, two overclaim negatives (AC4.2)
- `xtask/Cargo.toml` — `maos-bin` path dependency so the demo asks the ADAPTER (AC2.6/AC3.5)
- `xtask/kloc.toml` — ratified measured grant `maos-cli 4626 → 4642` (AC4.4)
- `xtask/src/check_j1_loopback_delegation.rs` — three new legs, three newly governed files,
  `ledger_leg_names()` (AC1.7)
- `xtask/src/demo_j1.rs` — de-conjoined P4 beat; `last_stdout_tl_ref`; `resolve_topology_worker`;
  adapter-derived clean-home, credential variable and `command_metadata`; `SIGNABLE_WORKER_CLIS`;
  unconditional redaction scans; FS-jail posture constants and narration
- `xtask/src/tests/demo_j1_tests.rs` — narrator fixtures repaired; the tl_ref conjunction test
  inverted into a two-direction control (Trap 9)
- `xtask/tests/j1_crosshost_1a_proven_red.rs` — `lay_green` extended with the three newly governed
  files so 1a's vectors keep failing for the RIGHT reason
- `.github/workflows/discipline.yml` — 2a proven-red, `worker_completion_2a` and
  `worker_manifests_2a` enrolled on the existing blocking job (AC1.8)

**Added**

- `crates/maos-bin/tests/worker_completion_2a.rs` — the relocated adapter contracts plus the
  structured-oracle vectors and the hermetic planted-refusal suite (kloc-free)
- `crates/maos-bin/tests/worker_manifests_2a.rs` — the worker-manifest reader and the isolation-posture
  assertion (kloc-free)
- `xtask/tests/j1_crosshost_2a_proven_red.rs` — 13 proven-red vectors incl. the enrollment falsifier
  (kloc-free)
- `spirits/topologies/j1-founder-loop-codex.toml` — ported from `60d4080a`, repaired (kloc-free)
- `spirits/worker/manifest-codex.toml` — ported from `60d4080a`, repaired (kloc-free)
- `spirits/worker/manifest-claude.toml` — authored with the ratified argv posture (kloc-free)

**Deleted** — none. `final_stdout_message_oracle` and `worker_cli.rs`'s in-`src` test module were
removed in place within modified files.

---

## Open Questions

_Both closed at the 2026-08-16 preflight round-table. Kept here with their resolutions so the record
shows what was decided and why, rather than looking unanswered._

**Q1 — Does the paid heterogeneous run happen in `2a`, or wait for `2c`? → RESOLVED: not in `2a`.**
This story makes a claude run *signable* (honest oracle, asserted credential posture, committed
manifests, derived capture metadata). It does not require one to be *signed*.
`tier2-live-agent-signed` is owned by `"--live-codex (operator-local, never CI)"`
(`demo_j1.rs:795`). Spending money to demonstrate the property proves strictly less than AC1.4's
hermetic planted-refusal vector does — the vector proves the oracle *refuses*, which a successful paid
run cannot. **Boundary on what `2a` may claim at close:** *"claude is signable"*, never *"claude
works"*. Those are different sentences and only the first one is earned here. If the operator wants
the paid run anyway it is additive and changes no AC.

**Q2 — Is the residual claude credential surface acceptable at `2a`? → RESOLVED: yes, and do NOT widen
`ambient_auth_path`'s return type.** A `Vec<PathBuf>` buys exactly one surface (`~/.claude.json`)
while implying the keychain and the environment were covered — the project's signature failure mode
wearing a type signature. `--bare` is the real control and it **deletes** rather than mitigates:
verbatim from `claude 2.1.233 --help`, *"OAuth and keychain are never read"*, leaving
`ANTHROPIC_API_KEY` and `apiKeyHelper`-via-`--settings`, both of which MAOS controls through the
argv-hashed manifest. What remains is one variable on an inherited environment channel, and F22 is the
honest sentence for it. Folded into AC2.2 and AC2.3; nothing is left for the dev to decide.

**Residual handed OUT of this story, not resolved in it:** F22's `env_clear` repair (kernel-core
`runtime.rs`, FLAG-Winston) and D19's seven blind story-file gates (**14-0** decomposes). Both have
named owners; neither is `2a`'s.

---

## Change Log

| Date | Change |
|---|---|
| 2026-08-16 | **§A6 REVIEW COMPLETE — Status `review` → `done`** (4-layer net: Blind + Edge + Acceptance + Test-Infra + runtime, under `zai/glm-5.2` — a different model than the dev pass, as the Dev Agent Record requires; runtime layer executed by the reviewer). 25 raw findings → 11 patch (all APPLIED and verified green), 3 defer (owners recorded in deferred-work.md), 4 dismissed with grounds. Headline patches: (P1) the enrolled CI step was RED on every fresh runner — nothing in the `check-j1-loopback-delegation` job builds `worker-cli-fixture`, which the fixture-immunity vector resolves at runtime (reproduced 39/40 → 40/40); (P2/P3) argv validation was token-presence with no bypass refusal, and `fs_jail: adapter-enforced-maos-declared` was sealed unconditionally — now GROUP-bound (flag,value adjacency), bypass-token-refusing at the production seam, and content-checked (`--settings` must enable `sandbox`) before any seal; (P4) hostless topology entries still exited 0 on a refusal — the AC1.5 defect one branch over, now enforced for every topology entry; (P5) multi-`[cli_wrapper]` topologies are refused at the signing preflight (it attests ONE worker); (P6) the dirty signed bundle is deleted on redaction refusal; (P7) empty-string `HOME` is treated as unset at both refusal sites; (P8) the gate legs needle the oracle CALL forms and the enrollment is scoped to the Blocking job (+2 proven-red vectors, 2a now 15, 1a 11); (P9/P10) claude's permission posture is machine-checked and the manifest reader is recursive; (P11) the signing controls gained unit coverage (24 xtask demo tests). Budget: review patches consumed +101 CODE in `maos-bin`; **Lunarpulse ratified a measured grant 16219 → 16260 (zero headroom)** recorded in `kloc.toml`, mirroring D15. Verification at close: `worker_completion_2a` 45/45, `worker_manifests_2a` 4/4 (the posture test now drives the PRODUCTION `refuse_missing_isolation`/`refuse_unsafe_argv` seams instead of its own token scan), 2a proven-red 15/15, 1a proven-red 11/11, smoke 3/3, drain 1/1, gate 5 legs GREEN, `check-kernel-baseline` 24472==24472 (ZERO kernel-Δ retained), kloc red ONLY on the standing keys (D13/D14/aggregate), fmt clean, `demo-j1` exit 0 with 11 PROVEN_BLOCKING beats. |
| 2026-08-16 | **DEV PASS COMPLETE — all 4 ACs satisfied, all 12 tasks closed; Status `ready-for-dev` → `review`** (`anthropic/claude-opus-5`, omp `bmad-dev-story`, baseline `5a921c0c`). **The ship-blocker is closed, and the closure was OBSERVED, not asserted:** `final_stdout_message_oracle` is DELETED, and before the tests were repaired exactly the three legs the Dev Notes table predicted went red for the predicted reason — prose on stdout plus exit 0 no longer certifies completion. `CodexCli` now consumes `codex exec --json` (terminal `turn.completed` **and** an applied `item.completed`/`file_change`), `ClaudeCli` consumes the joined `--output-format json` result object (`subtype == "success"`, `is_error == false`, `permission_denials` present **and** empty). Both contracts were derived from GROUND TRUTH, never memory: `codex exec --json` was executed unauthenticated to confirm `{"type":"thread.started",…}` and the item shape was read from `codex-rs/exec/src/exec_events.rs @ rust-v0.144.4`; claude's result emitter was read out of the `2.1.233` bundle. **The asymmetry AC1.2 forbade papering over is an EXECUTABLE claim**, not prose: one test drives "the model declined without attempting a tool call" through both adapters and asserts codex catches it while claude reports a COMPLETION. **AC1.1 funded the story as predicted and to the line:** `maos-bin` 16219 → 16017 (**exactly −202** = the measured 204 minus 2 CODE lines added to `lib.rs`), closing at **16159/16219, +60 GREEN** from zero headroom. **Both false-success surfaces are closed** — the topology path already enforced, and the standalone path now consults `is_completed()` instead of discarding it, which matters because the signed run's own runbook sends the operator down that path first. **F23's vacuous falsifier is repaired:** `.github/workflows/discipline.yml` joined the gate's governed files, so `xtask/tests/j1_crosshost_2a_proven_red.rs` (13 vectors, baseline-green control, three-part `assert_red`) can actually fire on a deleted `--test` line; `lay_green` lays a workflow into the fixture tree, and `j1_crosshost_1a_proven_red.rs::lay_green` was extended with the three newly-governed files so 1a's 11 vectors keep failing for the RIGHT reason. Gate GREEN at HEAD with **five** legs. **AC2.1 inverted a false green** (`ClaudeCli.ambient_auth_path == None` under a comment reading "only codex names the footgun") rather than adding a test beside it, and the two residuals are stated in code: a file check is a FILENAME control, and **the environment channel is inherited by design and unattested** (F22 — `nonsecret_env`'s doc comment was QUALIFIED, never made true). **AC2.5 did BOTH halves:** the scan is adapter-derived (`credential_env_var`), and the `if !secret.is_empty()` guards are gone so an UNEXECUTED scan can no longer emit byte-identical evidence to a passing one — the signing refuses instead, with no `"not-scanned"` value invented. **Nothing signs a run it did not make:** `command_metadata` is derived from the adapter, manifest, topology and real `argv_prefix`, `:1007` widened to an allowlist (never deleted — the fixture stays refused, and the check now fires both pre-run against the declared adapter and post-run against the actual one, plus a mismatch check). **Manifests PORTED, not authored** (`git show` + `diff`-confirmed byte-identical, then repaired: `priority_weight` stripped, `host` added, three stale header claims corrected, `--json` appended, the attested long-form `--sandbox workspace-write` bytes never retyped — pinned by its own test), and `manifest-claude.toml` authored with the ratified posture. They finally have a READER (`worker_manifests_2a.rs`), enrolled in CI, which also asserts the isolation posture AC4.1 needed so it stops being a doc comment in a different file. **AC4.2 mirrors the egress precedent field for field:** `fs_jail` exact-match `adapter-enforced-maos-declared` + required `fs_jail_followup` + negatives refusing BOTH the overclaim and the under-claim. **VERIFIED BY EXECUTION, not by inference:** full `-p maos-bin --test-threads=1` 0 failures (D16 respected), `-p xtask` / `-p maos-cli` / `-p worker` / `-p maos-journey-test` / `-p maos-manifest` all green, `cargo fmt --all --check` clean, clippy clean on every touched file, and `cargo run -p xtask -- demo-j1` run end to end (11 executed beats `PROVEN_BLOCKING`, 4 `ABSENT` with owners, exit 0) — necessary because `demo-j1` has ZERO CI invocation and the evidence ledger structurally does not enforce this lane (AC4.5). **ONE OPERATOR DECISION TAKEN, with the measurement attached:** AC4.2's capture surface pushed `maos-cli` +16 over (4642 vs 4626), so per `kloc.toml:63-65` Lunarpulse ratified a measured grant **4626 → 4642** (exact measured, zero headroom, mirroring D15) rather than compressing a fail-closed operator diagnostic to fit a ceiling. `check-kernel-baseline` GREEN at **24472 = 24472, ZERO kernel-core Δ**. `kloc-check` exits 1 on exactly the three standing keys — `maos-kernel-core` +685 (D13), `maos-domain` +50 (D14), `_aggregate` +717 of which **+492 is D17 and +225 is this story's measured net** — attributed by key, none absorbed. **Boundary on what this story claims: *claude is signable*, never *claude works*** (Q1) — no paid run happened and none is required. Residuals handed out with named owners: F22's `env_clear` (kernel-core, FLAG-Winston), D19's seven blind story-file gates (**14-0**), claude's bare-decline residual (needs the deferred effect oracle), and a NEW finding from AC3.3's reader — `bilateral-2-host-mira-nash.toml` declares `host` on CLASS Spirits, which `validate_remote_topology_target` REFUSES, so it parses but cannot be loaded today (`j1-crosshost-2b`'s). One stated deviation: the story asked for T1 in its own commit for attributability; `AGENTS.md` forbids committing unrequested, so the measurement was taken at the relocation boundary and recorded instead. **§A6 review net has NOT run and is NON-DEGRADABLE** (Blind · Edge · Acceptance · **Test-Infra** · runtime) — and five story-file discipline gates plus two more are BLIND to this filename, so a green CI is not evidence the net ran. Run `code-review` with a DIFFERENT model. |
| 2026-08-16 | **Preflight round-table** (Winston · Murat · Amelia · John · Mary · Paige · Sally; Vex walked on for the credential surface). Story stays `ready-for-dev`; **still 4 ACs**, no AC added — every fix landed as a constraint on an existing one, per the standing rule. **Two ship-blockers found in the story's OWN machinery.** (F6a) **AC3.1 and AC4.1 contradicted each other:** AC3.1 says port `manifest-codex.toml` verbatim from `60d4080a` *because authoring fresh risks a different argv than the signed bundle attests*, while AC4.1 said land `-s workspace-write` — but the ported file already carries `argv_prefix = ["exec", "--sandbox", "workspace-write"]` and that **exact long-form string is what the Ed25519-signed `j1-tier2-capture.json` attests** in `command_metadata`. `-s` and `--sandbox` are identical to codex and different bytes to `argv_prefix_hash`. AC4.1's verb changed *land* → **assert**; new Trap 22 forbids retyping a ported `argv_prefix`. This also corrects **F6**: "MAOS passes no sandbox flag" is true of the tracked tree and **false of the signed run** — the T6 jail was real and hash-bound, so AC4 now ratifies *"the FS jail is the adapter's, declared by MAOS in a hashed manifest, enforced by the adapter, not by MAOS"* instead of confessing a gap. (F23) **AC1.9's falsifier could not fire:** `check_j1_loopback_delegation.rs` governs six source files and `.github/workflows/discipline.yml` is not among them, so deleting the `--test` enrollment line leaves the gate green — a falsifier standing in for a falsifier. AC1.7 gains the workflow as a governed file plus an enrollment leg (idioms copied from `check_loom_substrate_drift.rs:66` and `check_epic_6_bridge.rs:619`); `lay_green` must now lay a workflow into the fixture tree. **New findings:** (F21) `--bare` is a **reproducibility** precondition, not credential hygiene — it skips hooks, plugins, auto-memory and **CLAUDE.md auto-discovery**, and with no `cwd` on the spawn a claude worker reads this repo's tracked `CLAUDE.md`, so AC3's "reproducible from the repo" is false without it; promoted into AC3.2 alongside AC2.3. (F22) **There is no host-side credential injection** despite `nonsecret_env`'s doc comment claiming it — zero setters for `CODEX_API_KEY`/`ANTHROPIC_API_KEY`/`OPENAI_API_KEY` in `crates/maos-bin/src`, no `env_clear` on the spawn; `refuse_ambient_auth` refuses an ambient credential *file* while the process inherits every ambient credential *variable*. `env_clear` is kernel-core/FLAG-Winston → out of scope; `2a` owes the honest sentence. (F24) claude's `--output-format json` is one object that may span many TL rows — join before parsing, unlike codex's JSONL. **Five forks closed:** AC1.5 → **close** the standalone discard (the ported manifest's own runbook box sends the operator down that path first, so a green exit there is the pre-flight check certifying the defect; fixture stays green); AC1.6 → **rename** `completion_tl_ref` → `last_stdout_tl_ref`, never gate on `Completed` (the failed run is exactly when the ref is needed; the field's in-code comment is already honest, only the name and the demo conjunction lie); AC2.2/**Q2** → **do not widen** the return type (buys one surface, implies eight) — `--bare` deletes OAuth+keychain structurally; AC2.5 → do **both** halves, provider-aware *and* an unexecuted scan structurally unable to emit `"verified"` (today "forgot to export" and "scan passed" are byte-identical signed evidence); **Q1** → paid run stays out of `2a`, and `2a` may claim *"claude is signable"*, never *"claude works"*. **T1 viability verified rather than assumed:** `Cargo.toml:15` is `default = ["network"]`, the test module imports only `use super::*` and names neither private item, and `tokei` on `:399-658` is **204 CODE exactly** — the tilde is struck everywhere. **Filed out:** **D19** in `epic-14-preflight-decisions.md` — seven blocking gates blind to `j1-*` filenames, open across four stories, owner **14-0**, deadline before the next `j1-*` story leaves `ready-for-dev`; `2a`'s disposition is downgraded from "disclose" to "disclose **and cite D19**". Traps 21 → 25. |
| 2026-08-16 | **Created** at `5a921c0c` from a five-scout preflight (completion oracle · credential posture · manifests & demo coupling · sandbox/argv posture · gates, CI & budget), following the 2026-08-15 ratification of the `2a/2b/2c` split. **Twenty premises disproved or corrected.** Headline: (F1) `worker_cli` is bin-private, so the ratified proven-red vector had no legal home — and the relocation that fixes it returns ≈+204 lines to `maos-bin`, funding a story that had zero headroom; (F2) `completion_tl_ref` is assigned independently of the oracle, so "no citable ref for a refusal" was unsatisfiable and the demo's P4 beat conjoins two unrelated facts; (F3) a **second** false-success surface exists at `main.rs:4357-4366`, which the oracle fix does not close; (F4) a structured oracle is not a drop-in — the adapter never sees `argv_prefix` — and codex/claude are asymmetric (codex `--json` emits `file_change`, an effect signal; a claude refusal reports `subtype:"success"`), so one uniform oracle AC would be false for one of them; (F5) the "missing" manifests are committed at `60d4080a` on `maos-j1-live`, and porting them verbatim reds a blocking CI leg **and** spawns codex with no task; (F6) "claude has no sandbox counterpart" is inverted — claude ships bwrap/seccomp, enforced egress and a fail-closed gate, while MAOS passes **no** sandbox flag for **either** adapter and every `--sandbox workspace-write` in the tree is a doc comment; (F7) `demo_j1.rs:1044` seals a hardcoded "codex" `command_metadata` into the **signed bundle**; (F10) the redaction claim a signed run makes is `CODEX_API_KEY`-keyed and never executes for a claude worker while `CaptureDoc::validate` accepts the word `"verified"`; (F11) the `kloc.toml:87` correctness-repair valve is prose — `kloc_check.rs` has no exemption mechanism at all; (F13) `ClaudeCli`'s missing clean-home invariant is **asserted** at `worker_cli.rs:489`, not forgotten; (F16) no gate anywhere owns the worker/CLI surface; (F17) 24 of 28 `maos-bin` test targets are dead in CI, including the harness being cloned; (F18) the basename allowlist is a naming constraint the repo's own test bypasses; (F20) 1a's boundary leg is satisfied by a `format!` literal **inside** `handle_intake_verified` and cannot observe its own trigger — handed to 1b. Ratified scope items 3 and 4 were re-shaped by measurement: argv posture is a manifest concern with zero adapter code, and the FS-jail item became "ratify a posture" rather than "confess a gap". 4 ACs, 21 traps, 12 tasks; ZERO kernel-Δ @24472 with the one breaching option (working-tree effect oracle, needs `cwd` on `BridgeSpawnSpec`) deferred by name. |
