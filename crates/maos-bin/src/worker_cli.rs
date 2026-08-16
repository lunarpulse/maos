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
    /// The adapter's structured stream carried an explicit terminal FAILURE:
    /// codex `turn.failed`, or a claude result object with `is_error: true` or a
    /// non-`success` `subtype`. Distinct from [`Self::NoCompletionMarker`] —
    /// there the CLI said nothing legible; here it said it failed.
    TurnFailed,
    /// The turn completed and claimed success, but the structured stream carries
    /// no evidence the declared write-class work happened (codex: no
    /// `item.completed` of type `file_change` with `status: "completed"`). This
    /// is the ship-blocker's exact shape — a clean exit and a fluent final
    /// message over a working tree the worker never touched.
    NoEffectEvidence,
    /// claude reported a non-empty `permission_denials` array: a tool call the
    /// model attempted and the permission posture refused. `--print` has no TTY
    /// to approve, so this is the mechanism behind the refusal that used to be
    /// journaled as `completed: true`.
    PermissionDenied,
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
            WorkerCompletion::NotCompleted(WorkerNonCompletion::TurnFailed) => {
                "not_completed:turn_failed"
            }
            WorkerCompletion::NotCompleted(WorkerNonCompletion::NoEffectEvidence) => {
                "not_completed:no_effect_evidence"
            }
            WorkerCompletion::NotCompleted(WorkerNonCompletion::PermissionDenied) => {
                "not_completed:permission_denied"
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
    ///
    /// **This is NOT a credential channel, and MAOS does not have one.** There is
    /// no host-side credential injection anywhere in `crates/maos-bin/src`: no
    /// setter for `CODEX_API_KEY`, `ANTHROPIC_API_KEY` or `OPENAI_API_KEY` exists,
    /// and the spawn is a bare `Command::new` with no `env_clear`
    /// (`maos-kernel-core::lifecycle::cli_wrapper::runtime`). The operator exports
    /// the key into `maos`'s own environment and the child INHERITS it. So: the
    /// credential control is "no ambient auth FILE" ([`Self::ambient_auth_path`]);
    /// **the environment channel is inherited by design and unattested.** The
    /// `env_clear` repair is kernel-core (`runtime.rs`) and is deliberately out of
    /// this seam's scope — do not read this comment as a claim that MAOS chose,
    /// injected, or knows which credential the child used.
    fn nonsecret_env(&self) -> Vec<(String, String)> {
        Vec::new()
    }

    /// The environment variable this CLI reads its credential FROM. `None` = this
    /// adapter needs no credential (the hermetic fixture).
    ///
    /// MAOS never sets it — see [`Self::nonsecret_env`]: there is no host-side
    /// injection and the child inherits the variable. This is here so a caller
    /// that must SCAN for the credential (the signed-run redaction check) can ask
    /// the adapter which value to scan for, instead of hardcoding one provider's
    /// variable and silently no-op'ing for every other adapter — which is exactly
    /// how a signed capture came to claim `redaction_result: "verified"` for a
    /// scan that never executed.
    fn credential_env_var(&self) -> Option<&'static str> {
        None
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

    /// How admission verifies this CLI at load time.
    ///
    /// The kernel's Story 6.2 output-shape probe spawns the CLI with a
    /// `--maos-bridge-probe` flag and expects it to emit its `output_shape_version`
    /// — a MAOS-specific handshake that ONLY the hermetic fixture implements. A
    /// real agent CLI (codex, claude) does not speak it and exits non-zero, so the
    /// kernel probe can never admit a real worker. For those, the WorkerCli
    /// adapter's [`parse_completion`](Self::parse_completion) IS the output-shape
    /// contract (verified at COMPLETION time), so admission only needs a liveness
    /// check that the binary is present and invokable. Default = a `--version`
    /// liveness probe; the fixture overrides to the bridge handshake.
    fn probe_strategy(&self) -> ProbeStrategy {
        ProbeStrategy::Liveness {
            argv: vec!["--version".to_string()],
        }
    }

    /// The argv token GROUPS this adapter REQUIRES in the manifest's hashed
    /// `argv_prefix`, enforced by [`refuse_missing_argv_flags`] at the
    /// composition root before the hash is bound into the cap-token and before
    /// the child is spawned. Each group is a CONTIGUOUS argv run — a (flag,
    /// value) pair must be adjacent, so `["--output-format","text",
    /// "--session-id","json"]` cannot satisfy a `--output-format json`
    /// requirement by token scatter (review 2a-P2: token-presence validation
    /// admitted malformed-but-plausible prefixes the oracle then misparsed).
    ///
    /// Why the seam exists: [`Self::parse_completion`] never sees `argv_prefix`
    /// — flags live only in the manifest, hashed into the cap-token — so an
    /// adapter whose oracle assumes a structured-output flag would read prose
    /// when the manifest omits it, fail to parse, and turn a REAL success into a
    /// false NEGATIVE: the exact inverse of the false-completion defect this
    /// seam was built to close. Declaring the dependency makes it checkable.
    ///
    /// Empty for the hermetic fixture — its marker oracle needs no flags.
    fn required_argv_flags(&self) -> &'static [&'static [&'static str]] {
        &[]
    }

    /// argv tokens whose presence BYPASSES the sandbox/permission posture —
    /// enforced by [`refuse_unsafe_argv`] at the same composition-root seam.
    /// Substring-matched against every token, so a bypass hiding as a flag
    /// VALUE (`--permission-mode bypassPermissions`) is caught too. For claude
    /// this is not only a jail concern: `bypassPermissions`/`dontAsk` also
    /// suppress `permission_denials`, the one field the oracle's verdict rests
    /// on, so a bypass flag makes the COMPLETION verdict itself untrustworthy.
    fn forbidden_argv_flags(&self) -> &'static [&'static str] {
        &[]
    }

    /// The argv-declared ISOLATION posture (AC4.1) — enforced where a POSITIVE
    /// `fs_jail: adapter-enforced-maos-declared` claim would be SEALED: the
    /// signed-run preflight (`demo-j1`) and the committed-manifest reader. NOT
    /// at plain `maos run` — a jail-less worker is legal; a capture that SAYS
    /// adapter-enforced without the declaration is a signed lie (review 2a-P3).
    /// Default `Ok` = exempt by construction (the hermetic fixture writes no
    /// files, so it has nothing to jail).
    fn refuse_missing_isolation(&self, argv_prefix: &[String]) -> Result<(), String> {
        let _ = argv_prefix;
        Ok(())
    }
}

/// How admission verifies a worker CLI (see [`WorkerCli::probe_strategy`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeStrategy {
    /// The kernel's Story 6.2 `--maos-bridge-probe` output-shape handshake.
    /// Fixture-only — the only CLI that answers the MAOS probe protocol.
    BridgeHandshake,
    /// A liveness probe for a real adapter-backed CLI: run it with `argv` and
    /// require a clean exit. The adapter (not a handshake) is the output-shape
    /// contract, verified at completion.
    Liveness { argv: Vec<String> },
}

/// Run a liveness probe: spawn `program argv`, require a clean exit within
/// `timeout`. Fail-closed on spawn error, non-zero exit, or timeout. Output is
/// discarded — a liveness probe checks invokability, NOT output shape (that is
/// the adapter's completion oracle). Mirrors the kernel probe's poll+timeout
/// shape so the real-CLI admission path has the same fail-loud guarantees.
pub fn run_liveness_probe(
    program: &str,
    argv: &[String],
    timeout: std::time::Duration,
) -> Result<(), String> {
    use std::process::{Command, Stdio};
    let mut child = Command::new(program)
        .args(argv)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("spawn failed for {program}: {e}"))?;
    let start = std::time::Instant::now();
    loop {
        match child
            .try_wait()
            .map_err(|e| format!("try_wait failed for {program}: {e}"))?
        {
            Some(status) if status.success() => return Ok(()),
            Some(status) => return Err(format!("non-zero exit ({status}) for {program}")),
            None => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("timed out after {timeout:?} for {program}"));
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        }
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

/// Refuse a manifest whose hashed `argv_prefix` omits any token GROUP the
/// selected adapter's completion oracle depends on
/// ([`WorkerCli::required_argv_flags`]). Fail-closed at the composition root,
/// BEFORE `argv_prefix_hash` is bound into the cap-token and BEFORE the spawn.
///
/// Matching is per GROUP (a contiguous argv run), never per isolated token:
/// `["--output-format","text","--session-id","json"]` carries both required
/// tokens but not the required PAIR, and admitting it hands the oracle prose
/// while the manifest reads as structured (review 2a-P2).
///
/// Two distinct failures this prevents, both of which produce a WRONG verdict
/// rather than an error:
/// - a missing structured-output flag (`codex exec --json`,
///   `claude --output-format json`) leaves the oracle parsing prose, so a real
///   success reports as a non-completion;
/// - a missing `claude --bare` leaves the run's behaviour a function of hooks,
///   LSP, plugin sync, auto-memory and `CLAUDE.md` auto-discovery — none of
///   which appear in the manifest or in `argv_prefix_hash`, and the child
///   inherits `maos`'s cwd in a repository that ships a tracked `CLAUDE.md`.
///   Without it, "reproducible from the repo" is a false claim, so `--bare` is
///   a REPRODUCIBILITY precondition and not merely credential hygiene.
pub fn refuse_missing_argv_flags(
    cli: &dyn WorkerCli,
    argv_prefix: &[String],
) -> Result<(), String> {
    let required = cli.required_argv_flags();
    let missing: Vec<String> = required
        .iter()
        .filter(|g| {
            !argv_prefix
                .windows(g.len())
                .any(|w| w.iter().zip(g.iter()).all(|(a, b)| a == b))
        })
        .map(|g| g.join(" "))
        .collect();
    if missing.is_empty() {
        return Ok(());
    }
    Err(format!(
        "worker cli '{}' requires argv_prefix token group(s) {missing:?} for its completion \
         oracle, but the manifest omits them. Refusing before probe and spawn: with the flag \
         missing the oracle reads prose, so a REAL success would be journaled as a \
         non-completion (and a claude run without --bare is not reproducible from the repo).",
        cli.name()
    ))
}

/// Refuse an argv that would NEUTRALIZE the posture the manifest declares
/// (review 2a-P2): a bypass flag from [`WorkerCli::forbidden_argv_flags`], or a
/// REPEATED isolation flag — adapters re-parse repeated flags last-wins, so
/// `--sandbox workspace-write … --sandbox danger-full-access` HASHES as a jail
/// while it EXECUTES without one. Enforced beside
/// [`refuse_missing_argv_flags`] at the composition root.
pub fn refuse_unsafe_argv(cli: &dyn WorkerCli, argv_prefix: &[String]) -> Result<(), String> {
    let hits: Vec<&str> = cli
        .forbidden_argv_flags()
        .iter()
        .filter(|f| argv_prefix.iter().any(|a| a.contains(*f)))
        .copied()
        .collect();
    if !hits.is_empty() {
        return Err(format!(
            "worker cli '{}' argv_prefix carries sandbox/permission bypass token(s) {hits:?}. A \
             manifest that declares a posture and then bypasses it is worse than one that \
             declares none — refusing before probe and spawn.",
            cli.name()
        ));
    }
    for flag in ["--sandbox", "--settings"] {
        if argv_prefix.iter().filter(|a| a.as_str() == flag).count() > 1 {
            return Err(format!(
                "worker cli '{}' repeats `{flag}` in argv_prefix: repeated isolation flags \
                 re-parse last-wins, so the hashed posture and the effective one silently \
                 diverge — refusing.",
                cli.name()
            ));
        }
    }
    Ok(())
}
/// The last non-empty, trimmed line of a captured stream (the "final message").
fn final_nonempty_line(lines: &[String]) -> Option<&str> {
    lines.iter().rev().map(|l| l.trim()).find(|l| !l.is_empty())
}

/// The codex completion oracle over `codex exec --json`.
///
/// codex emits a JSONL `ThreadEvent` stream — one object per line — so a
/// PER-LINE parse is correct here. (Contrast [`claude_result_object_oracle`],
/// which MUST join first.) Shapes below are the `codex-cli 0.144.4` contract
/// (`codex-rs/exec/src/exec_events.rs`, `ThreadEvent` is `#[serde(tag = "type")]`
/// and `ThreadItemDetails` is `#[serde(tag = "type", rename_all = "snake_case")]`
/// flattened into `ThreadItem`):
///
/// ```text
/// {"type":"thread.started","thread_id":"…"}
/// {"type":"item.completed","item":{"id":"i1","type":"file_change",
///   "changes":[{"path":"…","kind":"add"}],"status":"completed"}}
/// {"type":"item.completed","item":{"id":"i2","type":"agent_message","text":"…"}}
/// {"type":"turn.completed","usage":{…}}
/// {"type":"turn.failed","error":{"message":"…"}}
/// ```
///
/// Completion requires BOTH:
/// 1. the last terminal event is `turn.completed`, not `turn.failed`; and
/// 2. at least one `item.completed` carrying a `file_change` with a non-empty
///    `changes` array and `status: "completed"` — real, adapter-native EFFECT
///    evidence. A `file_change` with `status: "failed"` is a failed patch and is
///    NOT evidence.
///
/// (2) is sound rather than an over-constraint on read-only work because
/// [`CodexCli::required_argv_flags`] makes a missing `--sandbox workspace-write`
/// a REFUSAL: every codex run MAOS admits is write-class by construction, so
/// "completed a turn and wrote nothing" is a genuine non-completion.
///
/// The process exit code on `turn.failed` was NOT verified against the installed
/// binary, so nothing here is built on an assumed exit mapping — the airtight
/// case, and the one this oracle rules on, is "codex exits 0 on a *completed*
/// turn that produced no file".
fn codex_jsonl_oracle(stdout: &[String], exit: WorkerExit) -> WorkerCompletion {
    if !exit.is_clean() {
        return WorkerCompletion::NotCompleted(WorkerNonCompletion::ProcessCrash {
            exit_code: match exit {
                WorkerExit::Exited(c) => Some(c),
                WorkerExit::Crashed => None,
            },
        });
    }
    // `Some(true)` = the last terminal event was `turn.completed`; `Some(false)` =
    // `turn.failed`; `None` = the stream never reached a terminal event.
    let mut terminal: Option<bool> = None;
    let mut effect_evidence = false;
    let mut agent_message: Option<String> = None;
    for line in stdout {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line.trim()) else {
            continue;
        };
        match v.get("type").and_then(|t| t.as_str()) {
            Some("turn.completed") => terminal = Some(true),
            Some("turn.failed") => terminal = Some(false),
            Some("item.completed") => {
                let Some(item) = v.get("item") else { continue };
                match item.get("type").and_then(|t| t.as_str()) {
                    Some("file_change") => {
                        let applied =
                            item.get("status").and_then(|s| s.as_str()) == Some("completed");
                        let changed = item
                            .get("changes")
                            .and_then(|c| c.as_array())
                            .is_some_and(|c| !c.is_empty());
                        if applied && changed {
                            effect_evidence = true;
                        }
                    }
                    Some("agent_message") => {
                        if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                            agent_message = Some(text.to_string());
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    match terminal {
        Some(false) => WorkerCompletion::NotCompleted(WorkerNonCompletion::TurnFailed),
        None => WorkerCompletion::NotCompleted(WorkerNonCompletion::NoCompletionMarker),
        Some(true) if !effect_evidence => {
            WorkerCompletion::NotCompleted(WorkerNonCompletion::NoEffectEvidence)
        }
        // Completion does NOT depend on a message: the effect evidence is the
        // verdict. The agent message is carried when codex emitted one, so the
        // event has something citable, and is empty when it did not.
        Some(true) => WorkerCompletion::Completed {
            final_message: agent_message.unwrap_or_default(),
        },
    }
}

/// The claude completion oracle over `claude --print --output-format json`.
///
/// The result object, measured on `claude 2.1.233`:
///
/// ```text
/// {"type":"result","subtype":"success","is_error":false,"num_turns":3,
///  "result":"…","permission_denials":[],"total_cost_usd":0.0,…}
/// ```
///
/// Completion requires `subtype == "success"`, `is_error == false`, AND an
/// EMPTY `permission_denials`. An ABSENT `permission_denials` is refused, not
/// defaulted to empty: absence means the run cannot PROVE no tool permission was
/// denied, and an unprovable claim fails closed.
///
/// **This is NOT equivalent to [`codex_jsonl_oracle`], and the asymmetry must not
/// be papered over with shared wording.** codex proves EFFECT natively (an
/// applied `file_change`). claude's result object proves only that no tool
/// permission was DENIED: a refusal is emitted as `subtype: "success"`,
/// `is_error: false`, and a model that simply declines *without attempting a tool
/// call* leaves `permission_denials` empty and is indistinguishable from success.
/// claude's JSON detects the permission-denial defect; it is not an effect oracle.
/// That residual is claude's, is named here, and is not closed by this seam.
fn claude_result_object_oracle(stdout: &[String], exit: WorkerExit) -> WorkerCompletion {
    if !exit.is_clean() {
        return WorkerCompletion::NotCompleted(WorkerNonCompletion::ProcessCrash {
            exit_code: match exit {
                WorkerExit::Exited(c) => Some(c),
                WorkerExit::Crashed => None,
            },
        });
    }
    // `--output-format json` is ONE object that may be pretty-printed. Every
    // stdout line becomes its own `CliSubprocessOutput` TL row, so a per-line
    // parse (correct for codex's JSONL) would match no single line of a
    // pretty-printed object and convert a genuine success into a false negative.
    // The JOINED form parses whether the object arrived on one line or twenty.
    let joined = stdout.join("\n");
    let Ok(v) = serde_json::from_str::<serde_json::Value>(joined.trim()) else {
        return WorkerCompletion::NotCompleted(WorkerNonCompletion::NoCompletionMarker);
    };
    if v.get("type").and_then(|t| t.as_str()) != Some("result") {
        return WorkerCompletion::NotCompleted(WorkerNonCompletion::NoCompletionMarker);
    }
    let Some(denials) = v.get("permission_denials").and_then(|d| d.as_array()) else {
        return WorkerCompletion::NotCompleted(WorkerNonCompletion::NoCompletionMarker);
    };
    if v.get("subtype").and_then(|s| s.as_str()) != Some("success")
        || v.get("is_error").and_then(|e| e.as_bool()) != Some(false)
    {
        return WorkerCompletion::NotCompleted(WorkerNonCompletion::TurnFailed);
    }
    if !denials.is_empty() {
        return WorkerCompletion::NotCompleted(WorkerNonCompletion::PermissionDenied);
    }
    WorkerCompletion::Completed {
        final_message: v
            .get("result")
            .and_then(|r| r.as_str())
            .unwrap_or_default()
            .to_string(),
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

    /// The hermetic fixture is the ONE CLI that implements the kernel's
    /// `--maos-bridge-probe` output-shape handshake, so it uses it (hermetic
    /// Tier-1 keeps proving the kernel probe path).
    fn probe_strategy(&self) -> ProbeStrategy {
        ProbeStrategy::BridgeHandshake
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// codex — `codex exec --json "<task>"`: a JSONL ThreadEvent stream on stdout.
// ─────────────────────────────────────────────────────────────────────────────

/// The ratified first live worker: OpenAI `codex`. `codex exec` runs
/// non-interactively; the manifest's `argv_prefix` carries
/// `["exec", "--sandbox", "workspace-write", "--json"]` and the task is the
/// trailing argument.
///
/// The completion oracle is [`codex_jsonl_oracle`] over the `--json` event
/// stream, NOT the final stdout line: a clean exit with a fluent closing message
/// is exactly what a refusal looks like, and codex's stream carries native
/// `file_change` effect evidence instead. The long-form `--sandbox` spelling is
/// deliberate — `-s` is the same flag to codex and DIFFERENT BYTES to
/// `argv_prefix_hash`, and the Ed25519-signed T6 capture attests the long form.
pub struct CodexCli;

impl WorkerCli for CodexCli {
    fn name(&self) -> &'static str {
        "codex"
    }

    fn argv(&self, task: &str) -> Vec<String> {
        vec![task.to_string()]
    }

    /// `exec` is the non-interactive mode; `--json` is what
    /// [`codex_jsonl_oracle`] parses; `--sandbox workspace-write` is what makes
    /// requiring `file_change` effect evidence sound (every admitted codex run is
    /// write-class by construction) AND is the adapter-enforced FS jail this
    /// story ratifies. Long-form `--sandbox` only: `-s` is hash-different.
    /// Groups, not tokens: the (flag, value) pair must be ADJACENT in argv.
    fn required_argv_flags(&self) -> &'static [&'static [&'static str]] {
        &[&["exec"], &["--json"], &["--sandbox", "workspace-write"]]
    }

    fn forbidden_argv_flags(&self) -> &'static [&'static str] {
        &[
            "--dangerously-bypass-approvals-and-sandbox",
            "danger-full-access",
            "--yolo",
        ]
    }

    /// AC4.1's seal-side gate: codex's isolation posture IS the long-form
    /// `--sandbox workspace-write` pair (the bytes the Ed25519-signed T6 capture
    /// attests), so refusing to seal without it is refusing to sign a jail the
    /// argv never declared.
    fn refuse_missing_isolation(&self, argv_prefix: &[String]) -> Result<(), String> {
        if argv_prefix
            .windows(2)
            .any(|w| w[0] == "--sandbox" && w[1] == "workspace-write")
        {
            return Ok(());
        }
        Err(
            "codex's isolation posture is the ADAPTER-enforced long-form `--sandbox \
             workspace-write` (the exact bytes the signed T6 capture attests); this argv does \
             not carry it, so no adapter-enforced-maos-declared claim may be sealed"
                .to_string(),
        )
    }

    fn nonsecret_env(&self) -> Vec<(String, String)> {
        // Non-interactive; keeps codex from prompting for a TTY. The credential is
        // `CODEX_API_KEY` — NOT `OPENAI_API_KEY`. `codex exec` ignores
        // OPENAI_API_KEY for auth; it reads CODEX_API_KEY via load_auth's
        // enable_codex_api_key_env path (codex-rs login/src/auth/manager.rs:1226,
        // enabled for exec at exec/src/lib.rs:571). It is inherited host-side from
        // the maos process env, NEVER set here (so MAOS never holds the value).
        vec![("CODEX_NON_INTERACTIVE".to_string(), "1".to_string())]
    }

    /// `codex exec` IGNORES `OPENAI_API_KEY` for auth — it reads `CODEX_API_KEY`
    /// via `load_auth`'s `enable_codex_api_key_env` path. Getting this wrong makes
    /// a redaction scan search for a value the child never used.
    fn credential_env_var(&self) -> Option<&'static str> {
        Some("CODEX_API_KEY")
    }

    fn parse_completion(
        &self,
        stdout: &[String],
        _stderr: &[String],
        exit: WorkerExit,
    ) -> WorkerCompletion {
        codex_jsonl_oracle(stdout, exit)
    }

    fn ambient_auth_path(&self, home: &std::path::Path) -> Option<std::path::PathBuf> {
        // ChatGPT-login writes a plaintext subscription token here. Note
        // `CODEX_API_KEY` actually takes PRECEDENCE over auth.json (manager.rs:1226
        // is checked before the file store), so this is NOT about shadowing — it is
        // a hard refusal on the live path because an un-attestable subscription
        // credential must not exist in the signed run's sandbox home at all
        // (MAOS cannot prove it scrubbed a token it never held). Refuse-or-wipe.
        Some(home.join(".codex").join("auth.json"))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// claude — `claude --print --output-format json "<task>"`: one result object.
// ─────────────────────────────────────────────────────────────────────────────

/// Anthropic `claude --print` (non-interactive). The manifest's `argv_prefix`
/// carries `--print --output-format json --bare` plus an explicit permission
/// posture and an argv-hashed `--settings` document; the task is the trailing
/// argument and the result object lands on stdout.
///
/// The completion oracle is [`claude_result_object_oracle`], NOT the final stdout
/// line. A `--print` run has no TTY to approve a tool call, so a denied tool call
/// makes the model explain itself in prose and exit 0 — the false completion this
/// story exists to stop. The result object's `permission_denials` is the only
/// field that distinguishes that case; see the oracle for the residual it cannot
/// cover.
pub struct ClaudeCli;

impl WorkerCli for ClaudeCli {
    fn name(&self) -> &'static str {
        "claude"
    }

    fn argv(&self, task: &str) -> Vec<String> {
        vec![task.to_string()]
    }

    /// Under `--bare`, claude's auth is strictly `ANTHROPIC_API_KEY` or an
    /// `apiKeyHelper` named in `--settings` — OAuth and the keychain are never
    /// read. `ANTHROPIC_API_KEY` is therefore the value a signed claude run must
    /// scan for; scanning `CODEX_API_KEY` would be a silent no-op.
    fn credential_env_var(&self) -> Option<&'static str> {
        Some("ANTHROPIC_API_KEY")
    }
    /// `--output-format json` is what [`claude_result_object_oracle`] parses.
    /// `--bare` is NOT merely credential hygiene: it makes auth strictly
    /// `ANTHROPIC_API_KEY` or `apiKeyHelper` via `--settings` (OAuth and keychain
    /// are never read), DELETING two credential surfaces rather than mitigating
    /// them — and it skips hooks, LSP, plugin sync, auto-memory and **`CLAUDE.md`
    /// auto-discovery**. The spawn sets no `cwd`, so the child inherits `maos`'s
    /// working directory, and this repository ships a tracked `CLAUDE.md`; without
    /// `--bare` a signed run's behaviour depends on instruction files that appear
    /// in no manifest and no `argv_prefix_hash`, so "reproducible from the repo"
    /// would be false. Do not delete `--bare` as redundant credential hygiene.
    /// Groups, not tokens: `--output-format json` must be ADJACENT in argv.
    fn required_argv_flags(&self) -> &'static [&'static [&'static str]] {
        &[&["--print"], &["--output-format", "json"], &["--bare"]]
    }

    fn forbidden_argv_flags(&self) -> &'static [&'static str] {
        // `bypassPermissions`/`dontAsk` are doubly forbidden here: they skip the
        // permission posture AND suppress `permission_denials` — the one field
        // [`claude_result_object_oracle`]'s verdict rests on — so a bypass flag
        // makes the completion verdict itself untrustworthy.
        &[
            "bypassPermissions",
            "dontAsk",
            "danger-full-access",
            "--yolo",
        ]
    }

    /// AC4.1's seal-side gate. claude's jail is a `--settings` DOCUMENT, not a
    /// flag, so token presence proves nothing: the payload must parse as JSON
    /// and enable `sandbox` (`sandbox.enabled` is claude's fail-closed hard gate
    /// — it exits at startup if the sandbox cannot start). An empty `{}` or a
    /// settings doc without the sandbox object does not declare a jail, and no
    /// `adapter-enforced-maos-declared` claim may be sealed over it (review
    /// 2a-P3: the reader previously asserted the token only).
    fn refuse_missing_isolation(&self, argv_prefix: &[String]) -> Result<(), String> {
        let settings = argv_prefix
            .iter()
            .position(|a| a == "--settings")
            .and_then(|i| argv_prefix.get(i + 1))
            .ok_or(
                "claude's isolation posture is an argv-hashed `--settings` document; this argv \
                 declares none, so no adapter-enforced-maos-declared claim may be sealed",
            )?;
        let doc: serde_json::Value = serde_json::from_str(settings).map_err(|_| {
            "claude's `--settings` payload does not parse as JSON, so the isolation posture it \
             would declare cannot be read"
                .to_string()
        })?;
        match doc
            .get("sandbox")
            .and_then(|s| s.get("enabled"))
            .and_then(|e| e.as_bool())
        {
            Some(true) => Ok(()),
            _ => Err(
                "claude's `--settings` document does not enable `sandbox` (`sandbox.enabled` \
                 must be true — the adapter's fail-closed startup gate); without it no \
                 adapter-enforced-maos-declared claim may be sealed"
                    .to_string(),
            ),
        }
    }

    fn parse_completion(
        &self,
        stdout: &[String],
        _stderr: &[String],
        exit: WorkerExit,
    ) -> WorkerCompletion {
        claude_result_object_oracle(stdout, exit)
    }

    /// claude's OAuth/subscription credential file. Note the LEADING DOT on the
    /// filename — `~/.claude/.credentials.json`, unlike codex's `~/.codex/auth.json`.
    ///
    /// This inverts a green in-repo claim: the seam previously asserted
    /// `ClaudeCli.ambient_auth_path(home) == None` under a test whose comment read
    /// *"only codex names the footgun"*. That was a false statement with a passing
    /// test behind it, and it made `refuse_ambient_auth` a NO-OP for claude — so a
    /// signed claude run would have stamped a redaction claim over a subscription
    /// token MAOS never held.
    ///
    /// **Two residuals, stated here rather than implied away by a wider type.**
    ///
    /// 1. A file-existence check is a **filename** control, not a credential
    ///    control. `~/.codex/auth.json.bk` exists on the development box: the codex
    ///    invariant was satisfied by RENAMING, not by removing.
    /// 2. claude's credential surface also includes `~/.claude.json`, the OS
    ///    keychain, `ANTHROPIC_API_KEY`, `ANTHROPIC_AUTH_TOKEN`,
    ///    `CLAUDE_CODE_OAUTH_TOKEN`, an `apiKeyHelper` command named in settings,
    ///    enterprise managed settings, and Bedrock/Vertex ambient cloud
    ///    credentials — NONE reachable by a path check. Widening the return type to
    ///    `Vec<PathBuf>` would buy exactly one of those while IMPLYING the keychain
    ///    and the environment were covered, so the type stays a single `PathBuf`.
    ///    The real control is structural and lives in
    ///    [`Self::required_argv_flags`]: `--bare` DELETES the OAuth and keychain
    ///    surfaces outright.
    ///
    /// What remains after `--bare` is one variable on an inherited environment
    /// channel — see [`WorkerCli::nonsecret_env`] for why that channel is
    /// unattested and why closing it is not this seam's repair.
    fn ambient_auth_path(&self, home: &std::path::Path) -> Option<std::path::PathBuf> {
        Some(home.join(".claude").join(".credentials.json"))
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
