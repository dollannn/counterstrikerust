//! Real ConVar access wrapper
//!
//! Provides a safe wrapper for accessing and modifying game convars.

use std::ffi::CStr;

use cs2rust_sdk::{CVValue, ConVarData, ConVarRef, EConVarType};

use super::vtable::{call_change_callback, find_convar, get_convar_data};

/// Wrapper for accessing real game ConVars
///
/// ConVars are accessed via index reference, not direct pointer.
/// The actual data is retrieved through ICvar::GetConVarData.
///
/// # Example
/// ```ignore
/// use cs2rust_core::convars::ConVar;
///
/// if let Some(cheats) = ConVar::find("sv_cheats") {
///     if cheats.get_bool() {
///         tracing::warn!("Cheats are enabled!");
///     }
/// }
/// ```
pub struct ConVar {
    /// Reference to the convar (access index)
    cvar_ref: ConVarRef,
    /// Cached name for error messages
    name: String,
}

impl ConVar {
    /// Find a ConVar by name
    ///
    /// Returns None if the ConVar doesn't exist.
    pub fn find(name: &str) -> Option<Self> {
        let cvar_ref = find_convar(name);
        if !cvar_ref.is_valid() {
            return None;
        }

        Some(Self {
            cvar_ref,
            name: name.to_string(),
        })
    }

    /// Get the ConVarData pointer
    fn data(&self) -> Option<&ConVarData> {
        let ptr = get_convar_data(self.cvar_ref);
        if ptr.is_null() {
            return None;
        }
        unsafe { Some(&*ptr) }
    }

    /// Get mutable ConVarData pointer
    fn data_mut(&self) -> Option<&mut ConVarData> {
        let ptr = get_convar_data(self.cvar_ref);
        if ptr.is_null() {
            return None;
        }
        unsafe { Some(&mut *ptr) }
    }

    /// Get the current value pointer (slot 0 for servers)
    fn value_ptr(&self) -> Option<*mut CVValue> {
        let data = self.data()?;
        // For dedicated servers, only slot 0 is used
        Some(data.values.as_ptr() as *mut CVValue)
    }

    /// Get the ConVar name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the ConVarRef for low-level access
    pub fn cvar_ref(&self) -> ConVarRef {
        self.cvar_ref
    }

    /// Get the access index
    pub fn access_index(&self) -> u16 {
        self.cvar_ref.access_index
    }

    /// Get the ConVar type
    pub fn var_type(&self) -> EConVarType {
        self.data()
            .map(|d| d.var_type)
            .unwrap_or(EConVarType::Invalid)
    }

    /// Get the ConVar flags
    pub fn flags(&self) -> u64 {
        self.data().map(|d| d.flags).unwrap_or(0)
    }

    /// Get help text
    pub fn help_text(&self) -> &str {
        self.data()
            .and_then(|d| {
                if d.help_string.is_null() {
                    None
                } else {
                    unsafe { CStr::from_ptr(d.help_string).to_str().ok() }
                }
            })
            .unwrap_or("")
    }

    /// Get the number of times this convar has been changed
    pub fn times_changed(&self) -> u32 {
        self.data().map(|d| d.times_changed).unwrap_or(0)
    }

    // ==================== VALUE GETTERS ====================

    /// Get value as bool
    ///
    /// Returns false if type doesn't match or value is invalid.
    /// Performs type coercion for numeric types.
    pub fn get_bool(&self) -> bool {
        let Some(value_ptr) = self.value_ptr() else {
            return false;
        };

        unsafe {
            match self.var_type() {
                EConVarType::Bool => (*value_ptr).bool_value,
                EConVarType::Int16 => (*value_ptr).i16_value != 0,
                EConVarType::UInt16 => (*value_ptr).u16_value != 0,
                EConVarType::Int32 => (*value_ptr).i32_value != 0,
                EConVarType::UInt32 => (*value_ptr).u32_value != 0,
                EConVarType::Int64 => (*value_ptr).i64_value != 0,
                EConVarType::UInt64 => (*value_ptr).u64_value != 0,
                EConVarType::Float32 => (*value_ptr).f32_value != 0.0,
                EConVarType::Float64 => (*value_ptr).f64_value != 0.0,
                _ => false,
            }
        }
    }

    /// Get value as i32
    ///
    /// Performs type coercion for compatible types.
    pub fn get_int(&self) -> i32 {
        let Some(value_ptr) = self.value_ptr() else {
            return 0;
        };

        unsafe {
            match self.var_type() {
                EConVarType::Bool => (*value_ptr).bool_value as i32,
                EConVarType::Int16 => (*value_ptr).i16_value as i32,
                EConVarType::UInt16 => (*value_ptr).u16_value as i32,
                EConVarType::Int32 => (*value_ptr).i32_value,
                EConVarType::UInt32 => (*value_ptr).u32_value as i32,
                EConVarType::Int64 => (*value_ptr).i64_value as i32,
                EConVarType::UInt64 => (*value_ptr).u64_value as i32,
                EConVarType::Float32 => (*value_ptr).f32_value as i32,
                EConVarType::Float64 => (*value_ptr).f64_value as i32,
                _ => 0,
            }
        }
    }

    /// Get value as f32
    ///
    /// Performs type coercion for compatible types.
    pub fn get_float(&self) -> f32 {
        let Some(value_ptr) = self.value_ptr() else {
            return 0.0;
        };

        unsafe {
            match self.var_type() {
                EConVarType::Bool => (*value_ptr).bool_value as i32 as f32,
                EConVarType::Int16 => (*value_ptr).i16_value as f32,
                EConVarType::UInt16 => (*value_ptr).u16_value as f32,
                EConVarType::Int32 => (*value_ptr).i32_value as f32,
                EConVarType::UInt32 => (*value_ptr).u32_value as f32,
                EConVarType::Int64 => (*value_ptr).i64_value as f32,
                EConVarType::UInt64 => (*value_ptr).u64_value as f32,
                EConVarType::Float32 => (*value_ptr).f32_value,
                EConVarType::Float64 => (*value_ptr).f64_value as f32,
                _ => 0.0,
            }
        }
    }

    /// Get value as string
    ///
    /// For string convars, returns the actual string.
    /// For other types, returns a string representation.
    pub fn get_string(&self) -> String {
        let Some(data) = self.data() else {
            return String::new();
        };

        let Some(value_ptr) = self.value_ptr() else {
            return String::new();
        };

        unsafe {
            match data.var_type {
                EConVarType::String => {
                    // CUtlString stores a pointer at the start of the data
                    // The first 8 bytes are the string pointer
                    let str_ptr = *(value_ptr as *const *const i8);
                    if str_ptr.is_null() {
                        String::new()
                    } else {
                        CStr::from_ptr(str_ptr).to_str().unwrap_or("").to_string()
                    }
                }
                EConVarType::Bool => (*value_ptr).bool_value.to_string(),
                EConVarType::Int16 => (*value_ptr).i16_value.to_string(),
                EConVarType::UInt16 => (*value_ptr).u16_value.to_string(),
                EConVarType::Int32 => (*value_ptr).i32_value.to_string(),
                EConVarType::UInt32 => (*value_ptr).u32_value.to_string(),
                EConVarType::Int64 => (*value_ptr).i64_value.to_string(),
                EConVarType::UInt64 => (*value_ptr).u64_value.to_string(),
                EConVarType::Float32 => (*value_ptr).f32_value.to_string(),
                EConVarType::Float64 => (*value_ptr).f64_value.to_string(),
                _ => String::new(),
            }
        }
    }

    // ==================== VALUE SETTERS ====================

    /// Set value as bool
    ///
    /// Calls engine change callbacks after setting the value.
    pub fn set_bool(&self, value: bool) {
        self.set_value_internal(|ptr, var_type| unsafe {
            match var_type {
                EConVarType::Bool => (*ptr).bool_value = value,
                EConVarType::Int16 => (*ptr).i16_value = value as i16,
                EConVarType::UInt16 => (*ptr).u16_value = value as u16,
                EConVarType::Int32 => (*ptr).i32_value = value as i32,
                EConVarType::UInt32 => (*ptr).u32_value = value as u32,
                EConVarType::Int64 => (*ptr).i64_value = value as i64,
                EConVarType::UInt64 => (*ptr).u64_value = value as u64,
                EConVarType::Float32 => (*ptr).f32_value = if value { 1.0 } else { 0.0 },
                EConVarType::Float64 => (*ptr).f64_value = if value { 1.0 } else { 0.0 },
                _ => return false,
            }
            true
        });
    }

    /// Set value as i32
    ///
    /// Calls engine change callbacks after setting the value.
    pub fn set_int(&self, value: i32) {
        self.set_value_internal(|ptr, var_type| unsafe {
            match var_type {
                EConVarType::Bool => (*ptr).bool_value = value != 0,
                EConVarType::Int16 => (*ptr).i16_value = value as i16,
                EConVarType::UInt16 => (*ptr).u16_value = value as u16,
                EConVarType::Int32 => (*ptr).i32_value = value,
                EConVarType::UInt32 => (*ptr).u32_value = value as u32,
                EConVarType::Int64 => (*ptr).i64_value = value as i64,
                EConVarType::UInt64 => (*ptr).u64_value = value as u64,
                EConVarType::Float32 => (*ptr).f32_value = value as f32,
                EConVarType::Float64 => (*ptr).f64_value = value as f64,
                _ => return false,
            }
            true
        });
    }

    /// Set value as f32
    ///
    /// Calls engine change callbacks after setting the value.
    pub fn set_float(&self, value: f32) {
        self.set_value_internal(|ptr, var_type| unsafe {
            match var_type {
                EConVarType::Bool => (*ptr).bool_value = value != 0.0,
                EConVarType::Int16 => (*ptr).i16_value = value as i16,
                EConVarType::UInt16 => (*ptr).u16_value = value as u16,
                EConVarType::Int32 => (*ptr).i32_value = value as i32,
                EConVarType::UInt32 => (*ptr).u32_value = value as u32,
                EConVarType::Int64 => (*ptr).i64_value = value as i64,
                EConVarType::UInt64 => (*ptr).u64_value = value as u64,
                EConVarType::Float32 => (*ptr).f32_value = value,
                EConVarType::Float64 => (*ptr).f64_value = value as f64,
                _ => return false,
            }
            true
        });
    }

    /// Internal helper for setting values with change callbacks
    fn set_value_internal<F>(&self, setter: F)
    where
        F: FnOnce(*mut CVValue, EConVarType) -> bool,
    {
        let Some(value_ptr) = self.value_ptr() else {
            return;
        };

        let var_type = self.var_type();

        // Store old value for callback
        let old_value = unsafe { *value_ptr };

        // Apply the new value
        if !setter(value_ptr, var_type) {
            return;
        }

        // Get new value for callback
        let new_value = unsafe { *value_ptr };

        // Increment times_changed
        if let Some(data) = self.data_mut() {
            data.times_changed = data.times_changed.wrapping_add(1);
        }

        // Call engine change callback
        unsafe {
            call_change_callback(self.cvar_ref, 0, &new_value, &old_value);
        }
    }
}

impl std::fmt::Debug for ConVar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConVar")
            .field("name", &self.name)
            .field("type", &self.var_type())
            .field("flags", &format_args!("0x{:x}", self.flags()))
            .field("access_index", &self.cvar_ref.access_index)
            .finish()
    }
}

impl std::fmt::Display for ConVar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} = {}", self.name, self.get_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require a running game server to work
    // They are marked as ignored by default

    #[test]
    #[ignore]
    fn test_find_convar() {
        let sv_cheats = ConVar::find("sv_cheats");
        assert!(sv_cheats.is_some());

        let nonexistent = ConVar::find("this_convar_does_not_exist_12345");
        assert!(nonexistent.is_none());
    }

    #[test]
    #[ignore]
    fn test_get_bool() {
        if let Some(sv_cheats) = ConVar::find("sv_cheats") {
            let _ = sv_cheats.get_bool(); // Just check it doesn't crash
        }
    }
}
