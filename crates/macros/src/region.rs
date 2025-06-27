use darling::{util::Override, FromDeriveInput};
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use serde::Deserialize;
use std::collections::HashSet;
use std::fs::File;
use std::hash::Hash;
use std::io::Read;
use std::path::PathBuf;
use syn::Ident;
use url::Url;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(region))]
pub struct RegionImpl {
    ident: Ident,
    path: Override<PathBuf>,
}

#[derive(Debug, Deserialize)]
struct Regions(HashSet<Region>);

#[derive(Debug, Deserialize)]
struct Region {
    name: String,
    url: Url,
    latitude: f64,
    longitude: f64,
    demo: bool,
}

impl RegionImpl {
    fn regions(&self) -> anyhow::Result<Regions> {
        let base_path = self
            .path
            .as_ref()
            .explicit()
            .ok_or(anyhow::anyhow!("No path specified"))?;

        // Try multiple possible locations for the file
        let possible_paths = [
            // Direct path
            base_path.clone(),
            // Relative to current manifest dir
            std::env::var("CARGO_MANIFEST_DIR")
                .map(|dir| PathBuf::from(dir).join(base_path))
                .unwrap_or_else(|_| base_path.clone()),
            // Relative to workspace root (go up from crate to workspace)
            std::env::var("CARGO_MANIFEST_DIR")
                .map(|dir| {
                    PathBuf::from(dir)
                        .parent()
                        .unwrap()
                        .parent()
                        .unwrap()
                        .join(base_path)
                })
                .unwrap_or_else(|_| base_path.clone()),
        ];

        let file_path = possible_paths
            .iter()
            .find(|path| path.exists())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Could not find file at any of these locations: {:?}",
                    possible_paths
                )
            })?;

        let mut file = File::open(file_path)?;
        let mut buff = String::new();
        file.read_to_string(&mut buff)?;

        Ok(serde_json::from_str(&buff)?)
    }
}

impl ToTokens for RegionImpl {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = &self.ident;
        let implementation = &self.regions().unwrap();

        tokens.extend(quote! {
            impl #name {
                #implementation
            }
        });
    }
}

impl ToTokens for Regions {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let regions: &Vec<&Region> = &self.0.iter().collect();
        let demos: Vec<&Region> = regions.iter().filter_map(|r| r.get_demo()).collect();
        let demos_stream = demos.iter().map(|r| r.to_stream());
        let demos_url = demos.iter().map(|r| r.url());
        let reals: Vec<&Region> = regions.iter().filter_map(|r| r.get_real()).collect();
        let reals_stream = reals.iter().map(|r| r.to_stream());
        let reals_url = reals.iter().map(|r| r.url());

        tokens.extend(quote! {
            #(#regions)*

            pub fn demo_regions() -> Vec<(&'static str, f64, f64)> {
                vec![#(#demos_stream),*]
            }

            pub fn regions() -> Vec<(&'static str, f64, f64)> {
                vec![#(#reals_stream),*]
            }

            pub fn demo_regions_str() -> Vec<&'static str> {
                ::std::vec::Vec::from([#(#demos_url),*])
            }

            pub fn regions_str() -> Vec<&'static str> {
                ::std::vec::Vec::from([#(#reals_url),*])
            }
        });
    }
}

impl ToTokens for Region {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = self.name();
        let url = &self.url.to_string();
        let latitude = self.latitude;
        let longitude = self.longitude;
        tokens.extend(quote! {
            pub const #name: (&str, f64, f64) = (#url, #latitude, #longitude);
        });
    }
}

impl PartialEq for Region {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for Region {}

impl Hash for Region {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl Region {
    fn name(&self) -> Ident {
        Ident::new(&self.name.to_uppercase(), Span::call_site())
    }

    fn url(&self) -> TokenStream {
        let name = self.name();
        quote! {
            Self::#name.0
        }
    }

    fn to_stream(&self) -> TokenStream {
        let name = self.name();
        quote! {
            Self::#name
        }
    }

    fn get_demo(&self) -> Option<&Self> {
        if self.demo {
            Some(self)
        } else {
            None
        }
    }

    fn get_real(&self) -> Option<&Self> {
        if !self.demo {
            Some(self)
        } else {
            None
        }
    }
}
