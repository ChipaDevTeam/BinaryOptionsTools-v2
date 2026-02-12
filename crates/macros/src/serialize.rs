use quote::{quote, ToTokens};
use syn::{parse::Parse, Expr};

pub struct Serializer {
    value: Expr,
}

impl Parse for Serializer {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        input.parse().map(|value| Self { value })
    }
}

impl ToTokens for Serializer {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let value = &self.value;
        tokens.extend(quote! {
            ::serde_json::to_string(#value)
        });
    }
}
