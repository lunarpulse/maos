# Tracker notes archive — 2026-11-13 (Epic 21 audit, correct-course 2026-11-13)

Rows removed from `sprint-status.yaml` on 2026-11-13 by the Epic 21 audit of
`../planning-artifacts/sprint-change-proposal-2026-11-13.md` §2 (operator-approved 2026-11-13), applied on top of
`../planning-artifacts/sprint-change-proposal-2026-11-12.md` (its ruling block after §10 item 5, and §12). Each row is
copied verbatim from the tracker at tree `1a57035e` (original line number given). None was `done`; none is claimed
done. In each row's place the tracker keeps one `# MOVED` or `# DROPPED` comment line. Where a claim boundary applies,
it is stated in `RELEASE-HOLDS.md` and the GA notes (SCP 2026-11-13 §2, "Claim boundaries this creates at v1.0.0").

## 21-1-j4-incident-fixture-and-live-mira-nash

Tracker line 413, status `backlog` at removal. Moved and simplified to `22-8-always-on-scene`: AC2 (Mira and Nash inference seams) and AC3 (the in-process router) are kept; AC1's 50-incident cassette and the J4 closure rate are cut (claim boundary 4: the J4 closure rate is not measured).

```
21-1-j4-incident-fixture-and-live-mira-nash: backlog  # 50-incident fixture + prompt→AnomalySignal adapter + Mira/Nash inference seams + compressed clock; deny-path intake; κ corpus separate | RULED 2026-11-12 (decisions-2026-11-12-retro-preflight.md): cassette source D-21-PF-05 C (Ollama recording when available, else agent-authored, J4 closure NOT MEASURED), provenance label `seed` (PF-06 A)
```

## 21-2-nightly-postgres-and-ignored-journeys

Tracker line 414, status `backlog` at removal. Dropped. Its Postgres legs (21-6 collective tier, 21-7 consent on the live substrate, the PG migration of the consent side tables, the nightly Reza scene) are parked under `v25-reza-postgres-legs` (claim boundary 3).

```
21-2-nightly-postgres-and-ignored-journeys: backlog  # provision-loom-substrate in nightly; run ignored 13_6b + t_12_4a with --run-ignored; capture test un-ignored; reza leg Blocking
```

## 21-8-three-protocol-session-witness

Tracker line 420, status `backlog` at removal. Not GA. Moved to the Epic 23 exit `interop_session_23`, created by `23-5-a2a-v1-agent-card-and-jsonrpc-binding` (claim boundary 1: ACP, MCP and A2A are each proven; the one-session witness is not claimed until Epic 23).

```
21-8-three-protocol-session-witness: backlog  # ⚠ ADDED 2026-10-22 (routing audit §1): PRD success-criteria.md:50 — "≥1 MAOS Host launched by Zed (ACP), pulling tools from a public MCP server (MCP), and peering with another Host (A2A) — all in one session — by v1.0". ACP (5-5c), MCP (8-14b/c) and A2A (8-6) each exist; no story owns the combined binary-level witness. Hermetic: fixture ACP client + fixture MCP server + loopback A2A peer; the public-server leg is operator lane. Kernel-Δ 0. (ordering edge to 21-2 dropped by ruling, decisions-2026-11-12-retro-preflight.md)
```

## 21-11-agent-authoring-kit-and-agent-author-trial

Tracker line 423, status `backlog` at removal. Moved and simplified to `22-7-domain-spirit-kit` (kit, a working `maos-spirit validate`, one domain Spirit example, three recorded agent runs); D-21-PF-21 and D-21-PF-22 are superseded; the human-denominator legs of the trial gate are removed, not extended.

```
21-11-agent-authoring-kit-and-agent-author-trial: backlog  # ADDED 2026-11-12 by operator ruling authors = agents-first: "I want the agents to generate from the instructions for agents. like hermes agent or pi agent knows how they extend themselves using the docs and instructions and skills internally available to their agent harness system." Deliverable: an agent-facing authoring kit shipped with the product (instruction file + skill, built on templates/spirit-ts, templates/spirit-rust and the docs-site cookbook) from which an agent harness with no MAOS source knowledge generates, builds, signs and loads a working Spirit; the trial is a blank-context agent given only the published kit, recorded with honest N and author kind `agent`. The 12-human-author denominator (xtask/src/check_third_party_trial.rs:325; provisioning-checklist row `external third-party authors`) is re-ruled here: agent-authored trials count under their own label, human authors are optional and never block. Depends on 21-9 for the WASM form. Not a GA precondition unless the Epic-20 retrospective says so. §-section at the Epic-20 retrospective preflight. | RULED 2026-11-12 (decisions-2026-11-12-retro-preflight.md): not a GA precondition (PF-04 A); one gate with author_kind (PF-21 A); N=12 across >=3 harnesses/models (PF-22 A); implement maos-spirit validate (PF-23 A)
```

## Comments replaced on rows that stay

Six Epic 21 rows stay in the tracker with their status unchanged. Their comments were rewritten on 2026-11-13 to state
the disposition first and keep at most one sentence of the old text. The full comment before the rewrite is kept here
verbatim (tracker at tree `1a57035e`, original line number given).

### 21-3-durable-tofu-and-self-leaf-rotation

Tracker line 415, status `backlog`, comment before the 2026-11-13 rewrite:

```
21-3-durable-tofu-and-self-leaf-rotation: backlog  # durable TofuPinStore in maos-persistence (kept); sync verify path; own-leaf caller; T_grace cold+I with both 14-2a legs re-derived (14-2d record); typed post-grace token
```

### 21-4-one-instrument-and-env-registry

Tracker line 416, status `backlog`, comment before the 2026-11-13 rewrite:

```
21-4-one-instrument-and-env-registry: backlog  # tokei-code instrument; valid baseline TOML; kernel-crate-set.toml authored or citation retired; workspace env scan; registry → maos-domain; Secret class; REGION_HOME boot reconciliation (D4a/b/c, D11, D13, D14). ⚠ FILED BY 15-3 (AC7(d)) WITH D13(b), NOT FIXED: **nothing asserts the `-e tests` exclusion glob**, and it is the load-bearing premise of every ceiling in this repo — a tokei major bump that changed the glob's meaning would silently add ~11k charged lines in `xtask` alone. `kloc_check`'s `TOKEI_VERSION` pin is a PIN, not a property test: it detects a version change, not a semantics change. 15-3 confirmed the instrument is consistent (tokei charges Rust `///`/`//!` doc comments as CODE, matching every §9 anchor), which is exactly why the glob's behaviour deserves an assertion rather than trust. Instrument work lands with 21-4's unification per 15-2's precedent. *** FILED BY 15-1 (2026-09-08, rule 9): (1) R-5 - spawn_and_invoke (crates/maos-mcp/src/transport/stdio.rs) treats write-side EPIPE as FATAL even when the child already produced a complete answer; mechanism reproduced 300/300 outside the repo with a 2 ms scheduling delay standing in for load (fixture `sh -c "echo ..."` never reads stdin; the request write returns EPIPE, error string "write: Broken pipe (os error 32)", no "parse"); the fixture race is fixed by 15-1 AC6(c) but the transport robustness question stands for one-shot NDJSON servers; (2) the J1 `_2c.rs` blind spot - `_2c` is not in J1_TEST_SUFFIXES and all six j1-crosshost-2c tests live outside crates/maos-bin/tests, so derive_enrolled_targets cannot see the 2c lane at all.
```

### 21-6-memory-tier-quota-enforcement

Tracker line 418, status `backlog`, comment before the 2026-11-13 rewrite:

```
21-6-memory-tier-quota-enforcement: backlog  # ⚠ ADDED 2026-10-22 by operator ruling D-17-9-PF-R3 A (17-9 preflight; epic-21 §21-6): FR28 (prd/functional-requirements.md:74) — "kernel provides persistence and quota enforcement" for private/shared/collective memory tiers — was routed to 21-4 (requirements-inventory.md:556), whose ACs are the instrument and env registry; no memory-tier quota exists in code. A DIFFERENT UNIT from 17-9's capability-issuance window and 18-5's context tokens. Placed in Epic 21 (live substrate), where the recovery map already put FR28. ◆ FLAG-Winston: kernel figure measured by prototype at creation (E16-A7).
```

### 21-7-production-consent-granter-and-gate

Tracker line 419, status `backlog`, comment before the 2026-11-13 rewrite:

```
21-7-production-consent-granter-and-gate: backlog  # ⚠ ADDED 2026-10-22 (routing audit §1): FR29's production half — 17-8 labels honestly (rule 11) that "no production granter sets HeaderOnly/Withheld, no ConsentGate is installed in production" (17-8…md:114) and native senders journal pid 0 (:45); every consent vector is proven on suite-delivered frames. J3/Reza consent must be real on the live substrate. Kernel-Δ 0 (granter + gate are maos-bin). (ordering edge to 21-2 dropped by ruling, decisions-2026-11-12-retro-preflight.md)
```

### 21-9-wasm-host-default-on

Tracker line 421, status `backlog`, comment before the 2026-11-13 rewrite:

```
21-9-wasm-host-default-on: backlog  # ADDED 2026-11-12 by operator ruling wasm = on (decisions-2026-11-12-dedrift.md): the WASM-component Spirit form (ADR-060, the third-party form) ships in the default artifact. Enforcement points measured @23b2200b, all export-control precaution and nothing technical: crates/maos-bin/Cargo.toml:15 `default = ["network"]` and :33 `wasm-host`; xtask/src/check_export_control.rs:194,310 `check_wasm_host_absent_from_default` (retire or invert); xtask/tests/decision_adrs_and_provisioning.rs clauses (g) STABILITY.md PRESERVED:export fence and (h) README `--features wasm-host`; docs/compliance/export-counsel-precondition.md; RELEASE-HOLDS.md "Standing mitigations". Also owed: `maos-wasm-runner` ships beside `maos` (absent from packaging/, release.yml and Dockerfile today); the release size bound 35,651,584 B (xtask/src/prepush.rs:509) is re-measured with the engine in; the duplicate default / wasm-host CI builds collapse. NFR-Comp-1 stays NOT claimed (no ECCN determination was made); the release notes say the default artifact carries the wasmtime engine. Kernel-Δ expected 0. `v1.0.0` GA precondition (ga3). §-section at the Epic-20 retrospective preflight. | RULED 2026-11-12 (decisions-2026-11-12-retro-preflight.md): stays in the GA set (PF-01 A); inherited rows 25 ACs / 13 justified (PF-11 A); invert check-export-control (PF-08 A); re-rule Epic 20 exit line 9 (PF-09 A); re-measure size bound at landing (PF-10 A); PF-12..17 = B,A,A,A,A,B; D-E20-10 release.yml GA perf-ABSENT change is owned by ops-ga-tag-after-holds
```

### 21-10-invariant-ledger-proofs

Tracker line 422, status `backlog`, comment before the 2026-11-13 rewrite:

```
21-10-invariant-ledger-proofs: backlog  # ADDED 2026-11-12 by operator ruling ga3: PRD success-criteria.md "All 14 kernel invariants (I1–I14) empirically verified by v1.0 … No predicate, no claim". Measured @23b2200b: docs/invariants/I1..I14.md all read `claim: asserted`; `property_test: null` in 13 of 14 (I5 names principal_identity_20_3b.rs); 20-3b closed with its AC2 coverage generator not built (NO-GO). Deliverable: per invariant one falsifiable predicate, one property test and one adversarial vector that run in CI; the ledger flips to `guaranteed` only on that evidence; existing tests are linked before new ones are written. Ledger owners today: 20-3b (12 rows), 20-1b (I5), 20-9 (I14). `v1.0.0` GA precondition (ga3). §-section at the Epic-20 retrospective preflight. | RULED 2026-11-12 (decisions-2026-11-12-retro-preflight.md): owns a production caller for the kernel T3 admission arm (D-E20-1 A); edits docs/invariants under invariant-lock (PF-18 A); hand-rolled generators + fuzz targets for I1/I8/I11 (PF-19 C); all 14 invariants proven for GA (PF-20 A); LIVE = built-binary tests only (D-E20-8 A)
```
