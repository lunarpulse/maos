---
story_id: '20.1a'
story_key: 20-1a-registry-publish-install-exit-path
created: 2026-11-01
context_commit: a4c00897
baseline_commit: a4c00897
security_sensitive: true
kernel_grant: "none for 20-1a: kernel-free (AC1–AC5). The FLAG-Winston row R (D-20-1-02 A KQ1 + D-20-1-03 A P; measured +78 physical / +68 tokei from HEAD 26801 / 21011; ceilings kernel ≤ 26957 / 21147) is granted to 20-1b only, which lands last; the interaction with 20-7's row R is recorded in 20-1b's kernel_grant (cumulative tokei ≤ 21209, else STOP)."
kloc_grant: "RULED 2026-11-01/02 (ask rows201; D-20-1-11 A; D02 A ×2 measured / ×3 unmeasured). One-key ceilings (STOP above; aggregate hardfail = measured + 1 at landing): maos-registry ≤ 5163; maos-spirit-cli ≤ 2043; maos-cli ≤ 6666 (above its 6382 STOP; inside the reserved 6856 pin, D-20-1-05 A); maos-bin ≤ 34410; maos-shell ≤ 728; maos-compliance ≤ 2122; maos-domain ≤ 10153; xtask ≤ 42920; aggregate ≤ 193768. The kernel ceiling 26957 / 21147 is 20-1b's (see kernel_grant)."
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

### Story context creation

Created 2026-11-01 by `anthropic/claude-opus-5-5` (bmad-create-story, lighter measured contract) at `a4c00897`, without cargo (parent rule).

**Inputs:** E20 §20-1 at HEAD (amended 2026-10-30, preflight §3.1) — stale cites routed through records-owed; preflight §1–§5 and D03/D04/D09/D12; tracker `sprint-status.yaml:296` (D9, R10, D8, R8 notes); every `deferred-work.md` row naming 20-1; routing audit rows; direct reads of every cited range (`creation/cites-a4c00897.md`); kloc and kernel by the prebuilt gate binary on `/tmp/proto201` `git archive` copies (`creation/kloc-kernel-a4c00897-and-prototypes.json`); prototypes fmt0, KQ0, KQ1, KQ2, P, R, R′, Rmin, B, S1 saved as diffs, `rustfmt`'d, never applied to the tree; exit lines measured on the prebuilt HEAD binaries (`creation/runtime/`).

**Measurements owed:** `creation/measurements.md` "Not measured here" 1–7.

**Records owed:** `creation/records-owed.md`.

**Revert:** no crate source modified (prototypes only under `/tmp/proto201`, deleted); no cargo/test ran; `intent-lineage-coverage-report.md` untouched.

**Gates after the record edits** (prebuilt `/mnt/build/cargo-maos/debug/xtask`, `env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=…/xtask`, `--json`, in-tree; `creation/gates/*.json`): `check-dev-record-completeness` rc 0 (197 done stories, 150 deferred-owner assertions, 0 violations); `check-epic-close-coherence` rc 0 (21 epics, pin 26801, 0 violations); `kloc-check` rc 0 (aggregate 188816, `alarm=true`, `over_budget=[]`); `check-kernel-baseline` rc 0 (26801 / 95 files, set hash `c023d0d7…`, no drift).

### Agent Model Used

anthropic/claude-opus-5-5 (creation).

### Debug Log References

- Creation evidence: `20-1-registry-client-install-verb-vetter-and-yank-evidence/creation/{measurements.md, cites-a4c00897.md, records-owed.md, kloc-kernel-a4c00897-and-prototypes.json, fmt0-applier-format-control.diff, kq0-fence-only.diff, kq1-sticky-quarantine.diff, kq2-sticky-quarantine-plus-iac-egress.diff, p-poller-edge-trigger.diff, krec-kq1-p-bundle.diff, kalt-kq2-p-bundle.diff, kmin-kq0-p-bundle.diff, b-nonkernel-yank-quarantine.diff, s1-registry-server-binary.diff, runtime/{exit-probe.sh, exit-probe-a4c00897.txt}, gates/*}`.

### Completion Notes List

### File List

- Creation: this story file; `20-1-…-evidence/creation/*`; `sprint-status.yaml:296`.

### Change Log

- 2026-11-01: Story created at `a4c00897` (ready-for-dev; D09, D12 and D-20-1-01…11 PROPOSED). Inventory measured: exit lines 2/4 runnable, 5 rc 1, 7 rc 1, 8 rc 2, 9 `unknown spirit`; the daemon registry arm functional and its `registry.deprecate` unauthenticated; yank stamps process-relative and the cursor unwired; FR13 quarantine terminates. Kernel priced by prototype (R = KQ1 + P +78 / +68 incl. +40 / +40 rustfmt churn; R′ +79 / +69 + iac/domain +26; Rmin +53 / +49; non-kernel B maos-bin +43, FR13 still false); S1 maos-registry +38; total forecast +810…+1,686; envelope and operator-ruling list filed; split proposed (D-20-1-01 B).
- 2026-11-01: Split into 20-1a/20-1b under D-20-1-01 B (ask split201); rulings 2026-11-01/02 (ask rows201) recorded in Decisions. The 20-1 sprint key is retired; kernel grant R (KQ1 + P, +78 / +68) is held by 20-1b only.
