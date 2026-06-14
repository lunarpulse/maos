#![forbid(unsafe_code)]

//! Gate — `error-catalog-check`.
//!
//! FR63 typed error catalog: CI-enforced bijection between the
//! `xtask/error-catalog.toml` registry and the E\*-prefixed enum variants
//! (and E\*-prefixed error enums) discovered by AST scan.
//!
//! ## Discovery rules
//!
//! 1. **E\*-prefixed variant** — an enum variant whose name starts with `E[A-Z]`.
//!    Registered as `ParentEnum::VariantName`.
//! 2. **E\*-prefixed enum** — an enum whose name starts with `E[A-Z]` AND derives
//!    `thiserror::Error`. Every variant of such an enum is part of the E\* set,
//!    registered as `EnumName::VariantName`.
//!
//! ## Bijection check
//!
//! - **Un-catalogued**: E\* item found in source but absent from registry → FAIL.
//! - **Stale**: registry entry whose `rust_path` not found in source → FAIL.
//! - **Missing field**: any of the 6 metadata fields missing → FAIL.
//!
//! Preflight mandate (Murat): the check is a real oracle — enum enumeration
//! derives from AST/macro, not a hand-maintained list.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const REQUIRED_FIELDS: &[&str] = &[
    "code",
    "description",
    "severity",
    "recovery_class",
    "owner",
    "kernel_or_spirit",
    "since_version",
    "source_file",
];

const RUST_PATH_FIELD: &str = "rust_path";

const VALID_SEVERITIES: &[&str] = &["user", "policy", "security", "infra", "internal"];
const VALID_RECOVERY_CLASSES: &[&str] = &[
    "retry",
    "retry_with_correction",
    "reject",
    "fix_config",
    "escalate",
];
const VALID_KERNEL_OR_SPIRIT: &[&str] = &["kernel", "spirit", "both"];

// -----------------------------------------------------------------------
// TOML registry types
// -----------------------------------------------------------------------

#[derive(Debug, serde::Deserialize)]
struct Catalog {
    meta: CatalogMeta,
    error: Vec<ErrorEntry>,
}

#[derive(Debug, serde::Deserialize)]
struct CatalogMeta {
    #[allow(dead_code)]
    catalog_version: String,
    scan_dirs: Vec<String>,
    min_entry_count: usize,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
struct ErrorEntry {
    code: String,
    rust_path: String,
    source_file: String,
    description: String,
    severity: String,
    recovery_class: String,
    owner: String,
    kernel_or_spirit: String,
    since_version: String,
}

// -----------------------------------------------------------------------
// AST scanner — finds E*-prefixed variants and E*-prefixed error enums
// -----------------------------------------------------------------------

/// An E* item discovered in source via AST scan.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct DiscoveredItem {
    /// `EnumName::VariantName`
    rust_path: String,
    /// Relative source file path.
    source_file: String,
}

/// Returns true if the name starts with `E` followed by an uppercase letter.
fn is_e_prefixed(name: &str) -> bool {
    let mut chars = name.chars();
    matches!((chars.next(), chars.next()), (Some('E'), Some(c)) if c.is_ascii_uppercase())
}

/// Check if an enum derives `thiserror::Error` by inspecting its attributes.
///
/// Matches the literal path `thiserror::Error` or the unqualified `Error` token.
/// This avoids false positives on derive macros whose names merely contain
/// the substring "Error".
fn derives_thiserror(attrs: &[syn::Attribute]) -> bool {
    for attr in attrs {
        if attr.path().is_ident("derive") {
            let tokens = attr.meta.require_list().ok();
            if let Some(list) = tokens {
                let mut tokens = list.tokens.clone().into_iter();
                while let Some(token) = tokens.next() {
                    if let proc_macro2::TokenTree::Ident(first) = &token {
                        if first == "Error" {
                            return true;
                        }
                        if first == "thiserror" {
                            // Consume `::Error`
                            let mut saw_colons = 0u8;
                            while saw_colons < 2 {
                                match tokens.next() {
                                    Some(proc_macro2::TokenTree::Punct(p))
                                        if p.as_char() == ':' =>
                                    {
                                        saw_colons += 1;
                                    }
                                    _ => break,
                                }
                            }
                            if saw_colons == 2 {
                                if let Some(proc_macro2::TokenTree::Ident(last)) = tokens.next() {
                                    if last == "Error" {
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                    // Skip punctuation/commas between paths; ident handled above.
                }
            }
        }
    }
    false
}

/// Check if a block is inside `#[cfg(test)]`.
///
/// Parses the `cfg` predicate and looks for the exact identifier `test`,
/// avoiding false positives on attributes containing the substring "test"
/// (e.g. `feature = "contest"`).
fn is_cfg_test(attrs: &[syn::Attribute]) -> bool {
    use syn::Meta;

    for attr in attrs {
        if attr.path().is_ident("cfg") {
            if let Ok(list) = attr.meta.require_list() {
                let tokens = list.tokens.to_string();
                if tokens == "test" {
                    return true;
                }
                // Structured parse for compound predicates (e.g. cfg(test), cfg(not(test)),
                // cfg(all(test, ...))). `syn::Meta` can represent the simple path form.
                if let Ok(meta) = syn::parse_str::<Meta>(&tokens) {
                    if matches!(
                        meta,
                        Meta::Path(path) if path.is_ident("test")
                    ) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Scan a single Rust source file for E*-prefixed items.
///
/// Returns an error string on I/O or parse failure so that scan failures are
/// visible rather than silently producing an empty discovered set.
fn scan_file(path: &Path, rel_path: &str) -> Result<Vec<DiscoveredItem>, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
    let file = syn::parse_file(&content)
        .map_err(|e| format!("failed to parse {}: {}", path.display(), e))?;

    let mut items = Vec::new();
    scan_items(&file.items, rel_path, false, &mut items);
    Ok(items)
}

/// Recursively scan items, handling modules and cfg(test) filtering.
fn scan_items(
    items: &[syn::Item],
    rel_path: &str,
    in_cfg_test: bool,
    out: &mut Vec<DiscoveredItem>,
) {
    for item in items {
        match item {
            syn::Item::Mod(m) => {
                let cfg_test = in_cfg_test || is_cfg_test(&m.attrs);
                if let Some((_, ref inner_items)) = m.content {
                    scan_items(inner_items, rel_path, cfg_test, out);
                }
            }
            syn::Item::Enum(e) if !in_cfg_test => {
                let enum_name = e.ident.to_string();
                let enum_is_e_prefixed = is_e_prefixed(&enum_name);
                let enum_derives_error = derives_thiserror(&e.attrs);

                for variant in &e.variants {
                    let variant_name = variant.ident.to_string();

                    // Rule 1: E*-prefixed variant name
                    // Rule 2: variant inside E*-prefixed error enum
                    if is_e_prefixed(&variant_name) || (enum_is_e_prefixed && enum_derives_error) {
                        // Both discovery rules register as EnumName::VariantName; the
                        // registry is the canonical place for the human-readable code.
                        let rust_path = format!("{}::{}", enum_name, variant_name);
                        out.push(DiscoveredItem {
                            rust_path,
                            source_file: rel_path.to_string(),
                        });
                    }
                }
            }
            _ => {}
        }
    }
}

/// Walk a directory tree and collect all `.rs` files.
fn find_rs_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            find_rs_files(&path, files);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            files.push(path);
        }
    }
}

/// Discover all E* items across the configured scan directories.
fn discover_e_star_items(
    workspace_root: &Path,
    scan_dirs: &[String],
) -> Result<BTreeSet<DiscoveredItem>, String> {
    let mut all = BTreeSet::new();
    for dir in scan_dirs {
        let abs_dir = workspace_root.join(dir);
        let mut files = Vec::new();
        find_rs_files(&abs_dir, &mut files);
        for file in files {
            let rel = file
                .strip_prefix(workspace_root)
                .unwrap_or(&file)
                .to_string_lossy()
                .replace('\\', "/");
            for item in scan_file(&file, &rel)? {
                all.insert(item);
            }
        }
    }
    Ok(all)
}

// -----------------------------------------------------------------------
// Validation
// -----------------------------------------------------------------------

#[derive(Debug)]
struct Violation {
    kind: ViolationKind,
    detail: String,
}

#[derive(Debug)]
enum ViolationKind {
    UnCatalogued,
    Stale,
    MissingField,
    InvalidValue,
    MinCount,
}

impl std::fmt::Display for ViolationKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnCatalogued => write!(f, "un-catalogued"),
            Self::Stale => write!(f, "stale"),
            Self::MissingField => write!(f, "missing-field"),
            Self::InvalidValue => write!(f, "invalid-value"),
            Self::MinCount => write!(f, "min-count"),
        }
    }
}

fn validate(catalog: &Catalog, discovered: &BTreeSet<DiscoveredItem>) -> Vec<Violation> {
    let mut violations = Vec::new();

    // Build registry lookup by rust_path; detect duplicate rust_paths in TOML.
    let mut registry: BTreeMap<&str, &ErrorEntry> = BTreeMap::new();
    for entry in &catalog.error {
        if registry.insert(entry.rust_path.as_str(), entry).is_some() {
            violations.push(Violation {
                kind: ViolationKind::InvalidValue,
                detail: format!("duplicate rust_path `{}` in registry", entry.rust_path),
            });
        }
    }

    // Cross-check source_file for discovered items whose rust_path matches.
    let discovered_by_path: BTreeMap<&str, &DiscoveredItem> = discovered
        .iter()
        .map(|d| (d.rust_path.as_str(), d))
        .collect();

    // 1. Un-catalogued: in source but not in registry
    for item in discovered {
        if !registry.contains_key(item.rust_path.as_str()) {
            violations.push(Violation {
                kind: ViolationKind::UnCatalogued,
                detail: format!(
                    "E* item `{}` found in {} but absent from registry",
                    item.rust_path, item.source_file
                ),
            });
        }
    }

    // 2. Stale or source-file drift: in registry but not in source, or wrong source_file
    for entry in &catalog.error {
        match discovered_by_path.get(entry.rust_path.as_str()) {
            None => {
                violations.push(Violation {
                    kind: ViolationKind::Stale,
                    detail: format!(
                        "registry entry `{}` (code={}) not found in source",
                        entry.rust_path, entry.code
                    ),
                });
            }
            Some(discovered) if discovered.source_file != entry.source_file => {
                violations.push(Violation {
                    kind: ViolationKind::Stale,
                    detail: format!(
                        "registry entry `{}` (code={}) moved from `{}` to `{}`",
                        entry.rust_path, entry.code, entry.source_file, discovered.source_file
                    ),
                });
            }
            _ => {}
        }
    }

    // 3. Missing fields
    for entry in &catalog.error {
        for &field in REQUIRED_FIELDS {
            let value = match field {
                "code" => &entry.code,
                "description" => &entry.description,
                "severity" => &entry.severity,
                "recovery_class" => &entry.recovery_class,
                "owner" => &entry.owner,
                "kernel_or_spirit" => &entry.kernel_or_spirit,
                "since_version" => &entry.since_version,
                "source_file" => &entry.source_file,
                _ => continue,
            };
            if value.is_empty() {
                violations.push(Violation {
                    kind: ViolationKind::MissingField,
                    detail: format!("{}: required field `{}` is empty", entry.code, field),
                });
            }
        }

        // rust_path is required but not part of REQUIRED_FIELDS because it is
        // structural (used for the bijection lookup) and TOML cannot represent
        // an empty rust_path without an explicit empty-string value.
        if entry.rust_path.is_empty() {
            violations.push(Violation {
                kind: ViolationKind::MissingField,
                detail: format!(
                    "{}: required field `{}` is empty",
                    entry.code, RUST_PATH_FIELD
                ),
            });
        }

        // Validate enum values
        if !VALID_SEVERITIES.contains(&entry.severity.as_str()) && !entry.severity.is_empty() {
            violations.push(Violation {
                kind: ViolationKind::InvalidValue,
                detail: format!(
                    "{}: severity `{}` not in {:?}",
                    entry.code, entry.severity, VALID_SEVERITIES
                ),
            });
        }
        if !VALID_RECOVERY_CLASSES.contains(&entry.recovery_class.as_str())
            && !entry.recovery_class.is_empty()
        {
            violations.push(Violation {
                kind: ViolationKind::InvalidValue,
                detail: format!(
                    "{}: recovery_class `{}` not in {:?}",
                    entry.code, entry.recovery_class, VALID_RECOVERY_CLASSES
                ),
            });
        }
        if !VALID_KERNEL_OR_SPIRIT.contains(&entry.kernel_or_spirit.as_str())
            && !entry.kernel_or_spirit.is_empty()
        {
            violations.push(Violation {
                kind: ViolationKind::InvalidValue,
                detail: format!(
                    "{}: kernel_or_spirit `{}` not in {:?}",
                    entry.code, entry.kernel_or_spirit, VALID_KERNEL_OR_SPIRIT
                ),
            });
        }
    }

    // 4. Min entry count guard (F2 regression floor)
    if catalog.error.len() < catalog.meta.min_entry_count {
        violations.push(Violation {
            kind: ViolationKind::MinCount,
            detail: format!(
                "catalog has {} entries, below min_entry_count={}",
                catalog.error.len(),
                catalog.meta.min_entry_count
            ),
        });
    }

    violations
}

// -----------------------------------------------------------------------
// Generator — deterministic per-error artifact
// -----------------------------------------------------------------------

/// Generate a deterministic machine-readable catalog artifact.
/// Returns the artifact content as a byte-deterministic JSON string.
///
/// Panics if the catalog contains an unrecognized severity; run validation
/// before generation to obtain a structured error instead.
fn generate_catalog_artifact(catalog: &Catalog) -> String {
    // BTreeMap for deterministic ordering by code
    let mut entries: BTreeMap<&str, serde_json::Value> = BTreeMap::new();

    for entry in &catalog.error {
        let retryable = match entry.recovery_class.as_str() {
            "retry" => true,
            "retry_with_correction" => false,
            "reject" => false,
            "fix_config" => false,
            "escalate" => false,
            _ => false,
        };

        let cause_chain = match entry.severity.as_str() {
            "security" => "security policy enforcement",
            "policy" => "admission or runtime policy",
            "user" => "invalid user input or request",
            "infra" => "infrastructure or environment",
            "internal" => "kernel internal invariant",
            other => panic!(
                "unrecognized severity `{}` for `{}`; run validation before generation",
                other, entry.code
            ),
        };

        entries.insert(
            &entry.code,
            serde_json::json!({
                "code": entry.code,
                "rust_path": entry.rust_path,
                "severity": entry.severity,
                "recovery_class": entry.recovery_class,
                "retryable": retryable,
                "cause_chain_semantics": cause_chain,
                "owner": entry.owner,
                "kernel_or_spirit": entry.kernel_or_spirit,
                "since_version": entry.since_version,
                "version_stability": "stable within LTS cycle — no breaking error-code changes",
                "description": entry.description,
                "docs_url": format!("https://docs.maos.dev/errors/{}", entry.code),
            }),
        );
    }

    // Deterministic JSON: sorted keys, 2-space indent, trailing newline for
    // POSIX text-file hygiene.
    let mut json =
        serde_json::to_string_pretty(&entries).expect("catalog serialization cannot fail");
    json.push('\n');
    json
}

// -----------------------------------------------------------------------
// Public entry point
// -----------------------------------------------------------------------

pub fn run(catalog_path: &str, json: bool) -> Result<(), String> {
    let workspace_root = workspace_root_from_cargo_manifest_dir()?;

    // Load catalog
    let catalog_content = std::fs::read_to_string(catalog_path)
        .map_err(|e| format!("failed to read {}: {}", catalog_path, e))?;
    let catalog: Catalog = toml::from_str(&catalog_content)
        .map_err(|e| format!("failed to parse {}: {}", catalog_path, e))?;

    // AST-discover E* items
    let discovered = discover_e_star_items(&workspace_root, &catalog.meta.scan_dirs)?;

    // Validate
    let violations = validate(&catalog, &discovered);

    if json {
        let payload = serde_json::json!({
            "passed": violations.is_empty(),
            "discovered_count": discovered.len(),
            "registry_count": catalog.error.len(),
            "violation_count": violations.len(),
            "violations": violations.iter().map(|v| serde_json::json!({
                "kind": v.kind.to_string(),
                "detail": v.detail,
            })).collect::<Vec<_>>(),
        });
        println!("{}", payload);
    }

    if violations.is_empty() {
        if !json {
            eprintln!(
                "error-catalog-check: PASS — {} E* items registered, {} discovered in source",
                catalog.error.len(),
                discovered.len()
            );
        }
        return Ok(());
    }

    if !json {
        for v in &violations {
            eprintln!("error-catalog-check: [{}] {}", v.kind, v.detail);
        }
        eprintln!(
            "error-catalog-check: FAILED — {} violations ({} registered, {} discovered)",
            violations.len(),
            catalog.error.len(),
            discovered.len()
        );
    }

    Err(format!(
        "error-catalog-check failed: {} violations",
        violations.len()
    ))
}

/// Generate the catalog artifact to the given output path.
pub fn run_generate(catalog_path: &str, output_path: &str) -> Result<(), String> {
    let catalog_content = std::fs::read_to_string(catalog_path)
        .map_err(|e| format!("failed to read {}: {}", catalog_path, e))?;
    let catalog: Catalog = toml::from_str(&catalog_content)
        .map_err(|e| format!("failed to parse {}: {}", catalog_path, e))?;

    // Fail loudly on invalid catalog rather than producing an artifact with
    // placeholder / unknown values.
    let workspace_root = workspace_root_from_cargo_manifest_dir()?;
    let discovered = discover_e_star_items(&workspace_root, &catalog.meta.scan_dirs)?;
    let violations = validate(&catalog, &discovered);
    if !violations.is_empty() {
        return Err(format!(
            "catalog has {} violations; fix them before generating artifact",
            violations.len()
        ));
    }

    let artifact = generate_catalog_artifact(&catalog);

    // Ensure parent directory exists
    if let Some(parent) = Path::new(output_path).parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create {}: {}", parent.display(), e))?;
    }

    std::fs::write(output_path, &artifact)
        .map_err(|e| format!("failed to write {}: {}", output_path, e))?;

    eprintln!(
        "error-catalog-generate: wrote {} entries to {}",
        catalog.error.len(),
        output_path
    );
    Ok(())
}

/// Resolve the workspace root from `CARGO_MANIFEST_DIR` so the check is
/// independent of the current working directory.
fn workspace_root_from_cargo_manifest_dir() -> Result<PathBuf, String> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map_err(|e| format!("CARGO_MANIFEST_DIR not set: {}", e))?;
    Path::new(&manifest_dir)
        .parent()
        .map(|p| p.to_path_buf())
        .ok_or_else(|| {
            format!(
                "CARGO_MANIFEST_DIR `{}` has no parent; cannot resolve workspace root",
                manifest_dir
            )
        })
}

// -----------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_e_prefixed_positive() {
        assert!(is_e_prefixed("EFoo"));
        assert!(is_e_prefixed("EIntentLineageBroken"));
        assert!(is_e_prefixed("EAbiTooOld"));
    }

    #[test]
    fn is_e_prefixed_negative() {
        assert!(!is_e_prefixed("Error"));
        // lowercase after E
        assert!(!is_e_prefixed("Efoo"));
        assert!(!is_e_prefixed("ExportPayload"));
        assert!(!is_e_prefixed("ExitCause"));
        assert!(!is_e_prefixed(""));
        assert!(!is_e_prefixed("E"));
        assert!(!is_e_prefixed("Foo"));
    }

    #[test]
    fn scan_finds_e_prefixed_variant() {
        let src = r#"
            pub enum IacBusError {
                Foo,
                EIntentLineageBroken { from: String },
                Bar(String),
            }
        "#;
        let file = syn::parse_file(src).unwrap();
        let mut items = Vec::new();
        scan_items(&file.items, "test.rs", false, &mut items);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].rust_path, "IacBusError::EIntentLineageBroken");
    }

    #[test]
    fn scan_finds_e_prefixed_error_enum_variants() {
        let src = r#"
            #[derive(Debug, thiserror::Error)]
            pub enum EComplianceRejection {
                #[error("sig")]
                SignatureInvalid,
                #[error("drift")]
                ContextDrift { field: String },
            }
        "#;
        let file = syn::parse_file(src).unwrap();
        let mut items = Vec::new();
        scan_items(&file.items, "test.rs", false, &mut items);
        assert_eq!(items.len(), 2);
        let paths: Vec<&str> = items.iter().map(|i| i.rust_path.as_str()).collect();
        assert!(paths.contains(&"EComplianceRejection::SignatureInvalid"));
        assert!(paths.contains(&"EComplianceRejection::ContextDrift"));
    }

    #[test]
    fn scan_excludes_cfg_test() {
        let src = r#"
            #[cfg(test)]
            mod tests {
                pub enum ETestOnly {
                    EFoo,
                }
            }
            pub enum IacBusError {
                EReal { x: u32 },
            }
        "#;
        let file = syn::parse_file(src).unwrap();
        let mut items = Vec::new();
        scan_items(&file.items, "test.rs", false, &mut items);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].rust_path, "IacBusError::EReal");
    }

    #[test]
    fn scan_excludes_non_error_e_enum() {
        // E-prefixed enum without thiserror::Error — variants NOT included
        // unless the variant itself is E-prefixed
        let src = r#"
            #[derive(Debug, Clone)]
            pub enum ENotAnError {
                Foo,
                Bar,
            }
        "#;
        let file = syn::parse_file(src).unwrap();
        let mut items = Vec::new();
        scan_items(&file.items, "test.rs", false, &mut items);
        assert_eq!(items.len(), 0);
    }

    // -------------------------------------------------------------------
    // AC2 — Anti-tautology negative meta-tests
    // -------------------------------------------------------------------

    /// AC2(a): removing a required field from a registry entry causes failure.
    #[test]
    fn negative_meta_test_missing_field_detected() {
        let toml_src = r#"
            [meta]
            catalog_version = "1.0.0"
            scan_dirs = []
            min_entry_count = 0

            [[error]]
            code = "ETest"
            rust_path = "TestEnum::ETest"
            source_file = "test.rs"
            description = "test error"
            severity = "user"
            recovery_class = ""
            owner = "test"
            kernel_or_spirit = "kernel"
            since_version = "0.1.0"
        "#;

        let catalog: Catalog = toml::from_str(toml_src).unwrap();
        let mut discovered = BTreeSet::new();
        discovered.insert(DiscoveredItem {
            rust_path: "TestEnum::ETest".to_string(),
            source_file: "test.rs".to_string(),
        });

        let violations = validate(&catalog, &discovered);
        assert!(
            !violations.is_empty(),
            "checker MUST fail when recovery_class is empty — gate is falsifiable"
        );
        assert!(
            violations
                .iter()
                .any(|v| matches!(v.kind, ViolationKind::MissingField)),
            "must report a missing-field violation"
        );
    }

    /// AC2(b): un-catalogued E* error causes failure.
    #[test]
    fn negative_meta_test_uncatalogued_detected() {
        let toml_src = r#"
            [meta]
            catalog_version = "1.0.0"
            scan_dirs = []
            min_entry_count = 0

            [[error]]
            code = "EKnown"
            rust_path = "TestEnum::EKnown"
            source_file = "test.rs"
            description = "known error"
            severity = "user"
            recovery_class = "retry"
            owner = "test"
            kernel_or_spirit = "kernel"
            since_version = "0.1.0"
        "#;

        let catalog: Catalog = toml::from_str(toml_src).unwrap();
        let mut discovered = BTreeSet::new();
        discovered.insert(DiscoveredItem {
            rust_path: "TestEnum::EKnown".to_string(),
            source_file: "test.rs".to_string(),
        });
        // This one is NOT in the registry
        discovered.insert(DiscoveredItem {
            rust_path: "TestEnum::ENewUncatalogued".to_string(),
            source_file: "test.rs".to_string(),
        });

        let violations = validate(&catalog, &discovered);
        assert!(
            !violations.is_empty(),
            "checker MUST fail when an E* item is un-catalogued — gate is falsifiable"
        );
        assert!(
            violations
                .iter()
                .any(|v| matches!(v.kind, ViolationKind::UnCatalogued)),
            "must report an un-catalogued violation"
        );
    }

    /// AC2: stale registry entry (source removed) causes failure.
    #[test]
    fn negative_meta_test_stale_detected() {
        let toml_src = r#"
            [meta]
            catalog_version = "1.0.0"
            scan_dirs = []
            min_entry_count = 0

            [[error]]
            code = "EGone"
            rust_path = "TestEnum::EGone"
            source_file = "test.rs"
            description = "gone error"
            severity = "user"
            recovery_class = "retry"
            owner = "test"
            kernel_or_spirit = "kernel"
            since_version = "0.1.0"
        "#;

        let catalog: Catalog = toml::from_str(toml_src).unwrap();
        // Empty discovered set — the variant was removed from source
        let discovered = BTreeSet::new();

        let violations = validate(&catalog, &discovered);
        assert!(
            !violations.is_empty(),
            "checker MUST fail when a registry entry has no matching source — gate is falsifiable"
        );
        assert!(
            violations
                .iter()
                .any(|v| matches!(v.kind, ViolationKind::Stale)),
            "must report a stale violation"
        );
    }

    #[test]
    fn validate_passes_on_perfect_bijection() {
        let toml_src = r#"
            [meta]
            catalog_version = "1.0.0"
            scan_dirs = []
            min_entry_count = 1

            [[error]]
            code = "EFoo"
            rust_path = "MyEnum::EFoo"
            source_file = "test.rs"
            description = "foo"
            severity = "user"
            recovery_class = "retry"
            owner = "test"
            kernel_or_spirit = "kernel"
            since_version = "0.1.0"
        "#;

        let catalog: Catalog = toml::from_str(toml_src).unwrap();
        let mut discovered = BTreeSet::new();
        discovered.insert(DiscoveredItem {
            rust_path: "MyEnum::EFoo".to_string(),
            source_file: "test.rs".to_string(),
        });

        let violations = validate(&catalog, &discovered);
        assert!(
            violations.is_empty(),
            "perfect bijection must pass: {violations:?}"
        );
    }

    #[test]
    fn invalid_severity_detected() {
        let toml_src = r#"
            [meta]
            catalog_version = "1.0.0"
            scan_dirs = []
            min_entry_count = 0

            [[error]]
            code = "EFoo"
            rust_path = "MyEnum::EFoo"
            source_file = "test.rs"
            description = "foo"
            severity = "bogus"
            recovery_class = "retry"
            owner = "test"
            kernel_or_spirit = "kernel"
            since_version = "0.1.0"
        "#;

        let catalog: Catalog = toml::from_str(toml_src).unwrap();
        let mut discovered = BTreeSet::new();
        discovered.insert(DiscoveredItem {
            rust_path: "MyEnum::EFoo".to_string(),
            source_file: "test.rs".to_string(),
        });

        let violations = validate(&catalog, &discovered);
        assert!(
            violations
                .iter()
                .any(|v| matches!(v.kind, ViolationKind::InvalidValue)),
            "must report invalid-value for bogus severity"
        );
    }

    // -------------------------------------------------------------------
    // Generator determinism
    // -------------------------------------------------------------------

    #[test]
    fn generator_is_deterministic() {
        let toml_src = r#"
            [meta]
            catalog_version = "1.0.0"
            scan_dirs = []
            min_entry_count = 0

            [[error]]
            code = "EBeta"
            rust_path = "X::EBeta"
            source_file = "a.rs"
            description = "beta"
            severity = "user"
            recovery_class = "retry"
            owner = "test"
            kernel_or_spirit = "kernel"
            since_version = "0.1.0"

            [[error]]
            code = "EAlpha"
            rust_path = "X::EAlpha"
            source_file = "a.rs"
            description = "alpha"
            severity = "policy"
            recovery_class = "reject"
            owner = "test"
            kernel_or_spirit = "kernel"
            since_version = "0.1.0"
        "#;

        let catalog: Catalog = toml::from_str(toml_src).unwrap();
        let run1 = generate_catalog_artifact(&catalog);
        let run2 = generate_catalog_artifact(&catalog);
        assert_eq!(run1, run2, "two runs must produce identical bytes");

        // BTreeMap ordering — EAlpha before EBeta
        let alpha_pos = run1.find("EAlpha").unwrap();
        let beta_pos = run1.find("EBeta").unwrap();
        assert!(
            alpha_pos < beta_pos,
            "BTreeMap ordering: EAlpha must appear before EBeta"
        );
    }

    // -------------------------------------------------------------------
    // Link-shape test (AC3)
    // -------------------------------------------------------------------

    #[test]
    fn link_shape_test() {
        let toml_src = r#"
            [meta]
            catalog_version = "1.0.0"
            scan_dirs = []
            min_entry_count = 0

            [[error]]
            code = "EFoo"
            rust_path = "X::EFoo"
            source_file = "a.rs"
            description = "foo"
            severity = "user"
            recovery_class = "retry"
            owner = "test"
            kernel_or_spirit = "kernel"
            since_version = "0.1.0"
        "#;

        let catalog: Catalog = toml::from_str(toml_src).unwrap();
        let artifact = generate_catalog_artifact(&catalog);
        let parsed: BTreeMap<String, serde_json::Value> = serde_json::from_str(&artifact).unwrap();

        for (code, entry) in &parsed {
            let url = entry["docs_url"].as_str().unwrap();
            assert_eq!(
                url,
                &format!("https://docs.maos.dev/errors/{}", code),
                "each error must map to docs.maos.dev/errors/<ERR_NAME>"
            );
        }
    }

    // -------------------------------------------------------------------
    // Live catalog integration (runs against the actual workspace)
    // -------------------------------------------------------------------

    #[test]
    fn live_catalog_check_passes() {
        // This test runs the actual check against the workspace.
        // It will fail if the catalog drifts from source.
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();
        let catalog_path = workspace_root.join("xtask/error-catalog.toml");

        let catalog_content =
            std::fs::read_to_string(&catalog_path).expect("error-catalog.toml must exist");
        let catalog: Catalog =
            toml::from_str(&catalog_content).expect("error-catalog.toml must parse");

        let discovered = discover_e_star_items(&workspace_root, &catalog.meta.scan_dirs)
            .expect("discovering E* items must succeed");
        let violations = validate(&catalog, &discovered);

        if !violations.is_empty() {
            for v in &violations {
                eprintln!("[{}] {}", v.kind, v.detail);
            }
            panic!(
                "live catalog check failed with {} violations — run `cargo xtask error-catalog-check` for details",
                violations.len()
            );
        }
    }

    #[test]
    fn live_catalog_golden_file_determinism() {
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();
        let catalog_path = workspace_root.join("xtask/error-catalog.toml");

        let catalog_content =
            std::fs::read_to_string(&catalog_path).expect("error-catalog.toml must exist");
        let catalog: Catalog =
            toml::from_str(&catalog_content).expect("error-catalog.toml must parse");

        let run1 = generate_catalog_artifact(&catalog);
        let run2 = generate_catalog_artifact(&catalog);
        assert_eq!(
            run1.as_bytes(),
            run2.as_bytes(),
            "golden-file determinism: two runs must produce byte-identical output"
        );
    }

    #[test]
    fn live_catalog_link_shape() {
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();
        let catalog_path = workspace_root.join("xtask/error-catalog.toml");

        let catalog_content =
            std::fs::read_to_string(&catalog_path).expect("error-catalog.toml must exist");
        let catalog: Catalog =
            toml::from_str(&catalog_content).expect("error-catalog.toml must parse");

        let artifact = generate_catalog_artifact(&catalog);
        let parsed: BTreeMap<String, serde_json::Value> = serde_json::from_str(&artifact).unwrap();

        assert!(
            !parsed.is_empty(),
            "catalog must produce at least one entry"
        );

        for (code, entry) in &parsed {
            let url = entry["docs_url"]
                .as_str()
                .expect("docs_url must be a string");
            assert_eq!(
                url,
                &format!("https://docs.maos.dev/errors/{}", code),
                "link-shape: {} must map to docs.maos.dev/errors/<ERR_NAME>",
                code
            );
        }
    }
}
