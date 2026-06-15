# AG-1: Air-Gap Deployment Runbook

> Story 9.4 — deploying MAOS in network-isolated (air-gapped) environments.

## Overview

An **air-gap deployment** runs the MAOS Host binary with all network surface
compiled out at build time (`--features air-gap`). The resulting binary contains
zero networking code — no HTTP clients, no TCP listeners, no DNS resolvers.

## Prerequisites

| Item | Detail |
|------|--------|
| Rust toolchain | stable ≥ 1.88 |
| Build flags | `--no-default-features --features air-gap` |
| CI gate | `xtask check-air-gap` passes on the produced binary |
| Host access | No outbound network required |

## Build Procedure

```bash
# 1. Build the air-gap binary
cargo build -p maos-bin --release --no-default-features --features air-gap

# 2. Verify no network symbols leaked
cargo xtask check-air-gap \
  --binary target/release/maos \
  --dirty-fixture target/debug/dirty-network-fixture

# 3. (Optional) Run the netns corroborating harness (requires root/CAP_SYS_ADMIN)
sudo bash tests/air-gap-netns-corroborate.sh target/release/maos
```

## Host-Level Network Enforcement

Even with compile-time network surface removal, defense-in-depth mandates
host-level enforcement:

### Option A: Network Namespace Isolation

```bash
# Run the daemon inside a network namespace with no interfaces
unshare --net -- ./target/release/maos init
```

### Option B: Firewall Rules (iptables / nftables)

```bash
# Block all outbound traffic from the maos user
iptables -A OUTPUT -m owner --uid-owner maos -j DROP
iptables -A INPUT  -m owner --uid-owner maos -j DROP
```

### Option C: SELinux / AppArmor

Confine the `maos` binary to a profile that denies `network` access class.

## Spirit Import (Offline)

Air-gapped environments cannot pull Spirits from a registry. Use the offline
import flow (Story 7.2):

```bash
# On a networked machine: export a signed Spirit bundle
maosctl export --spirit hello-spirit --output hello-spirit.tar.gz

# Transfer to air-gapped host via removable media

# On the air-gapped host: import and verify
maosctl install --source ./hello-spirit.tar.gz
```

The import verifies:
- Ed25519 signature over the SHA-256 manifest
- Bundle integrity (all listed files present, hashes match)
- Trust-tier floor (operator policy enforced)

## Capability-Token Guidance

In air-gap mode:
- `Scope::NetworkOutbound` tokens are **never issued** (no network surface exists)
- `Scope::RegistryPoll` tokens are **never issued** (no registry client)
- Local-only scopes (`Scope::MemoryRead`, `Scope::FileRead`, etc.) work normally
- Spirit scheduling, journaling, and audit continue unchanged

## Transparency Log

The Transparency Log (SQLite) operates identically in air-gap mode. All frame
emissions, governance events, and audit proofs are recorded locally. The log
can be extracted via `maos audit query` for external review.

## Limitations & Honest-Risk Acknowledgment (R8-AG)

> **Honest-risk disclosure**: The air-gap build removes the network *surface*
> at compile time. It does NOT guarantee that a compromised Spirit (or a
> malicious dependency pulled at build time) cannot attempt I/O through
> non-network channels (e.g., filesystem, IPC, signals). The air-gap feature
> is a **layer** in a defense-in-depth strategy, not a standalone security
> boundary.

Specific limitations:
1. **No live registry sync** — Spirits must be imported offline
2. **No remote inference** — LLM providers are unreachable; Spirits requiring
   inference will receive `ProviderError::Unconfigured`
3. **No MCP over HTTP** — MCP servers must use stdio transport or be unavailable
4. **No A2A cross-Host** — TCP/mTLS transport is compiled out; only in-process
   loopback (if compiled) is available
5. **No mobile push** — Halt notifications cannot reach mobile devices

## Verification

The `xtask check-air-gap` gate (R-AG1) runs in CI and:
1. Builds the air-gap binary
2. Scans symbol table with `nm --demangle` for network-related symbols
3. Fails if any are found
4. Validates a dirty fixture (a binary that DOES link `TcpStream::connect`)
   is correctly rejected — proving the gate is not vacuous

The `tests/air-gap-netns-corroborate.sh` script (R-AG3) provides a
corroborating runtime check under `unshare -n`, but is **not merge-blocking**
(requires root/CAP_SYS_ADMIN, environment-fragile).
