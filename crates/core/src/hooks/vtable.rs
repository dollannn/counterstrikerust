//! VTable hooks via pointer replacement
//!
//! Simple and efficient hooking for virtual functions.

use parking_lot::RwLock;
use slotmap::{new_key_type, SlotMap};
use std::sync::LazyLock;

use super::inline::HookError;

new_key_type! {
    /// Handle for a vtable hook
    pub struct VTableHookKey;
}

/// Storage for a vtable hook
struct VTableHookEntry {
    /// Address of the vtable slot
    slot_address: *mut *const (),

    /// Original function pointer
    original: *const (),

    /// Our replacement function
    replacement: *const (),

    /// Whether currently active
    enabled: bool,

    /// Debug name
    name: String,
}

// SAFETY: We're careful about thread safety in the implementation
unsafe impl Send for VTableHookEntry {}
unsafe impl Sync for VTableHookEntry {}

/// Global vtable hook registry
static VTABLE_HOOKS: LazyLock<RwLock<SlotMap<VTableHookKey, VTableHookEntry>>> =
    LazyLock::new(|| RwLock::new(SlotMap::with_key()));

/// Hook a virtual table entry
///
/// # Safety
/// - `object` must be a valid pointer to a C++ object with a vtable
/// - `vtable_index` must be a valid index into the vtable
/// - `new_fn` must have a compatible signature with the original
///
/// # Arguments
/// * `name` - Debug name for the hook
/// * `object` - Pointer to the object (first member is vtable pointer)
/// * `vtable_index` - Index of the virtual function in the vtable
/// * `new_fn` - Your replacement function
///
/// # Returns
/// A key to manage the hook, and the original function pointer
pub unsafe fn create_vtable_hook(
    name: &str,
    object: *mut (),
    vtable_index: usize,
    new_fn: *const (),
) -> Result<(VTableHookKey, *const ()), HookError> {
    // Get vtable pointer (first member of object)
    let vtable_ptr = *(object as *const *mut *const ());
    let slot = vtable_ptr.add(vtable_index);

    // Read original function pointer
    let original = *slot;

    tracing::debug!(
        "Creating vtable hook '{}': object={:x}, vtable={:x}, slot[{}]={:x}, original={:x}",
        name,
        object as usize,
        vtable_ptr as usize,
        vtable_index,
        slot as usize,
        original as usize
    );

    // Make the vtable slot writable
    let slot_addr = slot as *const u8;
    region::protect(
        slot_addr,
        std::mem::size_of::<usize>(),
        region::Protection::READ_WRITE,
    )
    .map_err(|e| HookError::MemoryProtection(e.to_string()))?;

    // Write our function pointer
    *slot = new_fn;

    // Restore protection (optional, some games keep vtables writable)
    let _ = region::protect(
        slot_addr,
        std::mem::size_of::<usize>(),
        region::Protection::READ,
    );

    let entry = VTableHookEntry {
        slot_address: slot,
        original,
        replacement: new_fn,
        enabled: true,
        name: name.to_string(),
    };

    let key = VTABLE_HOOKS.write().insert(entry);

    tracing::info!("Created vtable hook '{}' at index {}", name, vtable_index);

    Ok((key, original))
}

/// Hook a virtual table entry by vtable address directly
///
/// # Safety
/// - `vtable` must be a valid vtable pointer
/// - `vtable_index` must be a valid index into the vtable
/// - `new_fn` must have a compatible signature with the original
///
/// # Arguments
/// * `name` - Debug name for the hook
/// * `vtable` - Pointer to the vtable
/// * `vtable_index` - Index of the virtual function in the vtable
/// * `new_fn` - Your replacement function
///
/// # Returns
/// A key to manage the hook, and the original function pointer
pub unsafe fn create_vtable_hook_direct(
    name: &str,
    vtable: *mut *const (),
    vtable_index: usize,
    new_fn: *const (),
) -> Result<(VTableHookKey, *const ()), HookError> {
    let slot = vtable.add(vtable_index);

    // Read original function pointer
    let original = *slot;

    tracing::debug!(
        "Creating direct vtable hook '{}': vtable={:x}, slot[{}]={:x}, original={:x}",
        name,
        vtable as usize,
        vtable_index,
        slot as usize,
        original as usize
    );

    // Make the vtable slot writable
    let slot_addr = slot as *const u8;
    region::protect(
        slot_addr,
        std::mem::size_of::<usize>(),
        region::Protection::READ_WRITE,
    )
    .map_err(|e| HookError::MemoryProtection(e.to_string()))?;

    // Write our function pointer
    *slot = new_fn;

    // Restore protection
    let _ = region::protect(
        slot_addr,
        std::mem::size_of::<usize>(),
        region::Protection::READ,
    );

    let entry = VTableHookEntry {
        slot_address: slot,
        original,
        replacement: new_fn,
        enabled: true,
        name: name.to_string(),
    };

    let key = VTABLE_HOOKS.write().insert(entry);

    tracing::info!(
        "Created direct vtable hook '{}' at index {}",
        name,
        vtable_index
    );

    Ok((key, original))
}

/// Disable a vtable hook (restore original pointer)
pub fn disable_vtable_hook(key: VTableHookKey) -> Result<(), HookError> {
    let mut hooks = VTABLE_HOOKS.write();
    let entry = hooks.get_mut(key).ok_or(HookError::NotFound)?;

    if !entry.enabled {
        return Ok(());
    }

    unsafe {
        let slot_addr = entry.slot_address as *const u8;

        region::protect(
            slot_addr,
            std::mem::size_of::<usize>(),
            region::Protection::READ_WRITE,
        )
        .map_err(|e| HookError::MemoryProtection(e.to_string()))?;

        *entry.slot_address = entry.original;

        let _ = region::protect(
            slot_addr,
            std::mem::size_of::<usize>(),
            region::Protection::READ,
        );
    }

    entry.enabled = false;
    tracing::info!("Disabled vtable hook '{}'", entry.name);

    Ok(())
}

/// Enable a vtable hook (restore replacement pointer)
pub fn enable_vtable_hook(key: VTableHookKey) -> Result<(), HookError> {
    let mut hooks = VTABLE_HOOKS.write();
    let entry = hooks.get_mut(key).ok_or(HookError::NotFound)?;

    if entry.enabled {
        return Ok(());
    }

    unsafe {
        let slot_addr = entry.slot_address as *const u8;

        region::protect(
            slot_addr,
            std::mem::size_of::<usize>(),
            region::Protection::READ_WRITE,
        )
        .map_err(|e| HookError::MemoryProtection(e.to_string()))?;

        *entry.slot_address = entry.replacement;

        let _ = region::protect(
            slot_addr,
            std::mem::size_of::<usize>(),
            region::Protection::READ,
        );
    }

    entry.enabled = true;
    tracing::info!("Enabled vtable hook '{}'", entry.name);

    Ok(())
}

/// Remove a vtable hook completely
pub fn remove_vtable_hook(key: VTableHookKey) -> Result<(), HookError> {
    // Disable first to restore original
    disable_vtable_hook(key)?;

    let mut hooks = VTABLE_HOOKS.write();
    let entry = hooks.remove(key).ok_or(HookError::NotFound)?;

    tracing::info!("Removed vtable hook '{}'", entry.name);
    Ok(())
}

/// Get the original function pointer for a vtable hook
pub fn get_vtable_original(key: VTableHookKey) -> Option<*const ()> {
    VTABLE_HOOKS.read().get(key).map(|e| e.original)
}

/// Check if a vtable hook is enabled
pub fn is_vtable_hook_enabled(key: VTableHookKey) -> bool {
    VTABLE_HOOKS
        .read()
        .get(key)
        .map(|e| e.enabled)
        .unwrap_or(false)
}

/// Helper macro for vtable hooks with typed original
#[macro_export]
macro_rules! vtable_hook {
    ($name:ident, $index:expr, fn($($arg:ty),*) $(-> $ret:ty)?) => {
        paste::paste! {
            static [<$name _KEY>]: std::sync::LazyLock<parking_lot::RwLock<Option<$crate::hooks::vtable::VTableHookKey>>> =
                std::sync::LazyLock::new(|| parking_lot::RwLock::new(None));

            static [<$name _ORIGINAL>]: std::sync::LazyLock<parking_lot::RwLock<Option<fn($($arg),*) $(-> $ret)?>>> =
                std::sync::LazyLock::new(|| parking_lot::RwLock::new(None));

            pub fn [<$name _install>](object: *mut (), detour: fn($($arg),*) $(-> $ret)?) -> Result<(), $crate::hooks::inline::HookError> {
                unsafe {
                    let (key, original) = $crate::hooks::vtable::create_vtable_hook(
                        stringify!($name),
                        object,
                        $index,
                        detour as *const (),
                    )?;
                    *[<$name _KEY>].write() = Some(key);
                    *[<$name _ORIGINAL>].write() = Some(std::mem::transmute(original));
                    Ok(())
                }
            }

            pub fn [<$name _original>]() -> Option<fn($($arg),*) $(-> $ret)?> {
                *[<$name _ORIGINAL>].read()
            }

            pub fn [<$name _disable>]() -> Result<(), $crate::hooks::inline::HookError> {
                if let Some(key) = *[<$name _KEY>].read() {
                    $crate::hooks::vtable::disable_vtable_hook(key)
                } else {
                    Err($crate::hooks::inline::HookError::NotFound)
                }
            }

            pub fn [<$name _enable>]() -> Result<(), $crate::hooks::inline::HookError> {
                if let Some(key) = *[<$name _KEY>].read() {
                    $crate::hooks::vtable::enable_vtable_hook(key)
                } else {
                    Err($crate::hooks::inline::HookError::NotFound)
                }
            }

            pub fn [<$name _remove>]() -> Result<(), $crate::hooks::inline::HookError> {
                if let Some(key) = [<$name _KEY>].write().take() {
                    $crate::hooks::vtable::remove_vtable_hook(key)?;
                }
                *[<$name _ORIGINAL>].write() = None;
                Ok(())
            }
        }
    };
}
