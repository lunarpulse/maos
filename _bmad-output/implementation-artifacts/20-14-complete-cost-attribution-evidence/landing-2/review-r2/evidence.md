# 20-14 L2 correction round 2 — evidence (base HEAD `998b36e6`; Dev2014R2; 2026-11-12)

Input, persisted verbatim (plus one trailing newline): `../review/gpt-review-r2.md` (GPT `Review2014R`: 1 P1, 1 P2, verdict incorrect). Kernel-free: no kernel line, no surface hash.
Live base after 20-13 L2: kernel 27044 / **21202**, shared STOP **22000** (ruling `lim=stop22`, 2026-11-12) = 798 left (797 at the 21203 the review quoted); the earlier "STOP 21250 / 47 left" annotations in the story, flag decision, tracker comment and round-1/landing evidence are corrected.

## Findings and fixes

| Finding | Fix | Red before (raw log) | Final witness |
|---|---|---|---|
| P1: completed-forget replay searched raw payload bytes; a newly appended codec-encoded cost frame (`sk-alice` -> `x:` hex) was invisible, the repeat returned the historical success | `operator_door.rs completed_forget` tail check calls the single erasure predicate `TransparencyLogAdapter::subject_in_frame(kind, body, principal)` (made `pub`; kind-aware, decoded value) instead of `principal_id_occurs`; retained-receipt exclusions, Distillate rule and 4,096-row bound untouched | `red/bin-HEAD-red.log`: `assertion left != right failed` at `erasure_million_20_11.rs:436`, both operation ids `op-ab786a8679423ee808b11999` (the historical report was replayed) | bin `ac5_replay_invalidates_for_new_matching_frames_and_changed_reason` (extended): forget `sk-alice`, append a `serde_json::to_vec(CostFrameBody)` schema-2 frame (asserted `x:` on the wire), repeat -> new operation, `replayed:false`, principal `[REDACTED]`, dimensions 50/30 equal, a further repeat replays |
| P2: erasure walker decoded every string; literal `x:61` in a cost model / governance string stopped matching `61` and was scrubbed by `a` | `walk(kind, v, w, key)` carries the object key; `coded = kind == 29 && key in {principal_id, task_ref}` selects `decode_identity`, every other string is matched as written | `red/iac-HEAD-red.log`: `assertion left == right failed: 61: cost model` at `cost_identity_cover_20_14.rs:77` | iac `erasure_decodes_only_the_codec_bearing_cost_fields`: subject `61` scrubs the literal `x:61` model and governance string and leaves the encoded principal/task; subject `a` scrubs only the encoded principal; subject `b` only the encoded `task_ref` |

## Plants (copy-back + `cmp` identical + `touch`; raw logs and `plant-summary.jsonl` in `plants/`)

| Plant | What it breaks | Red witness (raw) |
|---|---|---|
| p1-replay-predicate-raw-bytes | replay tail check back to `principal_id_occurs` on raw bytes | bin replay test: `left != right` at `erasure_million_20_11.rs:436` |
| p2-walker-decodes-every-string | `coded = true` (the pre-fix behaviour) | iac witness: `61: cost model` |
| p3-walker-never-decodes | `coded = false` | iac witness: `61: principal` (encoded `x:61` principal matched by the raw subject) and the `a`/`b` rows |

All three exit 100, restored identical (`restored identical` at the end of each log).

## Pins (exact formatted `cargo fmt --all` / `kloc-check --json`, priced first on a `git archive HEAD` copy: 8773 / 38564 / 202920)

| Crate | Live pin at `998b36e6` | After | Note |
|---|---:|---:|---|
| maos-iac | 8771 | **8773** (+2) | walker key parameter + two locals; over the zero-headroom pin: ceiling draw for Main adjudication |
| maos-bin | 38563 | **38564** (+1) | replay predicate call; same |
| aggregate / hardfail | 202917 / 202918 | **202920 / 202921** | measured + 1 |
| maos-kernel-core tokei / physical | 21202 / 27044 | unchanged | STOP 22000: 798 left |
| capability / domain / cli | 1454 / 10274 / 6612 | unchanged | Main-adjudicated 2026-11-12, supported as the cheapest correct design: see the review's overdraft assessment |

`xtask/kloc.toml` rows + hardfail re-pinned (kernel unchanged, so `kernel-core-baseline.toml`, `kernel_pin_content_hash_16_0.rs` and `recovery_lane_ceiling_rule.rs` untouched); the kernel row's older HISTORY text still reads "STOP 21250 (47 left)" because editing that row moves the kernel pin content hash: left as history.

## Verification observed (env `-u CI -u GITHUB_ACTIONS`; scratch venv with `cryptography` on PATH)

- Focused green (`green/`): iac `cost_identity_cover_20_14` 2 passed; bin `cost_attribution_20_14` 11 passed; bin `erasure_million_20_11` (million cases excluded) 5 passed; full `maos-iac` nextest **197 passed**; bin focused set (`cost_attribution_20_14`, `erasure_inventory/live/scan_20_11`, `erasure_uninstall_13_5b`, `forget_floors_18_6`, `forget_governance_20_14`, `maos_uninstall_16_4`, `erasure_million_20_11` minus million) **99 passed, 1 skipped**; maos-audit `erasure_smoke_test` rc 0; xtask `d11_xtask_ceiling_ratchet` + `recovery_lane_ceiling_rule` + `kernel_pin_content_hash_16_0` 46 passed.
- `cargo fmt --all -- --check` rc 0; workspace clippy `--all-targets --locked` rc 0, `clippy-ratchet` rc 0 (1331 occurrences / 869 source-distinct, unchanged; no clippy hit on the changed lines).
- Gates rc 0 (`gates/*.json`): kloc-check (aggregate 202920 <= hardfail 202921), check-kernel-baseline, check-service-boundary (no surface refresh: the `pub` method is not a surface item), check-dev-model-tier, check-dev-record-completeness (the new deferred rows classify as owned / ownerless-dispositioned), check-epic-close-coherence, check-exit-commands, check-decision-register, check-env-contract, check-serde-error-handling, check-fkcs, check-pub-field-constructors, check-dependency-closure, check-abi-ratification, check-manifest-schema-version, coverage-matrix, invariant-lock `--changed-files crates/maos-iac/src/adapter/transparency_log.rs`.
- Not run / unverified: the two million-row pending cases (known `20-7` timing family), the full workspace nextest (accepted process: hosted CI authoritative), `abi-diff` (dirty tree), prepush, hosted legs, the WASM witness, `stability-matrix --check` / `gen-abi-docs --check` (no ABI or doc-table change). The iac `|s|` closure typing was compile-verified by the green builds; no production `unwrap`/`expect` was added.
