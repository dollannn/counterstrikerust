//! Entity wrappers using SchemaClass derive
//!
//! This module provides type-safe wrappers for CS2 entity classes using
//! the `#[derive(SchemaClass)]` macro. Each wrapper provides getter/setter
//! methods for schema fields with automatic offset resolution.
//!
//! # Entity System
//!
//! The entity system provides access to game entities:
//!
//! ```ignore
//! use cs2rust_core::entities::{get_entity_by_index, get_all_entities};
//!
//! // Get entity by index
//! if let Some(ptr) = get_entity_by_index(5) {
//!     let entity = unsafe { BaseEntity::from_ptr(ptr) };
//! }
//!
//! // Iterate all entities
//! for entity_ptr in get_all_entities() {
//!     if let Some(entity_ref) = unsafe { EntityRef::from_entity_instance(entity_ptr) } {
//!         println!("Entity: {}", entity_ref.classname());
//!     }
//! }
//! ```
//!
//! # Player Access
//!
//! ```ignore
//! use cs2rust_core::entities::{get_player_controller, get_players, find_player_by_steamid};
//!
//! // Get player by slot
//! if let Some(controller) = get_player_controller(0) {
//!     println!("Player: {}", controller.name_string());
//! }
//!
//! // Iterate connected players
//! for controller in get_players() {
//!     println!("{} - {} HP", controller.name_string(), controller.pawn_health());
//! }
//! ```
//!
//! # Entity Handles
//!
//! Handles provide safe references to entities that may be deleted:
//!
//! ```ignore
//! use cs2rust_core::entities::{CHandle, PlayerPawn};
//!
//! let pawn_handle: CHandle<PlayerPawn> = controller.player_pawn();
//!
//! // Resolve to get the pawn (None if deleted)
//! if let Some(pawn) = pawn_handle.get() {
//!     pawn.set_health(100);
//! }
//! ```
//!
//! # Example
//!
//! ```ignore
//! use cs2rust_core::entities::{PlayerPawn, PlayerController};
//!
//! // Get a player pawn pointer from the entity system
//! let pawn_ptr: *mut c_void = /* ... */;
//!
//! if let Some(mut pawn) = unsafe { PlayerPawn::from_ptr(pawn_ptr) } {
//!     // Read health
//!     let health = pawn.health();
//!     println!("Player health: {}", health);
//!
//!     // Set health (auto-triggers network state change)
//!     pawn.set_health(100);
//! }
//! ```

pub mod entity_ref;
pub mod handle;
pub mod player;
pub mod system;

// Re-export entity types
pub use entity_ref::EntityRef;
pub use player::{BaseEntity, PlayerController, PlayerPawn};

// Re-export handle types
pub use handle::{CEntityHandle, CHandle};
pub use handle::{INVALID_EHANDLE_INDEX, MAX_EDICTS, MAX_EDICT_BITS, NUM_SERIAL_NUMBER_BITS};

// Re-export player utilities
pub use player::{
    find_player_by_steamid, get_all_player_controllers, get_player_controller,
    get_player_controller_by_index, get_player_controller_by_userid, get_players, player_count,
    PlayerConnectedState, MAX_PLAYERS,
};

// Re-export entity system functions
pub use system::{
    get_all_entities, get_entity_by_handle, get_entity_by_index, get_handle_from_entity,
    is_available, EntityIterator, MAX_CHUNKS, MAX_ENTITIES, MAX_ENTITIES_PER_CHUNK,
};
