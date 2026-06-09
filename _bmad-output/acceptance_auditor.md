# Acceptance Audit Findings — Story 8.14b

## 1. `handle_option_pick` does NOT call `mcp_port.write_linear_note` — violates AC2

**AC violated:** AC2 ("Butler calls `mcp_port.write_linear_note(title, content)` which calls `linear.create_issue` via MCP")

**Evidence:** `spirits/butler/src/lib.rs:511-547` — `handle_option_pick` is synchronous and takes no `mcp_port` parameter. For option `'a'`, it returns an `OptionPickOutcome` with `linear_note_written: true` without actually calling `write_linear_note`. The method signature is `fn handle_option_pick(&self, option: char) -> Result<OptionPickOutcome, ButlerMcpError>`, but the spec (FORK 2) explicitly requires `fn handle_option_pick(&self, option: char, mcp_port: &dyn ButlerMcpPort)`. The Linear write is stubbed out — the flag is set to `true` but no MCP call occurs. The `FakeButlerMcpPort` test (`handle_option_a_calls_linear_write`) confirms this: it only checks `outcome.linear_note_written` and `outcome.message`, never that `write_linear_note` was actually called on the fake port.

## 2. Kernel-core has a whitespace-only change — technically violates AC4 "byte-identical" constraint

**AC violated:** AC4 ("`maos-kernel-core/src/` is byte-identical to post-8.14a baseline")

**Evidence:** `git diff HEAD~1 -- crates/maos-kernel-core/src/` shows one blank line added to `crates/maos-kernel-core/src/capability/mod.rs` at line 225. The spec says "`git diff <pre-story-HEAD> -- crates/maos-kernel-core/src/ --stat` is empty" — this diff is not empty. The change is functionally harmless (one newline), but the AC's wording is "byte-identical" with "empty" diff stat, which this violates.

## 3. `ButlerMcpError::CallFailed.cause` is `String`, not `McpError` — deviates from FORK 1 locked spec

**AC violated:** FORK 1 ("`ButlerMcpError` enum: variants `CallFailed { server: String, tool: String, cause: McpError }`")

**Evidence:** `spirits/butler/src/lib.rs:50-68` — `CallFailed` has `cause: String`, not `cause: McpError`. The FORK 1 locked implementation explicitly specifies the `cause` field as `McpError` (the domain error type). Using `String` loses structured error information and breaks the typed-error chain that `thiserror` enables.

## 4. `journey_butler.rs` does not exist — JB-1 and JB-2 are absent, not `#[ignore]`

**AC violated:** AC3 ("PTY-level JB-1 and JB-2 in `crates/maos-journey-test/tests/journey_butler.rs` remain `#[ignore = "RED: 8.15 harness not built"]`")

**Evidence:** The directory `crates/maos-journey-test/tests/` contains only `jb3_self_tuning_halt.rs`. There is no `journey_butler.rs` file at all. The spec explicitly requires JB-1 and JB-2 to exist as `#[ignore]` tests in that file — they were supposedly authored by Story 8.11 and must be preserved.

## 5. JB-4 and JB-6 integration tests are missing — violates AC3 spec

**AC violated:** AC3 / Files to touch ("ADD JB-4 driver integration test" and "ADD JB-6 driver integration test")

**Evidence:** The spec requires: (a) JB-4 — Butler's `on_idle` with `TestButlerMcpPort` produces a digest whose completions have non-empty `source_log_ref`; (b) JB-6 — `LiveButlerMcpPort` returns `CapabilityDenied` when calling an undeclared tool. Neither test exists anywhere in the diff. `crates/maos-journey-test/tests/` only has `jb3_self_tuning_halt.rs`.

## 6. Shell option-pick is a hard-coded string prototype, not connected to Butler — deviates from AC2/FORK 2

**AC violated:** AC2 ("the shell dispatches the option pick to Butler's `handle_option_pick`") / FORK 2 ("Shell parses `@butler a` and calls `butler.handle_option_pick('a', mcp_port)`")

**Evidence:** `crates/maos-shell/src/lib.rs:173-190` — The shell dispatch for butler is entirely hard-coded string matching. It never calls `butler.handle_option_pick()` or accesses a Butler instance. The match arms return static strings like `"Linear note written: Calendar conflict — evt-a ↔ evt-b (stub...)"`. The spec says the shell should call `handle_option_pick` and render the `OptionPickOutcome`, but the shell has no dependency on a Butler instance — it only depends on the `butler` crate for the `@butler` name match. This means the shell test (`shell_butler_pick_renders_option_messages`) passes with hard-coded output, not by exercising the real dispatch path.

## 7. `LiveButlerMcpPort::new()` signature differs from FORK 1 locked spec

**AC violated:** FORK 1 ("`LiveButlerMcpPort::new(spirit_pid, posture_hash, mcp_client)`")

**Evidence:** `crates/maos-bin/src/main.rs:516-521` — `LiveButlerMcpPort` is a plain struct with public fields, constructed via struct literal at line 1618 (`LiveButlerMcpPort { spirit_pid: 0, posture_hash: [0u8; 32], mcp_client: adapter, capability: ... }`). There is no `new()` constructor. The FORK 1 locked spec requires `LiveButlerMcpPort::new(spirit_pid, posture_hash, mcp_client)`. Additionally, the constructor takes only 3 args but the struct has 4 fields — `capability` was added without spec approval. The `spirit_pid` and `posture_hash` are both hardcoded to `0`/`[0u8; 32]`, not injected from the composition root.

## 8. `--live` MCP wiring only reads `MAOS_MCP_CALENDAR_URI`, ignores others — partial implementation

**AC violated:** AC1 ("Butler calls real Calendar MCP and Slack MCP")

**Evidence:** `crates/maos-bin/src/main.rs:1539-1546` — The code reads all four env vars (`MAOS_MCP_CALENDAR_URI`, `MAOS_MCP_SLACK_URI`, `MAOS_MCP_LINEAR_URI`, `MAOS_MCP_FIGMA_URI`), but only `calendar_uri` is used (line 1547: `if !calendar_uri.is_empty()`). The other three are assigned to `_slack_uri`, `_linear_uri`, `_figma_uri` (underscore-prefixed, unused). All four servers use the same `StreamableHttpTransport` pointing at the calendar URI. Slack/Linear/Figma will all hit the calendar server's endpoint, not their own.

## 9. Missing `maos_bin::tests::butler_8_14b_mcp_drivers` subprocess test — violates AC3 test 4

**AC violated:** AC3 test 4 ("`maos_bin::tests::butler_8_14b_mcp_drivers` (subprocess) — `maos run butler --once` with isolated `MAOS_HOME` + a mock MCP server URL; assert JSON line `{"event":"on_idle_fired"}` appears + no CapabilityDenied error")

**Evidence:** `crates/maos-bin/tests/butler_8_14b.rs` contains 3 tests: `maos_run_butler_once_preserved_existing_halt_behavior`, `maos_run_butler_live_once_wires_mcp_port`, and `shell_butler_pick_renders_option_messages`. None matches the AC3 test 4 spec: there is no test named `butler_8_14b_mcp_drivers`, no assertion for `{"event":"on_idle_fired"}`, and no mock MCP server setup. The `maos_run_butler_live_once_wires_mcp_port` test only checks stderr for the wiring log message.

## 10. `FakeButlerMcpPort` named differently than spec — naming deviation

**AC violated:** FORK 1 ("`FakeButlerMcpPort` (`#[cfg(test)]` in butler) ships in the SAME commit as the trait")

**Evidence:** The spec uses the name `TestButlerMcpPort` in AC3 test descriptions (e.g., "`TestButlerMcpPort` (backed by `FixtureReplayMcpServer`) feeds a conflict scenario") but the implementation at `spirits/butler/src/lib.rs:941` names it `FakeButlerMcpPort`. This is a minor naming inconsistency but causes confusion when tracing AC3 test descriptions to actual test code.

## 11. JB-3 uses `cargo run -p maos-bin` instead of `CARGO_BIN_EXE_maos` — inconsistent subprocess pattern

**AC violated:** Dev Notes ("use `Command::new(env!("CARGO_BIN_EXE_maos"))` with `--once`") / Lessons from prior stories (shell_8_14a.rs pattern)

**Evidence:** `crates/maos-journey-test/tests/jb3_self_tuning_halt.rs:40` uses `Command::new("cargo")` with `args(["run", "-p", "maos-bin", "--", ...])`. The `butler_8_14b.rs` test uses the correct `Command::new(env!("CARGO_BIN_EXE_maos"))`. The JB-3 approach is slower (requires cargo build) and inconsistent with the established subprocess pattern. It also depends on `maos-bin` as a dev-dependency in `maos-journey-test/Cargo.toml` but doesn't use the binary artifact that `dev-dependency` + `CARGO_BIN_EXE_` provides.

## 12. `on_idle` swallows all MCP errors silently via `unwrap_or_default()` — deviates from spec intent

**AC violated:** AC1 ("LiveButlerMcpPort is injected into Butler, which calls real Calendar MCP") — intent is real data flow, not silent fallback

**Evidence:** `spirits/butler/src/lib.rs:395-396` — `block_on_sync(mcp.calendar_events()).unwrap_or_default()` and same for `comms_messages()`. If the MCP call fails (e.g. `TokenIssuanceFailed`, `Unauthorized`, network error), the error is silently discarded and Butler proceeds with empty data. The spec requires MCP errors to surface (especially `CapabilityDenied` / `Unauthorized`). No logging, no metric, no assessment adjustment — the failure is invisible.

## 13. Missing `butler_8_14b.rs` tests for mock MCP server and Linear write verification — violates AC2

**AC violated:** AC2 / Files to touch ("`butler_option_a_writes_linear_note` — same setup + send `a` to shell; assert mock linear server received `create_issue` call")

**Evidence:** The `shell_butler_pick_renders_option_messages` test sends `@butler pick a` to the shell and checks for `"Linear note written"` in stdout, but: (a) there is no mock MCP server; (b) the shell output is hard-coded (see Finding 6); (c) the test does not verify any MCP call was made. The spec requires a test that proves the Linear write actually reached a mock MCP server.