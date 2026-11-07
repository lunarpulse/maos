# Security review: 20-1b landing 1 (commit 62faff63) — security-reviewer layer

Saved by the orchestrator (reviewer had no write tool). Read-only; no cargo. Counts: 1 High, 2 Medium, 1 Low, 1 Info; 6 rejected candidates. Line cites marked `~` in the original are approximate: spot-check before citing.

## FKCS re-pin verdict (mandatory focus): justified
`verify_attestation` argument `&pkg.manifest_toml` -> `VettingSubject::from(pkg)` (`admission.rs:388-390`); the `From` impl (`crates/maos-compliance/src/vetting/mod.rs:203-210`) maps the same manifest slice, so the manifest-hash check is byte-identical; the subject additionally checks the artifact hash (`ArtifactHashMismatch`, mod.rs:238) and publisher-key equality (mod.rs:241), required by AC6. Test fixture change at `admission.rs:1237-1239` matches the new subject shape. Admission order frozen_surface_gate -> verify_public_untrusted_baseline -> verify_attestation (`admission.rs:354-390`). The pin preceded review (process concern only; this review is the re-examination `fkcs-repin-review.json` required).

## Findings
- **F1 HIGH — yank cursor poisoning.** `yank_auth.rs:17-29` signed tuple omits `sequence`; `yank.rs:~108-114` `poll_once` sets `cache.last_seen_ns = entry.sequence` after `observer.on_yank(entry)?`; `yank_policy.rs:~194-202` `on_yank` returns Ok without signature check for uninstalled spirit / version mismatch; cursor persisted (`yank.rs:~140-200`). A hostile or on-path registry (dev transport is plain http, `http.rs:34-38`; AC15 not landed) returns `sequence: u64::MAX` for an uninstalled name -> cursor poisoned permanently, all later yanks skipped. Fix: advance only to max(sequence) of entries that passed `yank_auth::verify_entry`; reject sequences above a server-authenticated head; later sign the response envelope with the AC15-pinned registry key.
- **F2 MEDIUM — install-time yank refusal consumes an unauthenticated list (AC16 bypass).** `install.rs ~40-60` calls `client.yanks_since(0)` and refuses on any matching entry without signature check; an on-path attacker strips the entry; the genuinely signed package installs. Fix: honor only entries passing `yank_auth::verify_entry` with the package's publisher key (fetched after package signature verification, before persistence), or require an authenticated list head.
- **F3 MEDIUM — registry `put` silently overwrites a same-key name@version.** `storage.rs:587-602` discards the existing-version result, `write_package` overwrites (`:~512`); `server.rs:331-334` publish routes to `put`. Not covered by any AC (AC14 covers installed generations). Fix: refuse when the version dir exists with different `signed_package.json` bytes (immutable per version) + plant in the registry tests.
- **F4 LOW — yank authorizations lack nonce/audience/max age** (`yank_auth.rs:17-29,41-56`; only `issued_at > now` rejected): cross-registry replay of an owner authorization (idempotent). Fix: bind registry identity and nonce/max age.
- **F5 INFO — no per-role domain separation across signing roles** (attestation, publisher, keyring events, sealed bundles, CRLs, yank tuples share the operator/audit key where applicable; no exploit today because preimage formats differ). Hardening: role-tag every preimage; refuse enrolling a key that is also the operator root or a publisher key.

## Rejected candidates
In-process TOCTOU verify->install; yank authorization replayed as attestation; publisher signing oracle; `path_segment` traversal/Unicode confusion; uninstall resetting the publisher pin; keyring enrollment bypass; hardcoded key material (test seeds only).

## Not covered
AC11-AC13 and the AC8 quarantine effect (kernel landing), AC15/F5 transport/root pinning (pending).
