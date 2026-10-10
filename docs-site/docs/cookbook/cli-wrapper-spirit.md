---
title: CLI Wrapper Spirit
sidebar_position: 12
description: Run a supported agent CLI as a contained Worker under MAOS.
---

# CLI Wrapper Spirit

## Problem

You want MAOS to run a supported agent CLI as a Worker without giving it the host's network or home directory. A `[cli_wrapper]` manifest describes the command, its adapter posture, and requested egress; T3 contains the Worker.

## Solution

Declare one `[cli_wrapper]` table instead of `[class]`. This example selects Codex and requests its API endpoint:

```toml
# A Worker manifest: [cli_wrapper] replaces [class]; never declare both.
[cli_wrapper]
command = "codex"
argv_prefix = ["exec", "--json", "--sandbox", "workspace-write"]
output_shape_version = "1.0.0"
recovery_policy = "escalate"
egress = ["api.openai.com:443"]

[cli_wrapper.posture]
stdio_shape = "ndjson_over_stdio"
control_channel = "signals"
shutdown_signal = "SIGTERM"

[sandbox]
tier = "T3"

[author]
name = "OpenAI"

[on_crash]
action = "nack"
```

An operator grant, matched on the manifest's `command` and `[author]` `name`, admits its tier and egress. Save a grant such as this in the TOML file named by `MAOS_HOST_GRANTS`; grants are never declared in the Worker manifest:

```
[[grant]]
attested_image = "codex"
signing_key_id = "OpenAI"
permitted_tier = "T3"
permitted_egress_destinations = ["api.openai.com"]
```

The operator must also configure `MAOS_T3_IMAGE_LOCK_PATH` and `MAOS_T3_IMAGE_TRUST_ANCHOR_PUB_HEX`, and ensure the locked image is present locally. Real CLIs require non-empty `MAOS_LIVE_AGENT`. For Codex egress, the vendor key is `CODEX_API_KEY`; the daemon reads it for the proxy only, not the Worker. Start the Worker with `maos run <manifest.toml> [--once]`. The operator door refuses Worker manifests.

## Discussion

**Manifest shape:** `[cli_wrapper]` is a single table and is mutually exclusive with `[class]`. Both together are refused at manifest gate 3. Unknown keys in `[cli_wrapper]` and `[cli_wrapper.posture]` are refused. The operator door refuses every Worker manifest at gate 3b; use `maos run` (or a topology file run by `maos run`).

**Supported commands and adapter rules:** Only `codex`, `claude`, `worker-cli-fixture`, and `worker-probe-fixture` are supported; other commands are refused before probe or spawn. `codex` requires `exec`, `--json`, and `--sandbox workspace-write` in `argv_prefix`. `claude` requires `--print`, `--output-format json`, `--bare`, and `--settings` JSON with `sandbox.enabled = true`. Bypass flags are refused. Real `codex` and `claude` also require non-empty `MAOS_LIVE_AGENT`. The admission probe runs the named binary on the host with a cleared environment; for real CLIs it is a liveness probe, and the adapter verifies output shape at completion.

**Posture and task flow:** T3 admits `ndjson_over_stdio` or `raw` only with `control_channel = "signals"`; `json_rpc_over_stdio`, `named_pipe`, and `stdin_commands` are refused because they keep stdin. The container has no stdin. The task is passed as a trailing argv argument, stdout is bridged into the Transparency Log, and the adapter decides the completion verdict.

**Egress:** Manifest destinations use strict `host:port`; a grant's bare host means `:443`. Requested manifest egress must be a subset of host-granted destinations. Upstream is HTTPS with verified TLS, except an explicitly granted IPv4 loopback literal, which uses plain HTTP. A per-spawn scoped bearer authorizes the Worker to its proxy; the daemon alone reads the vendor key. Every proxy refusal is journaled as a kind-8 row (`sandbox.block.egress.<host>:<port>`) before the `403`; inspect rows with `maos audit query --spirit <name>`.

**Recovery and death:** `recovery_policy` is checked at load; `respawn_with_context` is refused and is the default when omitted. `maos run` respawns nothing today, whichever policy is selected.

| `recovery_policy` | At load | After the Worker dies |
|---|---|---|
| `respawn_with_context` (default) | Refused | No respawn by `maos run` |
| `respawn_fresh` | Admitted | No respawn by `maos run` |
| `escalate` | Admitted | No respawn by `maos run` |

On death, MAOS journals `cli.subprocess.exit`, `unplanned_crash`, `lifecycle.crash`, `task.orphaned`, and `worker.completion-verdict`. `[on_crash] action` selects the in-flight task disposition: `nack` (default), `reassign-to-replica`, or `escalate-to-operator`. Under T3, SIGKILL is reported as `exited(code=137)` with `fault.non-zero-exit`; OOM is not distinguished from another SIGKILL. Authoritative container OOM and signal facts are follow-up work. A short-lived Worker that exits before MAOS finishes container PID inspection (about 2.25 s) is refused as a failed launch.

**Containment:** T3 is Linux-only; elsewhere launch is refused, never run bare. MAOS applies `--rm`, `--cap-drop=ALL`, `--security-opt=no-new-privileges`, `--network=none`, `--http-proxy=false`, `--read-only`, `--tmpfs=/tmp:rw,size=64m`, `--user=65534:65534`, a read-only bind mount of the Worker binary at `/maos/spirit`, and runtime defaults `--cpus=1`, `--memory=256m`, `--pids-limit=64`. With egress it mounts that Worker's proxy socket at `/proxy/egress.sock`. The container environment contains only named `--env` entries. No host home, `control.json`, or repository is mounted. Manifest resource limits for Workers are not applied; story 17-2 owns that work.

**What stays unisolated:** The admission probe runs on the host. First-party `rust-inproc` Spirits run inside the daemon at T0. T3 has no MAOS-specific seccomp profile. MAOS ships no vendor-CLI image or entry shim, so no live Codex or Claude run under this profile is claimed.
