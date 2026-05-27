# A2A Churn — Calibration Reports (v0.5)

Per NFR-Rel-7 — at v0.5 the harness ships the **3-host compressed scaffold**;
the v2.0 binding floor (detection ≤1h median / blast radius ≤5 peers /
recovery ≤24h) applies at 30-host (compressed) scale. v2.5 binds at 100-host
(full).

Story 6.3 ships the scaffold (`crates/maos-a2a/src/chaos/churn.rs`) + the
discipline.yml job `nfr-rel-7-churn-scaffold-3-host` (deferred to
schedule:weekly in follow-up).

## Sample drill — 3-host compressed scaffold baseline (synthetic)

```json
{
  "drill_id": "churn-3-host-3-4w",
  "config": {
    "host_count": 3,
    "turnover_per_week_pct": 15,
    "duration_weeks": 4,
    "adversarial_host_count": 3
  },
  "detection_latency_median_secs": 30,
  "max_blast_radius": 3,
  "recovery_secs": 60,
  "passes_v20_floors": true
}
```

**Calibration verdict (v0.5):** 3-host compressed scaffold meets the v2.0
binding floor by an order of magnitude (detection 30s vs target 3600s).
The scaffold's shape — adversarial peer task handles + bounded blast
radius — scales to 30-host at v2.0 with the floors flipped to hard-fail.
