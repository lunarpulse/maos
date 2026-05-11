# 9. Memory & Knowledge

## 9.1 The three tiers — re-cap

| Tier | Scope | Backed by | Lifetime | Use case |
|---|---|---|---|---|
| `private` | This Spirit instance | `Arc<RwLock<HashMap<...>>>` per-Spirit + `fs.write` to per-Spirit-namespaced filesystem | Spirit lifetime + episodic persistence if declared | Working memory, scratchpad, session state |
| `shared` | All Spirits on this Host | SQLite-backed key-value with namespace prefix per writer | Host lifetime | Cross-Spirit coordination on this Host (founder-loop Orchestrator-Worker handoff, peer telemetry sharing) |
| `collective` | Both Hosts in the bilateral pair | Postgres+pgvector exposed via MCP-Streamable-HTTP (Loom-lite) | Loom domain lifetime | ADR-pattern library, fix templates, regression-test references |

## 9.2 Memory file (`memory.md`)

Spirits MAY persist a `memory.md` file in their private namespace as a human-readable working memory dump. The `*.md` memory file convention is universal in the cohort (codex / openclaw / ironclaw / hermes / paperclip all use a similar pattern). It is the user's lever to read what the Spirit "remembers" and to edit it. The kernel does not interpret the file; it stores it like any other private-tier write.

## 9.3 Loom-lite — the collective tier

**Single-instance Postgres+pgvector, exposed as an MCP-Streamable-HTTP server.** The collective tier is a service the operator deploys; the kernel mediates access but does not host the data. Used by:

- The Diagnostic Engineer + Senior Architect bilateral pair for ADR-pattern lookup, fix-template retrieval, regression-test reference. Mira reads patterns to recognize known incident shapes; Nash writes new patterns when an incident produces a reusable diagnostic recipe.
- Distillation-shipping Spirits in the founder loop (Orchestrator) for cross-session digest retrieval if the operator opts in.
- Researcher for cross-session bibliography persistence.

**Schema.** The collective tier carries `kind: pattern` records (ADRs, fix templates, regression tests) with vector embeddings for similarity search. Curation is Spirit-side (Nash decides what is worth persisting); the kernel only enforces the I11/I12/I13 audit-chain invariants on writes.

**Backup / DR.** RPO ≤1h, RTO ≤4h; backup integrity verified weekly via Merkle-root cross-check.

## 9.4 Memory hot-swap

The Memory Manager swaps memory scope along with Spirit class (I6). Private memory is preserved through `swap()` (the swapping-in Spirit inherits via `on_swap_in`'s `predecessor_state` argument). For `migrate()` (cross-host bilateral), private memory is serialized into the migration payload; the receiving Host's Memory Manager rehydrates on `on_swap_in`.

`forgotten_set` semantics on swap-out: a Spirit may declare per-key TTLs in its manifest's `[memory.forgotten_set]` block; the Memory Manager garbage-collects expired keys on swap.

## 9.5 Distillation Pattern (substrate-level) — interface sketch

Spirits that aggregate from many peers — Orchestrator running an epic loop, Mira ingesting telemetry, distillation-shipping Spirits in any topology — face naive-append context overflow. The substrate's answer is a **documented pattern** built on kernel primitives, not a kernel feature. The kernel provides primitives (Transparency Log + I11 + I12 + I13 + `log.recall`); Spirit authors compose the pattern.

**Substrate interface (binding-v0.5).** Five contracts the kernel honors so the pattern works:

1. **Raw lands in Transparency Log first** (I2 — kernel writes log before any IAC delivery).
2. **Digest writes carry `source_log_ref` + `distillation_depth`** (I11 — `EDigestAuditChainMissing` on missing fields).
3. **Decision frames carry `working_memory_digest_refs`** (I12 — kernel attaches refs from declared in-context digests + shadow-recall on inbound events).
4. **Digest writes carry `intent_lineage`** computed kernel-side from input frames (I13 — `EIntentPromotionDenied` if consumer's `allowed-promotion-set` does not contain the digest's lineage).
5. **`log.recall(filter)`** is the on-demand raw retrieval API; calls are auditable.

**Acceptance floor (v0.5 ship gate, all distillation-shipping Spirits) — Table 9.5-1:**

| Metric | Floor |
|---|---|
| Digest-recall | ≥0.90 |
| Digest-faithfulness (no-contradiction) | ≥0.98 |
| Digest-hedge-preservation | ≥0.95 (requires IAA ≥0.85 gold corpus) |
| Digest-traceability | 100% (kernel-enforced via I11) |
| Digest-secret-leakage | 0% (kernel-mediated pre-write redaction) |

For the derivation of these floor values from the threat model and observed operational data — including why each metric was chosen and how to re-derive the thresholds when the threat model changes — see Appendix F.5.

**Implementation prose** — Spirit-author conventions (first-turn / last-turn anchoring, target token budget, compressor model class, hermes-agent reference implementation) and the full step-by-step pattern walkthrough live in **App-F "Distillation Pattern Body"**. The kernel does not enforce conventions; it enforces the five contracts above.
