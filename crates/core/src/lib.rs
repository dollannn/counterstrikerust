//! CS2 Rust Plugin - Core Logic
//!
//! This crate contains the core initialization and shutdown logic
//! for the CS2 Rust modding framework.
//!
//! # Re-exports
//!
//! This crate re-exports the SDK and engine crates for convenience:
//! - [`sdk`] - Source 2 interface types and version strings
//! - [`engine`] - Engine globals and interface loading

// Allow the crate to refer to itself as `cs2rust_core` for proc macro compatibility
extern crate self as cs2rust_core;

use tracing::info;

// Re-export SDK and engine crates
pub use cs2rust_engine as engine;
pub use cs2rust_sdk as sdk;

pub mod commands;
pub mod config;
pub mod convars;
pub mod entities;
pub mod events;
pub mod gamedata;
pub mod hooks;
pub mod listeners;
pub mod permissions;
pub mod schema;
pub mod tasks;
pub mod timers;

// Re-export commonly used items
pub use commands::{
    register_command, register_server_command, unregister_command, CommandContext, CommandInfo,
    CommandKey, CommandResult,
};
pub use events::{register_event, unregister_event, EventInfo, GameEventRef, HookResult};
pub use hooks::{frame_count, register_gameframe_callback, unregister_gameframe_callback};
pub use hooks::{
    hook, hook_mid, hook_vtable, hook_vtable_direct, HookError, HookKey, HookManager,
    InlineHookKey, MidHookContext, MidHookKey, VTableHookKey,
};
pub use schema::{get_offset, network_state_changed, SchemaError, SchemaField, SchemaObject};
pub use tasks::queue_task;
pub use timers::{add_repeating_timer, add_timer, add_timer_with_flags, remove_timer, TimerFlags, TimerKey};

// Re-export entity types
pub use entities::{BaseEntity, EntityRef, PlayerController, PlayerPawn};

// Re-export listeners
pub use listeners::{
    on_client_connect, on_client_disconnect, on_client_put_in_server, on_entity_created,
    on_entity_deleted, on_entity_spawned, on_map_end, on_map_start, on_tick, remove_listener,
    ListenerKey,
};

// Re-export convar types
pub use convars::{ConVar, ConVarValue, FakeConVar};

// Re-export config types
pub use config::{ConfigError, ConfigResult, CoreConfig, PluginConfig};

// Re-export permission types and functions
pub use permissions::{
    // Mutation (by SteamID)
    add_permissions, clear_permissions, remove_permissions, set_immunity, set_permissions,
    // Query (by SteamID)
    can_target, get_immunity, get_permissions, has_all_permissions, has_any_permission,
    has_permission, is_registered,
    // Mutation (by PlayerController)
    add_player_permissions, clear_player_permissions, remove_player_permissions,
    set_player_immunity, set_player_permissions,
    // Query (by PlayerController)
    get_player_immunity, get_player_permissions, player_can_target, player_has_all_permissions,
    player_has_any_permission, player_has_permission, player_is_registered,
    // Types
    flags as permission_flags, PermissionData,
};

// Re-export macros
pub use cs2rust_macros::{console_command, SchemaClass};

/// Shutdown the plugin
///
/// Called from the FFI layer when Metamod unloads the plugin.
pub fn shutdown() {
    info!("CS2Rust shutting down...");
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_sdk_types_exist() {
        // Verify SDK types are accessible
        use crate::sdk::IServerGameDLL;
        let _: *const IServerGameDLL = std::ptr::null();
    }
}
