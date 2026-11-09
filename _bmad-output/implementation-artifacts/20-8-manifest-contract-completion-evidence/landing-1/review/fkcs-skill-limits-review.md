# FKCS admission-path drift review — F4 skill submission size/pending caps

Scope: independent review of the exact working-tree change (on top of 64bf998d) that moves the
`xtask/fkcs-baseline.toml` `[admission_baseline]` fingerprint. Read-only review; no re-pin done here.

## Verdict: RE-PIN JUSTIFIED (with conditions C1-C3)

## Evidence

Declared file set (`xtask/fkcs-baseline.toml` `[admission_baseline].files`; hash = SHA-256 over
`path \0 len_u64_le content` per file, `xtask/src/check_fkcs.rs:841-856`):
`crates/maos-registry/src/admission.rs`, `crates/maos-skill/src/admission.rs`.

Independent recomputation (python, same framing):
- HEAD 64bf998d content: `136c579d6a2ae4804d0f57864cec8c583363b563e2f063a870f6ae6f38256a46` (== current pin, so HEAD is clean).
- Working tree: `3dc4d08e237fffb6338354d71fc504a7d4a20217230237471e13f7cbaec349f9` (== reported actual).

`git diff HEAD` over the declared set: `crates/maos-registry/src/admission.rs` UNCHANGED;
`crates/maos-skill/src/admission.rs` +31/-0 — the entire drift. (Companion non-fingerprinted
changes: `errors.rs` +6, `tests/live_submission_18_7.rs` +36, `xtask/error-catalog.toml` +26,
docs page, kloc pins.)

The 31 lines (`crates/maos-skill/src/admission.rs`):
- 12-13: `const MAX_SUBMISSION_BYTES: usize = 256 * 1024;` `const MAX_PENDING_PER_AUTHOR: usize = 64;`
- 133-158: new private `fn check_submission_limits(&self, payload:&str, ctx:&SubmissionContext)`:
  `payload.len() > MAX` -> `ESkillQueue::PayloadTooLarge`; count of `Pending` entries whose
  `author.class == ctx.author.class` `>= 64` -> `ESkillQueue::PendingLimit`. `&self`, read-only.
- 166: `self.check_submission_limits(&document, ctx)?;` first statement of `prepare_author`.
- 219: `self.check_submission_limits(&proposal.proposed_diff, ctx)?;` first statement of `prepare_proposal`.

### (1) Only adds rejection limits; existing checks/order/errors unchanged? YES
No existing line is modified or removed (pure additions). The two calls are inserted ahead of
`vet_principals` etc.; every pre-existing check (`vet_principals`, `parse_skill`,
`validate_authored_id`, namespace, capability escalation, version ceiling, `validate_evidence`,
`build_proposal`, ownership/stale/evidence-mismatch, `check_insert` duplicate) and its error is
byte-identical and runs in the same relative order. Only observable ordering effect: a submission
that is BOTH oversized (or from an author at the pending cap) AND would also fail a later check now
reports PayloadTooLarge/PendingLimit first instead of the later error — still a refusal, never an
acceptance. Typed errors: `ESkillQueue::PayloadTooLarge` / `PendingLimit` (`errors.rs:80-83`,
codes at `errors.rs:121-122`), converted via `.into()` to `ESkillStore::InvalidEntry` exactly like
the sibling refusals. `ESkillQueue` is `#[non_exhaustive]` (`errors.rs:75`) and no consumer outside
maos-skill matches it exhaustively (only `EntryNotFound` arms at `operator_door.rs:2824`,
`maos-cli/src/subcommands.rs:1324`), so the new variants cannot break a dispatch; the live port
derives its refusal receipt from the error's `code()` (`maos-bin/src/skill_authoring.rs:~236`
receipt) so both new codes flow through the existing path.

### (2) Can the limits refuse previously-valid legitimate operations? NO in-repo regression found
- Largest in-repo skill-like documents: `tests/fixtures/anthropic-skill/SKILL.md` = 1,621 B;
  every `prepare_author`/`prepare_proposal` caller in tests builds documents of < 1 KiB
  (`skill_door_18_7.rs:24`, `erasure_inventory_20_11.rs:250`, `maos-cli/tests/
  skill_queue_integration_test.rs:359,541`, `live_submission_18_7.rs`). 256 KiB is >150x the largest.
  `examples/`/`templates/` ship no skill docs that go through the author path (the prerequisite
  skill files are deleted in this tree; package-shipped skills enter via discovery, not
  `prepare_author`, and have `author: None` so they are not counted by the pending cap).
- Pending cap: existing multi-entry tests create <= 3 pending entries per class; 64 is far above.
  Admitted/Rejected entries are not counted, so only undecided backlog is bounded; an operator
  decision frees capacity.
- Boundary: exactly 262,144 bytes accepted (`>`), 262,145 refused (test `live_submission_18_7.rs:
  guest_submissions_are_bounded_per_payload_and_author`); 64 pending accepted, 65th refused.
- Semantics note: cap is per author CLASS (`author.class`), across pids/boots, consistent with the
  catalog text ("author class already has 64 pending") — intended, not a defect.

### (3) unwrap/expect/panic, dependencies, public signatures? NONE
No `unwrap`/`expect`/`panic!`/indexing in the added non-test code. No `Cargo.toml`/`Cargo.lock`
change (`git diff HEAD --stat` clean for them). `prepare_author`/`prepare_proposal`/
`SubmissionContext`/`SkillRevisionProposal` signatures unchanged; the new fn and consts are
private. The only public-surface delta is two new variants on the `#[non_exhaustive]`
`ESkillQueue` (outside the fingerprinted files; additive).

### (4) Ordered before any filesystem/queue write? YES
Both checks run first inside `prepare_*`, which are `&self` and documented side-effect-free
(`admission.rs:1-2`); journal row insert, `commit_prepared`, and `store.save` happen only after
`prepare_*` returns Ok in the sole production caller (`maos-bin/src/skill_authoring.rs:~158-190`).
A refusal therefore leaves queue, journal approval rows and store untouched (the new test asserts
`entries().len()` unchanged via `refused()`).

## Conditions
- C1 (must, same commit): re-pin `[admission_baseline].sha256` to
  `3dc4d08e237fffb6338354d71fc504a7d4a20217230237471e13f7cbaec349f9` together with the code,
  `errors.rs`, tests, catalog, and a provenance comment in `xtask/fkcs-baseline.toml` citing this
  review and the exact hunk (+31 lines, `maos-skill/src/admission.rs` only; registry file unchanged;
  no kernel/frozen-tag/file-set change). The re-pin must not precede this review being filed.
- C2 (should, not FKCS): `xtask/error-catalog.toml` now has `ESkillQueue::PendingLimit`, but only
  `docs-site/docs/errors/ESkillQueue-PayloadTooLarge.md` exists (untracked); there is no
  `ESkillQueue-PendingLimit.md`. If the error-catalog/docs gate requires one page per catalog
  entry it will fail — add the page. Does not affect the fingerprint.
- C3 (info): the new test and constants are the only pin of the 256 KiB / 64 values; changing either
  later is another fingerprint change requiring review.
