#![forbid(unsafe_code)]

//! `Mailbox::install_a2a_router` — the set-once cross-host router installer
//! (story `j1-crosshost-1a`, AC2.1) and its fail-closed negative (AC2.4-2.5).
//!
//! An integration test, not an in-`src` `#[cfg(test)]` module: in-`src` tests are
//! charged to `maos-iac`'s KLOC budget and are never executed by CI. This file
//! costs zero budget (`xtask/src/kloc_check.rs` excludes `tests/`) and runs.

use std::sync::Arc;

use async_trait::async_trait;
use maos_domain::frame::{
    FrameAddress, FramePayload, IacFrame, PosturePreferences, TaskAssignPayload,
};
use maos_domain::iac_bus_types::IacBusError;
use maos_domain::invariants::i1::IntentClass;
use maos_domain::invariants::i13::IntentLineage;
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i8::A2AIntent;
use maos_domain::ports::a2a::A2ARouter;
use maos_iac::adapter::mailbox::Mailbox;
use maos_iac::adapter::metrics::IacRtMetrics;
use maos_spirit_abi::identity::{FrameKind, HostId, SpiritId};

#[derive(Default)]
struct CountingRouter {
    calls: tokio::sync::Mutex<Vec<String>>,
}

#[async_trait]
impl A2ARouter for CountingRouter {
    async fn route_outbound(&self, _frame: IacFrame, peer: &HostId) -> Result<(), IacBusError> {
        self.calls.lock().await.push(peer.as_str().to_string());
        Ok(())
    }
}

fn task_assign_frame(to: &str, host: Option<&str>) -> IacFrame {
    let mut addrs = smallvec::SmallVec::<[FrameAddress; 1]>::new();
    addrs.push(FrameAddress {
        spirit_id: SpiritId::from(to),
        host_id: host.map(|h| HostId(h.to_string())),
        role: None,
    });
    IacFrame {
        frame_id: [9u8; 16],
        timestamp_ns: 1,
        logical_clock: 1,
        from: FrameAddress {
            spirit_id: SpiritId::from("sender"),
            host_id: None,
            role: None,
        },
        to: addrs,
        kind: FrameKind::TaskAssign,
        intent: IntentClass::Standard,
        payload: FramePayload::TaskAssign(TaskAssignPayload {
            goal: "delegated".into(),
            scope: vec![],
            success_criteria: "exit 0".into(),
            posture_preferences: PosturePreferences::default(),
            prior_distillate_ref: None,
        }),
        auto_marker: FrameOrigin::SpiritAuto,
        consent_envelope: None,
        intent_lineage: IntentLineage::new(vec![A2AIntent::new("development-task:write-workspace")]),
    }
}

/// AC2.1 — the installer works on an already-`Arc`'d mailbox (which the builder
/// `with_a2a_router` cannot reach because it consumes `self`), and it is
/// **set-once**: a second install is REFUSED so nothing can swap the cross-host
/// router after boot. Set-once is the security property, not the ergonomic.
#[tokio::test]
async fn install_a2a_router_is_set_once_and_routes_from_an_arc() {
    maos_capability::cap_tokens::init_monotonic_base();

    let first = Arc::new(CountingRouter::default());
    let second = Arc::new(CountingRouter::default());
    // The production shape: Arc FIRST, router installed afterwards.
    let mailbox = Arc::new(Mailbox::new(Arc::new(IacRtMetrics::new())));
    assert!(mailbox.install_a2a_router(first.clone()).is_ok());
    assert!(
        mailbox.install_a2a_router(second.clone()).is_err(),
        "set-once: a second install must be REFUSED, never swap the router after boot"
    );

    let _handle = mailbox.register_spirit("test-spirit").unwrap();
    mailbox
        .deliver(task_assign_frame("test-spirit", Some("remote-peer")))
        .await
        .expect("routes through the installed router");

    assert_eq!(first.calls.lock().await.as_slice(), ["remote-peer"]);
    assert!(
        second.calls.lock().await.is_empty(),
        "the refused router must never receive a frame"
    );
}

/// AC2.4 — the negative control: with NO router installed, a `host.is_some()`
/// frame fails closed with `CrossHostNotConfigured`. It is never silently
/// delivered locally.
///
/// AC2.5(a) — hardening: Phase 2 delivers same-host recipients *before* the
/// Phase-3 error, so the recipient here is **cross-host only**. A frame that also
/// named a local recipient would have delivered to it and the negative would pass
/// while proving nothing about the cross-host leg.
///
/// AC2.5(b) — hardening: `cross_host_targets` is recomputed *after* the consent
/// gate, so an installed `ConsentGate` that rejects the remote recipient makes the
/// target vanish and `deliver` returns **`Ok`** with no error at all. No gate is
/// installed on this mailbox, which is what makes the assertion meaningful.
#[tokio::test]
async fn absent_router_fails_closed_and_delivers_nothing_locally() {
    maos_capability::cap_tokens::init_monotonic_base();

    let mailbox = Arc::new(Mailbox::new(Arc::new(IacRtMetrics::new())));
    // Registered so an `UnknownSpirit` error cannot masquerade as fail-closed,
    // and so "received nothing" is a real observation of an open channel.
    let mut target = mailbox.register_spirit("developer-remote").unwrap();

    // Cross-host-ONLY recipient (AC2.5a).
    let err = mailbox
        .deliver(task_assign_frame(
            "developer-remote",
            Some("developer-remote-host"),
        ))
        .await
        .expect_err("a host-bearing frame with no router installed must fail closed");
    match err {
        IacBusError::CrossHostNotConfigured { ref host_id } => {
            assert_eq!(host_id, "developer-remote-host");
        }
        other => panic!("expected CrossHostNotConfigured, got {other:?}"),
    }

    assert!(
        matches!(target.try_recv(), Ok(None)),
        "fail-closed means the target received NOTHING — never a silent local delivery"
    );
}

/// AC2.4 companion — the same frame, same mailbox, WITH a router installed,
/// routes. Without this pair the negative above could pass because the frame was
/// malformed rather than because the router was absent.
#[tokio::test]
async fn the_same_frame_routes_once_a_router_is_installed() {
    maos_capability::cap_tokens::init_monotonic_base();

    let router = Arc::new(CountingRouter::default());
    let mailbox = Arc::new(Mailbox::new(Arc::new(IacRtMetrics::new())));
    let mut target = mailbox.register_spirit("developer-remote").unwrap();
    mailbox.install_a2a_router(router.clone()).unwrap();

    mailbox
        .deliver(task_assign_frame(
            "developer-remote",
            Some("developer-remote-host"),
        ))
        .await
        .expect("with a router installed the identical frame routes");

    assert_eq!(
        router.calls.lock().await.as_slice(),
        ["developer-remote-host"]
    );
    assert!(
        matches!(target.try_recv(), Ok(None)),
        "a cross-host frame is routed, NOT also delivered to the local mailbox"
    );
}
