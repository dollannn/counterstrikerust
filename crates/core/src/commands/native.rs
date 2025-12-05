//! Native console command hooking via ICvar::DispatchConCommand
//!
//! Hooks the ICvar vtable to intercept console command dispatch.

use std::ffi::{c_char, c_void, CStr};
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::OnceLock;

use super::manager::{dispatch_console_command, COMMANDS};
use super::CommandResult;
use crate::engine::engine;
use crate::hooks::{vtable, HookError, VTableHookKey};

/// Default VTable index for ICvar::DispatchConCommand (Linux)
/// Can be overridden via gamedata
#[cfg(target_os = "linux")]
const DEFAULT_DISPATCH_VTABLE_INDEX: usize = 7;

#[cfg(target_os = "windows")]
const DEFAULT_DISPATCH_VTABLE_INDEX: usize = 7;

/// Storage for hook key
static DISPATCH_HOOK_KEY: OnceLock<VTableHookKey> = OnceLock::new();

/// Storage for original function pointer
static ORIGINAL_DISPATCH: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());

/// CCommandContext from Source 2
/// Contains information about who is executing the command
#[repr(C)]
struct CCommandContext {
    /// Target entity (usually -1 for console)
    target: i32,
    /// Player slot who executed (-1 for server console)
    player_slot: i32,
}

/// CCommand from Source 2
/// Contains the parsed command arguments
#[repr(C)]
struct CCommand {
    /// Size of argv[0] (command name)
    argv0_size: i32,
    /// Raw argument buffer
    _args_buffer: [u8; 512],
    /// Buffer for argv pointers
    _argv_buffer: [u8; 512],
    /// Argument pointers (argv[0] is command name)
    args: [*const c_char; 64],
}

impl CCommand {
    /// Count the number of arguments
    fn arg_count(&self) -> usize {
        let mut count = 0;
        for ptr in &self.args {
            if ptr.is_null() {
                break;
            }
            count += 1;
        }
        count
    }

    /// Get argument by index as string slice
    fn arg(&self, index: usize) -> &str {
        if index < self.args.len() && !self.args[index].is_null() {
            unsafe { CStr::from_ptr(self.args[index]).to_str().unwrap_or("") }
        } else {
            ""
        }
    }

    /// Get all arguments as a Vec<String>
    fn get_args(&self) -> Vec<String> {
        (0..self.arg_count())
            .map(|i| self.arg(i).to_string())
            .collect()
    }

    /// Get the full command string
    fn get_command_string(&self) -> String {
        self.get_args().join(" ")
    }
}

/// ConCommandRef wrapper
#[repr(C)]
struct ConCommandRef {
    /// Access index for this command
    _access_index: u16,
    /// Registered index
    _registered_index: i32,
}

/// Function type for DispatchConCommand
type DispatchConCommandFn = extern "C" fn(
    this: *mut c_void,
    cmd: *const ConCommandRef,
    ctx: *const CCommandContext,
    args: *const CCommand,
);

/// Our hook for DispatchConCommand
extern "C" fn dispatch_con_command_hook(
    this: *mut c_void,
    cmd: *const ConCommandRef,
    ctx: *const CCommandContext,
    args: *const CCommand,
) {
    // Get original function
    let original_ptr = ORIGINAL_DISPATCH.load(Ordering::Acquire);
    if original_ptr.is_null() {
        tracing::error!("DispatchConCommand original is null!");
        return;
    }
    let original: DispatchConCommandFn = unsafe { std::mem::transmute(original_ptr) };

    // Parse command info safely
    let (command_name, command_args, raw_string, player_slot) = unsafe {
        let args_ref = &*args;
        let ctx_ref = &*ctx;

        let command_args = args_ref.get_args();
        let command_name = command_args.first().cloned().unwrap_or_default();
        let raw_string = args_ref.get_command_string();
        let player_slot = ctx_ref.player_slot;

        (command_name, command_args, raw_string, player_slot)
    };

    // Check if this is one of our commands
    let is_our_command = {
        let manager = COMMANDS.read();
        manager.find_by_name(&command_name).is_some()
    };

    if is_our_command {
        // Get player controller if this is from a client
        let player = if player_slot >= 0 {
            // TODO: Get player controller from slot via entity system
            None
        } else {
            None
        };

        let result =
            dispatch_console_command(&command_name, command_args, raw_string, player, player_slot);

        if result >= CommandResult::Handled {
            // Don't call original - we handled it
            tracing::trace!("Command '{}' handled by CS2Rust", command_name);
            return;
        }
    }

    // Call original for unhandled commands
    original(this, cmd, ctx, args);
}

/// Initialize native command hooks
///
/// Hooks ICvar::DispatchConCommand to intercept console commands.
pub fn init_command_hooks() -> Result<(), HookError> {
    let cvar = engine().cvar_ptr();
    if cvar.is_null() {
        return Err(HookError::InvalidAddress(0));
    }

    // Get vtable index from gamedata if available
    let vtable_index = if let Some(gd) = crate::gamedata::gamedata() {
        gd.get_offset("ICvar_DispatchConCommand")
            .map(|o| o as usize)
            .unwrap_or(DEFAULT_DISPATCH_VTABLE_INDEX)
    } else {
        DEFAULT_DISPATCH_VTABLE_INDEX
    };

    unsafe {
        let (key, original) = vtable::create_vtable_hook(
            "ICvar::DispatchConCommand",
            cvar as *mut (),
            vtable_index,
            dispatch_con_command_hook as *const (),
        )?;

        ORIGINAL_DISPATCH.store(original as *mut c_void, Ordering::Release);

        if DISPATCH_HOOK_KEY.set(key).is_err() {
            // Already initialized, remove the hook we just created
            vtable::remove_vtable_hook(key)?;
            return Err(HookError::DetourCreation("Already initialized".into()));
        }

        tracing::info!(
            "Hooked ICvar::DispatchConCommand at vtable[{}]",
            vtable_index
        );
    }

    Ok(())
}

/// Shutdown native command hooks
pub fn shutdown_command_hooks() {
    if let Some(key) = DISPATCH_HOOK_KEY.get() {
        if let Err(e) = vtable::remove_vtable_hook(*key) {
            tracing::warn!("Failed to remove DispatchConCommand hook: {:?}", e);
        }
    }

    ORIGINAL_DISPATCH.store(std::ptr::null_mut(), Ordering::Release);
    tracing::info!("Native command hooks removed");
}

/// Check if command hooks are initialized
pub fn is_initialized() -> bool {
    DISPATCH_HOOK_KEY.get().is_some()
}
