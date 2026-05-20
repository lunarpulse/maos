# Cross-Spirit Isolation Corpus v0 (scripted-v0)

**Story 4.5** — NFR-Sec-14 enforcement substrate for the v1.0 hermes-tenant
positioning sentence: "Spirit-A cannot observe Spirit-B's state under any of 200
adversarial scenarios."

## Tier

`scripted-v0` — deterministic seed-driven generation (seed `0x150C04A5`).
Per-category scenarios carry `tier_tag: "scripted-v0"` and are reviewer-attested
via `category-attestation.json` per the Epic 2 retro A2 methodology gate closure.

**v1.0 promotion plan** (`handauthored-v1`): ≥2 attestors per category, ≥10
hand-authored scenarios per category (≥80 hand-authored per split, ≥160 total).
Owned by Story 10.2 (third-party adversarial red-team gate at v1.0).

## Threat Model

Architecture §8.1 + ADR-040 split:
- **Sec-14a** (100 scenarios, same-Host): Spirit-A attempts to enumerate, read,
  side-channel, or timing-attack Spirit-B's substrate state through local kernel
  surfaces (`MemoryManagerAdapter::read`, `LogRecallAdapter::recall`, etc.).
- **Sec-14b** (100 scenarios, cross-Host): same attack categories but through
  the A2A (Agent-to-Agent) mTLS replay injection surface. At v0.3-β, Sec-14b
  runs structurally (kernel rejects cross-Host with `CrossHostUnsupported`);
  Story 6.3 wires the real mTLS envelope enforcement at v0.5+.

## Eight Attack Categories (≥25 scenarios per category aggregated)

1. **namespace_enumeration** — Cross-namespace PID spoofing, principal index
   enumeration via timing.
2. **working_memory_read_across** — Cross-PID tagged-scalar read, slot
   subscription as peer Spirit.
3. **decision_frame_observation** — LogRecallAdapter recall with Spirit-B's
   DecisionDispatch filter.
4. **halt_signal_observation** — Recall with EpistemicHalt filter, HaltRegistry
   pending_halt_ids enumeration, scalar.tap subscriber observing pre-halt drift.
5. **transparency_log_cross_read** — Cross-PID query_frames, frame fetch of
   peer-owned frame_id.
6. **working_memory_digest_cross_read** — DistillateWriter::admit_for_consumer
   with forged consumer_allowed_promotion_set, cross-PID distillate fetch.
7. **capability_token_forgery_cross_spirit** — Token byte replay with PID
   mismatch, token expiry, signature forgery.
8. **sandbox_escape_lateral** — Tier-T2 namespace escape, cgroup-v2 hierarchy
   attribution, shared inode crossing.

## Authoring Methodology

Scripted generation with `cargo xtask gen-isolation-corpus --seed 0x150C04A5`.
Each scenario carries deterministic attack_payload parameters keyed by category
and scenario index. Per-category reviewer attestation (`category-attestation.json`)
mirrors Story 4.4's IAA attestation pattern (Epic 2 retro A2 closure).

The generator is a one-shot dev tool. Generated artifacts are committed as-is
and are bit-stable across CI runs. CI does NOT regenerate the corpus.

## Directory Layout

```
isolation-corpus-v0/
├── README.md
├── methodology-attestation.json
├── sec-14a/
│   ├── namespace_enumeration/        (13 scenarios)
│   ├── working_memory_read_across/   (13 scenarios)
│   ├── decision_frame_observation/   (12 scenarios)
│   ├── halt_signal_observation/      (13 scenarios)
│   ├── transparency_log_cross_read/  (12 scenarios)
│   ├── working_memory_digest_cross_read/ (13 scenarios)
│   ├── capability_token_forgery_cross_spirit/ (12 scenarios)
│   └── sandbox_escape_lateral/       (12 scenarios)
└── sec-14b/                          (same 8 categories, complementary distribution:
                                       12/12/13/12/13/12/13/13 = 100)
```

Each category subdirectory contains `scenario-NNN.json` files plus a
`category-attestation.json` with per-attestor sign-off.
