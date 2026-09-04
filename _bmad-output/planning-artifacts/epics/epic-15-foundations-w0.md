# Epic 15 — Foundations (W0)

**Status:** `backlog` — Round 2 correct-course 2026-09-04 (`sprint-change-proposal-2026-09-04-round2.md`, Lunarpulse-ratified; Fork C). Replaces the morning's Epic 15 file. `sprint-status.yaml` is authoritative for status.

**Wave / duration:** 2–3 weeks (measured) · **Makes true:** the five blockers the preflight found, so Epics 16–21 can land a line and prove an exit

**Closes:** 4 red gates + 1 test red; 8 `CURRENT_PHASE` constants; zero kloc headroom in 8 crates; `release.yml` first-run defect; the four open design forks; no hermetic inference seam outside Researcher

**Hermetic exit command (CI, no secrets; the epic's first CI job, red until green):**

```
cargo run -p xtask -- check-empty-kernel && cargo run -p xtask -- check-service-boundary && cargo run -p xtask -- kloc-check && cargo run -p xtask -- check-env-contract && cargo test --workspace --no-fail-fast   # green on tree content
cargo run -p xtask -- check-exit-commands                                    # every epic exit verb resolves against --help
git tag v0.1.0-alpha.1 && git push --tags                                     # release.yml builds signed maos + maosctl for 3 targets
```

**Operator-lane legs (never in the exit):**
- `ops-provisioning-secrets-and-accounts` — `RELEASE_SIGNING_KEY`, `MAOS_RELEASE_PUBKEY`, tap repo, AUR account, ledger-branch permissions (checklist in 15-5)
- hotfix lane rows (model-pin gate/sweep, model env overrides, error surfacing) run in parallel, keys unchanged

**Kernel-Δ:** ZERO kernel-Δ @24472 except 15-1 AC2 (≤2 attribute lines, FLAG-Winston re-pin in the same commit) if the ADR-037 whitelist path is not chosen.

**Confidence rules (ratified 2026-09-04 R2, binding):** 1 verb-first (exit verbs exist at HEAD or are AC1 in this epic; `check-exit-commands`) · 2 hermetic exit, live optional (paid/human/clock legs are operator-lane rows) · 3 foundations first · 4 cite-or-cut (every AC names file:line + kernel-Δ) · 5 spike before build · 6 size from measurement (+30%) · 7 one decision per fork · 8 inventory from the tree.

**Dev-gate:** external holds (NFR-Sec-7, NFR-Comp-1) = GA ledger only. **Model/review:** frontier allowlist + §A6 net binding; non-degradable on stories marked ◆.

---

## Stories

| Key | Story | Closes · Δ |
|---|---|---|
| `15-1-green-at-head` | Green at HEAD | S4 (part), D17 · kernel-Δ ≤2 lines or 0 |
| `15-2-kloc-ceiling-rebase` | One-time kloc ceiling re-base ◆ | §2.2 of the preflight · kernel-Δ 0 |
| `15-3-single-phase-source-and-exit-command-check` | Single phase source and the exit-command check | S4 · rule 1 mechanical · kernel-Δ 0 |
| `15-4-release-repair-and-first-signed-tag` | Release repair and first signed tag | S5 · kernel-Δ 0 |
| `15-5-decision-adrs-and-provisioning-checklist` | Decision ADRs and the provisioning checklist ◆ | rule 7 · kernel-Δ 0 |
| `15-6-inference-replay-seam` | Inference record/replay seam for every Spirit | preflight §2.4 · kernel-Δ 0 |

AC1 is always the command and what the operator observes; every AC cites the code it changes; `bmad-create-story` may tighten but not add uncited ACs.

### 15-1-green-at-head — Green at HEAD

*Closes · Δ:* S4 (part), D17 · kernel-Δ ≤2 lines or 0

- **AC1 (command).** The first exit line above exits 0 on tree content and the discipline `aggregate` is green with no gate demoted to advisory.
- **AC2.** I9: `T3ImageVerificationConfig` (`security/mod.rs:124`) and `ValidatedRevocationRules` (`revocation/rules.rs:19`) documented in `docs/invariants/i9-exemptions.md`; the `#[i9_exempt]` at `security/mod.rs:120` moved onto `SecurityManagerAdapter` (`:130`, net-0); `ScbRuntimeSnapshot` (`control_block.rs:244`) and `VerifiedImageLock` (`t3/image_lock.rs:27`) either attributed (+2 kernel lines, re-pin 24472→24474 in the same commit) or added to `xtask/i9-whitelist.toml` with an ADR-037 rationale — the choice is written in the story.
- **AC3.** NFR-Test-2: the **16** `other` symbols get exact-path rows in `xtask/kernel-api-classes.toml` (`check_service_boundary.rs:236-241`) and `docs/ci-baselines/kernel-surface-v0.1-beta.json` is regenerated for the 19 signature-changed rows, each with a one-line rationale.
- **AC4.** kloc: `maos-domain` +51 and the two env lines in `maos-bin` are covered by 15-2 (this story lands after it); D17 closes when `_aggregate_hardfail` is re-derived.
- **AC5.** `MAOS_OPERATOR_BEARER_TOKEN` and `MAOS_OPERATOR_HTTP_BIND` registered in `crates/maos-bin/src/env_contract.rs:13` format.
- **AC6.** The `t_14_2a_post_grace_journal.rs:362` red is reproduced under default threads, its cause recorded (the test already uses `with_default` at `:267` on a `current_thread` runtime) and fixed; `check-cert-rotation-trigger` drops `--test-threads=1` (`check_cert_rotation_trigger.rs:613`) so gate-green equals suite-green.

### 15-2-kloc-ceiling-rebase — One-time kloc ceiling re-base ◆

*Closes · Δ:* §2.2 of the preflight · kernel-Δ 0

- **AC1 (command).** `cargo run -p xtask -- kloc-check` is green and every crate named in Epics 16–21 shows the headroom its measured stories need (table in the story: maos-bin +1200, maos-control +700, maos-cli +400, maos-shell +100, maos-secrets +300, maos-domain +300, maos-eval +600, maos-wasm-host +400, maos-registry +300, maos-spirit-cli +300, maos-providers +400, maos-a2a-core +150, maos-a2a-tcp +150, maos-cohort +100, maos-iac +100, maos-audit +100, kernel-core +150 for the named FLAG-Winston deltas, xtask net ≤0 after retirements).
- **AC2.** `xtask/kloc.toml` rows re-based to HEAD-measured values + allowance, each with a `MEASURED GRANT 2026-09-xx (operator-authorized)` comment in the existing format; `_aggregate_hardfail` re-derived by the formula at `kloc.toml:82`.
- **AC3.** The retro-only rule at `kloc.toml:501` is amended: the aggregate is re-derived at epic close **or** by an operator-ratified re-base story.
- **AC4.** The kernel pin rule is unchanged (equality at `check_kernel_baseline.rs:7`); each FLAG-Winston story re-pins in its own commit. Documented in the story.
- **AC5.** Operator ratification recorded in the story file at landing (T0 precedent: a grant lands with the lines it authorizes).

### 15-3-single-phase-source-and-exit-command-check — Single phase source and the exit-command check

*Closes · Δ:* S4 · rule 1 mechanical · kernel-Δ 0

- **AC1 (command).** `grep -rho "const CURRENT_PHASE" xtask/src | wc -l` prints `1` (from 8: `gate_common.rs:166`, `check_cohort_mesh.rs:8`, `check_enterprise_identity.rs:15`, `check_enterprise_pdp.rs:51`, `check_escape_detector.rs:62`, `check_fkcs.rs:16`, `check_trial_attestation.rs:23`, `check_wasm_form_equiv.rs:50`); the escape detector uses `BindingClass`.
- **AC2.** `tests/phase-config.toml` is generated from `gate_common` with an explicit `v1_5`→`v1.5` mapping (sole consumer `xtask/src/main.rs:289`; comparison at `coverage_matrix.rs:98`); the coverage-matrix phase move is recorded.
- **AC3.** `cargo run -p xtask -- check-exit-commands` parses the fenced exit block of every `epics/epic-*.md` whose sprint status is not `done` and resolves each `maos` / `maosctl` / `cargo run -p xtask --` token against the binaries' `--help`; proven-red with a planted absent verb; enrolled Blocking.
- **AC4.** `check-cna-registration` (file-presence only, `check_cna_registration.rs:51-76`) retired in exchange; `gate-registry.toml` and `discipline.yml` updated; xtask net lines ≤0.

### 15-4-release-repair-and-first-signed-tag — Release repair and first signed tag

*Closes · Δ:* S5 · kernel-Δ 0

- **AC1 (command).** On a `v*` tag `release.yml` produces `maos` **and** `maosctl` for the three targets with `SHA256SUMS` + Ed25519 signature and its verify step passes (fix: `download-artifact@v4` with `merge-multiple`; today `sha256sum maos-*` fails "Is a directory"; `maosctl` is not built at `release.yml:44`).
- **AC2.** `aarch64-apple-darwin` and `aarch64-unknown-linux-gnu` compile in a CI leg before the tag (never compiled today).
- **AC3.** `sign-and-publish` `needs:` the discipline aggregate, not only `build`.
- **AC4.** The `MAOS_RELEASE_PUBKEY` guardrail (`maos-audit/src/release_verify.rs:280-291`, `option_env!`) fails loudly in release context when unset.
- **AC5.** Workspace version `0.1.0-alpha.1`; `STABILITY.md` regenerated (`stability_matrix.rs:17,137`); `Cargo.lock` committed; `docs/release/tag-procedure.md` written.
- **AC6.** The tag is cut only after `ops-provisioning-secrets-and-accounts` records both secrets present.

### 15-5-decision-adrs-and-provisioning-checklist — Decision ADRs and the provisioning checklist ◆

*Closes · Δ:* rule 7 · kernel-Δ 0

- **AC1 (command).** `ls docs/adr/ADR-060-*.md docs/adr/ADR-061-*.md docs/adr/ADR-062-*.md docs/adr/ADR-063-*.md` lists four accepted ADRs: **060** Spirit forms by trust tier (Fork C: `rust-inproc` first-party only, never admitted from a registry; WASM = third-party/polyglot form; `cli_wrapper` = agent CLIs; FR5 by form; FR33 via WASM toolchains); **061** Worker egress + credential (T3 egress allowlist from manifest, `env_clear`, kernel-minted scoped credential, out-of-port TL record); **062** mutating operator surface (loopback HTTP POST + bearer minted by `maos init`, one trust path; `maos-control/src/lib.rs:68-76` amended); **063** revocation model (reissue-is-revocation or CRL, and the machine-readable post-grace token).
- **AC2.** `docs/runbooks/provisioning-checklist.md` lists every item of preflight §2.6 with a state `present` / `waived-by <name> <date>`; the operator-lane row `ops-provisioning-secrets-and-accounts` reports against it.
- **AC3.** `RELEASE-HOLDS.md` and `STABILITY.md` state that published binaries keep the WASM engine off until Hold 2; `--features wasm-host` documented for self-builders.
- **AC4.** PRD `[DELTA-2026-09-04 R2]` and architecture §13.1 note cross-checked against the ADR text.

### 15-6-inference-replay-seam — Inference record/replay seam for every Spirit

*Closes · Δ:* preflight §2.4 · kernel-Δ 0

- **AC1 (command).** `MAOS_INFERENCE_MODE=replay maos run spirits/researcher/manifest.toml --once` and `MAOS_INFERENCE_MODE=record …` work for **every** Spirit through `InferencePortAdapter` (`main.rs:3136`), generalizing the researcher-only `MAOS_REPLAY_CASSETTE` sites (`main.rs:4607-4640`); `live` is the default only when a provider is configured, otherwise a typed `Unconfigured` halt with non-zero exit.
- **AC2.** Cassette format versioned; hand-authored cassettes carry `provenance: seed` and the journey harness labels them.
- **AC3.** `journey-nightly.yml:94` no longer passes `--live` to nextest; the `tier-2-rerecord` leg uses `MAOS_INFERENCE_MODE=record`.
- **AC4.** `MAOS_INFERENCE_MODE` registered `UserFacing` in `env_contract.rs`; `MAOS_REPLAY_CASSETTE` kept as the cassette path.

## Dependencies

None (opens the lane). 15-2 before 15-1 lands; 15-5 before Epics 16–17 open; 15-6 before Epic 18.
