# Adversarial Review — §15 Full-Spectrum v2.2 (ADR-052/053/054/055)

**Reviewer lens:** adversarial implementation-divergence. Method: for each proposed decision, construct two implementation units one level down that each obey the written rules TO THE LETTER yet build incompatibly — clashing shared-data shapes, two owners of one entity, conflicting state-mutation paths, underspecified seams. Every incompatible pair is a hole; each hole gets a tightened rule.

**Target:** `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/15-full-spectrum-v2-2.md` (proposed-v2.2, 2026-07-06)
**Context read:** §10.7 (journey deferral/readiness), §7.2 (bilateral A2A incl. Story-8.8 fail-closed consent), §9.3 (Loom-lite), ADR-012, ADR-049, `xtask/kernel-core-baseline.toml` (src_lines = 23081), `xtask/kloc.toml`, `xtask/kernel-crates.toml`.
**Date:** 2026-07-06

**VERDICT: PASS-WITH-FIXES** — all four decisions are directionally sound and consistent with the lived Epic 9–11 discipline, but every one of them admits at least one pair of letter-compliant, mutually incompatible implementations. Two findings are blocker-severity (ADV-053-1, ADV-055-1); the rest close with one-sentence rule tightenings before party-mode ratification.

---

## 1. ADR-052 — Cohort mesh via signed static manifest

### ADV-052-1 [HIGH] — No single owner of the manifest version sequence; concurrent re-issue forks the consent matrix

**Written rule:** "a cohort = a versioned signed TOML manifest… operator-authored, Ed25519-signed"; "membership changes are manifest re-issues (signed, versioned, journaled), never runtime negotiation."

**The gap:** J3 is an 8-person team where *every member has their own Host* (§10.7.1) — i.e. up to 8 operators. The rule says "operator-authored" (singular noun, plural reality) and never names *which* key signs, *whose* counter allocates the version number, or *which* Transparency Log the re-issue is journaled to (the TL is per-Host SQLite, §7.3 — there is no cohort-level log).

**Incompatible pair, both letter-compliant:**
- **Unit A ("any-member-operator signs"):** any member's operator key may sign v(n+1); version = local monotonic counter; re-issue journaled to the issuer's own Host TL. Marcus signs v8 removing Nina; simultaneously Lena signs v8 adding a role for Jun. Two valid, signed, versioned "manifest v8" artifacts exist. Half the cohort trusts one, half the other. `role.query` — defined as "a read of signed, versioned cohort state" — now returns two different truths for the same query, and the consent matrix (which frames are admissible) has split-brained. Nothing in the rule was violated.
- **Unit B ("cohort-founder key signs"):** exactly one designated cohort-owner key may sign; other operators submit change requests out-of-band. No fork possible — but Unit B's manifests are *unverifiable* by Unit A hosts (A expects any pinned member-operator key to be a valid issuer; B expects exactly one) and vice versa. Two vendors shipping A and B cannot join one cohort even though both pass `check-cohort-mesh`'s "sign/verify/version bump round-trip" — the round-trip is self-referential and never exercises the cross-issuer case.

**Tightened rule:** The manifest carries a **single cohort-authority key** (or explicit k-of-n multisig set) declared *inside manifest v1 at cohort genesis*; only that authority signs re-issues; version is a strictly monotonic integer allocated by the authority; a member Host MUST refuse any manifest not signed by the genesis-declared authority and MUST refuse version regressions (`ECohortManifestFork` naming both versions). Journaling: the re-issue is journaled to the *authority's* TL, and every member journals its own *acceptance* of v(n+1) to its local TL (giving per-member observable adoption, which ADV-052-4 needs anyway). Add a concurrent-re-issue negative control to `check-cohort-mesh`.

### ADV-052-2 [HIGH] — Manifest distribution/refresh is entirely unspecified; a revoked member is honored indefinitely by stale hosts

**Written rule:** membership changes are re-issues; role queries are answered "from the manifest." Nothing says how a re-issue *reaches* the other 7 hosts, or how stale a host's copy may be.

**Incompatible pair:**
- **Unit A (pull/file):** each Host reads a local manifest file; the operator distributes new versions out-of-band (scp, config management). Staleness window: unbounded. Member M, revoked in v8, keeps its pinned cert and roles honored by any host still on v7 — forever, if an operator forgets one box. All consent checks pass, all signatures verify; the mesh is running two consent matrices with no error surfaced anywhere.
- **Unit B (push/frame):** the issuer pushes v(n+1) as an A2A frame on re-issue. But §7.2 (Story 8.8) is *unconditionally fail-closed on unclassified intent*: a `cohort.manifest-update` frame is DENIED at both seams unless its intent class is in the allowlist — and the allowlist lives in… the manifest being updated. Unit B either invents an out-of-band bootstrap channel (a de-facto fifth protocol, which the ADR's own "Prevents" clause forbids) or pre-seeds a mandatory intent class no rule requires. Unit A and Unit B hosts in one cohort disagree on whether a manifest-update frame is even legal traffic.

**Tightened rule:** Commit distribution explicitly: manifest v(n+1) propagates as a **reserved, always-allowlisted intent class** (`cohort.manifest.reissue` — present in every manifest by schema requirement, exactly like `retract`'s capacity-bypass carve-out in §7.1), pushed by the authority to all members, with pull-on-connect as fallback. Define a **staleness ceiling**: a host that cannot confirm it holds the latest version within T_stale (propose: the §7.2 partition NACK default, 30s, × a small factor) marks its cohort links degraded and refuses *new* consent-sensitive frames under the stale matrix (fail-closed, consistent with Story 8.8's posture). `check-cohort-mesh` gets a stale-member leg: revoke a member in v(n+1), prove the revoked member's frames are refused mesh-wide within T_stale.

### ADV-052-3 [HIGH] — "Per-(peer, role)" never says WHOSE role, or which role of a multi-role member

**Written rule:** "ADR-012's intent-class allowlists extend from per-peer to per-(peer, role) tuples carried in the cohort manifest." The §15.2 diagram shows `task.assign (intent ∈ allowlist(peer=J, role=coder))` — where `coder` is the *receiver's* (Jun's) role. But §7.2's two-seam evaluation (sender send-allowlist at `prepare_outbound`, receiver accept-allowlist at `handle_intake`) makes the mirror reading equally valid: the tuple keys on the *sender's* role ("what a coder may send").

**Incompatible pair:**
- **Unit A (receiver-role keyed):** allowlist entry `(peer=J, role=coder) → {task.assign}` means "I may send task.assign to J when addressing J's coder role." Sender evaluates against the receiver's manifest-declared roles; receiver evaluates against its own.
- **Unit B (sender-role keyed):** the same TOML bytes mean "peer J, acting in role coder, may send me task.assign." The consent matrix is the *transpose*. A frame Unit A admits, Unit B rejects with `EIntentDenied`, and both cite the same manifest line. Worse: Marcus holds two roles (Atlas + tech-lead). Does a frame match if *any* of his roles admits it (Unit A′), or must the frame *declare* the acting role and match exactly (Unit B′)? Any-role matching quietly re-opens the ADR-012 confused-deputy gap one level up — a member consented-to only as `wireframe` exercises its `coder` role's allowlist because the tuple lookup ORs across roles.
- Additionally: neither unit knows *when* the tuple binds. Sender on v7 (J has role coder) passes `prepare_outbound`; receiver already on v8 (role revoked) denies at `handle_intake`. Is that a consent violation to alarm on, or benign version skew? Unspecified — one implementation pages the operator, the other silently drops, and the in-flight-frame question the hot-swap choreography raises (frames in flight across a `drain → swap → re-pin`) has no answer at all.

**Tightened rule:** Commit the tuple semantics in the ADR text: **the role is the counterparty's manifest-declared role as seen from the evaluating seam** (sender checks `(receiver_peer, receiver_role)` in its send-allowlist; receiver checks `(sender_peer, sender_role)` in its accept-allowlist — both directions explicit in the manifest schema, no transposition ambiguity possible because send and accept tables are separate, as §7.2 already models). Multi-role members: **the frame's consent envelope carries the single acting role**; match is exact, never any-role OR (this is ADR-012's own rationale extended). Version skew: frames carry the sender's manifest version; a receiver on a *newer* version evaluates under its own (fail-closed wins); mismatch beyond ±1 version → typed refusal `ECohortManifestSkew`, distinct from `EIntentDenied` — same taxonomy discipline as Story 8.8's `-32009` vs `-32001` split. In-flight frames at role change: drained under the version they were admitted under; new admissions under v(n+1) only.

### ADV-052-4 [MED] — Migration-chain path selection is nondeterministic when the migrator graph is not a simple chain

**Written rule:** "the kernel chains single-step ADR-020 migrators hop-by-hop (v0.5→v0.7→v1.0), refusing with `EMigratorMissing` naming the specific missing hop; `maosctl swap --plan` surfaces the chain."

**Incompatible pair:** a Spirit author ships both `v0.5→v0.7` + `v0.7→v1.0` *and* (later, as an optimization) a direct `v0.5→v1.0` migrator. Now the graph is a diamond.
- **Unit A (shortest path):** picks `v0.5→v1.0`, one hop.
- **Unit B (declared-order / longest-verified chain):** picks `v0.5→v0.7→v1.0`.
Both "chain single-step migrators hop-by-hop." Migrators are arbitrary state transforms — the two paths can produce *different post-swap state* (the v0.7 hop may normalize a field the direct migrator preserves). In a cohort where members upgrade at different cadences, host 1 (Unit A) and host 2 (Unit B) migrate the same Spirit version pair to *divergent states*, and `maosctl swap --plan` on the operator's workstation can disagree with the chain the executing kernel actually runs — the plan is advisory prose, not a pinned artifact.

**Tightened rule:** The migrator set per Spirit MUST form a **linear chain** (each version has at most one outgoing migrator); registering a second outgoing migrator for the same source version is a manifest-validation error, not a runtime choice. (Alternative if diamonds are ever wanted: deterministic selection = fewest hops, tie → refuse; but linear-chain-only is the smaller rule and matches "repeated ADR-020, near-zero kernel delta.") `swap --plan` output must hash the resolved chain, and the kernel refuses to execute a chain whose hash differs from the plan's (`EMigrationPlanDrift`) — extends ADR-036's precondition check as the ADR already gestures at.

### ADV-052-5 [MED] — Halt receipt-presence has no defined emission point, and one natural implementation is denied by the fail-closed consent posture

**Written rule:** "the cohort surface is receipt-presence observability across members — the 11.2b per-region receipt-presence pattern generalized to per-member."

**The gap:** 11.2b's receipt-presence is observed *store-internally* over Loom rows in a substrate all regions share. A J3 cohort has **no shared store** (Loom collective tier is optional and Reza-scoped; J3 is 8 private Hosts + a digest Spirit). So "generalized to per-member" has no defined emission point:
- **Unit A (mesh frame):** each member emits a `halt.receipt` frame to peers. But per Story 8.8, an unclassified intent class is DENIED at both seams — unless every cohort manifest happens to allowlist `halt.receipt` for every pair, receipt observability *silently self-destructs*, and the gate's "receipt-presence per member under one induced member loss" leg can pass at N=8 in the lab (where the test manifest allowlists it) while a production manifest that omits the class shows zero receipts and no error.
- **Unit B (local TL + scrape):** each member journals halt receipts locally; the digest Spirit (or `maosctl`) scrapes each member's TL out-of-band. No consent interaction — but also no liveness: a member that halts *and loses connectivity* is indistinguishable from a member that never halted, which is precisely the case receipt-PRESENCE observability exists to catch (absence must be observable, 11.2b's whole point).

**Tightened rule:** Name the emission point in the ADR: halt receipts are **journaled locally (I2) and shipped as a reserved, schema-mandatory allowlisted intent class** (same reserved-class mechanism as ADV-052-2's manifest-update — one carve-out list, two entries), consumed by the digest Spirit; the observability assertion is "for each member, either a receipt frame or an explicit transport-level absence marker (NACK/timeout, §7.2's 30s) within T" — so absence is a first-class observable, not a missing row. State explicitly that receipt-presence frames are *observability, not arbitration* (the ADR already says arbitration is the Director's — keep that sentence, it's load-bearing).

---

## 2. ADR-053 — Multi-tenant Loom

### ADV-053-1 [BLOCKER] — "Re-attested cross-team write" has no per-team key: within one region, the crypto boundary is vacuous and two units will build incompatible signing schemes

**Written rule:** "cross-team sharing is an explicit, consented, re-attested write into the *other* team's database (the 11.2a `CrossRegionReplicationBundle` pattern applied cross-team), never a shared table."

**The gap:** ADR-049's entire verification chain is keyed **per-REGION**: `derive_region_signing_seed(base_seed, region)` / `derive_region_pubkey(claimed_source_region)` — the HKDF `info` mixes the *region tag*, nothing else. Reza's three teams (security, support, data) can — and in the single-org Cortex normally will — share a region. Apply the 11.2a pattern literally cross-team within one region and the bundle is signed under `derive_region_signing_seed(region)` and verified under `derive_region_pubkey(same region)`: **the destination team verifies the source team's bundle under a key the source team's infrastructure — and every other same-region team — can equally produce.** ADR-049 §2's own words are the indictment: a key the verifier cannot bind to the claimed source makes "forgery a one-liner." The cross-region weld made silent copying *structurally impossible*; the cross-team application of the same pattern makes it structurally *trivial* intra-region. The "Prevents" clause ("two teams' convergence proofs entangling") is defeated by the ADR's own cited mechanism.

**Incompatible pair:**
- **Unit A (reuse region keys, letter of the rule):** bundles signed/verified per-region as shipped. Team attribution is a plaintext field with no cryptographic binding; the guard chokepoint plus operator honesty is the whole wall. `check-multi-tenant-loom`'s "foreign-team row without valid re-attestation refused" leg passes — against forgeries by *outsiders*, while any same-region insider team forges "valid" re-attestations at will.
- **Unit B (mint per-team keys):** extends the HKDF derivation with a team tag (`derive_team_signing_seed(base_seed, region, team)`), signs `CrossTeamReplicationBundle` under team keys. Cryptographically sound — and wire-incompatible with Unit A: A's bundles fail B's `verify_bundle` (wrong derivation), B's fail A's. Also greenfield key machinery no document authorizes, touching `maos-audit`'s ratified 9.4b derivation (`REGION_INFO_PREFIX` is frozen `ascii-v1`).

Corollary shape-clash: `canonical_kv_leaf` (ADR-049 §4) has **no team field**, and §4's rule is explicit — "every persisted crossing column must participate in the leaf pre-image; a silently-excluded column collapses divergent states to one leaf and is a defect." If cross-team crossing adds a `source_team` column (it must, for the provenance stamp), Unit B puts it in the leaf pre-image (new leaf format, breaks byte-compat with 11.2a leaves) and Unit A leaves it out (violates §4's own defect definition). Per-team Merkle independence — the gate's third leg — is then computed over two different leaf grammars depending on which unit you bought.

**Tightened rule (pick one, state it in the ADR):**
1. **Per-team key weld (recommended):** define `derive_team_signing_seed` as a *second* HKDF stage over the region seed with a frozen team-tag grammar (mirrors 9.4b exactly; new `TEAM_INFO_PREFIX`, versioned); `verify_bundle` for cross-team bundles derives the pubkey from `(claimed_region, claimed_team)`, never from bundle contents. `source_team` becomes a persisted crossing column and **enters the leaf pre-image** via a `canonical_kv_leaf` v2 (versioned domain tag; 11.2a v1 leaves untouched — byte-compat preserved by construction, the 9.2b idiom). — *or* —
2. **Honest downgrade:** state that intra-region cross-team isolation is **guard + consent + physical datname separation only, with NO cryptographic team boundary**, and strike the "re-attested" word for the same-region case (re-attestation is real only when a region boundary is crossed). Then the threat model in `loom-threat-model.md` (§15.6 — correctly ordered *before* this ships) must carry same-region insider-team forgery as an accepted risk, in writing.
Either closes the hole; shipping the current text ships Unit A while reviewers believe they ratified Unit B.

### ADV-053-2 [HIGH] — Team↔region↔database mapping has three plausible owners and the spec names none

**Written rule:** "operator-assigned Postgres instances"; "a team's database lives in its region"; "team→region placement composes with the 11.2a residency machinery."

**Incompatible pair:**
- **Unit A (mapping in the cohort manifest):** teams are roles-writ-large; the ADR-052 signed manifest carries `team → {region, datname, connection}` — versioned, signed, journaled. Placement changes are manifest re-issues.
- **Unit B (mapping in loom-lite store config):** the mapping lives where `region_guard` precedent lives — store-internal config next to the connection pool, operator-edited, unsigned, unjournaled.
Both are "operator-assigned." Now compose a failure: team-data moves from region-A to region-B. Unit A's manifest re-issues; Unit B's store config on one Postgres host is not updated. `team_guard` (Unit B) checks incoming writes against its *stale local* mapping — every check passes, the gate's physical-absence leg was green at install time, and team data is now resident in the wrong region with zero red anywhere. Two owners of one entity (the mapping) is the classic two-writers hole; the spec licenses both.

**Tightened rule:** **One owner:** the team↔region↔datname mapping is a section of the signed cohort/org manifest (ADR-052's artifact — Reza's cross-team topology already lives there per §10.7.2(e)); `team_guard` loads it *only* from the manifest artifact, verifies the signature at load, caches by manifest version, and refuses reads/writes when its cached version trails the announced current version (`ETenantMapStale`, fail-closed — same posture as ADV-052-2). Store-local config may name connection credentials but never team membership or placement. `check-multi-tenant-loom` gains a stale-map leg: re-issue the mapping, prove the old placement refuses within the staleness ceiling.

### ADV-053-3 [HIGH] — Dual-team Spirit membership: guard keyed on identity vs connection admits opposite behaviors, and one of them is "a shared table with extra steps"

**Written rule:** the guard is "a store-internal `team_guard` chokepoint below `CollectiveMemoryPort`" and isolation is "physical (a team's rows live in a database the other team's connection string cannot name)."

**Incompatible pair:**
- **Unit A (identity-keyed):** `team_guard` resolves `spirit_pid → team` through a single-valued map; a Spirit belongs to exactly one team; a dual-membership request is a config error. Cross-team flow is forced through the consented re-attested write path. Strict, matches the "boundary stays loud" intent.
- **Unit B (connection-keyed):** the guard keys off which connection/`datname` the call arrived on — the literal reading of the "connection string cannot name" rule. A Spirit provisioned with *two* connection strings (one per team, which nothing forbids — Reza's Orchestrator "unifies recommendations across team-owned Spirits" and is the obvious candidate) reads team A and writes team B **through the front door of each guard**, no consent, no re-attestation, no bundle, no audit of the crossing. Every letter of the rule is satisfied: each team's rows live in a database the *other team's* connection string cannot name — this Spirit simply holds both strings. The "never a shared table" prohibition is defeated by a Spirit that *is* the shared table.
A third divergence hides inside: capability-token-keyed (the guard trusts the capability envelope's team claim) — different again, and the token issuer becomes an unnamed fourth owner of team membership.

**Tightened rule:** Team membership is a property of **Spirit identity, declared in the signed manifest, single-valued** (one team per `spirit_pid`; multi-team access is *only* via the cross-team consented path — a Spirit needing both teams' data is two Spirits with an ADR-012-consented channel between them, which is exactly the substrate's shape). `team_guard` MUST verify `(spirit_pid → team)` against the manifest mapping *and* that the connection in use is the one assigned to that team (belt and braces: identity is authoritative, connection mismatch is a loud typed error `ETenantConnectionMismatch`, never a silent allow). Add a gate leg: a dual-connection Spirit attempting a foreign-team read through its second connection is refused with the crossing audited. Also answer the row-ownership question in the ADR: a re-attested copy in team B's database **is team B's row** for Merkle, capacity, and GDPR-erasure purposes, with `source_team` provenance for forensic and cascade purposes (forget-cascade across a shared copy otherwise has no owner — I-9.2 erasure spine needs to know whom to cascade to).

---

## 3. ADR-054 — FR37 vetting machinery

### ADV-054-1 [HIGH] — Attestation binds the manifest hash; every Spirit upgrade changes the hash — tier survival is unspecified and the two readings are both dangerous

**Written rule:** "`VettingAttestation` is an Ed25519-signed envelope binding (manifest hash, from-tier, to-tier, vetter key id, expiry, revocation semantics)."

**Incompatible pair:**
- **Unit A (exact-hash binding):** the attestation is valid for one manifest hash. Any upgrade — a patch release, even a whitespace-touched manifest — yields a new hash with no attestation: the Spirit drops from `public-vetted` to `public-untrusted` at the strictest-of floor. Consequences cascade into §15.2's own choreography: a cohort hot-swap of a vetted Spirit (`drain → swap → re-pin`, with multi-step migration chains whose *intermediate hops* are also distinct manifest hashes) hits the admission floor mid-chain and refuses. Vetting flaps on every release; operators respond by pinning old versions (defeating the security purpose) or by pressuring vetters into rubber-stamp re-issuance.
- **Unit B (loose binding):** reads "manifest hash" as artifact-family identification and binds `(publisher key, name, version-range)`. Upgrades keep the tier — and code the vetter never saw runs at `public-vetted`. This is the supply-chain hole vetting exists to close.
Both units serialize an envelope with a field called `manifest_hash`; they are wire-compatible and *semantically* incompatible — the worst kind of pair, because interop testing won't catch it.

**Tightened rule:** Exact-hash binding is correct — keep it, and close the flap explicitly: the attestation envelope gains an optional **`successor_policy`** issued by the vetter (e.g. `exact-only` | `re-issue-required-with-expedited-review`), and the ADR states that **upgrade-without-current-attestation = admission refusal at the floor, by design** (the flap is the feature). For the hot-swap interaction: the admission check evaluates the *target* version's attestation **before** the chain starts (fold into `maosctl swap --plan`'s precondition, ADR-036) — never mid-chain; intermediate migration-hop states are execution steps of one admission decision, not separately-admitted artifacts (state this sentence in the ADR; it is the difference between Unit A being livable and being a mid-swap I14 violation).

### ADV-054-2 [HIGH] — Expiry/revocation semantics vs already-installed running Spirits: refuse-at-next-load and runtime-revoke are both compliant, and the runtime one contradicts zero-kernel-Δ

**Written rule:** the envelope carries "expiry, revocation semantics"; "revocation rides the existing CRL/yank path, FR59-distinguishable from operator-local revocation"; "kernel admission is unchanged"; "Zero kernel-Δ expected."

**Incompatible pair:**
- **Unit A (load-time only):** tier is evaluated exactly where the ADR says the kernel already reads it — at admission. An installed, running Spirit whose attestation expires (or is revoked) at t keeps running at `public-vetted` until its next load, potentially unbounded (founder-class Spirits run for weeks). "Revocation" revokes future installs only. Fully compliant with "kernel admission is unchanged."
- **Unit B (runtime revoke):** the registry/compliance layer watches expiry+CRL and force-terminates or tier-demotes running instances. Compliant with the plain meaning of "revocation" — but there is no out-of-kernel lever that demotes a *running* Spirit's effective tier: sandbox tier and capability floor were fixed at admission. Unit B therefore either (a) grows a kernel hook (blows "zero kernel-Δ expected"), or (b) abuses the yank/kill path for expiry — conflating three distinct events the ADR itself demands stay FR59-distinguishable (vetting-revocation ≠ registry-yank ≠ operator-local revocation; expiry-lapse is a *fourth* state the audit output must now also distinguish, and the ADR's list has three).
Two deployments, same registry, same envelopes: on one, a revoked Spirit dies within seconds; on the other it runs for a month. Auditors reading "revocation is journaled" will assume Unit B while operators run Unit A.

**Tightened rule:** Commit the semantics in the envelope, not in implementation folklore: `revocation_semantics ∈ {refuse-at-next-load, drain-and-refuse}` where **v2.2 ships `refuse-at-next-load` only** (honest about the zero-kernel-Δ constraint), *plus* a mandatory **journaled expiry/revocation observation event** at the compliance layer the moment the condition is detected while an affected Spirit is running ("Spirit X running at tier T with lapsed/revoked attestation A since ts") — the state is surfaced, never laundered (the exact dropped-audit-orphan discipline of ADR-049 §7, reused). `drain-and-refuse` (runtime action via the existing hot-swap/drain machinery, *not* a new kernel hook) is named as the v2.5 upgrade slot next to accredited external vetters. Audit output distinguishes all four terminal causes: vetting-revocation / expiry-lapse / registry-yank / operator-local.

### ADV-054-3 [MED] — Vetter-key registration is not itself journaled or bound to any trust root; "internal vetter key" is whoever holds a keypair

**Written rule:** "issuance, verification, and revocation are journaled to the Transparency Log" — key **registration** is conspicuously absent from that list; "internal vetter keys" is undefined beyond the adjective.

**Incompatible pair:**
- **Unit A (config-file trust):** valid vetter keys = a TOML list in registry config, operator-edited, unjournaled. Adding a vetter key is invisible to the TL; a compromised registry host (or a careless config commit) mints a vetter, back-issues attestations, and every one of them verifies and journals *cleanly* — the journal proves issuance by a key nobody can show was ever legitimately enrolled.
- **Unit B (journaled enrollment):** vetter keys are enrolled via a signed, journaled, revocable registration event chained to an operator root (the 9.4b TL-derived key or the operator audit key of §7.3's sealed-export). Verification of an attestation checks the chain: attestation sig → vetter key → journaled enrollment → root.
A's attestations verify at A (no chain check) and fail at B (no enrollment record); B's verify everywhere but A never produces enrollment records for B to check. Cross-org verification (`check-vetting-attestation`'s "round-trip on a clean host") silently tests only whichever model the test fixture assumes.

**Tightened rule:** Vetter-key lifecycle is **first-class and symmetric with attestations**: enrollment, rotation, and revocation of a vetter key are each Ed25519-signed events (signed by the operator audit key — the §7.3 sealed-export root, already trusted by external verifiers), journaled to the TL, and `verify` MUST walk attestation → vetter-key enrollment → operator root, refusing attestations whose vetter key has no journaled enrollment predating issuance. Add a forged-vetter-key negative control (unenrolled key, valid signature) to the gate next to the existing forged-signature control — it is the control the current gate list is missing.

---

## 4. ADR-055 — Post-v2.0 constitutional ceiling

### ADV-055-1 [BLOCKER] — "Kernel-crate-set ≤ 25 KLOC" has no pinned membership and two live, disagreeing counting regimes; the gate is red or green depending on which unit you build

**Written rule:** "Kernel-crate-set aggregate (residual core + extracted kernel crates) ≤ 25 KLOC through v3.0, alarm at 23.5K"; gate = "`check-kernel-baseline` + `xtask/kloc.toml` aggregate."

**Gap 1 — membership.** `xtask/kernel-crates.toml` exists but it is the **check-loom orchestration-symbol scan list** (`crates = ["maos-kernel-core"]`), owned by a different gate with different semantics — it does not pin the ceiling set, and reusing its name invites exactly the wrong file being read. "Extracted kernel crates" is unpinned prose:
- **Unit A (ADR-041 Phase-3/4 set):** {residual core ≤6,000, maos-scheduler ~1,961, maos-memory ~1,659, maos-hot-swap ~1,317, maos-supervision ~569} ≈ **11.5 KLOC** — sails under 25K with 13K of unmonitored headroom.
- **Unit B (all decomposition-plan extracts):** Unit A + maos-iac (5,736) + maos-manifest (4,012) + maos-capability (~1,271) ≈ **22.5 KLOC** — brushing the 23.5K alarm.
- **Unit C (everything the kernel links for its invariants):** Unit B + maos-domain (8,045 — it holds the ports and region types the kernel is generic over) ≈ **30.5 KLOC** — red on day one.
All three are defensible readings of "residual core + extracted kernel crates." A constitutional ceiling whose set is chosen by the implementer is not a ceiling; it is a menu.

**Gap 2 — counting regime.** The two instruments the gate names *measure the same crate differently today*:
- `kernel-core-baseline.toml` (`check-kernel-baseline`): raw `src/` line count, **including doc comments and in-`src/` test code** by its own HISTORY notes ("+59, incl. test LOC… remainder is doc comments + test code the line-count gate includes") → `maos-kernel-core = 23,081`.
- `kloc.toml` (kloc-check): "production Rust code only. Excludes: target/, tests/, benches/, examples/…" → the same crate measured **17,687**.
Δ ≈ 5,400 LOC (23%) *on one crate*. §15.5's own text quotes the 23,081 figure while pointing the aggregate gate at kloc.toml's regime. Under baseline-counting, kernel-core **alone** sits 419 lines under the 23.5K alarm — the ceiling is consumed on arrival and clause 3's decomposition becomes a prerequisite the ADR never sequences; under kloc-counting there is ~7.8K of headroom. Two teams implement "the 25K aggregate gate," both cite the ADR, one blocks every merge and the other blocks none. The exact failure mode Story 8.16 §A4 existed to kill — numbers locally true, never summed *in the same units* — reappears at constitutional level.

**Tightened rule:** (a) Create **`xtask/kernel-crate-set.toml`** (new file, distinct from the check-loom list) as the single pinned membership: name every member crate explicitly at ratification (recommend Unit B's set — it is what "the kernel's trusted computing base" honestly means post-ADR-041 — but *any* explicit set beats prose), with the same "to change this you MUST…" header discipline as `kernel-core-baseline.toml`, changes FLAG-Winston only. (b) Pin the regime: **the 25K/23.5K numbers are measured in kloc.toml units** (production-only, tests/benches excluded) because those are the units the aggregate gate already computes; state in the ADR that `check-kernel-baseline`'s src_lines is the *drift tripwire* (different instrument, different number, both listed with their regime named — never compared to each other or to the 25K figure). (c) Sequence clause 3 vs clause 4 explicitly: the 25K ceiling **binds at v2.2-wave close** (after Phase-3/4 extraction), with the alarm live from ratification in advisory mode — the §A7.5 WOULD-HAVE-BLOCKED banner idiom, already ratified for exactly this situation.

### ADV-055-2 [HIGH] — Clause 2 ("per-crate ceilings are not raised") contradicts the lived mechanism clause 1 ratifies: retros have raised them repeatedly, on the record

**Written rule:** clause 1 ratifies "the lived mechanism" as the constitutional instrument; clause 2 states "ADR-038 per-crate ceilings are not raised."

**The gap:** the lived mechanism *includes* retro-time ceiling re-pins. `kloc.toml`'s Epic-10 retro block records, in one table: maos-kernel-core 17,000→17,750, maos-iac 5,500→5,800, maos-manifest 4,000→4,050, maos-domain 7,000→8,100, maos-cli 2,000→3,750, aggregate hardfail 90,000→103,000 — each "set to the TIGHT measured residual" per "the established epic-retro process." Meanwhile the same file's Epic-5 header says "DO NOT raise the ceiling."

**Incompatible pair:**
- **Unit A (literal clause 2):** the gate hard-refuses any ceiling edit in kloc.toml. The next epic retro — which per the established process reconciles ceilings to measured residuals — breaks the build, and the retro process the ADR claims to constitutionalize is the thing the gate forbids.
- **Unit B (lived-mechanism clause 1):** retro-time tight-measured re-pins continue as before. Then clause 2 is decorative, and nothing distinguishes a disciplined tight re-pin from the "raise it to fit" move ADR-038 exists to forbid — the words are identical in the diff.
Both units are faithful to half the ADR. A constitutional text that contradicts itself in adjacent clauses will be arbitraged clause-by-clause.

**Tightened rule:** Replace clause 2's absolute with the actual discipline, named: **"per-crate ceilings move only (a) downward at any time, or (b) at epic retro, to the tight measured residual (+≤1% slack), with the measured value and driver recorded in the same commit — never to round headroom, never mid-epic, never to accommodate planned growth."** That is the rule Epics 5–10 actually lived; writing it down makes Unit A's gate implementable (mid-epic ceiling edits refuse; retro edits must carry the measured-residual annotation) and closes Unit B's laundering channel. Companion rule for **new** crates minted by Phase-3/4 extraction: initial ceiling = measured LOC at extraction +≤1%, recorded in the extraction commit — otherwise a generous new-crate ceiling is the raise clause 2 forbids, wearing a new name.

### ADV-055-3 [MED] — Clause 5's tenancy hand-off is sound but names no gate; the "outside the kernel" claim is only as strong as ADV-053-3's guard key

Clause 5 (single-tenant-per-kernel expires at v2.0; multi-tenancy arrives outside the kernel via §15.3) is the one clause with no incompatible pair *provided* ADR-053's holes close — a dual-connection Spirit (ADV-053-3 Unit B) is precisely a tenancy assumption re-entering through the side door while the kernel's posture claims "unchanged." **Tightened rule:** clause 5 cites `check-multi-tenant-loom` as its enforcement (the kernel's tenancy posture is unchanged *because* the tenant wall is proven at the store gate), making the constitutional claim gate-backed rather than narrative — one cross-reference line.

---

## 5. Cross-cutting observations (non-blocking)

- **The reserved-intent-class mechanism is needed twice** (ADV-052-2 manifest propagation, ADV-052-5 halt receipts) and should be specified once: a short schema-mandated list of always-allowlisted cohort-infrastructure intent classes, mirroring §7.1's `retract` capacity-bypass precedent. Without it, Story 8.8's (correct) fail-closed posture eats the cohort's own control traffic in any manifest an operator authors by hand.
- **ADR-052's manifest is becoming the org's identity spine** (members, roles, consent tuples, per ADV-053-2 also team↔region mapping). That is the right home — one signed versioned artifact — but it means manifest-fork/staleness (ADV-052-1/2) is the single highest-leverage seam in the whole §15 design: every other ADR's guard reads it. The tightened rules above are cheap; the fork they prevent is not.
- **Gate self-referentiality:** three of the four proposed gates (`check-cohort-mesh` round-trip, `check-vetting-attestation` round-trip, `check-multi-tenant-loom` guard-red) as currently worded can be passed by a single implementation testing itself. Each should include at least one leg where the artifact is produced by one code path and consumed by an independently-derived verifier — the ADR-049 "genuinely independent of the write codec" discipline, cited once and reused.

## 6. Findings index

| ID | Severity | ADR | One-line |
|---|---|---|---|
| ADV-053-1 | BLOCKER | 053 | Intra-region cross-team re-attestation reuses per-REGION keys → crypto boundary vacuous; per-team key derivation vs region-key reuse are wire-incompatible units; `source_team` in/out of leaf pre-image splits the Merkle oracle |
| ADV-055-1 | BLOCKER | 055 | Kernel-crate-set membership unpinned (kernel-crates.toml is the check-loom list, not this) + two live counting regimes disagree by ~5.4 KLOC on kernel-core alone (23,081 vs 17,687) → gate red or green by implementer's choice |
| ADV-052-1 | HIGH | 052 | 8-operator cohort, no single manifest-version owner → concurrent re-issue forks the consent matrix; any-signer vs founder-key units can't join one cohort |
| ADV-052-2 | HIGH | 052 | Distribution/staleness unspecified → revoked member honored indefinitely; push-unit's manifest-update frame is denied by fail-closed unclassified consent |
| ADV-052-3 | HIGH | 052 | per-(peer,role) never says whose role → transposed consent matrices; multi-role any-match reopens confused-deputy; version-skew on in-flight frames unanswered |
| ADV-053-2 | HIGH | 053 | Team↔region↔datname mapping has 3 plausible owners (manifest / store config / policy) → stale-map residency violation with all gates green |
| ADV-053-3 | HIGH | 053 | Guard keyed on identity vs connection → dual-connection Spirit is a consent-free shared table satisfying every written rule |
| ADV-054-1 | HIGH | 054 | Manifest-hash binding vs Spirit upgrade: exact-hash unit flaps tier on every release (and mid-hot-swap); loose unit runs unvetted code at vetted tier |
| ADV-054-2 | HIGH | 054 | Expiry/revoke vs running Spirits: load-time-only vs runtime-revoke both compliant; runtime unit contradicts zero-kernel-Δ; 4 terminal causes, ADR distinguishes 3 |
| ADV-055-2 | HIGH | 055 | "Ceilings not raised" (clause 2) contradicts the ratified lived mechanism (retros raised 6 ceilings on record) → gate either breaks every retro or enforces nothing |
| ADV-052-4 | MED | 052 | Migrator diamond → shortest-path vs declared-order units migrate the same pair to divergent states; swap --plan not binding on the executing chain |
| ADV-052-5 | MED | 052 | Halt receipt-presence has no emission point; mesh-frame unit is consent-denied, TL-scrape unit can't observe absence |
| ADV-054-3 | MED | 054 | Vetter-key registration not journaled/rooted → config-file-trust unit accepts attestations the journaled-enrollment unit refuses; forged-vetter-key control missing from gate |
| ADV-055-3 | MED | 055 | Clause 5 tenancy hand-off names no gate; claim depends on ADV-053-3 closing |

**Disposition recommendation:** ratify ADR-052/053/054/055 at party-mode **conditional on** folding the 14 tightened rules above into the ADR texts (they are rule-tightenings, not design changes — no fork choice made in §15 is overturned by any finding). ADV-053-1 requires one explicit fork decision (per-team key weld vs honest downgrade) that belongs on the party-mode agenda as a named fork, not a silent default.
