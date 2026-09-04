# Epic 20 — W5: Multi-Host on Live Substrate (v1.5 → v2.2 reality)

**Status:** `backlog` — created 2026-09-04 by `sprint-change-proposal-2026-09-04.md` (bmad-correct-course, Lunarpulse-ratified). One of six recovery epics (15–20); `sprint-status.yaml` is authoritative for status.

**Wave / duration:** 4–6 weeks · **Makes true:** v1.5 J4 · v2.2 J3 + Reza — the mechanism estate exercised by the journeys it was built for

**Closes:** J3/J4/Reza fixture-by-default; FR23b, FR28 quota, FR29 consent scope; NFR-Sec-13 own-leaf, NFR-Rel-6 durable pins, NFR-Maint-1 instrument; register D1, D4a/b/c, D11, D13, D14; the 63 unregistered `MAOS_*` reads

**Exit command (the epic is done when an operator runs this on a clean machine and sees what it says):**

```
cargo run -p xtask -- demo-reza --live     # 3 teams, 3 regions, real DBs; RED if any is missing; zero ignored journey tests in the nightly
```

**Kernel-Δ:** **ZERO kernel-Δ @24472** for 20-1/20-2/20-3/20-5; **20-4 is expected NEGATIVE** (module moved out of kernel-core) and is measured at landing.

**Dev-gate:** external holds (pen-test NFR-Sec-7 + export counsel NFR-Comp-1) = GA ledger only, non-gating for dev. **Model/review discipline:** frontier-class dev allowlist + §A6 full-layer review net is the binding control (E11 retro A1). **Story rules (CP-2):** AC1 of every story is a command plus what the operator observes; a story splits once; story files ≤3,000 words; governance work ≤20% of the epic.

---

## Stories

| Key | Story | Closes |
|---|---|---|
| `20-1-j4-corpus-to-mira-live` | J4 corpus reaches Mira, live | J4; NFR-Test-4 per class |
| `20-2-j3-mesh-and-reza-in-nightly` | J3 mesh and Reza in the nightly | J3/Reza; NFR-Scale-5 |
| `20-3-durable-tofu-and-self-leaf-rotation` | Durable TOFU and self-leaf rotation | NFR-Sec-13/Rel-6; RELEASE-HOLDS rows 12, 14-2 (c.2)/(c.4)/(c.6) |
| `20-4-kernel-hygiene-one-instrument` | Kernel hygiene, one instrument | NFR-Maint-1; D11/D13 |
| `20-5-env-contract-registry-and-secrets` | Env-contract registry and secrets | D4a/D4b/D4c/D14; §A6 non-degradable (secret classification) |

Per-story ACs below are sketches; `bmad-create-story` finalizes them at preflight (≤6 ACs, AC1 unchanged in kind).

### 20-1-j4-corpus-to-mira-live — J4 corpus reaches Mira, live

*Closes:* J4; NFR-Test-4 per class

- **AC1 (command).** `maos run spirits/topologies/j4-mira-nash.toml --live --corpus tests/corpora/safety-critical-mira-nash-v1.5.jsonl` runs the 50-scenario corpus with a real model; the operator sees the honest close rate (target ≥45/50 ≤90 min, ≥48/50 consent envelope) instead of a κ number computed elsewhere.
- **AC2.** Mira and Nash reason through the Inference Port; manifests carry live model provenance instead of `maos.mira.deterministic-diagnostic-v1`.
- **AC3.** The consent-rupture oracle in `journey_j4.rs` is un-ignored and green.

### 20-2-j3-mesh-and-reza-in-nightly — J3 mesh and Reza in the nightly

*Closes:* J3/Reza; NFR-Scale-5

- **AC1 (command).** `journey-nightly.yml` runs the N=8 digest-read test and the three-team Reza journey against Postgres services and reds on absence (no banner).
- **AC2.** The J3 day-30 scene consumes the mesh's digest output, not `fixtures/j3/day-30-raw.json`.
- **AC3.** `check-reza-production-path` is Blocking with a substrate leg; zero `#[ignore]` journey tests remain in the default nightly.

### 20-3-durable-tofu-and-self-leaf-rotation — Durable TOFU and self-leaf rotation

*Closes:* NFR-Sec-13/Rel-6; RELEASE-HOLDS rows 12, 14-2 (c.2)/(c.4)/(c.6)

- **AC1 (command).** A daemon restart preserves pins (a persistence-backed `TofuPinStore`; today only `InMemoryTofuPinStore` exists, `tofu.rs:508`); a rotated peer is not silently reverted to the retired fingerprint.
- **AC2.** Own-leaf rotation is live (`swap_serving_cert` gains a production caller; 14-2d's story file is the design record).
- **AC3.** An ADR states either a CRL/OCSP mechanism or that revocation IS manifest reissue, and the §7.2.1.a `cert_post_grace_reject` token is machine-readable in the journal row.
- **AC4.** Listen-side per-peer scoping of client leaves.

### 20-4-kernel-hygiene-one-instrument — Kernel hygiene, one instrument

*Closes:* NFR-Maint-1; D11/D13

- **AC1 (command).** One kernel-size instrument (tokei code) and one ceiling in one ADR: the ≤25K "kernel-crate-set" ceiling is either authored (with the file it names) or retired; `kernel-core-baseline.toml` is valid TOML.
- **AC2.** The `orchestrator/` module leaves `maos-kernel-core` (§4.0.7 naming; `check-loom` scan stops needing an exemption).
- **AC3.** `maos-persistence` and the three phantom `kloc.toml` budgets are deleted; the 19 `canonical_*` encoders and 10 `SigningKey` helpers consolidate into one `maos-domain` module.
- **AC4.** Register D1, D11, D13 close on measurement.

### 20-5-env-contract-registry-and-secrets — Env-contract registry and secrets

*Closes:* D4a/D4b/D4c/D14; §A6 non-degradable (secret classification)

- **AC1 (command).** `check-env-contract` scans the whole workspace and is green with 0 unregistered reads (63 today across 147 distinct `MAOS_*` names).
- **AC2.** `EnvStability::Secret` exists; `MAOS_ANTHROPIC_API_KEY`, `MAOS_OPENAI_API_KEY`, `MAOS_AUDIT_KEY` and peers are classified (D4c); the 16-4 keyring path is the default source.
- **AC3.** `MAOS_REGION_HOME` is reconciled against the signed `TeamEntry.region` at daemon boot (D4a); D4b and D14 close.

## Dependencies

Epic 19 (19-4 evidence states, 19-2 packaging for the multi-host runbooks). 17-4 before 20-2.

## Not in this epic

Anything whose only deliverable is a gate, ledger, registry or ceiling and is not named above. v2.5 parking rows (`v25-*`) stay parked.
