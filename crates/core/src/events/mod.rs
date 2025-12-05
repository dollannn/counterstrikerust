//! Game Event System
//!
//! Subscribe to and handle Source 2 game events (player_death, round_start, etc.)
//!
//! # Architecture
//!
//! ```text
//! IGameEventManager2 → FireEvent hook → Event dispatcher → Rust callbacks
//! ```
//!
//! # Example
//!
//! ```ignore
//! use cs2rust_core::events::{register_event, HookResult, GameEventRef};
//!
//! // Register a handler for player_death events
//! register_event("player_death", false, |event, info| {
//!     let attacker = event.get_string("attacker_name", "");
//!     let victim = event.get_string("victim_name", "");
//!     tracing::info!("{} killed {}", attacker, victim);
//!     HookResult::Continue
//! });
//!
//! // Or use typed events for better ergonomics:
//! use cs2rust_core::events::typed::{EventPlayerDeath, register_typed_event};
//!
//! register_typed_event::<EventPlayerDeath, _>(false, |event, info| {
//!     if event.headshot {
//!         tracing::info!("Headshot kill with {}", event.weapon);
//!     }
//!     HookResult::Continue
//! });
//! ```

mod manager;
mod raw;
pub mod typed;
mod types;

pub use manager::{register_event, set_game_event_manager, unregister_event, EventManager, EVENTS};
pub use raw::GameEventRef;
pub use types::{EventCallback, EventInfo, HookResult};

// Re-export common typed events
pub use typed::{
    register_typed_event, EventBombDefused, EventBombExploded, EventBombPlanted,
    EventPlayerConnect, EventPlayerDeath, EventPlayerDisconnect, EventPlayerHurt, EventPlayerSpawn,
    EventPlayerTeam, EventRoundEnd, EventRoundFreezeEnd, EventRoundStart, EventWeaponFire,
    GameEvent,
};

/// Initialize the event system
///
/// Called during plugin startup after engine interfaces are available.
/// Sets up hooks on IGameEventManager2::FireEvent.
pub fn init() -> Result<(), crate::hooks::HookError> {
    manager::init_event_hooks()
}

/// Shutdown the event system
///
/// Called during plugin unload. Removes all hooks and cleans up.
pub fn shutdown() {
    manager::shutdown_event_hooks();
}
