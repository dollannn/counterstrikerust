//! Mid-function hooks with full register context
//!
//! Allows hooking at arbitrary addresses within functions with access to all CPU registers.
//! Uses SafetyHook for proper hook chaining and multi-framework compatibility.

use parking_lot::RwLock;
use slotmap::{new_key_type, SlotMap};
use std::ffi::c_void;
use std::sync::LazyLock;

use super::context::MidHookContext;
use super::ffi;
use super::inline::HookError;

new_key_type! {
    /// Handle for a mid-function hook
    pub struct MidHookKey;
}

/// Callback type for mid-function hooks
pub type MidHookCallback = Box<dyn Fn(&mut MidHookContext) + Send + Sync>;

/// Storage for a mid-function hook
struct MidHookEntry {
    /// SafetyHook handle
    handle: *mut ffi::MidHookHandle,

    /// Target address being hooked (for logging)
    target: usize,

    /// Raw pointer to the leaked callback box.
    /// We leak it to ensure a stable pointer for C++ to call back into.
    callback_ptr: *mut MidHookCallback,

    /// Whether currently active
    enabled: bool,

    /// Debug name
    name: String,
}

unsafe impl Send for MidHookEntry {}
unsafe impl Sync for MidHookEntry {}

impl Drop for MidHookEntry {
    fn drop(&mut self) {
        // Destroy the SafetyHook handle first
        if !self.handle.is_null() {
            unsafe {
                ffi::safetyhook_destroy_mid(self.handle);
            }
        }
        // Then free the leaked callback
        if !self.callback_ptr.is_null() {
            unsafe {
                drop(Box::from_raw(self.callback_ptr));
            }
        }
    }
}

/// Global mid-hook registry
static MID_HOOKS: LazyLock<RwLock<SlotMap<MidHookKey, MidHookEntry>>> =
    LazyLock::new(|| RwLock::new(SlotMap::with_key()));

/// FFI callback that SafetyHook bridge calls.
/// Receives RustMidHookContext which is layout-compatible with MidHookContext.
extern "C" fn mid_hook_ffi_callback(ctx: *mut ffi::RustMidHookContext, user_data: *mut c_void) {
    if ctx.is_null() || user_data.is_null() {
        return;
    }

    unsafe {
        // user_data is a *mut MidHookCallback (pointer to Box<dyn Fn>)
        let callback_ptr = user_data as *mut MidHookCallback;
        let callback = &**callback_ptr;

        // RustMidHookContext is layout-compatible with MidHookContext
        // (both have same field order and sizes)
        let context = &mut *(ctx as *mut MidHookContext);

        callback(context);
    }
}

/// Create a mid-function hook at an arbitrary address
///
/// # Safety
/// - `target` must be a valid code address
/// - The hook site must have at least 5 bytes of instructions that can be relocated
///
/// # Arguments
/// * `name` - Debug name for the hook
/// * `target` - Address to hook
/// * `callback` - Function called with register context
pub unsafe fn create_mid_hook<F>(
    name: &str,
    target: *const u8,
    callback: F,
) -> Result<MidHookKey, HookError>
where
    F: Fn(&mut MidHookContext) + Send + Sync + 'static,
{
    tracing::debug!("Creating mid-hook '{}' at {:x}", name, target as usize);

    // Box the callback and leak it to get a stable pointer for C++ to call back into.
    // The leaked box will be freed when the hook entry is dropped.
    let callback_box: MidHookCallback = Box::new(callback);
    let callback_ptr = Box::into_raw(Box::new(callback_box));

    let mut handle: *mut ffi::MidHookHandle = std::ptr::null_mut();

    let result = ffi::safetyhook_create_mid(
        target as *const c_void,
        mid_hook_ffi_callback,
        callback_ptr as *mut c_void,
        &mut handle,
    );

    if !result.is_success() {
        // Free the leaked callback since we won't store it
        drop(Box::from_raw(callback_ptr));
        tracing::error!(
            "Failed to create mid-hook '{}': {}",
            name,
            result.to_error_string()
        );
        return Err(HookError::DetourCreation(
            result.to_error_string().to_string(),
        ));
    }

    let entry = MidHookEntry {
        handle,
        target: target as usize,
        callback_ptr,
        enabled: true,
        name: name.to_string(),
    };

    let key = MID_HOOKS.write().insert(entry);

    tracing::info!("Created mid-hook '{}' at {:x}", name, target as usize);

    Ok(key)
}

/// Enable a previously disabled mid-function hook
pub fn enable_mid_hook(key: MidHookKey) -> Result<(), HookError> {
    let mut hooks = MID_HOOKS.write();
    let entry = hooks.get_mut(key).ok_or(HookError::NotFound)?;

    if entry.enabled {
        return Ok(()); // Already enabled
    }

    let result = unsafe { ffi::safetyhook_enable_mid(entry.handle) };

    if !result.is_success() {
        return Err(HookError::EnableFailed(result.to_error_string().to_string()));
    }

    entry.enabled = true;
    tracing::info!("Enabled mid-hook '{}' at {:x}", entry.name, entry.target);

    Ok(())
}

/// Disable a mid-function hook
pub fn disable_mid_hook(key: MidHookKey) -> Result<(), HookError> {
    let mut hooks = MID_HOOKS.write();
    let entry = hooks.get_mut(key).ok_or(HookError::NotFound)?;

    if !entry.enabled {
        return Ok(()); // Already disabled
    }

    let result = unsafe { ffi::safetyhook_disable_mid(entry.handle) };

    if !result.is_success() {
        return Err(HookError::DisableFailed(
            result.to_error_string().to_string(),
        ));
    }

    entry.enabled = false;
    tracing::info!("Disabled mid-hook '{}' at {:x}", entry.name, entry.target);

    Ok(())
}

/// Check if a mid-hook is enabled
pub fn is_mid_hook_enabled(key: MidHookKey) -> bool {
    MID_HOOKS
        .read()
        .get(key)
        .map(|e| e.enabled)
        .unwrap_or(false)
}

/// Remove a mid-function hook completely
pub fn remove_mid_hook(key: MidHookKey) -> Result<(), HookError> {
    let mut hooks = MID_HOOKS.write();
    let entry = hooks.remove(key).ok_or(HookError::NotFound)?;

    // Entry will be dropped here, which calls safetyhook_destroy_mid

    tracing::info!("Removed mid-hook '{}' at {:x}", entry.name, entry.target);
    Ok(())
}
