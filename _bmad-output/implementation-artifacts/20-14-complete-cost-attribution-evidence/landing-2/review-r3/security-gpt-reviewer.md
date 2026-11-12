## Verdict: correct

**New findings: P0 0 · P1 0 · P2 0 · P3 0. Confidence: 0.94.**

No patch-introduced security blocker found in `456793ca..9996fe09`, with `edeedd25..9996fe09` reviewed as context. Both previous findings are addressed end to end.

### Correction closure

- **Completed-forget replay:** `crates/maos-bin/src/operator_door.rs:2388–2393` now calls `subject_in_frame`, sharing the decoded predicate used by discovery, replacement and certification. Newly appended encoded cost identities invalidate the historical success instead of bypassing another sweep. Principal/reason-bound replay keys, receipt validation, legal-hold checks and conservative tail limits remain intact.
- **Erasure field interpretation:** `crates/maos-iac/src/adapter/transparency_log.rs:2319–2336` decodes only cost `principal_id` and `task_ref` strings. Provider/model labels and governance strings retain literal matching. Consequently, literal label `x:61` matches `61`, while encoded principal `x:61` matches `a`.

### Security assessment and rejected candidates

| Candidate | Assessment |
|---|---|
| Rebind another principal’s token scope | Rejected. Shard-local `OnceLock::set` permits one successful bind; narrow kernel delegates expose no replacement or validity mutation. Production roots bind before returning the minted token. |
| Collide `alice` with literal `x:61:6c:69:63:65` | Rejected for current typed writers. `encode_identity` encodes every input beginning `x:` again; decoding occurs once. The literal and `alice` remain distinct. |
| Inject JSON or forge a scope through principal text | Rejected. Identity values pass through serde string serialization; task and principal scope originate at root mint sites, not request/frame fields. |
| Replay another subject’s completed forget | Rejected. The replay key includes length-delimited principal bytes and reason presence/content; the saved report must also match the requested principal and certified completion marker. |
| Introduce production serde unwrap/default handling | Rejected. No newly introduced production serde call ends in the prohibited unwrap/expect/default chain. New serde `expect` calls are test assertions; the gate explicitly excludes test code. |

### Existing deferred limitations

The six round-2 deferred rows remain recorded at `_bmad-output/implementation-artifacts/deferred-work.md:1533–1540`; none warrants a new finding here:

- Invisible-character refusal is **not exhaustive**.
- The reader mirrors the session-principal length cap, not all boot refusals.
- Concurrent binding lacks a dedicated witness; `OnceLock` supplies the structural guarantee.
- Production token eviction remains absent.
- Unversioned codec-shaped legacy literals remain ambiguous.
- KLOC adjudication is an accounting matter, not a security guarantee.

**Audit posture:** colon-hex encoding is reversible identity preservation, not encryption or secret redaction. It deliberately preserves credential-shaped principal identifiers through the existing filter; operators must not treat these fields as a safe destination for actual credentials.

### Evidence and verification limits

Reviewed regression witnesses cover encoded-principal replay invalidation, unchanged token dimensions, and literal-versus-encoded erasure. Recorded evidence contains three failing plants with restoration claims and a zero-violation serde gate result.

**No builds, tests, gates, plants or file writes were performed.** Recorded execution/restoration claims were inspected, not independently reproduced. No additional corrective witness is required for this patch; previously deferred coverage gaps remain open.