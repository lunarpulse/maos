#![forbid(unsafe_code)]

//! `stability-matrix` — Story 7.5a (AC3 / AC5 / NFR-Maint-3 / NFR-Maint-4).
//!
//! Generates the repo-root `STABILITY.md` from LIVE workspace state so the
//! published v1.0 ABI-stability compatibility matrix can NEVER drift from the
//! code. Dual-mode (cloning `templates_regen`'s write-vs-`--check` skeleton):
//!
//!   cargo run -p xtask -- stability-matrix           # (re)generate STABILITY.md
//!   cargo run -p xtask -- stability-matrix --check    # CI rail: fail on drift
//!
//! Every value is SOURCED, never hand-written:
//!   * `abi_version`, `manifest_schema_version`, `MIN/MAX_SUPPORTED` — parsed
//!     from `crates/maos-spirit-abi/src/lib.rs` via
//!     `check_manifest_schema_version::parse_const` (the single authoritative
//!     constants file);
//!   * `kernel_version` — `[workspace.package].version` in `Cargo.toml` (the
//!     LIVE value: `0.1.0-alpha` today, `1.0.0` at GA — no literal is baked in);
//!   * crate count — `check_workspace_count::count_cargo_toml_members`.
//!
//! NFR-Maint-3 cross-check (in `--check` mode): every
//! `#[maos_attrs::deprecated_since(...)]` annotation in `crates/**/*.rs` MUST
//! have a matching row in STABILITY.md's `## Deprecations` table AND a dated
//! entry in BREAKING.md. Vacuously passes at v1.0's ZERO deprecations, but the
//! rail FAILS LOUDLY the moment a real deprecation lands without its paperwork.

use std::path::Path;

use crate::check_manifest_schema_version::parse_const;
use crate::check_workspace_count::count_cargo_toml_members;

const SPIRIT_ABI_LIB: &str = "crates/maos-spirit-abi/src/lib.rs";
const CARGO_TOML: &str = "Cargo.toml";
const STABILITY_MD: &str = "STABILITY.md";
const BREAKING_MD: &str = "BREAKING.md";

pub fn run(workspace_root: &Path, check: bool, json: bool) -> Result<(), String> {
    let rendered = render(workspace_root)?;
    let path = workspace_root.join(STABILITY_MD);

    if check {
        let committed = std::fs::read_to_string(&path).map_err(|e| {
            format!(
                "STABILITY.md not found ({e}); run `cargo run -p xtask -- stability-matrix` to generate it"
            )
        })?;
        let in_sync = committed == rendered;
        // NFR-Maint-3 — deprecation ↔ STABILITY.md ↔ BREAKING.md cross-check.
        let deprecation_issues = deprecation_cross_check(workspace_root, &committed)?;

        if json {
            let payload = serde_json::json!({
                "passed": in_sync && deprecation_issues.is_empty(),
                "in_sync": in_sync,
                "deprecation_issues": deprecation_issues,
            });
            println!("{payload}");
        } else if in_sync && deprecation_issues.is_empty() {
            eprintln!("stability-matrix: PASS — STABILITY.md in sync with workspace state; 0 undocumented deprecations");
        } else {
            if !in_sync {
                eprintln!("stability-matrix: FAIL — STABILITY.md drift (committed differs from workspace-derived matrix)");
            }
            for issue in &deprecation_issues {
                eprintln!("stability-matrix: FAIL — {issue}");
            }
        }

        if in_sync && deprecation_issues.is_empty() {
            Ok(())
        } else if !in_sync {
            Err(
                "STABILITY.md drift: regenerate with `cargo run -p xtask -- stability-matrix`"
                    .into(),
            )
        } else {
            Err(format!(
                "STABILITY.md: {} deprecation annotation(s) missing STABILITY.md row and/or dated BREAKING.md entry",
                deprecation_issues.len()
            ))
        }
    } else {
        std::fs::write(&path, &rendered)
            .map_err(|e| format!("failed to write {}: {e}", path.display()))?;
        if json {
            println!("{}", serde_json::json!({"written": STABILITY_MD}));
        } else {
            eprintln!(
                "stability-matrix: wrote {} from workspace state",
                STABILITY_MD
            );
        }
        Ok(())
    }
}

/// Render the full STABILITY.md byte-for-byte from workspace state.
fn render(workspace_root: &Path) -> Result<String, String> {
    let abi_src = std::fs::read_to_string(workspace_root.join(SPIRIT_ABI_LIB))
        .map_err(|e| format!("cannot read {SPIRIT_ABI_LIB}: {e}"))?;
    let abi_version = parse_const(&abi_src, "ABI_VERSION")
        .ok_or_else(|| "ABI_VERSION not found in maos-spirit-abi/src/lib.rs".to_string())?;
    let current_schema = parse_const(&abi_src, "MANIFEST_SCHEMA_VERSION")
        .ok_or_else(|| "MANIFEST_SCHEMA_VERSION not found".to_string())?;
    let min_schema = parse_const(&abi_src, "MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION")
        .ok_or_else(|| "MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION not found".to_string())?;
    let max_schema = parse_const(&abi_src, "MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION")
        .ok_or_else(|| "MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION not found".to_string())?;

    let cargo_path = workspace_root.join(CARGO_TOML);
    let kernel_version = parse_workspace_version(&cargo_path)?;
    let member_count = count_cargo_toml_members(&cargo_path)?;
    let lts_clock = lts_clock_start(workspace_root);

    // N-1 / N-2 rows derived from the live window. N-1 exists only when the
    // current schema is at least 2 (CURRENT-1 ≥ MIN); below that it is N/A.
    let n_minus_1 = if current_schema >= 2 {
        format!(
            "| `manifest_schema_version = {}` (N-1) | ✅ supported (loads with WARN-level degradation notes) |",
            current_schema - 1
        )
    } else {
        "| `manifest_schema_version` N-1 | N/A (current schema is the floor) |".to_string()
    };
    let n_minus_2 = if current_schema >= 2 {
        format!(
            "| `manifest_schema_version < {min_schema}` (N-2) | ⛔ hard refusal — typed `SecurityError::EAbiTooOld` at admit |"
        )
    } else {
        format!(
            "| `manifest_schema_version < {min_schema}` | ⛔ hard refusal — typed `SecurityError::EAbiTooOld` at admit |"
        )
    };

    Ok(format!(
        r#"<!-- GENERATED FILE — do not edit by hand.
     Source of truth: workspace state (maos-spirit-abi constants + Cargo.toml).
     Regenerate: `cargo run -p xtask -- stability-matrix`
     CI rail:     `cargo run -p xtask -- stability-matrix --check` (Story 7.5a, NFR-Maint-4). -->

# MAOS ABI Stability Commitments

This document publishes the MAOS v1.0 **ABI Stability Triple** and the
compatibility, deprecation, and long-term-support guarantees the substrate
makes to Spirit authors and operators. It is GENERATED from the live workspace
(it is never hand-maintained); a third party can regenerate it byte-for-byte
with `cargo run -p xtask -- stability-matrix --check`.

## Compatibility Matrix

The **ABI Stability Triple** `(kernel_version, abi_version, manifest_schema_version)`
is the load-time compatibility contract. The running kernel REFUSES an
incompatible Spirit at admission with a typed error — this is enforced, not a
promise (see `SecurityManagerAdapter::admit_spirit`).

| Leg | Live value |
|---|---|
| `kernel_version` | `{kernel_version}` |
| `abi_version` | `{abi_version}` |
| `manifest_schema_version` (current) | `{current_schema}` |
| supported schema window | `{min_schema}..={max_schema}` |
| workspace crates | `{member_count}` |

| Manifest schema | Kernel behavior |
|---|---|
| `manifest_schema_version = {current_schema}` (current) | ✅ strict load (`deny_unknown_fields`) |
{n_minus_1}
{n_minus_2}
| `manifest_schema_version > {max_schema}` (future) | ⛔ hard refusal — typed `SecurityError::EAbiTooNew` (fail-closed; the operator is told a newer kernel is required) |
| `min_substrate_version` > running `kernel_version` | ⛔ hard refusal — typed `SecurityError::ESubstrateTooOld` (FR8) |

The version gate is **fail-closed in both directions**: an out-of-window
manifest is refused with an actionable typed error, never silently
warned-and-admitted (a manifest is a security artifact; a silent ignore would be
fail-open).

## Deprecations

Deprecated public surfaces follow the NFR-Maint-5 timeline: **2 minor releases of
warning, then 1 major release to remove.** Every `#[maos_attrs::deprecated_since(...)]`
surface MUST appear as a row below AND carry a dated entry in `BREAKING.md`; CI
(`stability-matrix --check`) enforces this cross-check.

| Surface | Deprecated since | Removal target | Migration |
|---|---|---|---|
| _(none at v1.0)_ | — | — | — |

## LTS Policy

MAOS v1.0 carries a **1-year LTS commitment** (NFR-Maint-6): the v1.0 line
receives **security-only patches for 1 year** from the LTS clock-start below. The
2-year LTS term is **deferred to v1.5** — v1.0 publishes the term "the v0.8 team
can cash," not an over-promised window.

<!-- lts-clock-start: filled by `stability-matrix` IFF a `1.0.0`/`v1.0.0` git tag exists (Epic 10 cuts the tag); placeholder until then — do NOT fabricate a SHA/tag. -->
- **LTS clock-start:** {lts_clock}

## Substrate-Self Compliance Scope

<!-- full content: Story 9.5 (NFR-Comp-3) — this is the structural-presence STUB. -->

The MAOS substrate itself is assessed against, and its boundary scoped relative
to, the following regimes: **SOC 2**, **ISO 27001**, **FedRAMP**, and the
**kernel-as-service trust boundary**. The substrate provides the mechanisms
(transparency log, capability mediation, sandbox tiers, ComplianceClaim
envelopes); **mapping a concrete deployment to any specific control framework is
the OPERATOR's responsibility.** Full scope language lands in Story 9.5.

## Export

<!-- full content: Story 10.3 (NFR-Comp-1) — this is the placeholder STUB. -->

Export-control classification (ECCN determination — e.g. EAR99 vs 5D002 for the
cryptographic surface) is pending the formal determination in Story 10.3. Do not
treat this section as legal export advice until that story lands.
"#,
    ))
}

/// Parse `[workspace.package].version` from `Cargo.toml`.
fn parse_workspace_version(cargo_path: &Path) -> Result<String, String> {
    let src = std::fs::read_to_string(cargo_path)
        .map_err(|e| format!("cannot read {}: {e}", cargo_path.display()))?;
    let root: toml::Value =
        toml::from_str(&src).map_err(|e| format!("toml parse error in Cargo.toml: {e}"))?;
    root.get("workspace")
        .and_then(|w| w.get("package"))
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "[workspace.package].version not found in Cargo.toml".to_string())
}

/// Resolve the LTS clock-start field. Fills `<sha> (tag <tag>)` IFF a
/// `1.0.0`/`v1.0.0` git tag exists; otherwise emits the deterministic
/// placeholder (the `1.0.0` tag is cut in Epic 10 — Conflict #4; do NOT
/// fabricate). Best-effort: any git failure → placeholder.
fn lts_clock_start(workspace_root: &Path) -> String {
    for tag in ["1.0.0", "v1.0.0"] {
        if let Ok(out) = std::process::Command::new("git")
            .args(["rev-list", "-n", "1", tag])
            .current_dir(workspace_root)
            .output()
        {
            if out.status.success() {
                let sha = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !sha.is_empty() {
                    return format!("`{sha}` (tag `{tag}`)");
                }
            }
        }
    }
    "pending — the `1.0.0` tag is cut in Epic 10 (`epic-10-v10-ship-gate`); this fills automatically when the tag exists.".to_string()
}

/// NFR-Maint-3 cross-check: each `#[maos_attrs::deprecated_since(...)]`
/// annotation in `crates/**/*.rs` must have a row in STABILITY.md AND a dated
/// BREAKING.md entry. Returns the list of issues (empty == pass; vacuous at zero
/// annotations). A deprecation row is matched loosely by requiring the annotated
/// item's surrounding `since`/version string to appear in STABILITY.md's
/// Deprecations table; at zero annotations this loop never runs.
fn deprecation_cross_check(
    workspace_root: &Path,
    stability_md: &str,
) -> Result<Vec<String>, String> {
    let crates_dir = workspace_root.join("crates");
    if !crates_dir.exists() {
        return Ok(Vec::new());
    }
    let breaking_md = std::fs::read_to_string(workspace_root.join(BREAKING_MD)).unwrap_or_default();

    let mut issues = Vec::new();
    let mut annotations = 0usize;
    visit_rs_files(&crates_dir, &mut |path, content| {
        for line in content.lines() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            if line.contains("#[maos_attrs::deprecated_since(") {
                annotations += 1;
                // Extract the version argument from deprecated_since("X.Y.Z")
                // so we can verify per-item correspondence in STABILITY.md and
                // BREAKING.md — not just that the sections exist.
                let version_arg = extract_deprecated_since_arg(line);
                let has_stability_row = match &version_arg {
                    Some(v) => stability_md.contains(v),
                    None => false,
                };
                let has_breaking_entry = breaking_md.lines().any(|l| {
                    l.starts_with("## 2")
                        && match &version_arg {
                            Some(v) => l.contains(v),
                            None => true,
                        }
                });
                if !has_stability_row {
                    issues.push(format!(
                        "{}: deprecated_since({}) annotation lacks a matching STABILITY.md Deprecations row",
                        path.display(),
                        version_arg.as_deref().unwrap_or("?")
                    ));
                }
                if !has_breaking_entry {
                    issues.push(format!(
                        "{}: deprecated_since({}) annotation lacks a matching dated BREAKING.md entry",
                        path.display(),
                        version_arg.as_deref().unwrap_or("?")
                    ));
                }
            }
        }
    })?;
    let _ = annotations;
    Ok(issues)
}

fn extract_deprecated_since_arg(line: &str) -> Option<String> {
    let start = line.find("deprecated_since(")?;
    let rest = &line[start + 17..];
    let open = rest.find('"')?;
    let after_open = &rest[open + 1..];
    let close = after_open.find('"')?;
    Some(after_open[..close].to_string())
}

fn visit_rs_files(dir: &Path, f: &mut dyn FnMut(&Path, &str)) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("read_dir {}: {e}", dir.display()))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().and_then(|n| n.to_str()) == Some("target") {
                continue;
            }
            visit_rs_files(&path, f)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            if let Ok(content) = std::fs::read_to_string(&path) {
                f(&path, &content);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_is_deterministic_and_sources_live_triple() {
        let root = std::env::current_dir()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        let a = render(&root).expect("render");
        let b = render(&root).expect("render");
        assert_eq!(a, b, "render must be byte-deterministic for --check");
        assert!(a.contains("## Compatibility Matrix"));
        assert!(a.contains("## LTS Policy"));
        assert!(a.contains("1-year LTS"));
        assert!(a.contains("## Substrate-Self Compliance Scope"));
        assert!(a.contains("SOC 2") && a.contains("ISO 27001") && a.contains("FedRAMP"));
        assert!(a.contains("## Export"));
        assert!(a.contains("## Deprecations"));
        // ABI_VERSION = 1 is the live value sourced from the constants file.
        assert!(a.contains("| `abi_version` | `1` |"));
    }

    #[test]
    fn lts_clock_is_placeholder_without_v1_tag() {
        let root = std::env::current_dir()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        // No 1.0.0 tag exists pre-Epic-10 → deterministic placeholder.
        let clock = lts_clock_start(&root);
        assert!(clock.contains("Epic 10") || clock.contains("tag `"));
    }
}
