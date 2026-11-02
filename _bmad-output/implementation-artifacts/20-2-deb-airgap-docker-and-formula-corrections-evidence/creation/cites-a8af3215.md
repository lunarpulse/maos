# 20-2 creation — cites re-measured at `a8af3215` (rule 9)

Every range below was re-read at HEAD `a8af3215` (`git log -1 -- crates` = `47631284`; the tree was clean). "E20" =
`_bmad-output/planning-artifacts/epics/epic-20-ship-it-w5.md`; "S" = `sprint-status.yaml`; "PF" =
`epic-20-preflight-2026-10-30.md`; "D" = `deferred-work.md`; "R" = `routing-audit-2026-10-22.md`; "W" = rows of
`witnesses-at-head.md`. The shared files (E20, D, PF, R) are NOT edited by this creation: the epic corrections are
owed through `records-owed.md` §A. The story file cites the NEW values.

## C — epic §20-2 (E20:110-120) and its stories row (E20:73)

| # | Where | OLD | NEW at `a8af3215` |
|---|---|---|---|
| C1 | E20:114 | `release.yml:71-91` builds maos-bin + maos-cli; "20-1 adds registry and publisher" | done by 20-1a: `release.yml:71-76` builds `-p maos-bin -p maos-cli -p maos-registry -p maos-spirit-cli`; `:79-85` stages four, `:86-101` uploads four, `:155-167` publishes 12 binaries + `SHA256SUMS` + `.sig`; `xtask/src/release_dry_run.rs:27-44` `ARTIFACTS` = four. No 20-2 action |
| C2 | E20:114 | deb `rules:10,13-22,24-56` "source anchors, not package proof" | lines same; **measured: the package has never been buildable** — `rules:29` *missing separator* (GNU make rc 2, W2) and no `debian/changelog` (dpkg-buildpackage rc 25, W1) |
| C3 | E20:114 | "current 32 MiB prepush bound is `docs/testing/proof-contract.md:170-176`" | proof-contract carries no size bound at HEAD (grep `32 MiB\|33554432\|binary size`: 0); the bound is `tests/integration/onb_nfr2_timing.sh:176` (`max_size=33554432`), strip/stat `:115-117`, job `discipline.yml:1532-1544` `onb-nfr2-timing` (in `aggregate.needs` `:6569`) |
| C4 | E20:114 | "current binary size unmeasured" | provenance-labelled measurement W8: 31,362,992 B stripped (shared-target release build, tree unknown); published alpha 30,538,368 B stripped |
| C5 | E20:114 | `check-air-gap` CLI `xtask/src/main.rs:869-882`, dispatch `:1479` onward | doc `:873`, `CheckAirGap` `:874-887`, dispatch `:1501-1506`, `mod` `:12`; gate `xtask/src/check_air_gap.rs` (`NETWORK_SYMBOLS` `:12-27`, `run` `:46-…`, `cargo build … --no-default-features --features air-gap` `:52-67`) |
| C6 | E20:114 | netns script `:29-48,54-55,82-85` can skip or mask | same lines (SKIP→`exit 0` at `:29-32`, `:34-37`, `:40-43`, `:46-49`; `--version` + `\|\| true` `:54-55`; inconclusive canary→0 `:82-85`) **plus** `:54` traces without `-f` (a multi-threaded binary's other threads are not traced) and `:57` counts every `socket(` incl. `AF_UNIX` |
| C7 | E20:116 | `Dockerfile:27`, `container.yml:21-80` | same; measured W3: the only `container.yml` run failed at Docker Hub login `:40-44` (GHCR login `:33-38` succeeded), build `:57-66` and cosign `:71-80` skipped — no image was ever built in CI; no `.dockerignore`; builder `rust:1.88-bookworm` (`Dockerfile:14`) vs `rust-toolchain.toml` `channel = "stable"` (not copied into the context, `:19-24`) |
| C8 | E20:116 | placeholders `maos.rb:7,20,25-47`, `PKGBUILD:6,27-36`, `maos.spec:4,20` | same, **plus** `version/pkgver/Version 0.5.0` (`maos.rb:15`, `PKGBUILD:12`, `maos.spec:9`) — no such release (W13); embedded key is the dev key, the published release verifies only under the production key (W5) |
| C9 | E20:116 | "Network `--version` source-resolved `main.rs:1858-1866`, air-gap `:1520-1524`" | network `main` `crates/maos-bin/src/main.rs:1855-1857`, `verbs::dispatch` `:1881`, `Version` arm `:1887-1891`; air-gap `main` `:1518-1558`, `Version` arm `:1532-1535` |
| C10 | E20:116 | "Per-Spirit uninstall remains `main.rs:7173-7211`" | one-shot `uninstall` arm `main.rs:7360-7364` → `run_uninstall_cascade` `:10454` |
| C11 | E20:118 (AC1) | `release.yml:46` (check-mock); `check_mock_not_in_release.rs:29-31`; air-gap main `main.rs:1318-1342` "prints `maos 0.1.0-alpha`"; `check_air_gap.rs:12-27,:46-60`; xtask `main.rs:767-768,:1342-1347`; `release_verify.rs:26-30` | `release.yml:77-78`; `check_mock_not_in_release.rs:28` (symbols), `:30` (`run`); air-gap main `:1518-1558`, version `0.1.0-alpha.1` (`Cargo.toml:67`); `check_air_gap.rs` as C5; xtask as C5; `DEFAULT_RELEASE_PUBKEY` `release_verify.rs:25-28`, `RELEASE_PUBKEY` `:56-59`, dev seed `:253-254`, signature convention `:150-168` |
| C12 | E20:118 (AC1) | "`control:5` needs `python3-cryptography`" | present at `control:5`; the same line also requires `cargo, rustc (>= 1.88)`, which a pre-built-binary package does not use and a rustup-provisioned runner does not satisfy through dpkg [INFERENCE; T0 measurement 4] |
| C13 | E20:118 (AC1) | air-gap "`--no-default-features --features air-gap` … compiles at HEAD" | anchors same (`crates/maos-bin/Cargo.toml:15` `default = ["network"]`, `:29` `air-gap = []` — an empty feature: the variant is `--no-default-features`); its closure still links `ureq`, `tokio-postgres`, `rustls`, tokio `net` (W9); its `run` is a stub (W10) |
| C14 | E20:118 (AC1) | release-dry-run "the same step runs in the discipline `release-dry-run` job against the dev-signed `./dist`" | job `discipline.yml:5821-5880`: dry-run `:5846-5847`, dev-seed sign `:5848-5851`, maosctl verify `:5852-5856` and forged-signature control `:5857-5874` (amd64 only, `:5853,:5858`), upload `:5875-5880` |
| C15 | E20:119 (AC2) | "today only the air-gap main has `--version`, `main.rs:1332`" | false: both mains answer via `verbs::dispatch` (C9); the formula `test` (`maos.rb:75-77`) is answerable by the network build [source-resolved; runtime on a packaged binary owed by AC2] |
| C16 | E20:120 (AC3) | `MAOS_ONE_SHOT_MODES` `verbs.rs:182`, cascade `main.rs:8883`; purge "removes what the `.deb`/formula installed (`/usr/bin/maos` per `rules:44`); asserted … with `dpkg -r`" | `verbs.rs:164` (const), `"uninstall"` `:196`; cascade C10. Contract superseded by D09 A (as recorded `20-1a-…md:162`, rows201): the package manager removes package-owned binaries, `maos purge` removes MAOS data; the purge hint `purge.rs:823,1042` is wrong for every route (W7) |
| C17 | E20:73 (stories row) | "kernel-Δ 0 · xtask +0–30 · CI/packaging only" | kernel-Δ 0 under the recommended D-row set (D-20-2-01 B priced **+38 / +32** by reverted prototype); maos-bin **−18** measured (`binA`); xtask **+2** measured + renderer +80…+150 [INFERENCE]; touches maos-bin `src/` (not "CI/packaging only") |
| C18 | E20:44, :47; S:341 | `ops-rc-signed-tag` acceptance: "`.deb` (amd64 + arm64) and the air-gap variant attached" | under D-20-2-01 A no air-gap artifact ships: "`.deb`/`.rpm` (amd64 + arm64) attached with their signed package manifest, the air-gap deployment witness green on the tag SHA"; unchanged under D-01 B |
| C19 | E20:7 | "Still open: D06, D09, D12" (2026-11-01) | `20-1a-…md:162` records "D09 = **A** and D12 = **A** (operator, rows201: 'as already ruled'; the ruling text is not recorded in a repo file)". The two records disagree; this story applies D09 A and asks the operator to confirm (D-row table) |

## T — tracker S:298

| # | OLD | NEW |
|---|---|---|
| T1 | ".deb via dpkg-deb" | epic AC1 says `dpkg-buildpackage -us -uc -b` (debhelper) |
| T2 | "the air-gap matrix leg MUST include a `cargo build -p maos-bin --no-default-features` compile step … BROKEN at HEAD" | RESOLVED 2026-09-21 (D:1040); the compile shape is exercised by `discipline.yml:557-560` (`cargo test -p maos-bin --no-default-features --test verb_table_15_3`) |
| T3 | "NFR-Onb-2 AC4 binary-size ceiling is BREACHED … BLOCKS THE MERGE" | RESOLVED 2026-09-21 by the measured 32 MiB re-base (`onb_nfr2_timing.sh:176`); D:1042 still names 20-2 as owner (closure owed, AC8) |

## P — preflight / routing audit / docs

| # | Where | OLD | NEW |
|---|---|---|---|
| P1 | PF:179 | netns "requires … substrate activity and journaling" | carried; adds the two W-measured defects (`-f`, `AF_UNIX`) and W10 (no air-gap substrate to witness) |
| P2 | PF:180 | `Dockerfile:27`, `container.yml:21-80`, "no secrets guard at job `:21-27`" | same; W3 adds that GHCR needs no operator secret (`GITHUB_TOKEN`, measured) and cosign keyless needs only `id-token: write` (`container.yml:26`; `docs/runbooks/provisioning-checklist.md:31`) |
| P3 | PF:183 | `purge.rs:1042` | same; plus the receipt field `purge.rs:823` |
| P4 | R:44, R:120, R:139 | `tests/air-gap-netns-corroborate.sh:51-55`; `packaging/rpm/maos.spec:3-6` | same lines |
| P5 | `docs/runbooks/ag-1-air-gap-deployment.md` | — | false at HEAD: `:7-9` "zero networking code — no HTTP clients" (W9); `:17` and `:111-118` "`xtask check-air-gap` … runs in CI" (W12); `:89-91` TL "operates identically in air-gap mode" (W10) |
| P6 | `docs/adr/ADR-067-maos-data-root-layout-and-purge.md:84-85`; `docs/maos.dev/run-maos.md:111-112` | "`cargo uninstall maos`" | rc 101 (W7) |
| P7 | `tests/coverage-matrix.yaml:341-349` (FR1) | gates `reproducible-build`; note "`cargo install --path crates/maos-cli`; full install workflow lands at v0.5+" | stale: no deb/rpm/recipe/cargo witness named |
| P8 | `tests/coverage-matrix.yaml:1318-1323` (NFR-Ops-12) | `gates: []`, `advisory-until-engagement` | unchanged at HEAD (AC8 names the witness) |
| P9 | `docs/runbooks/provisioning-checklist.md:24-25,31,33-34` | DOCKERHUB_* absent; cosign OIDC present; tap/AUR absent | same (W4) |
