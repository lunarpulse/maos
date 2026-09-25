# Provisioning checklist

**Report by editing this file.** Keep `ops-provisioning-secrets-and-accounts` at
`backlog` until every item it owns is complete. Promoting that sprint row to an
active status before a matching story file exists makes every D19-governed gate
fail at `xtask/src/gate_common.rs:96`. The checklist is the reporting surface;
the sprint-row comment is only its pointer.

**State vocabulary:** `present`, `absent`, `waived-by <name> <YYYY-MM-DD>`, or
`not-a-provisioning-item (<reason>)`.

**Inventory source:** the secret denominator is every distinct
`secrets.<NAME>` reference in `.github/workflows/*.yml` or `*.yaml`, excluding GitHub's
auto-provided `GITHUB_TOKEN`. The inventory currently contains eight names
(seven until 2026-09-24, when the operator ruled `CI_EVIDENCE_AUDIT_KEY` in).
Non-secret external dependencies are listed from their real tree anchors rather
than inherited from planning prose.

## Checklist

| Item | Consumer or tree anchor | Behavior when unavailable | Owning sprint row | State |
|---|---|---|---|---|
| `CI_EVIDENCE_AUDIT_KEY` | `.github/workflows/discipline.yml` `check-multi-tenant-loom` step "Install the CI-only evidence audit key" | hard fail for the gate: without it the required `reza-three-team-three-region-journey` leg is `ABSENT`, `check-multi-tenant-loom` is red, and through `v1-0-ship-gate` so is `aggregate` (the tag precondition). The step warns and writes nothing | `ops-provisioning-secrets-and-accounts` | `present` |
| `DOCKERHUB_TOKEN` | `.github/workflows/container.yml:44` | hard fail: Docker Hub login cannot authenticate | `ops-provisioning-secrets-and-accounts` | `absent` |
| `DOCKERHUB_USERNAME` | `.github/workflows/container.yml:43,52,80` | hard fail: Docker Hub login/image name cannot resolve | `ops-provisioning-secrets-and-accounts` | `absent` |
| `FUZZ_LEDGER_WRITE_TOKEN` | `.github/workflows/fuzz-cadence.yml:124` | skip-by-design: expression falls back to auto-provided `GITHUB_TOKEN` | `ops-provisioning-secrets-and-accounts` | `not-a-provisioning-item (GITHUB_TOKEN fallback is sufficient; fuzz branch is blocked by a code bug)` |
| `MAOS_ANTHROPIC_API_KEY` | `.github/workflows/journey-nightly.yml:88` | hard fail for the paid live re-record leg; hermetic replay is unaffected | `ops-nightly-cassette-rerecord` | `absent` |
| `MAOS_RELEASE_PUBKEY` | `.github/workflows/release.yml:41-46,90-110` | hard fail: an absent/empty key fails the exact lowercase parser; the shipped `maosctl` also refuses the bundled development key | `ops-provisioning-secrets-and-accounts` | `present` |
| `RELEASE_SIGNING_KEY` | `.github/workflows/release.yml:93,95` | hard fail once artifact assembly reaches `release-verify --sign` | `ops-provisioning-secrets-and-accounts` | `present` |
| `RTO_LEDGER_WRITE_TOKEN` | `.github/workflows/rpo-rto-cadence.yml:113` | skip-by-design: expression falls back to auto-provided `GITHUB_TOKEN` | `ops-provisioning-secrets-and-accounts` | `not-a-provisioning-item (GITHUB_TOKEN has already produced the live rto-ledger history)` |
| `GitHub OIDC for cosign` | `.github/workflows/container.yml:23-26,74-80` | invalid-ref only if workflow `id-token: write` is removed; no operator secret is required | `ops-provisioning-secrets-and-accounts` | `present` |
| `macOS/aarch64 release build` | `.github/workflows/release.yml:10-60`; `.github/workflows/discipline.yml` `release-dry-run` | partial: Linux aarch64 is blocking per commit; macOS arm64 remains a required manual pre-tag dispatch and tagged-release leg | `ops-provisioning-secrets-and-accounts` | `present` |
| `lunarpulse/homebrew-maos` | `packaging/homebrew/maos.rb` | hard fail for publication: repository does not exist and formula hashes remain placeholders | `ops-brew-tap-and-aur-publication` | `absent` |
| `AUR account and package publication` | `packaging/aur/PKGBUILD` | hard fail for publication: package was not submitted and hashes/version remain scaffold values | `ops-brew-tap-and-aur-publication` | `absent` |
| `rto-ledger branch permission` | `.github/workflows/rpo-rto-cadence.yml:111-135` | hard fail only if the existing `GITHUB_TOKEN` branch permission is removed | `ops-provisioning-secrets-and-accounts` | `present` |
| `fuzz-ledger branch permission` | `.github/workflows/fuzz-cadence.yml:122-158` | skip-by-design at HEAD: malformed jq array-plus-object update leaves nothing staged before push | `ops-provisioning-secrets-and-accounts` | `not-a-provisioning-item (blocked by fuzz-cadence.yml:146 code bug filed to 20-3)` |
| `self-hosted fuzz runner` | `.github/workflows/fuzz-cadence.yml` | skip-by-design: workflow uses GitHub-hosted runners and has no self-hosted selector | `ops-provisioning-secrets-and-accounts` | `absent` |
| `24-hour fuzz execution` | `.github/workflows/fuzz-cadence.yml:81-82` | skip-by-design: configured run is 600 seconds across four workers, not 24 hours | `ops-provisioning-secrets-and-accounts` | `not-a-provisioning-item (no 24-hour workflow leg exists)` |
| `external third-party authors` | `xtask/src/check_third_party_trial.rs:325` | hard fail below the gate's honest denominator; the current gate requires 12 rather than the former prose's 3 | `ops-external-cohort-and-pen-test` | `absent` |
| `external penetration test` | `RELEASE-HOLDS.md:24` | status and ship effect are authoritative only in Hold 1 | `ops-external-cohort-and-pen-test` | `not-a-provisioning-item (tracked by RELEASE-HOLDS.md Hold 1)` |
| `Google Calendar OAuth` | `ops-google-calendar-oauth-mcp` | skip-by-design: CI uses the conformant fixture; no credential name exists in the tree | `ops-google-calendar-oauth-mcp` | `absent` |
| `Gemini live credential` | `ops-live-gemini-leg` | skip-by-design: paid recording is operator-only and no credential name exists in the tree | `ops-live-gemini-leg` | `absent` |
| `Bedrock and Vertex credentials` | `later-bedrock-vertex-and-kms-backends` | skip-by-design: post-v1.0 drivers do not exist and no credential names exist in the tree | `later-bedrock-vertex-and-kms-backends` | `absent` |

## Evidence and blockers

- The repository secret, variable, and runner inventories were all empty when
  measured for Story 15-5. No workflow contains `vars.`, `environment:`, or
  `runs-on: self-hosted`.
- `rto-ledger` is present and operational: eight weekly appends existed since
  2026-07-19 when measured. Its success proves the default `GITHUB_TOKEN`
  permission; a dedicated write token is optional, not missing provisioning.
- `fuzz-ledger` is not a provisioning task. At
  `.github/workflows/fuzz-cadence.yml:146`, `.records` is an array while
  `$rec[0]` is an object, so jq refuses the addition. The failed command is left
  of `&&`; execution continues, nothing is staged, and the push is never
  attempted. Story 20-3 owns the code repair.
- Provisioning `MAOS_RELEASE_PUBKEY` is required at binary build time. The
  compile-time parser rejects an absent secret (GitHub supplies an empty value),
  non-lowercase input, and any length other than 64 hex characters. The release
  workflow then executes the shipped `maosctl release-pubkey` and refuses the
  documented development key before self-verification or publication. The
  existing `production_pubkey_must_differ_from_dev_seed` unit assertion remains
  an additional guard; it is not the artifact-level publication predicate.
- `RELEASE_SIGNING_KEY` is still absent. Artifact flattening, the explicit
  six-binary manifest, and signing invocation are wired; publication now reaches
  `release-verify --sign` and hard-fails when this key is unavailable.
- The penetration-test schedule, owner, findings threshold, and GA effect are
  not duplicated here. `RELEASE-HOLDS.md` Hold 1 is authoritative.
- **Re-measured 2026-09-24 (work started):** the inventory rule still yields
  exactly seven names (`DOCKERHUB_TOKEN`, `DOCKERHUB_USERNAME`,
  `FUZZ_LEDGER_WRITE_TOKEN`, `MAOS_ANTHROPIC_API_KEY`, `MAOS_RELEASE_PUBKEY`,
  `RELEASE_SIGNING_KEY`, `RTO_LEDGER_WRITE_TOKEN`), and the repository's
  Actions secrets and variables APIs both return `total_count: 0` — every
  `absent` row above is still absent. Key tooling in the tree:
  `maosctl audit keygen --output <path>` writes a 64-hex Ed25519 seed at 0600
  and prints only a truncated public-key fingerprint; `release-verify --sign`
  parses `RELEASE_SIGNING_KEY` with the same seed parser; `MAOS_AUDIT_KEY` is a
  PATH to a key file, not a key.
- **Blocker surfaced 2026-09-24 — RULED the same day (now the `CI_EVIDENCE_AUDIT_KEY` row):** `check-multi-tenant-loom`
  cannot pass in CI without an operator audit verification key. Its required
  `reza-three-team-three-region-journey` leg is `ABSENT` whenever
  `EvidenceVerifier::key_available()` is false
  (`xtask/src/check_multi_tenant_loom.rs:192`), while `EvidenceVerifier::load`
  documents that "CI may omit the key" (`xtask/src/evidence_ledger.rs:541-545`).
  The gate reaches `aggregate` through `v1-0-ship-gate`, so `aggregate` — and
  `check_release_precondition` for the tag — stays red until this is ruled.
  Measured on run 36002875453 (`cd4422ef`): every other leg green; this leg was
  already `ABSENT` on 6af9423a, masked because the gate names only its first
  red. **Operator ruling:** a DEDICATED CI-only audit key, never the
  operator's (R-RG1's operator-pinned key stays off CI); signatures in that job
  prove "ran in this CI job". The harness signer
  (`tests/harness/evidence_record.rs:82-83`) and the verifier read the same
  `MAOS_AUDIT_KEY` path, so one export makes the job sign and verify.

- **`macOS/aarch64 release build` present 2026-09-25:** first macOS build in project history, pre-tag rehearsal run 36089542264 on `0c661995` green, and the tagged release (run 36094027381) published `maos-darwin-arm64` + `maosctl-darwin-arm64`, both covered by the verified `SHA256SUMS.sig`.
- **Provisioned 2026-09-24 (state column holds only the legal vocabulary; the evidence lives here):**
  - `CI_EVIDENCE_AUDIT_KEY`: repository secret created 2026-09-24T13:53:06Z. Run 36007877995 on `3c1bc822` received it EMPTY — the run was created at 13:45:33, before the secret existed; its log shows `CI_EVIDENCE_AUDIT_KEY: ` rather than `***`. First delivery is expected on the next run created after 13:53:06.
  - `MAOS_RELEASE_PUBKEY`: repository secret created 2026-09-24T14:05:44Z; derived offline from the same seed as `RELEASE_SIGNING_KEY` with the procedure below; operator verified the derived key's first/last 8 hex equal `maosctl audit keygen`'s fingerprint. First CI use: `release.yml` `Verify artifacts (self-test)` at the tag push, which refuses to publish on a mismatched pair.
  - `RELEASE_SIGNING_KEY`: repository secret exists, created 2026-09-24T13:54:40Z, generated offline by the operator; name confirmed via the secrets API, value never read. Its pairing with `MAOS_RELEASE_PUBKEY` is first exercised by `release.yml` `sign-and-publish` at the tag push.
  - Delivery measured: run 36058940696 (`9b77c3e3`) received `CI_EVIDENCE_AUDIT_KEY` as `***`, it passed the step's 64-hex shape check, and `check-multi-tenant-loom` passed with `reza-three-team-three-region-journey` `PROVEN_LIVE_SIGNED`. The first delivered value was 65 bytes (one non-whitespace character too many) and was re-set by the operator.

## Operator procedures (the two KEY procedures were tested end to end with THROWAWAY keys, 2026-09-24; Docker Hub is account setup and untested here)

### `RELEASE_SIGNING_KEY` + `MAOS_RELEASE_PUBKEY` — generate OFFLINE (operator ruling)

The public half is compiled into every shipped `maosctl`; losing the seed
means no existing `maosctl` can ever verify a later release. Generate on a
trusted machine, keep an offline backup, never paste the seed into chat.

```sh
umask 077 && mkdir -p ~/maos-release-key && cd ~/maos-release-key
maosctl audit keygen --output ./release-signing.key   # 64-hex seed, 0600; prints a fingerprint aaaaaaaa..bbbbbbbb
# Full 64-hex public key (PKCS#8 Ed25519 prefix + seed -> DER pubkey -> last 32 bytes):
printf '302e020100300506032b657004220420%s' "$(cat ./release-signing.key)" \
  | xxd -r -p | openssl pkey -inform DER -pubout -outform DER | tail -c 32 | xxd -p -c 64
```

Check: the printed public key's first and last 8 hex digits must equal the
fingerprint `keygen` printed. Then set two repository Actions secrets:
`RELEASE_SIGNING_KEY` = the file's contents (64 lowercase hex), and
`MAOS_RELEASE_PUBKEY` = the derived public key (64 lowercase hex). Tested with a
throwaway key: fingerprint matched; a `SHA256SUMS` signed with it verified
under `maosctl install --from-local ./dist --verify-only --release-pubkey
<derived>` (rc 0) and was refused under a wrong public key (rc 1).

### `CI_EVIDENCE_AUDIT_KEY` — CI-only; any machine

```sh
maosctl audit keygen --output ./ci-evidence-audit.key   # never your ~/.config/maos/audit-signing.key
```

Set the repository Actions secret `CI_EVIDENCE_AUDIT_KEY` to the file's
contents, then delete the local copy — CI is its only holder. Tested with a
throwaway key under CI's own binding (`GITHUB_ACTIONS=true`, `GITHUB_SHA`,
`GITHUB_RUN_ID`, `GITHUB_RUN_ATTEMPT`) and the three-team substrate.

### `DOCKERHUB_USERNAME` + `DOCKERHUB_TOKEN`

Requires a Docker Hub account (the lead-time item). Create an access token with
push scope for the `maos` repository and set both secrets; consumers are
`.github/workflows/container.yml:43-52,80`.

After setting any secret, report it here by flipping its row to `present`; the
agent can confirm names (never values) via
`GET /repos/lunarpulse/maos/actions/secrets`.

## Related operator rows

Items outside `ops-provisioning-secrets-and-accounts` remain with their named
owners: `ops-nightly-cassette-rerecord`,
`ops-external-cohort-and-pen-test`,
`ops-brew-tap-and-aur-publication`, `ops-google-calendar-oauth-mcp`,
`ops-live-gemini-leg`, and `later-bedrock-vertex-and-kms-backends`.
