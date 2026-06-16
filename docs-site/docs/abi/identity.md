---
title: identity
sidebar_position: 5
description: "SpiritId, HostId, FrameKind, and SpiritRole — wire-stable identity and frame discrimination types."
---

# `identity` Module

The identity module provides wire-stable identity primitives for per-Spirit mailbox routing (Story 3.1) and cross-Host A2A addressing (Story 6.3). Wire-stable since v0.1-β.

## SpiritId

Unique Spirit identifier — String newtype keyed on PID-rebind safety. Per architecture §7.1, every IAC frame carries a `spirit_id` field for per-Spirit routing. The newtype prevents accidental use of a bare `String` where a `SpiritId` is required.

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct SpiritId(pub String);
```

### Methods

| Method | Signature | Description |
|---|---|---|
| `as_str()` | `fn as_str(&self) -> &str` | Borrow as string slice |

### Example

```rust
use maos_spirit_abi::identity::SpiritId;

// Construct from &str or String
let id = SpiritId::from("my-spirit");
let id2 = SpiritId::from(String::from("my-spirit"));
assert_eq!(id, id2);
assert_eq!(id.as_str(), "my-spirit");
```

## HostId

Host identifier — stable across process restarts. Same-Host routing uses `None` for `FrameAddress.host_id`; cross-Host A2A (Story 6.3) fills `Some(host_id)`.

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct HostId(pub String);
```

### Methods

| Method | Signature | Description |
|---|---|---|
| `as_str()` | `fn as_str(&self) -> &str` | Borrow as string slice |

### Example

```rust
use maos_spirit_abi::identity::HostId;

let host = HostId("node-east-1".into());
assert_eq!(host.as_str(), "node-east-1");
```

## FrameKind

Frame-kind discriminator for the IAC bus. The canonical source of truth for the frame kind taxonomy per architecture §7.1. Wire-stable since Story 1b.1.

```rust
#[repr(u8)]
pub enum FrameKind {
    // IAC bus frame kinds (0..=6)
    TaskAssign = 0,
    TaskComplete = 1,
    DecisionDispatch = 2,
    EpistemicHalt = 3,
    TelemetryEvent = 4,
    ConsentRequest = 5,
    Retract = 6,

    // Kernel-internal audit kinds (7..=9) — NOT routed through IAC
    CapabilityInvocation = 7,
    SandboxBlock = 8,
    InferenceCall = 9,

    // Extended kinds (Epic 6)
    CliSubprocessOutput = 21,  // Story 6.2 — CLI subprocess stdout/stderr
    ConsentRupture = 22,       // Story 6.4 — ADR-034 consent rupture event
    RateLimited = 23,          // Story 6.4 — token bucket exhaustion
    GatewayInbound = 24,       // Story 6.5 — inbound from external gateway
    GatewayOutbound = 25,      // Story 6.5 — outbound to external gateway
}
```

### Methods

| Method | Signature | Description |
|---|---|---|
| `from_u8(v)` | `fn from_u8(v: u8) -> Option<Self>` | Parse a `u8` discriminant into a `FrameKind`, returning `None` for unknown values |

### Example

```rust
use maos_spirit_abi::identity::FrameKind;

// Parse from wire format
let kind = FrameKind::from_u8(0);
assert_eq!(kind, Some(FrameKind::TaskAssign));

// Unknown discriminant returns None
let unknown = FrameKind::from_u8(99);
assert_eq!(unknown, None);

// Match on frame kind in a handler
fn handle_frame(kind: FrameKind) {
    match kind {
        FrameKind::TaskAssign => { /* dispatch task */ }
        FrameKind::GatewayInbound => { /* handle external message */ }
        FrameKind::EpistemicHalt => { /* trigger halt protocol */ }
        _ => { /* other frame kinds */ }
    }
}
```

## SpiritRole

Role for role-based IAC addressing. The kernel's channel class router resolves Spirit identity from a `SpiritRole` when the sender targets a role rather than a specific Spirit.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum SpiritRole {
    Director,
    Observer,
    Worker,
    Orchestrator,
}
```

v0.3-β supports these four Director-surface roles; the full role ontology ships in Story 6.1.

### Example

```rust
use maos_spirit_abi::identity::SpiritRole;

let role = SpiritRole::Worker;

match role {
    SpiritRole::Director => { /* orchestration logic */ }
    SpiritRole::Observer => { /* monitoring logic */ }
    SpiritRole::Worker => { /* task execution logic */ }
    SpiritRole::Orchestrator => { /* multi-Spirit coordination */ }
}
```
