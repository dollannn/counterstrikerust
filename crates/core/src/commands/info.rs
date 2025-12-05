//! Command information types

use crate::entities::PlayerController;

/// Context from which a command was called
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandContext {
    /// Called from server console (no player)
    ServerConsole,
    /// Called from client console
    ClientConsole,
    /// Called from public chat (!cmd)
    ChatPublic,
    /// Called from silent chat (/cmd)
    ChatSilent,
}

impl CommandContext {
    /// Returns true if command was triggered from chat
    pub fn is_chat(&self) -> bool {
        matches!(self, Self::ChatPublic | Self::ChatSilent)
    }

    /// Returns true if command was triggered from console
    pub fn is_console(&self) -> bool {
        matches!(self, Self::ServerConsole | Self::ClientConsole)
    }

    /// Returns true if this is a silent chat command (should not show in chat)
    pub fn is_silent(&self) -> bool {
        matches!(self, Self::ChatSilent)
    }
}

/// Result of command execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum CommandResult {
    /// Continue processing, allow other handlers
    Continue = 0,
    /// Command was handled, stop processing
    Handled = 1,
    /// Block the command entirely (prevent original behavior)
    Block = 2,
}

impl Default for CommandResult {
    fn default() -> Self {
        Self::Continue
    }
}

/// Information about a command invocation
pub struct CommandInfo {
    /// Raw command arguments (index 0 is the command name)
    args: Vec<String>,

    /// Full command string including command name
    raw_string: String,

    /// Player who executed command (None if server console)
    player: Option<PlayerController>,

    /// Context of command invocation
    context: CommandContext,

    /// Player slot (-1 for server console)
    player_slot: i32,
}

impl CommandInfo {
    /// Create new CommandInfo
    pub fn new(
        args: Vec<String>,
        raw_string: String,
        player: Option<PlayerController>,
        context: CommandContext,
        player_slot: i32,
    ) -> Self {
        Self {
            args,
            raw_string,
            player,
            context,
            player_slot,
        }
    }

    /// Get the number of arguments (including command name at index 0)
    pub fn arg_count(&self) -> usize {
        self.args.len()
    }

    /// Get argument by index (0 = command name)
    ///
    /// Returns empty string if index is out of bounds.
    pub fn arg(&self, index: usize) -> &str {
        self.args.get(index).map(|s| s.as_str()).unwrap_or("")
    }

    /// Get the command name (alias for arg(0))
    pub fn command_name(&self) -> &str {
        self.arg(0)
    }

    /// Get all arguments after command name as a single string
    pub fn arg_string(&self) -> String {
        if self.args.len() > 1 {
            self.args[1..].join(" ")
        } else {
            String::new()
        }
    }

    /// Get all arguments as a slice
    pub fn args(&self) -> &[String] {
        &self.args
    }

    /// Get the full raw command string
    pub fn get_command_string(&self) -> &str {
        &self.raw_string
    }

    /// Get the player who executed the command (None for server console)
    pub fn player(&self) -> Option<&PlayerController> {
        self.player.as_ref()
    }

    /// Get the calling context
    pub fn context(&self) -> CommandContext {
        self.context
    }

    /// Get player slot (-1 for server console)
    pub fn player_slot(&self) -> i32 {
        self.player_slot
    }

    /// Reply to the command (auto-routes to console or chat based on context)
    ///
    /// Uses ClientPrint when available, falls back to logging otherwise.
    pub fn reply(&self, message: &str) {
        use super::print::{self, HudDestination};

        match self.context {
            CommandContext::ServerConsole => {
                // Print to server console
                tracing::info!("[Server] {}", message);
            }
            CommandContext::ClientConsole => {
                if let Some(ref player) = self.player {
                    // Send to player's console
                    unsafe {
                        print::client_print(player.as_ptr(), HudDestination::Console, message);
                    }
                } else {
                    tracing::info!("[Reply] {}", message);
                }
            }
            CommandContext::ChatPublic | CommandContext::ChatSilent => {
                if let Some(ref player) = self.player {
                    // Send to player's chat
                    unsafe {
                        print::client_print(player.as_ptr(), HudDestination::Talk, message);
                    }
                } else {
                    tracing::info!("[Reply] {}", message);
                }
            }
        }
    }

    /// Reply with formatted message
    pub fn reply_fmt(&self, args: std::fmt::Arguments<'_>) {
        self.reply(&args.to_string());
    }
}

/// Type alias for command callback functions
pub type CommandCallback =
    Box<dyn Fn(Option<&PlayerController>, &CommandInfo) -> CommandResult + Send + Sync>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_context() {
        assert!(CommandContext::ChatPublic.is_chat());
        assert!(CommandContext::ChatSilent.is_chat());
        assert!(!CommandContext::ServerConsole.is_chat());

        assert!(CommandContext::ServerConsole.is_console());
        assert!(CommandContext::ClientConsole.is_console());
        assert!(!CommandContext::ChatPublic.is_console());

        assert!(CommandContext::ChatSilent.is_silent());
        assert!(!CommandContext::ChatPublic.is_silent());
    }

    #[test]
    fn test_command_info() {
        let info = CommandInfo::new(
            vec![
                "csr_test".to_string(),
                "arg1".to_string(),
                "arg2".to_string(),
            ],
            "csr_test arg1 arg2".to_string(),
            None,
            CommandContext::ServerConsole,
            -1,
        );

        assert_eq!(info.arg_count(), 3);
        assert_eq!(info.command_name(), "csr_test");
        assert_eq!(info.arg(0), "csr_test");
        assert_eq!(info.arg(1), "arg1");
        assert_eq!(info.arg(2), "arg2");
        assert_eq!(info.arg(999), "");
        assert_eq!(info.arg_string(), "arg1 arg2");
        assert_eq!(info.player_slot(), -1);
        assert!(info.player().is_none());
    }
}
