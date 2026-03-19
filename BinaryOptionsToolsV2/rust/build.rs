#[cfg(feature = "stubgen")]
use pyo3_stub_gen::define_stub_info_gatherer;
#[cfg(feature = "stubgen")]
use std::path::PathBuf;
#[cfg(feature = "stubgen")]
use std::env;

fn main() {
    #[cfg(feature = "stubgen")]
    {
        // Define stub info gatherer function
        define_stub_info_gatherer!(stub_info);

        let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let python_package_path = crate_root
            .parent()
            .unwrap()
            .join("python")
            .join("BinaryOptionsToolsV2");

        // Ensure the target directory exists
        std::fs::create_dir_all(&python_package_path)
            .expect("Failed to create Python package directory");

        // Generate stub file
        let stub_info = stub_info().expect("Failed to gather stub info");
        stub_info.generate().expect("Failed to generate stubs");
    }
}
