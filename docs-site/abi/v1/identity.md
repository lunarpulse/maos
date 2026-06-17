<!-- AUTO-GENERATED from maos-spirit-abi rustdoc — do not edit; regenerate via: cargo run -p xtask -- gen-abi-docs -->

# `identity` Module {#abi-identity-module}

## Related {#abi-identity-related}

- [ADR-010](https://github.com/lunarpulse/maos/blob/main/docs/adr/ADR-010-address-typing.md) — `SpiritId` / `HostId` newtype rationale
- [architecture §7.1](https://github.com/lunarpulse/maos/blob/main/_bmad-output/planning-artifacts/architecture-maos-minimal-opus/7-inter-agent-communication.md#71-same-host-the-mailbox) — IAC frame-kind taxonomy


*ABI_VERSION = 1 · MANIFEST_SCHEMA_VERSION = 3*

Spirit + Host identity types + FrameKind discriminator — wire-stable since v0.1-β.

These are the v0.3-β identity primitives for per-Spirit mailbox routing
(Story 3.1) and cross-Host A2A addressing (Epic 6 Story 6.3).


## Enums {#identity-enums}

### `FrameKind` {#maos-spirit-abi-identity-framekind}

Frame-kind discriminator — wire-stable since Story 1b.1.

The canonical source of truth for the IAC frame kind taxonomy per
architecture §7.1. Variants 0..=6 are IAC bus frame kinds; variants
7/8/9 are kernel-internal audit kinds that do NOT flow through the
IAC router.

# Example {#maos-spirit-abi-identity-framekind-example}

```rust
use maos_spirit_abi::identity::FrameKind;

let kind = FrameKind::from_u8(0);
assert_eq!(kind, Some(FrameKind::TaskAssign));

let unknown = FrameKind::from_u8(99);
assert_eq!(unknown, None);
```


```rust
pub enum FrameKind {
    TaskAssign,
    TaskComplete,
    DecisionDispatch,
    EpistemicHalt,
    TelemetryEvent,
    ConsentRequest,
    Retract,
    CapabilityInvocation,
    SandboxBlock,
    InferenceCall,
    CliSubprocessOutput,
    ConsentRupture,
    RateLimited,
    GatewayInbound,
    GatewayOutbound,
}
```

### Inherent Items {#maos-spirit-abi-identity-framekind-inherent-items}

Methods and associated functions implemented directly on this type.

### `from_u8` {#from-u8}

Parse a `u8` discriminant into a `FrameKind`, returning `None` for unknown values.

# Example {#from-u8-example}

```rust
use maos_spirit_abi::identity::FrameKind;

assert_eq!(FrameKind::from_u8(0), Some(FrameKind::TaskAssign));
assert_eq!(FrameKind::from_u8(25), Some(FrameKind::GatewayOutbound));
assert_eq!(FrameKind::from_u8(99), None);
```


```rust
pub fn from_u8(v: u8) -> Option<Self>
```

### `SpiritRole` {#maos-spirit-abi-identity-spiritrole}

Role for role-based IAC addressing.

Architecture §7.1 comment: per-frame-kind channel class router resolves
Spirit identity from a `SpiritRole` when the sender targets a role rather
than a specific Spirit. v0.3-β supports the four Director-surface roles
enumerated here; the full role ontology ships in Story 6.1.

# Example {#maos-spirit-abi-identity-spiritrole-example}

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


```rust
pub enum SpiritRole {
    Director,
    Observer,
    Worker,
    Orchestrator,
}
```
