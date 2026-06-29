# JetBrains ACP Wire Contract

Story 10.5 AC2 — JetBrains plugin-bridge for ACP (v1.5).

## Overview

The JetBrains IDE plugin communicates with the MAOS substrate via the existing
**Agent Communication Protocol (ACP)** — NDJSON over stdio. The Rust-side bridge
is the existing `maos-acp` server; no kernel modification is required.

The JetBrains plugin (Kotlin/Java) is **out of the Rust workspace** — it is
written externally against this wire schema, following the same convention as
the Zed and VSCode plugins (Story 5.5c).

## Protocol

The editor spawns `maos-bin` with `MAOS_ONE_SHOT=acp-server` and pipes
stdin/stdout. Each line is a JSON object terminated by `\n` (NDJSON).

### Frames: Editor → Server (`AcpFrameIn`)

All frames carry `"kind"` (tagged union discriminator) and `"session_id"`
(16-byte JSON array).

| Kind | Fields | Description |
|------|--------|-------------|
| `session_start` | `session_id`, `editor_id: "jetbrains"`, `editor_version` | Open a session |
| `session_end` | `session_id` | Close the session |
| `lifecycle_verb` | `session_id`, `decision_id`, `verb` (load/start/pause/resume/unload), `spirit_id` | Lifecycle control |
| `halt_resolve` | `session_id`, `decision_id`, `halt_id`, `resolution` (approve/accept/provide), `operator_note?` | Resolve a halt |

### Frames: Server → Editor (`AcpFrameOut`)

| Kind | Fields | Description |
|------|--------|-------------|
| `session_ready` | `supported_kinds: ["lifecycle_verb", "halt_resolve", "session_end"]` | Session established |
| `session_terminated` | `duration_ns` | Session closed |
| `lifecycle_receipt` | `spirit_pid`, `outcome`, `timestamp_ns`, `error?` | Lifecycle verb result |
| `halt_receipt` | `outcome` | Halt resolution result |
| `notification_dispatch` | `level`, `event` | Push notification to editor |
| `error` | `code`, `message` | Protocol error |

### Example Conversation

```ndjson
→ {"kind":"session_start","session_id":[10,5,0,0,0,0,0,0,0,0,0,0,0,0,0,0],"editor_id":"jetbrains","editor_version":"2024.3"}
← {"kind":"session_ready","supported_kinds":["lifecycle_verb","halt_resolve","session_end"]}
→ {"kind":"lifecycle_verb","session_id":[10,5,...],"decision_id":[172,2,...],"verb":"load","spirit_id":"my-spirit"}
← {"kind":"lifecycle_receipt","spirit_pid":42,"outcome":"ok","timestamp_ns":1719300000000}
→ {"kind":"session_end","session_id":[10,5,...]}
← {"kind":"session_terminated","duration_ns":5000000}
```

## Integration Test

The living specification is `crates/maos-acp/tests/jetbrains_bridge_test.rs` —
a scripted NDJSON conversation that exercises the full session lifecycle with
`editor_id: "jetbrains"`.

The `acp-server` launch arm wires the production `KernelLifecycleResolver` and
`KernelHaltResolver` from the daemon composition root. The companion
`maos-acp` crate-level test covers protocol framing in-process; the binary
integration test (`crates/maos-bin/tests/jetbrains_acp_server.rs`) proves the
JetBrains NDJSON script reaches the real `MAOS_ONE_SHOT=acp-server` surface.
