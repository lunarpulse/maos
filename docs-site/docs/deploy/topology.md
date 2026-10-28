---
title: Deployment Topology
sidebar_position: 1
description: Deployment topologies for MAOS — single-host, multi-host A2A, air-gapped, and contained Workers (T3).
---

# Deployment Topology

MAOS supports four deployment topologies. Choose the one that matches your operational requirements.

## Single-host

The simplest topology: one MAOS kernel process running all Spirits on a single machine.

```
┌─────────────────────────────────┐
│          MAOS Kernel            │
│  ┌─────────┐  ┌─────────┐      │
│  │ Spirit A │  │ Spirit B │     │
│  └─────────┘  └─────────┘      │
│  ┌──────────────────────┐      │
│  │  Transparency Log    │      │
│  │  (SQLite)            │      │
│  └──────────────────────┘      │
└─────────────────────────────────┘
```

**When to use:** development, testing, low-throughput production workloads.

**Setup:**

```bash
# Build with default features (includes networking)
cargo build -p maos-bin --release

# Initialize and run
./target/release/maos init
./target/release/maos run
```

All Spirits communicate via in-process channels. The Transparency Log is a local SQLite database at `$MAOS_HOME/audit/transparency.sqlite`.

## Multi-host (A2A)

Multiple MAOS kernel instances communicate via the Agent-to-Agent (A2A) protocol over mTLS. Each host runs its own kernel with its own Spirits.

```
┌──────────────────┐    mTLS/A2A    ┌──────────────────┐
│   Host A         │◄──────────────►│   Host B         │
│   MAOS Kernel    │                │   MAOS Kernel    │
│   ┌──────────┐   │                │   ┌──────────┐   │
│   │ Spirit 1 │   │                │   │ Spirit 3 │   │
│   │ Spirit 2 │   │                │   │ Spirit 4 │   │
│   └──────────┘   │                │   └──────────┘   │
└──────────────────┘                └──────────────────┘
```

**When to use:** distributed workloads, cross-team Spirit isolation, geographic distribution.

**Requirements:**

- mTLS certificates for each host (mutual authentication)
- Network connectivity between hosts on the A2A port
- Each host maintains its own Transparency Log

**Configuration:**

```bash
# On each host, configure A2A peer addresses
export MAOS_A2A_PEERS="host-b.example.com:9090,host-c.example.com:9090"
export MAOS_A2A_CERT="/etc/maos/tls/host.crt"
export MAOS_A2A_KEY="/etc/maos/tls/host.key"
export MAOS_A2A_CA="/etc/maos/tls/ca.crt"

./target/release/maos run
```

## Air-gapped

A hardened topology where the MAOS binary is built with **all network surface compiled out** at build time. No HTTP clients, no TCP listeners, no DNS resolvers exist in the binary.

```
┌─────────────────────────────────┐
│      Air-Gapped Host            │
│      (no network interfaces)    │
│                                 │
│   MAOS Kernel (--features       │
│              air-gap)           │
│   ┌──────────┐                  │
│   │ Spirit A │ (offline import) │
│   └──────────┘                  │
│   ┌──────────────────────┐     │
│   │  Transparency Log    │     │
│   └──────────────────────┘     │
└─────────────────────────────────┘
```

**When to use:** classified environments, regulatory-mandated network isolation, defense-in-depth deployments.

**Key constraints:**

- Spirits must be imported offline via `maosctl install --source ./bundle.tar.gz`
- No remote inference — LLM providers are unreachable
- No A2A cross-host communication
- No MCP over HTTP (stdio transport only)
- Capability tokens for `Scope::NetworkOutbound` and `Scope::RegistryPoll` are never issued

See the full [Air-Gap Deployment](./air-gap-deployment) guide for build instructions and verification.

## Contained Workers (T3)

One host runs the MAOS daemon, which runs the kernel and first-party `rust-inproc` Spirits in-process (`T0`, capability tokens only). On Linux, WASM-component Spirits run in the `T2` runner process. Each `[cli_wrapper]` Worker runs in its own `T3` rootless container; with `--network=none`, it reaches the network only through the daemon's egress proxy at `/proxy/egress.sock`. The daemon writes to one local SQLite Transparency Log.

```
┌─────────────────────────────────────────────────────┐
│ Host                                                │
│  ┌───────────────────────────────────────────────┐  │
│  │ MAOS daemon                                   │  │
│  │ kernel + rust-inproc Spirits (T0)             │  │
│  │ egress proxy · Transparency Log (SQLite)      │  │
│  └──────┬────────────────────────────────────────┘  │
│         │ /proxy/egress.sock (one per Worker)       │
│  ┌──────▼─────────────────┐  ┌───────────────────┐  │
│  │ T3 rootless container  │  │ T2 runner process │  │
│  │ [cli_wrapper] Worker   │  │ WASM component    │  │
│  │ --network=none         │  │ Spirit (Linux)    │  │
│  └────────────────────────┘  └───────────────────┘  │
└─────────────────────────────────────────────────────┘
```

**When to use:** running third-party agent CLIs you do not trust with your credentials, home directory, or network.

**Isolation layers:**

- `rust-inproc` Spirits run in the daemon process (`T0`), mediated by capability tokens; there is no process isolation.
- On Linux, WASM-component Spirits run in the `T2` runner process with seccomp allow-list, Landlock, and a delegated cgroup v2 subtree.
- Each Worker runs in a Linux-only `T3` rootless container with `--network=none`, read-only filesystem, no host home mount, and egress only through its daemon proxy socket at `/proxy/egress.sock`.
- Each Worker run gets its own workspace, `<MAOS_HOME>/workspaces/<container_name>/`, mounted at `/workspace` (`rw,U`) as its working directory; podman chowns it to the container's mapped subuid. It is kept after the Worker exits (remove it with `podman unshare rm -rf <workspace>`), and MAOS diffs it to decide whether a real `codex` or `claude` Worker produced an effect. See the [CLI-wrapper Spirit cookbook](/cookbook/cli-wrapper-spirit).

WASM log recall adds no descriptor, socket, or syscall to T2. Requests and replies ride the runner's existing ADR-032 stdin/stdout channel; the daemon mints a capability for each admitted call and audits every call and refusal. This introduces no deployment condition or change to the T2 allow-list.

**What stays unisolated by design:**

- `rust-inproc` Spirits run in the daemon process.
- The admission handshake of the repo-built Worker fixtures runs on the host; real `codex` and `claude` binaries are not executed on the host.
- T3 Worker manifest resource limits are applied independently to CPU percentage, memory MiB, PID/task count, and file descriptors; omitted values retain runtime defaults. Native T2 cgroup/fallback behavior and the local named-unit smoke helper are documented in the [CLI-wrapper Spirit cookbook](/cookbook/cli-wrapper-spirit).
- T3 has no MAOS-specific seccomp or AppArmor profile.
- T3 is Linux-only; elsewhere the launch is refused.

**Setup:** configure the operator-owned host grant and signed image lock, pull the locked image by digest with rootless podman, then run a Worker. The manifest and grant example are in the [CLI-wrapper Spirit cookbook](/cookbook/cli-wrapper-spirit). CI proof uses rootless podman on the Ubuntu 24.04 and 26.04 x64 images (T3 is x64-only); native T2 runtime proof also covers the arm images.

```bash
export MAOS_T3_IMAGE_LOCK_PATH="/path/to/operator-signed-image-lock.toml"
export MAOS_T3_IMAGE_TRUST_ANCHOR_PUB_HEX="ED25519_PUBLIC_KEY_HEX"
export MAOS_HOST_GRANTS="/path/to/host-grants.toml"

# Replace with the image reference and digest from the signed image lock.
podman pull "IMAGE@sha256:DIGEST"
./target/release/maos run worker-manifest.toml --once
```

## Topology comparison

| Feature | Single-host | Multi-host (A2A) | Air-gapped | Contained Workers (T3) |
|---------|-------------|------------------|------------|------------------------|
| Network required | Optional | Yes (mTLS) | No | Optional; Worker egress only through the proxy allowlist |
| Spirit communication | In-process | A2A over mTLS | In-process only | In-process for Spirits; stdio bridge for Workers |
| Remote inference | Yes | Yes | No | Workers reach vendor APIs only through the proxy allowlist |
| Transparency Log | Local SQLite | Per-host SQLite | Local SQLite | Local SQLite, written by the daemon |
| Isolation level | Capability tokens | Capability tokens + network | Capability tokens + no network | Capability tokens; T3 containers for Workers (no network, proxy-only egress); T2 runner for WASM Spirits |
| Complexity | Low | Medium | Medium | High |
