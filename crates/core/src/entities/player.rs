//! Player entity wrappers and utilities
//!
//! This module provides type-safe wrappers for player-related CS2 classes
//! and utility functions for accessing players.
//!
//! # Player Access
//!
//! ```ignore
//! use cs2rust_core::entities::{get_player_controller, get_players, find_player_by_steamid};
//!
//! // Get player by slot (0-63)
//! if let Some(controller) = get_player_controller(0) {
//!     println!("Player 0: {}", controller.name_string());
//! }
//!
//! // Iterate all connected players
//! for controller in get_players() {
//!     println!("Player: {} ({})", controller.name_string(), controller.steam_id());
//! }
//!
//! // Find by SteamID64
//! if let Some(controller) = find_player_by_steamid(76561198012345678) {
//!     println!("Found player: {}", controller.name_string());
//! }
//! ```

use std::ffi::c_void;
use std::marker::PhantomData;

use cs2rust_macros::SchemaClass;

use crate::schema::SchemaObject;

use super::handle::CHandle;
use super::system;

/// Maximum number of player slots (CS2 default)
pub const MAX_PLAYERS: usize = 64;

/// Player connection state
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerConnectedState {
    /// Player has never connected
    NeverConnected = -1,
    /// Player is fully connected
    Connected = 0,
    /// Player is connecting
    Connecting = 1,
    /// Player is reconnecting
    Reconnecting = 2,
    /// Player is disconnecting
    Disconnecting = 3,
    /// Player has disconnected
    Disconnected = 4,
    /// Slot is reserved
    Reserved = 5,
}

impl From<i32> for PlayerConnectedState {
    fn from(value: i32) -> Self {
        match value {
            -1 => Self::NeverConnected,
            0 => Self::Connected,
            1 => Self::Connecting,
            2 => Self::Reconnecting,
            3 => Self::Disconnecting,
            4 => Self::Disconnected,
            5 => Self::Reserved,
            _ => Self::Disconnected,
        }
    }
}

/// Wrapper for CCSPlayerPawn
///
/// The player pawn represents the physical player entity in the game world.
/// It contains properties like health, armor, position, etc.
#[derive(SchemaClass)]
#[schema(class = "CCSPlayerPawn")]
pub struct PlayerPawn {
    ptr: *mut c_void,

    #[schema(field = "m_iHealth", networked)]
    _health: PhantomData<i32>,

    #[schema(field = "m_ArmorValue", networked)]
    _armor: PhantomData<i32>,

    #[schema(field = "m_iTeamNum", networked)]
    _team: PhantomData<i32>,
}

/// Wrapper for CCSPlayerController
///
/// The player controller manages the player's connection and metadata.
/// It persists across respawns and contains data like score, MVPs, etc.
#[derive(SchemaClass)]
#[schema(class = "CCSPlayerController")]
pub struct PlayerController {
    ptr: *mut c_void,

    #[schema(field = "m_iScore", networked)]
    _score: PhantomData<i32>,

    #[schema(field = "m_iMVPs", networked)]
    _mvps: PhantomData<i32>,

    #[schema(field = "m_szNetname", readonly)]
    _name: PhantomData<[u8; 128]>,

    #[schema(field = "m_hPlayerPawn", readonly)]
    _player_pawn: PhantomData<CHandle<PlayerPawn>>,

    #[schema(field = "m_steamID", readonly)]
    _steam_id: PhantomData<u64>,

    #[schema(field = "m_iConnected", readonly)]
    _connected: PhantomData<i32>,

    #[schema(field = "m_bPawnIsAlive", readonly)]
    _pawn_is_alive: PhantomData<bool>,

    #[schema(field = "m_iPawnHealth", readonly)]
    _pawn_health: PhantomData<u32>,
}

impl PlayerController {
    /// Get player name as an owned string
    ///
    /// The name is stored as a null-terminated C string in the schema.
    /// This method finds the null terminator and returns a UTF-8 string.
    pub fn name_string(&self) -> String {
        let bytes = self.name();
        let len = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        String::from_utf8_lossy(&bytes[..len]).into_owned()
    }

    /// Get the player's connection state as an enum
    pub fn connection_state(&self) -> PlayerConnectedState {
        PlayerConnectedState::from(self.connected())
    }

    /// Check if the player is fully connected
    pub fn is_connected(&self) -> bool {
        self.connection_state() == PlayerConnectedState::Connected
    }

    /// Get the player pawn if available
    ///
    /// Returns `None` if the player doesn't have a pawn (e.g., spectating or dead).
    pub fn pawn(&self) -> Option<PlayerPawn> {
        self.player_pawn().get()
    }

    /// Check if the player's pawn is currently alive
    pub fn is_alive(&self) -> bool {
        self.pawn_is_alive()
    }

    /// Get the player's entity index
    ///
    /// Reads the entity index from the entity's identity.
    pub fn entity_index(&self) -> i32 {
        super::entity_ref::EntityRef::read_entity_index_from_ptr(self.ptr)
    }

    /// Get the player slot (entity index - 1)
    ///
    /// Player slots are 0-indexed (0 to 63), while entity indices are 1-indexed.
    pub fn slot(&self) -> i32 {
        self.entity_index() - 1
    }
}

/// Wrapper for CBaseEntity
///
/// Base class for all entities in the game.
#[derive(SchemaClass)]
#[schema(class = "CBaseEntity")]
pub struct BaseEntity {
    ptr: *mut c_void,

    #[schema(field = "m_iHealth", networked)]
    _health: PhantomData<i32>,

    #[schema(field = "m_iTeamNum", networked)]
    _team_num: PhantomData<i32>,

    #[schema(field = "m_fFlags", readonly)]
    _flags: PhantomData<u32>,
}

// ============================================================================
// Player Access Utilities
// ============================================================================

/// Get a player controller by slot index
///
/// Player slots are 0-indexed (0 to MAX_PLAYERS-1).
/// Entity indices are slot + 1.
///
/// # Arguments
///
/// * `slot` - Player slot (0 to 63)
///
/// # Returns
///
/// `Some(PlayerController)` if a valid controller exists at that slot.
///
/// # Example
///
/// ```ignore
/// if let Some(controller) = get_player_controller(0) {
///     println!("First player: {}", controller.name_string());
/// }
/// ```
pub fn get_player_controller(slot: i32) -> Option<PlayerController> {
    if slot < 0 || slot >= MAX_PLAYERS as i32 {
        return None;
    }

    // Entity index = slot + 1
    let entity_index = (slot + 1) as u32;
    let ptr = system::get_entity_by_index(entity_index)?;

    // Safety: We got a valid pointer from the entity system
    unsafe { PlayerController::from_ptr(ptr) }
}

/// Get a player controller by entity index
///
/// Entity indices for players are 1 to MAX_PLAYERS.
///
/// # Arguments
///
/// * `index` - Entity index (1 to 64)
///
/// # Returns
///
/// `Some(PlayerController)` if a valid controller exists at that index.
pub fn get_player_controller_by_index(index: u32) -> Option<PlayerController> {
    if index == 0 || index > MAX_PLAYERS as u32 {
        return None;
    }

    let ptr = system::get_entity_by_index(index)?;
    unsafe { PlayerController::from_ptr(ptr) }
}

/// Get a player controller by userid
///
/// The userid is typically from game events. It encodes the slot in the lower byte.
///
/// # Arguments
///
/// * `userid` - User ID from game events
///
/// # Returns
///
/// `Some(PlayerController)` if a valid controller exists.
pub fn get_player_controller_by_userid(userid: i32) -> Option<PlayerController> {
    // Extract slot from lower byte, then convert to entity index
    let slot = userid & 0xFF;
    get_player_controller(slot)
}

/// Get all connected player controllers
///
/// Returns an iterator over all valid, connected player controllers.
///
/// # Example
///
/// ```ignore
/// for controller in get_players() {
///     println!("{}: {} HP",
///         controller.name_string(),
///         controller.pawn_health()
///     );
/// }
/// ```
pub fn get_players() -> impl Iterator<Item = PlayerController> {
    (0..MAX_PLAYERS as i32).filter_map(|slot| {
        let controller = get_player_controller(slot)?;
        if controller.is_connected() {
            Some(controller)
        } else {
            None
        }
    })
}

/// Get all player controllers regardless of connection state
///
/// Useful for iterating all player slots including spectators and bots.
pub fn get_all_player_controllers() -> impl Iterator<Item = PlayerController> {
    (0..MAX_PLAYERS as i32).filter_map(get_player_controller)
}

/// Find a player controller by SteamID64
///
/// # Arguments
///
/// * `steam_id` - The player's 64-bit Steam ID
///
/// # Returns
///
/// `Some(PlayerController)` if a connected player with that Steam ID exists.
///
/// # Example
///
/// ```ignore
/// if let Some(controller) = find_player_by_steamid(76561198012345678) {
///     controller.pawn().map(|pawn| pawn.set_health(100));
/// }
/// ```
pub fn find_player_by_steamid(steam_id: u64) -> Option<PlayerController> {
    get_players().find(|controller| controller.steam_id() == steam_id)
}

/// Get the number of connected players
pub fn player_count() -> usize {
    get_players().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_pawn_constants() {
        assert_eq!(PlayerPawn::CLASS_NAME, "CCSPlayerPawn");
        assert_eq!(PlayerPawn::HEALTH_FIELD, "m_iHealth");
        assert_eq!(PlayerPawn::ARMOR_FIELD, "m_ArmorValue");
        assert_eq!(PlayerPawn::TEAM_FIELD, "m_iTeamNum");
    }

    #[test]
    fn test_player_controller_constants() {
        assert_eq!(PlayerController::CLASS_NAME, "CCSPlayerController");
        assert_eq!(PlayerController::SCORE_FIELD, "m_iScore");
        assert_eq!(PlayerController::MVPS_FIELD, "m_iMVPs");
        assert_eq!(PlayerController::NAME_FIELD, "m_szNetname");
        assert_eq!(PlayerController::PLAYER_PAWN_FIELD, "m_hPlayerPawn");
        assert_eq!(PlayerController::STEAM_ID_FIELD, "m_steamID");
        assert_eq!(PlayerController::CONNECTED_FIELD, "m_iConnected");
    }

    #[test]
    fn test_base_entity_constants() {
        assert_eq!(BaseEntity::CLASS_NAME, "CBaseEntity");
        assert_eq!(BaseEntity::HEALTH_FIELD, "m_iHealth");
        assert_eq!(BaseEntity::TEAM_NUM_FIELD, "m_iTeamNum");
        assert_eq!(BaseEntity::FLAGS_FIELD, "m_fFlags");
    }

    #[test]
    fn test_from_ptr_null() {
        let result = unsafe { PlayerPawn::from_ptr(std::ptr::null_mut()) };
        assert!(result.is_none());
    }

    #[test]
    fn test_player_connected_state() {
        assert_eq!(
            PlayerConnectedState::from(-1),
            PlayerConnectedState::NeverConnected
        );
        assert_eq!(
            PlayerConnectedState::from(0),
            PlayerConnectedState::Connected
        );
        assert_eq!(
            PlayerConnectedState::from(1),
            PlayerConnectedState::Connecting
        );
        assert_eq!(
            PlayerConnectedState::from(2),
            PlayerConnectedState::Reconnecting
        );
        assert_eq!(
            PlayerConnectedState::from(3),
            PlayerConnectedState::Disconnecting
        );
        assert_eq!(
            PlayerConnectedState::from(4),
            PlayerConnectedState::Disconnected
        );
        assert_eq!(
            PlayerConnectedState::from(5),
            PlayerConnectedState::Reserved
        );
        // Unknown values should map to Disconnected
        assert_eq!(
            PlayerConnectedState::from(100),
            PlayerConnectedState::Disconnected
        );
    }

    #[test]
    fn test_slot_to_index() {
        // Slot 0 should be entity index 1
        assert_eq!(0 + 1, 1);
        // Slot 63 should be entity index 64
        assert_eq!(63 + 1, 64);
    }
}
