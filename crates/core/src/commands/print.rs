//! Client print functionality for sending messages to players
//!
//! Uses signature scanning to find and call the game's ClientPrint function.

use std::ffi::{c_char, c_void, CString};
use std::sync::OnceLock;

use crate::gamedata::{find_signature, GamedataError};

/// Print destination for client messages
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudDestination {
    /// HUD notification area
    Notify = 1,
    /// Client console
    Console = 2,
    /// Chat area
    Talk = 3,
    /// Center of screen
    Center = 4,
}

/// ClientPrint function signature
/// void ClientPrint(CBasePlayerController* player, int msg_dest, const char* msg, ...)
type ClientPrintFn = unsafe extern "C" fn(
    player: *mut c_void,
    msg_dest: i32,
    msg: *const c_char,
    param1: *const c_char,
    param2: *const c_char,
    param3: *const c_char,
    param4: *const c_char,
);

/// UTIL_ClientPrintAll function signature
/// void UTIL_ClientPrintAll(int msg_dest, const char* msg, ...)
type ClientPrintAllFn = unsafe extern "C" fn(
    msg_dest: i32,
    msg: *const c_char,
    param1: *const c_char,
    param2: *const c_char,
    param3: *const c_char,
    param4: *const c_char,
);

/// Cached ClientPrint function pointer
static CLIENT_PRINT: OnceLock<Option<ClientPrintFn>> = OnceLock::new();

/// Cached ClientPrintAll function pointer
static CLIENT_PRINT_ALL: OnceLock<Option<ClientPrintAllFn>> = OnceLock::new();

/// Initialize print functions by scanning for signatures
///
/// # Safety
/// Module base and size must be valid for the server module.
pub unsafe fn init_print_functions(
    server_base: *const u8,
    server_size: usize,
) -> Result<(), GamedataError> {
    // Try to find ClientPrint
    let client_print = find_signature("ClientPrint", server_base, server_size).ok();
    if let Some(addr) = client_print {
        tracing::info!("Found ClientPrint at {:p}", addr);
        let _ = CLIENT_PRINT.set(Some(std::mem::transmute(addr)));
    } else {
        tracing::warn!("ClientPrint signature not found");
        let _ = CLIENT_PRINT.set(None);
    }

    // Try to find UTIL_ClientPrintAll
    let client_print_all = find_signature("UTIL_ClientPrintAll", server_base, server_size).ok();
    if let Some(addr) = client_print_all {
        tracing::info!("Found UTIL_ClientPrintAll at {:p}", addr);
        let _ = CLIENT_PRINT_ALL.set(Some(std::mem::transmute(addr)));
    } else {
        tracing::warn!("UTIL_ClientPrintAll signature not found");
        let _ = CLIENT_PRINT_ALL.set(None);
    }

    Ok(())
}

/// Check if ClientPrint is available
pub fn is_client_print_available() -> bool {
    CLIENT_PRINT.get().and_then(|opt| opt.as_ref()).is_some()
}

/// Check if ClientPrintAll is available
pub fn is_client_print_all_available() -> bool {
    CLIENT_PRINT_ALL
        .get()
        .and_then(|opt| opt.as_ref())
        .is_some()
}

/// Print a message to a specific player
///
/// # Arguments
/// * `player` - Pointer to the player controller
/// * `dest` - Where to display the message
/// * `message` - The message to send
///
/// # Safety
/// Player pointer must be valid or null.
pub unsafe fn client_print(player: *mut c_void, dest: HudDestination, message: &str) {
    if player.is_null() {
        tracing::warn!("client_print called with null player");
        return;
    }

    let Some(Some(func)) = CLIENT_PRINT.get() else {
        // Fall back to logging
        tracing::info!("[ClientPrint] {}", message);
        return;
    };

    let c_msg = match CString::new(message) {
        Ok(s) => s,
        Err(_) => {
            tracing::warn!("Invalid message for ClientPrint: contains null byte");
            return;
        }
    };

    func(
        player,
        dest as i32,
        c_msg.as_ptr(),
        std::ptr::null(),
        std::ptr::null(),
        std::ptr::null(),
        std::ptr::null(),
    );
}

/// Print a message to all players
///
/// # Arguments
/// * `dest` - Where to display the message
/// * `message` - The message to send
pub fn client_print_all(dest: HudDestination, message: &str) {
    let Some(Some(func)) = CLIENT_PRINT_ALL.get() else {
        // Fall back to logging
        tracing::info!("[ClientPrintAll] {}", message);
        return;
    };

    let c_msg = match CString::new(message) {
        Ok(s) => s,
        Err(_) => {
            tracing::warn!("Invalid message for ClientPrintAll: contains null byte");
            return;
        }
    };

    unsafe {
        func(
            dest as i32,
            c_msg.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
        );
    }
}

/// Print a message to a player's console
pub unsafe fn print_to_console(player: *mut c_void, message: &str) {
    client_print(player, HudDestination::Console, message);
}

/// Print a message to a player's chat
pub unsafe fn print_to_chat(player: *mut c_void, message: &str) {
    client_print(player, HudDestination::Talk, message);
}

/// Print a message to the center of a player's screen
pub unsafe fn print_to_center(player: *mut c_void, message: &str) {
    client_print(player, HudDestination::Center, message);
}

/// Print a message to all players' chat
pub fn print_to_chat_all(message: &str) {
    client_print_all(HudDestination::Talk, message);
}

/// Print a message to all players' consoles
pub fn print_to_console_all(message: &str) {
    client_print_all(HudDestination::Console, message);
}

/// Print a message to the center of all players' screens
pub fn print_to_center_all(message: &str) {
    client_print_all(HudDestination::Center, message);
}
