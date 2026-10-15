#![forbid(unsafe_code)]

extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemStruct};

#[proc_macro_attribute]
pub fn i9_exempt(attr: TokenStream, item: TokenStream) -> TokenStream {
    let _ = attr;
    let input = parse_macro_input!(item as ItemStruct);

    quote! { #input }.into()
}
