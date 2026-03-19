use std::collections::HashMap;

#[derive(zyn::Attribute)]
pub struct UniffiDocArgs {
    name: Option<String>,
    path: String,    
}

#[zyn::element]
pub fn uniffi_doc(#[zyn(input)]code: zyn::syn::Item, args: UniffiDocArgs) -> zyn::TokenStream {
    let path = std::path::Path::new(&args.path);
    let content = std::fs::read_to_string(path).expect("Failed to read documentation file");
    let data: HashMap<String, String> = match &args.name {
        Some(name) => {
            let all_data: HashMap<String, HashMap<String, String>> = serde_json::from_str(&content).expect("Failed to parse documentation JSON");
            all_data.get(name).cloned().expect(&format!("Documentation for '{}' not found in JSON", name))
        }
        None => serde_json::from_str(&content).expect("Failed to parse documentation JSON"),
    };
    
    let default = data.get("default").map(|s| String::from(s) + "\n");
    
    zyn::zyn! {
        @if (default.is_some()) {
            #[doc = {{ default.unwrap() }}]
        }
        @for (element in data.into_iter()) {
            @if (element.0 != "default") {
                #[cfg_attr(feature = {{ element.0 }}, doc = {{ element.1 }})]
            } 
        }
        {{ code }}
    }
}