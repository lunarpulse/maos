#![forbid(unsafe_code)]

extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemStruct};

#[proc_macro_attribute]
pub fn i9_exempt(attr: TokenStream, item: TokenStream) -> TokenStream {
    let _ = attr;
    let input = parse_macro_input!(item as ItemStruct);
    let ident = &input.ident;
    let attrs = &input.attrs;
    let vis = &input.vis;
    let fields = &input.fields;

    let reason: Option<String> = None;

    quote! {
        #(#attrs)*
        #vis struct #ident #fields
    }
    .into()
}
