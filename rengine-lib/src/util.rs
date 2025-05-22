use std::path::{Path, PathBuf};

pub fn resource_path(rel: &str) -> String {
    // Try CARGO_MANIFEST_DIR (for dev), else current dir
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let candidate = Path::new(&manifest_dir).join(rel);
        if candidate.exists() {
            return candidate.to_string_lossy().to_string();
        }
    }
    let candidate = PathBuf::from(rel);
    if candidate.exists() {
        return candidate.to_string_lossy().to_string();
    }
    rel.to_string() // fallback, may error at runtime
}
