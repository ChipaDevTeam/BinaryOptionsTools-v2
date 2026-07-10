#[cfg(feature = "stubgen")]
use pyo3_stub_gen::define_stub_info_gatherer;
#[cfg(feature = "stubgen")]
use std::env;
#[cfg(feature = "stubgen")]
use std::path::PathBuf;

fn main() {
    #[cfg(feature = "stubgen")]
    {
        // Define stub info gatherer function
        define_stub_info_gatherer!(stub_info);

        let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let python_package_path = crate_root
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("python")
            .join("BinaryOptionsToolsV2");
        let pyproject_toml_path = crate_root
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("python")
            .join("pyproject.toml");

        // Ensure the target directory exists
        std::fs::create_dir_all(&python_package_path)
            .expect("Failed to create Python package directory");

        // Generate stub file
        let stub_info = pyo3_stub_gen::StubInfo::from_pyproject_toml(&pyproject_toml_path)
            .expect("Failed to gather stub info");
        stub_info.generate().expect("Failed to generate stubs");
    }
}
