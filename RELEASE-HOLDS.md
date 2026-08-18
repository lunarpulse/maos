# RELEASE-HOLDS.md — GA Ledger

> **Scope:** This ledger tracks the items that stand between the merged,
> feature-complete `main` line and a **GA release tag**. It exists to keep
> *development* and *GA* decoupled: the holds below gate a GA tag, **not**
> ongoing development on `main` (founder functionality-first directive;
> E11 retro A3 / E12 retro B2).

## Status

- **Feature-completeness:** ✅ **Done and merged.** Two lines are on `main`:
  Epic 11 (v2.0) + Epic 12 (v2.2, J3 Team Nexus) via PR #5, merge-commit
  `b8caae53` (2026-07-14); and Epic 13 — the **v2.2 Reza single-org cross-team
  Cortex line** — via merge-commit `439b59fd` (2026-08-11), closing 21/21
  stories with `product_claim: PROVEN` on four operator-lane substrate gates.
  All PRD journeys served.
- **GA tag:** ⏳ **Held.** Blocked only by the two external items below. Neither
  blocks further development, branch merges, or internal milestones.

## Holds

| # | Hold | Requirement | Owner | Status | GA effect |
|---|------|-------------|-------|--------|-----------|
| 1 | External penetration test | **NFR-Sec-7** — external pen-test report, **zero P0/P1 findings open at ship**. Triage by joint panel (pen-test lead + MAOS security owner); disagreement → PRD-author tiebreak. P0/P1 per OWASP Risk Rating Methodology, frozen at engagement start. | Security owner + external firm | ❌ Open (external, not yet scheduled) | **Blocks GA tag.** |
| 2 | Export-control classification | **NFR-Comp-1** — ECCN classification letter on file; EAR99 vs 5D002 determination published in `STABILITY.md §Export`; dual-use review for kernel crypto primitives. | Export counsel + security owner | ❌ Open (external counsel review pending) | **Blocks GA tag** for any externally-published binary that includes the WASM engine. |

## Standing mitigations (in force now, not a GA precondition)

- **WASM engine is OFF by default.** The `wasm-host` feature
  (`crates/maos-bin/Cargo.toml`) is off-by-default and gates the vendored
  `wasmtime` engine. Off-by-default is the **export-control precondition for
  5D002.c.1** — the shipped default artifact carries no controlled crypto/WASM
  engine, so internal builds and non-WASM distribution proceed while Hold 2 is
  open. See `docs/compliance/export-counsel-precondition.md` and
  `docs/compliance/eccn-classification.md`.

## Claim boundaries — v2.2 Reza line (in force now, not a GA precondition)

> Epic 13 closed with seven **accepted risks**, each ratified at the Epic-13
> retrospective §4 (2026-08-11) with no successor assignment. They are not
> holds: none is a deliverable and none blocks a GA tag. They bound **what this
> release may assert**. A boundary is violated by over-claiming, not by
> shipping. Recorded here because a limit that lives only in a retrospective is
> a limit nobody re-reads.

| # | Boundary | What the release may NOT claim | Successor |
|---|---|---|---|
| 1 | Kernel `CollectivePortError` collapses eight causes into one; the Spirit path's six producible causes all surface as `Transport`. | That collective failures are explainable **on the Spirit path**. The host-initiated crossing publishes a typed status and *may* claim it. | Future FLAG-Winston kernel-widening decision. Not hidden Epic 13 debt. |
| 2 | Operator-axis authorization is absent from `MAOS_ONE_SHOT` legal-hold-release and collective-erase; the shipped idiom treats host execution rights as authority. | That the **operator axis** of NFR-Ops-11 is served. `team_guard` is placement, not operator authorization. The team axis *is* served. | Explicitly outside Epic 13; must not be represented as closed. |
| 3 | An unauthorized legal-hold bypass is **not constructible** on collective rows — they are principal-namespace-free and `CollectiveEraseReceipt` cannot represent `held`. | That a negative control proves the bypass is RED. The control does not exist. | Decision D (partition-by-construction) preserved by choice; building the negative would violate it. |
| 4 | `consent_grant` records the compile-time intent constant, not a grant id/version/lease. | That a cross-wall disclosure row names the **specific manifest grant** that authorized it. Authorization is reconstructable only from (home_team, remote_team, intent). | Consent-port widening across crate boundaries; not required by the published minimum-disclosure journey. |
| 5 | Region provenance is **presence-based**: the region guard enforces that `source_log_ref` is present, not that it is cryptographically valid. | That region provenance is independently cryptographically validated. | Trusted applied-root registry — distinct from the completed Reza evidence claim. |
| 6 | Malformed nested `source_log_ref` is silently dropped; oversized receipt depth truncates. | That receipt ingestion is total. Both are pre-existing deserialization limits; no Epic 13 path produces the oversized depth. | v2.5 provenance hardening. |
| 7 | 13.5g TL-artifact replacement TOCTOU — individual opens are no-follow, but phase-to-open identity is not continuous. | That artifact identity is stable across phases against an adversary. The benign concurrent-boot half was fixed in-story. | Descriptor/snapshot design. |
| 8 | `j1-crosshost-2b`: the crossing is proven in two logs; there is **NO** round trip, **NO** signing, and **NO** reconciled bundle (those are `j1-crosshost-2c`); the loopback rehearsal arm permanently does not bind wire identity; peer boot-nonce provisioning is manual with no automated channel; peers are bare `IP:port` with no DNS. | That any of the excluded `j1-crosshost-2c` properties are available, or that the loopback arm binds wire identity, boot-nonce provisioning is automated, or peers support DNS. | `j1-crosshost-2c` for the listed follow-on claims; loopback identity non-binding and manual/IP-only provisioning remain boundaries. |
| 9 | `j1-crosshost-2c`: **two signatures prove two keyed identities signed — not two machines, two processes, or two operators.** The host discriminator on `AuditBundle` defeats a forger who does not hold the other host's key; it does not prove physical separation, and under a shared base seed it proves nothing at all, which is why `reconcile_two_host_bundles` refuses two halves attested by one root. `region` cannot separate two hosts in one jurisdiction (same derived key), `boot_nonce` is per-boot and one `--range 1d` export swept eight, and `attester_pubkey` is bundle-supplied so R-RG1 forbids trusting it. The mTLS trust anchor between two hosts is established **out of band by a human operator**, not by the protocol; host B's audit signing key is **provisioned separately, by hand**. Both are capture fields whose overclaim direction is refused (`two_host_trust_anchor`, `two_host_host_b_audit_key`). | That a two-host bundle proves two machines, two processes or two operators; that the pairing or the key provisioning is automated; that `region`, `boot_nonce` or `attester_pubkey` can discriminate the halves. | None — these are permanent properties of the artifact format and of the manual pairing path, not debt. An automated pairing channel would be its own story. |
| 10 | `j1-crosshost-2c`: the **delegation** path's crash window between `journal(Written)` and the worker spawn is BOUNDED, not closed. A crash there leaves a task durably `Written` with no execution record, so every replay returns `Duplicate`. The digest-reply path's equivalent ordering defect **was** fixed (nothing is `Duplicate` until something is durable), but the delegation window spans two operations with no shared transaction: closing it needs a boot-time reconciliation pass over `Written`-without-execution rows. | That delegated-task idempotency is crash-safe, or that a `Duplicate` verdict on the delegation path implies the work was executed. The digest-reply path *may* claim the invariant. | The delegation recovery lane (`deferred-work.md`, 2b review, item 2). |
| 11 | `j1-crosshost-2c`: graceful shutdown can still wait unboundedly on a never-exiting **local** worker join. Every sender-side wire operation is now bounded (`TcpStream::connect` and `framed.send` were both unbounded; the operator-configured `partition_timeout_secs` now reaches the TCP path), so no A2A call can hang the loop — but the `spawn_blocking` join into `run_cli_wrapper_manifest` has no abort path, and giving it one is a `maos-kernel-core` change this story is pinned to leave byte-identical (24472). | That Ctrl-C is bounded end to end. The A2A half *is* bounded and typed. | Kernel lifecycle lane, FLAG-Winston (`deferred-work.md`, 2b review, item 1). |
| 12 | `j1-crosshost-2c`: TOFU pins are rebuilt from config at **every boot** — `InMemoryTofuPinStore` is the only `impl TofuPinStore` in the workspace and no durable store exists, despite comments claiming otherwise. `pin_first_contact` is a boot-time config loader, never an observed certificate. The listen side accepts **any** active pin; per-peer scoping exists only on the dial side. Both sides now journal a `PeerIdentityUnverified` rupture, so a refusal is queryable rather than silent. | That TOFU state survives a restart, that first contact is observed rather than configured, or that the listen side scopes a client leaf to a specific peer. | Durable TOFU store; listen-side per-peer scoping. Both outside this story. |

## GA-tag procedure (do not tag until both clear)

1. Hold 1: pen-test report received, joint-panel triage records **zero P0/P1
   open**; attach report reference here.
2. Hold 2: ECCN letter on file, `STABILITY.md §Export` reflects the final
   EAR99/5D002 determination; export counsel signs off on WASM-inclusive
   external publication (or that path stays flag-gated at GA).
3. Only then cut the GA tag. Update this ledger's Status to reflect the tag.

## References

- E12 retro action B2 (retro doc `epic-12-retro-2026-07-13.md`).
- `_bmad-output/planning-artifacts/epics/requirements-inventory.md`
  (NFR-Sec-7 §L131, NFR-Comp-1 §L247).
- `_bmad-output/planning-artifacts/implementation-readiness-report-2026-07-10.md`
  (both holds marked NON-GATING for dev).
- Epic 13 retro §4 disposition table (`epic-13-retro-2026-08-11.md`), action C4.
  Closed **2** residuals, accepted risk on **7** groups (above), handed **12**
  groups to Epic 14 / its preflight.
