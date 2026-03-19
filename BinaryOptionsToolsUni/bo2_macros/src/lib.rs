mod doc;

use doc::{UniffiDoc, UniffiDocArgs};

use zyn::{Args, syn::spanned::Spanned};

#[zyn::attribute]
pub fn uniffi_doc(args: Args) -> zyn::TokenStream {
    let args = match UniffiDocArgs::from_args(&args) {
        Ok(args) => args,
        Err(e) => {
            bail!("Invalid arguments for uniffi_doc: {}", e.to_string());
        }
    };
    zyn::zyn! {
        @uniffi_doc(args = args)
    }   
}