---
story_id: '20.2'
story_key: 20-2-deb-airgap-docker-and-formula-corrections
created: 2026-11-02
context_commit: cbed4dad
baseline_commit: cbed4dad
security_sensitive: true
kernel_grant: "RULED 2026-11-02 (ask s202): kernel-Δ 0; re-measured cbed4dad at 26841 physical / 21042 tokei. No kernel grant or pin change."
kloc_grant: "RULED ask s202, D-20-2-13 A / D02 A ×2 measured, ×3 unmeasured. Review rebase at 88c7277c: carry only unrelated drift since the initial 20-2 measured pins (+91 bin, +641 aggregate, xtask 0), not a renewed allowance. Ceilings maos-bin 34969, xtask 44920, aggregate 195042; hardfail ceiling195043. Exact review pins: bin34914 unchanged; xtask 44673 -> 44676; aggregate194740 -> 194743; hardfail194741 -> 194744. Kernel26841 physical/21042 tokei unchanged; other crates unchanged by 20-2 corrections."
review: "bmad-code-review under docs/testing/proof-contract.md. DIFFICULT tier (`proof-contract.md:63-71`): a security-critical control — Ed25519 release-signature verification in four package recipes and the container signing path (supply chain) — and a privileged CI job (`sudo unshare -n`). Blind and Edge on opus-5.5, Acceptance on opus-5.5, a Security reading on AC1, AC2, AC3 and AC5, Test Infrastructure on glm-5.3 (standing policy). Under D-20-2-01 B add the FLAG-Winston kernel reading. Creation anthropic/claude-opus-5-5."
---

# Story 20.2: deb, air-gap leg, Docker build/push split, formula corrections

Status: ready-for-dev

<!-- Created 2026-11-02 at a8af3215 (main; origin 47631284; `git log -1 -- crates` = 47631284, 20-1a's hosted-found fixes) by bmad-create-story under the lighter measured creation contract. Operator rulings in force: D01 A, D02 A, D11 A (2026-10-30); D09 A as recorded at `20-1a-…md:162` (rows201) — E20:7 still lists D09 open, confirmation asked below. D-20-2-01…14 are PROPOSED, not ruled — the operator rules; a different ruling rewrites only the clauses marked with that row. No cargo build ran: cites re-read at HEAD (`creation/cites-a8af3215.md`); kernel and kloc by the prebuilt gate binary on `git archive` copies (`creation/measurements.md`); failing-before witnesses measured on the dev host and read from GitHub (`creation/witnesses-at-head.md`). Stale epic cites are corrected here and owed to E20 (`creation/records-owed.md` §A). -->

**Status rationale — ready-for-dev, not blocked.** Every acceptance leg is hermetic against the dev-signed
`release-dry-run` dist (`discipline.yml:5846-5851`); no operator credential is needed to build, verify, install or
remove a package, to build the image or to run the netns witness. Credentials gate **publication** only: Docker Hub
(`DOCKERHUB_*`, absent — W4) and the tap/AUR accounts (`ops-provisioning-secrets-and-accounts`,
`ops-brew-tap-and-aur-publication`). Dev start: after the operator rules D-20-2-01…14 and after 20-1a closes its
landing (20-1a owns the edits to `release.yml` and `xtask/src/release_dry_run.rs` this story builds on; PF:120,140).

> **The capability:** *an operator installs MAOS through the package manager they already use — deb, rpm, Homebrew, AUR or cargo — and every route verifies the release signature before anything is installed, installs exactly the signed bytes, and removes them again; an air-gapped deployment is proven to boot, run and journal with zero outbound connections by a CI job that cannot pass by skipping; and the container image is built and checked on every change, while pushing it waits for the credentials that only the operator provisions.*

**Closes:**
- Epic-20 §20-2 (E20@HEAD:110-120) as amended 2026-10-30 (D01 A; PF §3.2): epic AC1 → AC1, AC3, AC4, AC5; epic AC2 → AC2; epic AC3 → AC6 (rewritten by D09 A).
- Routing-audit widening R:44 (also R:120, R:139): FR1 RPM build/publication + cargo distribution → AC2, AC7; NFR-Ops-12 witness runs substrate activity + journaling, not `--version` → AC3.
- `deferred-work.md:98` (NFR-Ops-12, owner `20-2`), `:1040` (air-gap compile, RESOLVED 2026-09-21, owner line still names 20-2), `:1042` (binary size, owner 20-2) → AC8.
- FR1 (`prd/functional-requirements.md:24`); NFR-Ops-12 (`prd/non-functional-requirements.md:167`), incl. its third clause (Spirit-author capability-token guidance) → AC3.7.

**Does NOT close, and says so:**
- Publication to the Homebrew tap and AUR (`ops-brew-tap-and-aur-publication`, S:331) and the Docker Hub push credentials (`ops-provisioning-secrets-and-accounts`, S:327). This story produces the rendered recipes and the push path; it never fakes an account or a secret.
- Removal of owned T3 containers/cgroups by `maos purge` (20-10, after this story; R:24). AC6 changes only purge's binary-removal hint.
- A macOS run of the Homebrew formula: macOS stays off the discipline matrix (D-15-4-F, `discipline.yml:5830-5831`); the formula's Linux leg is witnessed, the darwin leg rides `release.yml`'s tag build only.
- A genuine network-free air-gap binary — unless D-20-2-01 B is ruled.

## Story

As **Paige (operator docs) and Winston (release, kernel boundary)**, and every operator who installs MAOS,
I want **each install route to verify the signed release and install exactly its bytes, the air-gapped claim proven by behaviour rather than by a symbol scan of a binary that does not boot, and the container image built on every change with pushing separated from building**,
so that **FR1's "install via OS package manager, cargo, or a signature-verified binary" is true on the day of the rc tag, and NFR-Ops-12 is a CI result, not a runbook sentence.**

**Contract:**
- Epic §20-2 (E20@HEAD:110-120) as amended 2026-10-30; preflight §3.2 (PF:171-183) and D09 (PF:293). Create-story added no uncited AC: AC2's RPM/cargo halves and AC3's journaling clause are R:44; AC6 is D09 A; AC8 is the three ledger rows naming 20-2.
- Tracker `sprint-status.yaml:298` (this key), `:327`, `:331`, `:341`; E20 confidence rules 4, 6, 9–11; proof contract `docs/testing/proof-contract.md:10-31` (one final-tree run + one plant per AC), `:33-71` (review tiers).
- Rule 11 governs AC1–AC5: each is a control (verifies, refuses, detects) and must be wired, run and able to fail.

## Inventory at HEAD — measured, not assumed

Method: `creation/witnesses-at-head.md` (W1–W13). Nothing below was compiled; "hosted" rows are read from GitHub.

| Surface | Fact at `a8af3215` | W |
|---|---|---|
| `.deb` | Never buildable: no `debian/changelog` (`dpkg-buildpackage` rc 25) and `rules` does not parse — the Python heredoc `rules:28-41` has no recipe tab on `:29-41` (`make` rc 2, `debian/rules:29: *** missing separator`). `Build-Depends` (`control:5`) also demands `cargo, rustc` for a pre-built binary. `dh_strip` would rewrite the unstripped release binary (published 40,608,336 B vs 30,538,368 B stripped), so the installed bytes would not be the signed bytes. SHA256SUMS lookup is an unanchored regex (`rules:51`). | W1, W2, W8 |
| Recipes (deb/brew/AUR/rpm) | All four embed the **dev** key `bedd2ba6…69e7` (`rules:10`, `maos.rb:20`, `PKGBUILD:36`, `maos.spec:20`) and point at `v0.5.0` (`maos.rb:15`, `PKGBUILD:12`, `maos.spec:9`); the only release is `v0.1.0-alpha.1`, whose `SHA256SUMS.sig` verifies **only** under the production key `84cffd59…f40e` (embedded in its `maosctl`). Their signature convention Ed25519(SHA256(SHA256SUMS)) is right (`release_verify.rs:150-168`). No recipe can install the published release. | W5, W13 |
| Container | The only `container.yml` run (tag `v0.1.0-alpha.1`, 36094027288) failed at `Log in to Docker Hub` (`:40-44`); GHCR login (`:33-38`, `GITHUB_TOKEN`) succeeded; build and cosign skipped. No image has ever been built in CI; `discipline.yml` has no docker build. `DOCKERHUB_*` secrets absent. | W3, W4, W12 |
| Glibc floor | Published `maos-linux-amd64` needs `GLIBC_2.39` → not runnable on `distroless/cc-debian12` / Debian 12. | W6 |
| Air-gap variant | `air-gap = []` is empty (`maos-bin/Cargo.toml:29`): the variant is `--no-default-features`. Its closure still links `ureq` (maos-kernel-core `io/mod.rs:91,119`), `tokio-postgres` (loom-lite, cli), `rustls`, tokio `net`. Its `run` (`main.rs:1585-1617`) appends a text line to `transparency.airgap-stub.log` — no boot, no Spirit, no TL entry. | W9, W10 |
| Air-gap controls | `check-air-gap` and `tests/air-gap-netns-corroborate.sh` are enrolled nowhere; the script exits 0 on four SKIP branches (`:29-49`), traces only `--version` with `\|\| true` (`:54-55`), without `strace -f`, counts `AF_UNIX` sockets as network (`:57`), and passes an inconclusive canary (`:82-85`). The runbook claims "zero networking code" (`ag-1…md:7-9`), a CI gate (`:17`, `:111-118`) and TL parity (`:89-91`) — all false at HEAD. | W9, W12 |
| Purge hint | `maos purge` tells every operator `cargo uninstall maos` (`purge.rs:823,1042`; ADR-067:84-85; `run-maos.md:111-112`) — rc 101 for a Cargo install (the package is `maos-bin`) and meaningless for a package-manager install. | W7 |
| Size | 32 MiB bound `onb_nfr2_timing.sh:176`; shared-target release `maos` 31,362,992 B stripped (tree unknown). | W8 |

## Current-tree corrections — read before implementing

Full OLD → NEW table: `creation/cites-a8af3215.md` (C1–C19, T1–T3, P1–P9).

| Premise (epic / preflight / tracker) | Fact at `a8af3215` and consequence |
|---|---|
| "20-1 adds registry and publisher to build, staging, manifest and uploads" (E20:114) | done by 20-1a (`release.yml:71-101,155-167`; `release_dry_run.rs:27-44`) — packages carry four binaries (D-20-2-07). |
| deb rules "source anchors, not package proof" (E20:114) | the package has never built (W1, W2) — AC1 rebuilds `packaging/deb/` as a debian directory. |
| 32 MiB bound `proof-contract.md:170-176` (E20:114) | `tests/integration/onb_nfr2_timing.sh:176` (C3). |
| `check-air-gap` `main.rs:869-882`, dispatch `:1479` | `:873-887`, `:1501-1506` (C5). |
| network `--version` `main.rs:1858-1866`, air-gap `:1520-1524`; "only the air-gap main has `--version`" (E20:116,119) | `:1881,1887-1891` and `:1518-1558,1532-1535`; both answer (C9, C15). |
| per-Spirit uninstall `main.rs:7173-7211`; `verbs.rs:182`; cascade `:8883` | `main.rs:7360-7364`, `:10454`; `verbs.rs:164,196` (C10, C16). |
| check-mock `release.yml:46`; `check_mock_not_in_release.rs:29-31`; air-gap main `:1318-1342`; `release_verify.rs:26-30` | `release.yml:77-78`; `:28,30`; `:1518-1558`; `:25-28,56-59,253-254` (C11). |
| epic AC3: `maos purge` "removes what the `.deb`/formula installed" | D09 A: the package manager removes package-owned binaries; purge removes data (C16; AC6). |
| air-gap variant "compiles at HEAD" and is a release artifact (E20:118; S:341) | compiles, but is not network-free and cannot boot (W9, W10) — D-20-2-01. |
| kernel-Δ 0 · xtask +0–30 · CI/packaging only (E20:73) | kernel 0 unless D-20-2-01 B (+38 / +32); maos-bin −18; xtask +2 measured + renderer (C17). |
| tracker ".deb via dpkg-deb"; "`--no-default-features` BROKEN"; "size BREACHED" (S:298) | `dpkg-buildpackage`; both RESOLVED 2026-09-21 (T1–T3). |

## Kernel and kloc pricing — measured at creation

**Instrument:** prebuilt `/mnt/build/cargo-maos/debug/xtask` (`check-kernel-baseline --json`, `kloc-check --json`) on
`git archive a8af3215` copies; the untouched copy reproduces the pins exactly (26841 / 95 files / set hash
`db39bc4a…`; kernel 21042 tokei; aggregate 191826). Detail: `creation/measurements.md`,
`creation/kloc-kernel-a8af3215-and-prototypes.json`. Prototypes `rustfmt`'d, never compiled, never applied.

**Kernel:** zero under every recommended option. The only kernel-touching option is D-20-2-01 B:

| Shape | Kernel physical | Kernel tokei | Diff |
|---|---:|---:|---|
| KA — kernel-core `http` feature (default on) owning `ureq`; `not(http)` impl returns a typed `IoError::Transport` | +38 (26879) | +32 (21074) | `ka-kernel-core-http-feature.diff` |

KA is necessary, not sufficient, for B (feature unification through 13 crates + 7 Spirits; `tokio-postgres`;
a boot path) — `measurements.md` § Kernel prototype.

**kloc (tokei; HEAD = every pin, zero headroom):**

| Crate · component | Measured / forecast | Basis |
|---|---:|---|
| maos-bin · air-gap `run` → typed refusal (D-01 A) | **−20** | prototype `binA` |
| maos-bin · route-neutral purge hint (D-12 A) | **+2** | prototype `binA` (purge only) |
| xtask · `EXPECTED_GATES` enrolment (D-02 A, D-03) | **+2** | prototype `xt` |
| xtask · recipe renderer (D-08 A) | +80…+150 [INFERENCE] | `release_dry_run.rs:113-161` |
| packaging, workflows, Dockerfile, scripts, docs, tests | uncharged | — |
| **aggregate** | **−16 measured; +64…+134 forecast** | sum |

**Envelope (D-20-2-13 A, PROPOSED; D02 A ×2 measured / ×3 unmeasured; ceilings, not reservations):**

| Crate | HEAD | Forecast | Ceiling with allowance | Inherited room | Operator ruling needed |
|---|---:|---:|---:|---:|---|
| maos-kernel-core (physical / tokei) | 26841 / 21042 | 0 | unchanged | 0 | only if D-01 B: ≤ 26917 / 21106 (FLAG-Winston) |
| maos-bin | 33498 | −18 | **33534** (+2×18) | 0 | yes |
| xtask | 44406 | +2 + 80…150 | **44860** (+2×2 + 3×150) | 0 | yes |
| aggregate | 191826 | +64…+134 | **192316** (+36 + 454); hardfail = measured + 1 | 0 | yes |

Option deltas (add to the ceilings if ruled): D-01 B — kernel row above; maos-bin +300…+800 ×3 (+2400);
maos-cli / maos-loom-lite +40…+150 ×3 (+450); xtask +20…+40 ×3 (+120). D-12 B — maos-bin +30…+50 ×3 (+150) instead
of +2×2. D-08 B or C — xtask renderer 0 (ceiling 44410).

## Decisions — RULED 2026-11-02 (ask s202)

**Rulings 2026-11-02 (ask s202):** The operator ruled every D-20-2-01…14 row exactly as recommended, including D-20-2-02 A conditional on T0 measurement 1, with C if red, and confirmed preflight D09 A. D01 A, D02 A and D11 A remain in force. The grants and ceilings are recorded in frontmatter.

**Landing-1 T0 (2026-11-05, cbed4dad):** the required `check-air-gap --build-first
--dirty-fixture <default build>` exited 1 and detected `TcpListener` and
`getaddrinfo` in the no-default-features subject; the default fixture was
correctly rejected. The already-ruled conditional **D-20-2-02 C applies**:
advisory symbol observation, not a ship badge. The default-build netns witness
is blocking. Local `unshare -n` exits 1 (`Operation not permitted`), `strace`,
debhelper, Docker, Homebrew and rpmbuild are absent and no user bus exists.
Those hosted proofs are authored, not claimed run. A hosted outbound result
remains STOP under AC3.8. No tracker/story status flip is made by this agent.

| ID | Question | Options | Recommendation and evidence | Status |
|---|---|---|---|---|
| **D-20-2-01** | What NFR-Ops-12 is witnessed on, and what "the air-gap variant" ships as | **A** witness the **shipped default build** inside `unshare -n` (behaviour: boot, replay session, TL entries, zero `AF_INET`/`AF_INET6` traffic); the `--no-default-features` build is a compile-shape control only — not a release artifact — and its stub `run` becomes a typed refusal (−20 measured); ops-rc acceptance amended · **B** make the variant genuinely network-free and bootable and ship `maos-airgap-linux-{amd64,arm64}`: KA kernel row (+38/+32 measured, FLAG-Winston), default-features plumbing in 13 crates + 7 Spirits, `tokio-postgres` gated, a substrate boot path (+300…+800 bin, +40…+150 cli/loom-lite, +20…+40 xtask [INFERENCE]); witness on that binary · **C** ship the variant as is with its stub `run` labelled, witness on the default build | **A.** NFR-Ops-12 asks that the *substrate* boot, run and journal with zero outbound calls (`non-functional-requirements.md:167`); only the default build has a substrate (W10). B is a product feature the PRD does not require, with kernel bytes and an unmeasured boot path; C ships a binary whose `run` writes a file that reads like a boot record (rule 11). | RULED 2026-11-02: **A** (ask s202) |
| **D-20-2-02** | `check-air-gap` (symbol scan of the `--no-default-features` build) | **A** enrol blocking (registry, `EXPECTED_GATES` +2 measured, discipline job, coverage) **if** T0 measurement 1 is green; red ⇒ STOP + row (the repair is D-01 B scope) · **B** retire the gate and `tests/fixtures/dirty-network-fixture` (xtask −~240) because under D-01 A no air-gap binary ships · **C** enrol advisory, labelled with its measured symbol list | **A, conditional on T0 #1**; if red, **C** with the measured list. W9 puts `ureq`, `tokio-postgres` and tokio `net` in the closure, so a red is plausible; the gate still guards the compile shape against new network crates. | RULED 2026-11-02: **A** (ask s202); A conditional on T0 #1, C if red |
| **D-20-2-03** | The netns witness mechanism | **A** harden `tests/air-gap-netns-corroborate.sh` into a blocking discipline job: `sudo` only for `unshare -n`, workload dropped to the runner uid (`setpriv`), `strace -f`, count `AF_INET`/`AF_INET6` only, canary must detect, every missing prerequisite under declared CI exits non-zero, TL entries asserted · **B** a Rust integration test in maos-bin (`declared_ci()` pattern), run via `sudo` · **C** an xtask subcommand (`check-air-gap --netns`, +80…+150 charged) | **A.** Uncharged, one job, mirrors the 20-3a no-green-skip step (`discipline.yml:6141-6156`); B needs a root-run test binary inside nextest; C charges a zero-headroom crate for shell work. | RULED 2026-11-02: **A** (ask s202) |
| **D-20-2-04** | Container push and signing | **A** build on every change (discipline) and on tags; on tags push GHCR + keyless cosign now (no operator secret: `GITHUB_TOKEN`, OIDC — W3) and Docker Hub only when `DOCKERHUB_*` exist · **B** build on every change and on tags; **all** pushes and signing run only when `DOCKERHUB_*` exist, otherwise the job prints `SKIPPED: DOCKERHUB_* absent (ops-provisioning-secrets-and-accounts)` and succeeds · **C** tag-time build only, no discipline build | **B.** Epic-faithful: publication waits for the operator (E20:44); A publishes a public GHCR image on the first rc tag without a ruling; C leaves the image unproven until a tag. | RULED 2026-11-02: **B** (ask s202) |
| **D-20-2-05** | Dockerfile shape | **A** keep the multi-stage source build: `ARG FEATURES`, `.dockerignore`, toolchain pinned to `rust-version`, the include roots the build reads · **B** runtime-only image from the **signed release binary**, verified in a build stage against `SHA256SUMS` + sig, on `gcr.io/distroless/cc-debian13:nonroot` (glibc 2.39 floor, W6) · **C** both | **B.** The image then contains exactly the signed bytes; no second release compile in CI (20-15's build-once); `cc-debian12` cannot run the 2.39 binary. A's `ARG FEATURES` has no shipped variant to build under D-01 A. | RULED 2026-11-02: **B** (ask s202) |
| **D-20-2-06** | Release-signature verifier inside the recipes | **A** keep the independent Python `cryptography` verifier (deb/rpm/AUR already depend on it), move it into `packaging/verify-release.py`, fix the Homebrew dependency (`python@3`, `maos.rb:50`, believed not a formula — T0 #5); correct `maos.rb:5`'s comment · **B** `maosctl install --from-local --verify-only --release-pubkey <rendered>` (makes `maos.rb:5` true) · **C** `openssl pkeyutl -verify -rawin` (measured working with the production key, W5) | **A.** B runs a downloaded, not-yet-verified binary to verify itself; C needs OpenSSL ≥ 3 on every builder (macOS ships LibreSSL [INFERENCE]). | RULED 2026-11-02: **A** (ask s202) |
| **D-20-2-07** | What a package installs (and therefore owns and removes, D09 A) | **A** all four: `maos`, `maosctl`, `maos-spirit`, `maos-registry-server` · **B** `maos` + `maosctl` · **C** `maos` only (HEAD) | **A.** The 20-1a exit path uses all four as installed commands; 20-1a routed their packaging here (`20-1a-…md:33,170`). | RULED 2026-11-02: **A** (ask s202) |
| **D-20-2-08** | Where the recipes' version, hashes and key come from | **A** committed recipes are templates; xtask renders them per release from the signed `SHA256SUMS` and the built `maosctl release-pubkey` (+80…+150 xtask); `release.yml` attaches the rendered recipes; CI renders against the dev-signed dry-run dist and fails if any token survives · **B** same rendering in a Python script under `scripts/` (uncharged) · **C** commit recipes pinned to the last published release (`v0.1.0-alpha.1`: real sha256s, production key `84cffd59…`, W5) and bump them by a post-release commit | **A.** One release-tool home (xtask already owns `release-dry-run`/`release-verify`), tested; the rendered key is by construction the key the binaries embed, so the dev key can never reach a tag (`release.yml:140-147` refuses it first). C is correct only between releases and cannot cover the rc it ships with. | RULED 2026-11-02: **A** (ask s202) |
| **D-20-2-09** | Signature over the packages themselves | **A** a second manifest `SHA256SUMS-packages` + `.sig` over the `.deb`/`.rpm` files, signed with the same key by the existing `release-verify --sign`, self-verified (`release-verify --verify`), attached · **B** no package signature; CI asserts the packaged binaries are byte-identical to the signed ones · **C** two-phase single `SHA256SUMS` re-signed to include the packages | **A.** B leaves maintainer scripts and control metadata unsigned — a tampered `.deb` with identical binaries passes; A costs no xtask (the sign/verify verbs are generic); C changes the manifest the binaries' own verification reads. | RULED 2026-11-02: **A** (ask s202) |
| **D-20-2-10** | Witness depth per recipe | **A** execute every recipe against the dev-signed dist on hosted runners: deb amd64 (`ubuntu-24.04`) and arm64 (`ubuntu-24.04-arm`) build → install → identity → remove; rpm (`rpmbuild` + `rpm -i/-e` in a Fedora container); PKGBUILD (`makepkg -si` in an Arch container, non-root); formula (`brew install` from a local tap, Linux) · **B** deb + rpm executed; PKGBUILD and formula static (render, syntax, no token, key consistency) · **C** deb only | **A.** Rule 11 (b): a recipe that verifies signatures is a control and must run where it can fail; B/C leave two controls asserted, not run. Cost: T0 #11. | RULED 2026-11-02: **A** (ask s202) |
| **D-20-2-11** | cargo distribution route (FR1) | **A** document `cargo install --locked --git https://github.com/lunarpulse/maos --tag <tag> maos-bin maos-cli maos-spirit-cli maos-registry`; hosted witness installs `maos-bin` into a temp root, runs `--version`, `cargo uninstall --root … maos-bin` · **B** publish the workspace crates to crates.io (operator account, ~50 crate names) · **C** move the cargo route to v1.1 by PRD delta | **A.** No account, no new obligation; B is a publication programme; C is a scope cut never made implicitly. | RULED 2026-11-02: **A** (ask s202) |
| **D-20-2-12** | `maos purge`'s binary-removal hint (D09 A) | **A** route-neutral constant naming each installer's removal command, at `purge.rs:823` and `:1042` (+2 measured) · **B** detect the installer from `current_exe()` (+30…+50 [INFERENCE]) · **C** leave it | **A.** Correct for every route at +2; B guesses from paths that several package managers share (`/usr/bin`); C is wrong for every route (W7). | RULED 2026-11-02: **A** (ask s202) |
| **D-20-2-13** | Per-crate envelope (D02 A applied) | **A** the envelope above (×2 measured / ×3 unmeasured) · **B** exact measured grants, every review correction re-asks · **C** ×1.3 (rule 6) | **A.** Most of the xtask draw is unmeasured; 20-3b/3c/7 landed at 2–3× creation forecasts. | RULED 2026-11-02: **A** (ask s202) |
| **D-20-2-14** | RPM scope (FR1, R:44) | **A** build `.rpm` (x86_64 + aarch64) in `release.yml`, attach with the D-09 manifest, witness per D-10 · **B** build in CI as proof only, not attached · **C** RPM to v1.1 by PRD delta | **A.** FR1 names rpm; the spec exists; attaching costs one upload list. | RULED 2026-11-02: **A** (ask s202) |

## Acceptance Criteria

*(Epic §20-2 as amended 2026-10-30, under the recommended D-row options; a different ruling rewrites only the
clauses marked with its row. **local** = dev host; **hosted** = GitHub-hosted run on the story's tree.)*

### AC1 — the `.deb` builds, verifies the signed release and installs exactly its bytes (epic AC1; D-07, D-09, D-10) ◆ security

1. `packaging/deb/` is a buildable debian directory: `changelog` (version rendered, D-08), `control` (`Build-Depends: debhelper-compat (= 13), python3-cryptography` — no `cargo`/`rustc`; `Depends: ${shlibs:Depends}, ${misc:Depends}`), `rules` that parses: verification calls `packaging/verify-release.py` (D-06 A) from `override_dh_auto_configure`; the SHA256SUMS lookup matches the exact filename field, never a regex substring (`rules:51`); `override_dh_strip`/`override_dh_dwz` are no-ops so `/usr/bin/<bin>` is byte-identical to the signed artifact; the four binaries of D-07 install under `/usr/bin`.
2. The discipline `release-dry-run` job (`discipline.yml:5821-5880`) builds the package with `dpkg-buildpackage -us -uc -b` from the dev-signed dist (after `:5848-5851`), installs it (`sudo dpkg -i`), asserts `maos --version` prints the workspace version and that each installed binary's SHA-256 equals its `SHA256SUMS` line, then runs AC6.2's removal leg. amd64 on `ubuntu-24.04`; arm64 natively on `ubuntu-24.04-arm` (D-10 A; T0 #4).
3. `release.yml` `sign-and-publish` builds the amd64 and arm64 packages after signing (`:128-134`) with the rendered production key, signs `SHA256SUMS-packages` (D-09 A), self-verifies (`release-verify --verify`, as `:148-151`) and attaches them (`:152-174`, `fail_on_unmatched_files: true` kept).
4. **Failing-before (local, measured at creation):** `dpkg-buildpackage -us -uc -b` over `packaging/deb/{control,rules}` → rc 25 `cannot open file debian/changelog`; `make -f debian/rules -n` → rc 2 `debian/rules:29: *** missing separator` (W1, W2).
5. **Plant (hosted, one):** re-sign `SHA256SUMS` with the zero seed before `dpkg-buildpackage` (the `:5857-5874` pattern) ⇒ the package build reds at verification; restored ⇒ green.

### AC2 — RPM, AUR and Homebrew recipes reference the real artifact set (epic AC2; R:44; D-06, D-07, D-08, D-10, D-14) ◆ security

1. No recipe published with a release carries `SCAFFOLD`, `PLACEHOLDER_*`, a hard-coded version or the dev key (`maos.rb:7,15,18-20,25-47`; `PKGBUILD:6,12,27-36`; `maos.spec:4,9,18-20`): version, URLs, sha256 and pubkey are rendered (D-08 A) from the release's signed `SHA256SUMS` and the built `maosctl release-pubkey`; the version maps per format (deb/rpm `~` pre-release, Arch without hyphen) and each mapping is checked with the format's own comparator (T0 #7).
2. Each recipe verifies Ed25519(SHA256(SHA256SUMS)) with the rendered key, fail-closed, before installing; looks up its binaries by exact name; installs the D-07 set unmodified (rpm: no `brp-strip`/debuginfo; Arch: `options=('!strip' '!debug')`; Homebrew: no relocation of the binaries [INFERENCE, T0 #5]); `makedepends` drops `cargo`/`rust` (`PKGBUILD:19`).
3. `maos.rb`: `test` keeps `maos --version` (answered by the network build, `main.rs:1881,1887-1891`); the `:5` comment states what `install` does (D-06 A); its verifier dependency is a real formula (T0 #5).
4. **Hosted witness (D-10 A):** the rpm is built (`rpmbuild -bb`) and installed/removed in a Fedora container; the PKGBUILD runs `makepkg -si` in an Arch container as a non-root user; the formula installs from a local tap on the Linux runner — each against the dev-signed dist with the dev key rendered, each asserting installed SHA-256 = signed.
5. `release.yml` attaches the `.rpm` files (D-14 A, under D-09's manifest) and the rendered `maos.rb`, `PKGBUILD` and `maos.spec` (publication to the tap/AUR is `ops-brew-tap-and-aur-publication`).
6. **Failing-before (local):** the render check (or an `xtask/tests` vector) reds at HEAD on every placeholder token, the `0.5.0` versions and the dev key in all four recipes. Measured at creation: the published `v0.1.0-alpha.1` signature verifies only under `84cffd59…f40e`; the recipes embed `bedd2ba6…69e7` (W5).
7. **Plant (local):** leave one sha256 token unrendered ⇒ the render check reds.

### AC3 — NFR-Ops-12: an air-gapped deployment boots, runs and journals with zero outbound traffic (D-01 A, D-03 A) ◆ security

1. A blocking discipline job runs the shipped default build (from the dry-run dist) inside `sudo unshare -n`, the workload dropped to the runner uid (`setpriv`), under an isolated `HOME`/`XDG_*` and `strace -f -e trace=network`: `maos init`, then a replay-inference session (`MAOS_INFERENCE_MODE=replay`, the exit path's cassette `crates/maos-journey-test/cassettes/j0/shell-intro.json`). It asserts zero `connect`/`sendto`/`sendmsg` on `AF_INET`/`AF_INET6` (other families counted and printed, not failed), at least one new Transparency-Log entry for the session (Merkle root or row count before/after), and the session's expected response.
2. The negative-control canary (a TCP connect under the same namespace and strace) must be detected — zero ⇒ red. Under declared CI every missing prerequisite (`unshare`, `strace`, privilege, binary, cassette) exits non-zero (20-3a pattern, `discipline.yml:6141-6156`); locally a missing prerequisite prints `NOT RUN` and is never evidence.
3. `tests/air-gap-netns-corroborate.sh` is rewritten to AC3.1–3.2 (its `:29-49` SKIP→0, `:54-55` `--version`/`|| true`, `:57` `socket(` count and `:82-85` inconclusive→0 removed) or replaced and deleted.
4. The `--no-default-features` build's `run` (`main.rs:1585-1617`) becomes a typed refusal (exit 2, prototype `binA-air-gap-run-refusal.diff`); the variant is not a release artifact; its compile shape stays proven by `discipline.yml:557-560` and AC4. Purge keeps enumerating `transparency.airgap-stub.log` (`purge.rs:273`) for files older binaries left.
5. **Failing-before (hosted):** the HEAD script under declared CI on `ubuntu-24.04` exits 0 (a SKIP branch or a `--version`-only PASS) — run id recorded; HEAD's air-gap `run` writes no TL entry (W10).
6. **Plant (hosted, one):** the witnessed session with a deterministic outbound attempt injected (knob chosen by T0 #3) ⇒ the `AF_INET` attempt is caught ⇒ red.
7. `docs/runbooks/ag-1-air-gap-deployment.md` states what is true: the deployment model of D-01, the witness and how to run it, the measured closure (W9) in place of "zero networking code" (`:7-9`), no CI claim the tree does not run (`:17`, `:111-121`), TL behaviour per build (`:89-91`); the Spirit-author capability-token guidance (`:79-85`, NFR-Ops-12's third clause) re-checked against the scopes the default build issues.
8. If T0 #3 shows the default build attempting outbound traffic in the namespace, that is an NFR-Ops-12 product defect: STOP + D-row with the trace; never an allowlist.
- *Under D-01 B instead:* the witness runs the shipped `maos-airgap-*` binary; KA lands last as its own FLAG-Winston landing; this AC's 3.4 becomes "the variant boots".

### AC4 — `check-air-gap` enrolled (epic AC1; D-02)

1. T0 #1 measures `check-air-gap --build-first --dirty-fixture <default network maos>` at the story's base. Green ⇒ enrolled blocking: `xtask/gate-registry.toml` (gates list + `[[ship_gate]]`), `EXPECTED_GATES` (`check_ship_gate_completeness.rs:17`, prototype `xt-ship-gate-enrolment.diff`), a discipline job in `v1-0-ship-gate` and `aggregate` needs, `tests/coverage-matrix.yaml` NFR-Ops-12 `gates:`; `check-mock-not-in-release` (`check_mock_not_in_release.rs:28`) re-run on the same binary. Red ⇒ STOP + D-row (D-02 C proposed).
2. **Failing-before (local):** zero enrolment hits in `.github`, `gate-registry.toml`, the coverage matrix and `prepush.rs` (W12); `check-ship-gate-completeness` reds when `EXPECTED_GATES` alone is edited (its three-site check).
3. **Plant (hosted):** point the job at the default (network) build ⇒ red (in addition to the gate's built-in dirty-fixture bite).

### AC5 — the image is built and checked on every change; pushing is separate (epic AC1; D-04 B, D-05 B) ◆ security

1. `Dockerfile` (`:13-40`): a verify stage checks the release binary against `SHA256SUMS` + sig (dev key in discipline, production key on tags); the runtime stage is `gcr.io/distroless/cc-debian13:nonroot` with `maos` as entrypoint and the source SHA as a label.
2. Discipline job `container-build` (needs the `release-dry-run` dist artifact, `discipline.yml:5875-5880`): `docker buildx build --load`, no login, no push, no secrets, `permissions: contents: read`; smoke `docker run --rm maos:ci --version` and `--help`.
3. `container.yml` (`:9-80`): tag-only, builds always; login, push and cosign steps run only when both `DOCKERHUB_*` are present (a job-level env flag — `secrets` is not usable in `if:`), otherwise the job prints `SKIPPED: DOCKERHUB_* absent (ops-provisioning-secrets-and-accounts)` and succeeds. `docs/runbooks/provisioning-checklist.md:24-25,31` updated to the new anchors.
4. **Failing-before (hosted, measured):** run 36094027288 — failed at Docker Hub login, build skipped; no docker build in `discipline.yml` (W3, W12).
5. **Plant (hosted, one):** flip one byte of the staged binary in the build context ⇒ the verify stage reds.

### AC6 — package-owned binaries vs MAOS data (D09 A; epic AC3 rewritten; D-12 A)

1. Contract (ADR-067 §removal amended): the installer removes the binaries it placed (`dpkg -r maos`, `rpm -e maos`, `pacman -R maos`, `brew uninstall maos`, `cargo uninstall maos-bin`); `maos purge` removes MAOS data and prints a route-neutral removal hint in stdout (`purge.rs:1042`) and in the receipt's `next_step` (`:823`) (prototype `binA-purge-route-neutral-hint.diff`); `docs/adr/ADR-067-maos-data-root-layout-and-purge.md:84-85` and `docs/maos.dev/run-maos.md:111-112` corrected.
2. **Deb leg (hosted, in AC1.2):** after `dpkg -i`: `maos init`; `maos purge --yes` (data roots gone, receipt written); `dpkg -r maos` ⇒ all four binaries absent, `dpkg -L maos` empty, no MAOS data root left.
3. **Failing-before (local, measured):** `cargo uninstall maos` → rc 101 `package ID specification \`maos\` did not match any packages`; `cargo uninstall maos-bin` succeeds (W7).
4. **Plant (local):** restore the old hint ⇒ an assertion added to `crates/maos-bin/tests/maos_uninstall_16_4.rs` on the printed hint reds.

### AC7 — the cargo route (FR1; R:44; D-11 A)

1. The install page (AC8.1) names `cargo install --locked --git https://github.com/lunarpulse/maos --tag <tag> maos-bin maos-cli maos-spirit-cli maos-registry` and `cargo uninstall maos-bin …`.
2. **Hosted witness:** `cargo install --locked --path crates/maos-bin --root "$tmp"` → `"$tmp/bin/maos" --version` = workspace version → `cargo uninstall --root "$tmp" maos-bin` → absent. Wall-clock recorded (T0 #9).
3. **Failing-before:** no documented route; the FR1 coverage note names only `cargo install --path crates/maos-cli` (`coverage-matrix.yaml:341-349`). The plant is AC6.4's (the old `cargo uninstall maos` hint), not a second one.

### AC8 — install page, coverage, size and ledger

1. A user-facing install page (`docs/maos.dev/install.md`, linked from `docs/maos.dev/index.md`): each route (release binary + `maosctl install --from-local --verify-only`, deb, rpm, Homebrew, AUR, cargo, container), its verification step, removal, the glibc floor (W6: Ubuntu ≥ 24.04 / Debian ≥ 13 class [INFERENCE for distro versions]) and the air-gap page (AC3.7).
2. `tests/coverage-matrix.yaml`: FR1 (`:341-349`) names the deb/rpm/recipe/cargo witnesses; NFR-Ops-12 (`:1318-1323`) names the netns witness (and `check-air-gap` if enrolled).
3. The final-tree `onb-nfr2-timing` size leg (`onb_nfr2_timing.sh:115-117,176`) is green and its stripped figure recorded in the Dev Agent Record.
4. At `done`: `check-dev-record-completeness` passes with no open `deferred-work.md` row owned by `20-2`: `:98` → its witness; `:1040` (RESOLVED 2026-09-21) and `:1042` (resolved by the 32 MiB re-base; final-tree figure) each `CLOSED <date> by 20-2 @<commit>`, or re-owned by rule 10 with a reason.

## Tasks / Subtasks

- [ ] **T0 — Preflight.** Operator rulings on D-20-2-01…14 and the D09 confirmation recorded; tracker → `in-progress`. Run `creation/measurements.md` "Not measured here" 1–11 (hosted items on a `ci/20-2-verify` branch-only workflow, never merged). Re-measure every cite touched (rule 9). STOP + D-row on any figure above the ruled envelope or any result contradicting a ruled option (AC3.8, AC4.1).
- [x] **T1 — Verifier + deb (AC1.1, AC1.4).** Shared fail-closed verifier, buildable rendered debian directory, exact names, no strip, four binaries; local signature vectors and rendered make parse pass. Real deb build remains operator-hosted.
- [x] **T2 — Renderer (AC2.1, AC2.6–2.7; D-08).** Signed manifest verification, trusted built maosctl key, per-format version vectors, exact four-binary sets, token-survival refusal; actual unrendered-token plant red and byte-restored.
- [ ] **T3 — Recipes (AC2.2–2.5).** rpm, PKGBUILD, formula corrected; hosted witnesses per D-10.
- [ ] **T4 — Discipline legs (AC1.2, AC1.5, AC6.2).** deb build/install/identity/purge/remove in `release-dry-run` (amd64 + arm64); forged-signature plant.
- [ ] **T5 — Release integration (AC1.3, AC2.5; D-09, D-14).** `sign-and-publish`: packages, `SHA256SUMS-packages` + sig, self-verify, attachments; `release-rehearsal` label run recorded (no tag).
- [ ] **T6 — NFR-Ops-12 (AC3).** netns job, canary, TL assertion, fail-closed prerequisites; air-gap `run` refusal; runbook; hosted failing-before + plant.
- [x] **T7 — `check-air-gap` (AC4).** T0 measured RED; already-ruled D-02 C applied, advisory job/registry/coverage; blocking netns gate enrolled at all three sites and cardinality readers updated.
- [ ] **T8 — Container (AC5).** Dockerfile, `container-build` job, `container.yml` split, provisioning checklist; hosted plant.
- [x] **T9 — Purge hint, cargo route, install page (AC6.1, AC6.3–6.4, AC7, AC8.1).** Route-neutral stdout/receipt hints, old-hint plant red/restored, package ownership docs and complete install page; hosted Cargo execution authored.
- [ ] **T10 — Close (AC8.2–8.4).** Coverage rows; exact pins inside the ruled envelope (xtask HISTORY first segment cites this key with `->`); `check-kernel-baseline`, `kloc-check`, `check-ship-gate-completeness`, `check-exit-commands`, `check-decision-register`, `check-dev-record-completeness`, `check-epic-close-coherence`, `prepush`; `cargo nextest run -p xtask --locked --test d11_xtask_ceiling_ratchet --test recovery_lane_ceiling_rule` before the `done` flip (`proof-contract.md:43-48`); one hosted discipline run on the final tree + the `ci/20-2-verify` plant runs; known 20-7 flakes recorded by name and run id (D13 A). Revert `intent-lineage-coverage-report.md` if a run modifies it. No commit or push without the operator.
- *Under D-01 B:* L1 = T0–T10 kernel-free; L2 = KA + feature plumbing + variant boot + `maos-airgap-*` artifact, FLAG-Winston decision record (precedent `20-3c-flag-winston-decision.md`), landing last.

**Landing-1 checkbox interpretation:** T0/T3–T6/T8/T10 remain unchecked because
their hosted execution/rehearsal/discipline proof clauses have not run, not
because implementation is missing. Every specified implementation and operator
proof leg is authored; hosted execution is explicitly operator-only in this
assignment. No status is flipped. See the Dev Agent Record and evidence for
the separation of local results from unverified hosted acceptance.

## Dev Notes

### What changes, by file

| Area | Files | AC |
|---|---|---|
| Packaging | `packaging/deb/{changelog,control,rules}` (+ `source/format`), NEW `packaging/verify-release.py`; `packaging/rpm/maos.spec`; `packaging/aur/PKGBUILD`; `packaging/homebrew/maos.rb` (templates under D-08 A) | AC1, AC2 |
| xtask | renderer (D-08 A; beside `xtask/src/release_dry_run.rs`), `xtask/src/main.rs` subcommand; `xtask/src/check_ship_gate_completeness.rs:17` (+2); `xtask/gate-registry.toml`; `xtask/kloc.toml:339,517,718` | AC2, AC4, AC8 |
| maos-bin | `crates/maos-bin/src/main.rs:1585-1617` (air-gap `run`); `crates/maos-bin/src/purge.rs:823,1042` | AC3.4, AC6 |
| Workflows | `.github/workflows/discipline.yml` (`release-dry-run` `:5821-5880` deb legs; NEW netns, `check-air-gap`, `container-build`, rpm/Arch/brew, cargo-install jobs; `v1-0-ship-gate`/`aggregate` needs `:6362`, `:6512`); `.github/workflows/release.yml:103-174`; `.github/workflows/container.yml`; `ci/20-2-verify` branch-only | AC1–AC5, AC7 |
| Container | `Dockerfile`, NEW `.dockerignore` | AC5 |
| Tests (uncharged) | `tests/air-gap-netns-corroborate.sh`; `crates/maos-bin/tests/maos_uninstall_16_4.rs` (hint); render vectors in `xtask/tests/` | AC2, AC3, AC6 |
| Docs / records | `docs/runbooks/ag-1-air-gap-deployment.md`; `docs/adr/ADR-067-…md:84-85`; `docs/maos.dev/run-maos.md:111-112`; NEW `docs/maos.dev/install.md`; `docs/runbooks/provisioning-checklist.md:24-34`; `docs/release/tag-procedure.md` (packages + rendered recipes); `tests/coverage-matrix.yaml:341-349,1318-1323`; `deferred-work.md:98,1040,1042` | AC3, AC6, AC8 |

### Overlap (serialize; re-measure)

- **`release.yml`, `release_dry_run.rs`** — 20-1a (open landing; four-binary set) and 20-5 (version → `1.0.0-rc.1`, release precondition `check_release_precondition.rs`): the renderer reads the workspace version, so 20-5's bump flows through without an edit here.
- **`purge.rs`** — 20-10 (after this story) removes T3 residue in the same file; AC6 touches only the hint constant and its two uses.
- **`discipline.yml`** — 20-7 L2, 20-15 build-once archive: new jobs consume the `release-dry-run` dist artifact rather than recompiling.
- **`tests/coverage-matrix.yaml`, `gate-registry.toml`, `EXPECTED_GATES`** — any story enrolling gates; one integration owner per landing.

### Current state that must be preserved

- `release.yml` guardrails: production key ≠ dev seed (`:135-139`), refusal of the bundled dev key (`:140-147`), self-verify (`:148-151`), `prerelease` from the tag (`:174`), `fail_on_unmatched_files: true` (`:170`), the `aggregate` precondition (`:117-122`).
- The `release-dry-run` forged-signature control (`discipline.yml:5857-5874`) and maosctl verification (`:5852-5856`).
- `check-exit-commands`' both-builds verb-table step (`discipline.yml:557-560`) — the `--no-default-features` compile shape.
- The 32 MiB size leg (`onb_nfr2_timing.sh:176`); a breach attributes before it re-bases (D:1042 rule).
- `maos purge` semantics and its residue list (`purge.rs:273` keeps `transparency.airgap-stub.log`).
- Signature convention Ed25519(SHA256(SHA256SUMS)) (`release_verify.rs:150-179`), verified against the published release (W5).

### Guardrails

- Never commit a production or dev key into a recipe that a tag publishes; rendered keys come from the built `maosctl release-pubkey` only (D-08 A). The production key `84cffd59…` is public, but a committed literal would drift from the secret on rotation.
- No recipe executes a downloaded artifact before its signature and hash are verified; verification failures exit non-zero (make recipe, rpm `%prep`, `prepare()`, Homebrew `system`).
- Installed bytes = signed bytes on every route; no stripping step anywhere in the packaging path.
- `sudo` in CI only around `unshare -n`; the netns job holds no secrets and `permissions: contents: read`. `container.yml` never runs on `pull_request_target`.
- Never allowlist an outbound call to make AC3 green (AC3.8); never `continue-on-error`/`|| true` on a control step.
- One cargo process per checkout; `env -u CI -u GITHUB_ACTIONS`; inherit the target directory, never override or clean it; plants byte-restored in the real tree (`proof-contract.md:24-31`).
- No kernel line unless D-20-2-01 B is ruled; then only KA's shape, landing last.

### Proven-red controls (rule 11; proof contract)

| Control | Plant | Expected red | Leg |
|---|---|---|---|
| deb verification (AC1) | zero-seed re-signed `SHA256SUMS` | `dpkg-buildpackage` | hosted |
| Recipe render (AC2) | one unrendered sha256 token | render check | local |
| NFR-Ops-12 netns witness (AC3) | injected outbound attempt | `AF_INET` count | hosted |
| `check-air-gap` (AC4) | network build as subject | gate | hosted |
| Image verify stage (AC5) | one flipped byte in the staged binary | `docker build` | hosted |
| Purge hint (AC6, AC7) | old `cargo uninstall maos` hint | `maos_uninstall_16_4` assertion | local |
| Ledger (AC8) | — (`check-dev-record-completeness` negative control at `done`) | gate | local |

### Tests and commands

```bash
# local; one cargo process per checkout; inherit the target directory
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p xtask -p maos-bin
env -u CI -u GITHUB_ACTIONS cargo test --locked -p maos-bin --no-default-features --test verb_table_15_3 -- --test-threads=1
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- release-dry-run
RELEASE_SIGNING_KEY=794959d4c4dc813f968cd95eb4a45c4a02583a7c5211126e7b4583e4776d1c8d env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- release-verify --sign --key-env RELEASE_SIGNING_KEY --sha256sums dist/SHA256SUMS --output dist/SHA256SUMS.sig
cp -r packaging/deb /tmp/maos-deb/debian && cp dist/* /tmp/maos-deb/ && (cd /tmp/maos-deb && dpkg-buildpackage -us -uc -b)   # path shape chosen at T1
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-air-gap --build-first --json
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-ship-gate-completeness --json
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- kloc-check --json
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-kernel-baseline
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-dev-record-completeness
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-epic-close-coherence
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- prepush
# netns, rpm/Arch/brew, container and arm64 legs are hosted-only (W11); local SKIP lines are not evidence
```

### Review plan (`proof-contract.md:33-71`)

DIFFICULT: Blind + Edge (opus-5.5), Acceptance (opus-5.5), Security reading on AC1/AC2 (verification order, exact
names, key provenance, byte identity), AC3 (sudo scope, namespace escape, canary honesty) and AC5 (push/sign gating,
secret exposure), Test Infrastructure (glm-5.3). Focused suites: `cargo nextest run -p xtask -p maos-bin --locked`;
the reviewer reads the hosted plant runs, never re-runs them.

### Previous-story intelligence (20-1a, 20-7, 20-3a, 20-6, 15-4)

- 20-1a: four-binary release set and the hermetic `epic-20-exit` job (`discipline.yml:2897-…`) landed; its record routes package ownership of `maos-spirit`/`maos-registry-server` here (`20-1a-…md:33,170,260`). Its hosted-found fixes (`47631284`) show hosted legs find what local runs cannot — budget one `ci/20-2-verify` cycle.
- 20-7: a branch-only verify workflow, never merged, is the plant vehicle (`20-7-…md`, Debug Log); a split-key rename broke two hosted doc gates that prepush does not run — run `check-exit-commands` and `check-decision-register` after record edits.
- 20-3a: the "no green skips" observation step for strace-based air-gap controls (`discipline.yml:6141-6156`) is AC3.2's pattern.
- 20-6 / 15-4: macOS stays off the discipline matrix (D-15-4-F); `release-rehearsal`-labelled PR runs of `release.yml` are the pre-tag rehearsal (`release.yml:5-37`).
- Epic-16 retrospective: `--no-default-features` broke silently once (D:1040) — the compile leg stays.

### Git intelligence

`a8af3215` (20-7 rulings, docs), `47631284` (20-1a hosted-found fixes), `0836ff40` (20-1a verify trigger). No change to
`packaging/`, `Dockerfile` or `container.yml` since Story 9.4; `release.yml` last changed by 20-1a.

### Library / framework

No new Rust dependency under the recommendations. Tools: debhelper 13 (`debhelper-compat (= 13)`), `rpm`/`rpmbuild`,
`makepkg` (Arch container), Homebrew (Linux runner), Docker Buildx, `strace`, util-linux `unshare`/`setpriv`, Python
`cryptography` (`Ed25519PublicKey.from_public_bytes` + `verify`). Images: `gcr.io/distroless/cc-debian13:nonroot`
exists with amd64/arm64 variants (Distroless docs, `https://github.com/GoogleContainerTools/distroless`). GitHub
`ubuntu-24.04-arm` hosted runners for public repositories [INFERENCE: availability for this repository is T0 #4].

### References

E20@HEAD:7,44,47,73,110-120,274; preflight `epic-20-preflight-2026-10-30.md:50,76,120,140,171-183,293`; tracker
`sprint-status.yaml:298,327,331,341`; `deferred-work.md:98,1040,1042`; `routing-audit-2026-10-22.md:24,44,120,139`;
PRD `functional-requirements.md:24-25`, `non-functional-requirements.md:126,167`; ADR-067 (`:84-85`), ADR-069
(`:35-46`); `docs/runbooks/ag-1-air-gap-deployment.md`; `docs/runbooks/provisioning-checklist.md:24-34`;
`20-1a-registry-publish-install-exit-path.md:33,162,170,260`.

## Dev Agent Record

### Story context creation

Created 2026-11-02 by `anthropic/claude-opus-5-5` (bmad-create-story, lighter measured contract) at `a8af3215`,
without a cargo build (parent rule).

**Inputs:** E20 §20-2 at HEAD (amended 2026-10-30, PF §3.2) — stale cites corrected here and owed to E20
(`creation/records-owed.md` §A); preflight §1–§6 and D09; tracker `:298`, `:327`, `:331`, `:341`; every
`deferred-work.md` row naming 20-2; routing audit rows; 20-1a's D09/D12 record; direct reads of every cited range
(`creation/cites-a8af3215.md`); W1–W13 (`creation/witnesses-at-head.md`: dpkg/make on a `/tmp` copy, GitHub run and
release reads, `openssl` signature checks, `objdump`, `strip`, `cargo tree --offline` and `cargo uninstall` on a
`/tmp` root — no build); kloc and kernel by the prebuilt gate binary on `/tmp/proto202` `git archive` copies
(`creation/kloc-kernel-a8af3215-and-prototypes.json`); prototypes KA, binA, xt saved as diffs, `rustfmt`'d, never
applied or compiled.

**Measurements owed:** `creation/measurements.md` "Not measured here" 1–11.

**Records owed:** `creation/records-owed.md` (§A epic corrections; §B ledger; §O operator; tracker).

**Revert:** no crate source modified (prototypes only under `/tmp/proto202`, deleted); no cargo build or test ran;
`intent-lineage-coverage-report.md` untouched.

**Gates after the record edits** (prebuilt `/mnt/build/cargo-maos/debug/xtask`, `env -u CI -u GITHUB_ACTIONS
CARGO_MANIFEST_DIR=…/xtask`, `--json`, in-tree; `creation/gates/*.json`): `check-dev-record-completeness` rc 0 (197 done stories, 167 deferred-owner assertions, 0 violations); `check-epic-close-coherence` rc 0 (21 epics, pin 26841, 0 violations); `check-decision-register` rc 0 (23 rows, 6 open, 0 findings); `check-exit-commands` rc 0 (2 blocks, 10 tokens, 0 findings); `check-kernel-baseline` rc 0 (26841 / 95 files, set hash `db39bc4a…`, no drift). In-tree `kloc-check` rc 1 (`maos-wasm-host 2420 > 2412`, aggregate 191834) from a sibling's uncommitted `crates/maos-wasm-host/src/runner.rs` (+8), not this creation; the `git archive a8af3215` copy is green (aggregate 191826, `over_budget = []`).

### Agent Model Used

anthropic/claude-opus-5-5 (creation); openai-codex/gpt-6.1-sol (Dev202, landing 1 implementation).

### Debug Log References

- [Landing-1 preflight, re-base and limitations](20-2-deb-airgap-docker-and-formula-corrections-evidence/landing-1/preflight-decisions.md).
- [Remeasured baseline citations](20-2-deb-airgap-docker-and-formula-corrections-evidence/landing-1/cites-cbed4dad.md).
- [Operator-only hosted verification recipe](20-2-deb-airgap-docker-and-formula-corrections-evidence/ci/ci-20-2-verify.yml); never install this workflow on main.
- Local command transcripts under `20-2-deb-airgap-docker-and-formula-corrections-evidence/landing-1/` retain command, exit and elapsed time. The required final prepush transcript is `prepush.log`/`prepush.stdout`; it is not presumed green.
- [Review-correction witnesses, rebase and verification](20-2-deb-airgap-docker-and-formula-corrections-evidence/landing-1/review-corrections/record.md); original landing transcripts above remain historical and are not overwritten.
- [Hosted-1 fixes, per-leg proof boundaries and local witnesses](20-2-deb-airgap-docker-and-formula-corrections-evidence/landing-1/hosted-1/record.md).


### Completion Notes List

- Implemented all four signed-byte-preserving package recipes and the shared independent Python verifier. No unverified downloaded executable verifies itself in a recipe. `xtask render-packages` verifies the original manifest/files, reads the trusted built maosctl key and renders version/URL/hash/key tokens; a surviving token is fatal.
- Default-build air-gap workload is init + real J0 replay + expected response + real TL growth. Locally the workload passes without isolation; netns proof is **unverified** because namespace creation is denied and strace is absent. The no-default-features `run` refuses with `BuildShapeUnsupported`, exit 2; no fake TL entry remains. Legacy stub cleanup is preserved.
- D-20-2-02's conditional C applies: both baseline and final compile-shape symbol scans exit 1 on `TcpListener`/`getaddrinfo`, while dirty default fixtures are correctly rejected. Final same-binary mock-not-in-release scan passes. The netns job, not the symbol scan, is the blocking deployment control.
- Container builds verify signed release bytes on Debian 13, run independently of credentials, and gate **all** login/push/cosign on both Docker Hub secrets. Tag publication and the verify recipe's explicit operator publication dispatch are authored, not executed.
- Local plants: **two**: AC2 unrendered SHA token (**0.08 s**, renderer exit 1; restored render exit 0) and AC6/AC7 old hint (**45.96 s**, real purge assertion exit 101; restored full 23-test suite exit 0). Both restore with copy-back + cmp + touch (`ac2-restore.log`, `ac6-restore.log`). Hosted AC1/AC3/AC4/AC5 plants are authored and **not counted as run**.
- Local verification: 6 Python crypto/trace tests, 3 renderer vectors, 3 workflow vectors, 23 purge tests, 7 no-default-features verb tests, 21 tree-bound nextest tests pass. Release workflow vectors, 27 macOS-guard vectors, 8 ship-gate-binding vectors, coverage tests and the internal 48-gate assertion pass after updating every affected enrollment/matrix reader. No unrelated failed test was retried to green.
- Standard gates observed exit 0: kernel baseline, kloc, ship-gate completeness, exit commands, decision register, dev-record completeness, epic-close coherence, coverage matrix and test cost. Affected release binaries and genuine default debug maos/maosctl build. `cargo fmt --all` passes. Affected all-target clippy exits 0; 664 warning events in total, **0 diagnostics intersect changed production ranges** (span-based delta, local compiler, not a hosted-toolchain or warning-free proof).
- One explicit workspace execution reused workspace cargo/binaries metadata: run `24d43741-bd54-4575-943f-6c2fea571c5d`, **5646 run / 5640 pass / 6 fail / 138 skipped**, exit 100. Four enrollment/matrix/coverage failures introduced by this landing were fixed and their focused suites passed. The other two were missing system-Python cryptography in protected 20-11's verifier test and an unclassified cold-swap predecessor refusal in `one_daemon_one_door_16_1`; neither is called a known flake or suppressed. The metadata compile first exceeded its 2900-second cell deadline; its surviving compile completed, then metadata capture resumed in 27.27 s. An incompatible nextest CLI invocation exited 2 before tests; corrected metadata-only execution is the one run above.
- Exact formatted pins after `cargo fmt --all`: **maos-bin 34842 -> 34823**, **xtask 44466 -> 44673**, **aggregate 193911 -> 194099**, hardfail **193912 -> 194100**. Rebased ceilings **34878 / 44920 / 194401** under D02 A ×2 measured / ×3 unmeasured. Kernel **26841 physical / 21042 tokei**, set hash unchanged; every other crate unchanged.
- Final release maos stripped **31,677,432 B < 33,554,432 B**. The original onb-nfr2 size leg passes on a copy; the full script was not run because it invokes forbidden cargo clean. Final ELF dynamic-symbol glibc floor is **2.39**. Deferred compile/size rows are CLOSED with uncommitted-tree provenance; outstanding hosted NFR-Ops-12 proof is re-owned under rule 10 to `ops-rc-signed-tag` with the local capability reason.
- No hosted run, release-rehearsal, macOS formula execution, native arm64 install, Docker build/push/sign, netns zero-outbound result or added CI job-minute measurement is claimed. Independent DIFFICULT/security review remains the orchestrator's responsibility; no review agent was available to this implementation agent.
- **Prescribed prepush invoked once, incomplete:** exact command in `landing-1/prepush.log`; 2900.02 s deadline, wrapper **124**, process group terminated before buffered final JSON appeared. `prepush.stdout` is empty; no per-stage or green prepush result is claimed, and no retry ran. No tracked edit occurred while it ran. Main notified; this is an outstanding verification limitation, not a waived gate.
- **Cleanup:** /proc/open-descriptor scan found no active compiler; removed 515 stale incremental snapshots while preserving 6465 (newest per crate folder + touched in last 2 h), never deps/build/fingerprint. Own /tmp scratch and Python caches removed. Removed allocated-file-block accounting **6,895,853,568 B**; observed filesystem available-space increase **5,301,002,240 B** (compression/reflinks/concurrent host activity may differ). Generated intent-lineage report remained byte-identical. See `landing-1/cleanup.json` and `generated-report-preservation.json`.

### Review corrections (P1–P5; security F1–F10/F14, 2026-11-05)

- Baseline is **88c7277c0967d2abb8bb89f5f52e4e5a7ffe46e8**, after the separately committed 20-11 corrections. No protected 20-11 code/evidence was edited. The exclusive cargo handoff supplied bin34914 / iac8622 / audit8174 / registry4412 / xtask44673 / aggregate194740 / hardfail194741. Current formatted measurement is **xtask 44673 -> 44676**, **aggregate 194740 -> 194743**, hardfail **194741 -> 194744**; bin34914, kernel26841/21042 and every other crate are unchanged.
- The original D02 A allowance is **not renewed**: initial 20-2 bin34823→34914 is unrelated +91; aggregate194099→194740 is unrelated +641. Carry those only onto ceilings34878/44920/194401, yielding **34969/44920/195042**, hardfail-bound195043. The correction consumes +3 xtask, leaving244 xtask lines in the original ruled envelope; these are ceilings, not reservations or exact pins.
- P1/P2: release Ubuntu builds use explicit `rpm-ubuntu`, which imports Ed25519 cryptography before `rpmbuild --nodeps`; Fedora `rpm` mode retains RPM-database dependency checking. Runner pre-creates `dist/packages` before root containers. The operator recipe authors native amd64/arm64 before/final RPM database witnesses, root-directory PermissionError, actual runner renames, and four-package second-manifest assembly/signature proof. None is claimed locally executed.
- P3: empty dotted identifiers and numeric-leading prereleases are refused, while rc.1/alpha.1 remain supported. P4: the real Python crypto/trace suite is blocking in native release-dry-run after its cryptography prerequisite. P5: RPM/AUR/Homebrew wrong-key controls preserve the trusted key, adjust only signature-download hashes where needed, require `InvalidSignature` with no installed payload, restore with copy-back/cmp/touch, then install/compare/remove valid bytes. The operator recipe removes each actual verifier call and requires the lane to notice acceptance; these native package-manager plants remain hosted/unverified.
- F1: the second signed denominator now covers four packages, standalone verifier, all three rendered standalone recipes and the Debian recipe archive. A trusted source verifier/local xtask bootstraps authenticity, never a downloaded verifier authenticating itself. F2/F3: operator preparation builds only the workflow's own ref with contents-read permission and verifies both signed manifests before handoff; separate environment-referenced publication jobs alone have packages/OIDC write. Production key must exist and differ from the public development key. No tag code is built by the operator lane. `release` protection is an operator setting, **unverified** after API404; required-reviewer/tag protections are recorded under ops provisioning.
- F4/F5: four critical action families are full-commit pinned with observed version membership; Docker/Ubuntu/Fedora/Arch base indexes are pinned to registry-resolved digests whose response hashes matched. This proves resolution, not image execution. Remaining action refs, preexisting F11/F12 interpolation and F13 persisted writer credentials are explicitly dispositioned under same-line rule-10 rows (20-7/20-5 where their charters own the workflow; otherwise ownerless trigger-owned/open).
- F6/F8/F9/F10/F14: sendmmsg is traced, io_uring submissions deny a clean witness, and ADR047 scopes the claim to no outbound attempts in a witnessed replay session. Downloaded netns binaries are independently authenticated before execution. Container release waiting binds exact tag/SHA/run-id/current-attempt success. Runtime state is owned by65532; real missing-owner/fresh-volume init controls are authored hosted-only. Deb/RPM docs distinguish build-time verification from user pre-install manifest verification; Homebrew byte preservation is **claimed, not verified**.
- Observed correction verification so far: before Rust version vectors exit101; before workflow contracts exit101 (11 review assertions red); before Python trace controls exit1. A real combined manifest/selector/hash/signature-bypass plant makes the **enrolled Python command** exit1 (9 assertion/subtest failures); all four subjects were copy-back/cmp/touch restored, then12 Python controls pass. Focused Rust contracts22/22 pass, affected maos/maosctl/xtask debug build passes, `cargo fmt --all` passes; affected all-target clippy exits0 with690 warning events and **zero primary diagnostics on the changed renderer** (not warning-free or hosted lint proof).
- The one workspace nextest and one prepush observations above are retained honestly, not retried to green by this correction pass. Hosted netns/native packages/arm64/default-volume Docker/Hub push/cosign and Homebrew installed identity remain **unverified**. Story/tracker status is not flipped.
- Final correction gates: kernel baseline, exact kloc, ship-gate completeness, exit commands, decision register, dev-record completeness, epic-close coherence, coverage matrix and test cost all exit0. The first dev-record run exposed five citation-grammar errors in new deferred rows; fixed to same-line `Dispositioned by` and final gate passes. One misspelled `check-coverage-matrix` invocation exited2 before doing work; the actual `coverage-matrix --json` exits0. Tree-bound nextest is21/21; provisioning contracts3/3 and shell syntax pass.
- Current native release rebuild (`cargo build --release --locked -p maos-bin -p maos-cli -p maos-spirit-cli -p maos-registry`) exits0. Fresh four-binary manifest/sign/render/independent verification exits0 with the public development fixture key; rendered Debian alpha ordering passes real `dpkg`. Current exact size-oracle lines115–181 on a copy pass at **31,936,280 B < 33,554,432 B**, glibc floor2.39. This supersedes the historical size above; no native Arch comparator or hosted runtime execution is claimed.
- Review cleanup: /proc/open-descriptor scan found no active compiler or open incremental descriptor;1257 stale snapshots removed,7863 preserved (newest-per-crate and anything touched within2h). Own scratch and five import caches removed. Allocated-file-block removal **8,378,310,656 B**, including own scratch124,551,168 B; observed filesystem free delta **7,757,279,232 B**. No deps/build/fingerprint or unrelated scratch was deleted; accounting caveats are in `review-corrections/cleanup.json`.

### hosted-1 (2026-11-06; baseline ebf2ab03)

- First verify run **42224184744**, attempt1, `6fda747d` on `ci/20-2-verify`: **7 unintended failures /6 green /3 skipped**. Main discipline **42224167600**, attempt1, `ebf2ab03`: now observed completed/failure. Diagnosis is preserved in `landing-1/hosted-1/record.md` with job ids/log lines; no failed negative-control setup is relabelled an intended plant red.
- Fixed actual RPM reserved-macro collision: `%global suffix` and all nine references become `maos_artifact_suffix`. Fixed Arch's raw Ed25519 transport alias to `SHA256SUMS.ed25519`, copying to the independent verifier's expected filename at prepare start; SHA256 and signature checks remain enabled. Renderer regenerates the recipes; no checksum/crypto/dependency bypass is used.
- Added `build-essential` to discipline release-dry-run and verify deb prerequisites, **also** to the later Ubuntu native-packaging and production release containers that would otherwise hit the same implicit dependency. Corrected Brew's plant to match the actual two-line system call, retaining exactly-one-match. Found a further latent Arch plant bug: deleting the final verifier leaves false x86_64 architecture selection as prepare's return value; the deliberate bypass plant now ends with an explicit successful command, so it can reach acceptance detection.
- Local red-before: RPM/provisioning contracts both fail (cargo101); Arch alias/Brew regex controls fail (Python1); actual Arch bypass prepare fails (Python1) before its witness fix. Final Python **15/15**, renderer/workflow/release/budget Rust contracts **35/35**, fmt, actual renderer, and real Arch prepare valid/wrong-key/restored **0/1/0** pass with `InvalidSignature` on the wrong key and cp/cmp/touch restoration. rpmspec/rpmbuild/makepkg are absent; dpkg-buildpackage exists but the complete native dependency set is not available. Native package execution after these fixes remains unverified.
- Hosted proof gained (supersedes only matching pre-hosting limitations above): both default artifact builds, both rendered rc/alpha Arch+Debian orderings, authenticated netns download + detected outbound plant + replay/journal growth with zero outbound attempts, amd64 Docker changed-byte/state-owner plants and restored init, and temp-root **maos-bin-only** Cargo install/version/removal. Symbol advisory stays exit1. No universal air-gap claim, arm64 container-runtime proof, four-package Cargo install proof, registry push/cosign, native recipe install/identity, or second-denominator assembly proof is inferred.
- Hosted skips: operator-prepare/publication were correctly excluded on push (not proved missing-credential skips); assembly was blocked by failed native builds. Discipline's package-recipes/netns/container legs were blocked through failed release-dry-run. Its independent IAC perf instruction +4.1% versus2% finding is not a packaging defect and is not changed here.
- Parent-authorized model-currency calibration: hosted Σ**32.983s** /26 tests exceeded default30s; cb0abc06 was29.82s. Tried one copied tree shared across all11 plant vectors with exact restoration: matched test **5.92s→5.96s**, no material reduction; reverted the helper by cp/cmp/touch, dropping no vector. Added exactly one **42s** exception (ceil32.983×1.25), owner `20-4-gemini-driver-and-endpoint-pin`; default30 and total1100 unchanged. All26 restored tests pass locally (Σ12.520s); trialΣ21.989s/final local speed are not hosted improvement proof.
- No counted Rust source moved: formatted kloc passes with xtask**44676**, aggregate**194743**, hardfail**194744**, bin**34914**, kernel tokei**21042** unchanged; no pin/headroom change. Initial cargo witness compilation timed out600s before tests; resumed compilation then produced the two required actual failing assertions. No workspace/prepush/hosted rerun, commit, push, tracker flip or protected 20-11 edit in this fix pass.



- Final scoped gates (`check-kernel-baseline`, `kloc-check`, `check-test-cost`, `check-dev-record-completeness`) all exit0. Exact commands/exits and preserved trial/final JUnit are in `landing-1/hosted-1/`; rendered development recipes are evidence only, not published production assets.
- Hosted-1 cleanup: /proc/open-fd scan found no active compiler or incremental fd;190 stale snapshots removed,7769 retained (newest per crate plus anything touched within2h). Own scratch/five caches removed. Allocated-file-block removal **2,768,224,256 B** (scratch154,988,544 B); observed filesystem free delta **2,442,948,608 B**. No deps/build/fingerprint deletion; see `hosted-1/cleanup.json` for hardlink/compression caveats.

### hosted-2 (2026-11-06; non-cargo repair phase)

- Verify run **42293312694**, attempt1, `f0c4c917`: **12 green /2 unintended reds /2 operator skips**. Both native deb/RPM builds, both Debian install/identity/purge/removal lanes, RPM recipe controls and signed four-package assembly now pass, alongside artifact builds, ordering, netns, amd64 container and maos-bin Cargo controls. The per-leg table is appended to `landing-1/hosted-1/record.md`; source metadata is retained under `landing-1/hosted-2/`.
- Arch good prepare failed on source symlinks (job128160782826:523–529), after its correct wrong-key refusal. Materialize all four binary links before verification, leaving already-regular files alone and the shared verifier's guard intact. Brew tap-new failed128 without Git identity (job128160783009:215–228); the common witness exports process-local author/committer names/emails, with no persisted config.
- Preempt formula metadata/dependency ordering under strict audit, replace class constants with lexical release/hash/suffix values and preserve all resource URL/checksum bindings. Current upstream nounzip stages by copying, so no speculative resource-link workaround remains. Existing two-line Brew and one-line Arch P5 matches stay exact. New signed-symlink/readonly-Git/layout controls fail before and pass after; final Python20/20 and shell syntax pass.
- **No cargo while Dev201b holds the slot.** No Rust source or renderer token interface changed. Actual renderer regeneration, Rust recipe/workflow contracts and rendered/hash evidence refresh remain for Main's explicit handoff. Ruby/Homebrew/RuboCop are absent; full formula execution/audit and newly fixed hosted recipe installation remain unverified. See `landing-1/hosted-2/record.md` for exact pending steps, observed commands and proof limits. No commit/push/status flip.

### hosted-3 (2026-11-06; Homebrew bridge/strict-audit repair)

- Verify run **42300114383**, attempt1, `100bb9f3`: **13 green /1 unintended red /2 operator skips**. Arch now passes its bypass oracle, real wrong-key refusal, four regular signed binaries, installed identity and removal (job128180762193; retained log528,582). Brew again fails128 at tap-new before installation (job128180762283; retained log259–288). No failed setup is called a valid negative-control red.
- Plain GIT_* identity exports proved insufficient. Add all four documented HOMEBREW_GIT_* author/committer bridge inputs, retaining the ordinary exports and no persisted config. The new readonly-Git bridge regression resets ordinary identity before exercising the documented mapping; it is not a claim of local Homebrew execution.
- Preempt concrete source-supported audit hazards: historical actual checksum rows are294 characters versus118, with four strings requiring word-array style. The Rust renderer now emits aligned multiline `%w[...]` hashes for precisely the present suffixes; Arch output/hash order is unchanged. The formula supplies an explicit version only when Homebrew URL detection differs, avoiding redundant stable GitHub-release versions while preserving file/prerelease behavior. Strict audit output is printed before test/identity checks, and its saved status still fails the witness after normal cleanup.
- Source review of Linux source-install finishing versus bottle relocation finds no reason to weaken installed-byte identity: retain the actual four-bin comparison, not a staged-copy fallback; skip_clean is not represented as an ELF relocation opt-out. Class-body locals/freeze/odie, explicit clean paths, SPDX license, resources, dependency order, test stanza, URL/version/quote style and bin.install semantics were inspected against upstream third-party-tap rules. Actual Homebrew load/import/audit/install/test/identity/cleanup remain **HOSTED-UNVERIFIED**.
- Four local controls genuinely fail before/under their plants, restore by cp/cmp/touch (all0), then final Python **24/24** and shell syntax exit0. **No cargo while Dev201b owns the slot. Unlike hosted-2, counted Rust renderer source changed:** Main must validate the compiled renderer/Rust contracts and perform exact formatted KLOC/re-pin during handoff; old counts are not current measurements. No workspace/prepush/hosted rerun, commit/push/status flip or protected20-11 edit. Own scratch allocated blocks removed **30,257,152 B**, observed filesystem free delta0; shared incremental untouched. Full sources, logs, command exits and boundaries: `landing-1/hosted-3/record.md`; hosted-2 has a superseding append.
- Update the NFR-Ops-12 deferred receipt from blanket unverified wording to the scoped development-key branch proof; `ops-rc-signed-tag` remains open and matching signed-tag evidence is still owed. No RC/NFR acceptance or tracker status is changed.

### File List

- Packaging: `packaging/verify-release.py`, `build-packages.sh`, `check-installed.py`, `witness-deb.sh`, `witness-recipe.sh`, `test_release_20_2.py`; `packaging/deb/{control,rules,changelog,source/format}`, `packaging/rpm/maos.spec`, `packaging/aur/PKGBUILD`, `packaging/homebrew/maos.rb`.
- Review helpers: `packaging/{assemble-release-packaging.py,forge-recipe-signature.py,release-run.py,wait-release.sh}`; review tests extend the existing Python/Rust suites, and ADR047/provisioning/install docs record the narrower trust/evidence claims.
- Release/container integration: `.github/workflows/{discipline,release,container}.yml`, `Dockerfile`, `.dockerignore`.
- Production Rust: `xtask/src/{render_packages,main,lib,check_ship_gate_completeness}.rs`, `crates/maos-bin/src/{main,purge}.rs`; enrollment/pins `xtask/{gate-registry,kloc}.toml`. No new maos-bin source file or environment variable was introduced.
- Tests: `xtask/tests/{package_recipes_20_2,package_workflows_20_2,release_workflow_15_4,ship_gate_binding_20_3a,macos_compile_guard_20_6}.rs`, `crates/maos-bin/tests/maos_uninstall_16_4.rs`, `tests/air-gap-netns-corroborate.sh`, `tests/air-gap-trace.py`, `tests/coverage-matrix.yaml`.
- Hosted-1 regression/calibration files: `packaging/test_release_20_2.py`, `xtask/tests/{package_workflows_20_2,check_test_budget_proven_red}.rs`, `xtask/test-budgets.toml`; model_currency_gate.rs was a reverted timing experiment, not a final change. Observed command logs, rendered development fixture and JUnit copies are under `landing-1/hosted-1/`.
- Docs: `docs/maos.dev/{install,index,run-maos}.md`, `docs/runbooks/{ag-1-air-gap-deployment,provisioning-checklist}.md`, `docs/adr/ADR-067-maos-data-root-layout-and-purge.md`, `docs/release/tag-procedure.md`.
- Records: this story; `deferred-work.md`; `20-2-deb-airgap-docker-and-formula-corrections-evidence/landing-1/` command logs, measurements and acceptance record; `.../ci/ci-20-2-verify.yml`. Tracker and protected 20-11 files intentionally unchanged.


### Change Log

- 2026-11-02: Story created at `a8af3215` (ready-for-dev; D-20-2-01…14 PROPOSED; D09 A applied, confirmation asked). Measured at creation: the `.deb` has never been buildable (no changelog; `rules:29` missing separator); all four recipes embed the dev key and `v0.5.0` while the only release verifies under the production key; no container image ever built in CI (run 36094027288); the air-gap variant's `run` is a stub and its closure links `ureq`/`tokio-postgres`; `maos purge`'s `cargo uninstall maos` hint fails (rc 101). Kernel-Δ 0 under the recommended set; D-20-2-01 B priced +38 / +32 by reverted prototype.
- 2026-11-02: Operator rulings recorded (ask s202): D-20-2-01…14 as recommended and preflight D09 A confirmed; kernel-Δ 0 and kloc ceilings per frontmatter. Status remains ready-for-dev.
- 2026-11-05: Landing-1 implementation at cbed4dad working tree; current-pin re-base recorded, conditional D-02 C applied, all operator proof legs authored, local plants/tests/gates recorded, no commit/push/status flip. Hosted acceptance remains explicitly unverified.
- 2026-11-05: Applied P1–P5/security F1–F10,F14 review corrections from 88c7277c; preserved original allowance through unrelated-drift rebase, exact xtask+3 re-pin, local red/plant/restore/final evidence, operator-only native/publication controls, and same-line rule-10 dispositions for preexisting/out-of-scope findings. No commit/push/status flip.
- 2026-11-06: Hosted-1 repair pass from ebf2ab03: RPM macro, Debian provisioning in every builder, Arch signature transport, exact Brew/Arch negative plants, local red-before/final checks, and parent-authorized model-currency single-binary calibration. Hosted pass/fail/skip boundaries recorded without rerun-to-green or status flip.
- 2026-11-06: Hosted-2 non-cargo repair phase: materialize Arch source links, supply ephemeral Brew Git identity, reorder formula metadata/dependencies with unchanged resource verification semantics, add local regressions, and record run42293312694's12 green legs and2 unintended reds. Cargo/renderer evidence explicitly deferred to Main's slot handoff.
- 2026-11-06: Hosted-3 non-cargo pass: Homebrew-specific identity bridge, logged blocking audit, concrete checksum-layout and redundant-version corrections, four before/plant/restore regressions and24 final Python passes. Record run42300114383's newly green Arch proof, remaining Brew setup red, source-reviewed installed-byte preservation, and explicit compiled-renderer/KLOC handoff.
