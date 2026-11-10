# L1 provider-construction inventory

Inspection of `RingCryptoProvider` references after the kernel-free cutover:

- `crates/maos-bin/src/crypto_provider.rs:17` is the production selection-table construction. Lines 125 and 133 are the feature-gated test recorder's delegate type and construction; the feature intentionally fails to compile in release.
- `crates/maos-kernel-core/src/security/mod.rs:350` remains the direct T3 image-lock verifier construction. Its structural function/path exception belongs to AC7 / the final kernel landing, not L1. The older composition description in `crates/maos-kernel-core/src/security/crypto.rs:22-25` is consequently stale; updating kernel files is outside this landing.
- `crates/maos-bin/src/main.rs:15631,15892` and `enterprise_identity.rs:751` are test-only fixtures, not production composition. The kernel's other direct occurrences under `security/crypto.rs`, `revocation/parser.rs` and `security/sandbox/t3/image_lock.rs` are implementation or test fixtures.
- Benchmark harness source files `maos-bench/src/halt_eval.rs` and `maos-bench/src/harness/j4.rs` are exempted as benchmark harnesses by the composition gate.
- Only daemon boot `verify_installed` routes publisher and claim verification to the selected provider. Ring-direct production paths remain: install/import pre-checks (`maos-registry/src/install.rs:86-120`); explicit `maos run`, topology children, and worker manifests (`maos-bin/src/main.rs:5678,6222,10199` via `admission.rs:577-578`); vetted upgrade and uninstall identity (`main.rs:193,10780`); yank (`yank_policy.rs:100`); registry storage and publish (`maos-registry/src/storage.rs:475`, `handlers/publish.rs:13`); the structural shim (`compliance_verify.rs:66`); `maos-spirit-cli/src/vet.rs:51`; and vetting-attestation checks within `verify_installed`. Explicit `maos run` does not evaluate the claim.

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
