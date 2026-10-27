# 16-6 — `maosctl load` over the door — archived history (2026-10-27)

Scope: historical preflight, measurement and debug-log narrative moved verbatim from `16-6-maosctl-load-over-the-door.md` under the operator ruling of 2026-10-27. Acceptance criteria, decisions, operator rulings, open obligations, tasks and the Dev Agent Record gate sections stay in the story file; each heading below is left in place there with a one-line pointer to this file.

## 1. 🔴 THE VERB CREATES A STATE THE KERNEL CANNOT LEAVE — AC2 IS FALSE AND `kernel-Δ 0` IS FALSE

`fn is_transition_allowed` — `crates/maos-kernel-core/src/scheduler/control_block.rs:444-455`:

```rust
matches!(
    (from, to),
    (Loaded, Running)            // :448
        | (Running, Paused)      // :449
        | (Paused, Running)      // :450
        | (Running, Unloaded)    // :451
        | (Paused, Unloaded)     // :452
        | (Unloaded, Unloaded)   // :453  idempotent unload
)
```

**There is no `(Loaded, Unloaded)` arm.** A freshly loaded SCB is `Loaded`
(`control_block.rs:337`, `state: AtomicU8::new(ScbLifecycleState::Loaded as u8)`).

The epic's AC2 states verbatim (`epic-16-…md:193`): *"a door-loaded Spirit unloads through 16-1's `unload`."*
Traced through `SpiritSchedulerAdapter::unload` (`scheduler_loop.rs:472`) for a `Loaded` SCB:

| line | what happens |
|---|---|
| `:473-476` | `get_scb_optional` → `Some(scb)` — it IS in the map |
| `:478-481` | `current == Unloaded`? No, it is `Loaded`. No early `Ok` |
| `:483-486` | `scb.transition(Unloaded, Unload)?` → `control_block.rs:469-475` → **`Err(InvalidStateTransition)`. The `?` returns HERE.** |
| `:488-499` | ❌ never runs — no `lifecycle.unload` TL row |
| `:501-502` | ❌ `on_unload` never fires |
| `:504-512` | ❌ **`terminate_spirit` unreached — NO NFR-Rel-11 RECEIPT** |
| `:514` | ❌ `capability.revoke_all_for_pid` — **tokens stay live** |
| `:516` | ❌ `halt_registry.drain_for_spirit` — pending halts stay pending |
| `:518-521` | ❌ `spirits.remove(&pid)` — **the SCB LEAKS PERMANENTLY** |

Over the door this is `409 invalid_state_transition` (`operator_door.rs:479-488`).

**Second-order: the name is burned.** Because `spirits.remove` never runs, `resolve_pid` (`scheduler_loop.rs:178-186`,
a linear scan over `scb.spirit_id`) keeps matching forever, so every later `load` of that id returns
`AlreadyLoaded` (`:268-272`).

⚠ **Scope, precisely: this is true of `SpiritSchedulerAdapter::unload`, NOT of the tree.** An earlier draft of
this section claimed *"no force-unload, no purge, no admin escape hatch anywhere in the tree"* — **that was
false and is withdrawn.** Two production paths already remove a `Loaded` SCB, both by bypassing `transition`:

| path | how it bypasses | what the Spirit gets |
|---|---|---|
| `handle_crash` (`crates/maos-kernel-core/src/supervision/crash_detector.rs:120-124`) | `let _ = scb.transition(Unloaded, Unload);` — the `Err` is **discarded** and execution continues | tokens revoked `:127`, **`terminate_spirit` receipt `:130`**, `LifecycleEvent::Crash` journal `:181-189`, and `map.remove(&spirit_pid)` **`:191-194`** |
| cold-swap rollback (`crates/maos-kernel-core/src/lifecycle/upgrade.rs:200-203`) | direct `scbs().write().remove(&new_pid)`, no `transition` call at all | nothing — no row, no receipt, no revocation, no drain |

And `SpiritSchedulerAdapter::scbs()` (`scheduler_loop.rs:148`) is **`pub`**, handing out
`Arc<RwLock<BTreeMap<u32, Arc<SCB>>>>` — so any `maos-bin` caller could remove an entry in three lines at
**zero kernel-Δ**. That is a real alternative to the grant, and **D-16-6-A now argues against it explicitly**
rather than pretending it does not exist.

⚠ **AND AN EXISTING TEST PINS THE CURRENT BEHAVIOUR.** An earlier draft claimed *"zero test coverage of the
exact case … the falsifier AC2 needs has never existed"* — **false on both halves.**
`crates/maos-kernel-core/src/scheduler/control_block.rs:567-573`:

```rust
#[test]
fn loaded_to_unloaded_rejected() {
    assert!(!SpiritControlBlock::is_transition_allowed(
        ScbLifecycleState::Loaded, ScbLifecycleState::Unloaded));
}
```

Adding the arm **reds `cargo test -p maos-kernel-core`**. This test is an **eighth re-pin artifact**, it must be
rewritten in the shape of its five siblings (`loaded_to_running_allowed` `:515-525`), and because it is inline
`#[cfg(test)]` in `maos-kernel-core` it **is charged by `kloc-check`** — so §10's kernel row is `+3…+6`, not
`+1`.

**The tree already knows.** Two production comments name this defect:
- `crates/maos-bin/src/supervision.rs:1339-1344` — *"The SCB is `Loaded` and there is no `Loaded → Unloaded` transition, so it cannot be unloaded and gets no receipt — the declared residual on the `epic-16-retrospective` row."*
- `crates/maos-bin/src/shell_host.rs:242-247` — *"…so when `load` succeeded but `admit_spirit` did not, the SCB is still `Loaded` and this unload ALWAYS fails."*

And `unload_all_loaded` (`supervision.rs:139-167`) swallows it: a `Loaded` SCB lands in `report.failed` `:157`
and is skipped with `continue`.

⇒ **There is no zero-delta option that satisfies AC2 as written.** The three shapes are ruled in
**D-16-6-A**; the recommendation is the one-line kernel arm, and its true price is **seven artifacts**
(frontmatter `kernel_grant`), not the five 16-5 priced — because `RATIFIED_AT_EASING`
(`xtask/tests/recovery_lane_ceiling_rule.rs:268`) and the pin's **line-position assert**
(`kernel_pin_content_hash_16_0.rs:846-851`) are two nobody has counted before.

⚠ **Integration coverage, separately.** `crates/maos-kernel-core/tests/scheduler_five_verb_lifecycle.rs` always
`start`s first (`:143`, `:182`); `invalid_state_transition_rejected` (`:190-209`) tests **`pause` on `Loaded`**,
never `unload`. So the *behavioural* falsifier AC2 needs does not exist — but the *unit* assertion above does,
and it asserts the opposite of AC2.

## 2. 🔴 ADMISSION RUNS *AFTER* LOAD — THE EPIC'S AC1 ORDER IS INVERTED

The epic's AC1 (`epic-16-…md:192`) lists: *"manifest parse, `security.admit_spirit`, the rust-inproc class
construction, `SpiritSchedulerAdapter::load`."* **Measured, the order is the reverse of the middle two**, on
all three copies of the path:

| path | load | admit | start |
|---|---|---|---|
| shell | `main.rs:3800` | `:3813` | `:3855` |
| run / topology | `:4610-4725` | `:4729` | `:4769` |
| run / standalone | `:5181-5756` | `:5761` | `:5823` |

The tree states why, at `main.rs:4726-4727` and `:5757-5759`: **capability mediation is keyed by pid**, so
admitting a placeholder pid would make every live port fail closed. The pid does not exist until
`allocate_pid()` inside `load` (`scheduler_loop.rs:274`).

**Consequence for AC1.** An "extracted admission function" cannot be admission-then-load. It must own the
whole **load → admit → start** triple, *and* own the rollback that today's `?` at `main.rs:5788` does not do:
an `admit_spirit` rejection leaves the Spirit **loaded in the scheduler, unadmitted**, and exits `main` — and
by §1 that Spirit cannot be unloaded. **The partial-admission leak and the missing transition are the same
defect seen from two ends.**

⚠ **The kernel's own `load` admission is not the gate.** `SpiritSchedulerAdapter::load` calls
`security.admit_spirit` itself (`scheduler_loop.rs:277-323`) — but with **hardcoded** `SandboxTier::T2`, no
image pin, all-`None` `ResourceCaps`, **empty** `CapabilitiesRequired` and a fixed
`Cautious`/`AutonomousWithHalt` posture. It threads only `manifest.scheduling`, `.lifecycle`, `.on_crash`,
`.supervision` and `.class`. **A door `load` that called the kernel directly would admit a T3-requiring Spirit
at T2** — a sandbox downgrade. Every real gate lives in `maos-bin`, which is precisely why AC1 must reuse the
extracted function and not the kernel entry point.

## 2b. The gates `maos run` actually applies — re-derived, because AC1 owes one refusal vector per gate

**In `main.rs`, standalone path (the stricter of the two — see 2c):**

| # | gate | cite |
|---|---|---|
| 1 | manifest readable | `main.rs:4153-4158` |
| 2 | manifest TOML parses | `:4159-4160` |
| 3 | `[cli_wrapper]` ⊕ `[class]` mutual exclusion | `:4962-4969` |
| 4 | `[class]` present + parses | `:5083-5084` |
| 5 | **known class name** (`classify_spirit`) | `:5085-5091` |
| 6 | `[sandbox]`/`[resources]`/`[output_shape]`/`[posture]` parse | `:5092-5101` |
| 7 | `[epistemic_policy]`/`[scheduling]`/`[lifecycle]`/`[budget]` parse | `:5102-5125` |
| 8 | **model-provenance admission** (`maos_registry::admission::validate_model_provenance`) + FR62 TL event | `:5135-5143` |
| 9 | butler boot-loud self-halting-posture guard | `:5206-5214` |
| 10 | mira boot-loud scalar-port guard | `:5700-5706` |
| 11 | **`security.admit_spirit`** | `:5760-5786` |

**Inside `admit_spirit` (`crates/maos-kernel-core/src/security/mod.rs:245`):**

| gate | line |
|---|---|
| `EClassRequired` | `:278-280` |
| `ESubstrateTooOld` (unparseable / kernel < `min_substrate_version`) | `:295`, `:301` |
| `EAbiTooOld` / `EAbiTooNew` (manifest schema window) | `:317`, `:323` |
| N-1 WARN degradation (non-fatal) | `:332` |
| **T3 image-lock load + verify** → `T3AdmissionFailed` | `:385-402` |
| T3 `image_pin` resolve / `default_entry` | `:403-408` |
| `SandboxTierUnsupported` (T4) | `:410-412` |
| `SandboxTierUnsupported` (T1, fail-closed) | `:414-417` |
| resource-cap strictest-of (manifest vs operator floor) | `:418-426` |

⚠ **DISPROVED — there is NO trust-tier, signature, region-pin or vetting gate in the `maos run` path.**
A case-insensitive scan of `main.rs:4094-6047` for `trust_tier|signature|verify|region|vetted|attestation|keyring`
returns **zero** hits. `trust_tier` inside `admit_spirit` is *read from the policy table with a
`TrustTier::Verified` default* (`security/mod.rs:338-343`, `:372-377`) — an input to `effective_sandbox_tier`,
not a refusal. The real signature/region/vetting gating lives in a **different** `admit_spirit`
(`crates/maos-registry/src/admission.rs:179`) that `maos run` never calls. **AC1 must not claim gates the path
does not have.**

⚠ `enforce_vetted_upgrade_precondition` is **`main.rs:134`** (the epic's `:6452,:6734` are the `smoke-epic-4`
distillation arm and `PanicSpirit` respectively). Its only callers are the `BinPrivateOps` impl
(`main.rs:8840`) and, through it, `operator_door.rs:709` and `:799`. It is **not** a `maos run` gate — the
epic's AC1 says so and is right, but for the wrong reason.

## 2c. The two run paths disagree, and the extraction must pick one

- **standalone** runs model-provenance **before** `load` (`:5135`) and emits `emit_vetter_key_event` on **both**
  grant and rejection (`:5789` / `:5779`).
- **topology** runs model-provenance **after** `admit_spirit` (`:4757`) and emits the vetter event on **grant
  only** (`:4749`).

D-16-6-C picks the standalone order (refuse before allocating a pid). The divergence is a real behavioural
difference between two shipped paths. ⚠ **SUPERSEDED (operator, 2026-09-19 — §14 Q5 = Shape A, §17c): the
divergence is RESOLVED, not recorded.** AC1 re-points **both** file-manifest arms at the one extracted
function, so both adopt the standalone order and topology's `emit_vetter_key_event` gains its **rejection**
row (the `map_err` at `:4746-4748`, today silent). §11 row 2 is no longer a cut line. Shell stays out of scope
by name — it parses a compiled-in manifest and is not a third copy of this path.

## 3. 🔴 `maosctl start` HAS NO OPERATOR-CREATABLE SUBJECT AT HEAD — THIS STORY IS ITS FIRST

`lifecycle_command` (`crates/maos-bin/src/operator_door.rs:430`) gates `start` on the SCB state
(`:449-470`, **D-16-1-J**):

```rust
if verb == "start" {
    let current = self.scb_state(spirit_id);
    if current != Some(ScbLifecycleState::Loaded) {
        return OperatorOutcome::Conflict { code: "invalid_state_transition", … };
    }
}
```

`start` succeeds **only** from `Loaded`. But every root loads, admits and starts in one unbroken run
(`main.rs:5181` → `:5761` → `:5823`; topology `:4610` → `:4729` → `:4769`) **before** the serving loop the
door answers from. So at HEAD, by the time any `maosctl` command can reach the daemon, **no Spirit is ever in
`Loaded`** — and `maosctl start <spirit>` can only ever return 409.

16-1 shipped a verb with no operator-creatable subject. **`maosctl load` is what gives it one.** This is the strongest argument
for D-16-6-A Shape B: a `load` that auto-starts (Shape A) would leave `start` unreachable forever and leave
FR9 at 4/5 verbs while claiming 5/5.

⚠ **Precision — the two places a `Loaded` SCB does briefly exist, and why neither is a subject.** (a) Cold-swap
inserts a `Loaded` successor SCB into the map (`lifecycle/upgrade.rs:194-198`) and calls `start` on the very
next statement (`:199`); the window is a few instructions inside one `async fn`, it is not addressable by
`spirit_id` (the predecessor still holds the name), and cold-swap is **refused over the door** anyway
(`operator_door.rs:632-643`). (b) A failed `admit_spirit` (`main.rs:5788`) or a failed `on_load`
(`scheduler_loop.rs:368`, after the insert at `:348`) strands a `Loaded` SCB — but both return `Err` out of
`main`, so the process is exiting and no door answers.

⚠ **CORRECTED (validation round). A THIRD producer exists, it is durable, and it is inside a SERVING root — so
the bare claim *"no `Loaded` Spirit exists while a door serves"* is FALSE and is withdrawn.** The door is
constructed at `main.rs:3339` and **bound and serving at `:3362`, before the run block at `:4094`.** The Worker
path then runs inside that serving root: `worker_spawn.rs:697` → `supervision.rs:1334` `scheduler.load(...)` →
`:1339` `scheduler.start(pid)`. A failed `start` returns `Err` and **leaves the SCB durably `Loaded` in the
map**, under an id that passes `validate_id` (`worker` / `worker-N`, `supervision.rs:1289-1293`) — exactly the
residual its own comment declares at `:1340-1342`. Even the happy path leaves a `Loaded` window between the two.

**The true claim, and the one AC3 asserts:** there is **no OPERATOR-INITIATED `Loaded` subject** at HEAD. Every
root loads+admits+starts in one unbroken run before serving, and the only durable HEAD producer is a **failed
Worker `start`** — a fault an operator cannot cause on purpose, schedule, or rely on. `maosctl start` has
therefore never had a subject an operator could deliberately create, and `load` is what gives it one.

⚠ AC1 therefore asserts the **three-step operator sequence**, not a single verb:
`maosctl load <manifest>` → `lifecycle_state: Loaded` → `maosctl start <id>` → `Running` → `maosctl unload <id>`
→ gone, with a receipt.

## 4. 🔴 THE ROUTE SHAPE DOES NOT EXIST, AND THE SERIALIZATION KEY CANNOT BE COMPUTED

**There is no collection route.** `fn route` (`crates/maos-control/src/lib.rs:967`, match `segments.as_slice()` `:980`) matches by
path segments. The spirit arms are `["v1","spirits",id,"sandbox"]` (`:1001`), `["v1","spirits",id]` GET-only
(`:1014`) and `["v1","spirits",id,verb]` POST (`:1036`). **`["v1","spirits"]` is not an arm** — it falls to
`_ => 404` (`:1211`). So a new arm is clean, with no ordering conflict against the delicate
`{id}` / `{id}/sandbox` disambiguation the doc comment at `:963-966` warns about.

**The `{id}/{verb}` route cannot carry a load.** Three independent reasons:
1. `validate_id` (`:1443`) permits only `[A-Za-z0-9._-]{1,128}` — **a filesystem path can never be a path
   segment**.
2. `lifecycle_command` returns `spirit_not_loaded` (404) from `resolve_pid` **before doing anything**
   (`operator_door.rs:438-440`) — the guard is exactly inverted for a load.
3. The id does not exist until the manifest is parsed.

**The serialization key cannot be computed before the work.** `OperatorCommand::spirit_key()`
(`maos-control/src/lib.rs:345-364`) is a **total function** and is the D-16-1-R per-Spirit lock key
(`operator_door.rs:1311` `run_command`, lock registry `SpiritCommandLocks` `:162`). A
`Load { manifest }` has no `spirit_id` to return, so it must return `None` — i.e. **it is not serialized**,
exactly like `ForgetMemory` and `ReleaseLegalHold`, the two commands whose unserialized race 16-5 spent AC3
fixing.

**And the kernel's own duplicate guard is not atomic either.** In `load`, `resolve_pid` (`scheduler_loop.rs:268`)
and `spirits.insert` (`:348`) are **two separate `RwLock` acquisitions** separated by `allocate_pid()` and the
entire `admit_spirit` call. Two concurrent loads of the same id both observe `None` and both insert under
different pids. This is masked **only when** `revocation_rules` is `Some` — `admission_guard()`
(`crates/maos-kernel-core/src/revocation/rules.rs:33-35`) is a `tokio::sync::Mutex` held from `:256` to
end-of-`load`. **When it is `None` there is no serialization at all**, and AC2's "typed `409 AlreadyLoaded`"
is a check-then-act.

⇒ **D-16-6-D**: the door handler parses the manifest first, learns the id, and takes
`self.spirit_lock(&spirit_id)` **itself** before calling the extracted function — a correct critical section
even though `spirit_key()` is `None`. `route_budget()` (`:367-375`) gets a `Load` arm returning
`LONG_ROUTE_BUDGET` (30 s, `:227`) beside `Upgrade`/`Uninstall`/`ImportRevocations`/`HotSwapPrecheck`, because
admission does T3 image-lock verification.

⚠ **A new variant lands on TWO surfaces.** `crates/maos-bin/src/shell_host.rs:20,51,171` holds an
`Arc<dyn OperatorCommandPort>` and calls the same `submit_and_wait`. Whatever `load` does over HTTP, the shell
REPL path sees.

⚠ **The manifest path is the operator's, the daemon's CWD is not.** D-16-1-X (`maos-control/src/lib.rs:322-324`)
already ruled this — it is why `ImportRevocations` carries `crl: Vec<u8>` bytes, not a path. The CLI must
canonicalise before sending, using the existing `fn canonical_manifest` (`crates/maos-cli/src/subcommands.rs:93-103`,
missing ⇒ exit 1, non-canonicalisable ⇒ exit 1) exactly as `spirit upgrade` does (`:2291`). ⚠ Note that
`maos run` itself does **not** canonicalise (`main.rs:4152` `PathBuf::from` + `read_to_string` in its own CWD) —
so AC2's canonicalisation is **new behaviour, not HEAD behaviour**, and the extracted function must accept an
already-canonical path from both callers.

## 5. 🔴 THE CLASS SET IS EIGHT COMPILED-IN NAMES — "LOAD A MANIFEST" OVERSTATES THE TREE
<a id="sec-5-class-set"></a>

`fn classify_spirit` (`main.rs:456`, arms `:458-465`, `None` `:466`) is a fixed string match returning `enum LoadedSpiritKind`
(`:444-453`): `Butler, Researcher, Orchestrator, Architect, Reviewer, Mira, Nash, Digest`. `None` otherwise.

- **`rust-inproc` is not a class** — it is a `[class].forms` string.
- **`wasm` is not a class** at all.
- **`[cli_wrapper]` never reaches `classify_spirit`** — it is forked on first (topology `:4238`,
  standalone `:4962`).
- `SpiritSchedulerAdapter::load` is **generic over a compile-time Rust type**
  (`load<T: Spirit + Send + Sync + 'static>`, `scheduler_loop.rs:243`). A manifest cannot name a type; the
  class dispatch is a hand-written `match` monomorphising per arm.

⇒ **A door `load` can only ever admit a class already linked into `maos-bin`.** AC1 says so. The refusal
vector for an unknown class is gate #5 (`main.rs:5085-5091`).

⚠ **The construction switch already exists in a fourth place — but it is INCOMPLETE, and that is a trap.**
`successor_factory` (`main.rs:3123-3198`, the `class.name.as_str()` match at `:3130`) produces
`Arc<dyn AnySpiritObj>`, already `'static` and Arc-captured (`:3110-3122`). It is the class-dispatch half of
the extraction, pre-built — **but it covers SEVEN of the eight classes plus `"smoke-spirit"`, and has NO
`"researcher"` arm** (arms: `orchestrator` `:3131`, `architect` `:3134`, `reviewer` `:3137`, `mira` `:3141`,
`nash` `:3144`, `digest` `:3147`, `smoke-spirit` `:3150`, `butler` `:3163`, then
`unsupported => UpgradeError::SuccessorFactory` `:3186`). Meanwhile `classify_spirit` DOES map `"researcher"`
(`main.rs:459`) and the standalone arm loads one (`:5645`). **Building the extraction naively on
`successor_factory` would silently make `maosctl load` unable to admit a class `maos run` can** — a capability
regression disguised as reuse. Extend it with the missing arm, or dispatch classes separately; do not assume it
is complete, and do not write a fifth copy.

⚠ **`scheduler.load(...)` appears 20 times in `main.rs`**, not "a dozen": shell `:3801`; topology `:4612, :4623,
:4635, :4682, :4698, :4709`; standalone `:5363, :5645, :5669, :5678, :5688, :5725, :5739, :5748`; one-shot smoke
arms `:6655, :6742, :6777, :6830, :6985`. The extraction targets the **three real copies** (shell, topology,
standalone) and leaves the five smoke arms alone.

## 6. 🔴 THE GATE THAT SHOULD BIND THIS STORY IS VACUOUS

`cargo run -q -p xtask -- check-exit-commands --json` at HEAD:

```json
{"passed":true,"epics_in_roster":6,"blocks_read":6,"tokens_resolved":27,
 "owed":[{"token":"eval","owner_story":"18-2-…"},
         {"token":"maos-registry-server","owner_story":"20-1-…"},
         {"token":"maos-spirit","owner_story":"20-1-…"},
         {"token":"install","owner_story":"20-1-…"}],
 "findings":[],"surfaces":{"maos":8,"maosctl":25,"xtask":95,"one_shot_modes":32}}
```

(Abridged — the real payload carries `epic`/`owner_status`/`claimed_in` on every `owed` entry plus a
`surface_notes` array. **Four** owed at HEAD, not three; an earlier draft omitted `maos-spirit`.)

**It owes `16-6` nothing** — and it never will, because **epic-16's hermetic exit block
(`epic-16-…md:27-35`) contains no `load` token.** `grep "load"` over those nine lines returns zero matches.

Epic-16's **confidence rule 1** is *"verb-first (exit verbs exist at HEAD or are AC1 in this epic;
`check-exit-commands`)"* (`:57`). For this story that rule is **vacuous as the block stands**: the epic's own
hermetic exit does not exercise the capability 16-6 ships, and the gate stays green no matter what happens
here.

⇒ **D-16-6-F: add exit line 5b, and its provenance bullet, in the creation commit.** (Positioned after line 5's
root and **before** line 6's `kill "$RUN_PID"` — a line numbered 8 after line 7's `maos purge` would have had
no daemon to talk to, and renumbering would have invalidated every provenance bullet.)

✅ **DONE AND MEASURED IN THE CREATION COMMIT.** With the line and its bullet in place and the binaries built,
`./target/debug/xtask check-exit-commands --json` reports:

```json
{"passed":true,"tokens_resolved":31,"findings":[],
 "owed":[{"token":"load","epic":16,"owner_story":"16-6-maosctl-load-over-the-door",
          "owner_status":"ready-for-dev","claimed_in":"story file 16-6-…md"}, …]}
```

Resolved tokens **27 → 31** (the line's `spirit`, `start`, `unload`, `inspect` all resolve against real
`maosctl --help`), zero findings, and `load` is now **owed to this key by name** — so rule 1 binds, and the
gate will flip `load` from `owed` to `resolved` the moment the verb ships. `bash -n` over the whole 8-line
block is clean, and the line carries no `<placeholder>` so `PlaceholderHazard`
(`check_exit_commands.rs:849-877`) is not in play.
⚠ **The `maos`/`maosctl` binaries must be BUILT before running this gate** — it resolves verbs from live
`--help`, so an unbuilt tree reports `binary 'maos' does not exist at HEAD` and 12 spurious `UnresolvedVerb`
findings across four epics. That is a tooling artifact, not a red.
⚠ This is the **second** exit-block COMMAND edit in the epic's history (16-4's `--yes` was the first). It
requires a clean `bash -n` dry-parse, and any `<placeholder>` must be **quoted** or `PlaceholderHazard`
(`check_exit_commands.rs:849-877`) reds it — the F15 defect that already hit this block's line 3.

⚠ **And do not copy 16-4's CI precedent.** `grep -n "maos_uninstall_16_4\|purge" .github/workflows/discipline.yml`
returns **nothing**: 16-4's test has no named job and runs only inside the 45-minute `workspace-test-suite`
(`:3559`) — the exact "one red line in a 45-minute run" failure the `one-daemon-one-door` comment
(`:3640-3642`) exists to prevent, and it contradicts `epic-16-…md:25`, which asserts exit lines 5–7 run under
their own leg. 16-6 gets a **named job** (AC6).

## 10. SIZING AND BUDGET (rule 6, ×1.3; interval arithmetic)

| crate | headroom @ HEAD | change | raw | ×1.3 upper |
|---|---|---|---|---|
| `maos-kernel-core` | **0 — FLAG-Winston only** | one `(Loaded, Unloaded)` arm in `is_transition_allowed` (`control_block.rs:448-453`) **+1**, PLUS rewriting `fn loaded_to_unloaded_rejected` (`:567-573`) in the shape of its five siblings (`loaded_to_running_allowed` `:515-525`) — inline `#[cfg(test)]`, **which `kloc-check` charges** | **+3…+6** | **+8** |
| `maos-bin` | **0 — measured raise required** | the extracted `load → admit → start` function + its rollback (new module `admission.rs`), the `Load` handler in `operator_door.rs`, the `BinPrivateOps::load_spirit` method + `main.rs:8780` impl, **minus** the three collapsed copies (shell `:3740-3930`, topology `:4600-4792`, standalone `:5083-5828`) | **+180…+420** | **+546 (measured raise)** |
| `maos-control` | +527 | `OperatorCommand::Load` variant + `spirit_key`/`route_budget` arms + the `["v1","spirits"]` POST arm + body parse | +45…+90 | **+117** |
| `maos-cli` | +776 | `Load(LoadArgs)` clap variant + dispatch + `canonical_manifest` reuse + D-16-1-P mapping | +40…+85 | **+111** |
| `maos-journey-test` | +478 | exit-line-8 assertions if AC6's scene leg lands there | +0…+40 | **+52** |
| `maos-domain` | **0** | ⚠ **none intended.** If a typed refusal leaks into `maos-domain` — which is what happened to 16-5 (`kloc.toml:395`) — it is a **second uncosted raise**. Reuse `OperatorOutcome` (`maos-control/src/lib.rs:384-399`); do not mint a domain error | 0 | 0 |
| `xtask` | **0** | pin/ceiling artifacts only (TOML + `tests/`), which `kloc-check` does not charge | 0 | 0 |

**Two crates cross:** `maos-kernel-core` (FLAG-Winston only — the easing explicitly does **not** apply) and
`maos-bin` (measured raise under the RECOVERY-LANE CEILING RULE). **Code first, `cargo fmt --all`,
`kloc-check --json`, then the `kloc.toml:477` row carries the bare token `16-6` followed by a non-digit
non-dash character, the measured figure and the driver, same commit.** ⚠ The tokenization rule is enforced by
`fn story_keys` in `xtask/tests/d11_xtask_ceiling_ratchet.rs:164-179`, **not** by `kloc.toml:49-95` where a dev
will look for it.

⚠ **CORRECTED (validation round): they are NOT three verbatim copies, and an earlier draft biased this forecast
the wrong way against a ZERO-headroom crate.** Only **topology** (`:4610→4729→4769`) and **standalone**
(`:5181→5761→5823`) share a shape, and even they diverge — §2c records the provenance ordering AND the
vetter-event asymmetry. The **shell** triple (`:3800→3813→3855`) is a different animal: it parses a
**compiled-in** manifest string (`maos_spirit_hello::MANIFEST_TOML`, `main.rs:3741`) with **no file read, no
`classify_spirit`, no model-provenance gate and no `[cli_wrapper]` fork**, and hard-codes `HelloSpirit` —
`hello-spirit` is not one of the eight `LoadedSpiritKind` names at all.
**Consequences the dev must hold:** (a) AC1 and T2 re-point **the standalone arm only**, so the realistic
collapse is **one call site, not three** — do not price a three-copy saving; (b) `maos-bin` is at **ZERO
headroom**, so a forecast biased optimistic is the expensive direction to be wrong in; (c) treat **+420** as the
working number and **measure at T0 and again after the extraction, before asking for the raise.**

⚠ **The aggregate alarm is already FIRING** (167189 vs 158608, `kloc.toml:657`); hardfail is 170884 (`:658`),
leaving **+3695**, and this story's ×1.3 upper bound totals ≈ **+828**. State the new aggregate; do not silence
the alarm.
⚠ **CORRECTED (round 2, §17 R10/R15): the kernel row is not one line in one file.** The posture clamp lands in
**two further kernel-core files** — `capability/cap_policy/mod.rs` (the `Option<Posture>` field) and
`security/mod.rs` (two `min()`s, not one) — so the kernel change is **`+10…+16`** raw (`×1.3` upper **+21**),
THREE per-file digests move, and the re-pin is **twelve edits**. If §14 **Q4** is granted, a fourth file
(`scheduler/scheduler_loop.rs`) joins — and **§14 Q4 = Shape A was ratified, so it does join**: the kernel
change is **`+16…+26`** raw, **FOUR** per-file digests move, and the re-pin is **thirteen edits**. ⚠ And §14
**Q5 = Shape A** widens the `maos-bin` row too: the collapse is **two** call sites (standalone **and**
topology), so `+420` is a **floor**, not a ceiling. Re-measure at T0; do not price from this table.
⚠ **ROUND 3 (§18) MOVES TWO MORE ROWS.** The kernel row grows again: the `scheduler_loop.rs` edit is **three
coordinated hunks**, not one — cleanup ahead of the hook (R21 mechanism (x)), the
`hook_name == "on_unload"` guard on `check_hook_outcome`'s `handle_crash` spawn, and the inline
`task.orphaned`/`enforce_disposition` (R22) — so treat `+26` as a **floor** and measure. And **`maos-control`
and `maos-bin` each gain an unbudgeted edit** for R23's sixth key on `GET /v1/spirits/{id}`: `SpiritStatusRow`
(`maos-control/src/lib.rs:418-428`), the JSON literal (`:1024-1029`) and `spirit_status`
(`operator_door.rs:1462-1468`). `maos-control` has `+527` of headroom so it absorbs it; **`maos-bin` does not
and this is the third thing widening its raise** (topology re-point, `BinPrivateOps` widening, the sixth key).
✅ Measured and not a problem: `maos-kernel-core` headroom is **exactly ZERO**, the raise is required, and its
only tree-bound gate is `assert!(actual <= RATIFIED_AT_EASING)` (`recovery_lane_ceiling_rule.rs:275`) with a
documented same-commit escape already exercised by 16-3/16-4 and 16-5.

**Dependencies:** no new crate, no new workspace member (`check-workspace-count` PASSED 55/55 — unaffected),
no lockfile entry, **no new env var** (`check-env-contract` PASS, 94 vars — and its scope is `maos-bin/src`
only, so a `maos-cli` read would be invisible to it anyway; do not add one).
⚠ **One new file under `crates/maos-bin/src`** (`admission.rs`) ⇒ **`SCANNED_SOURCE_FILES` 23 → 24**
(`crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs:882`, directory-listing equality assert `:981-985`;
`ls crates/maos-bin/src/*.rs | wc -l` = 23 at HEAD). ⚠ **The epic's cite is stale for the third consecutive
story** — `epic-16-…md:53` says "22 at `:881`" with the assert at "`:967-981`".
⚠ **`verb_table_15_3.rs` is untouched** provided `load` is a **`maosctl` clap subcommand** and not a `maos`
verb and not a `MAOS_ONE_SHOT` mode: `VERBS.len() == 8` (`:137`), the two frozen per-build surface arrays
(`:161-170`), `MAOS_ONE_SHOT_MODES.len() == 32` (`:207-211`) and the frozen `--help` test (`:243`) all stay.
`maosctl --help` goes 25 → 26 surfaces; **no test pins that number.**

## 15. Validation round 2026-09-18 — six re-rulings against this story's own headlines

An independent validator in a fresh context re-derived every citation and attacked the three §-headlines.
**Three of them were FALSE.** Recorded here rather than silently patched, because a preflight that quietly
corrects itself teaches the next story nothing.

| # | This story claimed | Measured | Applied at |
|---|---|---|---|
| **V1** | *"Zero test coverage of the exact case … the falsifier AC2 needs has never existed"* | **FALSE.** `fn loaded_to_unloaded_rejected` (`control_block.rs:567-573`) asserts the CURRENT refusal, so AC2 **reds `cargo test -p maos-kernel-core`**. It is inline `#[cfg(test)]` in a ZERO-headroom crate ⇒ charged by `kloc-check` | §1; frontmatter artifact (8); AC5; §10 kernel row `+1`→`+3…+6`; T3; obligation (w) |
| **V2** | *"There is no force-unload, no purge, no admin escape hatch anywhere in the tree"* and *"the SCB LEAKS PERMANENTLY"* | **FALSE — true of `SpiritSchedulerAdapter::unload` ONLY.** `handle_crash` discards the transition `Err` (`crash_detector.rs:120-124`) and still revokes, receipts and **`map.remove`s** (`:127`/`:130`/`:191-194`); cold-swap's rollback removes outright (`upgrade.rs:200-203`); and `scbs()` is `pub` (`scheduler_loop.rs:148`), so maos-bin could do it in three lines at zero kernel-Δ | §1 scope table; **D-16-6-A Shape D re-argued on three grounds instead of on impossibility**; §11 row 3; obligation (m2) |
| **V3** | *"`maosctl start` has no reachable subject at HEAD"* | **FALSE.** The door binds at `main.rs:3362`, **before** the run block at `:4094`, and a failed Worker `start` (`supervision.rs:1334`→`:1339`) leaves a **durable** `Loaded` SCB under a `validate_id`-passing id inside a serving root. The surviving claim is **operator-creatable**, not *existent* | §3 heading and body; AC3 proven-red; obligation (i); **and the epic's line-5b provenance bullet (d), which had already shipped the false form** |
| **V4** | *"`successor_factory` … for all 8 classes"* | **FALSE — SEVEN + `smoke-spirit`; there is no `"researcher"` arm** (`main.rs:3123-3198`, match `:3130`), while `classify_spirit` maps it (`:459`) and the standalone arm loads one (`:5645`). Naive reuse would make `maosctl load` unable to admit a class `maos run` can | §5; Dev notes; D-16-6-B; T2 |
| **V5** | *"the re-pin touches SEVEN artifacts"* | **Undercounted — TEN edits across EIGHT artifacts.** `kernel_pin_content_hash_16_0.rs` holds **four** `24628` literals (`:802`, `:820`, `:829`, `:840`), and V1 adds the test. ⚠ This is the same undercount the story accuses 16-5's *"five"* of | frontmatter `kernel_grant`; AC5; T3 |
| **V6** | *"three verbatim copies"* collapse; *"the net could approach zero or go negative"* | **Overstated, in the expensive direction against a ZERO-headroom crate.** The shell triple parses a **compiled-in** manifest (`main.rs:3741`) with no file read, no `classify_spirit`, no provenance gate; and AC1/T2 re-point **the standalone arm only** — one call site, not three | §10 `maos-bin` row and its note |

Smaller corrections applied in the same pass: `check-exit-commands` owes **4** at HEAD, not 3 (`maos-spirit`
was omitted); **`Cargo.toml:60` is `default-members = []`**, so the gate needs `cargo build -p maos-bin -p
maos-cli --bins` first or it reports ~18 spurious findings; `scheduler.load` sites are **20**, not 21; AC6 now
describes the six-verb line that actually landed; and fifteen `file:line` cites were re-pinned
(`LONG_ROUTE_BUDGET` `:227`, route match `:980`, bearer `:901-910`, 413 `:765-770`, `route_budget` `:367-375`,
`OperatorOutcome` `:384-399`, CAS `:1382-1390`, `BinPrivateOpsImpl` `:3330-3338` as **arg 18 of 19**,
`current_state` `:1461`, cold-swap refusal `:632-643`, `Barrier` `:1044`, `successor_factory` `:3123-3198`).

## 16a. R3 — the escalation chain, measured at the table

The story inherits the epic's AC3 claim that *"a bearer holder can already exec `maos run`, so `load` adds no
privilege."* **Measured, that is FALSE**, and **16-1's own fast-fail is what makes it false**: a second root
binding the same endpoint dies `EndpointInUse` + exit 78 (`main.rs:3375-3391`), so `maosctl load` is **the only
runtime path a manifest has into the live root.**

Five steps, all shipped verbs, no new code, inside the eight-class bound of §5:

1. Read `<home>/control.json` — a bare Worker runs as the operator uid (`README.md:282-284`, no `env_clear` at
   `runtime.rs:453`, and `credential_posture_2c.rs:299` forbids adding one until 17-1).
2. Write a manifest naming a **compiled-in** class (`reviewer`) with `[posture] allowed_max = autonomous-with-halt`.
3. `POST /v1/spirits`.
4. `admit_spirit` stores the ceiling **VERBATIM** — `security/mod.rs:359-366`,
   `allowed_max: posture_section.allowed_max`, **no clamp of any kind**.
5. `maosctl posture <id> --shift autonomous-with-halt` — 16-1's own verb — passes its ceiling check at
   `cap_policy/mod.rs:255` (`if new_posture > state.allowed_max`), because the ceiling is *the attacker's own
   number*.

**The asymmetry is the proof.** `OperatorPolicyConfig` (`crates/maos-kernel-core/src/capability/cap_policy/mod.rs:22-43`)
carries **four** operator floors: `global_sandbox_floor` (*"applied to ALL Spirits regardless of manifest"*,
`:26`), `spirit_tier_floor` (`:24`), `resource_cap_floor` (`:30`), and two subtract-only capability deny sets
(`:38`, `:42`) whose comment states the design principle outright — *"without letting PDP permits raise the
manifest ceiling."* Resource caps are resolved strictest-of at `security/mod.rs:418-426`.

**Posture is the fifth axis of the leash and the only one with no floor.**

⇒ **The clamp ships in 16-6, as a new gate under AC1** — not a new AC (John), because AC1 already promises
*"every gate `maos run` applies, applies identically", and this is a gate that exists on **neither** path.
A missing gate cannot have a refusal vector written for it (Murat). Shape: one `Option<Posture>` field beside
the four existing floors + a `min()` at `security/mod.rs:361`. **It is a consistency repair, not a
governance subsystem** — Splinter withdrew his over-building objection on that basis.
⚠ **The admission record must carry BOTH the requested and the effective ceiling** (Sally): a clamped
admission that looks identical to an unclamped one is the claim-standing-in-for-a-control shape again.
⚠ **Budget:** the FLAG-Winston grant grows **+1 → ~+8** kernel lines. Same blast radius, same re-pin, same ten
edits — but it is **not the grant §14 Q1 originally described**, and Q1 is re-scoped accordingly.

**Falsifier (red at HEAD in one run):** operator floor `assistive`; manifest declares
`allowed_max = autonomous-with-halt`; `POST /v1/spirits` ⇒ stored ceiling must be **`assistive`**; then
`maosctl posture --shift autonomous-with-halt` ⇒ **`AboveCeiling`**. At HEAD the shift **succeeds**.

## 17. Preflight round 2 — 2026-09-19 — fourteen findings; §11's cut line 1 is withdrawn and three of AC1's own refusal vectors are unwritable

Convened on the same standing criterion: **spec fidelity + long-term correctness**. Six independent scouts read
the tree before anyone spoke. **Round 1's own R3 fix is the wrong line, §11 cut line 1 has a `load` trigger
after all, three of AC1's eleven refusal vectors cannot be written at HEAD, and AC4's compile-safety claim is
false.** Recorded here rather than silently patched — the §15 discipline.

| # | Finding | Measured | Applied at |
|---|---|---|---|
| **R7** | ⛔ **§11 cut line 1 has a `load` trigger — the arm makes the `on_unload` receipt gap reachable from the exact verb pair this story ships** | `SpiritSchedulerAdapter::unload` (`scheduler_loop.rs:471-523`) flips the state at `:483` **before** the fallible `check_hook_outcome?` at `:502`; receipt/revoke/drain/remove are all after it. Nothing between `:483` and `:502` is conditioned on the prior state, and `kernel_invocation_allowed` (`maos-spirit-abi/src/lifecycle.rs:510-515`) returns **`true` when `enabled_hooks` is empty**, so a default-manifest door load **does** dispatch `on_unload`. A `Panicked`/`BudgetExceeded` outcome (`hook_dispatch.rs:573-588`) leaves an `Unloaded` SCB still in the map; the operator's retry hits the `:479-481` early return and gets **`Ok(())` with no receipt**. §11 row 1's *"has no `load` trigger"* is **FALSE** | §11 row 1 **withdrawn**; AC2 new vector (g); §14 **Q4**; obligation (aa) |
| **R8** | §2's sandbox-downgrade argument for D-16-6-B is **FALSE** | The single `SpiritSchedulerAdapter::new` in maos-bin passes **`security_manager: None`** (`main.rs:2793`, comment *"constructed per-one-shot arm at v0.3-β"*), so the kernel's internal `admit_spirit` (`scheduler_loop.rs:276-322`) **never runs in the daemon** — it takes the `else` branch at `:323-334` and emits a bare `lifecycle.admit` frame. A door `load` calling the kernel directly would therefore admit **not at T2 but not at all**. D-16-6-B's ruling survives; its stated reason does not. (There is also no double-admission at HEAD, so §2's worry about it is moot) | §2 ⚠; D-16-6-B rejected-column; obligation (g) **re-aimed** |
| **R9** | ⛔ **THREE of AC1's eleven refusal vectors cannot be written — the manifest's `[sandbox] tier` is parsed and then discarded** | `effective_sandbox_tier` (`cap_policy/mod.rs:201-224`) seeds the manifest leg from **`manifest_scopes[pid].declared_tier`**, which is **absent on first admission** ⇒ `SandboxTier::DEFAULT_FLOOR`; `admit_spirit` then writes `declared_tier: effective` back (`security/mod.rs:349-355`). Its `manifest: &SandboxConfig` parameter is used **only** for `image_pin` (`:403`). A workspace grep confirms **no reader** of `SandboxConfig.tier` on the admission path. With `global_sandbox_floor` defaulting to `T2` and `spirit_tier_floor`/`trust_tier_floor` empty in production, `effective` is **always T2** for the eight compiled-in classes ⇒ `SandboxTierUnsupported` for **T4** (`:410-412`) and **T1** (`:414-417`) and **`T3AdmissionFailed`** (`:385-402`) are all three **unreachable from a manifest**. This is Vex's 12.1-R2 *accept-and-ignore* shape: a declared control that is parsed and unenforced | **AC1** vector list corrected; D-16-6-E `route_budget` justification corrected; §11 new row 7; obligation (ab) |
| **R10** | ⛔ **§16a's clamp is the WRONG LINE — `allowed_max`-only inverts the ceiling invariant** | `security/mod.rs:357-366` stores **both** `current: posture_section.default` **and** `allowed_max: posture_section.allowed_max` verbatim, and `RawPostureSection::validate` (`maos-manifest/src/manifest.rs:687-701`) enforces only `allowed_max >= default`. A `min()` at `:361` alone yields, for orchestrator (`default = autonomous-with-halt`) under a `assistive` floor, `current=AutonomousWithHalt, allowed_max=Assistive` — the Spirit **runs above its own ceiling** and the clamp clamps nothing. **Both fields must be `min()`-ed.** Good news, measured: `Posture` (`manifest.rs:643-653`) derives `Ord` least-privilege-first (`Cautious < Assistive < AutonomousWithHalt < Autonomous`) so `min()` is a real clamp; and `OperatorPolicyConfig` derives `Default` while `Posture` derives **no** `Default`, forcing `Option<Posture>` ⇒ `None` ⇒ **`maos run` behaviour provably unchanged** | **AC1** R3 clause; §16a; §14 Q1; obligation (ac) |
| **R11** | **AC4's compile-safety claim is FALSE — `route_budget` is not total, and there are THREE exhaustive matches, not four** | `route_budget` (`maos-control/src/lib.rs:367-381`) ends `_ => DEFAULT_ROUTE_BUDGET`. The exhaustive matches are **`spirit_key` (`:345-364`), `DoorInner::dispatch` (`operator_door.rs:329-420`), `command_verb` (`operator_door.rs:1526-1544`)** — the enum derives only `Debug, Clone, PartialEq, Eq` (`:270`), there is no `Display`/`Serialize`/metrics impl, and `spirit_command` (`:1222`) matches `&str` not the enum. So `Load` **silently inherits `DEFAULT_ROUTE_BUDGET` = 10 s** (`:227`) and the compiler never says so. ⚠ And R9 removes the story's stated reason for `LONG_ROUTE_BUDGET` (*"admission does T3 image-lock verification"* — it never does); the surviving reason is the `on_load`/`on_start` hook budgets inside the triple | **AC4**; D-16-6-E; obligation (l) **promoted from optional to binding** |
| **R12** | **D-16-6-D's lock lands after the point of no return, and its wait is unbounded** | The per-Spirit lock is taken in the **port**, `run_command` (`operator_door.rs:1323-1345`): `lock_wait = deadline − 250 ms`, `command.spirit_key()`, `timeout(lock_wait, lock.lock())` — **then** the `QUEUED → STARTED` CAS (`:1349-1357`), **then** `dispatch` (`:1370`). Because `Load`'s `spirit_key()` is `None`, no port lock is taken and D-16-6-D's handler lock runs *inside* `dispatch`, i.e. **after** the CAS: (a) a contended `Load` can never lose the withdraw CAS, so it answers **`handler_still_running`** where every other verb answers `spirit_busy`; (b) a bare `.lock().await` is **not bounded by `lock_wait`**, on a task deliberately never cancelled (`:1407`). ✅ Separately CONFIRMED: the lock is port-level, so **`shell_host.rs:171`'s path is covered** | **D-16-6-D** amended; **AC4**; obligation (ad) |
| **R13** | **An unlisted prerequisite: the door seam cannot admit or register a pid** | `BinPrivateOpsImpl` (`main.rs:8768-8776`) holds `memory`, `capability`, `transparency_log`, `audit_db_path`, `memory_db_path`, `shared_journal`, `upgrade_orchestrator` — **no `security`, no `pid_by_spirit_id`** — and `trait BinPrivateOps` (`operator_door.rs:54-107`) declares five methods, none able to load or admit. The standalone arm inserts into `pid_by_spirit_id` at `main.rs:5819`. So `BinPrivateOps`/`BinPrivateOpsImpl` must be **widened with `security` and `pid_by_spirit_id`**, which no AC or task says | **AC1**; **T4**; Dev notes |
| **R14** | **AC1's *"No second admission implementation"* is FALSE as T2 is scoped — and topology IS extractable** | After re-pointing the standalone arm only, two independent implementations survive: **shell** (`:3740-3856` — parses the compiled-in `maos_spirit_hello::MANIFEST_TOML` `:3741`, hard-codes `HelloSpirit` and the id `"hello-spirit"`, never calls `classify_spirit`, no provenance gate, no `pid_by_spirit_id` insert) and **topology** (`:4600-4790` — provenance **after** admit `:4757` vs standalone's **before** `:5135`, and `emit_vetter_key_event` on **grant only** `:4749`, so its `map_err` at `:4746-4748` leaves **no vetter TL row on rejection**, unlike shell `:3832` and standalone `:5779`). Extractability measured: the topology loop (`:4589`) is **one spirit per iteration** with every section locally owned (`:4551-4595`) — no `Vec<Spirit>`, no lifetime blocker; only Mira's `ButlerOrchestratorAdapter` (`:4664-4696`) needs the pid returned. **The blocker is effort, not types** | §14 **Q5**; **AC1** claim restated; §11 row 2 re-aimed |
| **R15** | **AC5 is UNDERSTATED and the edit count is 12, not 10** | R3 edits `capability/cap_policy/mod.rs` **and** `security/mod.rs`, whose digests are separate rows at `kernel-core-baseline.toml:548` and `:614`, beside `scheduler/control_block.rs` at `:601` ⇒ **THREE digests change by name**, not one; a dev satisfying AC5 literally passes while two kernel files silently change — the D-16-0-F failure mode the per-file map exists to catch. `PINNED_ENTRIES` stays **98**. ⚠ **NEW HOLE nobody counted:** every prior re-pin **prepended** a multi-line `# HISTORY:` block above `src_lines` (16-5's is `:496-504`, nine lines); doing that again pushes `src_lines` off 505 and reds `kernel_pin_content_hash_16_0.rs:842-851`. The 16-6 HISTORY row must be **line-count-neutral** | **AC5**; frontmatter `kernel_grant`; **T3**; obligation (ae) |
| **R16** | **The `spirits`-map write census is SIX, not three — obligation (m2) is wrong** | Writes: `scheduler_loop.rs:348` insert, `:520` remove; `upgrade.rs:198` insert, `:203` remove, **`:208` re-insert**; **`crash_detector.rs:193` `map.remove`**. `scbs()` is `pub` (`scheduler_loop.rs:148`) with ~30 external call sites across `main.rs`, `operator_door.rs`, `supervision.rs` and tests. And **`upgrade.rs:164` (cold-swap unloading the *predecessor*) is a caller the story never lists** — it changes behaviour when the predecessor was loaded-but-never-started | obligation **(m2) corrected**; §11 row 3 |
| **R17** | **The arm turns two production comments into lies and flips `report.failed` from over- to under-reporting** | `supervision.rs:1339-1342` (*"there is no `Loaded → Unloaded` transition, so it cannot be unloaded and gets no receipt"*) and `shell_host.rs:241-247` (*"this unload ALWAYS fails"*) both become **false** the moment the arm lands, as does the test comment at `shell_host_16_2.rs:545-547`. In `unload_all_loaded` (`supervision.rs:139-167`) a `Loaded` pid stops landing in `report.failed` — correct — but combined with R7 the **real** hook-failure now also disappears from it, and `finish_shell_session`'s `had_failures` short-circuit (`shell_host.rs:248-253`) stops skipping the admission-failure path. Mary's *read the build, not the prose* rule, applied to comments **this story creates** | **T3**; obligation (af) |
| **R18** | R2 is additive-safe **only as a new sibling key** | `GET /v1/daemon` (`maos-control/src/lib.rs:981-994`) returns `spirit_ids` as a bare `Vec<String>` (`DaemonStatusRow:436`), produced sorted at `operator_door.rs:1469-1476`. The only value assertion in the tree is `assert_eq!(daemon["spirit_ids"][0], "butler")` (`crates/maos-control/tests/post_surface_16_1.rs:1238`); `maos-cli` has **zero** consumers. So a new sibling key is safe, **reshaping `spirit_ids` into objects is not**. ⚠ `DaemonStatusRow` is **not `#[non_exhaustive]`** (`:430-439`) ⇒ a new field is a compile break at three struct literals (`operator_door.rs:1472`, `post_surface_16_1.rs:294`, `submit_and_wait_16_2.rs:49`) — compiler-caught, three sites | **AC1** R2 clause; obligation (y) sharpened |
| **R19** | `admission.rs` lands **inside an existing negative** | The `SCANNED_SOURCE_FILES` roster (`cohort_daemon_smoke_13_5c.rs:882`, **23 at HEAD, exact**) feeds the `manifest_scopes` negative at `:992-995`, and `admission.rs` is **not** whitelisted the way `enterprise_pdp_runtime.rs` is ⇒ **the extracted `load → admit → start` function must never touch `manifest_scopes`**. The directory-listing equality assert is at **`:971-985`** (the story says `:981-985`) | **AC6**; **T2**; Project Structure Notes |
| **R20** | The `successor_factory` researcher gap is wider than V4 said | A `researcher` admitted over the door can also never be **hot-upgraded** — it dies in `unsupported => UpgradeError::SuccessorFactory` (`main.rs:3186`). The round-trip test at `main.rs:13690-13718` asserts only `classify_spirit` totality, so **nothing guards factory/classify parity**. A dev note is not a control | **T2**; obligation (x) extended to a **parity test** |

## 18. Preflight round 3 — 2026-09-19 — "is it truly ready?" — FOUR DAY-ONE BLOCKERS, two of them in round 2's own fix

Asked whether the story was genuinely ready, the room did what it did at 12.1 round 3 and 13.2 round 2: it ran
four scouts **against its own ratified resolution** instead of reaffirming it. **It was not ready.** The
standing lesson holds a third time on a third story: *readiness is not a claim you make, it is a graph you draw
and a build you run.*

| # | Blocker | Measured | Forced correction |
|---|---|---|---|
| **R21** | ⛔ **Q4 Shape A offered TWO mechanisms and one of them is BROKEN. Mechanism (y) — "demote the `?` at `scheduler_loop.rs:502`" — DOUBLE-CLEANS.** | `check_hook_outcome`'s `Panicked` arm (`scheduler_loop.rs:573-595`) `tokio::spawn`s `CrashDetector::handle_crash`, which performs a **full parallel teardown** of the same pid: transition `:120-124`, `revoke_all_for_pid` `:127`, `terminate_spirit(UnplannedCrash)` `:130`, `task.orphaned` `:163-170`, `enforce_disposition`, journal `Crash` + `map.remove` `:191-194`. Today the `?` is the only thing keeping the two teardowns apart. Demote it and on `Panicked` both run concurrently ⇒ either (i) `unload` wins and the FR50 disposition/orphan frames are lost **non-deterministically**, or (ii) `handle_crash` wins and you get **TWO** receipt pairs for one termination — one `planned_unload`, one `unplanned_crash` — a duplicate revoke, and a `Crash` journal row for a *planned* unload. ⚠ `Ok(Err(join_err))` also maps a **cancelled** hook to `Panicked` (`hook_dispatch.rs:657-664`), so the race is not panic-only | **Mechanism (y) is REFUSED. AC2(g) pins mechanism (x)** — cleanup ahead of the hook — **plus an explicit guard: `check_hook_outcome`'s `Panicked` arm must NOT spawn `handle_crash` when `hook_name == "on_unload"`**, because `unload` already owns revoke + `terminate_spirit` + `map.remove`. ⚠ The weaker guard *"spawn only if the pid is still in the map"* **does not work** — at `:502` the pid IS still in the map, so the race persists. This makes the `scheduler_loop.rs` edit larger than Q4 priced |
| **R22** | ⛔ **`terminate_spirit` is NOT idempotent — which is what turns R21's race into a durable audit defect rather than a benign retry.** And mechanism (x) has two costs nobody priced | `HaltId::new(format!("term-{spirit_id}-{spirit_pid}-{timestamp_ns}"))` (`halt/termination.rs:44-45`) takes `timestamp_ns` from `SystemTime::now()` (`:34-37`); `boot_nonce` reaches only the receipt **body** (`:65`, `:101`), never the id. Two calls mint two different `HaltId`s and write **four** `EpistemicHalt` TL frames (`:53-60`, `:74-81`) with mismatched `kind_str`. No dedup exists. ⚠ **(x)-a:** with `spirits.remove` moved ahead, a panicking `on_unload`'s spawned `handle_crash` hits its Step-1 lookup (`crash_detector.rs:78-84`) and returns `NotLoaded`, swallowed by the `let _ =` at `scheduler_loop.rs:581` ⇒ **`task.orphaned` and `enforce_disposition` are silently lost**, which today still fire. ⚠ **(x)-b:** `terminate_spirit(PlannedUnload)` writes `HaltState::Terminated` (`termination.rs:63`, `:99`) **before the hook ever signals the child** (`WorkerSpirit::on_unload` → `begin_stop` → `execute`'s `Signal` arm → `strategy.signal_kill`, `supervision.rs:1670-1675`, `:700-707`, `:740-800`) ⇒ a receipt attesting termination of a **live** process | **AC2(g) must additionally (a) perform `task.orphaned` + `enforce_disposition` INLINE in the `on_unload`-failure path, since (x) takes them away from `handle_crash`; and (b) resolve the receipt-ordering fork — see §18a.** ✅ Otherwise (x) is mechanically SAFE: the hook ctx is `Ctx::for_rust_inproc_hook(CapabilityHandle(0), MailboxHandle(0))` built inside the `spawn_blocking` closure (`hook_dispatch.rs:594-602`) with only `spirit_pid` and the `spirits` map, so **nothing a hook can reach is destroyed by an early revoke/drain/remove**, and `fire_payload_hook` dispatches from the `&scb` Arc it was handed (`:580-601`), never from the map. ⚠ Callback to 13.1: `HookDispatcher::build_kernel_ctx` (`hook_dispatch.rs:146-177`) — which *would* wire capability/halt/TL — **has exactly one occurrence in the crate: its own definition. It is dead.** |
| **R23** | ⛔ **The posture clamp REDS A SHIPPED TEST, and the story's own "requested + effective ceiling" requirement has NOWHERE TO LIVE.** | `one_daemon_one_door_16_1.rs:390-403` reads `root.field("butler","posture")`, shifts butler to `cautious`, then `assert_ne!(before, after, "the shift must MOVE the posture the policy table serves")`. Butler's manifest is `default = "assistive"` (`spirits/butler/manifest.toml:43`), so `before == "Assistive"` today; under a `Cautious` floor `admit_spirit` stores `min(Cautious, Assistive) = Cautious` ⇒ `before == after` ⇒ **`assert_ne!` fails in this story's own end-to-end door fixture.** Separately the clamp is a **SILENT LOWERING**: `security/mod.rs:357-366` gains no error and no warn, and the Load journal row at `:437-444` is a `LifecycleEntry` (`i10.rs:121-129`) with **no posture field**. The only posture surface anywhere is `GET /v1/spirits/{id}`'s five fields (`maos-control/src/lib.rs:1024-1029`), where `posture` is `format!("{:?}", state.current)` (`operator_door.rs:1452-1456`) — the **requested** current and **both** ceilings are unobservable; `maosctl spirit inspect` is sandbox-only (`maos-cli/src/cli.rs:859-864`) | **(a)** The floor must be **null in every fixture that reuses the 16-1 daemon**, and the clamp's own vectors get their own root — stated, not assumed. **(b)** §16a's *"the admission record must carry BOTH the requested and the effective ceiling"* requires a **SIXTH key** on `GET /v1/spirits/{id}` plus a posture field on the admission record: `SpiritStatusRow` (`maos-control/src/lib.rs:418-428`), the JSON literal (`:1024-1029`) and `spirit_status`'s construction (`operator_door.rs:1462-1468`). ✅ Additive-safe — no `as_object().len()` pin on this route — but **round 2 budgeted none of it, and without it the AC is UNVERIFIABLE.** ✅ CLEAR, separately: the butler/mira boot-loud guards are **posture-independent** — `needs_port` = `requires_epistemic_halt_port` (`main.rs:564-580`) reads only `epistemic_policy.rules`, so no clamp can flip either guard; `posture_section.allowed_max` appears only inside butler's error *format string* (`:5208-5215`) |
| **R24** | ⛔ **The CI job AC6 tells the dev to copy VERBATIM never builds `maosctl` — a copied 16-6 job panics on a cold runner.** | `one-daemon-one-door` (`.github/workflows/discipline.yml:3643-3655`) has exactly one build step: `cargo build --locked -p worker --bin worker-cli-fixture`. But `one_daemon_one_door_16_1.rs:54-68` resolves `maosctl` beside `CARGO_BIN_EXE_maos` and **hard-asserts `path.is_file()`** with the message *"run `cargo build -p maos-cli` first"*. `maos-cli` is a plain **`[dependencies]`** entry of maos-bin (`crates/maos-bin/Cargo.toml:44`, section header `:39`), so cargo builds its **lib**, never its `[[bin]] maosctl` (`crates/maos-cli/Cargo.toml:10-12`). The existing job is green only because `Swatinem/rust-cache@v2` restores a `target/` that some other job populated — a gate passing for a reason unrelated to what it asserts | **AC6's job MUST add `cargo build --locked -p maos-cli --bin maosctl` beside the `worker-cli-fixture` step.** ⚠ And the 16-1 job has the same latent defect — report it to `epic-16-retrospective` rather than copying it. ✅ CLEAR on the axis the round was sent to check: the enrollment **does** block per-commit. `discipline.yml:3-7` is `push`/`pull_request` on `main` with no ref guard; `v1-0-ship-gate` carries only `if: always()` (`:3729`) and exits 1 on any failed/skipped/cancelled need (`:3780-3782`); and **`aggregate.needs` contains `v1-0-ship-gate` (`:3928`)** with its own `exit 1` (`:4155-4158`). The Epic-11 *"ran but returned Ok"* decay does **not** apply. ⚠ RISK only: `check_ship_gate_completeness` iterates `EXPECTED_GATES` one-directionally (`xtask/src/check_ship_gate_completeness.rs:212-230`) and **never** iterates `needs`, so omitting the job from `EXPECTED_GATES` is silently allowed and nothing asserts every workflow job is enrolled somewhere |

## 18b. Axes round 3 cleared

- **The kernel ceiling raise is mechanically possible.** `authorize_raise` (`xtask/tests/recovery_lane_ceiling_rule.rs:66-103`) is **fixture-only** — all seven callers are synthetic (`:128-204`); it never reads the real `kloc.toml`. The only tree-bound assertion is `kernel_core_ceiling_has_not_moved_under_the_easing` (`:266-281`, `assert!(actual <= RATIFIED_AT_EASING)` at `:275`), whose documented escape — move `RATIFIED_AT_EASING` in the same commit under the operator grant — was exercised by 16-3/16-4 and 16-5. Repo-wide, `19053`/`19_053` occur in exactly **two** places: `kloc.toml:247` and `recovery_lane_ceiling_rule.rs:268`.
- **But the raise IS required: headroom is ZERO, not `+26`.** `kloc.toml:247`'s own comment ends *"ZERO HEADROOM retained"*. `kloc-check` (`xtask/src/kloc_check.rs:220-388`) counts **tokei `code` lines** (comments and blanks excluded) with `-e target -e tests -e benches -e examples -e fuzz -e spirits`, so `crates/*/tests/` is free and **inline `#[cfg(test)]` is charged** — the `loaded_to_unloaded_rejected` rewrite consumes budget, as §15 V1 said.
- **The pin set reconciles: 97 `.rs` files under `crates/maos-kernel-core/src` + `security/sandbox/t3-image.lock` = the pinned 98** (`kernel-core-baseline.toml:543-640`, `PINNED_ENTRIES` `kernel_pin_content_hash_16_0.rs:33`), and `src_lines = 24628` is the **exact raw line sum** of those 97 files — verified by direct count at HEAD. So the pin is a plain sum and `--emit-pin` will round-trip.
- **No floor value bricks posture shifting.** `Posture` is `Cautious < Assistive < AutonomousWithHalt < Autonomous` (`maos-manifest/src/manifest.rs:645-652`), so `min()` is monotone-down; a floor of `Autonomous` is a no-op, and the tightest (`Cautious`) merely pins every Spirit to `Cautious` and makes `--shift assistive` return `above_posture_ceiling` — restrictive, **loud**, recoverable. ⚠ **RISK: the field named `floor` is implemented as a `min()`, i.e. a CEILING.** Anyone later "fixing" it to `max()` gets an instant brick at `Autonomous` via `shift_posture`'s `NonRuntimePosture` refusal (`cap_policy/mod.rs:239-241`). **Name it `operator_posture_ceiling`, or bound its legal values to `{cautious, assistive, autonomous-with-halt}`** — round 2 did neither.
- ⚠ **RISK: clamping `allowed_max` changes `posture_hash()`** (`security/posture.rs:108-110`, which hashes **both** fields), so a capability token minted against a pre-clamp snapshot verifies as `PostureMismatch`. Benign today **only** because admission precedes token issuance on both `maos run` arms — state it, do not assume it.

## Debug Log References

**T0 environment repairs — the tree was NOT green as the story claimed, for two
reasons that are both tooling, not code.** (1) `kloc-check` shells out to
`tokei` and pins version **14.0.0**; it was not installed
(`failed to read tokei version`), and installing 15.0.0 first produced
`tokei version mismatch: expected 14.0.0, found 15.0.0`. (2) This workstation
sets `CARGO_TARGET_DIR=/mnt/build/cargo`, but `check_exit_commands.rs` resolves
binaries at the hardcoded `root.join("target/debug")`, so with the binaries
genuinely built the gate still reported `surfaces {maos:0, maosctl:0}`, four
`owed` `maos`/`maosctl` rows and 12 `unresolved-verb` findings. A `target →
$CARGO_TARGET_DIR` symlink (already gitignored) resolves it. Both are worth
naming for the next story on this machine.

**T0 measured at `471608ea`, every number re-derived:** run block
`main.rs:4094-6047`; `classify_spirit` `:456` with 8 classes;
`SCANNED_SOURCE_FILES` 23; pin `src_lines` **24628** on line **505**, 98 files,
`set_hash 66ee5370…`; `RATIFIED_AT_EASING` **19_053** (at `:268`, not the
story's `:259`); aggregate **167189**, hardfail 170884; `maos-bin` 22996/22996
= **ZERO**; `maos-kernel-core` 19053/19053 = **ZERO**; `maos-cli` 6080/6856;
`maos-control` 1264/1791; `maosctl` surfaces **25**; `check-exit-commands` 31
resolved with `load` **owed to `16-6 [ready-for-dev]`**. `--emit-pin` confirmed
to round-trip **byte-identical** over lines 538-640 before a kernel line was
written.

**Proven-red captured BEFORE the kernel arm (obligation (b)).**
`crates/maos-kernel-core/tests/loaded_spirit_exit_16_6.rs` red at HEAD with
`InvalidStateTransition { spirit_id: "door-loaded", current: Loaded, verb: Unload }`,
and the hook-failure falsifier red because `on_unload` never dispatched at all
(the `?` returns before `fire_on_unload`). `terminate_spirit`'s
non-idempotence (obligation (al)) PASSED at HEAD — it is characterisation, and
it is why AC2(g)(i) guards the spawn rather than making the function
idempotent.

**Falsifications run, each restored afterwards.** (c)/(w) deleting the
`(Loaded, Unloaded)` arm reds `loaded_to_unloaded_allowed` AND both AC2
integration falsifiers while the AC1 posture suite stays 5/5 green — the two
ACs are separable and the grant buys exactly one thing. (ac) clamping
`allowed_max` only reds exactly one test,
`a_default_above_the_ceiling_is_clamped_too_never_left_inverted`. (l)/(ah)
deleting the `route_budget` `Load` arm compiles with **zero** errors, con­firming
R11: that match ends `_ => DEFAULT_ROUTE_BUDGET` and the arm is
uncompiled-checked. (ag) any mention of the policy-scope table in
`admission.rs` reds `composition_root_does_not_seed_manifest_scopes` — which
also caught the module's own doc comment, since the negative greps raw source
including comments. (u) prepending a two-line HISTORY block above `src_lines`
reds the position assert at line 507.

⚠ **(j) is an HONEST NEGATIVE and is recorded as one.** Deleting the door's
pre-CAS lock key and re-running the concurrent-load test still PASSES: two
`maosctl` process spawns do not interleave tightly enough to land inside the
kernel's `resolve_pid`/`insert` window. The test asserts the contract but does
not discriminate the lock; the lock's necessity rests on the measured shape of
`SpiritSchedulerAdapter::load`, not on that test going red. Stated in the
test's own doc comment rather than left as an implied proof.

**Two null controls found and repaired, both pre-existing.**
(1) `production_collective_calls_share_one_atomic_pid_binding`
(`cohort_daemon_smoke_13_5c.rs`) `include_str!`d `main.rs` and searched for
`fn collective_write(` — measured **0 occurrences in main.rs at HEAD**, 3 in
`cross_team_crossing.rs` where the methods were relocated. It had been
panicking on its first iteration and asserting nothing since that move.
Re-pointed at both owning files and **falsified**: planting a second
`self.spirit_pid.load(` reds it with the right message. (2)
`production_capability_parsers_are_all_schema_degraded` counted parsers in
`main.rs` alone; with the extraction it now spans `main.rs` + `admission.rs`
(4 parsers, 4 degrade sites — the count drops 5 → 4 because the two
`caps_required_or_empty` call sites collapsed into one shared path).

**Regression triage.** Five suites fail on this machine; **all five fail
identically at HEAD**, verified in a detached `git worktree` at `471608ea`
with matching pass/fail counts: `two_host_reconcile_2c` (8/2, python twin),
`equiv_harness` (17/3) and `t2_sandbox_kill` (2/1) (wasm host),
`abi_diff_integration` (0/5), and `service_boundary_integration` (the
pre-existing `KernelHaltResolver` removed+unclassified pair). ⚠ Worktrees
share `CARGO_TARGET_DIR`, which contaminated one intermediate run with HEAD's
`maos-control` rlib; `cargo clean -p` on the five touched crates and a full
re-run produced the final result.

**One surface baseline moved, one line.** Adding `operator_posture_ceiling`
changes `OperatorPolicyConfig`'s signature hash, which
`check-service-boundary` reads as a removed symbol.
`docs/ci-baselines/kernel-surface-v0.1-beta.json` re-pinned
`7ba7c38c…` → `21bbe125…`; the change is additive (the field is
`Option<Posture>` defaulting `None`) and the story's own violation is gone.

