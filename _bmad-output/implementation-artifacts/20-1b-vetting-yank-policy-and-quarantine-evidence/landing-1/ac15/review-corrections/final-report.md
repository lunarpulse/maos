# 20-1b kernel-free L1 — AC15 review-correction handoff

Dev201b / openai-codex/gpt-6.1-sol; correction baseline `9e8c7737b0aaae730024ae66966c20106563141e`, observed unchanged in the required final HEAD log. Earlier ebf2ab03/L1/c656a6e7/f9921251 measurements and before runs remain historical, not relabelled clean-baseline observations of this pass.

## Delivered files and behavior

Exact **36 source/config/record paths and hashes**: [source-inventory.json](source-inventory.json). The current run-evidence directory is additional. Main-owned cleanup reports are untouched and unattributed.

- Registry client, HTTP, security, install, server, storage, yank poller and query handler: strict shared operator config; authenticated complete heads/typed rollback without cursor reset; bounded accepted sockets, admission, first-byte handling, request/response growth and total client deadline; usable refusal status; sanitized hex diagnostics; private-CA replacement semantics.
- Bin composition and root shutdown: start after shared boot admission, join before stop/unload; ownership-aware seven-release/eleven-drain AST coverage and an actual one-child negative control.
- CLI/FKCS/shell/bin manifests and lock: remove registry TLS default-feature leakage and explicitly forward TLS. **The whole no-default graph is not TLS-free:** frozen kernel/A2A rustls remains under Main's permitted honest-record fallback.
- New/expanded registry TLS/limits/query, bin config/AST and xtask socket/workflow tests; shared harness upload hint; obsolete empty ignored server tests removed in favor of real wire coverage.
- Actual discipline and old20-1a invocations, current branch-only20-1b recipe, executable TypeScript README, SECURITY.md, coupled KLOC pins, story/tracker/deferred/preflight records.

P1–P12 and scoped security corrections are implemented. The discovered development-admission framing regression has a real before/plant and a bounded100ms dev-only refusal/drain fix, not plaintext on TLS or an expanded cap. Generic-storage callers retain documented custody responsibility; private seed bytes alone carry no file identity. W1–W7 and remaining F6/F8/F10/F11 limits have explicit owned dispositions, not falsely completed implementation claims.

## Acceptance witness table

The full before/plant table, current path/name:line coordinates and qualifications are in [witnesses.md](witnesses.md). All installed plants were restored by copy-back + cmp + touch, each exit0. Historical before runs were partially edited-tree observations, not invented clean-HEAD reds. Some coverage additions already passed before; only their genuine disabled-predicate plants are credited as reds.

| AC | Named witness:line | Before / plant | Final scoped result |
|---|---|---|---|
| AC6 | `crates/maos-registry/tests/vetting_attestation_gate.rs::artifact_swap_is_refused_ac6:411`, `publisher_swap_is_refused_ac6:415`; SpiritCLI lifecycle:8 | Historical subject-binding reds/restored plants retained | Pass: exact artifact/publisher and real issue/install/promote/revoke lifecycle |
| AC7 | `crates/maos-registry/tests/yank_auth_20_1b.rs::publisher_and_enrolled_vetter_yanks_are_signed_labelled_and_restart_safe_ac7:98`, cursor restart:190 | Historical auth/cursor plants; current signed-view/rollback plants | Pass: signed, origin-labelled propagation and restart cursor |
| AC8 | `crates/maos-bin/tests/yank_policy_20_1b.rs::signed_yank_decisions_warn_request_quarantine_and_apply_crl_ac8:207` | Historical policy/typed-boundary plants retained | Pass: signed decision, typed boundary outcome and visible quarantine request. **Quarantine EFFECT not claimed in L1** |
| AC9 | Same file `expiry_and_vetter_revocation_observe_once_and_request_policy_ac9:296`, local timer:766 | Historical expiry/local-observer plants retained | Pass: cause-labelled observations/policy requests, including no registry source |
| AC10 | Same file `registry_publishers_cannot_share_principal_spill_ac10:378`; registry publisher pin:77 | Historical principal/publisher-binding plants retained | Pass: publisher isolation, durable pinning |
| AC14 / F4 | `crates/maos-cli/tests/spirit_install_20_1.rs::installed_version_is_immutable_ac14:307`; registry durable pins:441 | Historical replacement/origin/durability plants retained | Pass: same-key replacement and different-registry overwrite refused |
| AC15 / F5 | `crates/maos-registry/tests/registry_tls_20_1b.rs::tls::cli_tls_install_authenticates_before_persistence_ac15:468`; wrong configured key:499 / URI:517; wrong-host:368 / expired:388 / untrusted-CA:408 | Prior root/TLS plants plus current signed-valid view, config, custody, status/deadline plants | All40 nonignored TLS/dev cases pass. Interception vectors exercise actual CLI and absent persistence root |
| AC16 / F6 | CLI `yanked_install_is_refused_before_persistence_ac16:404`; TLS `cli_tls_yanked_version_is_refused_before_persistence_ac15:1027` | Historical pre-persistence/yank plants retained | Pass: yanked version refuses before persistence, including authenticated transport |
| AC17 | Registry `unsigned_and_unauthorized_mutations_leave_storage_unchanged_ac17:12`, immutable/idempotent publish:407 | Historical unauthenticated/unauthorized mutation plants retained | Pass: publish/deprecate authentication and authorization; rejected mutation leaves storage unchanged |
| AC11–AC13 / AC8 effect | Final KQ1 + P landing | No kernel mutation/effect witness here | **Not claimed in L1**. No KQ0/B substitute |

The independent-review table also records actual reds/plants for accepted blocking mode/clone failure/backoff; per-source/idle/body caps; incomplete TLS error-write timeout; a real drip-response deadline; strict daemon config/banner; typed head-behind-cursor; signed-valid gap/duplicate/extra/empty/mismatched-head/request/version/signature; shutdown ordering and AST escapes; params-less notification/non-2xx status; sanitized hex; mode0440/store placement; CA replacement; and complete bounded query snapshots.

## Ruled F5 and review decisions

Original F5 D-20-1b-01/02/03 are **RULED A/A/A** (`f5rows`, 2026-11-07): explicit URI-bound configured Ed25519 root, leaf/server rustls with operator-provisioned key/cert paths, and explicit bilateral literal-loopback development opt-in. No response-supplied root or HTTPS-to-HTTP fallback. F5 is no longer blocked on these choices.

| Review row | Ruling carried into implementation |
|---|---|
| D-20-1b-09 / D1 | One strict fail-closed CLI/daemon reader and truthful banner, including no-URI unknown keys |
| D-20-1b-10 / D2 | Isolate registry TLS defaults; Main permits honest residual frozen kernel/A2A TLS limitation, not a kernel edit |
| D-20-1b-11 / D3 | Typed `HeadBehindCursor`, visible unchanged cursor/no automatic0; domain ceiling10155 =10137+18, inside existing aggregate envelope |
| D-20-1b-12 / D4 | Configured CA replaces bundled roots; absent CA uses bundled Mozilla, **not platform roots** |
| D-20-1b-13 / D5 | Exact CLI6429; explicitly forfeit427 of former6856 reserve, not source shrinkage or transferable headroom |

## Exact measurements after fmt

| Tokei pin | Baseline9e8c7737 | Delta | Final | Ceiling |
|---|---:|---:|---:|---:|
| maos-registry | 5668 | +483 | 6151 | 7081 |
| maos-bin | 35589 | -3 | 35586 | 35671 |
| maos-domain | 10137 | +7 | 10144 | 10155 |
| maos-cli | 6429 | 0 | 6429 | 6549; legacy427 forfeited |
| maos-spirit-cli | 942 | 0 | 942 | 984 |
| Aggregate | 196964 | +487 | 197451 | 198621 |

Strict aggregate hardfail **197452**, coupled live story-arrow histories updated together; exact KLOC gate0. Kernel unchanged **26841 physical /21042 tokei**,95 files/set hash `db39bc4a1ab6a6c22575990ae934e698a519d3efd46a8dbb38e62bc376697b59`. Audit8179 remains below correction8204; separate20-11 ceiling8261 has82 remaining, not old87. Xtask44696 is unchanged here; its earlier+20 is external20-2 hosted-3 `d84b83dd`.

## Observed verification, including reds

[commands.json](commands.json) contains exact argv, exit codes and elapsed times; per-command stdout/stderr are retained. Environment/metadata provenance and same-run JUnit are adjacent. No `CARGO_TARGET_DIR` or agent `--target-dir` override; fixture-builder internal isolated-fixture flags were pre-existing.

| Check | Observed result |
|---|---|
| Focused nextest with metadata reuse | **0**,827/827, UUID`a796a2f7-be25-4368-a940-7c4105bd0c3e`,101.271s wall /101.261s testcase sum |
| `cargo fmt --all` | **0**, before final measurement/default build; subsequent source unchanged |
| Genuine affected/default binaries | **0**, bin/CLI/registry/SpiritCLI/xtask, after fixture builder and metadata generation |
| Genuine bin no-default build / verb_table_15_3 | **0** / **0**,7/7 verbs |
| `cargo deny check` | **0**, existing allowance/skip warnings retained |
| Final affected all-target clippy | **0**;0 diagnostics on changed/new primary spans and0 new message fingerprints against initial correction-state clippy. No fabricated clean-baseline compilation |
| Final structural gates / exact KLOC | Twelve structural gates **0**, plus exact KLOC **0** |
| Separate `check-fkcs --json` | Agent command deadline at900s: **no exit code/report**, empty capture. Not credited pass; not retried. Subsequent real process scan found no remaining visible compiler/xtask |
| One standalone workspace | **100**,5833/5836; two missing-cryptography preparation failures plus existing1M pending43.366s. UUID`f6748ad2-1296-4b1c-a5ee-e387563850c1`; sum1144.817<1440. Six binary-sum advisories, no widening |
| One prescribed prepush | **1**,3518.903s in bounded wait cells;92 records =83 pass/7 explicit skip/1 workspace red/1 timing advisory. Tracked-tree **pass** |
| Fresh prepush workspace |5835/5836, sole1M pending45.523s; UUID`0e4b479b-e575-4500-8c98-3865f2ac3d5f`; sum1136.340<1440. Both verifier cases pass with hash-locked environment; standalone reds remain recorded |
| Copied prepush JUnit budget analysis | **1**,four30s binary sums: erasure-uninstall32.335, hook-pump31.944, one-door36.041, participant-recall41.741. Analysis only, no new tests |
| Hosted selector resolution | **0**,156 runnable/54 required markers, no missing names; one ignored public-CA case separately enrolled. Metadata selection, not hosted execution |
| Cleanup script | **0**, guarded proc scan and authorized deletions only |

Prepush scoped sums: root-shutdown15/1.504s; installed-dispatch2/4.440s; TLS40/5.832s; limits4/1.276s; cadence30.141s within its unchanged39s exception. There is no root-shutdown exception. Locally returning T3 prerequisite guards are not live-Worker proof.

The standalone verifier omission is the agent's environment preparation error, not a product/timing flake. Pinned `tools/verify-erasure/requirements.txt` wheels were provisioned in own venv with system-site-packages and a PATH prefix preserving system tools for the separately prescribed prepush. No standalone or prepush retry. Same-line rule10 rows under20-7 record all actual timing occurrences; functional1M owner remains20-11. The1440 total stays Main's hosted-authoritative ruling, with no per-binary row changes.

## Limitations and re-review focus

- Native Apple accepted-socket runtime, public-WebPKI private-CA exclusion, actual signed T3 live Worker, delegated cgroup/netns/interception substrate and actual hosted epic20-exit after-state are **authored, unverified**, not local/hosted passes. Operator-reported hosted42464850336 is accepted, not rerun to confirm failure. Branch recipe remains under story evidence, not installed as a verification workflow on main.
- W1 status/session coverage, W2 observer/join absolute bound, W3 log-rate, W5 key-role reuse, W6 O(n) journal/pagination/certificate vectors and W7 operator-config custody remain explicit scoped limits. No automatic cursor generation recovery or TLS1.3-only promise.
- Full current check-fkcs gate has no completed result; focused FKCS/admission controls passed, but do not substitute for that missing complete gate.
- Explicit re-review focus: inherited authorized `verify_attestation` input hunk and two claim fields at the136c579d… admission pin. Its re-pin preceded the review; these declared files/pin are unchanged by this correction.
- AC8 effect/AC11–AC13 await final KQ1+P; kernel source/manifest untouched.

## Cleanup and handoff

Removed **1529** old inactive incremental session directories; retained **11105**, newest per crate-cache/feature, all descendant mtimes within2h and any open reference. Own `/tmp/201b-ac15-corrections` and verifier venv removed. No deps/build/fingerprint, lockfile or Main cleanup report removed. Visible compiler list empty; PID1/7 proc custody permissions denied and recorded.

Observed build-filesystem available delta **+13,830,430,720 bytes**. Fully-unlinked inode allocation accounting **15,100,903,424 bytes**; unique removed inode logical size **17,967,899,575 bytes**. Scratch-filesystem delta **-573,440 bytes** is not claimed freed. Extent/reflink/shared-activity attribution limits and full plan/proc scan/script are retained in cleanup evidence.

No commit/push/status flip. Main's tracked-tree/HEAD freeze remains held until this handoff's final message explicitly releases it; no further cargo is planned by Dev201b.
