# L1 provider-construction inventory

Inspection of `RingCryptoProvider` references after the kernel-free cutover:

- `crates/maos-bin/src/crypto_provider.rs:17` is the production selection-table construction. Lines 118 and 124 are the feature-gated test recorder's delegate type and construction; the feature intentionally fails to compile in release.
- `crates/maos-kernel-core/src/security/mod.rs:350` remains the direct T3 image-lock verifier construction. It is the single narrowly located structural-gate exception and belongs to AC7 / the final kernel landing, not L1. The older composition description in `crates/maos-kernel-core/src/security/crypto.rs:22-25` is consequently stale; updating kernel files is outside this landing.
- `crates/maos-bin/src/main.rs:15609,15870` and `enterprise_identity.rs:751` are test-only fixtures, not production composition. The kernel's other direct occurrences under `security/crypto.rs`, `revocation/parser.rs` and `security/sandbox/t3/image_lock.rs` are implementation or test fixtures.
- Benchmark harness source files `maos-bench/src/halt_eval.rs` and `maos-bench/src/harness/j4.rs` are exempted as benchmark harnesses by the composition gate. The gate otherwise visits production `crates/*/src` Rust syntax, including provider constructions in the binary root, and has an observed negative control documented in `preflight.md`.
- The daemon boot admission now supplies runtime provider identity and resolved endpoint. `maosctl` install/import remain explicitly non-authoritative install-time pre-checks; their manifest-derived defaults are not evidence of daemon runtime identity.

The section B creation inventory's capability tokens, CRL parser, Worker T3
lock, and `maos-secrets::seal_at_rest` already use the trait; this cutover
changes their instance origin. ComplianceClaim and publisher verification now
also use that instance on daemon boot. Reference KMS wrapping and opening
remain the separately ruled D-11 boundary. Creation sections C and D remain
outside FR48: audit artifacts and erasure receipts, release and replication
integrity, cohort authority, producer/vetting/registry tooling, digest,
constant-time and TLS. The subsequent 20-1b additions in
`maos-registry/src/security.rs` and `yank_auth.rs` implement registry
transport/response and yank-authorization protocols, not these FR48 daemon
signature-verification surfaces; they remain direct primitives and are not a
claim that the selected module covers TLS or registry protocol signing.
