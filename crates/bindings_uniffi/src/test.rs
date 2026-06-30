use bo2_macros::uniffi_doc;

#[cfg(test)]
mod tests {
    #[test]
    fn test_uniffi_doc_macro_exists() {
        // Verify the uniffi_doc attribute macro is available
        let _ = uniffi_doc::uniffi_doc;
    }

    #[test]
    fn test_docs_json_exists() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("docs_json");
        assert!(path.exists(), "docs_json directory should exist");
    }
}
