//! AC4 / AC5 — structural-anomaly suspects detected Spirit-side and surfaced via
//! the REAL `NotificationDispatcher`; the interpretation of malice is never
//! kernel-side (§4.0.7).
//!
//! The three NFR-Sec-3 divergence inputs (syscall-pattern divergence, fd-table
//! growth, unexpected outbound IAC) are fixture-replayed at v0.5 (NFR-Sec-3 is
//! v2.0/ADR-024), shaped as the real `FrameKind::SandboxBlock = 8` discriminator
//! (Decision E). Observer classifies them and emits a `structural_anomaly_suspect`
//! as a `NotificationEvent::AnomalyFlagged` (Decision B). Fixture-driven from
//! `tests/fixtures/structural-scenarios.json`.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use serde::Deserialize;

use maos_director_surface::notification::{
    NotificationChannel, NotificationDispatcher, NotificationError, NotificationSurface,
};
use maos_domain::notification::{NotificationEvent, NotificationLevel};
use maos_spirit_abi::identity::FrameKind;

use observer::{Observer, PrincipalScope, StructuralSignal, SANDBOX_BLOCK_FRAME_KIND};

struct CapturingChannel {
    sink: Arc<Mutex<Vec<NotificationEvent>>>,
}

impl NotificationChannel for CapturingChannel {
    fn surface(&self) -> NotificationSurface {
        NotificationSurface::Terminal
    }
    fn dispatch(
        &self,
        event: &NotificationEvent,
        _level: NotificationLevel,
    ) -> Result<(), NotificationError> {
        self.sink.lock().unwrap().push(event.clone());
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StructuralScenario {
    name: String,
    #[serde(default)]
    #[allow(dead_code)]
    note: String,
    namespace: Vec<String>,
    signal: StructuralSignal,
    expect_suspect: bool,
}

fn load_scenarios() -> Vec<StructuralScenario> {
    let raw = include_str!("fixtures/structural-scenarios.json");
    serde_json::from_str(raw).expect("structural-scenarios.json parses")
}

#[test]
fn fixture_discriminator_is_bound_to_the_real_abi() {
    // Decision E — the fixtures are shaped as the REAL SandboxBlock discriminator.
    assert_eq!(SANDBOX_BLOCK_FRAME_KIND, FrameKind::SandboxBlock as u8);
    for sc in load_scenarios() {
        assert_eq!(
            sc.signal.frame_kind, SANDBOX_BLOCK_FRAME_KIND,
            "{}: structural fixture rides on FrameKind::SandboxBlock",
            sc.name
        );
    }
}

#[test]
fn structural_suspects_surface_via_the_real_dispatcher_kinds_covered() {
    let scenarios = load_scenarios();

    let captured = Arc::new(Mutex::new(Vec::new()));
    let mut dispatcher = NotificationDispatcher::new();
    dispatcher.register(Box::new(CapturingChannel {
        sink: Arc::clone(&captured),
    }));

    let mut kinds_surfaced: BTreeSet<String> = BTreeSet::new();

    for sc in &scenarios {
        // Observer with no drift watches — purely the structural path here.
        let observer =
            Observer::watching(PrincipalScope::from_patterns(sc.namespace.clone()), vec![]);
        let surface = observer.classify_signal(&sc.signal);

        assert_eq!(
            surface.is_some(),
            sc.expect_suspect,
            "{}: expected suspect={}, got {}",
            sc.name,
            sc.expect_suspect,
            surface.is_some()
        );

        if let Some(s) = surface {
            // §4.0.7 — the interpretation (the suspect verdict, the confidence) is
            // Spirit-side; the summary carries the classification + divergence kind.
            assert!(
                s.summary.contains("structural_anomaly_suspect"),
                "{}: summary names the suspect classification",
                sc.name
            );
            let note = s
                .to_notification(observer.observer_id())
                .expect("valid anomaly notification");
            let report = dispatcher
                .dispatch(note, NotificationLevel::Immediate)
                .expect("dispatch");
            assert_eq!(report.delivered, 1, "{}: surfaced to the operator", sc.name);
            // serde kebab tag of the divergence kind, for coverage tracking.
            let kind_tag = serde_json::to_value(&sc.signal.kind)
                .unwrap()
                .as_str()
                .unwrap()
                .to_string();
            kinds_surfaced.insert(kind_tag);
        }
    }

    // All three NFR-Sec-3 divergence kinds are exercised as real suspects.
    for kind in [
        "syscall-pattern-divergence",
        "fd-table-growth",
        "unexpected-outbound-iac",
    ] {
        assert!(
            kinds_surfaced.contains(kind),
            "divergence kind '{kind}' must be surfaced as a suspect; surfaced: {kinds_surfaced:?}"
        );
    }

    // Every surfaced event is the AnomalyFlagged variant (Decision B; ABI frozen).
    for ev in captured.lock().unwrap().iter() {
        assert!(matches!(ev, NotificationEvent::AnomalyFlagged { .. }));
    }
}
