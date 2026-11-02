# 20-11 creation — cites re-measured at HEAD `a8af3215` (confidence rule 9)

Every range below was read at `a8af3215` (main; origin `47631284`). No cargo ran. "OLD" is the cite as filed in the
epic / preflight / tracker / routing audit / requirements inventory; "NEW" is the line range at HEAD.

## C — corrections to filed cites (OLD → NEW)

| # | Filed at | OLD | NEW (fact at HEAD) |
|---|---|---|---|
| C1 | E20:283 (AC1), PF:248 | `crates/maos-bin/src/main.rs:10570-10584` "labels intent `VerifiedEmpty`, pending halts and schedules `CoverageGap`" | category vector `main.rs:10812-10861`; `intent_lineage` `VerifiedEmpty` `:10845-10848`; `pending_halts` `CoverageGap` `:10849-10854`; `scheduled_invocations` `CoverageGap` `:10855-10860`. Labels unchanged. |
| C2 | S:309 (tracker), routing-audit `:25`, `:185`; requirements-inventory `:596` | `main.rs:9290-9305` | as C1 (`:10845-10860`) |
| C3 | E20:284, PF:129, PF:249 | `deferred-work.md:1206` (EC-13, large-log `forget_frames` mutex scan) | `deferred-work.md:1331` (row text unchanged, owner `20-11-erasure-inventory-closure`, open) |
| C4 | S:309 | "pending halts are not enumerated" | still true, now an explicit `CoverageGap` (`main.rs:10849-10854`; PF:248 already says so) |
| C5 | requirements-inventory `:571`, `:596`; routing-audit `:185` | FR65 routed to `16-4` | 16-4 is `maos purge` (product data removal, `crates/maos-bin/src/purge.rs`); FR65's per-Spirit cascade is `run_uninstall_cascade` (`main.rs:10454-10949`); owner rows `deferred-work.md:75` (FR65 → 20-11), `:78` (NFR-Aud-12 → 20-11) |
| C6 | E20:84, PF:85 | kernel-Δ "unresolved pending prototype" / "0–small" | priced at creation: kernel-free shapes exist for every category; three optional kernel rows measured (KH +10/+6, KS +10/+6, KF +37/+33); see `measurements.md` |
| C7 | maos-cli `cli.rs:63-66` doc string | "Uninstall a Spirit — removes the Spirit from the registry … (Story 6.5 / FR65 v0.5 structural stub; full Merkle proof lands at Story 9.2)" | false at HEAD: no uninstall path touches the installed-registry inventory (`crates/maos-registry/src/storage.rs:136-175,179-220`; no remove function in `crates/maos-registry/src`) |

## F — facts read at HEAD that the story relies on

| # | Fact | Cite |
|---|---|---|
| F1 | Two production callers of the cascade: offline one-shot child (`MAOS_ONE_SHOT=uninstall`, LOCK_EX store set) and the operator door (live daemon) | `main.rs:7360-7399`; `main.rs:10097-10126`; `operator_door.rs:1605-1623`; lock comment `main.rs:2055-2062` |
| F2 | Door uninstall unloads first (terminate pending halts with `PlannedUnload` receipts, revoke tokens, remove SCB), stops the Worker, then runs the cascade; the cascade sees no halt count | `operator_door.rs:1609-1623`, `:609`; `scheduler_loop.rs:531-593` (`terminate_spirit` `:562-569`, receipts discarded `_receipts`, second unreceipted `drain_for_spirit` `:574`, SCB removal `:577`) |
| F3 | `HaltRegistry` is per-process, in memory; an offline child starts empty; durable trace is the TL (`invoke_halt` writes an EpistemicHalt frame + journal row, then registry insert) | `halt/mod.rs:97-112`, `:523-585`; receipts `halt/termination.rs:26-123`; dry run `:389-408`; metadata `:414-433` |
| F4 | Halt resolution writes an approval decision (`halt.resolve`), not a halt frame | `halt/mod.rs:51-80` |
| F5 | `TerminationKind` = PlannedUnload, HaltAccepted, UnplannedCrash, HaltRejection, RevocationTerminated | `crates/maos-domain/src/halt.rs:218-241` |
| F6 | Schedules live only in the SCB's runtime manifest; firing state is two private per-process `DashMap`s keyed `(spirit_id, schedule_id)` and `(pid, schedule_id)`; watchdog polls `scheduler.scbs()` so SCB removal stops firing; TL row `schedule.fire:<id>` before dispatch | `schedule_watchdog.rs:90-101`, `:158-268` (TL `:246-254`), `last_fire_ns` `:126-130`; spawn consumes the Arc `main.rs:9717-9728` |
| F7 | No checked-in manifest declares `[[schedule]]` (grep `spirits`, `examples`, `crates` `*.toml`); schedules reach production only via third-party/installed manifests | grep at HEAD |
| F8 | Intent lineage (I13) persists in Distillate (kind 11) TL frames written at the writer's `(boot_nonce, spirit_pid)`; writer must equal citer | `crates/maos-iac/src/adapter/distillate.rs:121-142` (`intent_lineage` `:136`), `:348-377`, `:404-416`; type `crates/maos-domain/src/distillation.rs:95-118` |
| F9 | IAC frames carry a separate NFR-Aud-14 `intent_lineage` (audit chain, not a digest) | `crates/maos-domain/src/frame.rs:36-50` |
| F10 | Uninstall touches distillates only through per-principal forget closure; a Spirit's principal-free distillates are never scrubbed | `main.rs:10604-10657`; `transparency_log.rs:1636-1719` |
| F11 | A Spirit with no principal data and no memory residue returns `NotFound` (exit 4) and writes no proof, whatever else it left | `main.rs:10721-10741` |
| F12 | Incarnations = latest boot only (`all_boots=false`); resolver supports all boots by `(boot_nonce, pid)` | `main.rs:10553-10573`; `crates/maos-audit/src/lib.rs:1660-1767` (`:1758-1763`) |
| F13 | Proof verifier P18: any `Removed{count>0}` needs inclusion/exclusion proofs; Python verifier checks `erased_frame_proofs` and `subject_exclusion_proofs` only (Rust also accepts `redacted_principal_frame_proofs`) | `crates/maos-audit/src/erasure/proof.rs:383-396`; `tools/verify-erasure/verify.py:304-312` |
| F14 | `forget_frames` holds the TL `inner` mutex across a full-table tombstone scan and a full principal/kind-11 scan; `scrub_forget_frames` repeats a full kind-11 scan under the lock | `crates/maos-iac/src/adapter/transparency_log.rs:1555-1633`, `:1703-1718` |
| F15 | Kernel forget holds `forget_serialization` across hold check + scan + scrub; release takes it too | `crates/maos-kernel-core/src/memory/mod.rs:86`, `:448-617` (`:456-459`), `:627-635` |
| F16 | The root's late sweep re-scans outside that serializer | `crates/maos-bin/src/hook_pump.rs:792-795` |
| F17 | The door's legal-hold release calls the TL directly, bypassing the kernel serializer (the offline one-shot is LOCK_EX-exclusive) | `operator_door.rs:2083-2100`; offline `main.rs:7188-7200` |
| F18 | Door forget and the cascade run the blocking forget synchronously inside async fns on the multi-thread runtime | `operator_door.rs:2039-2046`; `main.rs:10605`; runtime `main.rs:1856` |
| F19 | TL is a rowid table (BLOB primary key); only `VACUUM INTO` (copy) exists in production | `transparency_log.rs:517-537`; `enterprise_identity.rs:426` |
| F20 | Installed packages are boot-visible through `installed-index.json`; serving roots re-admit them every boot | `storage.rs:136-175`, `:179-220`; `main.rs:4308-4313`; `installed_spirits.rs:16-60` |
| F21 | Other per-Spirit stores outside FR65's list: FR11 manifest archive (`admission.rs:847`, `main.rs:10294`), skill queue entries with author pid (`crates/maos-skill/src/store.rs:22,47,138-142`), Lifecycle Journal entries by `spirit_id` (`main.rs:10419-10451`) | as cited |
| F22 | `maos purge` deletes the erasure-proofs directory with the product data | `crates/maos-bin/src/purge.rs:56,106` |
| F23 | Coverage rows FR65 / NFR-Aud-12: `gates: []`, advisory | `tests/coverage-matrix.yaml:896-903`, `:959-965` |
| F24 | No test pins the three labels; the offline harness is `erasure_uninstall_13_5b.rs` (TL seeded in-test, real binary run) | grep `crates/*/tests`; `crates/maos-bin/tests/erasure_uninstall_13_5b.rs:1-60` |
| F25 | Root runtime is `multi_thread` | `main.rs:1856` |
| F26 | Kernel pin 26841 physical / 95 files / set hash `db39bc4a…`; tokei 21042; aggregate 191826, hardfail 191827 | `xtask/kernel-core-baseline.toml:505`; `xtask/kloc.toml:264,718` |
