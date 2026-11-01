//! Story 20-7 AC5 (ADR-068 §1, D-20-7-07 A) — the Tier C harness: one CPU-bound
//! hot path per invocation, measured by instruction count under
//! `valgrind --tool=callgrind --toggle-collect=maos_tier_c_measured` and by
//! allocation count through the counting `#[global_allocator]` below.
//!
//! `tier_c_harness <path> <iterations>` with `path` one of `empty`,
//! `cap_token_verify`, `iac_routing`, `frame_lowering`. All setup (runtime, ring,
//! frames, buffers) happens before the measured loop; only
//! `maos_tier_c_measured` is collected, so callgrind's total is the loop alone
//! and the `empty` path is the call overhead `check-perf-counts` subtracts.
//! Prints `{"path","iterations","allocations"}` (allocations inside the loop).
//!
//! The allocator is this file's only `unsafe`: it forwards every call to
//! `System` and bumps one atomic. The library crate keeps `forbid(unsafe_code)`.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use maos_domain::frame::{
    FrameAddress, FramePayload, IacFrame, PosturePreferences, TaskAssignPayload,
};
use maos_domain::invariants::i1::{CapabilityToken, IntentClass, Scope};
use maos_domain::invariants::i13::IntentLineage;
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::ports::crypto::{CryptoError, CryptoProvider};
use maos_kernel_core::capability::cap_audit;
use maos_kernel_core::capability::cap_tokens::{
    init_monotonic_base, CapTokensShardRing, Ed25519SigningKey,
};
use maos_kernel_core::iac::{IacBusAdapter, Mailbox, TransparencyLogAdapter};
use maos_spirit_abi::identity::{FrameKind, SpiritId};

static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);

struct Counting;

// SAFETY: every method forwards unchanged to `System`, which upholds the
// `GlobalAlloc` contract; the only addition is a relaxed counter bump.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        System.alloc(layout)
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        System.alloc_zeroed(layout)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        System.realloc(ptr, layout, new_size)
    }
}

#[global_allocator]
static COUNTING: Counting = Counting;

/// The one function callgrind collects (`--toggle-collect`).
#[no_mangle]
#[inline(never)]
pub fn maos_tier_c_measured(work: &mut dyn FnMut(), iterations: u64) {
    for _ in 0..iterations {
        work();
    }
}

struct Mock;

impl CryptoProvider for Mock {
    fn verify_signature(&self, _pk: &[u8], _msg: &[u8], _sig: &[u8]) -> Result<(), CryptoError> {
        Ok(())
    }
    fn seal_for_export(
        &self,
        _k: &[u8],
        _n: &[u8],
        _a: &[u8],
        p: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        Ok(p.to_vec())
    }
    fn sign_capability_token(&self, _sk: &[u8], bytes: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let mut sig = [0u8; 64];
        for (i, b) in bytes.iter().enumerate() {
            sig[i % 64] ^= *b;
        }
        Ok(sig.to_vec())
    }
}

fn make_frame(seq: u64) -> IacFrame {
    let mut frame_id = [0u8; 16];
    frame_id[0..8].copy_from_slice(&seq.to_le_bytes());
    IacFrame {
        frame_id,
        timestamp_ns: 0,
        logical_clock: seq,
        from: FrameAddress {
            spirit_id: SpiritId::from("orchestrator"),
            host_id: None,
            role: None,
        },
        to: std::iter::once(FrameAddress {
            spirit_id: SpiritId::from("worker"),
            host_id: None,
            role: None,
        })
        .collect(),
        kind: FrameKind::TaskAssign,
        intent: IntentClass::Standard,
        payload: FramePayload::TaskAssign(TaskAssignPayload {
            goal: format!("synthetic-{seq}"),
            scope: vec![],
            success_criteria: "ok".into(),
            posture_preferences: PosturePreferences::default(),
            prior_distillate_ref: None,
        }),
        auto_marker: FrameOrigin::HumanAuthored,
        consent_envelope: None,
        intent_lineage: IntentLineage::default(),
    }
}

fn run(path: &str, iterations: u64) -> u64 {
    init_monotonic_base();
    match path {
        "empty" => measure(&mut || {}, iterations),
        "cap_token_verify" => {
            let crypto: Arc<dyn CryptoProvider> = Arc::new(Mock);
            let (audit_tx, mut audit_rx) = cap_audit::channel();
            std::thread::spawn(move || while audit_rx.blocking_recv().is_some() {});
            let ring = CapTokensShardRing::new(
                crypto,
                Ed25519SigningKey::new([0u8; 32]),
                0xDEAD_BEEF,
                audit_tx,
            );
            let posture = [1u8; 32];
            let token: CapabilityToken = ring
                .issue(
                    7,
                    Scope::FsRead {
                        subtree: "/tmp".into(),
                    },
                    60,
                    posture,
                    IntentClass::Standard,
                )
                .expect("issue token");
            measure(
                &mut || {
                    ring.verify(&token, posture, SandboxTier(2))
                        .expect("verify");
                },
                iterations,
            )
        }
        "iac_routing" => {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("tokio runtime");
            let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0));
            let metrics = Arc::new(maos_kernel_core::telemetry::iac_rt::IacRtMetrics::new());
            let adapter = IacBusAdapter::new(Arc::new(Mailbox::new(metrics)), tl);
            let mut worker = rt.block_on(async {
                adapter
                    .register_spirit_typed(&SpiritId::from("worker"))
                    .expect("register worker")
            });
            rt.spawn(async move { while worker.recv().await.is_some() {} });
            let mut frames: Vec<IacFrame> = (0..iterations).map(make_frame).collect();
            measure(
                &mut || {
                    let frame = frames.pop().expect("one frame per iteration");
                    rt.block_on(adapter.deliver_typed(frame, 0, None))
                        .expect("deliver");
                },
                iterations,
            )
        }
        "frame_lowering" => {
            let frame = make_frame(1);
            let mut wire: Vec<u8> = Vec::with_capacity(4096);
            measure(
                &mut || {
                    wire.clear();
                    let Ok(body) = serde_json::to_vec(&frame) else {
                        eprintln!("tier_c_harness: the frame does not serialise");
                        std::process::exit(3);
                    };
                    maos_frame_codec::write_frame(&mut wire, &body).expect("frame");
                },
                iterations,
            )
        }
        other => {
            eprintln!("tier_c_harness: unknown path `{other}`");
            std::process::exit(2);
        }
    }
}

/// Allocations counted across exactly the measured loop.
fn measure(work: &mut dyn FnMut(), iterations: u64) -> u64 {
    let before = ALLOCATIONS.load(Ordering::Relaxed);
    maos_tier_c_measured(work, iterations);
    ALLOCATIONS.load(Ordering::Relaxed) - before
}

fn main() {
    let mut args = std::env::args().skip(1);
    let (Some(path), Some(iterations)) = (args.next(), args.next()) else {
        eprintln!("usage: tier_c_harness <path> <iterations>");
        std::process::exit(2);
    };
    let iterations: u64 = iterations.parse().expect("iterations is a number");
    let allocations = run(&path, iterations);
    println!("{{\"path\":\"{path}\",\"iterations\":{iterations},\"allocations\":{allocations}}}");
}
