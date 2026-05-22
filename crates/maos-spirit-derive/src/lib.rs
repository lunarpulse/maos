#![forbid(unsafe_code)]

extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::spanned::Spanned;
use syn::{parse_macro_input, ImplItem, ItemImpl};

const HOOK_NAMES: &[&str] = &[
    "on_load",
    "on_start",
    "on_frame",
    "on_idle",
    "on_telemetry_event",
    "on_schedule",
    "on_swap_in",
    "on_pause",
    "on_resume",
    "on_unload",
    "on_consolidate",
    "on_swap_out",
    "snapshot",
    "migrate",
];

fn payload_type_for(hook: &str) -> Option<&str> {
    match hook {
        "on_frame" => Some("FramePayload"),
        "on_telemetry_event" => Some("TelemetryEventPayload"),
        "on_schedule" => Some("SchedulePayload"),
        "on_swap_in" => Some("SwapInPayload"),
        "on_consolidate" => Some("ConsolidatePayload"),
        _ => None,
    }
}

const BUDGET_KEYS: &[&str] = &[
    "context_window",
    "time_cap_seconds",
    "cpu_max_pct",
    "memory_max_mb",
    "fd_max",
];

struct SpiritAttrParser {
    name: Option<String>,
}

impl syn::parse::Parse for SpiritAttrParser {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut name = None;
        while !input.is_empty() {
            let meta: syn::Meta = input.parse()?;
            match &meta {
                syn::Meta::NameValue(nv) if nv.path.is_ident("name") => {
                    if let syn::Expr::Lit(expr_lit) = &nv.value {
                        if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                            let s = lit_str.value();
                            if s.is_empty() {
                                return Err(syn::Error::new(
                                    lit_str.span(),
                                    "#[spirit(name = \"...\")] must be non-empty",
                                ));
                            }
                            name = Some(s);
                        } else {
                            return Err(syn::Error::new(
                                expr_lit.span(),
                                "#[spirit(name = ...)] requires a string literal",
                            ));
                        }
                    } else {
                        return Err(syn::Error::new(
                            nv.value.span(),
                            "#[spirit(name = ...)] requires a string literal",
                        ));
                    }
                }
                _ => {
                    return Err(syn::Error::new(
                        meta.span(),
                        "unknown #[spirit] attribute — expected `name = \"...\"`",
                    ));
                }
            }
            if !input.is_empty() {
                let _: syn::Token![,] = input.parse()?;
            }
        }
        Ok(Self { name })
    }
}

fn parse_hook_budget_attr(attr: &syn::Attribute) -> Result<Option<String>, syn::Error> {
    let mnv: syn::MetaNameValue = attr.parse_args()?;
    if !mnv.path.is_ident("budget") {
        return Err(syn::Error::new(
            mnv.path.span(),
            "#[hook(...)] only supports `budget = \"...\"` attribute",
        ));
    }
    if let syn::Expr::Lit(expr_lit) = &mnv.value {
        if let syn::Lit::Str(lit_str) = &expr_lit.lit {
            let key = lit_str.value();
            if !BUDGET_KEYS.contains(&key.as_str()) {
                return Err(syn::Error::new(
                    lit_str.span(),
                    format!(
                        "unknown budget key `{key}` — must be one of: {}",
                        BUDGET_KEYS.join(", ")
                    ),
                ));
            }
            return Ok(Some(key));
        }
    }
    Err(syn::Error::new(
        mnv.value.span(),
        "#[hook(budget = \"...\")] requires a string literal",
    ))
}

/// `#[spirit]` proc-macro attribute.
///
/// Derives `maos_spirit_abi::lifecycle::Spirit` for the annotated impl block,
/// generates default no-op bodies for undeclared hooks, and exports a
/// `__maos_spirit_vtable_<Type>()` function returning a `&'static SpiritVtable<Self>`.
///
/// Accepts optional attribute: `#[spirit(name = "my-spirit")]`.
/// Per-hook budget attribute: `#[hook(budget = "context_window")]`.
#[proc_macro_attribute]
pub fn spirit(attr: TokenStream, item: TokenStream) -> TokenStream {
    let spirit_name = if attr.is_empty() {
        None
    } else {
        match syn::parse2::<SpiritAttrParser>(attr.into()) {
            Ok(parsed) => parsed.name,
            Err(e) => return e.to_compile_error().into(),
        }
    };

    let mut impl_block = parse_macro_input!(item as ItemImpl);

    if impl_block.trait_.is_some() {
        let err = syn::Error::new(
            impl_block.self_ty.span(),
            "#[spirit] cannot be applied to trait impls (`impl Trait for Type`) — use `impl MySpirit { ... }` instead",
        );
        return err.to_compile_error().into();
    }

    let self_ty = match &*impl_block.self_ty {
        syn::Type::Path(p) if p.qself.is_none() => p.path.get_ident().cloned(),
        _ => None,
    };

    let Some(self_ident) = self_ty else {
        let err = syn::Error::new(
            impl_block.self_ty.span(),
            "#[spirit] can only be applied to `impl MySpirit { ... }` (non-path self type)",
        );
        return err.to_compile_error().into();
    };

    let type_name_str = self_ident.to_string();

    let mut declared = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for item in &mut impl_block.items {
        if let ImplItem::Fn(method) = item {
            let name = method.sig.ident.to_string();
            if !HOOK_NAMES.contains(&name.as_str()) {
                let err = syn::Error::new(
                    method.sig.ident.span(),
                    format!(
                        "unknown hook `{name}` — must be one of: {}",
                        HOOK_NAMES.join(", ")
                    ),
                );
                return err.to_compile_error().into();
            }
            if !seen.insert(name.clone()) {
                let err = syn::Error::new(
                    method.sig.ident.span(),
                    format!("duplicate hook declaration: `{name}`"),
                );
                return err.to_compile_error().into();
            }

            let mut hook_attr_idx = None;
            for (i, attr) in method.attrs.iter().enumerate() {
                if attr.path().is_ident("hook") {
                    hook_attr_idx = Some(i);
                    break;
                }
            }
            if let Some(idx) = hook_attr_idx {
                if let Err(e) = parse_hook_budget_attr(&method.attrs[idx]) {
                    return e.to_compile_error().into();
                }
                method.attrs.remove(idx);
            }

            declared.push(name);
        } else {
            let err = syn::Error::new_spanned(
                item,
                "#[spirit] impl blocks may only contain hook methods (fn items)",
            );
            return err.to_compile_error().into();
        }
    }

    let ctx_path = quote! { maos_spirit_abi::ctx::Ctx };
    let vtable_path = quote! { maos_spirit_abi::lifecycle::SpiritVtable };
    let trait_path = quote! { maos_spirit_abi::lifecycle::Spirit };

    let mut trait_methods = Vec::new();
    let mut vtable_wrappers = Vec::new();
    let mut vtable_fields = Vec::new();

    for &hook_name in HOOK_NAMES {
        let hook_ident = format_ident!("{}", hook_name);
        let wrapper_ident = format_ident!("__maos_spirit_vt_{}", hook_name);
        let field_ident = format_ident!("{}", hook_name);
        let is_declared = declared.iter().any(|n| n == hook_name);

        // Story 5.2: snapshot and migrate have special signatures.
        if hook_name == "snapshot" {
            // snapshot returns Vec<u8>.
            let vec_path = quote! { Vec<u8> };
            let wrapper = if is_declared {
                quote! {
                    fn #wrapper_ident(s: &#self_ident, c: &mut #ctx_path) -> #vec_path {
                        s.#hook_ident(c)
                    }
                }
            } else {
                quote! {
                    fn #wrapper_ident(_s: &#self_ident, _c: &mut #ctx_path) -> #vec_path {
                        Vec::new()
                    }
                }
            };
            vtable_wrappers.push(wrapper);
            vtable_fields.push(quote! { #field_ident: #wrapper_ident });

            let method = if is_declared {
                quote! {
                    fn #hook_ident(&self, ctx: &mut #ctx_path) -> #vec_path {
                        self.#hook_ident(ctx)
                    }
                }
            } else {
                quote! {
                    fn #hook_ident(&self, _ctx: &mut #ctx_path) -> #vec_path {
                        Vec::new()
                    }
                }
            };
            trait_methods.push(method);
            continue;
        }

        if hook_name == "migrate" {
            // migrate takes &[u8] and returns Result<Vec<u8>, MigratorError>.
            let migrator_err_path = quote! { maos_spirit_abi::lifecycle::MigratorError };
            let vec_path = quote! { Vec<u8> };
            let wrapper = if is_declared {
                quote! {
                    fn #wrapper_ident(s: &#self_ident, c: &mut #ctx_path, p: &[u8]) -> Result<#vec_path, #migrator_err_path> {
                        s.#hook_ident(c, p)
                    }
                }
            } else {
                quote! {
                    fn #wrapper_ident(_s: &#self_ident, _c: &mut #ctx_path, _p: &[u8]) -> Result<#vec_path, #migrator_err_path> {
                        Err(#migrator_err_path::NotImplemented)
                    }
                }
            };
            vtable_wrappers.push(wrapper);
            vtable_fields.push(quote! { #field_ident: #wrapper_ident });

            let method = if is_declared {
                quote! {
                    fn #hook_ident(&self, ctx: &mut #ctx_path, predecessor_state: &[u8]) -> Result<#vec_path, #migrator_err_path> {
                        self.#hook_ident(ctx, predecessor_state)
                    }
                }
            } else {
                quote! {
                    fn #hook_ident(&self, _ctx: &mut #ctx_path, _predecessor_state: &[u8]) -> Result<#vec_path, #migrator_err_path> {
                        Err(#migrator_err_path::NotImplemented)
                    }
                }
            };
            trait_methods.push(method);
            continue;
        }

        if let Some(payload_str) = payload_type_for(hook_name) {
            let payload_ty_ident = format_ident!("{}", payload_str);
            let payload_path = quote! { maos_spirit_abi::lifecycle::#payload_ty_ident };

            let wrapper = quote! {
                fn #wrapper_ident<'a>(s: &#self_ident, c: &mut #ctx_path, p: &#payload_path<'a>) {
                    s.#hook_ident(c, p);
                }
            };
            vtable_wrappers.push(wrapper);
            vtable_fields.push(quote! { #field_ident: #wrapper_ident });

            let method = if is_declared {
                quote! {
                    fn #hook_ident<'a>(&self, ctx: &mut #ctx_path, payload: &#payload_path<'a>) {
                        self.#hook_ident(ctx, payload);
                    }
                }
            } else {
                quote! {
                    fn #hook_ident<'a>(&self, _ctx: &mut #ctx_path, _payload: &#payload_path<'a>) {}
                }
            };
            trait_methods.push(method);
        } else {
            let wrapper = if is_declared {
                quote! {
                    fn #wrapper_ident(s: &#self_ident, c: &mut #ctx_path) {
                        s.#hook_ident(c);
                    }
                }
            } else {
                quote! {
                    fn #wrapper_ident(_s: &#self_ident, _c: &mut #ctx_path) {}
                }
            };
            vtable_wrappers.push(wrapper);
            vtable_fields.push(quote! { #field_ident: #wrapper_ident });

            let method = if is_declared {
                quote! {
                    fn #hook_ident(&self, ctx: &mut #ctx_path) {
                        self.#hook_ident(ctx);
                    }
                }
            } else {
                quote! {
                    fn #hook_ident(&self, _ctx: &mut #ctx_path) {}
                }
            };
            trait_methods.push(method);
        }
    }

    let vtable_fn_name = format_ident!("__maos_spirit_vtable_{}", type_name_str);
    let static_name = format_ident!("__MAOS_SPIRIT_VTABLE_{}", type_name_str.to_uppercase());

    let name_static = if let Some(ref name) = spirit_name {
        let name_lit = name.as_str();
        quote! {
            #[doc(hidden)]
            pub static __MAOS_SPIR_NAME: &str = #name_lit;
        }
    } else {
        quote! {
            #[doc(hidden)]
            pub static __MAOS_SPIR_NAME: &str = #type_name_str;
        }
    };

    let expanded = quote! {
        #impl_block

        impl #trait_path for #self_ident {
            #(#trait_methods)*
        }

        #(#vtable_wrappers)*

        #name_static

        #[doc(hidden)]
        pub fn #vtable_fn_name() -> &'static #vtable_path<#self_ident> {
            static #static_name: std::sync::LazyLock<#vtable_path<#self_ident>> = std::sync::LazyLock::new(|| {
                #vtable_path {
                    #(#vtable_fields,)*
                    _phantom: core::marker::PhantomData,
                }
            });
            &#static_name
        }
    };

    expanded.into()
}
