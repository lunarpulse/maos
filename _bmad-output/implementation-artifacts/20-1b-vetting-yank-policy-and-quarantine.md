---
story_id: '20.1b'
story_key: 20-1b-vetting-yank-policy-and-quarantine
created: 2026-11-01
split_from: 20-1-registry-client-install-verb-vetter-and-yank (D-20-1-01 B, RULED 2026-11-01/02, operator ask split201)
context_commit: a4c00897
baseline_commit: a4c00897
security_sensitive: true
kernel_grant: "L1 KERNEL-FREE, RULED Main 2026-11-06: AC8 quarantine effect joins final row R (AC11-AC13 KQ1+P); no KQ0 substitute. Rebase prior measured prototype +78 physical/+68 tokei onto ebf2ab03 and current unchanged26841/21042: forecast26919/21110; D02 A x2 ceilings26997/21178, below shared STOP21250 tokei. No prototype rerun or kernel change claimed in L1. Historical superseded grant: ◆ FLAG-Winston RULED 2026-11-01/02 (ask rows201, operator): row R = KQ1 sticky quarantine (D-20-1-02 A) + P poller edge-trigger (D-20-1-03 A), measured by reverted prototype +78 physical / +68 tokei from HEAD 26801 / 21011 (+40 / +40 of it is the rustfmt churn any `revocation/applier.rs` edit pays, control `fmt0`). Granted ceilings: kernel <= 26957 physical / 21147 tokei. This grant is computed from HEAD 26801 / 21011 and lands in THIS story only (20-1a is kernel-free). INTERACTION with 20-7 row R = K1w + K2r (+40 / +31 measured; ceiling 26881 / 21073, also computed from HEAD 26801 / 21011): if both land, the second to land MUST re-base; cumulative measured tokei +68 +31 => expected 21110; cumulative tokei ceilings MUST NOT exceed 21209 (= 791 under the ADR-069 22000 alarm); STOP otherwise. Room per ADR-069: docs/adr/ADR-069-kernel-core-envelope-through-v1.md:35-46."
kloc_grant: "AC15 INDEPENDENT-REVIEW CORRECTIONS at9e8c7737, 2026-11-08: Main D-20-1b-09..13; existing ceilings registry7081/cli6549/bin35671/spirit-cli984/aggregate198621 retained; additional domain correction ceiling10155 = exact10137+18 authorized for D3/F5, no kernel draw. Exact after cargo fmt --all / canonical kloc-check: registry6151/bin35586/domain10144/cli6429/spirit-cli942/compliance2077/audit8179/xtask44696; aggregate197451, strict hardfail197452; delta487 = registry483 + domain7 - bin3. All within ruled ceilings; no renewed capacity or source-shrink claim for legacy CLI reserve. Operator D5 confirms exact6429 and forfeits427 from6856; preflight/tracker carry disposition. D4 replaces public roots with configured CA; otherwise bundled Mozilla, not platform. D2 registry edges isolated; frozen kernel/A2A TLS remains in whole no-default graph under Main honest-record fallback. Kernel26841/21042 and external20-2 xtask44696 d84b83dd unchanged. Audit8179/ceiling8204 and separate20-11 ceiling8261/remaining82 unchanged. | HISTORICAL AC15 IMPLEMENTATION: AC15 CARGO HANDOFF c656a6e7: orchestrator-adjudicated rebase, D02 A, disclosed to the operator; same granted increments2010/120/45/45 aggregate2220 on correction pins5071/6429/35626/939/196401. Ceilings registry7081/cli6549/bin35671/spirit-cli984/aggregate198621. Exact after cargo fmt --all and canonical kloc-check: registry5668/bin35589/cli6429/spirit-cli942/compliance2077/audit8179/domain10137/xtask44696; aggregate196964, strict hardfail196965; own delta563 = registry597 + spirit-cli3 - bin37. Kernel26841/21042 unchanged. Audit correction ceiling8204 and separate20-11 ceiling8261 unchanged; no renewed L1 capacity. | HISTORICAL L1 REVIEW CORRECTIONS (Main, 2026-11-07): existing L1 ceilings bin35837/registry5207/spirit-cli1895/compliance2122/domain10167/aggregate197774; additional P3 audit ceiling8204 =8174+30 under D02 A x3 forecast, separate20-11 ceiling8261 unchanged. Formatted correction pins bin35626/registry5071/spirit-cli939/compliance2077/audit8179/domain10137/xtask44696, aggregate196401 and strict hardfail196402; correction delta663, kernel0. Old AC15 absolute/auxiliary bounds below require Main re-base at the explicit handoff; not a source of L1 capacity. | HISTORICAL AC15 GRANT RECORD: RULED operator f5rows 2026-11-07: D-20-1b-01/02/03 A/A/A; AC15 second kernel-free pass AFTER L1 review/corrections and explicit Main CARGO HANDOFF AC15. Registry ceiling6637; aggregate ceiling197958. From last formatted L1 registry4627/aggregate195738, new unmeasured x3 allowances2010/2220; cli+40/bin+15/spirit-cli+15 forecasts x3 give auxiliary per-pass bounds6549/35467/981 from6429/35422/936. Remeasure intervening external drift under D02 A, do not renew unused L1 allowances. Compliance2122/domain10167/external xtask44920 unchanged. Rustls leaf/server dependency edge granted subject to cargo-deny permitted versions, existing-lock ring/aws-lc feature review, and no no-default-features leak. Operator-supplied cert/key paths; configured canonical-URI signing root; explicit both-end literal-loopback dev HTTP only; never HTTPS downgrade. RECORDS ONLY: exact pins remain registry4627/bin35422/cli6429/spirit-cli936/compliance2070/domain10137/xtask44696, aggregate195738/hardfail195739; kernel26841/21042 unchanged; no dependency/source/pin/status change or new verification. Historical L1 grant, superseded for AC15: L1 REBASE, operator f1b/Main 2026-11-06, D02 A x3 unmeasured kernel-free forecast: current external baseline bin35102, registry4412, spirit-cli755, compliance2032 measured (old reserved pin2117), domain10107, aggregate194954 including external20-2 xtask20. Forecast upper increments245/265/380/30/20; ceilings bin35837, registry5207, spirit-cli1895, compliance2122, domain10167, aggregate197774. Exact formatted L1 pins bin35422/registry4627/spirit-cli936/compliance2070/domain10137/xtask44696; aggregate195738, hardfail195739. xtask44676 -> 44696 is external20-2 hosted-3 d84b83dd, authorized ceiling44920, not own Rust growth. CLI6429 unchanged, existing6666 ceiling/reserved6856 pin unchanged. No kernel/audit/iac growth by L1; F5 prices remain PROPOSED D-20-1b-01/02/03 and are not granted. These are shared ceilings, not reservations; remeasure future competing draws. Historical superseded grant: RULED 2026-11-01/02 (ask rows201; D02 A; D-20-1-11 A). One-key ceilings (STOP above; aggregate hardfail = measured + 1 at landing): kernel <= 26957 / 21147 (row R, this story); maos-registry <= 5163; maos-spirit-cli <= 2043; maos-cli <= 6666 (above its 6382 STOP; inside the reserved 6856 pin because install logic lives in maos-registry, D-20-1-05 A); maos-bin <= 34410; maos-shell <= 728; maos-compliance <= 2122; maos-domain <= 10153; xtask <= 42920; aggregate <= 193768. Each ceiling is shared with 20-1a; the kernel row is 20-1b's alone."
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
  dispositions are superseded. F5's initially unruled design is superseded by
  operator ask `f5rows` below; this does not claim its implementation or proof.
- Main's explicit landing-split ruling: AC8's sticky quarantine **effect** and
  its effect witness move to the final KQ1 kernel landing with AC11–AC13.
  Landing 1 implements the kernel-free decision, typed boundary outcome and
  operator-visible `quarantine requested` record; **no quarantine effect is
  claimed in L1**. KQ0 / a resumable pause is not an approved substitute.
  The final kernel landing remains last; no kernel source change in L1.

### L1 independent-review correction rulings — 2026-11-07

Main authorized the `landing-1/review/triage.md` correction pass at `62faff63`:
P1–P18, security F1–F5 and the following D1–D5 choices. This is not the separate
AC15 cargo handoff. No kernel-dependent repair is included.

| ID | Review choice and disposition | Authority / price |
|---|---|---|
| D-20-1b-04 | D1 **A**: resolve the named installed `SignedPackage` at the operator door and carry it explicitly through snapshots/guards; prove a genuine vetted upgrade and swapped deployed bytes/publisher refusal. B (declare unsupported) rejected. | RULED A, Main correction handoff 2026-11-07; existing L1 bin/registry envelopes. |
| D-20-1b-05 | D2 **A**: durable publisher/version-digest ledger surviving uninstall, explicit local operator `release-pin`, and import of every retained legacy signed generation before deletion. B (per-installation pin) rejected. | RULED A, Main correction handoff 2026-11-07; existing L1 registry envelope. |
| D-20-1b-06 | D3 **A**: immutable server name/version; identical republish idempotent; initial package files staged and atomically renamed together. B (mutable publication) rejected. | RULED A, Main correction handoff 2026-11-07; existing L1 registry envelope. |
| D-20-1b-07 | D4 **A**: atomically renew verified `vetting.json` only when package and origin are identical; no uninstall/data-loss requirement. B (require uninstall) rejected. | RULED A, Main correction handoff 2026-11-07; existing L1 registry envelope. |
| D-20-1b-08 | D5 **A**: label enrollment/revoke-key witnesses fixture-keyring based; explicitly retain the missing shipped operator verbs as an open, rule-10-owned obligation. Do not add those verbs in this correction pass. | RULED A, Main correction handoff 2026-11-07; documentation only. |

Main separately authorized P3's audit correction ceiling **8204 = 8174 + 30**
(D02 A, unmeasured +5…10 forecast ×3). The separate 20-11 ceiling8261 is unchanged;
neither its unused room nor the later AC15 grant funds L1 corrections. No private
data migration is implemented: this pre-release cutover requires operator
export/reinstall for old bare-identity data; uninstall addresses both old and
publisher-bound owners.

### F5 design decisions — RULED A/A/A, `f5rows`, 2026-11-07

Operator ask `f5rows` rules D-20-1b-01/02/03 **A/A/A** and grants a second,
kernel-free AC15 pass **after the landing-1 review round and corrections**.
Main issued **`CARGO HANDOFF AC15` at clean HEAD `c656a6e7`** after committing the
L1 review corrections. The hold is fulfilled; this second pass is kernel-free.
The **orchestrator-adjudicated rebase, D02 A, disclosed to the operator** is
registry7081 /cli6549 /bin35671 /spirit-cli984 /aggregate198621, applying the
originally granted increments2010/120/45/45 (aggregate2220) to corrected exact
pins5071/6429/35626/939/196401. It does not renew the L1 correction envelopes.
The old absolute bounds and hold labels retained below are historical provenance.

Prices remain unmeasured forecasts, not exact pins or reserved capacity. D02 A
uses ×3 upper bounds for AC15; remeasure intervening external changes at handoff.
The creation baseline `ebf2ab03` remains historical: registry4412, spirit-cli755,
cli6429, bin34914, compliance2032, domain10107, xtask44676, aggregate194743;
kernel26841 physical /21042 tokei is unchanged.

| ID | Choice and options | Recommendation, price and envelope consequence | Status |
|---|---|---|---|
| D-20-1b-01 | First-contact root: **A** explicit registry signing public key in operator config, bound to canonical registry URI, with registry-signed artifact/publisher metadata; **B** TOFU only after mandatory out-of-band fingerprint confirmation; **C** operator-distributed signed root bundle with expiry and rotation. A response-supplied key alone is never a root. | **A**: no interactive trust race, unattended installs are deterministic. A: registry +100…180, cli +0…10; ×3 allowance +540/+30. B: registry +150…260, cli +30…60; +780/+180. C: registry +220…380, domain +20…40; +1140/+120. All kernel 0. Existing tier-only signature is insufficient: it does not bind artifact or publisher. | RULED A — operator `f5rows`, 2026-11-07; `CARGO HANDOFF AC15` fulfilled atc656a6e7; implemented in second kernel-free pass |
| D-20-1b-02 | Authenticated TLS/key custody: **A** rustls in the leaf HTTP transport and registry server; explicit certificate chain and private-key files supplied by path, hostname validation and bundled Mozilla WebPKI without a configured CA; a configured CA replaces public roots (D-20-1b-12); **B** external TLS terminator with registry accessible only through a separately authenticated local upstream. | **A**: end-to-end refusal can be witnessed in owned binaries without assuming a correctly configured proxy. Registry +240…420, bin/cli +0…15 each; ×3 allowance +1260/+45/+45; dependency/lock/deny review required. B: registry +100…180 plus deployment/harness work; +540 allowance, but upstream authentication must be designed rather than treating loopback as authentication. Key files live outside the package/inventory store, operator-owned 0600; certificate chain may be 0644. No generated production key or trust material is silently installed. All kernel 0. | RULED A — operator `f5rows`, 2026-11-07; `CARGO HANDOFF AC15` fulfilled atc656a6e7; implemented in second kernel-free pass |
| D-20-1b-03 | Loopback/development: **A** require explicit development-mode opt-in at both server and install; permit HTTP only for literal loopback addresses, keep mutation signatures and publisher pins; **B** TLS everywhere including loopback with operator-created test CA. Never downgrade HTTPS after failure. | **A**: retains executable local fixture workflows while preventing production URI downgrade. Registry +35…70, cli +5…15, spirit-cli +5…15; ×3 allowances +210/+45/+45. B: registry +10…25, CLI 0, more fixture certificate custody. A is an explicit exception, not authenticated transport evidence; F5 production hostile-registry/interception witnesses must use TLS and the configured root. All kernel 0. | RULED A — operator `f5rows`, 2026-11-07; `CARGO HANDOFF AC15` fulfilled atc656a6e7; implemented in second kernel-free pass |

**Granted A/A/A AC15 envelope (`f5rows`, 2026-11-07):** registry6637 and
aggregate197958. Upper forecasts registry+670, cli+40, bin+15, spirit-cli+15
give incremental ×3 allowances2010/120/45/45 (aggregate2220).
From last formatted L1 pins registry4627 /cli6429 /bin35422 /spirit-cli936,
aggregate195738, the auxiliary AC15 per-pass bounds are cli6549 /bin35467 /
spirit-cli981. These do not renew unused L1 allowances. Existing compliance2122,
domain10167 and externally owned xtask44920 ceilings are unchanged.

The approved registry/aggregate ceilings supersede L1's5207/197774 for AC15,
increases1430/184. They are **grants, not new exact pins**: no kloc row, aggregate
hardfail, dependency, source file or kernel pin changes in this recording pass.
At the eventual handoff rebase only observed external drift per D02 A.

**Unselected historical alternatives (not authorized by `f5rows`):**

- Individually, root B's ×3 registry ceiling5407 exceeds5207 by200; root C
  proposes registry5767 and domain10257, exceeding5207/10167 by560/90.
  Root A alone fits its registry allowance. TLS A alone proposes registry5887
  (680 above5207); TLS B alone and either development option fit the
  current registry ceiling. Combining choices adds their prices, not their
  separately rebased baselines; competing story draws must be remeasured.

The preceding `f5rows` stage was records-only. Main's `CARGO HANDOFF AC15` at
`c656a6e7` now fulfils that hold, with the explicitly adjudicated rebase above.
Implementation and refusal controls are in the AC15 Dev Agent Record and
`landing-1/ac15/`. Main's independent AC15 review is saved under
`landing-1/ac15/review/`; the subsequent correction pass is not its own re-review.


### AC15 independent-review correction rulings — 2026-11-08

Baseline `9e8c7737b0aaae730024ae66966c20106563141e`; Main holds HEAD/tree
writes until this agent explicitly releases the freeze. Review identifiers D1–D5
below belong to `landing-1/ac15/review/triage.md`, not the older L1 review.

| ID | Ruled choice and correction boundary | Authority / price |
|---|---|---|
| D-20-1b-09 | Review D1 A: fail-closed daemon; CLI and daemon share one strict reader and banner. Reject unknown keys even without a URI. HTTPS env-URI overrides cannot change the TOML-bound pin. | RULED A by Main; existing registry7081/bin35671/aggregate198621 ceilings, no kernel draw. |
| D-20-1b-10 | Review D2 A for registry edges: explicit forwarded TLS features, no default TLS through CLI/FKCS/shell. The overall no-default binary still has pre-existing frozen kernel/A2A TLS dependencies; withdraw the former “no rustls edge” claim rather than edit the frozen kernel. | Main explicitly permits honest-record fallback if total TLS removal is impossible. Manifest/test-only feature isolation; CLI code stays6429. A genuinely TLS-free whole graph needs a separately priced frozen-kernel/network-feature change, not a silent L1 edit. |
| D-20-1b-11 | Review D3 A: authenticated head below requested/durable cursor is typed `HeadBehindCursor`, visible and unchanged; never silently reset to0. Signing-key-bound cursor generations remain deferred. | RULED A by Main; separately authorized domain ceiling10155 = exact10137 +18, disclosed review-correction draw, inside aggregate198621. No kernel code change. |
| D-20-1b-12 | Review D4 A: configured CA replaces public roots; absent CA uses bundled Mozilla WebPKI, not platform roots. Ed25519 URI-bound envelope remains the metadata root. | Explicit operator ruling relayed by Main; existing registry envelope. Supersedes the earlier “platform/optional additive CA” wording. |
| D-20-1b-13 | Review D5: confirm exact CLI6429 and explicitly forfeit427 of the former6856 reservation. The427 is neither source shrinkage nor a transferable/renewed story allowance. | Explicit operator confirmation relayed by Main; recorded in tracker and Epic20 preflight as well as this story. AC15 ceiling6549 is separate from that forfeited legacy reserve. |

## Acceptance Criteria

### Part B — trust, yank and quarantine (L2 kernel-free + L3 ◆, or `20-1b` under D-20-1-01 B)

#### AC6 — the vetting attestation binds the exact artifact and publisher (D:1093); vetters can issue and revoke

1. `VettingClaim` (`crates/maos-compliance/src/vetting/attestation.rs:105-133`) binds `artifact_sha256` and the publisher public key; `verify_attestation` (`vetting/mod.rs:220`) and its caller (`crates/maos-registry/src/admission.rs:388`) check both; golden byte pin (`attestation.rs:226-233,280`) updated; ADR-056 §1 amended.
2. `maos-spirit vet issue|revoke` issues attestations and writes operator-signed attestation revocations. It does **not** enroll/revoke a vetter key; those key-lifecycle tests use fixture-built keyrings, with the missing operator verbs explicitly deferred (D-20-1b-08).
3. **Local witnesses:** swapped artifact/publisher under a valid attestation refuse; original assertion-red runs used a partially edited L1 tree, not a clean commit baseline. Real `vet issue` → install promotes, and `vet revoke` → next admission refuses. V2 role-domain vectors additionally refuse old unscoped attestation/key-event signatures.
4. **Plant (one, local):** `verify_attestation` ignores `artifact_sha256` ⇒ the swap vector red.

#### AC7 — yanks arrive signed, labelled and restart-safe (FR59; D-20-1-06 A, D-20-1-07 B)

1. `maos-spirit yank <name>@<ver>` (publisher key) and the vetter's yank (enrolled vetter key) send signed requests; the server verifies publisher yanks against the stored package's key and vetter yanks against the keyring; unsigned `registry.deprecate` is refused. `YankEntry` carries `origin` and a server sequence; the daemon re-verifies before acting and journals origin-labelled rows distinguishable from operator CRLs (intent + `RevocationOrigin` label).
2. The yank cursor survives a server restart and a daemon restart (persisted; F2, F3).
3. **Local witnesses:** unsigned deprecate refuses; server restart between yanks fetches the second; daemon restart does not replay journaled rows. The original restart cases had no observed before-red and are not represented otherwise. Propagation uses a separately budgeted real 30 s production loop; review adds actual `YankPoller`/`YankRuntime` injected-clock restart coverage, URI-bound cursors, verified-only advancement and rejected-row isolation. Authenticated omitted/rolled-back head detection remains AC15 work.
4. **Plant (one, local):** accept unsigned deprecate ⇒ red.

#### AC8 — yank policy `warn | quarantine | auto-revoke` (D-20-1-08 A)

1. `maos_registry::yank::YankPolicy` read by maos-bin from `MAOS_YANK_POLICY` (registered `UserFacing`; default `warn`), not a `RegistrySection` field. `warn` journals only; `quarantine` delivers a typed request and operator-visible record in L1; `auto-revoke` submits an operator-key-signed local CRL with origin `RegistryYank` through existing `apply_crl`.
2. **L1 witnesses:** policy selection, typed request, signed propagation and request record, plus warn and auto-revoke behavior. **Final-kernel dependency, not claimed in L1:** quarantine's next mediated call refused while still loaded, using the AC11 KQ1 sticky fence. Main ruled this effect/witness into the final kernel landing on 2026-11-06; no KQ0 substitute.
3. **Plant:** ignore policy selection ⇒ the typed quarantine-request vector red in L1; the sticky-effect vector belongs to the final kernel landing.

#### AC9 — a lapsed or revoked promotion reaches operators as a vetter yank (D9; D-20-1-10 A)

1. On the yank cadence the daemon re-evaluates each running `public-vetted` Spirit's attestation; expiry or revocation journals `RunningSpiritObservation` (`crates/maos-compliance/src/vetting/terminal.rs:99-152`) with its cause label and applies the yank policy as a vetter-origin yank. `enforce_vetted_upgrade_precondition` (`crates/maos-bin/src/main.rs:130`) stays the only refusal point; no crossing check.
2. **Local witnesses:** injected expiry and fixture-keyring revocation yield one cause-labelled observation and policy request each. The original tests were direct runtime calls, not daemon cadence proof. Review adds real poller/runtime clock+cursor restart and independent no-registry local-timer coverage; enrollment/revoke-key operator verbs remain open.
3. **Plant (one, local):** skip the re-evaluation ⇒ red.

#### AC10 — a name cannot be squatted (D:37; D-20-1-09 A)

1. The server pins the first publisher per name. Installation additionally retains publisher and version-digest pins across uninstall; only explicit local `release-pin` releases them. One shared identity formatter binds publisher-separated principal spill at production `load_admit_start`; local Spirits keep the bare identity.
2. **Local witnesses:** typed different-publisher refusal and restart; durable pin/version refusal after uninstall and legacy-generation import; two actual admission processes with recycled PID force K1→K2 sidecar replacement and refuse K1's spill. The original hand-bound witness did not cover production binding; review's production-binding mutation now does.
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
3. Under `f5rows` (2026-11-07), use the canonical-URI-bound operator-configured
   registry signing public key; a response-supplied key alone is never a root.
4. Use rustls in both leaf client and server, operator-supplied cert-chain/key
   paths, platform/WebPKI trust plus optional operator CA, and hostname validation.
   Keep private key material outside package/installation storage; no silent
   production key generation. Permit HTTP only for literal loopback addresses
   with explicit development opt-in at **both** server and install; preserve
   mutation signatures/publisher pins and never downgrade after HTTPS failure.
5. The rustls dependency edge is granted subject to cargo-deny and review of
   ring/aws-lc feature selection against the existing lock. No new crate
   versions beyond deny policy; rustls must not leak into the no-default-features
   build. The development exception is not TLS proof.
6. Plant: bypass root/peer verification ⇒ red. Main's `CARGO HANDOFF AC15` is
   fulfilled; executed negative controls/restoration and final results are recorded below.

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
  Its trust-root/TLS/development choices are now RULED A/A/A by `f5rows`,
  2026-11-07; implementation waits for the review round/corrections and Main's
  explicit cargo handoff. No first-contact/TLS closure is claimed by this record.
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

**Historical L1 handoff status, before `f5rows`:**

Operator `f1b` (2026-11-06) adds AC14–AC17. AC6–AC10's kernel-free work and AC14/16/17 are implemented with local refusal/effect witnesses. **AC15/F5 remains blocked on PROPOSED D-20-1b-01/02/03**, not silently designed. Main's explicit split puts AC8's sticky quarantine **effect and effect witness** with AC11–AC13 in the final KQ1 kernel landing: **not claimed in L1**. L1 delivers the signed decision, typed boundary outcome and operator-visible quarantine-requested record; no KQ0/resumable-pause substitute.

**Superseding records-only ruling — `f5rows`, 2026-11-07.** D-20-1b-01/02/03
are now RULED A/A/A with the AC15 envelope above. The preceding L1 handoff's
“blocked on PROPOSED D-rows” is historical. No new code or proof is claimed;
AC15 execution remains held until the review round/corrections finish and Main
sends `CARGO HANDOFF AC15`. Kernel-last KQ1 scope and all L1 observations remain
unchanged.

**Current supersession:** the explicit Main handoff at`c656a6e7` fulfilled that
records-only execution hold. The subsequent AC15 record below claims the observed
local implementation/proof only; no final-KQ1 or hosted security closure is inferred.

No kernel source, `.devcontainer`, protected worktree, workflow on main, commit, push or tracker status was changed by the original implementation agent. Main subsequently committed L1 at62faff63 and performed the independent review recorded in `landing-1/review/triage.md`; the correction pass below does not claim its own independent re-review or hosted proof.

Work began at `ebf2ab03ac64de5b828d995f72ae8b24199fa9a0`. At Main's park request, owned changes were archived, removed and an empty status was observed; after the exclusive cargo slot resumed, that archive was reapplied onto `063c13cf`. Current measured source includes external 20-2 commits through `bdfc1430`; `git log --oneline -5` and exact revision capture are preserved. `resume-baseline-kloc` ran **after WIP reapplication**, so its failing measurement is not represented as a clean baseline.

#### Historical implementation at original L1 handoff (superseded by corrections below)

- Required vetting fields bind exact manifest, artifact SHA-256 and publisher key. An independent `VettingSubject` reaches verification; both hostile swaps are correctly re-signed so a publisher-signature failure cannot masquerade as attestation binding. Canonical CBOR golden bytes and ADR-056 are updated; old manifest-only attestations require reissuance.
- Real `maos-spirit vet issue|revoke` signs the claim and operator-root CRL/keyring update; real publisher and enrolled-vetter `yank` commands send domain-separated signed authorizations. The real revoke/install lifecycle exposed the existing CRL fixed-byte CBOR decoder mismatch; accepting the serializer's byte-string representation fixes that codec without changing signed bytes. Wrong 63/31-byte vectors still refuse.
- Publish authenticates exact package bytes and signed manifest name/version, then authorizes against first-publisher ownership across versions. Deprecate authenticates publisher ownership or root-signed vetter enrollment. Unsigned/wrong-owner/unenrolled mutations leave storage unchanged. Server keyrings are startup snapshots requiring restart after enrollment/revocation changes, explicitly documented; daemon promotion re-evaluation instead reads the retained live keyring path.
- Installed package, publisher, origin and promotion metadata are immutable for an existing name/version, including inactive generations. Identical reinstalls reuse the original immutable generation; the additional observed-before regression where another version remained active is repaired and witnessed. Cross-process mutation serialization uses the repository's rustix flock pattern.
- Yanked versions refuse before local storage construction/persistence. Durable server sequences replace clock-derived cursors; legacy `since_ns`/`last_seen_ns` wire names explicitly mean sequence. Atomic cursor persistence is used in production polling and joined shutdown. Historical unsigned yanks can refuse installation but cannot authorize daemon effects; obsolete clock-remapping code is removed.
- Remote yanks are re-verified against locally installed publisher/enrollment evidence before origin-labelled journaling. `warn` records only; quarantine emits `QuarantineRequested` and its operator record; auto-revoke creates an operator-key-signed exact-version `RegistryYank` CRL through the existing applier and honors the declared action. The poll guard joins normal shutdown and aborts early-exit tasks; a disabled guard allocates nothing.
- The same cadence re-evaluates running promotions using an injected clock and live keyring, deduplicating observations labelled `expiry-lapse` or `vetting-revocation` before the same policy seam. No new crossing-time refusal is introduced.
- Registry admission derives a publisher-fingerprinted principal identity, hashes long names to stay below the encoded filename limit, and binds it at the existing load boundary. Signed sidecars feed boot, standalone and operator-door paths; ordinary local Spirits retain the bare identity.
- `MAOS_YANK_POLICY` is registered UserFacing; the new maos-bin module is in the source scanner. New tests invoke existing harness-resolved binaries, never cargo.

#### Historical AC witness matrix — original L1 results before `f5rows`

All paths below are repository-relative. Each plant is an executed runtime assertion failure (exit101), followed by copy-back + cmp + touch (exit0). Exact commands and elapsed times are in the linked evidence, not inferred from source.

| AC | Named witness and line | Before / plant | Final focused observation |
|---|---|---|---|
| AC6 | `crates/maos-registry/tests/vetting_attestation_gate.rs:411` artifact swap; `:415` publisher swap; `crates/maos-spirit-cli/tests/vet_lifecycle_20_1b.rs:8` real lifecycle | Assertion-red on a partially edited L1 tree, **not** a clean bdfc1430 baseline; artifact-ignore plant101 | Original focused registry/CLI0; review corrections supersede current-state claims below |
| AC7 | `crates/maos-registry/tests/yank_auth_20_1b.rs:89` signed origins/server restart; `:166` persisted daemon cursor; `yank_cadence_20_1b.rs:38` real server/production30s loop; CLI `vet_lifecycle_20_1b.rs:110` | Unsigned-deprecate plant101, 21.887s | Registry0, real cadence30.52s; publisher/vetter CLI0 |
| AC8 | `crates/maos-bin/tests/yank_policy_20_1b.rs:163` | Wrong typed quarantine outcome plant101, 127.254s | Policy0: warn leaves running, quarantine request is recorded, auto-revoke reaches existing applier; quarantine effect **not claimed in L1** |
| AC9 | `crates/maos-bin/tests/yank_policy_20_1b.rs:237` | Skip-reevaluation plant101, 72.735s | Policy0: one cause-labelled observation per fixture, requested policy outcome |
| AC10 | Original `yank_auth_20_1b.rs:69`; original `yank_policy_20_1b.rs:318` | Bare-name plant101 disclosed spill in a **hand-bound** fixture, not production admission; no original K1→K2 before-red | Original unit-level identity proof only; review replaces it with two real admission processes |
| AC14 / F4 | `crates/maos-cli/tests/spirit_install_20_1.rs:307` | Assertion-red on a partially edited tree; replacement plant101 | Original first same-key case was red; different-registry iteration varied the key and was not independently before-red. Review adds identical/changed same-key other-registry controls |
| AC15 / F5 | D-20-1b-01/02/03 | No plant or security proof invented | Design was blocked at L1 handoff; now RULED A/A/A, execution held for review/corrections and explicit cargo handoff; no root/TLS/interception proof |
| AC16 / F6 | Original `spirit_install_20_1.rs:380` | Assertion-red on a partially edited tree; removed-yank-refusal plant101 | Original signed-yank pre-persistence refusal; review verifies present-entry authority and explicitly preserves the AC15 omission dependency |
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
The following F5 price comparison was the **pre-f5rows proposal**. Its A/A/A
registry6637/aggregate197958 grant is now approved above; original arithmetic
and L1 measurements below are retained as provenance, not an unruled choice.
At L1 handoff the A/A/A proposal forecast registry+670, cli+40, bin+15 and spirit-cli+15; ×3 incremental allowances2010/120/45/45 (aggregate2220). Its proposed registry6637/aggregate197958, increases1430/184 over L1 ceilings, are now granted by `f5rows`. Exact pins remain unchanged; the prior L1 measurements are historical proof, not a new AC15 measurement.

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

F5 first-contact authenticity/TLS remains open, now RULED A/A/A rather than blocked on a design ruling. Mutation signatures/local pins are not that root. No hostile-registry/TLS-interception refusal, public deployment, new hosted result, delegated-cgroup/rootless/T3 quarantine effect or accredited-vetter ecosystem is claimed. The updated branch-only recipe and correction evidence below supersede the original30-witness selection.

AC8's quarantine-effect witness is **not claimed in L1** and depends on final KQ1 with AC11–AC13; story and tracker state this explicitly. Kernel-owned deferred FR13/poller rows and I5 remain untouched. Implemented local attestation-binding, principal-identity and inherited security rows now name their witnesses and historical findings; commit-backed closure remains Main-owned, rather than inventing an uncommitted landing hash. FR37 stays v2.0+; FR59 names its local witnesses without falsely closing F5 or KQ1.

Creation D/S numeric pointers in older acceptance headings are historical. Re-measured code positions are in `cites-ebf2ab03`; current `deferred-work.md` anchors are inherited security54–57, principal identity103, FR59 row122, attestation1160, final-kernel FR13 row1214/poller1336; `sprint-status.yaml` has20-1b at298. No old a4c00897 count is passed off as an ebf2ab03 measurement.

### L1 independent-review correction Dev Agent Record — 2026-11-07

**Authority/baseline.** Main handed off P1–P18, security F1–F5 and D1–D5 after
the independent review of committed L1 `62faff63`. The explicit pre-gate
`git log --oneline -5` still showed62faff63; external20-2 renderer20 remains
attributed to `d84b83dd`, not this pass. No commit, push, tracker status flip,
kernel source change or AC15 cargo handoff is claimed by this agent.

**Implemented correction cutover.**

- The operator door carries an explicitly resolved installed signed package
  through both snapshot guards. Shared verification compares actual manifest,
  artifact.bin and declared component bytes; temporary neighbors supply no identity.
- Durable `installed-pins.json` survives uninstall, preserves every legacy signed
  generation's publisher/version digest before removal, and has a real standalone
  local `release-pin` verb. Existing installed generations remain immutable even
  if that ledger entry is explicitly released. No private-data migration is added.
- Server publication is immutable/idempotent and initial files activate by one
  staged-directory rename. An identical package/origin may atomically renew newly
  verified promotion evidence without uninstall. Uninstall holds the cross-process
  mutation flock through staging/proof/accept or rollback.
- Present install yanks are authenticated against the package publisher/enrolled
  vetter before a typed Yanked refusal and before persistence. Omission/rolled-back
  head detection remains AC15, not a property inferred from checking present rows.
- Withdrawals use required v2 role/URI/nonce/max-age preimages, bounded clock skew,
  nonce-rebinding refusal and per-(name,version,role,signer-key) deduplication.
  The role component preserves a publisher withdrawal when the same operator-
  enrolled key previously acted as vetter; re-signing in one role remains idempotent.
  Accepted historical withdrawals do not expire with mutation credentials.
- Schema-2 cursors bind URI, persist each verified frontier and resync on unsigned
  gaps/overshoots. Unknown uninstalled publishers cannot acknowledge a frontier.
  Rejected rows are journaled separately from transient faults; one invalid row,
  local poll failure or corrupt Spirit does not starve later valid observations.
- Running remote yanks retain the RegistryYank terminal observation. Pending
  effects retry without re-journaling that observation, and no observation mutex
  is held across the kernel applier. Quarantine is still only a typed request and
  operator-visible record: its effect is **not claimed in L1**, final KQ1 only.
- Real production admission binds publisher-separated principal spill. The new
  forced K1→K2 witness uses two real processes and recycled scheduler PID, not a
  hand binding. One shared formatter also supplies uninstall's bare+bound owner
  and subject queries; the signed partial proof reports its nonzero private_before.
  P4 caller audit: topology admission now binds its original child path;
  `gate_staged` supplies provider declarations, not a memory-identity sink;
  enterprise control bootstrap uses the fixed first-party control PID and no
  MemoryManager identity. The positive D1 door test uses an installed signed
  native fixture through the real door, not remote WASM execution. Registry
  component execution is still export-held; no general cold-swap snapshot
  publisher-identity/all-state isolation claim is added. Remaining identity
  inventory is explicitly deferred, not silently covered by P4's boot proof.
- No remote poller starts without a registry source. Independent local reevaluation
  remains active for late installs, with cadence30..300/default300 and zero refused.
  Watch cancellation interrupts in-flight transport without moving kernel owners
  onto its worker thread; shell, standalone and oneshot drains join owners first.
- Cheap kernel-free v2 attestation/key-event role digests are independently
  vectored. Claim CBOR stays byte-pinned; old unscoped signatures need reissuance.
  Kernel CRL/attestation-revocation role migration remains a separately priced
  obligation; no kernel signature protocol is changed.
- Enrollment/revoke-key is explicitly fixture-keyring based, not a shipped verb.
  D5 and all review deferrals have same-line rule-10 dispositions in deferred-work;
  live-keyring rollback, startup snapshots, URI aliases and other bare-name state
  are not silently claimed repaired.

**Boot regression diagnosis.** Hosted42336032162 reported installed dispatch26.9s
instead of1.3s. A provisioned timestamp profile observed boot5.276212765s: the first
poll completed in9.4ms, but the audit writer waited5.0012s because the shell's early
drain still had poller-owned applier/capability senders. It was not DNS or the cadence
floor. The fixed no-source boot oracle keeps interval300 (not0), asserts <2s and
registry-fetch=disabled; the affected two-test binary observed2.69s total.
Removing the actual shell join reds at5.494s; blocked-network cancellation has an
independent3s before/plant and <2s shutdown oracle. Temporary timing code is removed.

**Evidence quality.** `review-corrections/plants.md` names every before/plant,
current name:line and applicability exception. Before runs used the evolving
correction tree, not a clean-commit reconstruction. P4/P5/P6 use triage-prescribed
mutation-only before controls; P15's runtime before was expressly waived; P16/P18
are documentary. The incorrect initial D1 CLI flags, stale profiling fixture,
two policy-harness assertion errors, missing Python backend, compilation typo and
two passing/incomplete plant probes remain recorded, not counted as behavioral reds.
Each actual plant has executed copy-back, cmp and touch exit0.

**Exact formatted source prices (canonical tokei14 exclusions/grouping).**

| Surface | Reviewed L1 pin | Correction delta | Exact correction pin | L1 ceiling |
|---|---:|---:|---:|---:|
| maos-bin | 35422 | +204 | 35626 | 35837 |
| maos-registry | 4627 | +444 | 5071 | 5207 |
| maos-spirit-cli | 936 | +3 | 939 | 1895 |
| maos-compliance | 2070 | +7 | 2077 | 2122 |
| maos-audit | 8174 | +5 | 8179 | 8204, additional Main P3 grant |
| maos-domain | 10137 | 0 | 10137 | 10167 |
| xtask | 44696 | 0 | 44696 | external20-2 ceiling44920 |
| aggregate | 195738 | +663 | 196401; strict hardfail196402 | 197774 |

CLI measured6429 and its unrelated reserved row remain unchanged. Kernel tokei
stays21042; existing physical26841/hash95-file pin is not enlarged. Exact coupled
rows and provenance are in kloc.toml; this pass does not borrow later AC15 capacity.
Audit's separate20-11 ceiling8261 remains unchanged: after this measured+5 its
absolute remaining room is82, not87. Spending the full older87 as well would8266,
requiring a later honest re-base; the unused P3 correction room is not a reservation.

**AC15 handoff price warning.** A/A/A is ruled, not implemented. Old bounds were
computed before these corrections. Re-applying only the originally priced ×3
increments2010/120/45/45 to current exact pins would require registry7081,
cli6549, bin35671, spirit-cli984 and aggregate198621, not the old absolute
6637/6549/35467/981/197958. These are **priced options, not a granted re-base**;
Main must adjudicate the new pass at explicit `CARGO HANDOFF AC15`. No TLS/root,
interception, hosted-only cgroup/T3 or final quarantine-effect proof is invented.

**FKCS review focus.** Independent review `review/triage.md` §0 ratified the
original argument hunk/fingerprint136c579d…, and P18 records that review.
Neither declared admission file changes in this correction pass; no new hash
re-pin is needed. The fingerprint proves declared-file content, **not unchanged
semantics**: artifact/publisher checks and the new v2 role preimages live outside
it. The correction round must re-examine that semantic boundary and explicit
upgrade evidence; no independent re-review of these corrections is claimed.

#### Correction AC witness matrix and observed verification

Aliases below are exact test files: R=`crates/maos-registry/tests/yank_auth_20_1b.rs`,
B=`crates/maos-bin/tests/yank_policy_20_1b.rs`,
I=`crates/maos-cli/tests/spirit_install_20_1.rs`,
D=`crates/maos-bin/tests/one_daemon_one_door_16_1.rs`,
E=`crates/maos-bin/tests/erasure_inventory_20_11.rs`.
The complete patch/decision table is `review-corrections/plants.md`; the fresh
JUnit-derived66-case selection is `review-corrections/workspace-summary.json`.
These are the **historical correction-handoff locations atc656a6e7**, not new
AC15 coordinates; fixture migration may shift them. The AC15 record supersedes
the former execution hold and supplies its own current witness locations.

| AC | Correction-handoff witness:line | Before / plant evidence | Observed corrected result |
|---|---|---|---|
| AC6 | `vetting_attestation_gate.rs` artifact_swap_is_refused_ac6:411 / publisher_swap_is_refused_ac6:415; `vet_lifecycle_20_1b.rs` vet_issue_install_promote_revoke_refuses_next_admission_ac6:8; R attestation/key-event role refusals:1268/1308 | Original partial-tree artifact plant; new unscoped-signature before101 and signature-domain plants100 | Gate9/9, real lifecycle and both independent role controls pass in66 |
| AC7 | R publisher_and_enrolled_vetter_yanks_are_signed_labelled_and_restart_safe_ac7:98 / daemon_cursor_restart_does_not_replay_journalled_yanks_ac7:190; `yank_cadence_20_1b.rs` signed_yank_crosses_real_server_at_30_second_floor_ac7:42 | Original unsigned-auth plant; new cursor/starvation/audience/nonce controls red/restored; no fabricated original restart before | Signed origins/restarts pass; genuine server/production cadence30.177s |
| AC8 | B signed_yank_decisions_warn_request_quarantine_and_apply_crl_ac8:207 | Original policy-outcome plant; restored remote-observation before101 / plant100 | Warn, typed quarantine request/record, RegistryYank observation and existing applier pass; quarantine EFFECT not claimed |
| AC9 | B expiry_and_vetter_revocation_observe_once_and_request_policy_ac9:296 / poller_runtime_clock_and_cursor_restart_reach_expiry_p6:731 / no_registry_local_timer_discovers_promotion_installed_after_start_p11:766 | Direct-runtime originals honestly labelled; real runtime-link mutation100; retry-dedupe before101 / plant100 | Both cause labels, actual poller/clock/restart and independent late-install timer pass; key revocation is fixture-keyring based |
| AC10 | R first_publisher_pin_is_typed_and_survives_restart_ac10:77 / installed_publisher_and_version_pins_survive_uninstall_d2:438; B registry_publishers_cannot_share_principal_spill_ac10:378; E registry_bound_principal_is_counted_and_erased_on_uninstall_p3:892 | Durable pin/erasure real before101; production bare-binding mutation100 discloses K1's secret; restored copy/cmp/touch0 | Pin/restart/uninstall/legacy cases and real recycled-PID production admission/erasure pass |
| AC14 | I installed_version_is_immutable_ac14:307; R durable pins:438 / legacy_inactive_version_pins_are_migrated_before_uninstall_d2:1177 | Original partial-tree replacement red; new same-key other-registry identical-byte origin mutation101, plus durable/legacy plants | All four replacement/origin branches preserve bytes/origin/inventory; uninstall does not reset publisher/version pins |
| AC15 | D-20-1b-01/02/03 A/A/A | No implementation/plant/security proof invented | Held for explicit Main cargo handoff and priced re-base, not blocked on design choice |
| AC16 | I yanked_install_is_refused_before_persistence_ac16:404; R install_refuses_present_yank_with_unpinned_authority_f2:817 | Original yank-refusal plant; forged-authority before101 / plant100 | Genuine signed yank refuses before persistence; forged row refuses authentication; omission/head dependency remains AC15 |
| AC17 | R unsigned_and_unauthorized_mutations_leave_storage_unchanged_ac17:12 / published_versions_are_immutable_and_identical_publish_is_idempotent_d3:404 | Original publish/deprecate auth plants unchanged; immutable publication before101 / storage plant100 | Unsigned/wrong-owner/unenrolled mutations leave storage unchanged; authorized immutable/idempotent publication passes |
| AC11–AC13 | Final KQ1/P kernel landing | No kernel source edit or effect plant here | Not claimed in L1 |

**Full workspace, once:** metadata reused after the canonical fixture builder and
genuine bin/CLI build. Cargo compilation hit a3000s tool deadline before any tests;
continuation completed the same prerequisite, not a runtime retry. Actual run
`3015a808-1010-47d2-a1f7-5dabde864f5b` executed5753 cases: **5752 passed,1 failed,
138 skipped**, exit100; wall881.646s / sum880.766s. Sole failure is the unchanged
20-7-owned1M pending oracle36.007s (route10.000111352s HandlerStillRunning), with
same-line rule-10 evidence in deferred-work. No retry or assertion/deadline change.
All66 selected L1 witnesses passed in that run; this is **not** a fully green suite.

**Boot costs from that fresh JUnit:** installed dispatch2.517s total: four hostile
boots2.012s and the <2s no-source boot oracle0.505s. This is local evidence, not a
claim that the hosted runner will have identical costs.

**Calibration ruling:** Main selected the hosted CI maximum, not local speed:
ceil(1151.8s from hosted42336032162 ×1.25)=**1440**. The comment also records local
post-fix880.766s; the next green hosted sum must be recorded and the total re-tightened
by20-7's rule-10-owned review row. No per-binary exception changed. The local budget
gate still exits1 solely for unchanged `maos-iac::participant_recall_17_8`
sum30.730s vs30s; separately recorded, not widened or retried.

**Other observed checks:** no-default verb table7/7; FKCS oracle11/11,
D11 count contract9/9 and recovery-lane12/12; compliance vetting35/35; final
FR59 fixture cleanup4/4. Workspace clippy and count ratchet exit0
(1319 occurrences/865 distinct); the last test-only unused clock was then removed
and its affected clippy/test targets exit0. No warning-free workspace claim.
Kernel baseline0 confirms26841 physical,95 unchanged files/hash
`db39bc4a1ab6a6c22575990ae934e698a519d3efd46a8dbb38e62bc376697b59`;
KLOC0 confirms196401 and exact coupled pins. FKCS surface/admission gate0,
environment, decision register, test-cost, exit commands, coverage/completeness,
dev-record and epic-coherence gates0. One initial ownerless-disposition wording
violation was corrected to cite its actual Main disposition; the red is retained.

**Prescribed prepush, exactly once:** exit1 after1536.080s; no tracked edits while
it ran. Its92 records are82 pass,8 deliberate scope/dynamic-marker skips,
1 workspace red and1 timing advisory. Fresh workspace JUnit
`1cab18a7-0a75-4d94-a47b-aede06a29283` contains5753 executed cases,
**5750 passed /3 failed**; all66 L1 selected witnesses pass again.
The three failures are the known20-7-owned1M pending oracle45.123s
(route10.005223439s), **plus two owned calibration-test pin omissions**:
`check_test_budget_proven_red::{the_committed_register_parses,
hosted_1_model_currency_calibration_changes_only_its_binary_budget}` still expected
1100 after the ruled1440 change. Both hardcoded expectations are now coupled to1440;
the entire focused budget-contract binary passes14/14 afterward. **Prepush was
not rerun, and is not represented as green.**

Its measured suite sum1186.373s is below1440; five per-binary timing advisories
remain (butler hallucination30.229, erasure_uninstall34.447, hook_pump32.110,
one_daemon_one_door37.776, participant_recall44.916 seconds against30).
No per-binary exception or assertion/deadline was changed; same-line20-7/rule-10
rows name both pending-oracle and timing evidence. The Door sum includes D1's
additional witness, not an untouched-binary claim. Scope-skipped docs build/routes,
unchanged-lock cargo-deny and indeterminate dynamic WASM marker counts are not
claimed exercised. Hosted first-contact/TLS/interception and final KQ1 effects
remain unverified/unimplemented as already labelled.

**End-of-work cleanup:** `/proc` scan27 entries (2 vanished/denied), no active
cargo/rustc/rustdoc/clippy process or referenced incremental session. Removed665
stale `debug/incremental/*/s-*` directories, retaining9630 newest-per-crate and1072
additional sessions touched within2h; no deps/build/fingerprint removal.
Unlinked incremental allocated footprint5,191,536,640 bytes; observed filesystem
available-space gain4,506,730,496 bytes (not conflated with shared/reflink physical
reclamation). Removed the two owned `/tmp/201b-review-corrections` and
`/tmp/201b-review-verifier` roots, allocated footprint32,350,208 bytes.
Detailed paths/process scan/measurements: `review-corrections/cleanup.json`.

### AC15/F5 kernel-free implementation Dev Agent Record — handoff `c656a6e7`

**Authority and scope:** Main's explicit `CARGO HANDOFF AC15` fulfils the prior
hold. D-20-1b-01/02/03 remain A/A/A. The **orchestrator-adjudicated rebase,
D02 A, disclosed to the operator** applies the same granted increments2010/120/45/45
(aggregate2220) to corrected pins5071/6429/35626/939/196401:
ceilings registry7081, CLI6549, bin35671, spirit-cli984 and aggregate198621.
No original L1 correction headroom is renewed. AC8 quarantine EFFECT and
AC11–AC13 remain final-KQ1 work, not claimed in this pass; no kernel source change.

**Implemented path:** all operator network entrypoints share the blocking registry
leaf. `[registry].uri` and `signing_pubkey` in the operator TOML bind an out-of-band
Ed25519 public key to the canonical `/mcp` URI; no response key becomes a root.
HTTPS requires this configuration before contact. Rustls uses explicit ring
providers, WebPKI roots plus optional operator `ca_certificate`, certificate
hostname/time validation and no redirect/failure downgrade. Server flags require
operator-supplied certificate/key paths, canonical HTTPS URI and signing seed;
Unix private-key checks use opened-file metadata, and the production argument
builder refuses private keys physically inside the package/inventory store.
No production key or certificate is silently generated.

**Omission closure:** v1 signed envelopes bind fresh256-bit nonce, URI and exact
request/response SHA-256 plus the complete durable yank head. Head and filtered
entries come from one snapshot, avoiding an append race between two reads.
Authenticated lists must be contiguous through that head; missing signatures,
missing/incomplete heads, altered bodies, URI/nonce rebinding and stripped yanks
refuse before local persistence. Present-entry publisher/vetter authorization
still applies. Request inspection borrows routing fields and ignores publication
payload fields rather than allocating a second artifact-sized JSON Value.

**Development exception:** server `--dev-http` and client operator configuration
`development_http = true` are both explicit; only literal loopback is permitted,
with a wire handshake. Mutation signatures and durable publisher/version/origin
pins remain enforced. Unsigned HTTP development still has no peer/confidentiality/
omission/freshness guarantee. A configured URI-bound root authenticates its
responses, not its plaintext confidentiality. A configured authority lying,
compromised signing keys, and authority-signed rollback to a fresh client are not
claimed solved. No live-vetter-keyring rollback or operator-key lifecycle closure
is borrowed from this grant.

**Current AC15 witness locations** (`T` =
`crates/maos-registry/tests/registry_tls_20_1b.rs`):

| Contract | Named witness:line | Before/plant and observed focused result |
|---|---|---|
| Real TLS install and metadata | T `cli_tls_install_authenticates_before_persistence_ac15`:364; `configured_root_and_ca_authenticate_real_registry_ac15`:209 | Pre-cutover CLI TLS-positive101; actual authenticated publish/fetch/yank positive0 |
| Wrong configured key/URI | T `hostile_registry_wrong_configured_root_is_refused_ac15`:257; CLI root:395 /URI:413 | Root-envelope plant101; configured URI/bootstrap plant100; focused refusals pass |
| Hostname/expired/untrusted CA | T `interception_wrong_host_certificate_is_refused_ac15`:294 /expired:304 /untrusted:314 | Real accept-any TLS-verifier plant101 for all3; focused refusals pass |
| Envelope binding | T missing signature:743 /wrong URI:747 /wrong nonce:751 | All independent envelope-disabled cases101; focused pass |
| Signed complete head/stripping | T missing head:755 /incomplete head:759 /stripped entry:763; CLI omission:858 | Envelope plant101, independent head-only plant100; rebuilt CLI otherwise installs stripped-yank package; focused refusal before any registry directory |
| No plaintext downgrade | T redirect:767 /handshake failure:894 | Real plaintext contact plants100/101; focused traps observe no connection |
| Both-end dev opt-in | T client:12 /server:24 /wire handshake:802 /positive and unsigned-yank refusal:787 | Actual baseline101 and mode/handshake plants100; focused pass |
| Secret custody | T private-key permission:126 /seed outside store:138 | Actual before101 and custody plant101; focused pass; no foreign-owner negative runtime claim |
| Genuine TLS yank | T `cli_tls_yanked_version_is_refused_before_persistence_ac15`:817 | Focused genuine signed refusal; present-row and omission controls remain distinct |
| Canonical default ports | `server.rs` `development_default_ports_use_the_same_canonical_uri_as_clients`:771 | Faithful raw-address formatting plant101; final IPv4/IPv6 unit0, charged in KLOC |

Complete source names, actual before limitations and restored copy/cmp/touch
custody are in `landing-1/ac15/plants.md`; exact commands and outcomes are retained
in that directory's `commands.json`. The first multi-control command was fail-fast;
unrun controls were separately executed, not counted silently. The initial
read-side handshake fallback plant passed because failure occurs during the first
write; only the faithful first-write red is credited. The two pre-cutover negative
CLI probes were authentication-phase failures, not a claim old HTTPS accepted
bad keys/certificates. The final CLI root/URI cases use real TLS.

**Exact measurement after formatting:** registry5071 ->5668 (+597), bin35626
->35589 (−37), spirit-cli939 ->942 (+3), CLI6429 unchanged; aggregate196401
->196964 (+563), strict hardfail196965. The inline default-port regression is
charged, not treated as an excluded tests-directory addition. CLI's legacy
reserved budget6856 is downward re-pinned to exact6429, not counted as source
shrinkage. Compliance2077/audit8179/domain10137/xtask44696 remain unchanged.
Audit ceiling8204 and20-11's separate8261/remaining82 are unchanged; no new draw.
Kernel remains26841 physical/21042 tokei; external20-2 xtask driver attribution
remains d84b83dd, not own growth. Exact count histories and aggregate pin move
together; old-count tests are dynamically coupled, with no new hardcoded count.

**Observed focused verification:** TLS26/26 (3.329s), actual TLS publication
positive0, final default-port unit0; affected CLI48/48 +install9/9 and SpiritCLI13/13;
registry security/server/auth binaries pass, including31 auth cases. Final
no-default production check0 and verb table7/7. The earlier “normal dependency
tree has no rustls edge” interpretation was **wrong**: the saved tree contains it.
The AC15 review correction below isolates registry feature edges and discloses the
remaining frozen kernel/A2A links. `cargo deny check`0; affected clippy0 with zero new warning fingerprints
against the preceding final workspace evidence, plus final registry clippy0.
No warning-free-workspace claim. The pre-pin measurement red is retained, not
misrepresented as a gate pass.

**Final structural and once-only standalone workspace:** all14 focused structural
gates0, including unchanged-kernel/FKCS admission negative controls; final cargo
deny0; bin all-targets clippy0 with zero new fingerprints. Genuine16-fixture
builder and `cargo build -p maos-bin -p maos-cli` both0 preceded metadata reuse.
The unfiltered standalone nextest run exited100:5783 run,5773 passed,10 failed,
138 filter-skipped, JUnit UUID`64ad0f22-ddd8-4997-a50e-ff7f383a017f`; sum807.751s.
All107 selected landing/cohort harness results are present and green, with the
live-Worker local prerequisites still explicitly skipped, not exercised passes.
Root-shutdown sum0.699s and installed-dispatch sum2.760s; budget gate0 with no findings.
One unchanged1M pending oracle failed36.270s (20-7 rule10 row); nine journeys
refused conservatively stale `maosctl`, **not a flake**. Cargo had reused unchanged
CLI despite the required genuine build, leaving its exe older than bin/main.rs;
the journey guard checks both source trees against each executable. Source-input
touch plus genuine recompilation of both packages repairs preparation before the
separately required once-only prepush; no standalone retry or artifact timestamp
touch. `workspace-summary.json` preserves exact failures/mtime evidence.
An earlier all-targets metadata-list preflight encountered non-libtest Criterion
benchmark output before any workspace tests; canonical test metadata omits
`--all-targets`, matching `xtask/src/prepush.rs:391`, without narrowing workspace tests.

**Once-only prescribed prepush:** exit1 in1832.115s,92 records:83 pass,
7 explicit skips,1 red workspace leg and1 timing advisory. Fresh JUnit
UUID`9554b0c5-9e63-477c-8745-7622029439e4`:5783 run,5782 passed, only the unchanged
1M pending oracle failed (43.305s). Sum1102.543s is under1440; per-binary timing
advisories are erasure-uninstall30.614, hook-pump30.093, operator-door31.998,
IAC participant-recall40.829 and unchanged xtask-bin31.167 (each default30s).
All107 selected contracts remain green harness results; the original live-Worker
bodies remain local skips. Root-shutdown sum1.237s, installed-dispatch3.952s;
no root-shutdown exception. All nine stale journey cases now have green results
after the genuine recompilation; no harness weakening or standalone retry.
The copied-JUnit budget-detail command only analyzes existing timing data,
not another test run. Both failure families/advisories have explicit rule10
dispositions; no retry-to-green or per-binary widening. `prepush-summary.json`
retains observations; final post-gate edits are evidence/story/deferred notes only.

**Independent-review focus:** configured root/URI and every operator network
entrypoint; TLS hostname/time/CA and downgrade refusal; complete-head snapshot
and nonce/request/body binding; key custody and explicit dev gap; cohort owner
ordering/AST coverage. Re-examine the inherited authorized FKCS
`verify_attestation` input hunk and two test claim fields at pin`136c579d…`;
its prior re-pin preceded bmad-code-review by Main's ruling, and this AC15 pass
does not alter those declared files or claim that review completed.

**Hosted/portability:** the branch-only `ci/ci-20-1b-verify.yml` selects107
actual named L1+AC15/cohort contracts, including generated-CA interception and dev
flows, provisions the real signed T3 action, and authors an Apple SDK cross-check.
The recipe is not installed under `.github/workflows/` on main and has not run hosted.
Local `xcrun`/SDKROOT are absent;
no unsupported Darwin C cross-build was attempted. macOS and hosted legs remain
unverified, not waived as passes. Independent AC15 security review remains Main's
next review, not a claimed completed exercise.

**Hosted correction 42418118608 (20-1b-owned production regression):** the local
vetting timer owned scheduler/revocation/TL audit senders until its task was
joined. The cohort arm alone awaited the writer while retaining that owner.
`main.rs:9297` now joins it before releasing `audit_tx`; normal300s cadence and
registry-fetch-disabled behavior remain intact. No interval-zero harness bypass,
timeout suppression, retry-to-green or per-binary exception was introduced.

| Cohort correction witness | Before / restored plant | Final focused observation |
|---|---|---|
| `root_shutdown_unload_16_3.rs::cohort::cohort_sigterm_without_worker_joins_local_yank_timer_and_drains`:2280 | Actual no-T3 daemon101 with10s audit timeout; removed-join plant101 with same timeout | Restored complete binary15/15,0.64s harness sum; live-Worker cases locally skipped |
| `root_shutdown_unload_16_3.rs::every_audit_sender_release_joins_the_yank_owner`:67 | Before101 and removed-join plant101 | Current AST guard0; seven release sites and eleven drains, lexical branch isolation |
| `root_shutdown_unload_16_3.rs::cohort::declared_capable_cohort_worker_prerequisite_never_green_skips`:2314 | Child deliberately lacks runtime with CI declared and must fail explicitly | Parent0; existing shared T3 helper already fails loud, no shared-helper change |
| Existing real live-Worker v5:2344 | Prescribed original command0/three cases0.00s; explicit nocapture records missing signed-substrate SKIP, **not a local red** | Hosted-only correction unverified here; branch recipe provisions T3 and requires the case |

The preceding local workspace/prepush did not catch the v5 regression because
the OCI substrate is unavailable and those cases explicitly return on their
local-only prerequisite guard. The new real no-Worker daemon case has no such
gate. On a declared-capable Linux runner, absent/invalid runtime and signed anchor
are hard failures; the new child-process control proves the missing-runtime leg.
The existing discipline profile-count guard is updated from12 to15 (release10,
debug cohort5), not weakened. Copy-back/cmp/touch all0; complete command logs and
the exhaustive teardown/early-return/panic classification are retained under
`landing-1/ac15/`. The root-shutdown budget row stays unchanged.

**Cleanup and custody:** `/proc` scan found no visible compiler processes (PID1/7
exe links were permission-denied, recorded rather than hidden). Removed209 stale
incremental session directories, retaining newest per crate cache/feature and
all descendants touched within2h; never deps/build/fingerprint or lockfiles.
Deleted own `/tmp/201b-ac15` and verifier venv. Observed build-filesystem available
bytes increased2,416,631,808; fully unlinked inode accounting3,350,249,472 allocated
bytes and4,115,228,024 logical bytes. Filesystem-delta attribution limits are in
`cleanup.json`. Prior `/tmp/201b-wip` was absent, not deleted again.
Exact35 owned non-AC15-evidence paths are in `source-inventory.json`, with the
new evidence directory separately named. Main's two DiskClean reports remain
intact and unattributed. No commit/push/tracker status change is made here.

**Owned file groups:** registry transport/security/client/server/handlers/storage
and their tests; bin manifest and registry composition; SpiritCLI manifest,
producer/yank wiring and URI help; shared registry harness; nextest serial group;
registry security policy, ADR056 and coverage notes; exact KLOC rows; story/tracker/
deferred dispositions, AC15 evidence and the branch-only recipe; root-shutdown
regression controls and the existing discipline profile-count guard. CLI source
and kernel/FKCS declared admission files are intentionally unchanged.

### AC15 independent-review corrections — baseline `9e8c7737` (2026-11-08)

Dev201b / openai-codex/gpt-6.1-sol. This record supersedes the preceding
AC15 implementation's correctness/coverage claims where the independent review
found a gap; it does not rewrite its historical before/plant observations.
Authoritative review: `landing-1/ac15/review/triage.md` and `security.md`.
Current exact commands, exits, restored plants and acceptance-name:line table:
`landing-1/ac15/review-corrections/{commands.json,witnesses.md}`.

**Implemented P1–P12 / security corrections:** actual discipline/verification
invocations and the executable TypeScript example explicitly opt in on both
development endpoints. Accepted sockets are reset to blocking; per-connection
clone failures no longer kill the listener and accept failures back off. Bounded
global/per-source admission, TLS-aware refusal workers, a tighter initial
handshake/first-byte bound and received-byte body growth replace eager allocation
and unbounded idle occupancy. Metadata is64KiB, publication64MiB; parsed routing
rechecks the upload hint. The discovered development overflow framing regression
has its own real red/plant and a bounded100ms dev-only refusal/drain fix.
HTTP uses a total30s deadline, bounded DNS workers and bounded response bodies;
unsigned non-2xx remains a refusal status, while success still authenticates.

One strict operator-config reader drives behavior and the banner, validates
registry keys even without a URI and starts the poller only after installed boot
admission. Standalone/cohort joins now precede supervisor stop and unload.
Ownership-aware AST controls cover qualified/parenthesized/select drops and
drains, helper/task moves, sibling scopes and exact named7-release/11-drain
coverage. The declared-capability negative control requires one actual child
test; it cannot green-pass a zero-test selector. Signed-valid malformed views
exercise head/sequence/request/signature guards independently of body hashes.
Configured CA replaces bundled roots; missing CA uses Mozilla, not platform
roots. Key hex errors are sanitized, custody vectors include0440 and store
placement, and interception witnesses now exercise the real CLI before persistence.
Queries use their lower bound, preserve the snapshot head and refuse a delta
over4096 rather than sign an incomplete successful response.

**Rulings/limits:** D-20-1b-09..13 record D1..D5. Typed `HeadBehindCursor`
does not mutate or reset the cursor; optional `YankList.head` consumes7 of Main's
18-line domain correction grant. Registry-specific TLS feature leaks are removed,
but frozen kernel/A2A rustls remains in the no-default graph under Main's explicit
honest-record fallback; the preceding whole-graph TLS-free claim is withdrawn.
CLI remains exactly6429 and forfeits427 of its legacy reserve. W1–W7 and remaining
F6/F8/F10/F11 have explicit owned dispositions in deferred-work.md, not false
completion claims. AC8 quarantine EFFECT and AC11–AC13 remain final KQ1 + P;
no kernel source/manifest or declared FKCS admission file is edited here.
The inherited `verify_attestation` input hunk/two claim fields at136c579d… remains
an explicit re-review focus; this pass does not re-pin or assert semantic change.

| Exact tokei pin | At9e8c7737 | Correction | Final | Ruled ceiling |
|---|---:|---:|---:|---:|
| maos-registry | 5668 | +483 | 6151 | 7081 |
| maos-bin | 35589 | -3 | 35586 | 35671 |
| maos-domain | 10137 | +7 | 10144 | 10155 |
| maos-cli | 6429 | 0 | 6429 | 6549; legacy427 forfeited |
| maos-spirit-cli | 942 | 0 | 942 | 984 |
| Aggregate | 196964 | +487 | 197451 | 198621 |

After fmt, coupled live story-arrow histories and strict hardfail197452 are
re-pinned exactly; `correction-kloc-exact` exits0. Kernel remains26841 physical /
21042 tokei,95 files/set hash unchanged. Audit8179 stays below correction8204;
20-11's separate8261 ceiling has82 remaining, not the old87. Xtask44696 is unchanged
in this pass, including external20-2 hosted-3 d84b83dd's20-line renderer draw.

**Observed focused/affected checks:**827/827 focused cases pass, UUID
`a796a2f7-be25-4368-a940-7c4105bd0c3e`,101.271s wall/101.261s testcase sum.
All current plants were restored by copy-back+cmp+touch, each0. Actual no-default
build0, verb contract7/7, cargo-deny0, affected default binaries0; fixture builder
preceded a genuine default binary build and one reused metadata generation.
Final affected all-target clippy0, no primary diagnostic intersects changed/new
source spans and no new message fingerprint versus the recorded initial
correction-state clippy; that comparison is not a fabricated clean-baseline run.
Twelve final structural gates plus exact KLOC pass. The separate check-fkcs
attempt reaches the agent's900s command deadline with empty captured output and
no process exit result; it is not credited as a surface-gate pass. A real process
scan afterwards finds no remaining cargo/rustc/rustdoc/xtask child.

**One standalone workspace, not retried:**5836 run/5833 pass/3 fail, exit100,
UUID`f6748ad2-1296-4b1c-a5ee-e387563850c1`,1144.817s testcase sum (<1440).
Two20-11 verifier cases fail because the agent omitted the pinned cryptography
environment from that run; this is preparation error, not product/timing-flake
attribution. The1M pending oracle separately fails43.366s at its unchanged bound.
Six30s binary-sum advisories have same-line rule10 owners20-7; no exception,
timeout/assertion or per-binary budget is widened. Hash-locked verifier wheels
are provisioned for the separately prescribed prepush; they do not retire the
standalone red. Full same-run JUnit/summary/budget output are retained.

**Hosted proof boundary:** the branch-only recipe resolves156 nonignored
contracts and54 required markers from actual metadata, plus one separately
enrolled ignored public-WebPKI exclusion control. It authors native Apple runtime,
the actual signed T3 substrate, no-default/verb and actual epic20-exit jobs.
All these hosted/native-capability legs remain unverified, not inferred from
Linux source checks or locally returning T3 prerequisites. Main's reported
42464850336 before-state is accepted without rerunning it to confirm failure.

**Once-only prescribed prepush:** exit1 in3518.903s, split into bounded wait
cells without stopping/restarting the command or writing tracked files while it
ran. Its92 records are83 pass,7 explicit skips,1 red workspace leg and1 timing
advisory; tracked-tree preservation passes. Fresh JUnit
UUID`0e4b479b-e575-4500-8c98-3865f2ac3d5f` has5836 run/5835 pass and only the
existing1M pending case fails45.523s. The two verifier cases pass with the
hash-locked environment; the earlier standalone preparation errors remain
recorded. Sum1136.340 stays below1440; four unchanged30s binary-sum advisories
have same-line20-7 rule10 dispositions. No second prepush or per-binary widening.
Root-shutdown15 cases sum1.504s, installed-dispatch2 sum4.440s, TLS40 sum5.832s,
limits4 sum1.276s and real cadence30.141s (<39s exception). T3 body prerequisites
still limit live-Worker proof despite green harness counts. The post-run budget
command only analyzes the copied same-run JUnit, not another test execution.
Exact command/report/JUnit/budget observations are retained in the correction
evidence directory; only documentation/evidence notes change after this run.

**Cleanup:** guarded `/proc`/fd/maps scan finds no visible compiler/xtask process;
PID1/7 custody links are permission-denied and recorded, not asserted inspected.
The script removes1529 old inactive `s-*` sessions, retaining11105 sessions,
newest per crate-cache/feature, every descendant touched within2h and any open
reference. It removes own `/tmp/201b-ac15-corrections`, including its verifier
venv and restored-plant scratch; never deps/build/fingerprint or lockfiles.
Observed build-filesystem available space increases13,830,430,720 bytes.
Fully-unlinked inode allocation accounting is15,100,903,424 bytes; unique removed
inode logical size17,967,899,575. The scratch filesystem's observed delta is
-573,440 bytes, not claimed as freed. Reflink/shared activity attribution limits,
exact candidates and the reproducible script are in
`review-corrections/{cleanup.json,cleanup-plan.json,cleanup-script.py}`.
Main-owned cleanup reports remain intact and unattributed.

**Current handoff boundary:** production corrections and local scoped witnesses
are complete, with no kernel landing or hosted pass claimed. The incomplete
check-fkcs command result, stable-runner1M/budget occurrences and hosted/native
legs remain explicitly open evidence limits, not green substitutes or waived
assertions. The inherited admission hunk remains a re-review focus. Final owned
paths/hashes and consolidated evidence are in `source-inventory.json` and
`final-report.md` under this correction directory. No commit/push/status flip.

### File List

Historical original-L1 inventories: initial42 paths in `landing-tracked-file-list.stdout`; final43 in `landing-tracked-file-list-final.stdout`. These are not the correction-pass inventory. Original grouped implementation inventory:

- `Cargo.lock`; `.config/nextest.toml`; `xtask/{kloc.toml,test-budgets.toml,fkcs-baseline.toml}`.
- `crates/maos-bin/src/{admission.rs,env_contract.rs,installed_spirits.rs,lib.rs,main.rs,yank_policy.rs}`; `crates/maos-bin/tests/{cohort_daemon_smoke_13_5c.rs,yank_policy_20_1b.rs}`.
- `crates/maos-domain/src/{ports/registry.rs,revocation.rs}`; `crates/maos-compliance/src/vetting/{attestation.rs,mod.rs}`.
- `crates/maos-registry/{Cargo.toml,SECURITY.md}`; `src/{admission.rs,client.rs,handlers/deprecate.rs,handlers/publish.rs,install.rs,lib.rs,operations.rs,server.rs,storage.rs,yank.rs,yank_auth.rs}`.
- `crates/maos-registry/tests/{end_to_end_test.rs,form_admission_17_3b.rs,fr59_yank_propagation_within_5min_test.rs,registry_security_20_1a.rs,registry_server_bin_20_1.rs,vetting_attestation_gate.rs,yank_auth_20_1b.rs,yank_cadence_20_1b.rs}`.
- `crates/maos-spirit-cli/{Cargo.toml,src/bin/maos-spirit.rs,src/lib.rs,src/vet.rs,src/yank.rs,tests/vet_lifecycle_20_1b.rs}`; `crates/maos-cli/tests/spirit_install_20_1.rs`.
- `tests/harness/{registry_20_1.rs,registry_trust_20_1b.rs}`; `tests/coverage-matrix.yaml`; `docs/adr/ADR-056-fr37-vetting-machinery.md`.
- This story; `_bmad-output/implementation-artifacts/{deferred-work.md,sprint-status.yaml}`; the landing-1 evidence tree and branch-only `ci/ci-20-1b-verify.yml`.
- The correction calibration additionally couples `xtask/tests/check_test_budget_proven_red.rs`'s two live total-budget assertions to1440; this is test-only, with no xtask production KLOC growth.

The preceding historical L1/AC15 implementation inventories exclude kernel, `.github/` and xtask production edits. The current AC15 independent-review correction **does** fix the actual `.github/workflows/discipline.yml` development invocations for P1; it installs no branch verification workflow on main. Kernel and `xtask/src/` remain untouched. External xtask renderer changes are committed20-2 drivers, not attributed here.

Correction-pass targets additionally/again edited:
- Registry Cargo/security, `src/{bin/server.rs,server.rs,handlers/deprecate.rs,install.rs,storage.rs,yank.rs,yank_auth.rs}` and `tests/{yank_auth_20_1b.rs,yank_cadence_20_1b.rs,fr59_yank_propagation_within_5min_test.rs}`.
- Bin `src/{admission.rs,env_contract.rs,main.rs,operator_door.rs,yank_policy.rs}`; `tests/{yank_policy_20_1b.rs,installed_spirit_dispatch_20_1.rs,one_daemon_one_door_16_1.rs,erasure_inventory_20_11.rs}` and the `support/{shell_world_16_2.rs,worker_revoke_root.rs}` mocks.
- `crates/maos-audit/src/lib.rs`; compliance `src/vetting/{mod.rs,attestation.rs,keyring.rs}`; `crates/maos-spirit-cli/src/yank.rs`; `crates/maos-cli/tests/spirit_install_20_1.rs`; shared `tests/harness/registry_trust_20_1b.rs`.
- Coupled KLOC/FKCS/budget records, coverage matrix, ADR-056, this story/tracker/deferred ledger, historical plant-label corrections, branch-only recipe and `landing-1/review-corrections/` evidence. Neither declared FKCS admission file is edited by this pass.

Current AC15 independent-review correction inventory (36 source/config/record paths;
exact paths/hashes in `landing-1/ac15/review-corrections/source-inventory.json`):
- Root/leaf manifests and Cargo.lock: bin, CLI, FKCS, shell; domain registry port.
- Registry `src/{client.rs,http.rs,install.rs,security.rs,server.rs,storage.rs,yank.rs,handlers/yanks_since.rs}`, SECURITY.md and four changed/new registry integration files.
- Bin main, root-shutdown integration, new registry-config/AST integration tests and `tests/support/yank_audit.rs`.
- New xtask registry socket/workflow source-contract integration tests; shared registry harness.
- Actual discipline workflow, old20-1a branch recipe and current20-1b branch recipe; TypeScript example README.
- Coupled KLOC rows; story, tracker, deferred ledger and epic20 preflight rulings.

### Change Log

- At the original L1 handoff, operator `f1b` scope and Main's kernel-last AC8 split were recorded while F5 choices remained priced PROPOSED D-rows; superseded by the dated `f5rows` entry below.
- Implemented exact vetting/real CLI lifecycle, authenticated authorized mutations, immutable installs/yank refusal, durable signed yank propagation, typed policy requests/live observations and publisher-separated principal spill identity.
- Added partially-edited-tree before assertions and restored runtime plants, real cadence budget enrollment, citations/pins and documented server/keyring/transport limits; these were not clean-commit baseline runs. No status flip or kernel landing claimed.
- 2026-11-07 (`f5rows`): recorded RULED A/A/A root/TLS/development choices and AC15
  ceilings registry6637/aggregate197958, D02 A auxiliary bounds, dependency/deny/
  no-default-feature constraints. Records only; implementation waits for the
  landing-1 review/corrections and explicit `CARGO HANDOFF AC15`; no cargo run.
- 2026-11-07 (Main L1 correction handoff at62faff63): implemented P1-P18/security F1-F5 and ruled D1-D5, including durable/releasable pins, immutable publication, safe evidence renewal, explicit vetted-door package/deployed-byte checks, production principal identity/erasure, authenticated scoped withdrawals/cursors and prompt no-source boot/network cancellation. Recorded real before/plants and applicability limits; AC15 and final KQ1 remain unimplemented here.
- 2026-11-08 (Dev201b, openai-codex/gpt-6.1-sol; Main AC15 handoff atc656a6e7): implemented ruled A/A/A URI-bound root/rustls/explicit dev mode and authenticated complete response heads; fixed hosted42418118608 cohort audit-owner regression with real no-T3 before/plant and AST/declared-capability controls. Exact aggregate196964/hardfail196965, kernel unchanged. One standalone workspace100 and one prepush1 are honestly recorded, not retried; independent/hosted review remains Main-owned. Cleanup and full acceptance/source/command evidence are under `landing-1/ac15/`.
- 2026-11-08 (Dev201b; independent AC15 review baseline9e8c7737): implemented P1–P12 and scoped security corrections/D1–D5, recorded W dispositions and correction witnesses, forfeited legacy CLI reserve, and re-pinned registry6151/bin35586/domain10144/aggregate197451 (hardfail197452). Exact kernel hash unchanged; no KQ1 effect claimed. Current detailed verification/limitations are in the correction Dev Agent Record and review-corrections evidence; no status flip or commit.
