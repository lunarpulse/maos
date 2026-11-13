# Epic 22 — The Always-On Host (W7)

**Status:** `backlog` — created by correct-course 2026-11-13 (`sprint-change-proposal-2026-11-13.md` §4–§6, operator-approved 2026-11-13). First epic of the Enterprise-Fit lane (Epics 22–26). The lane starts after `v1.0.0` (decision 1); one minor release per epic. `sprint-status.yaml` is authoritative for status.

**Wave:** W7; 9 stories, ≈ 10.5 median-story units `[INFERENCE]` · **Makes true:** One config file, one service, one image or chart; the standing cast returns after a restart; other systems can hand a Spirit work; the disk stays bounded.

**Closes:** FR77 (Host file and standing cast), FR76 (service, packaging), FR78 (directed task), FR71 (list, session), FR79 (bounded audit store), NFR-Ops-14 (image size and idle memory, values set at `22-3`), NFR-Onb-1 for agent authors (ex 21-11), journey J5, `prd/success-criteria.md:9` first 30 minutes (owner `22-8`; not measured at `v1.0.0`, claim boundary 6 of the proposal's §2); 21-3 AC6; 21-4 AC2's unregistered-variable finding; the J4 pair from the binary (ex 21-1 AC2 and AC3, ex 21-13). Re-owned rows: `deferred-work.md` F1–F5 (20-9 hosted-proof findings), `DiscardSuccessor` journals `SwapInFired`, spawned swap refuses a still-starting component, `LifecycleCallers` has no unwind cleanup, swap-invariant refusal has no WASM witness → `22-9` (rows and line numbers in §22-9).

**Hermetic exit command (CI, no secrets; red until green):**

```
cargo test -p maos-journey-test --test journey_always_on   # 1 — Host boots from host.toml with Mira and Nash, Mira takes a task, the advisory reaches Nash under consent, restart keeps cast, memory and pending halt (22-8 AC1)
```

**Exit-block provenance (measured at `1a57035e`):**
- Line 1 — `journey_always_on` is created by 22-8 AC1. **HEAD: red** — no such test target (`ls crates/maos-journey-test/tests` has no `journey_always_on.rs`); `cargo test` exits non-zero. Green when the scene of 22-8 AC1 passes.
- Shape: one `cargo test` line with a trailing `#` comment; no secret, key, human, wall-clock or network. The first-30-minutes measurement (22-8 AC3) is an operator-lane leg and is not in the exit.

**Kernel-Δ:** forecast 0, except the `22-9` rows (+2…+16 tokei, from the 21-9 forecast). Measured at landing.

**Lane rules (proposal §4, binding for Epics 22–26).** (1) Adapters stay outside the kernel behind existing ports; `24-3` is the only kernel-touching story. (2) No new `xtask` gate; an exit is a `cargo test` target or a `maos` verb created by a named AC. (3) One hermetic exit per epic. (4) Each external standard is pinned by version in its story, which re-reads the specification. (5) A control is wired or it is labelled. (6) Each epic is preflighted at the previous retrospective.

**Confidence rules.** Rules 1–11 are those printed in `epic-21-multi-host-on-live-substrate-w6.md` under **Confidence rules** and **Confidence rule 11**; they bind here and are not repeated.

**Risks (proposal §4).** `22-6` must rotate the audit store without weakening log-before-deliver or the detector that assumes row ids only grow. T3 Workers need rootless podman, which a pod does not give by default, so `22-3`'s chart excludes them. Assurance stays AI-agent review only.

---

## Stories

| Key | Story | Closes · size |
|---|---|---|
| `22-1-host-config-and-standing-cast` | Host configuration file and standing cast | FR77; 21-3 AC6; 21-4 AC2 · L |
| `22-2-service-mode-and-probes` | Service mode and probes | FR76 (service) · M |
| `22-3-image-compose-and-helm` | Image, compose file and Helm chart | FR76 (packaging), NFR-Ops-14 · M |
| `22-4-directed-task-ingress` | Directed task ingress | FR78 · M |
| `22-5-agents-list-and-session-view` | Agents list and session view | FR71 (list, session) · M |
| `22-6-bounded-audit-store` | Bounded audit store | FR79 · M–L |
| `22-7-domain-spirit-kit` | Domain Spirit kit | NFR-Onb-1 for agent authors (ex 21-11) · M |
| `22-8-always-on-scene` | Always-on scene | J5; first 30 minutes; J4 pair from the binary (ex 21-1, 21-13) · M |
| `22-9-wasm-form-residuals` | WASM form residuals | The 21-9 rows moved by the proposal's §2 · M |

AC1 is the command and what the operator observes. `bmad-create-story` re-measures every cite and may tighten, not add uncited ACs.

### 22-1-host-config-and-standing-cast — Host configuration file and standing cast

*Closes · Δ:* FR77; 21-3 AC6 (region against the cohort manifest); 21-4 AC2's unregistered-variable finding · size L · kernel-Δ forecast 0. Writes the new ADR for the Host configuration file and architecture §11.6 "Standing Host (server)" in `11-deployment-topologies.md` (`22-3` completes it).

- *HEAD at `1a57035e`:* `maos run <topology>` enters a continuous serving loop with a safe shutdown on ctrl_c/SIGTERM (`crates/maos-bin/src/main.rs:6215-6218`). Configuration is spread over 125 `EnvVar {` entries in `crates/maos-bin/src/env_contract.rs`, `~/.config/maos/operator.toml` (`crates/maos-kernel-core/src/security/operator_config.rs:56-61`) and `<home>/config.toml` (`crates/maos-bin/src/crypto_provider.rs:64`). No restore of Spirits loaded over the door was found `[INFERENCE: grep-negative]` — `rg -i 'restore|rehydrat|auto.?load' crates/maos-bin/src` returns 80 lines here; by their symbols they name backup restore, hot-swap and manifest restore, not a boot-time cast restore, and they were not each read. `MAOS_OPERATOR_HTTP_ENDPOINT` is read as `ENV_ENDPOINT` (`crates/maos-cli/src/door_client.rs:32`) and is absent from `env_contract.rs` (`grep -c` = 0). Region: `Region::resolve_home` (`crates/maos-domain/src/region.rs:89`, env read `:97`) and `TeamEntry.region` (`crates/maos-cohort/src/manifest.rs:156`).
- *Correction at application, 2026-11-13 (the proposal's §1 said the grep above returns 0 and that only the topology file's cast returns; both are wrong):* a restart already brings back **installed** Spirits on two roots. The daemon enumerates the installed manifests when it is a shell root or a serving root with no run arguments and no `MAOS_ONE_SHOT` (`crates/maos-bin/src/main.rs:4384-4390`) and boots each one through `InstalledSpirits::boot` (`main.rs:4454-4455`; `crates/maos-bin/src/installed_spirits.rs:16`). Under `maos run <topology>` that list is empty (`main.rs:4386`), so only the topology's Spirits return. Whether a Spirit loaded with `maosctl load` and never installed survives a restart was not measured. The story therefore starts from two existing boot paths (topology, installed set) and gives them one declared source; it is not a first restore mechanism.
- **AC1 (command).** `maos serve --config <host.toml>` boots the Host and the Spirits the file names; after a kill and restart the same cast is running, read back by `maosctl agents`.
- **AC2.** Every setting has one documented key; the environment overrides a key and never replaces the file; a `MAOS_*` read with no key or registry entry fails a test (the 21-4 AC2 finding: `MAOS_OPERATOR_HTTP_ENDPOINT`, `crates/maos-cli/src/door_client.rs:32`).
- **AC3.** An invalid file, or a region that disagrees with the cohort manifest (21-3 AC6), is a typed boot refusal.
- **AC4.** `maos run <topology>` keeps working.

### 22-2-service-mode-and-probes — Service mode and probes

*Closes · Δ:* FR76 (service) · size M · kernel-Δ forecast 0.

- *HEAD at `1a57035e`:* no systemd unit: `rg -il 'systemd|ExecStart' packaging docs/runbooks docs/maos.dev` = 0; `packaging/` holds `deb`, `rpm`, `homebrew` and `aur` recipes.
- **AC1 (command).** The deb and rpm install a systemd unit; `systemctl start maos` serves and `stop` drains.
- **AC2.** `maosctl status` exits 0 only when the daemon answers and its audit store is writable, so an exec probe needs no unauthenticated route.
- **Not in scope.** The unbounded local Worker join at shutdown (`RELEASE-HOLDS.md:85` row 11), which is kernel lifecycle work.

### 22-3-image-compose-and-helm — Image, compose file and Helm chart

*Closes · Δ:* FR76 (packaging), NFR-Ops-14 · size M · kernel-Δ forecast 0. Completes architecture §11.6.

- *HEAD at `1a57035e`:* the image has an entrypoint (`Dockerfile:20`, volume `:19`) and no documented run (`rg -i 'docker run|compose' docs/maos.dev/run-maos.md docs/runbooks` = 0). No Helm chart (`packaging/helm` absent). The binary bound is stated only for the release binary, 35,651,584 B (`xtask/src/prepush.rs:509`); idle memory and image size are not measured.
- **AC1 (command).** The release publishes a multi-arch image to the GitHub registry `[INFERENCE: token scope not verified]`; a compose file runs a Host with a persistent home.
- **AC2.** `packaging/helm/maos` deploys a Host with a persistent volume and `host.toml` plus secrets from values; the door stays loopback.
- **AC3.** Image size and idle memory are measured and written into NFR-Ops-14.
- **AC4.** The chart states that T3 Workers are not supported in a pod.

### 22-4-directed-task-ingress — Directed task ingress

*Closes · Δ:* FR78 · size M · kernel-Δ forecast 0. Writes the ADR-062 amendment (with `24-1`).

- *HEAD at `1a57035e`:* door verbs for a Spirit are lifecycle only (`crates/maos-control/src/lib.rs:1488-1584`, `"start"` … `"hot-swap-precheck"`); the shell reaches installed Spirits only inside its own process (`crates/maos-shell/src/lib.rs:547-549`, `dispatch_installed`).
- **AC1 (command).** `maosctl ask <spirit> "<text>"` returns a task id; `maosctl task show <id>` returns the result or the halt that stopped it.
- **AC2.** The door route is a new `OperatorCommand`, so it gets an operation id and a completion row.
- **AC3.** A Spirit that is not running is a typed refusal. Network callers arrive through `23-5`, not through the door.

### 22-5-agents-list-and-session-view — Agents list and session view

*Closes · Δ:* FR71 (list, session) · size M · kernel-Δ forecast 0.

- *HEAD at `1a57035e`:* `GET /v1/daemon` exists (`crates/maos-control/src/lib.rs:1139`) with no CLI consumer (`rg -c 'v1/daemon|daemon_status' crates/maos-cli/src` = 0); the in-flight task ledger is `task_assignments_in_flight` (`crates/maos-kernel-core/src/scheduler/control_block.rs:281`); `crates/maos-bin/src/delegation.rs:719,788` call `deliver_typed(frame, 0, None)`.
- **AC1 (command).** `maosctl agents` prints one row per Spirit: id, pid, state, form, originator, current task, age. Sources: `GET /v1/daemon` (`crates/maos-control/src/lib.rs:1139`) and a read route over `task_assignments_in_flight` (`crates/maos-kernel-core/src/scheduler/control_block.rs:281`).
- **AC2.** `maosctl agents show <id>` renders that pid's log rows; `--follow` polls.
- **AC3.** `crates/maos-bin/src/delegation.rs:719,788` journal at the emitter's pid, not 0.

### 22-6-bounded-audit-store — Bounded audit store

*Closes · Δ:* FR79 · size M–L · kernel-Δ forecast 0. Writes the new ADR for audit-store rotation.

- *HEAD at `1a57035e`:* the Transparency Log is never pruned: "no production `DELETE FROM transparency_log`; retention is `VACUUM INTO` a copy" (`crates/maos-bin/src/escape_detector_consumer.rs:18-19`); the escape detector's cursor assumes row ids only grow (`:15-19`).
- **AC1 (command).** A retention policy rotates the Transparency Log into a sealed, signed segment and starts a new one; `maosctl audit verify-bundle` accepts each segment alone.
- **AC2.** A query that spans segments reads them in order.
- **AC3.** Log-before-deliver holds across a rotation, and the escape detector's cursor (`crates/maos-bin/src/escape_detector_consumer.rs:15-19`) survives it; both have a planted red.

### 22-7-domain-spirit-kit — Domain Spirit kit

*Closes · Δ:* NFR-Onb-1 for agent authors (ex 21-11, simplified) · size M · kernel-Δ forecast 0. D-21-PF-21 and D-21-PF-22 are superseded.

- *Scope (proposal §2, row 21-11).* The kit, a working `maos-spirit validate`, one domain Spirit example, three recorded agent runs. No N=12 trial and no clean-VM attestation; the human-denominator legs of the gate are deleted, not extended.
- *HEAD at `1a57035e`:* `maos-spirit validate` and `inspect` exit 1 (`crates/maos-spirit-cli/src/bin/maos-spirit.rs:107-114`, "not yet implemented"). `check-third-party-trial`: N=12 floor `xtask/src/check_third_party_trial.rs:538`, ≥10 successes `:453-455`, `Participant` has no author-kind field (`:34-43`); enrolled at `.github/workflows/discipline.yml:4204-4225`. Kit sources: `templates/spirit-ts`, `templates/spirit-rust`, `docs/maos.dev/write-a-spirit.md`.
- **AC1 (command).** An agent given only the shipped kit generates, builds, signs and loads a domain Spirit that answers `maosctl ask`.
- **AC2.** `maos-spirit validate` works (it exits 1 today, `crates/maos-spirit-cli/src/bin/maos-spirit.rs:107-114`).
- **AC3.** Three agent runs are recorded with their transcripts and labelled as agent-authored.
- **AC4.** The human-denominator legs of `check-third-party-trial` are removed.

### 22-8-always-on-scene — Always-on scene

*Closes · Δ:* journey J5; `prd/success-criteria.md:9` first 30 minutes (operator-lane leg); the J4 pair from the binary (ex 21-1 AC2 and AC3; ex 21-13, not created) · size M · kernel-Δ forecast 0 (`spirits/`, `maos-bin`, `maos-journey-test`).

- *Scope (proposal §2, rows 21-1 and 21-13).* From 21-1, AC2 (inference seams) and AC3 (in-process router) are kept; AC1's 50-incident cassette and closure-rate line are cut, so the J4 closure rate stays not measured. The cassette-source choice moves with the story (2026-11-12 proposal §12). The first-30-minutes measurement is owned here (2026-11-12 proposal, ruling on item 1).
- **AC1 (command).** `cargo test -p maos-journey-test --test journey_always_on` boots from `host.toml` with Mira and Nash, hands Mira a task, sees her advisory reach Nash through the in-process router under typed-intent consent, restarts the Host, and finds the cast, the memory and a pending halt intact. This AC creates exit line 1.
- **AC2.** Mira and Nash gain inference seams (21-1 AC2) and the topology loader gains the router (21-1 AC3). The two carried ACs, cites re-measured at `1a57035e`:
  - *21-1 AC2.* A prompt→`AnomalySignal` adapter feeds Mira (`spirits/mira/src/lib.rs:274` `diagnose(&AnomalySignal)` takes a signal, not a prompt); Mira and Nash gain inference seams (`grep -cE 'Inference|provider' spirits/mira/src/lib.rs spirits/nash/src/lib.rs` = 0 for each): manifests gain `[capabilities.required] provider.complete` (the section Butler already declares, `spirits/butler/manifest.toml:19-21`, so no manifest-schema change); the Decision-I smoke assertions are inverted (`spirits/mira/tests/spirit_smoke.rs:99-103, 128-133`; `spirits/nash/tests/spirit_smoke.rs:89-92`) and the manifest comments updated (`spirits/mira/manifest.toml:17-18`, `spirits/nash/manifest.toml:14-15`); the topology `LoadedSpiritKind::Mira`/`Nash` arms (`crates/maos-bin/src/main.rs:5916-5965`; Mira gets only the `EpistemicScalarPort` adapter, `:5958-5960`) wire the deferred inference port as 18-1 does for Butler, with the Researcher arm (`Some(LoadedSpiritKind::Researcher)`, `:6641`) as the template, under the 15-6 selector; provenance: `spirits/mira/manifest.toml:67-68` (`covered_model_id = "maos.mira.deterministic-diagnostic-v1"`) is updated to the recorded model, and `spirits/nash/manifest.toml` gains a `[model_provenance]` section (it has none, `grep -c model_provenance` = 0). Spirit code is uncharged (`xtask/src/kloc_check.rs:310-311`, `-e spirits`).
  - *21-1 AC3.* The `maos run` topology path routes Mira's advisory to Nash through an in-process `LoopbackA2ARouter`, new in the topology loader: today the topology arms (`crates/maos-bin/src/main.rs:5916-5965`) load both Spirits into one scheduler with no router between them, and every `LoopbackA2ARouter::new` site in `main.rs` is inside a smoke function (`:14023, 14081, 14150` in `smoke_a2a_loopback_6_3`; `:14281` in `smoke_a2a_consent_vocab_8_7`; `:14466` in `smoke_a2a_fail_closed_8_8`). The template is `smoke_a2a_consent_vocab_8_7` (`:14218`; `ADMIT_INTENT = "diagnosis-handoff:read-only-evidence"`, `DENY_INTENT = "code-mutation-directive"`, `:14236-14237`; loopback in-process, the J3 daemons already prove TCP). Peer configs come from a new `[[topology.a2a_peers]]` section of the topology file (the parser rejects unknown keys, `spirits/topologies/j4-mira-nash.toml:4-5`; allowlists are not manifest fields, `spirits/nash/manifest.toml:16-17`) carrying each Spirit's `accept_allowlist`. A second topology `spirits/topologies/j4-mira-nash-deny.toml`, whose Nash allowlist excludes the advisory intent, produces the kind-22 `RuptureReason::IntentAllowlistMismatch` row (`crates/maos-domain/src/frame.rs:399`) through `A2ARouterCore::handle_intake` (`crates/maos-a2a-core/src/router.rs:1287`); `crates/maos-journey-test/tests/journey_j4.rs:116` (`j4_earned_consent_rupture_typed_oracle_deferred`, `:118`) is un-ignored against it. Accept path first, deny path second.
- **AC3.** Operator lane, never blocking and never in the exit: a blank-context agent deploys the released image and completes one task in 30 minutes or less. This is the first-30-minutes measurement of `prd/success-criteria.md:9`.

### 22-9-wasm-form-residuals — WASM form residuals

*Closes · Δ:* the 21-9 rows moved by the proposal's §2 (21-9 AC10 and the part of AC11 that is not the FR11 row) · size M · kernel-Δ forecast +2…+16 tokei (`DiscardSuccessor` row +2…+6 and F3 0…+10, from the 21-9 kernel forecast), measured at landing.

- *Rows (titles as in `deferred-work.md`; line numbers measured at `1a57035e`; the proposal printed `:1738-1742` and `:1760`, one line low).* The FR11 row "A hot-swapped spawned successor writes no FR11 archive record" (`:1685`) stays with `21-9-wasm-host-default-on` for GA.

| Row | Title | What 21-9 AC10 / AC11 specify |
|---|---|---|
| `:1739` | F1: a denied `author()` in the shipped TypeScript example ends the session with `OutOfFuel` | The denial declines without `OutOfFuel` (runner fuel 10,000,000, `crates/maos-bin/src/main.rs:2613-2616`) |
| `:1740` | F2: the successor's `swap-in` `console.log` never reaches `spirit.diagnostic` | Pre-warm `console.log` journaled as `spirit.diagnostic` (`Prewarm::next_stdout`, `crates/maos-bin/src/spawned_lifecycle.rs:151`) or the README claim corrected |
| `:1741` | F3: for a spawned Spirit under `maos run`, `idle` fires once per quiescence and nothing re-arms it | Re-arm decided (`crates/maos-kernel-core/src/scheduler/idle_watchdog.rs:93-95`) or documented as once per quiescence |
| `:1742` | F4: shutdown-time `lifecycle_session_ended` panic caught by the telemetry pump | A trigger after session end is dropped like BUSY. The row quotes `supervision.rs:2749` from the run-43217945522 log; at `1a57035e` `rg -n lifecycle_session_ended crates` = 0, so the story re-locates the site |
| `:1743` | F5: a guest `setTimeout` (positive monotonic-clock deadline) is expected to panic the T2 runner | Guest timers supported or refused typed, with an external-deadline witness (`crates/maos-wasm-host/src/runner.rs`) |
| `:1665` | The `DiscardSuccessor` consumer hard-codes the journaled phase `SwapInFired` | The abort row journals its real phase (`crates/maos-kernel-core/src/hot_swap/saga.rs:128-130`) |
| `:1677` | A spawned swap waits for a component that is still starting | The door waits for readiness (D-21-PF-15 A) or the retry stays documented |
| `:1721` | `LifecycleCallers` has no unwind cleanup and its joins are bounded, not guaranteed | `LifecycleCallers` gets an unwind guard |
| `:1761` | The swap-invariant refusal has no WASM-component witness | One `swap_invariants` vector in `spawned_swap_20_9` (`crates/maos-bin/tests/spawned_swap_20_9.rs`) |

- **AC1 (command).** The rows above are each closed with its witness or kept with a stated reason, as 21-9 AC10 and AC11 specify.

## Dependencies

- `v1.0.0` is tagged and the Epic 21 retrospective preflights this epic (lane rule 6). `22-1` opens the epic (proposal §7 step 5).
- Host file and model choice (2026-11-12 proposal, ruling block item 5): the Host file that `22-1` adds carries no model-default key; the served model stays manifest pin > profile `default_model` > `Unconfigured`.
- Spend figures for `22-5` come from story S2 of the reasoning-model adapter, which lands before GA. The adapter's later stories (Epic 27, keys assigned at their own story creation) run beside this lane, with no order against it.
- `22-7` and `22-8` use `maosctl ask` from `22-4`; `22-8` AC1 boots from the Host file of `22-1`; `22-8` AC3 deploys the image of `22-3`. `22-9` follows `21-9-wasm-host-default-on`, which keeps the GA rows.
- `23-5` brings network callers to the task path of `22-4`.
- Open D-row for this preflight (proposal §10): the numbering of the lane's minor releases, because "v1.1" and "v1.5" already name PRD phases with other content.
- Architecture: §11.4 and §11.5, owed since `15-full-spectrum-v2-2.md:129`, are outside this lane; the gap goes to the Epic 21 retrospective.
