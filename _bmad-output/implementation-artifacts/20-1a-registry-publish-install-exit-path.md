---
story_id: '20.1a'
story_key: 20-1a-registry-publish-install-exit-path
created: 2026-11-01
context_commit: a4c00897
baseline_commit: a4c00897
security_sensitive: true
kernel_grant: "none for 20-1a: kernel-free (AC1–AC5). The FLAG-Winston row R (D-20-1-02 A KQ1 + D-20-1-03 A P; measured +78 physical / +68 tokei from HEAD 26801 / 21011; ceilings kernel ≤ 26957 / 21147) is granted to 20-1b only, which lands last; the interaction with 20-7's row R is recorded in 20-1b's kernel_grant (cumulative tokei ≤ 21209, else STOP)."
kloc_grant: "RULED 2026-11-01/02 (ask rows201; D-20-1-11 A; D02 A ×2 measured / ×3 unmeasured). One-key ceilings (STOP above; aggregate hardfail = measured + 1 at landing): maos-registry ≤ 5163; maos-spirit-cli ≤ 2043; maos-cli ≤ 6666 (above its 6382 STOP; inside the reserved 6856 pin, D-20-1-05 A); maos-bin ≤ 34410; maos-shell ≤ 728; maos-compliance ≤ 2122; maos-domain ≤ 10153; xtask ≤ 45830 (20-7's ruled higher pin supersedes this story's historical 42920, operator landing instruction 2026-11-02); aggregate ≤ 193768. The kernel ceiling 26957 / 21147 is 20-1b's (see kernel_grant)."
review: "bmad-code-review under docs/testing/proof-contract.md. DIFFICULT tier (`proof-contract.md:57-71`): a kernel FLAG-Winston grant (D-20-1-02/03), signatures and supply-chain trust (AC6, AC7, AC10), cross-Spirit isolation (AC11). Blind and Edge on opus-5.5, Acceptance on opus-5.5, a Security reading on AC6/AC7/AC10/AC11, Test Infrastructure on glm-5.3 (standing policy; any session substitution recorded at review). Creation anthropic/claude-opus-5-5."
---

# Story 20.1a: Registry publish, install and exit path

Status: ready-for-dev

<!-- Split 2026-11-01/02 under operator ruling D-20-1-01 B (ask split201): Part B (AC6–AC13) and Tasks T6–T10 moved unchanged to `20-1b-vetting-yank-policy-and-quarantine.md`. 20-1 is retired; shared evidence stays in `20-1-registry-client-install-verb-vetter-and-yank-evidence/`. -->

<!-- Created 2026-11-01 at a4c00897 (= 77f7276a, 20-3c's final landing, + one docs commit; `git log -1 -- crates xtask` = a4c00897) by bmad-create-story under the lighter measured creation contract. Operator rulings in force: D01 A, D02 A, D03 A, D04 A (2026-10-30). D09, D12 (preflight §5) and D-20-1-01…11 are PROPOSED, not ruled — the operator rules; a different ruling rewrites only the clauses marked with that row. No cargo ran (parent rule): cites re-read at HEAD (`creation/cites-a4c00897.md`); kernel and kloc by the prebuilt gate binary run from `git archive` copies; exit lines 2, 4, 5, 7, 8, 9 and the daemon registry arm measured on the prebuilt HEAD binaries in an isolated home (`creation/runtime/`). Stale E20 §20-1 cites are corrected through `creation/records-owed.md` (E20 is a shared file; OLD→NEW in `creation/cites-a4c00897.md` C1–C33, X1–X10). -->

> **The capability:** *a third party signs and publishes a Spirit to a registry that runs from its own binary; an operator installs it by name with every signature, tier floor and claim verified and the publisher's key bound to the name; the installed Spirit is visible to the daemon and answers truthfully (on a published binary: the typed engine-off refusal); a vetter can promote it with an attestation bound to the exact artifact and key, and revoke that promotion; publisher and vetter yanks reach operators within one poll, signed and labelled by origin, and act by the operator's policy; and a quarantined Spirit keeps running but can take no further mediated action.*

**Closes:**
- Epic-20 §20-1 (E20@HEAD:94-107) as amended 2026-10-30 (D01 A; preflight §3.1) and D04 A (E20:102); renumbered AC1–AC13 here (Part A = exit path AC1–AC5, Part B = trust, yank and quarantine AC6–AC13).
- Exit block lines 5, 7, 8, 9 and the line-1 artifact set (E20:16-24); the `epic-20-exit` CI job (E20:27).
- FR35, FR36, FR37 (v2.2 machinery with internal vetter keys only), FR59, FR13 quarantine (PRD `functional-requirements.md:42,85-87,93`).
- `deferred-work.md:37` (same-name squatting, 20-3b row 06 A), `:56` (FR59 coverage row), `:1093` (attestation binding), `:1147` (FR13 quarantine), `:1264` (revocation-poller print, D-18-0-12 A).
- Tracker `sprint-status.yaml:296` D9 routing: a revoked or lapsed attestation on a promoted `public-vetted` Spirit reaches operators as a vetter yank carrying the operator policy; `enforce_vetted_upgrade_precondition` (`crates/maos-bin/src/main.rs:130-202`) stays the only refusal point.
- Routing audit `routing-audit-2026-10-22.md:43,124-127,169`; invariant I5's FR13 claim (`docs/invariants/I5.md:11`).

**Does NOT close, and says so:**
- The accredited external vetter ecosystem (FR37 v2.5) and an independent author (J6, `routing-audit:143`) — operator lanes; this story proves the mechanism with internal keys (D12).
- Packaging of the two new binaries in `.deb`/RPM/formula and the package-owned-path contract (D09 → 20-2).
- `maos-spirit validate|inspect` (exit-1 stubs, `maos-spirit.rs:25-28`) — out of scope.
- Execution of an installed WASM Spirit from a published binary (Hold 2; 15-5 AC3 statement, 17-3b AC3 refusal); FR8 manifest fields (20-8); crypto-provider selection (20-13, consumes AC6's binding).

## Story

As **an operator who installs third-party Spirits and the third-party author who publishes them**,
I want **a registry I can run, a publish that reaches it, an install that verifies everything and binds the publisher's key, a vetting promotion bound to the exact artifact, yanks that arrive signed and act by my policy, and a quarantine that actually isolates**,
so that **a Spirit from outside the source tree can be shared, trusted to the degree it is proven, withdrawn when it must be, and contained without being killed.**

**Contract:**
- Epic §20-1 (E20@HEAD:94-107) as amended 2026-10-30 (D01 A, preflight `epic-20-preflight-2026-10-30.md` §3.1, §2 row 20-1, §4 exit table) and D04 A (E20:102) — create-story may tighten, not add uncited ACs; AC7–AC10 are the received charter (`routing-audit:43`; S:296 D9 note; D:37, D:1264).
- Tracker `sprint-status.yaml:296`; rules 9–11 (E20 confidence rules); proof contract `docs/testing/proof-contract.md:10-31` (one final-tree run + one plant per AC), `:33-71` (review, tiers).
- Dependencies satisfied (PF:119): E19, 15-3/4/5/6, 16-1/2, 17-3b done; 20-3a/3b/3c done (gate inventory and audit surface stable).

## Inventory at HEAD — measured, not assumed

Method: every cite re-read at `a4c00897`; witnesses located by `grep` over `crates/*/tests` and inline test modules; exit verbs run on the prebuilt HEAD binaries (`creation/runtime/exit-probe-a4c00897.txt`); no cargo. "Exists" = code at HEAD with a named test; "missing" = no production path; "false" = a production path that does something other than the requirement.

| # | Charter item | Disposition | Code at HEAD | Witness / measurement | Part · AC |
|---|---|---|---|---|---|
| 1 | Registry server (library) | **exists, functional** | `crates/maos-registry/src/server.rs:51-123,229-261`; daemon arm `crates/maos-bin/src/main.rs:8980-8988` (fixed `127.0.0.1:6789`, `LocalFsRegistryStorage::new()`) | measured: ready in 3 polls, `registry.search` → `{"items":[]}`; `registry_roundtrip_test.rs` | A · AC1 |
| 2 | `maos-registry-server` binary (exit L5) | **missing** | stub `crates/maos-registry/src/bin/server.rs:4-6`; `[[bin]]` `Cargo.toml:36-38` | **rc 1** "not yet wired — Story 5.5d Task 13" | A · AC1 |
| 3 | Publisher transport (exit L7) | **missing** | guard `crates/maos-spirit-cli/src/publish.rs:266-271`; signing/tier/claim exist `:105-225` | **rc 1** (guard fires before the tier check); `publish_signing_test.rs`, `publish_tier_validation_test.rs`, `publish_happy_path_test.rs` (stub client) | A · AC2 |
| 4 | Leaf HTTP `IoSubsystemPort` | **missing** | only production impl `crates/maos-kernel-core/src/io/mod.rs:86`; transport `crates/maos-mcp/src/transport/streamable_http.rs:44-56` | — | A · AC2, AC3 |
| 5 | `maosctl spirit install` (exit L8) | **missing** | `SpiritOp` `crates/maos-cli/src/cli.rs:915` (no `Install`); reusable admission/persist `crates/maos-cli/src/subcommands.rs:1470-1519` | **rc 2** clap "unrecognized subcommand 'install'"; `import_air_gap_test.rs` covers the reused path | A · AC3 |
| 6 | `RegistryOrigin::Remote` | **missing** | `crates/maos-registry/src/origin.rs:9-17` | — | A · AC3 |
| 7 | Installed Spirit visible + dispatched (exit L9) | **missing** | shell `crates/maos-shell/src/lib.rs:506-531`; hello-only boot load `main.rs:4309-4440`; typed `WasmEngineOff` on the run path `main.rs:6607-6608`, gate `admission.rs:700-703` | measured: `maos: unknown spirit 'example-spirit-ts' — known: hello-spirit`; `wasm_form_admission_17_3b.rs` (run path) | A · AC4 |
| 8 | Four-binary release set (exit L1/L3) | **missing** (2 of 4) | `xtask/src/release_dry_run.rs:27-36`; `.github/workflows/release.yml:76-90` | source (line 1 builds; not run); lines 2 and 4 **rc 0** on a dev-signed two-binary dist | A · AC5 |
| 9 | `epic-20-exit` job | **missing** | 0 hits in `discipline.yml`, `gate-registry.toml` | — | A · AC5 |
| 10 | Example/template tier | **`local`** | `examples/example-spirit-ts/manifest.toml:9`; `templates/spirit-ts/manifest.toml:9` | — | A · AC2 |
| 11 | Registry form admission (wasm-component / rust-inproc) | **exists** (17-3b) | `crates/maos-registry/src/admission.rs:247-253` | `crates/maos-registry/tests/form_admission_17_3b.rs` (4 tests) | A · AC3 (reuse) |
| 12 | Vetting attestation verify/keyring | **exists**; **binding false** (manifest only) | `crates/maos-compliance/src/vetting/attestation.rs:105-129`; `mod.rs:197-209`; caller `maos-registry/src/admission.rs:388` | `vetting_attestation_gate.rs` (7); golden pin `attestation.rs:220-224,274` | B · AC6 |
| 13 | `maos-spirit vet issue|revoke` + revocation-list issuer | **missing** | `issue_attestation` `attestation.rs:150`: **0 production callers** (all `#[cfg(test)]` / `tests/`) | — | B · AC6 |
| 14 | ≤5 min yank poll | **exists, wired** (http registry URI only) | `main.rs:3321-3347`; `crates/maos-registry/src/yank.rs:62-128,150-210` (300 s default, clamp 30–3600 s) | `fr59_yank_propagation_within_5min_test.rs` (in-memory, 4) | B · AC7 |
| 15 | Yank restart safety | **false** | stamps `storage.rs:365-394` with process-relative `monotonic_now_ns()` `:499-503`; cursor I/O `yank.rs:212-306` has **no production caller** | measured `yanked_at_ns = 2885` on a fresh server | B · AC7 |
| 16 | Publisher/vetter yank authentication + origin | **missing** | `registry.deprecate` → `handlers/deprecate.rs:10-20` (no credential); `YankEntry` has no origin (`maos-domain/src/ports/registry.rs:331-340`) | measured: an unauthenticated `registry.deprecate` yanked `anyone-elses-spirit` | B · AC7 |
| 17 | Yank policy `warn|quarantine|auto-revoke` | **missing** | `TlYankObserver::on_yank` `main.rs:311-346` journals `SpiritRevoked` + observation only; `MAOS_YANK_POLICY` absent (`env_contract.rs:13`) | — | B · AC8 |
| 18 | Vetter lapse/revocation reaching operators (D9) | **missing producer** | taxonomy `crates/maos-compliance/src/vetting/terminal.rs:24-99`; `observe_running_spirit` production caller = the yank observer only (`main.rs:335`) | `vetting_attestation_gate.rs:329-346` (test-only inputs) | B · AC9 |
| 19 | Same-name squatting | **open** | registry `put` overwrites any `name/version` from any key (`storage.rs:211-250`, `handlers/publish.rs:9-22`); storage identity = bare `spirit_id` (`admission.rs:893`, `main.rs:4435`; opaque string `memory/private.rs:81-100`) | — | B · AC10 |
| 20 | FR13 quarantine isolation | **false** | `crates/maos-kernel-core/src/revocation/applier.rs:160-197`: marker, then drain deadline, then **terminate + unload**; fence API exists `capability/mod.rs:203,319,332-357` | `crates/maos-kernel-core/tests/fr51_pause_fence_19_2.rs` (the pause fence) | B · AC11 ◆ |
| 21 | Revocation-poller print | **open** | `crates/maos-kernel-core/src/revocation/poller.rs:44-56` prints every failed poll | — | B · AC12 ◆ |

**Exit verbs runnable at HEAD:** line 2 (rc 0), line 4 (rc 0, on a two-binary dist), line 9a `maos init` (rc 0), the daemon `MAOS_ONE_SHOT=registry-server` arm. Not runnable to the epic's observation: 1 (two of four artifacts; not run — cargo), 3 (two links would dangle), 5 (rc 1), 6 (no server), 7 (rc 1), 8 (rc 2), 9b (`unknown spirit`).

## Current-tree corrections — read before implementing

Full OLD → NEW: `creation/cites-a4c00897.md` (C1–C33, X1–X10, F1–F12, L1–L8).

| Premise (epic / preflight / tracker) | Fact at `a4c00897` and consequence |
|---|---|
| "kernel-Δ 0" (E20:71,96) | Superseded by D04 A; measured R = +78 / +68 (§Kernel). The `Closes · Δ` cell and the stories-table row are corrected through records-owed. |
| ≤5 min propagation is new work (E20:100, PF:119) | The poller is wired (F1); what is missing is policy, origin, authentication, **restart-safe cursors** (F2) and cursor persistence (F3). |
| Yanks are an operator-trust event | `registry.deprecate` accepts any caller (measured); `registry.publish` overwrites any name from any key (F4). AC7/AC10 close both. |
| D04 B "a sufficient existing non-kernel isolation API" | The Story 19-2 fence (`set_paused`) is that API for a **yank-policy** quarantine (prototype B, maos-bin +43); it cannot change the **CRL** Quarantine arm, which runs inside the kernel from the poller (`poller.rs:65`) and terminates after the drain deadline (`applier.rs:170-197`) — F7, F8. |
| Any small applier change | Pays +40 / +40 of rustfmt churn (F9, control `fmt0`): the `apply_crl` body is unformatted today because one `json!` line exceeds 100 columns. |
| Only the registry-server arm reads the store (E20:104(d)) | False: the eval-sidecar reader also does (`main.rs:801,821`); still no boot enumeration of installed Spirits (X9). |
| Line anchors E20:98-107, E20:29, E20:36-40 | Moved: daemon arm `main.rs:8980-8988`; `SpiritOp` `cli.rs:915`; import `subcommands.rs:1470-1542`; guard `publish.rs:266-271`; `TlYankObserver` `main.rs:273-347`; shell `lib.rs:506-531`; `WasmEngineOff` `main.rs:6607-6608` (C1–C33). |

## Kernel and kloc pricing — measured at creation

**Instrument:** prebuilt `/mnt/build/cargo-maos/debug/xtask` (`check-kernel-baseline --json`, `kloc-check --json`) run from a `git archive a4c00897` copy root per prototype; the untouched copy reproduces the tree exactly (26801 / 95 files / set hash `c023d0d7…`; aggregate 188816). Detail and diffs: `creation/measurements.md`, `creation/kloc-kernel-a4c00897-and-prototypes.json`.

**Kernel (◆ FLAG-Winston, PROPOSED):**

| Shape | Kernel physical | Kernel tokei | Other | Diff |
|---|---:|---:|---|---|
| fmt0 control (no semantic change) | +40 (26841) | +40 (21051) | — | `fmt0-applier-format-control.diff` |
| KQ0 pause fence, non-sticky (D-02 B) | +43 (26844) | +40 (21051) | — | `kq0-fence-only.diff` |
| KQ1 sticky quarantine fence (D-02 A) | +68 (26869) | +59 (21070) | — | `kq1-sticky-quarantine.diff` |
| KQ2 KQ1 + outbound IAC fence (D-02 C) | +69 (26870) | +60 (21071) | maos-iac +24, maos-domain +2 | `kq2-sticky-quarantine-plus-iac-egress.diff` |
| P poller edge-trigger (D-03 A) | +10 (26811) | +9 (21020) | — | `p-poller-edge-trigger.diff` |
| **R = KQ1 + P (recommended)** | **+78 (26879)** | **+68 (21079)** | — | `krec-kq1-p-bundle.diff` |
| R′ = KQ2 + P | +79 (26880) | +69 (21080) | maos-iac +24, maos-domain +2 | `kalt-kq2-p-bundle.diff` |
| Rmin = KQ0 + P | +53 (26854) | +49 (21060) | — | `kmin-kq0-p-bundle.diff` |
| B non-kernel (D-02 D; D04 B comparison) | 0 | 0 | maos-bin +43 | `b-nonkernel-yank-quarantine.diff` |

Both kernel pins move with any kernel shape (content hashes of `revocation/applier.rs`, `revocation/poller.rs`, `capability/mod.rs`, `scheduler/scheduler_loop.rs`). Not compiled ([INFERENCE]; measurement owed 1).

**kloc forecast** (tokei; HEAD = every pin, zero aggregate headroom; recommended options):

| Part · crate · component | Measured / forecast | Basis |
|---|---:|---|
| A · maos-registry · server binary (AC1) | **+38** | prototype `s1` |
| A · maos-registry · leaf blocking-HTTP `IoSubsystemPort` (AC2/AC3) | +80…+150 [INFERENCE] | the server's own reader `server.rs:131-224` (≈ 90 lines) |
| A · maos-registry · `RegistryOrigin::Remote` + install helper (AC3, D-05 A) | +64…+130 [INFERENCE] | `dispatch_import` admission/persist `subcommands.rs:1470-1519` (≈ 50 lines) |
| A · maos-spirit-cli · client over the leaf port, guard deleted (AC2) | +20…+50 [INFERENCE] | stub branch `publish.rs:242-264` |
| A · maos-cli · `SpiritOp::Install` thin verb (AC3, D-05 A) | +40…+90 [INFERENCE] | 13.4 precedent `cli.rs` +23 for one verb; summary `subcommands.rs:1521-1542` |
| A · maos-bin · arm parity + boot store enumeration + typed refusal table (AC1, AC4) | +63…+160 [INFERENCE] | hello load `main.rs:4309-4440` |
| A · maos-shell · `@<installed>` dispatch (AC4) | +10…+40 [INFERENCE] | `lib.rs:524-531` |
| A · xtask · `ARTIFACTS` +2, exit table (AC5) | +5…+20 [INFERENCE] | `release_dry_run.rs:27-36` |
| B · maos-kernel-core · R (AC11, AC12) | **+68** (physical +78) | prototype `krec` |
| B · maos-compliance · claim binds artifact + publisher key (AC6) | +12…+30 [INFERENCE] | `VettingClaim` `attestation.rs:105-129`, check `mod.rs:208-209` |
| B · maos-spirit-cli · `vet issue|revoke`, revocation-list issuer, publisher yank verb (AC6, AC7) | +190…+380 [INFERENCE] | `publish.rs` (≈ 290 lines for one verb) |
| B · maos-registry · signed yanks + origin, sequence cursor, publisher-key pin, `YankPolicy` (AC7, AC8, AC10) | +120…+265 [INFERENCE] | `handlers/deprecate.rs`, `storage.rs:211-250,365-407`, `yank.rs:150-210` |
| B · maos-domain · `YankEntry` origin/sequence/signature (AC7) | +5…+20 [INFERENCE] | `ports/registry.rs:331-350` |
| B · maos-bin · policy application, vetter lapse/revocation, keyed identity, cursor wiring (AC8–AC10) | +95…+245 [INFERENCE] | prototype `b` (+43) shape; `main.rs:311-346`; `admission.rs:893` |
| maos-capability, maos-iac, maos-mcp, maos-audit | 0 | untouched under R (KQ2: maos-iac +24, maos-domain +2) |
| **aggregate** | **+106 measured; +810…+1,686 total forecast** | Part A +320…+678; Part B +490…+1,008 |

**Envelope (D-20-1-11 A, PROPOSED; D02 A ×2 measured / ×3 unmeasured upper; ceilings, not reservations):**

| Crate | HEAD | Forecast (one key) | Ceiling with allowance | Part A ceiling | Part B ceiling | Inherited room | Operator ruling needed |
|---|---:|---:|---:|---:|---:|---:|---|
| maos-kernel-core (physical / tokei) | 26801 / 21011 | +78 / +68 | **26957 / 21147** | — | 26957 / 21147 | 0 | **yes — FLAG-Winston** |
| maos-registry | 3452 | +38 meas, +264…+545 | **5163** (pin 4282 raised) | 4368 | 4247 | 830 numeric, 0 aggregate | yes |
| maos-spirit-cli | 753 | +210…+430 | **2043** (pin 1273 raised) | 903 | 1893 | 520 numeric | yes |
| maos-cli | 6396 | +40…+90 | **6666** | 6666 | — | **none: STOP 6382 already exceeded by 14** (grant203c) | **yes — above STOP** |
| maos-bin | 33195 | +158…+405 | **34410** | 33675 | 33930 | 0 | yes |
| maos-shell | 608 | +10…+40 | **728** | 728 | — | 0 | yes |
| maos-compliance | 2032 | +12…+30 | **2122** (pin 2117 raised by 5) | — | 2122 | 85 numeric | yes (above pin) |
| maos-domain | 10093 | +5…+20 | **10153** | — | 10153 | 0 | yes |
| xtask | 42860 | +5…+20 | **42920** | 42920 | — | 0 | yes |
| aggregate | 188816 | +810…+1,686 | **193768** (+212 + 4,740); hardfail = measured + 1 | 190812 | 191772 | 1 | yes |

**maos-cli (above STOP):** the recommended placement (D-20-1-05 A) keeps maos-cli to the clap variant, the call and the summary print — +40…+90, ceiling 6666, inside the 6856 reserved pin; option B (an `import`-style verb in maos-cli) would forecast +200…+400 (ceiling 7596, above the reserved pin) and add a `maos-cli → maos-mcp` edge. It needs: a ruling above the E19 STOP 6382 (already exceeded at 6396 under grant203c) for +270 of allowance; nothing else in this story touches maos-cli.

Under D-20-1-01 A, L1 and L2 (kernel-free) land first and lower nothing; L3 asks its kernel row at its dev-plan creation against L2's exact pins. Under D-20-1-01 B, 20-1b re-measures its Part-B ceilings against 20-1a's exact landed pins (the table's Part B column is computed from HEAD). Pins land exact after `cargo fmt --all`; a lower measurement lowers the pin.

**Operator-ruling list (every draw above the inherited room; PROPOSED):** (1) FLAG-Winston kernel row R (D-20-1-02 + D-20-1-03; R′/Rmin/B priced for comparison); (2) maos-registry pin raise 4282 → ≤ 5163; (3) maos-spirit-cli pin raise 1273 → ≤ 2043; (4) maos-cli above STOP 6382 → ≤ 6666; (5) maos-bin envelope ≤ 34410; (6) maos-shell ≤ 728; (7) maos-compliance pin raise 2117 → ≤ 2122; (8) maos-domain ≤ 10153; (9) xtask ≤ 42920; (10) aggregate ≤ 193768, hardfail measured + 1; (11) the shape: one key with three landings or the split (D-20-1-01). Each figure lands exact, with a HISTORY row citing this key (or the split key).

## Decisions — RULED 2026-11-01/02 (table below = creation record)

Preflight rows that bear on this story stay as filed (`epic-20-preflight-2026-10-30.md:287,288,293,296`); their 20-1 consequence is added.

**Rulings 2026-11-01/02 (operator; asks split201 and rows201).** These supersede the Status column of the table below, which stays as filed at creation.

- D-20-1-01 = **B** (split). This file is `20-1a-registry-publish-install-exit-path` (AC1–AC5, kernel-free, L1). Part B (AC6–AC13) and Tasks T6–T10 are `20-1b-vetting-yank-policy-and-quarantine` (L2 kernel-free, then L3 kernel last). 20-1 is retired (records-owed B1–B5).
- D-20-1-02 = **A** (KQ1). D-20-1-03 = **A** (P). D-20-1-04 = **A** (offline CLI install). D-20-1-05 = **A** (`maos_registry::install`). D-20-1-06 = **A** (signed yanks). D-20-1-07 = **B** (server-assigned persistent sequence). D-20-1-08 = **A** (local CRL via `apply_crl`). D-20-1-09 = **A** (first-key pin + storage identity). D-20-1-10 = **A** (daemon re-evaluates attestation on the yank cadence). D-20-1-11 = **A** (envelope ×3 unmeasured / ×2 measured).
- D03 = **A** (RULED 2026-10-30). D04 = **A** (RULED 2026-10-30). D09 = **A** and D12 = **A** (operator, rows201: "as already ruled"; the ruling text is not recorded in a repo file).
- Kernel row R (FLAG-Winston, granted to 20-1b only): KQ1 + P, measured +78 physical / +68 tokei from HEAD 26801 / 21011; ceilings kernel ≤ 26957 / 21147. 20-7's row R (K1w + K2r; +40 / +31; ceilings 26881 / 21073) is also computed from HEAD 26801 / 21011. If both land, the second to land re-bases: cumulative measured tokei 21110; cumulative tokei ceilings must not exceed 21209 (= 791 under the ADR-069 22000 alarm); STOP otherwise.
- Other ceilings (rows201): maos-registry 5163; maos-spirit-cli 2043; maos-cli 6666 (inside the reserved 6856 pin, because install logic lives in maos-registry, D-20-1-05 A); maos-bin 34410; maos-shell 728; maos-compliance 2122; maos-domain 10153; xtask 42920; aggregate 193768.

| ID | Question | Options | Recommendation and evidence | Status |
|---|---|---|---|---|
| **D03** (preflight) | FR31 two-boot confidentiality/integrity | **A** include the repair (done at 20-3b) | 20-1 consequence: the same-name squatting residual 20-3b left labelled (row 06 A, D:37) closes here by publisher-key binding — AC10 (D-20-1-09). | RULED 2026-10-30: A |
| **D04** (preflight) | FR13 quarantine | **A** measure the real isolation path and grant the kernel change · **B** a sufficient existing non-kernel API with a hostile-effect witness · **C** ship drain + marker | RULED A. 20-1 consequence: shape is D-20-1-02; B was priced as asked (prototype `b`, maos-bin +43, kernel 0) and does **not** suffice for FR13's CRL path (F7, F8). | RULED 2026-10-30: A |
| **D09** (preflight) | Packaging/uninstall contract | **A** package manager removes owned binaries; `maos purge` removes MAOS data · **B** purge removes package binaries · **C** report-only | **A, at 20-2.** 20-1 consequence: `maos-registry-server` and `maos-spirit` join `release-dry-run` and `release.yml` only; whether the deb/RPM/formula ship them, and who removes them, is 20-2's under D09. No `packaging/` edit here. | PROPOSED (open) |
| **D12** (preflight) | rc vs v1.0 external-author claims | **A** rc mechanism-proven; full N before v1.0 claims · **B** full cohort before rc · **C** PRD N ≥ 3 | **A.** 20-1 consequence: the exit's "third-party key" is a fresh random key minted by the job (E20:22) and the vetter is internal: AC text and coverage notes say *mechanism-proven*, never J6 independence or an accredited vetter. | PROPOSED (open) |
| **D-20-1-01** | Story shape | **A** one key, three landings: L1 Part A exit path (AC1–AC5, kernel-free) · L2 Part B kernel-free (AC6–AC10) · L3 ◆ FLAG-Winston (AC11, AC12) + AC13 close; `done` after L3 · **B** split: `20-1a-registry-publish-install-exit-path` (AC1–AC5) and `20-1b-vetting-yank-policy-and-quarantine` (AC6–AC13), 20-1 retired, its five ledger rows, I5 owner and the FR13 coverage note re-pointed to 20-1b | **B.** Honest size: +810…+1,686 tokei across 9 crates (≈ 2.2–4.6× 20-3c's landed +365) on two review surfaces with different risk — a product exit path (20-2 waits on its artifact set) and a security-critical trust/revocation surface with a kernel grant (20-13 waits on AC6's binding, PF:141). Split lets 20-1a land and unblock the exit and 20-2 without waiting for a FLAG-Winston review; A keeps one key and one ledger owner at the cost of a three-landing story whose `done` waits on the kernel landing (20-3b/20-3c landed at 2–3× forecast through serial review raises). Under B this file splits mechanically at the Part boundary (records-owed B1–B5). | PROPOSED |
| **D-20-1-02** | FR13 quarantine shape (D04 A) | **A** KQ1: a dedicated quarantine fence in `CapabilityRegistryAdapter` (every existing fence site honours it), the CRL Quarantine arm sets it and never terminates, a director pause/resume cannot lift it, unload ends it (+68 / +59) · **B** KQ0: the arm reuses the pause fence (`set_paused`), never terminates (+43 / +40; a `pause`→`resume` lifts the quarantine) · **C** KQ2: A + the quarantined pid's outbound IAC frames refused (+69 / +60; maos-iac +24, maos-domain +2) · **D** non-kernel (D04 B comparison): maos-bin fences yank-policy quarantine only (+43 bin); the CRL arm still terminates | **A.** PRD: quarantine = "running but no new tool calls" (`developer-tool-specific-requirements.md:58`); the fence already covers inference, MCP tool calls, memory, worker spawn and the watchdog (F7), and WASM IAC sends are token-mediated (`maos-iac/src/adapter.rs:326-333`). B is cheaper but a routine resume silently ends a revocation; C adds bus isolation for in-proc senders the PRD does not ask for (+26 out of kernel, IacBusError matches unsearched); D leaves FR13 false and contradicts D04 A. Release: by unload only (no new verb). Worker (T3) Spirits: the fence refuses their next mediated action; in-container activity is bounded by T3 only — measured at the hosted leg (AC11.5) and named in the record if not frozen. | PROPOSED |
| **D-20-1-03** | Revocation-poller print (D:1264) | **A** land P in the kernel landing (+10 / +9): first failure, recovery with count · **B** re-dispose the row ownerless (rule 10) out of 20-1 | **A.** Ruled to 20-1 (D-18-0-12 A); the design (B, edge-trigger) is fixed; same module and the same FLAG-Winston review, so it costs no separate re-pin. | PROPOSED |
| **D-20-1-04** | "Authenticated control-plane install" (FR36) | **A** offline CLI: install verifies the publisher signature, ComplianceClaim and tier floor (existing admission) and, when a registry org key is configured (`RegistryClientConfig`, `main.rs:3314-3319`), the server's tier signature; runs with no daemon · **B** an operator-door verb: the daemon fetches, admits, persists and journals behind the door bearer · **C** both | **A.** Exit line 8 runs **before** line 9's `maos init`/`maos shell` (E20:23-24): no daemon exists to serve B; authenticity comes from signatures, not the HTTP-only loopback registry (`server.rs:1-8`). B reorders the exit (epic change) and moves the verb into maos-bin; C pays both. The daemon still journals admission when it loads the installed Spirit (AC4). | PROPOSED |
| **D-20-1-05** | Where install lives (maos-cli above STOP) | **A** `maos_registry::install` helper (fetch `registry.manifest` + `registry.artifact` over the leaf port, admit, persist with `Remote{uri}`, return the receipt); maos-cli = clap + call + summary (+40…+90); no `maos-cli → maos-mcp` edge · **B** an `import`-style dispatch in maos-cli (+200…+400; new edge; `check-dependency-closure` change) | **A.** maos-cli is 14 over its STOP; maos-registry already owns admission and the client, so one install implementation serves `maosctl` and later callers (20-8 reuses artifact admission, E20:259). | PROPOSED |
| **D-20-1-06** | Yank authentication and origin (FR59) | **A** signed yanks: a publisher yank is verified against the stored package's publisher key, a vetter yank against an enrolled vetter key; `YankEntry` carries `origin` (publisher\|vetter) + signature; unsigned `registry.deprecate` is refused; the daemon re-verifies before acting · **B** origin label only, still unauthenticated · **C** deprecate restricted to loopback, origin "registry" | **A.** Measured at HEAD: an unauthenticated `registry.deprecate` yanked an arbitrary Spirit (T1). FR59 requires publisher- and vetter-initiated yanks "distinguishable in audit" and a policy can quarantine or revoke — B lets anyone who reaches the registry quarantine any operator's Spirit; C breaks the remote registry the exit uses and still cannot tell publisher from vetter. | PROPOSED |
| **D-20-1-07** | Yank cursor across restarts | **A** server stamps wall-clock UNIX ns; the daemon persists the last value it saw · **B** server-assigned persistent sequence (`yanks.json` monotonic counter), `since` = sequence; `yanked_at_ns` kept as wall-clock for display; daemon persists the cursor | **B.** At HEAD yank stamps restart from zero per server process (F2) and the daemon never persists its cursor (F3): after either restart yanks are missed or replayed. A survives a restart but a server clock step backwards hides new yanks; B depends on no clock. | PROPOSED |
| **D-20-1-08** | `auto-revoke` semantics | **A** maos-bin mints a local CRL (`SignedRevocationList::new`, origin `RegistryYank`, signed with the operator audit key) for the yanked class/version and applies it through the existing applier (`apply_crl`, `operator_door.rs:2010` precedent): each matched Spirit executes its declared `[on_revocation].action`; kernel 0 · **B** maos-bin re-implements the applier's TerminateImmediately sequence (`applier.rs:150-158`) from pub primitives · **C** never terminate: refuse-at-next-load + quarantine | **A.** One revocation implementation, with FR13 semantics (declared action, including the real quarantine of D-20-1-02) and a journaled origin distinguishable from operator CRLs (`revocation.rs:296-310`); B duplicates the applier and drifts; C makes `auto-revoke` a synonym of `quarantine`. `warn` = journal + observation (today's behaviour); `quarantine` = the KQ1 fence on the running pids of that version. Yank alone (policy `warn`) never terminates (`yank.rs:7-10`). | PROPOSED |
| **D-20-1-09** | Same-name squatting (D:37) | **A** both: the registry and install pin the first publisher key per `spirit_id` (a different key is refused, typed) and admission binds storage identity `spirit_id` + publisher-key fingerprint for registry-installed Spirits (opaque identity, no kernel change; fingerprint truncated so `id-<hex>` stays under 255 bytes, F10) · **B** identity only (a different key gets a fresh directory, install allowed) · **C** pin only | **A.** The registry overwrites any name from any key (F4), so the pin closes the supply side; the identity binding closes the memory side 20-3b disclosed. B lets a squatter take the name and C leaves a pre-pin store or an imported bundle reusing the directory. Local unsigned Spirits keep the bare-`spirit_id` identity (no installed base to migrate, the 20-3b clean-cutover ruling). | PROPOSED |
| **D-20-1-10** | Vetter lapse/revocation of a running promoted Spirit (D9) | **A** the daemon re-evaluates each running `public-vetted` Spirit's attestation on the yank cadence (expiry against `expires_at`, revocation against the keyring/vetter yanks) and emits the `RunningSpiritObservation` + applies the yank policy as a vetter-origin yank · **B** vetter-signed yanks only (no local expiry check) · **C** next load only (today) | **A.** D9 (S:296) requires lapse **and** revocation to reach operators as a vetter yank; expiry is local time no registry event carries; C is the status quo D9 rejected. | PROPOSED |
| **D-20-1-11** | Per-crate envelope (D02 A applied) | **A** the envelope above (×3 unmeasured / ×2 measured) · **B** exact measured grants only, every review correction re-asks · **C** allowance ×1.3 (rule 6) | **A.** 20-3b needed three review-correction raises; 20-3c landed inside its ×2/×3 ceilings (`kloc.toml:264,517`); ×1.3 is not demonstrated adequate (preflight §1.3). | PROPOSED |

## Acceptance Criteria

*(Epic §20-1 as amended 2026-10-30, split 2026-11-01 into 20-1a (this story) and 20-1b under D-20-1-01 B; clauses marked (D-…) carry the RULED outcome recorded in Decisions above. Every AC names a failing-before witness and a plant; T = tier: local = hermetic on the dev host; hosted = the discipline run; T3 = the hosted capable runner.)*

### Part A — exit path (kernel-free; L1)

#### AC1 — the registry runs from its own binary (exit line 5)

1. `maos-registry-server --bind <addr> --root <dir> [--signing-key <path>]` runs `SpiritRegistryServer::new` (`server.rs:62-74`) over `LocalFsRegistryStorage::at_path(root)` (`storage.rs:126-135`), `with_server_signing_key` (`:77-80`) when given; it prints the bound address (`server.rs:88`) and exits 0 on SIGTERM. No new constructor (`at_path` is reused, E20:98). Prototype `s1` shape.
2. The daemon arm (`main.rs:8980-8988`) shares the construction; its store stays `LocalFsRegistryStorage::new()` unless a root is given.
3. **Failing-before witness (local):** `crates/maos-registry/tests/registry_server_bin_20_1.rs` runs the built binary on `127.0.0.1:0` with a temp root, publishes one package through the client and reads it back; asserts nothing is written under `$HOME/.local/share/maos/registry` — rc 1 at HEAD (measured).
4. **Plant (one, local):** ignore `--root` (use `new()`) ⇒ the root-isolation assertion red.

#### AC2 — publish reaches the real server (exit line 7)

1. The guard `publish.rs:266-271` is deleted; `maos-spirit publish` builds `McpSpiritRegistryClient` (`client.rs:46`) over `McpClientImpl` + `StreamableHttpTransport` (`streamable_http.rs:44-56`) with a leaf blocking-HTTP `IoSubsystemPort` in `maos-registry` (std::net; no tokio/hyper, as `server.rs:7-8`); the `stub` branch (`:242-264`) is kept. Tier, signature and ComplianceClaim behaviour unchanged (`publish_*_test.rs` green).
2. `templates/spirit-ts/manifest.toml:9` and `examples/example-spirit-ts/manifest.toml:9` read `trust_tier = "public-untrusted"` (hyphen, R17-62); `--tier=public_untrusted` matches (`publish.rs:115-120`); `maos run` of the example is unaffected (it never reads `trust_tier`, `main.rs:142-143` is the upgrade path only).
3. **Failing-before witness (local):** `crates/maos-spirit-cli/tests/publish_live_20_1.rs` starts an in-process `SpiritRegistryServer` on a temp root, publishes the example (prebuilt `dist/spirit.wasm` fixture or a small signed package) and asserts the receipt and `registry.manifest` round-trip — red at HEAD (rc 1, measured).
4. **Plant (one, local):** restore the guard ⇒ red.
5. The manifest change widens the focused review set (`proof-contract.md` manifest rule): every test naming the example manifest + `spirit_boundary_invariants` + `check-corpus`.

#### AC3 — `maosctl spirit install <name>@<ver> --registry-uri <uri>` (exit line 8; D-20-1-04 A, D-20-1-05 A)

1. `SpiritOp::Install { spec, registry_uri, vetting_attestation?, vetter_keyring? }` (`cli.rs:915`); the top-level release `install` (`cli.rs:44,427`) is untouched.
2. `maos_registry::install` fetches `registry.manifest` + `registry.artifact`, admits with the existing `admit_spirit` / `admit_spirit_with_attestation` (the import path, `subcommands.rs:1470-1496`; `public_untrusted` = signature + ComplianceClaim, `admission.rs:275`), refuses `rust-inproc` (`admission.rs:247`), persists with `publish_with_origin` under new `RegistryOrigin::Remote { uri }` (`origin.rs:9-17`), and prints the summary JSON (the import shape `subcommands.rs:1521-1534` + `origin`). When a registry org key is configured the server tier signature is required (`RegistryClientConfig`).
3. **Failing-before witness (local):** `crates/maos-cli/tests/spirit_install_20_1.rs` against an in-process server: install the example ⇒ store holds it with `origin.json` = `Remote{uri}`; negatives: `spirits/researcher` (`rust-inproc`) refused with the 17-3b typed error; a tampered artifact refused; a wrong-key signature refused — red at HEAD (rc 2, measured).
4. **Plant (one, local):** persist without admission ⇒ the `rust-inproc` negative red.
5. `check-dependency-closure` passes with no new `maos-cli → maos-mcp` edge.

#### AC4 — the installed Spirit is visible and answers truthfully (exit line 9)

1. Daemon boot enumerates the local store and runs each installed Spirit through `gate_manifest` + `load_admit_start` (`admission.rs:838`); a refusal is journaled and kept in a typed refusal table; on the default build an installed `wasm-component` Spirit is refused `WasmEngineOff` (`admission.rs:700-703`).
2. The shell (`lib.rs:524-531`) dispatches `@<installed>` to the loaded Spirit, or prints its typed boot refusal; "unknown spirit" only for a name neither loaded nor refused.
3. **Failing-before witness (local):** `crates/maos-bin/tests/installed_spirit_dispatch_20_1.rs`: an isolated home with the example installed (AC3), `maos init`, replay `maos shell` fed `@example-spirit-ts hello` ⇒ the typed engine-off refusal line, never `unknown spirit` — red at HEAD (measured line).
4. Hermetic execution leg (`--features maos-bin/wasm-host`, Epic 17 exit line 2's build): the installed manifest runs from the store `--once` (local if the WASM toolchain is present; else the exit job, AC5).
5. **Plant (one, local):** keep the hello-only fallback ⇒ red.

#### AC5 — the release set and the `epic-20-exit` job

1. `xtask/src/release_dry_run.rs:27-36` and `release.yml:76-90` build, stage, sum and upload `maos-registry-server` and `maos-spirit` with `maos`/`maosctl`; `check-exit-commands` resolves all four tokens.
2. `discipline.yml` gains `epic-20-exit` (ubuntu, no secrets), enrolled in the aggregate: builds the example fixture at the story SHA (componentize-js), isolates `HOME` and the registry root, readiness ≤ 5 s with an **explicit failure** after the loop (not fall-through), traps and reaps the server, asserts the exact engine-off refusal line.
3. **Plant (one, hosted, one image):** the server is not started ⇒ the job fails at the readiness assertion (`proof-contract.md:14-17`).

## Tasks / Subtasks

- [ ] **T0 — Preflight.** Operator rulings on D09, D12, D-20-1-01…11 recorded; tracker → `in-progress`. Run `creation/measurements.md` "Not measured here" 1–7; re-measure every cite this story touches (rule 9). If any measured figure exceeds the ruled envelope, STOP and return a D-row.
- [ ] **T1 — Server binary (AC1)** and daemon arm parity.
- [ ] **T2 — Leaf HTTP port + publish (AC2)**, template/example tier.
- [ ] **T3 — Install (AC3)**: helper, `Remote` origin, thin verb, negatives.
- [ ] **T4 — Boot enumeration + dispatch (AC4)**; `wasm-host` execution leg.
- [ ] **T5 — Release set + `epic-20-exit` (AC5)**; L1 landing (exact pins inside the ruled Part-A envelope; records).
- [ ] **T6–T10 moved** to `20-1b-vetting-yank-policy-and-quarantine.md` (binding and vet CLI, signed yanks and policy, lapse and squatting, L3 kernel, plants, close) under D-20-1-01 B.

## Dev Notes

### What changes, by file

| Area | Files | AC |
|---|---|---|
| Registry | `crates/maos-registry/src/{bin/server.rs, server.rs:229-261, storage.rs:108-135,211-250,365-407, origin.rs, client.rs, yank.rs, handlers/{deprecate,publish}.rs}`, new `install.rs` + leaf HTTP port module | AC1–AC3, AC7, AC8, AC10 |
| Publisher CLI | `crates/maos-spirit-cli/src/{publish.rs:226-273, bin/maos-spirit.rs:22-80}`, new `vet`/`yank` modules | AC2, AC6, AC7 |
| Operator CLI | `crates/maos-cli/src/{cli.rs:915, subcommands.rs}` (dispatch + summary only) | AC3 |
| Bin | `crates/maos-bin/src/main.rs:273-347,3251-3361,4309-4440,8980-8988`, `admission.rs:838-910`, `env_contract.rs`, `operator_door.rs` (status `fenced`, `:963`) | AC1, AC4, AC8–AC10 |
| Shell | `crates/maos-shell/src/lib.rs:506-531` | AC4 |
| Compliance / domain | `crates/maos-compliance/src/vetting/{attestation.rs:105-150, mod.rs:197-240}`; `crates/maos-domain/src/ports/registry.rs:331-350` | AC6, AC7 |
| Kernel ◆ | `crates/maos-kernel-core/src/{revocation/applier.rs:149-205, revocation/poller.rs:39-58, capability/mod.rs:135-180,332-357, scheduler/scheduler_loop.rs:572}`; pins `xtask/kernel-core-baseline.toml:505` + EOF hashes, `xtask/kloc.toml:264` | AC11, AC12 |
| Release / CI | `xtask/src/release_dry_run.rs:27-36`, `.github/workflows/{release.yml:76-90, discipline.yml}` | AC5 |
| Examples | `templates/spirit-ts/manifest.toml:9`, `examples/example-spirit-ts/manifest.toml:9` | AC2 |
| Docs / records | ADR-056 §1; `docs/invariants/I5.md`; `tests/coverage-matrix.yaml:366-371,617-632,841-846`; `4-kernel-design.md:661`; `deferred-work.md` rows (AC13) | AC6, AC11, AC13 |
| Tests (kloc-free) | new `registry_server_bin_20_1`, `publish_live_20_1`, `spirit_install_20_1`, `installed_spirit_dispatch_20_1`, `yank_auth_20_1`, `yank_policy_20_1`, `vetter_lapse_20_1`, `name_pin_20_1`, `quarantine_fr13_20_1`, `revocation_poller_print_20_1` | all |

### Overlap (serialize; re-measure)

- **`maos-kernel-core` pins** — 20-8, 20-9, 20-11, 20-13, 20-14 forecast kernel touches; one integration owner per pin update (preflight §6). L3 lands after L1/L2.
- **`main.rs` composition root** — 20-4 (provider registration) and 20-13 alter it; this story's edits are local to the cited ranges.
- **Artifact admission** — 20-8 reuses it (E20:259); 20-13 waits on AC6's crypto boundary (PF:141). AC6's claim shape is the contract both read.
- **Release set** — 20-2 consumes AC5's four-binary dist (D09).

### Current state that must be preserved

- `maosctl import` (offline, `subcommands.rs:1348-1544`) and the release `install --from-local` (`cli.rs:44,427`) unchanged.
- The yank vs CRL distinction (`yank.rs:7-10`): under `warn` a yank never terminates; operator CRLs keep their declared actions (TerminateImmediately, DrainThenTerminate unchanged).
- The Story 19-2 pause semantics (`fr51_pause_fence_19_2.rs`, `worker_freeze_19_2.rs`) and the door's `fenced` status field.
- 17-3b form admission at every tier (`form_admission_17_3b.rs`); `WasmEngineOff` on published binaries (Hold 2).
- I2 log-before-deliver on every new journal row; 20-3c's endpoint lease and audit-drain contracts on new one-shot paths.

### Guardrails

- No kernel line without the ruled FLAG-Winston row; any `crates/maos-kernel-core/src` change outside AC11/AC12 is STOP.
- No second admission implementation, transport convention or storage constructor (E20:98).
- Never weaken a witness to pass; never retry/skip a flaky leg (D13).
- One cargo process per checkout; `env -u CI -u GITHUB_ACTIONS`; inherit the target directory, never override or clean it; plants byte-restored in the real tree.
- The exit job's registry root and `HOME` are throwaway; never the developer's `~/.local/share/maos/registry`.

### Proven-red controls (rule 11; proof contract)

| Control | Plant | Expected red | Tier |
|---|---|---|---|
| Server root (AC1) | ignore `--root` | root-isolation assertion | local |
| Publish (AC2) | restore guard | live publish vector | local |
| Install admission (AC3) | persist without admit | `rust-inproc` negative | local |
| Dispatch (AC4) | hello-only fallback | refusal-line vector | local |
| Exit job (AC5) | server not started | readiness assertion | hosted, one image |
| Binding (AC6) | ignore `artifact_sha256` | swapped-artifact vector | local |
| Yank auth (AC7) | accept unsigned deprecate | unsigned-yank vector | local |
| Policy (AC8) | policy ignored | `quarantine` vector | local |
| Lapse (AC9) | skip re-evaluation | lapse vector | local |
| Squatting (AC10) | bare `spirit_id` identity | spill vector | local |
| FR13 (AC11) | restore drain-then-terminate | hostile-effect + still-loaded vectors | local (T3 Worker leg hosted, observation only) |
| Poller (AC12) | print every failure | one-line vector | local |
| Ledger (AC13) | one live row + `done` | completeness gate | local |

### Tests and commands

```bash
# local; one cargo process per checkout; inherit the target directory
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-registry -p maos-spirit-cli -p maos-cli -p maos-bin -p maos-shell -p maos-compliance -p maos-kernel-core -p xtask
env -u CI -u GITHUB_ACTIONS cargo test --locked -p maos-kernel-core --lib
env -u CI -u GITHUB_ACTIONS cargo test --locked -p maos-kernel-core --test spirit_boundary_invariants
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-corpus
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-kernel-baseline
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- kloc-check --json
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-dependency-closure
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-dev-record-completeness
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-epic-close-coherence
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- prepush
```

### Review plan (`proof-contract.md:33-71`)

DIFFICULT: Blind + Edge (opus-5.5), Acceptance (opus-5.5), Security reading on AC6 (binding), AC7 (who may yank), AC10 (who owns a name) and AC11 (what a quarantined Spirit can still do), Test Infrastructure (glm-5.3). Focused suites per touched crate + `cargo test -p maos-kernel-core --lib`; the manifest rule widens AC2's set.

### Previous-story intelligence (20-3c, 20-3b)

- 20-3c landed kernel +8 / +19 against a +12 / +26 prototype: the prototype is a price, not a patch — re-cut it at landing. Its landing-2 review found a precheck/gate mismatch the prototype did not model; budget for the same in AC11.
- 20-3b needed three review-correction raises (w8 → w9 → w10) — hence D-20-1-11.
- 20-3b row 06 A left same-name squatting labelled for this story (D:37); its identity encoding (`id-<hex>`) bounds AC10's fingerprint length.
- Both 20-3b and 20-3c final hosted runs carried known 20-7 flakes (`check-multi-tenant-loom`, `workspace-test-suite`); record by run id, never absorb (D13).
- 20-3c's `maos-cli` HTTP 411 fixture-door transient (D:3) is ownerless and may recur in the maos-cli focused suite; record, do not chase.

### Git intelligence

`a4c00897` (20-3c done records; clippy baseline ratchet), `77f7276a` (maos-a2a-tcp test warnings), `42489813` (20-3c clippy fixes, rustix flock), `1a1b11f3`/`6481e930` (20-3c landing-2 review). No registry, spirit-cli or vetting change since E17; the yank observer was last touched by 17-x (observation sink).

### Library / framework

No new crate dependency planned: the leaf HTTP port uses `std::net` (the server's own convention, `server.rs:7-8`); signatures reuse `ring`/existing Ed25519 helpers in `maos-compliance` and `maos-spirit-cli/src/signing.rs`. No web research needed.

### References

E20@HEAD:16-24,27,29,36-40,71,94-107; preflight `epic-20-preflight-2026-10-30.md:75,119,139-141,149-169,263-279,287-296`; `sprint-status.yaml:296`; `deferred-work.md:37,56,783,1093,1147,1264`; `routing-audit-2026-10-22.md:43,124-127,143,169`; PRD `functional-requirements.md:42,85-87,93-94`, `developer-tool-specific-requirements.md:58`; ADR-056, ADR-060, ADR-069 (`:35-46`); `docs/invariants/I5.md`; architecture `4-kernel-design.md:661`.

## Dev Agent Record

**Incident:** Hosted landing run `41372776259` on main `9f64dd02` had `clippy-advisory` red at 1385/899 (fixed at source; local 1339/875), and `check-exit-commands` and `check-decision-register` red from the stale `20-1` key after the split (fixed). The split-key rename broke two hosted doc gates that prepush does not run; when renaming story keys, run `check-exit-commands` and `check-decision-register`.

**Incident — hosted real-binary exit versus tiny in-process witness:** Hosted run `41424027458`, job `125206155549`, passed release creation/verification but failed publishing with `truncated or oversized HTTP header`; the in-process witness used an eight-byte artifact. Reproduced the exact diagnostic with the real publisher/server binaries and a valid4MiB component (red101). The actual12,453,178-byte example sends34,782,286 JSON body bytes: TCP capture showed valid request headers but **zero response bytes**. Atomic package JSON serialization wrote unbuffered to File, taking longer than the five-second request deadline; the server reused that expired deadline for responses/rejections and discarded the write failure. Buffered durable writes plus an independent bounded response phase repair both causes without relaxing request guards. Full evidence and additional hosted rail repairs: [hosted incident](20-1a-registry-publish-install-exit-path-evidence/landing-1/hosted-incident/summary.md).

**Incident — installed-component startup used the wrong build profile:** Hosted run `41456787892`, job `125315380721`, published/installed the real12,453,135-byte example, then the debug WASM runner hit the admitted manifest's unchanged360s startup deadline under its10% CPU cap (service360.149s; exit1; `/tmp/v201b.log:4382,4418-4423`). `spirit_session.rs:370-378` bounds child startup through compile, instantiate and on-start until Ready, not registry install. Local phase probes on the actual12,453,178-byte component measured debug compile188.606s versus release13.581s, instantiate0.003944/0.005675s and on-start0.000617/0.000484s; both direct runners returned0 without a CPU cgroup. Compilation is intentionally inside the T2 child (17-3c conformance/security contract); no untrusted deserialization cache or daemon-side compile was introduced. Both workflow copies now build and invoke release maos/runner/worker, matching the accepted17-3c proof, and retain timed execution, mandatory completion,360s and10%/768MiB caps. The workflow regression failed before the fix (exit101). Production code, deadlines, manifest and environment contract are unchanged; temporary phase probes were removed. Local real publish/install/init passed, but both actual installed boot attempts in explicit undelegated mode refused the missing writable delegated cgroup (exit1); systemd-run is unavailable (exit1), so neither local capped execution nor a new hosted success is claimed. Evidence, exact command exits, final checks and limitations: [startup incident](20-1a-registry-publish-install-exit-path-evidence/landing-1/startup-incident/summary.md).

### Story context creation

Created 2026-11-01 by `anthropic/claude-opus-5-5` (bmad-create-story, lighter measured contract) at `a4c00897`, without cargo (parent rule).

**Inputs:** E20 §20-1 at HEAD (amended 2026-10-30, preflight §3.1) — stale cites routed through records-owed; preflight §1–§5 and D03/D04/D09/D12; tracker `sprint-status.yaml:296` (D9, R10, D8, R8 notes); every `deferred-work.md` row naming 20-1; routing audit rows; direct reads of every cited range (`creation/cites-a4c00897.md`); kloc and kernel by the prebuilt gate binary on `/tmp/proto201` `git archive` copies (`creation/kloc-kernel-a4c00897-and-prototypes.json`); prototypes fmt0, KQ0, KQ1, KQ2, P, R, R′, Rmin, B, S1 saved as diffs, `rustfmt`'d, never applied to the tree; exit lines measured on the prebuilt HEAD binaries (`creation/runtime/`).

**Measurements owed:** `creation/measurements.md` "Not measured here" 1–7.

**Records owed:** `creation/records-owed.md`.

**Revert:** no crate source modified (prototypes only under `/tmp/proto201`, deleted); no cargo/test ran; `intent-lineage-coverage-report.md` untouched.

**Gates after the record edits** (prebuilt `/mnt/build/cargo-maos/debug/xtask`, `env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=…/xtask`, `--json`, in-tree; `creation/gates/*.json`): `check-dev-record-completeness` rc 0 (197 done stories, 150 deferred-owner assertions, 0 violations); `check-epic-close-coherence` rc 0 (21 epics, pin 26801, 0 violations); `kloc-check` rc 0 (aggregate 188816, `alarm=true`, `over_budget=[]`); `check-kernel-baseline` rc 0 (26801 / 95 files, set hash `c023d0d7…`, no drift).

### Agent Model Used

anthropic/claude-opus-5-5 (creation); openai-codex/gpt-6.1-sol (20-1a landing 1 implementation).

### Debug Log References

- Creation evidence: `20-1-registry-client-install-verb-vetter-and-yank-evidence/creation/{measurements.md, cites-a4c00897.md, records-owed.md, kloc-kernel-a4c00897-and-prototypes.json, fmt0-applier-format-control.diff, kq0-fence-only.diff, kq1-sticky-quarantine.diff, kq2-sticky-quarantine-plus-iac-egress.diff, p-poller-edge-trigger.diff, krec-kq1-p-bundle.diff, kalt-kq2-p-bundle.diff, kmin-kq0-p-bundle.diff, b-nonkernel-yank-quarantine.diff, s1-registry-server-binary.diff, runtime/{exit-probe.sh, exit-probe-a4c00897.txt}, gates/*}`.
- Landing evidence: [commands](20-1a-registry-publish-install-exit-path-evidence/landing-1/commands.md), [plants](20-1a-registry-publish-install-exit-path-evidence/landing-1/plants.md), `20-1a-registry-publish-install-exit-path-evidence/landing-1/{captured-commands.jsonl,commands.json,serial-junit.xml,*.log}`.
- Hosted-only handoff, not run or pushed: [ci-20-1a-verify.yml](20-1a-registry-publish-install-exit-path-evidence/ci/ci-20-1a-verify.yml), with `ac5-no-server.diff` and its SHA-256.

### Completion Notes List

#### 20-1a landing 1 implementation — 2026-11-02

**Scope and custody.** Implemented AC1–AC5 only. The parent explicitly reaffirmed the split: signed yanks/origin, persistent yank sequence/cursors, first-publisher pin/storage identity and the kernel poller remain 20-1b. The registry is therefore gated to loopback development use; no public listener/proxy is claimed safe. Main owns the corresponding shared deferred-work row. No status or tracker flip, commit, push, `.devcontainer` edit, kernel source edit or corpus tracking was performed. New environment-variable names: none; existing registry policy names are reused. `installed_spirits.rs` is enrolled in `SCANNED_SOURCE_FILES` (36 files).

**Implementation.** Shared `RegistryServerArgs` construction serves both entry points, respects explicit roots/signing keys, prints readiness and drains on SIGTERM. The standalone HTTP leaf feeds the existing MCP transport/client; the server accepts its `tools/call` envelope. Publish also needed to recognize schema-5 `[class]` identity, rather than only legacy top-level identity. Install lives in maos-registry; maos-cli is the clap variant/call/JSON print only. The artifact wire response now carries the signed ComplianceClaim needed for real admission. Install checks requested/signed identities, publisher signature, existing admission, optional vetting and configured server tier signatures before persistence, records `Remote { uri }`, and materializes the validated relative component artifact. The consumer floor comparison was corrected to a minimum, not a ceiling, so the default Local policy does not reject PublicUntrusted before admission. Boot enumerates active persisted versions, retains/journals typed refusals, and routes installed names through the shell host. The WASM-enabled branch uses the existing admission triple and supervised session; its feature build/execution remains unverified below. Release staging/signing/upload now declares all four binaries and honors the inherited target directory. The new aggregate job builds the fixture at its SHA, isolates HOME/root, explicitly fails readiness, reaps the server and asserts the exact engine-off line. FR35/FR36 coverage rows enroll that job without an independent-author claim.

**AC witnesses (all final-tree green in the serial workspace run).**

| AC | Witness | Red-before / plant | Green observation |
|---|---|---|---|
| AC1 | `crates/maos-registry/tests/registry_server_bin_20_1.rs:7` | Actual server stub red, rc101; ignore-root plant rc101 (`--root must own the package`) | Client round-trip/root custody/SIGTERM vector passed; non-loopback rejection passed |
| AC2 | `crates/maos-spirit-cli/tests/publish_live_20_1.rs:5` | Restored pre-story real-address guard, rc101 | Live in-process-server publish/receipt/manifest round-trip passed |
| AC3 | `crates/maos-cli/tests/spirit_install_20_1.rs:18,59,101,137` | Prebuild CLI rc2; admission-bypass plant rc101 on real `spirits/researcher` | Remote-origin persistence, tampered artifact, wrong publisher key, forged claim, rust-inproc refusal, org signature requirement and tier floor vectors passed |
| AC4 | `crates/maos-bin/tests/installed_spirit_dispatch_20_1.rs:7` | Creation exit probe `unknown spirit`; hello-only fallback reconstruction rc101 | Exact `maos: spirit 'example-spirit-ts' refused at boot: wasm_engine_off`, never unknown, passed |
| AC5 | `xtask/tests/registry_exit_20_1.rs:4` | Pre-landing two-binary denominator reconstructed, rc101 | Four-binary/job/aggregate enrollment vector and existing release workflow/manifest vectors passed; hosted execution is not claimed |

**Plants.** Four local AC1–AC4 mutations plus the separately named AC5 before-state reconstruction ran in the real checkout; all were copy-backed, `cmp`-verified and touched. Wall clocks and hashes are in `plants.md`. AC5's required no-server hosted plant is cut, SHA-256 `8cdf9a4ca1acf57083cdbc03f18d065b25d9c1541b8324b8d7f2643785fb33dc`, but has not run. The initial capture named `ac5-red` actually passed because compilation observed the edited table; it is explicitly not red evidence.

**Exact post-fmt measurement and pins.** `kloc-check --json` rc0 after re-pin; aggregate 191380, hardfail 191381, `over_budget=[]`. The final serde repair removes 20 lines of server fallback scaffolding, so maos-registry is 3905 rather than the earlier 3925. xtask **44297 → 44305** (20-1a landing 1; the 20-7 grant **45830** supersedes this story's historical 42920 ceiling). Every changed row retains its prior HISTORY. The maos-cli reserved pin remains 6856, with consumed 6396 → 6429 and the story STOP ceiling 6666 recorded.

| Crate | Measured | Ruled ceiling | Landed pin |
|---|---:|---:|---:|
| maos-registry | 3905 | 5163 | 3905 |
| maos-spirit-cli | 755 | 2043 | 755 |
| maos-cli | 6429 | 6666 | 6856 reserved |
| maos-bin | 33496 | 34410 | 33496 |
| maos-shell | 615 | 728 | 615 |
| maos-domain | 10099 | 10153 | 10099 |
| maos-compliance (unchanged) | 2032 | 2122 | unchanged |
| xtask | 44305 | 45830 | 44305 |
| aggregate | 191380 | 193768 | hardfail 191381 |
| kernel (physical / tokei) | 26841 / 21042 | no change allowed | unchanged |

`check-kernel-baseline --json` rc0: 95 files, changed/added/removed all empty; set hash `db39bc4a1ab6a6c22575990ae934e698a519d3efd46a8dbb38e62bc376697b59`.

**Historical red verification, preserved.** The initial focused nextest returned 100 (2699/2700): the new epic-20 hosted context needed the same local-mirroring exemption already used by epic-19; that test-only expectation was corrected. The first serial workspace run returned 100 (5507/5526, 19 failures, 138 skipped) because release fixture binaries were stale after the domain change; `serial-junit.xml` preserves it. `xtask build-test-fixtures` then returned 0, and the repaired serial workspace run passed **5526/5526**, 138 skipped (1099.808 s suite runtime). No test was narrowed or disabled.

**Final local matrix.** After Main explicitly resumed verification, `cargo fmt --all` returned 0; final workspace all-target build returned 0 (117.585 s after the serde repair). The WASM-feature all-target build returned 0, and the properly configured `MAOS_17_2_CGROUP_MODE=undelegated` WASM-feature suite passed **820/820**, 15 skipped, serially. The earlier feature attempt without that mode returned 100 and is retained rather than claimed green. Registry/spirit-cli `fixture_replay` passed 129/129 (2 skipped); kernel lib passed 253/253; `spirit_boundary_invariants` passed 1/1. The corrected 20-7 perf-gate selector passed 35/35 and the fanout contract tests passed 3/3 (the real T3 leg explicitly skips without substrate). The earlier nonexistent `perf_budgets_20_7` test-target invocation returned 101; it was a selector error, not a test failure. `check-perf-counts --dir /tmp/v207-tier-c-counts --json` returned 0 with all **six binding PASS** verdicts; exit-command and decision-register checks returned 0.

**Final gates.** All returned 0: kloc, kernel baseline, service boundary, env contract, serde error handling, composition root, J1 delegation, dev-record completeness, epic-close coherence, review findings, test cost, invariant lock, coverage matrix, coverage completeness, perf-runner policy, dependency closure, corpus, ship-gate completeness and cargo deny. The serde gate originally returned 1 on two existing server `to_vec(...).unwrap_or_else(...)` fallback blocks, newly exposed by the runnable server; both now propagate the real serde error with `?`, deleting 20 lines rather than suppressing the gate. Exact pins were lowered to the final measured values above.

**One fresh serial timing sample, not a default retry.** Main ruled the prior sample contention-tainted (disk pressure and concurrent cleanup/build work), forbade widening budgets and authorized one fresh serial junit. The pre-sample process check found no cargo/rustc/clippy-driver processes (ps exit 1 means no matches); load averages were 11.62/10.18/16.39. The fresh `--profile ci --test-threads 1` workspace run returned 0: **5526/5526 passed**, 138 skipped, 616.630 s suite runtime. `check-test-budget` on `serial-idle-final-junit.xml` returned 0: 591 binaries, Σ 616.423 s, no findings. Both samples remain recorded:

| Binary | Contention-tainted Σ seconds | Fresh Σ seconds | Unchanged budget |
|---|---:|---:|---:|
| `maos-bin::observer_activity_corpus_17_4` | 89.834 | 7.282 | 30 |
| `maos-bin::one_daemon_one_door_16_1` | 46.948 | 21.064 | 30 |
| `maos-iac::participant_recall_17_8` | 53.076 | 25.739 | 30 |
| `xtask::bin/xtask` | 33.745 | 16.380 | 30 |

The earlier serial budget check returned 1. Current budget configuration and its git history register no known-slow exception for these four names; no exception was added here.

**Clippy and benchmark exception.** A real, non-denying final workspace/all-target clippy stream returned 0 and the ratchet returned 0: **1339 occurrences / 875 source-distinct**, versus the unchanged **1383/897** baseline. Read-only hosted artifacts (41305010381 vs 41372776259) isolated two new lints behind hosted 1385/899: the 20-7 fanout helper's eight arguments and an unused `mut` in `maos-kernel-core/benches/hot_path_budgets.rs:146`. Main explicitly authorized only that benchmark `mut` removal; the helper now groups its requirements in a tuple. No lint allow or baseline change was made. Bench files are uncharged, and both kernel-baseline and kloc checks remained 0 with kernel **26841/21042** unchanged. No post-fix hosted count is claimed.

**Prescribed prepush.** With inherited `/mnt/build/cargo-maos`, `CI`/`GITHUB_ACTIONS` unset, `CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask` and `MAOS_17_2_CGROUP_MODE=undelegated`, the exact `.../debug/xtask prepush --json` returned **0** (446.140 s): 84 pass records, 7 skips, one advisory. The original prepush returned 1 only for the now-repaired serde gate. The final prepush's unrestricted-default timing advisory is not hidden: one-daemon Σ30.541 s and participant-recall Σ40.382 s exceeded 30 s; the separately required fresh serial budget check passed without changing either budget.

**Remaining observations and unverified custody.** The separately requested unrestricted default workspace run returned 100: 5520/5526 passed, five failed and one timed out, 138 skipped. Each failure's name, symptom-family/new classification and same-line rule-10 disposition is recorded in [default-failures](20-1a-registry-publish-install-exit-path-evidence/landing-1/default-failures.md); each passed serially, and that default run was not retried-to-green or erased. Hosted `epic-20-exit`, its no-server readiness plant, installed component execution and real T3 remain unrun locally and unpushed; the complete cut handoff is linked above. Review is not performed by the implementation author. 20-1b's auth/yank/cursor/key-pin/kernel work and shared deferred-work ownership remain outside this landing. All exercised commands and exits are in `commands.md`/`captured-commands.jsonl`; early Eval cargo commands lacked the exported target-dir variable, as recorded, while subsequent/final captures inherited the required target unchanged.


#### 20-1a landing 1 — review corrections after `850e4c00`

**Authority and scope.** Main adjudicated the independent `BmadReview201a` P1–P12 / deferred 1–3 and `SecReview201a` F1–F7 findings. This correction keeps the kernel unchanged and does not import 20-1b. Registry publications remain an untrusted inbox; only the install transaction can activate the separate operator inventory. Existing installations in the old inbox must be reinstalled through the verified install path; no unsafe automatic promotion or compatibility alias was added.

**Per-finding implementation and runtime vectors.** Paths below are repository-relative. All listed repaired focused vectors returned 0; negative-before observations are preserved rather than rerun-to-green. Source and vector locations refer to the formatted corrected tree.

| Finding | Change site | Vector and observed control |
|---|---|---|
| P1 / F2 | `crates/maos-registry/src/install.rs:111`, `storage.rs:142`; `crates/maos-bin/src/installed_spirits.rs:25` | `boot_rechecks_manifest_component_publisher_and_claim` (`crates/maos-bin/tests/installed_spirit_dispatch_20_1.rs:82`), and inbox-is-not-install (`registry_security_20_1a.rs:94`): red100 → green0. Boot re-verifies publisher signature, identity, semantic ComplianceClaim, manifest, stored artifact and materialized component bytes before structural/spawn admission. Invalid installs report `admission_refused`; they never reach launch. |
| F3 / deferred 1 | `crates/maos-registry/src/storage.rs:555` and checked spirit/version directory helpers | `traversal_publish_is_an_rpc_error_without_escape_files` (`crates/maos-registry/tests/registry_security_20_1a.rs:80`): red100 → green0; invalid name and version publish requests return JSON-RPC errors without escape files. |
| F1 | `crates/maos-registry/src/server.rs:369,395` | `browser_and_rebinding_requests_cannot_publish` (`registry_security_20_1a.rs:47`): red100 → green0. Enforces loopback Host, loopback Origin when present, JSON content type and exact `/mcp`; covers hostile Host, remote/null Origin, simple-request content type and wrong path. This is not mutation authentication. |
| F7 / P11 / deferred 3 | `crates/maos-registry/src/server.rs:110,157,395` | Header/count (`registry_security_20_1a.rs:118`), population (`:137`), absolute deadline (`:155`) and active-trickle SIGTERM (`:198`): red100 → green0. Limits are 16 KiB / 64 headers, eight connections and five-second absolute request I/O; shutdown actively cancels worker sockets before joining. |
| P2 | `xtask/src/release_dry_run.rs:163,211` | `configured_target_and_staging_use_the_same_built_executable` / `successful_command_without_an_executable_receipt_cannot_sign_stale_bytes` (`xtask/tests/release_build_integrity_20_1a.rs:57,78`): red100 → green0. Cargo metadata resolves the target once; emitted executable receipts must match it. No target-dir override is introduced; stale staged bytes cannot become a release manifest. |
| P3 | `crates/maos-registry/src/admission.rs:507`, `client.rs:170` | `server_tier_floor_uses_admission_policy_ordering` (`registry_security_20_1a.rs:252`): red100 → green0 over all sixteen tier/floor combinations. Client and admission share policy scores, not ABI enum discriminants. |
| P4 | `crates/maos-registry/src/storage.rs:179,578` | `materialization_failure_never_activates_a_partial_install` (`crates/maos-cli/tests/spirit_install_20_1.rs:198`): red100 → green0. All files are prepared in an unactivated generation; the inventory index switches atomically only afterwards, and failed generations are removed. |
| P5 | `crates/maos-registry/src/server.rs:216` | `malformed_tools_call_has_a_json_rpc_error` (`registry_security_20_1a.rs:64`): red100 → green0; missing tool name/arguments remain in the response-producing result path and preserve request id. |
| P6 | `.github/workflows/discipline.yml:2951,2956` | `registry_release_set_and_exit_job_are_enrolled` (`xtask/tests/registry_exit_20_1.rs:4`): red100 → green0. The blocking executable exit job now builds the WASM-enabled surface, provisions substrate and runs the actual admitted installed manifest `--once`, requiring `spirit_session_complete`. It uses the existing delegated helper's guarded `maos-17-3c-proof` unit. Actual hosted execution is not claimed. |
| P8 | `tests/harness/registry_20_1.rs:44`, `xtask/src/build_test_fixtures.rs`, `xtask/src/lib.rs:2` | `sibling_provenance_refuses_an_unrecorded_executable` (`registry_security_20_1a.rs:191`): red100 → green0. Registry server and publisher join the registered fixture table; all siblings resolve through its input/freshness provenance, never presence alone or an untracked prior build. |
| P9 | `crates/maos-cli/tests/spirit_install_20_1.rs:227` | Independently keyed server happy path and wrong-server-key/no-persistence negative: green0. Presence-only crypto-verifier plant made the wrong key succeed: red100; cp/cmp/touch restoration all0; restored vector green0. Publisher/server seeds are distinct. |
| P10 | `crates/maos-registry/src/install.rs:19` | `built_in_shell_names_are_reserved_before_admission` (`spirit_install_20_1.rs:273`): red100 → green0; both `hello-spirit` and `butler` are refused before installation. |
| P12 | `crates/maos-registry/src/http.rs:133` | `later_resolved_address_is_tried_after_connection_refusal` (`http.rs:176`): first-address-only plant red101, cp/cmp/touch all0, restored green0. Iterates resolved addresses with bounded per-attempt and total connect time. |
| deferred 2 | `crates/maos-registry/src/storage.rs:266,578` | `index_persistence_failure_cannot_return_a_receipt` (`registry_security_20_1a.rs:105`): red100 → green0. Serialization/write/rename failures surface, and failed index persistence rolls back the in-memory mutation. |

**Current exact formatted measurement.** `review-repaired-final-measurement.log` returned 0 after exact re-pins. Review delta is **+430**: registry +327, xtask +101, bin +2. D11 xtask **44305 → 44406** is recorded in the first HISTORY segment under this story key; prior history is preserved and the superseding 20-7 ceiling remains 45830. Read-only discovery and its regression vector account for the final +13 beyond the first review preflight.

| Scope | Measured | Ruled ceiling | Pin |
|---|---:|---:|---:|
| maos-registry | 4232 | 5163 | 4232 |
| maos-spirit-cli | 755 | 2043 | 755 |
| maos-cli | 6429 | 6666 | 6856 reserved, unchanged |
| maos-bin | 33498 | 34410 | 33498 |
| maos-shell | 615 | 728 | 615 |
| maos-domain | 10099 | 10153 | 10099 |
| maos-compliance | 2032 | 2122 | unchanged |
| xtask | 44406 | 45830 | 44406 |
| aggregate | 191810 | 193768 | hardfail 191811 |
| kernel physical / tokei | 26841 / 21042 | no change | unchanged |

**Observed review preflight.** Fmt0; repaired all-target build0 (383.144 s), final restored-source all-target build0 (3618.685 s); fixture provenance builds0. Focused registry13/13, install7/7, engine-off/boot2/2, release10/10 all0. The WASM-feature boot-integrity vector passed serially in undelegated mode (1/1); this is tamper refusal proof, not an invented real T3 execution. P9 and P12 restored controls both passed. All requested static gates, including exit commands and decision register, returned0; deny0. A real non-denying clippy stream and ratchet returned0 with **1339 occurrences / 875 source-distinct**, baseline **1383/897 unchanged**. The first all-target attempt returned101 because the shared fixture-target resolver was not exposed in the library root; the single module exposure repaired that compiler error and is included in the exact measurement.

**Adjudicated final-matrix repairs.** The first review serial run returned100 (5540/5544 passed, four failures); default returned100 (5541/5544, three failures); fixture-replay returned100 (140/141); prepush returned1 with workspace-nextest red. These results remain in the ledger. Three assigned issues were repaired, with Main approval: (1) boot enumeration now uses read-only `LocalFsRegistryStorage::default_installed_manifests` (`storage.rs:136`, `main.rs:4311`) rather than a constructor that creates registry directories after control readiness; the unchanged purge-filesystem-identity regression and the new absent-inventory read-only unit both passed; (2) the deadline oracle accepts real peer closure as EOF **or TCP reset**, never a read timeout, and keeps its seven-second bound; a new sliding-timeout production plant still failed100, then was cp/cmp/touch-restored byte-identically; (3) Main read and adjudicated P3’s behavior-preserving score extraction and explicitly authorized the **non-kernel FKCS declared-admission-files content-hash** re-pin, `9b66a40a2d25e4c8fabca5d3cd6185d8eade65c1de8989480d8dd107fd2451d6` → `5b3fe85c6abdbf45ea73b57edae6f427d1e4b445b6ae9c4ac24ba5a77798a4f4`. Frozen tag, declared file set, kernel and admission behavior remain unchanged. The real FKCS oracle, including its one-byte mutation controls, passed0.

**Corrected-source preflight.** Fmt0, all-target build0 (130.183 s), registered fixture build0; P9/P12 restored controls0; all eleven registry security vectors0; read-only unit0; unchanged purge identity witness0; FKCS oracle0; WASM-feature boot-integrity0. All required static gates returned0 again, as did deny and a new non-denying clippy stream/ratchet (**1339/875**, baseline **1383/897 unchanged**). No production deadline, test timeout, budget or kernel guard was weakened.

**Known observations retained, not retried-to-green.** `one_shot_drain_20_3c::hello_spirit_drains_its_audit_writer` took15.704532458 s against its5 s assertion in the first review serial run — known20-3c audit-drain timing symptom family; rule10: observation remains open, no timeout change or claimed repair. `t3_terminal_helpers_17_7::teardown_waits_for_a_stalled_snapshot_owner_then_removes_the_container_20_7` reported one of20 observers never entered inspect in the first review default run — known20-7 observer/stalled-snapshot family; rule10: observation remains open for Main, no kernel edit or narrowed test. The passing serial timing-budget gate is recorded separately from functional-suite success.

**Final corrected-source workspace matrix.** The serial workspace run returned0: **5545/5545 passed**, 138 skipped (`review-repaired-workspace-serial.log:7322`, suite616.113 s, command746.396 s). Its independently saved serial junit passed the budget gate0: 593 binaries, Σ615.957 s, no findings. Startup load was2.15/5.29/9.19 and the compiler-process check found none (ps1). The unrestricted default run also returned0: **5545/5545**, 138 skipped (`review-repaired-workspace-default.log:7314`,44.019 s suite). Fixture replay returned0: **142/142**, two skipped; kernel lib and spirit-boundary tests returned0. These broad runs followed actual owned-code/oracle/pin repairs; the earlier drain and observer failures remain recorded and are not claimed fixed.

**Prescribed prepush, final result remains red.** The exact required undelegated invocation returned **1** (498.647 s). Its sole red check is workspace-nextest: retained `review-repaired-prepush-junit.xml` identifies `t3_terminal_helpers_17_7::teardown_waits_for_a_stalled_snapshot_owner_then_removes_the_container_20_7` (`crates/maos-kernel-core/tests/t3_terminal_helpers_17_7.rs:564`),10.077 s, instance12 observer never entered inspect — known20-7 stalled-snapshot/observer family; rule10: no retry, kernel change or claimed repair, deferred to Main. All other required prepush records passed; skips and the advisory are retained in JSON. The unrestricted-default timing advisory names one-daemon Σ30.358 s and participant-recall Σ42.273 s against30 s; it is not the separately required serial budget result, which passed unchanged. This is not an all-green prepush or hosted-acceptance claim.

**Hosted custody and explicit 20-1b deferrals.** P7's hosted no-server readiness control remains proof owed, not closed by local enrollment. The current cut diff and SHA-256 remain `8cdf9a4ca1acf57083cdbc03f18d065b25d9c1541b8324b8d7f2643785fb33dc`; handoff mirrors the executable job and retains supplemental T3 root-shutdown controls. No CI push/run or new author-self-review is claimed. Main retains F4 overwrite/immutability/signer pinning, F5 trust-root pinning/TLS, F6 yanked-version install refusal, and server-side publish/deprecate authentication in 20-1b, together with its signed yanks/cursors/kernel work. No new environment names or maos-bin source files were added; existing scanner enrollment remains valid. No status flip, commit, push, kernel edit or `.devcontainer` change occurred.

#### 20-1a hosted-exit incident repair — 2026-11-02

**Repair and binary witnesses.** `storage.rs:588` buffers JSON token writes and flushes before sync/rename. `server.rs:218,355,359` retains the five-second absolute request deadline, separately bounds response writes, unifies complete Content-Length/Connection: close framing, distinguishes body408 from truncated400, half-closes writes and drains under a100ms absolute close bound. Worker errors reach stderr. No Host/Origin/path/content-type/header/body/population guard was weakened. `publish_live_20_1.rs:30` is a real publisher→registered server binary4MiB component witness (exact hosted error red101→green0,focused2.023s). `registry_server_bin_20_1.rs:109,154` requires complete framing and clean EOF for400,403 Host/Origin,404,405,411,413,415,431 byte/count,408 header/body and503 (body400 red101→all4 vectors green0). Original in-process witness retained.

**Workflow and other hosted causes.** Actual example componentization succeeded with ignored local Node22/native optional binding after system Node20/npm failed; no npm manifest/lock edit. Four-binary release build,dev-key verification,verify-only,example publish/name install/init/replay shell all0; exact engine-off refusal,never unknown; SIGTERM0. TCP response0→198 bytes; publish30.711s/exit1→0.915s/exit0. Hosted diagnostic diff41305010381→41423605076 identifies new server `collapsible_if` and boot dispatch `result_large_err`; the read-error refactor and formatting delivery results inside block_in_place remove them without lint allows/boxing. FR35's negative author disclaimer caused FF-J6's whole-file wildcard false positive; honest `no independent-author claim` relabel fixes it without a latency promise or gate change. Multi-tenant and whole-publisher CI jobs now build registered siblings before consumption; workspace job already builds all. No cargo-spawning tests/stale fallback. Empty fixture-manifest-directory setup0→unchanged boot witnesses2/2,exit0; ordering regression2/2.

**Final verification and pins.** Fmt0,final all-target build0,fixtures0,focused registry/publisher122/122(2 skipped),kernel lib253/253,fixture-replay145/145(2 skipped),WASM boot-integrity1/1 and supplemental root-shutdown0. Fresh serial **5549/5549**,138 skipped,exit0,suite650.632s; saved junit budget0,593 binaries,Σ650.247s,no findings. No compiler processes at start; host load106.48/123.09/119.84 disclosed,not described as idle. Unrestricted default **5549/5549**,138 skipped,exit0,suite42.974s. Requested static gates and applicable coverage/fixture/registry discipline gates0; deny0; actual non-denying clippy0,ratchet0 at **1335/873**,baseline **1383/897 unchanged**. Initial mixed-stderr ratchet parsing failure remains recorded. Registry **4248**≤5163; aggregate **191826**,hardfail**191827**≤193768; prior HISTORY retained. Bin33498,xtask44406 unchanged. Kernel **26841/21042**,95 files,set hash `db39bc4a1ab6a6c22575990ae934e698a519d3efd46a8dbb38e62bc376697b59`,changed/added/removed empty; no kernel/FKCS pin edit.

**Prepush and explicit open observations.** Prescribed undelegated prepush **0**,322.540s:83 pass,8 skip,1 advisory (default one-daemon33.575s/participant-recall40.651s versus unchanged30s budgets; separate serial budget0). Complete local multi-tenant gate **1**,1226.078s,stopped before installed tests at unrelated `hook_pump_17_4::one_budget_overrun_quarantines_the_subscriber:439` (delivered1/failed0 instead of delivered0/failed1); no retry-to-green or claimed repair. Live Postgres/operator-key legs locally absent. Actual T3 component execution remains blocked by rootless Podman clone EPERM(provisioner1/runtime125),systemd-run1(non-systemd PID1); execution-surface builds passed but no successful T3 execution claimed. Hosted rerun/no-server control remains owed. No commit,push,status flip,cargo clean,target override,or .devcontainer edit. Exact commands/exits,wire captures,diagnostic diff,junits and retained errors: [incident summary](20-1a-registry-publish-install-exit-path-evidence/landing-1/hosted-incident/summary.md),`hosted-incident/commands.jsonl`,`execution-env.json`. Historical landing/review failures remain intact.


### File List

- Creation: this story file; `20-1-…-evidence/creation/*`; `sprint-status.yaml:296`.
- Landing production: `Cargo.lock`; `crates/maos-registry/Cargo.toml`, `src/{bin/server.rs,server.rs,http.rs,client.rs,install.rs,lib.rs,origin.rs,storage.rs}`; `crates/maos-domain/src/ports/registry.rs`; `crates/maos-spirit-cli/src/{publish.rs,signing.rs}`; `crates/maos-cli/src/{cli.rs,subcommands.rs}`; `crates/maos-bin/src/{lib.rs,main.rs,installed_spirits.rs,shell_host.rs,spirit_session.rs}`; `crates/maos-shell/src/lib.rs`; `xtask/src/release_dry_run.rs`, `xtask/{gate-registry.toml,kloc.toml}`.
- Landing tests: `tests/harness/registry_20_1.rs`; `crates/maos-registry/tests/registry_server_bin_20_1.rs`; `crates/maos-spirit-cli/tests/publish_live_20_1.rs`; `crates/maos-cli/tests/spirit_install_20_1.rs`; `crates/maos-bin/tests/{installed_spirit_dispatch_20_1.rs,cohort_daemon_smoke_13_5c.rs}`; `xtask/tests/{registry_exit_20_1.rs,release_dry_run_gate.rs,release_workflow_15_4.rs,prepush_blocking_20_3b.rs}`.
- Authorized 20-7 diagnostic repairs during verification: `crates/maos-bin/tests/nfr_perf_8_real_spawn_fanout_20_7.rs` (group helper requirements, no behavioral weakening); `crates/maos-kernel-core/benches/hot_path_budgets.rs:146` (remove only unused `mut`, no pinned `src` change).
- Landing release/docs/evidence: `.github/workflows/{release.yml,discipline.yml}`; `examples/example-spirit-ts/{manifest.toml,README.md}`; `templates/spirit-ts/{manifest.toml,README.md}`; `tests/coverage-matrix.yaml` FR35/FR36 only; this story's Dev Agent Record; `20-1a-registry-publish-install-exit-path-evidence/{landing-1,ci}/**`.
- Review corrections: `crates/maos-registry/src/{admission.rs,client.rs,http.rs,install.rs,server.rs,storage.rs}`; `crates/maos-bin/src/{installed_spirits.rs,main.rs}`; `xtask/src/{build_test_fixtures.rs,lib.rs,release_dry_run.rs}`, `xtask/{kloc.toml,fkcs-baseline.toml}`; `tests/harness/registry_20_1.rs`; `crates/maos-registry/tests/registry_security_20_1a.rs`; `crates/maos-cli/tests/spirit_install_20_1.rs`; `crates/maos-bin/tests/installed_spirit_dispatch_20_1.rs`; `xtask/tests/{registry_exit_20_1.rs,release_build_integrity_20_1a.rs}`; `.github/workflows/discipline.yml`; example/template READMEs; this story and its review evidence/handoff.
- Hosted incident: `crates/maos-registry/src/{server.rs,storage.rs}`; `crates/maos-bin/src/installed_spirits.rs`; `crates/maos-registry/tests/registry_server_bin_20_1.rs`; `crates/maos-spirit-cli/tests/publish_live_20_1.rs`; `tests/harness/{registry_20_1.rs,fixtures.rs}`; `xtask/tests/registry_exit_20_1.rs`; `.github/workflows/discipline.yml`; `tests/coverage-matrix.yaml` FR35; `xtask/kloc.toml`; this record and `landing-1/{commands.md,hosted-incident/**}`.

### Change Log

- 2026-11-01: Story created at `a4c00897` (ready-for-dev; D09, D12 and D-20-1-01…11 PROPOSED). Inventory measured: exit lines 2/4 runnable, 5 rc 1, 7 rc 1, 8 rc 2, 9 `unknown spirit`; the daemon registry arm functional and its `registry.deprecate` unauthenticated; yank stamps process-relative and the cursor unwired; FR13 quarantine terminates. Kernel priced by prototype (R = KQ1 + P +78 / +68 incl. +40 / +40 rustfmt churn; R′ +79 / +69 + iac/domain +26; Rmin +53 / +49; non-kernel B maos-bin +43, FR13 still false); S1 maos-registry +38; total forecast +810…+1,686; envelope and operator-ruling list filed; split proposed (D-20-1-01 B).
- 2026-11-01: Split into 20-1a/20-1b under D-20-1-01 B (ask split201); rulings 2026-11-01/02 (ask rows201) recorded in Decisions. The 20-1 sprint key is retired; kernel grant R (KQ1 + P, +78 / +68) is held by 20-1b only.
- 2026-11-02: 20-1a landing 1 implementation, local red controls and continued verification recorded without status flips. Final source build, 5526/5526 serial workspace, 820/820 WASM-feature serial, required static gates, serial timing budget, deny and prescribed prepush passed; registry/aggregate were re-pinned after direct serde propagation removed 20 lines. The earlier unrestricted default failures, prepush default timing advisory and unrun hosted/T3 legs remain explicitly recorded.
- Review corrections after `850e4c00`: independent review findings patched with real failing-before vectors and byte-identically restored crypto/resolver plants; initial +417 and final +430 exact re-pins within the existing envelope; executable WASM component leg enrolled while hosted P7/T3 proof remains explicitly owed. No status flip.
- 2026-11-02: Hosted binary publish incident reproduced at realistic component size; buffered atomic storage and bounded complete HTTP responses repaired, rejection classes and real binary publish witnessed red→green. Hosted lint/FF-J6/fixture setup causes repaired. Final5549/5549 serial/default,serial budget,static gates,clippy ratchet,deny and prescribed prepush passed; unrelated local hook timing red and unavailable T3/hosted rerun remain explicit. No status flip.
