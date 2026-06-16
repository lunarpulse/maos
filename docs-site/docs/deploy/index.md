---
title: Deployment
sidebar_position: 0
description: Overview of MAOS deployment options — topologies, air-gap, backup, and release signing.
---

# Deployment

MAOS supports multiple deployment topologies from single-host development to air-gapped production environments. This section covers operational deployment guides.

## Guides

| Guide | Description |
|-------|-------------|
| [Deployment Topology](./topology) | Single-host, multi-host (A2A), air-gapped, and container-isolated modes |
| [Air-Gap Deployment](./air-gap-deployment) | Network-isolated deployment with compile-time network removal |
| [Backup & Restore Drill](./restore-drill) | Transparency Log backup, restore, and Merkle verification |
| [Release Signing](./release-signing) | Ed25519 release artifact signing and verification |

## Quick start

For a first deployment, start with the [single-host topology](./topology#single-host) and the default feature set. Production deployments should review the [air-gap guide](./air-gap-deployment) and [release signing](./release-signing) procedures.
