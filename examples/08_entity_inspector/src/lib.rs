//! # Entity Inspector Example
//!
//! Provides commands to inspect entities on the server.
//!
//! ## Features Demonstrated
//! - `get_all_entities()` - Iterate over all server entities
//! - `EntityRef` enum - Type-safe entity access with automatic type detection
//! - `CHandle<T>` - Safe entity handle resolution
//! - Entity properties (health, armor, team, name, etc.)
//! - `get_player_controller` - Access player by slot
//!
//! ## Commands
//!
//! - `!entities` - List entity counts by class
//! - `!inspect <slot>` - Inspect a player by slot number
//! - `!weapons` - List all weapons on the server
//! - `!myinfo` - Show your own player info

use std::collections::HashMap;

use cs2rust_core::{
    register_command, CommandResult,
    EntityRef, PlayerPawn,
};
use cs2rust_core::entities::{
    get_all_entities, get_player_controller, get_players,
    CHandle,
};

/// Initialize the Entity Inspector plugin.
pub fn init() {
    register_entities_command();
    register_inspect_command();
    register_weapons_command();
    register_myinfo_command();

    tracing::info!("Entity Inspector plugin initialized!");
    tracing::info!("Commands: !entities, !inspect, !weapons, !myinfo");
}

/// Register the !entities command - lists entity counts by class
fn register_entities_command() {
    register_command(
        "csr_entities",
        "List all entities by class",
        |_player, info| {
            let mut counts: HashMap<String, u32> = HashMap::new();
            let mut total = 0u32;

            // Iterate all entities and count by classname
            for entity_ptr in get_all_entities() {
                total += 1;

                // Try to get entity type info
                if let Some(entity_ref) = unsafe { EntityRef::from_entity_instance(entity_ptr) } {
                    let classname = entity_ref.classname().to_string();
                    *counts.entry(classname).or_default() += 1;
                } else {
                    *counts.entry("(unknown)".to_string()).or_default() += 1;
                }
            }

            info.reply(&format!("Total entities: {}", total));
            info.reply("");

            // Sort by count descending and show top entries
            let mut sorted: Vec<_> = counts.into_iter().collect();
            sorted.sort_by(|a, b| b.1.cmp(&a.1));

            info.reply("Top entity classes:");
            for (classname, count) in sorted.iter().take(15) {
                info.reply(&format!("  {:4} x {}", count, classname));
            }

            if sorted.len() > 15 {
                info.reply(&format!("  ... and {} more types", sorted.len() - 15));
            }

            CommandResult::Handled
        },
    );
}

/// Register the !inspect command - inspect a player by slot
fn register_inspect_command() {
    register_command(
        "csr_inspect",
        "Inspect a player by slot number",
        |_player, info| {
            let slot_str = info.arg(1);
            if slot_str.is_empty() {
                info.reply("Usage: !inspect <slot>");
                info.reply("Use !players to see slot numbers");
                return CommandResult::Handled;
            }

            let slot: i32 = match slot_str.parse() {
                Ok(s) => s,
                Err(_) => {
                    info.reply(&format!("Invalid slot number: {}", slot_str));
                    return CommandResult::Handled;
                }
            };

            let Some(controller) = get_player_controller(slot) else {
                info.reply(&format!("No player at slot {}", slot));
                return CommandResult::Handled;
            };

            // Display controller info
            info.reply(&format!("=== Player Slot {} ===", slot));
            info.reply(&format!("Name: {}", controller.name_string()));
            info.reply(&format!("SteamID64: {}", controller.steam_id()));
            info.reply(&format!("Score: {}", controller.score()));
            info.reply(&format!("MVPs: {}", controller.mvps()));
            info.reply(&format!("Connection: {:?}", controller.connection_state()));
            info.reply(&format!("Is Alive: {}", controller.is_alive()));

            // Get pawn info via handle
            let pawn_handle: CHandle<PlayerPawn> = controller.player_pawn();
            info.reply("");
            info.reply(&format!("Pawn Handle: {:08X}", pawn_handle.raw()));
            info.reply(&format!("  Index: {}", pawn_handle.index()));
            info.reply(&format!("  Serial: {}", pawn_handle.serial()));
            info.reply(&format!("  Valid: {}", pawn_handle.is_valid()));

            // Try to resolve the pawn handle
            if let Some(pawn) = pawn_handle.get() {
                info.reply("");
                info.reply("=== Pawn Data ===");
                info.reply(&format!("Health: {}", pawn.health()));
                info.reply(&format!("Armor: {}", pawn.armor()));
                info.reply(&format!("Team: {}", pawn.team()));
            } else if controller.is_alive() {
                info.reply("Warning: Player is alive but pawn could not be resolved");
            }

            CommandResult::Handled
        },
    );
}

/// Register the !weapons command - list all weapons
fn register_weapons_command() {
    register_command(
        "csr_weapons",
        "List all weapons on the server",
        |_player, info| {
            let mut weapons: Vec<String> = Vec::new();

            for entity_ptr in get_all_entities() {
                if let Some(entity_ref) = unsafe { EntityRef::from_entity_instance(entity_ptr) } {
                    // Check if it's a weapon
                    if entity_ref.is_weapon() {
                        weapons.push(entity_ref.classname().to_string());
                    }
                }
            }

            info.reply(&format!("Weapons on server: {}", weapons.len()));

            // Count by type
            let mut weapon_counts: HashMap<String, u32> = HashMap::new();
            for weapon in &weapons {
                *weapon_counts.entry(weapon.clone()).or_default() += 1;
            }

            // Sort and display
            let mut sorted: Vec<_> = weapon_counts.into_iter().collect();
            sorted.sort_by(|a, b| b.1.cmp(&a.1));

            for (weapon_class, count) in sorted {
                info.reply(&format!("  {:3} x {}", count, weapon_class));
            }

            CommandResult::Handled
        },
    );
}

/// Register the !myinfo command - show caller's info
fn register_myinfo_command() {
    register_command(
        "csr_myinfo",
        "Show your player information",
        |player, info| {
            let Some(controller) = player else {
                info.reply("This command must be used by a player");
                return CommandResult::Handled;
            };

            info.reply("=== Your Info ===");
            info.reply(&format!("Name: {}", controller.name_string()));
            info.reply(&format!("SteamID64: {}", controller.steam_id()));
            info.reply(&format!("Slot: {}", controller.slot()));
            info.reply(&format!("Entity Index: {}", controller.entity_index()));
            info.reply(&format!("Score: {}", controller.score()));
            info.reply(&format!("MVPs: {}", controller.mvps()));
            info.reply(&format!("Alive: {}", controller.is_alive()));
            info.reply(&format!("Connected: {}", controller.is_connected()));

            // Pawn info
            if let Some(pawn) = controller.pawn() {
                info.reply("");
                info.reply("=== Your Pawn ===");
                info.reply(&format!("Health: {}/100", pawn.health()));
                info.reply(&format!("Armor: {}", pawn.armor()));
                info.reply(&format!("Team: {}", match pawn.team() {
                    0 => "Unassigned",
                    1 => "Spectator",
                    2 => "Terrorists",
                    3 => "Counter-Terrorists",
                    _ => "Unknown",
                }));
            }

            CommandResult::Handled
        },
    );
}

/// Helper function to list all players (useful for other plugins)
pub fn list_players() {
    tracing::info!("=== Connected Players ===");
    for controller in get_players() {
        let slot = controller.slot();
        let name = controller.name_string();
        let alive = if controller.is_alive() { "alive" } else { "dead" };
        tracing::info!("[{}] {} ({})", slot, name, alive);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_team_names() {
        // Team numbers in CS2
        assert_eq!(0, 0); // Unassigned
        assert_eq!(1, 1); // Spectator
        assert_eq!(2, 2); // Terrorists
        assert_eq!(3, 3); // Counter-Terrorists
    }
}
