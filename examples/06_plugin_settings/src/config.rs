//! Plugin configuration structure
//!
//! This module defines the TOML configuration file format.

use serde::{Deserialize, Serialize};
use cs2rust_core::PluginConfig;

/// Plugin configuration loaded from TOML file.
///
/// Location: `game/csgo/addons/cs2rust/configs/plugins/plugin_settings/plugin_settings.toml`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PluginSettingsConfig {
    /// Enable the plugin
    pub enabled: bool,

    /// Welcome message shown to players
    pub welcome_message: String,

    /// Maximum number of warnings before kick
    pub max_warnings: i32,

    /// Enable debug logging
    pub debug: bool,

    /// Nested settings section
    pub features: FeatureSettings,
}

/// Feature toggle settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FeatureSettings {
    /// Enable player greetings
    pub greetings: bool,

    /// Enable kill announcements
    pub kill_announcements: bool,

    /// Enable VIP features
    pub vip_features: bool,
}

impl Default for PluginSettingsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            welcome_message: "Welcome to our server!".to_string(),
            max_warnings: 3,
            debug: false,
            features: FeatureSettings::default(),
        }
    }
}

impl Default for FeatureSettings {
    fn default() -> Self {
        Self {
            greetings: true,
            kill_announcements: true,
            vip_features: false,
        }
    }
}

impl PluginConfig for PluginSettingsConfig {
    const PLUGIN_NAME: &'static str = "plugin_settings";
}
