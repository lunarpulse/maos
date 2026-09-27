---
baseline_commit: "**`92911f59`** (main; working tree CLEAN; `v0.1.0-alpha.1` released). Authored 2026-09-25 from five scouts (WASM host + WIT, TS toolchain + spike precedent, recall path, podman/egress, planning provenance) plus direct measurement: `cargo run -q -p xtask -- kloc-check --json` → `passed: true, alarm: true, aggregate: 167819`; `tokei --types Rust spikes` → **144** code lines against the `\"(unknown:spikes)\" = 244` ceiling (`xtask/kloc.toml:600`); kernel pin `src_lines = 24920` (`xtask/kernel-core-baseline.toml:505`). **Cites are SYMBOL-first, line second**; every cite below was re-measured at `92911f59` and supersedes the epic's `e868ff28`/`9f920180` cites where they differ (OLD→NEW in §Premise corrections)."
depends_on: "Nothing open. ADR-060 (`docs/adr/ADR-060-spirit-forms-by-trust-tier.md`, D-C §`:96`) and ADR-061 (`docs/adr/ADR-061-worker-egress-and-scoped-credential.md`) are ACCEPTED (Story 15-5, `done`). The epic's Dependencies line (`epic-17-workers-and-third-party-form-w2.md:199`) orders **17-3a FIRST** — before 17-1 (it prices the proxy) and before 17-3b; `epic-16-retro-2026-09-20.md:200` confirms: *17-3a is the correct first story (rule 5: it prices the proxy, and it is kernel-Δ 0, budget-free and blocked by nothing).*"
blocks: "**`17-1-worker-egress-allowlist-and-scoped-credential`** may not leave `backlog` until Q6 has a MEASURED verdict (`epic-17…:199`; `preflight-r2/epic-17.md:176` *the spike should price both before 17-1 leaves backlog*). **`17-3b-wasm-third-party-form-with-log-recall`** may not leave `backlog` until Q1–Q5 have verdicts (`epic-17…:86` *17-3a AC1 (4) proves it with a stub guest before this story leaves backlog*). ⚠ **RT-8, operator decision 2026-09-25:** the T2 repair is the new story **`17-6-t2-sandbox-repair-and-proven-red`**, which may not leave `backlog` until Q4b has a MEASURED verdict (it spends exactly the kernel lines Q4b prices — E16-A7); `17-2-worker-cgroups-applied` and `17-3b` wait on 17-6 — T2 cannot start any program today (P21), 17-2 AC1 needs a live *T2 fixture child*, and 17-3b's S2-for-WASM needs the runner under T2."
spec_alignment: "Rule 9 applies mid-epic: this spike exists to disprove premises, and **it has already disproved several at creation time** (§Premise corrections P1–P21 — e.g. T2 cannot start ANY program where Landlock is enforced and a Rust binary could not reach `main` under its seccomp allowlist anyway (P21, P19 — measured), `spawn_and_bridge` applies NO sandbox, the T2 seccomp allowlist has no `socket`/`connect`, `LogRecallFilter` has no `spirit_pid` field, the provenance gate pins 4 components not 8). **§R5 part A** (the premises disproved at creation, P1–P21) was filed into the epic file by the 2026-09-25 preflight round-table (RT-5); the dev pass appends **§R5 part B** (the measured spike outcomes) in the SAME commit as the go/no-go (precedent: 16-6 `spec_alignment`, *THE EPIC IS AMENDED IN THE SAME COMMIT*). The spike measures and prices; it does NOT re-decide D-B/D-C (rule 7 closed those forks as ADRs) — a NO-GO names the ADR clause it falsifies and routes the re-decision to the operator."
split_from: "Not a split. Sizing: **≤3 days, fixed** (`epic-17…:47`; `preflight-r2/epic-17.md:172` *≤3 d → 0.6 wk*). Timebox is binding: an unanswered question is recorded NOT-MEASURED with its exact blocker — never answered by inference."
kernel_grant: "**NONE. Kernel-Δ 0** (`epic-17…:29`). Zero lines under `crates/maos-kernel-core/src`; `cargo run -p xtask -- check-kernel-baseline` must pass unchanged at 24920 with the 98-entry content-hash set untouched. Spike code may CALL `pub` kernel APIs (`spawn_sandboxed`, re-exported `LogRecallAdapter`) through path dependencies from an out-of-workspace spike crate; it may not EDIT them. The T2 repair (Q4b, P19/P21) is PRICED by expressing the candidate Landlock ruleset and seccomp filter in spike code (copies of `linux.rs:163-202` / `:208-347` plus additions) — the priced lines are `17-6-t2-sandbox-repair-and-proven-red`'s FLAG-Winston grant (RT-8, operator decision 2026-09-25), never spent here."
kloc_grant: "**NONE requested.** Measured headroom: `(unknown:spikes)` 144/244 = **+100 Rust code lines** (tokei `--types Rust`, `-e tests -e target …`, `xtask/src/kloc_check.rs:294-318`); aggregate 167819 vs `_aggregate_hardfail = 170884` (`kloc.toml:658`) = **+3065**, which is Epic 17's BUILD budget (Σ ask +940…+1920, `epic-17…:33`) and must not be spent on spike code. Rust beyond +100 lines is committed as `*.rs.txt` (the 11-0 idiom: `spikes/story-11-0-wasm-host/composition_root.rs.txt`). TS/JS/WIT/TOML/shell are not counted (Rust-only instrument)."
model: "frontier-class allowlist {opus-4-6, opus-4-7, opus-4-8, gpt-5.5, gpt-5.6, glm-5.1, glm-5.2, glm-5.3, opus-5, equiv}. `FRONTIER_FAMILIES` (`xtask/src/check_dev_model_tier.rs`) is the machine-checked set; `equiv` is prose, NOT a match token. Keep the literal `allowlist {` — it is a NEGATIVE marker: `check_dev_model_used_populated.rs:302` treats any line containing it as boilerplate and SKIPS it."
review: "§A6 net **binding** (epic `:37`), not ◆ (no production code ships). Layers: Blind + Acceptance + **Evidence re-execution** (a non-author re-runs the Q4 relay, the Q5 instantiate probe and the Q1 runner probe from the committed spike sources and reproduces the recorded verdicts) + Edge Case over the verdict table. E15-A6 question binding by name on every probe: *does this probe read the tree/runtime, or only what it set itself?* — e.g. a Q5 probe that bindgen's the host and the guest from the SAME edited WIT proves nothing about skew."
---

# 17-3a — Spike: WASM recall side-channel, WIT versioning, componentize-js and the 17-1 proxy

Status: done

> **The capability:** *Before anyone sizes 17-1 or 17-3b, the six premises they stand on are measured on a
> running system: a TypeScript Spirit becomes a `maos:spirit` component the shipped runner executes; a recall
> import can be added without breaking `@1.0.0` guests; `frame_bridge` can carry the fields it drops today; a
> guest's recall reaches a daemon-side service with zero kernel lines, under the sandbox the runner will actually
> run in; and a `--network=none` Worker container can reach a host-side proxy — or the spike says, with
> transcripts, exactly which of these is false and what it costs.*

**Closes:** confidence **rule 5** for Epic 17 (`correct-course-input-2026-09-04-confidence.md:27`: *≤3-day spike story
with a written go/no-go before the build story is sized — the 11-0 precedent*) · the single NOT-CHECKED item the R2
preflight handed here (`preflight-r2/epic-17.md:152`: *whether componentize-js bundles its StarlingMonkey engine*).

**Does NOT close, and says so:** FR29 / FR33 / FR5-by-form (17-3b) · any production code (AC2) · the decision
itself — D-B and D-C are ACCEPTED ADRs; a NO-GO is escalated, not re-decided here.

## Story

As the **Epic-17 build owners (Winston — FLAG-Winston pricing; Amelia — 17-3b)**,
I want **six MEASURED go/no-go verdicts, each with transcript evidence, a line/CI price and its consequence for
17-1/17-3b**,
so that **17-1 and 17-3b are sized from a running prototype rather than from premises the tree already disproves
(the 11-0 spike answered by code survey and left its runtime proof to 11-1a — this one may not).**

## Acceptance Criteria

**AC1 (command / deliverable).** §Go/No-Go below is filled for all six questions. Each row carries: **verdict**
∈ {`GO`, `GO-WITH-CONDITIONS`, `NO-GO`, `NOT-MEASURED`}; the **exact command(s)** run and a ≤20-line output
excerpt, with the full log committed under `spikes/story-17-3a-wasm-recall/evidence/`; the **price** (lines per
crate incl. `maos-kernel-core`, new deps, CI minutes/steps); and the **consequence** for 17-1/17-3b as OLD→NEW text.
*Measured* means executed and observed; an answer derived from reading code is `NOT-MEASURED` (the 11-0 lesson,
`story-11-0-wasm-host-spike.md:48` *the spike did NOT build a running wasmtime stack*). The six questions, tightened
to HEAD:

1. **componentize-js.** A TS Spirit under `spikes/…/ts-guest/` (sources may import from
   `examples/example-spirit-ts`, which today exports only `onIdle`, `src/index.ts`) is compiled `tsc` → ESM →
   `@bytecodealliance/componentize-js` (**pin exact `0.23.0`**, 2026-09-21) against the **pinned `@1.0.0` WIT itself**
   — `--wit wit/` read-only (the directory holds only `spirit.wit`; `package maos:spirit@1.0.0;` `:18`, world `spirit`
   `:219-230`), never the `@1.1.0` copy — and the **UNMODIFIED `maos-wasm-runner` built at
   HEAD** (`cargo build -p maos-wasm-host --bin maos-wasm-runner`) runs it: `on-start`, then one `handle-frame` fed a
   real Content-Length + canonical-CBOR ADR-032 frame on stdin, exit 0 and a decodable emitted frame on stdout.
   Record: component bytes; `Component::new` wall time vs `COMPILE_TIMEOUT` = 10 s (`runner.rs:140`); fuel per
   `handle-frame` vs default 10,000,000 (`runner.rs:109`); that `console.log` in the guest does NOT corrupt the
   stdout frame stream; the `npm ci` network footprint (registry-only, or post-install binary fetches?); Node 20
   (CI pin, `discipline.yml:1086-1088`) vs Node 22/24; whether StarlingMonkey is embedded (answers
   `preflight-r2/epic-17.md:152`).
2. **Additive WIT.** On a spike COPY of the WIT, `package maos:spirit@1.1.0` adds `interface recall`
   (`log.recall`/`log.fetch` shapes mirrored from `crates/maos-domain/src/log_recall.rs`) and `import recall;` in
   world `spirit`. Record: the `wit_corpus.rs` constants/rows that would change (hard-coded totals `:113-160`,
   per-record field table `:181-186`); the provenance cost — **4** pinned components, not 8 (P1) — including whether a
   rebuild of the 4 at HEAD **reproduces their pinned `wasm_sha256`** (`tests/fixtures/wasm/equiv-fixtures.provenance.toml`);
   and the `check-wasm-form-equiv` disposition that a 17-3b WIT edit must keep green (P3).
3. **`frame_bridge`.** On the spike WIT copy, `iac-frame` carries `intent`, `consent-envelope`, `intent-lineage`
   and a lossless `scope`; a spike copy of `lower`/`lift` round-trips a frame with all four populated
   byte-identically through CBOR (today: dropped at `lower` `frame_bridge.rs:403-415`; defaulted at `lift`
   `:427`/`:430`/`:431`; scope lowered as a Debug string `:216` and lifted through `scope_from_debug_string`, a
   literal `None`, `:395-397`). Price the WIT record growth into Q2's corpus/provenance tax.
4. **Recall relay, zero kernel lines — under the real sandbox.** A stub guest (Rust, `wasm32-wasip2`, the in-tree
   guest idiom) calls the `recall` import; a spike copy of the runner implements that import as a client of a
   **spike recall service** that wraps `LogRecallAdapter` (`crates/maos-iac/src/adapter/log_recall.rs:62-73`) over a
   `TransparencyLogAdapter` seeded with frames for two pids. Record, separately:
   - **(4a) transport:** round-trip latency for recall + fetch over a unix socket whose path the service chose (D-C
     as written, ADR-060 §D-C `:96-106`);
   - **(4b) sandbox — what T2 must become for the runner to run under it.** T2 as shipped cannot start ANY
     program (P21, measured at creation) and would abort any Rust program before `main` even if it could (P19,
     measured by emulation). The spike prices the repair as a four-layer ladder, each layer measured with the
     **candidate policy expressed in spike code** — a copy of `prepare_landlock` (`linux.rs:163-202`) and
     `build_seccomp_filters` (`:208-347`) plus the proposed additions — never by editing `linux.rs` (kernel-Δ 0):
     1. **Landlock:** the minimal rights that let a program start under the V1 handle-all ruleset — `Execute` +
        `ReadFile` on the resolved program, `ReadFile` on the dynamic loader and each `ldd` library, vs a
        statically linked runner (`-C target-feature=+crt-static`) that needs only its own file. Measurable
        locally (Landlock enforces on this host — P21 harness).
     2. **seccomp:** the minimal allowlist additions for Rust std start (at least `poll`/`ppoll` and
        `rt_sigaction`, P19) and then for the runner (threads: `clone3`; `set_robust_list`, `gettid`/`tgkill`,
        `sched_getaffinity`, …) — measured **in CI** under the real filter (RT-2; this host cannot nest seccomp,
        `Seccomp: 2` already) with `SECCOMP_RET_LOG` or `strace -f` + syscall injection as the ground truth.
     3. **Runner-side mitigations** that shrink layer 2: `Config::parallel_compilation(false)`,
        `signals_based_traps(false)`, compile outside the sandbox + `Component::deserialize`, no compile-watchdog
        thread (`runner.rs:200-205`) — each measured, not assumed.
     4. **Transport:** under the repaired candidate policy, does the recall channel connect? If a path socket
        cannot (no `socket`/`connect`), measure the **pre-connected fd** alternative (socketpair created by the
        service side, one end inherited by the runner) and price where the fd-passing line lives —
        `spawn_and_bridge` builds the `Command` inside the kernel (`runtime.rs:453`, `:465`), so an inherited fd
        is a kernel line unless the spike proves otherwise (P9).
     Every Landlock right and allowlist line that survives layers 1–3 is priced as an **exact count of kernel
     lines in `linux.rs`** (FLAG-Winston; E16-A7: the forecast is measured, not ranged). The vehicle that spends
     them is `17-6-t2-sandbox-repair-and-proven-red` (RT-8, operator decision 2026-09-25), never 17-3b;
   - **(4c) identity:** the caller pid is bound by the channel (per-spawn socket/fd), never supplied by the guest —
     `LogRecallFilter` has no pid field (P7). Recall as pid A returns only A's frames; `fetch` of B's frame id as A
     returns the typed `ScopeViolation` mapped to a WIT `recall-error` variant (domain error `log_recall.rs:341-345`,
     raised at adapter `:483-487`). **Hostile-connector vector (RT-3):** a second, unrelated process running as the
     same uid connects to A's channel (path socket: by path; fd: by `/proc/<runner>/fd` if reachable) — it must get
     nothing, or the verdict records exactly what it got. Same-uid is the real threat: a Worker runs as the operator
     uid (`sprint-status.yaml` 16-6 R3 chain, `control.json` readable at 0600);
   - **(4d) kernel-Δ:** `git diff --stat 92911f59 -- crates/maos-kernel-core/src` empty and `check-kernel-baseline`
     green, pasted.
5. **Version skew.** The committed `@1.0.0` component `tests/fixtures/wasm/echo_spirit_component.wasm` (built from
   the pinned `wit/spirit.wit`) is instantiated by a spike host `bindgen!`'d from the **`@1.1.0`** copy — result
   recorded (exit code; runner maps instantiate failure to exit 3, `runner.rs:56,:172`). Also record the reverse: a
   `@1.1.0` guest that imports `recall` against the HEAD runner (expected: unknown-import failure). Before either,
   `wasm-tools component wit` on the fixture records the actual import/export names on the wire (P12). If `@1.0.0`
   does not instantiate, "additive" is FALSE and 17-3b must re-issue the equiv fixtures (`preflight-r2/epic-17.md:184`).
6. **17-1 egress proxy price.** Measured **in CI on `ubuntu-latest`** (the runner 17-1's `epic-17-exit` job will
   use; podman there is **4.9.3** since 2026-09-06, P15), plus locally if rootless podman works in the dev
   environment. For each mechanism M1–M3 (Dev Notes §Q6) record: `podman --version` + `podman info` network
   backend; host-proxy reachable (yes/no); **internet reachable (must be NO**, ADR-061 `:59-61` *internet egress
   remains denied*); whether the ADR-061 gate literal `"--network=none".to_string()` in `t3/argv.rs` survives
   (`xtask/tests/decision_adrs_and_provisioning.rs:313-327`); argv lines added to `t3/argv.rs` (kernel lines);
   T3-image content required (e.g. an in-container forwarder → `t3-image.lock` re-sign); the network-escape
   corpus fixtures that would need re-authoring (5, P17). Verdict names ONE mechanism for 17-1 with its kernel-Δ
   figure against the granted +65–130 (`epic-17…:29`).

**AC2.** Spike code lives under **`spikes/story-17-3a-wasm-recall/`** (`spikes/story-11-0-wasm-host/` precedent;
tracker *HOLD-WINDOW no-merge spike*, `sprint-status.yaml:150`), never merged to `crates/`. Tightened: the story's
landing commit touches **no** path under `crates/`, `wit/`, `tests/fixtures/`, `sdks/`, `examples/`, `templates/`,
`spirits/` or `.github/workflows/` (the Q6 CI probe workflow exists only on a throwaway branch); at the landing commit
`check-kernel-baseline`, `check-equiv-fixture-provenance` and `cargo fmt --all -- --check` give results identical to `92911f59`, and
`kloc-check` with every row identical except `(unknown:spikes)` ≤ 244 and the aggregate moved by exactly that
row's delta; no `.wasm` larger than 1 MiB is committed (record `sha256` + size instead; the
componentize-js output embeds StarlingMonkey, README: *around 8MB*); `node_modules/`, `dist/` and any `target/`
under the spike are git-ignored; no default-feature or `Cargo.toml` change anywhere (Hold 2:
`check_wasm_host_absent_from_default` stays green, `RELEASE-HOLDS.md:28,:32-38`).

## Tasks / Subtasks

Order is cheapest-highest-information first; stop at the timebox and record the rest as `NOT-MEASURED`.

- [x] **T0 — Baseline (AC2).** Record HEAD; re-run the two creation-time experiments (Dev Notes) and commit them under `evidence/`; run and paste: `cargo run -p xtask -- check-kernel-baseline`,
  `cargo run -q -p xtask -- kloc-check --json` (expect aggregate 167819, `(unknown:spikes)` 144),
  `cargo run -p xtask -- check-equiv-fixture-provenance --json`. These are the "same results" AC2 compares against.
- [x] **T1 — Toolchain (AC1 all).** `rustup target add wasm32-wasip2`; `cargo install --locked wasm-tools`
  (record version); in `spikes/…/ts-guest/`: `npm i -E @bytecodealliance/componentize-js@0.23.0 @bytecodealliance/jco@1.35.0 typescript@5`
  (commit `package.json` + `package-lock.json`, git-ignore `node_modules/`). Record `node --version` (local 20.19.2).
- [x] **T2 — Q5 version skew (AC1-5).** `wasm-tools component wit tests/fixtures/wasm/echo_spirit_component.wasm`
  → record import/export names. Create `spikes/…/wit-1.1/spirit.wit` (copy + `@1.1.0` + `interface recall`).
  Spike host = out-of-workspace crate (`[workspace]` table in its own `Cargo.toml`, the guest idiom
  `crates/maos-wasm-host/guests/echo-spirit/Cargo.toml`) with `wasmtime = "=46.0.3"` (HEAD lock,
  `Cargo.lock:6131-6133`) + `wasmtime-wasi` 46, `bindgen!` over the 1.1 copy, `add_to_linker_sync` + a stub
  `recall` host impl. Instantiate the untouched fixture; then the reverse direction against the HEAD runner.
- [x] **T3 — Q1 componentize-js (AC1-1).** `jco guest-types wit/ --world-name spirit` for `.d.ts` (the pinned
  `@1.0.0` WIT, read-only — NOT the `wit-1.1` copy, or Q1 fails for Q5's reason); write `ts-guest/src/spirit.ts`
  exporting `handleFrame`, `onStart`, `onShutdown` (WIT record → object, `list<u8>` → `Uint8Array`, `u64` →
  `bigint`, `result` err → thrown value — verify against generated types); `tsc`, bundle if bare specifiers remain
  (componentize-js resolves relative imports only); `npx componentize-js dist/spirit.js --wit ../../../wit
  --world-name spirit --disable http fetch-event -o ../out/ts-spirit.wasm` (0.23.0 CLI: `-d, --disable <feature...>`,
  choices = `DEFAULT_FEATURES`; HEAD runner links
  WASI p2 only via `add_to_linker_sync`, `runner.rs:164-168` — no `wasi:http`). Drive the HEAD runner with a
  framed input: `maos_wasm_host::codec::{encode_cbor, write_frame}` (`codec.rs:132`, `:125`) over a real
  `maos_domain::frame::IacFrame` (or a frame taken from `equiv_harness.rs`) via a spike driver; decode stdout with
  `read_frame`/`decode_cbor` (`:39`, `:139`). Record all Q1 metrics.
- [x] **T4 — Q2 + Q3 on the WIT copy (AC1-2, AC1-3).**
  - [x] Draft `interface recall` (sketch in Dev Notes); note FrameKindLabel (30 variants, `#[non_exhaustive]`,
        `log_recall.rs:41-90`) ≠ WIT `frame-kind` (17 cases) — decide label enum vs reuse, record why.
  - [x] Add `intent`/`consent-envelope`/`intent-lineage`/lossless `scope` to the copy's `iac-frame`; spike copy of
        `lower`/`lift`; CBOR byte-equality round trip with all four populated.
  - [x] Rebuild the 4 pinned components at HEAD (`cargo build --release --target wasm32-wasip2 --manifest-path
        crates/maos-wasm-host/guests/<g>/Cargo.toml` into a scratch target dir) and compare to `wasm_sha256`
        — reproducible or not. **Do not write** the rebuilt blobs or new hashes into the tree.
  - [x] Enumerate the exact `wit_corpus.rs` edits a 17-3b WIT change needs (count gates `:113`, `:124`, `:135`,
        `:140`, `:146`, `:156`, table `:181-186`).
- [x] **T5 — Q4 relay (AC1-4).**
  - [x] Stub guest (Rust, `wit-bindgen = "0.44"` like the in-tree guests) importing `recall`.
  - [x] Spike recall service: `std::os::unix::net::UnixListener` (maos-bin's tokio has no `net` feature,
        `crates/maos-bin/Cargo.toml:89`; the `air-gap` feature compiles out `tokio::net`, `:28`), socket file 0600 in a
        0700 dir (custody precedent `maos-domain/src/operator_door.rs:37,:39-41,:441-460`); `TransparencyLogAdapter`
        via the `pub` constructors (`open_in_memory` `transparency_log.rs:560` or `open_read_only` `:419`), seeded
        with frames for pid A and B (`insert_frame_event` `:605`); `LogRecallAdapter::new` + `LogRecallPort::{recall,fetch}`
        (audited path; `recall` journals a CapabilityInvocation row first, `log_recall.rs:368-378`). Wire = CBOR/JSON
        of the serde domain types; `LogRecallError` has no serde (`log_recall.rs:332`) — hand-map to `recall-error`.
  - [x] (4a) latency; (4b) launch through `spawn_sandboxed` T2 from a spike harness (in the T6 CI job); if the
        socket path fails, measure the inherited-fd variant; (4c) pid-binding + `ScopeViolation` + hostile
        same-uid connector; (4d) kernel diff + baseline gate.
- [x] **T6 — ONE throwaway CI probe for Q4b + Q6 (AC1-4, AC1-6).** ✅ **AUTHORIZED by Lunarpulse, 2026-09-25.** Push a
  throwaway branch (`spike/17-3a-ci-probe`) with a temporary workflow (`on: push: branches: [spike/17-3a-ci-probe]` —
  `workflow_dispatch` is only exposed for default-branch workflows, `sprint-status.yaml` E15-A3 note); the dev agent
  pushes it over SSH (verified 2026-09-25: `ssh -T git@github.com` authenticates as `lunarpulse`; precedent
  `ops-darwin-rehearsal` *fixed on a throwaway branch (green run 36085769919)*). Two jobs, each on a two-label matrix
  **`ubuntu-24.04` + `ubuntu-26.04`**, never `ubuntu-latest`: that label moves to 26.04 (kernel 6.17 → 7.0) between
  2026-10-19 and 2026-11-19 (actions/runner-images#14748; job 107944202958 already carries the notice), so a verdict
  measured on one image would be re-litigated on the next — and 17-6 ships on both. **No `continue-on-error`** and no
  `|| echo` (the T3 jobs' `continue-on-error` + `|| echo skipped`, `discipline.yml:1590-1591` and `:1625-1626`, is
  exactly what made T3 failures invisible): **(a) `t2-probe`** — build the forbidden-syscall probe as `wasm-host-tests`
  does (`:2925-2926`), run `cargo test -p maos-wasm-host --test t2_sandbox_kill -- --nocapture` and **grep the log for
  `SKIP` / `landlock` / `seccomp apply failed`**, recording which layer refused (confirms or refutes P20 on CI's own
  kernel; without `--nocapture` libtest captures the `SKIP` line — P20 records what the operator's empty grep of job
  107944202958 does and does not show); then the (4b) ladder with the **candidate policy in spike code** (RT-1): as-is
  kernel ruleset (expect `EACCES`, P21) → + Landlock rights → + seccomp additions, for a hello-world Rust binary, then
  the runner, then the recall transport — exit status and signal recorded at every rung (EACCES ≠ SIGABRT ≠ SIGSEGV ≠
  SIGSYS ≠ 0); **(b) `egress-probe`** — `sudo apt-get install -y podman` exactly as `t3-smoke-busybox` (`:1614-1617`),
  print `podman --version` + `podman info`, run M1–M3 against a host-loopback listener (a trivial HTTP responder
  standing in for the proxy) and an internet probe. **Every verdict is also a workflow annotation** — one
  `::notice title=<job>/<image>::<rung> rc=<n> sig=<n>` per rung or mechanism, `::error` when a rung contradicts its
  expectation, ≤10 per step (GitHub truncates annotations per step) — because this environment holds no GitHub token:
  a job log is `403` unauthenticated, while a job's step conclusions and
  `GET /repos/lunarpulse/maos/check-runs/<job_id>/annotations` are public (both verified 2026-09-25 against job
  107944202958). Record run ids, annotations and the logs under `evidence/` (read with the D5 read-only token once it is installed, operator-downloaded until then); delete the branch
  afterwards; the workflow text is committed only as `spikes/…/ci-probe.yml.txt`. If the run itself cannot execute
  (runner outage, quota), Q4b and Q6 are `NOT-MEASURED` with that blocker (RT-7), local-only numbers attached as
  context.
  ✅ **DONE 2026-09-26** — run #9 `36210072648` (head `25981d48`): both `t2-probe` jobs SUCCESS, steps 7–17 green
  on both images, every rung an annotation (`evidence/t6-ci-run9-t2-annotations.log`). Run #10 `36210776858`
  (head `1389fd4b`, the terminated slice's last iteration) adds `runner-drop-ppoll leave-one-out-still-succeeded`
  and a per-syscall enumeration annotation, but failed its own step 14 (exit 1) on both images — supplementary,
  does not supersede #9 (`evidence/t6-ci-run10-t2-annotations.log`). P20 confirmed on CI: `skip-count=3 …
  result: ok` — the three suites pass by skipping. Branch deletion happens in T8 cleanup.
- [x] **T7 — Write the verdicts (AC1).** Fill §Go/No-Go; add a `## Findings` list for anything else measured;
  append **§R5 part B** (measured spike outcomes, OLD→NEW for 17-1 / 17-3b) under the §R5 part A the preflight
  round-table filed in `epic-17-workers-and-third-party-form-w2.md` (rule 9), in the same commit; every
  consequence routed by rule 10 to a named story (17-1, 17-3b, or 20-3a by charter), never a bucket. A
  measured outcome that contradicts an ACCEPTED ADR clause (e.g. ADR-060 §D-C *passes the socket path*) is
  written as a proposed ADR amendment for operator ratification — the spike does not amend the ADR itself.
- [x] **T8 — Land (AC2).** Re-run T0's gates (paste); `git diff --stat 92911f59 --` shows only
  `spikes/story-17-3a-wasm-recall/**`, this story file, the epic file (§R5), `sprint-status.yaml`, and — only if a
  finding is routed there by charter — `deferred-work.md` rows with an `**Owner:**` line; write
  `spikes/story-17-3a-wasm-recall/README.md` (11-0 README shape: what it answers, not a workspace member, files table,
  how to reproduce each probe incl. `.rs.txt` renames). Spike crates path-depend on workspace members
  (`maos-kernel-core`, `maos-iac`, `maos-domain`, `maos-wasm-host`) from their own `[workspace]` — expect a full
  separate dependency build; that cost is the spike's, not CI's.

### Review Findings

§A6 review 2026-09-26 (4 layers: Blind + Edge Case + Acceptance + Evidence re-execution; re-execution REPRODUCED Q4 and Q5, DIVERGED on Q1 artifact identity). 5 findings dismissed as noise.
//
// All 13 resolved 2026-09-26 by the §A6 review pass, operator-ruled: the Decision was ruled **6 entries** (`ppoll` dropped, per CI run #10 both images + local LOO); the 12 patches were applied (`t2-ladder/report.txt`, `egress/report.txt`, this file, `README.md`, `epic-17…md` exit line 2 + R17-41, `wit-1.2-same/spirit.wit` header, `sprint-status.yaml` paraphrase, and the review-captured evidence `evidence/q6-actions-annotations-run4.json.log`).

- [x] [Review][Decision] 17-6 seccomp-allowlist price is self-contradictory: 6 vs 7 entries, and `ppoll` measured droppable — `t2-ladder/report.txt:3,8,13` says "retain the seven measured syscall entries" (incl. `ppoll`) and "Leave-one-out rejects each", but `README.md:23`, the ratification line (`:246`) and Completion Notes say **6** additions, and the committed evidence contradicts the report: CI run #10 marks `runner-drop-ppoll leave-one-out-still-succeeded` as a FAILING annotation on **both** images (`evidence/t6-ci-run10-t2-annotations.log:21-22,52-53`), and the local LOO relay also succeeds without `ppoll` (`evidence/q4b-loo-spike-runner-inherited-fd-local.log:66`). Operator must rule the 17-6 figure (6-entry per evidence / 7-entry conservative / re-measure / defer to 17-6 creation).
- [x] [Review][Patch] One-shot path accept is attacker-first-DoS-able: `recall_service.rs.txt:93-98` unlinks after ANY accept, before the `SO_PEERCRED` check, so a first same-uid attacker leaves the legitimate runner with ENOENT; `run_hostile_probe.py:32-38` only exercises attacker-after-runner. Record as a condition on the ADR-060 §D-C amendment item (`:246`).
- [x] [Review][Patch] Inherited-fd safety against a same-uid attacker is conditional on host policy (Yama `ptrace_scope=1`) but is not a routed condition: `fd_attacker.py:12-33` always exits 0 (serializes theft results; caller `run_fd_vector_probe.py:53-54` gates only on rc), so the probe stays green wherever `/proc/<pid>/fd` or `pidfd_getfd` succeeds; R17-42 carries Yama only as a parenthetical. Promote to an explicit condition on the §D-C transport amendment.
- [x] [Review][Patch] Q6 primary run `36207588609` (jobs `108307271210`/`108307271391`) has NO committed evidence — `grep` over `evidence/` finds neither ID — and citations disagree (`egress/report.txt:6,11` cites run #9 `36210072648` + run-1 annotations; the story row and R17-45 cite run #4). Verified during review: both check-runs' annotations are publicly fetchable (HTTP 200, 10 each) and match the recorded M1/M2/M3 values — commit the captures and reconcile the three citations.
- [x] [Review][Patch] Q6 price asserts unmeasured lines "fall inside the granted +65–130" (`:243`) — an inference, contrary to the story's own NOT-MEASURED rule; reword to mark it as expectation, not measurement.
- [x] [Review][Patch] Node floor overstated: README `:20,:34` say "Node ≥ 22.20", but locked `@napi-rs/lzma@1.5.1` engines are `^22.20 || ^24.12 || >=25` (`ts-guest/package-lock.json:629-630`) — Node 23.x and 24.0–24.11 satisfy the README yet are unsupported. Fix README, story Q1 condition, R17-38.
- [x] [Review][Patch] Q3 "lossless" overstated: the lift defaults nested fields — `PriorDistillateRef.intent_lineage` (`bridge/src/bridge.rs.txt:338`) and `DecisionDispatch.working_memory_digest_refs` (`:350`) — and the probe used `prior_distillate_ref: None` (`bridge/src/main.rs.txt:68`), so byte-equality was measured only for frames without nested payloads. Qualify Q3 row, README `:22`, R17-41.
- [x] [Review][Patch] Epic-17 exit line 2 still builds/invokes the debug runner (`epic-17-workers-and-third-party-form-w2.md:20`), which Q1 measured as a deterministic compile-bomb exit 3 — contradicting R17-38 NEW ("`--release` … `target/release/…`") filed in the same commit; align the exit block.
- [x] [Review][Patch] Q1 component artifact identity is not reproducible: recorded `72f6f923…/12,259,559 B`, but a re-execution with identical inputs (Node 22.23.3, componentize-js 0.23.0, hash-equal WIT) produced `8925b36e…/12,259,556` and then `5f0a4dae…/12,259,547` on an immediate repeat — the build is nondeterministic; record the caveat (the recorded sha identifies the tested artifact, not a reproducible output).
- [x] [Review][Patch] Findings F9 is ownerless ("method; no owner", `:358`) under a header that says "each routed by rule 10" (`:346`) — exempt it explicitly as a method record or assign an owner.
- [x] [Review][Patch] File List says "113 `evidence/` logs" (`:614`) — actual: 115 `evidence/` files, 106 of them `.log`.
- [x] [Review][Patch] README §Reproduce inconsistencies: `host/run_*.py` paths (`README.md:67-68`) do not resolve from the repo root the section mandates (`:29`); the Q1 line invokes shared `target/release/maos-wasm-runner` (`:60`), contradicting §Build environment's private-copy rule for HEAD's runner (`:32-33`).
- [x] [Review][Patch] `wit-1.2-same/spirit.wit:1-13` header describes the `@1.1.0` file ("differs … in exactly three places … `package maos:spirit@1.1.0;`") but the control's actual package line is `@1.2.0` (`:15`) — misleading metadata on the Q5 package-only control.

## Go/No-Go (the AC1 deliverable — filled by the dev pass)

| Q | Verdict | Evidence (command → excerpt → `evidence/` file) | Price (per crate · kernel-Δ · CI) | Consequence for 17-1 / 17-3b (OLD→NEW) |
|---|---|---|---|---|
| 1 componentize-js | **GO-WITH-CONDITIONS** | `npx componentize-js dist/spirit.js --wit ../../../wit --world-name spirit --disable http fetch-event -o out/ts-spirit-node22.wasm`: the direct pinned 0.23.0 CLI under Node 22.23.3 (jco 1.35.0 nests 0.22.0, so it was not used) → 12,259,559 B. Then `q1-driver encode \| target/release/maos-wasm-runner --component …/ts-spirit-node22.wasm \| q1-driver decode` → `on-start`, one `handle-frame`, `decoded-count=1`, rc `0 0 0 0`; the committed-fixture control is identical. The debug HEAD runner fails the same component with `component compile exceeded 10s — treating as a compile-bomb`, rc 3. Node 20.19.2: a clean `npm ci` warns `@napi-rs/lzma@1.5.1` needs Node ^22.20, then `Cannot find module '@napi-rs/lzma-linux-x64-gnu'`. StarlingMonkey is embedded (strings). `console.log` corrupts nothing and appears nowhere. → `q1-componentize-final-node22.log`, `q1-release-pipeline-final.log`, `q1-release-fixture-control.log`, `q1-debug-runner-final.log`, `q1-componentize-node20.log`, `q1-starlingmonkey-strings.log`, `q1-fuel-bisect-final.log`; re-executed by a non-author in `a6-reexec-orchestrator.log` | kernel-Δ 0 · no workspace dependency. 17-3b pays: one Node-22 step in the job that componentizes (`npm ci` ~2 s + componentize 2.25 s locally); `example-spirit-ts-tests`'s pin `discipline.yml:1086-1088` 20 → 22; a CI-built 12.26 MB artifact, never committed (> 1 MiB); and a release-profile runner (debug `Component::new` 10,506.735 ms vs `COMPILE_TIMEOUT` 10 s at `runner.rs:140`; release 792.820 ms). Fuel per `handle-frame` 2,313,954 of the default 10,000,000 (`runner.rs:109`). | **17-3b** AC1 + exit line 2 (`epic-17…:98`, `:20`). OLD: `cargo build … && target/debug/maos run …` after an unqualified componentize-js build. NEW: Node 22, the direct pinned `componentize-js` 0.23.0 CLI (not `jco componentize`), a `--release` build and `target/release/…` in exit line 2 (or an engine-config-matched precompiled `.cwasm`, see Q4), and the component as a CI artifact. R17-18 stands, with these prerequisites. **17-1**: none. |
| 2 additive WIT | **GO-WITH-CONDITIONS** (recall-only `@1.1.0`) | In a scratch worktree with `wit/spirit.wit` := `wit-1.1/spirit.wit`: `cargo test -p maos-wasm-host --test wit_corpus` → `must cover all 16 record types … left: 21 right: 16`, exit 101. `check-equiv-fixture-provenance --json` → 4 × `source_sha256 drift`, exit 1. `check-wasm-form-equiv` → `PASSED — base 20/0, anti-canned 23/0`. `cargo build -p maos-wasm-host --bin maos-wasm-runner` → exit 0, so **P4 is false**: an import forces no host impl at compile time, and a recall guest fails only at instantiate (Q5). Rebuilding the 4 pinned components at HEAD (rustc 1.98.1, wit-bindgen 0.44.0, wasm-tools 1.259.0) → **none** matches its `wasm_sha256`; the cause is NOT-MEASURED, because the provenance file records no toolchain. `abi-diff` via cargo-public-api 0.52.0 → no `LogRecall*`/`LogFetch`, exit 0. → `q2-recall-only-tax.log`, `q2-fixture-rebuild.log`, `q2-abi-diff-scope.log`, `q2-recall-wit-surface.log` | kernel-Δ 0. `wit_corpus.rs:156` record total 16 → 21, plus field-table rows `recall-cursor=2`, `recall-filter=6`, `recall-entry=6`, `recall-page=2`, `fetch-response=7`; the other six count gates are unchanged. All 4 provenance rows re-pinned. 4 component rebuilds, not reproducible today. No new CI step, but the provenance step reds until the re-pin lands. | **17-3b** AC2 (`epic-17…:99`). OLD: "the 8 fixture guests … re-[built]" plus an `abi-ratifications.toml` entry. NEW: re-issue the **4** provenance-pinned components with the toolchain recorded and a CI step that proves reproducibility; delete the ratification clause (moot, measured); keep `wit_corpus` + provenance as the WIT controls. Updates R17-28 and R17-30. **17-1**: none. |
| 3 frame_bridge | **NO-GO** as part of the additive `@1.1.0`. The mechanism measured GO — for frames without nested payloads: the spike lift still defaults `PriorDistillateRef.intent_lineage` (`bridge.rs.txt:338`) and `DecisionDispatch.working_memory_digest_refs` (`:350`), and the probe ran with `prior_distillate_ref: None` (review). | `cargo run --release --manifest-path spikes/…/bridge/Cargo.toml --bin frame-bridge-probe` → `extended_cbor_equal=true bytes=1025`. The same-target HEAD control gives `head_control_cbor_equal=false head_bytes=476`, defaults `readonly/none/empty-lineage/empty-scope`. Instantiating the committed `@1.0.0` fixture under a host from `wit-1.1-frame` → `type-checking export func handle-frame … expected record of 11 fields, found 8 fields`. HEAD's `frame_bridge.rs` against that WIT fails with E0277 `:216`, E0308 `:305` and E0063 `:404`. → `q3-frame-bridge.log`, `q3-wit-surface.log`, `q2-frame-growth-tax.log`; re-executed in `a6-reexec-orchestrator.log` | kernel-Δ 0. WIT 21 → 25 records: `iac-frame` 8 → 11 fields, `task-assign-body.scope` `list<string>` → `list<scope>`, new `consent-envelope=5`, `scope-mcp-call=2`, `scope-cli-subprocess-spawn=3`, `scope-gateway-send=2`. Spike bridge conversion is 575 lines. It is a **breaking** change for every `@1.0` component, so all 4 fixtures are re-issued. | **17-3b** AC3 (`epic-17…:100`) cannot ride AC2's additive `@1.1.0`. NEW: 17-3b chooses at creation (rule 9) between (a) a separate breaking revision, i.e. `@2.0.0`, the 4 components re-issued, and a dual-world host or a flag day, and (b) an additive carrier for the four fields, e.g. an import that serves them by frame id [INFERENCE — not measured]. **17-1**: none. |
| 4 recall relay (4a–4d) | **GO-WITH-CONDITIONS** — the §D-C mechanism works with kernel-Δ 0; under T2 the transport is the inherited fd, not the socket path | `Q4_COUNT=1000 host/run_relay_probe.sh`: the `@1.1` guest's `recall` import reaches the real `LogRecallAdapter`; `recall` returns A's frames, `fetch` of B's → `recall-error::scope-violation`; `SO_PEERCRED` on (pid bound at spawn) → the same-uid attacker is refused, off (same-target control) → the attacker reads A's frames; one-shot accept + unlink → later connect ENOENT; the inherited fd works (`/proc/<runner>/fd/3` `O_PATH` ok, `open` → ENXIO, `pidfd_getfd` → EPERM under Yama=1). Latency over 1,000 calls: recall p50 0.71–0.75 ms, p99 1.28–1.32 ms; per-call cost climbs ~0.09 ms (first 10) → ~1.2 ms (last 10), cause NOT-MEASURED. **Q4b** (CI run #9 `36210072648`, both images, steps 7–17 green; #10 `36210776858` partial): verbatim seccomp order → install EPERM (**F1**); kill-first + Landlock program/loader/libs + additions `poll, rt_sigaction, clone3, memfd_create, sendto, recvfrom` (leave-one-out proven necessary; `ppoll` proven unnecessary) → runner, hello-dynamic and hello-static all rc 0; the forbidden-syscall probe → **SIGSYS 31** (local); `relay-path` denied by the candidate while five `relay-fd` variants complete; dropping `recvfrom` or `sendto` → denied. → `q4-relay-*.log`, `q4b-*-local.log`, `t6-ci-run9-t2-annotations.log`, `t6-ci-run10-t2-annotations.log` | relay kernel-Δ 0 (`q4-kernel-delta.log`); the T2 repair (Q4b) is **+25/−1 across 5 files** — `linux.rs` +16/−1, `runtime.rs` +6, `worker_spawn.rs` +1, `j1.rs` +1, `cli_wrapper_bridge_8_12.rs` +1 — prototype applied and built `--locked` rc 0 (`q4b-kernel-price-prototype.log`) | 17-3b AC1: `SO_PEERCRED` + one-shot accept + 0600 socket in a 0700 dir become normative (R17-42); **ADR-060 §D-C amendment proposed**: the daemon passes a connected socketpair end at spawn — the socket path is NO-GO under T2 (R17-44). 17-6 takes the measured grant (R17-44) |
| 5 version skew | **GO-WITH-CONDITIONS** | `wasm-tools component wit tests/fixtures/wasm/echo_spirit_component.wasm` → `import maos:spirit/frames@1.0.0` and unversioned `handle-frame`/`on-start`/`on-shutdown` exports (P12 confirmed). The `@1.1` `spike-runner` runs the untouched `@1.0` fixture: `decoded-frames=1`, exit 0. The HEAD runner (sha256 `cf3a4e04…`) runs the `@1.1` recall guest: exit 3, `component imports instance maos:spirit/recall@1.1.0, but a matching implementation was not found`. **Control, package-only bump** (`wit-1.2-same`, the diff is the `package` line): the `@1.2` host runs the `@1.1` guest → exit 0, and the `@1.1` host runs a `@1.2` guest that calls recall → exit 0. **One added `frame-kind-label` case** (`wit-1.2-enum`): exit 3 in **both** directions; a matching pair → 0. wasmtime 46.0.3 `component/linker.rs:41-54` applies semver lookup only when both versions have the same API. → `q5-wire-names.log`, `q5-host-1.1-fixture-1.0.log`, `q5-head-host-1.0-guest-1.1.log`, `q5-enum-growth.log`, `q5-version-only-control.log`; re-executed in `a6-reexec-orchestrator.log` | kernel-Δ 0. 17-3b adds a compatibility-matrix check: the `@1.0` fixture on the new host; a new guest refused on the old host; a package-only bump in both directions. | **17-3b** AC2 (`epic-17…:99`). OLD: "additive". NEW: additive for **old guests on new hosts only**, so hosts ship first; a package bump is free; **any change to a recall type is a lock-step break**. A closed `frame-kind-label` enum therefore makes every new `FrameKindLabel` (30 today, `#[non_exhaustive]`, `log_recall.rs:41-90`) break every deployed recall guest. Choose an open label (`kind: string`) or accept lock-step upgrades per label. **17-1**: none. |
| 6 egress proxy (M1–M3) | **GO-WITH-CONDITIONS**: **M3** selected; M1 and M2 **NO-GO** | Run in GitHub Actions on the throwaway branch `spike/17-3a-ci-probe` (workflow text: `egress/ci-probe.yml.txt`). `egress/egress-probe.sh` ran on **ubuntu-24.04** (podman 4.9.3, netavark, slirp4netns 1.2.1) and on **ubuntu-26.04** (podman 5.7.0, netavark, slirp4netns 1.3.3, pasta 0.0~git20260120). Run #4 `36207588609`, jobs `108307271210` and `108307271391`; run #1 `36206431636` agrees. The annotations are identical on both images. M1 `--network=slirp4netns:allow_host_loopback=true` → `host=yes internet=yes`. M2 pasta → `host=no internet=yes`. M3 `--network=none` + a `--volume`-mounted Unix socket + an in-container socat on 127.0.0.1 → `host=yes internet=no lo=yes socket-0600=no socket-0666=yes rc=0`. Rootless podman on the dev host fails `cannot clone: Operation not permitted` (`q6-local-podman.log`), so CI is the measurement. → `q6-actions-annotations-run4.json.log` (run #4, captured in review), `q6-actions-annotations-*.json.log`, `t6-*.log`, `egress/report.txt` | kernel-Δ: M3 keeps the gated literal `"--network=none".to_string()` (`t3/argv.rs:72`; `decision_adrs_and_provisioning.rs:313-327` stays green). It adds **one** argv entry to `t3/argv.rs`, `--volume=<daemon-owned socket>:/proxy/egress.sock`, plus the socket hand-off into the T3 spawn spec. The lines beyond that entry are 17-1's design and NOT-MEASURED (expected — not measured — to fit the granted +65–130). The T3 image gains socat (or an equivalent static forwarder), so `t3-image.lock` is re-signed. The five `network_escape/*.json` + `capability_escape_cap_net_raw_002.json` fixtures keep `--network=none` as their expected blocker, so nothing is re-authored. CI: 33–42 s per image. | **17-1** AC1–AC4 (R17-32). OLD: no container-network mechanism chosen; ADR-061 `:59-61` says *a daemon-side proxy on host loopback*. NEW: **M3**. `--network=none` stays; the daemon owns a Unix socket bind-mounted at `/proxy/egress.sock`; a forwarder in the signed T3 image serves `http://127.0.0.1:<port>` to `ANTHROPIC_BASE_URL` / codex `base_url`. The socket is **0666** under the rootless user-namespace remap (0600 is refused, measured) and sits in a daemon-owned 0700 directory [INFERENCE: the directory is what keeps other host users out]. M1 and M2 are rejected: both reach the internet on both images. **Proposed ADR-061 amendment** (operator ratification, RT-4): *host loopback* → *a daemon-owned Unix socket mounted into the `--network=none` container*. **17-3b**: none. |

**Overall:** 17-3b may leave backlog: ☐ yes ☒ with conditions ☐ no · 17-1 may leave backlog: ☐ yes ☒ with conditions ☐ no.
**Operator ratification needed after the spike (RT-4):** ☒ **RATIFIED 2026-09-26 (Lunarpulse) — written into `docs/adr/ADR-060-spirit-forms-by-trust-tier.md` §Amendment:** ADR-060 §D-C transport amendment (a connected socketpair end inherited at spawn — the socket path is NO-GO under T2; the amendment text must carry two review-measured conditions: (i) the inherited fd's same-uid safety depends on host ptrace protection — `pidfd_getfd`/`/proc/<pid>/fd` theft succeeds where Yama `ptrace_scope=0` (`q4-fd-vectors.log`); (ii) a path-socket one-shot accept is attacker-first-DoS-able — the unlink precedes the `SO_PEERCRED` check (`recall_service.rs.txt:93-98`), so availability must not rest on one-shot path accepts) · ☐ ADR-061 egress amendment (M3: `--network=none` + a bind-mounted daemon socket; *on host loopback* is superseded) · ☒ **RATIFIED 2026-09-26 (Lunarpulse) at the figure RE-MEASURED by the 17-6 preflight round-table — kernel +71 tokei / +83 `src_lines` across 4 files plus the arm-measured aarch64 entry; the figure below is superseded (17-6 §Kernel grant):** T2 repair lines (kill-first order + the 6 measured seccomp additions + Landlock program/loader/libs rights; +25/−1 across 5 files) as `17-6`'s FLAG-Winston grant (RT-8) · ☐ none.

### Excerpts (≤ 20 lines per question; full logs under `spikes/story-17-3a-wasm-recall/evidence/`)

**Q1**: `a6-reexec-orchestrator.log` (a non-author re-run against the verified HEAD runners) and `q1-componentize-node20.log:10,20`.
```text
$ sha256sum ts-spirit-node22.wasm; stat -c %s ts-spirit-node22.wasm
72f6f9232be94bac3076a02610fa64de64ffdcacea890078e317043ebef374ff
12259559
$ q1-driver encode | target/release/maos-wasm-runner --component ts-spirit-node22.wasm | q1-driver decode
    kind: TaskAssign,
decoded-count=1
pipe rc=0 0 0 0
$ (same-target control) … --component tests/fixtures/wasm/echo_spirit_component.wasm …
decoded-count=1
$ release runner --fuel 303157 / 303158 (no frame); 2617111 / 2617112 (one frame)
rc=4 / rc=0; rc=4 / rc=0 (decoded-count=1)
$ target/debug/maos-wasm-runner --component ts-spirit-node22.wasm </dev/null
maos-wasm-runner: InvalidComponent: component compile exceeded 10s — treating as a compile-bomb
rc=3
$ npx componentize-js … (Node 20.19.2, clean npm ci)
Error: Cannot find native binding. npm has a bug related to optional dependencies …
  cause: Error: Cannot find module '@napi-rs/lzma-linux-x64-gnu'
```
**Q2**: `q2-recall-only-tax.log:578-591,1324-1330,2099,5635-5696` and `q2-fixture-rebuild.log` (scratch worktree, `wit/spirit.wit` := `wit-1.1/spirit.wit`).
```text
$ cargo test --locked -p maos-wasm-host --test wit_corpus
assertion `left == right` failed: must cover all 16 record types from the parsed .wit AST …
  left: 21
 right: 16
[exit=101]
$ cargo run --locked -p xtask -- check-equiv-fixture-provenance --json
check-equiv-fixture-provenance: FAIL — 4 fixture drift/mismatch(es):
- echo_spirit_component.wasm: source_sha256 drift — manifest=9ab91c1f…, actual=4fec8fdd…   (+ identity, divergent, cosmetic)
[exit=1]
check-wasm-form-equiv: PASSED — oracle green (base: 20 passed/0 failed, anti-canned: 23 passed/0 failed); BLOCKING at v1_5
$ cargo build --locked -p maos-wasm-host --bin maos-wasm-runner
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 21.93s
[exit=0]
$ rebuild the 4 pinned components at HEAD; compare to wasm_sha256
echo_spirit.wasm: sha256=a41ad244… bytes=69735 pinned=1ab9c5b8…   (identity, divergent, cosmetic: no match either)
```
**Q3**: `q3-frame-bridge.log`; re-run in `a6-reexec-orchestrator.log`.
```text
$ cargo run --locked --offline --release --manifest-path spikes/story-17-3a-wasm-recall/bridge/Cargo.toml --bin frame-bridge-probe
extended_cbor_equal=true bytes=1025
head_control_cbor_equal=false head_bytes=476
head_control_defaults=readonly/none/empty-lineage/empty-scope
frame_growth_skew=type-mismatch
frame_growth_error=type-checking export func `handle-frame`
Caused by:
    expected record of 11 fields, found 8 fields
```
**Q4/Q4b**: `evidence/q4b-inherited-fd-local.log`, `evidence/q4b-loo-spike-runner-inherited-fd-local.log`, `evidence/q4b-socket-escape-surfaces-local.log` (local); `evidence/t6-ci-run9-t2-annotations.log` (CI, both images identical).
```text
$ relay 1000 calls: recall p50 0.71-0.75 ms p99 1.28-1.32 ms; per-call ~0.09 ms (first 10) -> ~1.2 ms (last 10)
$ hostile same-uid connector: SO_PEERCRED on -> refused; control (off) -> reads A's frames
$ one-shot accept+unlink -> connect ENOENT; /proc/<runner>/fd/3: O_PATH ok, open ENXIO, pidfd_getfd EPERM (Yama=1)
CI run #9 36210072648 (ubuntu-24.04 == ubuntu-26.04):
t2-probe/runner-as-is-verbatim     install=errno=1             <- F1: allow-list installs first, 2nd filter EPERM
t2-probe/runner-as-is-kill-first   install=ok rc=none sig=11   <- P19: poll/rt_sigaction missing
t2-probe/runner-candidate-kill-fir install=ok rc=0
t2-probe/strace-additions          rust-start,clone3,memfd_create
t2-probe/relay-path                candidate denied socket-path recall; inherited descriptor remained the tested alternative
t2-probe/relay-fd-{precompiled,no-compile-watchd,no-signals-based-,no-parallel-compi,default}
                                   expected inherited-descriptor recall relay completed under candidate seccomp
t2-probe/relay-drop-recvfrom       harness=1 required inherited-socket operation was denied
t2-probe/relay-drop-sendto         harness=0 required inherited-socket operation was denied
local LOO (kill-first + rights): drop poll|rt_sigaction -> sig 11; clone3 -> rc 101; memfd_create -> rc 3;
                                   sendto|recvfrom -> recall denied; ppoll -> rc 0 (run #10 confirms on CI)
local forbidden-syscall-probe under the selected candidate: rc=none sig=31 errno=none   <- SIGSYS: 17-6 AC1 red
```
**Q5**: `q5-wire-names.log:8,24-26`; the matrix re-run in `a6-reexec-orchestrator.log`.
```text
$ wasm-tools component wit tests/fixtures/wasm/echo_spirit_component.wasm
  import maos:spirit/frames@1.0.0;
  export handle-frame: func(frame: iac-frame) -> result<list<iac-frame>, halt>;
$ frame-driver encode | spike-runner (@1.1 host) --component echo_spirit_component.wasm (@1.0) | frame-driver decode
frame-driver: decoded-frames=1          pipe rc=0 0 0 0 0
$ maos-wasm-runner-HEAD (sha256 cf3a4e04…) --component story_17_3a_recall_guest.wasm (@1.1)
… component imports instance `maos:spirit/recall@1.1.0`, but a matching implementation was not found in the linker
rc=3
$ spike-linker-1-2-same --component story_17_3a_recall_guest.wasm       rc=0   (package-only bump, @1.2 host / @1.1 guest)
$ spike-runner --component story_17_3a_version_only_guest.wasm          recall-import-boundary-ns=1683  rc=0
$ spike-linker-1-2 --component story_17_3a_recall_guest.wasm            … `maos:spirit/recall@1.1.0` … not found  rc=3
$ spike-runner --component story_17_3a_enum_guest.wasm                  … `maos:spirit/recall@1.2.0` … not found  rc=3
$ spike-linker-1-2 --component story_17_3a_enum_guest.wasm              rc=0
```
**Q6**: public annotations, run `36207588609`, jobs `108307271210` (24.04) and `108307271391` (26.04).
```text
egress-probe/ubuntu-24.04  podman-version=podman version 4.9.3 · network-backend=netavark · slirp4netns 1.2.1
egress-probe/ubuntu-24.04  M1 host=yes internet=yes rc=0/0 sig=0      ::error M1 expected-host=yes-internet=no
egress-probe/ubuntu-24.04  M2 host=no internet=yes rc=1/0 sig=0       ::error M2 expected-host=yes-internet=no
egress-probe/ubuntu-24.04  M3 host=yes internet=no lo=yes socket-0600=no socket-0666=yes rc=0 sig=0
egress-probe/ubuntu-26.04  podman-version=podman version 5.7.0 · network-backend=netavark · slirp4netns 1.3.3 · pasta 0.0~git20260120.386b5f5-1
egress-probe/ubuntu-26.04  M1 host=yes internet=yes rc=0/0 sig=0      ::error M1 expected-host=yes-internet=no
egress-probe/ubuntu-26.04  M2 host=no internet=yes rc=1/0 sig=0       ::error M2 expected-host=yes-internet=no
egress-probe/ubuntu-26.04  M3 host=yes internet=no lo=yes socket-0600=no socket-0666=yes rc=0 sig=0
```

## Findings (measured beyond the six rows; each routed by rule 10)

| # | Finding | Evidence | Owner |
|---|---|---|---|
| F1 | **T2's seccomp layer has never installed, on any host: the filters go in the wrong order.** `linux.rs:339` documents *install kill filter first, then allow-list filter*; `:346` returns `vec![bpf, kill_bpf]`, allow-list first. `pre_exec` applies them in that order (`:95-102`). seccompiler 0.4.0's `apply_filter` issues `prctl(PR_SET_NO_NEW_PRIVS)` and then `seccomp(SECCOMP_SET_MODE_FILTER)` (`seccompiler-0.4.0/src/lib.rs:347`, `:363-364`), and neither is on the allow-list, so the second install gets the allow-list's `Errno(EPERM)`. Measured on this host, whose process already sits under a filter (`Seccomp: 2`): one filter → install ok; allow-list → kill → spawn `Err(PermissionDenied, os error 1)`; kill → allow-list → install ok. CI shows the same refusal: `maos: seccomp apply failed` in `t2_sandbox_kill`'s output on both images. The fix is one changed line (net 0). Run-time semantics do not depend on install order: every filter runs and `KILL` > `ERRNO` > `ALLOW`. | `evidence/a6-seccomp-order.rs.txt`, `a6-reexec-orchestrator.log` §Seccomp install ORDER; CI run `36208373890` annotations | **`17-6`** AC3 |
| F2 | **P20's cause is corrected; its conclusion stands.** The three T2 suites do report `ok` by skipping: CI shows `SKIP t2_benign_control: sandbox setup requires CAP_SYS_ADMIN / no_new_privs` ×2 and the raw pre-exec write `maos: seccomp apply failed` ×2 on both images. The refusal comes at the **seccomp install** (F1), before `execve`, not at `execve` with `EACCES` (P20's inference), and not from *this host cannot nest seccomp* (F1 measures that it can). P21's Landlock `EACCES` is real (confirmed on both CI kernels) but is masked in the real T2 path until F1 is fixed. | CI run `36208373890` `p20` annotations; F1 | **`17-6`** AC4 |
| F3 | Fuel exhausted during instantiation (`--fuel 0`) exits 3 with `InvalidComponent: component does not conform to maos:spirit@1.0 (instantiate failed): wasm trap: all fuel consumed`, not `OutOfFuel` exit 4. | `q1-fuel-bisect-final.log:5-12` | **`17-3b`** (R17-39) |
| F4 | Guest `console.log` reaches neither stdout nor stderr (the WASI context inherits no stdio, `host_state.rs:20-25`); `jco guest-types` omits the thrown `halt` type. | `q1-release-pipeline-final.log`, `q1-result-error-runner.log` | **`17-3b`** (R17-39) |
| F5 | `wit/spirit.wit:216-218` says the runner *is sandboxed by the existing T2 path*. That is false (P9, P21, F1) and the same class as `runner.rs:30-37` (RT-9). | this story, P9/P21; F1 | **`17-3b`** (R17-39) |
| F6 | Recall/fetch per-call cost climbs within one service lifetime, from ~0.09 ms over the first 10 calls to ~1.2 ms over the last 10 of 1,000. `idx_tlog_spirit_pid` exists (`transparency_log.rs:300-301`); the cause is NOT-MEASURED. | `q4-latency.log`; re-run in `a6-reexec-orchestrator.log` | **`17-3b`** (R17-42) |
| F7 | A precompiled `.cwasm` is bound to the engine configuration: a default-config artifact run with the mitigation flags exits 3, and a same-flag artifact passes. | `q4-runner-options.log`, `q4b-precompiled-local.log` | **`17-3b`** (R17-42) |
| F8 | Two gates fail closed but name the wrong cause, or none: `abi-diff --json` drops cargo's stderr (`{"passed":false,"output":""}`), and `check-wasm-form-equiv` calls a compile failure *vacuous … a re-stubbed harness*. | `q2-abi-diff-scope.log:8,1887`, `q2-frame-growth-tax.log:2151,2233` | **`20-3a-gate-honesty`** (`deferred-work.md`) |
| F9 | Spike method: on the authoring host every crate and worktree shares one target dir (`CARGO_TARGET_DIR=/mnt/build/cargo`, and `target` links to it). A scratch-worktree build briefly replaced the shared `target/debug/maos-wasm-runner`. Every HEAD-runner measurement in this story used a private copy whose sha256 matches `cf3a4e04…`, or was re-run after verification. | `a6-reexec-orchestrator.log` §Shared-target provenance, §Q1 re-run | method record — exempt from rule-10 routing (mitigated in-story; §Review Findings) |
| F10 | The componentize-js artifact is not bit-reproducible: identical inputs (Node 22.23.3, CLI 0.23.0, hash-equal pinned WIT) produced a different sha256/size on consecutive builds — `8925b36e…`/12,259,556 then `5f0a4dae…`/12,259,547 vs the recorded `72f6f923…`/12,259,559; both rebuilds still ran correctly through the release HEAD runner (`decoded-count=1`, rc 0). | review Evidence re-execution layer (§Review Findings) | **`17-3b`** (R17-38 — the artifact is a CI artifact, never committed, so its identity cannot be sha-verified by rebuild) |

## Preflight round-table (2026-09-25 — Winston · Murat · Amelia · John · Mary · Sally · Vex · Grumbal)

Convened on the question *is this spike aimed at the right premises for the long term?* Three findings were new —
P19 and P21 measured during the round-table itself, P20 explained by them; every ruling below is applied in this
file, the epic's §R5 part A, and `deferred-work.md`; the two left to the operator (RT-8, RT-9) were decided by Lunarpulse on 2026-09-25, with T6's authorization, and are recorded in place.

| # | Ruling | Why (one line) |
|---|---|---|
| RT-1 | **Q4b rebuilt as a four-layer ladder:** Landlock (can anything exec?) → seccomp (can Rust start?) → runner-side mitigations → transport; each layer measured with the candidate policy in spike code and every surviving line priced as an exact FLAG-Winston kernel count. | P19 + P21: "is the socket reachable" was the fourth question, not the first (Murat, then Amelia's harness). |
| RT-2 | **Q4b's seccomp layer and Q6 share ONE throwaway CI probe** (T6); the Landlock layer is measured locally (it enforces here — P21). | This host cannot nest seccomp (`Seccomp: 2`), and its T2 suite says PASS anyway (P20, Grumbal). |
| RT-3 | **Hostile same-uid connector vector added to Q4c.** | A Worker runs as the operator uid; a path socket is reachable by anything at that uid (Vex). |
| RT-4 | **The spike measures; it does not amend ADR-060 §D-C.** A measured contradiction (e.g. transport must be an inherited fd, or T2 needs allowlist lines) is written as a proposed amendment for operator ratification. | Rulings rot when a story re-rules them (Murat, standing rule). |
| RT-5 | **§R5 part A filed NOW** (twelve rows, R17-23…R17-34, in `epic-17-workers-and-third-party-form-w2.md` §R5), not deferred to the dev pass: premises already measured at creation must not sit in 17-1/17-3b as live text for the length of a spike. | 17-3b AC1 and exit line 2 currently promise a refusal the type cannot express (John). |
| RT-6 | **Routing (rule 10):** P7/P9/P19 consequences → 17-3b; fixture rebuild automation + the gate doc's phantom nightly job (`check_equiv_fixture_provenance.rs:15-17`) → 17-3b (it re-pins the fixtures); CI Node 20 (EOL) → 17-3b (it adds the componentize-js step to that job family); ADR-061 journal-before-return vs `try_send` emitters → 17-1; skip-reads-as-pass in `t2_sandbox_kill` + T3 `continue-on-error` masking → `20-3a-gate-honesty` (E16-A2 charter). *Narrowed 2026-09-25:* the three T2 suites moved to `17-6` AC4 with the RT-8 decision, and `t3-smoke-busybox` was already 17-1 AC4's (epic R17-36); 20-3a keeps the class (pulled forward, D4), and the operator moved `t3-escape-corpus` to 17-1 AC4 too (D3). | Each lands on the story that already touches the file, or on a charter match — no bucket. |
| RT-7 | **Timebox unchanged (≤3 days).** If the CI probe is not authorized, Q4b/Q6 are `NOT-MEASURED` and the blocks hold; the spike does not grow to compensate. | Dana's rule, carried by John: a spike that expands to fit its findings is a build story. |
| RT-8 | **The T2 repair needs a vehicle, and the room recommends a NEW ◆ FLAG-Winston story sequenced after 17-3a (which prices it) and before 17-2 and 17-3b** — scope: Landlock rights so the resolved program (and its loader/libs, or a static binary) can start; the measured seccomp additions; T2 suites that run for real in CI with a proven-red (a genuine SIGSYS kill on `ptrace`, observed in the job log). Alternatives on the table: fold into **17-2** (rule 10(b): it already edits `linux.rs` and needs a T2 child — but it queues behind 17-1's egress proxy and buries the proven-red in a cgroup story) or into **17-3b** (whose *kernel-Δ 0 by construction* would then be false). **OPERATOR DECISION (Lunarpulse, 2026-09-25): the new story — `17-6-t2-sandbox-repair-and-proven-red`**, minted with its tracker key, epic row and §R5 targets in one change (John's condition). Operator directive with it: real behaviour, not green runs — every 17-6 AC is an observed kill or a proven-red, never a passing suite. | A sandbox tier that has never started a process is the lane's largest *claim standing in for a control*; it gets its own key, owner and proven-red, not a sub-AC (Winston, Murat; John argued 10(b) and conceded on sequencing). |
| RT-9 | **False claims routed, not fixed here (rule 10(b)):** `crates/maos-wasm-host/src/runner.rs:30-37` (*sandboxed by the existing T2 path … exercised against this same binary in `tests/t2_sandbox_kill.rs`* — false twice, P9/P21) → **17-3b**, which edits the runner's launch path; the published cookbook `docs-site/docs/cookbook/cli-wrapper-spirit.md:89` (+ its `ko` twin `:90`) — *CLI wrappers … OS-level sandboxing (seccomp, landlock)*; Workers are spawned bare (`runtime.rs:465`) → **17-1**, which makes the Worker's sandbox real and must state it as T3/podman; the three skip-as-pass T2 suites → **`17-6`** AC4 (narrowed by the RT-8 decision; the class stays with `20-3a-gate-honesty`, E16-A2). **OPERATOR DECISION (Lunarpulse, 2026-09-25): no hot-fix — route to 17-1, made binding** as 17-1 **AC5** in the epic, so the fix is a reviewed acceptance criterion rather than a docs chore. Applying it found a third false text and a phantom tier — `deploy/topology.md`'s isolation layers, and the cookbook's *`hardened`* tier, which `parse_sandbox_tier` rejects (epic R17-35) — both added to AC5, en + ko. | A docs line nobody owns is how a false claim survives three epics (Sally's question: *what does the person reading the cookbook believe?*). |

**Operator follow-up decisions (Lunarpulse, 2026-09-25)**, ruled on the room's suggested list after RT-8/RT-9, with the directive *real behaviour and real operation, not tests passing on empty criteria*: **D1** confidence rule 11 — *a control is real, or it is labelled*, deliberately not maximal — added to the rules blocks of epics 17–21; **D2 = a** T2 becomes a production path in 17-6 (AC5), T3's applied-or-refused lands with 17-1 AC4, and the refusal is scoped to spawned forms because nine first-party `rust-inproc` manifests declare T2 (epic R17-37 — ruled 2026-09-25, re-ruled 2026-09-26 on the re-measurement: the nine relabel to `T0`, and 17-6 AC6 refuses any other declared tier on `rust-inproc`); **D3** 17-1 takes the `t3-escape-corpus` unmasking (AC4); **D4** `20-3a-gate-honesty` pulled forward to start right after this spike, in parallel with 17-6; **D5** a read-only CI token for the agent host (operator lane — `ops-provisioning-secrets-and-accounts`, checklist row `agent CI-log read token`); **D6** every sandbox or egress job in epic 17 runs on both `ubuntu-24.04` and `ubuntu-26.04`; **D7** a stable performance runner, designed in `docs/adr/ADR-068-stable-performance-runner.md` (ACCEPTED — ratified 2026-09-25, sourcing a rented dedicated server), provisioned by `ops-stable-perf-runner` and built by `20-7-stable-runner-perf-gates`. Not added: a T6 step pricing the two T3 suites unmasked — offered with D3, not approved.

## Dev Notes

### Premise corrections at HEAD `92911f59` (measured at story creation — feed §R5)

| # | Epic / preflight premise | Measured now |
|---|---|---|
| P1 | *all 8 tracked guests rebuilt under `check-equiv-fixture-provenance`* (`epic-17…:79`) | Provenance pins **4 components** (`echo_spirit`, `equiv_identity/divergent/cosmetic`), each listing `wit/spirit.wit` as a source → ANY in-tree WIT edit changes all 4 `source_sha256`. The other 4 `.wasm` (`echo/spin/benign/mutator`) are core modules with committed `.wat` twins, used only by `fuel_t2_matrix.rs`, outside the gate. No CI or nightly job builds any `.wasm` (the *scoped-nightly byte-rebuild job* cited at `check_equiv_fixture_provenance.rs:15-17` does not exist). |
| P2 | `check-equiv-fixture-provenance` at `discipline.yml:2907-2910`/`:3017-3021` | A **step** inside job `check-wasm-form-equiv` (`:3235`), at `:3303-3304`; enrolled in `aggregate.needs`. |
| P3 | `check-wasm-form-equiv` advisory at `v1_5`, `gate-registry.toml:200-201` | Row is `:212-213` (`v1_0/v1_5=advisory, v2_0=blocking`); `:199-201` is `check-cross-form-equiv`. ⚠ `BindingClass::Blocking` + `dev_enforced_red_blocks` means a RED oracle hard-fails CI **today** — an in-tree WIT edit without the rebuild reds the aggregate. |
| P4 | runner pins `@1.0` at `runner.rs:17,55` | Doc/error text only: `:17`, `:56`, `:172`, `:214`. The real pin is `bindgen!{ path: "../../wit/spirit.wit", world: "spirit" }` (`wit_guest.rs:8-11`) — no version literal, no `with`, no `async`. Adding a world `import` forces a host trait impl (no silent ignore). |
| P5 | `frame_bridge.rs:389-395,401-402,418-431` | Gap doc `:9-21`; scope Debug-lowering `:216`; `scope_from_debug_string` = `None` `:395-397`; `lower` `:403`; `lift` `:419`, defaults `:427`/`:430`/`:431`; `PriorDistillateRef.intent_lineage` defaulted `:315`. Domain types: `IntentClass` `maos-domain/src/invariants/i1.rs:135`, `Scope` `:60`, `ConsentEnvelope` `frame.rs:431`, `IntentLineage(Vec<A2AIntent>)` `invariants/i13.rs:34`; `IacFrame` `frame.rs:26` (intent `:33`, consent `:36`). |
| P6 | *a `xtask/abi-ratifications.toml` entry covers the Rust-side `LogRecall*` types* (17-3b AC2) | The types live in **`crates/maos-domain/src/log_recall.rs`**, not `maos-spirit-abi` (zero recall hits there). `abi-diff` is `cargo-public-api` over `crates/maos-spirit-abi/Cargo.toml` only (`xtask/src/abi_diff.rs:8`) → it cannot see them; no recall row exists in `abi-ratifications.toml`. Q2 states whether 17-3b AC2's ratification clause is moot. |
| P7 | *a filter naming another `spirit_pid` returns the typed refusal* (17-3b AC1) | **Not expressible.** `LogRecallFilter` (`log_recall.rs:13-38`) has no pid/team field — pinned as a security property by `crates/maos-bin/tests/cross_team_consent_13_3.rs:290-308`. Scope is the positional caller pid: `query_page` stamps `FrameFilter{spirit_pid: Some(pid)}` (`adapter/log_recall.rs:309,:315-316`). The only typed cross-Spirit refusal is `fetch` → `ScopeViolation` (`:482-487`). ⇒ the side channel must bind pid from the channel (Q4c). |
| P8 | *in-proc today only via `KernelCtx.log_recall`* (`kernel_ctx.rs:27,85`) | `KernelCtx` has the field `:27` + builder `with_log_recall` `:85-88` and **no getter/op**. `LogRecallAdapter` is in **maos-iac** (`adapter/log_recall.rs:62-73`, `new(Arc<TransparencyLogAdapter>)`), re-exported by `maos-kernel-core/src/iac.rs:13`. Root builds it at `main.rs:2397-2398`, `Arc` `:2425`. Closest out-of-kernel shape: `crates/maos-bin/src/cross_wall_log_read.rs` (~70 lines, `open_read_only` → `query_page` `:68`). |
| P9 | *runner launched through `spawn_and_bridge` under the existing T2 path; exercised by `t2_sandbox_kill.rs:29-39`* (17-3b AC1) | **`spawn_and_bridge` applies NO sandbox** — bare `Command::new(&spec.program)` (`runtime.rs:453`, `:465`); `spawn_sandboxed` (`security/sandbox/mod.rs:136`) has **zero production callers**; `t2_sandbox_kill.rs` never launches the runner (it runs a native probe, `/bin/true`, `cat` via `spawn_sandboxed`). T2 seccomp allowlist (`linux.rs:214-267`, +legacy `:26-28`) lacks `socket`, `connect`, `rt_sigaction`, `clone3`, `poll*`/`epoll*`; unmatched → `EPERM` (`:306-311`). ⇒ runner-under-T2 is unproven, and D-C's *socket path handed via argv/env* cannot `socket()`/`connect()` under it. Q4b measures both. [INFERENCE until measured: wasmtime's trap-handler `sigaction` and the compile-watchdog thread spawn may fail under EPERM.] |
| P10 | `resolve_launch(WasmComponent)` caller `worker_spawn.rs:482` | Production caller `worker_spawn.rs:531-537`, `SpiritForm::NativeSubprocess` hard-coded at `:532`; port built `main.rs:2321-2325`; trait `maos-host/src/lib.rs:137-140`. |
| P11 | unix socket service reusable | **Zero** `UnixListener`/`UnixStream` in `crates/`; maos-bin tokio features `rt-multi-thread, macros, signal, time` (`Cargo.toml:89`), no `net`; `maos-wasm-host` has tokio as a dev-dependency only (`Cargo.toml:57`). |
| P12 | *the world imports nothing* (`spirit.wit:219-232`) | True at WIT level (`:219-230`, zero `import`). [INFERENCE — T2 verifies with `wasm-tools component wit`] `use frames.{iac-frame, halt}` (`:220`) makes the component import the type-only instance `maos:spirit/frames@1.0.0`; that is where the package version appears on the wire. World-level function exports (`handle-frame`, `on-start`, `on-shutdown`) are unversioned names. |
| P13 | Node for componentize-js | CI pins `node-version: '20'` (`discipline.yml:1086-1088`, `:2980-2982`); Node 20 reached end-of-life 2026-04-30. componentize-js 0.23.0 was published from Node 24.20.0; its npm metadata declares no `engines`. Q1 records which Node works. |
| P14 | toolchain | Local: node 20.19.2, npm 9.2.0; `wasm-tools`, `jco`, `cargo-component`, `wasm32-wasip2` target **absent**. 11-1a built the in-tree guests as `wasm32-wasip2` with `wit-bindgen = "0.44"` (`11-1a-…md:192,238`); guest Cargo.tomls carry their own `[workspace]` and committed `Cargo.lock`. |
| P15 | podman in CI | All T3 jobs `runs-on: ubuntu-latest` with unpinned `sudo apt-get install -y podman` (`discipline.yml:1614-1617`, `:1581-1582`); rootless at test time. GitHub downgraded Ubuntu 24.04 images from podman 5.8.4 to the distro **4.9.3**, rollout completed 2026-09-06 (actions/runner-images#14642). 4.9.x rootless default network is slirp4netns unless `containers.conf` selects pasta (pasta default arrives with podman 5). Local dev host: podman 5.4.2, but rootless `podman info` fails here with `cannot clone: Operation not permitted` → CI is the binding measurement. |
| P16 | kloc for spike code | `spikes/` is counted by `kloc-check` as `(unknown:spikes)` (ceiling 244, measured 144). See `kloc_grant`. |
| P17 | `--network=none` is free to change | It is a gated literal: `decision_adrs_and_provisioning.rs:313-327` requires `"--network=none".to_string()` in `t3/argv.rs` (`:72`); five `crates/maos-eval/fixtures/t3-escape-corpus-v0/network_escape/*.json` + `capability_escape_cap_net_raw_002.json` encode it as the expected blocker. `t3/mod.rs:39-40` (DR-5.5a-6) records *MCP outbound routed through parent* — a parent-relay design, never built. |
| P18 | Q1's TS Spirit lives *under `examples/example-spirit-ts`* (`epic-17…:79`) | Moved to `spikes/…/ts-guest/` by AC2 (spike code never merges; `examples/` is untouched). The example exports only `onIdle` (`src/index.ts`) and maps to none of the world's three exports, so a WIT-facing adapter is new code either way. 17-3b owns the in-place rewrite of the example (and `manifest.toml:7` `forms = ["ts-inproc"]`, R17-18). Filed to the epic via §R5. |
| P19 | T2 can host a Rust process (`epic-17…:88` 17-3b AC1: *the runner is launched … under the existing T2 path*) | **MEASURED by emulation at creation (2026-09-25).** An `LD_PRELOAD` shim returning `EPERM` from `poll()` — the allow-list's mismatch action (`linux.rs:306-311`) for a syscall it omits — kills a hello-world Rust binary before `main`: `rc=134` (SIGABRT), no output. `signal()`→`EPERM` alone does the same (`fatal runtime error: assertion failed: signal(libc::SIGPIPE, handler) != libc::SIG_ERR`). Control — shim loaded, not toggled — `rc=0`. The T2 suite's own `forbidden-syscall-probe` and `maos-wasm-runner --component tests/fixtures/wasm/echo_spirit_component.wasm` both die the same way (`rc=134`; baseline `rc=0`), never reaching `ptrace` / `on-start`. Cause: Rust std `fn init` → `sanitize_standard_fds` (`poll` on fds 0–2, `abort()` on any errno but EINTR/EINVAL/EAGAIN/ENOMEM) → `reset_sigpipe` (`rtassert!` on `signal`); `poll`/`ppoll`/`rt_sigaction` are absent from the allowlist (`:214-267`, legacy `:26-28`). [INFERENCE] Under the real filter the death signal differs — glibc `abort()` needs `gettid`/`tgkill`/`rt_sigaction`, all denied — but it is **never SIGSYS**, which is what `t2_sandbox_kill` asserts. Q4b layer 2. |
| P20 | T2 enforcement is proven by `t2_sandbox_kill` (and by `sandbox_enforcement_linux.rs` and the escape-detector corpus) | **None of the three can have observed T2 kill anything.** Locally: `maos: seccomp apply failed` ×3, then `test result: ok. 3 passed` — `skip_if_perm_denied` (`t2_sandbox_kill.rs:32-41`; same helper `sandbox_enforcement_linux.rs:19-23`) turns `PermissionDenied` into an early `return`, and `maos-escape-detector/tests/common/mod.rs:152-166` skips on `PermissionDenied` **or** `Unsupported`. In CI, `wasm-host-tests`, `workspace-test-suite` and `check-escape-detector` are `success` on run 36094638768 @`92911f59`. A kernel without Landlock returns `ENOSYS` (`linux.rs:80-83`), which makes the two kernel/wasm helpers PANIC (red). Green ⇒ Landlock enforced ⇒ `execve` gets `EACCES` (P21) — or seccomp refuses — ⇒ `PermissionDenied` ⇒ **skip, reported as pass**; and even with exec allowed, P19 means the probe could never die with the asserted SIGSYS. **Operator read of that job's log (2026-09-25): `grep -E 'SKIP\|landlock\|seccomp apply failed'` → empty.** Provided the fetched log was non-empty, that leaves one path: the pre-exec failures (`maos: landlock not enforced` / `landlock restrict_self failed` / `seccomp apply failed`, `linux.rs:81-99`) are raw `write(2)` calls from the child that libtest cannot capture — so on CI Landlock was enforced and seccomp installed — while `SKIP …` is an `eprintln!` (`t2_sandbox_kill.rs:36`) that libtest captures for a passing test, and the job runs `cargo test -p maos-host -p maos-wasm-host --locked` without `--nocapture` (`discipline.yml:2928`; its own comment says the T2 column *self-skips*, `:2912-2913`). What remains is `execve` → `EACCES` → skip. [INFERENCE until T6(a) prints the `SKIP` line with `--nocapture`; the log is unreadable without a token — unauthenticated `GET /repos/lunarpulse/maos/actions/jobs/107944202958/logs` → 403.] The three suites → `17-6` AC4; the class (skipped ≠ passed) → `20-3a-gate-honesty` (RT-9). |
| P21 | T2 can start a program at all | **MEASURED at creation — it cannot, wherever Landlock is enforced.** `prepare_landlock` handles every V1 right (`AccessFs::from_all(ABI::V1)`, `linux.rs:172`, which includes `Execute` and `ReadFile`) and only ever adds `ReadFile\|ReadDir` (`FsRead`, `:181-184`) or `+WriteFile` (`FsWrite`, `:189-192`) — never `Execute`, and never read access to the program, loader or libs unless a Spirit scope happens to cover them. Reproduced with the locked `landlock 0.4.4`, the ruleset copied verbatim and `restrict_self` in `pre_exec` as `spawn_sandboxed` does (`:72-84`): control (no Landlock) `/bin/true` → exit 0; kernel ruleset with `t2_spec`'s empty scopes → spawn `Err(PermissionDenied, os error 13)`; + an `FsRead` grant on `/usr` → same `EACCES`; + `Execute\|ReadFile\|ReadDir` on `/` (positive control) → exit 0. Same four results for `forbidden-syscall-probe`. `spawn_sandboxed` has zero production callers (P9), so no shipped path depends on T2 — but every T2 proof in the repo is vacuous (P20), 17-2 AC1's *T2 fixture child* cannot exist, and 17-3b's S2-for-WASM needs kernel lines (Q4b layer 1; vehicle RT-8). |

### Creation-time experiments (re-run first in T0; commit sources + logs under `evidence/`)

Both ran outside the repo on 2026-09-25 against HEAD `92911f59`. The dev pass re-runs them, commits the sources as
`evidence/p19-shim.c` and `evidence/p21-landlock-repro.rs.txt` (Rust is `.rs.txt` — P16), and pastes the output.

- **P19 (seccomp layer, emulated).** `shim.c`: `poll()` returns `-1/EPERM` when `P19_POLL` is set, `signal()`
  returns `SIG_ERR/EPERM` when `P19_SIGNAL` is set, otherwise both behave normally (the fallback converts the
  `timeout` argument for `ppoll` — passing `NULL` blocks forever and hangs the control). `gcc -shared -fPIC -o
  shim.so shim.c`; run each target with `LD_PRELOAD=./shim.so` and each toggle, plus the no-toggle control.
  Observed: `hello` 0 / `P19_POLL` 134 / `P19_SIGNAL` 134 (`fatal runtime error: assertion failed:
  signal(libc::SIGPIPE, handler) != libc::SIG_ERR`) / control 0; `forbidden-syscall-probe` 0 → 134;
  `maos-wasm-runner --component tests/fixtures/wasm/echo_spirit_component.wasm </dev/null` 0 → 134, control 0.
  Limit: emulates only the two calls Rust std makes first; the real filter's full effect is Q4b layer 2 (CI).
- **P21 (Landlock layer, real).** Scratch crate, own `[workspace]`, `landlock = "=0.4.4"` (the locked version),
  `cargo build --offline`. `prepare_landlock` copied verbatim (`ABI::V1`, `CompatLevel::BestEffort`,
  `handle_access(AccessFs::from_all(abi))`, `create()`), `restrict_self()` inside `Command::pre_exec`, a
  `NotEnforced` status mapped to `ENOSYS` as `linux.rs:80-83` does. Four cases per target: no Landlock (control);
  empty scopes (`t2_spec`); `+ PathBeneath("/usr", ReadFile|ReadDir)` (what an `FsRead` scope adds);
  `+ PathBeneath("/", Execute|ReadFile|ReadDir)` (positive control). Observed for `/bin/true` and
  `forbidden-syscall-probe`: exit 0 / `EACCES` / `EACCES` / exit 0. ⚠ Apply `restrict_self` only in the child: a
  first draft that called it in the parent restricted the harness itself and denied even the control.

### Design in force (do not re-decide — measure)

- **D-C** (ADR-060 §D-C `:96-106`, normative): *WASM recall is built as a `maos-bin` side-channel service over a
  Unix socket. The daemon chooses and passes the socket path to the runner. The runner's stdio continues to be the
  ADR-032 kernel bridge and remains byte-identical … kernel-core delta is zero … Story 17-3a prices and validates the
  design before Story 17-3b builds it. The WIT world gains only the versioned capability surface selected by that
  spike.* Rejected alternative: in-bridge request/response, kernel-Δ +80–150 (`preflight-r2/epic-17.md:183`).
- **D-B / ADR-061** (`:56-94`): Worker keeps T3 network isolation; *the Worker's only reachable endpoint is a
  daemon-side proxy on host loopback* (`:59-61`); proxy holds the raw key, allowlists, journals every refusal
  before returning it (`:63-66`); per-spawn audience-bound bearer, NOT the wire `CapabilityToken` (`:68-73`);
  ordered cutover, `env_clear` never standalone (`:75-85`). **The ADR names no container-network mechanism** —
  selecting and pricing one is Q6. Rejected: kernel-proxied inference (`preflight-r2/epic-17.md:180`).
- **ADR-060 forms:** `wasm-component` is the third-party token; `wasm` stays invalid; `rust-inproc` refused from
  the registry (17-3b AC4, not here).
- **Hold 2** (`RELEASE-HOLDS.md:28`, mitigation `:32-38`; `docs/compliance/export-counsel-precondition.md`):
  publishing a WASM-bearing binary is blocked; dev-line source/fixtures are permitted. The spike publishes nothing.

### Current state of what the spike reads (UPDATE files: none)

No production file is modified. The files the spike **copies or calls** and must leave byte-identical:
`wit/spirit.wit` (4 provenance `source_sha256` depend on it); `crates/maos-wasm-host/src/{runner.rs, wit_guest.rs,
frame_bridge.rs, host_state.rs}` (runner: sync `Store`, fuel on, `Linker` = WASI p2 only, `WasiCtxBuilder::new().build()`
— no preopens/network/inherited stdio, `host_state.rs:20-25`; stdin/stdout are the ADR-032 bridge, `runner.rs:185-188`,
`pump_frames` `runner.rs:233-271`; argv only `--component`/`--fuel`, unknown arg fatal `runner.rs:127`; exit codes 0/1/3/4 `runner.rs:50-64`);
`tests/fixtures/wasm/*` + `equiv-fixtures.provenance.toml`; `crates/maos-kernel-core/src/security/sandbox/**`
(98-file content-hash pin); `crates/maos-iac/src/adapter/{log_recall.rs, transparency_log.rs}`.

### Q4 — recall WIT sketch (a starting point, the spike decides)

```wit
package maos:spirit@1.1.0;
interface recall {
    // FrameKindLabel has 30 variants and is #[non_exhaustive] (log_recall.rs:41-90);
    // frames.frame-kind has 17 — a separate label enum is likely needed.
    enum frame-kind-label { /* … */ }
    record recall-cursor { last-timestamp-ns: u64, last-frame-id: list<u8> }          // [u8;16]
    record recall-filter { kind: option<frame-kind-label>, since-ns: option<u64>, until-ns: option<u64>,
                           limit: u32, cursor: option<recall-cursor>, intent-filter: option<string> }  // NO pid, by design (P7)
    record recall-entry  { frame-id: list<u8>, timestamp-ns: u64, kind: frame-kind-label, intent: string,
                           peer-spirit-pid: u32, payload-available: bool }
    record recall-page   { entries: list<recall-entry>, next-cursor: option<recall-cursor> }
    record fetch-response { frame-id: list<u8>, timestamp-ns: u64, kind: frame-kind-label, intent: string,
                            payload-redacted: list<u8>, capability-token: option<list<u8>>, origin: frames.frame-origin }
    variant recall-error { scope-violation(list<u8>), frame-not-found(list<u8>), storage(string),
                           invalid-cursor(string), limit-exceeded(u32), cross-wall-denied(string) }
    recall: func(filter: recall-filter) -> result<recall-page, recall-error>;
    fetch:  func(frame-id: list<u8>) -> result<fetch-response, recall-error>;
}
// world spirit { import recall; … existing exports … }
```
`limit` is `usize` in the domain, clamped to `MAX_LIMIT = 1024` (`log_recall.rs:95,:98-115`).

### Q6 — mechanisms to measure (all rootless, CI-first)

| | Mechanism | Expected tension (verify, do not assume) |
|---|---|---|
| M1 | `--network=slirp4netns:allow_host_loopback=true` → host at `10.0.2.2` (`/usr/share/containers/containers.conf:663-664`) | Restores a full outbound netns → internet likely reachable (rootless podman cannot program nftables, `epic-17…:61`); drops the gated literal (P17). |
| M2 | pasta with host-loopback mapping / `host.containers.internal` | Availability on podman 4.9.3 + Ubuntu `passt` package unknown; same outbound-internet question as M1. |
| M3 | keep `--network=none` + `--volume=<host dir>/egress.sock` + in-container TCP→unix forwarder on `127.0.0.1` (vendor CLIs take http base URLs) | Keeps the literal and denies internet by construction; costs a forwarder in the signed T3 image and a socket the container's mapped uid (`--user=65534:65534`, `t3/argv.rs:75`, remapped by the rootless userns) can connect to — record the file-mode/custody implication vs the 0600 precedent. Verify `lo` is up under `--network=none`. |

`--network=host` defeats T3 and reds the ADR-061 gate — not a candidate.

### Guardrails (disasters this story exists to prevent)

- **Do not edit `wit/spirit.wit` or any fixture in-tree.** P1/P3: that reds `check-wasm-form-equiv` today. All WIT
  work is on `spikes/…/wit-1.1/`.
- **Do not answer from reading.** Every GO needs a transcript. `NOT-MEASURED` is an honest, acceptable verdict;
  an inferred GO is a review finding.
- **Q5's host and guest must come from DIFFERENT WIT versions** (E15-A6: a probe that bindgen's both sides from the
  edited copy only proves the copy compiles).
- **Q4 is not answered by an unsandboxed relay alone** — 17-3b AC1 claims the runner runs under T2 (P9); a relay
  that only works bare is `GO-WITH-CONDITIONS` at best, with the condition priced.
- **No new dependency in any workspace member**; spike crates carry their own `[workspace]` + `Cargo.lock`.
- **Do not use the `-e tests` kloc exclusion as a hiding place** (naming a spike dir `tests/` to escape the count);
  use `.rs.txt` per the 11-0 idiom.
- **No `continue-on-error` in the Q6 probe job**, and it never lands on `main`.
- Rule 10: every finding names a vehicle (17-1, 17-3b, 20-3a by charter) — none goes to `epic-17-retrospective` by default.

### Latest technical facts (fetched 2026-09-25)

- `@bytecodealliance/componentize-js` **0.23.0** (2026-09-21; deps `@bytecodealliance/jco ^1.15.1`, `wizer ^10`,
  `weval ^0.5`); experimental (*no guarantees … breaking changes may be made*); SpiderMonkey via StarlingMonkey,
  *total embedding size is around 8MB*; imports must be synchronous (only exports may be async); default features
  `stdio, random, clocks, http, fetch-event` import WASI interfaces — disable `http`/`fetch-event` for the HEAD
  runner; `disableFeatures: all` yields a pure component that *will not report errors and will instead trap*.
  CLI: `componentize-js --wit <dir> -o out.wasm src.js` or `jco componentize`; sources resolve relative imports only.
- `@bytecodealliance/jco` **1.35.0** (2026-09-24) — `jco guest-types` for TS declarations; `jco componentize` wraps
  componentize-js; also bundles `componentize-qjs` (QuickJS) as an alternative engine. ⚠ jco 1.35.0 depends on
  `@bytecodealliance/componentize-js ^0.22.0` (plus an aliased 0.19.3), so `jco componentize` runs **0.22.x**, not the
  pinned 0.23.0. Invoke the `componentize-js` CLI (or its `componentize()` API) directly for Q1, and record
  `npm ls @bytecodealliance/componentize-js` so the engine version under measurement is explicit.
- wasmtime **46.0.3** (HEAD lock). `component::Linker` docs, *Names and Semver*: when looking up names,
  *semver-compatible names are automatically consulted* — a component importing `a:b/c@0.2.0` resolves to a
  host-defined `a:b/c@0.2.1`, and the reverse. This covers **imports**; whether `bindgen!`-generated **export**
  lookups and type-only instance imports tolerate `1.0.0` vs `1.1.0` is exactly Q5 — measure it.
- GitHub `ubuntu-latest` (24.04, image 20260907): Podman **4.9.3**, Node 22.23.2 default (CI overrides to 20),
  Rust 1.98.1.

### Previous story intelligence

- **11-0 (the rule-5 precedent):** loose files under `spikes/`, README + findings story; kernel-Δ proven by survey;
  its *What the spike did NOT prove* section (`story-11-0-wasm-host-spike.md:46-54`) is the failure mode this story
  forbids for Q1/Q4/Q5/Q6. Reuse its layout; reuse its `.rs.txt` idiom for over-budget Rust.
- **11-1a / 11-1b:** real `wasm32-wasip2` guests via `wit-bindgen 0.44`; *No real `maos:spirit@1.0` component fixture
  existed; core-module fallback masked this* (`11-1a-…md:209`) — do not let a core module stand in for a component
  in any probe. The equiv fixtures share logic across forms (`11-1b-…md:44,73`).
- **Epic 16 / 16-6:** stories disprove their own epics' premises at creation; the fix is §R-numbered OLD→NEW edits
  filed in the same commit, never silent drift. E15-A6's null-control question found something in all seven Epic-16
  nets (`sprint-status.yaml` E15-A6 row).
- **Epic-16 retro:** routes nothing to 17-3a (`deferred-work.md` has no `17-3` row); R17-22 (bridge teardown) rides
  17-1, not here.

### Git intelligence (last commits, `main`)

`92911f59` records v0.1.0-alpha.1 released/verified · `0c661995` first macOS build, kernel re-pin 24922→24920
(FLAG-Winston) · `01655f2a` SLO live test · `67bf70fa` two test races fixed by reproduction then mutation ·
`79610248` ship-gate completeness re-verify. None touches the WASM host, WIT, recall or T3 argv; the relevant
constraint they add is the kernel pin **24920** and a green aggregate the spike must not disturb.

### Project Structure Notes

```
spikes/story-17-3a-wasm-recall/
  README.md                 # 11-0 shape: answers, not-a-member, files table, reproduce steps
  .gitignore                # node_modules/, dist/, target/, out/
  wit-1.1/spirit.wit        # the @1.1.0 copy (recall + frame fields)
  ts-guest/                 # package.json, package-lock.json, tsconfig.json, src/spirit.ts
  rust-guest/               # stub recall guest ([workspace], Cargo.lock)
  host/                     # spike runner copy + recall service + T2 harness ([workspace], Cargo.lock)
  egress/                   # Q6 probe script + the throwaway workflow text (.yml.txt) + forwarder source
  evidence/                 # raw logs per question (q1-*.log … q6-*.log)
```
Not a workspace member: root `Cargo.toml` `members` is explicit (`:3-58`), `default-members = []` (`:59`),
`exclude = ["templates", "templates/spirit-ts"]` (`:61`); fmt gate is members-only; no xtask gate and no workflow
mentions `spikes`. Only `kloc-check` sees it (P16).

### Testing standards

A spike ships **no permanent tests** and adds nothing to any crate's test suite. Proof = executed probes with
committed logs, re-executable by the §A6 evidence layer. The repository gates in T0/T8 are the regression check.

### References

- `_bmad-output/planning-artifacts/epics/epic-17-workers-and-third-party-form-w2.md` — `:47` row, `:75-80` 17-3a,
  `:82-92` 17-3b, `:29` kernel-Δ, `:33` kloc asks, `:35` rules, `:123-195` §R4, `:199-201` dependencies.
- `_bmad-output/planning-artifacts/review-2026-09-04/preflight-r2/epic-17.md` — `:152`, `:172`, `:176`, `:180`, `:183`, `:184`, `:223`.
- `_bmad-output/planning-artifacts/sprint-change-proposal-2026-09-05-round3.md:34-35,:63` (D-B, D-C).
- `_bmad-output/planning-artifacts/epics/epic-15-21-preflight-r2-2026-09-05.md:31,:51-52`.
- `_bmad-output/planning-artifacts/correct-course-input-2026-09-04-confidence.md:27` (rule 5).
- `docs/adr/ADR-060-spirit-forms-by-trust-tier.md` (§D-C `:96`, consumers `:126`); `docs/adr/ADR-061-worker-egress-and-scoped-credential.md:56-94`; `docs/adr/ADR-031-*`, `ADR-032-*`.
- `RELEASE-HOLDS.md:28,:32-38`; `docs/compliance/export-counsel-precondition.md`.
- `_bmad-output/planning-artifacts/prd/functional-requirements.md:28` (FR5), `:75` (FR29), `:83` (FR33).
- `_bmad-output/implementation-artifacts/story-11-0-wasm-host-spike.md`; `spikes/story-11-0-wasm-host/README.md`.
- Code: `wit/spirit.wit`; `crates/maos-wasm-host/src/{runner,wit_guest,frame_bridge,host_state}.rs`;
  `crates/maos-wasm-host/tests/wit_corpus.rs`; `tests/fixtures/wasm/equiv-fixtures.provenance.toml`;
  `xtask/src/check_equiv_fixture_provenance.rs`, `check_wasm_form_equiv.rs`; `crates/maos-domain/src/log_recall.rs`;
  `crates/maos-iac/src/adapter/log_recall.rs`; `crates/maos-kernel-core/src/security/sandbox/{linux.rs,mod.rs,t3/*}`;
  `crates/maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs:453-477`; `xtask/tests/decision_adrs_and_provisioning.rs:296-328`.
- External: https://www.npmjs.com/package/@bytecodealliance/componentize-js ,
  https://github.com/bytecodealliance/ComponentizeJS (README, release 0.23.0),
  https://docs.rs/wasmtime/latest/wasmtime/component/struct.Linker.html (Names and Semver),
  https://github.com/actions/runner-images/issues/14642 (podman 4.9.3 on ubuntu-24.04).

## Dev Agent Record

### Agent Model Used

anthropic/claude-opus-5-5, the orchestrator. It ran T0/T1, wrote the shared `wit-1.1` contract, re-executed the Q1, Q3, Q4 and Q5 probes as a non-author (`evidence/a6-reexec-orchestrator.log`), integrated the verdicts and landed the story. It ran four parallel sub-agents, one per slice: `Q1TsComponentize` (T3), `Q2Q3WitTax` (T4), `Q4Q5Relay` (T2, T5 4a/4c/4d) and `Q4bQ6CiProbe` (Q4b, T6).

### Debug Log References

- T0: `evidence/t0-baseline-gates.log` (kernel pin 24920 · 98 files; kloc `passed`, aggregate 167819, `(unknown:spikes)` 144; provenance 4 fixtures; fmt clean) and `evidence/t0-p19-p21-rerun.log` (P19 rc 0/0/134/134 for `hello`, the probe and the runner; P21 EACCES under the verbatim ruleset for all four targets, and every control runs its own target, which the creation-time draft did not).
- T1: `evidence/t1-toolchain.log` (wasm32-wasip2; wasm-tools 1.259.0).
- Slices: `evidence/q1-*.log`, `q2-*.log`, `q3-*.log`, `q4-*.log`, `q5-*.log`, `q4b-*.log`, `q6-*.log`, `t6-*.log`. The per-slice reports are `q1-driver/q1-componentize-js-report.md`, `wit-1.1-frame/q2-additive-wit-report.md`, `bridge/q3-frame-bridge-report.md`, `t2-ladder/report.txt` and `egress/report.txt`.
- Shared-target provenance: `CARGO_TARGET_DIR=/mnt/build/cargo`, with the repo's `target` a symlink to it. A scratch-worktree build during Q2 briefly replaced the shared `target/debug/maos-wasm-runner`. Every Q1 runner check was therefore repeated against verified main-tree hashes (debug `cf3a4e04…`, release `a596e1b1…`), and the result is logged in `a6-reexec-orchestrator.log`.
- T6 runs #1/#2 failed with exit 101. Root cause, reproduced from the exact branch tree (`git archive 4e944b10`): the branch held 44 of the spike's files and missed `wit-1.2-enum/`, so `host`'s `spike-linker-1-2` bin failed `bindgen!`. The probe was broken, not the platform, so RT-7 did not apply and T6 was re-opened.

### Completion Notes List

- Verdicts: Q1 GO-WITH-CONDITIONS · Q2 GO-WITH-CONDITIONS · Q3 NO-GO as an additive `@1.1` · Q4 GO-WITH-CONDITIONS · Q5 GO-WITH-CONDITIONS · Q6 GO-WITH-CONDITIONS (**M3**). Full rows in §Go/No-Go; §R5 part B rows R17-38…R17-45.
- Q4b in one line: T2's seccomp layer never installed anywhere (F1 order defect); the measured repair is kill-first order + 6 allow-list syscalls + Landlock program/loader/libs rights (+25/−1 across 5 files, prototype built `--locked` rc 0); a forbidden syscall dies SIGSYS 31; the socket-path transport is NO-GO under the candidate (escape surface), the inherited fd completes.
- The `Q4bQ6CiProbe` sub-agent terminated mid-flight (the provider refused the turn, `code=cyber_policy`); the orchestrator completed its remains: the CI runs #9/#10 readout, the socket-escape and LOO close-out, and the kernel-price prototype.
- The CI read token is still absent on this host (provisioning checklist row stays `absent`); every CI verdict above is a public annotation or step conclusion — the evidence channel T6 was designed around.

### File List

- `spikes/story-17-3a-wasm-recall/**` — 188 git-visible files (3.4 MB): the shared WIT contracts (`wit-1.1/`, `wit-1.1-frame/`, `wit-1.2-enum/`, `wit-1.2-same/`), `ts-guest/`, `q1-driver/`, `bridge/`, `host/`, `rust-guest/`, `t2-ladder/` (incl. the archived LOO drivers), `egress/`, `README.md`, `.gitignore`, and 116 `evidence/` files, 107 of them `.log` (`t0-`, `t1-`, `q1-`…`q6-`, `q4b-`, `t6-`, `a6-`, `t8-`; the dev pass committed 115/106, +1 is the review-captured `q6-actions-annotations-run4.json.log`).
- `_bmad-output/planning-artifacts/epics/epic-17-workers-and-third-party-form-w2.md` — §R5 part B, rows R17-38…R17-45.
- `_bmad-output/implementation-artifacts/sprint-status.yaml` — rows 17-1, 17-3a (`done`), 17-3b, 17-6.
- `_bmad-output/implementation-artifacts/deferred-work.md` — "Deferred from: Story 17-3a dev pass" (owner `20-3a-gate-honesty`).
- `_bmad-output/implementation-artifacts/17-3a-wasm-recall-and-componentize-spike.md` — this file (verdicts, excerpts, findings F1–F10 (F10 by the §A6 review), Dev Agent Record).
