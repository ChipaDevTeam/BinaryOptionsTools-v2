use quote::{quote, ToTokens};
use syn::{parse::Parse, Expr, Token, Type};

pub struct Deserializer {
    res_type: Type,
    data: Expr,
}

impl Parse for Deserializer {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let res_type = input.parse()?;
        let _: Token![,] = input.parse()?;
        let data = input.parse()?;
        Ok(Self { res_type, data })
    }
}

impl ToTokens for Deserializer {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let res_type = &self.res_type;
        let data = &self.data;
        tokens.extend(quote! {
            ::serde_json::from_str::<#res_type>(&std::string::ToString::to_string(#data))
        });
    }
}
