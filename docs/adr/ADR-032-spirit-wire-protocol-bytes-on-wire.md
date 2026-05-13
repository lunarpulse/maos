---
Status: binding-v0.1
Phase: binding-v0.1
Gate: binding-v0.1 (types only; full byte-equal corpus at v1.0) | byte-equal golden corpus per frame variant per SDK
Decided: 2026-04-15
Accepted-in-PR: <PR_NUMBER>
Revisits: §5.2
---

# ADR-032 — Spirit Wire Protocol bytes-on-wire

**Decision.** LSP-style `Content-Length` framing over stdout: `Content-Length: <decimal>\r\n\r\n` followed by exactly N bytes of CBOR-encoded payload. Header is ASCII, case-insensitive name, max header block 4 KiB. Stderr reserved for diagnostics; never multiplexed onto stdout. EOF after a clean frame = `Halt::Voluntary`; mid-frame EOF = `Halt::Fault(Truncated)`. Backpressure via credit-based windowing on bounded `mpsc<Frame>(64)`.

**Rationale.** LSP framing is well-understood and implementations are abundant. CBOR is compact, language-neutral, schema-evolved. The framing details are spelled out so subprocess implementations across languages produce byte-equal output.

**Alternatives considered.** Newline-delimited JSON (rejected: large payloads break easily on partial newline encoding). Raw JSON-RPC without length prefix (rejected: parser ambiguity on partial frames).

**What would force a revisit.** A use case emerges where Content-Length framing cannot represent the message structure cleanly.
