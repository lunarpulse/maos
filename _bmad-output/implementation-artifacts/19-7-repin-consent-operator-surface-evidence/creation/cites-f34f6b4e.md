# 19-7 creation — cites re-measured at `f34f6b4e` (2026-10-28)

Method: direct `read` of every range; `git grep -n` across revisions for moved rows; clean tree. No cargo run.

## Epic §19-7 / tracker cites (OLD → NEW)

| Cite (where) | OLD | NEW @f34f6b4e | How measured |
|---|---|---|---|
| deferred-work row (epic `:205`; also `:189`, `:90`, `:142`; tracker `:290` text) | `deferred-work.md:1010` | `deferred-work.md:931` | `git grep -n -F 'cohort vectors fail on a cross-host pin mismatch' <rev> -- deferred-work.md` → f889b3e4 `:1010`, 84e0e659 `:1010`, f34f6b4e `:931` (moved by the 2026-10-27 archival) |
| "default hook returns `TimedOut`" (epic `:207`; tracker `:290`; `root_shutdown_unload_16_3.rs:1734`; `discipline.yml:1755`) | `router.rs:1235-1236` | comment `crates/maos-a2a-core/src/router.rs:1228-1230`; the hook itself `crates/maos-a2a-core/src/tofu.rs:186` (`Default` `:180-189`, field `:174-177`) | read |
| "branch structurally unreachable until two processes" (tracker `:290`) | `router.rs:1239-1243` | `router.rs:1233-1237` (sentinel check `:1349`) | read |
| `await_repin_consent` zero production callers (epic `:207`, tracker `:293`) | `tofu.rs:128,612` | holds (trait `:128-133`, impl `:612-646`) | read; `grep await_repin_consent( crates/*/src` = trait + impl + in-src tests only |
| `verify_pinned_sync` → `NotPinned` (epic `:207`) | `tofu.rs:457-466` | holds (`:457-498`, `NotPinned` `:466`) | read |
| 19-4 tracker row (epic `:205,:207`) | `sprint-status.yaml:290` | holds; 19-7 row `:293`; 21-3 durable TOFU `:318` | read |
| restart→PinInvalidated mapping (`deferred-work.md:931` text) | `router.rs:1264` | `router.rs:1258-1261` | read |
| config edit + restart (deferred text) | `router.rs:1234` | `router.rs:1228-1229` | read |
| boot-nonce debug override (`root_shutdown_unload_16_3.rs:1723`, deferred text) | `main.rs:1948`, getrandom `:1956-1959` | `crates/maos-bin/src/main.rs:2140-2146`, getrandom `:2147-2152` | read |
| receiver restart NACK (`root_shutdown_unload_16_3.rs:1727`) | `router.rs:1372-1380` | `router.rs:1360-1375` | read |

## Kernel

- `crates/maos-kernel-core/src/**.rs` physical lines (git-tracked, `xargs cat | wc -l`) = **26378** = pin `xtask/kernel-core-baseline.toml:505`; tokei 20619 = `xtask/kloc.toml:264`.
- grep `tofu|Tofu|repin` in `crates/maos-kernel-core/src` → no TOFU/re-pin code (only `boot_nonce` fields of SCB/caps/halts). The re-pin store is `maos-a2a-core`, the door `maos-control` + `maos-bin`, the Approval Decision Log writer `maos-iac` (`crates/maos-iac/src/adapter/transparency_log.rs:2168`), `FrameKind` `maos-iac` (`:57`).

## Kloc

`tokei-f34f6b4e.json` (same file): the kloc-check instrument replicated (tokei 14.0.0, argv of `xtask/src/kloc_check.rs:294-318`) — maos-a2a-core 4928, maos-a2a-tcp 1442, maos-control 1344, maos-cli 6155, maos-bin 29497, aggregate 188504.
