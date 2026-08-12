use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use syn::spanned::Spanned;
use syn::visit::Visit;

/// Services that will be supervised at v0.5+ (architecture §4.0.8).
/// At v0.1-alpha these are modules inside maos-kernel-core; the const is declared
/// but NOT iterated because the v0.5+ crate layout (`crates/services/<name>/`)
/// does not yet exist. Story 2.2 owns the iteration.
const SUPERVISED_SERVICES: &[&str] = &["security", "memory", "iac", "capability"];
const SUPERVISOR: &str = "spirit-scheduler";

/// Adapter names per service, in canonical iteration order for the per-service payload.
///
/// AC1 (P1 — single supervising owner per service): every adapter must be constructed
/// exactly once in the composition root (`crates/maos-bin/src/main.rs` at v0.1-β).
const SERVICE_ADAPTERS: &[(&str, &str)] = &[
    ("security", "SecurityManagerAdapter"),
    ("memory", "MemoryManagerAdapter"),
    ("iac", "IacBusAdapter"),
    ("capability", "CapabilityRegistryAdapter"),
    ("io", "IoSubsystemAdapter"),
    ("telemetry", "TelemetryStreamAdapter"),
    ("spirit-scheduler", "SpiritSchedulerAdapter"),
];

/// Adapters that lack a corresponding Port trait re-export at v0.1-β.
///
/// AC2 (P2 — ports-not-adapters): every Adapter in `api::*` must be paired with its
/// Port trait. Entries here document the explicit exemption rationale.
const ADAPTER_PORT_EXEMPTIONS: &[(&str, &str, &str)] = &[
    (
        "TransparencyLogAdapter",
        "N/A",
        "Story 1b.1 audit-side adapter; no Port trait at v0.1-β — exemption per §4.0.8 supervisor-exception-shaped rationale",
    ),
    (
        "JournalAdapter",
        "N/A",
        "Story 1b.1 audit-side adapter; no Port trait at v0.1-β — exemption per §4.0.8 supervisor-exception-shaped rationale",
    ),
    // Story 7.1.7 baseline-reset (P2 triage): the following api::* re-exports are
    // NOT adapter↔port pairs and so have no Port trait to require. Each carries a
    // written rationale per the AC3 escape-hatch discipline.
    (
        "IacBusAdapter",
        "IacBusPort",
        "Story 7.1.7: the IAC bus is a module inside maos-kernel-core (`crate::iac`), not a standalone `iac/` service-dir, at the v0.1-β services-as-modules layout. The IacBusPort trait lives in maos-domain::ports and is consumed directly; no in-crate `iac/` service module re-export exists by design.",
    ),
    (
        "take_io_journal",
        "N/A",
        "Story 7.1.7: free function (io-journal drain accessor) re-exported via api::io for operator tooling — not an Adapter, so no Port trait pairing applies.",
    ),
    (
        "SetScalarError",
        "N/A",
        "Story 7.1.7: error type re-exported via api::capability — not an Adapter, so no Port trait pairing applies.",
    ),
    (
        "WorkingMemorySlot",
        "N/A",
        "Story 7.1.7: working-memory value type re-exported via api::capability — not an Adapter; its port surface is the CapabilityRegistryPort, no dedicated WorkingMemorySlotPort exists.",
    ),
    (
        "WorkingMemoryStore",
        "N/A",
        "Story 7.1.7: working-memory store type re-exported via api::capability — not an Adapter; its port surface is the CapabilityRegistryPort, no dedicated WorkingMemoryStorePort exists.",
    ),
    (
        "HotSwapCoordinator",
        "N/A",
        "Story 7.1.7: kernel-internal hot-swap supervisor composite re-exported via api::hot_swap for the composition root — it IS the orchestrator and has no consumer-facing Port trait (supervisor-exception per §4.0.8).",
    ),
    (
        "McpClientAdapter",
        "McpClientPort",
        "Story 7.1.7: MCP client adapter re-exported via api::mcp; the McpClientPort trait is consumed from maos-domain::ports directly and is not re-exported through the in-crate `mcp/` service module at v0.1-β. Port-pairing re-export tracked as deferred tidy-up.",
    ),
];

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Violation {
    pub file: String,
    pub line: usize,
    pub path: String,
    pub message: String,
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct KernelSurface {
    pub crate_name: String,
    pub abi_baseline_version: String,
    pub items: Vec<SurfaceItem>,
}

#[derive(
    Debug, serde::Serialize, serde::Deserialize, Ord, PartialOrd, Eq, PartialEq, Clone, Hash,
)]
pub struct SurfaceItem {
    pub kind: String,
    pub path: String,
    pub signature_hash: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Report {
    pub passed: bool,
    pub violations: Vec<Violation>,
    pub current_surface: KernelSurface,
    pub p1_p4_status: serde_json::Value,
    pub spirit_abi_types: serde_json::Value,
}

#[derive(Debug, serde::Deserialize)]
struct ApiClasses {
    classes: BTreeMap<String, String>,
}

pub fn run(
    path: Option<&str>,
    baseline_path: &str,
    classes_path: &str,
    p4_denylist_path: &str,
    p4_exemptions_path: &str,
    spirit_abi_lifecycle: &str,
    spirit_abi_derive: &str,
    json: bool,
) -> Result<(), String> {
    let report = check_service_boundary(
        path.map(Path::new),
        Path::new(baseline_path),
        Path::new(classes_path),
        Path::new(p4_denylist_path),
        Path::new(p4_exemptions_path),
        Path::new(spirit_abi_lifecycle),
        Path::new(spirit_abi_derive),
    )?;

    if json {
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    } else {
        if report.passed {
            println!("check-service-boundary: PASSED (0 violations)");
        } else {
            for v in &report.violations {
                eprintln!("{v}");
            }
        }
        println!(
            "INFO: {}",
            serde_json::to_string(&report.p1_p4_status).unwrap()
        );
    }

    if !report.passed {
        return Err("check-service-boundary failed".into());
    }

    Ok(())
}

fn check_service_boundary(
    path: Option<&Path>,
    baseline_path: &Path,
    classes_path: &Path,
    p4_denylist_path: &Path,
    p4_exemptions_path: &Path,
    spirit_abi_lifecycle: &Path,
    spirit_abi_derive: &Path,
) -> Result<Report, String> {
    let crate_path = path.unwrap_or(Path::new("crates/maos-kernel-core"));
    let workspace_root = if crate_path.file_name() == Some(std::ffi::OsStr::new("maos-kernel-core"))
    {
        crate_path.ancestors().nth(2).unwrap_or(Path::new("."))
    } else {
        crate_path
    };

    let current = snapshot_kernel_surface(crate_path)?;
    let classes: ApiClasses = if classes_path.exists() {
        load_toml(classes_path)?
    } else {
        ApiClasses {
            classes: BTreeMap::new(),
        }
    };

    let mut violations = Vec::new();

    // Diff against baseline if it exists and is non-empty.
    if baseline_path.exists() {
        let baseline_src = fs::read_to_string(baseline_path)
            .map_err(|e| format!("cannot read {}: {e}", baseline_path.display()))?;
        let trimmed = baseline_src.trim();
        if trimmed.is_empty() {
            // Truly empty (0 bytes) baseline -> skip diffing.
        } else if !trimmed.starts_with('{') {
            return Err(format!(
                "baseline file {} is not empty but does not contain valid JSON (whitespace-only?)",
                baseline_path.display()
            ));
        } else {
            let baseline: KernelSurface = serde_json::from_str(&baseline_src)
                .map_err(|e| format!("json parse error in {}: {e}", baseline_path.display()))?;

            let baseline_items: std::collections::HashSet<SurfaceItem> =
                baseline.items.into_iter().collect();
            let current_items: std::collections::HashSet<SurfaceItem> =
                current.items.clone().into_iter().collect();

            // Removed items are violations (monotonicity).
            for item in &baseline_items {
                if !current_items.contains(item) {
                    violations.push(Violation {
                    file: baseline_path.display().to_string(),
                    line: 1,
                    path: item.path.clone(),
                    message: format!(
                        "NFR-Test-2 violation: removed public kernel symbol '{}' — kernel surface is monotonically additive within a major version (see ABI Stability Triple)",
                        item.path
                    ),
                });
                }
            }

            // Added items must be classified.
            for item in &current_items {
                if !baseline_items.contains(item) {
                    let class = classes
                        .classes
                        .get(&item.path)
                        .cloned()
                        .unwrap_or_else(|| "other".into());
                    if class == "other" {
                        violations.push(Violation {
                        file: baseline_path.display().to_string(),
                        line: 1,
                        path: item.path.clone(),
                        message: format!(
                            "NFR-Test-2 violation: new public kernel symbol '{}' has class 'other' (must be one of: universal-arithmetic, data-movement, supervision); add classification to xtask/kernel-api-classes.toml via invariant-lock review",
                            item.path
                        ),
                    });
                    }
                }
            }
        }
    }

    // P1 — single supervising owner per service (AC1).
    let p1_main_rs = workspace_root.join("crates/maos-bin/src/main.rs");
    let p1_path = if p1_main_rs.exists() {
        &p1_main_rs
    } else {
        &workspace_root.join("src/main.rs")
    };
    if p1_path.exists() {
        violations.extend(check_p1_single_owner(p1_path)?);
    }

    // P2 — ports-not-adapters at service boundary (AC2).
    violations.extend(check_p2_port_pairing(workspace_root)?);

    // P3 — state ownership behind Arc/DashMap/RwLock/atomic (AC3).
    violations.extend(check_p3_state_ownership(workspace_root)?);

    // P4 — audit-chain integrity via call-graph reachability (AC4).
    violations.extend(check_p4_audit_chain(
        workspace_root,
        p4_denylist_path,
        p4_exemptions_path,
    )?);

    // Spirit ABI type reflection (AC5).
    let (spirit_abi_violations, spirit_abi_json) =
        check_spirit_abi_types(workspace_root, spirit_abi_lifecycle, spirit_abi_derive)?;
    violations.extend(spirit_abi_violations);

    let passed = violations.is_empty();
    let p1_p4_status = p1_p4_status_payload(&violations);
    Ok(Report {
        passed,
        violations,
        current_surface: current,
        p1_p4_status: serde_json::json!({
            "v0_1_layout": "services-as-modules-under-maos-kernel-core",
            "supervised_services": SUPERVISED_SERVICES,
            "supervisor": SUPERVISOR,
            "service_classifications": {
                "scheduler": "supervision",
                "security": "supervision",
                "memory": "data-movement",
                "iac": "data-movement",
                "capability": "universal-arithmetic",
                "io": "data-movement",
                "telemetry": "data-movement",
            },
            "p1_p4_per_service": p1_p4_status,
        }),
        spirit_abi_types: spirit_abi_json,
    })
}

fn snapshot_kernel_surface(crate_path: &Path) -> Result<KernelSurface, String> {
    let mut items = Vec::new();
    let src_dir = crate_path.join("src");
    let lib_rs = src_dir.join("lib.rs");

    if lib_rs.exists() {
        walk_mod(&lib_rs, &src_dir, "maos_kernel_core", &mut items)?;
    }

    items.sort();
    items.dedup();

    Ok(KernelSurface {
        crate_name: "maos-kernel-core".into(),
        abi_baseline_version: "v0.1-beta".into(),
        items,
    })
}

fn walk_mod(
    file: &Path,
    src_dir: &Path,
    mod_path: &str,
    items: &mut Vec<SurfaceItem>,
) -> Result<(), String> {
    let src =
        fs::read_to_string(file).map_err(|e| format!("cannot read {}: {e}", file.display()))?;
    let ast =
        syn::parse_file(&src).map_err(|e| format!("parse error in {}: {e}", file.display()))?;

    for item in &ast.items {
        match item {
            syn::Item::Fn(i) if is_pub(&i.vis) => {
                items.push(surface_item(
                    "fn",
                    &format!("{}::{}", mod_path, i.sig.ident),
                    item,
                ));
            }
            syn::Item::Struct(i) if is_pub(&i.vis) => {
                items.push(surface_item(
                    "struct",
                    &format!("{}::{}", mod_path, i.ident),
                    item,
                ));
            }
            syn::Item::Enum(i) if is_pub(&i.vis) => {
                items.push(surface_item(
                    "enum",
                    &format!("{}::{}", mod_path, i.ident),
                    item,
                ));
            }
            syn::Item::Trait(i) if is_pub(&i.vis) => {
                items.push(surface_item(
                    "trait",
                    &format!("{}::{}", mod_path, i.ident),
                    item,
                ));
            }
            syn::Item::Type(i) if is_pub(&i.vis) => {
                items.push(surface_item(
                    "type",
                    &format!("{}::{}", mod_path, i.ident),
                    item,
                ));
            }
            syn::Item::Const(i) if is_pub(&i.vis) => {
                items.push(surface_item(
                    "const",
                    &format!("{}::{}", mod_path, i.ident),
                    item,
                ));
            }
            syn::Item::Static(i) if is_pub(&i.vis) => {
                items.push(surface_item(
                    "static",
                    &format!("{}::{}", mod_path, i.ident),
                    item,
                ));
            }
            syn::Item::Use(i) if is_pub(&i.vis) => {
                for path in collect_use_paths(&i.tree, mod_path) {
                    items.push(surface_item("use", &path, item));
                }
            }
            syn::Item::Mod(i) if is_pub(&i.vis) && !is_test_cfg_mod(i) => {
                // Recurse into pub mod for child items, but do NOT emit mod as surface item.
                if let Some((_, content)) = &i.content {
                    let child_path = format!("{}::{}", mod_path, i.ident);
                    for child in content {
                        walk_inline_mod_item(child, &child_path, items)?;
                    }
                } else {
                    let parent = file.parent().unwrap_or(src_dir);
                    let child_name = i.ident.to_string();
                    let child_file = parent.join(format!("{}.rs", child_name));
                    let child_mod_dir = parent.join(&child_name).join("mod.rs");
                    let child_path = format!("{}::{}", mod_path, i.ident);
                    if child_file.exists() {
                        walk_mod(&child_file, src_dir, &child_path, items)?;
                    } else if child_mod_dir.exists() {
                        walk_mod(&child_mod_dir, src_dir, &child_path, items)?;
                    }
                }
            }
            _ => {}
        }
    }

    Ok(())
}

fn walk_inline_mod_item(
    item: &syn::Item,
    mod_path: &str,
    items: &mut Vec<SurfaceItem>,
) -> Result<(), String> {
    match item {
        syn::Item::Fn(i) if is_pub(&i.vis) => {
            items.push(surface_item(
                "fn",
                &format!("{}::{}", mod_path, i.sig.ident),
                item,
            ));
        }
        syn::Item::Struct(i) if is_pub(&i.vis) => {
            items.push(surface_item(
                "struct",
                &format!("{}::{}", mod_path, i.ident),
                item,
            ));
        }
        syn::Item::Enum(i) if is_pub(&i.vis) => {
            items.push(surface_item(
                "enum",
                &format!("{}::{}", mod_path, i.ident),
                item,
            ));
        }
        syn::Item::Trait(i) if is_pub(&i.vis) => {
            items.push(surface_item(
                "trait",
                &format!("{}::{}", mod_path, i.ident),
                item,
            ));
        }
        syn::Item::Type(i) if is_pub(&i.vis) => {
            items.push(surface_item(
                "type",
                &format!("{}::{}", mod_path, i.ident),
                item,
            ));
        }
        syn::Item::Const(i) if is_pub(&i.vis) => {
            items.push(surface_item(
                "const",
                &format!("{}::{}", mod_path, i.ident),
                item,
            ));
        }
        syn::Item::Static(i) if is_pub(&i.vis) => {
            items.push(surface_item(
                "static",
                &format!("{}::{}", mod_path, i.ident),
                item,
            ));
        }
        syn::Item::Use(i) if is_pub(&i.vis) => {
            for path in collect_use_paths(&i.tree, mod_path) {
                items.push(surface_item("use", &path, item));
            }
        }
        syn::Item::Mod(i) if is_pub(&i.vis) && !is_test_cfg_mod(i) => {
            // Recurse into inline pub mod, but do NOT emit mod as surface item.
            if let Some((_, content)) = &i.content {
                let child_path = format!("{}::{}", mod_path, i.ident);
                for child in content {
                    walk_inline_mod_item(child, &child_path, items)?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn is_pub(vis: &syn::Visibility) -> bool {
    matches!(vis, syn::Visibility::Public(_))
}

/// A `mod` gated behind a test-bearing `#[cfg(...)]` is NOT part of the shipped
/// kernel surface: it does not exist in a release build.
///
/// The P4 walk has always applied this rule (`walk_p4_mod` /
/// `walk_p4_inline_item`); the SURFACE walk never received it, so the two halves
/// of this gate disagreed about the same module. It went unnoticed until Story
/// 13.6c added `maos_kernel_core::memory::spill_test_faults` — the first `pub
/// mod` under `#[cfg(any(test, debug_assertions))]` to reach the surface walk —
/// whose five functions were then reported as unclassified public kernel API,
/// reddening a blocking gate that is in `aggregate`'s needs.
///
/// Classifying them would have been the wrong fix: it would bless test
/// fault-injection as permanent public kernel API and assert something false,
/// since the symbols do not exist in a release build. `kloc_check` already
/// excludes this module for the same reason. Predicate matched by substring,
/// exactly as the P4 walk does, so `test`, `any(test, debug_assertions)` and
/// `all(test, feature = "...")` are all covered.
fn is_test_cfg_mod(item: &syn::ItemMod) -> bool {
    item.attrs.iter().any(|attr| {
        attr.meta.path().is_ident("cfg")
            && attr
                .meta
                .require_list()
                .is_ok_and(|list| list.tokens.to_string().contains("test"))
    })
}

fn surface_item(kind: &str, path: &str, item: &syn::Item) -> SurfaceItem {
    let sig = canonicalize_signature(item);
    SurfaceItem {
        kind: kind.into(),
        path: path.into(),
        signature_hash: sha256_hex(&sig),
    }
}

/// Render an item's surface via `quote!` to a string, strip doc attributes and
/// whitespace, and hash.
///
/// Item identity in the baseline diff is `(kind, path, signature_hash)`, so
/// whatever this hashes IS what the monotonicity check treats as the kernel
/// surface. Two things are therefore deliberately excluded:
///
/// - **Function bodies.** A function's ABI is its signature; the body is an
///   implementation detail. Hashing `quote!(#item)` for a fn made every body
///   edit read as "removed public kernel symbol" + "new unclassified symbol".
///   Epic 13's first CI run reported exactly that for `spawn_and_bridge` after
///   the J1 stdin-deadlock fix (`0a03468f`), which changed no signature and
///   removed nothing. For every OTHER item kind the whole item is the surface —
///   struct fields, enum variants and const values are all ABI — so those are
///   still hashed in full.
/// - **Doc attributes.** The previous filter dropped lines starting with `///`,
///   but `quote!` renders doc comments as `#[doc = "..."]` attributes on a
///   single line, so it never matched anything — a null control that let a
///   pure documentation edit red the gate with a "removed symbol" message.
///
/// TODO: `quote!`-based signature hashing is not stable across `syn` major versions.
/// Migrate to `cargo-public-api` in Story 1a.1 (same deferred migration as `abi_diff.rs`).
fn canonicalize_signature(item: &syn::Item) -> String {
    let tokens = match item {
        syn::Item::Fn(f) => {
            let (vis, sig) = (&f.vis, &f.sig);
            quote::quote!(#vis #sig).to_string()
        }
        _ => quote::quote!(#item).to_string(),
    };
    // Normalize whitespace.
    strip_doc_attrs(&tokens)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Remove `#[doc = "..."]` attributes from a `quote!`-rendered token string.
///
/// `quote!` emits them as `# [doc = "..."]` with single spaces. The string
/// literal is scanned with escape handling so a doc comment containing `\"`
/// (or a `]`) cannot terminate the scan early.
fn strip_doc_attrs(tokens: &str) -> String {
    const OPEN: &str = "# [doc = ";
    let mut out = String::with_capacity(tokens.len());
    let mut rest = tokens;
    while let Some(start) = rest.find(OPEN) {
        out.push_str(&rest[..start]);
        let after_open = &rest[start + OPEN.len()..];
        match doc_attr_end(after_open) {
            Some(end) => rest = &after_open[end..],
            // Malformed / unterminated: keep the remainder verbatim rather
            // than silently dropping surface.
            None => {
                out.push_str(&rest[start..]);
                return out;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Byte offset just past the `"..."]` that closes a `#[doc = ...]` attribute.
fn doc_attr_end(after_open: &str) -> Option<usize> {
    let bytes = after_open.as_bytes();
    if bytes.first() != Some(&b'"') {
        return None;
    }
    let mut i = 1;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            b'"' => {
                let close = after_open[i + 1..].find(']')?;
                return Some(i + 1 + close + 1);
            }
            _ => i += 1,
        }
    }
    None
}

fn sha256_hex(input: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn collect_use_paths(tree: &syn::UseTree, prefix: &str) -> Vec<String> {
    match tree {
        syn::UseTree::Name(name) => vec![format!("{}::{}", prefix, name.ident)],
        syn::UseTree::Rename(rename) => {
            vec![format!(
                "{}::{} (as {})",
                prefix, rename.ident, rename.rename
            )]
        }
        syn::UseTree::Glob(_) => vec![format!("{}::*", prefix)],
        syn::UseTree::Group(group) => group
            .items
            .iter()
            .flat_map(|t| collect_use_paths(t, prefix))
            .collect(),
        syn::UseTree::Path(path) => {
            let new_prefix = format!("{}::{}", prefix, path.ident);
            collect_use_paths(&path.tree, &new_prefix)
        }
    }
}

fn load_toml<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    let src =
        fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    toml::from_str(&src).map_err(|e| format!("toml parse error in {}: {e}", path.display()))
}

// ------------------------------------------------------------------
// P1 — single supervising owner per service (AC1)
// ------------------------------------------------------------------

/// AST-scan `main.rs` and count `<Adapter>::new` call sites.
///
/// AC1: every supervised service's adapter must be constructed exactly once
/// in the composition root. The supervisor (`SpiritSchedulerAdapter`) also
/// satisfies P1 per §4.0.8.
fn check_p1_single_owner(main_rs: &Path) -> Result<Vec<Violation>, String> {
    let src = fs::read_to_string(main_rs)
        .map_err(|e| format!("cannot read {}: {e}", main_rs.display()))?;
    let ast =
        syn::parse_file(&src).map_err(|e| format!("parse error in {}: {e}", main_rs.display()))?;

    let mut visitor = P1OwnerVisitor {
        counts: BTreeMap::new(),
    };
    visitor.visit_file(&ast);

    // Inline `// p1-allow:` markers exempt a single construction site that is NOT
    // the supervised composition-root owner (e.g. a one-shot manifest-admission
    // probe inside the CLI dispatcher). The marker may sit on the construction
    // line or the line directly above it. (Story 7.1.7 baseline-reset.)
    let src_lines: Vec<&str> = src.lines().collect();
    let is_p1_allowed = |ln: usize| -> bool {
        // Check the constructor line itself.
        let on_line = src_lines
            .get(ln.saturating_sub(1))
            .map(|l| l.contains("// p1-allow:"))
            .unwrap_or(false);
        if on_line {
            return true;
        }
        // Scan backward through blank/whitespace lines to find the
        // `// p1-allow:` marker — resilient to blank-line insertion
        // between the marker and the construction site.
        let mut offset = 2;
        while ln >= offset {
            let prev = src_lines.get(ln.saturating_sub(offset));
            match prev {
                Some(l) if l.trim().is_empty() => {
                    offset += 1;
                    continue;
                }
                Some(l) => return l.contains("// p1-allow:"),
                None => break,
            }
        }
        false
    };

    let mut violations = Vec::new();
    for (_, adapter) in SERVICE_ADAPTERS {
        let lines: Vec<usize> = visitor
            .counts
            .get(*adapter)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|&ln| !is_p1_allowed(ln))
            .collect();
        if lines.len() > 1 {
            violations.push(Violation {
                file: main_rs.display().to_string(),
                line: lines[1],
                path: (*adapter).to_string(),
                message: format!(
                    "P1 violation: {} constructed N={} times in {}; expected exactly 1 (single owner per §4.0.8 supervision-tree analysis)",
                    adapter,
                    lines.len(),
                    main_rs.display()
                ),
            });
        }
    }
    Ok(violations)
}

struct P1OwnerVisitor {
    counts: BTreeMap<String, Vec<usize>>,
}

impl<'ast> syn::visit::Visit<'ast> for P1OwnerVisitor {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        // `smoke_*` functions are standalone CLI smoke-subcommand handlers — each
        // is its own isolated mini-root that legitimately builds a throwaway
        // adapter for one scenario. They are NOT the production composition root,
        // so their constructions must not count toward single-owner P1.
        // (Story 7.1.7 baseline-reset: gate-correctness fix.)
        //
        // NOTE: This skip affects ONLY the P1OwnerVisitor. Future visitor
        // extensions should be added in separate visitor structs or `impl`
        // blocks — not appended to this one — so they are not blocked by this
        // smoke-function subtree skip.
        if node.sig.ident.to_string().starts_with("smoke_") {
            return;
        }
        syn::visit::visit_item_fn(self, node);
    }

    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if let syn::Expr::Path(path_expr) = &*node.func {
            let path_str = quote::quote!(#path_expr).to_string().replace(' ', "");
            for (_, adapter) in SERVICE_ADAPTERS {
                let pattern = format!("{}::new", adapter);
                if path_str == pattern || path_str.ends_with(&format!("::{}", pattern)) {
                    self.counts
                        .entry((*adapter).to_string())
                        .or_default()
                        .push(node.func.span().start().line);
                }
            }
        }
        syn::visit::visit_expr_call(self, node);
    }
}

// ------------------------------------------------------------------
// P2 — ports-not-adapters at service boundary (AC2)
// ------------------------------------------------------------------

/// Assert every Adapter exported through `api::*` is paired with its Port trait
/// re-export in the corresponding service module.
fn check_p2_port_pairing(workspace_root: &Path) -> Result<Vec<Violation>, String> {
    let api_rs = workspace_root.join("crates/maos-kernel-core/src/api.rs");
    let api_rs = if api_rs.exists() {
        api_rs
    } else {
        workspace_root.join("src/api.rs")
    };

    if !api_rs.exists() {
        return Ok(Vec::new());
    }

    let src = fs::read_to_string(&api_rs)
        .map_err(|e| format!("cannot read {}: {e}", api_rs.display()))?;
    let ast =
        syn::parse_file(&src).map_err(|e| format!("parse error in {}: {e}", api_rs.display()))?;

    let mut adapters = Vec::new();
    for item in &ast.items {
        if let syn::Item::Use(i) = item {
            if is_pub(&i.vis) {
                adapters.extend(extract_adapter_uses(&i.tree));
            }
        }
    }

    let mut violations = Vec::new();
    for (service, adapter) in &adapters {
        if ADAPTER_PORT_EXEMPTIONS.iter().any(|(a, _, _)| a == adapter) {
            continue;
        }

        let port_name = if adapter == "RingCryptoProvider" {
            // Story 2.3: CryptoProvider re-export now present in security/mod.rs, but
            // this name-mapping workaround is still needed because RingCryptoProvider
            // (the adapter struct) does not follow the *Adapter/*Port naming convention.
            "CryptoProvider".to_string()
        } else {
            adapter.replace("Adapter", "Port")
        };

        let service_dir = workspace_root.join(format!("crates/maos-kernel-core/src/{}", service));
        let service_dir = if service_dir.exists() {
            service_dir
        } else {
            workspace_root.join(format!("src/{}", service))
        };

        if !service_dir.exists() {
            violations.push(Violation {
                file: api_rs.display().to_string(),
                line: 1,
                path: adapter.clone(),
                message: format!(
                    "P2 violation: {} exported via api::* but service module '{}' does not exist",
                    adapter, service
                ),
            });
            continue;
        }

        let mut found = false;
        let mut rs_files = Vec::new();
        crate::fs_walk::collect_rs_files(&service_dir, &mut rs_files);
        for file in &rs_files {
            if check_port_reexport_ast(file, &port_name) {
                found = true;
                break;
            }
        }

        if !found {
            violations.push(Violation {
                file: api_rs.display().to_string(),
                line: 1,
                path: adapter.clone(),
                message: format!(
                    "P2 violation: {} exported via api::* but no {} trait re-export found in service module '{}'",
                    adapter, port_name, service
                ),
            });
        }
    }

    Ok(violations)
}

fn check_port_reexport_ast(file: &Path, port_name: &str) -> bool {
    let src = match fs::read_to_string(file) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let ast = match syn::parse_file(&src) {
        Ok(a) => a,
        Err(_) => return false,
    };
    let mut visitor = PortReexportVisitor {
        port_name,
        found: false,
    };
    visitor.visit_file(&ast);
    visitor.found
}

struct PortReexportVisitor<'a> {
    port_name: &'a str,
    found: bool,
}

impl<'ast, 'a> syn::visit::Visit<'ast> for PortReexportVisitor<'a> {
    fn visit_item_use(&mut self, node: &'ast syn::ItemUse) {
        if !self.found && is_pub(&node.vis) {
            let path_str = quote::quote!(#node).to_string().replace(' ', "");
            if path_str.contains(&format!("maos_domain::ports::{}", self.port_name)) {
                self.found = true;
            }
        }
    }
}

fn extract_adapter_uses(tree: &syn::UseTree) -> Vec<(String, String)> {
    match tree {
        syn::UseTree::Path(path) if path.ident == "crate" => extract_adapter_uses(&path.tree),
        syn::UseTree::Path(path) => {
            let service = path.ident.to_string();
            match &*path.tree {
                syn::UseTree::Name(name) => vec![(service, name.ident.to_string())],
                syn::UseTree::Group(group) => group
                    .items
                    .iter()
                    .filter_map(|t| {
                        if let syn::UseTree::Name(name) = t {
                            Some((service.clone(), name.ident.to_string()))
                        } else {
                            None
                        }
                    })
                    .collect(),
                _ => Vec::new(),
            }
        }
        syn::UseTree::Group(group) => group.items.iter().flat_map(extract_adapter_uses).collect(),
        _ => Vec::new(),
    }
}

// ------------------------------------------------------------------
// P3 — state ownership behind Arc/DashMap/RwLock/atomic (AC3)
// ------------------------------------------------------------------

/// Cross-reference to `check-empty-kernel` — the I9 walker output is the
/// authoritative oracle. P3 violations are a re-interpretation of I9
/// violations with a cross-reference message.
fn check_p3_state_ownership(workspace_root: &Path) -> Result<Vec<Violation>, String> {
    let kernel_path = if workspace_root.join("crates/maos-kernel-core/src").exists() {
        workspace_root.join("crates/maos-kernel-core")
    } else {
        workspace_root.to_path_buf()
    };

    let whitelist_path = if workspace_root.join("i9-whitelist.toml").exists() {
        workspace_root.join("i9-whitelist.toml")
    } else {
        Path::new("xtask/i9-whitelist.toml").to_path_buf()
    };

    let denylist_path = if workspace_root.join("i9-denylist.toml").exists() {
        workspace_root.join("i9-denylist.toml")
    } else {
        Path::new("xtask/i9-denylist.toml").to_path_buf()
    };

    let exemptions_path = if workspace_root.join("i9-exemptions.md").exists() {
        workspace_root.join("i9-exemptions.md")
    } else {
        Path::new("docs/invariants/i9-exemptions.md").to_path_buf()
    };

    let report = crate::check_empty_kernel::run_silent(
        &kernel_path,
        &whitelist_path,
        &denylist_path,
        &exemptions_path,
    )?;

    let mut violations = Vec::new();
    for v in report.violations {
        violations.push(Violation {
            file: v.file,
            line: v.line,
            path: format!("{}::{}", v.struct_name, v.field_name),
            message: format!(
                "P3 violation: {}.{}: {}; see check-empty-kernel for full I9 context",
                v.struct_name, v.field_name, v.field_type
            ),
        });
    }
    Ok(violations)
}

// ------------------------------------------------------------------
// P4 — audit-chain integrity via call-graph reachability (AC4)
// ------------------------------------------------------------------

#[derive(Debug, serde::Deserialize)]
struct P4DenylistConfig {
    #[serde(rename = "denylist")]
    section: P4DenylistSection,
}

#[derive(Debug, serde::Deserialize)]
struct P4DenylistSection {
    patterns: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
struct P4ExemptionsConfig {
    #[serde(rename = "exempt")]
    section: P4ExemptionsSection,
}

#[derive(Debug, serde::Deserialize)]
struct P4ExemptionsSection {
    paths: Vec<String>,
}

/// AST-scan every `pub fn` reachable from the kernel-core api surface and
/// assert the function body does not call any denylisted external-I/O entry
/// point outside the mediated-lane exemption paths.
fn check_p4_audit_chain(
    workspace_root: &Path,
    denylist_path: &Path,
    exemptions_path: &Path,
) -> Result<Vec<Violation>, String> {
    if !denylist_path.exists() {
        return Ok(Vec::new());
    }

    let denylist: P4DenylistConfig = load_toml(denylist_path)?;
    let exemptions: P4ExemptionsConfig = if exemptions_path.exists() {
        load_toml(exemptions_path)?
    } else {
        P4ExemptionsConfig {
            section: P4ExemptionsSection { paths: Vec::new() },
        }
    };

    let kernel_src = workspace_root.join("crates/maos-kernel-core/src");
    let kernel_src = if kernel_src.exists() {
        kernel_src
    } else {
        workspace_root.join("src")
    };

    let mut violations = Vec::new();
    let lib_rs = kernel_src.join("lib.rs");
    let mod_path = if kernel_src.exists()
        && kernel_src
            .parent()
            .map(|p| p.file_name() == Some(std::ffi::OsStr::new("maos-kernel-core")))
            .unwrap_or(false)
    {
        "maos_kernel_core"
    } else {
        "fixture"
    };

    if lib_rs.exists() {
        walk_p4_mod(
            &lib_rs,
            &kernel_src,
            mod_path,
            &denylist.section.patterns,
            &exemptions.section.paths,
            &mut violations,
        )?;
    }

    Ok(violations)
}

fn walk_p4_mod(
    file: &Path,
    src_dir: &Path,
    mod_path: &str,
    denylist: &[String],
    exemptions: &[String],
    violations: &mut Vec<Violation>,
) -> Result<(), String> {
    let src =
        fs::read_to_string(file).map_err(|e| format!("cannot read {}: {e}", file.display()))?;
    let ast =
        syn::parse_file(&src).map_err(|e| format!("parse error in {}: {e}", file.display()))?;
    let file_str = file.display().to_string();

    for item in &ast.items {
        match item {
            syn::Item::Fn(i) if is_pub(&i.vis) => {
                let fn_path = format!("{}::{}", mod_path, i.sig.ident);
                let mut visitor = P4BodyVisitor {
                    file: file_str.clone(),
                    fn_path,
                    denylist,
                    exemptions,
                    violations,
                };
                visitor.visit_block(&i.block);
            }
            syn::Item::Impl(i) => {
                let self_ty = match &*i.self_ty {
                    syn::Type::Path(tp) => tp
                        .path
                        .segments
                        .last()
                        .map(|s| s.ident.to_string())
                        .unwrap_or_default(),
                    _ => String::new(),
                };
                for impl_item in &i.items {
                    if let syn::ImplItem::Fn(method) = impl_item {
                        if is_pub(&method.vis) {
                            let fn_path = if self_ty.is_empty() {
                                format!("{}::{}", mod_path, method.sig.ident)
                            } else {
                                format!("{}::{}::{}", mod_path, self_ty, method.sig.ident)
                            };
                            let mut visitor = P4BodyVisitor {
                                file: file_str.clone(),
                                fn_path,
                                denylist,
                                exemptions,
                                violations,
                            };
                            visitor.visit_block(&method.block);
                        }
                    }
                }
            }
            syn::Item::Mod(i) => {
                if i.attrs.iter().any(|a| {
                    a.meta.path().is_ident("cfg")
                        && a.meta
                            .require_list()
                            .map_or(false, |ml| ml.tokens.to_string().contains("test"))
                }) {
                    continue;
                }
                let child_path = format!("{}::{}", mod_path, i.ident);
                if let Some((_, content)) = &i.content {
                    for child in content {
                        walk_p4_inline_item(
                            child,
                            &child_path,
                            &file_str,
                            denylist,
                            exemptions,
                            violations,
                        )?;
                    }
                } else {
                    let parent = file.parent().unwrap_or(src_dir);
                    let child_name = i.ident.to_string();
                    let child_file = parent.join(format!("{}.rs", child_name));
                    let child_mod_dir = parent.join(&child_name).join("mod.rs");
                    if child_file.exists() {
                        walk_p4_mod(
                            &child_file,
                            src_dir,
                            &child_path,
                            denylist,
                            exemptions,
                            violations,
                        )?;
                    } else if child_mod_dir.exists() {
                        walk_p4_mod(
                            &child_mod_dir,
                            src_dir,
                            &child_path,
                            denylist,
                            exemptions,
                            violations,
                        )?;
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn walk_p4_inline_item(
    item: &syn::Item,
    mod_path: &str,
    file: &str,
    denylist: &[String],
    exemptions: &[String],
    violations: &mut Vec<Violation>,
) -> Result<(), String> {
    match item {
        syn::Item::Fn(i) if is_pub(&i.vis) => {
            let fn_path = format!("{}::{}", mod_path, i.sig.ident);
            let mut visitor = P4BodyVisitor {
                file: file.to_string(),
                fn_path,
                denylist,
                exemptions,
                violations,
            };
            visitor.visit_block(&i.block);
        }
        syn::Item::Impl(i) => {
            let self_ty = match &*i.self_ty {
                syn::Type::Path(tp) => tp
                    .path
                    .segments
                    .last()
                    .map(|s| s.ident.to_string())
                    .unwrap_or_default(),
                _ => String::new(),
            };
            for impl_item in &i.items {
                if let syn::ImplItem::Fn(method) = impl_item {
                    if is_pub(&method.vis) {
                        let fn_path = if self_ty.is_empty() {
                            format!("{}::{}", mod_path, method.sig.ident)
                        } else {
                            format!("{}::{}::{}", mod_path, self_ty, method.sig.ident)
                        };
                        let mut visitor = P4BodyVisitor {
                            file: file.to_string(),
                            fn_path,
                            denylist,
                            exemptions,
                            violations,
                        };
                        visitor.visit_block(&method.block);
                    }
                }
            }
        }
        syn::Item::Mod(i) => {
            if i.attrs.iter().any(|a| {
                a.meta.path().is_ident("cfg")
                    && a.meta
                        .require_list()
                        .map_or(false, |ml| ml.tokens.to_string().contains("test"))
            }) {
                return Ok(());
            }
            if let Some((_, content)) = &i.content {
                let child_path = format!("{}::{}", mod_path, i.ident);
                for child in content {
                    walk_p4_inline_item(
                        child,
                        &child_path,
                        file,
                        denylist,
                        exemptions,
                        violations,
                    )?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

struct P4BodyVisitor<'a> {
    file: String,
    fn_path: String,
    denylist: &'a [String],
    exemptions: &'a [String],
    violations: &'a mut Vec<Violation>,
}

impl<'a, 'ast> syn::visit::Visit<'ast> for P4BodyVisitor<'a> {
    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if let syn::Expr::Path(path_expr) = &*node.func {
            let path_str = quote::quote!(#path_expr).to_string().replace(' ', "");
            for pattern in self.denylist {
                if path_str == *pattern || path_str.starts_with(&format!("{}::", pattern)) {
                    if !is_file_exempt(&self.file, self.exemptions) {
                        self.violations.push(Violation {
                            file: self.file.clone(),
                            line: node.func.span().start().line,
                            path: self.fn_path.clone(),
                            message: format!(
                                "P4 violation: {} calls {} outside the mediated I/O lane; see xtask/p4-mediated-io-paths.toml for exempt lanes",
                                self.fn_path, pattern
                            ),
                        });
                    }
                    break;
                }
            }
        }
        syn::visit::visit_expr_call(self, node);
    }
}

fn is_file_exempt(file: &str, exemptions: &[String]) -> bool {
    exemptions.iter().any(|e| {
        let e_norm = e.trim_end_matches('/');
        file == e_norm || file.starts_with(&format!("{e_norm}/"))
    })
}

// ------------------------------------------------------------------
// Spirit ABI type reflection (AC5)
// ------------------------------------------------------------------

/// Load the expected Spirit-trait hook count from `xtask/spirit-abi-hook-count.toml`.
/// Per Epic 5 retro §A4 Debt 2c: this used to be a hard-coded `11` literal; now
/// configured so future hook additions (Story 5.2's hot-swap trio, future
/// `epistemic_resolve`) don't require gate code changes.
fn load_expected_hook_count(workspace_root: &Path, config_path: Option<&Path>) -> usize {
    let resolved = config_path
        .map(|p| {
            if p.is_absolute() {
                p.to_path_buf()
            } else {
                workspace_root.join(p)
            }
        })
        .unwrap_or_else(|| workspace_root.join("xtask/spirit-abi-hook-count.toml"));

    if resolved.exists() {
        return load_hook_count_from_file(&resolved);
    }

    // The workspace-root-relative path did not resolve (e.g. --path points at
    // a fixture directory). Try the process cwd as a second explicit attempt,
    // but warn loudly so this never passes silently.
    let cwd_path = std::env::current_dir()
        .unwrap_or_default()
        .join("xtask/spirit-abi-hook-count.toml");
    if cwd_path.exists() {
        eprintln!(
            "WARNING: load_expected_hook_count: {} not found; falling back to cwd-relative {}. \
             Pass an explicit config path or run from the workspace root to silence this.",
            resolved.display(),
            cwd_path.display(),
        );
        return load_hook_count_from_file(&cwd_path);
    }

    panic!(
        "load_expected_hook_count: cannot find spirit-abi-hook-count.toml \
         (tried {} and {}). Provide an explicit --spirit-abi-hook-count path \
         or run from the workspace root.",
        resolved.display(),
        cwd_path.display(),
    );
}

fn load_hook_count_from_file(path: &Path) -> usize {
    let content = fs::read_to_string(path).unwrap_or_else(|e| {
        panic!(
            "load_expected_hook_count: cannot read {}: {e}",
            path.display()
        )
    });
    let toml: toml::Value = content.parse().unwrap_or_else(|e| {
        panic!(
            "load_expected_hook_count: invalid TOML in {}: {e}",
            path.display()
        )
    });
    toml.get("expected_count")
        .and_then(|v| v.as_integer())
        .map(|n| n as usize)
        .unwrap_or_else(|| {
            panic!(
                "load_expected_hook_count: missing `expected_count` key in {}",
                path.display(),
            )
        })
}

/// AST-walk `maos-spirit-abi/src/lifecycle.rs` + `maos-spirit-derive/src/lib.rs`
/// and assert trait method count, vtable fields, #[repr(C)], HOOK_NAMES, and
/// count_hooks!() all agree on the expected hook count loaded from
/// `xtask/spirit-abi-hook-count.toml`.
fn check_spirit_abi_types(
    workspace_root: &Path,
    lifecycle_path: &Path,
    derive_path: &Path,
) -> Result<(Vec<Violation>, serde_json::Value), String> {
    let expected_hooks = load_expected_hook_count(workspace_root, None);
    let mut violations = Vec::new();

    let lifecycle_file = if lifecycle_path.exists() {
        lifecycle_path.to_path_buf()
    } else {
        workspace_root.join(lifecycle_path)
    };
    let derive_file = if derive_path.exists() {
        derive_path.to_path_buf()
    } else {
        workspace_root.join(derive_path)
    };

    if !lifecycle_file.exists() || !derive_file.exists() {
        return Ok((
            Vec::new(),
            serde_json::json!({
                "trait_method_count": 0,
                "vtable_field_count": 0,
                "hook_names_match": true,
                "repr_c_present": false,
                "count_hooks_macro_matches": true,
            }),
        ));
    }

    let lifecycle_src = fs::read_to_string(&lifecycle_file)
        .map_err(|e| format!("cannot read {}: {e}", lifecycle_file.display()))?;
    let lifecycle_ast = syn::parse_file(&lifecycle_src)
        .map_err(|e| format!("parse error in {}: {e}", lifecycle_file.display()))?;

    let derive_src = fs::read_to_string(&derive_file)
        .map_err(|e| format!("cannot read {}: {e}", derive_file.display()))?;
    let derive_ast = syn::parse_file(&derive_src)
        .map_err(|e| format!("parse error in {}: {e}", derive_file.display()))?;

    let mut trait_methods = BTreeSet::new();
    let mut vtable_fields = BTreeSet::new();
    let mut repr_c_present = false;
    let mut count_hooks_literal = 0usize;

    for item in &lifecycle_ast.items {
        match item {
            syn::Item::Trait(t) if t.ident == "Spirit" => {
                for trait_item in &t.items {
                    if let syn::TraitItem::Fn(f) = trait_item {
                        trait_methods.insert(f.sig.ident.to_string());
                    }
                }
            }
            syn::Item::Struct(s) if s.ident == "SpiritVtable" => {
                for attr in &s.attrs {
                    if attr.path().is_ident("repr") {
                        if let Ok(meta) = attr.meta.require_list() {
                            let tokens_str = meta.tokens.to_string().replace(' ', "");
                            if tokens_str == "C" {
                                repr_c_present = true;
                            }
                        }
                    }
                }
                for field in &s.fields {
                    if let Some(ident) = &field.ident {
                        if ident != "_phantom" {
                            vtable_fields.insert(ident.to_string());
                        }
                    }
                }
            }
            syn::Item::Macro(m)
                if m.ident
                    .as_ref()
                    .map(|i| i == "count_hooks")
                    .unwrap_or(false) =>
            {
                let tokens = m.mac.tokens.to_string();
                for part in tokens.split(|c: char| !c.is_ascii_digit()) {
                    if !part.is_empty() {
                        if let Ok(val) = part.parse::<usize>() {
                            count_hooks_literal = val;
                            break;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    let mut hook_names = BTreeSet::new();
    for item in &derive_ast.items {
        if let syn::Item::Const(c) = item {
            if c.ident == "HOOK_NAMES" {
                let expr = match &*c.expr {
                    syn::Expr::Reference(r) => &*r.expr,
                    other => other,
                };
                if let syn::Expr::Array(arr) = expr {
                    for elem in &arr.elems {
                        if let syn::Expr::Lit(syn::ExprLit {
                            lit: syn::Lit::Str(lit_str),
                            ..
                        }) = elem
                        {
                            hook_names.insert(lit_str.value());
                        }
                    }
                }
            }
        }
    }

    if trait_methods.len() != expected_hooks {
        violations.push(Violation {
            file: lifecycle_file.display().to_string(),
            line: 1,
            path: "Spirit".into(),
            message: format!(
                "spirit-ABI-drift: Spirit trait has {} methods but xtask/spirit-abi-hook-count.toml expects {}",
                trait_methods.len(),
                expected_hooks,
            ),
        });
    }

    if vtable_fields.len() != expected_hooks {
        violations.push(Violation {
            file: lifecycle_file.display().to_string(),
            line: 1,
            path: "SpiritVtable".into(),
            message: format!(
                "spirit-ABI-drift: SpiritVtable has {} hook fields but expected {} (excluding _phantom)",
                vtable_fields.len(),
                expected_hooks,
            ),
        });
    }

    if trait_methods != vtable_fields {
        let diff: Vec<_> = trait_methods.symmetric_difference(&vtable_fields).collect();
        violations.push(Violation {
            file: lifecycle_file.display().to_string(),
            line: 1,
            path: "Spirit/SpiritVtable".into(),
            message: format!(
                "spirit-ABI-drift: trait method names do not match vtable fields: {:?}",
                diff
            ),
        });
    }

    if trait_methods != hook_names {
        let diff: Vec<_> = trait_methods.symmetric_difference(&hook_names).collect();
        violations.push(Violation {
            file: derive_file.display().to_string(),
            line: 1,
            path: "HOOK_NAMES".into(),
            message: format!(
                "spirit-ABI-drift: trait method names do not match HOOK_NAMES array: {:?}",
                diff
            ),
        });
    }

    if !repr_c_present {
        violations.push(Violation {
            file: lifecycle_file.display().to_string(),
            line: 1,
            path: "SpiritVtable".into(),
            message: "spirit-ABI-drift: SpiritVtable missing #[repr(C)] attribute".into(),
        });
    }

    if count_hooks_literal != expected_hooks {
        violations.push(Violation {
            file: lifecycle_file.display().to_string(),
            line: 1,
            path: "count_hooks!".into(),
            message: format!(
                "spirit-ABI-drift: count_hooks!() expands to {} but expected {}",
                count_hooks_literal, expected_hooks,
            ),
        });
    }

    let json = serde_json::json!({
        "trait_method_count": trait_methods.len(),
        "vtable_field_count": vtable_fields.len(),
        "hook_names_match": trait_methods == hook_names,
        "repr_c_present": repr_c_present,
        "count_hooks_macro_matches": count_hooks_literal == expected_hooks,
        "expected_hook_count": expected_hooks,
    });

    Ok((violations, json))
}

// ------------------------------------------------------------------
// Per-service per-property status payload
// ------------------------------------------------------------------

fn p1_p4_status_payload(violations: &[Violation]) -> serde_json::Value {
    let p1_uniform = "enforced";
    let p2_uniform = "enforced";
    let p3_uniform = if violations
        .iter()
        .any(|v| v.message.starts_with("P3 violation:"))
    {
        "violated"
    } else {
        "enforced"
    };
    let p4_uniform = if violations
        .iter()
        .any(|v| v.message.starts_with("P4 violation:"))
    {
        "violated"
    } else {
        "enforced"
    };

    let mut per_service = serde_json::Map::new();
    for (service, adapter) in SERVICE_ADAPTERS {
        let p1 = if violations
            .iter()
            .any(|v| v.message.starts_with("P1 violation:") && v.path == *adapter)
        {
            "violated"
        } else {
            p1_uniform
        };
        let p2 = if violations
            .iter()
            .any(|v| v.message.starts_with("P2 violation:") && v.path == *adapter)
        {
            "violated"
        } else {
            p2_uniform
        };
        let p3 = if *service == SUPERVISOR {
            "supervisor-exception"
        } else {
            p3_uniform
        };
        per_service.insert(
            (*service).to_string(),
            serde_json::json!({ "p1": p1, "p2": p2, "p3": p3, "p4": p4_uniform }),
        );
    }
    serde_json::Value::Object(per_service)
}

#[cfg(test)]
mod tests {
    include!("tests/check_service_boundary_tests.rs");
}
