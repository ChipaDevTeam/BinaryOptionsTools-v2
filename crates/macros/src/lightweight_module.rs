use darling::FromMeta;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{FnArg, ItemFn, Pat, PatType};

#[derive(Debug, FromMeta)]
pub struct LightweightModuleArgs {
    #[darling(default)]
    pub name: Option<String>,
    #[darling(default)]
    pub rule_pattern: Option<String>,
    #[darling(default)]
    pub rule_fn: Option<syn::Path>,
    #[darling(default)]
    pub rule_builder: Option<syn::Expr>,
    #[darling(default)]
    pub state_type: Option<syn::Type>,
}

pub fn expand(args: LightweightModuleArgs, func: ItemFn) -> TokenStream {
    let fn_name = &func.sig.ident;
    let struct_name_str = args.name.clone().unwrap_or_else(|| {
        let name = fn_name.to_string();
        let name = name.trim_start_matches("handle_");
        let mut chars = name.chars();
        match chars.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
        }
    });

    let struct_name = syn::Ident::new(&struct_name_str, func.sig.ident.span());

    let mut needs_state = false;
    let mut needs_sender = false;
    let mut needs_runner_cmd = false;

    let state_ty = args.state_type.clone();

    for arg in &func.sig.inputs {
        if let FnArg::Typed(PatType { pat, .. }) = arg {
            if let Pat::Ident(pat_ident) = &**pat {
                let arg_name = pat_ident.ident.to_string();
                if arg_name == "state" {
                    needs_state = true;
                    if state_ty.is_none() {
                        // Extract inner type if it's &Arc<State> or similar
                        // For simplicity in macro, user can provide state_type attribute
                        // Or we fallback to the default below
                    }
                } else if arg_name == "sender" {
                    needs_sender = true;
                } else if arg_name == "runner_cmd" {
                    needs_runner_cmd = true;
                }
            }
        }
    }

    let state_type =
        state_ty.unwrap_or_else(|| syn::parse_quote!(crate::pocketoption::state::State));

    let mut struct_fields = vec![
        quote! { receiver: binary_options_tools_core::reimports::AsyncReceiver<std::sync::Arc<binary_options_tools_core::reimports::Message>> },
    ];
    let new_args = vec![
        quote! { state: std::sync::Arc<#state_type> },
        quote! { sender: binary_options_tools_core::reimports::AsyncSender<binary_options_tools_core::reimports::Message> },
        quote! { receiver: binary_options_tools_core::reimports::AsyncReceiver<std::sync::Arc<binary_options_tools_core::reimports::Message>> },
        quote! { runner_cmd: binary_options_tools_core::reimports::AsyncSender<binary_options_tools_core::traits::RunnerCommand> },
    ];

    let mut new_assigns = vec![quote! { receiver }];

    if needs_state {
        struct_fields.push(quote! { state: std::sync::Arc<#state_type> });
        new_assigns.push(quote! { state });
    }
    if needs_sender {
        struct_fields.push(quote! { sender: binary_options_tools_core::reimports::AsyncSender<binary_options_tools_core::reimports::Message> });
        new_assigns.push(quote! { sender });
    }
    if needs_runner_cmd {
        struct_fields.push(quote! { runner_cmd: binary_options_tools_core::reimports::AsyncSender<binary_options_tools_core::traits::RunnerCommand> });
        new_assigns.push(quote! { runner_cmd });
    }

    let rule_impl = if let Some(rb) = args.rule_builder {
        quote! { Box::new(#rb) }
    } else if let Some(rf) = args.rule_fn {
        quote! { #rf() }
    } else if let Some(rp) = args.rule_pattern {
        quote! { Box::new(binary_options_tools_core::rules::RuleBuilder::text_starts_with(#rp).build()) }
    } else {
        quote! { Box::new(binary_options_tools_core::rules::RuleBuilder::any().build()) }
    };

    let call_args: Vec<_> = func
        .sig
        .inputs
        .iter()
        .filter_map(|arg| {
            if let FnArg::Typed(PatType { pat, .. }) = arg {
                if let Pat::Ident(pat_ident) = &**pat {
                    let name = &pat_ident.ident;
                    if name == "msg" {
                        return Some(quote! { msg.as_ref() });
                    } else {
                        return Some(quote! { &self.#name });
                    }
                }
            }
            None
        })
        .collect();

    let struct_docs = &func.attrs;

    quote! {
        #(#struct_docs)*
        pub struct #struct_name {
            #(#struct_fields),*
        }

        #[async_trait::async_trait]
        impl binary_options_tools_core::traits::LightweightModule<#state_type> for #struct_name {
            fn new(
                #(#new_args),*
            ) -> Self {
                Self {
                    #(#new_assigns),*
                }
            }

            fn rule() -> Box<dyn binary_options_tools_core::traits::Rule + Send + Sync> {
                #rule_impl
            }

            async fn run(&mut self) -> binary_options_tools_core::error::CoreResult<()> {
                #func

                while let Ok(msg) = self.receiver.recv().await {
                    #fn_name(#(#call_args),*).await?;
                }
                Err(binary_options_tools_core::error::CoreError::LightweightModuleLoop(stringify!(#struct_name).into()))
            }
        }
    }
}
