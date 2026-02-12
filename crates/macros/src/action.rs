use darling::FromDeriveInput;
use quote::{ToTokens, quote};
use syn::Ident;

/// Auto implement the ActionName trait for types on the ExpertOptions API.
#[derive(FromDeriveInput)]
#[darling(attributes(action))]
pub struct ActionImpl {
    ident: Ident,
    name: String,
}

impl ActionImpl {
    /// As most of the ExpertOptions API responses contains the action name, this macro also generates a struct implementing the Rule trait.
    fn generate_rule(&self) -> proc_macro2::TokenStream {
        let rule_name = format!("{}Rule", self.ident);
        let rule_ident = Ident::new(&rule_name, self.ident.span());
        let pattern = format!("{{\"action\":\"{}\"", self.name);
        quote! {
            pub struct #rule_ident;

            impl ::binary_options_tools_core_pre::traits::Rule for #rule_ident {
                fn call(&self, msg: &::binary_options_tools_core_pre::reimports::Message) -> bool {
                    if let ::binary_options_tools_core_pre::reimports::Message::Binary(text) = msg {
                        text.starts_with(#pattern.as_bytes())
                    } else {
                        false
                    }
                }

                fn reset(&self) {
                    // no state to reset
                }
            }
        }
        //         fn call(&self, msg: &Message) -> bool {
        //     // tracing::info!("Called with message: {:?}", msg);
        //     match msg {
        //         Message::Text(text) => {
        //             for pattern in &self.patterns {
        //                 if text.starts_with(pattern) {
        //                     self.valid.store(true, Ordering::SeqCst);
        //                     return false;
        //                 }
        //             }
        //             false
        //         }
        //         Message::Binary(_) => {
        //             if self.valid.load(Ordering::SeqCst) {
        //                 self.valid.store(false, Ordering::SeqCst);
        //                 true
        //             } else {
        //                 false
        //             }
        //         }
        //         _ => false,
        //     }
        // }

        // fn reset(&self) {
        //     self.valid.store(false, Ordering::SeqCst)
        // }
    }
    /// Generate the implementation tokens for the ActionName trait
    pub fn generate_impl(&self) -> proc_macro2::TokenStream {
        let ident = &self.ident;
        let action_name = &self.name;
        let rule = self.generate_rule();
        quote! {
            #rule

            impl ActionName for #ident {
                fn name(&self) -> &str {
                    #action_name
                }
            }

        }
    }
}

impl ToTokens for ActionImpl {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let impl_tokens = self.generate_impl();
        tokens.extend(impl_tokens);
    }
}
