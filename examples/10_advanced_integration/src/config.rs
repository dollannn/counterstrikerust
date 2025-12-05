//! Advanced Integration plugin configuration

use serde::{Deserialize, Serialize};
use cs2rust_core::PluginConfig;

/// Plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AdvancedConfig {
    /// Enable entity lifecycle logging
    pub log_entity_lifecycle: bool,

    /// Enable frame debugging (logs every N frames)
    pub debug_frames: bool,

    /// Frame debug interval (log every N frames)
    pub frame_debug_interval: u64,

    /// VIP spawn armor amount
    pub vip_spawn_armor: i32,

    /// Enable async demo feature
    pub enable_async_demo: bool,
}

impl Default for AdvancedConfig {
    fn default() -> Self {
        Self {
            log_entity_lifecycle: false,
            debug_frames: false,
            frame_debug_interval: 1000,
            vip_spawn_armor: 100,
            enable_async_demo: true,
        }
    }
}

impl PluginConfig for AdvancedConfig {
    const PLUGIN_NAME: &'static str = "advanced_integration";
}
