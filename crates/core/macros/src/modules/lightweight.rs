#![allow(unused)]
use zyn::syn::Ident;


#[derive(zyn::Attribute)]
pub(crate) struct LightweightArgs {
    name: Option<String>
}

#[derive(zyn::Attribute)]
#[zyn("rule")]
pub(crate) enum Mode {
    Querry(String),
    Struct { object: Ident},
    Fn { func: Ident },
    
}
#[zyn::element]
pub(crate) fn lightweight_module(item: zyn::syn::ItemFn, args: LightweightArgs) -> zyn::TokenStream {
    
    zyn::zyn! {}
}