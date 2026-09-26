# Story 17-3a — WASM recall, WIT versioning, componentize-js and the 17-1 egress proxy (NO-MERGE SPIKE)

**Status:** spike artifact, measured 2026-09-25/26 against HEAD `92911f59`. It is **not a workspace member**: the root
`Cargo.toml` lists its members explicitly with `default-members = []`, so nothing here is built by the workspace,
formatted by `cargo fmt --all`, or linked into a crate. Each spike crate carries its own `[workspace]` table and
`Cargo.lock`. Each Rust source is named `*.rs.txt` and reaches rustc through Cargo `path =` or `#[path]`; rustc compiles
any path it is given, and `kloc-check` counts only `.rs`, so `(unknown:spikes)` stays at 144 (story P16). **Kernel-Δ 0**:
nothing under `crates/` changed.

The verdicts, prices and consequences live in
`_bmad-output/implementation-artifacts/17-3a-wasm-recall-and-componentize-spike.md` §Go/No-Go. The OLD→NEW edits they
force on 17-1, 17-3b and 17-6 are in `_bmad-output/planning-artifacts/epics/epic-17-workers-and-third-party-form-w2.md`
§R5 part B (R17-38 onward). Every claim below was executed and observed. The logs are in `evidence/`, and
`evidence/a6-reexec-orchestrator.log` holds a non-author's re-execution of the Q1, Q3, Q4 and Q5 probes.

## What this spike answers

| Q | Verdict | In one line |
|---|---|---|
| 1 componentize-js | GO-WITH-CONDITIONS | A TS Spirit becomes a `maos:spirit@1.0.0` component that the **unmodified release** HEAD runner runs. It needs Node 22.20–22.x or ≥ 24.12 (locked `@napi-rs/lzma` engines — 23.x and 24.0–24.11 are unsupported) and a release-built runner (debug compile 10.5 s > the 10 s watchdog), and the component is 12.26 MB (StarlingMonkey embedded). |
| 2 additive WIT | GO-WITH-CONDITIONS | A recall-only `@1.1.0` costs 5 `wit_corpus` records and 4 provenance re-pins. The 4 pinned components do not rebuild to their pinned hashes at HEAD. |
| 3 frame_bridge | NO-GO as an additive `@1.1.0` | A lossless bridge works for the four frame fields (byte-identical CBOR; nested `prior-distillate-ref.intent_lineage` / `working-memory-digest-refs` still default — review), but growing `iac-frame` breaks every `@1.0` component (`expected record of 11 fields, found 8`). |
| 4 recall relay | GO-WITH-CONDITIONS | 4a/4c/4d: the relay reaches the real `LogRecallAdapter` with kernel-Δ 0; `SO_PEERCRED` + one-shot accept are mandatory (off → a same-uid attacker reads A's frames); latency p50 0.71–0.75 ms, per-call cost climbing to ~1.2 ms (cause NOT-MEASURED). 4b (CI run #9, both images): kill-first order + 6 syscall additions + Landlock program/loader/libs runs the runner rc 0; a forbidden syscall dies SIGSYS 31; the socket-path transport is NO-GO (escape surface) — the inherited fd completes. T2 repair price +25/−1 across 5 files. |
| 5 version skew | GO-WITH-CONDITIONS | Old guests run on new hosts, and a package-only bump links both ways. **One** added `frame-kind-label` case breaks both ways. |
| 6 egress proxy | GO-WITH-CONDITIONS: M3 | Measured in CI on ubuntu-24.04 (podman 4.9.3) and ubuntu-26.04 (podman 5.7.0), with identical results. M3 (`--network=none` + a mounted Unix socket + an in-container forwarder) reaches the proxy and not the internet; the socket needs 0666 under the rootless userns remap. M1 and M2 both reach the internet. |

## Build environment

Run everything from the repository root. The binary paths below assume `export CARGO_TARGET_DIR=$PWD/target`. On the
authoring host `target` is a symlink to `/mnt/build/cargo`, which is also `CARGO_TARGET_DIR`, so every crate (spike
crates and scratch worktrees too) shares one target dir. A scratch-tree build can therefore overwrite the plain
`target/{debug,release}/maos-wasm-runner`. A probe that needs **HEAD's** runner builds it from the main tree with
`--locked`, copies it to a private path, and logs its sha256 (`cf3a4e04…` debug, `a596e1b1…` release at `92911f59`).
Toolchain: `rustup target add wasm32-wasip2`; `cargo install --locked wasm-tools` (1.259.0 measured); Node 22.20–22.x
or ≥ 24.12 for `ts-guest/` (locked `@napi-rs/lzma@1.5.1` engines `^22.20 || ^24.12 || >=25` — NOT 23.x, NOT 24.0–24.11).

## Files

| Path | What it is | Q |
|---|---|---|
| `wit-1.1/spirit.wit` | `maos:spirit@1.1.0` = the pinned WIT + `interface recall` + `import recall;`. `interface frames` is byte-identical to `@1.0.0`. | 2, 4, 5 |
| `wit-1.1-frame/spirit.wit` | `wit-1.1` + `iac-frame` intent / consent-envelope / intent-lineage / lossless `scope` | 3 |
| `wit-1.2-enum/spirit.wit`, `wit-1.2-same/spirit.wit` | `@1.2.0` with one extra `frame-kind-label` case / with only the package line changed (the control) | 5 |
| `ts-guest/` | TS Spirit (`src/spirit.ts`), pinned `package.json` + `package-lock.json`, generated `.d.ts` | 1 |
| `q1-driver/` | frames one real ADR-032 frame onto stdin, decodes stdout, times `Component::new` with the runner's `Config`; Q1 report | 1 |
| `bridge/` | spike copy of `frame_bridge` lower/lift extended for the four fields + the round-trip and frame-growth skew probe; Q3 report | 3 |
| `host/` | `spike-runner` (HEAD runner copy, host from `wit-1.1`, implements `recall`), `recall-service` (real `LogRecallAdapter`), `frame-driver`, `spike-linker-1-2{,-same}`; relay/hostile/fd probes | 4, 5 |
| `rust-guest/` (+ `enum-1-2/`, `version-1-2-same/`) | stub recall guests (wasm32-wasip2, wit-bindgen 0.44.0) | 4, 5 |
| `t2-ladder/` | verbatim copies of `prepare_landlock` / `build_seccomp_filters` with candidate additions as data, the local and CI ladder scripts; Q4b report | 4b |
| `egress/` | `egress-probe.sh` (M1–M3), the socat forwarder image, host responders, the throwaway workflow text `ci-probe.yml.txt`; Q6 report | 6 |
| `evidence/` | one log per probe (`t0-`, `t1-`, `q1-` … `q6-`, `q4b-`, `t6-`, `a6-`), the P19 shim and the P21 Landlock repro | all |

## Reproduce

- **T0 (baseline and the creation-time experiments):** `bash spikes/story-17-3a-wasm-recall/evidence/t0-rerun.sh`
  (P19 LD_PRELOAD EPERM matrix and P21 verbatim Landlock ruleset against `/bin/true`, `hello`, the forbidden-syscall
  probe and the HEAD runner; every control runs its own target).
- **Q1:** see `q1-driver/q1-componentize-js-report.md` §Reproduce. In short: Node 22, `npm ci && npm run build` in
  `ts-guest/`, the direct `npx componentize-js … --disable http fetch-event`, then
`target/release/q1-driver encode | target/release/maos-wasm-runner --component <ts-spirit.wasm> | target/release/q1-driver decode` — building HEAD's runner from the main tree `--locked` into a **private** copy per §Build environment (never a shared target dir).
- **Q2:** see `wit-1.1-frame/q2-additive-wit-report.md` §Reproduce. It runs in a scratch worktree with a private
  `CARGO_TARGET_DIR`, and never edits `wit/spirit.wit` in the main tree.
- **Q3:** `cargo run --locked --offline --release --manifest-path spikes/story-17-3a-wasm-recall/bridge/Cargo.toml --bin frame-bridge-probe`.
- **Q4 (4a/4c/4d):** `cargo build --release --manifest-path spikes/story-17-3a-wasm-recall/host/Cargo.toml`;
  `cargo build --locked --release --target wasm32-wasip2 --manifest-path spikes/story-17-3a-wasm-recall/rust-guest/Cargo.toml`;
  `Q4_COUNT=1000 Q4_FETCH_OWNER=A spikes/story-17-3a-wasm-recall/host/run_relay_probe.sh`;
  `Q4_FETCH_OWNER=B spikes/story-17-3a-wasm-recall/host/run_relay_probe.sh`; then `spikes/story-17-3a-wasm-recall/host/run_hostile_probe.py`,
  `…/host/run_fd_vector_probe.py`, `…/host/run_one_shot_idle_probe.py`, `…/host/run_auto_socket_probe.py`.
- **Q5:** `wasm-tools component wit tests/fixtures/wasm/echo_spirit_component.wasm`; the `@1.1` host on the `@1.0`
  fixture via `frame-driver … | spike-runner --component tests/fixtures/wasm/echo_spirit_component.wasm | frame-driver decode`;
  HEAD's runner on the `@1.1` guest; `spike-linker-1-2{,-same}` × `story_17_3a_{recall,enum,version_only}_guest.wasm`
  (commands and exit codes in `evidence/q5-*.log` and `evidence/a6-reexec-orchestrator.log`).
- **Q4b and Q6:** `t2-ladder/report.txt` and `egress/report.txt` §Reproduce. The seccomp layer and the podman
  matrix run only in GitHub Actions (this host cannot install a seccomp filter and its rootless podman cannot
  clone). Copy `egress/ci-probe.yml.txt` to `.github/workflows/` on a throwaway branch, push it, and read the job
  annotations: every measured value is an annotation, because job logs need a token.

## What the spike did NOT prove

- **Production placement.** The relay ran as a separate spike process; the D-C service inside `maos-bin` (and its
  spawn-time pid binding) is 17-3b's to build and measure.
- **Why the pinned components do not reproduce.** The provenance file records no toolchain, so the cause is unknowable
  from the tree (Q2).
- **An additive carrier for the four frame fields.** Q3 measured that growing `iac-frame` breaks `@1.0`; the alternative
  (an import that serves the fields by frame id) was not built.
- **Where recall latency growth comes from.** Q4 measured per-call cost climbing from ~0.09 ms to ~1.2 ms within one
  service lifetime; its cause, and the latency against a realistically sized log, are 17-3b's to measure.
