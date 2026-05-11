# Appendix F — Distillation Pattern Body

§9.5 in the main body sketches the substrate interface (the five contracts the kernel honors so the pattern works). This appendix gives the **full Spirit-author convention prose** — the implementation guidance that does not need to live in the binding-v0.5 document body. Conventions here are non-binding; the binding floors are the five-metric gate in §9.5.

## F.1 — Reference implementation (hermes-agent)

[hermes-agent's `trajectory_compressor.py`](https://github.com/NousResearch/hermes-agent/blob/main/trajectory_compressor.py) is the canonical reference for distillation-after-execution. Its compression strategy:

- Protect first turns (system, human, first gpt, first tool) — uncompressed.
- Protect last N turns (final actions and conclusions) — uncompressed.
- Compress only middle turns into a single human summary message.
- Effective-temperature tuning per compressor model.
- Target-token-budget enforcement.
- Percent-sample compression for long trajectories.

MAOS distillation Spirit-authors should adopt this shape unless they have a domain-specific reason not to.

## F.2 — The pattern, step by step

1. **Raw frame lands in the Transparency Log.** Invariant I2 — the kernel does this regardless of distillation.
2. **Spirit-side LLM distillation** compresses the raw payload into a decision-relevant digest. The Spirit chooses the summarization model, the prompt, the token budget, and the redaction policy.
3. **Digest is written to working memory** (in-process Spirit state — no kernel involvement). Optionally elevated to **episodic memory** (`fs.write` to private namespace) for cross-session retention or **shared/collective memory** (existing `memory.share` capability) for inter-Spirit dissemination. Per Invariant I11, every persisted digest carries `source_log_ref` and `distillation_depth`; the kernel rejects digest writes that lack these fields.
4. **Active LLM context** contains digests + decisions + recent I/O + queued external input. Raw payloads are *not* in active context.
5. **Raw is recalled on demand** via `log.recall` when a downstream decision needs full evidence. Recall is auditable (recall-of-recall chain).
6. **Decisions record their digest grounding.** Invariant I12 — every `decision.*` frame the Spirit emits carries `working_memory_digest_refs` so post-hoc audit can prove which summaries the agent actually reasoned over.
7. **User-input queuing.** Human-originated frames arriving during in-flight work are buffered by the Spirit's persona logic and processed at safe sequence points (between task completions, before new dispatches) — preventing preemption of in-flight delegations.

## F.3 — Multi-hop generalization

Digests of digests compound information loss. `source_log_ref` flattens transitively at write time so any digest at any hop references the *original raw frames*, not intermediate digests. Auditors and Spirits walk a single hop from any digest back to raw evidence. `distillation_depth` is monotonic; Spirits may decide policy on max acceptable depth (e.g., halt-and-escalate at depth 3+).

## F.4 — Hermes-informed conventions

**First-turn / last-turn anchoring.** Distillation-shipping Spirits SHOULD preserve the original task statement (the first turn that initiated the work) and the final output (the closing turn that delivered the result) uncompressed in the digest. Compress only the middle. The v0.5 ship-gate test corpus measures task-preservation via cosine-similarity ≥0.95 between the digest's task-statement section and the original task statement.

**Target token budget.** Distillation-shipping Spirits SHOULD declare `target_max_tokens` per distillation invocation; default `max(2048, 0.15 × original_tokens)`, overridable per Spirit class via manifest `[distillation].target_max_tokens`. Compression ratios outside `[0.05, 0.25]` (relative to original) indicate either a compressor that is dropping content (too aggressive) or not compressing (too conservative); the v0.5 ship-gate flags both.

**Compressor model class.** Distillation reliability is downstream of compressor model quality. Spirit-author convention: the compression LLM call SHOULD use a model class ≥ Sonnet-tier or 70B+ open-weights, with temperature ≤0.3.

## F.5 — Acceptance criteria — derivation

This appendix derives the metric floor values whose normative current-version specification appears in §9.5 (Distillation Pattern interface, Table 9.5-1). Reference §9.5 for the values that govern conformance; this appendix explains how they were chosen and how to re-derive them when the threat model or operational data changes.

**Why these five metrics, not three or seven?** Recall, faithfulness, and traceability are the irreducible triple — without recall the digest is useless, without faithfulness the digest is misleading, without traceability the digest is unauditable. Hedge-preservation and secret-leakage were added when the bounded test populations (10⁴/10⁵) revealed two specific failure modes the irreducible triple did not catch: (a) digests that flatten "possibly X" into "X" pass faithfulness checks but degrade decision quality downstream, and (b) digests that summarize secret-bearing raw frames may leak the secret pattern even after pre-write redaction catches the literal token.

**Why ≥0.90 for digest-recall?** Held-out replicator LLM was tuned against the v0.5 R&D corpus; ≥0.90 is the threshold above which inter-replicator-LLM disagreement (the noise floor) dominates over true digest-quality variance. Below 0.90, the metric still discriminates between bad and good digests; at-or-above, the metric becomes noise-limited. ≥0.90 is therefore the highest meaningful floor.

**Why ≥0.98 for faithfulness, not ≥0.95 or ≥0.99?** Faithfulness measures unflagged contradictions — a stricter floor than recall because false positives here propagate into downstream decisions silently. ≥0.98 was chosen because the judge-LLM's own false-flag rate on the v0.5 R&D corpus measured ~0.5%; allowing a 2% unflagged contradiction window (1 - 0.98) leaves headroom above the judge's noise floor while gating real contradictions. ≥0.99 would be tighter than the judge-LLM's own resolution.

**Why ≥0.95 for hedge-preservation, gated on IAA ≥0.85?** Hedge-preservation is the only metric that requires inter-annotator agreement because hedges are linguistically ambiguous ("might be" vs "could be" vs "appears to" — different annotators score these differently). The IAA ≥0.85 floor (Cohen's κ) ensures the gold corpus is itself reliable before the metric becomes load-bearing. Below IAA 0.85, the hedge-preservation score is calibrated against noise. ≥0.95 on the metric itself is then the achievable target conditional on a reliable corpus.

**Why 0% for secret-leakage, not "≤N"?** Same argument as §7.2.1's zero-data-plane-error floor on mTLS rotation: any non-zero error budget on a security-critical path creates an incentive to suppress the metric rather than fix the underlying issue. Zero is absolute.

**Corpus size derivation.** 10 hedge-preservation cases is the minimum for the IAA computation to converge on the binary hedge/no-hedge label set. 10 contradiction cases is the minimum for the judge-LLM to discriminate above its own noise floor. 10 planted-secret cases is the minimum for the kernel's pre-write redaction to be exercised against each major secret class (API keys, capability tokens, mTLS private-key bytes — three classes × ≥3 instances). The 10⁵ secret-leakage corpus is a separate scaling assertion: at 10⁵ frames the false-negative rate of the redaction filter has measurable confidence intervals.

For current floor values and the canonical metric table, see §9.5.

## F.6 — Intent provenance interaction (I13)

Every digest also carries `intent_lineage: [intent_class, ...]` — the union of `intent` field values from all input frames it summarizes, computed by the kernel from `log.recall` tracking. Consumers operating under intent `Y` admit the digest only if `intent_lineage ⊆ allowed-promotion-set(Y)` declared in their manifest (typed error `EIntentPromotionDenied` on rejection). This closes consent-laundering through distillation: data received under `consult` cannot be silently re-purposed under `delegate` via a digest hop. The mechanism is kernel-side (not Spirit-self-reported), which is what prevents the asymmetric-enforcement gap.

---

*Architecture is the practice of arranging trade-offs so future-you can change your mind without burning everything down. Thirty-nine ADRs, fourteen invariants, ten open questions. The substrate ships in six phases, terminating at v1.5 with the diagnostic-architect bilateral pair operational. The kernel grows slowly so the ecosystem can grow fast. The hermes-tenant positioning claim cashes at v1.0 through capability tokens, the Transparency Log, the Approval Decision Log, the Spirit registry with three trust tiers, the ComplianceClaim envelope, the cross-Spirit memory isolation corpus, and the externally-verifiable uninstall receipt. Same primitives compose, by configuration alone, into single-user laptop deployments and diagnostic-architect production pairs — without architectural rewrite at any tier transition.*
