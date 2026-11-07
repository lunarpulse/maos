# 20-1b L1 — observed failure plants and restoration

Each row is an executed runtime red, not an uncompiled patch or a claimed baseline failure. The adjacent `<ac>-plant-diff.stdout` is the exact applied mutation; `<ac>-plant.stdout/.stderr` capture the failed assertions. Every plant was restored with the recorded copy-back + cmp + touch command (exit0), before the next plant. Wall times include compiling/executing the red and restoration. Later legitimate fixes are separately recorded; no mutation remains in the implementation.

| AC | Fault | Runtime exit | Wall seconds | Restore exit |
|---|---|---:|---:|---:|
| AC6 | Ignore the attested artifact digest; a valid attestation promotes swapped bytes. | 101 | 29.352 | 0 |
| AC7 | Skip signed deprecate authentication; unsigned mutation succeeds. | 101 | 21.887 | 0 |
| AC8 | Map quarantine to Warned; independently expected quarantine-requested record disappears. | 101 | 127.254 | 0 |
| AC9 | Skip running-promotion re-evaluation; the expiry/revocation observation disappears. | 101 | 72.735 | 0 |
| AC10 | Use bare spirit_id; a second publisher reads the first publisher's real private spill. | 101 | 34.684 | 0 |
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

## Separate genuine before observations

- `ac6-both-before`: both correctly re-signed artifact/publisher swaps were admitted PublicVetted before the binding change; both refusal assertions failed (exit101).
- `ac14-ac16-before`: changed installed bytes and a yanked install were accepted; both refusal assertions failed (exit101).
- `inactive-generation-before`: after the immutable-generation cutover, re-installing identical v1.0.0 while v1.0.1 was active incorrectly returned success without reactivating the original generation (exit101). The repair reuses only the unchanged original generation and refuses all changed package/origin/vetting metadata; registry-trust-final passes this extended vector.

AC15/F5 has no plant or refusal proof: design rows remain PROPOSED. AC8 sticky quarantine effects and AC11–AC13 belong to the final kernel landing and are not claimed by these plants. No local or hosted TLS interception, T3/delegated-cgroup quarantine-effect or independent security-review result is inferred.
