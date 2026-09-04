# Epic 20 — Ship It (W5)

**Status:** `backlog` — Round 2 correct-course 2026-09-04 (`sprint-change-proposal-2026-09-04-round2.md`, Lunarpulse-ratified; Fork C). Replaces the morning's Epic 20 file. `sprint-status.yaml` is authoritative for status.

**Wave / duration:** 5–7 weeks (measured) · **Makes true:** v1.0-rc · a third party publishes and installs a Spirit without the source tree; the release is real; the gates say what they measured

**Closes:** S4, S5; FR1, FR3 (partial), FR35, FR36, FR37, FR59; NFR-Meta-3, NFR-Ops-12; D1, D8, D9, D10, D20

**Hermetic exit command (CI, no secrets; the epic's first CI job, red until green):**

```
# clean VM
curl -L <release>/maos-<target>.tar.gz | tar xz && ./maosctl install --from-local ./maos --verify-only
./maos-registry-server --bind 127.0.0.1:7500 &  &&  maos-spirit publish --registry-uri http://127.0.0.1:7500 --tier=local spirits/researcher
maosctl spirit install researcher@1.0 --registry-uri http://127.0.0.1:7500 && MAOS_INFERENCE_MODE=replay maos shell
```

**Operator-lane legs (never in the exit):**
- `ops-brew-tap-and-aur-publication` (tap repo + token, AUR account)
- `ops-external-cohort-and-pen-test` (≥3 external authors, pen-test firm and dates)
- `ops-ga-tag-after-holds` (`v1.0.0` + LTS clock after Holds 1–2)

**Kernel-Δ:** ZERO kernel-Δ @24472.

**Confidence rules (ratified 2026-09-04 R2, binding):** 1 verb-first (exit verbs exist at HEAD or are AC1 in this epic; `check-exit-commands`) · 2 hermetic exit, live optional (paid/human/clock legs are operator-lane rows) · 3 foundations first · 4 cite-or-cut (every AC names file:line + kernel-Δ) · 5 spike before build · 6 size from measurement (+30%) · 7 one decision per fork · 8 inventory from the tree.

**Dev-gate:** external holds (NFR-Sec-7, NFR-Comp-1) = GA ledger only. **Model/review:** frontier allowlist + §A6 net binding; non-degradable on stories marked ◆.

---

## Stories

| Key | Story | Closes · Δ |
|---|---|---|
| `20-1-registry-client-install-verb-vetter-and-yank` | Registry client, `spirit install`, vetter CLI, yank policy | FR35/36/37/59 · kernel-Δ 0 |
| `20-2-deb-airgap-docker-and-formula-corrections` | deb, air-gap leg, Docker arg, formula corrections | FR1 · NFR-Ops-12 · kernel-Δ 0 |
| `20-3-gate-honesty-pass` | Gate honesty pass ◆ | S4 · D1/D8/D9/D10/D20 · NFR-Sec-5/Ops-9/Meta-3 · kernel-Δ 0 |
| `20-4-gemini-driver-and-endpoint-pin` | Gemini driver and endpoint pin enforcement | FR3 (partial) · kernel-Δ 0 |
| `20-5-v1-0-rc-tag-and-lts-rule` | `v1.0.0-rc.1` and the LTS rule | NFR-Maint-6 · kernel-Δ 0 |

AC1 is always the command and what the operator observes; every AC cites the code it changes; `bmad-create-story` may tighten but not add uncited ACs.

### 20-1-registry-client-install-verb-vetter-and-yank — Registry client, `spirit install`, vetter CLI, yank policy

*Closes · Δ:* FR35/36/37/59 · kernel-Δ 0

- **AC1 (command).** Exit lines 2–3: `maos-spirit publish` reaches the real server (the stub-only guard at `publish.rs:267-272` deleted; `McpSpiritRegistryClient` from `main.rs:2881` wired into `maos-spirit-cli`); `maosctl spirit install <name>@<ver>` (new verb — `install` already means the release binary) fetches manifest + artifact and runs the admission `import` runs today (`subcommands.rs:942-1085`).
- **AC2.** `maos-spirit vet issue|revoke` wraps `issue_attestation` (`attestation.rs:150`, zero callers today) plus a revoke primitive and keyring enrollment.
- **AC3.** Yank applies operator policy warn / quarantine / auto-revoke (today `TlYankObserver::on_yank` journals only, `main.rs:305-338`).
- **AC4.** WASM artifacts (17-3b) install through the same path.

### 20-2-deb-airgap-docker-and-formula-corrections — deb, air-gap leg, Docker arg, formula corrections

*Closes · Δ:* FR1 · NFR-Ops-12 · kernel-Δ 0

- **AC1 (command).** `release.yml` produces a `.deb` via `dpkg-deb` and an air-gap matrix leg (`--no-default-features --features air-gap`; `check-mock-not-in-release` re-run); `Dockerfile` takes `ARG FEATURES`; `check-air-gap` is enrolled; the netns script reports SKIP as non-green.
- **AC2.** `packaging/homebrew/maos.rb` and `packaging/aur/PKGBUILD` reference the real artifact set (no `SCAFFOLD`/`PLACEHOLDER_SHA256`); publication is the operator-lane row.
- **AC3.** `maos uninstall` (16-4) removes what the package installed.

### 20-3-gate-honesty-pass — Gate honesty pass ◆

*Closes · Δ:* S4 · D1/D8/D9/D10/D20 · NFR-Sec-5/Ops-9/Meta-3 · kernel-Δ 0

- **AC1 (command).** `cargo run -p xtask -- check-fuzz-floor` and the other **23** absent-pass gates report `EvidenceState::Absent` as non-green (7 carry the seam today); `check-rto-gate` no longer exits 0 as `skipped`.
- **AC2.** `check-third-party-trial` gains an honest-N mode (`participants_total < 12` → `preliminary`, not FAIL; the strata floor dated), so the real cohort file can be committed.
- **AC3.** `fuzz-ledger` and `rto-ledger` branches exist; fuzz runs as chained ≤6 h jobs; the floor stays advisory until the ledger spans 90 days, and the AC says so.
- **AC4.** `tests/coverage-matrix.yaml` is generated from `gate-registry.toml` plus a `covers = [...]` field per gate (no proc-macro crate); `invariant-lock` re-ratified.
- **AC5.** No signed erasure attestation asserts a completion it did not observe (14-e1); every gate carries `BindingClass` (e12-b1, D20).
- **AC6.** ≥10 gates retired with invariant-lock review; xtask net lines ≤0.

### 20-4-gemini-driver-and-endpoint-pin — Gemini driver and endpoint pin enforcement

*Closes · Δ:* FR3 (partial) · kernel-Δ 0

- **AC1 (command).** `MAOS_DEFAULT_PROVIDER=gemini MAOS_INFERENCE_MODE=replay maos shell` replies from a cassette; live with a key is operator lane.
- **AC2.** `provider_endpoint_pin` is enforced in the `maos-bin`/`maos-providers` adapter (today parse-only, `maos-manifest:2103-2171`); a mismatch is a typed refusal.
- **AC3.** `xtask/model-currency.toml` governs the Gemini IDs; Bedrock/Vertex/Vault are the `later-bedrock-vertex-and-kms-backends` row.

### 20-5-v1-0-rc-tag-and-lts-rule — `v1.0.0-rc.1` and the LTS rule

*Closes · Δ:* NFR-Maint-6 · kernel-Δ 0

- **AC1 (command).** `git tag v1.0.0-rc.1 && git push --tags` produces signed artifacts including the `.deb` and the air-gap variant; `STABILITY.md` shows the rc row.
- **AC2.** The LTS clock starts at `v1.0.0`, which is cut by `ops-ga-tag-after-holds` after RELEASE-HOLDS Holds 1–2 clear; `docs/release` says so.

## Dependencies

Epic 19 (the J1 demo is what a cohort installs). 20-1 before 20-5.
