//! Command manager - registration and dispatch

use std::collections::HashMap;
use std::sync::LazyLock;

use parking_lot::RwLock;
use slotmap::{new_key_type, SlotMap};

use super::info::{CommandCallback, CommandContext, CommandInfo, CommandResult};
use crate::entities::PlayerController;

new_key_type! {
    /// Handle for a registered command
    pub struct CommandKey;
}

/// Default command prefix for CS2Rust
pub const DEFAULT_PREFIX: &str = "csr_";

/// CounterStrikeSharp compatibility prefix
pub const CSS_PREFIX: &str = "css_";

/// Registered command information
struct CommandEntry {
    /// Full command name (e.g., "csr_ping")
    name: String,
    /// Short name without prefix (e.g., "ping")
    short_name: String,
    /// Command description
    description: String,
    /// Callback function
    callback: CommandCallback,
    /// Whether this is server-only
    server_only: bool,
    /// Required permission (e.g., "@css/ban")
    required_permission: Option<String>,
}

/// Global command manager
pub struct CommandManager {
    /// Commands indexed by key
    commands: SlotMap<CommandKey, CommandEntry>,

    /// Lookup by full command name (case-insensitive, lowercase)
    by_name: HashMap<String, CommandKey>,

    /// Lookup by short name for chat commands (case-insensitive, lowercase)
    by_short_name: HashMap<String, CommandKey>,
}

impl CommandManager {
    fn new() -> Self {
        Self {
            commands: SlotMap::with_key(),
            by_name: HashMap::new(),
            by_short_name: HashMap::new(),
        }
    }

    /// Register a command
    fn register(
        &mut self,
        name: &str,
        description: &str,
        server_only: bool,
        required_permission: Option<String>,
        callback: CommandCallback,
    ) -> Option<CommandKey> {
        let name_lower = name.to_lowercase();

        // Check if already registered
        if self.by_name.contains_key(&name_lower) {
            tracing::warn!("Command '{}' already registered", name);
            return None;
        }

        // Extract short name (remove prefix if present)
        let short_name = if name_lower.starts_with(DEFAULT_PREFIX) {
            name_lower[DEFAULT_PREFIX.len()..].to_string()
        } else if name_lower.starts_with(CSS_PREFIX) {
            // Support CSS prefix for compatibility
            name_lower[CSS_PREFIX.len()..].to_string()
        } else {
            // No known prefix, use full name as short name
            name_lower.clone()
        };

        let entry = CommandEntry {
            name: name.to_string(),
            short_name: short_name.clone(),
            description: description.to_string(),
            callback,
            server_only,
            required_permission,
        };

        let key = self.commands.insert(entry);
        self.by_name.insert(name_lower, key);

        // Only register short name if it's different from the full name
        if short_name != name.to_lowercase() {
            self.by_short_name.insert(short_name, key);
        }

        tracing::debug!("Registered command: {}", name);
        Some(key)
    }

    /// Unregister a command by key
    fn unregister(&mut self, key: CommandKey) -> bool {
        if let Some(entry) = self.commands.remove(key) {
            self.by_name.remove(&entry.name.to_lowercase());
            self.by_short_name.remove(&entry.short_name);
            tracing::debug!("Unregistered command: {}", entry.name);
            true
        } else {
            false
        }
    }

    /// Find command by full name
    pub fn find_by_name(&self, name: &str) -> Option<CommandKey> {
        self.by_name.get(&name.to_lowercase()).copied()
    }

    /// Find command by short name (for chat commands)
    pub fn find_by_short_name(&self, name: &str) -> Option<CommandKey> {
        self.by_short_name.get(&name.to_lowercase()).copied()
    }

    /// Execute a command by key
    fn execute(
        &self,
        key: CommandKey,
        player: Option<&PlayerController>,
        info: &CommandInfo,
    ) -> CommandResult {
        if let Some(entry) = self.commands.get(key) {
            // Check server-only restriction
            if entry.server_only && player.is_some() {
                info.reply("This command can only be executed from the server console.");
                return CommandResult::Handled;
            }

            // Check permission requirement
            if let Some(ref permission) = entry.required_permission {
                // Server console always has permission
                if let Some(p) = player {
                    if !crate::permissions::player_has_permission(p, permission) {
                        info.reply(&format!(
                            "You don't have permission to use this command. Required: {}",
                            permission
                        ));
                        return CommandResult::Handled;
                    }
                }
            }

            (entry.callback)(player, info)
        } else {
            CommandResult::Continue
        }
    }

    /// Get command description
    pub fn get_description(&self, key: CommandKey) -> Option<&str> {
        self.commands.get(key).map(|e| e.description.as_str())
    }

    /// Get command name
    pub fn get_name(&self, key: CommandKey) -> Option<&str> {
        self.commands.get(key).map(|e| e.name.as_str())
    }

    /// Iterate over all registered commands
    pub fn iter(&self) -> impl Iterator<Item = (CommandKey, &str, &str)> {
        self.commands
            .iter()
            .map(|(key, entry)| (key, entry.name.as_str(), entry.description.as_str()))
    }

    /// Get total number of registered commands
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Check if command manager has no registered commands
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

/// Global command manager instance
pub static COMMANDS: LazyLock<RwLock<CommandManager>> =
    LazyLock::new(|| RwLock::new(CommandManager::new()));

/// Register a command with the default prefix (csr_)
///
/// # Arguments
/// * `name` - Command name (should include prefix, e.g., "csr_ping")
/// * `description` - Help text for the command
/// * `callback` - Function to call when command is executed
///
/// # Returns
/// A key to manage the command, or None if registration failed
///
/// # Example
/// ```ignore
/// use cs2rust_core::commands::{register_command, CommandResult};
///
/// let key = register_command("csr_ping", "Respond with pong", |player, info| {
///     info.reply("Pong!");
///     CommandResult::Handled
/// });
/// ```
pub fn register_command<F>(name: &str, description: &str, callback: F) -> Option<CommandKey>
where
    F: Fn(Option<&PlayerController>, &CommandInfo) -> CommandResult + Send + Sync + 'static,
{
    COMMANDS
        .write()
        .register(name, description, false, None, Box::new(callback))
}

/// Register a command with extended options
///
/// This is the extended version that supports optional permission requirements.
/// Called by the `#[console_command]` macro when a permission is specified.
///
/// # Arguments
/// * `name` - Command name (should include prefix, e.g., "css_ban")
/// * `description` - Help text for the command
/// * `permission` - Optional required permission (e.g., "@css/ban")
/// * `callback` - Function to call when command is executed
///
/// # Example
/// ```ignore
/// use cs2rust_core::commands::{register_command_ex, CommandResult};
///
/// let key = register_command_ex(
///     "css_ban",
///     "Ban a player",
///     Some("@css/ban"),
///     |player, info| {
///         // Only runs if player has @css/ban permission
///         CommandResult::Handled
///     }
/// );
/// ```
pub fn register_command_ex<F>(
    name: &str,
    description: &str,
    permission: Option<&str>,
    callback: F,
) -> Option<CommandKey>
where
    F: Fn(Option<&PlayerController>, &CommandInfo) -> CommandResult + Send + Sync + 'static,
{
    COMMANDS.write().register(
        name,
        description,
        false,
        permission.map(|s| s.to_string()),
        Box::new(callback),
    )
}

/// Register a server-only command
///
/// Server-only commands can only be executed from the server console,
/// not by players in-game.
pub fn register_server_command<F>(name: &str, description: &str, callback: F) -> Option<CommandKey>
where
    F: Fn(Option<&PlayerController>, &CommandInfo) -> CommandResult + Send + Sync + 'static,
{
    COMMANDS
        .write()
        .register(name, description, true, None, Box::new(callback))
}

/// Unregister a command
pub fn unregister_command(key: CommandKey) -> bool {
    COMMANDS.write().unregister(key)
}

/// Dispatch a command from console
pub(crate) fn dispatch_console_command(
    command_name: &str,
    args: Vec<String>,
    raw_string: String,
    player: Option<PlayerController>,
    player_slot: i32,
) -> CommandResult {
    let manager = COMMANDS.read();

    let context = if player.is_some() {
        CommandContext::ClientConsole
    } else {
        CommandContext::ServerConsole
    };

    let info = CommandInfo::new(args, raw_string, player, context, player_slot);

    if let Some(key) = manager.find_by_name(command_name) {
        manager.execute(key, info.player(), &info)
    } else {
        CommandResult::Continue
    }
}

/// Dispatch a command from chat
pub(crate) fn dispatch_chat_command(
    short_name: &str,
    args: Vec<String>,
    raw_string: String,
    player: PlayerController,
    player_slot: i32,
    is_silent: bool,
) -> CommandResult {
    let manager = COMMANDS.read();

    let context = if is_silent {
        CommandContext::ChatSilent
    } else {
        CommandContext::ChatPublic
    };

    let info = CommandInfo::new(args, raw_string, Some(player), context, player_slot);

    // First try to find by short name
    if let Some(key) = manager.find_by_short_name(short_name) {
        return manager.execute(key, info.player(), &info);
    }

    // Try with default prefix
    let prefixed_name = format!("{}{}", DEFAULT_PREFIX, short_name);
    if let Some(key) = manager.find_by_name(&prefixed_name) {
        return manager.execute(key, info.player(), &info);
    }

    // Try with css_ prefix for compatibility
    let css_prefixed = format!("{}{}", CSS_PREFIX, short_name);
    if let Some(key) = manager.find_by_name(&css_prefixed) {
        return manager.execute(key, info.player(), &info);
    }

    CommandResult::Continue
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_command() {
        let mut manager = CommandManager::new();

        let key = manager
            .register(
                "csr_test",
                "Test command",
                false,
                None,
                Box::new(|_, _| CommandResult::Handled),
            )
            .unwrap();

        assert!(manager.find_by_name("csr_test").is_some());
        assert!(manager.find_by_name("CSR_TEST").is_some()); // Case insensitive
        assert!(manager.find_by_short_name("test").is_some());
        assert_eq!(manager.get_description(key), Some("Test command"));
    }

    #[test]
    fn test_register_css_prefix() {
        let mut manager = CommandManager::new();

        manager
            .register(
                "css_slap",
                "Slap command",
                false,
                None,
                Box::new(|_, _| CommandResult::Handled),
            )
            .unwrap();

        assert!(manager.find_by_name("css_slap").is_some());
        assert!(manager.find_by_short_name("slap").is_some());
    }

    #[test]
    fn test_unregister_command() {
        let mut manager = CommandManager::new();

        let key = manager
            .register(
                "csr_temp",
                "Temporary",
                false,
                None,
                Box::new(|_, _| CommandResult::Handled),
            )
            .unwrap();

        assert!(manager.find_by_name("csr_temp").is_some());
        assert!(manager.unregister(key));
        assert!(manager.find_by_name("csr_temp").is_none());
    }

    #[test]
    fn test_duplicate_registration() {
        let mut manager = CommandManager::new();

        let key1 = manager.register(
            "csr_dupe",
            "First",
            false,
            None,
            Box::new(|_, _| CommandResult::Handled),
        );
        let key2 = manager.register(
            "csr_dupe",
            "Second",
            false,
            None,
            Box::new(|_, _| CommandResult::Handled),
        );

        assert!(key1.is_some());
        assert!(key2.is_none()); // Should fail - duplicate
    }
}
