# 20-14 L2 correction review — round 2

**2 findings: 1 P1, 1 P2. Verdict: incorrect.**  
Full report follows because no permitted file-writing tool is available. No builds, tests, plants or gates were rerun.

## Findings

### 1. [P1] Invalidate forget replay for newly encoded cost identities

**Location:** `/home/lunarpulse/dev_ws/crates/maos-domain/src/cost.rs:175–180`  
**Confidence:** 0.99

The new serialization preserves principals such as `sk-alice` reversibly, but the completed-forget replay consumer still searches raw payload bytes. After a successful door-mediated forget, append another cost frame for that principal and repeat the identical forget while the tail remains below 4,096 rows. `/home/lunarpulse/dev_ws/crates/maos-bin/src/operator_door.rs:2387–2392` cannot find `sk-alice` inside its `x:` encoding, so `completed_forget` returns the historical successful report and `forget_command` bypasses the new sweep. The newly stored, recoverable identity remains despite an “erased” response.

**Exact fix:** Make completed-forget tail invalidation use the same kind-aware, decoded-value subject predicate used by erasure discovery and certification. Preserve the existing retained-receipt exclusions and bounded-tail behavior.

**Witness needed:** Extend `ac5_replay_invalidates_for_new_matching_frames_and_changed_reason`: complete a forget for `sk-alice`, append a genuinely serialized schema-2 cost frame, repeat the identical request, and require a new operation, `replayed: false`, and a redacted principal with unchanged token dimensions.

### 2. [P2] Decode only fields that actually use the identity codec

**Location:** `/home/lunarpulse/dev_ws/crates/maos-iac/src/adapter/transparency_log.rs:2316–2319`  
**Confidence:** 0.99

The changed string arm applies `decode_identity` to every traversed value, including all ordinary kind-28 strings and kind-29 provider/model labels. Those fields are not codec-encoded by their writers. For example, a cost frame whose raw model is `x:61` previously matched principal `61` under the existing token-boundary predicate; now the walker tests decoded `a`, misses the subject, and leaves the original `x:61` untouched. Conversely, forgetting `a` can scrub that unrelated literal label. Discovery, replacement and certification share this erroneous interpretation, so certification does not catch the missed erasure.

**Exact fix:** Carry field/path context into the walker. Decode only codec-bearing cost `principal_id` and `task_ref` values; preserve raw matching for provider/model labels and governance values.

**Witness needed:** Cover literal `x:61` in both a cost model and an ordinary governance string. Forgetting `61` must scrub them; forgetting `a` must not. Retain encoded-principal/task erasure coverage.

## Prior-finding closure

| Prior concern | Judgment |
|---|---|
| Wide `tokens()` accessor | **Closed.** It is removed. Production callers use the two narrow delegates, which expose neither issuance nor validity/quota mutation. |
| Historical-map scan | **Closed.** Binding performs one lookup in the selected shard; no separate scope map or historical scan remains. |
| Redactor-shaped identities | **Original vectors fixed, integration incomplete.** Typed serialization, report decoding and ordinary erasure work; the two findings above remain. |
| Principal length/invisible hardening | **Partially closed.** The byte cap and listed ranges are implemented; “all invisible/Cf characters refused” is broader than the implementation. |
| Grant inconsistency | **Clarified, not fully approved.** Records now distinguish the adjudicated 1,444 capability figure from pending re-adjudication at 1,454. |

## Token-state and race assessment

- `OnceLock::set` correctly permits exactly one successful concurrent bind. Neither delegate exposes `take`, replacement, or mutable state access.
- Binding an **unbound revoked token** still succeeds. This cannot un-revoke it, refill quota, or bypass verification. Current production roots bind freshly issued tokens before returning them; no newly reachable authorization bypass was found.
- Eviction takes the shard write lock, so it cannot remove state during a bind/read holding the read lock. Scope leaves with the state.
- **Production eviction remains absent.** Scope lifetime therefore remains process lifetime alongside retained token state; this redesign removes the duplicate map and quadratic work, not historical retention.
- Paid completion after revocation is handled correctly: attribution remains readable, cost is journaled, and the post-call recheck denies the response.
- The lifecycle test covers sequential duplicate binding and eviction, not competing binds. OnceLock provides the structural race guarantee; a concurrent-bind witness is absent.

## Codec and compatibility assessment

- Current lowercase-hex detection matches the redactor’s lowercase-hex heuristic.
- All 16 current prefix rules are covered, including overlapping `sk-` rules and both PEM headers. Quotes and backslashes force encoding.
- New typed values beginning `x:`—valid hex or otherwise—are themselves encoded, preventing new-writer ambiguity.
- Malformed codec input is **accepted unchanged**, not rejected. This is explicit behavior, not fail-closed validation.
- Codec-shaped **legacy literals** remain ambiguous because decoding is unconditional and unversioned. Supported boot/memory principal constructors reject colons, limiting principal reachability; ordinary provider/model/governance strings do not have that protection.
- Serde on `PrincipalRef::Resolved.principal_id` covers both payload and wrapper readers/writers. Wrapper `task_ref` has its own optional-field glue. Reconcile outputs decoded ordinary strings, preserving its NDJSON report shape.
- The `include_str!` guard is acceptable as a supplementary drift check: it extracts all current literal prefix rows and exercises the actual redactor. It is **not** an exhaustive behavioral oracle: source-format changes or different rule construction can escape extraction, and it does not verify erasure/replay routing.

## Principal-validation assessment

The 128-byte limit is checked in bytes, including multibyte Unicode. U+0085 is rejected through `is_control()`. The listed bidi/tag ranges are rejected.

Residuals remain: embedded U+2028/U+2029, soft hyphen and several other Cf characters, and variation selectors are accepted. The reader checks session-principal length only; it does not mirror invisible-character rejection. Plain output escapes these classes, while NDJSON preserves them. These are largely inherited hardening limitations, **not additional patch-introduced findings**; retain a Low disposition rather than claim complete Cf refusal.

Normalization/confusable handling remains exact and normalization-sensitive. No normalization requirement was established, and silently normalizing identities would change identity semantics.

The proxy length exemption is defensible: proxy principals are unbounded at their existing source, and applying the new session-only cap would discard legitimate historical spend. It is not a general resource-bound guarantee.

## Overdraft assessment

Recorded measurements agree: kernel **21,203**, capability **1,454**, domain **10,274**, IAC **8,771**, CLI **6,612**, bin **38,556**; aggregate **202,935**, hardfail **202,936**.

- Current kernel STOP is **22,000**, leaving **797**, not 47. The correction’s older STOP annotations are stale.
- Capability remains **+10 above** the adjudicated redesign figure; CLI is **2 above** its 6,610 ceiling. Domain +57 and IAC +1 remain explicitly pending.
- Narrow delegates and shard-local storage are supportable correctness costs. A plain optional scope under the existing shard write lock could avoid OnceLock; the encoder could also avoid per-byte temporary strings. Neither alternative has a measured line-count saving here.
- Green KLOC against newly raised pins establishes measurement consistency, **not approval of pending overdrafts**.

## Evidence and claim honesty

The nine plant logs contain the expected failing assertions, exit 101, and recorded identical-restoration markers. Their contents are internally consistent; those markers are recorded claims, not independently authenticated restoration evidence.

Compile-red capability/kernel logs establish missing APIs, not runtime behavior. The bin red vectors establish actual redaction/malformed-frame failures. The new same-principal/two-task witness and report-key plant now directly exercise task separation.

Recorded green results include capability **1**, kernel **6**, bin **11**, CLI **9**; domain **3** and IAC cover **1** appear green in the workspace log. Workspace outcome is correctly disclosed: **6,039 passed, 1 failed**, the 61.752-second million-row pending case.

Hosted execution, clean-tree `abi-diff`, prepush and the WASM witness remain explicitly unverified. No introduced production `unwrap`/`expect` after serde parsing was found.

**Overall correctness:** incorrect.  
**Explanation:** The authority and quadratic-binding defects are closed, but encoded identities bypass completed-forget replay invalidation, and decoding is applied to fields whose writers never encode them. Both require correction before merge.  
**Confidence:** 0.99.

**Headlines: 1 P1 — encoded identity survives repeated forget; 1 P2 — codec interpretation corrupts raw-label erasure.**
