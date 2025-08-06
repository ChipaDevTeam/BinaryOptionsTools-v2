use proc_macro2::Span;
use quote::{quote, ToTokens};
use syn::{parse::Parse, Expr, FnArg, ItemFn, Pat, PatIdent, Token};

pub struct Timeout {
    args: TimeoutArgs,
    body: TimeoutBody,
}

pub struct TimeoutArgs {
    time_args: TimeoutInnerArgs,
    tracing_args: Option<TracingArgs>,
}

pub struct TimeoutBody {
    body: ItemFn,
}

pub struct TimeoutInnerArgs(Expr);

pub struct TracingArgs(Vec<Expr>);

impl Parse for TimeoutArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let time_args = input.parse()?;
        let mut tracing_args = None;
        let lookahead = input.lookahead1();
        if lookahead.peek(Token![,]) {
            let _: Token![,] = input.parse()?;
            let lookahead = input.lookahead1();
            if lookahead.peek(kw::tracing) {
                tracing_args = Some(input.parse()?);
            }
        }
        Ok(Self {
            time_args,
            tracing_args,
        })
    }
}

impl Parse for TracingArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let _ = input.parse::<kw::tracing>();
        let content;
        let _ = syn::parenthesized!(content in input);
        let args = content
            .parse_terminated(Expr::parse, Token![,])?
            .into_iter()
            .collect();

        Ok(Self(args))
    }
}

impl Parse for TimeoutInnerArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        input.parse().map(Self)
    }
}

impl Parse for TimeoutBody {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let body: ItemFn = input.parse()?;
        match body.sig.asyncness {
            Some(_) => Ok(Self { body }),
            None => Err(syn::Error::new(
                Span::call_site(),
                "Expected function to be async",
            )),
        }
    }
}

impl ToTokens for TracingArgs {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let args = &self.0;
        if let Some(first) = args.first() {
            let args = &args[1..];
            tokens.extend(quote! {
                #[::tracing::instrument(#first #(, #args)*)]
            });
        } else {
            tokens.extend(quote! {
                #[::tracing::instrument]
            });
        }
    }
}

impl ToTokens for TimeoutInnerArgs {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let time = &self.0;

        tokens.extend(quote! {
            ::std::time::Duration::from_secs(#time)
        });
    }
}

impl ToTokens for TimeoutBody {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let body = &self.body;
        tokens.extend(quote! {
            #body
        });
    }
}

impl Timeout {
    pub fn new(body: TimeoutBody, args: TimeoutArgs) -> Self {
        Self { body, args }
    }
}

impl ToTokens for Timeout {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let TimeoutArgs {
            time_args,
            tracing_args,
        } = &self.args;
        let TimeoutBody { body } = &self.body;
        let fn_name = &body.sig.ident;
        let fn_name_str = fn_name.to_string();
        let inputs = &body.sig.inputs;
        let input_names = inputs.iter().filter_map(|a| match a {
            FnArg::Receiver(_) => None,
            FnArg::Typed(tp) => {
                if let Pat::Ident(PatIdent { ident, .. }) = &*tp.pat {
                    Some(ident)
                } else {
                    None
                }
            }
        });
        // let output = match &body.sig.output {
        //     ReturnType::Default => quote! { () },
        //     ReturnType::Type(_, tp) => quote! { #tp }
        // };
        let output = &body.sig.output;

        tokens.extend( quote! {
            #tracing_args
            async fn #fn_name(#inputs) #output {
                #body
                let res = ::tokio::select! {
                    res = #fn_name(#(#input_names ,)*) => Ok(res),
                    _ = ::tokio::time::sleep(#time_args) => Err(::binary_options_tools_core_pre::error::CoreError::TimeoutError { task: ::std::string::ToString::to_string(#fn_name_str), duration: #time_args })
                };
                res?
            }
        });
    }
}

mod kw {
    syn::custom_keyword!(tracing);
}
