# Sprint Change Proposal — 2026-09-05, Round 3 (bmad-correct-course, Batch)

**Trigger:** the Round-2 preflight of Epics 15–21 (`epics/epic-15-21-preflight-r2-2026-09-05.md`, seven sequential scouts @`9f920180`) scored every epic `needs-rework` (confidence 38–55 against targets 55–75; none `blocked`) and produced 139 concrete OLD→NEW edits plus ten decisions. The operator instructed "do the next step" on the preflight's recommendation; the recommended resolution of each decision is therefore applied as the default and recorded here for override.

**Mode:** Batch. **Scope class:** Moderate (backlog re-organisation; no new epics or stories; keys unchanged). **Status:** APPLIED 2026-09-05 (see §4 for the per-epic application record and §6 for residuals).

---

## 1. Issue summary

Round 2 (2026-09-04) ratified Fork C and eight confidence rules and rewrote the recovery lane as Epics 15–21. The same-day-plus-one preflight measured the rewrite against the tree and found that the rules had been ratified but not executed at planning time:

- **Rule 1 (verb-first) is not mechanisable as designed** — `maos` has no clap and no `--help` (`crates/maos-bin/src/main.rs:1653` hand-parses argv; `maos --help` boots the daemon), so `check-exit-commands` cannot resolve `maos` tokens.
- **Exit blocks were written, not run** — wrong flag shapes (E16), four non-existent tokens (E17), wrong cassette path + three undeclared flags (E18), an unreachable target state and a red-at-HEAD `demo-j1` (E19), a stub server that exits 1 and a bash syntax error (E20), a route that does not exist (E21).
- **Second-generation assumed foundations** — Workers have no SCB (16-3); the production Worker spawn is bare and T3 is never applied (17-1); the WIT world imports nothing (17-3b); 18-4's mechanisms have zero occurrences; the Orchestrator has no scalar port and `morning_digest` has no production caller (19-4); 20-4 stands on two hotfix-lane rows.
- **A fifth red Blocking gate at HEAD** — `check-j1-loopback-delegation` (suffix allowlist `_2a.rs`, `check_j1_loopback_delegation.rs:114`, catches `cert_rotation_trigger_14_2a.rs`) — owned by no story.
- **Budgets that add up only on paper** — maos-bin's 15-2 allowance (+1200) over-subscribed ~2×; kernel-core +150 reverses the Epic-13 retro's refusal; maos-domain +300 vs 21-4's ≈+750.
- **ACs that promise what is already true** — 21-2 (tests already run per-commit), 18-2 (number already asserted at ≥0.90), 18-4 (`jb3` not ignored), 15-4 (`Cargo.lock` tracked).

## 2. Impact analysis

- **Epics:** all seven re-edited in place; story count 34 unchanged; keys unchanged; epic status `backlog` unchanged. Measured durations replace stated ones (lane 34–54 weeks serial / 30–48 parallel, vs 28–40 stated).
- **Stories:** every affected AC rewritten per the notes' §K (OLD → NEW recorded in the notes; the epic files carry the NEW text). Three stories change kernel-Δ class: 19-3 becomes FLAG-Winston (+3–5, child cwd on `BridgeSpawnSpec`); 17-1 is re-priced +65–130; 15-1 fixes +2 (attribute path).
- **Tracker:** seven operator-lane rows added (`ops-first-signed-tag`, `ops-live-key-j0-leg`, `ops-live-worker-under-17-1-profile`, `ops-nightly-cassette-rerecord`, `ops-google-calendar-oauth-mcp`, `ops-j1-live-agent-signed-take`, `ops-j4-live-incident-recording`); 249 rows.
- **PRD / architecture:** `[DELTA-2026-09-05 R3]` paragraphs record the ten decisions; scope, phases and journeys unchanged.
- **Register / deferred-work:** no key changes → no re-pointing.
- **Gates that read these files:** `check-epic-close-coherence` (index counts, kernel-Δ pin citations), `check-decision-register`, `check-dev-record-completeness` — re-run after application (§5).

## 3. Recommended approach — Direct Adjustment, with ten decisions taken by default

| # | Fork | Decision applied (override by editing the epic file + this table) |
|---|---|---|
| D-A | Who binds the door (16-1) | The `maos run` / `maos shell` daemon process binds the loopback POST surface; `maos init` mints the bearer and writes `MAOS_HOME/control.json`; every `maosctl` verb discovers the endpoint from it; `maos-cli` gains an HTTP client and never links `maos-control` |
| D-B | Egress + credential (17-1) | Host-side proxy holds the vendor key and enforces the manifest allowlist; `--network=none` stays on the container; kernel-minted scoped token in the child env; `env_clear` test inverted |
| D-C | WASM recall transport (17-3b) | maos-bin side-channel over a unix socket (the runner's stdio is the kernel bridge and stays unchanged); priced by 17-3a |
| D-D | One replay selector | `MAOS_INFERENCE_MODE={record,replay,live}` + `MAOS_REPLAY_CASSETTE=<path>`; `--replay-llm` retired; no `--replay` flag; unset = today's deterministic path |
| D-E | Digest render (19-4) | Daemon renders via `Butler::morning_digest`; Butler does not join the J1 topology; Epic 18 is not a founder-loop dependency |
| D-F | Kernel headroom (15-2) | E13 retro refusal overruled in writing; per-story kernel figures granted (15-1 +2, 16-5 +5–25, 17-1 +65–130, 17-2 ≤+10, 19-3 +3–5, 21-5 negative) |
| D-G | Absent-pass gates (20-3) | The 19 gates named in the Epic-20 note; ten retirees named with arithmetic; `BindingClass` scoped to the 40 `[[ship_gate]]` rows |
| D-H | I9 exemption (15-1) | Attribute path, single-line form, +2 kernel lines, re-pin 24472→24474; no whitelist edit |
| D-I | Env registry home (21-4) | Stays a data file read by `check_env_contract`; no relocation into maos-domain |
| D-J | Orchestrator move (21-5) | A named leaf crate, ordered after 21-4; story stays optional |

Effort: one planning day (this round) + the re-scout. Risk: low — every edit is traceable to a measured finding; the decisions are reversible by editing text before any story leaves `backlog`. Timeline: the lane's stated 28–40 weeks becomes 34–54 (30–48 with the parallel tracks); the operator lane runs alongside.

## 4. Detailed change record (per epic)

### Epic 15 — Foundations (`epic-15-foundations-w0.md`)
- **Applied 18/18** §K items; D-D, D-F, D-H, D-I folded; D-A/B/C/E written as ADR content in 15-5 AC1 (ADR-060..**064**; 064 = one replay selector).
- **Exit block** dry-parsed: line 1 = the five red gates + `cargo test` (each red reproduced @9f920180, `check-j1-loopback-delegation` added as the fifth); line 2 `check-exit-commands` (created by 15-3 AC3); line 3 `release-dry-run` (created by 15-4 AC1; the tag/push moved to `ops-first-signed-tag`).
- **15-2 table** rebuilt from the notes' §E: maos-bin +3000 (lane-wide; the arithmetic shows the gross ask is +1.9k–4.2k and names the uncovered upper bounds), xtask +400 (retirement arithmetic required), kernel-core +250 per story (15-1 +2, 16-5 +5–25, 17-1 +65–130, 17-2 ≤+10, 19-3 +3–5, 21-5 negative; E13 retro refusal overruled in writing), new rows maos-manifest +150, maos-mcp +300, maos-journey-test +400, maos-bench +200, `maos-egress` +300–600 (new leaf crate from 17-1), `no change` rows recorded.
- **15-3 AC3** gained the shell-token clause (trailing `&`, `VAR=$!`, `kill`, `cargo test --test`) so Epic 16's block parses. **Order corrected:** 15-2 → 15-3 → 15-1 (maos-bin has zero headroom; the verb table cannot land before the grant).
- Suggested rename (not performed): `15-4-release-repair-and-first-signed-tag` → `…-and-release-dry-run`.

### Epic 16 — One Daemon, One Door (`epic-16-one-daemon-one-door-j0-w1.md`)
- **Applied 18/18**; two overridden by ratified decisions (§K-4's `run/operator.json` layout → D-A `MAOS_HOME/control.json`; §K-2's REPL transcript → `cargo test -p maos-journey-test --test journey_j0`). 16-1 marked ◆ and carries D-A as the single design incl. the `maos-cli` HTTP client that never links `maos-control` and the fail-fast on a second root.
- **Exit block** 7 lines, `bash -n` clean, every token verified (real clap shapes `cli.rs:700-738`; `maos uninstall --keep-log` created by 16-4 AC1); `MAOS_REPLAY_CASSETTE=…/j-butler/on-idle-halt.json` per D-D.
- 16-3 rewritten on the SCB + `task_assignments_in_flight` record Workers lack (kernel-Δ 0; `FaultCause::SignaledByKernel` reused; `spirit_pid 0` at `worker_spawn.rs:732` replaced); 16-4 `linux-native` keyutils + `file` backend via `MAOS_SECRETS_BACKEND`; 16-5 legal-hold serialised without changing `ForgetOutcome`, five router notes → typed variants, shell callers propagate `record_invocation`.
- Inventories replaced by grep-backed counts (18 sites / 21 values; 9 fake-`maos` contract test files ≈36 tests + 2 CI scripts). New hermetic tests named (`one_daemon_one_door_16_1`, `maos_uninstall_16_4`) and `OperatorHttpError::EndpointInUse` — editor's names, to be confirmed by `bmad-create-story`.
- Kloc flag: maos-shell ask ~+100–150 vs +100 granted.

### Epic 17 — Workers and the Third-Party Form (`epic-17-workers-and-third-party-form-w2.md`)
- **Applied 20/20.** D-B written as the proxy design in 17-1 (piped `spawn_t3` route so the Worker is actually under T3; `credential_posture_2c.rs:299-330` inverted by a named AC; kernel-Δ +65–130 FLAG-Winston). D-C applied as a **unix-socket** side-channel service (the runner's stdio *is* the kernel bridge, so "over stdio" was corrected in the compiled preflight, the proposal table, PRD and architecture deltas). The proxy lives in a NEW leaf crate `maos-egress` (+300–600; 15-2 row added).
- **Exit block** rewritten on real tokens: `spirits/topologies/worker-sandbox.toml` + probe fixture (created 17-1 AC1); `maos audit query --spirit worker --format plain` as its own executable line; `cargo build -p maos-bin -p maos-wasm-host --features maos-bin/wasm-host …` + `examples/example-spirit-ts/manifest.toml` rewritten to `forms = ["wasm-component"]` (schema v5, N-1 compat) by 17-3b AC1. Honest observables stated per sandbox; TL kind-8 rows emitted by a named mechanism. Lines need network for dependencies (podman base image, `npm ci`) — precedented in discipline, no secret; recorded.
- 17-2: root layout deleted; single delegation-compatible layout; podman flags documented as the third mechanism; CI delegation decision stated; `cgroup_ceiling_smoke` gets a job. 17-4: Observer implements `on_telemetry_event`.
- Dependencies: 17-3a before 17-1 (prices the proxy) and 17-3b; Epic 16's `main.rs` lock released before 17-1.

### Epic 18 — Spirits That Think (`epic-18-spirits-that-think-w3.md`)
- **Applied 15/15** (22 sub-edits); §K #9's new `--deterministic` flag refused under D-D (unset selector = deterministic path).
- **Exit block** 4 lines (`bash -n` clean): lines 1/3 use `MAOS_INFERENCE_MODE=replay MAOS_REPLAY_STRICT=1 MAOS_REPLAY_CASSETTE=<real path>`; line 2 `maos eval halt --class butler` (created 18-2 AC1; resolvable after 15-3's verb table); **line 4 added in this round**: 18-4's `--clock compressed:21d --acceptance …` so J-Butler has a machine-checked leg.
- 18-1 AC2: NEW fixture MCP server in `maos-journey-test` with `initialize`/session/bearer and three proven-red negatives. 18-2 reframed: the per-class number already exists at ≥0.90/≥0.85 in CI; the story makes it model-dependent under replay and publishes it on one surface (signed benchmark sidecar). 18-3: hermetic string metrics vs nightly judged metrics; AC3 → spike (`time_cap_seconds` from the real manifest). 18-4 rewritten on named new mechanisms (`--clock`/`--acceptance` from maos-bin, `Clock` port in `maos-spirit-sdk` shared with 21-1, tuning rule + TL ledger); wrong citations corrected.
- FR30/FR55/FR56 dropped from Closes (no AC). 21-2 AC4's `jb8` owner is NOT 18-4 (handed to the Epic-21 editor).

### Epic 19 — The Founder Loop (`epic-19-founder-loop-w4.md`)
- **Applied 19/21** (#18 tracker rows = rule 3, already present; #21 = done by the Epic-15 editor). D-E (daemon renders via `Butler::morning_digest`, Epic 18 dropped from Dependencies), D-D (no `--replay`; env pair), D-F (19-3 FLAG-Winston +3–5: cwd on `BridgeSpawnSpec`).
- **Exit block** one command: `MAOS_INFERENCE_MODE=replay MAOS_REPLAY_CASSETTE=…/j1/founder-loop.json cargo run -p xtask -- demo-j1` (hermetic default = absence of `--live-codex`, `xtask/src/main.rs:994-1009`); red at HEAD via the J1 gate, green only after 15-1 narrows the suffix, then 19-1..19-4. 19-4 AC1 rewritten to the reachable state (ABSENT = {`halt-resume-referential-identity`} until new AC5; two-host beat INDETERMINATE/PROVEN; tier-2 beat operator-only).
- 19-1 `--epic` = deterministic split of `### Story` sections; 19-3 keeps `MAOS_LIVE_AGENT` and the split test as the control, fixture Worker hermetically, concurrency re-scoped to sequential (`worker_sequence_19_3.rs`); 19-4 one ambiguity tag repo-wide, Orchestrator `with_scalar_port` + daemon allowlist, new AC5 resume beat.
- Suggested rename (not performed): `19-3-real-worker-default-with-effect-oracle` → `19-3-fixture-worker-hermetic-effect-oracle`.

### Epic 20 — Ship It (`epic-20-ship-it-w5.md`)
- **Applied 18/18** (+ D-D, D-G, rule 2). Exit block rewritten to 9 lines, `bash -n` clean, zero placeholders: `release-dry-run` (15-4) → `release-verify --sign` with the bundled dev seed (`release_verify.rs:254`; its pubkey is also the packaging "placeholder", so `./dist` verifies hermetically) → `maosctl install --from-local ./dist --verify-only` → `maos-registry-server --bind … --root …` (created 20-1 AC1; the stub exits 1 today) with a `/dev/tcp` readiness loop → `maos-spirit publish --tier=public_untrusted --manifest examples/example-spirit-ts/manifest.toml --artifact …/spirit.wasm` → `maosctl spirit install example-spirit-ts@0.1.0` → `maos init && … maos shell` under the D-D selector. No `curl`, no tag, no secret.
- 20-1: server binary wired, `maos-cli → maos-mcp` edge, yank knob outside the kernel, `maosctl spirit install` verb; `maos-registry-server` + `maos-spirit` added to 15-4's artifact set (cross-epic note). 20-2: existing debhelper `packaging/deb/rules`; netns location named; Docker build hermetic, push = ops. 20-3: **measured** `[[ship_gate]]` = 38 vs `EXPECTED_GATES` = 40 — the 4/2 drift closes in AC6 (42 names; 18 modules gain `BindingClass`); 17 gates converted; `check-cross-form-equiv` retired instead of an unnamed tenth. 20-4: option (a) — the hotfix-lane rows `provider-model-env-overrides` and `model-pin-currency-gate-and-retired-pin-sweep` added to Dependencies. 20-5: version bump `Cargo.toml:64` → `1.0.0-rc.1`, `prerelease: true`, the rc-row mechanic `stability_matrix.rs` must gain; the tag = `ops-rc-signed-tag`.
- **Tracker:** `ops-rc-signed-tag` and `ops-live-gemini-leg` added; `ops-provisioning-secrets-and-accounts` acceptance now includes `DOCKERHUB_*` + cosign. (251 rows.)
- Kloc: maos-cli +250–400 **collides with 16-1's +700** (15-2 row must be +950–1100 or 20-1's verb re-scoped); xtask net −4.5k…−5.2k (adds +1.1–1.8k, retires 6406; `check_epic_6_bridge.rs` 4592 is load-bearing) — a NEGATIVE xtask delta, so the +400 row is a ceiling, not a need.
- Note: 15-5 AC3 keeps the WASM engine off in published binaries until Hold 2, so this exit proves install + admission, not WASM execution (recorded in the epic).

### Epic 21 — Multi-Host on Live Substrate (`epic-21-multi-host-on-live-substrate-w6.md`)
- **Applied 22/22** (two smoothed: `--once` dropped from 21-1's line because 18-4's `--clock` is "N passes then drain"; the deleted `tl-phase-b` AC3 replaced by enrolling the nightly `services:` block in `check-loom-substrate-drift`, which reads only `discipline.yml:66,174-209`). D-I (registry stays a data file; maos-bin nets −100…+280 after −479), D-J (new leaf crate `maos-orchestrator-buffer` = 917; kernel-core −817 with the ceiling lowered), D-D, and 18-4's `Clock` port + `--clock` reused for 21-1's scenario clock.
- **Exit block**: `cargo run -p xtask -- demo-reza` (exists `xtask/src/main.rs:1012-1019`; red at HEAD: "substrate incomplete", `demo_reza.rs:71-81`; no workflow invokes it) and `cargo test -p maos-journey-test --test ignored_owner_audit` (created 21-2 AC4; red: no such target).
- 21-1 on real mechanisms (`--incidents`, fixture, A2A router in the `maos run` topology path, each created by a named AC; blocked until `ops-j4-live-incident-recording` produces `cassettes/j4/mira-nash-50.json`); 21-2 recut to the nightly `demo-reza` scene + regenerate-and-diff capture + in-crate owner audit (the ignored journeys already run per-commit); `jb8` gets a `deferred-work.md` row, not 18-4; 21-3 new `GET /v1/a2a/pins` + `SyncPinLookup`, AC2 absorbs the six 14-2d ACs, AC3 `RuptureReason::CertPostGraceReject`; 21-4 retires the ≤25K citation and names the three phantom budgets; measured env counts 54 src / 63 all / 15 of 16 kernel (note said 55/64/16; both grep-stated).
- Dependencies: `17-3b → 21-2` dropped; 20-3 kept as a ledger-only edge; 21-2 ∥ 21-3 ∥ 21-4, 21-5 after 21-4; "Closes D1" removed.
- Suggested renames (not performed): `21-2-…` → `…-nightly-reza-scene-and-ignored-owner-audit`; `21-3-…` → `…-and-14-2d-self-leaf-rotation`.


## 5. Verification

- **Application count:** 130 of 132 §K items applied (15: 18/18 · 16: 18/18 · 17: 20/20 · 18: 15/15 (22 sub-edits) · 19: 19/21 · 20: 18/18 · 21: 22/22); the two unapplied Epic-19 items were rule-3 (tracker rows already present) and already-done-by-Epic-15. Six items were applied in a form overridden by a ratified decision (recorded per epic in §4).
- **Exit blocks:** all seven dry-parsed at HEAD `9f920180` — every token resolved to an existing verb/flag/path or to the AC that creates it; `bash -n` clean; expected HEAD result recorded per line (all red or vacuous-green, as an unlanded epic's exit must be); nothing needs a secret, a paid key, a human or a tag (Epic 17 fetches build dependencies; Epic 20 signs with the bundled dev seed).
- **Gates after application:** `check-epic-close-coherence: PASSED (21 epics re-derived against pin 24472)` · `check-decision-register: 23 rows, 15 open` (D19 note pre-existing) · `check-dev-record-completeness: PASSED (147 done-status stories checked)` · tracker YAML parses, 251 rows.
- **Re-scout (one scout, all seven files, changed sections only):** `review-2026-09-04/preflight-r2/rescout-r3.md` — all seven exit blocks `bash -n` clean, **121 of 124 provenance tokens TRUE** at `9f920180`, every Status line intact, exactly one "Round 3 applied" paragraph per file, one replay selector everywhere, one ambiguity tag, the `Clock` port created once (18-4) and reused once (21-1), kernel figures summing to 172 against the +250 grant, dependency graph acyclic. Confidence **52–66** (15·66 · 16·64 · 17·55 · 18·52 · 19·62 · 20·54 · 21·58), up from 38–55. It found 11 defects; **all 11 were fixed in this round** (two ship-blockers: 15-5 AC1 stated D-C as "over the runner's stdio", the design D-C rejects → corrected to the unix socket; Epic 20's nine-line block used shell 15-3 AC3's tokeniser could not read → AC3's tokeniser spec extended to `for/done`, `$( )`, `export`, `seq`, `od`, `tr`, `sleep`, `printf`, `/dev/tcp`, pipes, and bare binaries resolved against 15-4's artifact set. Plus: `136 needs:` → 123; the duplicate `maos-persistence` grant deleted (`kloc.toml:326` already grants 1999 headroom); three stale `maos-cli +700` references → +1100; the file's suggested order → 15-2 → 15-3 → 15-1; Epic 18's flag set corrected at both sites and line 4 given a provenance bullet; two wrong `sprint-status.yaml` line cites; the componentize-js grep restated as the one that reproduces; the ambiguity-tag literal `lib.rs:150` → `:148`). Re-verified after the fixes: seven blocks still parse, no defect string remains, `check-epic-close-coherence` PASSED.
- **Caveat the re-scout states about itself:** its seven per-epic citation auditors died on the provider rate limit, so the long tail of story-body citations is sampled, not exhaustive; exit blocks, decisions, arithmetic and cross-file consistency were checked directly.


## 6. Residuals (not applied in this round)

Recorded, deliberately not applied in this round; each is a `bmad-create-story`-time decision or a coordination note.

1. **Key renames suggested by the editors** (15-4 → `…-release-dry-run`; 19-3 → `…-fixture-worker-hermetic-effect-oracle`; 21-2 → `…-nightly-reza-scene-and-ignored-owner-audit`; 21-3 → `…-and-14-2d-self-leaf-rotation`; 17-3b title now says "over a recall side-channel"). A rename cascades into the register, `deferred-work.md`, `index.md` and three gates; do it in the story's own commit or not at all.
2. **Kloc rows still tight**: maos-shell +100 granted vs ~+100–150 asked (16-2); maos-bin +3000 vs a gross ask of +1.9k–4.2k (upper bounds named in the 15-2 table; D-I returns ≈−479); xtask +400 is a ceiling (Epic 20 nets −4.5k…−5.2k); maos-cohort +100 plus the 14-2d T8 grant; 18-2's maos-bin estimate +300–680 vs +250 named (absorbed by ungoverned `spirits/*` + maos-eval — flagged, not granted).
3. **Cross-epic ownership**: 20-1 AC4 sets `examples/example-spirit-ts/manifest.toml` to `public_untrusted` (Epic 17 owns the file); 16-1's new test names (`one_daemon_one_door_16_1`, `maos_uninstall_16_4`, `OperatorHttpError::EndpointInUse`) are the editor's, to be confirmed by the story; 21-2 AC4 files the `jb8-posture-shift-cognition` deferred-work row at story time.
4. **Rule-2 edge**: Epic 17's exit lines fetch dependencies (podman base image, `npm ci`) — precedented in `discipline.yml:989,1342`, no secret, but not offline; Epic 20's exit signs with the bundled dev seed (`release_verify.rs:254`), which is what makes it hermetic — a reader must not mistake it for the release key.
5. **Register**: D19's deadline stays UNQUERYABLE (pre-existing, filed at the j1 lane); no register edit in this round.
6. **Documents not re-derived**: `epics/epic-15-21-preflight-r2-2026-09-05.md` keeps its pre-Round-3 numbers as the record of what was found; §4 D-C and §7 step 3 were corrected in place (unix-socket side-channel; 15-2 before 15-3).


## 7. Implementation handoff

- **Route:** Product Owner / Developer (Moderate). The recovery lane opens with `bmad-create-story 15-2-kloc-ceiling-rebase` (maos-bin has zero headroom; nothing lands before the grant), then `15-3-single-phase-source-and-exit-command-check` (the `maos --help` verb table is the lever that makes rule 1 a control), then `15-1-green-at-head`. `bmad-create-story` may tighten but not add uncited ACs.
- **Success criteria:** every exit block in Epics 15–21 dry-parses at HEAD with a provenance line per command; `check-epic-close-coherence`, `check-decision-register`, `check-dev-record-completeness` green; the re-scout finds no FALSE citation in the edited sections.
- **Checklist:** §1 trigger ✔ · §2 epic impact ✔ · §3 artifact conflicts (PRD/arch deltas) ✔ · §4 path = Direct Adjustment ✔ · §5 proposal ✔ · §6 handoff ✔.
