# Binary-size attribution: Stories 18-4 and 18-5

Measured 2026-10-26, before changing the ceiling. No tracked source, manifest, feature, or gate changes were made. All Cargo builds used `/mnt/build/cargo`; older trees were checked out in one detached `/tmp/maos-size-attribution` worktree, subsequently removed. No tool was installed.

## Decision summary

- **The first landing breach is real:** the archived CI jobs measured **33,373,312 B** at `b96d123a` and **33,666,176 B** at `79f49709`: **+292,864 B**, consuming the previous **181,120 B** margin and exceeding 32 MiB by **111,744 B**.
- The three controlled local builds measured **33,601,008 / 33,795,056 / 33,854,712 B**, respectively. Local deltas are **+194,048 B** for 18-4 and **+59,656 B** for 18-5. These are a separate, same-substrate attribution series, not replacements for the CI observations.
- **No new dependency or Cargo feature caused either step.** `Cargo.lock`, the root manifest, and all crate manifests are unchanged across these commits. The first step adds Butler's acceptance/scene/digest behavior and its serialization/collection monomorphisations. The second adds context-budget reporting, IAC context signals, and the corresponding frame/async/serialization generics.
- There is a measured alternative to immediately raising the ceiling: changing **only the executable's codegen units to 1**, leaving dependency libraries unchanged, produced **30,304,632 B**, saving **3,550,080 B** and leaving **3,249,800 B (3.099 MiB)** below the existing 32 MiB ceiling. This is size evidence, **not latency or behavioral validation**.
- **Recommendation:** first evaluate a narrowly scoped codegen-units change against the normal CI compiler, §13.1 latency benches, and behavior gates. Do not silently raise the ceiling or change panic semantics. If the release must retain its current build profile, the attributed, rule-compliant fallback ceiling is **36,962,304 B (35.25 MiB)**: HEAD plus **3,107,592 B (2.964 MiB)** explicit headroom for the remaining Epics 18–21. No ceiling has been changed here.

## Measurement method and substrate

The gate at `tests/integration/onb_nfr2_timing.sh:56-62,115-171` builds:

```sh
cargo build -p maos-bin --release --locked
strip target/release/maos
stat -c%s target/release/maos
```

No feature flags: `maos-bin` defaults to `network`; `wasm-host` remains **off**. There is no workspace release-profile override. Cargo's documented release defaults are opt-level 3, codegen-units 16, local ThinLTO (`lto = false`), no debug info, and panic unwind. The dev/test-only overrides in the workspace manifest do not affect these builds. See [Cargo profile defaults](https://doc.rust-lang.org/cargo/reference/profiles.html#release).

For every baseline tree, the exact build arguments and `strip` operation above were used, with `env -u CI -u GITHUB_ACTIONS CARGO_TARGET_DIR=/mnt/build/cargo`. The actual output path is therefore `/mnt/build/cargo/release/maos`, not the script's literal `target/release/maos`. Before stripping, a copy was retained for `nm -C --print-size --size-sort` and `size -A`; an identical copy was stripped and its file length measured. This preserves symbols without changing the measured executable. Shared artifacts were reused: the script's `cargo clean`, response execution, and timing assertions were **not** run. This investigation reproduces the build/profile/features/strip **size measurement**, not a cold-build timing run.

Local substrate:

- `rustc 1.98.1 (48a229cea 2026-09-01)`, LLVM 22.1.8, `x86_64-unknown-linux-gnu`.
- `cargo 1.98.1 (797e8a9bc 2026-08-05)`.
- GNU `strip`, `nm`, and `size` 2.44.
- No `RUSTFLAGS`, encoded Rust flags, or release-profile environment overrides were present in the build shell.
- `cargo bloat --version` reported that the subcommand was absent; **nm was used instead**, with no installation.

### Why local sizes differ from the landing gate

The archived CI logs identify **rustc 1.99.1 (58cf8f9c3 2026-10-13), LLVM 23.1.1** for both landings, on the Ubuntu 24.04 runner image `20261012.334`. `rust-toolchain.toml` says `stable`, not an immutable compiler version. Thus the local and CI compiler/link substrates differ. The local baseline already exceeds 32 MiB; this does **not** contradict the observed earlier CI pass. The precise contribution of compiler versus the rest of the runner substrate was not separately measured.

Sources:

- [Last passing landing, job 121301286078](https://github.com/lunarpulse/maos/actions/runs/40237233599/job/121301286078): `maos-bin stripped size: 33373312 bytes (limit: 33554432)`.
- [First breaching landing, job 121345745513](https://github.com/lunarpulse/maos/actions/runs/40251998973/job/121345745513): `maos-bin stripped size: 33666176 bytes (limit: 33554432)`.

The CI increase is **98,816 B larger** than the local increase. Do not mix the CI baseline with local section or symbol counts, or present local symbol deltas as an exact allocation of the CI's +292,864 B.

## Stripped size matrix

One MiB is 1,048,576 bytes. Margin is `33,554,432 - measured_size`; negative means a breach.

| Commit | Tree | Local stripped bytes | MiB | Step delta | Margin to 32 MiB |
|---|---|---:|---:|---:|---:|
| `b96d123a` | Last CI pass; 18-3 reviewed | 33,601,008 | 32.044418 | — | -46,576 |
| `79f49709` | First CI breach; 18-3 patches + 18-4 | 33,795,056 | 32.229477 | +194,048 | -240,624 |
| `be12ca0b` | HEAD; adds 18-5 | 33,854,712 | 32.286369 | +59,656 | -300,280 |

HEAD has **no remaining space** under the current ceiling on this local substrate: it needs at least **300,280 B** removed merely to meet 32 MiB, before restoring growth headroom. Across both local steps the increase is **253,704 B**.

Stripped SHA-256 identities:

| Commit | SHA-256 |
|---|---|
| `b96d123a` | `75b05063c951618f8de3b3d29b6e452e34d9d1325e02df8e3f867a72b88d7f9a` |
| `79f49709` | `060f5e4d1f1946c22fe07e396cd2f85666ce03b92291953bd56dd4b3a41ffc0b` |
| `be12ca0b` | `34950edbe6698ac162d0be9d8df1447a4643e266d2628304d2d530e8f13cacb9` |

## ELF section attribution

`size -A` on the unstripped copies; sizes are bytes. Every changing section is listed, plus the unchanged `.data` baseline. Other sections are unchanged.

| Section | `b96d123a` | `79f49709` | `be12ca0b` | 18-4 delta | 18-5 delta |
|---|---:|---:|---:|---:|---:|
| `.text` | 24,194,146 | 24,355,106 | 24,384,562 | +160,960 | +29,456 |
| `.eh_frame` | 2,719,004 | 2,731,416 | 2,747,228 | +12,412 | +15,812 |
| `.gcc_except_table` | 2,343,108 | 2,353,056 | 2,351,092 | +9,948 | -1,964 |
| `.rodata` | 2,183,656 | 2,187,752 | 2,193,704 | +4,096 | +5,952 |
| `.rela.dyn` | 1,004,088 | 1,007,352 | 1,010,016 | +3,264 | +2,664 |
| `.data.rel.ro` | 721,920 | 724,160 | 726,152 | +2,240 | +1,992 |
| `.eh_frame_hdr` | 349,988 | 351,636 | 353,876 | +1,648 | +2,240 |
| `.got` | 42,376 | 42,696 | 42,824 | +320 | +128 |
| `.data` | 26,696 | 26,696 | 26,704 | 0 | +8 |
| `.dynsym` | 4,896 | 4,920 | 4,920 | +24 | 0 |
| `.relro_padding` | 3,232 | 1,760 | 3,512 | -1,472 | +1,752 |
| `.dynstr` | 2,758 | 2,764 | 2,764 | +6 | 0 |
| `.bss` | 1,696 | 1,696 | 1,712 | 0 | +16 |
| `.gnu.version_r` | 512 | 528 | 528 | +16 | 0 |
| `.gnu.version` | 408 | 410 | 410 | +2 | 0 |
| **All section sizes** | **33,602,002** | **33,795,466** | **33,853,522** | **+193,464** | **+58,056** |
| **Stripped file length** | **33,601,008** | **33,795,056** | **33,854,712** | **+194,048** | **+59,656** |

Section sizes are not file length: ELF headers, section tables, alignment, stripping, and sections with memory size but no file payload prevent direct equality. The file-delta minus section-delta residual is **584 B / 1,600 B**. In the first step `.text` explains approximately 83% of file growth; in the second, code explains approximately 49%, with unwind metadata an important additional contributor.

## Per-crate symbol attribution

The tables aggregate **code symbols** (`T/t/W/w`) by the first demangled crate path component, including a leading `<crate::Type as Trait>` implementer's crate. `maos` is the executable crate; `maos_bin` is its separately linked library. `core`, `alloc`, serde, Tokio, and TOML roots include monomorphisations for downstream MAOS types, not just their libraries' source code.

Exact `(address, size)` aliases are counted once per executable. Identically named functions at **different addresses** remain distinct emitted code and are counted. This matters: the normal HEAD executable contains three distinct copies of `maos::main::{closure#0}`. Crate-root grouping identifies emitted symbols, not exclusive feature ownership; optimizer/inlining/codegen placement can move costs across roots even when that crate's source is unchanged. In particular, closure ordinal names are unstable between revisions and are not a reliable function-by-function cross-version identity.

| Root | `b96d123a` code B | `79f49709` code B | `be12ca0b` code B | 18-4 delta | 18-5 delta |
|---|---:|---:|---:|---:|---:|
| `butler` | 38,459 | 89,427 | 94,154 | **+50,968** | +4,727 |
| `toml_edit` | 1,081,374 | 1,119,711 | 1,127,659 | **+38,337** | **+7,948** |
| `core` | 4,015,333 | 4,041,448 | 4,059,382 | **+26,115** | **+17,934** |
| `maos` | 1,737,897 | 1,762,626 | 1,712,047 | **+24,729** | -50,579 |
| `maos_acp` | 21,609 | 45,192 | 48,354 | **+23,583** | +3,162 |
| `serde_json` | 1,605,499 | 1,623,087 | 1,652,668 | +17,588 | **+29,581** |
| `maos_bin` | 650,917 | 659,626 | 661,724 | +8,709 | +2,098 |
| `alloc` | 1,550,539 | 1,555,919 | 1,556,553 | +5,380 | +634 |
| `rustls` | 521,008 | 526,148 | 526,148 | +5,140 | 0 |
| `observer` | 9,608 | 12,186 | 9,608 | +2,578 | -2,578 |
| `maos_iac` | 177,331 | 177,643 | 210,516 | +312 | **+32,873** |
| `maos_domain` | 323,720 | 324,217 | 329,262 | +497 | +5,045 |
| `maos_a2a_core` | 160,469 | 160,595 | 164,441 | +126 | +3,846 |
| `hashbrown` | 342,600 | 343,187 | 346,901 | +587 | +3,714 |
| `tokio` | 597,090 | 596,699 | 610,119 | -391 | **+13,420** |
| `serde` | 156,389 | 130,921 | 134,304 | -25,468 | +3,383 |
| `std` | 716,886 | 716,441 | 718,078 | -445 | +1,637 |
| `maos_kernel_core` | 1,110,193 | 1,110,173 | 1,058,816 | -20 | -51,357 |

Top five **positive** contributors are bolded for each step; negatives and other positive roots must not be omitted when interpreting totals. All deduplicated code-symbol deltas, including roots not shown, sum to **+159,452 B / +26,653 B**. The remaining `.text` changes, **+1,508 B / +2,803 B**, are not allocated to a crate by this sized-symbol accounting (including alignment and unnamed code). This is close reconciliation, not a claim to exact feature-level ownership.

### Step 1: `b96d123a` → `79f49709`

The source inventory changes Butler, the Spirit SDK's new clock/principal surface, and root wiring; it adds no dependency. Notable emitted-symbol evidence:

- `butler::Butler::scene_step`: **+9,053 B**; `digest_value`: **+8,097 B**; `AcceptanceFixture::parse`: **+4,375 B**; `digest_over`: **+3,708 B**; `handle_option_pick`: **+3,204 B**; `record_decision`: **+2,932 B**. These are the concrete acceptance-ledger, compressed-clock scene, and digest capability shipped by 18-4.
- New `serde_json` `deserialize_struct` instantiations for `AcceptanceRecord`, `FlaggedPayload`, `FixtureHeader`, and `FixtureLine`: **5,222 / 4,236 / 3,307 / 2,908 B**. These directly connect serializer growth to the new behavior.
- New `alloc` B-tree insertion instantiations for the acceptance ledger, fixture-day map, and mode-rate map: **4,972 / 4,435 / 3,713 B**. Other collection symbols shrink, so these gross additions exceed `alloc`'s net +5,380 B.
- `core::ptr::drop_glue::<butler::Butler>` grows **3,080 B**; new/changed async and closure drop glue contributes to the `core` net.
- `maos_bin::escape_detector_consumer::anomaly_flagged_payload`: **+3,265 B**; pid-bound principal reads/writes: **+1,497 / +1,014 B**; run-argument parser: **+1,383 B**.
- The large `toml_edit` and `maos_acp` rows are **not new dependencies or newly implemented protocol features**. Their larger out-of-line symbols include deserialization for existing `TcpA2AConfig` (**9,015 B**), `PinnedFingerprint` (**8,171 B**), and `AcpFrameIn` (**22,484 B**). Concurrent reductions include `serde` **-25,468 B**, `maos_a2a_tcp` **-7,716 B**, and `maos_cohort` **-6,369 B**. [INFERENCE] This is changed monomorphisation/inlining placement in the rebuilt graph, rather than a fresh TOML or ACP dependency cost; root totals are therefore more reliable than attributing each positive symbol in isolation.
- The SDK acquires **2,323 B** of standalone code symbols where it previously had no standalone code-root entry. Its dependency already existed; a newly visible symbol root is not a new crate.

### Step 2: `79f49709` → `be12ca0b`

Again no manifest/lockfile changes. The source changes add context budgets, per-pid reports and admission gates, context frames, and their root/Spirit bindings. Notable symbols:

- `maos_iac::adapter::IacBusAdapter::deliver_typed` grows **13,777 B**; new `context_signal::deliver` contributes **10,960 B**; `ContextSignalAdapter::report_context_used` contributes **1,293 B**.
- New context-signal future drop glue contributes **4,063 B**; `deliver_typed` drop glue grows **3,929 B**; `FramePayload` drop glue grows **3,046 B**. These connect generic/async growth and the unwind-section increase to the new typed frame path.
- `serde_json`'s `serialize_newtype_variant::<ContextEnvelope>` adds **3,126 B**. Existing deserializer/serializer placements also change, including scope-map serialization **+8,752 B**, and Butler CalendarEvent/CommsMessage deserialization **+5,257 / +4,802 B**. Do not count those as newly added calendar/comms features.
- `toml_edit`'s `RawBudget` deserializer grows **1,085 B**, consistent with the added manifest thresholds; the root's net is +7,948 B.
- `maos_capability` adds **1,775 B** net, including `ContextBudget::report` (**354 B**) and `admits` (**153 B**); its `DashMap<u32, ContextLevel>` insertion/removal generic symbols appear under `dashmap` (**833 / 661 B**).
- There are **no newly appearing code crate roots**. The largest offsets are `maos_kernel_core` **-51,357 B** and executable `maos` **-50,579 B**. These root decreases do not imply the context gate was removed: substantial new code appears in IAC, serialization, and generic roots while optimization and placement change elsewhere.

## Remediation options, with numbers

### A. Attributed ceiling re-base, retaining the current product/profile

The gate's rule is **measured size plus explicit approximately 2.75–3 MiB growth headroom**, not an arbitrary next large integer ceiling. Use the largest measured default-profile tree, HEAD, rather than ignoring 18-5:

```text
Measured HEAD                       33,854,712 B = 32.286369 MiB
Proposed ceiling                    36,962,304 B = 35.250000 MiB
Explicit remaining growth budget     3,107,592 B =  2.963631 MiB
```

This is a **+3.25 MiB** change to the old ceiling, but only **2.964 MiB** of free space above today's measured binary. It covers the remaining Epics 18–21 as an explicit finite allowance; it is not a forecast that their unknown future work will fit. By comparison, a 35 MiB ceiling leaves only **2,845,448 B (2.714 MiB)** on this substrate, below the nominal 2.75 MiB lower end; 36 MiB leaves 3.714 MiB and exceeds the precedent's nominal headroom range. Quarter-MiB rounding yields 35.25 MiB without abandoning that rule.

This option preserves the existing runtime capabilities and compile strategy. If selected, the actual gate edit must carry the measurements and attribution, and the next breach must again attribute. The local/CI substrate caveat must stay visible; no CI HEAD size was measured here.

### B. Release compilation changes

A cheap, executable-only experiment was performed without editing Cargo.toml:

```sh
env -u CI -u GITHUB_ACTIONS CARGO_TARGET_DIR=/mnt/build/cargo \
  cargo rustc -p maos-bin --bin maos --release --locked -- -C codegen-units=1
# Copy, strip, and stat exactly as for the three baseline binaries.
```

| HEAD variant | Stripped bytes | MiB | Difference from default | Margin to 32 MiB |
|---|---:|---:|---:|---:|
| Default release | 33,854,712 | 32.286369 | — | -300,280 |
| Executable only, codegen-units=1 | **30,304,632** | **28.900749** | **-3,550,080** | **+3,249,800** |

The trial reduces `.text` by **2,737,696 B**, `.eh_frame` by **314,976 B**, `.gcc_except_table` by **241,608 B**, `.rodata` by **110,608 B**, `.rela.dyn` by **61,080 B**, and `.data.rel.ro` by **48,360 B**. The executable's main future has **three distinct emitted copies totaling 617,409 B** in the default build, versus **one copy of 203,917 B** in the trial. That illustrates actual duplicate/generic code removal rather than a strip-only accounting trick.

Scope matters: this changes codegen units for the **`maos` executable rustc invocation only**. It does not measure `CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1` for the whole workspace, nor a `[profile.release.package.maos-bin]` override, which would affect the package's library too. Either production configuration needs its own size and latency result. A normal, unmodified release build was run after the experiment to restore the shared target's normal build output.

- **codegen-units=1:** strongest measured size-only option; saves substantially more than the current breach and restores roughly the script's desired headroom without removing features. §13.1 latency, runtime behavior, CI-toolchain size, and cold compile/link costs were **not** measured in this investigation. Do not treat the size-only trial as release approval.
- **opt-level `z` / fat LTO:** not measured; the gate explicitly records that these regress the §13.1 latency benches. No numerical savings or new performance claim is asserted here. Neither is needed to obtain the measured codegen-units saving.
- **panic abort:** not measured. HEAD has **5,098,320 B** in `.eh_frame` plus `.gcc_except_table`, but that is **not** a reclaimable-size estimate: dependencies, unwind tables, compiler decisions, and runtime semantics matter. Abort changes a task/process panic from unwinding to process termination, so it is not a size-only substitution. A full panic-strategy experiment rebuilds the dependency closure and was not part of the cheap trial.

### C. Move reference Spirits/features out of the default executable

The reference Spirits are unconditional dependencies at `crates/maos-bin/Cargo.toml:80-89`; `network` is default-on and `wasm-host` is already off. Thus merely avoiding `--features wasm-host` cannot reclaim the breach, and reducing the network surface would change the default evaluator/product rather than reproduce it.

HEAD code symbols with direct reference-Spirit crate roots:

| Reference root | Direct code bytes |
|---|---:|
| `butler` | 94,154 |
| `researcher` | 52,353 |
| `maos_digest` | 20,275 |
| `observer` | 9,608 |
| `orchestrator` | 4,550 |
| `architect` | 4,078 |
| `nash` | 3,042 |
| `reviewer` | 2,823 |
| `mira` | 2,687 |
| `worker` | 0 standalone sized code bytes |
| **Total** | **193,570** |

Removing just those direct-root bytes would still leave **106,710 B** of the local breach. However, that subtraction is **not an actual feature-removal build**: downstream monomorphisations, wiring, metadata and shared libraries change too. As an additional inventory, code symbols whose demangled names mention any of those Spirit crate paths total **762,499 B**, spread across roots including generic support; this is a candidate-impact inventory, **not an independently removable or guaranteed saving**. The zero worker row means no standalone retained code under that root, not that Worker functionality has zero size.

[INFERENCE] A deliberate slim evaluator binary could reclaim more than direct-root bytes by removing its scene/fixture parsing, Spirit-specific glue, and associated generics together. But moving Butler/Researcher out would remove the capabilities these stories ship, and removing the other Spirits requires a real feature/binary composition change. No numerical net saving can honestly be promised without that cut and a fresh linked measurement. This is a product-packaging choice, not an immediate, source-free remediation. Prefer the measured compilation option before spending product scope.

## Commands used and reproducibility

The sequence below records the substantive measurement operations. The worktree switched commits only after the preceding build and capture completed. Copies and counters were driven by an in-memory Python collector; the shell spelling below expresses the same copy/strip/stat operations.

```sh
rustc -vV
cargo --version
strip --version
nm --version
size --version
cargo bloat --version                   # absent; no installation

git diff --numstat b96d123a 79f49709 -- crates spirits Cargo.lock Cargo.toml
git diff --numstat 79f49709 be12ca0b -- crates spirits Cargo.lock Cargo.toml
git diff --name-only b96d123a be12ca0b -- Cargo.lock Cargo.toml '**/Cargo.toml'
# The final command emitted no paths: manifests and lockfile unchanged.

git worktree add --detach /tmp/maos-size-attribution b96d123a
# In that worktree:
env -u CI -u GITHUB_ACTIONS CARGO_TARGET_DIR=/mnt/build/cargo \
  cargo build -p maos-bin --release --locked
cp /mnt/build/cargo/release/maos /tmp/maos-size-evidence/b96d123a.unstripped
nm -C --print-size --size-sort /tmp/maos-size-evidence/b96d123a.unstripped
size -A /tmp/maos-size-evidence/b96d123a.unstripped
cp /tmp/maos-size-evidence/b96d123a.unstripped /tmp/maos-size-evidence/b96d123a.stripped
strip /tmp/maos-size-evidence/b96d123a.stripped
stat -c%s /tmp/maos-size-evidence/b96d123a.stripped

git switch --detach 79f49709
# Repeat the identical build/copy/nm/size/strip/stat sequence, label 79f49709.
git switch --detach be12ca0b
# Repeat the identical build/copy/nm/size/strip/stat sequence, label be12ca0b.
# Run the executable-only codegen-units trial above and capture identically.
# Run the original cargo build command again to restore the default output.

gh api repos/lunarpulse/maos/actions/runs/40237233599/jobs --paginate --slurp
gh api repos/lunarpulse/maos/actions/runs/40251998973/jobs --paginate --slurp
gh api --allow-escape-sequences repos/lunarpulse/maos/actions/jobs/121301286078/logs
gh api --allow-escape-sequences repos/lunarpulse/maos/actions/jobs/121345745513/logs

git worktree remove /tmp/maos-size-attribution
```

Collector accounting, sufficient to reproduce the crate tables from each nm output:

```python
import collections, re
seen = set()
code_by_crate = collections.Counter()
for line in nm_output.splitlines():
    m = re.match(r'^([0-9a-fA-F]+)\s+([0-9a-fA-F]+)\s+(\S)\s+(.*)$', line)
    if not m:
        continue
    address, length, kind, name = m.groups()
    key = (int(address, 16), int(length, 16))
    if key in seen:
        continue
    seen.add(key)
    if kind not in 'TtWw':
        continue
    owner = re.match(r'([A-Za-z_][A-Za-z_0-9]*)::', name.lstrip('<').lstrip('&').removeprefix('mut '))
    root = owner.group(1) if owner else '[unqualified/C/other]'
    code_by_crate[root] += int(length, 16)
```

Temporary raw binaries, nm listings, section listings, JSON measurements and downloaded CI logs were retained outside the repository in `/tmp/maos-size-evidence` for this investigation. The permanent deliverable is this report, not a new measurement harness. No tests or latency benchmarks were run, no source was changed, and no commit or push was made.

## Landing ruling applied — package-only codegen units (2026-10-26)

The operator selected `[profile.release.package.maos-bin] codegen-units = 1`
for the 18-4 landing repair, without changing the **33,554,432 B (32 MiB)**
limit. This package override applies to both the package's library and
executable; it is not the earlier executable-only experiment above.

At the patched `be12ca0b` working tree, the gate's exact normal build
`env -u CI -u GITHUB_ACTIONS CARGO_TARGET_DIR=/mnt/build/cargo cargo build -p maos-bin --release --locked`,
then `strip /mnt/build/cargo/release/maos` and `stat -c%s`, measured
**29,864,312 B (28.480827 MiB)**, leaving **3,690,120 B** under the unchanged
ceiling. Stripped SHA-256:
`a74d019fd0900e701a75c57d207d2b4f323dc1605c441cfa6aa61b0629a661fb`.
Cargo reported **47.59 s** for the cached build; this is not a cold-build
timing result. Checked-in command output:
[`18-5-evidence/review-release-size.log`](../18-5-evidence/review-release-size.log).

The local focused behavioral suites are recorded with Story
`18-5-context-token-budget-accounting`. CI §13.1 latency gates remain the
landing-run validation; no local latency result or hosted Actions pass is
claimed. The operator's conditional 35.25 MiB fallback is **not applied**.

