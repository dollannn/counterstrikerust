//! # Plugin Settings Example
//!
//! Demonstrates configuration files and FakeConVars.
//!
//! ## Features Demonstrated
//! - `PluginConfig` trait - TOML configuration file loading/saving
//! - `FakeConVar<T>` - Runtime-adjustable settings via console
//! - `ConVar::find()` - Read game engine convars
//! - Builder pattern for FakeConVar constraints
//! - Change callbacks for settings
//!
//! ## Configuration File
//!
//! Creates a TOML config at:
//! `game/csgo/addons/cs2rust/configs/plugins/plugin_settings/plugin_settings.toml`
//!
//! ```toml
//! enabled = true
//! welcome_message = "Welcome to our server!"
//! max_warnings = 3
//! debug = false
//!
//! [features]
//! greetings = true
//! kill_announcements = true
//! vip_features = false
//! ```
//!
//! ## Console Commands (FakeConVars)
//!
//! - `ps_enabled` - Enable/disable plugin (0/1)
//! - `ps_debug` - Enable debug mode (0/1)
//! - `ps_max_warnings` - Set max warnings (1-10)

pub mod config;

use std::sync::{LazyLock, RwLock};

use cs2rust_core::{ConVar, FakeConVar, PluginConfig};
use cs2rust_core::on_map_start;

pub use config::PluginSettingsConfig;

// =============================================================================
// FakeConVars - Runtime-adjustable settings via console commands
// =============================================================================

/// Enable/disable the plugin
///
/// Console: `ps_enabled 1` or `ps_enabled 0`
pub static PS_ENABLED: LazyLock<FakeConVar<bool>> = LazyLock::new(|| {
    FakeConVar::new("ps_enabled", true, "Enable plugin features")
        .with_on_change(|old, new| {
            tracing::info!("Plugin enabled changed: {} -> {}", old, new);
        })
});

/// Enable debug logging
///
/// Console: `ps_debug 1`
pub static PS_DEBUG: LazyLock<FakeConVar<bool>> = LazyLock::new(|| {
    FakeConVar::new("ps_debug", false, "Enable debug output")
});

/// Maximum warnings before action
///
/// Console: `ps_max_warnings 5`
pub static PS_MAX_WARNINGS: LazyLock<FakeConVar<i32>> = LazyLock::new(|| {
    FakeConVar::new("ps_max_warnings", 3, "Maximum warnings before kick")
        .with_min(1)
        .with_max(10)
        .with_on_change(|old, new| {
            tracing::info!("Max warnings changed: {} -> {}", old, new);
        })
});

/// Greeting message (string convar)
///
/// Console: `ps_greeting "Hello!"`
pub static PS_GREETING: LazyLock<FakeConVar<String>> = LazyLock::new(|| {
    FakeConVar::new(
        "ps_greeting",
        "Welcome to the server!".to_string(),
        "Greeting message shown to players",
    )
});

/// Damage multiplier (float convar)
///
/// Console: `ps_damage_mult 1.5`
pub static PS_DAMAGE_MULT: LazyLock<FakeConVar<f32>> = LazyLock::new(|| {
    FakeConVar::new("ps_damage_mult", 1.0, "Damage multiplier")
        .with_min(0.1)
        .with_max(10.0)
});

// =============================================================================
// Runtime configuration storage
// =============================================================================

/// Loaded configuration (can be reloaded at runtime)
static CONFIG: LazyLock<RwLock<PluginSettingsConfig>> = LazyLock::new(|| {
    let config = PluginSettingsConfig::load().unwrap_or_else(|e| {
        tracing::warn!("Failed to load config: {}, using defaults", e);
        PluginSettingsConfig::default()
    });
    RwLock::new(config)
});

// =============================================================================
// Public API
// =============================================================================

/// Initialize the Plugin Settings example.
pub fn init() {
    // Load and log config
    let config = get_config();
    tracing::info!("Plugin Settings initialized:");
    tracing::info!("  Welcome: {}", config.welcome_message);
    tracing::info!("  Max warnings: {}", config.max_warnings);
    tracing::info!("  Features: greetings={}, kills={}, vip={}",
        config.features.greetings,
        config.features.kill_announcements,
        config.features.vip_features
    );

    // Initialize FakeConVars from config
    PS_ENABLED.set(config.enabled);
    PS_DEBUG.set(config.debug);
    PS_MAX_WARNINGS.set(config.max_warnings);
    PS_GREETING.set(config.welcome_message.clone());

    // Force registration of FakeConVars
    let _ = PS_ENABLED.get();
    let _ = PS_DEBUG.get();
    let _ = PS_MAX_WARNINGS.get();
    let _ = PS_GREETING.get();
    let _ = PS_DAMAGE_MULT.get();

    // Demonstrate reading engine convars
    log_engine_convars();

    // Register map start handler
    on_map_start(|_map_name| {
        if PS_ENABLED.get() {
            tracing::info!("[PS] {}", PS_GREETING.get());
        }
    });

    tracing::info!("Plugin Settings initialized! Use ps_* commands to adjust.");
}

/// Get a copy of the current configuration.
pub fn get_config() -> PluginSettingsConfig {
    CONFIG.read().unwrap().clone()
}

/// Reload configuration from disk.
pub fn reload_config() -> Result<(), cs2rust_core::ConfigError> {
    let mut config = CONFIG.write().unwrap();
    config.reload()?;

    // Update FakeConVars to match new config
    PS_ENABLED.set(config.enabled);
    PS_DEBUG.set(config.debug);
    PS_MAX_WARNINGS.set(config.max_warnings);
    PS_GREETING.set(config.welcome_message.clone());

    tracing::info!("Configuration reloaded");
    Ok(())
}

/// Save current FakeConVar values back to config file.
pub fn save_config() -> Result<(), cs2rust_core::ConfigError> {
    let mut config = CONFIG.write().unwrap();

    // Update config from FakeConVars
    config.enabled = PS_ENABLED.get();
    config.debug = PS_DEBUG.get();
    config.max_warnings = PS_MAX_WARNINGS.get();
    config.welcome_message = PS_GREETING.get();

    config.save()?;
    tracing::info!("Configuration saved");
    Ok(())
}

/// Check if plugin is enabled (shorthand).
pub fn is_enabled() -> bool {
    PS_ENABLED.get()
}

/// Check if debug mode is enabled (shorthand).
pub fn is_debug() -> bool {
    PS_DEBUG.get()
}

// =============================================================================
// Internal helpers
// =============================================================================

/// Log some engine convars as demonstration
fn log_engine_convars() {
    // Read some common engine convars
    if let Some(sv_cheats) = ConVar::find("sv_cheats") {
        tracing::info!("Engine: sv_cheats = {}", sv_cheats.get_bool());
    }

    if let Some(mp_autoteambalance) = ConVar::find("mp_autoteambalance") {
        tracing::info!("Engine: mp_autoteambalance = {}", mp_autoteambalance.get_bool());
    }

    if let Some(mp_maxrounds) = ConVar::find("mp_maxrounds") {
        tracing::info!("Engine: mp_maxrounds = {}", mp_maxrounds.get_int());
    }

    if let Some(sv_airaccelerate) = ConVar::find("sv_airaccelerate") {
        tracing::info!("Engine: sv_airaccelerate = {}", sv_airaccelerate.get_float());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = PluginSettingsConfig::default();
        assert!(config.enabled);
        assert_eq!(config.max_warnings, 3);
        assert!(!config.debug);
    }

    #[test]
    fn test_config_serialize() {
        let config = PluginSettingsConfig::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        assert!(toml_str.contains("enabled = true"));
        assert!(toml_str.contains("max_warnings = 3"));
    }
}
