//! # Basic Commands Example
//!
//! Demonstrates the console and chat command system.
//!
//! ## Features Demonstrated
//! - `register_command` - Register a new command
//! - `CommandInfo` - Access command arguments and reply
//! - `CommandResult` - Control command flow
//! - `CommandContext` - Determine where command was called from
//! - `get_players` - Iterate over connected players
//!
//! ## Registered Commands
//! - `!ping` / `csr_ping` - Simple ping/pong
//! - `!players` / `csr_players` - List online players
//! - `!say <message>` / `csr_say` - Echo a message
//!
//! ## Command Prefixes
//! Commands registered with `csr_` prefix can be called as:
//! - `csr_ping` in console
//! - `!ping` in public chat
//! - `/ping` in silent chat (only sender sees output)

use cs2rust_core::{register_command, CommandContext, CommandResult};
use cs2rust_core::entities::get_players;

/// Initialize the Basic Commands plugin.
///
/// Registers several demonstration commands.
pub fn init() {
    register_ping_command();
    register_players_command();
    register_say_command();

    tracing::info!("Basic Commands plugin initialized!");
    tracing::info!("Available commands: !ping, !players, !say <message>");
}

/// Register the ping command - responds with "Pong!"
fn register_ping_command() {
    register_command(
        "csr_ping",
        "Respond with pong",
        |_player, info| {
            // Simple response
            info.reply("Pong!");

            // Log the context for demonstration
            match info.context() {
                CommandContext::ServerConsole => {
                    tracing::debug!("Ping from server console");
                }
                CommandContext::ClientConsole => {
                    tracing::debug!("Ping from client console");
                }
                CommandContext::ChatPublic => {
                    tracing::debug!("Ping from public chat (!ping)");
                }
                CommandContext::ChatSilent => {
                    tracing::debug!("Ping from silent chat (/ping)");
                }
            }

            // Return Handled to stop processing
            CommandResult::Handled
        },
    );
}

/// Register the players command - lists all online players
fn register_players_command() {
    register_command(
        "csr_players",
        "List online players",
        |_player, info| {
            // Count connected players
            let players: Vec<_> = get_players().collect();
            let count = players.len();

            info.reply(&format!("Online players: {}", count));

            // List each player
            for controller in players {
                let name = controller.name_string();
                let slot = controller.slot();
                let alive = if controller.is_alive() { "alive" } else { "dead" };

                info.reply(&format!("  [{}] {} ({})", slot, name, alive));
            }

            CommandResult::Handled
        },
    );
}

/// Register the say command - echoes a message back
fn register_say_command() {
    register_command(
        "csr_say",
        "Echo a message",
        |player, info| {
            // Get all arguments after command name
            let message = info.arg_string();

            if message.is_empty() {
                info.reply("Usage: !say <message>");
                info.reply("Example: !say Hello world!");
                return CommandResult::Handled;
            }

            // Include who said it (if from a player)
            if let Some(p) = player {
                info.reply(&format!("{} says: {}", p.name_string(), message));
            } else {
                // Server console
                info.reply(&format!("Server says: {}", message));
            }

            // Demonstrate argument access
            let arg_count = info.arg_count();
            tracing::debug!(
                "Say command had {} args: {:?}",
                arg_count,
                info.args()
            );

            CommandResult::Handled
        },
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_command_registration_does_not_panic() {
        // Smoke test - in real environment, commands would be registered
        // but without the game running, we just verify the code compiles
    }
}
