#![forbid(unsafe_code)]

//! `worker_cli` — the Worker-CLI **adapter** seam (J1 Tier-2 bridge, Task T2).
//!
//! J1's thesis is "swap the worker binary, not the bridge." This module is that
//! seam: one [`WorkerCli`] trait the deterministic hermetic fixture AND every
//! real agent CLI (`codex`, `claude`) share. The live run swaps only the binary
//! + argv; the kernel bridge (`maos-kernel-core::lifecycle::cli_wrapper`) stays
//! CLI-agnostic (ZERO kernel-Δ — it runs argv and returns raw stdout/stderr/exit;
//! completion interpretation lives HERE, in the composition root).
//!
//! Two responsibilities per adapter:
//! 1. [`WorkerCli::argv`] — the per-invocation task arguments appended AFTER the
//!    manifest's hashed `argv_prefix` (the prefix carries the cap-token TOCTOU
//!    binding; the task is a trailing, non-hashed argument).
//! 2. [`WorkerCli::parse_completion`] — the **per-CLI completion oracle** over the
//!    captured output. A raw process exit is NEVER completion (spec "Never"): each
//!    CLI signals completion through its own documented output shape.
//!
//! The captured `stdout`/`stderr` handed to [`WorkerCli::parse_completion`] are the
//! journaled `FrameKind::CliSubprocessOutput` lines read back from the Transparency
//! Log (already redacted at insert) — so completion is decided from the persisted
//! evidence chain, never a side channel.

/// How the bridge classified the child's termination. A SECONDARY signal only —
/// completion is decided by the CLI's output oracle, not by this.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerExit {
    /// The child exited with this status code (`0` or non-zero).
    Exited(i32),
    /// The child was killed by a signal, or the cause was indeterminate.
    Crashed,
}

impl WorkerExit {
    /// A clean `exit(0)` — a *necessary* but NOT *sufficient* condition for
    /// completion. The oracle still requires the CLI's completion marker.
    fn is_clean(self) -> bool {
        matches!(self, WorkerExit::Exited(0))
    }
}

/// Why a worker run did not reach a citable completion. A typed lifecycle
/// result — never a digest-citable completion; Tier-2 stays open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerNonCompletion {
    /// The child crashed / was signaled / exited non-zero before completing.
    ProcessCrash { exit_code: Option<i32> },
    /// The child exited cleanly but its output carried no completion marker
    /// (no answer, empty stdout, or output that never signaled completion).
    NoCompletionMarker,
}

/// The typed outcome of interpreting a worker CLI's captured output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerCompletion {
    /// The CLI signaled task completion via its documented oracle. `final_message`
    /// is the (already-redacted) terminal stdout message the oracle matched.
    Completed { final_message: String },
    /// The CLI ran but did not signal completion.
    NotCompleted(WorkerNonCompletion),
}

impl WorkerCompletion {
    /// True only for a genuine, oracle-confirmed completion.
    pub fn is_completed(&self) -> bool {
        matches!(self, WorkerCompletion::Completed { .. })
    }

    /// A short, non-secret label for events/capture (never the message text).
    pub fn label(&self) -> &'static str {
        match self {
            WorkerCompletion::Completed { .. } => "completed",
            WorkerCompletion::NotCompleted(WorkerNonCompletion::ProcessCrash { .. }) => {
                "not_completed:process_crash"
            }
            WorkerCompletion::NotCompleted(WorkerNonCompletion::NoCompletionMarker) => {
                "not_completed:no_completion_marker"
            }
        }
    }
}

/// A swappable worker-CLI adapter — the ONE seam the fixture and every real
/// agent CLI share (the "swap the binary, not the bridge" contract).
pub trait WorkerCli: Send + Sync {
    /// Stable, non-secret adapter identity (emitted into events + the signed
    /// capture as the "live-agent identity").
    fn name(&self) -> &'static str;

    /// The per-invocation task arguments appended AFTER the manifest's hashed
    /// `argv_prefix`. The task text is a trailing argument, never part of the
    /// hashed invocation shape.
    fn argv(&self, task: &str) -> Vec<String>;

    /// Non-secret environment this CLI needs (e.g. codex's `CODEX_NON_INTERACTIVE`).
    /// Credentials are injected separately, host-side — NEVER returned here.
    fn nonsecret_env(&self) -> Vec<(String, String)> {
        Vec::new()
    }

    /// Decide completion from the CAPTURED output — never from `exit` alone.
    /// `stdout`/`stderr` are the journaled `CliSubprocessOutput` lines in order.
    fn parse_completion(
        &self,
        stdout: &[String],
        stderr: &[String],
        exit: WorkerExit,
    ) -> WorkerCompletion;

    /// The ambient auth file whose presence in the sandbox home lets this CLI use
    /// a credential MAOS never holds (codex's ChatGPT-login `~/.codex/auth.json`),
    /// making redaction unattestable → a failed Tier-2. `None` = no such footgun.
    /// The live path refuses when this file exists (the clean-home invariant).
    fn ambient_auth_path(&self, _home: &std::path::Path) -> Option<std::path::PathBuf> {
        None
    }
}

/// The clean-home invariant (spec "Never"): on the live path, refuse if the CLI's
/// ambient auth file exists in the sandbox home. codex's `~/.codex/auth.json`
/// (ChatGPT-login) shadows the injected API key with a token MAOS never holds, so
/// it cannot prove redaction — an un-attestable secret is a FAILED Tier-2, never a
/// silent inherit. Returns the offending path on refusal.
pub fn refuse_ambient_auth(
    cli: &dyn WorkerCli,
    home: &std::path::Path,
) -> Result<(), std::path::PathBuf> {
    match cli.ambient_auth_path(home) {
        Some(p) if p.exists() => Err(p),
        _ => Ok(()),
    }
}

/// The last non-empty, trimmed line of a captured stream (the "final message").
fn final_nonempty_line(lines: &[String]) -> Option<&str> {
    lines.iter().rev().map(|l| l.trim()).find(|l| !l.is_empty())
}

/// Shared oracle for CLIs whose final assistant message lands on stdout
/// (`codex exec`, `claude -p`): a clean exit AND a non-empty final stdout line.
/// A crash/non-zero exit is `ProcessCrash`; a clean exit with empty stdout is
/// `NoCompletionMarker` (the CLI produced no answer → not a completion).
fn final_stdout_message_oracle(stdout: &[String], exit: WorkerExit) -> WorkerCompletion {
    match exit {
        WorkerExit::Exited(0) => match final_nonempty_line(stdout) {
            Some(msg) => WorkerCompletion::Completed {
                final_message: msg.to_string(),
            },
            None => WorkerCompletion::NotCompleted(WorkerNonCompletion::NoCompletionMarker),
        },
        WorkerExit::Exited(code) => {
            WorkerCompletion::NotCompleted(WorkerNonCompletion::ProcessCrash {
                exit_code: Some(code),
            })
        }
        WorkerExit::Crashed => {
            WorkerCompletion::NotCompleted(WorkerNonCompletion::ProcessCrash { exit_code: None })
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Fixture — the deterministic in-crate `worker-cli-fixture` (hermetic Tier-1).
// ─────────────────────────────────────────────────────────────────────────────

/// The hermetic fixture worker. Completion oracle: the terminal
/// `worker: task complete` line (see `worker::CANNED_OUTPUT_LINES`).
pub struct FixtureCli;

/// The completion marker the fixture's final stdout line carries.
pub const FIXTURE_COMPLETION_MARKER: &str = "worker: task complete";

/// The hermetic fixture's adapter name — the ONE worker CI is allowed to spawn.
pub const FIXTURE_CLI_NAME: &str = "worker-cli-fixture";

impl WorkerCli for FixtureCli {
    fn name(&self) -> &'static str {
        FIXTURE_CLI_NAME
    }

    fn argv(&self, task: &str) -> Vec<String> {
        vec![task.to_string()]
    }

    fn parse_completion(
        &self,
        stdout: &[String],
        _stderr: &[String],
        exit: WorkerExit,
    ) -> WorkerCompletion {
        if !exit.is_clean() {
            return WorkerCompletion::NotCompleted(WorkerNonCompletion::ProcessCrash {
                exit_code: match exit {
                    WorkerExit::Exited(c) => Some(c),
                    WorkerExit::Crashed => None,
                },
            });
        }
        match final_nonempty_line(stdout) {
            Some(last) if last.contains(FIXTURE_COMPLETION_MARKER) => WorkerCompletion::Completed {
                final_message: last.to_string(),
            },
            _ => WorkerCompletion::NotCompleted(WorkerNonCompletion::NoCompletionMarker),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// codex — `codex exec "<task>"`: progress → stderr, final message → stdout.
// ─────────────────────────────────────────────────────────────────────────────

/// The ratified first live worker: OpenAI `codex`. `codex exec` runs
/// non-interactively; the manifest's `argv_prefix` carries `["exec", "--sandbox",
/// "workspace-write"]`, and the task is the trailing argument. Progress is on
/// stderr; the final assistant message is on stdout (the completion oracle).
pub struct CodexCli;

impl WorkerCli for CodexCli {
    fn name(&self) -> &'static str {
        "codex"
    }

    fn argv(&self, task: &str) -> Vec<String> {
        vec![task.to_string()]
    }

    fn nonsecret_env(&self) -> Vec<(String, String)> {
        // Non-interactive; keeps codex from prompting for a TTY. The credential
        // (`OPENAI_API_KEY`) is injected host-side, NOT here.
        vec![("CODEX_NON_INTERACTIVE".to_string(), "1".to_string())]
    }

    fn parse_completion(
        &self,
        stdout: &[String],
        _stderr: &[String],
        exit: WorkerExit,
    ) -> WorkerCompletion {
        final_stdout_message_oracle(stdout, exit)
    }

    fn ambient_auth_path(&self, home: &std::path::Path) -> Option<std::path::PathBuf> {
        // ChatGPT-login writes a plaintext token here; on the live path its
        // presence is a hard refusal (it shadows the injected `OPENAI_API_KEY`).
        Some(home.join(".codex").join("auth.json"))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// claude — `claude -p "<task>"`: the result lands on stdout.
// ─────────────────────────────────────────────────────────────────────────────

/// Anthropic `claude -p` (print/non-interactive). `claude -p` IS current (the
/// deprecation premise was corrected at preflight); the manifest's `argv_prefix`
/// carries `["-p"]`, the task is the trailing argument, the result is on stdout.
pub struct ClaudeCli;

impl WorkerCli for ClaudeCli {
    fn name(&self) -> &'static str {
        "claude"
    }

    fn argv(&self, task: &str) -> Vec<String> {
        vec![task.to_string()]
    }

    fn parse_completion(
        &self,
        stdout: &[String],
        _stderr: &[String],
        exit: WorkerExit,
    ) -> WorkerCompletion {
        final_stdout_message_oracle(stdout, exit)
    }
}

/// Select the adapter for a resolved CLI binary path by its file-name base.
/// `None` ⇒ an unsupported wrapper: the caller MUST fail closed (spec T1:
/// "fail closed on ... unsupported wrappers").
pub fn select_worker_cli(command: &str) -> Option<Box<dyn WorkerCli>> {
    let base = std::path::Path::new(command)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(command);
    // Strip a trailing platform suffix so `codex`/`codex.exe` both resolve.
    let base = base.strip_suffix(".exe").unwrap_or(base);
    match base {
        "worker-cli-fixture" => Some(Box::new(FixtureCli)),
        "codex" => Some(Box::new(CodexCli)),
        "claude" => Some(Box::new(ClaudeCli)),
        _ => None,
    }
}

/// The names of the supported worker CLIs, for a fail-closed diagnostic.
pub const SUPPORTED_WORKER_CLIS: &[&str] = &["worker-cli-fixture", "codex", "claude"];

/// T5 — the CI/local split gate. The hermetic fixture may ALWAYS spawn; a real
/// agent CLI (`codex`/`claude`) requires the operator to opt in with
/// `MAOS_LIVE_AGENT` (local-only). CI never sets the flag, so CI physically
/// cannot spawn a paid agent — it runs the fixture through the same bridge. This
/// is a positive allowlist (an unknown real CLI is refused by default), stronger
/// than the kernel's `ci_default_guard` denylist, and it lives at the actual
/// spawn site (which the kernel guard never did).
pub fn live_agent_gate(worker_name: &str, live_agent: bool) -> Result<(), String> {
    if worker_name == FIXTURE_CLI_NAME || live_agent {
        Ok(())
    } else {
        Err(format!(
            "refusing to spawn real agent CLI '{worker_name}' on the hermetic path — set \
             MAOS_LIVE_AGENT=1 for the local live run (CI runs the fixture only, never a paid agent)"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    // ── select_worker_cli ──────────────────────────────────────────────────

    #[test]
    fn select_resolves_fixture_codex_claude_by_basename() {
        assert_eq!(
            select_worker_cli("/tmp/target/debug/worker-cli-fixture")
                .unwrap()
                .name(),
            "worker-cli-fixture"
        );
        assert_eq!(select_worker_cli("/usr/bin/codex").unwrap().name(), "codex");
        assert_eq!(select_worker_cli("codex.exe").unwrap().name(), "codex");
        assert_eq!(select_worker_cli("claude").unwrap().name(), "claude");
    }

    #[test]
    fn select_fails_closed_on_unsupported_wrapper() {
        // The fail-closed contract: an unknown wrapper yields no adapter, so the
        // caller refuses before spawn.
        assert!(select_worker_cli("/usr/bin/rm").is_none());
        assert!(select_worker_cli("bash").is_none());
    }

    #[test]
    fn argv_appends_task_as_trailing_arg() {
        assert_eq!(FixtureCli.argv("do the thing"), s(&["do the thing"]));
        assert_eq!(CodexCli.argv("scaffold a CLI"), s(&["scaffold a CLI"]));
        assert_eq!(ClaudeCli.argv("scaffold a CLI"), s(&["scaffold a CLI"]));
    }

    #[test]
    fn only_codex_declares_noninteractive_env_and_no_secret_leaks() {
        assert_eq!(
            CodexCli.nonsecret_env(),
            vec![("CODEX_NON_INTERACTIVE".to_string(), "1".to_string())]
        );
        assert!(FixtureCli.nonsecret_env().is_empty());
        assert!(ClaudeCli.nonsecret_env().is_empty());
        // No adapter's non-secret env may carry a credential-shaped key.
        for env in [
            CodexCli.nonsecret_env(),
            ClaudeCli.nonsecret_env(),
            FixtureCli.nonsecret_env(),
        ] {
            for (k, _) in env {
                assert!(!k.contains("KEY") && !k.contains("TOKEN") && !k.contains("SECRET"));
            }
        }
    }

    // ── T5 CI/local split gate ──────────────────────────────────────────────

    #[test]
    fn live_agent_gate_lets_the_fixture_run_without_the_flag() {
        assert!(live_agent_gate(FIXTURE_CLI_NAME, false).is_ok());
        assert!(live_agent_gate(FIXTURE_CLI_NAME, true).is_ok());
    }

    #[test]
    fn live_agent_gate_refuses_real_cli_without_the_flag() {
        // The load-bearing negative: CI (no MAOS_LIVE_AGENT) cannot spawn a paid
        // agent — codex/claude are refused fail-closed.
        assert!(live_agent_gate("codex", false).is_err());
        assert!(live_agent_gate("claude", false).is_err());
    }

    #[test]
    fn live_agent_gate_permits_real_cli_only_with_the_local_optin() {
        assert!(live_agent_gate("codex", true).is_ok());
        assert!(live_agent_gate("claude", true).is_ok());
    }

    #[test]
    fn codex_ambient_auth_json_is_refused_but_fixture_is_immune() {
        use std::path::Path;
        // Path shape: only codex names the footgun.
        let home = Path::new("/home/demo");
        assert_eq!(
            CodexCli.ambient_auth_path(home),
            Some(home.join(".codex").join("auth.json"))
        );
        assert_eq!(FixtureCli.ambient_auth_path(home), None);
        assert_eq!(ClaudeCli.ambient_auth_path(home), None);

        // Refusal against a real temp home (std-only, no dep).
        let tmp =
            std::env::temp_dir().join(format!("maos-authtest-{}-{}", std::process::id(), line!()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join(".codex")).unwrap();
        // No auth.json yet → permitted.
        assert!(refuse_ambient_auth(&CodexCli, &tmp).is_ok());
        // Plant the footgun → refused fail-closed.
        std::fs::write(tmp.join(".codex").join("auth.json"), b"{\"token\":\"x\"}").unwrap();
        assert!(refuse_ambient_auth(&CodexCli, &tmp).is_err());
        // The fixture is never shadowed by an ambient codex token.
        assert!(refuse_ambient_auth(&FixtureCli, &tmp).is_ok());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    // ── the completion oracle — a raw exit is NEVER completion ──────────────

    #[test]
    fn fixture_completes_on_terminal_marker_line() {
        let stdout = s(&[
            "worker: received task assignment",
            "worker: executing fixture-replayed work",
            "worker: task complete",
        ]);
        let c = FixtureCli.parse_completion(&stdout, &[], WorkerExit::Exited(0));
        assert!(c.is_completed());
        assert!(
            matches!(c, WorkerCompletion::Completed { final_message } if final_message.contains(FIXTURE_COMPLETION_MARKER))
        );
    }

    #[test]
    fn fixture_exit0_without_marker_is_not_completion() {
        // The load-bearing negative: exit code 0 but no completion marker ⇒ NOT a
        // completion. A raw exit is never the oracle.
        let stdout = s(&["worker: started", "worker: still working"]);
        let c = FixtureCli.parse_completion(&stdout, &[], WorkerExit::Exited(0));
        assert_eq!(
            c,
            WorkerCompletion::NotCompleted(WorkerNonCompletion::NoCompletionMarker)
        );
    }

    #[test]
    fn fixture_crash_is_process_crash_even_with_marker_present() {
        // Even if the marker somehow appears, a non-clean exit is a crash — a
        // signaled death is never silently upgraded to completion.
        let stdout = s(&["worker: task complete"]);
        let c = FixtureCli.parse_completion(&stdout, &[], WorkerExit::Crashed);
        assert_eq!(
            c,
            WorkerCompletion::NotCompleted(WorkerNonCompletion::ProcessCrash { exit_code: None })
        );
        let c2 = FixtureCli.parse_completion(&stdout, &[], WorkerExit::Exited(137));
        assert_eq!(
            c2,
            WorkerCompletion::NotCompleted(WorkerNonCompletion::ProcessCrash {
                exit_code: Some(137)
            })
        );
    }

    #[test]
    fn codex_completes_on_final_stdout_message() {
        let stdout = s(&["created main.rs", "ran the test — passed"]);
        let stderr = s(&["progress: thinking", "progress: writing files"]);
        let c = CodexCli.parse_completion(&stdout, &stderr, WorkerExit::Exited(0));
        assert_eq!(
            c,
            WorkerCompletion::Completed {
                final_message: "ran the test — passed".to_string()
            }
        );
    }

    #[test]
    fn codex_exit0_but_empty_stdout_is_no_completion() {
        // Progress-only on stderr, nothing on stdout ⇒ codex produced no final
        // message ⇒ not a completion (Tier-2 stays open).
        let c = CodexCli.parse_completion(&[], &s(&["progress: ..."]), WorkerExit::Exited(0));
        assert_eq!(
            c,
            WorkerCompletion::NotCompleted(WorkerNonCompletion::NoCompletionMarker)
        );
    }

    #[test]
    fn codex_nonzero_exit_is_process_crash() {
        let c = CodexCli.parse_completion(&s(&["partial"]), &[], WorkerExit::Exited(1));
        assert_eq!(
            c,
            WorkerCompletion::NotCompleted(WorkerNonCompletion::ProcessCrash {
                exit_code: Some(1)
            })
        );
    }

    #[test]
    fn claude_shares_the_final_stdout_oracle() {
        let c =
            ClaudeCli.parse_completion(&s(&["done: patch applied"]), &[], WorkerExit::Exited(0));
        assert_eq!(
            c,
            WorkerCompletion::Completed {
                final_message: "done: patch applied".to_string()
            }
        );
    }

    #[test]
    fn label_is_nonsecret_and_never_leaks_message() {
        let c = CodexCli.parse_completion(
            &s(&["SECRET sk-abc123 leaked into the message"]),
            &[],
            WorkerExit::Exited(0),
        );
        // The label carries no message text (the message may echo redacted content).
        assert_eq!(c.label(), "completed");
        assert!(!c.label().contains("SECRET") && !c.label().contains("sk-"));
    }
}
