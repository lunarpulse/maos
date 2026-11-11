# 20-1b FLAG-Winston decision record

**Recorded:** 2026-11-11 by Dev201bK at base `c2beedb4`, final kernel landing (landing 2), uncommitted.
**Authority:** operator ask `rows201` (2026-11-01/02: D-20-1-02 A KQ1, D-20-1-03 A P), Main's landing-split
ruling 2026-11-06 (the AC8 quarantine **effect** joins row R; no KQ0 substitute), shared whole-ledger STOP
21250 (ask `halt_grant`, 2026-11-04). Story: [20-1b](20-1b-vetting-yank-policy-and-quarantine.md);
precedent [20-3c](20-3c-flag-winston-decision.md), [20-11](20-11-flag-winston-decision.md).
**Evidence:** `20-1b-vetting-yank-policy-and-quarantine-evidence/landing-2/` (T0, red-head, green, plants,
diffs, gates, `commands.md`); creation prototypes `20-1-registry-client-install-verb-vetter-and-yank-evidence/creation/`
(never compiled; re-cut here).

## Grant and measurement

| Figure | Creation (base 26801 / 21011) | Granted (x2, D02 A) at creation | **T0 at live base 26889 / 21073** | Landed |
|---|---:|---:|---:|---:|
| R = KQ1 + P physical | +78 | 26957 | **+78** (`git archive HEAD` copy, formatted) | **26967** |
| R = KQ1 + P tokei | +68 | 21147 | **+68** | **21141** |
| Ceiling re-based onto the live base (base + 2 x R) | | | 26889 + 156 = **27045**; 21073 + 136 = **21209** | inside |
| Shared whole-ledger STOP | | 21250 | 21141 <= 21250 | inside |
| ADR-069 alarm / envelope (tokei) | | 22000 / 23000 | 21141 | inside both |

T0 method: `git archive HEAD` copy in `/tmp`, the creation `krec-kq1-p-bundle.diff` re-cut by hand (its scheduler hunk
no longer applies after 20-11; the one-line `unload` clear was re-placed), `rustfmt`, then the prebuilt gate binary from
the copy root: `check-kernel-baseline` 26967 (set hash `7441a294...`, later `06386b44...` once the comment-only doc edit
and the final files landed), `kloc-check` kernel 21141. The measured delta is **identical** to the creation prototype
(+78 / +68), so nothing exceeds the grant. +40 / +40 of it is the rustfmt churn any `revocation/applier.rs` edit pays
(control `fmt0`: the one over-long `json!` line makes rustfmt skip the whole `apply_crl` body).

## What landed (shape unchanged from the ruling)

- **KQ1.** `CapabilityRegistryAdapter` gains a private `quarantined` pid set, `set_quarantined` / `is_quarantined`;
  `is_paused` honours it, so every existing fence site (`issue_with_mediation`, `verify_and_audit`, `recheck`, hence MCP,
  inference and memory mediation) refuses with `CapError::Revoked`. The CRL `Quarantine` arm sets the fence, journals
  one `spirit.quarantined` frame (replacing `spirit.quarantine_requested`) and spawns no drain/terminate. A director
  `resume` clears only the pause set; `scheduler.unload` clears the quarantine.
- **P.** The revocation poller prints the first failure (`... (repeats counted until recovery)`), counts repeats and
  prints `recovered after N failed poll(s)`; the telemetry error count is unchanged per failure.
- **AC8 effect (bin, kernel-free).** `YankRuntime` takes the `CapabilityRegistryAdapter`; for the `quarantine` policy it
  calls `set_quarantined(pid, true)` on each signed-yank-matched Running Spirit and journals the same
  `spirit.quarantined` receipt. The L1 signed-yank/authority checks gate the request: forged, wrong-publisher,
  uninstalled-name and wrong-registry rows are `Rejected`; a version mismatch is verified but not actionable.

## Public surface — exactly one symbol, the named one

`check-service-boundary` hashes structs in full. The private `quarantined` field moves only
`maos_kernel_core::capability::CapabilityRegistryAdapter`: `bb8e817f...32f4b` -> `6e32a642...c3197` in
`docs/ci-baselines/kernel-surface-v0.1-beta.json` (ADR-066 hand-edit, one in-place line). D-20-1-02 A names this
struct as the home of the fence. The gate flagged **no other symbol** (verified by diffing every baseline entry against
the current surface: one changed, none added, none removed); the new `pub fn`s are not surface items; no
`xtask/kernel-api-classes.toml` row moves. `docs/invariants/*.md` is untouched (ADR-037).

## Pins moved together

| Pin | Before | After |
|---|---|---|
| `xtask/kernel-core-baseline.toml` `src_lines` | 26889 | **26967** |
| set hash | `1eb6670a...400bd` | `06386b442acc7a7241442eea761ba406a3e8ee8527a73030044d8a89c3224e5f` |
| file digests | | `capability/mod.rs`, `revocation/applier.rs`, `revocation/poller.rs`, `scheduler/scheduler_loop.rs` |
| `xtask/kloc.toml` `maos-kernel-core` | 21073 | **21141** |
| `xtask/kloc.toml` `maos-bin` | 38362 | **38392** (+30, inside the 20-1b bin ceiling 35671: 35586 + 85 room, 55 remaining) |
| `xtask/kloc.toml` `maos-audit` | 8286 | **8297** (+11: the FR4 `WRITER_SHAPES` entry the new bin receipt writer requires and the renamed kernel marker; inside the 20-11 ceiling 8313 and the 20-1b audit correction ceiling 8204 = exact 8179 + 25) |
| `xtask/kloc.toml` `_aggregate_hardfail` | 202506 | **202615** (clean aggregate 202614 = +68 kernel +30 bin +11 audit, + 1) |
| `xtask/tests/kernel_pin_content_hash_16_0.rs` | 26889 x4 | 26967 x4 |
| `xtask/tests/recovery_lane_ceiling_rule.rs` | 21_073 | 21_141 |

Clean figures are from a `git archive HEAD` copy overlaid with the landing's files (local untracked operator files
excluded).

## Not delivered by this row (disclosed)

- **KQ2** (outbound IAC fence) was priced and not selected (D-20-1-02 A). A quarantined Spirit's outbound IAC is fenced
  only through token issuance/verification, not through a dedicated egress check.
- The fence is an **in-memory pid set**: it ends with the process, and a reloaded same-class Spirit after a restart or a
  crash is a new pid and is **not** fenced (the yank cursor persists, so the row is not replayed). Persisting
  quarantine by identity is not part of the ruled KQ1 shape; it is a separately priced row (deferred-work).
- AC11.5 hosted T3 leg (does a container keep running activity under the fence) is unverified.
