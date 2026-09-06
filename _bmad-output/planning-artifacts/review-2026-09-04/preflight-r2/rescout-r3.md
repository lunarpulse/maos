# Re-scout R3 — Epics 15–21 after the Round-3 correct-course (2026-09-06)

**Pinned at:** `9f920180` + the uncommitted Round-3 planning edits. **Scope:** only what Round 3 changed — the seven exit blocks and their provenance lists, citations Round 3 introduced or altered, the ten decisions D-A..D-J, cross-file consistency, and the Status/Round-3 header lines. Citations the R2 notes list under "CONFIRMED TRUE" were skipped by instruction.

**Method:** every exit block extracted to a file and `bash -n`'d; every exit-block provenance token resolved against `git show 9f920180:<path>`; the arithmetic claims (kloc formula, gate retirement sum, `BindingClass` reach, `EXPECTED_GATES` ∪ `[[ship_gate]]`) recomputed; the inventory greps re-run. Seven parallel per-epic citation auditors were dispatched and all seven died on a provider rate limit, so the deep sampling below is the coordinator's own, weighted to the ~430 `path:line` citations Round 3 added.

## 1. Verdict table

| Epic | Exit block `bash -n` | Provenance tokens checked / FALSE | Other citations checked / FALSE | Decisions written as one design | Cross-file conflicts |
|---|---|---|---|---|---|
| 15 Foundations | **PASS** (3 lines) | 14 / 0 | 41 / 3 | D-D ✓ · D-F ✓ · D-H ✓ · D-I ✓ · **D-C stated as "over the runner's stdio" — contradicts D-C** | 4 (D-C spelling, maos-cli +700 vs +1100, story order, `demo-j1 --replay`) |
| 16 One Daemon, One Door | **PASS** (7 lines) | 24 / 1 | 22 / 0 | D-A ✓ (single design, incl. HTTP client + fail-fast) · D-D ✓ | 2 (maos-cli grant, `check-exit-commands` token list) |
| 17 Workers + third-party form | **PASS** (3 lines) | 21 / 1 | 18 / 1 | D-B ✓ · D-C ✓ (unix socket) · D-F ✓ | 2 (D-C vs E15, `target/debug/maos` token) |
| 18 Spirits That Think | **PASS** (4 lines) | 12 / 2 | 20 / 0 | D-D ✓ (`--deterministic` refused in writing) · D-E ✓ | 1 (`worker_spawn.rs:53-66` flag set) |
| 19 Founder Loop | **PASS** (1 cmd + 5 comments) | 12 / 0 | 26 / 0 | D-A ✓ · D-D ✓ · D-E ✓ · D-F ✓ | 0 |
| 20 Ship It | **PASS** (9 lines) | 27 / 0 | 31 / 2 | D-D ✓ · D-G ✓ (19 named, 10 retired with arithmetic) | 2 (exit block unparseable by 15-3 AC3; maos-cli note) |
| 21 Multi-host on live substrate | **PASS** (2 lines) | 11 / 0 | 34 / 0 | D-D ✓ · D-I ✓ · D-J ✓ | 1 (kernel-Δ unit −817 vs E15's "≈ −1000") |

**Status lines:** all seven `**Status:** \`backlog\`` intact; all seven carry exactly one `**Round 3 applied (2026-09-05):**` paragraph; zero residual "Preflight R2" paragraphs. One cosmetic carry-over: `epic-21-…w6.md:3` says "Replaces the morning's **Epic 20** file" (should be Epic 21) — pre-existing, not a Round-3 edit.

**Exit-block hermeticity (rule 2):** no secret, paid key, human or network in E15/E16/E18/E19/E21. E17 lines 1/2 pull a podman base image and run `npm ci` — declared, precedented (`discipline.yml:1342-1345`, `:989-992`), no secret. E20 line 2 signs with the bundled dev seed `release_verify.rs:254`, whose pubkey **is** the `RELEASE_PUBKEY` default at `:26-30` and the same key embedded at `packaging/deb/rules:10`, `maos.rb:20`, `PKGBUILD:36`, `maos.spec:20` — verified; the block is hermetic and the "not the release key" caveat is stated. E20 line 6's `sleep 0.1` readiness loop is bounded at 5 s, not a wall-clock oracle.

**Newly re-measured and TRUE** (Round-3 figures that reproduce exactly): `[[ship_gate]]` = 38 · `EXPECTED_GATES` = 40 · `BindingClass` modules = 16 · `aggregate` `needs:` = **123** · retirement sum 4592+421+217+158+108+100+69+164+463+114 = **6406** · `check_cna_registration.rs` = 260 · `coverage-matrix.yaml` 1982 lines, `mode: warning` at `:3`, NFR-Test-4/Aud-7/Perf-6 rows all `gates: [] corpora: []` · orchestrator 5 files = 1028 physical (220+90+531+97+90) · `maos-persistence/src/lib.rs` = 8 lines · `env_contract.rs` 87 `EnvVar {` / 507 physical, `EnvStability = {HarnessOnly, UserFacing}` at `:8-11` · `grep -c MAOS_ONE_SHOT subcommands.rs` = 21 with the 16 enumerated literal lines exact, +`:1461`+`:1164` = 18 sites · nine `MAOS_BIN_PATH` files, 42 `#[test]`, 42−6 = **36** · `SCANNED_SOURCE_FILES` 17 == 17 `.rs` files · `grep -n posture-shift sprint-status.yaml` = 0 · `_aggregate_hardfail` formula: 155498 + max(100, ⌈0.02·155498⌉=3110) = **158608** ✓ · every `xtask/kloc.toml` HEAD/ceiling pair in the 15-2 table (`:195` 18933, `:212` 41953, `:278` 8644, `:281` 1100, `:319` 4856, `:322` 1500, `:326` 2000, `:329` 1500, `:343` 17109, `:482` 5857, `:485` 3696, `:488` 593, `:501` 147057, and the maos-cli 5270 / bench 1631 / registry 3615 / spirit-cli 853 / wasm-host 1129 rows) · all nine `ops-*` rows exist in `sprint-status.yaml:281-293` (tracker = **251** rows) and the row-line citations `:288-289` (E18), `:291` (E21), `:209` (E19), `:218-219` (E21), `:223`/`:224` (E18/E20) are all TRUE.

## 2. Defect list (OLD → NEW)

### Ship-blockers — a decision written two ways, and a gate that cannot parse its own epic's block

**D1 · `epic-15-foundations-w0.md:129` (15-5 AC1) — D-C written as the design D-C rejects.**
The proposal §4 records that "over stdio" was corrected in place to a unix socket, and `epic-17…:81` carries the corrected design ("the runner's stdio **is** the ADR-032 bridge and stays byte-identical; the recall service listens on a unix socket"). ADR-060's content in 15-5 still says the opposite. Two ADRs' worth of design in one lane; rule 7 violated at the file that authors the ADR.
OLD → `**D-C** (WASM recall transport = a maos-bin side-channel service over the runner's stdio, kernel-Δ 0, priced by 17-3a; the WIT world imports nothing)`
NEW → `**D-C** (WASM recall transport = a maos-bin side-channel service on a **unix socket** whose path maos-bin hands to \`maos-wasm-runner\`; the runner's stdio IS the ADR-032 kernel bridge (\`runtime.rs:562-608,687\`) and stays byte-identical; kernel-Δ 0, priced by 17-3a; the WIT world imports nothing)`

**D2 · `epic-20-ship-it-w5.md:13-23` — the exit block is shell that `check-exit-commands` (15-3 AC3) cannot tokenise.**
15-3 AC3's declared tokeniser handles env-var prefixes, `&&`, a trailing `&`, `VAR=$!`, `kill "$VAR"`, `cargo test -p <c> --test <n>` and `#` comments. Epic 20's block additionally uses `for … do … done` (lines 3, 6), `ln -sf`, `export PATH=`, `REG_ROOT=$(mktemp -d)`, `$(seq 1 50)`, `(exec 3<>/dev/tcp/…)`, `sleep`, `$(od … | tr …)` and a `printf | ENV=… maos shell` pipe (line 9) — and names four bare binaries (`maos-registry-server`, `maos-spirit`) that AC3 has no resolution rule for at all. Rule 1 is mechanical only if the gate can read every non-`done` epic's block; as written it reds or vacuously passes on Epic 20.
OLD (15-3 AC3, `epic-15…:111`) → `The tokeniser skips shell-only tokens that the Epic-16 block uses — a trailing \`&\`, \`VAR=$!\` assignments, \`kill "$VAR"\`, \`cargo test -p <crate> --test <name>\` (resolved against \`ls crates/<crate>/tests\`), and \`#\` comments`
NEW → `The tokeniser skips shell-only tokens that the Epic-16 and Epic-20 blocks use — a trailing \`&\`, \`VAR=$!\` and \`VAR=$(…)\` assignments, \`kill "$VAR"\`, \`for … do … done\`, \`export\`, \`ln\`, \`seq\`, \`od\`, \`tr\`, \`sleep\`, \`printf\`, \`(exec 3<>/dev/tcp/…)\`, pipes, and \`#\` comments; \`cargo test -p <crate> --test <name>\` resolves against \`ls crates/<crate>/tests\`; a bare binary token that 15-4/20-1 add to the \`release-dry-run\` artifact set (\`maos-registry-server\`, \`maos-spirit\`) resolves against that set, not against \`--help\`. A block that is valid \`bash -n\` never fails the gate for shell syntax alone.`
(Companion: `epic-20…:28` OLD `the symlinks let \`check-exit-commands\` (15-3 AC3) resolve bare \`maos\`/\`maosctl\`/\`maos-registry-server\`/\`maos-spirit\` tokens` → NEW `… resolve the four bare binary tokens against 15-4's artifact set (the gate is static; the symlinks are for the runtime PATH)`.)

### Material defects

**D3 · `epic-15-foundations-w0.md:56` (15-1 AC1) — `136 needs:` is FALSE; measured 123.**
`discipline.yml:3510` **is** the `aggregate` job (TRUE), but its `needs:` list holds 123 entries — the number `epic-20…:94` (20-3 AC7) states correctly. Two epics, two counts, one list.
OLD → `the discipline \`aggregate\` (\`discipline.yml:3510\`, 136 \`needs:\`)` → NEW → `the discipline \`aggregate\` (\`discipline.yml:3510\`, **123** \`needs:\` @9f920180)`

**D4 · `epic-15-foundations-w0.md:90` — the new `maos-persistence` row contradicts the same table's row 102 and Epic 21.**
`kloc.toml:326` grants `maos-persistence = 2000` against 1 tokei line → headroom **1999**; row 102 records exactly that as `no change`, and `epic-21…:28` says "+300–600 … **no grant needed**". Row 90 invents a grant against a crate that already has one and prints a physical line count (8) in a tokei column.
OLD → `|maos-persistence|8 / — / —|+300–600 (new row; durable \`TofuPinStore\`)|21-3 AC1|`
NEW → *(delete the row; row 102 already carries `maos-persistence 1/2000/1999 → \`no change\`, 21-3 +300–600`)*

**D5 · maos-cli grant stale in three places after the 15-2 table went to +1100.**
- `epic-16…:39` OLD `maos-cli **+400–700** (against 15-2's +700: …)` → NEW `maos-cli **+400–700** (against 15-2's **+1100**, shared with 20-1's +250–400: …)`
- `epic-16…:106` OLD `15-2 allowances incl. maos-cli +700` → NEW `15-2 allowances incl. maos-cli **+1100**`
- `epic-20…:47` OLD `— **note 16-1 books +700 of the same +700 grant**` → NEW `— 16-1 books +400–700 of the same **+1100** grant (raised 2026-09-05); 20-1 draws the remainder and re-measures at Epic-20 open`

**D6 · `epic-15-foundations-w0.md:145` — the story order was corrected in the proposal but not in the file.**
Proposal §4 ("Order corrected: 15-2 → 15-3 → 15-1") and §7 ("opens with 15-2 … then 15-3 … then 15-1") vs the file's suggested order.
OLD → `Suggested order: 15-2 → 15-1 → 15-3 → {15-6, 15-5 in parallel} → 15-4`
NEW → `Suggested order: 15-2 → 15-3 → 15-1 → {15-6, 15-5 in parallel} → 15-4` *(the `maos --help` verb table is the lever rule 1 rests on, and 15-1's aggregate-green depends on nothing 15-3 changes)*

**D7 · `epic-18-spirits-that-think-w3.md:17, :81` — `worker_spawn.rs:53-66` accepts three flags, not two.**
`:54 --live`, `:55 --once`, `:59 --replay-llm` (an accepted no-op). Epic 21 states the flag set correctly (`epic-21…:52`); Epic 18 states it wrong twice, and the omitted flag is exactly the one D-D retires.
OLD (both sites) → `parser \`worker_spawn.rs:53-66\` accepts only \`--live\`/\`--once\` @9f920180`
NEW → `parser \`worker_spawn.rs:53-66\` accepts only \`--live\` (\`:54\`), \`--once\` (\`:55\`) and the \`--replay-llm\` no-op (\`:56-59\`, retired by 15-6 under D-D) @9f920180`

**D8 · `epic-18-spirits-that-think-w3.md:20-25` — line 4 has no provenance bullet, and the "Shape" line is stale.**
Round 3 added exit line 4; the provenance list still stops at "Line 3" and then says "The job runs **the three lines** plus the two twins". Its shape claim is also false for line 2 (`maos eval halt --class butler` carries no `ENV=` prefix).
NEW bullet after `- Line 3 —` → `- Line 4 — selector tokens as line 1; \`--clock compressed:<dur>\`, \`--acceptance <path>\` and \`tests/fixtures/j-butler/acceptance-21d.jsonl\` are **created by 18-4 AC1/AC2** (\`ls tests/fixtures/\` = \`anthropic-skill anthropic-skill-invalid dirty-network-fixture wasm\`, no \`j-butler/\`; the parser accepts only \`--live\`/\`--once\`/\`--replay-llm\`, \`worker_spawn.rs:53-66\`). **HEAD: red** — \`maos run: unknown argument '--clock'\` (\`worker_spawn.rs:63-68\`), exit 1. The \`Clock\` port is 18-4's and is re-used by 21-1.`
OLD → `every line is \`ENV=… command args # comment\`; … The job runs the three lines plus the two twins` → NEW → `lines 1, 3 and 4 are \`ENV=… command args # comment\`; line 2 is a bare \`maos\` verb. … The job runs **the four lines** plus the two twins`

**D9 · `epic-20-ship-it-w5.md:93` and `:102` — two FALSE `sprint-status.yaml` line citations.**
- `E12-B1 (\`sprint-status.yaml:300\`, "remaining reach gap … tracked forward")` — line 300 is **blank**; the text lives at `sprint-status.yaml:309`. OLD `:300` → NEW `:309`.
- `Bedrock/Vertex/Vault are \`later-bedrock-vertex-and-kms-backends\` (\`sprint-status.yaml:285\`)` — line 285 is `ops-first-signed-tag`; the row is at `:294`. OLD `:285` → NEW `:294`.

**D10 · `epic-17-workers-and-third-party-form-w2.md:22` — the componentize-js inventory grep does not reproduce.**
`git grep -il componentize 9f920180` outside `node_modules`/`target` returns **8** files (all planning artifacts: `sprint-status.yaml`, `correct-course-input-2026-09-04-confidence.md`, this epic, `epics/index.md`, `requirements-inventory.md`, two PRD files, `…round2.md`). The substance ("absent from the code and toolchain") is true; the stated grep is false, and rule 8 requires the grep that produced the count.
OLD → `componentize-js absent from the repo, grep outside \`node_modules\`/\`target\` empty`
NEW → `componentize-js absent from the code and toolchain — \`git grep -il componentize -- crates spirits sdks examples xtask templates '*.json'\` = 0 @9f920180 (the 8 repo-wide hits are all \`_bmad-output\` planning prose)`

**D11 · `epic-16-one-daemon-one-door-j0-w1.md:13` — the ambiguity-tag literal is at `:148`, not `:150`.**
`crates/maos-spirit-hello/src/lib.rs:148` is `"task.acceptance_criterion.ambiguous",`; `:150` is the prompt format string. `epic-19…:82` cites `:148` correctly — the two files disagree on the spelling's home. (The tag spelling itself is consistent everywhere: `task.…` in E16/E19, `story.…` only as the three `lcas.rs:84,124,164` seeds E19 AC2 re-spells. Verified at HEAD: `task.` ×2 sites, `story.` ×3 seeds.)
OLD → `tag literal \`crates/maos-spirit-hello/src/lib.rs:150\`` → NEW → `tag literal \`crates/maos-spirit-hello/src/lib.rs:148\``

### Minor defects (fix at `bmad-create-story` time)

| File:line | Wrong | Fix |
|---|---|---|
| `epic-15…:20, :56` | "6 violations (**five structs + two** undocumented-register rows)" — 5+2=7 | `6 violations (four structs + two undocumented-register rows)` — reconcile against the AC2 symbol list (4 symbols named) |
| `epic-15…:78` | xtask row says `19 +150 (\`demo-j1 --replay\`)` — D-D retired that flag; Epic 19's own kloc line says "cassette env forwarding" | `19 +150 (cassette env forwarding + beat re-labelling)` |
| `epic-15…:79` vs `epic-21…:26` | kernel-core row says `21-5 ≈ −1000` (physical) where 21-5 is priced `−817 tokei / −1028 physical` | `21-5 ≈ −817 tokei (−1028 physical), ceiling lowered by the same amount` |
| `epic-16…:13` | "`check-exit-commands` … resolves only `maos` / `maosctl` / `cargo run -p xtask --` tokens" — omits the `cargo test --test` rule its own line 2 needs | add `and \`cargo test -p <crate> --test <name>\` (resolved against \`ls crates/<crate>/tests\`)` |
| `epic-16…:30` | butler "constructed `main.rs:447`" — `:447` is the `classify_spirit` match arm | `classified \`main.rs:447\`, constructed in the Butler arm \`main.rs:4188\`` |
| `epic-17…:16` | exit line 2 runs `target/debug/maos run …` — a path token 15-3 AC3 has no rule for | resolve it via D2's amended tokeniser, or state it as a `cargo run -p maos-bin --` form |
| `epic-17…:53` | "admitted as T3 at `worker_spawn.rs:596-603`" — those lines are the liveness-probe admission `eprintln!`; the tier assertion is upstream | re-cite the tier-assertion line, or say "the admission block at `:588-603`" |
| `epic-20…:93` | `EXPECTED_GATES` cited as `check_ship_gate_completeness.rs:20-60` — the array runs `:20-102` (40 entries, verified) | `:20-102` |
| `epic-20…:34` | "the shell dispatches only the in-proc hello-spirit (`maos-shell/src/lib.rs:164,:176-200`)" — `:201` dispatches butler | `dispatches only the in-proc hello-spirit and the v0.3-β butler prototype (\`:176-201\`)` |
| `epic-21…:53` | Mira's "declares no `[capabilities.required]`" comment is at `manifest.toml:17-18`, cited `:16-17` (Nash's `:14-15` is exact) | `:17-18` |
| `epic-21…:20, :64` | `grep -n '#\[ignore' crates/maos-journey-test/tests/*.rs` returns **3** lines — `journey_j4.rs:108` is a doc comment mentioning `#[ignore]` | state the grep as `'#\[ignore = '` (2 hits), which is also what 21-2 AC4's audit must match |
| `epic-21…:3` | "Replaces the morning's **Epic 20** file" | `Epic 21` (pre-existing, not a Round-3 edit) |

### Not defects — checked and cleared

`--replay` appears nowhere as a live flag: every occurrence in E15/E16/E18/E19/E21 is inside a retirement sentence ("no `--replay` flag", "`--replay-llm` … retired"), except the one stale kloc parenthesis at `epic-15…:78` above. `--deterministic` appears only where E18/E15 record its refusal under D-D. `MAOS_INFERENCE_MODE` + `MAOS_REPLAY_CASSETTE` is the sole selector in all six files that name one; `MAOS_REPLAY_STRICT=1` is used only where a drift/exhaustion red is claimed (E18 ×3, E21 ×1) and both variables exist at HEAD (`env_contract.rs:255,265`) while `MAOS_INFERENCE_MODE` correctly greps to 0. `maos-egress` and `maos-orchestrator-buffer` each have a 15-2 row and one consuming epic; the `= 917` figure matches 817 + max(100, ⌈2 %⌉). The `Clock` port is created once (18-4 AC2) and re-used once (21-1 AC1) with the edge declared in both Dependencies lines. `check-j1-loopback-delegation`'s narrow-suffix fix is owned by 15-1 AC1 and disclaimed by 19-1 AC2 and 19-4 AC1. Every story key named in a Dependencies line exists in the tracker; the dependency graph is acyclic (15 → {16,17,18,19,20,21}; 16 → {17,19,20}; 17-3b → 20; 18-4 → 21-1; 20-{2,3} → 21; no back-edge). Kernel figures agree file-to-file: 15-1 +2 (re-pin 24472→24474), 16-5 +5–25, 17-1 +65–130, 17-2 net ≤+10, 19-3 +3–5, 21-5 negative — total positive ask 172 ≤ the +250 D-F grant.

## 3. Projected confidence

| Epic | R2 | R2's post-edit projection | **Now** | One sentence |
|---|---|---|---|---|
| 15 Foundations | 55 | 70 | **66** | The exit block is real and its three lines' provenance is exact, but the file that authors the ADRs states D-C as the design D-C rejects, inverts its own ratified story order, and carries the lane's only FALSE count (`136 needs:`). |
| 16 One Daemon, One Door | 48 | 60–65 | **64** | Every one of the seven exit lines resolves against a verb that exists or an AC that creates it, and the re-counted inventories (18 sites/21 values, 9 files/42 tests, 17==17) reproduce exactly; only the tag-literal line and a stale +700 stand between it and its target. |
| 17 Workers + third-party form | 38 | 50–55 | **55** | D-B and D-C are written as single, buildable designs with the piped `spawn_t3` route and the kernel-Δ 0 argument stated by construction; the residue is one grep that does not reproduce and two exit tokens the 15-3 gate cannot yet resolve. |
| 18 Spirits That Think | 40 | 55 | **52** | The four exit lines are the right four and line 4 finally gives J-Butler a machine-checked leg, but that line arrived without a provenance bullet, left "the three lines" behind it, and rests on a flag-set claim that is wrong at both sites where it is made. |
| 19 Founder Loop | 40 | 55–60 | **62** | The cleanest file in the lane: one hermetic command, every token verified at HEAD, the red reproduced and assigned to the story that owns it, and the unreachable target state replaced by one that can actually occur. |
| 20 Ship It | 42 | 55 | **54** | The gate arithmetic is the most rigorous work in the round — 6406, 42, 18, 19/17 all recompute — yet the epic's own nine-line exit block is shell that the rule-1 gate cannot parse, and two tracker citations point at the wrong lines. |
| 21 Multi-host on live substrate | 42 | 55 | **58** | Two exit lines, both verified red at HEAD for the reason stated, and the measurement discipline is the round's best (1028/817, 8-line placeholder, 87/507, the 15 kernel-core names) — the defects are all off-by-one. |

**Lane:** 52–66, mean ≈ 59, against R2's 38–55 (mean 44). The gain is real and came from executing the exit blocks instead of writing them: all seven now `bash -n` clean, and 121 of the 124 provenance tokens sampled resolve at `9f920180` exactly as claimed.

**What did not get fixed by Round 3, in one line:** the round proved every command it wrote, and then wrote the one decision (D-C) and the one order (15-2 → 15-3 → 15-1) that no command checks — *a rule that says "run the command" leaves the prose unguarded, and the prose is where the design lives.*
