# 20-1b L1 — observed failure plants and restoration

Each row is an executed runtime red, not an uncompiled patch or a claimed baseline failure. The adjacent `<ac>-plant-diff.stdout` is the exact applied mutation; `<ac>-plant.stdout/.stderr` capture the failed assertions. Every plant was restored with the recorded copy-back + cmp + touch command (exit0), before the next plant. Wall times include compiling/executing the red and restoration. Later legitimate fixes are separately recorded; no mutation remains in the implementation.

This is the historical original-L1 ledger. Review corrections and their observed
commands live in `review-corrections/`; neither this table nor its before-runs
are evidence from a clean `bdfc1430` checkout.

| AC | Fault | Runtime exit | Wall seconds | Restore exit |
|---|---|---:|---:|---:|
| AC6 | Ignore the attested artifact digest; a valid attestation promotes swapped bytes. | 101 | 29.352 | 0 |
| AC7 | Skip signed deprecate authentication; unsigned mutation succeeds. | 101 | 21.887 | 0 |
| AC8 | Map quarantine to Warned; independently expected quarantine-requested record disappears. | 101 | 127.254 | 0 |
| AC9 | Skip running-promotion re-evaluation; the expiry/revocation observation disappears. | 101 | 72.735 | 0 |
| AC10 | Bare identity in the hand-bound identity-helper fixture discloses private spill; this plant did **not** cover production `load_admit_start` (review P4 supersedes that gap). | 101 | 34.684 | 0 |
| AC14 | Remove the existing-generation immutability guard; a same-key replacement installs. | 101 | 21.527 | 0 |
| AC16 | Remove the pre-persistence yank refusal; a yanked package installs. | 101 | 14.916 | 0 |
| AC17 | Remove publish signature verification; an unauthenticated package publishes. | 101 | 15.950 | 0 |

## Exact restoration commands

- `bash -c 'cp /tmp/201b-plants/ac6.rs crates/maos-compliance/src/vetting/mod.rs && cmp /tmp/201b-plants/ac6.rs crates/maos-compliance/src/vetting/mod.rs && touch crates/maos-compliance/src/vetting/mod.rs'` — exit0.
- `bash -c 'cp /tmp/201b-plants/ac7.rs crates/maos-registry/src/handlers/deprecate.rs && cmp /tmp/201b-plants/ac7.rs crates/maos-registry/src/handlers/deprecate.rs && touch crates/maos-registry/src/handlers/deprecate.rs'` — exit0.
- `bash -c 'cp /tmp/201b-plants/ac8.rs crates/maos-registry/src/yank.rs && cmp /tmp/201b-plants/ac8.rs crates/maos-registry/src/yank.rs && touch crates/maos-registry/src/yank.rs'` — exit0.
- `bash -c 'cp /tmp/201b-plants/ac9.rs crates/maos-bin/src/yank_policy.rs && cmp /tmp/201b-plants/ac9.rs crates/maos-bin/src/yank_policy.rs && touch crates/maos-bin/src/yank_policy.rs'` — exit0.
- `bash -c 'cp /tmp/201b-plants/ac10.rs crates/maos-bin/src/admission.rs && cmp /tmp/201b-plants/ac10.rs crates/maos-bin/src/admission.rs && touch crates/maos-bin/src/admission.rs'` — exit0.
- `bash -c 'cp /tmp/201b-plants/ac14.rs crates/maos-registry/src/storage.rs && cmp /tmp/201b-plants/ac14.rs crates/maos-registry/src/storage.rs && touch crates/maos-registry/src/storage.rs'` — exit0.
- `bash -c 'cp /tmp/201b-plants/ac16.rs crates/maos-registry/src/install.rs && cmp /tmp/201b-plants/ac16.rs crates/maos-registry/src/install.rs && touch crates/maos-registry/src/install.rs'` — exit0.
- `bash -c 'cp /tmp/201b-plants/ac17.rs crates/maos-registry/src/handlers/publish.rs && cmp /tmp/201b-plants/ac17.rs crates/maos-registry/src/handlers/publish.rs && touch crates/maos-registry/src/handlers/publish.rs'` — exit0.

## Partial-tree assertion-red observations (not commit-baseline runs)

- `ac6-both-before`: on a partially edited L1 tree, both correctly re-signed artifact/publisher swaps were admitted PublicVetted before the binding guard; both refusal assertions failed (exit101). This was not a `bdfc1430` baseline run.
- `ac14-ac16-before`: on a partially edited L1 tree, changed installed bytes and a yanked install were accepted; both refusal assertions failed (exit101). The different-registry loop branch was not reached and had no independent before-red.
- `inactive-generation-before`: after the immutable-generation cutover, identical v1.0.0 reinstall while v1.0.1 was active returned success without reactivation (exit101). That repair reused the unchanged generation. Promotion metadata was immutable at that stage; review ruling D4 subsequently authorizes verified atomic evidence renewal.
- No original before-red was recorded for AC7 cursor/server restart or AC10 first-publisher restart. Original spill tests bound identities by hand; original cadence tests used observer fixtures. They are not production-binding or real-runtime cadence proof.

AC15/F5 has no original plant or transport proof. Its design rows are now RULED A/A/A (`f5rows`, 2026-11-07), with implementation held for the separate cargo handoff. AC8 sticky quarantine effects and AC11–AC13 remain final-kernel work, not claimed by these plants. The independent review at `review/triage.md` occurred after the original handoff; hosted/TLS/T3 effects and review of the subsequent corrections are not inferred from it.
