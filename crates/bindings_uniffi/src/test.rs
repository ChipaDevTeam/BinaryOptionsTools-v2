use bo2_macros::uniffi_doc;

#[cfg(test)]
mod tests {
    use super::uniffi_doc;

    #[test]
    fn test_uniffi_doc_macro_exists() {
        #[uniffi_doc(name = "test", path = "docs_json/test.json")]
        #[allow(dead_code)]
        struct Dummy;
    }

    #[test]
    fn test_docs_json_exists() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("docs_json");
        assert!(path.exists(), "docs_json directory should exist");
    }
}
