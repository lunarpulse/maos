---
Status: binding-v0.1
Phase: binding-v0.1
Gate: binding-v0.1 (types only at v0.1; runtime at v0.9) | A2A Gateway rejects frames with intent not in allowlist (at v0.9)
Decided: 2026-04-15
Accepted-in-PR: <PR_NUMBER>
Revisits: §7.2, §13 v0.9 row
---

# ADR-012 — Typed-intent A2A consent

**Decision.** Cross-Host A2A consent is `(peer-identity, intent-class)`, not `(peer-identity)`. A read-only Spirit cannot pass a payload to a writeable Spirit that, when interpreted, causes a write the read-only Spirit was forbidden from.

**Rationale.** Channel-consent does not imply transaction-consent. The confused-deputy class of attacks at the inter-Spirit boundary requires intent-class scoping. Mira's `diagnosis-handoff:read-only-evidence` is admissible at Nash; `code-mutation-directive` is rejected. Without ADR-012, Mira could trigger a Nash-side action she cannot trigger directly.

**Alternatives considered.** Channel-consent only (rejected: leaves the confused-deputy gap open). Typed-intent at the IAC bus layer for ALL frames (considered: more uniform; rejected because cross-Host frames are where the trust boundary actually is, and same-Host IAC frames already inherit the kernel's process-internal trust).

**What would force a revisit.** A workload pattern emerges where intent-class cardinality grows pathologically.

## Implementation note — Story 17-8

`ConsentEnvelope.recall` is a daemon-side recall restriction, not a WIT field or a guest-granted permission. `Full` is the default, `HeaderOnly` lists metadata with `payload-available = false` but refuses fetch, and `Withheld` (including unknown serialized values) omits the frame. Expiry and a later matching grant-scoped rupture also omit it. These restrictions bind recorded recipients other than the sender; fetch returns `consent-withheld` and writes `log.fetch.refused:consent-withheld`. Both native and WASM forms use the same kernel-mediated participant recall (sender OR recorded recipient, this boot); role never confers participation. The lossless-frame exception is ratified as `17-8-consent-recall-policy`.

**Production limits (AC3.6 / rule 11):** no production granter at HEAD sets `HeaderOnly`/`Withheld`, no production `ConsentGate` is installed, and envelope-bearing frames reach admitted local recipients only through A2A delegation intake. Peer-identity ruptures have no envelope and withhold nothing. Native compiled-in bus sends still journal at pid 0 (non-goal). Cross-wall recall remains emitter-scoped under ADR-058 D2.
