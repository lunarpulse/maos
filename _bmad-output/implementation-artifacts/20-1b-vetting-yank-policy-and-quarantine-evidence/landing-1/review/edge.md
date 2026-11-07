# Edge Case Hunter — commit 62faff63 (20-1b L1) — read-only path trace

Format per bmad-review-edge-case-hunter (location / trigger_condition / guard_snippet / potential_consequence).
No severity labels by method; `note` lines are context from call-site reads, not rankings.

```json
[
  {
    "location": "crates/maos-registry/src/yank.rs:112 + crates/maos-bin/src/yank_policy.rs:194-233",
    "trigger_condition": "on_yank returns Err for one row (vetter yank vs unpromoted install, revoked vetter, signer clock ahead, bad auth)",
    "guard_snippet": "match self.observer.on_yank(entry) { Err(e) if e.is_row_permanent() => { journal_rejected(entry,&e); cache.last_seen_ns = entry.sequence; continue } Err(e) => return Err(e), Ok(()) => cache.last_seen_ns = entry.sequence }",
    "potential_consequence": "Cursor never passes the bad row; every later yank is blocked forever (head-of-line stall).",
    "note": "Concrete legit trigger: registry.deprecate accepts origin=Vetter for ANY version (handlers/deprecate.rs). Daemon on_yank loads a keyring ONLY if installed dir has vetting.json (yank_policy.rs:203-224); for a normally installed public-untrusted copy keyring=None -> yank_auth.rs:81 'no enrolled keyring' -> Err -> poll_once returns Err at yank.rs:112 each interval. Same for a vetter revoked after the row was accepted (verify_enrollment re-run at now), or daemon clock < issued_at_unix_ms (yank_auth.rs:71, zero skew tolerance). A hostile registry can insert one garbage row for an installed name@version to stall all propagation."
  },
  {
    "location": "crates/maos-registry/src/yank.rs:105 + crates/maos-bin/src/yank_policy.rs:99-150",
    "trigger_condition": "any running Spirit's local re-evaluation errors (keyring file missing/unreadable, ManifestHashMismatch, NotYetValid, audit-key load, corrupt signed_package.json)",
    "guard_snippet": "for spirit in spirits { if let Err(e) = self.reevaluate_one(&spirit, now) { errors.push(e); continue } } ; in poll_once: let pre = self.observer.on_poll(); /* still fetch yanks */ ... pre?",
    "potential_consequence": "One bad Spirit aborts re-eval of all later Spirits AND on_poll()? short-circuits before fetch_yanks, so no remote yanks are processed."
  },
  {
    "location": "crates/maos-registry/src/yank.rs:121-127,188-200 + storage.rs:812",
    "trigger_condition": "registry data reset/restored (sequence restarts lower) or MAOS_REGISTRY_URI switched while yank_cursor.json holds higher sequence",
    "guard_snippet": "store {registry_uri, registry_epoch/id} in YankCursorFile; on load mismatch or server max_sequence < cursor -> reset cursor to 0 and replay",
    "potential_consequence": "Server filters sequence > since, so new yanks <= stale cursor are silently never delivered; global single cursor path is registry-agnostic."
  },
  {
    "location": "crates/maos-bin/src/yank_policy.rs:195-201 + crates/maos-registry/src/install.rs:62-67 + yank.rs:109-110",
    "trigger_condition": "yank row polled before the package is installed (installed()==None => Ok, cursor advances), then installed from a different/mirror registry URI; also legacy sequence==0 rows",
    "guard_snippet": "persist verified yank rows (name,version,origin,seq) in a local yank journal and consult it in install()/verify_installed; at least journal-skip 'not installed' rows; warn on skipped legacy sequence==0 rows",
    "potential_consequence": "Yanked name@version installs and runs with no policy action: install checks only the source registry's list, and the daemon already discarded the row."
  },
  {
    "location": "crates/maos-bin/src/admission.rs:483-494 (storage_identity) vs crates/maos-bin/src/main.rs:10741,10750 + crates/maos-kernel-core/src/memory/mod.rs:141-143 + crates/maos-audit/src/lib.rs:2106",
    "trigger_condition": "uninstall of a registry-installed Spirit whose memory owner dir is id-hex('name:<pubhash>')",
    "guard_snippet": "uninstall cascade must derive the same identity: resolve installed signed_package.json publisher -> bind_registry_publisher identity, and pass it to principal_ids_for_spirit / private_tier_principal_row_count_for_spirit / scoped_owner",
    "potential_consequence": "Cascade queries id-hex('name') only; publisher-bound private memory is neither found nor erased and proof can report VerifiedEmpty (false erasure). Pre-existing registry installs also lose access to their prior id-hex('name') memory with no migration."
  },
  {
    "location": "crates/maos-registry/src/storage.rs:283-330 (uninstall_package) + 410-470 (check_publisher on installed/<id>)",
    "trigger_condition": "uninstall renames installed/<id> away, then another publisher (or altered bytes at same version) installs the same name",
    "guard_snippet": "keep a durable pin record outside installed/<id> (e.g. pins/<id>.json written by install_package, removed only by explicit operator action) and check it in install_package",
    "potential_consequence": "First-publisher pin and name@version immutability exist only while the directory exists; uninstall/reinstall defeats both."
  },
  {
    "location": "crates/maos-registry/src/storage.rs:283-296 vs :359-369,429",
    "trigger_condition": "uninstall_package (process A) runs while install_package (process B holds .mutation.lock)",
    "guard_snippet": "take the same flock(.mutation.lock) inside uninstall_package (or one shared lock fn on root) before load_installs()/rename",
    "potential_consequence": "Cross-process lost update on installed-index.json (load_installs->write_json_atomic) and rename of installed/<id> mid-install; the new flock only serializes one side."
  },
  {
    "location": "crates/maos-bin/src/main.rs:3288-3290",
    "trigger_condition": "daemon starts with no registry client and zero installs, installs/promotions occur later while daemon runs",
    "guard_snippet": "always start PollTask when interval>0 (NullSpiritRegistryClient source) or re-check has_installs per tick inside on_poll",
    "potential_consequence": "Running-promotion expiry/revocation re-evaluation (AC7) never runs for Spirits installed after boot until restart."
  },
  {
    "location": "crates/maos-registry/src/yank_auth.rs:19-45 (message tuple)",
    "trigger_condition": "captured signed authorization replayed to another registry that holds the same name@version from the same publisher/vetter",
    "guard_snippet": "include registry identity/audience (server id or URI hash) and a max-age window in the signed tuple; reject issued_at older than N",
    "potential_consequence": "Yank signed for registry A is accepted verbatim by registry B (no audience, no expiry, no nonce)."
  },
  {
    "location": "crates/maos-bin/src/yank_policy.rs:166-178",
    "trigger_condition": "policy_request errs (CRL sign/apply failure) after observe_running_spirit already journaled; observed.insert(key) is skipped",
    "guard_snippet": "insert key before side effects or journal observation only after policy_request succeeds",
    "potential_consequence": "Every poll interval re-journals the terminal observation (duplicate TL rows) until the failure clears."
  },
  {
    "location": "crates/maos-registry/src/yank_auth.rs:71",
    "trigger_condition": "signer clock ahead of verifier by any amount (issued_at_unix_ms > now)",
    "guard_snippet": "if self.issued_at_unix_ms > now.saturating_add(SKEW_MS) { return Err(...) }",
    "potential_consequence": "Fresh, valid cross-host signature rejected at deprecate; on the daemon the same check feeds the head-of-line stall above."
  },
  {
    "location": "crates/maos-registry/src/storage.rs:~765-775 (yank dedupe)",
    "trigger_condition": "same name@version yanked repeatedly with different reason/issued_at (distinct authorization bytes)",
    "guard_snippet": "dedupe on (spirit_id, version, origin) or cap rows per name@version",
    "potential_consequence": "Unbounded yanks.json growth (re-read in full on every yanks_since/install) and repeated policy actions/TL rows per row on the daemon."
  },
  {
    "location": "crates/maos-registry/src/storage.rs install_package idempotent branch (stored_vetting.as_ref() != vetting) + install.rs:verify_installed",
    "trigger_condition": "installed promotion attestation expires; operator reinstalls with a reissued attestation",
    "guard_snippet": "allow replacing only vetting.json (atomic) when package bytes/origin equal and new attestation verifies; keep package immutable",
    "potential_consequence": "ImmutableVersion error; verify_installed fails at next boot on expiry, so renewal requires full uninstall (which triggers the erasure cascade)."
  },
  {
    "location": "crates/maos-spirit-cli/src/vet.rs:~92-118 (Revoke)",
    "trigger_condition": "two concurrent `vet revoke` (or revoke vs enrollment edit) on the same keyring path",
    "guard_snippet": "take an flock on the keyring path (or a sidecar .lock) around read-modify-write",
    "potential_consequence": "Lost revocation: both read the old keyring, last atomic rename wins, the other revocation silently disappears."
  },
  {
    "location": "crates/maos-registry/src/storage.rs:354-369 (mutation_lock)",
    "trigger_condition": "build on non-unix target",
    "guard_snippet": "#[cfg(not(unix))] { return Err(...); } #[cfg(unix)] { flock(...)?; Ok(file) } (single tail expression per cfg)",
    "potential_consequence": "`return Err(..); Ok(file)` is unreachable code (warning; error under -D warnings) and all registry mutations hard-fail on non-unix."
  }
]
```
