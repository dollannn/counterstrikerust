//! Configuration system for CS2Rust
//!
//! This module provides a trait-based configuration system that supports:
//! - Type-safe config structs via serde
//! - TOML file format
//! - Auto-generation of default configs
//! - Manual reload capability
//!
//! # Example
//!
//! ```ignore
//! use serde::{Deserialize, Serialize};
//! use cs2rust_core::PluginConfig;
//!
//! #[derive(Default, Serialize, Deserialize)]
//! pub struct MyPluginConfig {
//!     pub max_players: i32,
//!     pub welcome_message: String,
//! }
//!
//! impl PluginConfig for MyPluginConfig {
//!     const PLUGIN_NAME: &'static str = "my_plugin";
//! }
//!
//! fn load_config() {
//!     let config = MyPluginConfig::load().unwrap_or_default();
//!     println!("Max players: {}", config.max_players);
//! }
//! ```

mod loader;

use serde::{de::DeserializeOwned, Deserialize, Serialize};

pub use loader::{configs_dir, core_config_path, cs2rust_base_dir, plugin_config_path};

/// Configuration system errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// Failed to read or write config file
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Failed to parse TOML content
    #[error("Failed to parse TOML: {0}")]
    ParseError(#[from] toml::de::Error),

    /// Failed to serialize config to TOML
    #[error("Failed to serialize config: {0}")]
    SerializeError(#[from] toml::ser::Error),

    /// Could not determine config directory from plugin location
    #[error("Config directory not available - could not resolve plugin base path")]
    NoConfigDirectory,
}

/// Result type for config operations
pub type ConfigResult<T> = Result<T, ConfigError>;

/// Trait for plugin configuration types.
///
/// Implement this trait on your config struct to enable automatic loading,
/// saving, and reloading of configuration files.
///
/// # Requirements
///
/// Your config type must implement:
/// - `Default` - for generating initial config files
/// - `Serialize` - for saving to TOML
/// - `DeserializeOwned` - for loading from TOML
/// - `Send + Sync` - for thread-safe access
///
/// # File Location
///
/// Configs are stored at:
/// `game/csgo/addons/cs2rust/configs/plugins/{PLUGIN_NAME}/{PLUGIN_NAME}.toml`
pub trait PluginConfig: Default + Serialize + DeserializeOwned + Send + Sync {
    /// The plugin name used for config file path resolution.
    ///
    /// This determines the config file location:
    /// `configs/plugins/{PLUGIN_NAME}/{PLUGIN_NAME}.toml`
    const PLUGIN_NAME: &'static str;

    /// Load config from file, creating default if missing.
    ///
    /// If the config file doesn't exist, a default config is created and saved.
    fn load() -> ConfigResult<Self> {
        let path = plugin_config_path(Self::PLUGIN_NAME)?;

        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let config: Self = toml::from_str(&content)?;
            tracing::debug!("Loaded config for {} from {:?}", Self::PLUGIN_NAME, path);
            Ok(config)
        } else {
            let default = Self::default();
            default.save()?;
            tracing::info!(
                "Created default config for {} at {:?}",
                Self::PLUGIN_NAME,
                path
            );
            Ok(default)
        }
    }

    /// Save config to file.
    ///
    /// Creates parent directories if they don't exist.
    fn save(&self) -> ConfigResult<()> {
        let path = plugin_config_path(Self::PLUGIN_NAME)?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        tracing::debug!("Saved config for {} to {:?}", Self::PLUGIN_NAME, path);
        Ok(())
    }

    /// Reload config from file.
    ///
    /// Updates self with the current file contents.
    fn reload(&mut self) -> ConfigResult<()> {
        let path = plugin_config_path(Self::PLUGIN_NAME)?;
        let content = std::fs::read_to_string(&path)?;
        *self = toml::from_str(&content)?;
        tracing::debug!("Reloaded config for {} from {:?}", Self::PLUGIN_NAME, path);
        Ok(())
    }
}

/// Core framework configuration.
///
/// This config controls framework-level settings and is loaded from:
/// `game/csgo/addons/cs2rust/configs/core.toml`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CoreConfig {
    /// Config version for future migration support
    pub version: u32,

    /// Enable debug logging
    pub debug: bool,
}

impl Default for CoreConfig {
    fn default() -> Self {
        Self {
            version: 1,
            debug: false,
        }
    }
}

impl CoreConfig {
    /// Load core config from file, creating default if missing.
    ///
    /// Uses the core config path instead of the plugin config path.
    pub fn load() -> ConfigResult<Self> {
        let path = core_config_path()?;

        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let config: Self = toml::from_str(&content)?;
            tracing::debug!("Loaded core config from {:?}", path);
            Ok(config)
        } else {
            let default = Self::default();
            default.save()?;
            tracing::info!("Created default core config at {:?}", path);
            Ok(default)
        }
    }

    /// Save core config to file.
    pub fn save(&self) -> ConfigResult<()> {
        let path = core_config_path()?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        tracing::debug!("Saved core config to {:?}", path);
        Ok(())
    }

    /// Reload core config from file.
    pub fn reload(&mut self) -> ConfigResult<()> {
        let path = core_config_path()?;
        let content = std::fs::read_to_string(&path)?;
        *self = toml::from_str(&content)?;
        tracing::debug!("Reloaded core config from {:?}", path);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default, Debug, PartialEq, Serialize, Deserialize)]
    struct TestConfig {
        pub value: i32,
        pub name: String,
    }

    #[test]
    fn test_config_serialize_deserialize() {
        let config = TestConfig {
            value: 42,
            name: "test".to_string(),
        };

        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: TestConfig = toml::from_str(&toml_str).unwrap();

        assert_eq!(config, parsed);
    }

    #[test]
    fn test_core_config_default() {
        let config = CoreConfig::default();
        assert_eq!(config.version, 1);
        assert!(!config.debug);
    }

    #[test]
    fn test_core_config_serialize() {
        let config = CoreConfig {
            version: 2,
            debug: true,
        };

        let toml_str = toml::to_string_pretty(&config).unwrap();
        assert!(toml_str.contains("version = 2"));
        assert!(toml_str.contains("debug = true"));
    }
}
