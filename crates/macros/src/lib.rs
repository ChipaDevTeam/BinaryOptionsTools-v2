mod action;
mod config;
mod deserialize;
mod impls;
// mod lightweight_module;
mod region;
mod serialize;
mod timeout;

use action::ActionImpl;
use config::Config;
use deserialize::Deserializer;
use region::RegionImpl;
use timeout::{Timeout, TimeoutArgs, TimeoutBody};

use darling::FromDeriveInput;
use proc_macro::TokenStream;
use quote::quote;
use serialize::Serializer;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro]
pub fn deserialize(input: TokenStream) -> TokenStream {
    let d = parse_macro_input!(input as Deserializer);
    quote! { #d }.into()
}

#[proc_macro]
pub fn serialize(input: TokenStream) -> TokenStream {
    let s = parse_macro_input!(input as Serializer);
    quote! { #s }.into()
}

/// This macro wraps any async function and transforms it's output `T` into `anyhow::Result<T>`,
/// if the function doesn't end before the timeout it will raise an error
/// The macro also supports creating a `#[tracing::instrument]` macro with all the params inside `tracing(args)`
/// Example:
///     #[timeout(10, tracing(skip(non_debug_input)))]
///     #[timeout(12)]
#[proc_macro_attribute]
pub fn timeout(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as TimeoutArgs);
    let body = parse_macro_input!(item as TimeoutBody);
    let timeout = Timeout::new(body, args);
    let q = quote! { #timeout };

    // println!("{q}");
    q.into()
}

#[proc_macro_derive(Config, attributes(config))]
pub fn config(input: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(input as DeriveInput);
    let config = Config::from_derive_input(&parsed).unwrap();
    quote! { #config }.into()
}

#[proc_macro_derive(RegionImpl, attributes(region))]
pub fn region(input: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(input as DeriveInput);
    let region = RegionImpl::from_derive_input(&parsed).unwrap();
    quote! { #region }.into()
}

// #[proc_macro_attribute]
// pub fn lightweight_module(attr: TokenStream, item: TokenStream) -> TokenStream {
//     let attr_args = match darling::ast::NestedMeta::parse_meta_list(attr.into()) {
//         Ok(v) => v,
//         Err(e) => return TokenStream::from(darling::Error::from(e).write_errors()),
//     };
//     let args = match darling::FromMeta::from_list(&attr_args) {
//         Ok(v) => v,
//         Err(e) => return TokenStream::from(e.write_errors()),
//     };

//     let func = parse_macro_input!(item as syn::ItemFn);
//     lightweight_module::expand(args, func).into()
// }

#[proc_macro_derive(ActionImpl, attributes(action))]
pub fn action_impl(input: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(input as DeriveInput);
    let action = match ActionImpl::from_derive_input(&parsed) {
        Ok(action) => action,
        Err(e) => return e.write_errors().into(),
    };
    quote! {
        #action
    }
    .into()
}
