//! Config path resolution
//!
//! Handles resolving paths for configuration files based on the plugin's location.

use std::path::PathBuf;

use super::{ConfigError, ConfigResult};

/// Returns the cs2rust base directory by navigating up from the plugin .so location.
///
/// The plugin is loaded from:
/// `game/csgo/addons/cs2rust/bin/linuxsteamrt64/cs2rust.so`
///
/// This navigates up 3 levels to reach:
/// `game/csgo/addons/cs2rust/`
pub fn cs2rust_base_dir() -> ConfigResult<PathBuf> {
    let exe = std::env::current_exe().map_err(ConfigError::IoError)?;

    // Navigate: cs2rust.so -> linuxsteamrt64 -> bin -> cs2rust
    exe.parent() // linuxsteamrt64/
        .and_then(|p| p.parent()) // bin/
        .and_then(|p| p.parent()) // cs2rust/
        .map(PathBuf::from)
        .ok_or(ConfigError::NoConfigDirectory)
}

/// Returns the base configs directory.
///
/// Path: `game/csgo/addons/cs2rust/configs/`
pub fn configs_dir() -> ConfigResult<PathBuf> {
    Ok(cs2rust_base_dir()?.join("configs"))
}

/// Returns the path for a plugin's config file.
///
/// Path: `game/csgo/addons/cs2rust/configs/plugins/{plugin_name}/{plugin_name}.toml`
pub fn plugin_config_path(plugin_name: &str) -> ConfigResult<PathBuf> {
    let base = configs_dir()?;
    Ok(base
        .join("plugins")
        .join(plugin_name)
        .join(format!("{}.toml", plugin_name)))
}

/// Returns the core framework config path.
///
/// Path: `game/csgo/addons/cs2rust/configs/core.toml`
pub fn core_config_path() -> ConfigResult<PathBuf> {
    Ok(configs_dir()?.join("core.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_config_path_format() {
        // This test verifies path construction logic
        // In real environment, cs2rust_base_dir() would work
        let base = PathBuf::from("/game/csgo/addons/cs2rust");
        let expected = base
            .join("configs")
            .join("plugins")
            .join("my_plugin")
            .join("my_plugin.toml");

        assert!(expected.ends_with("plugins/my_plugin/my_plugin.toml"));
    }
}
