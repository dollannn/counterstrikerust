//! Unified hook manager API
//!
//! Provides a single entry point for all hook types.

use super::context::MidHookContext;
use super::inline::{self, HookError, InlineHookKey};
use super::midhook::{self, MidHookKey};
use super::vtable::{self, VTableHookKey};

/// Unified hook key (can be any hook type)
#[derive(Debug, Clone, Copy)]
pub enum HookKey {
    Inline(InlineHookKey),
    VTable(VTableHookKey),
    Mid(MidHookKey),
}

impl From<InlineHookKey> for HookKey {
    fn from(key: InlineHookKey) -> Self {
        HookKey::Inline(key)
    }
}

impl From<VTableHookKey> for HookKey {
    fn from(key: VTableHookKey) -> Self {
        HookKey::VTable(key)
    }
}

impl From<MidHookKey> for HookKey {
    fn from(key: MidHookKey) -> Self {
        HookKey::Mid(key)
    }
}

/// Central hook manager
pub struct HookManager;

impl HookManager {
    /// Create an inline (detour) hook for a function
    ///
    /// # Safety
    /// Target must be a valid function pointer with matching signature
    ///
    /// # Example
    /// ```ignore
    /// extern "C" fn my_game_frame(simulating: bool, first: bool, last: bool) {
    ///     // Call original via trampoline
    ///     let original: extern "C" fn(bool, bool, bool) = unsafe { std::mem::transmute(ORIGINAL.get().unwrap()) };
    ///     original(simulating, first, last);
    /// }
    ///
    /// let (key, original) = unsafe {
    ///     HookManager::create_inline(
    ///         "GameFrame",
    ///         game_frame_addr,
    ///         my_game_frame as *const (),
    ///     )?
    /// };
    /// ```
    pub unsafe fn create_inline(
        name: &str,
        target: *const (),
        detour: *const (),
    ) -> Result<(InlineHookKey, *const ()), HookError> {
        inline::create_inline_hook(name, target, detour)
    }

    /// Hook a virtual table entry on a C++ object
    ///
    /// # Safety
    /// Object must have a valid vtable, index must be valid
    ///
    /// # Example
    /// ```ignore
    /// let (key, original) = unsafe {
    ///     HookManager::hook_vtable(
    ///         "TakeDamage",
    ///         player as *mut (),
    ///         TAKE_DAMAGE_INDEX,
    ///         my_take_damage as *const (),
    ///     )?
    /// };
    /// ```
    pub unsafe fn hook_vtable(
        name: &str,
        object: *mut (),
        vtable_index: usize,
        new_fn: *const (),
    ) -> Result<(VTableHookKey, *const ()), HookError> {
        vtable::create_vtable_hook(name, object, vtable_index, new_fn)
    }

    /// Hook a virtual table entry directly by vtable pointer
    ///
    /// # Safety
    /// VTable pointer must be valid, index must be valid
    pub unsafe fn hook_vtable_direct(
        name: &str,
        vtable: *mut *const (),
        vtable_index: usize,
        new_fn: *const (),
    ) -> Result<(VTableHookKey, *const ()), HookError> {
        vtable::create_vtable_hook_direct(name, vtable, vtable_index, new_fn)
    }

    /// Create a mid-function hook with full register context
    ///
    /// # Safety
    /// Target must be a valid code address with at least 5 bytes available
    ///
    /// # Example
    /// ```ignore
    /// let key = unsafe {
    ///     HookManager::create_mid(
    ///         "DamageCalc",
    ///         damage_calc_addr,
    ///         |ctx| {
    ///             // Double the damage in RDI
    ///             ctx.rdi *= 2;
    ///         },
    ///     )?
    /// };
    /// ```
    pub unsafe fn create_mid<F>(
        name: &str,
        target: *const u8,
        callback: F,
    ) -> Result<MidHookKey, HookError>
    where
        F: Fn(&mut MidHookContext) + Send + Sync + 'static,
    {
        midhook::create_mid_hook(name, target, callback)
    }

    /// Enable a hook by key
    pub fn enable(key: HookKey) -> Result<(), HookError> {
        match key {
            HookKey::Inline(k) => inline::enable_inline_hook(k),
            HookKey::VTable(k) => vtable::enable_vtable_hook(k),
            HookKey::Mid(_k) => {
                // Mid hooks don't support enable/disable toggle yet
                tracing::warn!("Mid hooks cannot be re-enabled after disable");
                Ok(())
            }
        }
    }

    /// Disable a hook by key
    pub fn disable(key: HookKey) -> Result<(), HookError> {
        match key {
            HookKey::Inline(k) => inline::disable_inline_hook(k),
            HookKey::VTable(k) => vtable::disable_vtable_hook(k),
            HookKey::Mid(k) => midhook::disable_mid_hook(k),
        }
    }

    /// Remove a hook completely
    pub fn remove(key: HookKey) -> Result<(), HookError> {
        match key {
            HookKey::Inline(k) => inline::remove_inline_hook(k),
            HookKey::VTable(k) => vtable::remove_vtable_hook(k),
            HookKey::Mid(k) => midhook::remove_mid_hook(k),
        }
    }

    /// Check if a hook is enabled
    pub fn is_enabled(key: HookKey) -> bool {
        match key {
            HookKey::Inline(k) => inline::is_inline_hook_enabled(k),
            HookKey::VTable(k) => vtable::is_vtable_hook_enabled(k),
            HookKey::Mid(k) => midhook::is_mid_hook_enabled(k),
        }
    }
}

/// Global convenience functions

/// Create an inline hook
///
/// # Safety
/// Target must be a valid function pointer with matching signature
pub unsafe fn hook(
    name: &str,
    target: *const (),
    detour: *const (),
) -> Result<(InlineHookKey, *const ()), HookError> {
    HookManager::create_inline(name, target, detour)
}

/// Create a vtable hook
///
/// # Safety
/// Object must have a valid vtable, index must be valid
pub unsafe fn hook_vtable(
    name: &str,
    object: *mut (),
    index: usize,
    new_fn: *const (),
) -> Result<(VTableHookKey, *const ()), HookError> {
    HookManager::hook_vtable(name, object, index, new_fn)
}

/// Create a vtable hook directly by vtable pointer
///
/// # Safety
/// VTable pointer must be valid, index must be valid
pub unsafe fn hook_vtable_direct(
    name: &str,
    vtable: *mut *const (),
    index: usize,
    new_fn: *const (),
) -> Result<(VTableHookKey, *const ()), HookError> {
    HookManager::hook_vtable_direct(name, vtable, index, new_fn)
}

/// Create a mid-function hook
///
/// # Safety
/// Target must be a valid code address
pub unsafe fn hook_mid<F>(
    name: &str,
    target: *const u8,
    callback: F,
) -> Result<MidHookKey, HookError>
where
    F: Fn(&mut MidHookContext) + Send + Sync + 'static,
{
    HookManager::create_mid(name, target, callback)
}
