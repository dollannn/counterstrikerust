//! Permission system for CS2Rust
//!
//! This module provides a centralized permission registry where plugins
//! can register and query player permissions. The framework itself doesn't
//! manage permission lifecycle - plugins are responsible for adding/removing
//! permissions when appropriate (e.g., on player connect/disconnect, config reload).
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                    CS2Rust Core                          │
//! │  ┌─────────────────────────────────────────────────┐    │
//! │  │         Permission Registry (DashMap)            │    │
//! │  │         Key: SteamID64 → PermissionData          │    │
//! │  └─────────────────────────────────────────────────┘    │
//! │       ▲              ▲              │                    │
//! │       │ mutate       │ mutate       │ query              │
//! │       │              │              ▼                    │
//! │  ┌────┴────┐   ┌─────┴─────┐   ┌───────────┐            │
//! │  │Plugin A │   │ Plugin B  │   │ Commands  │            │
//! │  │(MySQL)  │   │ (Config)  │   │ (checks)  │            │
//! │  └─────────┘   └───────────┘   └───────────┘            │
//! └─────────────────────────────────────────────────────────┘
//! ```
//!
//! # Permission Format
//!
//! Permissions use the format `@domain/flag`:
//! - `@css/ban` - Ban permission in the css domain
//! - `@myplugin/vip` - VIP permission for a custom plugin
//! - `@css/root` - Root flag that grants all `@css/*` permissions
//!
//! # Usage
//!
//! ```ignore
//! use cs2rust_core::permissions::{add_permissions, has_permission, clear_permissions};
//!
//! // Plugin loads permissions from database on player connect
//! fn on_player_connect(steam_id: u64) {
//!     let perms = load_from_database(steam_id);
//!     add_permissions(steam_id, &perms);
//! }
//!
//! // Plugin cleans up on disconnect
//! fn on_player_disconnect(steam_id: u64) {
//!     clear_permissions(steam_id);
//! }
//!
//! // Check permissions before an action
//! fn try_ban_player(admin_id: u64, target_id: u64) {
//!     if has_permission(admin_id, "@css/ban") {
//!         // Perform ban
//!     }
//! }
//! ```

mod registry;
mod types;

use std::collections::HashSet;

use crate::entities::PlayerController;

// Re-export types
pub use types::{extract_domain, flags, PermissionData, PERMISSION_PREFIX};

// Re-export registry functions
pub use registry::{
    add_permissions, can_target, clear_all, clear_permissions, get_immunity, get_permissions,
    has_all_permissions, has_any_permission, has_permission, is_registered, player_count,
    remove_permissions, set_immunity, set_permissions,
};

// ============================================================================
// PlayerController Convenience Wrappers
// ============================================================================

/// Add permission(s) to a player controller
pub fn add_player_permissions(player: &PlayerController, permissions: &[&str]) {
    add_permissions(player.steam_id(), permissions);
}

/// Remove permission(s) from a player controller
pub fn remove_player_permissions(player: &PlayerController, permissions: &[&str]) {
    remove_permissions(player.steam_id(), permissions);
}

/// Set all permissions for a player controller (replaces existing)
pub fn set_player_permissions(player: &PlayerController, permissions: &[&str]) {
    set_permissions(player.steam_id(), permissions);
}

/// Clear all permissions for a player controller
pub fn clear_player_permissions(player: &PlayerController) {
    clear_permissions(player.steam_id());
}

/// Set immunity level for a player controller
pub fn set_player_immunity(player: &PlayerController, immunity: u32) {
    set_immunity(player.steam_id(), immunity);
}

/// Check if a player controller has a specific permission
pub fn player_has_permission(player: &PlayerController, permission: &str) -> bool {
    has_permission(player.steam_id(), permission)
}

/// Check if a player controller has any of the given permissions
pub fn player_has_any_permission(player: &PlayerController, permissions: &[&str]) -> bool {
    has_any_permission(player.steam_id(), permissions)
}

/// Check if a player controller has all of the given permissions
pub fn player_has_all_permissions(player: &PlayerController, permissions: &[&str]) -> bool {
    has_all_permissions(player.steam_id(), permissions)
}

/// Get all permissions for a player controller
pub fn get_player_permissions(player: &PlayerController) -> HashSet<String> {
    get_permissions(player.steam_id())
}

/// Get immunity level for a player controller
pub fn get_player_immunity(player: &PlayerController) -> u32 {
    get_immunity(player.steam_id())
}

/// Check if source player can target destination player
pub fn player_can_target(source: &PlayerController, target: &PlayerController) -> bool {
    can_target(source.steam_id(), target.steam_id())
}

/// Check if a player controller has any permissions registered
pub fn player_is_registered(player: &PlayerController) -> bool {
    is_registered(player.steam_id())
}
