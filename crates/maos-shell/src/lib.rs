#![forbid(unsafe_code)]

//! `maos-shell` — J0 evaluator surface.
//!
//! Provides:
//! - `init`    — scaffold `~/.maos/` (config, slots, skills, logs)
//! - `shell`   — kernel-rendered REPL (`@<spirit> <msg>`)
//! - `audit`   — thin read-side alias over `maos_audit::query`

use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

use maos_cli::accessibility::ColorChoice;
use maos_domain::ports::inference::InferencePort;
use maos_kernel_core::capability::CapabilityRegistryAdapter;
use maos_kernel_core::capability::CapabilityRegistryPort;

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

/// Run `maos init` — scaffold `~/.maos/` if absent.
///
/// Idempotent: re-running prints "already initialized" and exits 0.
pub fn run_init(color_choice: ColorChoice) -> Result<(), Box<dyn std::error::Error>> {
    let home = maos_home();
    let config_path = home.join("config.toml");

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
                print_line(
                    color_choice,
                    &format!(
                        "maos: already initialized — {} exists",
                        config_path.display()
                    ),
                );
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

    // Create directory tree.
    std::fs::create_dir_all(&home)?;
    std::fs::create_dir_all(home.join("skills"))?;
    std::fs::create_dir_all(home.join("audit"))?;
    std::fs::create_dir_all(home.join("journal"))?;
    std::fs::create_dir_all(home.join("logs"))?;

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
        color_choice,
        &format!("maos: initialized {}", home.display()),
    );
    print_line(
        color_choice,
        &format!(
            "maos: config written to {}  (6 default slots + retention=persist)",
            config_path.display()
        ),
    );
    let audit_path = maos_audit::default_transparency_log_path();
    print_line(
        color_choice,
        &format!("maos: Transparency Log will be at {}", audit_path.display()),
    );
    print_line(
        color_choice,
        &format!("maos: to remove all data, run:  rm -rf {}", home.display()),
    );
    Ok(())
}

/// Run `maos audit query` — thin alias over `maos_audit::query`.
pub fn run_audit_query(
    spirit: Option<&str>,
    format: &str,
    _color_choice: ColorChoice, // Accepts caller's NO_COLOR intent; library functions emit no ANSI regardless.
) -> Result<(), Box<dyn std::error::Error>> {
    let db_path = maos_audit::default_transparency_log_path();
    if !db_path.exists() {
        eprintln!("maos: no Transparency Log found at {}", db_path.display());
        return Err("audit log not found".into());
    }

    let mut filter = maos_audit::AuditFilter::default();
    if let Some(s) = spirit {
        let pid = resolve_spirit_pid(s).ok_or_else(|| format!("unknown spirit '{s}'"))?;
        filter.spirit_pid = Some(pid);
    }

    let entries = maos_audit::query(&db_path, filter)?;

    let stdout = std::io::stdout();
    let mut lock = stdout.lock();

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

/// Run the kernel-rendered shell REPL.
///
/// `inference`  — the composed Inference Port (live or deterministic).
/// `capability` — the Capability Registry (to issue tokens).
///
/// Reads lines from stdin, parses `@<spirit> <msg>`, dispatches in-proc.
pub fn run_shell(
    inference: Arc<dyn InferencePort + Send + Sync>,
    capability: Arc<CapabilityRegistryAdapter>,
    color_choice: ColorChoice,
    default_provider: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use maos_domain::invariants::i1::{CapabilityToken, IntentClass, Scope};

    print_line(
        color_choice,
        "maos shell — type @hello-spirit <msg>  (Ctrl-D to exit)",
    );

    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    for line_result in stdin.lines() {
        let line = match line_result {
            Ok(l) => l,
            Err(_) => break,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Parse `@<spirit> <msg>`.
        let (spirit_name, msg) = match parse_at_line(trimmed) {
            Some(p) => p,
            None => {
                print_line(color_choice, "maos: expected @<spirit> <message>");
                continue;
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
            writeln!(stdout, "{message}")?;
            stdout.flush()?;
            continue;
        }
        if spirit_name != "hello-spirit" {
            print_line(
                color_choice,
                &format!(
                    "maos: unknown spirit '{spirit_name}' — known: hello-spirit, butler (pick only)",
                ),
            );
            continue;
        }
        let token: CapabilityToken = capability
            .issue_with_mediation(
                0, // hello-spirit PID
                Scope::ProviderInfer {
                    provider: default_provider.into(),
                },
                60,        // ttl_secs
                [0u8; 32], // posture_hash (deterministic fallback)
                IntentClass::Standard,
            )
            .map_err(|e| format!("capability issue failed: {e}"))?;

        // Dispatch to hello-spirit.
        let token_for_audit = token.clone();
        let response = if msg.to_lowercase().starts_with("say hi") {
            maos_spirit_hello::say_hi(&*inference, token)
        } else {
            maos_spirit_hello::dispatch_directive(&*inference, token, msg)
        };

        match response {
            Ok(resp) => {
                writeln!(
                    stdout,
                    "{introduction}\n\nPosture: {posture}\nCapability scope: {scope:?}\nHalt tags: {tags:?}\nTransparency Log: {log}",
                    introduction = resp.introduction,
                    posture = resp.posture,
                    scope = resp.capability_scope,
                    tags = resp.halt_tags,
                    log = resp.transparency_log,
                )?;
                let payload = serde_json::json!({
                    "user": msg,
                    "response": resp.introduction,
                })
                .to_string();
                let _ = capability.record_invocation(
                    &token_for_audit,
                    "shell.turn".into(),
                    payload.as_bytes(),
                );
            }
            Err(maos_spirit_hello::HelloError::Ambiguous { tag, prompt }) => {
                writeln!(stdout, "[HALT {tag}] {prompt}")?;
                let payload = serde_json::json!({
                    "user": msg,
                    "halt": tag,
                })
                .to_string();
                let _ = capability.record_invocation(
                    &token_for_audit,
                    "shell.halt".into(),
                    payload.as_bytes(),
                );
            }
            Err(e) => {
                writeln!(stdout, "maos: error: {e}")?;
            }
        }
        stdout.flush()?;
    }

    print_line(color_choice, "maos: shell exiting");
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal helpers
// ─────────────────────────────────────────────────────────────────────────────

fn maos_home() -> PathBuf {
    std::env::var("MAOS_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .filter(|h| !h.is_empty())
                .map(|h| PathBuf::from(h).join(".maos"))
        })
        .expect("maos: neither MAOS_HOME nor HOME is set — set one to proceed")
}

fn default_config_toml() -> String {
    r#"# MAOS user configuration (generated by `maos init`)

[slots]
worker = ["w1", "w2", "w3", "w4", "w5"]
orchestrator = ["orch1"]

[retention]
default = "persist"
# Set to "ephemeral" to remove ~/.maos/ on `cargo uninstall maos` (manual).

[paths]
home = "~/.maos"
audit = "~/.maos/audit"
journal = "~/.maos/journal"
logs = "~/.maos/logs"
"#
    .to_string()
}

fn resolve_spirit_pid(name: &str) -> Option<u32> {
    match name {
        "hello-spirit" => Some(0),
        _ => None,
    }
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

fn print_line(color_choice: ColorChoice, text: &str) {
    match color_choice {
        ColorChoice::Never | ColorChoice::Auto => println!("{text}"),
        ColorChoice::Always => println!("\x1b[1m{text}\x1b[0m"),
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
