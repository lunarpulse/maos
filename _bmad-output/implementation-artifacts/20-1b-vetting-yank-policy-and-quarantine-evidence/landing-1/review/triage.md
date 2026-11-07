# 20-1b landing 1 — code-review triage (commit 62faff63, parent bdfc1430)

Layers: Blind (`blind.md`), Edge (`edge.md`), Acceptance (`acceptance.md`) plus the orchestrator's `security.md` (not spawned here). Orchestrator read the code at each location before rating. Read-only; no cargo/xtask run. Severity: high = intolerable, medium = tolerable, low = cosmetic/none.

**Counts:** decision_needed 5 · patch 18 · defer 10 · dismissed 9. Failed layers: none.

---
## 0. Mandatory focus — FKCS re-pin (5b3fe85c… → 136c579d…): **JUSTIFIED; ratify, with two conditions**

Evidence (independently re-derived, not taken from the evidence file):
- Recomputed the pin algorithm (`sha256(path, NUL, len u64 LE, bytes)` over the two declared files) from `git show` of both commits: `bdfc1430` → `5b3fe85c…a4f4`, `62faff63` → `136c579d…6a46`. Both match `fkcs-baseline.toml` old/new exactly.
- `git diff bdfc1430 62faff63 -- crates/maos-registry/src/admission.rs` is exactly the claimed hunk: line 388-390 `&pkg.manifest_toml` → `VettingSubject::from(pkg)`, plus `artifact_sha256`/`publisher_pubkey` in the `#[cfg(test)]` helper `attestation_for` (1237-1238). `crates/maos-skill/src/admission.rs` and every kernel path are unchanged (`git diff --stat` empty).
- The argument does **not** alter what is verified for the manifest: `From<&SignedPackage>` yields `manifest_toml: &pkg.manifest_toml` (same slice), so `sha256(subject.manifest_toml) == claim.manifest_hash` is byte-identical. It only **adds** two checks (`ArtifactHashMismatch`, `PublisherKeyMismatch`, `vetting/mod.rs:238,241`), both of which can only turn an admit into a refusal. Order unchanged: `frozen_surface_gate` → `verify_public_untrusted_baseline` (which verifies the publisher signature over the very `publisher_pubkey` and artifact bytes the subject then reuses, `admission.rs:152`) → `verify_attestation`. No loosening, no new admit path.
- Pinned negative controls (`every_declared_admission_file_reds_a_one_byte_mutation`, `frozen_snapshot…`, gate leg `admission-path-unmodified`) are unchanged and green in `fkcs-oracle-repin.stdout`/`fkcs-gate-repin.stdout`.

Conditions / residual observations (carried as P18 and D1):
1. **The fingerprint covers the call site, not the behavior change.** The semantic strengthening lives in `maos-compliance/src/vetting/{mod,attestation}.rs`, which no FKCS leg covers; the pin proves only "the two frozen-surface files changed by this hunk". Do not cite the re-pin as evidence the admission *semantics* are unchanged — they changed on purpose (AC6) outside the pinned files. The other `verify_attestation` caller (`install.rs:34`) and the `main.rs:206` precondition caller are likewise un-pinned; the latter has a real regression (D1).
2. **Process:** the baseline comment itself demands a re-pin "in the SAME commit as the change, after its review". This pin preceded review (disclosed in the baseline text). This review is the missing review for the *argument hunk*: it is conformant. P18 amends the baseline comment to say the independent review has now occurred (cite this triage) so the file no longer asserts "no independent review is claimed". No re-pin is needed unless a patch below edits `crates/maos-registry/src/admission.rs`; none planned does (P4/P7 are tests or in maos-bin).

---
## 1. decision_needed (operator choice required)

### D1 — HIGH — Vetted upgrade through the operator door can no longer succeed
`crates/maos-bin/src/main.rs:189-205` now reads `<target_manifest parent>/signed_package.json`; `operator_door.rs:1250-1262` always passes `ManifestSnapshot` = `$TMPDIR/maos-restore-<ulid>.toml` (`operator_door.rs:311-326`) and `upgrade_with_plan_guard` re-runs the closure on it (`main.rs:10096-10099`). No sidecar ever exists beside a temp snapshot ⇒ every public-vetted `maosctl spirit upgrade --vetting-attestation` is refused ("read vetted upgrade package"). The attestation now binds artifact+publisher, and the door only carries a manifest; the old check (manifest hash only) passed. No positive vetted-upgrade test exists (only refusals, `one_daemon_one_door_16_1.rs:1895-1990`), so green hid it. Secondary: a stray `/tmp/signed_package.json` would be trusted for artifact/publisher (acceptance F2; blind #9).
Options: (A) door resolves the target `SignedPackage` from the install directory of the named target (an upgrade target must be a registry-installed package) and passes it as an explicit parameter; the temp snapshot never supplies identity. (B) Declare vetted-upgrade-by-door unsupported in L1, make the refusal explicit/typed, and own the capability in the final kernel landing. Recommend **A**. Required witness either way: positive vetted upgrade (attested target admitted, swapped artifact refused, swapped publisher refused) failing before.

### D2 — MEDIUM — First-publisher pin and version immutability live only as long as the install (AC10/AC14)
`uninstall_package` (`storage.rs:283-352`) removes `installed/<id>/`; `check_publisher` derives pin and immutability only from that tree (`:373-415`). uninstall → install by another key (or changed bytes) succeeds: a name can be squatted the moment it is uninstalled, and AC14 immutability resets. Blind/Edge/Acceptance rate it real; the security layer rejected it (`security.md` "rejected: uninstall resetting the publisher pin") — conflicting; AC10 text says "install pin the first publisher key per `spirit_id`" with no lifetime. Options: (A) durable pin ledger `installed-pins.json` (id → publisher key, versions → package sha) surviving uninstall, cleared only by an explicit operator verb; (B) accept: pin is per-installation, document it in AC10/SECURITY.md and drop "cannot be squatted" for the post-uninstall case. Recommend **A** (uninstall already runs the erasure cascade and must not reopen the name).

### D3 — MEDIUM — Registry server `put` silently overwrites a same-key `name@version`
`storage.rs:587-602` `let _ = Self::check_publisher(...)?` discards the existing-version hit; `write_package` (`:512-518`) overwrites `manifest.toml`/`artifact.bin` before the atomic `signed_package.json`, readers take no lock. A pinned key can replace the bytes of a published, already-yanked or already-vetted version (vetters' attestations then fail; clients can see a torn read). AC14 covers installed generations only, so this is unclaimed scope. Options: (A) refuse republish of an existing version unless byte-identical (typed `ImmutableVersion`) and write the three files via one atomic generation swap; (B) leave mutable and document. Recommend **A**.

### D4 — MEDIUM-LOW — Renewing an expired/revoked attestation on an installed version is impossible without uninstall
`install_package` idempotent branch refuses when `stored_vetting != vetting` (`storage.rs:455-461`): a lapsed promotion (AC9) can only be renewed by uninstall → reinstall, which runs the erasure cascade (data loss) and, per D2, resets the pin. Option (A): allow an atomic `vetting.json` replacement iff package bytes and origin are identical and the new attestation verifies (vetting is evidence, not package bytes); (B) require uninstall (document). Recommend **A**.

### D5 — LOW — AC6.2 cites vetter-key enrollment/revocation (`keyring.rs:117,334`) but no shipped verb enrolls or revokes a vetter key
`vet.rs` wraps attestation issue/revoke only; AC7 vetter yank and AC9 "vetter key revoked" are reachable only with a fixture-built keyring (`yank_policy_20_1b.rs`, `vet_lifecycle_20_1b.rs`). Either add `vet enroll|revoke-key` or relabel AC6.2/AC9 witnesses as fixture-keyring based. Recommend relabel now + one-line deferred row (not silently).

---
## 2. patch (fix without further input)

Each patch needs a **failing-before witness**: the test must be written first, observed red on the L1 tree for the stated reason, then green after the fix, plus a mutation plant for the fixed guard.

### P1 — HIGH — One unverifiable yank row wedges the whole poller (AC7 propagation)  [blind+edge+auditor]
`crates/maos-registry/src/yank.rs:105,112`; `crates/maos-bin/src/yank_policy.rs:194-239`, `99-180`.
Triggers (all verified in code): vetter-origin row for an installed name@version with no `vetting.json` → `keyring=None` → `verify_entry` "no enrolled keyring" → `Err` every poll; vetter revoked after signing (`verify_enrollment` checks "active at `now`"); signer clock ahead (zero skew, see P10); `local_yank_crl`/`apply_crl` errors; `reevaluate_at` returning `Err` (corrupt install, missing keyring file, any non-Expired/Revoked rejection) from `on_poll`, which runs *before* `fetch_yanks`.
Fix: (a) `on_yank` splits "cannot verify / not actionable" (journal `yank-rejected` with reason, return `Ok` so the cursor advances) from transient faults (I/O, TL; `Err`, retry same row); (b) `poll_once` must call `fetch_yanks` even if `on_poll` fails (log, continue); (c) `reevaluate_at` handles each Spirit independently (a bad install is journaled and skipped; other Spirits still evaluated).
Failing-before witness: `poll_once` over rows `[vetter-origin row for unpromoted install, valid publisher row]` ⇒ second row delivered (before: `Err` forever, 0 delivered); `on_poll` returning `Err` still delivers rows; one corrupt install does not stop another Spirit's `expiry-lapse` observation. Plant: revert (a) ⇒ first test red.

### P2 — HIGH — Yank cursor can be poisoned/rolled back; not bound to a registry  [security F1 + blind + edge + auditor F3]
`yank.rs:106-114,121-131,188-226`; `yank_policy.rs:195-201`; `storage.rs:812`.
`sequence` is server-assigned, not signed (`yank_auth.rs:17-29`); `on_yank` returns `Ok` for uninstalled/other-version rows without any check, so one row with `sequence = u64::MAX` (hostile or on-path registry; AC15 not landed) permanently blinds the poller (persisted, `yanks_since` filters `> since`). The cursor file stores only a number at a fixed path: a different `MAOS_REGISTRY_URI`, a restored/reset registry (sequences restart at 1) or a `/tmp` default silently drop every later yank.
Fix: cursor file `{schema_version:2, registry_uri, last_seen}`; on URI mismatch reset to 0 (replay is idempotent, signed). Enforce server contiguity (server assigns `last+1`): the first row after cursor must be `cursor+1` and rows contiguous; a gap/overshoot ⇒ do not advance past it, journal `yank-sequence-anomaly`, resync from 0. Save the cursor after each handled row (not only after a whole successful poll) so a mid-list failure does not replay journaled rows. Document residual (a registry that withholds/rolls back its head cannot be detected without AC15 authenticated transport).
Failing-before witness: row with `sequence=u64::MAX` followed by a legit row at `cursor+1` ⇒ legit row still delivered; cursor from registry A with value 500 against registry B at 3 ⇒ B's yanks delivered (before: skipped). Plant: ignore URI in cursor ⇒ red.

### P3 — HIGH — Uninstall erasure cascade looks up the old private-memory identity (new storage identity regression)  [edge #5 + blind]; verified
`admission.rs:483-494` binds registry Spirits to `id-hex("name:pubhash")` (`memory/private.rs:92`), but `main.rs:10741-10760` / `maos_audit::private_tier_principal_row_count_for_spirit` (`audit/lib.rs:2081-2106`) scope on `id-hex(spirit_id)`. For a registry-installed Spirit the cascade neither removes private memory nor counts it, and the uninstall proof can report zero. Pre-existing registry-installed Spirits also lose access to memory written under the bare identity (see D-note below).
Fix: resolve the storage identity from the installed sidecar (`bind_registry_package_at` logic) and pass it to the private-memory removal/count so both `id-hex(name)` and `id-hex(name:pubhash)` owners are erased/reported; one function owns identity derivation (no duplicated format). Migration of existing bare-identity memory for already-installed Spirits: pre-release repository ⇒ record explicitly in the story as "no migration, operator re-install" unless the operator wants a copy step (add to D-list if so).
Failing-before witness: install a registry Spirit, write private memory under its bound identity, `uninstall` ⇒ private rows absent and `private_before > 0` reported (before: rows survive, count 0). Plant: use bare id in the scope ⇒ red.

### P4 — MEDIUM — AC10 witness does not drive the production binding; plant does not cover it  [auditor F4]
`yank_policy_20_1b.rs:318-383` computes `storage_identity` then calls `memory.bind_storage_identity` by hand. Production path is `load_admit_start` (`admission.rs:1033-1036`) via `installed_spirits.rs:50`, `main.rs:6187`, `main.rs:10156`; the topology arm, `gate_staged` and the enterprise control path call `gate_manifest` without `bind_registry_package_at` (reachability unproven by the author).
Fix: drive `load_admit_start`/`InstalledSpirits` boot admission with two sidecars (K1,K2), assert K2 does not read K1's spill; plant the `load_admit_start` binding line; audit the three un-bound `gate_manifest` callers and either bind or document why unreachable for registry packages.
Failing-before witness: revert `admission.rs:1033` to `&spirit_id` ⇒ the new test red (current suite stays green — that gap is the finding).

### P5 — MEDIUM — AC14 "different-registry" witness only varies the publisher key  [auditor F5]
`spirit_install_20_1.rs:307-374` second iteration uses key `0x54` ⇒ the AC10 pin fires, not the origin guard `storage.rs:442` (`stored_origin != *origin`). Ledger/matrix claim "same-key replacement and different-registry overwrite" is witnessed. The before-run failed at the first iteration, so iteration 2 was never red-before.
Fix: add same-key, other-registry (changed bytes and identical bytes) witness; plant removing the `stored_origin` comparison; re-word the matrix/ledger row to what is proven.
Failing-before witness: plant (drop origin compare) ⇒ new test red; existing tests stay green.

### P6 — MEDIUM — No witness that the cadence reaches `YankRuntime` / daemon wiring (AC7/AC9)  [auditor F6]
`on_poll` → `reevaluate_at` link (`yank_policy.rs:240`) and `PollTask::start` / `has_installs` / `MAOS_YANK_POLICY` parse (`main.rs:3283-3303`) are untested; cadence/restart tests use stub observers.
Fix: one `YankPoller` + `YankRuntime` test (fixture source, promoted install, injected expiry, cursor restart), or relabel the matrix rows "unit-level". Failing-before: default `on_poll() -> Ok(())` ⇒ red.

### P7 — MEDIUM — Remote-yank `RegistryYank` running-Spirit observation silently removed  [auditor F7]
Parent `TlYankObserver::on_yank` also called `observe_running_spirit(.., TerminalInputs{registry_yanked:true})` (ADR-056 §3 four distinguishable causes). `YankRuntime::on_yank` (`yank_policy.rs:194-239`) no longer does; not claimed/authorized anywhere.
Fix: emit the `registry-yank` observation for a matched running Spirit before `policy_request`, or record the deliberate removal in story+ADR. Failing-before: remote yank for a running Spirit ⇒ one `registry-yank` observation (before: none).

### P8 — MEDIUM-LOW — `uninstall_package` is outside the new cross-process `.mutation.lock`  [edge #7, blind, auditor F9]
`storage.rs:283-352` takes only the in-process mutex; `install_package`/`put`/`yank` take the flock. Cross-process `uninstall` ∥ `install` can lose an `installed-index.json` update or point the reuse-branch at a just-removed generation (AC14 TOCTOU surface). Fix: take `mutation_lock()` (after `INSTALLED_UNINSTALL`) in `uninstall_package`. Witness: two-process (or flock-held) test proving uninstall blocks while the lock is held (before: proceeds).

### P9 — MEDIUM-LOW — Artifact binding checks the sidecar's own `artifact_bytes`, not the deployed artifact  [blind]
`bind_registry_package_at` (`admission.rs:498`) and the patched `enforce_vetted_upgrade_precondition` (`main.rs:189-205`) trust `signed_package.json` for artifact/publisher and never compare it to the on-disk `artifact.bin`/component (`verify_installed` does, `install.rs:~105-120`). Fix: compare on-disk artifact bytes to `pkg.artifact_bytes` in both (shared helper). Witness: genuine manifest + genuine sidecar + modified `artifact.bin` ⇒ refusal (before: binds the genuine publisher's identity). Interacts with D1.

### P10 — LOW — Zero clock-skew tolerance on yank authorization  [edge, blind, auditor F12]
`yank_auth.rs:71` `issued_at_unix_ms > now` rejects any signer clock ahead by 1 ms (feeds P1). Fix: bounded skew (e.g. ≤ 5 min) as a named constant, applied in `verify` only; document. Witness: auth issued `now+1s` ⇒ accepted; `now+1h` ⇒ refused (before: both refused).

### P11 — LOW-MEDIUM — Poller lifecycle/observability: `INTERVAL_S=0`, boot-time `has_installs`, silent quarantine, 3600 s clamp  [edge #8, blind, auditor F10]
`main.rs:3288-3303`: `interval == 0` disables local expiry/revocation re-evaluation as well as yank polling (the removed `eprintln!` lines left no signal); the poller starts only if installs exist at boot, so later installs get no re-evaluation until restart; `MAOS_YANK_POLICY=quarantine` prints nothing though it has no effect in L1; `resolve_poll_interval` clamps to 3600 s, but AC7/FR59 claim ≤ 5 min (`yank.rs:169-179`).
Fix: always start the poller when a registry or any install capability exists (NullSource when no registry); `0` disables only registry fetching (or is rejected) — never local re-evaluation; clamp max to 300 s; print one boot line `yank poller interval=…s policy=…` and, for `quarantine`, `quarantine requested only (effect lands with KQ1)`. Witness: interval 0 with an expired promotion still yields the `expiry-lapse` observation (before: none).

### P12 — LOW — `reevaluate_at` re-journals the observation every poll when `policy_request` fails; mutex held across `block_on`  [edge #10, blind]
`yank_policy.rs:166-178`. Fix: insert the dedupe key only after `observe_running_spirit` succeeded and de-couple journaling from the retried policy request (or record both before releasing the lock); do not hold `observed` across `block_in_place`. Witness: failing auto-revoke ⇒ exactly one observation after N polls.

### P13 — LOW — Server stores unbounded duplicate yank rows per name@version  [edge #12, blind]
`storage.rs:~762-776` dedupe key includes authorization bytes ⇒ every re-signing (new `issued_at`) appends a row; each row re-triggers daemon policy. Fix: one row per (name, version, signer key); later identical-signer requests return the existing receipt. Witness: 3 re-signed deprecates ⇒ `yanks_since(0)` has 1 row.

### P14 — LOW — Cursor default path falls back to `/tmp` when `HOME` is unset  [blind]
`yank.rs:188-194`: a predictable shared `/tmp/.local/share/maos/registry/yank_cursor.json` lets another local user plant `last_seen_ns = u64::MAX`. Fix: error out (no cursor persistence) when neither `MAOS_REGISTRY_YANK_CURSOR_PATH` nor `HOME` is set. Witness: unset HOME ⇒ `save_cursor` errors, never writes under `/tmp`.

### P15 — LOW — Racy `set_var`/`remove_var` in registry tests  [blind]
`crates/maos-registry/tests/yank_auth_20_1b.rs` mutates `HOME`/`MAOS_REGISTRY_*` from parallel `#[test]`s without the shared lock the maos-bin file uses. Fix: one `ENV` mutex (or `serial`). No failing-before required (flake removal).

### P16 — LOW — Evidence wording over-claims
`plants.md`/matrix: "Genuine before-red" for AC6/AC14/AC16 ran on a partially edited tree (`ac6-before`/`ac14-ac16-before` stderr show new `yank.rs` warnings), not at bdfc1430; no before-red recorded for AC7.3 restart witnesses nor AC10.2 K1→K2; "real second-boot isolation" is hand-bound (P4); "different-registry" is different-key (P5); AC7/AC9 "on the cadence" is unit-level (P6). Fix: re-label to what was measured, or record real before-runs at bdfc1430 (worktree). Not-claimed labels for AC8 effect, AC11-13, AC15 are otherwise honest and consistent (story, tracker, SECURITY.md, ledger, plants.md).

### P17 — LOW — `verify_installed` swallows `exists()` errors for `vetting.json`  [blind]
`install.rs:~121`: `promotion_path.exists()` returns false on a permission error ⇒ promotion/revocation check skipped for non-vetted manifests. Fix: `try_exists`/read-and-match `NotFound`; any other error ⇒ refuse. Witness: unreadable `vetting.json` ⇒ refusal.

### P18 — LOW (process) — FKCS baseline comment must record the review
`xtask/fkcs-baseline.toml` lines 55-56 say the re-pin precedes review and "no independent review is claimed". After this review, amend to cite `landing-1/review/triage.md` §0 (hunk verified byte-for-byte, fingerprint recomputed, conditions in §0). This edits a comment only, not the `sha256` (the pin is over `admission.rs` content, unaffected). If any later patch edits `crates/maos-registry/src/admission.rs`, re-pin again in the same commit as that edit.

---
## 3. defer (real, not actionable in L1 or belongs elsewhere)

1. **Install-time yank refusal consumes an unauthenticated list; on-path stripping bypasses AC16** [security F2 MEDIUM]. Verifying entries (security's fix) does not stop *omission*; closing it needs authenticated first contact/TLS = AC15 (blocked on D-20-1b-01/02/03). Record as a named AC15 dependency in AC16, not as a closed property. (Fail-closed on forged rows is fine.)
2. **Yank authorization has no audience/nonce/max-age** [security F4, edge #9, blind]: cross-registry replay is idempotent for the same (name, version, key, reason); bind registry identity + max age with the AC15 root work.
3. **Vetter keyring read from a live path with no integrity anchor** (restoring an older keyring un-revokes) [blind]: design-level (`InstalledVetting.keyring_path`); pin keyring head hash at install in a later landing.
4. **Server vetter keyring is a startup snapshot** [acceptance F12, edge]: documented; revoked vetter can yank until server restart (daemon re-verifies with the live keyring, so with P1 fixed the row is journaled-rejected).
5. **Audit key reused as CRL signer / vetter root / attestation-revocation signer** [blind, security F5 INFO]: design smell; role-tag preimages later.
6. **Running Spirit ↔ install matched by name+version only** (a local Spirit with the same name/version is hit by a registry yank) [blind]: pre-existing identity model.
7. **`mutation_lock` has unreachable `Ok(file)` after `return` on non-unix** [edge #15]: warns/errs only on non-unix with `-D warnings`; the registry is Unix-only here.
8. **`vet revoke` rewrites the keyring without a lock** [edge #14]: single-operator CLI; add flock with the next `vet` change.
9. **Origin comparison by exact URI string** makes identical reinstall via `localhost` vs `127.0.0.1` refuse [blind]: fail-closed; normalize with AC15 URI policy.
10. **Other per-Spirit state keyed by bare `spirit_id`** (only private-memory binding uses `storage_identity`) [acceptance F4 note]: AC10 claims principal-spill only; inventory separately when the kernel landing touches identity.

## 4. dismissed (9)

- Unicode/normalization/case of `name@version`: `path_segment` (`storage.rs:863-875`) admits ASCII `[A-Za-z0-9._+-]` only, ≤128; no Unicode/normalization surface; case aliasing on a case-insensitive FS fails closed (`stored != pkg` ⇒ `ImmutableVersion`). Version strings compare exactly (`1.0.0+a` vs `+b` are distinct, acceptable).
- Yank `sequence` unsigned: inherent (server-assigned); mitigation is P2 contiguity/URI binding, not signing.
- Auto-revoke on expiry/revocation applies a permanent CRL with origin `RegistryYank` [blind]: AC9.1 says "applies the yank policy as a vetter-origin yank"; spec-conformant, and the label is `registry-yank: expiry-lapse|vetting-revocation` which names the cause.
- `block_in_place` requires a multi-thread runtime: the daemon uses default `#[tokio::main]` multi-thread; test flavor passes.
- `panic!` in `journal_terminal_observation`: moved verbatim from the parent observer, only reachable on a `serde_json` serialization failure of a plain struct.
- `deserialize_bytes`/`visit_bytes` CRL codec change: `visit_seq` retained (JSON arrays still decode), wrong lengths rejected, roundtrip tested.
- Test names `*_ac8` asserting only the typed request: labels already say "quarantine effect not claimed".
- `--no-default-features`/cfg: `yank_policy` is `#[cfg(feature = "network")]` (`lib.rs:126`), `maos-registry` optional dep; no unguarded use found. `unwrap`/`expect` after serde: none new in production paths (`expect("artifact has version parent")` at `storage.rs:488` is pre-existing on a path-join invariant).
- Security-layer rejected candidates (TOCTOU verify→install under the lock, yank-as-attestation replay, publisher signing oracle, path traversal, enrollment bypass, hardcoded keys) — re-checked: install check+write are under `INSTALLED_UNINSTALL` + flock; signature domains differ; concur. (Its rejection of "uninstall resetting the pin" is *not* concurred: see D2.)

## 5. Focus-list disposition (one line each)
- Signed yank scope/replay/rollback/restart/clock: scope covers name,version,reason,origin,key,key-id,issued_at (good); replay cross-registry → defer 2; cursor rollback/poison → **P2**; restart persistence real (atomic write + fsync) but see P2/P14; clock → **P10**.
- Vetter enrollment/revocation and attestation binding to artifact AND publisher: swaps genuinely closed in `verify_attestation`; sidecar/upgrade subject paths → **P9, D1**; vetter yank authority not tied to an issued attestation → by spec (AC7.1), noted.
- First-publisher pin / spill isolation: pin real while installed → **D2**; spill isolation wiring unproven → **P4**; erasure regression → **P3**.
- Install immutability: check+write serialized in-process and cross-process for install paths → **P8**; different-registry witness → **P5**; renewal → **D4**.
- Yanked-install refusal (AC16): refusal precedes any file write (yank check is first, before fetch and `install_package`); TOCTOU between check and a later yank is inherent → defer 1.
- Publish/deprecate auth (AC17): publish = self-signature + manifest identity + first-publisher pin (TOFU, by design); deprecate = signed, owner/enrolled-vetter; loopback bind enforced; no default-open mode found; vetter keyring absent ⇒ vetter yanks refused. Same-key republish → **D3**.
- Cadence poller: **P1, P2, P6, P11, P12, P14**.
- Typed boundary to kernel: `YankPolicyOutcome` is returned by `policy_request` but dropped by both callers (auditor F10) — covered by P11's notice; no kernel edit in L1 (`git diff` shows none).
- Pins/budget: new cadence binary has its own budget row; kloc/FKCS pins verified (§0).
