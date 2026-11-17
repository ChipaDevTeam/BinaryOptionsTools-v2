// `args = args` inside `zyn!` is the macro's own assignment syntax, which clippy
// mistakes for a redundant struct field.
#![allow(clippy::redundant_field_names)]

mod doc;

use doc::{UniffiDoc, UniffiDocArgs};

use zyn::{Args, syn::spanned::Spanned};

#[zyn::attribute]
pub fn uniffi_doc(args: Args) -> zyn::TokenStream {
    let args = match UniffiDocArgs::from_args(&args) {
        Ok(args) => args,
        Err(e) => {
            bail!("Invalid arguments for uniffi_doc: {e}");
        }
    };
    zyn::zyn! {
        @uniffi_doc(args = args)
    }
}
