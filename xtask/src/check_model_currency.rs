#![forbid(unsafe_code)]

//! Story `17-5` — **the model-pin currency gate** (E14-A3 → E15-A2 → E16-A4).
//!
//! A manifest's `provider.complete = ["anthropic.claude-3-haiku-20240307"]` is a
//! pin on a vendor model. Vendors retire models on a schedule nobody here
//! controls; at `0d73cac3` five pins named models Anthropic had already retired
//! (`claude-3-haiku-20240307` retired 2026-04-20) and nothing in the tree noticed
//! — the lane slipped three times because no control read the pins at all.
//!
//! This gate reads them. It is **offline by construction**: it reads only
//! repo-local files — the surfaces below and its data, `xtask/model-currency.toml`
//! (per-provider `allowed` and `retired`, every retired entry carrying a
//! display-only `replacement`). It opens no socket, so it cannot learn of a new
//! vendor retirement — a human edits the data file when a vendor page changes,
//! and this gate then reds every pin that fell behind.
//!
//! **Scanned surfaces, exactly** (epic-17 §17-5 AC3):
//! * `spirits/*/manifest.toml`, `templates/*/manifest.toml`,
//!   `examples/*/manifest.toml` — the `capabilities.required.provider.complete`
//!   entries, whether written dotted or as a `[…provider]` table, and the
//!   Story 5.5b `[providers]` `primary`/`fallback` `model_id` pins;
//! * `xtask/provider-pricing.toml` — every `[[entries]]` `(provider, model)`;
//! * `crates/maos-bin/src/main.rs` — the model argument (the third) of every
//!   `AnthropicProvider`, `OpenAiProvider` and `OllamaProvider` constructor
//!   (`::new` / `::with_api_key`); all three must be found.
//!
//! Hermetic `#[cfg(test)]` fixtures and fuzz seeds in OTHER files are NOT scanned:
//! they pin a retired id as inert test data, and the debt is disclosed, not
//! laundered. `main.rs` is read whole — a host-provider constructor in its test
//! module would be judged too (none exists), the conservative direction.
//!
//! Every violation names `file:line`. **Fails closed:** an unreadable or
//! internally inconsistent data file, a surface that cannot be read, or zero
//! manifests is an `Err`; a manifest that does not parse, an empty price book,
//! or a provider constructor whose model argument is not a readable literal is
//! a finding at that file — never a pass, because `findings.is_empty()` is blind
//! to a gate that scans nothing. **Binding class: Blocking** — hermetic, no
//! `CURRENT_PHASE` coupling and no advisory tail; findings mean exit 1.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

pub const GATE: &str = "check-model-currency";
/// The gate's local data. Its path is quoted in every violation message.
pub const DATA: &str = "xtask/model-currency.toml";
const PRICING: &str = "xtask/provider-pricing.toml";
const MAIN_RS: &str = "crates/maos-bin/src/main.rs";
const MANIFEST_ROOTS: [&str; 3] = ["spirits", "templates", "examples"];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Data {
    providers: BTreeMap<String, Provider>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Provider {
    allowed: Vec<String>,
    #[serde(default)]
    retired: Vec<Retired>,
}

/// `replacement` is required (no serde default): a retired id with no guidance
/// is a data defect, not a gate pass.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Retired {
    id: String,
    replacement: String,
}

/// One model id found at one place in the tree.
struct Pin {
    file: String,
    line: usize,
    provider: String,
    model: String,
}

#[derive(Debug, Serialize)]
pub struct Finding {
    pub file: String,
    pub line: usize,
    pub detail: String,
}

pub struct Report {
    pub files: usize,
    pub pins: usize,
    pub findings: Vec<Finding>,
}

/// Parse and cross-check the data file. A `replacement` outside the provider's
/// own `allowed` list, or a retired id that is blank, listed twice (two
/// replacements) or also allowed, would make the display-only guidance lie, so
/// each is refused here.
pub fn parse_data(text: &str) -> Result<Data, String> {
    let data: Data = toml::from_str(text).map_err(|e| format!("{DATA}: {e}"))?;
    let refuse = |why: String| Err(format!("{DATA}: {why}"));
    if data.providers.is_empty() {
        return refuse("no providers — the gate would govern nothing".into());
    }
    for (name, p) in &data.providers {
        if p.allowed.is_empty() || p.allowed.iter().any(|m| m.trim().is_empty()) {
            return refuse(format!("provider `{name}` needs non-empty `allowed` ids"));
        }
        for r in &p.retired {
            if !p.allowed.contains(&r.replacement) {
                return refuse(format!(
                    "`{name}` retired `{}` must name a `replacement` that is in `{name}`'s `allowed`",
                    r.id
                ));
            }
            let twice = p.retired.iter().filter(|o| o.id == r.id).count() > 1;
            if p.allowed.contains(&r.id) || r.id.trim().is_empty() || twice {
                return refuse(format!(
                    "`{name}.{}` is blank, retired twice, or both allowed and retired",
                    r.id
                ));
            }
        }
    }
    Ok(data)
}

/// Why `pin` is not current, or `None` when it is.
fn judge(data: &Data, pin: &Pin) -> Option<String> {
    let id = format!("{}.{}", pin.provider, pin.model);
    let Some(p) = data.providers.get(&pin.provider) else {
        return Some(format!(
            "UNKNOWN provider for `{id}` — add the provider to {DATA} or fix the pin"
        ));
    };
    if p.allowed.contains(&pin.model) {
        return None;
    }
    Some(match p.retired.iter().find(|r| r.id == pin.model) {
        Some(r) => format!(
            "RETIRED model id `{id}` — replacement (display-only guidance): `{}`",
            r.replacement
        ),
        None => format!(
            "UNKNOWN model id `{id}` — not in {DATA} `allowed` (typo, or a policy edit is owed)"
        ),
    })
}

/// 1-based line of the next code occurrence of `needle` as a complete quoted TOML
/// string (`"x"` or `'x'`); `seen` counts the occurrences this file already
/// placed. Comments — whole-line or trailing — never match, and every occurrence
/// counts, so an id repeated on one line or on two lines names its own place.
fn locate(text: &str, needle: &str, seen: &mut BTreeMap<String, usize>) -> usize {
    let nth = seen.entry(needle.into()).or_default();
    *nth += 1;
    let quoted = [format!("\"{needle}\""), format!("'{needle}'")];
    let mut count = 0;
    text.lines()
        .position(|l| {
            let code = l.split('#').next().unwrap_or_default();
            count += code.matches(&quoted[0]).count() + code.matches(&quoted[1]).count();
            count >= *nth
        })
        .map_or(1, |i| i + 1)
}

fn manifest_pins(file: &str, text: &str) -> Result<Vec<Pin>, String> {
    let table: toml::Table = text.parse().map_err(|e| format!("unparseable TOML: {e}"))?;
    let (mut pins, mut seen) = (Vec::new(), BTreeMap::new());
    let mut pin = |provider: &str, model: &str, needle: &str| Pin {
        file: file.into(),
        line: locate(text, needle, &mut seen),
        provider: provider.into(),
        model: model.into(),
    };
    let section = table.get("capabilities").and_then(|c| c.get("required"));
    if let Some(section) = section.and_then(|r| r.get("provider")) {
        let complete = section
            .get("complete")
            .and_then(toml::Value::as_array)
            .ok_or("`capabilities.required.provider` has no `complete` array — a pin written any other way would escape the scan")?;
        for id in complete {
            let id = id
                .as_str()
                .ok_or("`provider.complete` holds a non-string entry")?;
            // A dotless entry grants a whole provider and pins no model.
            if let Some((provider, model)) = id.split_once('.') {
                pins.push(pin(provider, model, id));
            }
        }
    }
    // Story 5.5b `[providers]`: `primary` and each `fallback` may pin a `model_id`.
    let providers = table.get("providers");
    let fallback = providers.and_then(|p| p.get("fallback")?.as_array());
    let primary = providers.and_then(|p| p.get("primary"));
    for config in primary.into_iter().chain(fallback.into_iter().flatten()) {
        let field = |k: &str| config.get(k).and_then(toml::Value::as_str);
        if let (Some(provider), Some(model)) = (field("id"), field("model_id")) {
            pins.push(pin(provider, model, model));
        }
    }
    Ok(pins)
}

fn pricing_pins(file: &str, text: &str) -> Result<Vec<Pin>, String> {
    let table: toml::Table = text.parse().map_err(|e| format!("unparseable TOML: {e}"))?;
    let entries = table
        .get("entries")
        .and_then(toml::Value::as_array)
        .filter(|rows| !rows.is_empty())
        .ok_or("no `[[entries]]` — the price book would govern nothing")?;
    let mut seen = BTreeMap::new();
    entries
        .iter()
        .map(|entry| {
            let field = |k: &str| entry.get(k).and_then(toml::Value::as_str);
            let (Some(provider), Some(model)) = (field("provider"), field("model")) else {
                return Err("an `[[entries]]` row lacks a string `provider` or `model`".to_string());
            };
            Ok(Pin {
                file: file.into(),
                line: locate(text, model, &mut seen),
                provider: provider.into(),
                model: model.into(),
            })
        })
        .collect()
}

/// The model of each `XProvider::new(…)` / `::with_api_key(…)` call: its THIRD
/// argument (`transport, endpoint_url, model_id, …` in `maos-providers`), read
/// from the call's own argument list — nesting and string literals respected,
/// `//` comments dropped, so a commented argument is no pin — and required to be
/// a `"…".into()`-style literal. A call after `//` on its line opens nothing.
/// All three host providers must be found — one silently dropping out would
/// leave its pin unscanned.
fn main_pins(file: &str, text: &str) -> Result<Vec<Pin>, String> {
    // From line start, with no `//` before the call: a commented call opens nothing.
    let ctor = regex::Regex::new(
        r"(?m)^(?:[^/\n]|/[^/\n])*?\b(Anthropic|OpenAi|Ollama)Provider::(?:new|with_api_key)\(",
    )
    .expect("static");
    let literal =
        regex::Regex::new(r#"^\s*"([^"]+)"\.(?:into|to_string|to_owned)\(\)\s*$"#).expect("static");
    let mut pins: Vec<Pin> = Vec::new();
    for c in ctor.captures_iter(text) {
        let open = c.get(0).map_or(0, |m| m.end());
        let line = text[..open].matches('\n').count() + 1;
        // The call's own argument list, comments dropped, top-level commas → `\0`.
        let (mut list, mut depth, mut quoted, mut comment) = (String::new(), 1, false, false);
        for ch in text[open..].chars() {
            match ch {
                '\n' => comment = false,
                _ if comment => continue,
                '"' => quoted = !quoted,
                '/' if !quoted && list.ends_with('/') => {
                    comment = list.pop() == Some('/');
                    continue;
                }
                '(' | '[' | '{' if !quoted => depth += 1,
                ')' | ']' | '}' if !quoted => depth -= 1,
                _ => {}
            }
            if depth == 0 {
                break;
            }
            let separator = ch == ',' && depth == 1 && !quoted;
            list.push(if separator { '\0' } else { ch });
        }
        let Some(m) = list.split('\0').nth(2).and_then(|a| literal.captures(a)) else {
            return Err(format!(
                "line {line}: `{}Provider` constructor has no readable model literal as its third argument — the pin could rot unscanned",
                &c[1]
            ));
        };
        let skip: usize = list.split('\0').take(2).map(|a| a.len() + 1).sum();
        let at = skip + m.get(1).map_or(0, |g| g.start());
        pins.push(Pin {
            file: file.into(),
            line: line + list[..at].matches('\n').count(),
            provider: c[1].to_ascii_lowercase(),
            model: m[1].to_string(),
        });
    }
    for host in ["anthropic", "openai", "ollama"] {
        if !pins.iter().any(|p| p.provider == host) {
            return Err(format!(
                "no `{host}` provider constructor found — its model pin would go unscanned"
            ));
        }
    }
    Ok(pins)
}

fn read(root: &Path, rel: &str) -> Result<String, String> {
    std::fs::read_to_string(root.join(rel)).map_err(|e| format!("{GATE}: cannot read {rel}: {e}"))
}

/// Scan `root` (a workspace root) and judge every pin found.
pub fn audit(root: &Path) -> Result<Report, String> {
    let data = parse_data(&read(root, DATA)?)?;
    let (mut pins, mut findings, mut files) = (Vec::new(), Vec::new(), 0);
    let mut absorb = |file: &str, scanned: Result<Vec<Pin>, String>| match scanned {
        Ok(found) => pins.extend(found),
        Err(detail) => findings.push(Finding {
            file: file.into(),
            line: 1,
            detail,
        }),
    };
    for dir in MANIFEST_ROOTS {
        let unlisted = |e: std::io::Error| format!("{GATE}: cannot list {dir}/: {e}");
        let mut names = Vec::new();
        for entry in std::fs::read_dir(root.join(dir)).map_err(unlisted)? {
            let entry = entry.map_err(unlisted)?;
            if entry
                .path()
                .join("manifest.toml")
                .symlink_metadata()
                .is_ok()
            {
                names.push(entry.file_name().to_string_lossy().into_owned());
            }
        }
        names.sort();
        for name in names {
            let file = format!("{dir}/{name}/manifest.toml");
            absorb(&file, manifest_pins(&file, &read(root, &file)?));
            files += 1;
        }
    }
    if files == 0 {
        return Err(format!(
            "{GATE}: no manifest found under spirits/, templates/ or examples/"
        ));
    }
    absorb(PRICING, pricing_pins(PRICING, &read(root, PRICING)?));
    absorb(MAIN_RS, main_pins(MAIN_RS, &read(root, MAIN_RS)?));
    findings.extend(pins.iter().filter_map(|pin| {
        judge(&data, pin).map(|detail| Finding {
            file: pin.file.clone(),
            line: pin.line,
            detail,
        })
    }));
    Ok(Report {
        files: files + 2,
        pins: pins.len(),
        findings,
    })
}

pub fn run(json: bool) -> Result<(), String> {
    let root = std::env::current_dir().map_err(|e| format!("{GATE}: no cwd: {e}"))?;
    let report = audit(&root)?;
    let (files, pins, findings) = (report.files, report.pins, report.findings);
    let passed = findings.is_empty();
    if json {
        println!(
            "{}",
            serde_json::json!({
                "gate": GATE,
                "passed": passed,
                "files_scanned": files,
                "pins_checked": pins,
                "findings": findings,
            })
        );
    }
    if passed {
        if !json {
            println!("{GATE}: PASS ({pins} pins across {files} files)");
        }
        return Ok(());
    }
    let detail: String = findings
        .iter()
        .map(|f| format!("- {}:{} — {}\n", f.file, f.line, f.detail))
        .collect();
    let count = findings.len();
    let msg = format!("{GATE}: FAIL — {count} finding(s) over {pins} pin(s):\n{detail}");
    crate::gate_common::emit_command(json, "error", &msg);
    if !json {
        eprintln!("{msg}");
    }
    Err(msg)
}
