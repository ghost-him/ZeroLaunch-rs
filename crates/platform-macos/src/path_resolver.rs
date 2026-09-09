use std::path::PathBuf;
use zerolaunch_plugin_api::host::HostApiError;
use zerolaunch_plugin_api::services::path::{KnownPath, PathResolver};

/// Resolves ZeroLaunch and conventional user directories on macOS.
///
/// Used by storage, logging, icon-cache, and application discovery services.
pub struct MacosPathResolver;

impl MacosPathResolver {
    pub fn new() -> Self {
        Self
    }

    fn home() -> Result<PathBuf, HostApiError> {
        dirs::home_dir().ok_or_else(|| HostApiError::PathResolutionFailed {
            path: "home".into(),
            reason: "cannot determine the user home directory".into(),
        })
    }

    fn application_support() -> Result<PathBuf, HostApiError> {
        Ok(Self::home()?.join("Library/Application Support/ZeroLaunch-rs"))
    }
}

impl Default for MacosPathResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl PathResolver for MacosPathResolver {
    fn resolve_path(&self, path: KnownPath) -> Result<String, HostApiError> {
        let home = Self::home()?;
        let value = match path {
            KnownPath::CommonStartMenu => PathBuf::from("/Applications"),
            KnownPath::UserStartMenu => home.join("Applications"),
            KnownPath::UserDesktop => home.join("Desktop"),
            KnownPath::UserAppDataRoaming => home.join("Library/Application Support"),
            KnownPath::AppDataDir => Self::application_support()?,
            KnownPath::AppLogDir => home.join("Library/Logs/ZeroLaunch-rs"),
            KnownPath::AppIconCacheDir => Self::application_support()?.join("icons"),
            KnownPath::AppConfigDir => Self::application_support()?.join("config"),
            KnownPath::AppCacheDir => Self::application_support()?.join("plugin-cache"),
        };
        Ok(value.to_string_lossy().into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_macos_conventional_paths() {
        let resolver = MacosPathResolver::new();
        assert_eq!(
            resolver.resolve_path(KnownPath::CommonStartMenu).unwrap(),
            "/Applications"
        );
        assert!(resolver
            .resolve_path(KnownPath::AppDataDir)
            .unwrap()
            .ends_with("Library/Application Support/ZeroLaunch-rs"));
        assert!(resolver
            .resolve_path(KnownPath::AppLogDir)
            .unwrap()
            .ends_with("Library/Logs/ZeroLaunch-rs"));
    }
}
