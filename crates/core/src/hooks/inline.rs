//! Inline function hooks using SafetyHook
//!
//! Provides cross-platform function detouring for x86_64 using SafetyHook.
//! SafetyHook provides proper hook chaining for multi-framework compatibility.

use parking_lot::RwLock;
use slotmap::{new_key_type, SlotMap};
use std::ffi::c_void;
use std::sync::LazyLock;

use super::ffi;

new_key_type! {
    /// Handle for an inline hook
    pub struct InlineHookKey;
}

/// Error type for hook operations
#[derive(Debug, thiserror::Error)]
pub enum HookError {
    #[error("Failed to create detour: {0}")]
    DetourCreation(String),

    #[error("Failed to enable hook: {0}")]
    EnableFailed(String),

    #[error("Failed to disable hook: {0}")]
    DisableFailed(String),

    #[error("Hook not found")]
    NotFound,

    #[error("Memory protection failed: {0}")]
    MemoryProtection(String),

    #[error("Invalid address: {0:x}")]
    InvalidAddress(usize),

    #[error("Hook already enabled")]
    AlreadyEnabled,

    #[error("Hook already disabled")]
    AlreadyDisabled,

    #[error("Instruction relocation failed: {0}")]
    RelocationFailed(String),
}

impl From<ffi::HookResult> for HookError {
    fn from(result: ffi::HookResult) -> Self {
        HookError::DetourCreation(result.to_error_string().to_string())
    }
}

/// Internal storage for an inline hook
struct InlineHookEntry {
    /// SafetyHook handle
    handle: *mut ffi::InlineHookHandle,

    /// Target function address (for logging/debugging)
    target: usize,

    /// Trampoline (original function) pointer
    trampoline: *const (),

    /// Whether the hook is currently enabled
    enabled: bool,

    /// Description for debugging
    name: String,
}

// SAFETY: Hook entries are protected by RwLock, handles are thread-safe
unsafe impl Send for InlineHookEntry {}
unsafe impl Sync for InlineHookEntry {}

impl Drop for InlineHookEntry {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe {
                ffi::safetyhook_destroy_inline(self.handle);
            }
        }
    }
}

/// Global inline hook registry
static INLINE_HOOKS: LazyLock<RwLock<SlotMap<InlineHookKey, InlineHookEntry>>> =
    LazyLock::new(|| RwLock::new(SlotMap::with_key()));

/// Create an inline hook for a function
///
/// # Safety
/// - `target` must be a valid function pointer
/// - `detour` must be a valid function pointer with a compatible signature
///
/// # Arguments
/// * `name` - Debug name for the hook
/// * `target` - Pointer to the function to hook
/// * `detour` - Your replacement function pointer
///
/// # Returns
/// A key to manage the hook, and a pointer to call the original function
pub unsafe fn create_inline_hook(
    name: &str,
    target: *const (),
    detour: *const (),
) -> Result<(InlineHookKey, *const ()), HookError> {
    tracing::debug!(
        "Creating inline hook '{}' at {:x} -> {:x}",
        name,
        target as usize,
        detour as usize
    );

    let mut handle: *mut ffi::InlineHookHandle = std::ptr::null_mut();
    let mut trampoline: *const c_void = std::ptr::null();

    let result = ffi::safetyhook_create_inline(
        target as *const c_void,
        detour as *const c_void,
        &mut handle,
        &mut trampoline,
    );

    if !result.is_success() {
        tracing::error!(
            "Failed to create inline hook '{}': {}",
            name,
            result.to_error_string()
        );
        return Err(result.into());
    }

    let entry = InlineHookEntry {
        handle,
        target: target as usize,
        trampoline: trampoline as *const (),
        enabled: true,
        name: name.to_string(),
    };

    let key = INLINE_HOOKS.write().insert(entry);

    tracing::info!("Created inline hook '{}' at {:x}", name, target as usize);

    Ok((key, trampoline as *const ()))
}

/// Enable an inline hook
pub fn enable_inline_hook(key: InlineHookKey) -> Result<(), HookError> {
    let mut hooks = INLINE_HOOKS.write();
    let entry = hooks.get_mut(key).ok_or(HookError::NotFound)?;

    if entry.enabled {
        return Ok(()); // Already enabled
    }

    let result = unsafe { ffi::safetyhook_enable_inline(entry.handle) };

    if !result.is_success() {
        return Err(HookError::EnableFailed(result.to_error_string().to_string()));
    }

    entry.enabled = true;
    tracing::info!(
        "Enabled inline hook '{}' at {:x}",
        entry.name,
        entry.target
    );
    Ok(())
}

/// Disable an inline hook (keeps it installed but restores original bytes)
pub fn disable_inline_hook(key: InlineHookKey) -> Result<(), HookError> {
    let mut hooks = INLINE_HOOKS.write();
    let entry = hooks.get_mut(key).ok_or(HookError::NotFound)?;

    if !entry.enabled {
        return Ok(()); // Already disabled
    }

    let result = unsafe { ffi::safetyhook_disable_inline(entry.handle) };

    if !result.is_success() {
        return Err(HookError::DisableFailed(
            result.to_error_string().to_string(),
        ));
    }

    entry.enabled = false;
    tracing::info!(
        "Disabled inline hook '{}' at {:x}",
        entry.name,
        entry.target
    );
    Ok(())
}

/// Remove an inline hook completely
pub fn remove_inline_hook(key: InlineHookKey) -> Result<(), HookError> {
    let mut hooks = INLINE_HOOKS.write();
    let entry = hooks.remove(key).ok_or(HookError::NotFound)?;

    // Entry will be dropped here, which calls safetyhook_destroy_inline

    tracing::info!(
        "Removed inline hook '{}' at {:x}",
        entry.name,
        entry.target
    );
    Ok(())
}

/// Check if an inline hook is enabled
pub fn is_inline_hook_enabled(key: InlineHookKey) -> bool {
    INLINE_HOOKS
        .read()
        .get(key)
        .map(|e| e.enabled)
        .unwrap_or(false)
}

/// Get the target address of an inline hook
pub fn get_inline_hook_target(key: InlineHookKey) -> Option<usize> {
    INLINE_HOOKS.read().get(key).map(|e| e.target)
}

/// Get the original function trampoline for an inline hook
pub fn get_inline_hook_original(key: InlineHookKey) -> Option<*const ()> {
    INLINE_HOOKS.read().get(key).map(|e| e.trampoline)
}

/// Typed wrapper for inline hooks with proper original calling
pub struct TypedInlineHook<F> {
    name: &'static str,
    detour: F,
    key: RwLock<Option<InlineHookKey>>,
    original: RwLock<Option<*const ()>>,
}

// SAFETY: The hook is protected by RwLock
unsafe impl<F: Send> Send for TypedInlineHook<F> {}
unsafe impl<F: Sync> Sync for TypedInlineHook<F> {}

impl<F: Copy> TypedInlineHook<F> {
    pub const fn new(name: &'static str, detour: F) -> Self {
        Self {
            name,
            detour,
            key: RwLock::new(None),
            original: RwLock::new(None),
        }
    }

    /// Install the hook at the target address
    ///
    /// # Safety
    /// Target must be a valid function with matching signature
    pub unsafe fn install(&self, target: *const ()) -> Result<(), HookError> {
        let detour_ptr = &self.detour as *const F as *const ();
        let (key, original) = create_inline_hook(self.name, target, detour_ptr)?;
        *self.key.write() = Some(key);
        *self.original.write() = Some(original);
        Ok(())
    }

    /// Get pointer to call the original function
    ///
    /// Returns None if hook is not installed
    pub fn original_ptr(&self) -> Option<*const ()> {
        *self.original.read()
    }

    /// Check if the hook is installed
    pub fn is_installed(&self) -> bool {
        self.key.read().is_some()
    }

    /// Check if the hook is enabled
    pub fn is_enabled(&self) -> bool {
        self.key.read().map(is_inline_hook_enabled).unwrap_or(false)
    }

    /// Enable the hook
    pub fn enable(&self) -> Result<(), HookError> {
        if let Some(key) = *self.key.read() {
            enable_inline_hook(key)
        } else {
            Err(HookError::NotFound)
        }
    }

    /// Disable the hook
    pub fn disable(&self) -> Result<(), HookError> {
        if let Some(key) = *self.key.read() {
            disable_inline_hook(key)
        } else {
            Err(HookError::NotFound)
        }
    }

    /// Remove the hook
    pub fn remove(&self) -> Result<(), HookError> {
        if let Some(key) = self.key.write().take() {
            remove_inline_hook(key)?;
        }
        *self.original.write() = None;
        Ok(())
    }
}

/// Macro for creating typed inline hooks with proper signature handling
///
/// # Example
/// ```ignore
/// // Define the hook
/// typed_inline_hook! {
///     /// Hook for GameFrame
///     pub static GAME_FRAME_HOOK: fn(bool, bool, bool) = game_frame_detour;
/// }
///
/// fn game_frame_detour(simulating: bool, first: bool, last: bool) {
///     // Pre-hook logic
///     if let Some(original) = GAME_FRAME_HOOK.original_ptr() {
///         let original: fn(bool, bool, bool) = unsafe { std::mem::transmute(original) };
///         original(simulating, first, last);
///     }
///     // Post-hook logic
/// }
/// ```
#[macro_export]
macro_rules! typed_inline_hook {
    (
        $(#[$meta:meta])*
        pub static $name:ident: fn($($arg:ty),*) $(-> $ret:ty)? = $detour:ident;
    ) => {
        $(#[$meta])*
        pub static $name: std::sync::LazyLock<$crate::hooks::inline::TypedInlineHook<fn($($arg),*) $(-> $ret)?>> =
            std::sync::LazyLock::new(|| {
                $crate::hooks::inline::TypedInlineHook::new(stringify!($name), $detour)
            });
    };
}
