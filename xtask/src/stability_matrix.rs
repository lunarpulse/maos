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
// Story 10.3 — reuse the consumer's fence contract (single source of truth) so
// the producer and `check_export_control` can never drift apart on what the
// §Export fence is.
use crate::check_export_control::{
    extract_export_fence, EXPORT_FENCE_END, EXPORT_FENCE_START, STUB_MARKER,
};

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

        // Story 10.3 AC-1 (NFR-Comp-1) — the §Export `<!-- PRESERVED:export -->`
        // fence must carry hand-authored classification (not the placeholder
        // stub). Only meaningful when in_sync: a drifted file reports its drift
        // first, since a stale fence's stub marker is downstream of regeneration.
        let export_issue = if in_sync {
            export_non_stub_issue(&committed)
        } else {
            None
        };

        if json {
            let payload = serde_json::json!({
                "passed": in_sync && deprecation_issues.is_empty() && export_issue.is_none(),
                "in_sync": in_sync,
                "deprecation_issues": deprecation_issues,
                "export_issue": export_issue,
            });
            println!("{payload}");
        } else if in_sync && deprecation_issues.is_empty() && export_issue.is_none() {
            eprintln!("stability-matrix: PASS — STABILITY.md in sync with workspace state; 0 undocumented deprecations; §Export present (non-stub)");
        } else {
            if !in_sync {
                eprintln!("stability-matrix: FAIL — STABILITY.md drift (committed differs from workspace-derived matrix)");
            }
            for issue in &deprecation_issues {
                eprintln!("stability-matrix: FAIL — {issue}");
            }
            if let Some(issue) = &export_issue {
                eprintln!("stability-matrix: FAIL — {issue}");
            }
        }

        if in_sync && deprecation_issues.is_empty() && export_issue.is_none() {
            Ok(())
        } else if !in_sync {
            Err(
                "STABILITY.md drift: regenerate with `cargo run -p xtask -- stability-matrix`"
                    .into(),
            )
        } else if let Some(issue) = export_issue {
            Err(issue)
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

    let export_content = extract_preserved_export(workspace_root)?;
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

MAOS v1.5 carries a **2-year LTS commitment** (NFR-Maint-6): the v1.0 line
receives **security-only patches for 2 years** from the LTS clock-start below.
The 2-year term takes effect at v1.5, extending the original v1.0 1-year window.

<!-- lts-clock-start: filled by `stability-matrix` IFF a `1.0.0`/`v1.0.0` git tag exists (Epic 10 cuts the tag); placeholder until then — do NOT fabricate a SHA/tag. -->
- **LTS clock-start:** {lts_clock}

## Substrate-Self Compliance Scope

<!-- NFR-Comp-3 — full scope language (Story 9.5a). -->

The MAOS substrate draws a **kernel-as-service trust boundary**: the kernel
provides mechanisms (Transparency Log, capability mediation, sandbox tiers
T0–T3, ComplianceClaim envelopes, GDPR Art. 17 erasure cascade); it does
**not** assert compliance of any deployment, operator, or Spirit running on it.

**Compliance-framework scope is the OPERATOR's responsibility.**

| Framework | Substrate provides | Operator owns |
|---|---|---|
| **SOC 2** | Append-only audit trail (TL); capability-token TTL + PID binding; sandbox-tier enforcement; sealed-export for external audit | Control mapping; access reviews; monitoring; incident response |
| **ISO 27001** | Asset inventory via Spirit manifest + TL; cryptographic key derivation (HKDF-SHA256, operator-local seed); region-pinning (NFR-Comp-4) | ISMS scope; risk assessment; Statement of Applicability; corrective actions |
| **FedRAMP** | Pluggable crypto-provider seam (FR48) — FIPS-validated module is operator/distributor choice; boundary definition via sandbox tiers; continuous-monitoring data (TL + posture-delta) | System Security Plan (SSP); POA&M; 3PAO engagement; ATO package |

The trust root is **operator-local** and **air-gap compatible**: the
Transparency Log signing key is derived from the operator's seed via
HKDF-SHA256 with no online CA, OCSP, or key-server dependency
([ADR-047](docs/adr/ADR-047-trust-anchor-framing-carry-forward.md),
NFR-Ops-12). The substrate's competitive framing is
**substrate-as-substrate** — infrastructure in the Linux/Postgres/Kubernetes
reference class — not a certifying authority (ADR-047 §2, considered and
rejected).

## Export

{EXPORT_FENCE_START}
{export_content}
{EXPORT_FENCE_END}
"#,
    ))
}

/// Story 10.3 AC-1 (NFR-Comp-1) — the §Export section is a STATIC PRESERVED
/// block, hand-authored (a legal ECCN assertion has a different change cadence
/// than the code-derived matrix). The generator emits the
/// `<!-- PRESERVED:export -->` / `<!-- END PRESERVED:export -->` fence and
/// preserves whatever committed content lives between the markers, never
/// overwriting it during regeneration. `--check` rejects a fence that still
/// holds the placeholder stub. The fence markers + stub phrase are SHARED with
/// the consumer `check_export_control` (single source of truth).

/// Placeholder content emitted inside the fence on first generation (before the
/// dev agent authors the real classification). Contains the stub phrase so
/// `--check` (via `check_export_control::STUB_MARKER`) rejects it as non-shipped.
fn default_export_stub() -> &'static str {
    "Export-control classification (ECCN determination — e.g. EAR99 vs 5D002 for \
     the cryptographic surface) is pending the formal determination in Story \
     10.3. Do not treat this section as legal export advice until that story lands."
}

/// Extract the hand-authored content between the §Export fence markers from the
/// committed STABILITY.md, reusing the consumer's `extract_export_fence`
/// (line-based) so producer and gate agree on what the fence is.
///
/// - File absent → `Ok(default_export_stub())` (first-generation bootstrap).
/// - File present + well-formed fence → `Ok(inner)` (CRLF normalized to LF so a
///   CRLF commit cannot cause permanent spurious drift).
/// - File present + missing/malformed fence → `Err` (REFUSE to render;
///   silently substituting the stub would destroy hand-authored classification
///   with no error — the data-loss fail-open this closes, P2).
fn extract_preserved_export(workspace_root: &Path) -> Result<String, String> {
    let path = workspace_root.join(STABILITY_MD);
    let contents = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Ok(default_export_stub().to_string()),
    };
    match extract_export_fence(&contents) {
        // P10: normalize CRLF → LF so a Windows/CRLF commit does not yield a
        // mixed-ending render the byte-for-byte `--check` can never converge.
        Some(inner) => Ok(inner.replace("\r\n", "\n")),
        None => Err(format!(
            "STABILITY.md exists but the {EXPORT_FENCE_START}/{EXPORT_FENCE_END} \
             fence is missing or malformed — refusing to regenerate (would destroy \
             hand-authored §Export content). Repair the fence markers and re-run."
        )),
    }
}

/// Returns `Some(message)` when the committed §Export fence is missing,
/// malformed, empty, contains a nested fence marker, or still holds the
/// placeholder stub; `None` when it carries shipped classification. Reuses the
/// consumer's `extract_export_fence` + `STUB_MARKER`. Self-consistent: a missing
/// fence returns `Some` (matches this docstring); in `run()` it is only reached
/// when `in_sync`, where the fence is guaranteed present, but the function is
/// correct in isolation.
fn export_non_stub_issue(stability_md: &str) -> Option<String> {
    let Some(inner) = extract_export_fence(stability_md) else {
        return Some("STABILITY.md §Export fence is missing or malformed".into());
    };
    let inner = inner.trim();
    if inner.is_empty() {
        return Some("STABILITY.md §Export fence is empty".into());
    }
    if inner.contains(EXPORT_FENCE_START) || inner.contains(EXPORT_FENCE_END) {
        return Some(
            "STABILITY.md §Export fence contains a nested fence marker — repair the content".into(),
        );
    }
    if inner.contains(STUB_MARKER) {
        return Some(
            "STABILITY.md §Export is still the placeholder stub — author the \
             classification inside the <!-- PRESERVED:export --> fence"
                .into(),
        );
    }
    None
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
        assert!(a.contains("2-year LTS"));
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

    /// Proven-red negative test for `--check` (Story 9.5a, AC-2, D8/Epic-8
    /// disabled-gate lesson): mutate the committed output → assert the check
    /// comparison detects drift. Proves the `--check` path can actually fail,
    /// not just vacuously pass.
    #[test]
    fn check_detects_drift_proven_red() {
        let root = std::env::current_dir()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        // Isolate the mutation: render from the TEMPDIR, not the real workspace,
        // so the only delta in the check is the mutation itself. Rendering from
        // `&root` diverges from `render(&tmp)` once a `1.0.0` git tag exists —
        // lts_clock_start resolves a SHA in the real repo but the placeholder in
        // a git-less tempdir — which would let the test pass without the mutation.
        let tmp_dir = tempfile::TempDir::new().expect("tempdir");
        let tmp = tmp_dir.path();

        // Copy the real workspace files the renderer reads.
        let copy = |rel: &str| {
            let src = root.join(rel);
            let dst = tmp.join(rel);
            std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
            std::fs::copy(&src, &dst).unwrap();
        };
        copy(SPIRIT_ABI_LIB);
        copy(CARGO_TOML);
        copy(BREAKING_MD);
        // The deprecation cross-check walks crates/**/*.rs — create an empty
        // crates dir so it doesn't error out (vacuously passes at zero files).
        std::fs::create_dir_all(tmp.join("crates")).unwrap();

        // Render from the tempdir so the check comparison is self-consistent
        // (the mutation is the ONLY difference between rendered and on-disk).
        let rendered = render(&tmp).expect("render succeeds");

        // Mutate the compliance-scope section — the exact text this story added.
        let mutated = rendered.replace("OPERATOR's responsibility", "SUBSTRATE's responsibility");
        assert_ne!(
            rendered, mutated,
            "mutation must produce different output (pre-condition)"
        );

        // Write the MUTATED STABILITY.md (not the correct one).
        let stability_path = tmp.join(STABILITY_MD);
        std::fs::write(&stability_path, &mutated).unwrap();

        // Run the check: it must FAIL (Err) because the mutated file ≠ rendered.
        let result = run(&tmp, true, false);
        assert!(
            result.is_err(),
            "stability-matrix --check must FAIL on a mutated STABILITY.md (proven-red)"
        );
        let err = result.unwrap_err();
        assert!(
            err.contains("drift"),
            "error message must mention drift, got: {err}"
        );

        // tmp_dir cleans itself up on Drop.
    }

    /// Verify the NFR-Comp-3 scope text is present in the rendered output
    /// (not the stub).
    #[test]
    fn render_contains_full_nfr_comp_3_scope() {
        let root = std::env::current_dir()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        let rendered = render(&root).expect("render");
        // Full scope language must be present (not the stub).
        assert!(
            rendered.contains("kernel-as-service trust boundary"),
            "NFR-Comp-3: must contain kernel-as-service trust boundary"
        );
        assert!(
            rendered.contains("OPERATOR's responsibility"),
            "NFR-Comp-3: must state operator responsibility"
        );
        assert!(
            rendered.contains("ADR-047"),
            "NFR-Comp-3: must reference ADR-047"
        );
        assert!(
            !rendered.contains("Full scope language lands in Story 9.5"),
            "NFR-Comp-3: stub text must be gone"
        );
    }

    #[test]
    fn export_non_stub_issue_flags_placeholder() {
        let stub =
            format!("<!-- PRESERVED:export -->\n{STUB_MARKER}\n<!-- END PRESERVED:export -->");
        let issue = export_non_stub_issue(&stub).expect("stub must be flagged");
        assert!(issue.contains("stub"), "issue must name the stub: {issue}");
    }

    #[test]
    fn export_non_stub_issue_passes_real_content() {
        let real = "<!-- PRESERVED:export -->\nEAR99 — ancillary cryptography on file.\n<!-- END PRESERVED:export -->";
        assert!(export_non_stub_issue(real).is_none());
    }

    #[test]
    fn export_non_stub_issue_flags_empty_fence() {
        let empty = "<!-- PRESERVED:export -->\n<!-- END PRESERVED:export -->";
        let issue = export_non_stub_issue(empty).expect("empty fence must be flagged");
        assert!(
            issue.contains("empty"),
            "issue must name emptiness: {issue}"
        );
    }

    /// Copy the real workspace inputs a self-consistent render reads, into a
    /// tempdir (mirrors `check_detects_drift_proven_red`'s isolation).
    fn isolated_workspace() -> (tempfile::TempDir, std::path::PathBuf) {
        let root = std::env::current_dir()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        let tmp_dir = tempfile::TempDir::new().expect("tempdir");
        let tmp = tmp_dir.path().to_path_buf();
        let copy = |rel: &str| {
            let src = root.join(rel);
            let dst = tmp.join(rel);
            std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
            std::fs::copy(&src, &dst).unwrap();
        };
        copy(SPIRIT_ABI_LIB);
        copy(CARGO_TOML);
        copy(BREAKING_MD);
        // deprecation_cross_check walks crates/**/*.rs — empty dir vacuously passes.
        std::fs::create_dir_all(tmp.join("crates")).unwrap();
        (tmp_dir, tmp)
    }

    #[test]
    fn render_preserves_committed_export_fence_content() {
        let (_guard, tmp) = isolated_workspace();
        let custom = "EAR99 ancillary-cryptography classification — see docs/compliance/eccn-classification.md";
        let stability = format!(
            "## Export\n\n<!-- PRESERVED:export -->\n{custom}\n<!-- END PRESERVED:export -->\n"
        );
        std::fs::write(tmp.join(STABILITY_MD), &stability).unwrap();
        let rendered = render(&tmp).expect("render");
        assert!(
            rendered.contains(custom),
            "hand-authored export content must be preserved on regeneration"
        );
        assert!(
            !rendered.contains(STUB_MARKER),
            "stub marker must not appear when real content is committed"
        );
        assert!(rendered.contains("<!-- PRESERVED:export -->"));
        assert!(rendered.contains("<!-- END PRESERVED:export -->"));
    }

    #[test]
    fn check_passes_with_nonstub_export_fence() {
        let (_guard, tmp) = isolated_workspace();
        // No STABILITY.md yet → render bootstraps the fence with the default stub.
        let rendered = render(&tmp).expect("render");
        // Swap the stub for shipped (non-stub) classification content.
        let non_stub = rendered.replace(
            default_export_stub(),
            "EAR99 — ancillary cryptography. Full classification in docs/compliance/eccn-classification.md",
        );
        std::fs::write(tmp.join(STABILITY_MD), &non_stub).unwrap();
        let result = run(&tmp, true, false);
        assert!(
            result.is_ok(),
            "non-stub export fence must pass --check: {:?}",
            result
        );
    }

    /// Proven-red (Epic 9 §A1): the §Export fence carrying the placeholder
    /// stub MUST fail `--check`, even when the rest of the file is in sync.
    #[test]
    fn check_rejects_stub_export_fence_proven_red() {
        let (_guard, tmp) = isolated_workspace();
        let rendered = render(&tmp).expect("render");
        // Write the rendered output verbatim — in_sync is TRUE, but the export
        // fence holds the default stub → the check must FAIL on the stub alone.
        std::fs::write(tmp.join(STABILITY_MD), &rendered).unwrap();
        let result = run(&tmp, true, false);
        assert!(result.is_err(), "stub §Export fence must fail --check");
        let err = result.unwrap_err();
        assert!(
            err.contains("stub") || err.contains("Export"),
            "error must name the stub §Export: {err}"
        );
    }
    #[test]
    fn extract_preserved_export_refuses_malformed_fence_proven_red() {
        // P2 data-loss guard: a partial fence (START present, END missing) in an
        // existing STABILITY.md must make render() REFUSE (Err), NOT silently
        // substitute the stub and overwrite the hand-authored content.
        let (_guard, tmp) = isolated_workspace();
        let partial =
            "## Export\n\n<!-- PRESERVED:export -->\nEAR99 hand-authored classification.\n";
        std::fs::write(tmp.join(STABILITY_MD), partial).unwrap();
        let result = render(&tmp);
        assert!(
            result.is_err(),
            "a partial fence must make render refuse (not destroy content)"
        );
        let err = result.unwrap_err();
        assert!(
            err.contains("refusing to regenerate"),
            "error must name the refusal: {err}"
        );
    }

    #[test]
    fn export_non_stub_issue_flags_missing_fence() {
        // P3: a missing fence returns Some (self-consistent with the docstring),
        // not a silent None pass.
        let no_fence = "## Export\n\nNo fence markers here at all.\n";
        let issue = export_non_stub_issue(no_fence).expect("missing fence must be flagged");
        assert!(issue.contains("missing") || issue.contains("malformed"));
    }

    #[test]
    fn export_non_stub_issue_flags_nested_marker() {
        // P3 inner-marker guard: preserved content quoting a fence marker is rejected.
        let nested = "<!-- PRESERVED:export -->\nNote: the fence is <!-- END PRESERVED:export --> inside.\n<!-- END PRESERVED:export -->";
        let issue = export_non_stub_issue(nested);
        assert!(issue.is_some(), "a nested fence marker must be flagged");
    }
}
