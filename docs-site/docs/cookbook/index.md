---
title: Cookbook
sidebar_position: 1
description: Practical patterns for building, deploying, and operating MAOS Spirits.
---

# Cookbook

Bite-sized recipes for common MAOS tasks. Every pattern follows the same structure: **Problem** (what you need), **Solution** (working code you can copy), and **Discussion** (when and why to use it).

## Getting Started

| Pattern | Summary |
|---|---|
| [Hello-World Spirit](./hello-world-spirit) | Minimal Spirit with a single `on_idle` hook — the 30-minute path |
| [Manifest Fields](./manifest-fields) | Complete `spirit.toml` manifest covering all schema v3 sections |
| [Lifecycle Hooks](./lifecycle-hooks) | Implementing multiple lifecycle hooks in one Spirit |

## Reliability & Operations

| Pattern | Summary |
|---|---|
| [Cancellation Handling](./cancellation-handling) | Checking the cancellation signal in long-running hooks |
| [Capability Scoping](./capability-scoping) | Requesting and using capability tokens safely |
| [Error Handling](./error-handling) | Typed errors and recovery classes |
| [Hot-Swap Migration](./hot-swap-migration) | State transfer via `snapshot()` + `migrate()` |

## Advanced Patterns

| Pattern | Summary |
|---|---|
| [Scheduled Invocations](./scheduled-invocations) | Periodic tasks via `on_schedule` and `[[schedule]]` entries |
| [Gateway Integration](./gateway-integration) | Implementing a `GatewaySubmodule` for Telegram, Slack, or Discord |
| [CLI Wrapper Spirit](./cli-wrapper-spirit) | Wrapping an external CLI tool as a Spirit |
| [Compliance Claims](./compliance-claim) | Attaching a `ComplianceClaimEnvelope` to a Spirit |
| [Testing with the Spirit SDK](./testing-with-spirit-sdk) | Using `SpiritTest` harness and `LocalRunner` |
