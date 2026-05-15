use proc_macro2::TokenStream;
use quote::quote;
use syn::ItemStruct;

pub fn expand_lightweight_module(item: ItemStruct) -> TokenStream {
    let name = &item.ident;
    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();

    quote! {
        #item

        #[async_trait::async_trait]
        impl #impl_generics LightweightModule<State> for #name #ty_generics #where_clause {
            // Default implementations can be added here
        }
    }
}

pub fn expand_api_module(item: ItemStruct) -> TokenStream {
    let name = &item.ident;
    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();

    quote! {
        #item

        #[async_trait::async_trait]
        impl #impl_generics ApiModule<State> for #name #ty_generics #where_clause {
            // Default implementations can be added here
        }
    }
}
