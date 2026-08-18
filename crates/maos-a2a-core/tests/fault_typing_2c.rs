//! Story `j1-crosshost-2c` AC3.2 — SHIP-BLOCKER: type `CODE_INTERNAL` and
//! `CODE_TIMEOUT` at `interpret_response`.
//!
//! `2b` filed this against this story **in the source itself**
//! (`router.rs:384-388`): the `CODE_INTERNAL` NACK it newly emits falls through
//! `interpret_response`'s catch-all into `A2AError::TransportFailed`, then into
//! `IacBusError::CrossHostTransportFailure`. **A dropped-receiver internal NACK
//! was byte-identical at the sender to a genuine network partition.**
//!
//! AC3's fault windows inject exactly those two faults, so without this repair
//! AC3.3's assertions cannot distinguish the faults AC3.4 injects.
//!
//! Scope wall, copied verbatim from `2b`'s H13: repair ONLY these two codes,
//! record the census (machine-checked in
//! `crates/maos-bin/tests/bounded_postures_2b.rs`), touch nothing else.

use std::sync::Arc;

use maos_a2a_core::router::{map_a2a_error_to_iac_bus, A2ARouterCore};
use maos_a2a_core::transport::json_rpc::{
    CODE_FRAME_TOO_LARGE, CODE_INTERNAL, CODE_INVALID_REQUEST, CODE_METHOD_NOT_FOUND,
    CODE_PARSE_ERROR, CODE_TIMEOUT,
};
use maos_a2a_core::{A2AError, A2AJsonRpcResponse, InMemoryTofuPinStore, TofuPinStore};
use maos_domain::iac_bus_types::IacBusError;
use maos_spirit_abi::identity::HostId;

/// `interpret_response` performs no peer lookup, so an empty peer table is
/// sufficient.
fn core() -> A2ARouterCore {
    A2ARouterCore::new(
        vec![],
        Arc::new(InMemoryTofuPinStore::new()) as Arc<dyn TofuPinStore>,
    )
}

fn peer() -> HostId {
    HostId("host-b".to_string())
}

fn interpret(code: i32, message: &str) -> A2AError {
    core()
        .interpret_response(&peer(), A2AJsonRpcResponse::nack(7, code, message))
        .expect_err("a NACK must reject the sender")
}

/// The receiver reported an internal failure, so the frame was **NOT delivered**.
/// That is a different fact from "the network ate it", and the sender must be
/// able to tell them apart.
#[test]
fn an_internal_nack_is_a_peer_internal_failure_never_a_transport_failure() {
    let error = interpret(
        CODE_INTERNAL,
        "intake sink full or receiver dropped — frame NOT delivered",
    );
    match &error {
        A2AError::PeerInternalFailure { peer, message } => {
            assert_eq!(peer, "host-b");
            assert!(
                message.contains("receiver dropped"),
                "the receiver's own reason must survive: {message}"
            );
        }
        other => panic!("CODE_INTERNAL must be typed, got {other:?}"),
    }
    assert!(
        !matches!(error, A2AError::TransportFailed(_)),
        "a dropped-receiver NACK must never be presented as a network partition"
    );
}

/// `CODE_TIMEOUT` is the code AC3's own injected timeouts produce at the
/// receiver. It is not a partition of the wire.
#[test]
fn a_timeout_nack_is_a_peer_intake_timeout_never_a_transport_failure() {
    let error = interpret(CODE_TIMEOUT, "intake read exceeded 30s");
    match &error {
        A2AError::PeerIntakeTimeout { peer, message } => {
            assert_eq!(peer, "host-b");
            assert!(message.contains("30s"), "{message}");
        }
        other => panic!("CODE_TIMEOUT must be typed, got {other:?}"),
    }
    assert!(!matches!(error, A2AError::TransportFailed(_)));
}

/// **The property AC3.3 depends on.** The three faults AC3 injects must be three
/// distinguishable outcomes at the `IacBusError` boundary the kernel sees — not
/// one `CrossHostTransportFailure` wearing three hats.
#[test]
fn the_three_injected_faults_are_three_distinct_bus_errors() {
    let internal = map_a2a_error_to_iac_bus(interpret(CODE_INTERNAL, "receiver dropped"), "host-b");
    let intake_timeout =
        map_a2a_error_to_iac_bus(interpret(CODE_TIMEOUT, "intake stalled"), "host-b");
    let partition = map_a2a_error_to_iac_bus(
        A2AError::PartitionTimeout {
            peer: "host-b".to_string(),
            frame_id: [0x11; 16],
            timeout_secs: 30,
        },
        "host-b",
    );

    // A genuine partition keeps its own typed bus variant.
    assert!(
        matches!(
            partition,
            IacBusError::CrossHostPartitionTimeout {
                timeout_secs: 30,
                ..
            }
        ),
        "a real partition must stay typed: {partition:?}"
    );
    // Neither receiver-side fault may impersonate it.
    assert!(
        !matches!(internal, IacBusError::CrossHostPartitionTimeout { .. }),
        "an internal NACK is not a partition"
    );
    assert!(
        !matches!(
            intake_timeout,
            IacBusError::CrossHostPartitionTimeout { .. }
        ),
        "a receiver intake timeout is not a wire partition"
    );
    // And neither collapses into the catch-all that used to swallow both.
    for (label, err) in [("internal", &internal), ("intake timeout", &intake_timeout)] {
        assert!(
            !matches!(err, IacBusError::CrossHostTransportFailure { .. }),
            "{label} must not render as CrossHostTransportFailure: {err:?}"
        );
    }
    // The three renderings are pairwise different strings, which is what an
    // operator reading the Transparency Log actually sees.
    let rendered = [
        internal.to_string(),
        intake_timeout.to_string(),
        partition.to_string(),
    ];
    assert_ne!(rendered[0], rendered[1]);
    assert_ne!(rendered[0], rendered[2]);
    assert_ne!(rendered[1], rendered[2]);
    assert!(
        rendered[0].contains("internal failure"),
        "the internal case must say so: {}",
        rendered[0]
    );
    assert!(
        rendered[1].contains("intake timeout"),
        "the intake-timeout case must say so: {}",
        rendered[1]
    );
}

/// SCOPE WALL — the four codes this story does NOT touch must stay in the
/// catch-all. A nine-arm refactor inside a frozen crate is a different story;
/// this leg makes the boundary machine-checked instead of a comment.
#[test]
fn the_remaining_four_codes_stay_in_the_catch_all() {
    for code in [
        CODE_PARSE_ERROR,
        CODE_INVALID_REQUEST,
        CODE_METHOD_NOT_FOUND,
        CODE_FRAME_TOO_LARGE,
    ] {
        let error = interpret(code, "unchanged");
        assert!(
            matches!(error, A2AError::TransportFailed(_)),
            "code {code} is outside this story's scope wall but was typed: {error:?}"
        );
    }
}
