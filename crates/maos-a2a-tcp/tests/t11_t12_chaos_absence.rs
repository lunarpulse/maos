//! AC-T11 — REAL-socket cert-rotation chaos as its OWN AC (3-endpoint topology
//! over real sockets + real TLS handshakes, explicitly NOT the OLD synthetic
//! rotation-drill timing model). AC-T12 — falsifiable absence assertions
//! (kernel performs ZERO auto-retry; the kernel crate is byte-identical).
//!
//! NOTE: this module deliberately does NOT reference the OLD synthetic
//! rotation-drill report class — AC-T11's grep guard is satisfied by absence.

mod support;

use maos_a2a_core::router::{A2APeerRouter, A2ATransport};
use maos_a2a_core::{A2AError, HandshakeRetryPolicy, PeerId, TofuPinStore};
use maos_a2a_tcp::{TcpA2ATransport, TcpTimeouts};
use maos_domain::invariants::i1::IntentClass;
use maos_spirit_abi::identity::HostId;
use support::*;

const HOSTS: [&str; 3] = ["host_a", "host_b", "host_c"];
const NONCES: [u64; 3] = [10, 20, 30];

/// Bind one mesh node that pins + accepts/sends `readonly` with the other two
/// nodes. Dial endpoints are placeholders, wired after all nodes bind.
async fn bind_node(
    clock: &Clock,
    ca: &Ca,
    idx: usize,
    own: &Leaf,
    peers: &[(usize, &Leaf)],
) -> TcpA2ATransport {
    let peer_pins: Vec<_> = peers
        .iter()
        .map(|(j, leaf)| pin(HOSTS[*j], &leaf.fingerprint, NONCES[*j]))
        .collect();
    let peer_cfgs: Vec<_> = peers
        .iter()
        .map(|(j, leaf)| {
            peer_cfg(
                HOSTS[*j],
                "tls://127.0.0.1:0",
                &leaf.fingerprint,
                &["readonly"],
                &["readonly"],
            )
        })
        .collect();
    bind_endpoint(
        own,
        Some(ca),
        NONCES[idx],
        peer_pins,
        peer_cfgs,
        clock,
        TcpTimeouts::test_profile(),
        HandshakeRetryPolicy::default(),
    )
    .await
}

/// Build a wired 3-node mesh from three leaves (one per node).
async fn build_mesh(clock: &Clock, ca: &Ca, leaves: &[Leaf; 3]) -> Vec<TcpA2ATransport> {
    let mut nodes = Vec::new();
    for i in 0..3 {
        let peers: Vec<(usize, &Leaf)> =
            (0..3).filter(|j| *j != i).map(|j| (j, &leaves[j])).collect();
        nodes.push(bind_node(clock, ca, i, &leaves[i], &peers).await);
    }
    // Wire real readback addresses (H3/H4) now that all listeners are bound.
    let addrs: Vec<_> = nodes.iter().map(|n| n.local_addr().unwrap()).collect();
    for (i, node) in nodes.iter().enumerate() {
        for j in 0..3 {
            if j != i {
                node.set_peer_endpoint(&HostId(HOSTS[j].into()), format!("tls://{}", addrs[j]));
            }
        }
    }
    nodes
}

/// Assert full NxN directed reachability (6 dials, all ACK).
async fn assert_nxn_reachable(nodes: &[TcpA2ATransport]) {
    for i in 0..3 {
        for j in 0..3 {
            if i == j {
                continue;
            }
            let frame = make_frame(HOSTS[i], HOSTS[j], IntentClass::Readonly, (i * 3 + j) as u64 + 1);
            nodes[i]
                .route_outbound(frame, &HostId(HOSTS[j].into()))
                .await
                .unwrap_or_else(|e| panic!("AC-T11: {} → {} must ACK, got {e}", HOSTS[i], HOSTS[j]));
            assert!(
                nodes[i].last_dial_attempts() <= HandshakeRetryPolicy::default().max_attempts as usize,
                "AC-T11: retry counters bounded by max_attempts"
            );
        }
    }
}

/// Assert every node's pin for every peer equals the expected per-node fp.
async fn assert_pins_converged(nodes: &[TcpA2ATransport], leaves: &[Leaf; 3]) {
    for (i, node) in nodes.iter().enumerate() {
        for j in 0..3 {
            if j == i {
                continue;
            }
            let p = node
                .pins()
                .get_pin(&PeerId::new(HOSTS[j]))
                .await
                .unwrap_or_else(|| panic!("node {i} missing pin for {}", HOSTS[j]));
            assert_eq!(
                p.fingerprint, leaves[j].fingerprint,
                "AC-T11: node {i}'s pin for {} must equal that node's serving fp",
                HOSTS[j]
            );
        }
    }
}

#[tokio::test]
async fn t11_real_socket_rotation_chaos_3_host() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-good");

    // ── Phase 1: fp_old on all three. Full NxN reachability, pins converged.
    let old: [Leaf; 3] = [
        valid_leaf(&ca, &clock),
        valid_leaf(&ca, &clock),
        valid_leaf(&ca, &clock),
    ];
    let mesh_old = build_mesh(&clock, &ca, &old).await;
    assert_nxn_reachable(&mesh_old).await;
    assert_pins_converged(&mesh_old, &old).await;

    // ── Phase 2: rotate each endpoint's serving cert fp_old → fp_new and re-pin
    // per the rotation protocol (operator re-pin = rebuild with new peer_pins).
    // Drop the old mesh first (H6 deterministic teardown frees the old ports).
    drop(mesh_old);
    let new: [Leaf; 3] = [
        valid_leaf(&ca, &clock),
        valid_leaf(&ca, &clock),
        valid_leaf(&ca, &clock),
    ];
    let mesh_new = build_mesh(&clock, &ca, &new).await;

    // Oracle: post-convergence, full NxN reachability over REAL sockets, and the
    // final pin-store state on all 3 == fp_new.
    assert_nxn_reachable(&mesh_new).await;
    assert_pins_converged(&mesh_new, &new).await;
}

// ─────────────────────────── AC-T12 — falsifiable absence ───────────────────

/// AC-T12(a) — the kernel performs ZERO auto-retry: the ONLY retrier is
/// `HandshakeRetryPolicy` on the transport side. Falsifiable structural proof:
/// `maos-a2a-tcp` does not depend on `maos-kernel-core` at all, so the kernel
/// literally cannot retry an A2A dial. (The transport-side retry counter is
/// exercised by AC-T5.)
#[test]
fn t12a_kernel_zero_auto_retry_dep_absent() {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let src = std::fs::read_to_string(&manifest).expect("read Cargo.toml");
    assert!(
        !src.contains("maos-kernel-core"),
        "AC-T12(a): maos-a2a-tcp MUST NOT depend on maos-kernel-core (kernel cannot retry A2A)"
    );
}

/// AC-T12(b) — `maos-kernel-core` is byte-identical to its pre-story state.
/// Falsifiable line-count gate (analogous to Story 8.4's 15505 check): the
/// extraction added a crate but touched NOTHING in the kernel.
#[test]
fn t12b_kernel_core_byte_identical_line_count() {
    // Story 8.16 §A4: the pinned count is no longer hard-coded here. It is read
    // from `xtask/kernel-core-baseline.toml` — the SINGLE source of truth shared
    // with the `check-kernel-baseline` xtask gate — so the kernel line count can
    // never drift unsummed across a multi-story phase (the 16263-vs-21128 gap).
    // Bump it ONLY in that toml, alongside an authorized charter delta.
    let baseline_toml = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("xtask")
        .join("kernel-core-baseline.toml");
    let baseline_src =
        std::fs::read_to_string(&baseline_toml).expect("read xtask/kernel-core-baseline.toml");
    let kernel_core_src_lines: usize = baseline_src
        .lines()
        .map(str::trim)
        .find(|l| !l.starts_with('#') && l.starts_with("src_lines"))
        .and_then(|l| l.rsplit('=').next())
        .map(str::trim)
        .and_then(|v| v.parse().ok())
        .expect("parse `src_lines = N` from kernel-core-baseline.toml");
    let kernel_src = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("maos-kernel-core")
        .join("src");
    let total = count_rs_lines(&kernel_src);
    assert_eq!(
        total, kernel_core_src_lines,
        "AC-T12(b): maos-kernel-core/src line count changed ({total} != {kernel_core_src_lines}, \
         pinned in xtask/kernel-core-baseline.toml) — the extraction must leave the kernel byte-identical"
    );
}

fn count_rs_lines(dir: &std::path::Path) -> usize {
    let mut total = 0;
    let entries = std::fs::read_dir(dir).expect("read kernel-core src dir");
    for entry in entries {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            total += count_rs_lines(&path);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            let content = std::fs::read_to_string(&path).expect("read rs file");
            total += content.lines().count();
        }
    }
    total
}

/// Compile-time anchor that this module never references the OLD synthetic
/// rotation-drill report class (AC-T11 grep guard is enforced by its absence).
#[allow(dead_code)]
fn _no_synthetic_rotation_report() {
    let _ = A2AError::ConfigInvalid(String::new());
}
