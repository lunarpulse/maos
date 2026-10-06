---
title: '17-3c: isolated WASM bus-session and safe-runner candidate'
type: 'feature'
created: '2026-10-02'
status: 'done'
review_loop_iteration: 0
baseline_commit: '5075443c8a87523581cfcf6cb0b6af815f7ebd4f'
context:
  - '{project-root}/_bmad-output/implementation-artifacts/epic-17-context.md'
  - '{project-root}/_bmad-output/implementation-artifacts/17-3c-early-start-flag-winston-decision.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** WASM launch remains refused; the probe lacks production session/governance binding and safe, four-image execution.

**Approach:** Implement complete AC2/AC3 plus AC4 proof in a disposable snapshot; measure before requesting grants/cutover.

## Boundaries & Constraints

**Always:** Trusted scheduler/admission binding; enterprise/kernel mediation; log-before-deliver; supervision/pidfd and receipts; runner-only guest execution; bounded framing, decoding, diagnostics, lifting, and resources.

**Ask First:** Approval ratifies the sender-scope contract below for the candidate. Numerical grants, higher manifest resource ceilings, and original cutover need separate approval. If 64 MiB cannot fit, report the measured requirement.

**Never:** Reset/stash/commit main or change its kernel/pins/ledger. No enterprise bypass, daemon guest execution, bare fallback, recall/socket/fd/WIT changes, skipped acceptance, or 17-3b closure.

## I/O & Edge-Case Matrix

| State | Expected behavior | Failure handling |
|---|---|---|
| Valid session | Ready, repeated mailbox turns, bound output | No success before readiness |
| Scoped TaskAssign, clean/forged claims | Same trusted-policy verdict | Denied assignment not delivered |
| Bad guest/wire or timeout | Typed failure and receipt | No fallback/partial delivery |
| Idle/EOF/stop/crash | Idle not stalled; termination accounted | Planned kill not sandbox violation |

</frozen-after-approval>

## Code Map

- `crates/maos-bin/src/` — admission, supervisor, composition, governed mint.
- `crates/maos-wasm-host/src/` — contained runner, resources, CBOR (no daemon-side probe remains).
- `crates/maos-kernel-core/src/` — transport/T2 primitive.
- `crates/maos-iac/src/adapter/transparency_log.rs` — attributed/redacted insertion.

## Tasks & Acceptance

**Execution:**
- [x] New `crates/maos-frame-codec/{Cargo.toml,src/lib.rs}` — move framing into one std-only leaf, bounded before allocation; migrate consumers/Cargo edges without re-exports. CBOR stays outside kernel.
- [x] Kernel `lifecycle/cli_wrapper/runtime.rs`, `security/sandbox/linux.rs` — raw duplex/EOF, bounded LF stderr, surfaced errors, primitive SIGSYS audit; preserve allocation-free pre_exec. Additional image-specific syscall leave-one-out, including arm, is waived by the user's final acceptance amendment below; not claimed executed.
- [x] WASM `src/{adapter,conformance,config,runner,host_state,codec}.rs` — delete daemon probe/timeout API; bounded regular-file checks. Empty records mark readiness/turn completion; migrate all consumers. Diagnostics only on bounded stderr. Preserve exits 2/3/4/5 and startup/post-ready distinction.
- [x] `crates/maos-wasm-host/src/{runner,host_state,codec}.rs` — StoreLimits; disable shared/memory64; 16 MiB hostcall fuel; instruction fuel per export; reservations/compilation within admitted whole-runner caps. Measure memory/table/instance needs. Bound daemon decode expansion before collection allocation.
- [x] Bin `src/{admission,supervision}.rs` — prepare/adopt one SCB with full SandboxSpec; artifact read scope is sandbox-only. Supervise active turns, revoke owned tokens, emit every terminal receipt.
- [x] New bin `src/spirit_session.rs`, `src/{lib,main,worker_spawn}.rs` — mailbox pump, trusted sender/origin, all-or-nothing governed scope checks, kind/payload validation, typed denial, decision journal, authorized delivery. Standalone `--once` uses operator TaskAssign; unsupported topology/door/hot-swap paths typed-refuse. Retain default/no-network posture.
- [x] `crates/maos-iac/src/adapter.rs` — make trusted sender pid explicit in `deliver_typed`; migrate callers. Preserve lineage/replay/DRR/log-before-deliver; no pid-zero shadow row.
- [x] New bin `tests/spawned_scope_17_3c.rs`; existing WASM `tests/{e2e_roundtrip,t2_sandbox_kill}.rs` — real MCP admissions, authority/boundary/lifecycle regressions, amplifying/aliasing guest output, guarded debug ptrace. Migrate contracts; delete incidental wording/source-copy assertions.
- [x] `Cargo.toml`, `Cargo.lock`, `.github/workflows/discipline.yml`, architecture workspace-count sentinel, generated `STABILITY.md`, affected docs/templates — register leaf/member, migrate launch claims, preserve TS steps; collect authorized Ubuntu25.10 x86_64 proof/provenance/mutations and exact dependency/API/KLOC approvals. Broader image acceptance is waived, not reported passed. Original pins stay fixed.

**Acceptance Criteria:**
- Given real TS artifact/admission, when release `maos run … --once` runs, then scheduler-mailbox output is attributed/audit-queryable and termination produces a receipt.
- Given admitted native/WASM allow/deny MCP manifests, when clean/forged requests traverse the session, then eight actual kernel decisions through enterprise governance are nonvacuous and pairwise/cross-form equal; no manual pid/FsRead seeding.
- Given compiled kernel deny bypass, when the gate runs, then deny assertions RED; restoration GREEN.
- Given amplifying/aliasing output below wire cap, when lifting exhausts hostcall fuel, then allocation fails before serialization; removing the setter breaks the regression.
- Given the completed Ubuntu 25.10 x86_64 evidence, when the actual governed TS CLI runs under CPU10%/768 MiB/360 s, then Ready, mailbox delivery, attributed audit, clean receipt and owned cgroup cleanup pass. The user explicitly accepts this completed platform testing as sufficient. Ubuntu24.04/26.04, aarch64 and additional image-specific syscall controls are no longer story acceptance gates; unexecuted controls are not claimed passed. Existing actual SIGSYS/escape-consumer regression evidence remains unchanged.
- Given refusal/launch revisions, when compared, then manifests match. Approved resource changes require a fresh matched pair; historical evidence is not relabeled.

## Spec Change Log

## Design Notes

Proposed contract: TaskAssign senders must be entitled to every delegated scope; other emitted kinds mint nothing. Bind sender/origin from endpoint; retain intent/consent as untrusted claims. Use permitting enterprise policy for kernel comparisons; separately prove configured enterprise denial. Preserve redaction.

Wasmtime 46 charges list/string lifting before host allocation; StoreLimits alone is insufficient. The 64 MiB RLIMIT_AS probe failed during Rayon initialization, exit 3; uncapped success is not resource proof.

## Verification

Release-build both binaries with `maos-bin/wasm-host`; exercise CLI/audit in isolated HOME. Preserve completed session, release resource/roundtrip, debug fault-runner and compiled-mutation evidence. Use the user-accepted Ubuntu25.10 x86_64 proof; no further image execution is required for story acceptance. Preserve default/no-network builds and workspace/stability/dependency/API/template governance evidence. Record approved candidate deltas and original baseline/content hashes/index without manufacturing green.

### Historical detached execution evidence and approvals (superseded by final acceptance and review handoff)

> `artifact://NNN` references in this spec are non-durable session handles, not retained evidence. They are superseded by the files under `17-3c-evidence/review-2026-10-06/`.

- Historical execution-stage status was in progress. Current candidate status is `review` after the final user amendment below; no original cutover, publication, or story closure.
- Actual six-process-gate suite: `cargo test --offline -p maos-bin --features wasm-host --test spawned_scope_17_3c -- --nocapture`, **6 passed** (`artifact://711`). Eight native/WASM grant × clean/forged cases use a real configured permitting Cedar adapter and shared scheduler-installed kernel policy. Four further native/WASM cases prove real Cedar forbid and atomic mixed-scope refusal, with no guest effect.
- The mixed-scope run exposed a real panic for ABI `SkillAuthorSelf` (`artifact://705`). Intent projection now returns existing typed `PolicyDenied` for an unmediated action before token construction, rather than panicking or coercing another intent. Real native/WASM paths pass (`artifact://707`, `artifact://711`).
- Current kernel policy compares scope variants, not MCP server/tool selectors. This gate proves current issuance policy, **not** fine-grained selector entitlement. That existing policy is intentionally unchanged.
- Fresh compiled kernel-deny bypass is **RED** (`artifact://709`, delivered 1 where 0 required); restored source and rebuilt test are **GREEN** (`artifact://711`). Earlier hostile-lifting setter RED/GREEN remains separately recorded, not relabeled.
- SIGSYS control reaches actual Ready, dies by SIGSYS 31, persists primitive SandboxBlock, and produces one anomaly through the production escape consumer. Planned stop produces neither. Repeated mailbox export survives 32 seconds of idle with no active ledger entry or stall. The new first assignment after idle resets the watchdog baseline; an already-active assignment does not.
- Explicit operator-approved candidate runtime grant: **768 MiB process / 180 s time and compilation guard**, retaining **64 MiB guest / 10M instructions / CPU 10%**. TS launch acceptance remains blocked, not weakened to undeclared CPU.
- Explicit operator-approved exact formatted candidate numerical grants: physical kernel **25,032 → 25,149 (+117)**, 98 files, set hash `76dec9f7563007168134157c728ea2611855beea144012ebca9857c9e5014eeb`; production KLOC kernel **19,309 → 19,412**, bin **23,472 → 24,213**, IAC **7,084 → 7,088**, new std-only frame codec **218**. Zero headroom. Aggregate **169,654**, unchanged hard-fail **170,884**; existing alarm **158,608** still fires honestly.
- Separate explicit candidate surface approval: classify `BridgePoll` and `FrameWriter` as data-movement; record private `SpawnedBridge` ownership-field rehash and additive `SpawnError::CgroupUnavailable`. No symbols removed; exhaustive enum consumers must migrate. Exact hashes in `17-3c-evidence/kernel-surface-grant.json`. No compatibility aliases.
- Approved pin/KLOC, service-boundary, empty-kernel/I9, workspace **57/57**, stability, dependency closure and template drift gates pass. Kernel unit suite **262 passed** (`artifact://721`). Current feature-on release CLI builds (`artifact://722`); existing warnings remain visible, not suppressed.
- Four-image CI proof vehicle retains Ubuntu 24.04/26.04 × x64/arm64 and TS artifact production. A dedicated `Delegate=yes` systemd service provisions only its own CPU/memory subtree; no ancestor mutation or fail-open production fallback. Live CI execution, syscall leave-one-out across images, and matched accepted TS launch remain unproven.
- Current local approved TS manifest fails closed before Ready: CPU 10% requires a writable delegated cgroup. Local cgroup root is read-only; no systemd user bus. Authorized remote `cosmo@192.168.100.135` rejects default SSH identities and `rsa_nopwd`; an authorized identity/SSH agent is required. Remote permission is execution only, not publication.
- Original preservation checked: HEAD `5075443c8a87523581cfcf6cb0b6af815f7ebd4f`, staged tree `4fd937d15144c0a01a934d072b90a1d27215cbe2`, kernel **25,032 / 98 files / bdcb3b2ef64bc63cd68ec1f32415c197d2fc02601422933f2b084885ce773776**, all unchanged. Frozen approved lines 13–38 are unchanged.
- Fresh current-source release refusals now match exactly: manifest SHA-256 `2ee93955e0e9c69cb64e75f2503cbe105ae597a8dbdfcd1c3db846bcb1473bd7`, TS component `ca8909781b945cf3714e6bf5a2b364be9ecf8f0a565bf95a2fe6f7bf51a4be4d`, approved 768 MiB/180 s/CPU 10% limits. Feature-off returns `wasm_engine_off`; feature-on fails closed for cgroup delegation, both before Ready. This is **two refusal records, not an accepted launch pair**. Exact executable hashes/stderr are retained in `17-3c-evidence/local-current-cli-refusals.json`; approved grants and preservation in `candidate-grants-and-verification.json`. Feature-on release build restored as the current CLI (`artifact://733`).

### Authorized 100.20 remote execution and superseding time grant

- The user supplied `cosmo@192.168.100.20` with a dedicated SSH identity. Actual noninteractive SSH succeeds. Ubuntu 25.10 x86_64, kernel 6.17.0-41-generic; user systemd delegates cpu/memory/pids and its transient-service subtree is writable. No sudo or ancestor controller mutation was needed. This clears the former SSH/delegation prerequisite for this endpoint, not the four-image requirement.
- Two real CPU 10% / 768 MiB / 180 s runs failed before Ready at the startup deadline. `/proc` plus actual child cgroup files confirm `cpu.max=10000 100000`, `memory.max=805306368`, and fuel 10M. `OptLevel::None` did not resolve the failure and was reverted; production remains the original Cranelift configuration.
- A separately authorized, bounded **measurement-only** CPU 100% / 768 MiB / 180 s run completes actual Ready, mailbox turn, trusted attributed authorization/audit, clean terminal receipt, and owned cgroup cleanup (`TS-MEASUREMENT-ONLY`). Initial measured child CPU usage was 30.58 seconds, total wall 30.70 seconds; this is not CPU10 acceptance.
- The real successful measurement exposed a proof-vehicle bug: daemon `spirit_loaded` events are printed to stdout, not stderr. `scripts/prove_17_3c.py` now reads that actual stream; the corrected prover completes the real measurement's audit/resource assertions.
- The user explicitly selected **CPU10 retained / time 360 s**, superseding only the previous 180-second candidate time and compilation guard. Memory768 MiB, guest64 MiB and fuel10M remain unchanged. Both first-party TS manifests, both READMEs, the runner guard and proof-vehicle contract are migrated. No original pin, source, publication or cutover changes. Exact grant and measurement evidence: `17-3c-evidence/remote-100-20-probes-and-grant.json`.
- Final six real-process gates pass at the approved guard (`artifact://771`); format, template drift, unchanged physical kernel pin and exact KLOC gates pass. Aggregate remains 169,654 with the existing alarm still visible.
- A fresh release engine-off CLI refuses the new matching 360-second manifest with typed `wasm_engine_off`. Manifest SHA-256 `ad5b19e71b9acebff3f1405e4075d243fdfc2b66f1236bce48b48fb1fcbb8f89`; new runner `4dc938fa81bab7ca15a37f78040d3e180f8da02569e7c7dc31b561c370dcdc7c`. Actual CPU10 success evidence is being collected separately, not relabeled from CPU100 or the earlier 180-second records.
- Four required Ubuntu 24.04/26.04 x64/arm64 workers remain unavailable. This host is supplemental Ubuntu25.10 x64; no QEMU/container/VM CLI is installed, free home-filesystem space is 6.3 GiB, and sudo requires interactive authentication. No host packages, VMs, OS upgrades or publication were performed.
- **Actual CPU10 acceptance now passes** on supplemental Ubuntu25.10 x86_64: `TS-ADMITTED-PROOF`, actual Ready, PID1/child5165, one mailbox turn and delivery, trusted Standard/SpiritAuto authorization, persistent attributed audit, exit code0 non-crash receipt, unload/revocation, and owned cgroup removal. CLI elapsed **322.3445 s** under the newly approved 360-second grant; observed child VM peak **590,816 KiB**, RSS peak **505,852 KiB**. Full JSON and actual SQLite archive: `remote-100-20-accepted.json`, `remote-100-20-accepted-runtime.tar.gz`.
- **Fresh actual refusal/launch revision pair is complete**, not just a feature-off control: preserved original source with `wasm-host` ON refuses typed `wasm_launch_not_built`; candidate actually launches the identical approved manifest/component. Original refusal CLI SHA `89664a72d142cecfe2a19558a6d34fcee035a0d8aaea2fd474a909db81754481`; candidate SHA `782d08fdb071058f9eef08265962eafca957207f95d146f3b5318c37900b1c82`. Exact records: `preserved-refusal-revision-360.json`, `matched-refusal-launch-360.json`.
- TS static resource inspection of the exact deployed component: **4 allocated core-module instantiation sites**, **1 defined memory initially 145×64 KiB**, **2 fixed tables of 7,688 and 25 entries**. Adapter/fixup import existing identities. Synthetic host export bundles are not allocated module instances. Unchanged StoreLimits are instances16/memories4/tables16/elements65,536/64 MiB each; the actual startup/export passes within them. Static capacity is not a runtime guest-memory high-water counter. Details: `ts-resource-capacity.json`.
- Original refusal revision was built read-only with `--locked --offline` and an isolated target directory. A first shared-target attempt mixed incompatible cached candidate metadata; the isolated original-source build succeeds without any source repair. Use isolated artifact targets when comparing worktree revisions.
- Final candidate binaries were independently built in isolated target `/mnt/build/maos-17-3c-final.oitpeZ` (`artifact://791`) and **actually re-exercised** remotely at the unchanged approved CPU10/768 MiB/360 s contract. **PASS:** PID1/child5709, one turn/delivery, attributed audit, clean receipt, owned cgroup cleanup; CLI elapsed **323.9006 s**, observed VM peak **605,380 KiB**, RSS **509,420 KiB**. Final CLI SHA `71297d7d08eee1ef42a276b2be6193fda0c63e8adc3a8e5f7ffd2f6d7d1ddd88`, runner SHA `521869ef96d1bf983f50cdf3c1d383a0db0a5983db6666ec30bd200647d25629`. Final proof and actual SQLite archive: `remote-100-20-final-isolated.json`, `remote-100-20-final-isolated-runtime.tar.gz`; fresh matched-revision record includes this final build.
- Host surface gate passes with **0 additions / 2 intentional removal entries**: `SpiritHostError::Timeout` and its `timeout_ms` field. The existing host gate's closed-allowlist policy allows removals; no host baseline was silently re-pinned. Template drift passes after the actual acceptance documentation update.
- Original HEAD, staged tree and all 98 kernel-file digests were rechecked after the read-only refusal build and remain unchanged. Local measurement scaffolds were removed; measurement-only evidence was archived and the owned remote measurement workspace removed. No packages, VM provisioning, publication, original cutover or story closure.
- Historical acceptance snapshot before the user's final correction: actual governed TS CLI and matched provenance were complete, with four-image/syscall repetitions still blocked. This snapshot is superseded by the final Ubuntu25.10 x86_64 acceptance amendment below; it is not a current blocker or current story status.

### User-approved platform acceptance amendment — SUPERSEDED (2026-10-05)

> Superseded on 2026-10-05 by the final Ubuntu 25.10 x86_64 amendment below; retained as history only. The Ubuntu 24.04-only requirement is not current.

- User explicitly accepts **Ubuntu 24.04 x86_64 only** for story platform acceptance; aarch64 instances are unavailable. The prior Ubuntu24.04/26.04 × x64/arm64 acceptance requirement is superseded, not reported as passed or skipped.
- Preserve actual launch, approved resource enforcement, necessary-syscall omission and SIGSYS/audit/escape-consumer controls on Ubuntu24.04 x86_64. Existing Ubuntu25.10 x86_64 success is retained as valid supplemental evidence, not relabeled Ubuntu24.04.
- Remaining prerequisite is an authorized Ubuntu24.04 x86_64 worker. The currently supplied `192.168.100.20` was observed as Ubuntu25.10; no Ubuntu24.04 endpoint or actual result has been supplied.
- No ARM/Ubuntu26 portability claim, original cutover, publication or story closure is authorized by this scope amendment. The broader repository CI matrix may remain; it is not proof of this story's unexecuted platforms.

### Final user correction: completed Ubuntu25.10 x86_64 testing is sufficient

- User clarified: “I mean ubuntu 25.10 x86_64 is sufficient testing that. record and move on.” This supersedes the immediately preceding Ubuntu24.04 interpretation.
- The completed isolated-binary Ubuntu25.10 x86_64 proof is accepted as sufficient platform testing: CPU10%, process768 MiB, time/compile360 s, guest64 MiB, fuel10M; PID1/child5709; one mailbox turn/delivery; attributed audit; clean receipt and owned cgroup cleanup. CLI elapsed 323.9006 s. Evidence: `remote-100-20-final-isolated.json` and its actual SQLite archive.
- No Ubuntu24 worker, Ubuntu26 run or ARM instance is required. Additional platform-specific syscall omission/SIGSYS repetitions are excluded from the remaining acceptance gates by this approval, not represented as executed. Existing real SIGSYS/escape-consumer and mutation regressions remain evidence for the exercised environment only.
- Candidate implementation and acceptance phase is complete under the amended scope; proceed to candidate review/integration preparation, without reopening platform testing. The wider repository CI matrix remains separate.
- Original story closure, original checkout cutover, push/publication and deployment are not performed by this record. Original source/index/pins remain unchanged.

### Current review handoff (2026-10-05)

- Candidate and original sprint trackers route 17-3b and 17-3c to `review`. This specification frontmatter is `review`: implementation/amended acceptance complete, independent review pending; no story marked `done`.
- Review target is `/tmp/maos-17-3c-candidate-wecx8ef7`, including its source and `17-3c-evidence/`. Original checkout retains the refusal source with updated tracking documents only; it is not the candidate implementation.
- 17-3b's final unchecked finding now points to the real admitted-session gate and compiled mutation evidence here. It remains unchecked until independent verification; prior 17-3b review/CI evidence is unchanged.
- Current summary is `17-3c-evidence/candidate-grants-and-verification.json`; actual isolated binary/audit provenance is `remote-100-20-final-isolated.json` and its SQLite archive, cross-referenced by `matched-refusal-launch-360.json`. No extra platform run, original cutover, commit or push is authorized by this reconciliation.

### Review closure and integration (2026-10-06)

- The §A6 bmad-code-review is complete; every finding was patched, decided or deferred with an owner (story §Review Findings). The frozen intent block is unchanged.
- Fresh evidence is under `17-3c-evidence/review-2026-10-06/`:
  - the AC1 remote proof on Ubuntu25.10 x86_64;
  - `spawned_scope_17_3c` 15/15;
  - lineage, partial-turn, `iac.send` and kernel deny-bypass mutation pairs, each RED then GREEN.
  This supersedes the `artifact://NNN` handles above.
- The candidate was integrated into the original checkout and verified byte-identical for all non-tracking paths. 17-3b and 17-3c closed `done` in two local commits; nothing was pushed or published.
