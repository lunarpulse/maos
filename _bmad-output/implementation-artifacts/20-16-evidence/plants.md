# AC4 plant — route-manifest `error_codes` drift (D-20-16-08 A) — 2026-10-27

Plant count: **1** (one code removed from one file). Tree: committed candidate `38e5b52a`, clean before the plant. Exclusive cargo use, target `/mnt/build/cargo`.

## Plant

sha256 of `docs-site/route-manifest.json` before: `1ac1facef63f8b20ae527cab3b368a2e2c2e18af25529b4bac64201c546a623d`. One array element was removed; `jq -e '.error_codes|length'` → 66 (was 67), so the JSON stayed valid.

```diff
diff --git a/docs-site/route-manifest.json b/docs-site/route-manifest.json
index ba558296..91116761 100644
--- a/docs-site/route-manifest.json
+++ b/docs-site/route-manifest.json
@@ -263,7 +263,6 @@
   "expected_distinct_pages": 135,
   "error_codes": [
     "EAbiTooNew",
-    "EAbiTooOld",
     "EClassRequired",
     "ECliBinaryNotFound",
     "ECliProbeFailed",
```

## Red

```text
env -u CI -u GITHUB_ACTIONS CARGO_TARGET_DIR=/mnt/build/cargo timeout 2400 cargo run -q --locked -p xtask -- prepush
```

Exit **1**, shell wall **231.72 s**, internal wall 225.065 s. Summary: `prepush: 65 checks run, 6 skipped-with-reason, 3 red, 56 captured SKIP output line(s) (diagnostic only), wall 225.065s; landing CI remains authoritative`.

```text
RED docs:troubleshoot-bidi — docs-site/route-manifest.json / docs/errors/error-catalog.json — expected exit 0, actual exit status: 1; 0 lines omitted; Catalog codes: 67 Troubleshoot pages: 67 PASS: gate:troubleshoot-bidi — symmetric difference is empty FAIL: route-manifest error_codes != error-catalog keys in catalog not manifest: EAbiTooOld
RED docs:build — docs-site/build — required tool "npm": No such file or directory (os error 2)
RED docs:routes — docs-site/build / route-manifest.json — fresh docs build unavailable; refusing stale build route proof
```

- The AC4 red line names `docs-site/route-manifest.json` and the removed code `EAbiTooOld`.
- The docs-site change triggered the conditional docs legs; they no longer SKIP as they do on the clean tree. `docs:build` went RED because this host has no `npm` (`node` is present at `/usr/bin/node`). That is an environment refusal, not a docs defect. `docs:routes` then refused stale route proof rather than reusing an old build. So no fresh docs build or route result is claimed from this host.
- Other records were the same as on the clean tree: `ADVISORY test-budget` (local timing), SKIP cargo-deny and five dynamic `ci-pins:marker` SKIPs. Part of the wall is a maos-bin rebuild left from restoring the AC3 incremental probe edit just before (the restore changed mtimes).

## Restore and green

```text
git checkout -- docs-site/route-manifest.json
```

- sha256 after restore: `1ac1facef63f8b20ae527cab3b368a2e2c2e18af25529b4bac64201c546a623d`, the same as before (byte-identical). `git status --porcelain` was empty.
- Green rerun of the reddened check, using the same command prepush runs, from `docs-site/`: `node scripts/gate-troubleshoot-bidi.js` → exit **0** (`PASS: route-manifest error_codes == error-catalog keys`). The whole aggregate on this exact byte-identical tree is the AC3 warm run #2: exit 0, 0 red (`timings-2026-10-27.md`).

| Interval | Wall (s) |
|---|---:|
| touch → red (plant applied → prepush finished) | 236.65 |
| red → restore | 5.43 |
| touch → restore | 242.08 |
| touch → green (`gate-troubleshoot-bidi.js` exit 0) | 249.87 |
