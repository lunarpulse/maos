//! NFR-Perf-4 posture-shift propagation latency proof (Story 3.2, AC8).
//!
//! 1000-shift corpus: P99 ≤ 2s, P99.9 ≤ 5s.
//! Measures the time from `shift_posture` call to when a spawned task
//! observes the new posture via `evaluate_with_posture`.

use std::sync::Arc;

use maos_kernel_core::capability::cap_policy::decision::PolicyDecision;
use maos_kernel_core::capability::cap_policy::PolicyTable;
use maos_kernel_core::security::manifest::{EpistemicAction, EpistemicPolicySection, Posture};
use maos_kernel_core::security::posture::{ApprovalClass, PostureState};

fn seed_spirit(policy: &PolicyTable, pid: u32, posture: Posture, allowed_max: Posture) {
    let mut inner = (*policy.inner().load_full()).clone();
    inner.spirit_postures.insert(
        pid,
        PostureState {
            current: posture,
            allowed_max,
            epistemic_policy: EpistemicPolicySection {
                rules: vec![],
                default_action: EpistemicAction::VerbalizeOnly,
            },
        },
    );
    policy.update(inner);
}

#[tokio::test(flavor = "multi_thread")]
async fn nfr_perf_4_1000_shift_propagation_corpus() {
    let policy = Arc::new(PolicyTable::new());

    let num_spirits: u32 = 10;
    for pid in 0..num_spirits {
        seed_spirit(&policy, pid, Posture::Cautious, Posture::AutonomousWithHalt);
    }

    let mut latencies_ns: Vec<u64> = Vec::with_capacity(1000);
    let postures = [
        Posture::Cautious,
        Posture::Assistive,
        Posture::AutonomousWithHalt,
    ];

    for i in 0..1000 {
        let pid = (i as u32) % num_spirits;
        let new_posture = postures[(i as usize) % postures.len()];
        // Ensure we don't request the same posture (would be a no-op shift)
        let new_posture = if policy
            .inner()
            .load_full()
            .spirit_postures
            .get(&pid)
            .map(|s| s.current == new_posture)
            .unwrap_or(false)
        {
            postures[((i as usize) + 1) % postures.len()]
        } else {
            new_posture
        };

        let policy_clone = Arc::clone(&policy);
        let t0 = std::time::Instant::now();

        let new_hash = policy.shift_posture(pid, new_posture).unwrap();

        // Spawn a task to observe the propagated posture via evaluate_with_posture
        let observed = tokio::spawn(async move {
            let inner = policy_clone.inner().load_full();
            // Verify the PostureState reflects the new posture
            let state = inner.spirit_postures.get(&pid).unwrap();
            assert_eq!(state.current, new_posture);
            // Verify evaluate_with_posture returns the expected decision
            let decision = policy_clone.evaluate_with_posture(pid, ApprovalClass::Mutating);
            let expected = if matches!(new_posture, Posture::AutonomousWithHalt) {
                PolicyDecision::Allow
            } else {
                PolicyDecision::RequireApproval {
                    class:
                        maos_kernel_core::capability::cap_policy::decision::ApprovalClass::Mutating,
                }
            };
            assert_eq!(
                std::mem::discriminant(&decision),
                std::mem::discriminant(&expected),
                "decision for (posture={:?}, Mutating) should match matrix",
                new_posture
            );
            let _ = new_hash;
        })
        .await
        .unwrap();

        let t1 = std::time::Instant::now();
        let duration_ns = (t1 - t0).as_nanos() as u64;
        latencies_ns.push(duration_ns);
        let _ = observed;
    }

    // Compute percentiles
    latencies_ns.sort_unstable();
    assert!(!latencies_ns.is_empty(), "histogram must not be empty");

    // Integer percentile: P_n index = (len * n + 99) / 100 - 1 (0-indexed)
    let len = latencies_ns.len();
    let p50_idx = (len * 50 + 99) / 100 - 1;
    let p99_idx = (len * 99 + 99) / 100 - 1;
    let p999_idx = (len * 999 + 999) / 1000 - 1;

    let p50 = latencies_ns[p50_idx.min(len - 1)];
    let p99 = latencies_ns[p99_idx.min(len - 1)];
    let p999 = latencies_ns[p999_idx.min(len - 1)];

    eprintln!(
        "NFR-Perf-4: {}-shift corpus, P50={:.3}ms P99={:.3}ms P99.9={:.3}ms",
        latencies_ns.len(),
        p50 as f64 / 1_000_000.0,
        p99 as f64 / 1_000_000.0,
        p999 as f64 / 1_000_000.0,
    );

    // P50 should be sub-millisecond (CoW swap is ~ns, cross-task is ~µs)
    assert!(
        p50 <= 1_000_000,
        "P50 {} ns exceeds 1ms stretch assertion",
        p50
    );

    assert!(
        p99 <= 2_000_000_000,
        "P99 {} ns exceeds 2s NFR-Perf-4 floor",
        p99
    );

    assert!(
        p999 <= 5_000_000_000,
        "P99.9 {} ns exceeds 5s NFR-Perf-4 floor",
        p999
    );
}
