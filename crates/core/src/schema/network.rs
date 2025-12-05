//! Network State Change Implementation
//!
//! This module handles notifying the Source 2 engine when networked entity
//! properties are modified, ensuring changes are replicated to clients.
//!
//! # Implementation
//!
//! Source 2 uses two patterns for network state changes:
//!
//! 1. **Direct entities**: Call `SetStateChanged` virtual function on entity
//! 2. **Chained entities**: Follow `__m_pChainEntity` to get actual entity
//!
//! The implementation uses the vtable-based approach (`SetStateChanged`)
//! which is more stable across game updates than signature scanning.

use std::ffi::c_void;
use std::sync::LazyLock;

use dashmap::DashMap;
use tracing::{debug, trace};

use super::system::get_offset;

/// Platform-specific vtable index for CEntityInstance::SetStateChanged
///
/// This virtual function notifies the network system that a field has changed.
/// Index verified from CounterStrikeSharp gamedata.json.
#[cfg(target_os = "linux")]
const SET_STATE_CHANGED_VFUNC_INDEX: usize = 26;
#[cfg(target_os = "windows")]
const SET_STATE_CHANGED_VFUNC_INDEX: usize = 25;

/// Network state change information passed to the engine
///
/// This structure mirrors Source 2's internal CNetworkStateChangedInfo.
/// The engine uses this to track which fields have changed and need replication.
///
/// Layout based on CounterStrikeSharp schema.h and SwiftlyS2 implementations.
#[repr(C)]
struct NetworkStateChangedInfo {
    /// Number of offsets in the offset data (always 1 for single field changes)
    size: i32,
    /// Offset data - uses inline storage for simple cases
    /// In the full implementation this would be CUtlVector<uint32_t>
    /// but we use inline array for single-field changes
    offset_data_size: i32,
    offset_data_ptr: *mut u32,
    offset_data_capacity: i32,
    offset_data_grow_size: i32,
    /// Inline storage for offset (used when size <= 1)
    offset_inline: u32,
    /// Field name (for debugging, can be null)
    field_name: *const i8,
    /// File name (for debugging, can be null)
    file_name: *const i8,
    /// Unknown field, always -1
    unk_30: u32,
    /// Array index for array fields (-1 for non-array)
    array_index: u32,
    /// Path index for chained entities (-1 for direct)
    path_index: u32,
    /// Unknown field
    unk_3c: u16,
    /// Padding to match expected size
    _pad: u16,
}

impl NetworkStateChangedInfo {
    /// Create a new NetworkStateChangedInfo for a single field change
    ///
    /// # Arguments
    /// * `offset` - The field offset within the entity
    /// * `array_index` - Array index if field is an array element, -1 otherwise
    /// * `path_index` - Path index for chained entities, -1 for direct entities
    fn new(offset: u32, array_index: i32, path_index: i32) -> Self {
        let mut info = Self {
            size: 0,
            offset_data_size: 1,
            offset_data_ptr: std::ptr::null_mut(),
            offset_data_capacity: 0,
            offset_data_grow_size: 0,
            offset_inline: offset,
            field_name: std::ptr::null(),
            file_name: std::ptr::null(),
            unk_30: u32::MAX, // -1 as unsigned
            array_index: array_index as u32,
            path_index: path_index as u32,
            unk_3c: 0,
            _pad: 0,
        };
        // Point to inline storage
        info.offset_data_ptr = &mut info.offset_inline as *mut u32;
        info
    }
}

/// Cache for __m_pChainEntity offsets per class
///
/// Key: FNV-1a hash of class name
/// Value: Chain offset (0 if class has no chain entity)
static CHAIN_OFFSET_CACHE: LazyLock<DashMap<u32, i16>> = LazyLock::new(DashMap::new);

/// The field name used to find chain entities in schema classes
const CHAIN_ENTITY_FIELD: &str = "__m_pChainEntity";

/// CNetworkVarChainer structure
///
/// This is embedded in entity classes that use chaining for network state.
/// Layout from SwiftlyS2 schema.cpp.
#[repr(C)]
struct CNetworkVarChainer {
    /// Pointer to the actual CEntityInstance
    entity: *mut c_void,
    /// Padding
    _pad: [u8; 24],
    /// Path index for this chainer
    path_index: i32,
}

/// Notify the engine of a networked property change
///
/// This must be called after modifying networked fields for the change
/// to be replicated to clients. The `#[schema(networked)]` attribute
/// will automatically call this in generated setters.
///
/// # Safety
/// - `entity_ptr` must be a valid pointer to a CEntityInstance or derived class
/// - `offset` must be a valid field offset within the object
///
/// # Implementation
///
/// This function calls the `SetStateChanged` virtual function on the entity.
/// For classes that use chain entities, it follows the chain to find the
/// actual entity instance.
#[inline]
pub unsafe fn network_state_changed(entity_ptr: *mut c_void, offset: i32) {
    if entity_ptr.is_null() {
        return;
    }

    trace!(
        "network_state_changed: entity={:p}, offset={}",
        entity_ptr,
        offset
    );

    // Create the state change info
    let info = NetworkStateChangedInfo::new(offset as u32, u32::MAX as i32, u32::MAX as i32);

    // Call SetStateChanged virtual function
    call_set_state_changed(entity_ptr, &info);
}

/// Extended version of network_state_changed for entities with chain support
///
/// # Safety
/// Same requirements as `network_state_changed`
///
/// # Arguments
/// * `entity_ptr` - Pointer to the entity
/// * `class_name` - The schema class name (used for chain offset lookup)
/// * `offset` - The field offset
#[inline]
pub unsafe fn network_state_changed_ex(entity_ptr: *mut c_void, class_name: &str, offset: i32) {
    if entity_ptr.is_null() {
        return;
    }

    // Check if this class uses chain entities
    let chain_offset = get_chain_offset(class_name);

    if chain_offset != 0 {
        // Follow the chain to get the actual entity
        let chainer_ptr = entity_ptr.byte_add(chain_offset as usize) as *const CNetworkVarChainer;
        let chainer = &*chainer_ptr;

        if !chainer.entity.is_null() {
            trace!(
                "network_state_changed_ex: using chain entity {:p} with path_index={}",
                chainer.entity,
                chainer.path_index
            );

            let info =
                NetworkStateChangedInfo::new(offset as u32, u32::MAX as i32, chainer.path_index);
            call_set_state_changed(chainer.entity, &info);
            return;
        }
    }

    // No chain or chain entity is null, call directly
    let info = NetworkStateChangedInfo::new(offset as u32, u32::MAX as i32, u32::MAX as i32);
    call_set_state_changed(entity_ptr, &info);
}

/// Call the SetStateChanged virtual function on an entity
///
/// # Safety
/// `entity_ptr` must be a valid CEntityInstance pointer
#[inline]
unsafe fn call_set_state_changed(entity_ptr: *mut c_void, info: &NetworkStateChangedInfo) {
    // Get vtable pointer (first pointer in object)
    let vtable = *(entity_ptr as *const *const usize);

    // Get function pointer from vtable
    let func_ptr = *vtable.add(SET_STATE_CHANGED_VFUNC_INDEX);

    // Cast to function signature:
    // void CEntityInstance::SetStateChanged(CNetworkStateChangedInfo* info)
    let set_state_changed: extern "C" fn(*mut c_void, *const NetworkStateChangedInfo) =
        std::mem::transmute(func_ptr);

    set_state_changed(entity_ptr, info);

    trace!(
        "SetStateChanged called: entity={:p}, offset={}",
        entity_ptr,
        info.offset_inline
    );
}

/// Get the chain offset for a class (cached)
///
/// Returns 0 if the class has no `__m_pChainEntity` field.
fn get_chain_offset(class_name: &str) -> i16 {
    let class_hash = super::hash::fnv1a_32(class_name.as_bytes());

    // Check cache first
    if let Some(offset) = CHAIN_OFFSET_CACHE.get(&class_hash) {
        return *offset;
    }

    // Query schema system for __m_pChainEntity
    let offset = match get_offset(class_name, CHAIN_ENTITY_FIELD) {
        Ok(schema_offset) => {
            debug!(
                "Found chain offset for {}: {}",
                class_name, schema_offset.offset
            );
            schema_offset.offset as i16
        }
        Err(_) => {
            // Class has no chain entity field
            trace!("No chain offset for {}", class_name);
            0
        }
    };

    CHAIN_OFFSET_CACHE.insert(class_hash, offset);
    offset
}

/// Clear the chain offset cache
///
/// Should be called when reloading schemas or for hot-reload scenarios.
pub fn clear_chain_cache() {
    CHAIN_OFFSET_CACHE.clear();
    debug!("Chain offset cache cleared");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_state_changed_info_size() {
        // Verify struct layout is reasonable
        let info = NetworkStateChangedInfo::new(100, -1, -1);
        assert_eq!(info.offset_inline, 100);
        assert_eq!(info.offset_data_size, 1);
        assert_eq!(info.array_index, u32::MAX);
        assert_eq!(info.path_index, u32::MAX);
    }

    #[test]
    fn test_null_entity_safety() {
        // Should not crash with null pointer
        unsafe {
            network_state_changed(std::ptr::null_mut(), 0);
        }
    }
}
