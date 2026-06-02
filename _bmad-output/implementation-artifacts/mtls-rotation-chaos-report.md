---
dev_model_used: claude-opus-4-7
---

# mTLS Rotation Chaos — Calibration Reports (v0.5)

Per architecture §7.2.1.b — at v0.5 the harness operates in **calibration
mode**: all three timing-floor distributions are measured and reported but
NOT enforced. v0.7 flips revocation propagation + re-handshake to hard-fail;
v1.0 adds `cert_post_grace_reject` ≤0.1% enforcement.

Story 6.3 ships the harness scaffolding (`crates/maos-a2a/src/chaos/`) +
discipline.yml job `nfr-sec-13-mtls-rotation-chaos-3-host` (deferred to
schedule:weekly in follow-up). Production drills run quarterly per §7.2.1
"Quarterly, on calendar (not opportunistic)".

## Sample drill — happy-path 3-host baseline (synthetic)

_Drift baseline established 2026-05-26 against `DrillConfig::default()` —
3 agents, p99_handshake_rtt = 500ms (cold-deployment floor), days_of_history
= 7, target propagation [10s, 15s, 20s], target re-handshake [5s, 8s, 12s]._

```json
{
  "drill_id": "harness-3-host-default",
  "host_count": 3,
  "p99_handshake_rtt_ms": 500,
  "t_grace_ms": 5000,
  "per_agent": [
    {"agent_id": "agent-0", "t_0_ns": 0, "t_1_ns": 10000000000, "t_2_ns": 15000000000},
    {"agent_id": "agent-1", "t_0_ns": 0, "t_1_ns": 15000000000, "t_2_ns": 23000000000},
    {"agent_id": "agent-2", "t_0_ns": 0, "t_1_ns": 20000000000, "t_2_ns": 32000000000}
  ],
  "revocation_propagation_p50_ms": 15000,
  "revocation_propagation_p99_ms": 20000,
  "re_handshake_p50_ms": 8000,
  "re_handshake_p99_ms": 12000,
  "end_to_end_p50_ms": 23000,
  "end_to_end_p99_ms": 32000,
  "post_grace_reject_rate": 0.0,
  "passes_v07_floors": true,
  "passes_v10_floors": true
}
```

**Calibration verdict (v0.5):** baseline passes v0.7 + v1.0 floors at the
synthetic 3-host scale. Production rotation drills against real OCSP poll
infrastructure follow once §7.2 cert-issuance tooling ships (Epic 7+).
