---
title: Deployment Topology
sidebar_position: 1
description: Deployment topologies for MAOS — single-host, multi-host A2A, air-gapped, and container-isolated.
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

## Container-isolated

Each Spirit runs in its own container with the kernel orchestrating via a container runtime. Provides process-level and filesystem-level isolation beyond the kernel's capability-token enforcement.

```
┌──────────────────────────────────────┐
│          Host                        │
│  ┌────────────────────────────────┐  │
│  │       MAOS Kernel              │  │
│  └────────────────────────────────┘  │
│  ┌──────────┐  ┌──────────┐         │
│  │Container │  │Container │         │
│  │ Spirit A │  │ Spirit B │         │
│  └──────────┘  └──────────┘         │
│  ┌────────────────────────────────┐  │
│  │  Shared Transparency Log      │  │
│  └────────────────────────────────┘  │
└──────────────────────────────────────┘
```

**When to use:** multi-tenant production, untrusted Spirit workloads, compliance environments requiring process isolation.

**Isolation layers:**

1. **Capability tokens** — kernel-level scope enforcement (always active)
2. **Container namespaces** — PID, network, mount, and user namespace isolation
3. **Seccomp/AppArmor** — system-call filtering

**Setup with network namespace isolation:**

```bash
# Run each Spirit subprocess in its own network namespace
# (the kernel handles this when configured for container mode)
unshare --net -- ./target/release/maos run --spirit hello-spirit
```

## Topology comparison

| Feature | Single-host | Multi-host (A2A) | Air-gapped | Container-isolated |
|---------|-------------|------------------|------------|--------------------|
| Network required | Optional | Yes (mTLS) | No | Optional |
| Spirit communication | In-process | A2A over mTLS | In-process only | In-process |
| Remote inference | Yes | Yes | No | Yes |
| Transparency Log | Local SQLite | Per-host SQLite | Local SQLite | Shared SQLite |
| Isolation level | Capability tokens | Capability tokens + network | Capability tokens + no network | Capability tokens + containers |
| Complexity | Low | Medium | Medium | High |
