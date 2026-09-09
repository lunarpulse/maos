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
auto-provided `GITHUB_TOKEN`. The inventory currently contains seven names.
Non-secret external dependencies are listed from their real tree anchors rather
than inherited from planning prose.

## Checklist

| Item | Consumer or tree anchor | Behavior when unavailable | Owning sprint row | State |
|---|---|---|---|---|
| `DOCKERHUB_TOKEN` | `.github/workflows/container.yml:44` | hard fail: Docker Hub login cannot authenticate | `ops-provisioning-secrets-and-accounts` | `absent` |
| `DOCKERHUB_USERNAME` | `.github/workflows/container.yml:43,52,80` | hard fail: Docker Hub login/image name cannot resolve | `ops-provisioning-secrets-and-accounts` | `absent` |
| `FUZZ_LEDGER_WRITE_TOKEN` | `.github/workflows/fuzz-cadence.yml:124` | skip-by-design: expression falls back to auto-provided `GITHUB_TOKEN` | `ops-provisioning-secrets-and-accounts` | `not-a-provisioning-item (GITHUB_TOKEN fallback is sufficient; fuzz branch is blocked by a code bug)` |
| `MAOS_ANTHROPIC_API_KEY` | `.github/workflows/journey-nightly.yml:88` | hard fail for the paid live re-record leg; hermetic replay is unaffected | `ops-nightly-cassette-rerecord` | `absent` |
| `MAOS_RELEASE_PUBKEY` | `.github/workflows/release.yml:77,82` | vacuous pass: compile-time guard is omitted when unset | `ops-provisioning-secrets-and-accounts` | `absent` |
| `RELEASE_SIGNING_KEY` | `.github/workflows/release.yml:74,76` | hard fail once artifact assembly reaches `release-verify --sign` | `ops-provisioning-secrets-and-accounts` | `absent` |
| `RTO_LEDGER_WRITE_TOKEN` | `.github/workflows/rpo-rto-cadence.yml:113` | skip-by-design: expression falls back to auto-provided `GITHUB_TOKEN` | `ops-provisioning-secrets-and-accounts` | `not-a-provisioning-item (GITHUB_TOKEN has already produced the live rto-ledger history)` |
| `GitHub OIDC for cosign` | `.github/workflows/container.yml:23-26,74-80` | invalid-ref only if workflow `id-token: write` is removed; no operator secret is required | `ops-provisioning-secrets-and-accounts` | `present` |
| `macOS/aarch64 release build` | `.github/workflows/release.yml:10-54` | skip-by-design until a release tag dispatches the matrix | `ops-provisioning-secrets-and-accounts` | `absent` |
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
- Provisioning `MAOS_RELEASE_PUBKEY` alone does not make its guard reliable.
  `production_pubkey_must_differ_from_dev_seed` uses `option_env!` at
  `crates/maos-audit/src/release_verify.rs:280-293`, `maos-audit` has no
  `build.rs` or `cargo:rerun-if-env-changed`, and
  `.github/workflows/release.yml:66` restores `rust-cache`. A cached rlib can
  therefore retain the wrong compile-time value.
- Provisioning `RELEASE_SIGNING_KEY` alone cannot produce a signature at HEAD.
  `.github/workflows/release.yml:67-73` downloads matrix artifacts without
  `merge-multiple: true`; `sha256sum maos-*` receives directories and fails
  before the key is read. Story 15-4 owns that repair. The workflow also
  packages only `maos`, while `ops-first-signed-tag` expects the final release
  artifact set.
- The penetration-test schedule, owner, findings threshold, and GA effect are
  not duplicated here. `RELEASE-HOLDS.md` Hold 1 is authoritative.

## Related operator rows

Items outside `ops-provisioning-secrets-and-accounts` remain with their named
owners: `ops-nightly-cassette-rerecord`,
`ops-external-cohort-and-pen-test`,
`ops-brew-tap-and-aur-publication`, `ops-google-calendar-oauth-mcp`,
`ops-live-gemini-leg`, and `later-bedrock-vertex-and-kms-backends`.
