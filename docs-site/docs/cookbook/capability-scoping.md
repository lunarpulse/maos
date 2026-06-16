---
title: Capability Scoping
sidebar_position: 6
description: Requesting and using capability tokens to access kernel services safely.
---

# Capability Scoping

## Problem

Your Spirit needs to call an inference provider, send IAC frames, or access an MCP tool server. MAOS enforces Invariant I1: Spirits cannot bypass the Capability Registry. Every privileged operation requires a capability token declared in the manifest and mediated at runtime.

## Solution

Declare required capabilities in your manifest:

```toml
[capabilities.required]

# Inference provider access
[capabilities.required.provider]
complete = ["anthropic/claude-3"]

# MCP tool-server access
[capabilities.required.mcp]
[[capabilities.required.mcp.servers]]
name = "search-server"
tools = ["web_search", "document_search"]
```

At runtime, use the `CapabilityHandle` from `Ctx` to mediate calls through the kernel:

```rust
use maos_spirit_abi::lifecycle::Spirit;
use maos_spirit_abi::ctx::Ctx;

pub struct InferenceSpirit;

impl Spirit for InferenceSpirit {
    fn on_idle(&self, ctx: &mut Ctx) {
        if ctx.cancellation().is_cancelled() {
            return;
        }

        // The capability handle is an opaque u64 the kernel resolves
        // at mediation time. You never construct tokens yourself.
        let cap_handle = ctx.capability();

        // The mailbox handle lets you send/receive IAC frames —
        // also mediated through the kernel's capability registry.
        let mailbox = ctx.mailbox();

        // Use cap_handle and mailbox with the Spirit SDK's typed
        // wrappers to make inference calls, send frames, etc.
        // The kernel verifies every call against the declared scopes.
    }
}
```

## Discussion

Capability scoping is the mechanism behind MAOS Invariant I1 ("No Spirit bypasses the Capability Registry"). The flow is:

1. **Declaration** — the manifest's `[capabilities.required]` section lists every scope the Spirit needs. At admission, the kernel converts these to `Scope` values (`Scope::ProviderInfer`, `Scope::McpCall`, etc.).

2. **Issuance** — the kernel's Capability Registry issues tokens for the declared scopes and binds them to the Spirit's principal namespace. The Spirit sees only an opaque `CapabilityHandle(u64)`.

3. **Mediation** — every privileged SDK call passes the handle to the kernel, which resolves it to the actual token and checks the scope. A call outside the declared scope returns a `CapError`.

4. **Revocation** — the kernel can revoke tokens at any time (operator action, policy change, Spirit misbehaviour). The manifest's `[on_revocation]` section declares how the Spirit responds.

**Scoping principles:**

- Request the **minimum** set of capabilities your Spirit needs. The kernel logs every capability issuance to the transparency log.
- Prefer **named tools** over wildcard access — `tools = ["web_search"]` rather than omitting the `tools` field.
- The `trust_tier` in `[class]` affects which capabilities can be issued. A `local` Spirit can access local providers; accessing remote endpoints may require `community` or `audited` tier.
