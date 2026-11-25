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

**Shipped state (Story 21-7):** the daemon installs one production consent granter and `ConsentGate` over each admitted Spirit's declared `[consent.recall]` (recipient spirit id → `full`, `header_only` or `withheld`). The granter is the authenticated sender: its outbound frames carry a daemon-stamped envelope and a guest-supplied one is discarded. A sender that declares grants has any local addressee it did not grant refused as `ConsentNotGranted`; a sender with no declaration is unrestricted. A retraction reaches every recorded recipient that is still the admitted incarnation; cross-host retract is unsupported. "Consent" here is a technical recall and delivery restriction, not legal consent, a GDPR lawful basis, or counsel-reviewed. Peer-identity ruptures have no envelope and withhold nothing. Compiled-in Spirits have no bus-send primitive, and the unbound bus port refuses a Spirit sender rather than journal it at pid 0. The Postgres-backed collective tier does not carry this consent state. Cross-wall recall remains emitter-scoped under ADR-058 D2.

**Shipped state (Story 22-9, proposed schema 9):** a `[consent]` table is admitted only at manifest schema 9 or later, and a `[consent.recall]` recipient the Host does not know (not installed, not in its cast or topology, not loaded) is refused `consent_recipient_unknown`, whatever the load order. A hot swap that auto-reverts restores the predecessor's declaration together with its scopes and promotion set, unless another incarnation has since installed its own. The guest-visible refusal is still `IntentAllowlistMismatch`: the WIT `rupture-reason` has no `consent-not-granted` case (21-7 F7 stays a claim boundary).

**Scope of the guarantee (Story 21-7):** recall-only. The restriction governs what a recipient can later read back through the log (`recall`, `fetch`); it does not recall or hide a frame already delivered to the recipient's own session.
