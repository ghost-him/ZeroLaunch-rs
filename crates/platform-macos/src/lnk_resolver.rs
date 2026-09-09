use zerolaunch_plugin_api::services::shell::LnkResolver;
/// Resolves Windows `.lnk` targets on macOS, where the format is unsupported.
///
/// Used by shell integrations to report the platform capability boundary.
pub struct MacosLnkResolver;
impl MacosLnkResolver {
    pub fn new() -> Self {
        Self
    }
}
impl Default for MacosLnkResolver {
    fn default() -> Self {
        Self::new()
    }
}
impl LnkResolver for MacosLnkResolver {
    fn resolve_lnk_target(&self, _lnk_path: &str) -> Option<String> {
        None
    }
}
