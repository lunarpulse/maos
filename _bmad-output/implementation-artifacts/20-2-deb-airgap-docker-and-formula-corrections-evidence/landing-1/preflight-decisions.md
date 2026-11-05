# 20-2 landing 1 — preflight and ruled re-base

Baseline: `cbed4dad`. No commit, push, tracker change, `.devcontainer` edit or
20-11 protected-code edit was performed. `cites-cbed4dad.md` remeasures 40 cited
baseline source/record files, superseding creation offsets without rewriting
historical creation observations.

## D02 A arithmetic (ask s202, D-20-2-13 A)

| Surface | Exact baseline | Allowance retained from ruled scope | Rebased STOP ceiling | Formatted final measurement / pin |
|---|---:|---:|---:|---:|
| maos-bin | 34,842 | 2 × 18 = 36 measured | 34,878 | **34,823** (−19) |
| xtask | 44,466 | 2 × 2 + 3 × 150 = 454 | 44,920 | **44,673** (+207) |
| aggregate | 193,911 | 36 + 454 = 490 | 194,401 | **194,099** (+188); hardfail **194,100** |
| kernel physical / tokei | 26,841 / 21,042 | 0 | unchanged | **26,841 / 21,042** |

The old creation ceilings 33,534 / 44,860 / 192,316 were calculated against
33,498 / 44,406 / 191,826 and cannot fund this landing without that re-base.
The formatted implementation fits the newly rebased ruled ceilings. Every
other crate measurement is unchanged, including iac 8,551, audit 7,785,
registry 4,322 and domain 10,107. Pins were replaced exactly, with no borrowed
headroom or speculative reservations. xtask's first HISTORY segment cites the
live story key with `44466 -> 44673`; aggregate is measured + 1.

## T0 observations and disposition

1. **Measured RED → already-ruled D-20-2-02 C.** `t0-default-build.log` builds a
   genuine default maos; `t0-airgap.log` runs `check-air-gap --build-first
   --binary ... --dirty-fixture <saved default maos> --json`, exit 1. The subject
   contains `TcpListener` and `getaddrinfo`; the default dirty fixture is
   correctly rejected. Full symbol lines, not a cleaned subset, are in that
   log. Enroll the symbol job advisory only. The behavioral netns job is blocking.
2. `unshare -n true` exits 1, `Operation not permitted` (`t0-netns-prerequisite.log`).
   strace is absent. No local network-isolation or zero-outbound result exists.
3. The actual final release workload (without netns/strace) performs init,
   hello-spirit replay, expected response and real TL growth, exit 0
   (`ac3-local-workload-only.log`). This is workload validation **only**, not an
   air-gap result. Hosted outbound traffic is still STOP, never allowlisted.
4. dpkg-buildpackage exists; debhelper does not. Native amd64/arm64 package build,
   install, identity, purge and remove are authored in discipline and the verify
   recipe; **unverified** locally. Rendered make rules parse (exit 0), unlike the
   historical malformed heredoc. No local install/remove is claimed.
5. Homebrew is absent. Actual formula metadata confirms `cryptography` is a
   real formula (old name `python-cryptography`) and `python@3.14` is supported:
   <https://formulae.brew.sh/api/formula/cryptography.json>. Formula API documents
   targeted `skip_clean`; `:all` is deprecated and no longer matches paths:
   <https://docs.brew.sh/rubydoc/Formula.html#skip_clean-class_method>.
   Linux source-install linkage fixing only relativizes absolute symlinks:
   <https://docs.brew.sh/rubydoc/OS/Linux/FormulaInstaller.html>.
   The recipe preserves each signed bin explicitly; hosted brew install/audit
   and installed-byte comparison remain **unverified**.
6. Docker/rpmbuild/makepkg are absent; no docker socket or user systemd bus.
   Real Fedora and non-root Arch recipe lanes are authored, **unverified**.
7. dpkg rc prerelease ordering is green locally (`version-comparator.log`).
   Rust vectors cover both alpha and rc mappings. RPM/Arch own-comparator checks
   for both versions are authored in their hosted lanes, **unverified**.
8. Docker verify-stage mutation, distroless Debian 13 smoke and image size are
   authored, **unverified**. No image was built locally.
9. Genuine temporary-root Cargo install/uninstall and wall-clock capture are
   authored, **unverified**. Source-route documentation is present.
10. Final maos stripped size **31,677,432 B**, limit **33,554,432 B**. The original
    onb-nfr2 script's size leg (lines 115–181) ran unchanged except its executable
    path redirected to a copy to preserve signed bytes; exit 0. The whole script
    was not run because it invokes forbidden `cargo clean`.
11. Added hosted job-minutes cannot be measured without hosted execution. The
    branch recipe records real wall-clock logs; no forecast is called measured.

## Operator-only proof vehicle

`../ci/ci-20-2-verify.yml` is a recipe, not an enabled main workflow. The operator
copies it into `.github/workflows/ci-20-2-verify.yml` **only** on branch
`ci/20-2-verify`, pushes that branch, and records its Actions run ids/artifacts.
Never merge that verification workflow onto main. It supplies native release
builds, baseline and wrong-key deb scenes, netns baseline/outbound/final scenes,
image byte-flip/restored smoke, real RPM/Arch/Homebrew execution and Cargo/symbol
scenes. Plants restore bytes with copy-back + cmp + touch; the netns injection
changes only the process tree and is absent from the final scene.

Its explicit `workflow_dispatch.publish_tag` input additionally enables the
operator's real Docker Hub/GHCR push + keyless sign + verify lane for an existing
signed release. An ordinary branch push never publishes. Missing Docker Hub
credentials print the required explicit skip; no publication result is inferred.
The separate `release-rehearsal` label path remains available for a real pre-tag
three-target compile; no label, dispatch, tag or hosted run was issued here.

NFR-Ops-12's outstanding hosted/tag-SHA proof is re-owned to existing
`ops-rc-signed-tag` with this exact environment reason under rule 10. The historic
compile-shape and size rows are closed against the explicitly **uncommitted**
landing-1 tree based on cbed4dad, not falsely attributed to baseline code.
