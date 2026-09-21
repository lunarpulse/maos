#![forbid(unsafe_code)]

//! `maos-shell` — J0 evaluator surface.
//!
//! Provides:
//! - `init`    — scaffold `~/.maos/` (config, slots, skills)
//! - `shell`   — kernel-rendered REPL (`@<spirit> <msg>`)
//! - `audit`   — thin read-side alias over `maos_audit::query`

use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::sync::Arc;

use maos_cli::accessibility::ColorChoice;
use maos_domain::halt::HaltState;
use maos_domain::invariants::i1::CapabilityToken;
use maos_domain::ports::capability::CapError;
use maos_domain::ports::inference::InferencePort;

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

fn transparency_log_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let home_team = std::env::var("MAOS_LOOM_HOME_TEAM").ok();
    let collective_configured = std::env::var_os("MAOS_LOOM_POSTGRES").is_some();
    let path = maos_audit::transparency_log_path_for_tenant_mode(
        collective_configured,
        home_team.as_deref(),
    )?;
    maos_audit::validate_transparency_log_path(&path)?;
    if collective_configured {
        if let Some(team) = home_team
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            let team = maos_domain::team::TeamId::new(team)?;
            maos_audit::validate_transparency_log_team_binding(&path, &team)?;
        }
    }
    Ok(path)
}

/// Run `maos init` — scaffold the MAOS home if absent.
///
/// Idempotent: re-running prints "already initialized" and exits 0, and never
/// rewrites a byte of `control.json`.
///
/// ⚠ Story 16-1 — the directory tree is created FIRST. Until this story the
/// `create_new(true)` open of `config.toml` ran BEFORE `create_dir_all(&home)`,
/// so `maos init` exited 1 with `ENOENT` on every machine that did not already
/// have the home — J0's very first command, on a clean install. All four init
/// tests passed because all four fixtures pre-created the directory.
pub fn run_init(color_choice: ColorChoice) -> Result<(), Box<dyn std::error::Error>> {
    // `maos init` prints to the real stdout (not a REPL seam); routed through
    // the same writer-taking helper so there is ONE rendering rule.
    let mut out = std::io::stdout();
    let home = maos_home()?;
    let config_path = home.join("config.toml");

    // Story 16-1 — the home exists BEFORE anything is opened inside it, at
    // `0700` (D-16-1-D: any uid that can `open()` a store directory can hold a
    // shared flock on it and keep a root from booting).
    maos_domain::operator_door::ensure_home_dir(&home)?;
    for leaf in ["skills", "audit", "journal"] {
        std::fs::create_dir_all(home.join(leaf))?;
    }

    // Atomic create-exclusive: fails if config.toml already exists (idempotent guard).
    let mut file = match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&config_path)
    {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            // Validate existing config is non-trivial.
            let existing = std::fs::read_to_string(&config_path).unwrap_or_default();
            if existing.contains("[slots]") && existing.contains("[retention]") {
                // ⚠ The door config is minted even here. A home initialised
                // before Story 16-1 is complete by this test and has NO
                // `control.json`; taking the shortcut would leave every such
                // machine with no operator door and no way to get one short of
                // deleting `config.toml`.
                ensure_operator_door(&home, color_choice)?;
                print_line(
                    &mut out,
                    color_choice,
                    &format!(
                        "maos: already initialized — {} exists",
                        config_path.display()
                    ),
                )?;
                return Ok(());
            }
            // Truncated/corrupt config — fall through to recreate.
            eprintln!("maos: config exists but is incomplete; regenerating...");
            std::fs::OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(&config_path)?
        }
        Err(e) => return Err(e.into()),
    };

    // Write config.toml.
    let config = default_config_toml();
    file.write_all(config.as_bytes())?;

    // Stage BMAD skills if the repo skill set is present.
    if let Ok(repo_skills) = std::env::var("MAOS_REPO_ROOT") {
        let src = PathBuf::from(repo_skills).join("_bmad").join("skills");
        if src.is_dir() {
            let _ = copy_dir_all(&src, &home.join("skills"));
        }
    }

    print_line(
        &mut out,
        color_choice,
        &format!("maos: initialized {}", home.display()),
    )?;
    print_line(
        &mut out,
        color_choice,
        &format!(
            "maos: config written to {}  (6 default slots + retention=persist)",
            config_path.display()
        ),
    )?;
    ensure_operator_door(&home, color_choice)?;
    let audit_path = transparency_log_path()?;
    print_line(
        &mut out,
        color_choice,
        &format!("maos: Transparency Log will be at {}", audit_path.display()),
    )?;
    print_line(
        &mut out,
        color_choice,
        "maos: to remove all data, run:  maos purge --yes",
    )?;
    Ok(())
}

/// Story 16-1 / ADR-062 — `maos init` is the ONE writer of `control.json`.
///
/// The daemon does not write it at bind (that would be a second trust path for
/// the same token), and neither does `maosctl`. Rotation is
/// `rm control.json && maos init`.
fn ensure_operator_door(
    home: &std::path::Path,
    color_choice: ColorChoice,
) -> Result<maos_domain::operator_door::ControlFileState, Box<dyn std::error::Error>> {
    use maos_domain::operator_door::{ensure_control_file, ControlFileState};
    let (control, state) = ensure_control_file(home)?;
    if state == ControlFileState::Minted {
        print_line(
            &mut std::io::stdout(),
            color_choice,
            &format!(
                "maos: operator door endpoint {} recorded in {}",
                control.endpoint,
                maos_domain::operator_door::control_file_path(home).display()
            ),
        )?;
    }
    Ok(state)
}

/// Run `maos audit query` — thin alias over `maos_audit::query`.
pub fn run_audit_query(
    spirit: Option<&str>,
    format: &str,
    _color_choice: ColorChoice, // Accepts caller's NO_COLOR intent; library functions emit no ANSI regardless.
) -> Result<(), Box<dyn std::error::Error>> {
    let db_path = transparency_log_path()?;
    if !db_path.exists() {
        eprintln!("maos: no Transparency Log found at {}", db_path.display());
        return Err("audit log not found".into());
    }

    let mut filter = maos_audit::AuditFilter::default();
    if let Some(s) = spirit {
        // Story 16-2 / D-16-2-F — the private `hello-spirit => pid 0` table
        // is DELETED; the name resolves through the identity rows like every
        // Spirit, filtered on BOTH the latest boot's nonce and its pid (a
        // pid-only filter mixes butler's pid 1 with hello-spirit's pid 1
        // across boots on one Transparency Log).
        let pairs = maos_audit::resolve_spirit_name(&db_path, s, false)?;
        let Some((boot_nonce, pid)) = pairs.first() else {
            return Err(format!("unknown spirit '{s}'").into());
        };
        filter.boot_nonce = Some(*boot_nonce);
        filter.spirit_pid = Some(*pid);
    }

    let entries = maos_audit::query(&db_path, filter)?;

    let stdout = std::io::stdout();
    let mut lock = stdout.lock();

    let audit_health = maos_kernel_core::capability::cap_audit::audit_health_snapshot();
    if audit_health.degraded {
        if format == "ndjson" {
            writeln!(
                lock,
                "{}",
                serde_json::json!({
                    "kind": "audit.health",
                    "degraded": true,
                    "dropped_events": audit_health.total_drops,
                })
            )?;
        } else {
            writeln!(
                lock,
                "WARNING audit.health degraded=true dropped_events={}",
                audit_health.total_drops
            )?;
        }
    }

    let fr4_mode = spirit.is_some();
    match (fr4_mode, format) {
        (true, "ndjson") => maos_audit::to_fr4_ndjson(entries, &mut lock)?,
        (true, _) => maos_audit::to_fr4_plain(entries, &mut lock)?,
        (false, "ndjson") => maos_audit::to_ndjson(entries, &mut lock)?,
        (false, _) => maos_audit::to_plain(entries, &mut lock)?,
    }

    // Ensure trailing newline (plain table already adds one; NDJSON may not).
    let _ = writeln!(lock);

    Ok(())
}

/// Story 16-2 / D-16-2-B — the port between the REPL and the kernel side.
///
/// The REPL stays testable against a fake host (no kernel objects — the
/// `crates/maos-shell/tests/` vectors drive the halt, refusal and resolution
/// renders through one), and `maos-bin` implements it in
/// `crates/maos-bin/src/shell_host.rs` over the daemon's real Transparency
/// Log, shared journal, halt registry, memory manager and operator-door port.
pub trait ShellHost: Send + Sync {
    /// The scheduler-assigned pid of the loaded hello-spirit.
    fn spirit_pid(&self) -> u32;

    /// Issue this turn's capability token (`cap.issue` at the real pid).
    fn issue_turn_token(&self) -> Result<CapabilityToken, CapError>;

    /// Record a completed turn (`shell.turn`). The result passes through
    /// UNCHANGED — the one swallow stays at the call site in [`run_shell`]
    /// (the `let _ = host.record_turn(..)` Story 16-5 AC1 cites).
    fn record_turn(&self, token: &CapabilityToken, payload: &[u8]) -> Result<(), CapError>;

    /// Raise the ambiguity halt through the kernel's `invoke_halt` — the ONE
    /// owner of the TL row, the Lifecycle Journal `Halt` entry and the
    /// registry insert. Returns the minted halt id (ULID, the kernel's
    /// halt-id grammar).
    fn raise_ambiguity_halt(
        &self,
        tag: &str,
        prompt: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;

    /// Read-only registry lookup of the halt's current state. The REPL polls
    /// this while a halt is pending (D-16-2-D: the REPL observes the
    /// registry, not a message).
    fn halt_state(&self, halt_id: &str) -> Option<HaltState>;

    /// Read the resolver-delivered context (`halt_context::<id>` from the
    /// Spirit's private tier — the resolver's own delivery channel).
    /// `None` until the resolver's memory write lands: the registry state
    /// flips BEFORE the write (Trap 5), so a `Resumed` state can briefly
    /// still read no context.
    fn take_context(&self, halt_id: &str) -> Option<String>;

    /// Submit the operator's typed clarification through the SAME operator
    /// door `maosctl halt resolve` reaches (D-16-2-C: one mechanism, two
    /// surfaces) — `ResolveHalt { provided_context }` — and wait on its
    /// completion: same validation, same `KernelHaltResolver`, same
    /// approval-log row, same completion TL row.
    ///
    /// `Ok(())` means the door accepted the resolution (or the command is
    /// still queued/running — the tick observes the registry either way);
    /// `Err` carries the door's typed refusal (`halt_already_resolved`,
    /// `halt_not_pending`, …), which the REPL prints without un-pending —
    /// the winner still renders on the next tick.
    fn resolve_with_context(&self, halt_id: &str, text: &str) -> Result<(), ResolveRefusal>;
}

/// A typed refusal from the door, as [`ShellHost::resolve_with_context`]
/// reports it. The code is the door's own (`halt_already_resolved`, …) —
/// never an HTTP status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveRefusal {
    pub code: String,
    pub detail: String,
}

impl std::fmt::Display for ResolveRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.detail)
    }
}

/// Run the kernel-rendered shell REPL.
///
/// `inference` — the composed Inference Port (live or deterministic).
/// `host`      — the kernel-side port (tokens, turn rows, halts, context).
/// `input`     — stdin seam; `'static + Send` because D-16-2-D moves the
///               reader onto its own thread. Production passes
///               `std::io::BufReader::new(std::io::stdin())` — NOT
///               `stdin().lock()` (a `MutexGuard`, not `Send`).
/// `output`    — every REPL line goes through here (banner, refusals,
///               resolution renders, `shell exiting`) — production passes
///               `std::io::stdout()`; tests pass `&mut Vec<u8>`.
///
/// Reads lines, parses `@<spirit> <msg>`, dispatches in-proc.
pub fn run_shell(
    inference: Arc<dyn InferencePort + Send + Sync>,
    host: &dyn ShellHost,
    input: impl BufRead + Send + 'static,
    mut output: impl Write,
    color_choice: ColorChoice,
    default_provider: &str,
    transparency_log_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let _ = default_provider; // provider choice lives behind ShellHost::issue_turn_token

    print_line(
        &mut output,
        color_choice,
        "maos shell — type @hello-spirit <msg>  (Ctrl-D to exit)",
    )?;

    // D-16-2-D — the REPL observes the registry, not a message. stdin is
    // read on a dedicated thread into a channel; the loop `recv_timeout`s
    // at a fixed tick so a halt resolved from ANOTHER surface (maosctl over
    // the door) renders here without any local input. Trap 13: the channel
    // disconnects at EOF — Disconnected IS EOF, never a spin.
    let (line_tx, line_rx) = std::sync::mpsc::channel::<String>();
    std::thread::spawn(move || {
        for line in input.lines() {
            match line {
                Ok(l) => {
                    if line_tx.send(l).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    let mut pending: Option<PendingHalt> = None;
    loop {
        match line_rx.recv_timeout(REPL_TICK) {
            Ok(line) => {
                let trimmed = line.trim().to_string();
                if trimmed.is_empty() {
                    continue;
                }
                if let Err(error) = handle_line(
                    &inference,
                    host,
                    &trimmed,
                    &mut pending,
                    &mut output,
                    color_choice,
                    transparency_log_path,
                ) {
                    let _ = output.flush();
                    print_line(&mut output, color_choice, "maos: shell exiting")?;
                    return Err(error);
                }
                output.flush()?;
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // The tick: render a resolution that arrived from any
                // surface, then proceed with the turn if the Spirit resumes.
                if let Err(error) = tick(
                    &inference,
                    host,
                    &mut pending,
                    &mut output,
                    color_choice,
                    transparency_log_path,
                ) {
                    let _ = output.flush();
                    print_line(&mut output, color_choice, "maos: shell exiting")?;
                    return Err(error);
                }
                output.flush()?;
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break, // EOF
        }
    }

    print_line(&mut output, color_choice, "maos: shell exiting")?;
    Ok(())
}

/// The REPL tick — the resolution cadence (D-16-2-D).
const REPL_TICK: std::time::Duration = std::time::Duration::from_millis(100);

/// Trap 5 — the registry state flips BEFORE the resolver's memory write, so
/// a `Resumed` halt can briefly still read no context. The REPL retries
/// `take_context` for a bounded number of ticks and NEVER proceeds with an
/// empty context.
const CONTEXT_WAIT_TICKS: u32 = 50; // 50 × 100 ms = 5 s
const MAX_CLARIFICATION_BYTES: usize = 64 * 1024;

/// The REPL's pending-halt state machine.
struct PendingHalt {
    id: String,
    /// The directive that halted — the proceeding turn re-runs it.
    directive: String,
    /// `Some(state)` once the registry left `PendingResolution` and the
    /// resolution line has NOT yet rendered (context wait in progress).
    resolved: Option<HaltState>,
    /// Ticks spent waiting for the resolver's context write (Trap 5).
    context_ticks: u32,
}

/// One REPL input line (a directive, a clarification, or a refusal to
/// render). Returns `Err` only for the shell-fatal class (15-6 D2).
fn handle_line(
    inference: &Arc<dyn InferencePort + Send + Sync>,
    host: &dyn ShellHost,
    trimmed: &str,
    pending: &mut Option<PendingHalt>,
    output: &mut impl Write,
    color_choice: ColorChoice,
    transparency_log_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // A pending halt reshapes every line: `@…` is refused naming the id, any
    // other non-empty line is the clarification (D-16-2-D).
    if let Some(p) = pending.as_ref() {
        if trimmed.starts_with('@') {
            writeln!(
                output,
                "maos: a halt is pending — halt {} is waiting for a clarification; \
                 resolve it first (or: maosctl halt resolve {} --spirit hello-spirit \
                 --kind provided-context --text \"…\")",
                p.id, p.id
            )?;
            return Ok(());
        }
        if trimmed.len() > MAX_CLARIFICATION_BYTES {
            writeln!(
                output,
                "maos: clarification refused — {} bytes exceeds the {}-byte limit",
                trimmed.len(),
                MAX_CLARIFICATION_BYTES
            )?;
            return Ok(());
        }
        let text = trimmed.to_string();
        match host.resolve_with_context(&p.id, &text) {
            Ok(()) => {
                // No render on submit (the falsifier pins this): the TICK
                // renders the resolution from the registry, so the REPL and
                // the door path are indistinguishable. Proven red: replacing
                // the tick render with a submit render leaves the door-path
                // vector (no stdin line) without a resolution line.
            }
            Err(refusal) if refusal.code == "halt_already_resolved" => {
                // R8 — a late clarification reads like user error; say what
                // happened to the text instead of silently dropping it. The
                // halt stays pending in the REPL's state; the winner renders
                // on the next tick.
                let kind = resolution_kind_name(host.halt_state(&p.id));
                writeln!(
                    output,
                    "no halt is pending — halt {} was already resolved ({kind}); \
                     your text was not sent",
                    p.id
                )?;
            }
            Err(refusal) => {
                writeln!(output, "maos: halt resolve refused: {refusal}")?;
            }
        }
        return Ok(());
    }

    // Parse `@<spirit> <msg>`.
    let (spirit_name, msg) = match parse_at_line(trimmed) {
        Some(p) => p,
        None => {
            print_line(output, color_choice, "maos: expected @<spirit> <message>")?;
            return Ok(());
        }
    };

    // Dispatch (v0.1-β: hello-spirit wired; v0.3-β: butler pick prototype).
    if spirit_name == "butler" {
        // Story 8.14b FORK 2 — option-pick dispatch surface.
        // Full scheduler access (real running Butler) is Epic 9.
        // Prototype: render option-pick outcome message directly.
        let option = msg
            .trim()
            .strip_prefix("pick ")
            .and_then(|s| s.chars().next())
            .unwrap_or('?');
        let message = match option {
            'a' => "Linear note written: Calendar conflict — evt-a ↔ evt-b (stub — real dispatch requires scheduler access, Epic 9)",
            'b' => "Butler: Slack message queued for [partner] (live send v0.4)",
            'c' => "Butler: snoozed — will re-check at 12:00 UTC (stub)",
            _ => "maos: butler pick error: no pending notification to pick from",
        };
        writeln!(output, "{message}")?;
        return Ok(());
    }
    if spirit_name != "hello-spirit" {
        print_line(
            output,
            color_choice,
            &format!(
                "maos: unknown spirit '{spirit_name}' — known: hello-spirit, butler (pick only)",
            ),
        )?;
        return Ok(());
    }
    let token: CapabilityToken = host
        .issue_turn_token()
        .map_err(|e| format!("capability issue failed: {e}"))?;

    // Dispatch to hello-spirit.
    let token_for_audit = token.clone();
    let response = if msg.to_lowercase().starts_with("say hi") {
        maos_spirit_hello::say_hi(&**inference, token, transparency_log_path)
    } else {
        maos_spirit_hello::dispatch_directive(&**inference, token, msg, transparency_log_path)
    };

    match response {
        Ok(resp) => {
            render_turn(output, msg, &resp)?;
            // Operator ruling 2026-09-17 (D-16-5-D): drop + disclose — the
            // session survives; the counted drop and the daemon's degraded
            // latch carry the failure.
            record_turn_row(host, &token_for_audit, msg, &resp);
        }
        Err(maos_spirit_hello::HelloError::Ambiguous { tag, prompt }) => {
            // D-16-2-B — the halt goes through the kernel's `invoke_halt`
            // (TL row + journal + registry). The line is printed ONLY after
            // it returned Ok, so `halt list` can never race an id the
            // Transparency Log does not have yet. The old `shell.halt`
            // `record_invocation` call is DELETED: no call was made, and a
            // `capability.invocation` row for a halt claims one.
            match host.raise_ambiguity_halt(&tag, &prompt) {
                Ok(halt_id) => {
                    writeln!(output, "[HALT {tag}] {prompt}")?;
                    print_line(
                        output,
                        color_choice,
                        &format!(
                            "halt {halt_id} — type a clarification, or: \
                             maosctl halt resolve {halt_id} --spirit hello-spirit \
                             --kind provided-context --text \"…\""
                        ),
                    )?;
                    *pending = Some(PendingHalt {
                        id: halt_id,
                        directive: msg.to_string(),
                        resolved: None,
                        context_ticks: 0,
                    });
                }
                Err(e) => {
                    writeln!(output, "maos: error: {e}")?;
                }
            }
        }
        Err(e) => {
            // 15-6 §A6 review D2: a failed inference turn (exhausted
            // cassette, strict drift, unreachable provider) must fail the
            // shell — printed AND non-zero, never a quiet exit 0.
            writeln!(output, "maos: error: {e}")?;
            return Err(e.into());
        }
    }
    Ok(())
}

/// The tick: observe the registry, render a resolution from ANY surface, and
/// run the proceeding turn (D-16-2-D).
fn tick(
    inference: &Arc<dyn InferencePort + Send + Sync>,
    host: &dyn ShellHost,
    pending: &mut Option<PendingHalt>,
    output: &mut impl Write,
    color_choice: ColorChoice,
    transparency_log_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(p) = pending.as_mut() else {
        return Ok(());
    };
    if p.resolved.is_none() {
        match host.halt_state(&p.id) {
            Some(HaltState::PendingResolution) => return Ok(()),
            Some(state) => p.resolved = Some(state),
            None => {
                // The registry no longer knows the id (terminated and
                // drained). The EOF path owns receipts; nothing to render.
                *pending = None;
                return Ok(());
            }
        }
    }
    let Some(state) = p.resolved else {
        return Ok(());
    };
    match state {
        HaltState::Resumed => {
            // Trap 5 — bounded context read; never proceed with an empty
            // context.
            match host.take_context(&p.id) {
                Some(context) => {
                    writeln!(output, "halt {} resolved: provided_context", p.id)?;
                    let directive = p.directive.clone();
                    *pending = None;
                    return proceed_with(
                        inference,
                        host,
                        output,
                        &directive,
                        Some(&context),
                        transparency_log_path,
                    );
                }
                None => {
                    p.context_ticks += 1;
                    if p.context_ticks >= CONTEXT_WAIT_TICKS {
                        writeln!(
                            output,
                            "halt {} resolved: provided_context (context not delivered)",
                            p.id
                        )?;
                        *pending = None;
                    }
                }
            }
        }
        HaltState::Terminated => {
            writeln!(output, "halt {} resolved: accepted_halt", p.id)?;
            print_line(output, color_choice, "turn abandoned (accepted_halt)")?;
            *pending = None;
        }
        HaltState::Overridden => {
            writeln!(output, "halt {} resolved: authorized_override", p.id)?;
            let directive = p.directive.clone();
            *pending = None;
            return proceed_with(
                inference,
                host,
                output,
                &directive,
                None,
                transparency_log_path,
            );
        }
        HaltState::PendingResolution => {}
    }
    Ok(())
}

/// The proceeding turn — rendered and journaled exactly like a normal turn
/// (`cap.issue`, the response, `shell.turn`).
fn proceed_with(
    inference: &Arc<dyn InferencePort + Send + Sync>,
    host: &dyn ShellHost,
    output: &mut impl Write,
    directive: &str,
    context: Option<&str>,
    transparency_log_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let token = host
        .issue_turn_token()
        .map_err(|e| format!("capability issue failed: {e}"))?;
    let token_for_audit = token.clone();
    let response = maos_spirit_hello::proceed_with_context(
        &**inference,
        token,
        directive,
        context,
        transparency_log_path,
    )?;
    render_turn(output, directive, &response)?;
    let summary = match context {
        Some(text) => format!("{directive} [clarification: {text}]"),
        None => format!("{directive} [authorized override]"),
    };
    record_turn_row(host, &token_for_audit, &summary, &response);
    Ok(())
}

fn render_turn(
    output: &mut impl Write,
    _msg: &str,
    resp: &maos_spirit_hello::HelloResponse,
) -> Result<(), Box<dyn std::error::Error>> {
    writeln!(
        output,
        "{introduction}\n\nPosture: {posture}\nCapability scope: {scope:?}\nHalt tags: {tags:?}\nTransparency Log: {log}",
        introduction = resp.introduction,
        posture = resp.posture,
        scope = resp.capability_scope,
        tags = resp.halt_tags,
        log = resp.transparency_log,
    )?;
    Ok(())
}

fn record_turn_row(
    host: &dyn ShellHost,
    token: &CapabilityToken,
    msg: &str,
    resp: &maos_spirit_hello::HelloResponse,
) {
    let payload = serde_json::json!({
        "user": msg,
        "response": resp.introduction,
    })
    .to_string();
    // D-16-5-D / AC1: the response is already rendered, so this site cannot
    // undo the invocation and is NOT converted to a refusal. The drop is
    // counted by the class-wide instrument (the port's send error) and the
    // operator gets one honest line; the turn and the session survive.
    if let Err(error) = host.record_turn(token, payload.as_bytes()) {
        eprintln!(
            "maos: WARNING audit row for this turn was dropped (audit sink unavailable: {error}); \
             the session log for this turn is incomplete"
        );
    }
}

/// The resolution kind name the renders use — the door's vocabulary.
fn resolution_kind_name(state: Option<HaltState>) -> &'static str {
    match state {
        Some(HaltState::Resumed) => "provided_context",
        Some(HaltState::Terminated) => "accepted_halt",
        Some(HaltState::Overridden) => "authorized_override",
        _ => "resolved",
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Story 16-1 / D-16-1-C — ONE home rule, owned by `maos-domain`.
///
/// This was a private second copy of the rule (`MAOS_HOME`, else
/// `$HOME/.maos`) that `panicked` when neither was set. `maosctl` cannot depend
/// on `maos-shell` — the crate edge runs the other way — so the rule had to
/// move to the one crate `maos init`, every daemon root and `maosctl` all
/// already depend on.
fn maos_home() -> Result<PathBuf, Box<dyn std::error::Error>> {
    maos_domain::operator_door::maos_home()?.ok_or_else(|| {
        Box::<dyn std::error::Error>::from(
            "maos: neither MAOS_HOME nor HOME is set — set one to proceed",
        )
    })
}

fn default_config_toml() -> String {
    r#"# MAOS user configuration (generated by `maos init`)

[slots]
worker = ["w1", "w2", "w3", "w4", "w5"]
orchestrator = ["orch1"]

[retention]
default = "persist"
# `maos purge --yes` removes local state regardless of this runtime policy.

[paths]
home = "~/.maos"
audit = "~/.maos/audit"
journal = "~/.maos/journal"
"#
    .to_string()
}

fn parse_at_line(line: &str) -> Option<(&str, &str)> {
    let line = line.trim_start();
    if !line.starts_with('@') {
        return None;
    }
    let rest = &line[1..];
    let mut parts = rest.splitn(2, |c: char| c.is_ascii_whitespace());
    let spirit = parts.next()?;
    let msg = parts.next()?.trim();
    if spirit.is_empty() || msg.is_empty() {
        return None;
    }
    Some((spirit, msg))
}
fn print_line(
    output: &mut impl Write,
    color_choice: ColorChoice,
    text: &str,
) -> std::io::Result<()> {
    // D-16-2-B: every REPL line goes through the output seam — a fake-host
    // test reads the `&mut Vec<u8>`, and nothing lands on the process
    // stdout. A write error PROPAGATES (never a panic): the Err-arm exit
    // contract (a failed shell is exit non-zero) depends on it.
    match color_choice {
        ColorChoice::Never | ColorChoice::Auto => writeln!(output, "{text}"),
        ColorChoice::Always => writeln!(output, "\x1b[1m{text}\x1b[0m"),
    }
}

fn copy_dir_all(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst.join(entry.file_name()))?;
        } else {
            std::fs::copy(entry.path(), dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}
