use std::collections::HashMap;

#[derive(zyn::Attribute)]
pub struct UniffiDocArgs {
    name: Option<String>,
    path: String,
}

#[zyn::element]
pub fn uniffi_doc(#[zyn(input)] code: zyn::syn::Item, args: UniffiDocArgs) -> zyn::TokenStream {
    let path = std::path::Path::new(&args.path);
    let content = if path.exists() {
        std::fs::read_to_string(path)
    } else {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
        let manifest_path = std::path::Path::new(&manifest_dir);
        let try1 = manifest_path.join(&args.path);
        if try1.exists() {
            std::fs::read_to_string(try1)
        } else {
            let stripped_path = if args.path.starts_with("crates/bindings_uniffi/") {
                &args.path["crates/bindings_uniffi/".len()..]
            } else if args.path.starts_with("crates\\bindings_uniffi\\") {
                &args.path["crates\\bindings_uniffi\\".len()..]
            } else {
                &args.path
            };
            let try2 = manifest_path.join(stripped_path);
            if try2.exists() {
                std::fs::read_to_string(try2)
            } else {
                let try3 = manifest_path.parent()
                    .and_then(|p| p.parent())
                    .map(|p| p.join(&args.path));
                if let Some(ref p) = try3 {
                    if p.exists() {
                        std::fs::read_to_string(p)
                    } else {
                        panic!(
                            "Failed to find documentation file '{}'. Tried:\n  - {:?}\n  - {:?}\n  - {:?}\n  - {:?}",
                            args.path, path, try1, try2, try3
                        );
                    }
                } else {
                    panic!(
                        "Failed to find documentation file '{}'. Tried:\n  - {:?}\n  - {:?}\n  - {:?}",
                        args.path, path, try1, try2
                    );
                }
            }
        }
    }.expect("Failed to read documentation file");
    let data: HashMap<String, String> = match &args.name {
        Some(name) => {
            let all_data: HashMap<String, HashMap<String, String>> =
                serde_json::from_str(&content).expect("Failed to parse documentation JSON");
            all_data
                .get(name)
                .cloned()
                .unwrap_or_else(|| panic!("Documentation for '{}' not found in JSON", name))
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
