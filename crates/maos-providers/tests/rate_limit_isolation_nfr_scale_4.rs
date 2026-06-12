#![forbid(unsafe_code)]

//! Story 6.4 / NFR-Scale-4 — per-(provider, credential) bucket isolation.
//!
//! Eight integration scenarios per AC4.1-4.8.

use std::sync::Arc;
use std::time::Duration;

use maos_providers::rate_limit::{
    BucketKey, ProviderQuota, ProviderRateLimitConfig, ProviderRateLimiter, TokenBucket,
};
use maos_providers::{
    fingerprint_credential, AnthropicProvider, OllamaProvider, OpenAiProvider, Provider,
};

fn build_limiter(rpm: u32) -> ProviderRateLimiter {
    let mut cfg = ProviderRateLimitConfig {
        per_provider: std::collections::HashMap::new(),
    };
    cfg.per_provider.insert("anthropic", ProviderQuota { rpm });
    cfg.per_provider.insert("openai", ProviderQuota { rpm });
    cfg.per_provider.insert("ollama", ProviderQuota { rpm });
    ProviderRateLimiter::new(cfg)
}

/// AC4.1 — Same provider + same credential = shared bucket exhaustion.
#[test]
fn rate_4_1_same_provider_same_credential_shared_bucket() {
    let limiter = build_limiter(2); // capacity 2
    let key = BucketKey::new("anthropic", 12345);
    // Spirit A — exhaust the bucket.
    assert!(limiter.try_consume(key).is_ok(), "A.1");
    assert!(limiter.try_consume(key).is_ok(), "A.2");
    // Spirit B uses the same key → next call exhausted.
    assert!(
        limiter.try_consume(key).is_err(),
        "B.1 must return RateLimited"
    );
}

/// AC4.2 — Same provider + different credentials = independent buckets.
#[test]
fn rate_4_2_same_provider_different_credentials_isolated() {
    let limiter = build_limiter(2);
    let k1 = BucketKey::new("anthropic", 1);
    let k2 = BucketKey::new("anthropic", 2);
    assert!(limiter.try_consume(k1).is_ok());
    assert!(limiter.try_consume(k1).is_ok());
    assert!(limiter.try_consume(k1).is_err());
    // K2 has its own bucket; still full.
    assert!(limiter.try_consume(k2).is_ok());
    assert!(limiter.try_consume(k2).is_ok());
    assert!(limiter.try_consume(k2).is_err());
}

/// AC4.3 — Different providers = independent buckets.
#[test]
fn rate_4_3_different_providers_isolated() {
    let limiter = build_limiter(2);
    let anth = BucketKey::new("anthropic", 1);
    let oai = BucketKey::new("openai", 1);
    assert!(limiter.try_consume(anth).is_ok());
    assert!(limiter.try_consume(anth).is_ok());
    assert!(limiter.try_consume(anth).is_err());
    // OpenAI bucket is independent.
    assert!(limiter.try_consume(oai).is_ok());
    assert!(limiter.try_consume(oai).is_ok());
}

/// AC4.4 — Bucket refill: explicit time advance regenerates tokens.
#[test]
fn rate_4_4_bucket_refill_after_elapsed_time() {
    // Capacity 5, refill rate 1 token/sec.
    let bucket = Arc::new(TokenBucket::new(5, 1.0));
    for _ in 0..5 {
        assert!(bucket.try_consume().is_ok());
    }
    assert!(bucket.try_consume().is_err(), "bucket empty");
    bucket.force_refill_for_test(Duration::from_secs(5));
    for _ in 0..5 {
        assert!(bucket.try_consume().is_ok(), "5 tokens regenerated");
    }
    assert!(bucket.try_consume().is_err(), "next consume RateLimited");
}

/// AC4.5 — Concurrent CAS correctness: N tasks racing on capacity = N/2.
#[test]
fn rate_4_5_concurrent_cas_correctness() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(8)
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        let bucket = Arc::new(TokenBucket::new(50, 0.0));
        let mut handles = Vec::new();
        let succ_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let fail_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        for _ in 0..100u32 {
            let b = Arc::clone(&bucket);
            let s = Arc::clone(&succ_count);
            let f = Arc::clone(&fail_count);
            handles.push(tokio::spawn(async move {
                match b.try_consume() {
                    Ok(()) => {
                        s.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    }
                    Err(_) => {
                        f.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    }
                }
            }));
        }
        for h in handles {
            let _ = h.await;
        }
        let s = succ_count.load(std::sync::atomic::Ordering::SeqCst);
        let f = fail_count.load(std::sync::atomic::Ordering::SeqCst);
        assert_eq!(s, 50, "exactly 50 succeed on capacity=50, got s={s}");
        assert_eq!(f, 50, "exactly 50 fail, got f={f}");
    });
}

/// AC4.6 — Retry-after computed from bucket refill rate.
#[test]
fn rate_4_6_retry_after_refill_window() {
    let bucket = TokenBucket::new(1, 1.0); // 1 token/sec refill
    assert!(bucket.try_consume().is_ok());
    let err = bucket.try_consume().unwrap_err();
    // Expect approximately 1000ms (one full token at 1/sec).
    // Tightened tolerance from 2× to 1.5× (Story 6.4 review fix).
    assert!(
        err.retry_after_ms >= 800 && err.retry_after_ms <= 1500,
        "retry_after_ms={} out of range [800, 1500]",
        err.retry_after_ms
    );
}

/// AC4.7 — Default retry-after when refill rate = 0 (no auto-refill).
#[test]
fn rate_4_7_no_refill_reports_u64_max_or_inf_marker() {
    let bucket = TokenBucket::new(1, 0.0);
    assert!(bucket.try_consume().is_ok());
    let err = bucket.try_consume().unwrap_err();
    // Default fallback marker is u64::MAX or a very large number when
    // refill_per_sec=0; spec allows the router to substitute a 5000ms floor.
    assert!(
        err.retry_after_ms == u64::MAX || err.retry_after_ms > 1_000_000,
        "retry_after_ms={} unexpected for zero refill rate",
        err.retry_after_ms
    );
}

/// AC4.8 — Provider trait `credential_fingerprint` override per concrete provider.
///
/// Verifies the cross-credential isolation key derivation: two AnthropicProvider
/// instances with different api_keys produce different fingerprints, which
/// drive different buckets in the limiter.
#[test]
fn rate_4_8_provider_credential_fingerprint_distinct() {
    use maos_domain::ports::io_subsystem::IoError;
    use maos_domain::ports::IoSubsystemPort;

    struct MockTransport;
    impl IoSubsystemPort for MockTransport {
        fn http_get(&self, _: &str) -> Result<Vec<u8>, IoError> {
            unimplemented!()
        }
        fn http_post(&self, _: &str, _: &[u8], _: &[(&str, &str)]) -> Result<Vec<u8>, IoError> {
            Ok(vec![])
        }
    }

    let anth_a = AnthropicProvider::with_api_key(
        Arc::new(MockTransport),
        "http://test".into(),
        "claude-3".into(),
        "key-A".into(),
    );
    let anth_b = AnthropicProvider::with_api_key(
        Arc::new(MockTransport),
        "http://test".into(),
        "claude-3".into(),
        "key-B".into(),
    );
    let oai = OpenAiProvider::with_api_key(
        Arc::new(MockTransport),
        "http://test".into(),
        "gpt-4".into(),
        "key-A".into(),
    );
    let ollama = OllamaProvider::new(
        Arc::new(MockTransport),
        "http://localhost:11434".into(),
        "llama3".into(),
    )
    .unwrap();
    assert_ne!(anth_a.credential_fingerprint(), 0);
    assert_ne!(
        anth_a.credential_fingerprint(),
        anth_b.credential_fingerprint()
    );
    // Same key string across providers — fingerprints match (sha256 is
    // deterministic over bytes; cross-provider isolation comes from the
    // BucketKey carrying provider_id).
    assert_eq!(
        anth_a.credential_fingerprint(),
        oai.credential_fingerprint(),
        "same key bytes → same fingerprint; isolation comes from provider_id"
    );
    // Ollama uses base_url; distinct from anth_a's key.
    assert_ne!(
        anth_a.credential_fingerprint(),
        ollama.credential_fingerprint()
    );

    // The bucket keying confirms isolation: same fingerprint but different
    // provider_ids → different bucket keys.
    let k_anth_a = BucketKey::new("anthropic", anth_a.credential_fingerprint());
    let k_oai_a = BucketKey::new("openai", oai.credential_fingerprint());
    assert_ne!(k_anth_a, k_oai_a);

    // `fingerprint_credential` standalone produces the same value.
    assert_eq!(
        fingerprint_credential("key-A"),
        anth_a.credential_fingerprint()
    );
}
