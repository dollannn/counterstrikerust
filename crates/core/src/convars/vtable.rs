//! ICvar vtable method wrappers
//!
//! Provides safe wrappers around ICvar vtable methods for convar access.
//! VTable indices can be configured via gamedata.

use std::ffi::{c_char, c_void, CString};

use crate::engine::engine;
use crate::gamedata::gamedata;
use cs2rust_sdk::{CVValue, ConVarData, ConVarRef, ICvar};

/// Default VTable indices for ICvar methods (Linux)
/// These are offsets from the start of the ICvar vtable, which comes after IAppSystem.
/// Values can be overridden via gamedata.
#[cfg(target_os = "linux")]
mod default_indices {
    /// ICvar::FindConVar - first method after IAppSystem base
    pub const FIND_CONVAR: usize = 9;
    /// ICvar::CallChangeCallback - triggers change callbacks
    pub const CALL_CHANGE_CALLBACK: usize = 12;
    /// ICvar::GetConVarData - get internal data pointer
    pub const GET_CONVAR_DATA: usize = 37;
}

#[cfg(target_os = "windows")]
mod default_indices {
    pub const FIND_CONVAR: usize = 9;
    pub const CALL_CHANGE_CALLBACK: usize = 12;
    pub const GET_CONVAR_DATA: usize = 37;
}

/// Function signature for ICvar::FindConVar
/// ConVarRef FindConVar(const char* name, bool allow_defensive)
type FindConVarFn = unsafe extern "C" fn(
    this: *mut ICvar,
    name: *const c_char,
    allow_defensive: bool,
) -> ConVarRef;

/// Function signature for ICvar::GetConVarData
/// ConVarData* GetConVarData(ConVarRef cvar)
type GetConVarDataFn = unsafe extern "C" fn(this: *mut ICvar, cvar_ref: ConVarRef) -> *mut ConVarData;

/// Function signature for ICvar::CallChangeCallback
/// void CallChangeCallback(ConVarRef cvar, CSplitScreenSlot slot, CVValue_t* new, CVValue_t* old, void* unk)
type CallChangeCallbackFn = unsafe extern "C" fn(
    this: *mut ICvar,
    cvar_ref: ConVarRef,
    slot: i32,
    new_value: *const CVValue,
    old_value: *const CVValue,
    unk: *mut c_void,
);

/// Get vtable pointer from an interface
#[inline]
unsafe fn get_vtable(ptr: *mut ICvar) -> *const *const c_void {
    *(ptr as *const *const *const c_void)
}

/// Get vtable index, checking gamedata first
fn get_index(gamedata_key: &str, default: usize) -> usize {
    gamedata()
        .and_then(|gd| gd.get_offset(gamedata_key).ok())
        .map(|o| o as usize)
        .unwrap_or(default)
}

/// Find a convar by name
///
/// # Arguments
/// * `name` - The convar name to search for
///
/// # Returns
/// ConVarRef with access_index = 0xFFFF if not found
pub fn find_convar(name: &str) -> ConVarRef {
    let cvar = engine().cvar_ptr();
    if cvar.is_null() {
        tracing::warn!("ICvar pointer is null");
        return ConVarRef::invalid();
    }

    let c_name = match CString::new(name) {
        Ok(s) => s,
        Err(_) => {
            tracing::warn!("Invalid convar name: contains null byte");
            return ConVarRef::invalid();
        }
    };

    let index = get_index("ICvar_FindConVar", default_indices::FIND_CONVAR);

    unsafe {
        let vtable = get_vtable(cvar);
        let func: FindConVarFn = std::mem::transmute(*vtable.add(index));
        func(cvar, c_name.as_ptr(), false)
    }
}

/// Get ConVarData from a ConVarRef
///
/// # Arguments
/// * `cvar_ref` - The convar reference obtained from find_convar
///
/// # Returns
/// Pointer to ConVarData, or null if the ref is invalid
pub fn get_convar_data(cvar_ref: ConVarRef) -> *mut ConVarData {
    if !cvar_ref.is_valid() {
        return std::ptr::null_mut();
    }

    let cvar = engine().cvar_ptr();
    if cvar.is_null() {
        return std::ptr::null_mut();
    }

    let index = get_index("ICvar_GetConVarData", default_indices::GET_CONVAR_DATA);

    unsafe {
        let vtable = get_vtable(cvar);
        let func: GetConVarDataFn = std::mem::transmute(*vtable.add(index));
        func(cvar, cvar_ref)
    }
}

/// Call the engine's change callback for a convar
///
/// # Arguments
/// * `cvar_ref` - The convar reference
/// * `slot` - Split-screen slot (use 0 for dedicated servers)
/// * `new_value` - Pointer to new value
/// * `old_value` - Pointer to old value
///
/// # Safety
/// The value pointers must be valid and match the convar's type.
pub unsafe fn call_change_callback(
    cvar_ref: ConVarRef,
    slot: i32,
    new_value: *const CVValue,
    old_value: *const CVValue,
) {
    if !cvar_ref.is_valid() {
        return;
    }

    let cvar = engine().cvar_ptr();
    if cvar.is_null() {
        return;
    }

    let index = get_index("ICvar_CallChangeCallback", default_indices::CALL_CHANGE_CALLBACK);

    let vtable = get_vtable(cvar);
    let func: CallChangeCallbackFn = std::mem::transmute(*vtable.add(index));
    func(cvar, cvar_ref, slot, new_value, old_value, std::ptr::null_mut());
}
