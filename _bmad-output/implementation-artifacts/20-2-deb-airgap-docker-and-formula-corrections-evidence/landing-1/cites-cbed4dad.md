# Rule-9 citation remeasurement at cbed4dad

All source bytes below were read with `git show cbed4dad:<path>` (exit 0 per file); offsets are baseline, not future landing offsets. This supersedes creation C1–C19, T1–T3 and P1–P9 for the touched surfaces. Historical hosted run 36094027288 is not re-run or relabelled as a new hosted observation.

## .github/workflows/release.yml

- `.github/workflows/release.yml:71` — `- name: Build release binaries`
- `.github/workflows/release.yml:77` — `- name: Check mock-not-in-release`
- `.github/workflows/release.yml:79` — `- name: Package artifacts`
- `.github/workflows/release.yml:103` — `sign-and-publish:`
- `.github/workflows/release.yml:128` — `- name: Generate SHA256SUMS and sign`
- `.github/workflows/release.yml:135` — `- name: "Guardrail: production pubkey must differ from dev seed"`
- `.github/workflows/release.yml:140` — `- name: Refuse bundled development release key`
- `.github/workflows/release.yml:148` — `- name: Verify artifacts (self-test)`
- `.github/workflows/release.yml:152` — `- name: Create GitHub Release`

## .github/workflows/discipline.yml

- `.github/workflows/discipline.yml:536` — `check-exit-commands:`
- `.github/workflows/discipline.yml:6505` — `echo "  check-exit-commands: ${{ needs.check-exit-commands.result }}"`
- `.github/workflows/discipline.yml:560` — `cargo test -p maos-bin --no-default-features --test verb_table_15_3 -- --test-threads=1`
- `.github/workflows/discipline.yml:1532` — `onb-nfr2-timing:`
- `.github/workflows/discipline.yml:5825` — `release-dry-run:`
- `.github/workflows/discipline.yml:5852` — `- name: Sign staged manifest with development seed`
- `.github/workflows/discipline.yml:5861` — `- name: Reject signature from different seed`
- `.github/workflows/discipline.yml:5879` — `- name: Upload dry-run artifacts`
- `.github/workflows/discipline.yml:6366` — `v1-0-ship-gate:`
- `.github/workflows/discipline.yml:6516` — `aggregate:`

## xtask/src/main.rs

- `xtask/src/main.rs:12` — `mod check_air_gap;`
- `xtask/src/main.rs:778` — `ReleaseDryRun {`
- `xtask/src/main.rs:1455` — `Commands::ReleaseDryRun {`
- `xtask/src/main.rs:875` — `CheckAirGap {`
- `xtask/src/main.rs:1501` — `Commands::CheckAirGap {`

## xtask/src/release_dry_run.rs

- `xtask/src/release_dry_run.rs:27` — `pub const ARTIFACTS: &[Artifact] = &[`
- `xtask/src/release_dry_run.rs:113` — `pub fn generate_manifest(dist_dir: &Path, requested_targets: &[&str]) -> Result<Report, String> {`
- `xtask/src/release_dry_run.rs:163` — `fn build_and_stage(dist_dir: &Path, requested_targets: &[&str]) -> Result<(), String> {`

## xtask/src/check_air_gap.rs

- `xtask/src/check_air_gap.rs:12` — `const NETWORK_SYMBOLS: &[&str] = &[`
- `xtask/src/check_air_gap.rs:46` — `pub fn run(`
- `xtask/src/check_air_gap.rs:52` — `if build_first {`

## xtask/src/check_mock_not_in_release.rs

- `const MOCK_SYMBOLS`: no literal hit (renamed/format-sensitive; do not invent an offset).
- `xtask/src/check_mock_not_in_release.rs:31` — `pub fn run(binary_path: &str, build_first: bool, json: bool) -> Result<(), String> {`

## xtask/src/check_ship_gate_completeness.rs

- `xtask/src/check_ship_gate_completeness.rs:17` — `const EXPECTED_GATES: &[&str] = &[`

## crates/maos-bin/src/main.rs

- `crates/maos-bin/src/main.rs:1586` — `fn print_air_gap_usage() {`
- `crates/maos-bin/src/main.rs:1610` — `fn air_gap_run(_args: &[String]) {`
- `crates/maos-bin/src/main.rs:1543` — `#[cfg(not(feature = "network"))]`
- `crates/maos-bin/src/main.rs:1585` — `#[cfg(not(feature = "network"))]`
- `crates/maos-bin/src/main.rs:1599` — `#[cfg(not(feature = "network"))]`
- `crates/maos-bin/src/main.rs:1609` — `#[cfg(not(feature = "network"))]`
- `crates/maos-bin/src/main.rs:1644` — `#[cfg(not(feature = "network"))]`
- `crates/maos-bin/src/main.rs:1713` — `#[cfg(not(feature = "network"))]`
- `crates/maos-bin/src/main.rs:1749` — `#[cfg(not(feature = "network"))]`
- `crates/maos-bin/src/main.rs:1768` — `#[cfg(not(feature = "network"))]`
- `crates/maos-bin/src/main.rs:1863` — `#[cfg(not(feature = "network"))]`
- `crates/maos-bin/src/main.rs:3176` — `#[cfg(not(feature = "network"))]`
- `crates/maos-bin/src/main.rs:3190` — `#[cfg(not(feature = "network"))]`
- `crates/maos-bin/src/main.rs:3200` — `#[cfg(not(feature = "network"))]`
- `crates/maos-bin/src/main.rs:7388` — `#[cfg(not(feature = "network"))]`
- `crates/maos-bin/src/main.rs:1544` — `fn main() {`
- `crates/maos-bin/src/main.rs:1882` — `async fn main() -> Result<(), Box<dyn std::error::Error>> {`
- `crates/maos-bin/src/main.rs:1557` — `verbs::Outcome::Version => {`
- `crates/maos-bin/src/main.rs:1912` — `verbs::Outcome::Version => {`
- `crates/maos-bin/src/main.rs:10137` — `async fn run_uninstall_cascade(`
- `crates/maos-bin/src/main.rs:10629` — `async fn run_uninstall_cascade(`
- `crates/maos-bin/src/main.rs:10691` — `async fn run_uninstall_cascade_inner(`

## crates/maos-bin/src/purge.rs

- `crates/maos-bin/src/purge.rs:276` — `for name in ["export-seq.json", "transparency.airgap-stub.log"] {`
- `crates/maos-bin/src/purge.rs:830` — `next_step: format!(`
- `crates/maos-bin/src/purge.rs:831` — `"{}. Remove the Cargo-owned binary with: cargo uninstall maos",`
- `crates/maos-bin/src/purge.rs:1074` — `println!("maos purge: remove the Cargo-owned binary with: cargo uninstall maos");`

## crates/maos-bin/src/verbs.rs

- `crates/maos-bin/src/verbs.rs:164` — `pub const MAOS_ONE_SHOT_MODES: &[&str] = &[`
- `crates/maos-bin/src/verbs.rs:199` — `/// A mode resolved through [`MAOS_ONE_SHOT_MODES`].`
- `crates/maos-bin/src/verbs.rs:239` — `MAOS_ONE_SHOT_MODES`
- `crates/maos-bin/src/verbs.rs:319` — `for mode in MAOS_ONE_SHOT_MODES {`
- `crates/maos-bin/src/verbs.rs:196` — `"uninstall",`

## crates/maos-bin/Cargo.toml

- `crates/maos-bin/Cargo.toml:15` — `default = ["network"]`
- `crates/maos-bin/Cargo.toml:29` — `air-gap = []  # Compile-out all networking: reqwest, hyper client, tokio::net, DNS resolvers`

## crates/maos-audit/src/release_verify.rs

- `crates/maos-audit/src/release_verify.rs:25` — `pub const DEFAULT_RELEASE_PUBKEY: [u8; 32] = [`
- `crates/maos-audit/src/release_verify.rs:58` — `None => DEFAULT_RELEASE_PUBKEY,`
- `crates/maos-audit/src/release_verify.rs:56` — `pub const RELEASE_PUBKEY: [u8; 32] = match option_env!("MAOS_RELEASE_PUBKEY") {`
- `crates/maos-audit/src/release_verify.rs:174` — `pub fn sign_sha256sums(sha256sums_bytes: &[u8], seed: &[u8; 32]) -> [u8; 64] {`
- `crates/maos-audit/src/release_verify.rs:154` — `pub fn verify_release_signature(`
- `crates/maos-audit/src/release_verify.rs:205` — `pub fn verify_release(`

## packaging/deb/control

- `packaging/deb/control:5` — `Build-Depends: debhelper-compat (= 13), cargo, rustc (>= 1.88), python3-cryptography`

## packaging/deb/rules

- `packaging/deb/rules:5` — `# The embedded RELEASE_PUBKEY must be replaced with the production key`
- `packaging/deb/rules:10` — `RELEASE_PUBKEY := bedd2ba634da724027983f369149f108541f43e624a846438c01452ca7f469e7`
- `packaging/deb/rules:31` — `pubkey = Ed25519PublicKey.from_public_bytes(bytes.fromhex('$(RELEASE_PUBKEY)'))`
- `packaging/deb/rules:28` — `python3 - <<'PY'`
- `packaging/deb/rules:51` — `expected_hash=$$(awk -v fname="$(MAOS_BINARY)" '$$0 ~ "  " fname {print $$1}' SHA256SUMS); \`

## packaging/rpm/maos.spec

- `packaging/rpm/maos.spec:4` — `# NOTE: At v0.5 this is a SCAFFOLD. The RPM must be built against the first`
- `packaging/rpm/maos.spec:9` — `Version:        0.5.0`
- `packaging/rpm/maos.spec:20` — `%global release_pubkey bedd2ba634da724027983f369149f108541f43e624a846438c01452ca7f469e7`
- `packaging/rpm/maos.spec:49` — `pubkey = Ed25519PublicKey.from_public_bytes(bytes.fromhex('%{release_pubkey}'))`

## packaging/aur/PKGBUILD

- `packaging/aur/PKGBUILD:6` — `# NOTE: At v0.5 this is a SCAFFOLD. The AUR package must be submitted`
- `packaging/aur/PKGBUILD:12` — `pkgver=0.5.0`
- `packaging/aur/PKGBUILD:7` — `# separately after the first tagged release. The embedded RELEASE_PUBKEY`
- `packaging/aur/PKGBUILD:36` — `RELEASE_PUBKEY='bedd2ba634da724027983f369149f108541f43e624a846438c01452ca7f469e7'`
- `packaging/aur/PKGBUILD:45` — `pubkey = Ed25519PublicKey.from_public_bytes(bytes.fromhex('${RELEASE_PUBKEY}'))`
- `packaging/aur/PKGBUILD:27` — `sha256sums_x86_64=('PLACEHOLDER_SHA256_LINUX_AMD64'`
- `packaging/aur/PKGBUILD:28` — `'PLACEHOLDER_SHA256_SUMS'`
- `packaging/aur/PKGBUILD:29` — `'PLACEHOLDER_SHA256_SIG')`
- `packaging/aur/PKGBUILD:30` — `sha256sums_aarch64=('PLACEHOLDER_SHA256_LINUX_ARM64'`
- `packaging/aur/PKGBUILD:31` — `'PLACEHOLDER_SHA256_SUMS'`
- `packaging/aur/PKGBUILD:32` — `'PLACEHOLDER_SHA256_SIG')`

## packaging/homebrew/maos.rb

- `packaging/homebrew/maos.rb:7` — `# NOTE: At v0.5 this formula is a SCAFFOLD. The tap repository`
- `packaging/homebrew/maos.rb:15` — `version "0.5.0"`
- `packaging/homebrew/maos.rb:9` — `# template for the first tagged release. The embedded RELEASE_PUBKEY must`
- `packaging/homebrew/maos.rb:20` — `RELEASE_PUBKEY = "bedd2ba634da724027983f369149f108541f43e624a846438c01452ca7f469e7".freeze`
- `packaging/homebrew/maos.rb:64` — `verify_ed25519_signature(buildpath/"SHA256SUMS", buildpath/"SHA256SUMS.sig", RELEASE_PUBKEY)`
- `packaging/homebrew/maos.rb:50` — `depends_on "python@3" => :build`
- `packaging/homebrew/maos.rb:75` — `test do`

## Dockerfile

- `Dockerfile:14` — `FROM rust:1.88-bookworm AS builder`
- `Dockerfile:30` — `FROM gcr.io/distroless/cc-debian12:nonroot`
- `Dockerfile:27` — `RUN cargo build --release -p maos-bin --locked`
- `Dockerfile:40` — `ENTRYPOINT ["/usr/local/bin/maos"]`

## .github/workflows/container.yml

- `.github/workflows/container.yml:40` — `- name: Log in to Docker Hub`
- `.github/workflows/container.yml:57` — `- name: Build and push`
- `.github/workflows/container.yml:74` — `- name: Sign GHCR container image`

## tests/air-gap-netns-corroborate.sh

- `tests/air-gap-netns-corroborate.sh:30` — `echo "SKIP: unshare not found (requires util-linux)"`
- `tests/air-gap-netns-corroborate.sh:35` — `echo "SKIP: strace not found"`
- `tests/air-gap-netns-corroborate.sh:41` — `echo "SKIP: unshare -n requires root or CAP_SYS_ADMIN"`
- `tests/air-gap-netns-corroborate.sh:47` — `echo "SKIP: binary not found at $BINARY (build with --features air-gap first)"`
- `tests/air-gap-netns-corroborate.sh:54` — `strace -e trace=network -o "$STRACE_LOG" \`
- `tests/air-gap-netns-corroborate.sh:74` — `strace -e trace=network -o "$CANARY_LOG" \`
- `tests/air-gap-netns-corroborate.sh:57` — `SYN_COUNT=$(grep -cE '^(connect|socket)\(' "$STRACE_LOG" 2>/dev/null || echo 0)`
- `tests/air-gap-netns-corroborate.sh:84` — `echo "CORROBORATING result: INCONCLUSIVE"`

## docs/runbooks/ag-1-air-gap-deployment.md

- `docs/runbooks/ag-1-air-gap-deployment.md:9` — `zero networking code — no HTTP clients, no TCP listeners, no DNS resolvers.`
- `docs/runbooks/ag-1-air-gap-deployment.md:82` — `- `Scope::NetworkOutbound` tokens are **never issued** (no network surface exists)`
- `docs/runbooks/ag-1-air-gap-deployment.md:89` — `The Transparency Log (SQLite) operates identically in air-gap mode. All frame`
- `docs/runbooks/ag-1-air-gap-deployment.md:113` — `The `xtask check-air-gap` gate (R-AG1) runs in CI and:`

## docs/runbooks/provisioning-checklist.md

- `docs/runbooks/provisioning-checklist.md:24` — `| `DOCKERHUB_TOKEN` | `.github/workflows/container.yml:44` | hard fail: Docker Hub login cannot authenticate | `ops-provisioning-secrets-and-accounts` | `absent` |`
- `docs/runbooks/provisioning-checklist.md:83` — `exactly seven names (`DOCKERHUB_TOKEN`, `DOCKERHUB_USERNAME`,`
- `docs/runbooks/provisioning-checklist.md:150` — `### `DOCKERHUB_USERNAME` + `DOCKERHUB_TOKEN``
- `docs/runbooks/provisioning-checklist.md:25` — `| `DOCKERHUB_USERNAME` | `.github/workflows/container.yml:43,52,80` | hard fail: Docker Hub login/image name cannot resolve | `ops-provisioning-secrets-and-accounts` | `absent` |`
- `docs/runbooks/provisioning-checklist.md:31` — `| `GitHub OIDC for cosign` | `.github/workflows/container.yml:23-26,74-80` | invalid-ref only if workflow `id-token: write` is removed; no operator secret is required | `ops-provisioning-secrets-and-accounts` | `present` |`
- `docs/runbooks/provisioning-checklist.md:33` — `| `lunarpulse/homebrew-maos` | `packaging/homebrew/maos.rb` | hard fail for publication: repository does not exist and formula hashes remain placeholders | `ops-brew-tap-and-aur-publication` | `absent` |`
- `docs/runbooks/provisioning-checklist.md:34` — `| `AUR account and package publication` | `packaging/aur/PKGBUILD` | hard fail for publication: package was not submitted and hashes/version remain scaffold values | `ops-brew-tap-and-aur-publication` | `absent` |`

## docs/adr/ADR-067-maos-data-root-layout-and-purge.md

- `docs/adr/ADR-067-maos-data-root-layout-and-purge.md:95` — ``cargo uninstall maos` for the Cargo-owned executable.`

## docs/maos.dev/run-maos.md

- `docs/maos.dev/run-maos.md:175` — ``cargo uninstall maos`.`

## docs/testing/proof-contract.md

- `docs/testing/proof-contract.md:24` — `## Plants (AC7)`
- `docs/testing/proof-contract.md:33` — `## Review re-execution (AC5)`
- `docs/testing/proof-contract.md:67` — `Review is tiered by risk. A story is **difficult** when it carries a kernel FLAG-Winston grant, a security-critical`

## tests/integration/onb_nfr2_timing.sh

- `tests/integration/onb_nfr2_timing.sh:117` — `strip target/release/maos`
- `tests/integration/onb_nfr2_timing.sh:176` — `max_size=33554432  # 32 MiB — Epic-16 retrospective measured re-baseline (2026-09-20)`

## tests/coverage-matrix.yaml

- `tests/coverage-matrix.yaml:341` — `FR1:`
- `tests/coverage-matrix.yaml:1357` — `NFR-Ops-12:`

## _bmad-output/implementation-artifacts/sprint-status.yaml

- `_bmad-output/implementation-artifacts/sprint-status.yaml:300` — `20-2-deb-airgap-docker-and-formula-corrections: ready-for-dev  # 📝 CREATED 2026-11-02 @a8af3215 (bmad-create-story, anthropic/claude-opus-5-5; lighter measured contract): story 20-2-deb-airgap-docker-and-formula-corrections.md; creation evidence 20-2-deb-airgap-docker-and-formula-corrections-evidence/creation/. D-20-2-01…14 RULED as recommended (2026-11-02, ask s202); D09 A confirmed. Kernel-Δ 0 under the recommended set (D-20-2-01 B priced +38 physical / +32 tokei by reverted prototype); maos-bin −18 and xtask +2 measured, renderer +80…+150 forecast; ruled ceilings maos-bin 33534, xtask 448`
- `_bmad-output/implementation-artifacts/sprint-status.yaml:352` — `ops-rc-signed-tag: backlog  # tag v1.0.0-rc.1 after 20-5 lands and ops-provisioning records the secrets; acceptance: release.yml green with prerelease: true, .deb/.rpm (amd64 + arm64) attached with SHA256SUMS-packages + sig, and the air-gap deployment witness green on the tag SHA (D-20-2-01 A, ask s202; moved out of the Epic-20 exit block, rule 2)`
- `_bmad-output/implementation-artifacts/sprint-status.yaml:342` — `ops-brew-tap-and-aur-publication: backlog  # publish the corrected formula/PKGBUILD (20-2) to the tap repo and AUR. 20-2 attaches rendered maos.rb/PKGBUILD/maos.spec to each release (D-20-2-08 A, ask s202); publication copies those, never the committed templates.`

## _bmad-output/implementation-artifacts/deferred-work.md

- `_bmad-output/implementation-artifacts/deferred-work.md:123` — `| NFR-Ops-12 | Owner: `20-2`; air-gapped substrate and journaling witness charter. 20-2 created 2026-11-02: subject is the shipped default build under `unshare -n` (D-20-2-01 A RULED, ask s202); the air-gap variant's `run` is a stub (W10). Owner unchanged; implementation closure still owed. |`
- `_bmad-output/implementation-artifacts/deferred-work.md:1065` — `- **`maos-bin` does not compile with `--no-default-features` — the air-gap build is broken by Story 16-6** [`crates/maos-bin/src/operator_door.rs:125,576,1454,1464`] — `pub mod admission;` is correctly gated `#[cfg(feature = "network")]` (`crates/maos-bin/src/lib.rs:8-9`) because `admission.rs:129,140,155,608` genuinely need `maos_registry::admission`, which is a `network` dep. But `pub mod operator_door;` (`lib.rs:48`) is **ungated** and reaches `crate::admission` at four sites, so `cargo build -p maos-bin --no-default-features` fails with `E0432: unresolved import crate::admission` + 4 × `E0`
- `_bmad-output/implementation-artifacts/deferred-work.md:1067` — `- **NFR-Onb-2 AC4 binary-size ceiling BREACHED — Epic 16 added 4.15 MiB to the shipped binary** [`tests/integration/onb_nfr2_timing.sh:117-133`] — measured, both ends, stripped, same toolchain, same `--release` profile: **`843d5365` (Epic-15 close) = 26,222,192 bytes (25.01 MiB)**, **`cc3e7089` (HEAD) = 30,574,448 bytes (29.16 MiB)**, limit `max_size = 29360128` (28 MiB, the Story 15-1 re-baseline). Epic 16 spent **+4,352,256 bytes (+16.6 %)** and the binary is now **1,214,320 bytes over**. The job `onb-nfr2-timing` is in `aggregate.needs` (`.github/workflows/discipline.yml:3843+`), so this BL`

## _bmad-output/planning-artifacts/epics/epic-20-ship-it-w5.md

- `_bmad-output/planning-artifacts/epics/epic-20-ship-it-w5.md:47` — `- Line 2 — `release-verify --sign --key-env … --sha256sums … --output …` exists (`xtask/src/main.rs:737-766`; the same invocation as `release.yml:74`); red at HEAD: `dist/SHA256SUMS` absent (line 1). The seed is the documented development key (`crates/maos-audit/src/release_verify.rs:253-262`, in the test module); `RELEASE_PUBKEY` defaults to its pubkey when `MAOS_RELEASE_PUBKEY` is unset at build time (`:21-24,:26-30,:55-58`), and that same pubkey is the one embedded in `packaging/deb/rules:10`, `maos.rb:20`, `PKGBUILD:36`, `rpm/maos.spec:20` — so the dev-signed `./dist` verifies through `mao`
- `_bmad-output/planning-artifacts/epics/epic-20-ship-it-w5.md:58` — `- `ops-provisioning-secrets-and-accounts` — `RELEASE_SIGNING_KEY` + `MAOS_RELEASE_PUBKEY`, tap repo + token, AUR account, `fuzz-ledger`/`rto-ledger` branch creation + push permission (`fuzz-cadence.yml:25-29`, `rpo-rto-cadence.yml:17-18`), **and** `DOCKERHUB_USERNAME`/`DOCKERHUB_TOKEN` + cosign for `container.yml:40-44,:71-80` (20-2 splits build from push); measured at 20-2 creation: the only run (36094027288) failed at Docker Hub login, before any image build (W3).`
- `_bmad-output/planning-artifacts/epics/epic-20-ship-it-w5.md:60` — `- `ops-brew-tap-and-aur-publication` — publish the 20-2-corrected formula/PKGBUILD to the tap repo and AUR; publication copies the rendered `maos.rb`/`PKGBUILD`/`maos.spec` attached by 20-2, never the committed templates (D-20-2-08 A, ask s202).`
- `_bmad-output/planning-artifacts/epics/epic-20-ship-it-w5.md:61` — `- `ops-rc-signed-tag` (NEW row) — `git tag v1.0.0-rc.1 && git push --tags` after 20-5 lands and the row above records both secrets; acceptance: `release.yml` green on the tag with `prerelease: true`, `.deb`/`.rpm` (amd64 + arm64) with `SHA256SUMS-packages` + sig attached, the air-gap deployment witness green on the tag SHA (D-20-2-01 A, ask s202), `container.yml` skipped-or-green.`
- `_bmad-output/planning-artifacts/epics/epic-20-ship-it-w5.md:69` — `**Kloc (asks for the 15-2 table, measured @9f920180 with `cargo run -p xtask -- kloc-check`, exit 1):** maos-bin +100–200 of the lane-wide +3000 (20-1 yank-policy application in `TlYankObserver` +60–120; 20-4 bin +145 measured +24…+54 forecast [INFERENCE], ceiling 33950 (ask s204)) · maos-cli +250–400 (20-1 `spirit install`; 13.4 precedent `cli.rs` +23 / `subcommands.rs` +104 for one verb, this one fetches + admits) — **the ratified maos-cli grant is +1586: 16-1 books +400–700, 18 books +0–120, and 20-1 books +250–400; each story re-measures its draw** · maos-registry +230–440 (server-bin wiri`
- `_bmad-output/planning-artifacts/epics/epic-20-ship-it-w5.md:87` — `| `20-2-deb-airgap-docker-and-formula-corrections` | deb (debhelper), air-gap leg, Docker build/push split, formula corrections | FR1 · NFR-Ops-12 · created 2026-11-02 @a8af3215 — ready-for-dev; D-20-2-01…14 RULED as recommended (ask s202); kernel-Δ 0 (rejected D-01 B priced +38/+32); maos-bin −18, xtask +2 measured + renderer +80…+150 [INFERENCE]; ceilings bin 33534, xtask 44860, aggregate 192316 |`
- `_bmad-output/planning-artifacts/epics/epic-20-ship-it-w5.md:124` — `### 20-2-deb-airgap-docker-and-formula-corrections — deb (debhelper), air-gap leg, Docker build/push split, formula corrections`
- `_bmad-output/planning-artifacts/epics/epic-20-ship-it-w5.md:128` — `⚠ **AMENDED 2026-11-02 (ask s202; 20-2 creation C1–C6):** 20-1a already added registry and publisher to build, staging, manifest and uploads (`release.yml:71-101,155-167`; `xtask/src/release_dry_run.rs:27-44`). `packaging/deb/control:5` includes python3-cryptography; the deb has never been buildable (no changelog; `rules:29` missing separator). Both no-default-feature compile shapes remain acceptance requirements; the old regression was resolved. The 32 MiB bound is `tests/integration/onb_nfr2_timing.sh:176`: published alpha 30,538,368 B stripped; shared-target release build 31,362,992 B strip`
- `_bmad-output/planning-artifacts/epics/epic-20-ship-it-w5.md:130` — `FR1 includes RPM and cargo distribution. Docker build/push separation remains owed (`Dockerfile:27`, `.github/workflows/container.yml:21-80`); the only run (36094027288) failed at Docker Hub login before build/signing (W3). D-20-2-04 B keeps all pushes/signing conditional on `DOCKERHUB_*`; D-05 B uses a runtime-only image from signed release bytes. Recipes retain placeholders/dev-key anchors (`packaging/homebrew/maos.rb:7,20,25-47`, `packaging/aur/PKGBUILD:6,27-36`, `packaging/rpm/maos.spec:4,20`) and `0.5.0` versions (`maos.rb:15`, `PKGBUILD:12`, `maos.spec:9`), while published `v0.1.0-alpha.`
- `_bmad-output/planning-artifacts/epics/epic-20-ship-it-w5.md:233` — `- **AC1 (command).** The discipline `release-dry-run` (15-4) at the story's HEAD contains `.deb` (amd64 + arm64) and the air-gap variant beside the three-target binaries (20-2), `cargo run -p xtask -- stability-matrix --check` (`xtask/src/main.rs:619`, was :595) is green, and the tree is tag-ready: workspace `version = "1.0.0-rc.1"` (`Cargo.toml:67`, `0.1.0-alpha.1` today; was :64 / `0.1.0-alpha`), `Cargo.lock` committed; `STABILITY.md` regenerated — `stability_matrix.rs` gains (i) a `release_channel` row derived from the pre-release suffix of `parse_workspace_version` (`:335-346`: `rc` → "pre`
- `_bmad-output/planning-artifacts/epics/epic-20-ship-it-w5.md:294` — `⚠ **AMENDED 2026-11-02 (ask s2010; D-20-10-09 B):** Starts when picked, without waiting for 20-2; `purge.rs:1042` and the binary-removal sentence stay with 20-2, later lander re-bases the shared docs paragraph. FR2, kernel-Δ 0; maos-bin ceiling 34548, aggregate 192876.`
- `_bmad-output/planning-artifacts/epics/epic-20-ship-it-w5.md:297` — `- **AC2.** Preserve existing Worker workspace removal (`purge.rs:1346-1454`, 19-3), receipts and rootless user-namespace mappings ("idmaps" read as [INFERENCE] `/etc/subuid`, `/etc/subgid`, podman storage/config; purge never edits them); do not duplicate it. Package-owned binary versus runtime-data contract: D09 RULED A (rows201; `20-1a-registry-publish-install-exit-path.md:162`) — binaries are the package manager's (20-2, `purge.rs:1042`), runtime residue is purge's.`
- `_bmad-output/planning-artifacts/epics/epic-20-ship-it-w5.md:349` — `*Why (measured 2026-10-25):* FR64's enterprise gate is per-tenant cost accounting, and the v1.0 cost frame carries neither a task nor an authoritative principal. ADR-046 R3 dropped `task_ref` because both of its sources were absent (`docs/adr/ADR-046-cost-attribution-and-reconciliation.md:38-40`): `InferenceRequest` has no correlation field (`crates/maos-domain/src/ports/inference.rs:30-44`) and the SCB has no current-task marker (`crates/maos-kernel-core/src/scheduler/control_block.rs:258`; only the in-flight ledger `task_assignments_in_flight`, `:278`); the omission is pinned by a negative t`
- `_bmad-output/planning-artifacts/epics/epic-20-ship-it-w5.md:406` — `Epic 19 (the J1 demo is what a cohort installs); specifically 15-3 (`check-exit-commands`; `maos --help/--version` verb table — exit line 3's bare tokens and 20-2 AC2's formula test), 15-4 (`release-dry-run` + the artifact layout the exit stages in `./dist`; 20-1 AC1 adds two binaries to that set), 15-5 (ADR-060 forms by tier, ADR-064 replay selector; Hold-2 engine-off rule in AC3), 15-6 (`MAOS_INFERENCE_MODE` + `MAOS_REPLAY_CASSETTE`), 16-1/16-2 (`maos init` mints `control.json`; the shell binds the door — exit line 9), 16-4 (`maos purge` — 20-2 AC3; ⚠ R8: the token is `purge`, not `uninstall`
- `_bmad-output/planning-artifacts/epics/epic-20-ship-it-w5.md:21` — `⚠ **AMENDED 2026-10-30 (operator ratified D01 A; preflight):** [Epic 20 preflight](../epic-20-preflight-2026-10-30.md) §3 is applied by the dated amendments below; these supersede conflicting historical prose and anchors retained as provenance. **RULED 2026-10-30:** D03 A, D04 A, D08 A (asks `d03`, `d04`, `d08`); D05–D07, D09–D10, D12 and D13 remain open. D02 A ratifies the funding policy below. D11 A directs operator provisioning, not a claim of availability. **RULED 2026-11-01 (asks `rows203c`, `grant203c`):** D05 A; D07 A (decided at 20-9); D10 A; D13 A; 20-3c D-20-3c-01…11 as recommended (`
- `_bmad-output/planning-artifacts/epics/epic-20-ship-it-w5.md:134` — `- **AC3.** D09 A (confirmed ask s202): the package manager removes package-owned binaries (`dpkg -r`, rpm, brew, pacman or the documented cargo uninstall commands); `maos purge` removes MAOS data and owned runtime residue, never `/usr/bin/maos`. Prove both in deb/RPM install/remove legs. Correct the route-neutral binary-removal hint (`purge.rs:823,1042`, D-12 A). `uninstall` remains per-Spirit erasure (`MAOS_ONE_SHOT_MODES` `crates/maos-bin/src/verbs.rs:164,196`; one-shot `main.rs:7360-7364` → cascade `:10454`), not product purge.`
- `_bmad-output/planning-artifacts/epics/epic-20-ship-it-w5.md:92` — `| `20-5-v1-0-rc-tag-and-lts-rule` | `v1.0.0-rc.1` readiness and the LTS rule (the tag = `ops-rc-signed-tag`) | NFR-Maint-6 · created 2026-11-04 @73e7afbc — ready-for-dev; D-20-5-01…16 RULED 2026-11-04 (operator, ask d205): as recommended; grants bind at dev start; kernel-Δ 0 measured for the recommended `[cli_wrapper]` peel (D-20-5-02 A; struct field B +2/+2, kernel check C +19/+16 priced by reverted prototype); measured xtask +213, manifest +26, domain +8, spirit-abi +3, bin +2; ruled ceilings (binding at dev start) xtask 45043, manifest 4656, domain 10123, spirit-abi 1048, bin 34568, aggrega`

## _bmad-output/planning-artifacts/epic-20-preflight-2026-10-30.md

- `_bmad-output/planning-artifacts/epic-20-preflight-2026-10-30.md:115` — `| `check_epic_6_bridge.rs` | 4,592 | **4,590** | **4,023** | Load-bearing deletion first; active story-walker coverage must move, not vanish (governance test citation §3.2). |`
- `_bmad-output/planning-artifacts/epic-20-preflight-2026-10-30.md:187` — `### 3.2 20-2, 20-3b, 20-5 and shared release machinery`
- `_bmad-output/planning-artifacts/epic-20-preflight-2026-10-30.md:320` — `| **Retirement removes live protection** | Ten source modules across expanded callers; 3b must migrate prepush/contract/negative controls, not delete retired CLI names while callers remain. Existing 46 registry ship rows and 20-16 changed topology invalidate old six-site recipe. §3.2; Proof:154-169. |`
- `_bmad-output/planning-artifacts/epic-20-preflight-2026-10-30.md:98` — `| 20-10 | Kernel 0; maos-bin +180…+350 forecast [INFERENCE] | Priced at creation @a8af3215; RULED 2026-11-02 (ask s2010): bin ceiling 34548, aggregate 192876. Package-owned paths remain 20-2 under D09 A; optional kernel shapes rejected. |`
- `_bmad-output/planning-artifacts/epic-20-preflight-2026-10-30.md:136` — `| 20-2 | After 20-1a; real four-binary dry-run/release layout. D09 A confirmed, D-20-2-01…14 ruled (ask s202). | **No; 0** ruled. | deb/RPM builders, Linux netns + strace/canary, container build; arm64 hosted proof; **Docker Hub creds/cosign only push**, tap token/AUR account only publication. Operator publication remains separate. | Own packages/air-gap/Docker split, **RPM and cargo distribution** (R:44). 10 owns runtime T3 residue; `dpkg -r` owns package files, not an unprivileged purge. |`
- `_bmad-output/planning-artifacts/epic-20-preflight-2026-10-30.md:199` — `| 2/10: `maos purge` removes `/usr/bin/maos` / `dpkg -r` same operation | Package owns `/usr/bin/maos` (`/home/lunarpulse/dev_ws/packaging/deb/rules:43-44`); purge tells operator `cargo uninstall maos` (`/home/lunarpulse/dev_ws/crates/maos-bin/src/purge.rs:823,1042`). **Separate package uninstall from runtime data purge** (D09 A confirmed, ask s202); `uninstall` one-shot still per-Spirit erasure at main.rs:7360-7364 → cascade :10454, not kernel binary removal. |`
- `_bmad-output/planning-artifacts/epic-20-preflight-2026-10-30.md:309` — `| **D09** | Packaging/uninstall contract. **A** package manager removes owned binaries, `maos purge` removes MAOS data/owned runtime residue, prove both in deb/RPM legs; **B** extend purge to privileged package binary removal; **C** treat report-only residue as clean uninstall. | **A**, confirmed RULED 2026-11-02 (ask s202) | Avoids privilege escalation/competing package ownership while preserving FR1/2; B needs explicit privilege/rollback design and larger scope; C fails FR2. E20:93; R:24,44; purge.rs:1042,1950-1981. |`
- `_bmad-output/planning-artifacts/epic-20-preflight-2026-10-30.md:64` — `| xtask | 47,682 | 47,682 | **0** | 47,688 / **6** | 20-2 +0–30, 20-5 +20–40, 20-7 +250–450; old 3b retirement is load-bearing. K:339; E19:41. |`
- `_bmad-output/planning-artifacts/epic-20-preflight-2026-10-30.md:90` — `| 20-2 | Creation @a8af3215: bin −18; xtask +2 measured + renderer +80…+150 [INFERENCE] | RULED 2026-11-02 (ask s202): kernel 0; ceilings bin 33534, xtask 44860, aggregate 192316; RPM/cargo and default-build netns substrate/TL witness included. |`
- `_bmad-output/planning-artifacts/epic-20-preflight-2026-10-30.md:144` — `| 20-10 | Starts when picked, not after 20-2 (D-20-10-09 B, ask s2010); epic §20-10 exists and story expands it to AC1–AC7. | **No; 0** ruled. | Capable rootless OCI/cgroup host, owned-container cleanup and refusal twins; no Docker Hub credential needed. | Remove owned T3 containers/cgroups and custody; preserve 19-3 Worker workspace removal and receipts. "idmaps" read as [INFERENCE] rootless mappings (`/etc/subuid`, `/etc/subgid`, podman storage/config), never edited by purge. Binary hint/sentence stay with 20-2. |`
- `_bmad-output/planning-artifacts/epic-20-preflight-2026-10-30.md:156` — `| **3** | **20-2** and **20-3c** after their prerequisites | Packaging consumes real artifact set; audit-truth consumes stable gate inventory. Parallel planning is possible, shared bin/audit edits have one owner. E20:87-93,151. |`
- `_bmad-output/planning-artifacts/epic-20-preflight-2026-10-30.md:157` — `| **4** | **20-8 → 20-9**; **20-10 when picked** (D-20-10-09 B, ask s2010; binary-removal hint/sentence reserved to 20-2); **20-11 after 3c**; **20-13** after registry crypto boundary | One ABI/schema sequence avoids re-bumping WIT independently; purge and erasure close distinct guarantees. 13 before final audit/cost integration avoids hard-wired provider reintroduction. R:22-25; E20:215-223; [INFERENCE: integration order]. |`
- `_bmad-output/planning-artifacts/epic-20-preflight-2026-10-30.md:195` — `| 2: air-gap corroboration :29-48 / “run it” sufficient | `/home/lunarpulse/dev_ws/tests/air-gap-netns-corroborate.sh:29-48` still SKIP→0; :54-55 traces **`--version` and masks exit**; :82-85 canary inconclusive→0. `:54` also lacks `-f`, `:57` counts `AF_UNIX`; the air-gap variant has only a stub `run` (W10). D-20-2-01 A witnesses the shipped default build; R:44 requires substrate activity **and journaling**, fail-closed prerequisites/canary on capable CI. |`
- `_bmad-output/planning-artifacts/epic-20-preflight-2026-10-30.md:196` — `| 2: Dockerfile :27 / container.yml:40-44,71-80 | **Same** build and credential/signing anchors, `/home/lunarpulse/dev_ws/Dockerfile:27`, `/home/lunarpulse/dev_ws/.github/workflows/container.yml:21-80`; no secrets guard at job :21-27. Only run 36094027288 failed at Docker Hub login before any image build (W3); GHCR login uses `GITHUB_TOKEN`, keyless cosign OIDC is present. D-20-2-04 B nevertheless keeps all pushes/signing credential-dependent; build split still owed. |`
- `_bmad-output/planning-artifacts/epic-20-preflight-2026-10-30.md:197` — `| 2: formula/PKGBUILD/RPM placeholders and embedded key cites | Still live at `/home/lunarpulse/dev_ws/packaging/homebrew/maos.rb:7,20,25-47`, `/home/lunarpulse/dev_ws/packaging/aur/PKGBUILD:6,27-36`, `/home/lunarpulse/dev_ws/packaging/rpm/maos.spec:4,20`; recipes also say 0.5.0 (`maos.rb:15`, `PKGBUILD:12`, `maos.spec:9`), while W5 measured the published alpha under the production key. D-20-2-08 A renders per-release values; publication copies rendered attachments. |`

## _bmad-output/planning-artifacts/prd/functional-requirements.md

- `_bmad-output/planning-artifacts/prd/functional-requirements.md:24` — `- **FR1:** User can install MAOS kernel via OS package manager (Homebrew/AUR/deb/rpm), `cargo install`, or signed GitHub Releases binary with mandatory Ed25519 signature verification.`

## _bmad-output/planning-artifacts/prd/non-functional-requirements.md

- `_bmad-output/planning-artifacts/prd/non-functional-requirements.md:167` — `- **NFR-Ops-12** [NEW, Mary]: Air-gapped deployment validation. Substrate boots, runs, produces transparency-log entries with zero outbound network calls; structural test in CI via network-namespace isolation; documented Spirit-author guidance for air-gapped capability tokens. v1.0.`
- `_bmad-output/planning-artifacts/prd/non-functional-requirements.md:219` — `**v1.0 (Team-ready):** HSIS (NFR-Rel-3). CCAC N=600 (NFR-Aud-9). Black-box third-party trial N=12 (NFR-Test-8). Manifest fuzz + wire fuzz (NFR-Sec-5, NFR-Sec-6). External pen-test (NFR-Sec-7). Typed error catalog (NFR-Doc-2). **1-year LTS announcement** (NFR-Maint-6, **2-year deferred to v1.5**). GDPR right-to-be-forgotten (NFR-Aud-10). Cascade erasure receipt (NFR-Aud-12). Time-to-erasure SLA (NFR-Aud-13). Storage cascade completeness (NFR-Aud-12). Cost-attribution accuracy (NFR-Cost-1). Region-pinning primitive (NFR-Comp-4). Spirit model-provenance manifest (NFR-Comp-5). Crypto-module plugga`


## _bmad-output/planning-artifacts/routing-audit-2026-10-22.md — cited ranges re-read

```text
20: | `19-5-cross-form-upgrade-and-cold-swap` | 19-3 | FR10, FR11, FR49 | state/token-preserving upgrade parity for the subprocess form; admission rejects an archived predecessor without a migrator; cold-swap policy ships (`ColdSwapUnsupported` today) | ◆ FLAG-Winston (scheduler) | `16-1…md:43-46,478-486` (→ `:43-46,303-309` @61af7461, 19-5 creation) compiled Butler swap only; `05-fr1-20-scout.md:13` — ⚠ 19-5 creation 2026-10-28 (P1, D-19-5-10 C ruled, ask `w3_19_5_d10`): the `[cli_wrapper]` Worker is not a Spirit form (ADR-060 :27-31); the subprocess-form in-flight swap clause is routed to `20-9-spawned-form-lifecycle-hooks` |
21: | `19-6-crash-originator-delivery-and-disposition` | 19-1 | FR12, FR50, NFR-Rel-1 | crash → mailbox delivery to the task originator with reassignment; all declared crash dispositions applied (production `NullReplicaResolver` replaced) | 0–small | `16-3…md:35-37` journaled, never mailbox-delivered; `main.rs:2795-2796` (→ `crates/maos-bin/src/main.rs:2997-2998` @d9bd59c4, 19-6 creation) |
22: | `20-8-manifest-contract-completion` | 20-1 (→ 20-1a after the D-20-1-01 B split; D-20-8-02 A PROPOSED) | FR8 | manifest declares `skills`, `explanation_shape`, `intent_promotion_set`, `swap_invariants`; signed manifest verified at admission | ◆ FLAG-Winston — priced 2026-11-03 @8c809f9c: KS +38/+33 (KB +10/+8, K0 0), PROPOSED | `05-fr1-20-scout.md:10` four fields missing; unsigned admission (→ 20-8 creation C4: installed packages verified at install and every boot, `crates/maos-registry/src/install.rs:109-150`; no declaration journaled, `crates/maos-bin/src/admission.rs:63-117`) |
23: | `20-9-spawned-form-lifecycle-hooks` | 20-8 (→ 20-8 L1; L2 after 20-8 L2 and 20-11 L2; D-20-9-02 A PROPOSED) | FR55 (+ received FR10 spawned clause, D-19-5-10 C) | budgeted lifecycle triggers (incl. 17-4's idle-quiescent consolidation + live telemetry hooks) reach spawned forms (~~subprocess Worker,~~ WASM — the `[cli_wrapper]` Worker is not a Spirit form, ADR-060); v1.0 third-party Spirits otherwise lack the hook ABI | ◆ FLAG-Winston — priced 2026-11-03 @8c809f9c: watchdog only, R +16/+21 (D07 A suppression + PF-04 `poll_once`), PROPOSED; hooks, WIT 2.4.0, spawned swap/cold swap kernel-free | `17-4…md:51,135` (→ `:53,71,135,341`, 20-9 creation C8) "spawned forms receive neither new hook" (→ 20-9 creation F1: every kernel hook reaches a spawned SCB as a `WorkerSpirit` no-op, `crates/maos-bin/src/supervision.rs:2612-2633`) |
24: | `20-10-purge-residue-removal` | starts when picked (D-20-10-09 B, ask s2010) | FR2 | `maos purge` removes owned T3 containers/cgroups/custody. The 16-4 report keys on `maos-` name (never cleanup authority, `t3/argv.rs:16-21`); `/sys/fs/cgroup/maos` is never created at HEAD, owned leaves are `maos.slice/spirit-*`. | 0, RULED ask s2010 | `purge.rs:1950-1984`; 20-10 creation C6/C7 |
25: | `20-11-erasure-inventory-closure` | 20-3c | FR65 | per-Spirit erasure inventory enumerates and removes intent lineage, pending halts and scheduled invocations (today `intent_lineage` is `VerifiedEmpty` by assumption; `scheduled_invocations` is `CoverageGap`) and the proof covers them | KS +10/+6, RULED (ask s2011) | `crates/maos-bin/src/main.rs:10845-10860` |
26: | `21-7-production-consent-granter-and-gate` | 21-2 | FR29 (production half) | a production granter sets `HeaderOnly`/`Withheld`, a production `ConsentGate` is installed, native senders carry a real pid (not 0); J3/Reza consent is real, not suite-delivered | 0 | `17-8…md:114` (rule-11 label), `:45` |
27: | `21-8-three-protocol-session-witness` | 21-2 | success-criteria.md:50 | one binary-level session: launched by an ACP editor host, pulling tools from an MCP server, peering over A2A — ACP (5-5c), MCP (8-14b/c), A2A (8-6) each exist; the combined witness does not | 0 | no owner in the recovery map |
28: 
29: Totals after ADD: E17 14 · E18 7 · E19 6 · E20 11 · E21 8 (remaining backlog 26 → 35, +35 %). Rule 6 "≤6 stories/epic" is already exceeded in E17 and E20; it is recorded, not re-asserted.
30: 
31: ## 2. WIDEN — ACs to fold at the owner's preflight (routing updated in the inventory now)
32: 
33: | Owner | Clause to add | Evidence |
34: |---|---|---|
35: | `18-1` | FR17 digest floors: first session ≤30 s, all fields, per-Spirit 100-digest floors (19-4 disclaims them: `epic-19:86`); FR58 Butler zero-config reference response (moved from 16-2) | `epic-18:63`; `16-2…md:25-34` |
36: | `18-2` | NFR-Test-4 Researcher and Orchestrator class corpora/gates (Butler only today); FR15 per-class halt floors (moved from 16-2) | `epic-18:77`; `16-2…md:25-26` |
37: | `18-3` | NFR-Aud-7 five thresholds binding per shipping class (faithfulness/hedge "never a CI floor" today); J-Researcher: eight-way tools, contradiction/resume, four output sections, citation/time floors | `epic-18:85-87` |
38: | `18-4` | FR31 principal-namespace persistence of the acceptance ledger ("deferred" today); J-Butler persistent learning, actual option effects, variance-led halt reconciliation | `epic-18:62,93-94` |
39: | `19-1` | FR14 human task envelope (goal/scope/success criteria/posture) with terminal/ACP/push delivery; FR19 per-Spirit per-tag halt-policy schema + CLI consumed live; FR24 whole re-emission provenance | `epic-19:59-62` — machine dispatch only |
40: | `19-2` | FR51(a)(d) bounded director interruption and revocation (16-1 has no interrupt mechanism); FR51(c) already AC3 | `16-1…md:35-42`; `sprint-status.yaml:284` — ✅ absorbed at 19-2 creation 2026-10-28 (D-19-2-01 B): AC7/AC8 |
41: | `19-3` | FR21 fan-out floors (50 concurrent Workers, P99 ≤500 ms, zero drops at 10/s for 1 h, envelope fields); FR25 bridge/persona skills asserted in agent context; FR40 declared-version mismatch refuses startup for real adapters; FR4 1000-call mediation proof | `epic-19:76-78`; `worker_spawn.rs:759-805`; `17-1…md:111-121` |
42: | `19-4` | J1 DAG/independent continuation, skill loading, overnight session; keep the 2026-09-21 re-pin precondition | `epic-19:78,84-88` |
43: | `20-1` | FR36 authenticated control-plane install; FR37 post-promotion lapse/revocation consequences (tracker-only today); FR59 ≤5 min yank sync + vetter-origin propagation; FR13 quarantine isolation | `epic-20:76-79`; `sprint-status.yaml:289`; `17-1…md:143` |
44: | `20-2` | FR1 RPM build/publication + cargo distribution; NFR-Ops-12 air-gap witness runs substrate activity + journaling, not `--version` | `epic-20:84-85`; `tests/air-gap-netns-corroborate.sh:51-55` |
45: | `20-3b` | LIVE-UNVERIFIED witnesses: FR31 namespace isolation, FR34 SDK external-author coverage, FR53 halt identity across swap (test asserts re-firing only, `one_daemon_one_door_16_1.rs:957-961`), FR63 catalog/retryability/cause-chain; I1–I14 invariant ledger published as *guaranteed* (property test in CI) or *asserted* (PRD `success-criteria.md:38` sanctions the downgrade); adversarial-corpus coverage per invariant (`success-criteria.md:39`) | `08-fr21-65-scout.md`; `docs/invariants/README.md:7` |
46: | `20-3c` | FR41 `--tag` filter implemented (`subcommands.rs:2756-2765` ignores `_tag`; `audit_9_1_headline_smoke.sh:181-194` expects refusal); FR62(b) Transparency-Log emitter for ABI proposals/ratifications; NFR-Ops-9 operational RPO/RTO, not collector honesty | `17-3b…md:37,252-253`; `check_rto.rs:23-26` |
47: | `20-4` | FR3 Kimi + per-Spirit provider selection | `epic-20:152-154` |
48: | `20-7` | FR16 posture-shift latency corpus (already `epic-20:190`); the six PRD hot-path budgets (`success-criteria.md:40-46`): issuance <5 µs, dispatch <5 µs, memory/read 50 µs/5 ms, hot-swap 50/500 ms, telemetry <1 µs, iac/send <10 µs | `epic-20:186-193` |
```
```text
116: ## Appendix B — Epic 20 / 21 / operator-lane / parking slice
117: 
118: | Req | PRD clause | Routed home | Verdict | Evidence | Proposed action |
119: |---|---|---|---|---|---|
120: | FR1 | Package-manager, cargo, or signature-verified install | 15-4; 20-2; ops-brew-tap | PARTIAL: RPM/cargo unowned | `epic-20:84-85`; `packaging/rpm/maos.spec:3-6` | widen `20-2` |
121: | FR3 | Six providers per Spirit with endpoint locking | 20-4; later-bedrock | PARTIAL: Kimi, per-Spirit selection | `epic-20:152-154`; `sprint-status.yaml:323` | widen `20-4` |
122: | FR23b | Cross-host PKI, rotation, revocation, skew, partition | 21-3 | PARTIAL: no ±5 min skew, ≤10 s partition, CRL closure | `epic-21:72-77`; `08-fr21-65-scout.md:3` | widen `21-3` |
123: | FR28 | Tiered persistence and quotas | 21-6 | OWNED | `epic-21:105-108` | none |
124: | FR35 | Publish signed schema-conformant packages | 20-1 | OWNED | `epic-20:20,76` | none |
125: | FR36 | Authenticated install verifies signatures, trust floors, claims | 20-1 | PARTIAL: authenticated control-plane install | `epic-20:76,79` | widen `20-1` |
126: | FR37 | Signed, journaled vetting promotion with revocation semantics | 20-1 | PARTIAL: lapse/revocation consequences tracker-only | `epic-20:77-78`; `sprint-status.yaml:289` | widen `20-1` |
127: | FR59 | Yanks synchronize ≤5 min with operator policy | 20-1 | PARTIAL: bounded sync, vetter-origin propagation | `epic-20:78` | widen `20-1` |
128: | FR48 | Operator-selectable crypto across three surfaces | later | PARKED; PRD v0.5 | `requirements-inventory.md:564`; `functional-requirements.md:116` | DECIDE-1 → RULED split: `20-13` (v1.0 mechanism), FIPS module v1.5; ⚠ created 2026-11-03 @8c809f9c, ready-for-dev (`20-13-pluggable-crypto-provider-selection.md`): kernel KT3 +1/−1 (T3 admission), bin +188, shell +46, compliance +22, registry +21, xtask +59 measured; secrets/audit 0; D-20-13-01…16 PROPOSED; also owns D:1379's FR38 runtime-context crypto half (AC4) |
129: | FR54 | Gateway lifecycle and scope contracts | v2.5 | PARKED; parks kernel machinery | `requirements-inventory.md:565`; `functional-requirements.md:68,119` | DECIDE-3 → RULED keep v2.5 + PRD DELTA-2026-10-25; no story |
130: | FR64 | Cost attribution by Spirit/task/principal | later | PARKED; PRD v1.0 | `requirements-inventory.md:570`; `functional-requirements.md:122` | DECIDE-2 → RULED split: `20-14` (v1.0 per-task + per-principal tokens), CPU/storage-I/O v1.1; **Created 2026-11-03 @8c809f9c** (ready-for-dev; D-20-14-01…12 PROPOSED): kernel KT +35/+25 recommended (cumulative 21203 with the 20-8/20-9/20-13 PROPOSED rows ≤ STOP 21209, ×2 ceiling 21228 > STOP → D-20-14-12); measured domain −1, cli +25, audit +29, bin +35; envelope asked kernel 26911/21092, domain 10101, cli 6524, audit 7779, bin 34441, aggregate 192974 |
131: | NFR-Sec-5 | 24 h manifest fuzz | 20-3a; 20-5 | OWNED | `20-3a-gate-honesty.md:92`; rc-tag refusal `20-5-v1-0-rc-tag-and-lts-rule.md` AC4 (created 2026-11-04) | none |
132: | NFR-Ops-9 | Hourly RPO, 4 h RTO, weekly Merkle verify | 20-3a | PARTIAL: operational RPO | `20-3a…md:30,89-91`; `check_rto.rs:23-26` | widen `20-3c` |
133: | NFR-Meta-3 | Generated coverage report blocks uncovered requirements | 20-3b | OWNED | `epic-20:114-115` | none |
134: | NFR-Onb-1 | Twelve external authors meet timing floors | ops-external-cohort | PARTIAL: ≥3 ≠ N=12 | `sprint-status.yaml:310`; `non-functional-requirements.md:125` | widen ops row |
135: | NFR-Test-8 | Twelve externals ship ten signed Spirits | ops-external-cohort | PARTIAL | `non-functional-requirements.md:85` | widen ops row |
136: | NFR-Sec-13 | 3-/10-host rotation loses no conversations | 21-3 | PARTIAL: no scale/percentile acceptance | `epic-21:73`; `14-2d…md:416-430` | widen `21-3` |
137: | NFR-Rel-6 | Restart invalidates TOFU until re-pin | 21-3 | OWNED | `epic-21:72` | none |
138: | NFR-Maint-1 | Trusted core ≤20 KLOC | 21-4 | PARTIAL: instrument, not bound | `epic-21:83-87`; `07-nfr-scout.md:7` | DECIDE-4 → RULED: ADR-069 envelope ≤23,000 |
139: | NFR-Ops-12 | Air-gapped substrate runs and journals | 20-2 | PARTIAL: witness runs `--version` only | `epic-20:84`; `tests/air-gap-netns-corroborate.sh:51-55` | widen `20-2` |
140: | J3 | Eight-host digest and conflict halts | 21-2 | PARTIAL: fixture regeneration | `epic-21:64-65`; `user-journeys.md:205-225` | widen `21-2`; DECIDE-5 |
141: | J4 | Cross-host diagnosis to deployed fix ≤90 min | 21-1 | PARTIAL: same-process; proposal = fix | `epic-21:54,56`; `user-journeys.md:169-195` | widen `21-1`; DECIDE-5 |
142: | Reza | Cross-team proposal, provenance, timed halt, approval | 21-2 | PARTIAL: no live loop | `epic-21:64`; `user-journeys.md:235-252` | widen `21-2`; DECIDE-5 |
```

## docs/adr/ADR-069-kernel-core-envelope-through-v1.md — cited ranges re-read

```text
30: Median ◆ grant ≈ +80, mean ≈ +273. Forecast for the remaining lane: 7 × 273 + ≈ 200 small ≈ **+2,100**
31: → ≈ 22,700.
32: 
33: ## Decision
34: 
35: 1. **Envelope.** Through `v1.0.0` GA, the `maos-kernel-core` production-line ceiling in
36:    `xtask/kloc.toml` may not exceed **23,000**. This replaces NFR-Maint-1's 20,000 letter for that period.
37: 2. **Nothing else is eased.** The row keeps **zero headroom**; every kernel line still needs its own
38:    measured FLAG-Winston grant, cited to an open story, landed with its driver in the same commit.
39:    `xtask/kernel-core-baseline.toml`'s pin (anti-drift, raw `src/` lines + file set + content hash) is
40:    untouched and is never compared with this figure. The envelope is a maximum, not a reservation: no
41:    story may cite it as headroom.
42: 3. **Alarm at 22,000.** The first grant that takes the row above 22,000 must say so in the row's own
43:    HISTORY text (`ENVELOPE ALARM`), and the story's FLAG-Winston decision record must state the remaining
44:    envelope and whether `21-5-orchestrator-port-and-move` (negative, ≈ −1,028 lines) is promoted from
45:    optional to required.
46: 4. **Beyond 23,000** is a new operator decision, not a grant: the gate refuses it.
47: 5. **Review point.** Re-measured at each epic retrospective; re-decided at `v1.0.0` GA, where NFR-Maint-1's
48:    v2.0 letter and the never-landed kernel-crate-set instrument are reconsidered together.
```

## _bmad-output/implementation-artifacts/20-1a-registry-publish-install-exit-path.md — cited ranges re-read

```text
28: - Tracker `sprint-status.yaml:296` D9 routing: a revoked or lapsed attestation on a promoted `public-vetted` Spirit reaches operators as a vetter yank carrying the operator policy; `enforce_vetted_upgrade_precondition` (`crates/maos-bin/src/main.rs:130-202`) stays the only refusal point.
29: - Routing audit `routing-audit-2026-10-22.md:43,124-127,169`; invariant I5's FR13 claim (`docs/invariants/I5.md:11`).
30: 
31: **Does NOT close, and says so:**
32: - The accredited external vetter ecosystem (FR37 v2.5) and an independent author (J6, `routing-audit:143`) — operator lanes; this story proves the mechanism with internal keys (D12).
33: - Packaging of the two new binaries in `.deb`/RPM/formula and the package-owned-path contract (D09 → 20-2).
34: - `maos-spirit validate|inspect` (exit-1 stubs, `maos-spirit.rs:25-28`) — out of scope.
35: - Execution of an installed WASM Spirit from a published binary (Hold 2; 15-5 AC3 statement, 17-3b AC3 refusal); FR8 manifest fields (20-8); crypto-provider selection (20-13, consumes AC6's binding).
36: 
```
```text
155: 
156: Preflight rows that bear on this story stay as filed (`epic-20-preflight-2026-10-30.md:287,288,293,296`); their 20-1 consequence is added.
157: 
158: **Rulings 2026-11-01/02 (operator; asks split201 and rows201).** These supersede the Status column of the table below, which stays as filed at creation.
159: 
160: - D-20-1-01 = **B** (split). This file is `20-1a-registry-publish-install-exit-path` (AC1–AC5, kernel-free, L1). Part B (AC6–AC13) and Tasks T6–T10 are `20-1b-vetting-yank-policy-and-quarantine` (L2 kernel-free, then L3 kernel last). 20-1 is retired (records-owed B1–B5).
161: - D-20-1-02 = **A** (KQ1). D-20-1-03 = **A** (P). D-20-1-04 = **A** (offline CLI install). D-20-1-05 = **A** (`maos_registry::install`). D-20-1-06 = **A** (signed yanks). D-20-1-07 = **B** (server-assigned persistent sequence). D-20-1-08 = **A** (local CRL via `apply_crl`). D-20-1-09 = **A** (first-key pin + storage identity). D-20-1-10 = **A** (daemon re-evaluates attestation on the yank cadence). D-20-1-11 = **A** (envelope ×3 unmeasured / ×2 measured).
162: - D03 = **A** (RULED 2026-10-30). D04 = **A** (RULED 2026-10-30). D09 = **A** and D12 = **A** (operator, rows201: "as already ruled"; the ruling text is not recorded in a repo file).
163: - Kernel row R (FLAG-Winston, granted to 20-1b only): KQ1 + P, measured +78 physical / +68 tokei from HEAD 26801 / 21011; ceilings kernel ≤ 26957 / 21147. 20-7's row R (K1w + K2r; +40 / +31; ceilings 26881 / 21073) is also computed from HEAD 26801 / 21011. If both land, the second to land re-bases: cumulative measured tokei 21110; cumulative tokei ceilings must not exceed 21209 (= 791 under the ADR-069 22000 alarm); STOP otherwise.
164: - Other ceilings (rows201): maos-registry 5163; maos-spirit-cli 2043; maos-cli 6666 (inside the reserved 6856 pin, because install logic lives in maos-registry, D-20-1-05 A); maos-bin 34410; maos-shell 728; maos-compliance 2122; maos-domain 10153; xtask 42920; aggregate 193768.
165: 
166: | ID | Question | Options | Recommendation and evidence | Status |
167: |---|---|---|---|---|
168: | **D03** (preflight) | FR31 two-boot confidentiality/integrity | **A** include the repair (done at 20-3b) | 20-1 consequence: the same-name squatting residual 20-3b left labelled (row 06 A, D:37) closes here by publisher-key binding — AC10 (D-20-1-09). | RULED 2026-10-30: A |
169: | **D04** (preflight) | FR13 quarantine | **A** measure the real isolation path and grant the kernel change · **B** a sufficient existing non-kernel API with a hostile-effect witness · **C** ship drain + marker | RULED A. 20-1 consequence: shape is D-20-1-02; B was priced as asked (prototype `b`, maos-bin +43, kernel 0) and does **not** suffice for FR13's CRL path (F7, F8). | RULED 2026-10-30: A |
170: | **D09** (preflight) | Packaging/uninstall contract | **A** package manager removes owned binaries; `maos purge` removes MAOS data · **B** purge removes package binaries · **C** report-only | **A, at 20-2.** 20-1 consequence: `maos-registry-server` and `maos-spirit` join `release-dry-run` and `release.yml` only; whether the deb/RPM/formula ship them, and who removes them, is 20-2's under D09. No `packaging/` edit here. | PROPOSED (open) |
171: | **D12** (preflight) | rc vs v1.0 external-author claims | **A** rc mechanism-proven; full N before v1.0 claims · **B** full cohort before rc · **C** PRD N ≥ 3 | **A.** 20-1 consequence: the exit's "third-party key" is a fresh random key minted by the job (E20:22) and the vetter is internal: AC text and coverage notes say *mechanism-proven*, never J6 independence or an accredited vetter. | PROPOSED (open) |
172: | **D-20-1-01** | Story shape | **A** one key, three landings: L1 Part A exit path (AC1–AC5, kernel-free) · L2 Part B kernel-free (AC6–AC10) · L3 ◆ FLAG-Winston (AC11, AC12) + AC13 close; `done` after L3 · **B** split: `20-1a-registry-publish-install-exit-path` (AC1–AC5) and `20-1b-vetting-yank-policy-and-quarantine` (AC6–AC13), 20-1 retired, its five ledger rows, I5 owner and the FR13 coverage note re-pointed to 20-1b | **B.** Honest size: +810…+1,686 tokei across 9 crates (≈ 2.2–4.6× 20-3c's landed +365) on two review surfaces with different risk — a product exit path (20-2 waits on its artifact set) and a security-critical trust/revocation surface with a kernel grant (20-13 waits on AC6's binding, PF:141). Split lets 20-1a land and unblock the exit and 20-2 without waiting for a FLAG-Winston review; A keeps one key and one ledger owner at the cost of a three-landing story whose `done` waits on the kernel landing (20-3b/20-3c landed at 2–3× forecast through serial review raises). Under B this file splits mechanically at the Part boundary (records-owed B1–B5). | PROPOSED |
173: | **D-20-1-02** | FR13 quarantine shape (D04 A) | **A** KQ1: a dedicated quarantine fence in `CapabilityRegistryAdapter` (every existing fence site honours it), the CRL Quarantine arm sets it and never terminates, a director pause/resume cannot lift it, unload ends it (+68 / +59) · **B** KQ0: the arm reuses the pause fence (`set_paused`), never terminates (+43 / +40; a `pause`→`resume` lifts the quarantine) · **C** KQ2: A + the quarantined pid's outbound IAC frames refused (+69 / +60; maos-iac +24, maos-domain +2) · **D** non-kernel (D04 B comparison): maos-bin fences yank-policy quarantine only (+43 bin); the CRL arm still terminates | **A.** PRD: quarantine = "running but no new tool calls" (`developer-tool-specific-requirements.md:58`); the fence already covers inference, MCP tool calls, memory, worker spawn and the watchdog (F7), and WASM IAC sends are token-mediated (`maos-iac/src/adapter.rs:326-333`). B is cheaper but a routine resume silently ends a revocation; C adds bus isolation for in-proc senders the PRD does not ask for (+26 out of kernel, IacBusError matches unsearched); D leaves FR13 false and contradicts D04 A. Release: by unload only (no new verb). Worker (T3) Spirits: the fence refuses their next mediated action; in-container activity is bounded by T3 only — measured at the hosted leg (AC11.5) and named in the record if not frozen. | PROPOSED |
174: | **D-20-1-03** | Revocation-poller print (D:1264) | **A** land P in the kernel landing (+10 / +9): first failure, recovery with count · **B** re-dispose the row ownerless (rule 10) out of 20-1 | **A.** Ruled to 20-1 (D-18-0-12 A); the design (B, edge-trigger) is fixed; same module and the same FLAG-Winston review, so it costs no separate re-pin. | PROPOSED |
```
```text
250: | Release / CI | `xtask/src/release_dry_run.rs:27-36`, `.github/workflows/{release.yml:76-90, discipline.yml}` | AC5 |
251: | Examples | `templates/spirit-ts/manifest.toml:9`, `examples/example-spirit-ts/manifest.toml:9` | AC2 |
252: | Docs / records | ADR-056 §1; `docs/invariants/I5.md`; `tests/coverage-matrix.yaml:366-371,617-632,841-846`; `4-kernel-design.md:661`; `deferred-work.md` rows (AC13) | AC6, AC11, AC13 |
253: | Tests (kloc-free) | new `registry_server_bin_20_1`, `publish_live_20_1`, `spirit_install_20_1`, `installed_spirit_dispatch_20_1`, `yank_auth_20_1`, `yank_policy_20_1`, `vetter_lapse_20_1`, `name_pin_20_1`, `quarantine_fr13_20_1`, `revocation_poller_print_20_1` | all |
254: 
255: ### Overlap (serialize; re-measure)
256: 
257: - **`maos-kernel-core` pins** — 20-8, 20-9, 20-11, 20-13, 20-14 forecast kernel touches; one integration owner per pin update (preflight §6). L3 lands after L1/L2.
258: - **`main.rs` composition root** — 20-4 (provider registration) and 20-13 alter it; this story's edits are local to the cited ranges.
259: - **Artifact admission** — 20-8 reuses it (E20:259); 20-13 waits on AC6's crypto boundary (PF:141). AC6's claim shape is the contract both read.
260: - **Release set** — 20-2 consumes AC5's four-binary dist (D09).
261: 
262: ### Current state that must be preserved
263: 
264: - `maosctl import` (offline, `subcommands.rs:1348-1544`) and the release `install --from-local` (`cli.rs:44,427`) unchanged.
265: - The yank vs CRL distinction (`yank.rs:7-10`): under `warn` a yank never terminates; operator CRLs keep their declared actions (TerminateImmediately, DrainThenTerminate unchanged).
```

## crates/maos-kernel-core/src/io/mod.rs — cited ranges re-read

```text
82:         Self::new()
83:     }
84: }
85: 
86: impl IoSubsystemPort for IoSubsystemAdapter {
87:     fn http_get(&self, url: &str) -> Result<Vec<u8>, IoError> {
88:         #[cfg(feature = "io_call_journal")]
89:         journal_record("GET", url);
90: 
91:         let response = ureq::get(url)
92:             .call()
93:             .map_err(|e| IoError::Transport(format!("ureq GET {url}: {e}")))?;
94:         read_capped(response, "GET")
95:     }
96: 
97:     fn http_post(
98:         &self,
99:         url: &str,
100:         body: &[u8],
101:         headers: &[(&str, &str)],
102:     ) -> Result<Vec<u8>, IoError> {
103:         let exchange = self.http_post_exchange(url, body, headers, "")?;
104:         match exchange.status {
105:             200..=299 => Ok(exchange.body),
106:             s => Err(IoError::Transport(format!("ureq POST {url}: status {s}"))),
107:         }
108:     }
109: 
110:     fn http_post_exchange(
111:         &self,
112:         url: &str,
113:         body: &[u8],
114:         headers: &[(&str, &str)],
115:         response_header: &str,
116:     ) -> Result<HttpExchange, IoError> {
117:         #[cfg(feature = "io_call_journal")]
118:         journal_record("POST", url);
119:         let mut req = ureq::post(url);
120:         for (k, v) in headers {
121:             req = req.set(k, v);
122:         }
123:         let response = match req.send_bytes(body) {
124:             Ok(response) | Err(ureq::Error::Status(_, response)) => response,
125:             Err(e) => return Err(IoError::Transport(format!("ureq POST {url}: {e}"))),
```

## xtask/src/release_verify.rs — cited ranges re-read

```text
138:     // Build references for verify_release
139:     let file_refs: Vec<(&str, &[u8])> = files
140:         .iter()
141:         .map(|(name, data)| (name.as_str(), data.as_slice()))
142:         .collect();
143: 
144:     // Run the full verification pipeline in strict mode: every manifest entry
145:     // must be present in the artifacts directory.
146:     let verified = verify_release(&sums_bytes, &sig_array, &RELEASE_PUBKEY, &file_refs, false)
147:         .map_err(|e| format!("release verification failed: {e}"))?;
148: 
```

## crates/maos-domain/src/invariants/i1.rs — cited ranges re-read

```text
60: pub enum Scope {
61:     /// Read access to a filesystem subtree.
62:     FsRead { subtree: String },
63:     /// Write access to a filesystem subtree.
64:     FsWrite { subtree: String },
65:     /// HTTPS network call to a specific domain.
66:     NetHttps { domain: String },
67:     /// Execute a specific binary.
68:     ProcExec { binary: String },
69:     /// Spawn a sub-Spirit of a given class.
70:     SubSpiritSpawn { class: String },
71:     /// Invoke an LLM provider (Story 1b.4).
72:     ProviderInfer { provider: String },
73:     /// Send an IAC frame to a peer class.
74:     IacSend { peer_class: String },
75:     /// Read from a memory scope.
76:     MemRead { scope: String },
77:     /// Write to a memory scope.
78:     MemWrite { scope: String },
79:     /// Story 4.3 — Self-telemetry read (FR56).  Spirit reads its own
80:     /// performance telemetry without per-read operator admission.
81:     SelfTelemetryRead,
82:     /// Story 4.4 — Participant-scoped log recall (emitter-side at v0.3-β).
83:     LogRecall,
84:     /// Story 4.4 — Single-frame payload fetch (lazy-load, A2A consent aware).
85:     LogFetch,
86:     /// Story 4.4 — Distillate write with kernel-enforced I11 audit chain.
87:     DistillateWrite,
88:     /// Story 5.5c — MCP tool invocation. server = manifest [mcp].servers[i].name;
89:     /// tool = the tool identifier exposed by the MCP server.
```

## crates/maos-bin/src/shell_host.rs — cited ranges re-read

```text
104:     fn dispatch_installed(&self, name: &str, message: &str) -> Option<String> {
105:         let (installed, iac) = self.installed.as_ref()?;
106:         installed.dispatch(name, message, iac)
107:     }
108:     fn spirit_pid(&self) -> u32 {
109:         self.spirit_pid
110:     }
111: 
112:     fn issue_turn_token(&self) -> Result<CapabilityToken, CapError> {
113:         self.capability.issue_with_mediation(
114:             self.spirit_pid,
115:             Scope::ProviderInfer {
116:                 provider: self.default_provider.clone(),
117:             },
118:             60,        // ttl_secs
119:             [0u8; 32], // posture_hash (deterministic fallback)
120:             IntentClass::Standard,
121:         )
122:     }
123: 
124:     fn record_turn(&self, token: &CapabilityToken, payload: &[u8]) -> Result<(), CapError> {
```
