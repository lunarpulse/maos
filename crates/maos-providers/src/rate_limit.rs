#![forbid(unsafe_code)]

//! Story 6.4 / NFR-Scale-4 — per-(provider, credential) token bucket.
//!
//! Each bucket is keyed by `(provider_id, credential_fingerprint)` where
//! `credential_fingerprint = first-8-bytes-of-SHA256(api_key_bytes)`. The
//! 8-byte prefix is sufficient for cross-credential disambiguation within
//! a single Host's bucket map (2^64 keyspace); the full credential is NEVER
//! stored beyond the existing provider driver's in-memory api_key field.
//!
//! Refill is continuous (per-second) at
//! `refill_per_sec = capacity / refill_window_secs`. At v0.5 the refill
//! window is 60 (RPM semantics); TPM is a future second bucket — out of
//! scope per Story 6.4 (RPM-only at v0.5).
//!
//! The bucket lives PER-PROCESS. Cross-Host bucket coordination (e.g., a
//! 30-host fleet sharing an Anthropic key) is a v2.0+ concern.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
#[cfg(test)]
use std::time::Duration;

use dashmap::DashMap;
use sha2::{Digest, Sha256};

/// Stable provider identifier. At v0.5 we accept any `&'static str`; the
/// composition root interns the strings (`"anthropic"`, `"openai"`, `"ollama"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BucketKey {
    /// Stable provider identifier.
    pub provider_id: &'static str,
    /// First 8 bytes of SHA-256(api_key) as little-endian u64 — opaque.
    pub credential_fingerprint: u64,
}

impl BucketKey {
    pub fn new(provider_id: &'static str, credential_fingerprint: u64) -> Self {
        Self {
            provider_id,
            credential_fingerprint,
        }
    }
}

/// Compute `credential_fingerprint` for a raw secret. Returns the first
/// 8 bytes of SHA-256 as a little-endian `u64`.
pub fn fingerprint_credential(secret: &str) -> u64 {
    let digest = Sha256::digest(secret.as_bytes());
    let bytes: [u8; 8] = digest[..8].try_into().expect("sha256 has ≥8 bytes");
    u64::from_le_bytes(bytes)
}

/// Refill window in seconds — v0.5 uses RPM semantics (60s window).
pub const REFILL_WINDOW_SECS: u32 = 60;

/// Lock-free token bucket carrying `(remaining_millitokens, last_refill_ns)`
/// packed into a single `AtomicU64`. The fields are:
///   * upper 32 bits = remaining * 1000 (milli-tokens; supports fractional refill)
///   * lower 32 bits = (last_refill_ns >> 16) — coarse 64µs resolution, fits in 32 bits for the bucket's lifetime
///
/// On every `try_consume`, the bucket performs a CAS loop that:
///   1. Reads current state
///   2. Computes elapsed time → top up milli-tokens
///   3. Subtracts 1000 milli-tokens (one full token)
///   4. CAS-writes the new state
///
/// A bucket starts FULL.
#[derive(Debug)]
pub struct TokenBucket {
    capacity: u32,
    /// Tokens per second of refill rate (e.g., 1000/60 ≈ 16.66 tokens/s for
    /// Anthropic free tier).
    refill_per_sec: f32,
    /// Packed `(remaining_x_1000 << 32) | (last_refill_ns_scaled)`.
    state: AtomicU64,
    /// Start instant for relative ns measurement (avoids u64 overflow for
    /// `last_refill_ns_scaled` in a single Host's lifetime).
    epoch: Instant,
}

/// State returned to a caller on bucket exhaustion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryAfter {
    pub retry_after_ms: u64,
    pub snapshot: BucketSnapshot,
}

/// Snapshot of bucket state at a point in time (read-only).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BucketSnapshot {
    pub capacity: u32,
    pub remaining: u32,
    pub refill_per_sec: u32,
}

impl TokenBucket {
    /// Construct a new bucket at full capacity.
    pub fn new(capacity: u32, refill_per_sec: f32) -> Self {
        // Clamp capacity to avoid u64 overflow in millitoken arithmetic.
        let safe_cap = capacity.min(u32::MAX / 1000);
        let state_packed = ((safe_cap as u64) * 1000) << 32;
        Self {
            capacity: safe_cap,
            refill_per_sec,
            state: AtomicU64::new(state_packed),
            epoch: Instant::now(),
        }
    }

    /// Per-RPM constructor. capacity = rpm; refill = rpm / 60.
    /// Rejects rpm == 0 to avoid permanently-bricked buckets.
    pub fn with_rpm(rpm: u32) -> Self {
        assert!(rpm > 0, "TokenBucket rpm must be > 0; got {}", rpm);
        Self::new(rpm, rpm as f32 / REFILL_WINDOW_SECS as f32)
    }

    fn now_scaled(&self) -> u64 {
        // Scaled-ns counter: relative to epoch, divided by 65536 (~64µs).
        // Returns u64 to avoid 3.2-day wrap-around (Story 6.4 review fix).
        let dur = self.epoch.elapsed().as_nanos() as u64;
        dur >> 16
    }

    fn unpack(state: u64) -> (u32, u64) {
        let remaining_millitokens = (state >> 32) as u32;
        let last_refill_scaled = state & 0xFFFF_FFFF;
        (remaining_millitokens, last_refill_scaled)
    }

    fn pack(remaining_millitokens: u32, last_refill_scaled: u64) -> u64 {
        ((remaining_millitokens as u64) << 32) | (last_refill_scaled & 0xFFFF_FFFF)
    }

    /// Try to consume one token. Returns `Ok(())` on success,
    /// `Err(RetryAfter)` on empty.
    pub fn try_consume(&self) -> Result<(), RetryAfter> {
        loop {
            let now_scaled = self.now_scaled();
            let cur = self.state.load(Ordering::Acquire);
            let (cur_milli, last_scaled) = Self::unpack(cur);

            // Compute elapsed real ns since last refill.
            let elapsed_scaled = now_scaled.wrapping_sub(last_scaled);
            // Convert scaled-ns back to seconds (each scaled tick = 64µs ≈ 2^-14 s).
            let elapsed_secs = (elapsed_scaled as f32) * (65_536.0 / 1_000_000_000.0);
            let added_milli = (elapsed_secs * self.refill_per_sec * 1000.0) as u64;
            let cap_milli = (self.capacity as u64) * 1000;
            let next_milli = (cur_milli as u64 + added_milli).min(cap_milli) as u32;

            if next_milli < 1000 {
                // Compute retry-after-ms: time to refill ONE token.
                let need_milli = 1000.0 - (next_milli as f32);
                let retry_secs = if self.refill_per_sec > 0.0 {
                    (need_milli / 1000.0) / self.refill_per_sec
                } else {
                    f32::INFINITY
                };
                let retry_after_ms = if retry_secs.is_finite() {
                    (retry_secs * 1000.0).max(1.0) as u64
                } else {
                    // No refill — retry never; report a long fallback.
                    u64::MAX
                };
                return Err(RetryAfter {
                    retry_after_ms,
                    snapshot: BucketSnapshot {
                        capacity: self.capacity,
                        remaining: next_milli / 1000,
                        refill_per_sec: self.refill_per_sec as u32,
                    },
                });
            }
            let new_milli = next_milli.saturating_sub(1000);
            let new_state = Self::pack(new_milli, now_scaled);
            match self.state.compare_exchange_weak(
                cur,
                new_state,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return Ok(()),
                Err(_) => continue,
            }
        }
    }

    /// Read-only snapshot. Best-effort consistency (read may race a
    /// concurrent `try_consume`).
    pub fn snapshot(&self) -> BucketSnapshot {
        let cur = self.state.load(Ordering::Acquire);
        let (milli, _) = Self::unpack(cur);
        BucketSnapshot {
            capacity: self.capacity,
            remaining: milli / 1000,
            refill_per_sec: self.refill_per_sec as u32,
        }
    }

    /// Manually advance time inside this bucket — TEST ONLY. Walltime
    /// dependence in tests is forbidden per Story 6.3 NFR-Sec-13; this
    /// hook lets unit/integration tests deterministically refill. The
    /// `_for_test` suffix flags it as not for production code paths.
    #[doc(hidden)]
    pub fn force_refill_for_test(&self, elapsed: std::time::Duration) {
        let added_milli = (elapsed.as_secs_f32() * self.refill_per_sec * 1000.0) as u64;
        let cap_milli = (self.capacity as u64) * 1000;
        loop {
            let cur = self.state.load(Ordering::Acquire);
            let (cur_milli, last_scaled) = Self::unpack(cur);
            let next_milli = (cur_milli as u64 + added_milli).min(cap_milli) as u32;
            let new_state = Self::pack(next_milli, last_scaled);
            if self
                .state
                .compare_exchange(cur, new_state, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                break;
            }
        }
    }
}

/// Per-Host `ProviderRateLimiter` — the bucket registry the inference
/// router consults BEFORE forwarding to `Provider::complete`.
#[derive(Debug)]
pub struct ProviderRateLimiter {
    buckets: DashMap<BucketKey, Arc<TokenBucket>>,
    config: ProviderRateLimitConfig,
}

#[derive(Debug, Clone)]
pub struct ProviderRateLimitConfig {
    /// Per-provider defaults; operator overrides via env vars
    /// (see `ProviderRateLimitConfig::from_env`).
    pub per_provider: std::collections::HashMap<&'static str, ProviderQuota>,
}

#[derive(Debug, Clone, Copy)]
pub struct ProviderQuota {
    /// Requests per minute. Anthropic free tier ~50; tier-1 ~1000.
    pub rpm: u32,
}

impl ProviderRateLimitConfig {
    /// Default configuration sourced from env vars:
    ///   - `MAOS_ANTHROPIC_RPM` (default 1000)
    ///   - `MAOS_OPENAI_RPM` (default 3500)
    ///   - `MAOS_OLLAMA_RPM` (default 999_999_999)  // local; effectively unbounded
    pub fn from_env() -> Self {
        fn read_rpm(env_key: &str, default: u32) -> u32 {
            let raw = std::env::var(env_key).ok();
            let parsed = raw.as_ref().and_then(|s| s.parse::<u32>().ok());
            match parsed {
                Some(v) if v > 0 => v,
                Some(0) => {
                    eprintln!(
                        "maos: WARN {}=0 is invalid (rpm must be > 0); using default {}",
                        env_key, default
                    );
                    default
                }
                _ => {
                    if raw.is_some() {
                        eprintln!(
                            "maos: WARN {} is not a valid u32; using default {}",
                            env_key, default
                        );
                    }
                    default
                }
            }
        }
        let mut per_provider = std::collections::HashMap::new();
        per_provider.insert(
            "anthropic",
            ProviderQuota {
                rpm: read_rpm("MAOS_ANTHROPIC_RPM", 1000),
            },
        );
        per_provider.insert(
            "openai",
            ProviderQuota {
                rpm: read_rpm("MAOS_OPENAI_RPM", 3500),
            },
        );
        per_provider.insert(
            "ollama",
            ProviderQuota {
                rpm: read_rpm("MAOS_OLLAMA_RPM", 999_999_999),
            },
        );
        Self { per_provider }
    }
}

// Note: Intentionally NO `Default` impl — `from_env()` has side effects
// and `Default::default()` must be pure. Callers use `from_env()` explicitly.

impl ProviderRateLimiter {
    pub fn new(config: ProviderRateLimitConfig) -> Self {
        Self {
            buckets: DashMap::new(),
            config,
        }
    }

    /// Look up the bucket for `key`; create on first reference.
    fn bucket_for(&self, key: BucketKey) -> Arc<TokenBucket> {
        if let Some(b) = self.buckets.get(&key) {
            return Arc::clone(b.value());
        }
        let quota = self
            .config
            .per_provider
            .get(key.provider_id)
            .copied()
            .unwrap_or(ProviderQuota { rpm: 1000 });
        let bucket = Arc::new(TokenBucket::with_rpm(quota.rpm));
        let entry = self
            .buckets
            .entry(key)
            .or_insert_with(|| Arc::clone(&bucket));
        Arc::clone(entry.value())
    }

    /// Try to consume one token for `(provider_id, credential_fingerprint)`.
    pub fn try_consume(&self, key: BucketKey) -> Result<(), RetryAfter> {
        let bucket = self.bucket_for(key);
        bucket.try_consume()
    }

    /// Read a snapshot of the bucket for `key`. Returns `None` if the bucket
    /// has not been touched (no `try_consume` call yet).
    pub fn snapshot(&self, key: BucketKey) -> Option<BucketSnapshot> {
        self.buckets.get(&key).map(|b| b.value().snapshot())
    }

    /// Test-only: install an explicit bucket for `key` (overrides the
    /// config-derived default). Used by tests that need a tiny capacity.
    #[cfg(test)]
    pub fn install_bucket_for_test(&self, key: BucketKey, bucket: Arc<TokenBucket>) {
        self.buckets.insert(key, bucket);
    }
}

impl ProviderRateLimiter {
    /// Create a limiter with `ProviderRateLimitConfig::from_env()`.
    pub fn from_env() -> Self {
        Self::new(ProviderRateLimitConfig::from_env())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_credential_stable() {
        let fp1 = fingerprint_credential("k1");
        let fp2 = fingerprint_credential("k2");
        let fp1_again = fingerprint_credential("k1");
        assert_ne!(fp1, fp2);
        assert_eq!(fp1, fp1_again);
    }

    #[test]
    fn token_bucket_starts_full() {
        let bucket = TokenBucket::new(3, 0.0);
        for _ in 0..3 {
            assert!(bucket.try_consume().is_ok());
        }
        assert!(bucket.try_consume().is_err());
    }

    #[test]
    fn token_bucket_refill_recovers_capacity() {
        // capacity 5; refill 1 tok/sec.
        let bucket = TokenBucket::new(5, 1.0);
        for _ in 0..5 {
            assert!(bucket.try_consume().is_ok());
        }
        assert!(bucket.try_consume().is_err());
        bucket.force_refill_for_test(Duration::from_secs(3));
        for _ in 0..3 {
            assert!(bucket.try_consume().is_ok());
        }
        assert!(bucket.try_consume().is_err());
    }

    #[test]
    fn rate_limiter_isolates_per_key() {
        let limiter = ProviderRateLimiter::new(ProviderRateLimitConfig {
            per_provider: {
                let mut m = std::collections::HashMap::new();
                m.insert("anthropic", ProviderQuota { rpm: 2 });
                m.insert("openai", ProviderQuota { rpm: 2 });
                m
            },
        });
        let k1 = BucketKey::new("anthropic", 1);
        let k2 = BucketKey::new("anthropic", 2); // different credential
        let k3 = BucketKey::new("openai", 1); // different provider
        // Exhaust K1's bucket.
        assert!(limiter.try_consume(k1).is_ok());
        assert!(limiter.try_consume(k1).is_ok());
        assert!(limiter.try_consume(k1).is_err());
        // K2 still has full bucket (different credential).
        assert!(limiter.try_consume(k2).is_ok());
        // K3 still has full bucket (different provider).
        assert!(limiter.try_consume(k3).is_ok());
    }

    #[test]
    fn retry_after_reports_refill_window() {
        let bucket = TokenBucket::new(1, 1.0); // 1 tok/sec refill
        assert!(bucket.try_consume().is_ok());
        let err = bucket.try_consume().unwrap_err();
        // Need 1 token at 1/sec ≈ 1000ms.
        assert!(
            err.retry_after_ms >= 1 && err.retry_after_ms <= 2000,
            "retry_after_ms={} out of expected range",
            err.retry_after_ms
        );
    }

    #[test]
    fn from_env_overrides_defaults() {
        std::env::set_var("MAOS_ANTHROPIC_RPM", "42");
        let cfg = ProviderRateLimitConfig::from_env();
        assert_eq!(cfg.per_provider.get("anthropic").unwrap().rpm, 42);
        // OpenAI default unchanged.
        assert!(cfg.per_provider.get("openai").unwrap().rpm > 0);
        std::env::remove_var("MAOS_ANTHROPIC_RPM");
    }
}
