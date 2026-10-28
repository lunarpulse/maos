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

The source tree's live manifests declare their vendor endpoint: `spirits/worker/manifest-codex.toml` requests `egress = ["api.openai.com:443"]` and `spirits/worker/manifest-claude.toml` requests `egress = ["api.anthropic.com:443"]`; the host grant still bounds both. On the operator lane, the topology selects the adapter and `--live-codex` is the only live switch: `cargo run -p xtask -- demo-j1 --live-codex --codex-topology spirits/topologies/j1-founder-loop-claude.toml` runs one real `claude` Worker and prints its workspace path.

## Discussion

**Manifest shape:** `[cli_wrapper]` is a single table and is mutually exclusive with `[class]`. Both together are refused at manifest gate 3. Unknown keys in `[cli_wrapper]` and `[cli_wrapper.posture]` are refused. The operator door refuses every Worker manifest at gate 3b; use `maos run` (or a topology file run by `maos run`).

**Supported commands and adapter rules:** Only `codex`, `claude`, `worker-cli-fixture`, and `worker-probe-fixture` are supported; other commands are refused before probe or spawn. `codex` requires `exec`, `--json`, and `--sandbox workspace-write` in `argv_prefix`. `claude` requires `--print`, `--output-format json`, `--bare`, and `--settings` JSON with `sandbox.enabled = true`. Bypass flags are refused. Real `codex` and `claude` also require non-empty `MAOS_LIVE_AGENT`. Real `codex` and `claude` are never executed on the host: admission checks requested egress and the T3 floor, the T3 spawn is the invokability check, and the adapter verifies output shape at completion. Only `worker-cli-fixture` and `worker-probe-fixture` run an admission handshake on the host, with a cleared environment. MAOS resolves them by name — an absolute `command` as written, otherwise the `maos` executable's directory, then that directory's parent, then `$PATH` — and does not check where the binary came from, so whichever executable of that name is found first runs on the host.

**Posture and task flow:** T3 admits `ndjson_over_stdio` or `raw` only with `control_channel = "signals"`; `json_rpc_over_stdio`, `named_pipe`, and `stdin_commands` are refused because they keep stdin. The container has no stdin. The task is passed as a trailing argv argument, stdout is bridged into the Transparency Log, and the adapter decides the completion verdict.

**Workspace:** Each Worker run gets a fresh workspace at `<workspaces root>/<container_name>/`. The workspaces root is `MAOS_WORKER_WORKSPACES_ROOT` when set (an absolute path, used as written; a relative path is refused), otherwise `<MAOS_HOME>/workspaces` (`MAOS_HOME`, otherwise `$HOME/.maos`). MAOS creates the root `0700` if it is missing and refuses it unless it is a directory, not a symlink, owned by the daemon's user with mode `0700`. It creates each workspace `0755` inside the root and mounts it with `--volume=<dir>:/workspace:rw,U` and `--workdir=/workspace`. It is the Worker's working directory and its only persistent writable path; the daemon's own working directory never reaches the Worker. The container root is `--read-only` with podman's default read-only tmpfs mounts, so the Worker can also write to the 64 MiB `/tmp` tmpfs and to `/run`, `/var/tmp` and `/dev/shm`; all of them are ephemeral, go away with the container, and never count as an effect. Because of `:U`, podman chowns the workspace to the container user's mapped subuid, so the files the Worker writes are owned by that subuid on the host. The workspace is kept after the Worker exits and MAOS does not delete it; its absolute path is the `workspace` field of the `cli_wrapper_loaded` and `worker_completion` events and of the `worker.completion-verdict` Transparency Log row. If the spawn fails before the Worker runs, MAOS removes the still-empty workspace; when it cannot, it prints `maos run: Worker workspace <dir> kept after a failed spawn: <error>`. Inspect or remove it inside the rootless user namespace: `podman unshare ls -laR <workspace>`, `podman unshare rm -rf <workspace>`. `maos purge` covers both `<MAOS_HOME>/workspaces` and a configured `MAOS_WORKER_WORKSPACES_ROOT`: it removes Worker-owned files with `podman unshare rm -rf --one-file-system`, and when that fails it leaves them and prints that command as residue.

**Disk use:** Workspaces have no size quota. A runaway Worker can write until the filesystem holding the workspaces root is full; with the default root, that is the filesystem holding `MAOS_HOME`, including the Transparency Log, and retained workspaces accumulate across runs. Set `MAOS_WORKER_WORKSPACES_ROOT` to a directory on a separate filesystem to keep Worker output off the audit store. After each Worker run MAOS checks the free space on the workspace's filesystem and, below 1 GiB (1073741824 bytes), prints `maos run: WARNING: Worker workspaces filesystem at <workspace> has <n> bytes free (below 1073741824); …` to stderr; if the check itself fails it prints `maos run: WARNING: cannot check free space on Worker workspaces filesystem at <workspace>: <error>`. The warning neither refuses nor stops the run, and it is not journaled. The missing size cap is a claim boundary (`RELEASE-HOLDS.md`, Claim boundaries row 20): MAOS does not claim that a Worker's disk use is bounded.

**Effect oracle:** After the Worker exits, MAOS compares the workspace with its pre-spawn snapshot (empty, because the workspace is fresh) by metadata only: relative path, file type, size, and `mtime_ns`. No file content is read, symlinks are recorded and never followed, and the walk stops at 10 000 entries. A larger tree, or an entry that cannot be read, is a typed refusal (`workspace walk refused …`), never a silent truncation. The result is reported as `workspace_effect`, either `{"created":n,"modified":n,"removed":n}` or `{"refused":"<reason>"}`, on the `worker_completion` event and in the `worker.completion-verdict` row. Because the workspace starts empty, a walk refused after it listed an entry (over the cap, or an unreadable entry below `/workspace`) still proves the Worker created something and counts as an effect; only a workspace that cannot be listed at all leaves the effect unproven. For `codex` and `claude`, a `Completed` adapter verdict with an empty diff or an unlistable workspace becomes `not_completed:no_effect_evidence`; `codex` still also needs its native `file_change` evidence, and `claude` still needs an empty `permission_denials`. The fixtures report the effect but are not gated. A non-empty diff proves that the Worker changed something in its workspace, not that it produced the requested artifact.

**Egress:** Manifest destinations use strict `host:port`; a grant's bare host means `:443`. Requested manifest egress must be a subset of host-granted destinations. Upstream is HTTPS with verified TLS, except an explicitly granted IPv4 loopback literal, which uses plain HTTP. A per-spawn scoped bearer authorizes the Worker to its proxy; the daemon alone reads the vendor key. Every proxy refusal is journaled as a kind-8 row (`sandbox.block.egress.<host>:<port>`) before the `403`; inspect rows with `maos audit query --spirit <name>`.

**Recovery and death:** `recovery_policy` is checked at load; `respawn_with_context` is refused and is the default when omitted. `maos run` respawns nothing today, whichever policy is selected.

| `recovery_policy` | At load | After the Worker dies |
|---|---|---|
| `respawn_with_context` (default) | Refused | No respawn by `maos run` |
| `respawn_fresh` | Admitted | No respawn by `maos run` |
| `escalate` | Admitted | No respawn by `maos run` |

T3 startup is runtime-owned: MAOS runs `create` → `init`, reads the runtime-written pidfile, then supervises `start --attach`. Containers are not auto-removed (`--rm` is not used) and use `--restart=no`. After terminal facts are read once while the owned CID is retained, teardown performs bounded `rm --force --volumes` and confirms absence with `container exists` for that CID. Short tasks complete without a startup hold. On Linux ≥6.15, payload pidfd `PIDFD_INFO_EXIT` distinguishes an intentional `exit(137)`, external SIGKILL (`fault.signaled`), and OOM (`fault.oom_killed`); when authoritative payload facts are unavailable, documented runtime-evidence fallback rows apply. Exit rows include typed container residue when cleanup cannot be confirmed. Worker unload performs a post-unload registry stop, and its lease ends only after termination is confirmed.

**Resource ceilings:** Manifest and operator limits resolve independently to the stricter value for CPU percentage, memory MiB, PID/task count, and file descriptors. Worker defaults when omitted are `--cpus=1`, `--memory=256m`, and `--pids-limit=64`; omission does not add a manifest ceiling. `cpu_max_pct = 10` maps to `--cpus=0.10`, and `100` to `--cpus=1.00`. `pids_max` independently controls `--pids-limit`; `fd_max` adds explicit equal soft/hard `--ulimit=nofile` limits, never a PID alias. If the requested FD ceiling exceeds the inherited hard ceiling, launch refuses rather than clamps. This T3 route is Linux x64 only; supported native runtime proof covers Ubuntu 24.04/26.04 x64 and arm images, while T3 is x64 only. No macOS/Windows resource equivalence is claimed.

For native Linux T2, MAOS requires a writable delegated cgroup v2 domain for explicit CPU or PID caps and refuses before guest code if unavailable. A memory-only request may instead retain `RLIMIT_AS` (and optional `RLIMIT_NOFILE`) and emit one persisted `ResourceCeilingDowngraded=20` informational lifecycle row on each successful fallback launch; FD-only and no-resource launches do not emit it. This is not a state transition. The native journal adapter appends after live Worker ownership/observer registration and before startup publication; append failure still panics and `sync_data` errors remain ignored, so no power-loss durability guarantee is added. Run local native cgroup smoke coverage through `python3 scripts/run_cgroup_ceiling_17_2.py run --mode delegated|undelegated --unit maos-17-2-NAME --timeout SECONDS -- COMMAND` (add `--rootless-canary` for the same-UID rootless canary).

Root shutdown takes custody before container creation, joins in-flight initialization before removal, and prevents a cancelled launch from starting afterward. Exit observation, explicit stop, bridge finalization, and guard cleanup share the original absolute teardown deadline. Truncated runtime diagnostics or a pipe left open by a runtime descendant cannot certify volume cleanup, even when `rm` exits successfully; they retain typed residue. A failed CID kill is reported independently of later successful removal. When `XDG_RUNTIME_DIR` is unavailable, temporary custody directories are scoped by effective UID so one user cannot block another user's launch. OOM attribution also requires the runtime's OOM marker; pidfd evidence alone proves the signal, not its cause.

**What stays unisolated:** The admission handshake of `worker-cli-fixture` and `worker-probe-fixture` runs on the host, and the binary that runs is whichever one the name resolves to, not a provenance-checked build; real `codex` and `claude` do not run on the host. First-party `rust-inproc` Spirits run inside the daemon at T0. T3 has no MAOS-specific seccomp profile. Worker workspaces have no size quota. MAOS ships no vendor-CLI image or entry shim, so no live Codex or Claude run under this profile is claimed.
