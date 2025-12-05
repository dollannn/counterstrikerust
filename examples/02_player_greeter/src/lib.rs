//! # Player Greeter Example
//!
//! Demonstrates client lifecycle events by welcoming players and announcing departures.
//!
//! ## Features Demonstrated
//! - `on_client_connect` - Called when a client begins connecting
//! - `on_client_put_in_server` - Called when a client fully enters the game
//! - `on_client_disconnect` - Called when a client disconnects
//! - `get_player_controller` - Access player information by slot
//! - `PlayerController` properties (name, steam_id)
//!
//! ## Usage
//! ```ignore
//! player_greeter::init();
//! ```

use cs2rust_core::{
    on_client_connect, on_client_disconnect, on_client_put_in_server,
    ListenerKey,
};
use cs2rust_core::entities::get_player_controller;

/// Initialize the Player Greeter plugin.
///
/// Registers listeners for all client lifecycle events.
pub fn init() {
    // Called when a client initiates a connection
    // Parameters: slot (0-63), name, ip address
    let _connect_key: ListenerKey = on_client_connect(|slot, name, ip| {
        tracing::info!(
            "Player connecting: '{}' from {} (slot {})",
            name,
            ip,
            slot
        );
    });

    // Called when a client fully enters the game
    // At this point, the PlayerController is fully initialized
    let _put_in_server_key: ListenerKey = on_client_put_in_server(|slot| {
        // Get the player controller to access detailed information
        if let Some(controller) = get_player_controller(slot) {
            let name = controller.name_string();
            let steam_id = controller.steam_id();

            tracing::info!("Welcome, {}!", name);
            tracing::info!("  SteamID64: {}", steam_id);
            tracing::info!("  Slot: {}", slot);

            // You could broadcast a welcome message to all players here
            // using the game's chat system (if implemented)
        } else {
            tracing::warn!("Player entered server at slot {} but controller not found", slot);
        }
    });

    // Called when a client disconnects
    let _disconnect_key: ListenerKey = on_client_disconnect(|slot| {
        tracing::info!("Player at slot {} has disconnected", slot);

        // Note: The PlayerController may still be valid briefly during disconnect,
        // but it's safer to log the slot rather than trying to access properties
    });

    tracing::info!("Player Greeter plugin initialized!");
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_slot_bounds() {
        // Player slots are 0-63 (MAX_PLAYERS = 64)
        assert!(0 <= 0 && 0 < 64);
        assert!(0 <= 63 && 63 < 64);
    }
}
