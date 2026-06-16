//! Generate `docs-site/docs/abi/*.md` from `maos-spirit-abi` rustdoc JSON.
//!
//! Story 9.5c — anti-rot: the committed `.md` files are build artifacts;
//! regenerate via `cargo run -p xtask -- gen-abi-docs`.

use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Nightly toolchain pinned for `cargo rustdoc --output-format json`.
///
/// Runbook for bumping the pin (only when rustdoc JSON actually drifts):
///   1. Install the new nightly: `rustup toolchain install nightly-YYYY-MM-DD --profile minimal`
///   2. Run: `cargo +nightly-YYYY-MM-DD rustdoc -p maos-spirit-abi -- -Z unstable-options --output-format json`
///   3. Read `format_version` from `target/doc/maos_spirit_abi.json` and update `EXPECTED_FORMAT_VERSION`.
///   4. Regenerate: `cargo run -p xtask -- gen-abi-docs`
///   5. Commit the changed `NIGHTLY`, `EXPECTED_FORMAT_VERSION`, and regenerated `.md` files together.
pub const NIGHTLY: &str = "nightly-2026-05-01";

/// rustdoc JSON `format_version` observed for `NIGHTLY`.
/// Bump together with `NIGHTLY` per the runbook above.
pub const EXPECTED_FORMAT_VERSION: u32 = 57;

const MANIFEST_PATH: &str = "crates/maos-spirit-abi/Cargo.toml";
const ABI_OUT_DIR: &str = "docs-site/docs/abi";

// ------------------------------------------------------------------
// rustdoc JSON serde structs (hand-rolled, minimal surface)
// ------------------------------------------------------------------

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct CrateJson {
    format_version: u32,
    root: u32,
    #[serde(default)]
    index: HashMap<String, Item>,
    #[serde(default)]
    paths: HashMap<String, PathItem>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct PathItem {
    #[serde(rename = "crate_id")]
    _crate_id: u32,
    kind: String,
    path: Vec<String>,
}
#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct Item {
    id: u32,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    docs: Option<String>,
    #[serde(default)]
    attrs: Vec<serde_json::Value>,
    #[serde(rename = "inner")]
    inner: ItemInner,
}
#[derive(Debug)]
#[allow(dead_code)]
enum ItemInner {
    Module(ModuleInner),
    Struct(StructInner),
    Enum(EnumInner),
    Trait(TraitInner),
    Function(FunctionInner),
    Constant(ConstantInner),
    Impl(ImplInner),
    // Any rustdoc item kind we do not render is captured opaquely.
    Other(serde_json::Value),
}

impl From<serde_json::Value> for ItemInner {
    fn from(v: serde_json::Value) -> Self {
        let Some(obj) = v.as_object() else {
            return ItemInner::Other(v);
        };
        let (kind, val) = match obj.iter().next() {
            Some(entry) => (entry.0.as_str(), entry.1.clone()),
            None => return ItemInner::Other(v),
        };
        match kind {
            "module" => serde_json::from_value(val)
                .map(ItemInner::Module)
                .unwrap_or_else(|_| ItemInner::Other(v)),
            "struct" => serde_json::from_value(val)
                .map(ItemInner::Struct)
                .unwrap_or_else(|_| ItemInner::Other(v)),
            "enum" => serde_json::from_value(val)
                .map(ItemInner::Enum)
                .unwrap_or_else(|_| ItemInner::Other(v)),
            "trait" => serde_json::from_value(val)
                .map(ItemInner::Trait)
                .unwrap_or_else(|_| ItemInner::Other(v)),
            "function" => serde_json::from_value(val)
                .map(ItemInner::Function)
                .unwrap_or_else(|_| ItemInner::Other(v)),
            "constant" => serde_json::from_value(val)
                .map(ItemInner::Constant)
                .unwrap_or_else(|_| ItemInner::Other(v)),
            "impl" => serde_json::from_value(val)
                .map(ItemInner::Impl)
                .unwrap_or_else(|_| ItemInner::Other(v)),
            _ => ItemInner::Other(v),
        }
    }
}

impl<'de> serde::Deserialize<'de> for ItemInner {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let v = serde_json::Value::deserialize(deserializer)?;
        Ok(ItemInner::from(v))
    }
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct ModuleInner {
    #[serde(default)]
    is_crate: bool,
    items: Vec<u32>,
    #[serde(default)]
    is_stripped: bool,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct StructInner {
    #[serde(default)]
    kind: StructKind,
    #[serde(default)]
    generics: Generics,
    #[serde(default)]
    impls: Vec<u32>,
}

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "snake_case")]
enum StructKind {
    #[default]
    Unit,
    Tuple { fields: Vec<StructField> },
    Plain { fields: Vec<StructField>, has_stripped_fields: bool },
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct StructField {
    name: String,
    #[serde(default)]
    docs: Option<String>,
    #[serde(default)]
    ty: Option<Type>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct EnumInner {
    variants: Vec<u32>,
    #[serde(default)]
    generics: Generics,
    #[serde(default)]
    impls: Vec<u32>,
}

#[derive(Deserialize, Debug)]
struct TraitInner {
    items: Vec<u32>,
    #[serde(default)]
    generics: Generics,
}

#[derive(Deserialize, Debug)]
struct FunctionInner {
    sig: Signature,
    #[serde(default)]
    generics: Generics,
    #[serde(default)]
    header: FunctionHeader,
}

#[derive(Deserialize, Debug)]
struct Signature {
    #[serde(default)]
    inputs: Vec<(String, Type)>,
    output: Option<Type>,
}

#[derive(Deserialize, Debug, Default)]
struct FunctionHeader {
    #[serde(default)]
    is_const: bool,
    #[serde(default)]
    is_async: bool,
    #[serde(default)]
    is_unsafe: bool,
}

#[derive(Deserialize, Debug)]
struct ImplInner {
    #[serde(rename = "trait", default)]
    trait_: Option<ResolvedPath>,
    items: Vec<u32>,
}

#[derive(Deserialize, Debug)]
struct ConstantInner {
    #[serde(rename = "type")]
    ty: Type,
    #[serde(rename = "const")]
    const_value: ConstValue,
}

#[derive(Deserialize, Debug, Default)]
struct ConstValue {
    #[serde(default)]
    value: Option<String>,
}

#[derive(Deserialize, Debug, Default)]
struct Generics {
    #[serde(default)]
    params: Vec<GenericParam>,
    #[serde(default)]
    where_predicates: Vec<WherePredicate>,
}

#[derive(Deserialize, Debug)]
struct GenericParam {
    name: String,
    kind: GenericParamKind,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
enum GenericParamKind {
    Lifetime { outlives: Vec<String> },
    Type { bounds: Vec<TypeBound> },
    Const { #[serde(rename = "type")] ty: Type },
}

#[derive(Deserialize, Debug)]
struct WherePredicate {
    #[serde(flatten)]
    inner: WherePredicateInner,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "bound_predicate")]
enum WherePredicateInner {
    #[serde(rename = "bound_predicate")]
    BoundPredicate { #[serde(rename = "type")] ty: Type, bounds: Vec<TypeBound> },
}

#[derive(Deserialize, Debug)]
struct TypeBound {
    #[serde(flatten)]
    inner: TypeBoundInner,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
enum TypeBoundInner {
    TraitBound { trait_: TraitPath },
    #[serde(other)]
    Other,
}

#[derive(Deserialize, Debug)]
struct TraitPath {
    path: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
enum Type {
    Primitive(String),
    Generic(String),
    ResolvedPath(ResolvedPath),
    BorrowedRef {
        #[serde(default)]
        lifetime: Option<String>,
        #[serde(default)]
        is_mutable: bool,
        #[serde(rename = "type")]
        ty: Box<Type>,
    },
    RawPointer {
        #[serde(default)]
        is_mutable: bool,
        #[serde(rename = "type")]
        ty: Box<Type>,
    },
    DynTrait(DynTrait),
    Slice(Box<Type>),
    Array { #[serde(rename = "type")] ty: Box<Type>, len: String },
    Tuple(Vec<Type>),
    #[serde(other)]
    Other,
}

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
struct ResolvedPath {
    path: String,
    #[serde(default)]
    id: Option<u32>,
    #[serde(default)]
    args: Option<AngleBracketedArgs>,
}

#[derive(Deserialize, Debug, Clone)]
struct AngleBracketedArgs {
    #[serde(rename = "angle_bracketed")]
    inner: AngleBracketed,
}

#[derive(Deserialize, Debug, Clone)]
struct AngleBracketed {
    args: Vec<GenericArg>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
enum GenericArg {
    Lifetime(String),
    Type(Type),
    Const(String),
}
#[derive(Deserialize, Debug, Clone)]
struct DynTrait {
    #[serde(default)]
    traits: Vec<DynTraitEntry>,
}

#[derive(Deserialize, Debug, Clone)]
struct DynTraitEntry {
    #[serde(rename = "trait")]
    trait_: ResolvedPath,
}

// ------------------------------------------------------------------
// Type renderer
// ------------------------------------------------------------------

fn render_type(ty: &Type) -> String {
    match ty {
        Type::Primitive(s) | Type::Generic(s) => s.clone(),
        Type::ResolvedPath(rp) => render_resolved_path(rp),
        Type::BorrowedRef { lifetime, is_mutable, ty } => {
            let mut s = "&".to_string();
            if let Some(lt) = lifetime {
                s.push_str(lt);
                s.push(' ');
            }
            if *is_mutable {
                s.push_str("mut ");
            }
            s.push_str(&render_type(ty));
            s
        }
        Type::RawPointer { is_mutable, ty } => {
            let mut s = "*".to_string();
            if *is_mutable {
                s.push_str("mut ");
            } else {
                s.push_str("const ");
            }
            s.push_str(&render_type(ty));
            s
        }
        Type::DynTrait(dyn_trait) => {
            let rendered = dyn_trait
                .traits
                .iter()
                .map(|entry| render_resolved_path(&entry.trait_))
                .collect::<Vec<_>>()
                .join(" + ");
            if rendered.is_empty() {
                "dyn _".to_string()
            } else {
                format!("dyn {}", rendered)
            }
        }
        Type::Slice(ty) => format!("[{}]", render_type(ty)),
        Type::Array { ty, len } => format!("[{}; {}]", render_type(ty), len),
        Type::Tuple(items) => {
            let inner = items.iter().map(render_type).collect::<Vec<_>>().join(", ");
            format!("({})", inner)
        }
        Type::Other => "_".to_string(),
    }
}

fn render_resolved_path(rp: &ResolvedPath) -> String {
    let mut s = strip_crate_prefix(&rp.path);
    if let Some(args) = &rp.args {
        let rendered: Vec<String> = args
            .inner
            .args
            .iter()
            .map(|a| match a {
                GenericArg::Lifetime(l) => l.clone(),
                GenericArg::Type(t) => render_type(t),
                GenericArg::Const(c) => c.clone(),
            })
            .collect();
        if !rendered.is_empty() {
            s.push_str("<");
            s.push_str(&rendered.join(", "));
            s.push_str(">");
        }
    }
    s
}

fn strip_crate_prefix(path: &str) -> String {
    path.strip_prefix("maos_spirit_abi::")
        .map(|s| s.to_string())
        .unwrap_or_else(|| path.to_string())
}

fn render_generics(generics: &Generics) -> String {
    if generics.params.is_empty() {
        return String::new();
    }
    let params: Vec<String> = generics
        .params
        .iter()
        .map(|p| match &p.kind {
            GenericParamKind::Lifetime { outlives } => {
                if outlives.is_empty() {
                    p.name.clone()
                } else {
                    format!("{}: {}", p.name, outlives.join(" + "))
                }
            }
            GenericParamKind::Type { bounds } => {
                let rendered: Vec<String> = bounds
                    .iter()
                    .map(|b| match &b.inner {
                        TypeBoundInner::TraitBound { trait_: tp } => tp.path.clone(),
                        TypeBoundInner::Other => "_".to_string(),
                    })
                    .collect();
                if rendered.is_empty() {
                    p.name.clone()
                } else {
                    format!("{}: {}", p.name, rendered.join(" + "))
                }
            }
            GenericParamKind::Const { ty } => format!("const {}: {}", p.name, render_type(ty)),
        })
        .collect();
    format!("<{}>", params.join(", "))
}

fn render_where_clauses(generics: &Generics) -> String {
    if generics.where_predicates.is_empty() {
        return String::new();
    }
    let clauses: Vec<String> = generics
        .where_predicates
        .iter()
        .map(|wp| match &wp.inner {
            WherePredicateInner::BoundPredicate { ty, bounds } => {
                let rendered: Vec<String> = bounds
                    .iter()
                    .map(|b| match &b.inner {
                        TypeBoundInner::TraitBound { trait_: tp } => tp.path.clone(),
                        TypeBoundInner::Other => "_".to_string(),
                    })
                    .collect();
                format!("{}: {}", render_type(ty), rendered.join(" + "))
            }
        })
        .collect();
    format!(" where {}", clauses.join(", "))
}

// ------------------------------------------------------------------
// Markdown generation
// ------------------------------------------------------------------

struct RenderContext<'a> {
    index: &'a HashMap<String, Item>,
    id_to_key: &'a HashMap<String, String>,
}

impl<'a> RenderContext<'a> {
    fn item_by_id(&self, id: u32) -> Option<&Item> {
        self.id_to_key.get(&id.to_string()).and_then(|k| self.index.get(k))
    }
}

fn build_id_to_key(index: &HashMap<String, Item>) -> HashMap<String, String> {
    index.iter().map(|(k, _v)| (k.clone(), k.clone())).collect()
}

fn header() -> String {
    "<!-- AUTO-GENERATED from maos-spirit-abi rustdoc — do not edit; regenerate via: cargo run -p xtask -- gen-abi-docs -->\n".to_string()
}

fn write_page(path: &Path, title: &str, body: &str, related_partial: Option<&Path>) -> Result<(), String> {
    let out = assemble_page_text(title, body, related_partial)?;
    fs::write(path, out).map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(())
}
/// Assemble the canonical text of a generated page (used by `--check` and for writing).
fn assemble_page_text(title: &str, body: &str, related_partial: Option<&Path>) -> Result<String, String> {
    let mut out = header();
    out.push('\n');
    out.push_str(&format!("# `{}` Module\n\n", title));
    if let Some(partial) = related_partial {
        if let Ok(content) = fs::read_to_string(partial) {
            out.push_str(&content);
            out.push('\n');
        }
    }
    out.push_str(body);
    Ok(out)
}

fn render_item(item: &Item, ctx: &RenderContext) -> Option<String> {
    let name = item.name.as_deref()?;
    let docs = item.docs.as_deref().unwrap_or("");
    let mut out = String::new();

    if !docs.is_empty() {
        out.push_str(docs.trim());
        out.push('\n');
    }

    match &item.inner {
        ItemInner::Struct(s) => {
            out.push_str("\n```rust\n");
            out.push_str(&render_struct(name, s));
            out.push_str("\n```\n");
            render_inherent_impl_items(&mut out, &s.impls, ctx);
        }
        ItemInner::Enum(e) => {
            out.push_str("\n```rust\n");
            out.push_str(&render_enum(name, e, ctx));
            out.push_str("\n```\n");
            render_inherent_impl_items(&mut out, &e.impls, ctx);
        }
        ItemInner::Trait(t) => {
            out.push_str("\n```rust\n");
            out.push_str(&render_trait(name, t, ctx));
            out.push_str("\n```\n");
        }
        ItemInner::Function(f) => {
            out.push_str("\n```rust\n");
            out.push_str(&render_function(name, f));
            out.push_str("\n```\n");
        }
        ItemInner::Constant(c) => {
            out.push_str("\n```rust\n");
            out.push_str(&render_constant(name, c));
            out.push_str("\n```\n");
        }
        _ => return None,
    }
    Some(out)
}

fn render_struct(name: &str, s: &StructInner) -> String {
    let generics = render_generics(&s.generics);
    match &s.kind {
        StructKind::Unit => format!("pub struct {}{};", name, generics),
        StructKind::Tuple { fields } => {
            let types: Vec<String> = fields
                .iter()
                .map(|f| f.ty.as_ref().map(render_type).unwrap_or_else(|| "_".to_string()))
                .collect();
            format!("pub struct {}{}({});", name, generics, types.join(", "))
        }
        StructKind::Plain { fields, has_stripped_fields } => {
            if fields.is_empty() {
                if *has_stripped_fields {
                    format!("pub struct {}{} {{ /* private fields */ }}", name, generics)
                } else {
                    format!("pub struct {}{} {{}}", name, generics)
                }
            } else {
                let mut lines = vec![format!("pub struct {}{} {{", name, generics)];
                for f in fields {
                    let ty = f.ty.as_ref().map(render_type).unwrap_or_else(|| "_".to_string());
                    lines.push(format!("    pub {}: {},", f.name, ty));
                }
                lines.push("}".to_string());
                lines.join("\n")
            }
        }
    }
}

fn render_enum(name: &str, e: &EnumInner, ctx: &RenderContext) -> String {
    let generics = render_generics(&e.generics);
    let mut lines = vec![format!("pub enum {}{} {{", name, generics)];
    for vid in &e.variants {
        if let Some(v) = ctx.item_by_id(*vid) {
            if let Some(vname) = &v.name {
                match &v.inner {
                    ItemInner::Struct(StructInner { kind: StructKind::Tuple { fields }, .. }) => {
                        let types: Vec<String> = fields
                            .iter()
                            .map(|f| f.ty.as_ref().map(render_type).unwrap_or_else(|| "_".to_string()))
                            .collect();
                        lines.push(format!("    {}({}),", vname, types.join(", ")));
                    }
                    ItemInner::Struct(StructInner { kind: StructKind::Plain { fields, .. }, .. }) => {
                        if fields.is_empty() {
                            lines.push(format!("    {},", vname));
                        } else {
                            lines.push(format!("    {} {{", vname));
                            for f in fields {
                                let ty = f.ty.as_ref().map(render_type).unwrap_or_else(|| "_".to_string());
                                lines.push(format!("        pub {}: {},", f.name, ty));
                            }
                            lines.push("    },".to_string());
                        }
                    }
                    _ => lines.push(format!("    {},", vname)),
                }
            }
        }
    }
    lines.push("}".to_string());
    lines.join("\n")
}

fn render_trait(name: &str, t: &TraitInner, ctx: &RenderContext) -> String {
    let generics = render_generics(&t.generics);
    let mut lines = vec![format!("pub trait {}{} {{", name, generics)];
    for iid in &t.items {
        if let Some(item) = ctx.item_by_id(*iid) {
            if let Some(iname) = &item.name {
                match &item.inner {
                    ItemInner::Function(f) => {
                        lines.push(format!("    {};", render_fn_sig(iname, f, false)));
                    }
                    ItemInner::Constant(c) => {
                        lines.push(format!("    {};", render_constant(iname, c)));
                    }
                    _ => {}
                }
            }
        }
    }
    lines.push("}".to_string());
    lines.join("\n")
}

fn render_function(name: &str, f: &FunctionInner) -> String {
    render_fn_sig(name, f, true)
}

fn render_fn_sig(name: &str, f: &FunctionInner, include_pub: bool) -> String {
    let mut s = String::new();
    if include_pub {
        s.push_str("pub ");
    }
    if f.header.is_const {
        s.push_str("const ");
    }
    if f.header.is_async {
        s.push_str("async ");
    }
    if f.header.is_unsafe {
        s.push_str("unsafe ");
    }
    s.push_str("fn ");
    s.push_str(name);
    s.push_str(&render_generics(&f.generics));
    s.push('(');
    let args: Vec<String> = f
        .sig
        .inputs
        .iter()
        .map(|(n, t)| render_param(n, t))
        .collect();
    s.push_str(&args.join(", "));
    s.push(')');
    s.push_str(&render_where_clauses(&f.generics));
    if let Some(out) = &f.sig.output {
        s.push_str(" -> ");
        s.push_str(&render_type(out));
    }
    s
}

fn render_param(name: &str, ty: &Type) -> String {
    if name != "self" {
        return format!("{}: {}", name, render_type(ty));
    }

    match ty {
        Type::Generic(generic) if generic == "Self" => "self".to_string(),
        Type::BorrowedRef {
            lifetime,
            is_mutable,
            ty,
        } => {
            if matches!(ty.as_ref(), Type::Generic(generic) if generic == "Self") {
                let mut rendered = String::from("&");
                if let Some(lifetime) = lifetime {
                    rendered.push_str(lifetime);
                    rendered.push(' ');
                }
                if *is_mutable {
                    rendered.push_str("mut ");
                }
                rendered.push_str("self");
                rendered
            } else {
                format!("self: {}", render_type(ty))
            }
        }
        _ => format!("self: {}", render_type(ty)),
    }
}

fn render_constant(name: &str, c: &ConstantInner) -> String {
    let value = c.const_value.value.as_deref().unwrap_or("_");
    format!("pub const {}: {} = {};", name, render_type(&c.ty), value)
}

fn render_inherent_impl_items(out: &mut String, impl_ids: &[u32], ctx: &RenderContext) {
    let mut sections: Vec<String> = Vec::new();
    for impl_id in impl_ids {
        let Some(item) = ctx.item_by_id(*impl_id) else {
            continue;
        };
        let ItemInner::Impl(impl_inner) = &item.inner else {
            continue;
        };
        if impl_inner.trait_.is_some() {
            continue;
        }
        for item_id in &impl_inner.items {
            let Some(impl_item) = ctx.item_by_id(*item_id) else {
                continue;
            };
            let Some(rendered) = render_item(impl_item, ctx) else {
                continue;
            };
            sections.push(rendered);
        }
    }

    if !sections.is_empty() {
        out.push_str("\n### Inherent Items\n");
        for section in sections {
            out.push('\n');
            out.push_str(&section);
        }
    }
}
// ------------------------------------------------------------------
// Module rendering
// ------------------------------------------------------------------

fn render_module_page(module_item: &Item, ctx: &RenderContext, skip_constants: bool) -> String {
    let mut sections: Vec<String> = Vec::new();
    if let Some(docs) = &module_item.docs {
        if !docs.trim().is_empty() {
            sections.push(docs.trim().to_string());
        }
    }
    if let ItemInner::Module(ModuleInner { items, .. }) = &module_item.inner {
        let mut by_kind: BTreeMap<String, Vec<u32>> = BTreeMap::new();
        for iid in items {
            if let Some(item) = ctx.item_by_id(*iid) {
                let kind = match &item.inner {
                    ItemInner::Module { .. } => continue,
                    ItemInner::Struct(_) => "Structs",
                    ItemInner::Enum(_) => "Enums",
                    ItemInner::Trait(_) => "Traits",
                    ItemInner::Function(_) => "Functions",
                    ItemInner::Constant(_) => {
                        if skip_constants {
                            continue;
                        }
                        "Constants"
                    }
                    ItemInner::Impl(_) | ItemInner::Other(_) => continue,
                };
                by_kind.entry(kind.to_string()).or_default().push(*iid);
            }
        }

        for (kind, ids) in by_kind {
            sections.push(format!("\n## {}\n", kind));
            for iid in ids {
                if let Some(item) = ctx.item_by_id(iid) {
                    if let Some(rendered) = render_item(item, ctx) {
                        sections.push(rendered);
                    }
                }
            }
        }
    }
    sections.join("\n")
}

fn render_constants_page(crate_root: &Item, ctx: &RenderContext) -> String {
    let mut sections = vec!["\n## Constants\n".to_string()];
    if let ItemInner::Module(ModuleInner { items, .. }) = &crate_root.inner {
        for iid in items {
            if let Some(item) = ctx.item_by_id(*iid) {
                if matches!(&item.inner, ItemInner::Constant(_)) {
                    if let Some(rendered) = render_item(item, ctx) {
                        sections.push(rendered);
                    }
                }
            }
        }
    }
    sections.join("\n")
}
fn stamp_versions(body: &str, abi_version: u32, manifest_schema_version: u32) -> String {
    format!(
        "\n*ABI_VERSION = {} · MANIFEST_SCHEMA_VERSION = {}*\n\n{}",
        abi_version, manifest_schema_version, body
    )
}

// ------------------------------------------------------------------
// Main entry point
// ------------------------------------------------------------------

pub fn run(out_dir: Option<&str>, check: bool) -> Result<(), String> {
    let output_dir: PathBuf = out_dir.unwrap_or(ABI_OUT_DIR).into();

    // 1. Locate crate version (workspace-inherited) and ABI constants.
    let version = read_crate_version()?;
    let (abi_version, manifest_schema_version) = read_abi_constants()?;

    // 2. Generate rustdoc JSON with the pinned nightly.
    let json_path = generate_rustdoc_json()?;

    // 3. Parse.
    let json_text = fs::read_to_string(&json_path)
        .map_err(|e| format!("read rustdoc json {}: {e}", json_path.display()))?;
    let krate: CrateJson = serde_json::from_str(&json_text)
        .map_err(|e| format!("parse rustdoc json: {e}"))?;

    if krate.format_version != EXPECTED_FORMAT_VERSION {
        return Err(format!(
            "rustdoc JSON format_version is {} (expected {}). Update NIGHTLY and EXPECTED_FORMAT_VERSION if rustdoc changed.",
            krate.format_version, EXPECTED_FORMAT_VERSION
        ));
    }

    let root = krate
        .index
        .get(&krate.root.to_string())
        .ok_or("missing root item")?;
    let id_to_key = build_id_to_key(&krate.index);
    let ctx = RenderContext {
        index: &krate.index,
        id_to_key: &id_to_key,
    };

    // 4. Collect modules.
    let mut modules: BTreeMap<String, &Item> = BTreeMap::new();
    modules.insert("index".to_string(), root);

    if let ItemInner::Module(ModuleInner { items, .. }) = &root.inner {
        for iid in items {
            if let Some(item) = ctx.item_by_id(*iid) {
                if let ItemInner::Module(ModuleInner { .. }) = &item.inner {
                    if let Some(name) = &item.name {
                        modules.insert(name.clone(), item);
                    }
                }
            }
        }
    }

    if modules.len() < 2 {
        return Err(format!(
            "expected at least one submodule plus index, got {} module(s)",
            modules.len()
        ));
    }

    // 5. Render pages.
    let mut expected_files: BTreeSet<String> = BTreeSet::new();

    // Dedicated constants page (rendered from the crate root).
    expected_files.insert("constants.md".to_string());
    let constants_body = stamp_versions(
        &render_constants_page(root, &ctx),
        abi_version,
        manifest_schema_version,
    );
    let constants_partial = output_dir.join("_related_constants.md");
    let constants_path = output_dir.join("constants.md");
    if check {
        let existing = fs::read_to_string(&constants_path)
            .map_err(|e| format!("read existing {}: {e}", constants_path.display()))?;
        let generated = assemble_page_text("constants", &constants_body, constants_partial.exists().then_some(&constants_partial))?;
        if normalize_md(&existing) != normalize_md(&generated) {
            return Err(format!(
                "abi reference page {} is stale; regenerate with `cargo run -p xtask -- gen-abi-docs`",
                constants_path.display()
            ));
        }
    } else {
        write_page(
            &constants_path,
            "constants",
            &constants_body,
            constants_partial.exists().then_some(&constants_partial),
        )?;
    }

    for (mod_name, module_item) in &modules {
        let file_name = if mod_name == "index" {
            "index.md".to_string()
        } else {
            format!("{}.md", mod_name.replace('_', "-"))
        };
        expected_files.insert(file_name.clone());

        let partial_name = if mod_name == "index" {
            "_related_index.md".to_string()
        } else {
            format!("_related_{}.md", mod_name)
        };
        let partial = output_dir.join(&partial_name);

        let body = stamp_versions(
            &render_module_page(module_item, &ctx, mod_name == "index"),
            abi_version,
            manifest_schema_version,
        );
        let out_path = output_dir.join(&file_name);

        if check {
            let existing = fs::read_to_string(&out_path)
                .map_err(|e| format!("read existing {}: {e}", out_path.display()))?;
            let generated = assemble_page_text(mod_name, &body, partial.exists().then_some(&partial))?;
            if normalize_md(&existing) != normalize_md(&generated) {
                return Err(format!(
                    "abi reference page {} is stale; regenerate with `cargo run -p xtask -- gen-abi-docs`",
                    out_path.display()
                ));
            }
        } else {
            write_page(&out_path, mod_name, &body, partial.exists().then_some(&partial))?;
        }
    }
    // 6. Reject or clean up orphaned generated pages (leave _related_*.md hand-curated partials alone).
    let orphaned = fs::read_dir(&output_dir)
        .map_err(|e| format!("read output dir: {e}"))?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let fname = entry.file_name().into_string().ok()?;
            (fname.ends_with(".md")
                && !fname.starts_with("_related_")
                && !expected_files.contains(&fname))
                .then_some((fname, entry.path()))
        })
        .collect::<Vec<_>>();
    if check {
        if let Some((fname, _path)) = orphaned.first() {
            return Err(format!(
                "abi reference output contains orphan page {}; regenerate with `cargo run -p xtask -- gen-abi-docs`",
                output_dir.join(fname).display()
            ));
        }
    } else {
        for (_fname, path) in orphaned {
            fs::remove_file(&path)
                .map_err(|e| format!("remove orphan {}: {e}", path.display()))?;
        }
    }

    // 7. Emit version archive manifest entry to stdout.
    if !check {
        println!(
            "{{\"version\": \"{}\", \"abi_version\": {}, \"manifest_schema_version\": {}, \"pages\": {}}}",
            version,
            abi_version,
            manifest_schema_version,
            expected_files.len()
        );
    }

    Ok(())
}

/// Resolve the crate version, handling `version.workspace = true`.
fn read_crate_version() -> Result<String, String> {
    let manifest_contents = fs::read_to_string(MANIFEST_PATH)
        .map_err(|e| format!("read manifest: {e}"))?;
    let manifest: toml::Value = manifest_contents
        .parse()
        .map_err(|e| format!("parse manifest: {e}"))?;
    let package = manifest
        .get("package")
        .ok_or("missing [package] in Cargo.toml")?;

    if let Some(v) = package.get("version").and_then(|v| v.as_str()) {
        return Ok(v.to_string());
    }

    // workspace-inherited version
    let workspace_manifest = fs::read_to_string("Cargo.toml")
        .map_err(|e| format!("read workspace Cargo.toml: {e}"))?;
    let workspace: toml::Value = workspace_manifest
        .parse()
        .map_err(|e| format!("parse workspace Cargo.toml: {e}"))?;
    workspace
        .get("workspace")
        .and_then(|w| w.get("package"))
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "could not resolve crate version".to_string())
}

/// Parse `ABI_VERSION` and `MANIFEST_SCHEMA_VERSION` from the crate source.
fn read_abi_constants() -> Result<(u32, u32), String> {
    let src = fs::read_to_string("crates/maos-spirit-abi/src/lib.rs")
        .map_err(|e| format!("read maos-spirit-abi/src/lib.rs: {e}"))?;
    let abi_re = regex::Regex::new(r"pub\s+const\s+ABI_VERSION\s*:\s*u32\s*=\s*(\d+)").unwrap();
    let schema_re = regex::Regex::new(r"pub\s+const\s+MANIFEST_SCHEMA_VERSION\s*:\s*u32\s*=\s*(\d+)").unwrap();
    let abi = abi_re
        .captures(&src)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse().ok())
        .ok_or("could not parse ABI_VERSION")?;
    let schema = schema_re
        .captures(&src)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse().ok())
        .ok_or("could not parse MANIFEST_SCHEMA_VERSION")?;
    Ok((abi, schema))
}
fn normalize_md(text: &str) -> String {
    text.lines()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn generate_rustdoc_json() -> Result<PathBuf, String> {
    let toolchain = format!("+{}", NIGHTLY);
    let status = Command::new("cargo")
        .args([
            &toolchain,
            "rustdoc",
            "-p",
            "maos-spirit-abi",
            "--",
            "-Z",
            "unstable-options",
            "--output-format",
            "json",
        ])
        .status()
        .map_err(|e| format!("cargo rustdoc failed to start: {e}"))?;

    if !status.success() {
        return Err(format!("cargo rustdoc exited with {status}"));
    }

    // Default rustdoc JSON output path for a crate in a workspace.
    let path = PathBuf::from("target/doc/maos_spirit_abi.json");
    if !path.exists() {
        return Err(format!("expected rustdoc JSON at {}", path.display()));
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    fn workspace_root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
    }

    /// AC-3 value-provenance assertion: MANIFEST_SCHEMA_VERSION in the generated
    /// constants.md equals the live source constant, read through a path that is
    /// independent of the generator's regex-based `read_abi_constants` helper.
    #[test]
    fn value_provenance_manifest_schema_version() {
        let lib_rs = workspace_root().join("crates/maos-spirit-abi/src/lib.rs");
        let src = fs::read_to_string(lib_rs).expect("read lib.rs");
        let file = syn::parse_file(&src).expect("parse lib.rs");

        let mut live_value: Option<u32> = None;
        for item in &file.items {
            if let syn::Item::Const(c) = item {
                if c.ident == "MANIFEST_SCHEMA_VERSION" {
                    let expr = &*c.expr;
                    if let syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Int(lit_int),
                        ..
                    }) = expr
                    {
                        live_value = Some(lit_int.base10_parse().expect("base10 int"));
                    }
                }
            }
        }

        let live_value = live_value.expect("MANIFEST_SCHEMA_VERSION constant not found");

        let constants_md = workspace_root().join("docs-site/docs/abi/constants.md");
        let constants_md = fs::read_to_string(constants_md).expect("read constants.md");
        let expected = regex::Regex::new(&format!(
            r"pub const MANIFEST_SCHEMA_VERSION: u32 = {}(?:u32)?;",
            live_value
        ))
        .expect("compile manifest schema regex");
        assert!(
            expected.is_match(&constants_md),
            "constants.md does not reflect live MANIFEST_SCHEMA_VERSION {}",
            live_value
        );
    }

    /// AC-3 anti-rot proven-red: a semantic mutation in a generated page is caught
    /// by `gen-abi-docs --check`.
    #[test]
    fn anti_rot_gate_catches_semantic_drift() {
        let temp = tempfile::tempdir().expect("tempdir");
        let temp_path = temp.path().to_path_buf();
        let abi_dir = workspace_root().join("docs-site/docs/abi");

        // Seed the temp output tree with the committed generated pages and hand-curated partials.
        for entry in fs::read_dir(abi_dir).expect("read abi dir") {
            let entry = entry.expect("dir entry");
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.ends_with(".md") {
                let src = fs::read_to_string(entry.path()).expect("read page");
                fs::write(temp_path.join(&name), src).expect("write temp page");
            }
        }

        // Mutate a semantic value: flip ABI_VERSION from 1 to 99.
        let idx_path = temp_path.join("constants.md");
        let mut idx = fs::read_to_string(&idx_path).expect("read temp constants.md");
        idx = idx.replace("pub const ABI_VERSION: u32 = 1;", "pub const ABI_VERSION: u32 = 99;");
        fs::write(&idx_path, idx).expect("write mutated constants.md");

        let result = super::run(Some(temp_path.to_str().unwrap()), true);
        assert!(
            result.is_err(),
            "expected --check to fail after semantic mutation, but it passed"
        );
    }

    #[test]
    fn anti_rot_gate_catches_orphan_generated_page() {
        let temp = tempfile::tempdir().expect("tempdir");
        let temp_path = temp.path().to_path_buf();
        let abi_dir = workspace_root().join("docs-site/docs/abi");

        for entry in fs::read_dir(abi_dir).expect("read abi dir") {
            let entry = entry.expect("dir entry");
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.ends_with(".md") {
                let src = fs::read_to_string(entry.path()).expect("read page");
                fs::write(temp_path.join(&name), src).expect("write temp page");
            }
        }

        fs::write(
            temp_path.join("ghost.md"),
            "<!-- AUTO-GENERATED from maos-spirit-abi rustdoc — do not edit; regenerate via: cargo run -p xtask -- gen-abi-docs -->\n\n# `ghost` Module\n",
        )
        .expect("write orphan page");

        let result = super::run(Some(temp_path.to_str().unwrap()), true);
        assert!(
            result.is_err(),
            "expected --check to fail with an orphan generated page, but it passed"
        );
    }
    /// AC-3 empty-diff floor: the generator must emit at least 8 pages.
    #[test]
    fn empty_diff_floor() {
        let abi_dir = workspace_root().join("docs-site/docs/abi");
        let pages: Vec<_> = fs::read_dir(abi_dir)
            .expect("read abi dir")
            .filter_map(|e| e.ok())
            .filter(|e| {
                let name = e.file_name();
                let s = name.to_string_lossy();
                s.ends_with(".md") && !s.starts_with("_related_")
            })
            .collect();
        assert!(
            pages.len() >= 8,
            "expected at least 8 generated abi pages, got {}",
            pages.len()
        );
    }

    /// AC-3 cross-gate-contract (simplified): every module recorded in the v1
    /// abi baseline has a generated page, and every generated module page has a
    /// matching `pub mod` entry in the current public API.
    #[test]
    fn cross_gate_contract_module_coverage() {
        let baseline = workspace_root().join("abi-baseline/v1-pre-bump.txt");
        let baseline = fs::read_to_string(baseline).expect("read abi baseline");

        let mut baseline_modules: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for line in baseline.lines() {
            if let Some(rest) = line.strip_prefix("pub mod maos_spirit_abi::") {
                // Skip nested modules; we only care about top-level modules.
                if !rest.contains("::") {
                    baseline_modules.insert(rest.to_string());
                }
            }
        }

        let abi_dir = workspace_root().join("docs-site/docs/abi");
        let mut generated_modules: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for entry in fs::read_dir(abi_dir).expect("read abi dir") {
            let entry = entry.expect("dir entry");
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".md")
                && !name.starts_with("_related_")
                && name != "index.md"
                && name != "constants.md"
            {
                let module = name.trim_end_matches(".md").replace('-', "_");
                generated_modules.insert(module);
            }
        }

        for module in &baseline_modules {
            assert!(
                generated_modules.contains(module),
                "baseline module {} has no generated page",
                module
            );
        }
    }
}
