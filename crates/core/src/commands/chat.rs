//! Chat command handling via Host_Say hook
//!
//! Hooks the Host_Say function to intercept chat messages and parse commands.

use std::ffi::{c_char, c_void, CStr};
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::OnceLock;

use parking_lot::RwLock;

use super::manager::{dispatch_chat_command, COMMANDS};
use crate::entities::PlayerController;
use crate::gamedata::find_signature;
use crate::hooks::{inline, HookError, InlineHookKey};
use crate::schema::SchemaObject;

/// Default public chat trigger
pub const DEFAULT_PUBLIC_TRIGGER: char = '!';

/// Default silent chat trigger
pub const DEFAULT_SILENT_TRIGGER: char = '/';

/// Chat trigger configuration
#[derive(Clone)]
pub struct ChatTriggers {
    /// Public trigger character (message shown in chat)
    pub public: char,
    /// Silent trigger character (message hidden from chat)
    pub silent: char,
}

impl Default for ChatTriggers {
    fn default() -> Self {
        Self {
            public: DEFAULT_PUBLIC_TRIGGER,
            silent: DEFAULT_SILENT_TRIGGER,
        }
    }
}

/// Current chat triggers
static CHAT_TRIGGERS: RwLock<ChatTriggers> = RwLock::new(ChatTriggers {
    public: DEFAULT_PUBLIC_TRIGGER,
    silent: DEFAULT_SILENT_TRIGGER,
});

/// Storage for hook key
static HOST_SAY_HOOK_KEY: OnceLock<InlineHookKey> = OnceLock::new();

/// Storage for original function pointer
static ORIGINAL_HOST_SAY: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());

/// CCommand from Source 2
#[repr(C)]
struct CCommand {
    /// Size of argv[0]
    argv0_size: i32,
    /// Raw argument buffer
    _args_buffer: [u8; 512],
    /// Buffer for argv pointers
    _argv_buffer: [u8; 512],
    /// Argument pointers
    args: [*const c_char; 64],
}

impl CCommand {
    /// Get argument by index as string slice
    fn arg(&self, index: usize) -> &str {
        if index < self.args.len() && !self.args[index].is_null() {
            unsafe { CStr::from_ptr(self.args[index]).to_str().unwrap_or("") }
        } else {
            ""
        }
    }

    /// Get the raw argument string (everything after command name)
    fn arg_s(&self) -> &str {
        // In Source, ArgS() returns everything after the command name
        // For chat, args[0] is "say" and args[1] is the message
        self.arg(1)
    }
}

/// Host_Say function signature
/// void Host_Say(CEntityInstance* pController, CCommand& args, bool teamonly, int unk1, const char* unk2)
type HostSayFn = unsafe extern "C" fn(
    controller: *mut c_void,
    args: *mut CCommand,
    team_only: bool,
    unk1: i32,
    unk2: *const c_char,
);

/// Check if a message starts with a chat trigger
///
/// Returns (is_silent, command_text) if a trigger is found
fn check_chat_trigger(message: &str) -> Option<(bool, &str)> {
    let triggers = CHAT_TRIGGERS.read();
    let first_char = message.chars().next()?;

    if first_char == triggers.public {
        Some((false, &message[first_char.len_utf8()..]))
    } else if first_char == triggers.silent {
        Some((true, &message[first_char.len_utf8()..]))
    } else {
        None
    }
}

/// Parse a chat command into name and arguments
fn parse_chat_command(text: &str) -> (String, Vec<String>) {
    let parts: Vec<&str> = text.split_whitespace().collect();
    let command_name = parts.first().map(|s| s.to_lowercase()).unwrap_or_default();
    let args: Vec<String> = parts.iter().map(|s| s.to_string()).collect();
    (command_name, args)
}

/// Our detour for Host_Say
unsafe extern "C" fn host_say_detour(
    controller: *mut c_void,
    args: *mut CCommand,
    team_only: bool,
    unk1: i32,
    unk2: *const c_char,
) {
    let original_ptr = ORIGINAL_HOST_SAY.load(Ordering::Acquire);
    if original_ptr.is_null() {
        tracing::error!("Host_Say original is null!");
        return;
    }
    let original: HostSayFn = std::mem::transmute(original_ptr);

    // Get the message from args
    let message = (*args).arg_s();

    // Check for command trigger
    let (is_silent, command_text) = match check_chat_trigger(message) {
        Some(result) => result,
        None => {
            // Not a command, call original
            original(controller, args, team_only, unk1, unk2);
            return;
        }
    };

    // Parse command
    let (command_name, command_args) = parse_chat_command(command_text);

    if command_name.is_empty() {
        // Just a trigger character with no command
        original(controller, args, team_only, unk1, unk2);
        return;
    }

    // Check if command exists
    let command_exists = {
        let manager = COMMANDS.read();
        manager.find_by_short_name(&command_name).is_some()
            || manager
                .find_by_name(&format!("csr_{}", command_name))
                .is_some()
            || manager
                .find_by_name(&format!("css_{}", command_name))
                .is_some()
    };

    if !command_exists {
        // Not a registered command, let the message through
        original(controller, args, team_only, unk1, unk2);
        return;
    }

    // If not silent, call original first to show message in chat
    if !is_silent {
        original(controller, args, team_only, unk1, unk2);
    }

    // Get player controller from entity
    let player = if !controller.is_null() {
        PlayerController::from_ptr(controller)
    } else {
        None
    };

    // Get player slot (TODO: implement proper slot lookup)
    let player_slot = 0; // Placeholder

    // Dispatch the command
    if let Some(player) = player {
        let result = dispatch_chat_command(
            &command_name,
            command_args,
            command_text.to_string(),
            player,
            player_slot,
            is_silent,
        );

        tracing::trace!("Chat command '{}' result: {:?}", command_name, result);
    }
}

/// Initialize chat command hooks
///
/// Finds Host_Say via signature scanning and installs an inline hook.
///
/// # Safety
/// Module base and size must be valid for the server module.
pub unsafe fn init_chat_hooks(server_base: *const u8, server_size: usize) -> Result<(), HookError> {
    // Find Host_Say via signature
    let host_say_addr = find_signature("Host_Say", server_base, server_size)
        .map_err(|e| HookError::DetourCreation(format!("Host_Say signature not found: {:?}", e)))?;

    tracing::debug!("Found Host_Say at {:p}", host_say_addr);

    // Create inline hook
    let (key, original) = inline::create_inline_hook(
        "Host_Say",
        host_say_addr as *const (),
        host_say_detour as *const (),
    )?;

    ORIGINAL_HOST_SAY.store(original as *mut c_void, Ordering::Release);

    if HOST_SAY_HOOK_KEY.set(key).is_err() {
        inline::remove_inline_hook(key)?;
        return Err(HookError::DetourCreation("Already initialized".into()));
    }

    tracing::info!("Hooked Host_Say at {:p}", host_say_addr);
    Ok(())
}

/// Shutdown chat command hooks
pub fn shutdown_chat_hooks() {
    if let Some(key) = HOST_SAY_HOOK_KEY.get() {
        if let Err(e) = inline::remove_inline_hook(*key) {
            tracing::warn!("Failed to remove Host_Say hook: {:?}", e);
        }
    }

    ORIGINAL_HOST_SAY.store(std::ptr::null_mut(), Ordering::Release);
    tracing::info!("Chat hooks removed");
}

/// Check if chat hooks are initialized
pub fn is_initialized() -> bool {
    HOST_SAY_HOOK_KEY.get().is_some()
}

/// Set custom chat triggers
pub fn set_triggers(public: char, silent: char) {
    let mut triggers = CHAT_TRIGGERS.write();
    triggers.public = public;
    triggers.silent = silent;
    tracing::info!(
        "Chat triggers set: public='{}', silent='{}'",
        public,
        silent
    );
}

/// Get current chat triggers
pub fn get_triggers() -> ChatTriggers {
    CHAT_TRIGGERS.read().clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_chat_trigger() {
        assert_eq!(check_chat_trigger("!ping"), Some((false, "ping")));
        assert_eq!(check_chat_trigger("/ping"), Some((true, "ping")));
        assert_eq!(check_chat_trigger("hello"), None);
        assert_eq!(check_chat_trigger(""), None);
    }

    #[test]
    fn test_parse_chat_command() {
        let (name, args) = parse_chat_command("ping");
        assert_eq!(name, "ping");
        assert_eq!(args, vec!["ping"]);

        let (name, args) = parse_chat_command("slap player1 100");
        assert_eq!(name, "slap");
        assert_eq!(args, vec!["slap", "player1", "100"]);
    }
}
