#![forbid(unsafe_code)]

//! Story 20-7 AC4 — NFR-Perf-4: posture-shift propagation over the 1000-shift
//! corpus (FR16; `non-functional-requirements.md:12`): P99 <= 2 s, P99.9 <= 5 s.
//!
//! The corpus is `nfr_perf_4_1000_shift_propagation_corpus`'s
//! (`maos-kernel-core/tests`, Story 3.2 AC8): from each `shift_posture` call to
//! the instant a spawned task observes the new posture and its approval
//! decision through `evaluate_with_posture`. That test keeps its own functional
//! asserts; this bench carries the budgets, from `xtask/perf-budgets.toml`
//! (rows `nfr-perf-4-posture-shift-p99` / `-p999`).

use std::process::ExitCode;
use std::sync::Arc;
use std::time::Instant;

use maos_bench::budgets;
use maos_kernel_core::capability::cap_policy::decision::PolicyDecision;
use maos_kernel_core::capability::cap_policy::PolicyTable;
use maos_kernel_core::security::manifest::{EpistemicAction, EpistemicPolicySection, Posture};
use maos_kernel_core::security::posture::{ApprovalClass, PostureState};

const SHIFTS: usize = 1000;
const SPIRITS: u32 = 10;

fn seed_spirit(policy: &PolicyTable, pid: u32) {
    let mut inner = (*policy.inner().load_full()).clone();
    inner.spirit_postures.insert(
        pid,
        PostureState {
            current: Posture::Cautious,
            allowed_max: Posture::AutonomousWithHalt,
            epistemic_policy: EpistemicPolicySection {
                rules: vec![],
                default_action: EpistemicAction::VerbalizeOnly,
            },
        },
    );
    policy.update(inner);
}

fn quantile(sorted_ns: &[u64], per_mille: usize) -> f64 {
    let rank = (sorted_ns.len() * per_mille)
        .div_ceil(1000)
        .clamp(1, sorted_ns.len());
    sorted_ns[rank - 1] as f64 / 1_000.0
}

fn main() -> ExitCode {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    let policy = Arc::new(PolicyTable::new());
    for pid in 0..SPIRITS {
        seed_spirit(&policy, pid);
    }
    let postures = [
        Posture::Cautious,
        Posture::Assistive,
        Posture::AutonomousWithHalt,
    ];
    let mut latencies_ns: Vec<u64> = Vec::with_capacity(SHIFTS);
    for i in 0..SHIFTS {
        let pid = (i as u32) % SPIRITS;
        let mut next = postures[i % postures.len()];
        let current = policy
            .inner()
            .load_full()
            .spirit_postures
            .get(&pid)
            .map(|state| state.current);
        if current == Some(next) {
            next = postures[(i + 1) % postures.len()];
        }
        let observer = Arc::clone(&policy);
        let start = Instant::now();
        policy.shift_posture(pid, next).expect("shift posture");
        let observed = rt.block_on(async move {
            tokio::spawn(async move {
                let inner = observer.inner().load_full();
                let state = inner.spirit_postures.get(&pid).expect("seeded spirit");
                assert_eq!(state.current, next, "the shift propagated");
                let decision = observer.evaluate_with_posture(pid, ApprovalClass::Mutating);
                let allowed = matches!(decision, PolicyDecision::Allow);
                assert_eq!(
                    allowed,
                    matches!(next, Posture::AutonomousWithHalt),
                    "decision for {next:?} matches the posture matrix"
                );
            })
            .await
        });
        observed.expect("observer task");
        latencies_ns.push(start.elapsed().as_nanos() as u64);
    }
    latencies_ns.sort_unstable();
    let (p99, p999) = (quantile(&latencies_ns, 990), quantile(&latencies_ns, 999));
    eprintln!("posture-shift: {SHIFTS}-shift corpus P99={p99:.1}us P99.9={p999:.1}us");
    let mut within = budgets::report("nfr-perf-4-posture-shift-p99", p99);
    within &= budgets::report("nfr-perf-4-posture-shift-p999", p999);
    budgets::finish(within)
}
