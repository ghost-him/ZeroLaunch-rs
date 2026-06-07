use std::path::PathBuf;

/// Scan the plugins directory for subdirectories containing a manifest.toml.
pub fn scan_plugins_dir(plugins_dir: &std::path::Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    if !plugins_dir.exists() {
        return found;
    }
    if let Ok(entries) = std::fs::read_dir(plugins_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let manifest = path.join("manifest.toml");
                if manifest.exists() {
                    found.push(path);
                }
            }
        }
    }
    found
}
