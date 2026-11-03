# 20-13 creation — crypto bypass inventory at HEAD `8c809f9c`

Method: on the `git archive` copy, every `crates/*/src` and `xtask/src` `.rs` file scanned for `RingCryptoProvider`, `ring::`,
`ed25519_dalek`, `aes_gcm`, `SigningKey`/`VerifyingKey`, `.sign(`, `.verify(&`; hits after the first `#[cfg(test)]`/`#[cfg(all(test …))]`
module are test code; every remaining hit read by hand (false positives removed: `maos-bench/src/bin/tier_c_harness.rs:156` is a
`CapTokensShardRing` named `ring`). The construction scan is `gate-sim-base.txt` (the AC3 structural check's failing-before).

## A. `RingCryptoProvider` constructions (AC1, AC3, AC7)

| Site | Context | Disposition |
|---|---|---|
| `crates/maos-bin/src/main.rs:2218-2219` | daemon root (`fn main`, `#[cfg(feature = "network")]`); feeds capability `:2261-2262`, enterprise runtime `:2339-2340` | **becomes the selection** (AC1): `crypto_provider::select` |
| `crates/maos-bin/src/main.rs:3238-3239` | daemon root, second instance for CRL poller `:3240-3245`, door `:3653,3819` | **removed** — clone of the selected instance (AC1) |
| `crates/maos-bin/src/worker_spawn.rs:888` | `maos run` Worker T3 image lock (`run_cli_wrapper_manifest`) | **routed** — parameter from the root (3 callers + host-B context) (AC1) |
| `crates/maos-bin/src/main.rs:8403` | `smoke-t3-sandbox-5` arm | **routed** — the arm runs in `fn main` after the root's selection |
| `crates/maos-kernel-core/src/security/mod.rs:350` | kernel T3 admission (`SecurityManagerAdapter::admit_spirit`) | **routed at L2** ◆ (AC7, D-20-13-04); a named pending gate entry at L1 |
| `main.rs:15354,15615,16090,16119`; `enterprise_identity.rs:751` | `#[cfg(test)]` modules | allowed (test) |
| `maos-bench/src/halt_eval.rs:625`, `maos-bench/src/harness/j4.rs:154` | benchmark harnesses, not a daemon root | allowed, named in the gate's allowlist with reason |
| `maos-kernel-core/src/security/crypto.rs:37-98` | the provider itself | allowed |

## B. Direct primitive use — FR48's three surfaces

| File | Use | Process | Disposition |
|---|---|---|---|
| `maos-capability/src/cap_tokens/mod.rs:176-179` | token signing through `self.crypto` | daemon | **routed already** (C11) |
| `maos-kernel-core/src/revocation/{parser.rs:58, poller.rs:20}` | CRL verify through `&dyn CryptoProvider` | daemon | **routed already**; instance origin fixed by AC1 |
| `maos-kernel-core/src/security/sandbox/t3/{image_lock.rs:152-185, image_verify.rs:64,98}` | image-lock verify through `&dyn CryptoProvider` | daemon | **routed already**; callers fixed by AC1 / AC7 |
| `maos-secrets/src/lib.rs:481-483` | `seal_at_rest` through `crypto.seal_for_export` | daemon | **routed already**; instance from the root (`main.rs:2339-2340`) |
| `maos-secrets/src/lib.rs:16,584-606` | `ring::aead` for the reference KMS key-wrap and `open_at_rest` | daemon | **recorded** — KMS wrap is `KeyManagementPort`'s own crypto (dev/CI reference KMS, `lib.rs:3-7`); open path D-20-13-11 |
| `maos-compliance/src/evaluator.rs:100-106,317-321` | ComplianceClaim signature, ring direct | daemon boot re-verify, maosctl, corpus | **routed for the daemon** via `evaluate_envelope_with` (AC4, D-20-13-05); ring-direct entry kept for maosctl/corpus, allowlisted with reason |
| `maos-registry/src/admission.rs:549-562` | publisher signature, ring direct | daemon boot re-verify, maosctl | **routed for the daemon** via `verify_publisher_sig_with` (AC4, D-20-13-05) |

## C. Direct primitive use — outside FR48's three surfaces (recorded, D-20-13-10)

| File | Use | Process | Why outside |
|---|---|---|---|
| `maos-audit/src/sealed_export.rs:6,296-334,336-380,553-630,677-680` | audit bundle + two-host receipt Ed25519 sign/verify | maosctl (`subcommands.rs:3268,3817,3944`) | audit artifact signature, not encryption (C10); not a kernel operation |
| `maos-audit/src/erasure/proof.rs:12,109,252,305-313` | erasure proof sign/verify | daemon one-shot (`main.rs:10526`), `tools/verify-erasure` | audit artifact |
| `maos-audit/src/erasure/regional_teardown.rs:36,260,314-320` | regional teardown receipt | maosctl | audit artifact |
| `maos-audit/src/release_verify.rs:16,159-177` | release signature verify | `maos`/`maosctl` `release-verify` | supply-chain verification of MAOS itself |
| `maos-domain/src/audit_key.rs:47-50,398-401` | audit key derivation | daemon, maosctl | key derivation for the audit artifacts above |
| `maos-loom-lite/src/replication/bundle.rs:28,260-1035` | replication bundle sign/verify | daemon (loom) | A2A/Loom replication integrity (v2.x surface) |
| `maos-cohort/src/{manifest.rs:38,536-548; pin.rs:37-139}` | cohort authority pins | daemon, maosctl | cohort trust (Epic 12/13) |
| `maos-cli/src/subcommands.rs:381-403` | `maosctl cohort sign` | maosctl | operator tooling |
| `maos-compliance/src/builder.rs:92-128`, `vetting/mod.rs:312-332` | claim producer, vetting attestation sign/verify | publisher/vetter tooling; vetting binding is 20-1b AC6 | producer side; 20-1b owns the binding |
| `maos-registry/src/client.rs:155-159` | registry server tier signature | maosctl install | install-time (D-20-13-05) |
| `maos-registry/src/handlers/manifest.rs:39-41` | registry server signing | `maos-registry-server` | server tooling |
| `maos-spirit-cli/src/{compliance_claim.rs:8,128; publish.rs:130; signing.rs:19}` | publisher signing | `maos-spirit` | publisher tooling (epic AC2 names it) |
| `maos-corpus-gen/src/ccac/expansion.rs:88,205` | CCAC corpus claims | corpus generator | test-corpus tooling |
| `maos-bin/src/main.rs:8288-8290` (`smoke-upgrade-revoke-5`), `:8797-8988` (`smoke-registry-5d`) | smoke fixtures sign test CRLs/packages | smoke arms | fixture producers |

## D. `ring` used for something else (not a provider operation)

`maos-capability/src/cap_tokens/body.rs:45` (`ring::digest`, scope hash); `maos-control/src/lib.rs:17` (`ring::constant_time`);
rustls TLS provider `maos-a2a-core/src/mtls.rs:108`, `maos-a2a-tcp/src/transport.rs:1287,1340,1368`, `maos-a2a-tcp/src/verifier.rs:102`,
`maos-egress/src/upstream.rs:273`. The TLS stack is a FIPS-module concern for v1.5 (recorded in AC6's claim boundary), not an FR48 surface.

**Counts:** 21 non-test `src` files sign/verify/seal with `ring`/`ed25519-dalek` directly besides the provider (B: 3 — `maos-secrets`
KMS/open, `maos-compliance` evaluator, `maos-registry` admission; C: 18 recorded); 4 FR48 consumers already go through the port (B);
6 files use `ring` for hashing / constant time / TLS (D). The epic's "16 files" (2026-10-23) is superseded.
