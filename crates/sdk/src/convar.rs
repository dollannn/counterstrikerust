//! ConVar type definitions for Source 2
//!
//! This module provides type definitions for the Source 2 ConVar system,
//! including the ConVarRef index-based reference, EConVarType enum,
//! CVValue union, and ConVarData structure.

use std::ffi::c_char;

/// Invalid convar access index constant
pub const INVALID_CONVAR_INDEX: u16 = 0xFFFF;

/// ConVar type enumeration
///
/// Maps to EConVarType in the engine (tier1/convar.h)
#[repr(i16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EConVarType {
    Invalid = -1,
    Bool = 0,
    Int16 = 1,
    UInt16 = 2,
    Int32 = 3,
    UInt32 = 4,
    Int64 = 5,
    UInt64 = 6,
    Float32 = 7,
    Float64 = 8,
    String = 9,
    Color = 10,
    Vector2 = 11,
    Vector3 = 12,
    Vector4 = 13,
    Qangle = 14,
}

impl EConVarType {
    /// Check if this is a primitive (non-reference) type
    pub fn is_primitive(&self) -> bool {
        matches!(
            self,
            Self::Bool
                | Self::Int16
                | Self::UInt16
                | Self::Int32
                | Self::UInt32
                | Self::Int64
                | Self::UInt64
                | Self::Float32
                | Self::Float64
        )
    }
}

/// ConVar reference - uses access index, NOT direct pointer
///
/// This is how Source 2 references convars. The access_index is used
/// with ICvar::GetConVarData to get the actual ConVarData pointer.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ConVarRef {
    /// Access index into ICvar's internal convar list
    /// 0xFFFF = invalid
    pub access_index: u16,
    /// Registration index (set when convar is registered)
    pub registered_index: i32,
}

impl ConVarRef {
    /// Create an invalid reference
    pub const fn invalid() -> Self {
        Self {
            access_index: INVALID_CONVAR_INDEX,
            registered_index: 0,
        }
    }

    /// Check if this reference is valid
    pub fn is_valid(&self) -> bool {
        self.access_index != INVALID_CONVAR_INDEX
    }
}

impl Default for ConVarRef {
    fn default() -> Self {
        Self::invalid()
    }
}

/// Union containing all possible convar value types
///
/// Based on CVValue_t from HL2SDK (tier1/convar.h).
/// For string values, the string_ptr points to internal engine memory.
#[repr(C)]
#[derive(Clone, Copy)]
pub union CVValue {
    pub bool_value: bool,
    pub i16_value: i16,
    pub u16_value: u16,
    pub i32_value: i32,
    pub u32_value: u32,
    pub i64_value: i64,
    pub u64_value: u64,
    pub f32_value: f32,
    pub f64_value: f64,
    /// For String type - pointer to CUtlString internal buffer
    /// Note: CUtlString is more complex but we only need the char*
    pub string_data: [u8; 24], // CUtlString is typically 16-24 bytes
    pub color: [u8; 4],        // Color RGBA
    pub vec2: [f32; 2],        // Vector2D
    pub vec3: [f32; 3],        // Vector
    pub vec4: [f32; 4],        // Vector4D
    pub qangle: [f32; 3],      // QAngle
}

impl Default for CVValue {
    fn default() -> Self {
        Self { u64_value: 0 }
    }
}

impl std::fmt::Debug for CVValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Just show raw bytes since we don't know the type
        f.debug_struct("CVValue")
            .field("raw", unsafe { &self.u64_value })
            .finish()
    }
}

/// ConVar data structure - the actual convar storage
///
/// This is what ICvar::GetConVarData returns.
/// Based on ConVarData from HL2SDK (tier1/convar.h).
#[repr(C)]
pub struct ConVarData {
    /// ConVar name (null-terminated C string)
    pub name: *const c_char,

    /// Default value pointer
    pub default_value: *mut CVValue,

    /// Minimum value pointer (null if no min)
    pub min_value: *const CVValue,

    /// Maximum value pointer (null if no max)
    pub max_value: *const CVValue,

    /// Help string
    pub help_string: *const c_char,

    /// Value type
    pub var_type: EConVarType,

    /// Version (from gameinfo)
    pub version: i16,

    /// Times value has changed
    pub times_changed: u32,

    /// ConVar flags (FCVAR_*)
    pub flags: u64,

    /// Callback index into internal list
    pub callback_index: u32,

    /// Filter callback index
    pub filter_cb_index: u32,

    /// Completion callback index
    pub completion_cb_index: u32,

    /// GameInfo flags
    pub game_info_flags: i32,

    /// UserInfo byte index
    pub user_info_byte_index: i32,

    /// Padding to align m_Values
    _pad: u32,

    /// Values array (one per split-screen slot)
    /// For dedicated servers, only slot 0 is used
    pub values: [CVValue; 1],
}

// ConVar flag constants
pub mod flags {
    /// No flags
    pub const FCVAR_NONE: u64 = 0;
    /// Allows concommand callback chaining
    pub const FCVAR_LINKED_CONCOMMAND: u64 = 1 << 0;
    /// Hidden in released products
    pub const FCVAR_DEVELOPMENTONLY: u64 = 1 << 1;
    /// Defined by the game DLL
    pub const FCVAR_GAMEDLL: u64 = 1 << 2;
    /// Defined by the client DLL
    pub const FCVAR_CLIENTDLL: u64 = 1 << 3;
    /// Hidden from find/autocomplete
    pub const FCVAR_HIDDEN: u64 = 1 << 4;
    /// Protected (password-like)
    pub const FCVAR_PROTECTED: u64 = 1 << 5;
    /// Singleplayer only
    pub const FCVAR_SPONLY: u64 = 1 << 6;
    /// Saved to config
    pub const FCVAR_ARCHIVE: u64 = 1 << 7;
    /// Notify players when changed
    pub const FCVAR_NOTIFY: u64 = 1 << 8;
    /// Changes client info string
    pub const FCVAR_USERINFO: u64 = 1 << 9;
    /// ConVar is a reference
    pub const FCVAR_REFERENCE: u64 = 1 << 10;
    /// Don't log changes
    pub const FCVAR_UNLOGGED: u64 = 1 << 11;
    /// Initial value set
    pub const FCVAR_INITIAL_SETVALUE: u64 = 1 << 12;
    /// Replicated to clients
    pub const FCVAR_REPLICATED: u64 = 1 << 13;
    /// Only with sv_cheats
    pub const FCVAR_CHEAT: u64 = 1 << 14;
    /// Per-user (splitscreen)
    pub const FCVAR_PER_USER: u64 = 1 << 15;
    /// Record in demo
    pub const FCVAR_DEMO: u64 = 1 << 16;
    /// Don't record in demo
    pub const FCVAR_DONTRECORD: u64 = 1 << 17;
    /// Currently calling callbacks
    pub const FCVAR_PERFORMING_CALLBACKS: u64 = 1 << 18;
    /// Available in release
    pub const FCVAR_RELEASE: u64 = 1 << 19;
    /// Menubar item
    pub const FCVAR_MENUBAR_ITEM: u64 = 1 << 20;
    /// Command-line enforced
    pub const FCVAR_COMMANDLINE_ENFORCED: u64 = 1 << 21;
    /// Cannot change when connected
    pub const FCVAR_NOT_CONNECTED: u64 = 1 << 22;
    /// VConsole fuzzy matching
    pub const FCVAR_VCONSOLE_FUZZY_MATCHING: u64 = 1 << 23;
    /// Server can execute on clients
    pub const FCVAR_SERVER_CAN_EXECUTE: u64 = 1 << 24;
    /// Client can execute
    pub const FCVAR_CLIENT_CAN_EXECUTE: u64 = 1 << 25;
    /// Server cannot query
    pub const FCVAR_SERVER_CANNOT_QUERY: u64 = 1 << 26;
    /// VConsole set focus
    pub const FCVAR_VCONSOLE_SET_FOCUS: u64 = 1 << 27;
    /// ClientCmd can execute
    pub const FCVAR_CLIENTCMD_CAN_EXECUTE: u64 = 1 << 28;
    /// Execute per tick
    pub const FCVAR_EXECUTE_PER_TICK: u64 = 1 << 29;
    /// Defensive flag
    pub const FCVAR_DEFENSIVE: u64 = 1 << 32;
}
