//! # Admin System Example
//!
//! Demonstrates the permission system with admin commands.
//!
//! ## Features Demonstrated
//! - `has_permission` - Check if player has a permission flag
//! - `add_permissions` - Grant permissions to a player
//! - `set_immunity` - Set player's immunity level
//! - `can_target` - Check if admin can target another player (immunity)
//! - `permission_flags` - Built-in permission flag constants
//! - Entity manipulation (`PlayerPawn.set_health()`)
//!
//! ## Permission Flags
//!
//! Built-in flags follow the `@domain/permission` format:
//! - `@css/root` - Root admin (all permissions)
//! - `@css/kick` - Can kick players
//! - `@css/ban` - Can ban players
//! - `@css/slay` - Can slay/damage players
//! - `@css/changemap` - Can change maps
//!
//! ## Commands
//!
//! - `!slay <player>` - Kill a player (requires @css/slay)
//! - `!kick <player>` - Kick a player (requires @css/kick)
//! - `!heal <player>` - Heal a player (requires @css/slay)
//! - `!admin` - Check your admin status

use cs2rust_core::{
    register_command, CommandResult,
    has_permission, can_target, add_permissions, set_immunity,
    get_permissions, get_immunity,
    permission_flags as flags,
};
use cs2rust_core::entities::{get_players, PlayerController};

/// Initialize the Admin System plugin.
///
/// Registers admin commands and sets up test admins.
pub fn init() {
    // Register admin commands
    register_slay_command();
    register_kick_command();
    register_heal_command();
    register_admin_status_command();
    register_grant_admin_command();

    tracing::info!("Admin System plugin initialized!");
    tracing::info!("Commands: !slay, !kick, !heal, !admin, !grantadmin");
}

/// Helper to find a player by partial name match
fn find_target_player(name: &str) -> Option<PlayerController> {
    let name_lower = name.to_lowercase();

    get_players().find(|p| {
        p.name_string().to_lowercase().contains(&name_lower)
    })
}

/// Register the !slay command
fn register_slay_command() {
    register_command(
        "csr_slay",
        "Slay a player (requires @css/slay)",
        |player, info| {
            // Get admin player
            let Some(admin) = player else {
                info.reply("This command cannot be used from server console");
                return CommandResult::Handled;
            };

            // Check permission
            let admin_steam_id = admin.steam_id();
            if !has_permission(admin_steam_id, flags::SLAY) {
                info.reply("You don't have permission to use this command");
                info.reply(&format!("Required: {}", flags::SLAY));
                return CommandResult::Handled;
            }

            // Get target name
            let target_name = info.arg(1);
            if target_name.is_empty() {
                info.reply("Usage: !slay <player>");
                return CommandResult::Handled;
            }

            // Find target player
            let Some(target) = find_target_player(target_name) else {
                info.reply(&format!("Player '{}' not found", target_name));
                return CommandResult::Handled;
            };

            // Check immunity
            let target_steam_id = target.steam_id();
            if !can_target(admin_steam_id, target_steam_id) {
                info.reply("You cannot target this player (higher immunity)");
                return CommandResult::Handled;
            }

            // Slay the player by setting health to 0
            if let Some(mut pawn) = target.pawn() {
                pawn.set_health(0);
                info.reply(&format!("Slayed {}", target.name_string()));
                tracing::info!(
                    "Admin {} slayed {}",
                    admin.name_string(),
                    target.name_string()
                );
            } else {
                info.reply(&format!("{} is not alive", target.name_string()));
            }

            CommandResult::Handled
        },
    );
}

/// Register the !kick command
fn register_kick_command() {
    register_command(
        "csr_kick",
        "Kick a player (requires @css/kick)",
        |player, info| {
            let Some(admin) = player else {
                info.reply("This command cannot be used from server console");
                return CommandResult::Handled;
            };

            let admin_steam_id = admin.steam_id();
            if !has_permission(admin_steam_id, flags::KICK) {
                info.reply("You don't have permission to use this command");
                info.reply(&format!("Required: {}", flags::KICK));
                return CommandResult::Handled;
            }

            let target_name = info.arg(1);
            if target_name.is_empty() {
                info.reply("Usage: !kick <player> [reason]");
                return CommandResult::Handled;
            }

            let Some(target) = find_target_player(target_name) else {
                info.reply(&format!("Player '{}' not found", target_name));
                return CommandResult::Handled;
            };

            let target_steam_id = target.steam_id();
            if !can_target(admin_steam_id, target_steam_id) {
                info.reply("You cannot target this player (higher immunity)");
                return CommandResult::Handled;
            }

            // Get kick reason (all args after player name)
            let reason = if info.arg_count() > 2 {
                info.args()[2..].join(" ")
            } else {
                "Kicked by admin".to_string()
            };

            // In a real implementation, you would call the engine's kick function
            // For now, we just log it
            info.reply(&format!("Kicked {} ({})", target.name_string(), reason));
            tracing::info!(
                "Admin {} kicked {}: {}",
                admin.name_string(),
                target.name_string(),
                reason
            );

            CommandResult::Handled
        },
    );
}

/// Register the !heal command
fn register_heal_command() {
    register_command(
        "csr_heal",
        "Heal a player to full health (requires @css/slay)",
        |player, info| {
            let Some(admin) = player else {
                info.reply("This command cannot be used from server console");
                return CommandResult::Handled;
            };

            let admin_steam_id = admin.steam_id();
            if !has_permission(admin_steam_id, flags::SLAY) {
                info.reply("You don't have permission to use this command");
                return CommandResult::Handled;
            }

            let target_name = info.arg(1);
            if target_name.is_empty() {
                info.reply("Usage: !heal <player>");
                return CommandResult::Handled;
            }

            let Some(target) = find_target_player(target_name) else {
                info.reply(&format!("Player '{}' not found", target_name));
                return CommandResult::Handled;
            };

            let target_steam_id = target.steam_id();
            if !can_target(admin_steam_id, target_steam_id) {
                info.reply("You cannot target this player (higher immunity)");
                return CommandResult::Handled;
            }

            if let Some(mut pawn) = target.pawn() {
                pawn.set_health(100);
                pawn.set_armor(100);
                info.reply(&format!("Healed {}", target.name_string()));
            } else {
                info.reply(&format!("{} is not alive", target.name_string()));
            }

            CommandResult::Handled
        },
    );
}

/// Register the !admin command to check admin status
fn register_admin_status_command() {
    register_command(
        "csr_admin",
        "Check your admin status",
        |player, info| {
            let Some(p) = player else {
                info.reply("Server console has full access");
                return CommandResult::Handled;
            };

            let steam_id = p.steam_id();
            let perms = get_permissions(steam_id);
            let immunity = get_immunity(steam_id);

            if perms.is_empty() {
                info.reply("You are not an admin");
            } else {
                info.reply(&format!("Your permissions ({}):", perms.len()));
                for perm in &perms {
                    info.reply(&format!("  - {}", perm));
                }
                info.reply(&format!("Immunity level: {}", immunity));
            }

            CommandResult::Handled
        },
    );
}

/// Register a command to grant admin (for testing)
fn register_grant_admin_command() {
    register_command(
        "csr_grantadmin",
        "Grant yourself admin (server console only)",
        |player, info| {
            // Only allow from server console for safety
            if player.is_some() {
                info.reply("This command can only be used from server console");
                return CommandResult::Handled;
            }

            let target_name = info.arg(1);
            if target_name.is_empty() {
                info.reply("Usage: csr_grantadmin <player> [level]");
                info.reply("Levels: basic, full, root");
                return CommandResult::Handled;
            }

            let Some(target) = find_target_player(target_name) else {
                info.reply(&format!("Player '{}' not found", target_name));
                return CommandResult::Handled;
            };

            let level = info.arg(2);
            let steam_id = target.steam_id();

            match level {
                "root" => {
                    add_permissions(steam_id, &[flags::ROOT]);
                    set_immunity(steam_id, 100);
                    info.reply(&format!("Granted ROOT admin to {}", target.name_string()));
                }
                "full" => {
                    add_permissions(steam_id, &[
                        flags::KICK,
                        flags::BAN,
                        flags::SLAY,
                        flags::CHANGEMAP,
                    ]);
                    set_immunity(steam_id, 50);
                    info.reply(&format!("Granted FULL admin to {}", target.name_string()));
                }
                _ => {
                    // Default: basic
                    add_permissions(steam_id, &[flags::KICK, flags::SLAY]);
                    set_immunity(steam_id, 25);
                    info.reply(&format!("Granted BASIC admin to {}", target.name_string()));
                }
            }

            CommandResult::Handled
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_flag_format() {
        // Permission flags use @domain/permission format
        assert!(flags::ROOT.starts_with("@css/"));
        assert!(flags::KICK.starts_with("@css/"));
        assert!(flags::BAN.starts_with("@css/"));
        assert!(flags::SLAY.starts_with("@css/"));
    }
}
