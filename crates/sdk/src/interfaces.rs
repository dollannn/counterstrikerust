//! Source 2 engine interface type definitions
//!
//! These are opaque types representing C++ engine interfaces.
//! We don't need their internal structure - just pointers.
//! The actual vtables are called through C++ or raw pointer arithmetic.

use std::ffi::c_void;

/// Opaque type for ISmmAPI (Metamod API)
#[repr(C)]
pub struct ISmmAPI {
    _opaque: [u8; 0],
}

/// Opaque type for IServerGameDLL
/// Primary server-side game interface
#[repr(C)]
pub struct IServerGameDLL {
    _opaque: [u8; 0],
}

/// Opaque type for IServerGameClients
/// Handles client connection, disconnection, and lifecycle events
#[repr(C)]
pub struct IServerGameClients {
    _opaque: [u8; 0],
}

/// Opaque type for CSchemaSystem
/// Used for runtime type information and field offset lookup
#[repr(C)]
pub struct CSchemaSystem {
    _opaque: [u8; 0],
}

/// Opaque type for IGameEventSystem
/// Source 2's new game event system (replaces IGameEventManager2)
#[repr(C)]
pub struct IGameEventSystem {
    _opaque: [u8; 0],
}

/// Opaque type for IGameEventManager2
/// Legacy game event system (used for game events like player_death, round_start)
#[repr(C)]
pub struct IGameEventManager2 {
    _opaque: [u8; 0],
}

/// Opaque type for IGameEvent
/// Represents a single game event instance with key-value data
#[repr(C)]
pub struct IGameEvent {
    _opaque: [u8; 0],
}

/// Opaque type for IGameEventListener2
/// Interface for receiving game event notifications
#[repr(C)]
pub struct IGameEventListener2 {
    _opaque: [u8; 0],
}

/// Opaque type for CGlobalVars
/// Global game variables (tick count, frametime, etc.)
#[repr(C)]
pub struct CGlobalVars {
    _opaque: [u8; 0],
}

/// Opaque type for INetworkServerService
/// Network server management
#[repr(C)]
pub struct INetworkServerService {
    _opaque: [u8; 0],
}

/// Opaque type for IEngineServiceMgr
/// Engine service manager
#[repr(C)]
pub struct IEngineServiceMgr {
    _opaque: [u8; 0],
}

/// Opaque type for ISource2GameEntities
/// Game entity management interface
#[repr(C)]
pub struct ISource2GameEntities {
    _opaque: [u8; 0],
}

/// Opaque type for CGameEntitySystem
/// Entity system (acquired via StartupServer hook, not CreateInterface)
#[repr(C)]
pub struct CGameEntitySystem {
    _opaque: [u8; 0],
}

/// Opaque type for ISource2Server
/// Server interface
#[repr(C)]
pub struct ISource2Server {
    _opaque: [u8; 0],
}

/// Opaque type for ICvar
/// Console variable system
#[repr(C)]
pub struct ICvar {
    _opaque: [u8; 0],
}

/// CreateInterface function signature
///
/// This is the standard Source engine pattern for acquiring interfaces.
/// Each module (server.dll, engine2.dll, etc.) exports a CreateInterface function.
///
/// # Arguments
/// * `name` - Interface version string (e.g., "Source2Server001")
/// * `return_code` - Optional pointer to receive error code (0 = success)
///
/// # Returns
/// Pointer to the interface, or null if not found
pub type CreateInterfaceFn =
    unsafe extern "C" fn(name: *const std::ffi::c_char, return_code: *mut i32) -> *mut c_void;
