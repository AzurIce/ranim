pub mod mesh_items;
pub mod viewport;
pub mod vitems;

/// Shared path for the GPU smoke tests' rendered images.
#[cfg(test)]
pub(crate) fn test_output_path(filename: &str) -> std::path::PathBuf {
    let output_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../output");
    std::fs::create_dir_all(&output_dir).expect("Failed to create output directory");
    output_dir.join(filename)
}
