use std::collections::HashMap;
use std::path::Path;
use zerolaunch_plugin_api::services::shell::ResourceLoader;
/// Loads localized resource names from macOS application resources.
///
/// Used by resource-backed candidate providers through the platform abstraction.
pub struct MacosResourceLoader;
impl MacosResourceLoader {
    pub fn new() -> Self {
        Self
    }
}
impl Default for MacosResourceLoader {
    fn default() -> Self {
        Self::new()
    }
}
impl ResourceLoader for MacosResourceLoader {
    fn parse_localized_names_from_dir(&self, _dir: &Path) -> HashMap<String, String> {
        HashMap::new()
    }
}
