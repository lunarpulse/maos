# Appendix D — Terminal-Shape Sketches

This appendix describes terminal shapes — directions the architecture is biased toward but has not committed to. **Nothing in this appendix is binding.** Sections in the main body (§0–§14) reference appendix entries by ID (e.g., D.3) when a v0.1-or-later decision deliberately leaves room for one of these shapes.

The convention is simple: **if a behavior is in §0–§14, it is binding for its declared phase. If a behavior is in App-D / E / F, it is non-binding by construction.** A reader in the main body never has to ask "is this real?" — if it is in §0–§14, it is real.

## D.1 — Multi-host topologies beyond bilateral

Bilateral A2A (exactly two pre-paired Hosts, mTLS+TOFU) is the v1.5 commitment. Triadic and N-host meshes have appeared in cohort discussions (gateway-mediated, supervisor-fanout, peer-discovery DHT) but no journey in scope demands them. If a future deployment justifies one, the substrate's primitives (typed-intent consent, mTLS pinning, logical clock) extend additively. The wire format does not change.

## D.2 — In-process Rust Spirits unlocked via measurement gate

The v0.1 commitment is subprocess-only (ADR-002). The full measurement gate spec — harness, latency budgets per journey, Prometheus alert rules, three-condition unlock check — is **canonically specified in §13.1**. This appendix entry exists only to record the *terminal-shape* implication: if the gate trips and a superseding ADR lands, MAOS gains a second Spirit form (`rust-inproc`) without altering ABI for existing subprocess Spirits. ADR-031 (Cross-Form Spirit Equivalence) is `speculative-vNext`; resolution depends on this gate firing.

Normative specification: §13.1.

## D.3 — Cognitive predicate vocabulary beyond universal arithmetic

The kernel exercises only four universal-arithmetic predicates (`on_value_above`, `on_value_below`, `on_value_within`, `on_value_outside`) per ADR-022. Spirit classes with richer cognitive needs (Negotiator comparing scalars to scalars; Tutor needing trajectory-shape predicates) may, in some future phase, justify a stdlib of additional predicates loaded from user-space. The substrate's current position: the universal-arithmetic surface is sufficient for the journeys this architecture ships. If a future Spirit class justifies an extension, the kernel surface stays untouched and the predicates live in a user-space stdlib (the ADR-039 number is reserved for that proposal; see §12).

## D.4 — Federation tier between `org-internal` and `public-untrusted`

The three-tier trust model (ADR-009) covers `local`, `org-internal`, `public-untrusted`. A federation tier — Spirits authored by one organization and consumed by a partnered organization — has appeared in compliance discussions. It would slot between `org-internal` and `public-untrusted` with a peer-organization signing key pinned in operator policy. No journey in scope exercises this; the slot is reserved by structure, not by code.

## D.5 — Hot-swap migration patterns beyond single-step

ADR-020 specifies single-step `migrate(predecessor_state) -> successor_state`. Multi-step migration (e.g., `v0.5 → v0.7 → v1.0` via two intermediate hops) would be useful when Spirit classes evolve schemas faster than operators upgrade kernels. The substrate's invariant (kernel refuses load with `EMigratorMissing`) extends to multi-step trivially via chained migrators; the operator UX for chain composition is the open question.
