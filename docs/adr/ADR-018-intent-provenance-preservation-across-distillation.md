## ADR-018 — Intent provenance preservation across distillation (introduces I13)

`Status: binding-v0.5` · `Gate: kernel-computed intent_lineage; consumer admission rejects with EIntentPromotionDenied` · `Decided: 2026-04-15` · `Revisits: §3.2 invariant I13; §9.5`

**Decision.** Add invariant I13. The kernel computes `intent_lineage` from input frame_ids on every digest write — the union of intent classes of all input frames the digest was distilled from. A consumer that operates under intent `Y` rejects digests whose `intent_lineage` is not contained in `allowed-promotion-set(Y)` declared in the consuming Spirit's manifest. Producer-side enforcement is kernel-computed (not Spirit-self-reported).

**Rationale.** Closes consent-laundering through distillation: data received under `consult` cannot be silently re-purposed under `delegate` via a digest hop. Kernel-computed (not Spirit-self-reported) closes the asymmetric-enforcement gap.

**Alternatives considered.** Make intent_lineage advisory (rejected: makes I13 advisory; consent laundering becomes silent the moment one Spirit forgets to propagate). Track intent_lineage at the IAC bus layer for ALL frames, not just digests (considered: more uniform, but explodes header overhead for frames that never cross consent boundaries).

**What would force a revisit.** A workload pattern emerges where intent_lineage cardinality grows pathologically.
