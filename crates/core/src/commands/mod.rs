//! Console and Chat Command System
//!
//! Provides unified command handling for both console and chat commands.
//!
//! # Architecture
//!
//! ```text
//! ICvar::DispatchConCommand / Host_Say → CommandManager → Rust callbacks
//! ```
//!
//! # Prefixes
//!
//! - Commands with `csr_` prefix (default) auto-register as chat commands
//! - Commands with `css_` prefix also work for CounterStrikeSharp compatibility
//! - Example: `csr_ping` can be called as `!ping` or `/ping` in chat
//!
//! # Example
//!
//! ```ignore
//! use cs2rust_core::commands::{register_command, CommandInfo, CommandResult};
//!
//! // Register a command with csr_ prefix
//! register_command("csr_ping", "Respond with pong", |player, info| {
//!     info.reply("Pong!");
//!     CommandResult::Handled
//! });
//!
//! // Now callable as:
//! // - `csr_ping` in console
//! // - `!ping` or `/ping` in chat
//! ```

pub mod chat;
mod info;
mod manager;
mod native;
pub mod print;

pub use info::{CommandCallback, CommandContext, CommandInfo, CommandResult};
pub use manager::{
    register_command, register_command_ex, register_server_command, unregister_command,
    CommandKey, CommandManager, COMMANDS, CSS_PREFIX, DEFAULT_PREFIX,
};

use crate::hooks::HookError;

/// Initialize the command system (console commands only)
///
/// This sets up the ICvar hook for console command handling.
/// For chat commands, call `init_chat_hooks` separately with server module info.
/// Should be called during plugin startup.
pub fn init() -> Result<(), HookError> {
    tracing::info!("Command system initializing...");

    // Initialize console command hook (ICvar::DispatchConCommand)
    native::init_command_hooks()?;

    tracing::info!("Command system initialized (console commands only)");
    tracing::info!("Call init_chat_hooks() with server module info to enable chat commands");
    Ok(())
}

/// Initialize chat command hooks
///
/// Must be called separately after `init()` because it requires the server module base address.
///
/// # Arguments
/// * `server_base` - Base address of the server module
/// * `server_size` - Size of the server module
///
/// # Safety
/// Module base and size must be valid.
pub unsafe fn init_chat_hooks(server_base: *const u8, server_size: usize) -> Result<(), HookError> {
    chat::init_chat_hooks(server_base, server_size)?;
    tracing::info!("Chat command hooks initialized");
    Ok(())
}

/// Shutdown the command system
///
/// Removes all hooks and clears registered commands.
/// Should be called during plugin shutdown.
pub fn shutdown() {
    tracing::info!("Command system shutting down...");

    // Remove chat command hook (if initialized)
    if chat::is_initialized() {
        chat::shutdown_chat_hooks();
    }

    // Remove console command hook
    native::shutdown_command_hooks();

    tracing::info!("Command system shutdown complete");
}

/// Check if console command hooks are initialized
pub fn is_initialized() -> bool {
    native::is_initialized()
}

/// Check if chat command hooks are initialized
pub fn is_chat_initialized() -> bool {
    chat::is_initialized()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_public_api() {
        // Test that the public API is accessible
        let _context = CommandContext::ServerConsole;
        let _result = CommandResult::Handled;
    }
}
