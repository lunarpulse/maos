# 17-1 FLAG-Winston decision record

**Date:** 2026-10-06  
**Status:** `ratified` 2026-10-06 by the operator (Lunarpulse) — FW-01…FW-04 as below, with variant **B** and the aggregate raise exact at landing (≤ +1182). Proposed earlier the same day.  
**Scope:** story creation and a **throwaway measurement prototype only**. At creation this record changed no runtime, kernel, baseline, KLOC ledger, ratification file, ADR or model allowlist, and the story stayed `backlog`; the operator's ruling (§Disposition) then wrote the ADR-061 amendment and moved the story to `ready-for-dev`. The kernel and KLOC figures move only in 17-1's landing commit.  
**Model after:** `17-3c-early-start-flag-winston-decision.md` (a FLAG-Winston is an architectural *location charter*, not a numerical grant; D-17-3c-ES-04).

## Decision

| ID | Decision | Status |
|---|---|---|
| **D-17-1-FW-01** | The kernel **locations** for 17-1 are `lifecycle/cli_wrapper/runtime.rs`, `security/sandbox/t3/spawn.rs`, `t3/argv.rs`, `lifecycle/cli_wrapper/admission.rs` and — by the measurement — `lifecycle/cli_wrapper/lifecycle.rs` (a test struct literal, +1). `security/manifest.rs`, named by the epic, is a 9-line `pub use maos_manifest::*` shim and carries **0**. **Extended 2026-10-07 (story Q10, deletion-only):** `security/sandbox/t3/cap_audit_bridge.rs` and `t3/quarantine.rs` are deleted, with their two `pub mod` lines in `t3/mod.rs` — no line may be *added* to these three locations. | **Ratified 2026-10-06** — a charter, not a number; deletion-only extension ruled 2026-10-07 |
| **D-17-1-FW-02** | The **numerical ceiling**: variant **B** (delete the dead `ci_default_guard`) — `src_lines` 25293 → **25353** (+60), kloc 19529 → **19564**; or variant **A** (keep it) — 25425 (+132), 19621. The landing commit re-measures after `cargo fmt --all` and re-pins the exact figure; it may not exceed the ratified one without a new request. | **Ratified 2026-10-06: variant B** (≤ 25353 / ≤ 19564) |
| **D-17-1-FW-03** | Out-of-kernel ceilings: `maos-domain` 9270 → **9274** (measured); `maos-manifest` **no raise** (4445 measured, +29 free); `maos-bin`, `maos-egress` (phantom row 780 → exact), `xtask` — exact measured raises **at landing**; **`_aggregate_hardfail` 170884** — the aggregate has **+450** free against an envelope of **+585…+1182**, so an exact raise at landing is part of the request. | **Ratified 2026-10-06:** exact at landing; `_aggregate_hardfail` raise ≤ +1182 |
| **D-17-1-FW-04** | **ADR-061 M3 amendment** (text below): transport by a daemon-owned Unix socket bind-mounted into a `--network=none` container; *on host loopback* superseded; bearer daemon-minted; stale cites corrected. | **Ratified 2026-10-06** — written as `## Amendment — Story 17-1 (transport; ratified 2026-10-06)`, body-anchored in `xtask/tests/decision_adrs_and_provisioning.rs` |
| **D-17-1-FW-05** | Story status: **`backlog`** until FW-02…FW-04 are ruled and Q1–Q7 of the story are answered. The parent moves it to `ready-for-dev` (17-6 precedent, `f86b5d4f`). | **Done 2026-10-06** — `ready-for-dev` |
| **D-17-1-FW-06** | Kernel offset: `t3/cap_audit_bridge.rs` (35 lines, fire-and-forget `emit_t3_escape_block`, superseded by D-10's confirmed write) and `t3/quarantine.rs` (73 lines, `quarantine_spirit`, a stub that always returns `Err` and whose doc falsely claims the revocation applier calls it) have zero callers (R17-83/R17-89). Measured at landing (≈ −110 `src_lines`; tokei unmeasured); file set 98 → 96. It lowers the landing figure and absorbs the lines the prototype omitted (story F-9, D-15); it is **never** spent as a raise elsewhere. | **Taken 2026-10-07** (story Q10 / D-16) |

## Evidence and boundary

Four read-only scouts re-measured every cite of epic §17-1, the header and §R4–§R8 at `0d73cac3` (kernel-core; maos-bin + spirits + tests; CI + xtask + governance + schema; docs + ADR + deferred-work + precedent stories). Their findings are filed as epic §R9 (R17-70…R17-89) and as the story's P1–P22. The two facts that decided the kernel figure:

- **The seam is `spawn_child`** (`runtime.rs:513-532`, 17-6's): `T0 | T3 => cmd.spawn()` bare at `:523-526`. `SpawnedBridge.child` is `Option<BridgeChild>` (`Plain`/`Sandboxed`), so T3 needs a third variant wrapping `SandboxedContainerChild` (R17-5 re-derived); `BridgeSpawnSpec` carries neither the verified image nor a `T3SpawnContext`.
- **Under T3, `env_clear()` on the `Command` clears the podman client**, not the container (podman never forwards the client environment): the container's environment is `--env=NAME` flags with values on the client's environment (never `NAME=VALUE` — argv is readable). `env_clear()` still applies at the one `Command` build for every other tier, which closes the 17-3c deferred row.

## Isolated executable measurement (2026-10-06)

**Method.** Detached worktree at `0d73cac3`; the kernel-side changes written to compile (`cargo check -p maos-kernel-core --lib`; then `maos-bin`, `maos-bench` and `xtask`, which depend on `BridgeSpawnSpec`, with mechanical `t3: None` / `egress: vec![]` scaffolding), `cargo fmt --all`, then `check-kernel-baseline` (physical lines — the pinned number) and `kloc-check --json` (tokei 14.0.0 — the budgeted number). All prototype code was **reverted** with `git checkout`; the final tree changes only story, epic, tracker, deferred-work and evidence files. Evidence in `17-1-evidence/`: `prototype.diff` (variant A), `prototype-with-dead-guard-deleted.diff` (variant B, the whole prototype), `measurements.json`.

| Counter | Base | Variant A | Δ | Variant B | Δ |
|---|---:|---:|---:|---:|---:|
| Kernel `src/` physical lines | 25,293 | 25,425 | **+132** | 25,353 | **+60** |
| Kernel tokei (`kloc-check`) | 19,519 | 19,621 | **+102** | 19,564 | **+45** |
| Kernel files | 98 | 98 | 0 | 98 | 0 |
| `kloc.toml` ceiling / raise needed | 19,529 | — | +92 | — | +35 |
| `maos-domain` tokei | 9,270 | 9,274 | +4 | 9,274 | +4 |
| `maos-manifest` tokei | 4,445 | 4,447 | +2 | 4,447 | +2 |
| Aggregate tokei (with scaffolding) | 170,434 | 170,549 | +115 | 170,492 | +58 |

| Kernel file (`src_lines`, net) | A | B |
|---|---:|---:|
| `lifecycle/cli_wrapper/runtime.rs` | +67 | −4 |
| `lifecycle/cli_wrapper/admission.rs` | +27 | +27 |
| `security/sandbox/t3/spawn.rs` | +23 | +23 |
| `security/sandbox/t3/argv.rs` | +14 | +14 |
| `lifecycle/cli_wrapper/lifecycle.rs` | +1 | +1 |
| `lifecycle/cli_wrapper/mod.rs` | 0 | −1 |
| `security/manifest.rs` | 0 | 0 |

**What the figure contains:** the T3 argv extras (`--volume=…:/proxy/egress.sock:rw`, `--http-proxy=false`, `--env=NAME`); piped `spawn_t3` with a client environment allowlist and the guard built *before* `inspect_container_host_pid`; `BridgeChild::Container` with a `kill` that also runs `<runtime> kill`; `T3Launch` and `BridgeSpawnSpec.t3` (a T3 spec without it is `SandboxRefused`, never bare); `env_clear()` at the `Command` build; a bounded `read_newline_delimited` (the 17-3c deferred row); a stderr-marker kind-8 branch in `pump_to_journal` (T3, stderr only, confirmed write); `check_egress` + two parameters on `admit_cli_wrapper_journaled` + a probe `env_clear()`.

**What it does not contain (the dev adds, and the landing figure is re-measured):** `#[cfg(unix)]` on the pump branch (+1); any test placed under `src/` (put new tests under `tests/`); a bounded post-exit drain if the R17-22 measurement under `spawn_t3` shows the defect does not dissolve; surface-pin rows. The prototype's admission doc-comment sits above `check_egress` (should sit above `admit_cli_wrapper_journaled`) and `T3SpawnContext.env` trips `check-service-boundary`'s P3 — both are measurement artifacts, fixed when re-derived.

**Gates on the prototype tree** (variant B): `check-kernel-baseline`, `kloc-check`, `check-fkcs` (reads the same pin), `check-service-boundary` (unclassified new public symbols + P3) fail — each **only** because of the pending grant / surface pin; `check-exit-commands` and `gen-abi-docs` also failed on the first run — an environment fault of the measuring shell (`CARGO_TARGET_DIR=/mnt/build/cargo` was exported, so the `target/debug/{maos,maosctl}` and `target/doc/*.json` those gates read were outside the worktree's `target/`), not of the prototype; the other eight pass. **On the final docs-only tree (own target, variable unset) all 14 gates and `cargo fmt --all --check` pass and `cargo test --offline --locked -p xtask` reports 941 passed.**

**Scratch proof that the grant is what is missing** (variant B applied, then reverted): `src_lines = 25353` plus the `--emit-pin` `[kernel_src]` block spliced into a scratch copy → `check-kernel-baseline: PASSED (maos-kernel-core/src = 25353 lines, 98 files, pinned 25353)`; scratch `kloc.toml` kernel 19564 / domain 9274 / bin 24605 → `kloc-check --json` `passed: true`, `over_budget: []`. Not exercised in the scratch (it is the landing commit's work, listed in the story): the four `25293` literals in `kernel_pin_content_hash_16_0.rs`, `RATIFIED_AT_EASING`, the surface pin.

## Proposed ADR-061 amendment (for ratification; to be written under `## Amendment — Story 17-1 (transport; ratified <date>)` with `require_body_contains` anchors, precedents ADR-060 `:185`, ADR-062 `:115`)

> The Decision's *a daemon-side proxy on host loopback* is superseded. A `--network=none` container cannot reach host loopback by construction, and the two mechanisms that can — `slirp4netns:allow_host_loopback=true` (M1) and `pasta` (M2) — were measured to reach the **internet** on ubuntu-24.04 (podman 4.9.3) and ubuntu-26.04 (podman 5.7.0) (17-3a Q6, runs 36207588609, 36206431636). The Worker's only reachable endpoint is a **daemon-owned Unix socket** bind-mounted into the container at `/proxy/egress.sock` (mode 0666 inside a daemon-owned 0700 directory: a 0600 socket is refused under the rootless user-namespace remap), served inside the container by a forwarder on `127.0.0.1:18080`; `--network=none` is retained. The bearer is **minted by the daemon's composition root**, not the kernel (the Decision never required a kernel mint), stored only as a hash by the proxy, and verified at the proxy boundary. The proxy journals every refusal by a **confirmed** write (`TransparencyLogAdapter::insert_frame_event`, now `transparency_log.rs:612`) before it returns the refusal. The Context's *+65–130 … 18935/18935* kernel paragraph is replaced by the measured figure ratified for the story. Order is unchanged: proxy and bearer, then the T3 route, then `env_clear` and the guard inversion in the same commit.

## Grant protocol (this record grants none of it)

The operator decides FW-02/03/04. If approved, the **landing commit** re-pins in one change, in this order (`kernel-core-baseline.toml:786-821` at `d371b5a4`; `:782-807` at creation): class rows in `xtask/kernel-api-classes.toml` first → regenerate the surface pin → `src_lines` + HISTORY (line 505 stays line 505) → file digests + `set_hash` (`:684`; the 17-3c CI repair re-hashed `linux.rs` line-neutrally after creation) → the four literals in `kernel_pin_content_hash_16_0.rs` → `kloc.toml` rows (kernel, domain, bin, `maos-egress`, xtask, `_aggregate_hardfail`) → `RATIFIED_AT_EASING` → `check-service-boundary` **and** `check-kernel-baseline` at HEAD, with every addition and removal enumerated in the story record. If the work lands before the grant, it leaves those files unchanged and returns the request with the measured figure. ⚠ **Zero slack (2026-10-07 frontier re-verification):** the ruled ceiling 25353 equals the variant-B prototype exactly, and the story already names lines the prototype omits; a landing figure above 25353 is a new request (story Q10), never a re-pin.

## Disposition

- **Story:** created, validated against the create-story checklist, **`backlog`** — *story created 2026-10-06; awaiting operator kernel grant*. 17-1 does **not** start development until the operator rules FW-02…FW-04 and Q1–Q7.
- **Operator questions:** Q1 (variant A/B + aggregate), Q2 (ADR amendment), Q3 (one story or two), Q4 (exit lines 2/3 on 24/26), Q5 (admission executes the manifest binary on the host; `deny_unknown_fields` owner), Q6 (hermetic-suite sweep), Q7 (escape-corpus scope) — in the story.
- **Ruling (2026-10-06, Lunarpulse):** FW-02 variant B; FW-03 exact at landing, aggregate ≤ +1182; FW-04 ratified; Q3 one story / two commits; Q4 fix at origin; Q5 probe → `19-3`, `deny_unknown_fields` → `20-3c`; Q6 D-8 triage; Q7 five `--network=none` scenarios. Story moved `backlog` → `ready-for-dev`; rulings table in the story §Operator questions.
- **Ruling (2026-10-07, consensus under the operator's decision rule — long-term correctness, PRD strictly, correct drift, honour contracts, fill gaps along the roadmap):** story Q8–Q11 (D-5, D-14…D-17). For this record: FW-01 extended deletion-only and FW-06 taken (Q10); FW-02's ceiling unchanged (25353 / 19564) and now expected to be undershot; FW-03's `maos-domain` 9274 is **exceeded by D-14** (Q9: `EgressDestination` beside `HostGrant`), so the landing returns its exact figure to the operator as a new request (ledger envelope *17 +25–50*), with D-14's `maos-manifest` fallback if refused; FW-03 gains a `maos-capability` decrease (D-16 removes two `AuditDropSite` variants). Q5's `deny_unknown_fields` owner moves from `20-3c` to 17-1 (Q8).
- **Operator sign-off (2026-10-07, Lunarpulse — binding):** S1 approved (`deny_unknown_fields` on `[cli_wrapper]` in 17-1; Q5's second half superseded; schema-version gap → `20-5`); **S2 approved — FW-03 `maos-domain` ceiling 9274 → ≤ 9320** (covers the two variants and D-14's `EgressDestination`; exact figure pinned at landing; D-14's `maos-manifest` fallback retired); **S3 approved — FW-01 deletion-only extension and FW-06 taken** (FR13 quarantine semantics → `20-1`). FW-02's kernel ceiling (≤ 25353 / ≤ 19564) is unchanged.

## Development measurement request — 2026-10-07 (not ratified)

The Task-1 candidate is formatted with `cargo fmt --all` and measured by the
existing `xtask kloc-check --json` instrument. `maos-domain` is **9333** versus
the signed S2 ceiling **9320**: **13 LOC over the ceiling**, **63 LOC over the
9270 baseline**. The candidate contains `EgressDestination` (strict DNS/IPv4
authority parsing, canonical serialization, legacy bare grants meaning port
443), typed grant carriers, the two admission errors, and the required migration
of the existing in-source grant fixture. The parser reuses its input allocation
and serialization uses `collect_str`; grammar and length checks are retained.

The first parser measurement was 9326. Simplifying port parsing and replacing
the obsolete egress-grant commentary reduced the parser-only candidate to
9321. The complete typed candidate above is the request, not a ratification.
No KLOC ledger, kernel baseline, surface pin, or model allowlist was changed.
Development is halted at Task 1 pending a revised domain ceiling; the kernel
and aggregate grants remain unchanged. The manifest measures **4456**, within
its unchanged **4474** ceiling; aggregate candidate **170839** is below the
existing hard-fail **170884**. `maos-bin` measures **24607**, whose exact landing
row is already authorized by FW-03.

### Operator ruling — 2026-10-08

Lunarpulse selected **Authorize ceiling 9,333** for the measured Task-1 domain
candidate: S2 / FW-03 is revised from ≤ 9320 to **≤ 9333**, exact at landing.
Kernel and aggregate ceilings are unchanged; no ledger or pin moves before
the landing re-measurement. This clears the domain-grant halt above.

### Operator sequencing and interim-validation ruling — 2026-10-08

Lunarpulse selected **Move 1.5 after Task 5**: complete the published
`enforced` disposition only after the real T3 proxy route is exercised.
Subtasks 1.1–1.4 retain their original order; Task 1 remains incomplete until
1.5 is verified. This avoids publishing containment before the control exists.

Lunarpulse selected **Allow interim advancement**: Tasks 1–8 may advance on
green focused behavioral tests while landing-dependent governance pins await
Task 9. Final full-regression and code-quality gates remain mandatory before
the story can enter `review`. Neither ruling relaxes any acceptance criterion
or increases a kernel, aggregate, or package ceiling.

### Operator Codex routing ruling — 2026-10-08

Lunarpulse selected **Use explicit CLI config override** after primary-source
research found that current Codex no longer reads `OPENAI_BASE_URL`. The
adapter supplies `-c openai_base_url="http://127.0.0.1:18080/api.openai.com:443/v1"`.
This is an explicit exception for the **non-secret base URL**, not credentials:
the scoped bearer stays only in `CODEX_API_KEY`; the raw key stays in the
daemon proxy; every podman `--env` argument still contains a name only.
The obsolete environment-only Codex route is removed, not retained as a shim.
Claude continues to use `ANTHROPIC_BASE_URL` and scoped `ANTHROPIC_API_KEY`.

Evidence: Codex `rust-v0.155.1` `core/src/config/mod.rs` consumes the
`openai_base_url` config field at lines 3744–3750
([source](https://github.com/openai/codex/blob/rust-v0.155.1/codex-rs/core/src/config/mod.rs#L3744-L3750));
the CLI documents `--config, -c key=value`
([reference](https://developers.openai.com/codex/cli/reference)).

### Task-4 credential-custody location request — 2026-10-08 (not ratified)

FW-01 omits three subprocess-owning T3 files:
`runtime_detect.rs` (runtime version probe), `image_verify.rs` (image
inspection), and `child.rs` (container stop/removal). Their commands inherit
the daemon environment. Clearing only the `run` command would leave these
children holding the vendor key, contrary to AC3 and the security mandate.

An actual invocation of `inspect_image_sha` against a controlled temporary
runtime parsed its registry digest and independently observed the public
`CODEX_API_KEY` sentinel in that child. No real key was used. The throwaway
source was removed; evidence is
`17-1-evidence/development-custody-charter-2026-10-08.json`.

Request: extend FW-01 to those three locations **only for using the common
scrubbed runtime-command path** (`env_clear`, then PATH/HOME/XDG_RUNTIME_DIR/
DBUS_SESSION_BUS_ADDRESS). Detection, image/container inspection, run, kill,
stop and removal use the same custody boundary. No unrelated changes and no
ceiling raise: kernel remains ≤ 25353 / ≤ 19564; aggregate raise remains
≤ 1182. The kernel behavior cutover has not started pending this ruling.

### Operator custody-location ruling — 2026-10-09

Lunarpulse selected **Extend custody locations**. FW-01 now also permits
`security/sandbox/t3/{runtime_detect,image_verify,child}.rs` **only to use the
shared scrubbed runtime-command path**. No unrelated behavior change is
authorized in these three files. Kernel source/KLOC ceilings remain
≤ 25353 / ≤ 19564 and aggregate raise remains ≤ 1182. The Task-4 location
prerequisite is cleared.

### Operator HALT-clearance and native-compile ruling — 2026-10-09

Lunarpulse selected **Resume targeted fixes** after the three-consecutive-
implementation-failure HALT. Correct the environment tuple iterator and
bounded reader reborrow, then rerun focused regressions and the actual kernel
smoke. Task 4 remains unchecked until every applicable gate passes.

Lunarpulse selected **Authorize native compile CI**: the throwaway-branch
authorization now also permits **one macOS compile job and one Windows
compile job** for the Task-4 platform gate. Reuse the existing all-workspace/
all-targets check commands with explicit runner labels; keep `contents: read`,
no repository secrets, purpose-named `ci/17-1-*` branches, evidence captures
and branch deletion. This exception does not alter the main-branch
`macos-check` single-job contract or authorize other native-runner jobs.
Kernel/domain/aggregate ceilings and final completion gates are unchanged.

### Operator native-portability HALT clearance — 2026-10-09

Lunarpulse selected **Resume targeted repairs** after three native CI
iterations failed the exhaustive Windows all-targets check. Repair the three
identified test locations at origin: `maos-audit`'s unit-test file symlink,
the spirit-publish key fixture's POSIX permissions, and the kernel resource
ceiling test's `unwrap_err` Debug requirement. Retain behavioral refusal
assertions and the exhaustive native check. No production-handle Debug
exposure, dependency addition, warning suppression or numerical-ceiling
raise is authorized. Task 4 remains unchecked pending verification.

### Operator Task-6 prerequisite sequencing ruling — 2026-10-09

Lunarpulse selected **Move the fixture prerequisite**: implement Task 7's
complete `worker-probe-fixture` and its four admission integrations before
Task 6's verification/prebuild gate. The rest of Task 7 remains after Task 6.
No acceptance criterion, test, CI proof, numerical threshold or final gate is
removed. Task 6 remains unchecked until its complete verification passes.

### Operator Task-6 HALT clearance and remaining native proof — 2026-10-09

Lunarpulse selected **Resume targeted correction**: distinguish explicitly local
non-CI checks from declared-capable CI, preserving hard refusal on CI without
the signed substrate. Replace the obsolete empty token-list expectation with
actual bound-token observation and revocation verification; retain crash,
orphan, disposition and cleanup assertions. Resume the authorized real T3
proofs. No test budget, acceptance criterion or numerical ceiling is lowered.

Lunarpulse selected **Extend native compile scope**: the existing throwaway-
branch permission also covers the remaining story snapshot with one
macOS-15 ARM64 compile job and one Windows-2025 X64 compile job, the same
exhaustive commands, `contents: read`, no repository secrets, captured
receipts and purpose-branch deletion. Main platform-job contracts are unchanged.

### Capable Task-6 verification HALT request — 2026-10-09 (not approved)

Three consecutive capable Linux iterations failed: `37909394818`,
`37910898713`, `37915868047`. Step-5 HALT applies before further implementation.
The latest snapshot `df5a5a7d958da989dca48bcaf2aa03752eef6b96` passes the actual
negative/positive static probe, live kernel T3 custody boundary, complete
Darwin workspace check and Windows build/complete workspace check. The moved
probe/admission prerequisite is runtime-proven; Task 6 and parent Task 7 are
not complete.

Both Ubuntu images execute the unchanged 99-test sweep: 77 pass, 22 fail
across seven targets. Actual errors include `image not known` in isolated-home
Root subprocesses and `PID not available after 10 attempts` in the direct
Host-B ordinary fixture. Downstream stall and named-container assertions time
out. [INFERENCE] isolated `HOME` selects a different rootless image store,
and the ordinary fixture exits before PID inspection observes it.

Requested guidance: **Resume targeted test-infrastructure repairs**. Preserve
application state isolation while retaining the provisioned OCI image store,
and make T3 fixture lifetime cover actual inspection without changing native
supervision budgets. No production retry/bare fallback, inherited secret,
declared-capable skip, numerical raise or reduced final gate is requested.
Durable evidence: `17-1-evidence/development-capable-halt-2026-10-09.json`,
the three full capable artifact archives and exact runner captures.
No implementation patch follows the third failed verdict.

### Operator capable-sweep HALT clearance — 2026-10-09

Lunarpulse replied **continue** to the preceding targeted-repair proposal.
Resume test-infrastructure image-store selection and fixture-lifetime repairs,
then the complete capable matrix and remaining story. Preserve real T3,
environment custody, hard refusal on declared-capable CI, native supervision
budgets, numerical ceilings and every final acceptance gate. The existing
bounded native compile-pair authorization remains unchanged.

### Capable-sweep HALT request after clearance — 2026-10-09 (not approved)

Iterations `37928459013`, `37932612178` and `37934183235` failed consecutively,
each closer: 22 failures, then 3 failures plus a hang, then 1 failure, then 1.
Two production defects were exposed and fixed with RED/GREEN regressions:
FR4 Spirit resolution displaced by the T3 report's pid-0 journal echo, and
Worker egress socket directories colliding across roots sharing one data home
or after a crash. The latest snapshot passes 98/99 on both Ubuntu images with
zero T3 skips; Darwin and Windows pass.

The remaining failure is the migrated cohort v5c assertion `exit == 1`. That
value was T0's grace-path exit while a descendant held the pipe. Under T3 the
named-container stop removes the descendant, and the daemon exits through
ordinary teardown; v5 exits 0 in 4/4 capable runs. Requested guidance:
**Resume the single v5c test correction** to v5's teardown contract, then
re-run the complete capable matrix. No production change, skip, retry or
threshold change is requested.

### Operator rulings — 2026-10-09

Lunarpulse **approved the one-line cohort v5c test correction**, clearing the
post-clearance capable-sweep HALT for that correction and the complete capable
re-run. Lunarpulse also **approved the code-size budget request for
ratification at landing**: Task 9 pins the exact `cargo fmt`-measured
per-crate rows and `_aggregate_hardfail` (latest measured aggregate 173146)
in the landing commit. The figure is ratified at landing, not pre-raised.
Tasks 7–11 add production lines only with a measured driver. The T3
exit-code 137 question is under analysis, not ruled.

Later the same day Lunarpulse **ruled A on the T3 exit code**: a SIGKILLed
Worker stays recorded as `exited(code=137)` / `fault.non-zero-exit`, and the
cookbook documents it. Authoritative OOM/signal facts are proposed for
`17-2-worker-cgroups-applied`. At Task 11 Lunarpulse **ratified** the ABI
entry `17-1-AC2-cli-wrapper-egress` and assigned the §A6 review to the
`review` phase. Landed aggregate 173289; `_aggregate_hardfail` 173290.
