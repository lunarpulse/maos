# Acceptance audit — 20-1b landing-1 (commit 62faff63, parent bdfc1430)

Scope: AC6–AC10, AC14, AC16, AC17 against `20-1b-vetting-yank-policy-and-quarantine.md` and 20-1a, plus
`commands.md`, `plants.md`, `verification-summary.json`, the story's witness matrix and the deferred-work ledger rows.
Read-only. No cargo/xtask run. The FKCS re-pin is ignored as instructed.
Priorities: P1 = fix before merge, P2 = fix this landing or re-label, P3 = low.

## Verdict
AC6, AC14 (same-key), AC16 and AC17 are implemented as specified and their refusal paths are real. Three things are
not acceptable as claimed:
1. A functional regression: the vetted-upgrade precondition can no longer be satisfied through the operator door (F2).
2. The yank poller can be wedged permanently by one row it cannot verify, which defeats AC7 propagation (F1).
3. Several "witnessed" claims in the matrix and ledger are not what the cited tests prove: AC10 wiring, AC14 different-registry,
   AC9/AC7 daemon cadence (F4–F6).

---
## Findings

### F1 (P1, conf 0.8) — One unverifiable yank row blocks the cursor and all later yanks forever (AC7)
- `crates/maos-registry/src/yank.rs:105,112`: `poll_once` does `self.observer.on_poll()?` before fetching and `self.observer.on_yank(entry)?`
  per row. On `Err` the cursor stays below that row, so every later poll re-fails on the same row and later rows are never reached.
- `crates/maos-bin/src/yank_policy.rs:194-239`, `on_yank` returns `Err` for cases that are not transport faults:
  - A vetter-origin row for an installed name@version with no `vetting.json`. `keyring = None`, so `verify_entry` fails with
    "unauthorized vetter yank: no enrolled keyring" (`yank_auth.rs`). The server accepts that row because its own keyring enrolled the vetter.
  - A row from a vetter revoked later. `verify_enrollment` rejects whenever `revoked_at <= now`, not `<= issued_at`
    (`keyring.rs` ~l.282), so rows the server accepted before revocation fail at the daemon.
  - A row whose `issued_at_unix_ms` is ahead of the daemon clock ("not yet valid", zero skew allowance, `yank_auth.rs`).
  - `local_yank_crl` failing under auto-revoke: `RevocationEntry::new` rejects an `=<version>` range that `parse_range` refuses (l.~280-300).
  - `reevaluate_at` returning `Err` (corrupt `vetting.json`, a deleted keyring file mapped to `MalformedClaim`, a bad install signature). That happens
    in `on_poll`, which runs before the fetch, so it also blocks yank delivery for unrelated Spirits.
- Trigger: any enrolled vetter yanks a version that a client has installed unpromoted; or an operator revokes a vetter whose rows
  daemons have not consumed yet. Consequence: every yank after that row stops propagating, and AC7's 5-minute propagation claim fails
  silently (only `eprintln!("yank poll failed")` every interval).
- Fix: in `on_yank`, split "cannot verify or not actionable" from "transient fault". For the former, journal a `yank-rejected` row (reason) and
  return `Ok` so the cursor advances. Reserve `Err` for I/O or TL faults. Make `on_poll` failures non-fatal to the fetch:
  log them and continue to `fetch_yanks`.
- No test covers any rejection path through `poll_once`. `yank_policy_20_1b.rs` only checks that a forged row returns `Err` in isolation.

### F2 (P1, conf 0.8) — Vetted upgrade through the operator door always fails now (AC6 caller change in `main.rs`)
- `crates/maos-bin/src/main.rs:189-205` (patched `enforce_vetted_upgrade_precondition`) now reads `target_manifest_path.parent()/signed_package.json`
  and refuses with "read vetted upgrade package: …" if absent.
- `crates/maos-bin/src/operator_door.rs:1250-1262,1340-1350` always passes `&snapshot.0`, a `ManifestSnapshot` at
  `$TMPDIR/maos-restore-<ulid>.toml` (`ManifestSnapshot::new`, l.311-326), and `upgrade_with_plan_guard` runs the same closure on that snapshot.
  No sibling `signed_package.json` ever exists, so every `public-vetted` target is refused even with a valid attestation and keyring.
  This breaks 16-1 D-16-1-W(4) and the AC9 premise that `enforce_vetted_upgrade_precondition` is the refusal point for vetted upgrades.
- Tests miss it. `one_daemon_one_door_16_1.rs` (l.1902-1989) and `spirit_upgrade_test.rs` only assert refusals
  (`vetting_precondition_failed`). No positive vetted-upgrade test exists, so the tree stayed green.
- Secondary: if `$TMPDIR/signed_package.json` exists (shared `/tmp`), the artifact hash is taken from that sidecar, not from the deployed
  component, so the new artifact binding would not cover what runs.
- Fix: carry the `SignedPackage` (or at least artifact bytes and publisher key) as an explicit parameter from the door, resolved from the install
  directory, not from a path sibling of a temp snapshot. Add a positive vetted-upgrade witness (attested target succeeds, swapped artifact refused).

### F3 (P2, conf 0.6) — Yank cursor is not bound to a registry identity; rollback or re-point silently drops yanks (AC7 cursor/restart)
- `yank.rs:121-131,195-228`: the cursor file holds only `last_seen_ns` (a bare sequence) at a fixed path. `fetch_yanks(cursor)` returns `sequence > cursor`
  (`storage.rs` `yanks_since`).
- Trigger: `MAOS_REGISTRY_URI` pointed at another registry, or the server restored or reset, so its sequence is below the stored cursor.
  New yanks with sequence <= cursor are never fetched, and `poll_once` also skips them (`entry.sequence <= cache.last_seen_ns`).
- Consequence: silent loss of yank propagation. Fix: store `{registry_uri, last_seen}` (and optionally a server epoch) and ignore or reset the
  cursor on mismatch. If the server's max sequence is below the cursor, replay from 0.
- Related, lower: `save_cursor` is only called after a whole successful poll (`yank_poller_production_loop`). A poll that fails after journaling entry 1 or
  a crash before the save replays already-journaled rows. AC7.3 says "no replay"; the witness covers clean restarts only (P3).

### F4 (P2, conf 0.85) — AC10 witness does not exercise the production binding; the plant does not cover it
- `yank_policy_20_1b.rs:318-383` computes `storage_identity` via `bind_registry_package_at` and then calls
  `memory.bind_storage_identity(1, &gated.storage_identity)` by hand. It never goes through `load_admit_start` (`admission.rs:1033-1036`), which is
  what binds in production, and never through the three call sites that opt in (`installed_spirits.rs:50`, `main.rs:6187`, `main.rs:10156`).
- The AC10 plant (`ac10-plant-diff.stdout`) mutates only `bind_registry_publisher`. Reverting `load_admit_start` to `&spirit_id`, or dropping any
  `bind_registry_package_at` call, leaves every test green.
- The topology arm (`main.rs:~5639`), `gate_staged` (`main.rs:~870`) and the enterprise control-Spirit path (`main.rs:~11899`) call `gate_manifest` without the
  bind. If any of them can load a registry-installed manifest, it takes the bare identity. I did not prove reachability.
- Completion notes say "real second-boot isolation". The story asked for the maos-bin admission level (AC10.2). Fix: drive `load_admit_start` (or
  `InstalledSpirits` boot admission) with two sidecars and assert the spill, plus plant the `:1034` binding.

### F5 (P2, conf 0.8) — AC14 "different-registry overwrite" is witnessed only with a different publisher key
- `spirit_install_20_1.rs:307-374`: iteration 2 publishes to a second server with key `0x54`; the refusal text asserted is "publisher key differs"
  (the AC10 pin), not the AC14 immutable-origin path. The same-key, different-registry, changed-bytes case is not tested.
- The branch that implements it is `storage.rs:442` (`stored != *pkg || stored_origin != *origin`). Removing the `stored_origin` comparison
  (the AC14 case that would re-home an install to another registry) keeps all tests green.
- The ac14-ac16-before run failed at the first iteration (`spirit_install_20_1.rs:329`), so the second iteration was never red-before.
- Story matrix and ledger (`deferred-work.md` F4 row) say "same-key replacement and different-registry overwrite refuse". The ledger says
  "different-key"; the matrix does not. The plant ran `yank_auth_20_1b::installed_replacement_…`, not the cited CLI test. Add a same-key,
  other-registry, same-or-changed-bytes witness, or re-label the claim.

### F6 (P2, conf 0.75) — AC9/AC7 "on the cadence" and daemon wiring have no witness
- `YankObserver::on_poll` (`yank_policy.rs:240`) is the only link from `poll_once` to `reevaluate_at`. Tests call `reevaluate_at(...)` directly. Replacing
  `on_poll` with the default `Ok(())`, or deleting the `PollTask::start` block (`main.rs:3283-3303`, including the `has_installs` gate and the
  `MAOS_YANK_POLICY` parse), is detected by nothing.
- `daemon_cursor_restart_does_not_replay_journalled_yanks_ac7` and `yank_cadence_20_1b` use stub `YankObserver`s. No test runs
  `YankRuntime` through `YankPoller`.
- The AC9 plant (`take(0)` inside `reevaluate_at`) proves the loop body matters, not that the cadence reaches it.
- The matrix claims AC7 "persisted daemon cursor" and AC9 "one observation each with policy effect" on the cadence. Add one `YankPoller` + `YankRuntime` test
  (fixture source, promoted install, expiry), or re-label as unit-level.

### F7 (P2, conf 0.6) — Remote-yank `RegistryYank` running-Spirit observation was dropped
- Parent `TlYankObserver::on_yank` (bdfc1430 `main.rs` ~l.300-340) emitted the TL row and also called `observe_running_spirit` with
  `registry_yanked: true`, producing a `vetting-terminal-observation` with cause `registry-yank` (ADR-056 §3: four distinguishable causes).
- New `YankRuntime::on_yank` (`yank_policy.rs:194-239`) journals `yank-poller:remote-yank-propagated` and a policy row but never emits that observation.
  Only the expiry and vetting-revocation causes remain reachable.
- The story does not authorize removing it; no test asserts either way (`vetting_attestation_gate.rs:341-364` tests the classifier only).
  Restore the observation, or record the deliberate removal in the story and ADR. It is also not claimed as a change anywhere.

### F8 (P2, conf 0.5) — `vet revoke` does not cover vetter-key revocation or enrollment (AC6.2 reference list)
- AC6.2 cites `keyring.rs:117,157,288,334` (`issue_event`, `push_attestation_revocation`, `verify_attestation_not_revoked`, `is_revoked`).
  `vet.rs` wraps only 157 (and the 288 verifier path). No shipped verb creates or revokes a vetter-key event.
- The AC7 vetter yank and the AC9 "vetter key revoked" cause are therefore only reachable with a fixture-built keyring (`trust::keyring()`,
  `yank_policy_20_1b.rs:~170`). The lifecycle test also supplies a fixture keyring.
- ADR-056 states `vet` does not enroll implicitly, but there is no explicit verb either. If out of scope, label that in AC6/AC9. Otherwise add
  `vet enroll|revoke-key` over `issue_event`.

### F9 (P3, conf 0.5) — Registry uninstall is not under the new cross-process lock
- The story and `SECURITY.md` say mutations are serialized across processes with `rustix::flock` (`mutation_lock`, `storage.rs:354`). `install_package` and
  `put`/`yank` take it. `uninstall_package` (`storage.rs:283-352`) takes only the in-process `INSTALLED_UNINSTALL` mutex.
- A concurrent `maosctl spirit uninstall` and `spirit install` (separate processes) can lose an `installed-index.json` update
  (each does read-modify-write) and leave a dangling or resurrected inventory entry. This is the AC14 TOCTOU surface, but only across a command pair.
- Fix: take `.mutation.lock` in `uninstall_package` too, or narrow the "cross-process" claim.

### F10 (P3, conf 0.6) — `quarantine` delivers a journal row only; no typed consumer and no operator-facing signal
- `policy_request` returns `YankPolicyOutcome`, but both call sites (`yank_policy.rs:~164-170` and `~236-243`) discard it. AC8.1 says "delivers a typed request".
  The only artefact is a TL JSON row with an outcome string.
- Nothing at boot or on stderr tells the operator that `MAOS_YANK_POLICY=quarantine` currently behaves like `warn` (no fence). Only the `env_contract.rs` purpose
  text and the TL row's `quarantine_effect` field say so. Main's split ruling makes the effect itself legitimately out of scope. The "typed boundary" is not a
  consumable boundary yet. Consider returning it through a trait the final KQ1 landing implements, and printing a one-line boot notice for `quarantine`.

### F11 (P3, conf 0.5) — Failing-before evidence is thinner than AC7.3/AC10.2 require; "before" is not HEAD
- `plants.md` and `commands.md` record before-red only for AC6 (swaps) and AC14/AC16. For AC7.3 (server restart then second yank fetched; daemon restart no replay)
  and AC10.2 (K1 then K2 refused) there is no recorded before-red; only AC7's unsigned-deprecate plant exists.
- The `ac6-before` and `ac14-ac16-before` stderr show a tree that already contains the new `yank.rs` (its warnings). The "before" is a partially
  edited tree, not `bdfc1430`. Both are still real assertion failures. The matrix wording "Genuine before-red" should say "partial-tree before".
- Plant coverage is otherwise honest: eight runtime assertion failures with exit 101 and exact restore, and AC15/AC11–13 are not planted.

### F12 (P3, conf 0.4) — Clock handling
- `yank_auth.rs` `verify`: `issued_at_unix_ms > now` refuses with no skew tolerance. A publisher or vetter machine clock ahead of the server or daemon by even
  1 ms fails `registry.deprecate` or (via F1) wedges the daemon. Add a small bounded skew window, or document it.
- Server vetter keyring is a startup snapshot (documented). A revoked vetter can still yank until the registry restarts, while daemons reject its rows (F1 interaction).

---
## Verified OK (no finding)
- AC6: `VettingSubject` is supplied independently of the claim; `verify_attestation` checks manifest, artifact and publisher; golden CBOR pin updated; ADR-056 amended.
  Both swaps are correctly re-signed, so the publisher-signature failure path cannot mask them. `ac6-both-before` shows both admitted `PublicVetted` before, then refused.
- AC7 server side: unsigned deprecate refused; publisher yank checked against the stored owner; vetter yank against operator-root enrollment; domain-separated CBOR
  message binds name, version, reason, origin, key, vetter id and time; sequences are durable and server-assigned; `since_ns == 0` replays all; duplicates
  are idempotent.
- AC10: server pin spans all versions under flock; install pin spans active and inactive generations; typed `PublisherMismatch`; long-name hashing keeps
  `id-<hex>` ≤ 255 bytes; `:` cannot collide because class names are `[a-z0-9-]`.
- AC14 same-key: guard runs before any `create_dir`; changed package, origin or promotion metadata → `ImmutableVersion`; identical reinstall reuses the original generation.
- AC16: yank refusal precedes manifest and artifact fetch and `install_package`; CLI and registry tests assert no `installed/` tree.
- AC17: publish authenticates exact bytes and manifest identity; deprecate authenticates and authorizes; refusals leave storage and `yanks.json` untouched; non-loopback bind refused.
- Cfg and pins: `yank_policy` is `#[cfg(feature = "network")]`; `MAOS_YANK_POLICY` registered `UserFacing`; the new cadence binary has its own 39 s budget row.
  `unwrap`/`expect` after serde: none new in production paths (one `panic!` on observation serialization is inherited).
- "Not claimed" labels (AC8 effect, AC11–13, AC15/F5): consistent across story, tracker, SECURITY.md, ledger and `plants.md`. No quarantine-effect, TLS or kernel claim leaks.
  Ledger rows are explicitly "open pending landing reference".
- CRL codec change (`revocation.rs` `deserialize_bytes`, `visit_bytes` + `visit_seq`) keeps JSON array input working and rejects wrong lengths.

## Summary table
| ID | Pri | AC | Type |
|---|---|---|---|
| F1 | P1 | AC7 | Poller head-of-line stall on unverifiable or unactionable rows |
| F2 | P1 | AC6/D9 | Vetted door upgrade precondition always refused (temp snapshot has no sidecar) |
| F3 | P2 | AC7 | Cursor not bound to registry identity; rollback or re-point drops yanks |
| F4 | P2 | AC10 | Admission wiring unwitnessed; plant misses `:1034` and call sites |
| F5 | P2 | AC14 | Different-registry witness uses a different key; origin guard untested |
| F6 | P2 | AC7/AC9 | Cadence to `YankRuntime` and main wiring untested |
| F7 | P2 | AC7/ADR-056 | `RegistryYank` running-Spirit observation removed |
| F8 | P2 | AC6.2/AC9 | No vetter-key enroll or revoke verb |
| F9 | P3 | AC14 | Uninstall outside cross-process lock |
| F10 | P3 | AC8 | Quarantine request has no consumer or operator notice |
| F11 | P3 | evidence | Missing before-red for AC7.3/AC10.2; partial-tree "before" |
| F12 | P3 | AC7 | Zero clock-skew tolerance; server keyring snapshot |
