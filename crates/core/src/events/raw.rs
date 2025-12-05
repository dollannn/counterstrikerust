//! Raw IGameEvent wrapper
//!
//! Provides safe access to IGameEvent data through vtable calls.

use cs2rust_sdk::IGameEvent;
use std::ffi::{c_char, c_void, CStr, CString};

/// VTable indices for IGameEvent methods (Linux)
mod vtable {
    pub const GET_NAME: usize = 1;
    pub const GET_ID: usize = 2;
    pub const IS_RELIABLE: usize = 3;
    pub const IS_LOCAL: usize = 4;
    pub const IS_EMPTY: usize = 5;
    pub const GET_BOOL: usize = 6;
    pub const GET_INT: usize = 7;
    pub const GET_UINT64: usize = 8;
    pub const GET_FLOAT: usize = 9;
    pub const GET_STRING: usize = 10;
    pub const GET_PTR: usize = 11;
    // Skip entity-related methods (12-18)
    pub const SET_BOOL: usize = 19;
    pub const SET_INT: usize = 20;
    pub const SET_UINT64: usize = 21;
    pub const SET_FLOAT: usize = 22;
    pub const SET_STRING: usize = 23;
    pub const SET_PTR: usize = 24;
}

/// Wrapper around a raw IGameEvent pointer
///
/// Provides safe access to event data through vtable calls.
/// This is a borrowed reference - the underlying event is owned by the engine.
#[derive(Debug)]
pub struct GameEventRef {
    ptr: *mut IGameEvent,
}

// SAFETY: The event pointer is only accessed on the game thread
unsafe impl Send for GameEventRef {}
unsafe impl Sync for GameEventRef {}

impl GameEventRef {
    /// Create a new GameEventRef from a raw pointer
    ///
    /// # Safety
    /// The pointer must be a valid IGameEvent pointer from the engine.
    pub unsafe fn from_ptr(ptr: *mut IGameEvent) -> Option<Self> {
        if ptr.is_null() {
            None
        } else {
            Some(Self { ptr })
        }
    }

    /// Get the raw pointer
    pub fn as_ptr(&self) -> *mut IGameEvent {
        self.ptr
    }

    /// Get the event name
    pub fn get_name(&self) -> &str {
        unsafe {
            let vtable = *(self.ptr as *const *const *const c_void);
            let get_name_fn: extern "C" fn(*mut IGameEvent) -> *const c_char =
                std::mem::transmute(*vtable.add(vtable::GET_NAME));
            let name_ptr = get_name_fn(self.ptr);
            if name_ptr.is_null() {
                ""
            } else {
                CStr::from_ptr(name_ptr).to_str().unwrap_or("")
            }
        }
    }

    /// Get the event ID
    pub fn get_id(&self) -> i32 {
        unsafe {
            let vtable = *(self.ptr as *const *const *const c_void);
            let get_id_fn: extern "C" fn(*mut IGameEvent) -> i32 =
                std::mem::transmute(*vtable.add(vtable::GET_ID));
            get_id_fn(self.ptr)
        }
    }

    /// Check if event is reliable
    pub fn is_reliable(&self) -> bool {
        unsafe {
            let vtable = *(self.ptr as *const *const *const c_void);
            let is_reliable_fn: extern "C" fn(*mut IGameEvent) -> bool =
                std::mem::transmute(*vtable.add(vtable::IS_RELIABLE));
            is_reliable_fn(self.ptr)
        }
    }

    /// Check if event is local only
    pub fn is_local(&self) -> bool {
        unsafe {
            let vtable = *(self.ptr as *const *const *const c_void);
            let is_local_fn: extern "C" fn(*mut IGameEvent) -> bool =
                std::mem::transmute(*vtable.add(vtable::IS_LOCAL));
            is_local_fn(self.ptr)
        }
    }

    /// Get a boolean value from the event
    pub fn get_bool(&self, key: &str, default: bool) -> bool {
        let c_key = match CString::new(key) {
            Ok(s) => s,
            Err(_) => return default,
        };
        unsafe {
            let vtable = *(self.ptr as *const *const *const c_void);
            let get_bool_fn: extern "C" fn(*mut IGameEvent, *const c_char, bool) -> bool =
                std::mem::transmute(*vtable.add(vtable::GET_BOOL));
            get_bool_fn(self.ptr, c_key.as_ptr(), default)
        }
    }

    /// Get an integer value from the event
    pub fn get_int(&self, key: &str, default: i32) -> i32 {
        let c_key = match CString::new(key) {
            Ok(s) => s,
            Err(_) => return default,
        };
        unsafe {
            let vtable = *(self.ptr as *const *const *const c_void);
            let get_int_fn: extern "C" fn(*mut IGameEvent, *const c_char, i32) -> i32 =
                std::mem::transmute(*vtable.add(vtable::GET_INT));
            get_int_fn(self.ptr, c_key.as_ptr(), default)
        }
    }

    /// Get a 64-bit unsigned integer value from the event
    pub fn get_uint64(&self, key: &str, default: u64) -> u64 {
        let c_key = match CString::new(key) {
            Ok(s) => s,
            Err(_) => return default,
        };
        unsafe {
            let vtable = *(self.ptr as *const *const *const c_void);
            let get_uint64_fn: extern "C" fn(*mut IGameEvent, *const c_char, u64) -> u64 =
                std::mem::transmute(*vtable.add(vtable::GET_UINT64));
            get_uint64_fn(self.ptr, c_key.as_ptr(), default)
        }
    }

    /// Get a float value from the event
    pub fn get_float(&self, key: &str, default: f32) -> f32 {
        let c_key = match CString::new(key) {
            Ok(s) => s,
            Err(_) => return default,
        };
        unsafe {
            let vtable = *(self.ptr as *const *const *const c_void);
            let get_float_fn: extern "C" fn(*mut IGameEvent, *const c_char, f32) -> f32 =
                std::mem::transmute(*vtable.add(vtable::GET_FLOAT));
            get_float_fn(self.ptr, c_key.as_ptr(), default)
        }
    }

    /// Get a string value from the event
    pub fn get_string(&self, key: &str, default: &str) -> String {
        let c_key = match CString::new(key) {
            Ok(s) => s,
            Err(_) => return default.to_string(),
        };
        let c_default = match CString::new(default) {
            Ok(s) => s,
            Err(_) => return default.to_string(),
        };
        unsafe {
            let vtable = *(self.ptr as *const *const *const c_void);
            let get_string_fn: extern "C" fn(
                *mut IGameEvent,
                *const c_char,
                *const c_char,
            ) -> *const c_char = std::mem::transmute(*vtable.add(vtable::GET_STRING));
            let result = get_string_fn(self.ptr, c_key.as_ptr(), c_default.as_ptr());
            if result.is_null() {
                default.to_string()
            } else {
                CStr::from_ptr(result)
                    .to_str()
                    .unwrap_or(default)
                    .to_string()
            }
        }
    }

    /// Get a pointer value from the event
    pub fn get_ptr(&self, key: &str) -> *mut c_void {
        let c_key = match CString::new(key) {
            Ok(s) => s,
            Err(_) => return std::ptr::null_mut(),
        };
        unsafe {
            let vtable = *(self.ptr as *const *const *const c_void);
            let get_ptr_fn: extern "C" fn(*mut IGameEvent, *const c_char) -> *mut c_void =
                std::mem::transmute(*vtable.add(vtable::GET_PTR));
            get_ptr_fn(self.ptr, c_key.as_ptr())
        }
    }

    /// Set a boolean value on the event
    pub fn set_bool(&self, key: &str, value: bool) {
        let c_key = match CString::new(key) {
            Ok(s) => s,
            Err(_) => return,
        };
        unsafe {
            let vtable = *(self.ptr as *const *const *const c_void);
            let set_bool_fn: extern "C" fn(*mut IGameEvent, *const c_char, bool) =
                std::mem::transmute(*vtable.add(vtable::SET_BOOL));
            set_bool_fn(self.ptr, c_key.as_ptr(), value);
        }
    }

    /// Set an integer value on the event
    pub fn set_int(&self, key: &str, value: i32) {
        let c_key = match CString::new(key) {
            Ok(s) => s,
            Err(_) => return,
        };
        unsafe {
            let vtable = *(self.ptr as *const *const *const c_void);
            let set_int_fn: extern "C" fn(*mut IGameEvent, *const c_char, i32) =
                std::mem::transmute(*vtable.add(vtable::SET_INT));
            set_int_fn(self.ptr, c_key.as_ptr(), value);
        }
    }

    /// Set a 64-bit unsigned integer value on the event
    pub fn set_uint64(&self, key: &str, value: u64) {
        let c_key = match CString::new(key) {
            Ok(s) => s,
            Err(_) => return,
        };
        unsafe {
            let vtable = *(self.ptr as *const *const *const c_void);
            let set_uint64_fn: extern "C" fn(*mut IGameEvent, *const c_char, u64) =
                std::mem::transmute(*vtable.add(vtable::SET_UINT64));
            set_uint64_fn(self.ptr, c_key.as_ptr(), value);
        }
    }

    /// Set a float value on the event
    pub fn set_float(&self, key: &str, value: f32) {
        let c_key = match CString::new(key) {
            Ok(s) => s,
            Err(_) => return,
        };
        unsafe {
            let vtable = *(self.ptr as *const *const *const c_void);
            let set_float_fn: extern "C" fn(*mut IGameEvent, *const c_char, f32) =
                std::mem::transmute(*vtable.add(vtable::SET_FLOAT));
            set_float_fn(self.ptr, c_key.as_ptr(), value);
        }
    }

    /// Set a string value on the event
    pub fn set_string(&self, key: &str, value: &str) {
        let c_key = match CString::new(key) {
            Ok(s) => s,
            Err(_) => return,
        };
        let c_value = match CString::new(value) {
            Ok(s) => s,
            Err(_) => return,
        };
        unsafe {
            let vtable = *(self.ptr as *const *const *const c_void);
            let set_string_fn: extern "C" fn(*mut IGameEvent, *const c_char, *const c_char) =
                std::mem::transmute(*vtable.add(vtable::SET_STRING));
            set_string_fn(self.ptr, c_key.as_ptr(), c_value.as_ptr());
        }
    }

    /// Set a pointer value on the event
    pub fn set_ptr(&self, key: &str, value: *mut c_void) {
        let c_key = match CString::new(key) {
            Ok(s) => s,
            Err(_) => return,
        };
        unsafe {
            let vtable = *(self.ptr as *const *const *const c_void);
            let set_ptr_fn: extern "C" fn(*mut IGameEvent, *const c_char, *mut c_void) =
                std::mem::transmute(*vtable.add(vtable::SET_PTR));
            set_ptr_fn(self.ptr, c_key.as_ptr(), value);
        }
    }
}
