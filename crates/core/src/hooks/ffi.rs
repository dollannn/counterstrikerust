//! FFI bindings to SafetyHook C++ bridge
//!
//! These functions are implemented in `crates/plugin/cpp/safetyhook_bridge.cpp`
//! and linked at compile time through the plugin crate.

use std::ffi::c_void;

/// Opaque handle for inline hooks
#[repr(C)]
pub struct InlineHookHandle {
    _opaque: [u8; 0],
}

/// Opaque handle for mid-function hooks
#[repr(C)]
pub struct MidHookHandle {
    _opaque: [u8; 0],
}

/// Result codes from SafetyHook operations
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookResult {
    Success = 0,
    ErrorAllocation = 1,
    ErrorDecode = 2,
    ErrorUnprotect = 3,
    ErrorNotEnoughSpace = 4,
    ErrorUnsupported = 5,
    ErrorIpRelative = 6,
    ErrorInvalid = 7,
}

impl HookResult {
    pub fn is_success(&self) -> bool {
        matches!(self, HookResult::Success)
    }

    pub fn to_error_string(&self) -> &'static str {
        match self {
            HookResult::Success => "Success",
            HookResult::ErrorAllocation => "Failed to allocate memory for hook",
            HookResult::ErrorDecode => "Failed to decode instruction at target",
            HookResult::ErrorUnprotect => "Failed to change memory protection",
            HookResult::ErrorNotEnoughSpace => "Not enough space at target for hook",
            HookResult::ErrorUnsupported => "Unsupported instruction in trampoline",
            HookResult::ErrorIpRelative => "IP-relative instruction out of range",
            HookResult::ErrorInvalid => "Invalid handle or parameter",
        }
    }
}

/// Context structure matching C++ RustMidHookContext exactly.
/// This is passed to mid-hook callbacks and allows reading/modifying registers.
#[repr(C)]
pub struct RustMidHookContext {
    pub xmm: [[u8; 16]; 16], // 16 XMM registers, 16 bytes each
    pub rflags: u64,
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rbp: u64,
    pub rdx: u64,
    pub rcx: u64,
    pub rbx: u64,
    pub rax: u64,
    pub rsp: u64,
}

/// Callback type for mid-function hooks.
/// Receives a pointer to the context and user data.
pub type MidHookCallback = extern "C" fn(*mut RustMidHookContext, *mut c_void);

extern "C" {
    // === Inline Hook API ===

    /// Create an inline hook at target, redirecting to destination.
    /// On success, returns the trampoline (original function) pointer.
    pub fn safetyhook_create_inline(
        target: *const c_void,
        destination: *const c_void,
        out_handle: *mut *mut InlineHookHandle,
        out_trampoline: *mut *const c_void,
    ) -> HookResult;

    /// Enable a previously disabled inline hook.
    pub fn safetyhook_enable_inline(handle: *mut InlineHookHandle) -> HookResult;

    /// Disable an inline hook (can be re-enabled later).
    pub fn safetyhook_disable_inline(handle: *mut InlineHookHandle) -> HookResult;

    /// Destroy an inline hook and free all resources.
    pub fn safetyhook_destroy_inline(handle: *mut InlineHookHandle);

    /// Check if an inline hook is currently enabled.
    pub fn safetyhook_is_inline_enabled(handle: *mut InlineHookHandle) -> bool;

    /// Get the trampoline address for an inline hook.
    pub fn safetyhook_get_inline_trampoline(handle: *mut InlineHookHandle) -> *const c_void;

    // === Mid Hook API ===

    /// Create a mid-function hook with full register context access.
    pub fn safetyhook_create_mid(
        target: *const c_void,
        callback: MidHookCallback,
        user_data: *mut c_void,
        out_handle: *mut *mut MidHookHandle,
    ) -> HookResult;

    /// Enable a previously disabled mid hook.
    pub fn safetyhook_enable_mid(handle: *mut MidHookHandle) -> HookResult;

    /// Disable a mid hook (can be re-enabled later).
    pub fn safetyhook_disable_mid(handle: *mut MidHookHandle) -> HookResult;

    /// Destroy a mid hook and free all resources.
    pub fn safetyhook_destroy_mid(handle: *mut MidHookHandle);

    /// Check if a mid hook is currently enabled.
    pub fn safetyhook_is_mid_enabled(handle: *mut MidHookHandle) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem;

    #[test]
    fn test_context_size() {
        // Verify RustMidHookContext matches expected size
        // 256 (xmm) + 8*17 (registers: rflags + r15-r8 + rdi,rsi,rbp,rdx,rcx,rbx,rax + rsp)
        // = 256 + 136 = 392 bytes
        assert_eq!(mem::size_of::<RustMidHookContext>(), 392);
    }

    #[test]
    fn test_context_alignment() {
        // Context should be naturally aligned
        assert!(mem::align_of::<RustMidHookContext>() >= 8);
    }
}
