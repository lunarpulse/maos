---
title: CLI Wrapper Spirit
sidebar_position: 12
description: Wrapping an external CLI tool as a MAOS Spirit using the cli_wrapper manifest section.
---

# CLI Wrapper Spirit

## Problem

You have an existing command-line tool — a linter, a data pipeline, a code formatter — that you want to run under MAOS governance. You do not want to rewrite it in Rust or implement the Spirit trait directly. The CLI Wrapper mechanism lets the kernel manage the tool as a Spirit with full lifecycle, capability mediation, and supervision.

## Solution

Create a manifest-only Spirit that declares a `[cli_wrapper]` section:

```toml
[class]
name = "jq-spirit"
version = "1.0.0"
abi = "1.0"
manifest_schema_version = 3
min_substrate_version = "0.1.0-alpha"
forms = ["subprocess"]
trust_tier = "local"
description = "Wraps jq as a MAOS Spirit for JSON transformation."

[author]
name = "ops-team"

[sandbox]
tier = "hardened"

[resources]
max_memory_mb = 128
max_cpu_ms = 10000

[budget]
max_inference_calls = 0
time_cap_seconds = 120

[posture]
default = "supervised"
allowed_max = "supervised"

# ── CLI Wrapper section (Story 6.2 / ADR-021) ────────────
[cli_wrapper]
command = "/usr/bin/jq"
output_shape_version = 1
recovery_policy = "restart"

[cli_wrapper.posture]
default = "supervised"
allowed_max = "supervised"

[supervision]
heartbeat_interval_ms = 5000
progress_threshold_ms = 30000
silent_failure_threshold_ms = 30000

[on_crash]
action = "restart"
```

The kernel handles the subprocess lifecycle. Inbound IAC frames are piped to the CLI tool's stdin; the tool's stdout is captured and validated against the declared `output_shape_version`.

## Discussion

CLI Wrapper Spirits (Story 6.2 / ADR-021) are the bridge between existing tools and the MAOS governance model. Key design points:

**Mutual exclusivity:** A manifest declares **either** `[cli_wrapper]` or native Spirit hooks — never both. The kernel rejects a manifest that has a `[cli_wrapper]` section and also declares `enabled_hooks` or implements the Spirit trait. This prevents ambiguity about who owns the process lifecycle.

**Output shape validation:** The `output_shape_version` field declares the expected structure of the CLI tool's stdout. At admission, the kernel runs a probe invocation and validates the output against the declared shape. If the observed output does not match, admission fails with `CliWrapperAdmissionError::EOutputShapeMismatch` — there is no fallback parsing (ADR-021).

**Recovery policy options:**

| Policy | Behaviour |
|---|---|
| `restart` | Kernel restarts the subprocess on non-zero exit |
| `fail` | Kernel marks the Spirit as failed; operator intervention required |
| `ignore` | Kernel logs the failure and continues (use for optional tools) |

**When to use CLI wrappers vs. native Spirits:**

- Use `[cli_wrapper]` when the tool already exists, is well-tested, and does not need deep kernel integration (IAC routing, hot-swap, capability-mediated inference).
- Use native Spirits when you need multiple lifecycle hooks, hot-swap state transfer, or fine-grained capability scoping.
- CLI wrappers run in `subprocess` form — they do not share the kernel's address space. This provides natural isolation but adds IPC overhead.

**Sandbox implications:** CLI wrappers run in `subprocess` form, so the `[sandbox].tier` applies OS-level sandboxing (seccomp, landlock) to the spawned process. The `hardened` tier is recommended for wrapping untrusted tools.
