---
story_id: '20.1b'
story_key: 20-1b-vetting-yank-policy-and-quarantine
created: 2026-11-01
split_from: 20-1-registry-client-install-verb-vetter-and-yank (D-20-1-01 B, RULED 2026-11-01/02, operator ask split201)
context_commit: a4c00897
baseline_commit: a4c00897
security_sensitive: true
kernel_grant: "L1 KERNEL-FREE, RULED Main 2026-11-06: AC8 quarantine effect joins final row R (AC11-AC13 KQ1+P); no KQ0 substitute. Rebase prior measured prototype +78 physical/+68 tokei onto ebf2ab03 and current unchanged26841/21042: forecast26919/21110; D02 A x2 ceilings26997/21178, below shared STOP21250 tokei. No prototype rerun or kernel change claimed in L1. Historical superseded grant: ◆ FLAG-Winston RULED 2026-11-01/02 (ask rows201, operator): row R = KQ1 sticky quarantine (D-20-1-02 A) + P poller edge-trigger (D-20-1-03 A), measured by reverted prototype +78 physical / +68 tokei from HEAD 26801 / 21011 (+40 / +40 of it is the rustfmt churn any `revocation/applier.rs` edit pays, control `fmt0`). Granted ceilings: kernel <= 26957 physical / 21147 tokei. This grant is computed from HEAD 26801 / 21011 and lands in THIS story only (20-1a is kernel-free). INTERACTION with 20-7 row R = K1w + K2r (+40 / +31 measured; ceiling 26881 / 21073, also computed from HEAD 26801 / 21011): if both land, the second to land MUST re-base; cumulative measured tokei +68 +31 => expected 21110; cumulative tokei ceilings MUST NOT exceed 21209 (= 791 under the ADR-069 22000 alarm); STOP otherwise. Room per ADR-069: docs/adr/ADR-069-kernel-core-envelope-through-v1.md:35-46."
kloc_grant: "L1 REBASE, operator f1b/Main 2026-11-06, D02 A x3 unmeasured kernel-free forecast: current external baseline bin35102, registry4412, spirit-cli755, compliance2032 measured (old reserved pin2117), domain10107, aggregate194954 including external20-2 xtask20. Forecast upper increments245/265/380/30/20; ceilings bin35837, registry5207, spirit-cli1895, compliance2122, domain10167, aggregate197774. Exact formatted L1 pins bin35422/registry4627/spirit-cli936/compliance2070/domain10137/xtask44696; aggregate195738, hardfail195739. xtask44676 -> 44696 is external20-2 hosted-3 d84b83dd, authorized ceiling44920, not own Rust growth. CLI6429 unchanged, existing6666 ceiling/reserved6856 pin unchanged. No kernel/audit/iac growth by L1; F5 prices remain PROPOSED D-20-1b-01/02/03 and are not granted. These are shared ceilings, not reservations; remeasure future competing draws. Historical superseded grant: RULED 2026-11-01/02 (ask rows201; D02 A; D-20-1-11 A). One-key ceilings (STOP above; aggregate hardfail = measured + 1 at landing): kernel <= 26957 / 21147 (row R, this story); maos-registry <= 5163; maos-spirit-cli <= 2043; maos-cli <= 6666 (above its 6382 STOP; inside the reserved 6856 pin because install logic lives in maos-registry, D-20-1-05 A); maos-bin <= 34410; maos-shell <= 728; maos-compliance <= 2122; maos-domain <= 10153; xtask <= 42920; aggregate <= 193768. Each ceiling is shared with 20-1a; the kernel row is 20-1b's alone."
review: "bmad-code-review under docs/testing/proof-contract.md. DIFFICULT tier (`proof-contract.md:57-71`): a kernel FLAG-Winston grant (D-20-1-02/03), signatures and supply-chain trust (AC6, AC7, AC10), cross-Spirit isolation (AC11). Blind and Edge on opus-5.5, Acceptance on opus-5.5, a Security reading on AC6/AC7/AC10/AC11, Test Infrastructure on glm-5.3 (standing policy)."
---

# Story 20.1b: Vetting, yank policy and FR13 quarantine

**Shared kernel STOP supersession — 2026-11-04 (ask `halt_grant`).** Historical internal STOP
21209 / 21,209 and the 20-9-only 21230 / 21,230 are superseded by **21250 / 21,250 for the whole
cumulative measured kernel ledger at each landing**. D-20-14-12 C's joint 21250 now equals that shared STOP,
not a separate exception. With ruled 20-11 O4, cumulative measured tokei is **21203 + 12 = 21215**
(physical **27032 + 16 = 27048**). ADR-069 alarm **22000** and envelope **23000** are unchanged.
Older arithmetic/rulings below remain historical; later kernel landers must re-measure and re-base.


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

### Landing-1 rulings — 2026-11-06

- Operator ask `f1b`: inherited F4, F5, F6 and publish/deprecate authentication
  are **IN scope**, recorded as AC14–AC17 below. The earlier PROPOSED scope
  dispositions are superseded; F5's implementation design remains unruled.
- Main's explicit landing-split ruling: AC8's sticky quarantine **effect** and
  its effect witness move to the final KQ1 kernel landing with AC11–AC13.
  Landing 1 implements the kernel-free decision, typed boundary outcome and
  operator-visible `quarantine requested` record; **no quarantine effect is
  claimed in L1**. KQ0 / a resumable pause is not an approved substitute.
  The final kernel landing remains last; no kernel source change in L1.

### F5 design decisions — PROPOSED, not implementation authority

All prices below are **unmeasured forecasts**, additional to the kernel-free
AC6–AC10 and AC14/16/17 forecast. D02 A means ×3 upper-bound allowances;
they are not exact pins or reserved capacity. Baseline is `ebf2ab03`:
registry 4412, spirit-cli 755, cli 6429, bin 34914, compliance 2032,
domain 10107, xtask 44676, aggregate 194743. Kernel remains 26841 / 21042.

| ID | Choice and options | Recommendation, price and envelope consequence | Status |
|---|---|---|---|
| D-20-1b-01 | First-contact root: **A** explicit registry signing public key in operator config, bound to canonical registry URI, with registry-signed artifact/publisher metadata; **B** TOFU only after mandatory out-of-band fingerprint confirmation; **C** operator-distributed signed root bundle with expiry and rotation. A response-supplied key alone is never a root. | **A**: no interactive trust race, unattended installs are deterministic. A: registry +100…180, cli +0…10; ×3 allowance +540/+30. B: registry +150…260, cli +30…60; +780/+180. C: registry +220…380, domain +20…40; +1140/+120. All kernel 0. Existing tier-only signature is insufficient: it does not bind artifact or publisher. | PROPOSED 2026-11-06; Main/operator ruling required |
| D-20-1b-02 | Authenticated TLS/key custody: **A** rustls in the leaf HTTP transport and registry server; explicit certificate chain and private-key files supplied by path, platform/WebPKI trust with optional operator CA, hostname verification; **B** external TLS terminator with registry accessible only through a separately authenticated local upstream. | **A**: end-to-end refusal can be witnessed in owned binaries without assuming a correctly configured proxy. Registry +240…420, bin/cli +0…15 each; ×3 allowance +1260/+45/+45; dependency/lock/deny review required. B: registry +100…180 plus deployment/harness work; +540 allowance, but upstream authentication must be designed rather than treating loopback as authentication. Key files live outside the package/inventory store, operator-owned 0600; certificate chain may be 0644. No generated production key or trust material is silently installed. All kernel 0. | PROPOSED 2026-11-06; Main/operator ruling required |
| D-20-1b-03 | Loopback/development: **A** require explicit development-mode opt-in at both server and install; permit HTTP only for literal loopback addresses, keep mutation signatures and publisher pins; **B** TLS everywhere including loopback with operator-created test CA. Never downgrade HTTPS after failure. | **A**: retains executable local fixture workflows while preventing production URI downgrade. Registry +35…70, cli +5…15, spirit-cli +5…15; ×3 allowances +210/+45/+45. B: registry +10…25, CLI 0, more fixture certificate custody. A is an explicit exception, not authenticated transport evidence; F5 production hostile-registry/interception witnesses must use TLS and the configured root. All kernel 0. | PROPOSED 2026-11-06; Main/operator ruling required |

Recommended A/A/A combined F5 upper forecast: registry +670, cli +40,
bin +15, spirit-cli +15; ×3 allowances +2010/+120/+45/+45, aggregate
+2220. These are incremental to other L1 work and competing story draws.
The initial `ebf2ab03` baseline above is retained as creation provenance, not
the resumed-tree pin. After external `20-11`/`20-2` changes, the formatted L1
measurement is registry4627 / cli6429 / bin35422 / spirit-cli936,
aggregate195738 (kernel unchanged). Against rebased kernel-free ceilings
registry5207 / bin35837 / spirit-cli1895 / aggregate197774:

- Recommended A/A/A raw upper forecast yields registry5297 (**90 above**
  the existing registry envelope), cli6469 / bin35437 / spirit-cli951 and
  aggregate196478 (inside the current aggregate ceiling).
- Applying D02 A ×3 to this new, unmeasured F5 work, without renewing the
  prior L1 allowance, proposes registry6637 (**1430 above** its current
  ceiling), cli6549 / bin35467 / spirit-cli981 and aggregate197958
  (**184 above** the current aggregate ceiling). These increases require
  the D-row ruling; none is silently granted or pinned.
- Individually, root B's ×3 registry ceiling5407 exceeds5207 by200; root C
  proposes registry5767 and domain10257, exceeding5207/10167 by560/90.
  Root A alone fits its registry allowance. TLS A alone proposes registry5887
  (680 above5207); TLS B alone and either development option fit the
  current registry ceiling. Combining choices adds their prices, not their
  separately rebased baselines; competing story draws must be remeasured.

No F5 implementation, hostile-registry/TLS-interception refusal witness,
or security closure is claimed. First-contact/TLS/development choices remain
PROPOSED until Main/operator rules D-20-1b-01/02/03.


## Acceptance Criteria

### Part B — trust, yank and quarantine (L2 kernel-free + L3 ◆, or `20-1b` under D-20-1-01 B)

#### AC6 — the vetting attestation binds the exact artifact and publisher (D:1093); vetters can issue and revoke

1. `VettingClaim` (`crates/maos-compliance/src/vetting/attestation.rs:105-133`) binds `artifact_sha256` and the publisher public key; `verify_attestation` (`vetting/mod.rs:220`) and its caller (`crates/maos-registry/src/admission.rs:388`) check both; golden byte pin (`attestation.rs:226-233,280`) updated; ADR-056 §1 amended.
2. `maos-spirit vet issue|revoke` wraps `issue_attestation` (`attestation.rs:154`) and the keyring's revocation (`keyring.rs:117,157,288,334`); the signed revocation-list **issuer** lives in maos-spirit-cli, not maos-compliance (E20:105).
3. **Failing-before witnesses (local):** swapped artifact under a valid attestation ⇒ refused; swapped publisher key ⇒ refused — both admitted `PublicVetted` at HEAD (D:1093). `vet issue` → `maosctl spirit install … --vetting-attestation` promotes; `vet revoke` → the next admission refuses.
4. **Plant (one, local):** `verify_attestation` ignores `artifact_sha256` ⇒ the swap vector red.

#### AC7 — yanks arrive signed, labelled and restart-safe (FR59; D-20-1-06 A, D-20-1-07 B)

1. `maos-spirit yank <name>@<ver>` (publisher key) and the vetter's yank (enrolled vetter key) send signed requests; the server verifies publisher yanks against the stored package's key and vetter yanks against the keyring; unsigned `registry.deprecate` is refused. `YankEntry` carries `origin` and a server sequence; the daemon re-verifies before acting and journals origin-labelled rows distinguishable from operator CRLs (intent + `RevocationOrigin` label).
2. The yank cursor survives a server restart and a daemon restart (persisted; F2, F3).
3. **Failing-before witnesses (local):** unsigned deprecate of another publisher's Spirit ⇒ refused (accepted at the baseline); server restart between two yanks ⇒ the second is fetched; daemon restart ⇒ no replay of already-journaled yanks. Propagation ≤ 5 min is witnessed by `poll_once` and one bounded cross-process production-loop run at the 30 s clamp floor (`crates/maos-registry/src/yank.rs:143-179`), separately budgeted under 20-12.
4. **Plant (one, local):** accept unsigned deprecate ⇒ red.

#### AC8 — yank policy `warn | quarantine | auto-revoke` (D-20-1-08 A)

1. `maos_registry::yank::YankPolicy` read by maos-bin from `MAOS_YANK_POLICY` (registered `UserFacing`; default `warn`), not a `RegistrySection` field. `warn` journals only; `quarantine` delivers a typed request and operator-visible record in L1; `auto-revoke` submits an operator-key-signed local CRL with origin `RegistryYank` through existing `apply_crl`.
2. **L1 witnesses:** policy selection, typed request, signed propagation and request record, plus warn and auto-revoke behavior. **Final-kernel dependency, not claimed in L1:** quarantine's next mediated call refused while still loaded, using the AC11 KQ1 sticky fence. Main ruled this effect/witness into the final kernel landing on 2026-11-06; no KQ0 substitute.
3. **Plant:** ignore policy selection ⇒ the typed quarantine-request vector red in L1; the sticky-effect vector belongs to the final kernel landing.

#### AC9 — a lapsed or revoked promotion reaches operators as a vetter yank (D9; D-20-1-10 A)

1. On the yank cadence the daemon re-evaluates each running `public-vetted` Spirit's attestation; expiry or revocation journals `RunningSpiritObservation` (`crates/maos-compliance/src/vetting/terminal.rs:99-152`) with its cause label and applies the yank policy as a vetter-origin yank. `enforce_vetted_upgrade_precondition` (`crates/maos-bin/src/main.rs:130`) stays the only refusal point; no crossing check.
2. **Failing-before witness (local):** a promoted fixture whose attestation expires (injected clock) and one whose vetter key is revoked ⇒ one observation each with labels `expiry-lapse` / `vetting-revocation` and the policy effect — nothing at HEAD (F6).
3. **Plant (one, local):** skip the re-evaluation ⇒ red.

#### AC10 — a name cannot be squatted (D:37; D-20-1-09 A)

1. The registry and install pin the first publisher key per `spirit_id`; a different key is refused, typed. Registry admission derives `spirit_id` + a truncated publisher-key fingerprint (`crates/maos-bin/src/admission.rs:483`) and binds it at load (`:1034`); local Spirits keep the bare `spirit_id`.
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

#### AC14 — installed name@version is immutable (F4; RULED `f1b`, 2026-11-06)

1. Refuse any change to an already installed `name@version`, including a
   same-key replacement and an overwrite supplied by a different registry.
2. Witness both attempts; installed bytes, origin and active inventory remain
   unchanged. A refusal must occur before activating replacement persistence.
3. Plant: permit same-key replacement ⇒ the unchanged-inventory witness red.

#### AC15 — authenticated first contact and transport (F5; RULED `f1b`, 2026-11-06)

1. Registry installation requires an authenticated first-contact trust root
   and authenticated TLS transport. A registry response is not its own root.
2. Hostile-registry and network-interception witnesses refuse before persistence.
3. Implementation awaits D-20-1b-01/02/03; the explicit dev exception, if ruled,
   must not be reported as TLS proof. Plant: bypass root/peer verification ⇒ red.

#### AC16 — yanked versions are refused before persistence (F6; RULED `f1b`, 2026-11-06)

1. Installing a yanked `name@version` is refused before persistence or inventory
   activation, with a typed yank refusal.
2. Witness a valid signed but yanked package and absence of installed files.
3. Plant: omit yank refusal ⇒ the no-persistence witness red.

#### AC17 — registry mutations authenticate and authorize (RULED `f1b`, 2026-11-06)

1. Server-side authentication and authorization are required for
   `registry.publish` and `registry.deprecate`; CSRF/Host/Origin is not auth.
2. Unauthenticated and authenticated-but-unauthorized mutation attempts
   are refused without changing packages, yanks or ownership.
3. Plant: bypass mutation authentication ⇒ the refusal witness red.

## Inherited from 20-1a review

- **F4 (security review):** RULED in scope as AC14 by operator `f1b`, 2026-11-06.
  Same-key immutability supplements AC10's publisher pin; both same-registry
  replacement and different-registry overwrite have refusal witnesses.
- **F5 (security review):** RULED in scope as AC15 by operator `f1b`, 2026-11-06.
  Its trust-root/TLS/development choices remain PROPOSED D-20-1b-01/02/03,
  not implementation authority. No first-contact or TLS closure is claimed.
- **F6 (security review):** RULED in scope as AC16 by operator `f1b`, 2026-11-06.
  Signed yank propagation for running Spirits does not replace install refusal.
- **Mutation authentication (security/BMAD reviews):** RULED in scope as AC17
  by operator `f1b`, 2026-11-06. Publish and deprecate must authenticate and
  authorize independently of the existing browser/rebinding guards.


## Tasks / Subtasks

- [ ] **T6 — Binding + vet CLI (AC6)**; ADR-056 §1; golden pin.
- [ ] **T7 — Signed, labelled, restart-safe yanks (AC7)**; policy (AC8); vetter lapse/revocation (AC9); squatting (AC10). L2 landing.
- [ ] **T8 — L3 kernel (AC11, AC12)** under the ruled FLAG-Winston row: apply the ruled bundle (re-cut, never applied blindly), the hostile-effect witness, I5/coverage/architecture text, both kernel pins + hashes, decision record.
- [ ] **T9 — Plants** (one per AC, `proof-contract.md:24-31`), local except AC5 (hosted) and AC11.5 (hosted T3); wall-clock per plant in the Dev Agent Record.
- [ ] **T10 — Close (AC13).** `check-kernel-baseline`, `check-service-boundary`, `check-pub-field-constructors`, `check-dependency-closure`, `check-env-contract`, `check-dev-record-completeness`, `check-epic-close-coherence`, `prepush`; one hosted discipline run on the final tree (known 20-7 flakes by name and run id, D13). Revert `intent-lineage-coverage-report.md` if a run modifies it. No commit or push without the operator.

## Dev Notes

Shared Dev Notes (what changes by file, guardrails, proven-red controls, tests and commands, review plan) are in `20-1a-registry-publish-install-exit-path.md` `## Dev Notes`. Part-B-specific file rows: `maos-spirit-cli` vet/yank wiring, `maos-compliance` attestation binding, `kernel-core` quarantine fence and poller (AC11, AC12), `docs/invariants/I5.md`, `tests/coverage-matrix.yaml`.

## Dev Agent Record

### Agent Model Used

`openai-codex/gpt-6.1-sol` — kernel-free landing-1 implementation.

### Debug Log References

- Shared creation/ruling history: `20-1-registry-client-install-verb-vetter-and-yank-evidence/creation/`. Its a4c00897 citations/prices are historical, not current pins.
- Landing evidence root: `20-1b-vetting-yank-policy-and-quarantine-evidence/landing-1/`.
- Exact commands, return codes and elapsed seconds: `commands.json`, adjacent labelled `.stdout/.stderr`; the separate workspace sequence records `workspace-commands.json`.
- Observed red mutations, exact copy-back/cmp/touch restorations and wall times: `plants.md`, `plants.json`, `restoration-commands.json`, `ac*-plant-diff.stdout`, `ac*-plant.*`, `ac*-restore.*`.
- Re-measured baseline/current source positions and hashes: `cites-ebf2ab03.md/.json`; exact counts/pin drivers: `formatted-generation-measurement.stdout`, `final-kloc-check.stdout`, `exact-pin-updates.json`.
- Real, non-canned workspace clippy stream: `clippy-final.stdout/.stderr`; ratchet: `clippy-ratchet-final-corrected.stdout`; owned diagnostics: `clippy-owned-surface-final.json`.
- Branch-only hosted recipe: `20-1b-vetting-yank-policy-and-quarantine-evidence/ci/ci-20-1b-verify.yml`, `push: branches: ['ci/20-1b-verify']`; authored, not installed on main or executed.

### Completion Notes List

#### Authority, scope and custody

Operator `f1b` (2026-11-06) adds AC14–AC17. AC6–AC10's kernel-free work and AC14/16/17 are implemented with local refusal/effect witnesses. **AC15/F5 remains blocked on PROPOSED D-20-1b-01/02/03**, not silently designed. Main's explicit split puts AC8's sticky quarantine **effect and effect witness** with AC11–AC13 in the final KQ1 kernel landing: **not claimed in L1**. L1 delivers the signed decision, typed boundary outcome and operator-visible quarantine-requested record; no KQ0/resumable-pause substitute.

No kernel source, `.devcontainer`, protected worktree, workflow on main, commit, push or tracker status was changed. Original task/closure workflow state remains Main-owned. Independent DIFFICULT-tier Blind/Edge/Acceptance/Security/Test Infrastructure/Winston review and hosted execution are not claimed.

Work began at `ebf2ab03ac64de5b828d995f72ae8b24199fa9a0`. At Main's park request, owned changes were archived, removed and an empty status was observed; after the exclusive cargo slot resumed, that archive was reapplied onto `063c13cf`. Current measured source includes external 20-2 commits through `bdfc1430`; `git log --oneline -5` and exact revision capture are preserved. `resume-baseline-kloc` ran **after WIP reapplication**, so its failing measurement is not represented as a clean baseline.

#### Implementation and clean cutover

- Required vetting fields bind exact manifest, artifact SHA-256 and publisher key. An independent `VettingSubject` reaches verification; both hostile swaps are correctly re-signed so a publisher-signature failure cannot masquerade as attestation binding. Canonical CBOR golden bytes and ADR-056 are updated; old manifest-only attestations require reissuance.
- Real `maos-spirit vet issue|revoke` signs the claim and operator-root CRL/keyring update; real publisher and enrolled-vetter `yank` commands send domain-separated signed authorizations. The real revoke/install lifecycle exposed the existing CRL fixed-byte CBOR decoder mismatch; accepting the serializer's byte-string representation fixes that codec without changing signed bytes. Wrong 63/31-byte vectors still refuse.
- Publish authenticates exact package bytes and signed manifest name/version, then authorizes against first-publisher ownership across versions. Deprecate authenticates publisher ownership or root-signed vetter enrollment. Unsigned/wrong-owner/unenrolled mutations leave storage unchanged. Server keyrings are startup snapshots requiring restart after enrollment/revocation changes, explicitly documented; daemon promotion re-evaluation instead reads the retained live keyring path.
- Installed package, publisher, origin and promotion metadata are immutable for an existing name/version, including inactive generations. Identical reinstalls reuse the original immutable generation; the additional observed-before regression where another version remained active is repaired and witnessed. Cross-process mutation serialization uses the repository's rustix flock pattern.
- Yanked versions refuse before local storage construction/persistence. Durable server sequences replace clock-derived cursors; legacy `since_ns`/`last_seen_ns` wire names explicitly mean sequence. Atomic cursor persistence is used in production polling and joined shutdown. Historical unsigned yanks can refuse installation but cannot authorize daemon effects; obsolete clock-remapping code is removed.
- Remote yanks are re-verified against locally installed publisher/enrollment evidence before origin-labelled journaling. `warn` records only; quarantine emits `QuarantineRequested` and its operator record; auto-revoke creates an operator-key-signed exact-version `RegistryYank` CRL through the existing applier and honors the declared action. The poll guard joins normal shutdown and aborts early-exit tasks; a disabled guard allocates nothing.
- The same cadence re-evaluates running promotions using an injected clock and live keyring, deduplicating observations labelled `expiry-lapse` or `vetting-revocation` before the same policy seam. No new crossing-time refusal is introduced.
- Registry admission derives a publisher-fingerprinted principal identity, hashes long names to stay below the encoded filename limit, and binds it at the existing load boundary. Signed sidecars feed boot, standalone and operator-door paths; ordinary local Spirits retain the bare identity.
- `MAOS_YANK_POLICY` is registered UserFacing; the new maos-bin module is in the source scanner. New tests invoke existing harness-resolved binaries, never cargo.

#### AC witness matrix — observed local results

All paths below are repository-relative. Each plant is an executed runtime assertion failure (exit101), followed by copy-back + cmp + touch (exit0). Exact commands and elapsed times are in the linked evidence, not inferred from source.

| AC | Named witness and line | Before / plant | Final focused observation |
|---|---|---|---|
| AC6 | `crates/maos-registry/tests/vetting_attestation_gate.rs:411` artifact swap; `:415` publisher swap; `crates/maos-spirit-cli/tests/vet_lifecycle_20_1b.rs:8` real issue/promote/revoke | Both swaps genuine before-red; artifact-ignore plant101, 29.352s | Registry all-targets0; real CLI lifecycle0 |
| AC7 | `crates/maos-registry/tests/yank_auth_20_1b.rs:89` signed origins/server restart; `:166` persisted daemon cursor; `yank_cadence_20_1b.rs:38` real server/production30s loop; CLI `vet_lifecycle_20_1b.rs:110` | Unsigned-deprecate plant101, 21.887s | Registry0, real cadence30.52s; publisher/vetter CLI0 |
| AC8 | `crates/maos-bin/tests/yank_policy_20_1b.rs:163` | Wrong typed quarantine outcome plant101, 127.254s | Policy0: warn leaves running, quarantine request is recorded, auto-revoke reaches existing applier; quarantine effect **not claimed in L1** |
| AC9 | `crates/maos-bin/tests/yank_policy_20_1b.rs:237` | Skip-reevaluation plant101, 72.735s | Policy0: one cause-labelled observation per fixture, requested policy outcome |
| AC10 | `crates/maos-registry/tests/yank_auth_20_1b.rs:69`; `crates/maos-bin/tests/yank_policy_20_1b.rs:318` | Bare-name plant101, 34.684s, actually discloses first publisher's spill | Registry/policy0: first-key refusal and real second-boot isolation; local/long-name controls |
| AC14 / F4 | `crates/maos-cli/tests/spirit_install_20_1.rs:307` | Genuine before-red; replacement-guard plant101, 21.527s | Install9/9: same-key replacement and different-registry overwrite refuse with unchanged bytes/origin/inventory |
| AC15 / F5 | D-20-1b-01/02/03 | No plant or security proof invented | **Blocked on design ruling**, no root/TLS/interception closure |
| AC16 / F6 | `crates/maos-cli/tests/spirit_install_20_1.rs:380` | Genuine before-red; removed-yank-refusal plant101, 14.916s | Install9/9: signed yanked version refuses before persistence |
| AC17 | `crates/maos-registry/tests/yank_auth_20_1b.rs:11` | Dropped publish-auth plant101, 15.950s; AC7 independently plants deprecate auth | Registry0: unauthenticated and unauthorized publish/deprecate leave storage unchanged |
| AC11–AC13 | Final row R / KQ1+P and final ledger closure | Not part of L1 | Not claimed or modified |

#### Exact measurements and rebased envelopes

`cargo fmt --all` precedes the exact final measurement. The initial ebf2ab03 aggregate was194743; external 20-11 changes contribute bin+188/iac+3, giving resumed194934; external 20-2 contributes xtask+20, giving194954. Own L1 growth is784, yielding195738. Pins are exact, not the ceilings below.

| Surface | External/current baseline | Own measured delta | Exact pin | Rebased ruled ceiling |
|---|---:|---:|---:|---:|
| maos-bin | 35102 | +320 | 35422 | 35837 |
| maos-registry | 4412 | +215 | 4627 | 5207 |
| maos-spirit-cli | 755 | +181 | 936 | 1895 |
| maos-compliance | 2032 measured; old reserved pin2117 | +38 | 2070 | 2122 |
| maos-domain | 10107 | +30 | 10137 | 10167 |
| xtask | 44676 | **0 own; +20 external** | 44696 | 44920, owner20-2 |
| aggregate | 194954 | +784 | 195738; hardfail195739 | 197774 |
| kernel | 26841 physical / 21042 tokei | **0** | unchanged, same95-file hash | Final R only, below shared STOP21250 |

The x3 unmeasured kernel-free upper forecasts are registry265, spirit-cli380, bin245, compliance30, domain20 (sum940), rebased onto current external exact counts. Bin/compliance/domain exceed their raw forecast uppers by75/8/10 but remain within the ruled x3 envelopes; this is recorded, not hidden by stale pins. No allowance is renewed or reserved against competing stories. maos-cli6429, audit8174 and iac8625 are unchanged by L1; existing unrelated reserved CLI pin remains intentionally unchanged.

The xtask **44676 -> 44696** driver is external `20-2` hosted-3 `d84b83dd`, `render_packages.rs` aligned multiline `%w[]` checksum arrays, confirmed by Main; subsequent `bdfc1430` packaging corrections do not change this exact count. The HISTORY first segment cites live story keys and carries the attribution; the D11 tree-bound tests pass. Compliance's old reserved2117 is lowered to exact2070. Aggregate hardfail is exactly measured+1, with no hidden headroom.

Kernel source remains26841/21042 with set hash `db39bc4a1ab6a6c22575990ae934e698a519d3efd46a8dbb38e62bc376697b59`. Carry the historical reverted R prototype +78/+68, not a new prototype run: forecast26919/21110; D02 A x2 ceilings26997/21178 remain below shared21250. R is final and unimplemented here.

Recommended F5 A/A/A remains a **proposal**: registry+670, cli+40, bin+15, spirit-cli+15 upper forecast; x3 incremental allowances2010/120/45/45 (aggregate2220). From final L1 pins that proposes registry6637, aggregate197958, **1430/184 above** the current respective envelopes; no such increase is applied. The D-row section prices alternatives and custody/development implications.

#### Observed verification and failure accounting

- Registered fixture builder final: exit0, all16 artifacts. Genuine `cargo build --locked -p maos-bin -p maos-cli`: exit0; no target-directory override supplied by this agent.
- Focused final commands: registry all-targets111 passed/2 existing ignored; compliance62 passed; domain309 passed; yank-policy3/3; install9/9; real vet lifecycle2/2; no-default-features verb table7/7. Every command exits0. These are503 passing cases, not a substitute for the separately required workspace execution.
- Real workspace clippy exits0; final ratchet exits0 at **1319 occurrences / 865 source-distinct**, below unchanged1383/897 baseline. The earlier stream was1320/866; the one new needless-borrow test warning is fixed. New production modules have no diagnostics in the final stream; unrelated baseline debt is not claimed closed.
- D11 pin provenance9/9 and test-budget proven-red14/14 pass. New isolated cadence binary owns a39s budget: first observed30.57s ×1.25 rounded up; no existing budget or assertion is widened to hide a failure.
- Structural gates all exit0: kloc-check (alarmtrue, no over-budget rows), kernel baseline, service boundary, pub-field constructors, dependency closure, environment contract122 registrations, serde error handling, exit commands, decision register, test cost, epic-close coherence and dev-record completeness. Ownerless rows reported by the last gate already have recorded dispositions; no unrelated ledger debt is erased.
- Earlier compiler/oracle/fixture failures remain in `commands.json`: legacy yank fixtures lacked sequences, an old server roundtrip was unsigned, Tokio Notify lacked its feature, a policy variable shadowed capability policy, and the initial policy expectation used underscore rather than actual kebab labels. These were corrected at their cause; none is called a successful mutation witness.
- Real CLI revocation exposed the CRL CBOR byte-string decoder defect; its fix passes roundtrip and short-byte refusal vectors. A later stale-fixture guard was respected by rebuilding registered artifacts, not disabled. The inactive-generation red was repaired as described above.
- Invalid `clippy-ratchet --json` invocation exits2; the documented no-json command processes the same real stream and exits0. Missing `python` exits127 **before any cargo/workspace run**; the sequence uses the observed `/usr/bin/python3`. A supplementary async fixture delivery omitted its numeric exit code and is explicitly not used as final gate evidence; the later captured builder exits0.
- Two metadata startup refusals execute **zero tests**: exit2 for incompatible cargo-selection flags during reuse, exit96 for full-list JSON lacking `rust-binaries`. The corrected recipe uses `--list-type binaries-only`, then no cargo-selection flags with `--binaries-metadata`. This is not a retry of an executed test.
- A predecessor JUnit UUID `24d43741-bd54-4575-943f-6c2fea571c5d` was copied after the first startup refusal. That attribution is explicitly invalidated in `metadata-startup-provenance.json` and `workspace-commands.json`; its budget findings are **not** this landing's results. Actual execution requires a newly produced report.
- After standalone clippy, only an obsolete cursor comment was corrected at the same line; no Rust semantics changed. The final workspace sequence refreshes registered fixtures and genuine binaries before metadata reuse.
- No full-workspace or prepush success is inferred from focused passes. Their exact observed records are retained separately as executed; no failed unchanged test is retried-to-green.

#### Standalone workspace and FKCS adjudication

- Final source refresh: formatter0, registered fixtures0/all16, genuine maos/maosctl build0, workspace **binaries-only** metadata0. One actual standalone workspace execution reuses that metadata: run `faf653c8-419b-459c-94bf-845333158f1c`, **5720 run / 5716 passed / 4 failed / 138 skipped**, exit100. All30 landing witness cases pass. Kernel libtest separately passes253/253, exit0.
- Fresh same-run budget gate exits0:613 binaries, sum799.917s against1100s, no findings. The stale predecessor report is not substituted for this UUID or these numbers.
- Two failures are the declared Python `cryptography` backend missing in `erasure_review_20_11` and `erasure_inventory_20_11`, not flakes. Hash-locked `tools/verify-erasure/requirements.txt` is installed into a temporary system-site-packages venv before the independently prescribed prepush; no source assertion is weakened and no standalone test rerun is used to retire those failures.
- The unchanged 1M pending-operation case fails its next observation after route10.000081473s/case36.367s. Its existing timing/I/O family is recorded under owner20-7, same-line rule10 evidence at `deferred-work.md:1468`; no new causal diagnosis, deadline/budget change or AC5 closure is asserted.
- The own FKCS oracle failure is the expected byte fingerprint drift in the deliberately changed registry admission caller. Main explicitly authorizes **pre-bmad-code-review** re-pinning, conditional on the exact one production argument adaptation (`&pkg.manifest_toml` → `VettingSubject::from(pkg)`) plus two test claim fields, unchanged surface/skill admission and unchanged passing negative controls. That exact diff is preserved in `fkcs-admission-review-diff.stdout`.
- Independent canonical path/NUL/u64-LE-length/content hashing reproduces **5b3fe85c… → 136c579d6a2ae4804d0f57864cec8c583363b563e2f063a870f6ae6f38256a46**. `xtask/fkcs-baseline.toml` records Main's authority and explicitly states that this re-pin precedes the independent review. No frozen tag/commit, declared file set, kernel code or gate ordering changes.
- After the authorized fingerprint update, the unchanged FKCS oracle passes11/11 and the actual FKCS gate is fully green, both exit0; the unchanged attested off-surface negative passed in the actual workspace (`workspace-nextest-execution.stderr:3916`). Exact kloc/aggregate/kernel pin checks also exit0 unchanged. Code and all these pin sets must travel together in Main's eventual commit; no commit is fabricated here.
- **Mandatory independent-review focus:** re-examine the one admission caller argument hunk, its two test claim fields, and the authorized FKCS fingerprint change. The local re-pin is not represented as a completed bmad-code-review round.

#### Prescribed prepush — once, frozen tree, not green

Exact invoked argv (`prepush-command.json`), with the hash-locked verifier venv temporarily prepended to PATH:

```bash
env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask MAOS_17_2_CGROUP_MODE=undelegated /mnt/build/cargo-maos/debug/xtask prepush --json
```

- Invoked **once**, completed908.019s, exit1; no deadline truncation or retry. Main acknowledged the tracked-tree/HEAD freeze; this agent performed no tracked writes while it ran. The actual `tracked-tree` record passes.
- Raw `prepush.json` has92 records:83 pass,7 policy skips,1 red,1 advisory. The sole blocking red is workspace-nextest: fresh UUID `e3b670a9-2f63-4ff1-ad10-9307d33a2860`,5720 cases/5719 passes/1 failure. All30 landing witness cases pass again; the sole failure is the unchanged20-11 1M pending-operation observation, route10.00013685s/case42.549s.
- The independently prescribed prepush does not retire the standalone failure. Its own occurrence is owner20-7 at `deferred-work.md:1470`, with same-run failure/JUnit evidence. No unchanged test was retried-to-green.
- The test-budget record is advisory with six findings: uninstall31.429s, hook-pump30.782s, one-door36.376s, participant-recall40.208s and xtask30.811s against unchanged30s binary budgets, plus suite1102.920s against1100s. Exact qualified names and limits are in `prepush-budget-findings.json`, owner20-7 at `deferred-work.md:1471`; no existing budget/exception is widened.
- Kernel libtest, workspace doctests, cargo-deny, formatter, stripped release binary-size and the exercised structural/document gates pass. Docs-site build/routes remain policy-skipped; the preserved T3/strace capability diagnostics are not hosted, delegated-cgroup/rootless/netns or TLS proof.
- The prior missing-cryptography cases and FKCS oracle pass in this separately required run after the documented prerequisite installation and authorized pin update. Their original standalone failures remain recorded; the independent FKCS review still remains pending as explicitly required above.
- The generated intent-lineage report is byte-identical to its recorded HEAD baseline before and after prepush; no unrelated generated report change is carried.

#### Cleanup and final custody

- Final record-completeness and epic-coherence gates exit0 after the observed verification/timing dispositions. Exact argv/exit/elapsed evidence is collected in `commands.md`, `commands.json`, `workspace-commands.json` and `verification-summary.json`.
- Fresh `/proc` scan precedes pruning:26 processes scanned, no active target build or open incremental reference observed;2 process entries disappeared/were unreadable. Complete bounded directory reads work around the tool's reported undocumented200-result glob clamp; no deletion occurs from its partial inventory.
- Removed182 stale incremental session directories, retaining the newest session in each9595 incremental crate directories and every inspected node touched within2h. Removed allocated footprint **2616016896 bytes** (regular-file logical2528538121); never deps/build/fingerprint or the protected worktree.
- Removed all11 owned `/tmp/201b-*` scratch roots, allocated footprint **30552064 bytes** (regular-file logical27899769), including the temporary verifier venv and the Main-authorized park archive. `park-MANIFEST.txt` was copied first with matching SHA-256; `park-custody.json` records the authority and completed disposition. Future verifier runs still require the declared hash-locked requirements; no global Python installation is claimed.
- Observed target-filesystem available-space increase: **1962065920 bytes**. `cleanup.json` distinguishes allocated/unlinked footprint from physical shared/reflink reclamation and records the shared-host measurement, rather than claiming those quantities are identical.

#### Limits and ledger disposition

F5 first-contact authenticity/TLS remains open; signing mutations and locally pinning the first publisher do not establish the initial trust root. The server is still a loopback plaintext development listener. No hostile-registry/TLS-interception refusal, public deployment, hosted runner result, delegated-cgroup/rootless/T3 quarantine effect or independently accredited vetter ecosystem is claimed. The branch recipe uses actual harness fixture/test names and labels only the30 kernel-free witnesses.

AC8's quarantine-effect witness is **not claimed in L1** and depends on final KQ1 with AC11–AC13; story and tracker state this explicitly. Kernel-owned deferred FR13/poller rows and I5 remain untouched. Implemented local attestation-binding, principal-identity and inherited security rows now name their witnesses and historical findings; commit-backed closure remains Main-owned, rather than inventing an uncommitted landing hash. FR37 stays v2.0+; FR59 names its local witnesses without falsely closing F5 or KQ1.

Creation D/S numeric pointers in older acceptance headings are historical. Re-measured code positions are in `cites-ebf2ab03`; current `deferred-work.md` anchors are inherited security54–57, principal identity103, FR59 row122, attestation1160, final-kernel FR13 row1214/poller1336; `sprint-status.yaml` has20-1b at298. No old a4c00897 count is passed off as an ebf2ab03 measurement.

### File List

Tracked inventories: initial42 paths in `landing-tracked-file-list.stdout`; final43, including the authorized FKCS fingerprint, in `landing-tracked-file-list-final.stdout`. Grouped implementation inventory:

- `Cargo.lock`; `.config/nextest.toml`; `xtask/{kloc.toml,test-budgets.toml,fkcs-baseline.toml}`.
- `crates/maos-bin/src/{admission.rs,env_contract.rs,installed_spirits.rs,lib.rs,main.rs,yank_policy.rs}`; `crates/maos-bin/tests/{cohort_daemon_smoke_13_5c.rs,yank_policy_20_1b.rs}`.
- `crates/maos-domain/src/{ports/registry.rs,revocation.rs}`; `crates/maos-compliance/src/vetting/{attestation.rs,mod.rs}`.
- `crates/maos-registry/{Cargo.toml,SECURITY.md}`; `src/{admission.rs,client.rs,handlers/deprecate.rs,handlers/publish.rs,install.rs,lib.rs,operations.rs,server.rs,storage.rs,yank.rs,yank_auth.rs}`.
- `crates/maos-registry/tests/{end_to_end_test.rs,form_admission_17_3b.rs,fr59_yank_propagation_within_5min_test.rs,registry_security_20_1a.rs,registry_server_bin_20_1.rs,vetting_attestation_gate.rs,yank_auth_20_1b.rs,yank_cadence_20_1b.rs}`.
- `crates/maos-spirit-cli/{Cargo.toml,src/bin/maos-spirit.rs,src/lib.rs,src/vet.rs,src/yank.rs,tests/vet_lifecycle_20_1b.rs}`; `crates/maos-cli/tests/spirit_install_20_1.rs`.
- `tests/harness/{registry_20_1.rs,registry_trust_20_1b.rs}`; `tests/coverage-matrix.yaml`; `docs/adr/ADR-056-fr37-vetting-machinery.md`.
- This story; `_bmad-output/implementation-artifacts/{deferred-work.md,sprint-status.yaml}`; the landing-1 evidence tree and branch-only `ci/ci-20-1b-verify.yml`.

No `crates/maos-kernel-core/`, `.github/` or `xtask/src/` file is in the owned diff. External xtask renderer changes are committed 20-2 drivers, not attributed to this file list.

### Change Log

- Operator `f1b` scope and Main's kernel-last AC8 split recorded; F5 choices remain priced PROPOSED D-rows.
- Implemented exact vetting/real CLI lifecycle, authenticated authorized mutations, immutable installs/yank refusal, durable signed yank propagation, typed policy requests/live observations and publisher-separated principal spill identity.
- Added genuine before/plant/final evidence, real cadence budget enrollment, current citations/pins and documented server/keyring/transport limits; no status flip or kernel landing claimed.
