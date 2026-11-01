//! P99 latency assertions for cap-token verify, against the one budget file.
//!
//! NFR-Perf-3: P99 < 100µs overall (verify + audit-enqueue) — the named
//! catastrophe guard asserted here on the shared hosted runner.
//! ADR-030's 5µs hot-path claim is a Tier C count per pull request plus a Tier W
//! P99 on the stable runner (`xtask/perf-budgets.toml`), never a shared-runner
//! wall-clock assertion (Story 20-7 D-20-7-04 A).

const BUDGETS: &str = include_str!("../../../xtask/perf-budgets.toml");

use std::sync::Arc;
use std::time::Instant;

use maos_domain::invariants::i1::{CapabilityToken, IntentClass, Scope};
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::ports::crypto::{CryptoError, CryptoProvider};
use maos_kernel_core::capability::cap_audit;
use maos_kernel_core::capability::cap_tokens::{
    init_monotonic_base, CapTokensShardRing, Ed25519SigningKey,
};

struct MockCryptoProvider;

impl CryptoProvider for MockCryptoProvider {
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
    fn sign_capability_token(
        &self,
        _sk: &[u8],
        token_bytes: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        let mut sig = [0u8; 64];
        for (i, b) in token_bytes.iter().enumerate() {
            sig[i % 64] ^= *b;
        }
        Ok(sig.to_vec())
    }
}

fn make_test_ring() -> (CapTokensShardRing, cap_audit::Sender) {
    let crypto: Arc<dyn CryptoProvider> = Arc::new(MockCryptoProvider);
    let signing_key = Ed25519SigningKey::new([0u8; 32]);
    let (audit_tx, mut audit_rx) = cap_audit::channel();
    std::thread::spawn(move || while audit_rx.blocking_recv().is_some() {});
    let ring = CapTokensShardRing::new(crypto, signing_key, 0xDEAD_BEEF, audit_tx.clone());
    (ring, audit_tx)
}

fn make_test_token(ring: &CapTokensShardRing) -> CapabilityToken {
    let posture = [1u8; 32];
    ring.issue(
        7,
        Scope::FsRead {
            subtree: "/tmp".into(),
        },
        60,
        posture,
        IntentClass::Standard,
    )
    .unwrap()
}

/// P99 of `samples` (nanoseconds), after sorting.
fn p99_ns(mut samples: Vec<u128>) -> u128 {
    samples.sort_unstable();
    samples[samples.len() * 99 / 100 - 1]
}

/// The budget row `id` of the one budget file, in nanoseconds (rows are in µs).
fn budget_ns(id: &str) -> u128 {
    let file: toml::Table = BUDGETS.parse().expect("perf-budgets.toml parses");
    let row = file["metric"]
        .as_array()
        .expect("metric rows")
        .iter()
        .find(|row| row["id"].as_str() == Some(id))
        .unwrap_or_else(|| panic!("perf-budgets.toml has no metric `{id}`"));
    assert_eq!(row["unit"].as_str(), Some("us"), "{id} is a µs row");
    row["budget"].as_integer().expect("integer budget") as u128 * 1_000
}

const WARMUP: usize = 2_000;
const SAMPLES: usize = 10_000;

/// NFR-Perf-3's 100 µs P99 as a named CATASTROPHE GUARD (20-7 D-20-7-04 A):
/// debug build, mock crypto, inside a parallel suite on a shared runner — it
/// can only catch a gross regression. ADR-030's 5 µs hot-path claim is NOT
/// asserted here: a wall-clock P99 on this runner cannot carry it (hosted
/// samples 5670/5480 ns). It lives in `perf-budgets.toml` as the Tier C
/// `cap-token-verify-*` count rows (every pull request) and the Tier W
/// `adr-030-cap-token-verify-p99` row (`benches/cap_token_verify_p99.rs`, on
/// the stable runner).
#[test]
fn cap_token_verify_p99_catastrophe_guard_nfr_perf_3() {
    init_monotonic_base();
    let (ring, _audit_tx) = make_test_ring();
    let token = make_test_token(&ring);
    let posture = [1u8; 32];
    for _ in 0..WARMUP {
        ring.verify(&token, posture, SandboxTier(2)).unwrap();
    }
    let mut samples = Vec::with_capacity(SAMPLES);
    for _ in 0..SAMPLES {
        let start = Instant::now();
        ring.verify(&token, posture, SandboxTier(2)).unwrap();
        samples.push(start.elapsed().as_nanos());
    }
    let measured = p99_ns(samples);
    let guard = budget_ns("nfr-perf-3-cap-token-verify-p99");
    eprintln!("cap_token_verify P99 = {measured}ns (NFR-Perf-3 catastrophe guard: {guard}ns)");
    assert!(
        measured < guard,
        "NFR-Perf-3 catastrophe guard: cap_token_verify P99 = {measured}ns, budget = {guard}ns"
    );
}

/// NFR-Perf-3 "overall": verify PLUS the audit enqueue the capability registry
/// performs after every verification (`record_verification`), timed together.
#[test]
fn cap_token_verify_plus_audit_enqueue_p99_under_nfr_perf_3() {
    init_monotonic_base();
    let (ring, audit_tx) = make_test_ring();
    let token = make_test_token(&ring);
    let posture = [1u8; 32];
    let enqueue = || {
        let result = ring.verify(&token, posture, SandboxTier(2));
        let _ = audit_tx.try_send(cap_audit::CapAuditEvent::Verify {
            token_id: token.token_id,
            spirit_pid: token.spirit_pid,
            outcome: cap_audit::VerifyOutcome::from_result(&result),
        });
        result.unwrap();
    };
    for _ in 0..WARMUP {
        enqueue();
    }
    let mut samples = Vec::with_capacity(SAMPLES);
    for _ in 0..SAMPLES {
        let start = Instant::now();
        enqueue();
        samples.push(start.elapsed().as_nanos());
    }
    let measured = p99_ns(samples);
    let budget = budget_ns("nfr-perf-3-cap-token-verify-p99");
    eprintln!("cap_token_verify + audit enqueue P99 = {measured}ns (NFR-Perf-3: {budget}ns)");
    assert!(
        measured < budget,
        "NFR-Perf-3 binding broken: verify + audit-enqueue P99 = {measured}ns, budget = {budget}ns"
    );
}
