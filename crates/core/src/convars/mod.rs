//! ConVar System - Access game convars and create plugin settings
//!
//! This module provides two main features:
//!
//! 1. **Real ConVar Access** - Read and modify existing game convars like `sv_cheats`
//! 2. **Fake ConVars** - Create plugin-specific settings with validation and callbacks
//!
//! # Real ConVar Example
//!
//! ```ignore
//! use cs2rust_core::convars::ConVar;
//!
//! if let Some(cheats) = ConVar::find("sv_cheats") {
//!     if cheats.get_bool() {
//!         tracing::warn!("Cheats are enabled!");
//!     }
//! }
//! ```
//!
//! # Fake ConVar Example
//!
//! ```ignore
//! use std::sync::LazyLock;
//! use cs2rust_core::convars::FakeConVar;
//!
//! static PLUGIN_ENABLED: LazyLock<FakeConVar<bool>> = LazyLock::new(|| {
//!     FakeConVar::new("my_plugin_enabled", true, "Enable the plugin")
//! });
//!
//! static MAX_PLAYERS: LazyLock<FakeConVar<i32>> = LazyLock::new(|| {
//!     FakeConVar::new("my_plugin_max_players", 10, "Max players in queue")
//!         .with_min(1)
//!         .with_max(64)
//!         .with_on_change(|old, new| {
//!             tracing::info!("Max players changed: {} -> {}", old, new);
//!         })
//! });
//!
//! fn check_enabled() -> bool {
//!     PLUGIN_ENABLED.get()
//! }
//! ```

mod convar;
mod fake;
mod vtable;

// Re-export main types
pub use convar::ConVar;
pub use fake::{ConVarValue, FakeConVar};

// Re-export SDK types for convenience
pub use cs2rust_sdk::convar::{flags, ConVarData, ConVarRef, CVValue, EConVarType, INVALID_CONVAR_INDEX};
