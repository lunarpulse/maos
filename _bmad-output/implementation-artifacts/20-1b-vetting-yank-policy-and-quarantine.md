---
story_id: '20.1b'
story_key: 20-1b-vetting-yank-policy-and-quarantine
created: 2026-11-01
split_from: 20-1-registry-client-install-verb-vetter-and-yank (D-20-1-01 B, RULED 2026-11-01/02, operator ask split201)
context_commit: a4c00897
baseline_commit: a4c00897
security_sensitive: true
kernel_grant: "◆ FLAG-Winston RULED 2026-11-01/02 (ask rows201, operator): row R = KQ1 sticky quarantine (D-20-1-02 A) + P poller edge-trigger (D-20-1-03 A), measured by reverted prototype +78 physical / +68 tokei from HEAD 26801 / 21011 (+40 / +40 of it is the rustfmt churn any `revocation/applier.rs` edit pays, control `fmt0`). Granted ceilings: kernel <= 26957 physical / 21147 tokei. This grant is computed from HEAD 26801 / 21011 and lands in THIS story only (20-1a is kernel-free). INTERACTION with 20-7 row R = K1w + K2r (+40 / +31 measured; ceiling 26881 / 21073, also computed from HEAD 26801 / 21011): if both land, the second to land MUST re-base; cumulative measured tokei +68 +31 => expected 21110; cumulative tokei ceilings MUST NOT exceed 21209 (= 791 under the ADR-069 22000 alarm); STOP otherwise. Room per ADR-069: docs/adr/ADR-069-kernel-core-envelope-through-v1.md:35-46."
kloc_grant: "RULED 2026-11-01/02 (ask rows201; D02 A; D-20-1-11 A). One-key ceilings (STOP above; aggregate hardfail = measured + 1 at landing): kernel <= 26957 / 21147 (row R, this story); maos-registry <= 5163; maos-spirit-cli <= 2043; maos-cli <= 6666 (above its 6382 STOP; inside the reserved 6856 pin because install logic lives in maos-registry, D-20-1-05 A); maos-bin <= 34410; maos-shell <= 728; maos-compliance <= 2122; maos-domain <= 10153; xtask <= 42920; aggregate <= 193768. Each ceiling is shared with 20-1a; the kernel row is 20-1b's alone."
review: "bmad-code-review under docs/testing/proof-contract.md. DIFFICULT tier (`proof-contract.md:57-71`): a kernel FLAG-Winston grant (D-20-1-02/03), signatures and supply-chain trust (AC6, AC7, AC10), cross-Spirit isolation (AC11). Blind and Edge on opus-5.5, Acceptance on opus-5.5, a Security reading on AC6/AC7/AC10/AC11, Test Infrastructure on glm-5.3 (standing policy)."
---

# Story 20.1b: Vetting, yank policy and FR13 quarantine

Status: backlog

<!-- Split from story 20-1 on 2026-11-01/02 under operator ruling D-20-1-01 B (ask split201). Part B (AC6–AC13) and Tasks T6–T10 moved here unchanged; the story file for the exit path is `20-1a-registry-publish-install-exit-path.md`. Shared evidence: `20-1-registry-client-install-verb-vetter-and-yank-evidence/`. Decisions, rulings, pricing and Dev Notes live in 20-1a and are not repeated here. -->

> **Dependency:** 20-1b starts only when `20-1a-registry-publish-install-exit-path` is `done` (tracker row `sprint-status.yaml`). It is kernel-last: the FLAG-Winston kernel row lands here, after the kernel-free trust and yank work (AC6–AC10).

**Closes:**
- Epic-20 §20-1 Part B (E20 §20-1b, split at the Part boundary) as amended 2026-10-30 (D01 A; preflight §3.1) and D04 A (E20:102).
- `deferred-work.md:37` (same-name squatting, 20-3b row 06 A), `:56` (FR59 coverage row), `:1093` (attestation binding), `:1147` (FR13 quarantine), `:1264` (revocation-poller print, D-18-0-12 A). Owner re-pointed from `20-1` to `20-1b-vetting-yank-policy-and-quarantine` by D-20-1-01 B (B3).
- Tracker D9 routing: a revoked or lapsed attestation on a promoted `public-vetted` Spirit reaches operators as a vetter yank carrying the operator policy; `enforce_vetted_upgrade_precondition` (`crates/maos-bin/src/main.rs:130-202`) stays the only refusal point.
- Invariant I5's FR13 claim (`docs/invariants/I5.md:11`); `docs/invariants/I5.md:4`, `docs/invariants/README.md:33`, `tests/coverage-matrix.yaml:371` (B4).

**Does NOT close, and says so:**
- The accredited external vetter ecosystem (FR37 v2.5) and an independent author (J6, `routing-audit:143`) — operator lanes; this story proves the mechanism with internal keys (D12).

## Decisions and rulings

Rulings are recorded in the Decisions table of `20-1a-registry-publish-install-exit-path.md` (section "Rulings 2026-11-01/02"). Rows that bind this story: D-20-1-02 A (KQ1), D-20-1-03 A (P), D-20-1-06 A, D-20-1-07 B, D-20-1-08 A, D-20-1-09 A, D-20-1-10 A, D03 A, D04 A, D09 A, D12 A.

## Acceptance Criteria

### Part B — trust, yank and quarantine (L2 kernel-free + L3 ◆, or `20-1b` under D-20-1-01 B)

#### AC6 — the vetting attestation binds the exact artifact and publisher (D:1093); vetters can issue and revoke

1. `VettingClaim` (`attestation.rs:105-129`) binds `artifact_sha256` and the publisher public key; `verify_attestation` (`mod.rs:197`) and its caller (`admission.rs:388`) check both; golden byte pin (`attestation.rs:220-224,274`) updated; ADR-056 §1 amended (`docs/adr/ADR-056-fr37-vetting-machinery.md:24,95-101`).
2. `maos-spirit vet issue|revoke` wraps `issue_attestation` (`attestation.rs:150`) and the keyring's revocation (`keyring.rs:117,157,288,334`); the signed revocation-list **issuer** lives in maos-spirit-cli, not maos-compliance (E20:105).
3. **Failing-before witnesses (local):** swapped artifact under a valid attestation ⇒ refused; swapped publisher key ⇒ refused — both admitted `PublicVetted` at HEAD (D:1093). `vet issue` → `maosctl spirit install … --vetting-attestation` promotes; `vet revoke` → the next admission refuses.
4. **Plant (one, local):** `verify_attestation` ignores `artifact_sha256` ⇒ the swap vector red.

#### AC7 — yanks arrive signed, labelled and restart-safe (FR59; D-20-1-06 A, D-20-1-07 B)

1. `maos-spirit yank <name>@<ver>` (publisher key) and the vetter's yank (enrolled vetter key) send signed requests; the server verifies publisher yanks against the stored package's key and vetter yanks against the keyring; unsigned `registry.deprecate` is refused. `YankEntry` carries `origin` and a server sequence; the daemon re-verifies before acting and journals origin-labelled rows distinguishable from operator CRLs (intent + `RevocationOrigin` label).
2. The yank cursor survives a server restart and a daemon restart (persisted; F2, F3).
3. **Failing-before witnesses (local):** unsigned deprecate of another publisher's Spirit ⇒ refused (accepted at HEAD, measured); server restart between two yanks ⇒ the second is fetched (missed at HEAD); daemon restart ⇒ no replay of already-journaled yanks. Propagation ≤ 5 min is witnessed by driving `poll_once` with an injected source and by one bounded cross-process run at the 30 s clamp floor (`yank.rs:200-210`), budgeted under 20-12.
4. **Plant (one, local):** accept unsigned deprecate ⇒ red.

#### AC8 — yank policy `warn | quarantine | auto-revoke` (D-20-1-08 A)

1. `maos_registry::yank::YankPolicy` read by maos-bin from `MAOS_YANK_POLICY` (registered in `env_contract.rs:13`, `UserFacing`; default `warn`), not a `RegistrySection` field (`operator_config.rs:13`). `warn` = today's rows; `quarantine` = the AC11 fence on running pids of the yanked version; `auto-revoke` = a local CRL (origin `RegistryYank`, operator-key signed, `SignedRevocationList::new`) through `apply_crl` — each Spirit executes its declared action.
2. **Failing-before witness (local):** `crates/maos-bin/tests/yank_policy_20_1.rs` — one yank per policy against a running fixture Spirit: `warn` ⇒ rows only, still mediated; `quarantine` ⇒ next mediated call refused, still loaded; `auto-revoke` ⇒ the declared action's receipts. Red at HEAD (no policy).
3. **Plant (one, local):** policy ignored ⇒ the `quarantine` vector red.

#### AC9 — a lapsed or revoked promotion reaches operators as a vetter yank (D9; D-20-1-10 A)

1. On the yank cadence the daemon re-evaluates each running `public-vetted` Spirit's attestation; expiry or revocation journals the `RunningSpiritObservation` (`terminal.rs:99-152`) with its cause label and applies the yank policy as a vetter-origin yank. `enforce_vetted_upgrade_precondition` stays the only refusal point (S:296); no crossing check.
2. **Failing-before witness (local):** a promoted fixture whose attestation expires (injected clock) and one whose vetter key is revoked ⇒ one observation each with labels `expiry-lapse` / `vetting-revocation` and the policy effect — nothing at HEAD (F6).
3. **Plant (one, local):** skip the re-evaluation ⇒ red.

#### AC10 — a name cannot be squatted (D:37; D-20-1-09 A)

1. The registry and install pin the first publisher key per `spirit_id`; a different key is refused, typed. Admission binds storage identity `spirit_id` + a truncated publisher-key fingerprint for registry-installed Spirits (`admission.rs:893`); local Spirits keep the bare `spirit_id`.
2. **Failing-before witnesses (local):** publish `X` with key K1, then `X` with K2 ⇒ refused (overwrites at HEAD, F4); a second boot admitting `X` under K2 (forced through import) does not read K1's principal spill (extends `crates/maos-kernel-core/tests/principal_identity_20_3b.rs` at the maos-bin admission level).
3. **Plant (one, local):** bind identity to `spirit_id` only ⇒ the spill vector red.

#### AC11 — FR13 quarantine isolates and never kills (D04 A; D-20-1-02 A) ◆ FLAG-Winston

1. Kernel: `kq1-sticky-quarantine.diff` shape — the CRL `Quarantine` arm (`applier.rs:160-197`) sets the quarantine fence, journals `spirit.quarantined`, and spawns no drain/terminate; `is_paused` honours quarantine at every fence site (`capability/mod.rs:203,319,345`); a director `resume` cannot lift it; `unload` ends it (`scheduler_loop.rs:572`).
2. **Hostile-effect witness (local, kernel):** `crates/maos-kernel-core/tests/quarantine_fr13_20_1.rs` — a fixture Spirit declaring `[on_revocation] action = "quarantine"` receives a CRL; it then attempts an MCP tool call, an inference call, a memory write and a token verification ⇒ each refused (`CapError::Revoked`), nothing reaches the effect, the Spirit is still loaded and Running past the old drain deadline; pause→resume leaves it fenced; unload clears it. Red at HEAD (terminated after the deadline; mediated calls succeed until then).
3. `docs/invariants/I5.md:11` FR13 → witnessed by this test; `tests/coverage-matrix.yaml:366-371` FR13 note updated; architecture `4-kernel-design.md:661` sentence corrected.
4. **Plant (one, local):** restore drain-then-terminate in the arm ⇒ AC11.2 red.
5. **Hosted T3 leg:** a T3 Worker Spirit quarantined by CRL — its next mediated action refused; whether its container keeps running activity is recorded (measurement owed 4); if it does, the record names it.
6. Pins: both kernel pins, content hashes; HISTORY rows cite this key; FLAG-Winston decision record (precedent `20-3c-flag-winston-decision.md`).

#### AC12 — the revocation poller says a failure once (D:1264; D-20-1-03 A) ◆ FLAG-Winston

1. `poller.rs:44-56`: the first failure prints, repeats are counted, recovery prints the count; telemetry unchanged (`p-poller-edge-trigger.diff` shape).
2. **Failing-before witness (local):** a fresh home (no `latest.signed.json`) with the fast cadence (`MAOS_REVOCATION_FAST`) for ~1 s ⇒ exactly one failure line — one per tick at HEAD.
3. **Plant (one, local):** print on every failure ⇒ red.

#### AC13 — the ledger stops naming this key the day it is true

1. `check-dev-record-completeness` passes at the final landing with no open `deferred-work.md` row owned by `20-1` (D:37, :56, :1093, :1147, :1264 → `CLOSED <date> by 20-1 @<commit>` with witness, or rule-10 re-owned); FR35/FR36/FR59 coverage rows (`coverage-matrix.yaml:617,622,841`) name their witnesses; FR37 stays `v2.0+` with the machinery witnessed; I5 owner re-pointed or closed.
2. **Negative control:** with one live row left naming `20-1` and S:296 flipped to `done` in a scratch copy, the gate reds.


## Tasks / Subtasks

- [ ] **T6 — Binding + vet CLI (AC6)**; ADR-056 §1; golden pin.
- [ ] **T7 — Signed, labelled, restart-safe yanks (AC7)**; policy (AC8); vetter lapse/revocation (AC9); squatting (AC10). L2 landing.
- [ ] **T8 — L3 kernel (AC11, AC12)** under the ruled FLAG-Winston row: apply the ruled bundle (re-cut, never applied blindly), the hostile-effect witness, I5/coverage/architecture text, both kernel pins + hashes, decision record.
- [ ] **T9 — Plants** (one per AC, `proof-contract.md:24-31`), local except AC5 (hosted) and AC11.5 (hosted T3); wall-clock per plant in the Dev Agent Record.
- [ ] **T10 — Close (AC13).** `check-kernel-baseline`, `check-service-boundary`, `check-pub-field-constructors`, `check-dependency-closure`, `check-env-contract`, `check-dev-record-completeness`, `check-epic-close-coherence`, `prepush`; one hosted discipline run on the final tree (known 20-7 flakes by name and run id, D13). Revert `intent-lineage-coverage-report.md` if a run modifies it. No commit or push without the operator.

## Dev Notes

Shared Dev Notes (what changes by file, guardrails, proven-red controls, tests and commands, review plan) are in `20-1a-registry-publish-install-exit-path.md` `## Dev Notes`. Part-B-specific file rows: `maos-spirit-cli` vet/yank wiring, `maos-compliance` attestation binding, `kernel-core` quarantine fence and poller (AC11, AC12), `docs/invariants/I5.md`, `tests/coverage-matrix.yaml`.

## Dev Agent Record

Not started. Filled at 20-1b dev start (model, wall-clock per plant, file list, change log).
