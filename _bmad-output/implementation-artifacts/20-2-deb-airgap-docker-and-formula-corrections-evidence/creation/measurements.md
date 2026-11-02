# 20-2 creation — measurements

## Instrument

Prebuilt `/mnt/build/cargo-maos/debug/xtask` (`$CARGO_TARGET_DIR`, inherited; no cargo build ran), invoked as
`env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=<copy>/xtask <xtask> {check-kernel-baseline,kloc-check} --json` from a
`git archive a8af3215` copy per prototype under `/tmp/proto202/` (deleted after measurement). Prototypes were
formatted with `rustfmt 1.9.0` (edition 2021, default config), never applied to the tree, **never compiled**.
Raw figures: `kloc-kernel-a8af3215-and-prototypes.json`. Diffs: `*.diff` in this directory.

## Baseline (untouched copy)

`check-kernel-baseline` passed: **26841** physical, 95 files, set hash `db39bc4a…7b59`; `kloc-check` passed: kernel
**21042** tokei, aggregate **191826**, `alarm = true`, `over_budget = []`; maos-bin 33498, xtask 44406, maos-cli 6429,
maos-loom-lite 5277, maos-shell 615. Matches the pins (`xtask/kernel-core-baseline.toml`, `xtask/kloc.toml:264,339,489,517,691,718`; hardfail 191827).

## Kernel prototype (only D-20-2-01 B touches the kernel)

| Shape | Files | Physical | Tokei | Diff |
|---|---|---:|---:|---|
| **KA** — `maos-kernel-core` gains a default-on `http` feature owning `ureq` (`Cargo.toml:40` → optional); `io/mod.rs` gates `read_capped`, `std::io::Read` and the `ureq` `IoSubsystemPort` impl on it, and adds a `not(http)` impl returning a typed `IoError::Transport("… HTTP is compiled out of this build")` for `http_get`/`http_post`/`http_post_exchange` | `src/io/mod.rs` (+ `Cargo.toml`, outside the pinned `src` set) | **+38 (26879)** | **+32 (21074)** | `ka-kernel-core-http-feature.diff` |

`check-kernel-baseline` on the KA copy: `changed = ["io/mod.rs"]`; `kloc-check`: `over_budget = ["aggregate 191858 >= 191827",
"maos-kernel-core 21074 > 21042"]`. ADR-069 (`docs/adr/ADR-069-kernel-core-envelope-through-v1.md:35-46`): 21074 is
926 under the 22000 alarm; cumulative with 20-1b's pending row R (+68 tokei, re-based on 20-7 → 21110 expected) the
two would measure **21142**, 858 under the alarm; no ENVELOPE ALARM row.

KA alone does **not** make the variant network-free: every crate in the closure that depends on
`maos-kernel-core` with default features (maos-shell, maos-control, maos-bench, maos-audit, maos-secrets, … —
13 crates + 7 Spirits, `grep -l 'maos-kernel-core = ' {crates,spirits}/*/Cargo.toml`) re-enables `http` by feature unification unless
each declares `default-features = false` and forwards it; `tokio-postgres` (maos-loom-lite, maos-cli), tokio `net`
and a substrate boot path for the variant are further work (forecast below, not prototyped).

## Non-kernel prototypes (measured)

| Shape | D-row | maos-bin | xtask | aggregate | Diff |
|---|---|---:|---:|---:|---|
| air-gap `run` stub → typed refusal (exit 2) **and** route-neutral purge hint | D-20-2-01 A + D-20-2-12 A | **−18** (33480) | 0 | 191808 | `binA-air-gap-run-refusal.diff`, `binA-purge-route-neutral-hint.diff` |
| purge hint only (`const BINARY_REMOVAL_HINT`, used at `purge.rs:823` and `:1042`) | D-20-2-12 A | **+2** (33500) | 0 | 191828 | `binA-purge-route-neutral-hint.diff` |
| `EXPECTED_GATES` gains `check-air-gap` and `air-gap-netns-witness` | D-20-2-02 A, D-20-2-03 | 0 | **+2** (44408) | 191828 | `xt-ship-gate-enrolment.diff` |

Kernel unchanged in all three (`check-kernel-baseline` rc 0 on the binA copy).

## Non-kernel forecasts (not measured — [INFERENCE] from analogues)

| Crate · component | D-row | Forecast (tokei) | Analogue |
|---|---|---:|---|
| xtask · recipe renderer (version mapping per format, sha256 from `SHA256SUMS`, pubkey from `maosctl release-pubkey`, token-survival check) | D-20-2-08 A | +80…+150 | `release_dry_run.rs::generate_manifest` (`:113-161`) |
| xtask · air-gap artifact variant in `ARTIFACTS` (features + distinct name) | D-20-2-01 B | +20…+40 | `release_dry_run.rs:27-44,163-209` |
| maos-bin · route detection from `current_exe()` (dpkg/rpm/pacman `/usr/bin`, Homebrew Cellar, `~/.cargo/bin`) | D-20-2-12 B | +30…+50 | `purge.rs:816-818` already reads `current_exe` |
| maos-bin · air-gap substrate boot (kernel + in-proc Spirit + Transparency Log without network crates) | D-20-2-01 B | +300…+800 (wide: the network boot core it would mirror is thousands of lines) | `main.rs:1855-…` network boot |
| maos-cli / maos-loom-lite · gate `tokio-postgres` consumers out of the variant | D-20-2-01 B | +40…+150 | `maos-cli/Cargo.toml:37`, `maos-loom-lite/Cargo.toml:17-18` |
| scripts, workflows, Dockerfile, `packaging/`, docs, tests | all | uncharged | `xtask/src/kloc_check.rs` counts crate/xtask `src` only |

## Not measured here (owed at T0; a figure above the ruled envelope, or a result contradicting a ruled option, is STOP + D-row)

1. Air-gap build + `check-air-gap --dirty-fixture <default build>` at HEAD: which `NETWORK_SYMBOLS` (`check_air_gap.rs:12-27`) the `--no-default-features` binary links, given W9's closure. Decides D-20-2-02.
2. Hosted `ubuntu-24.04`: `sudo unshare -n` + `setpriv --reuid/--regid` + `strace -f -e trace=network` work; whether unprivileged user namespaces are blocked (AppArmor `kernel.apparmor_restrict_unprivileged_userns`). Local host cannot create a netns (W11).
3. Default build under `unshare -n`: does `maos init` + a replay session issue any `AF_INET`/`AF_INET6` `connect`/`sendto`/`sendmsg`? If yes, that is an NFR-Ops-12 product defect: STOP + D-row, never an allowlist. Which knob makes a deterministic outbound attempt for AC3's plant (`MAOS_INFERENCE_MODE=live` with a dummy key, or an OTLP endpoint).
4. `dpkg-buildpackage` on `ubuntu-24.04` (debhelper install; `python3-cryptography` presence; `dpkg-checkbuilddeps` on `cargo, rustc`); arm64: native `ubuntu-24.04-arm` for this public repository vs cross `dpkg-shlibdeps` (needs `libc6:arm64`).
5. Homebrew on the `ubuntu-24.04` image; `brew install` from a local tap with `file://` URLs; a valid formula name for the Python verifier (`python@3`, `maos.rb:50`, is believed not to be one [INFERENCE]); `brew audit --strict`.
6. Fedora / Arch container pulls on hosted runners; `rpmbuild -bb` from apt `rpm`; `makepkg` as non-root; `rpm`'s `brp-strip` and makepkg `strip` disabled → installed sha256 = signed.
7. Version comparators: `dpkg --compare-versions 1.0.0~rc.1-1 lt 1.0.0-1`, `rpmdev-vercmp 1.0.0~rc.1 1.0.0`, `vercmp 1.0.0rc1 1.0.0` (and the `0.1.0-alpha.1` dry-run mapping).
8. `docker build` on the hosted runner: the release binary (GLIBC_2.39, W6) runs on `gcr.io/distroless/cc-debian13:nonroot`; image size.
9. `cargo install --locked --path crates/maos-bin --root <tmp>` wall-clock on hosted.
10. Final-tree stripped size on `onb-nfr2-timing` (W8 is provenance-labelled only).
11. CI job-minutes added (deb ×2, rpm, Arch, brew, container, netns, cargo-install) against 20-12's test-cost contract.
