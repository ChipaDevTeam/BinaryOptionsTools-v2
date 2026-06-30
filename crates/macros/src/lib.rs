mod action;
mod region;
mod timeout;

use action::ActionImpl;
use region::RegionImpl;
use timeout::{Timeout, TimeoutArgs, TimeoutBody};

use darling::FromDeriveInput;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

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

#[proc_macro_derive(RegionImpl, attributes(region))]
pub fn region(input: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(input as DeriveInput);
    let region = match RegionImpl::from_derive_input(&parsed) {
        Ok(region) => region,
        Err(e) => return e.write_errors().into(),
    };
    quote! { #region }.into()
}

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
