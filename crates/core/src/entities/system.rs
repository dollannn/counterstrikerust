//! Entity System wrapper for CGameEntitySystem
//!
//! Provides access to entity lookup and iteration via the Source 2 entity system.
//!
//! # Architecture
//!
//! CGameEntitySystem uses a chunked array structure for entity storage:
//!
//! ```text
//! CGameEntitySystem
//! ├── m_EntityList (array of 64 chunk pointers)
//! │   ├── Chunk 0 (512 CEntityIdentity entries)
//! │   ├── Chunk 1 (512 CEntityIdentity entries)
//! │   └── ...
//! └── m_FirstActiveEntity (linked list head)
//!
//! CEntityIdentity (0x70 bytes each)
//! ├── +0x00: m_pInstance (CEntityInstance*)
//! ├── +0x08: m_pClass (CEntityClass*)
//! ├── +0x10: m_EHandle (CEntityHandle)
//! ├── +0x14: m_nameStringableIndex
//! ├── +0x18: m_name (CUtlSymbolLarge)
//! ├── +0x20: m_designerName (CUtlSymbolLarge)
//! └── +0x58: m_pNext/m_pPrev (linked list)
//! ```

use std::ffi::c_void;

use cs2rust_engine::engine;

/// Maximum number of entities (2^15 = 32768)
pub const MAX_ENTITIES: usize = 32768;

/// Number of entities per chunk (512)
pub const MAX_ENTITIES_PER_CHUNK: usize = 512;

/// Number of chunks (64)
pub const MAX_CHUNKS: usize = MAX_ENTITIES / MAX_ENTITIES_PER_CHUNK;

/// Size of CEntityIdentity structure (0x70 bytes)
pub const SIZE_OF_ENTITY_IDENTITY: usize = 0x78;

/// Offset to CEntityHandle within CEntityIdentity
pub const HANDLE_OFFSET: usize = 0x10;

/// Offset to m_pInstance (entity pointer) within CEntityIdentity
pub const INSTANCE_OFFSET: usize = 0x00;

/// Offset to m_pNext within CEntityIdentity (for linked list iteration)
pub const NEXT_OFFSET: usize = 0x58;

/// Offset to m_designerName within CEntityIdentity
pub const DESIGNER_NAME_OFFSET: usize = 0x20;

/// Offset to entity list chunks in CGameEntitySystem
/// This is the offset to m_EntityList which is an array of chunk pointers
pub const ENTITY_LIST_OFFSET: usize = 0x10;

/// Get entity pointer by index
///
/// Returns the raw entity pointer if an entity exists at the given index.
///
/// # Arguments
///
/// * `index` - Entity index (0 to MAX_ENTITIES-1)
///
/// # Returns
///
/// `Some(ptr)` if an entity exists at the index, `None` otherwise.
///
/// # Example
///
/// ```ignore
/// if let Some(ptr) = get_entity_by_index(5) {
///     let entity = unsafe { BaseEntity::from_ptr(ptr) };
/// }
/// ```
pub fn get_entity_by_index(index: u32) -> Option<*mut c_void> {
    if index >= MAX_ENTITIES as u32 - 1 {
        return None;
    }

    let entity_system_ptr = engine().entity_system_ptr()? as *mut c_void;

    unsafe { get_entity_by_index_unchecked(entity_system_ptr, index) }
}

/// Get entity by index without checking if entity system is available
///
/// # Safety
///
/// Caller must ensure entity_system_ptr is valid.
unsafe fn get_entity_by_index_unchecked(
    entity_system_ptr: *mut c_void,
    index: u32,
) -> Option<*mut c_void> {
    let chunk_index = index as usize / MAX_ENTITIES_PER_CHUNK;
    let entry_index = index as usize % MAX_ENTITIES_PER_CHUNK;

    // Get pointer to chunk array (at offset ENTITY_LIST_OFFSET from entity system)
    let chunks_ptr = entity_system_ptr.byte_add(ENTITY_LIST_OFFSET) as *const *const c_void;

    // Get the chunk pointer
    let chunk_ptr = *chunks_ptr.add(chunk_index);
    if chunk_ptr.is_null() {
        return None;
    }

    // Calculate identity pointer within chunk
    let identity_ptr = chunk_ptr.byte_add(SIZE_OF_ENTITY_IDENTITY * entry_index);

    // Read the handle and verify index matches
    let handle = *(identity_ptr.byte_add(HANDLE_OFFSET) as *const u32);
    let handle_index = handle & 0x7FFF; // Lower 15 bits

    if handle_index != index {
        return None;
    }

    // Read entity instance pointer (first field of CEntityIdentity)
    let entity_ptr = *(identity_ptr as *const *mut c_void);
    if entity_ptr.is_null() {
        return None;
    }

    Some(entity_ptr)
}

/// Get entity pointer by handle
///
/// Resolves a handle (index + serial number) to an entity pointer.
/// Returns `None` if the handle is invalid or the entity no longer exists.
///
/// # Arguments
///
/// * `raw_handle` - The raw 32-bit handle value
///
/// # Returns
///
/// `Some(ptr)` if the handle resolves to a valid entity, `None` otherwise.
pub fn get_entity_by_handle(raw_handle: u32) -> Option<*mut c_void> {
    // Check for invalid handle sentinel
    if raw_handle == super::handle::INVALID_EHANDLE_INDEX {
        return None;
    }

    let index = raw_handle & 0x7FFF; // Lower 15 bits

    // Check index bounds
    if index >= MAX_ENTITIES as u32 - 1 {
        return None;
    }

    let entity_system_ptr = engine().entity_system_ptr()? as *mut c_void;

    unsafe { get_entity_by_handle_unchecked(entity_system_ptr, raw_handle) }
}

/// Get entity by handle without checking if entity system is available
///
/// # Safety
///
/// Caller must ensure entity_system_ptr is valid.
unsafe fn get_entity_by_handle_unchecked(
    entity_system_ptr: *mut c_void,
    raw_handle: u32,
) -> Option<*mut c_void> {
    let index = raw_handle & 0x7FFF;
    let chunk_index = index as usize / MAX_ENTITIES_PER_CHUNK;
    let entry_index = index as usize % MAX_ENTITIES_PER_CHUNK;

    // Get pointer to chunk array
    let chunks_ptr = entity_system_ptr.byte_add(ENTITY_LIST_OFFSET) as *const *const c_void;

    // Get the chunk pointer
    let chunk_ptr = *chunks_ptr.add(chunk_index);
    if chunk_ptr.is_null() {
        return None;
    }

    // Calculate identity pointer within chunk
    let identity_ptr = chunk_ptr.byte_add(SIZE_OF_ENTITY_IDENTITY * entry_index);

    // Read the stored handle and compare with requested handle
    // This validates both index AND serial number
    let stored_handle = *(identity_ptr.byte_add(HANDLE_OFFSET) as *const u32);
    if stored_handle != raw_handle {
        return None;
    }

    // Read entity instance pointer
    let entity_ptr = *(identity_ptr as *const *mut c_void);
    if entity_ptr.is_null() {
        return None;
    }

    Some(entity_ptr)
}

/// Get the raw handle value for an entity pointer
///
/// Reads the entity's handle from its CEntityIdentity.
///
/// # Safety
///
/// The entity_ptr must be a valid CEntityInstance pointer.
pub unsafe fn get_handle_from_entity(entity_ptr: *mut c_void) -> u32 {
    if entity_ptr.is_null() {
        return super::handle::INVALID_EHANDLE_INDEX;
    }

    // CEntityInstance has m_pEntity at offset 0x10 pointing to CEntityIdentity
    const ENTITY_IDENTITY_PTR_OFFSET: usize = 0x10;

    let identity_ptr = *(entity_ptr.byte_add(ENTITY_IDENTITY_PTR_OFFSET) as *const *const c_void);
    if identity_ptr.is_null() {
        return super::handle::INVALID_EHANDLE_INDEX;
    }

    // Read handle from identity
    *(identity_ptr.byte_add(HANDLE_OFFSET) as *const u32)
}

/// Iterator over all active entities
///
/// Uses the entity system's linked list of active entities for efficient iteration.
pub struct EntityIterator {
    current: *const c_void,
}

impl EntityIterator {
    /// Create a new iterator starting from the first active entity
    pub fn new() -> Option<Self> {
        let entity_system_ptr = engine().entity_system_ptr()?;

        // First active entity is at a specific offset in CGameEntitySystem
        // In CounterStrikeSharp this is accessed as FirstActiveEntity
        // It's typically at offset 0x210 (may vary by game version)
        const FIRST_ACTIVE_OFFSET: usize = 0x210;

        unsafe {
            let first_active =
                *(entity_system_ptr.byte_add(FIRST_ACTIVE_OFFSET) as *const *const c_void);
            Some(Self {
                current: first_active,
            })
        }
    }
}

impl Iterator for EntityIterator {
    type Item = *mut c_void;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current.is_null() {
            return None;
        }

        unsafe {
            // Read entity instance pointer from CEntityIdentity
            let entity_ptr = *(self.current as *const *mut c_void);

            // Advance to next entity in linked list
            self.current = *(self.current.byte_add(NEXT_OFFSET) as *const *const c_void);

            if entity_ptr.is_null() {
                // Skip null entries and continue
                self.next()
            } else {
                Some(entity_ptr)
            }
        }
    }
}

impl Default for EntityIterator {
    fn default() -> Self {
        Self::new().unwrap_or(Self {
            current: std::ptr::null(),
        })
    }
}

/// Get an iterator over all active entities
///
/// # Example
///
/// ```ignore
/// for entity_ptr in get_all_entities() {
///     let entity = unsafe { EntityRef::from_entity_instance(entity_ptr) };
///     if let Some(EntityRef::PlayerPawn(pawn)) = entity {
///         println!("Found player pawn!");
///     }
/// }
/// ```
pub fn get_all_entities() -> EntityIterator {
    EntityIterator::default()
}

/// Check if the entity system is available
pub fn is_available() -> bool {
    engine().entity_system_ptr().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(MAX_ENTITIES, 32768);
        assert_eq!(MAX_ENTITIES_PER_CHUNK, 512);
        assert_eq!(MAX_CHUNKS, 64);
        assert_eq!(MAX_ENTITIES / MAX_ENTITIES_PER_CHUNK, MAX_CHUNKS);
    }

    #[test]
    fn test_chunk_calculation() {
        // Entity 0 should be in chunk 0, entry 0
        assert_eq!(0 / MAX_ENTITIES_PER_CHUNK, 0);
        assert_eq!(0 % MAX_ENTITIES_PER_CHUNK, 0);

        // Entity 511 should be in chunk 0, entry 511
        assert_eq!(511 / MAX_ENTITIES_PER_CHUNK, 0);
        assert_eq!(511 % MAX_ENTITIES_PER_CHUNK, 511);

        // Entity 512 should be in chunk 1, entry 0
        assert_eq!(512 / MAX_ENTITIES_PER_CHUNK, 1);
        assert_eq!(512 % MAX_ENTITIES_PER_CHUNK, 0);

        // Entity 32767 should be in chunk 63, entry 511
        assert_eq!(32767 / MAX_ENTITIES_PER_CHUNK, 63);
        assert_eq!(32767 % MAX_ENTITIES_PER_CHUNK, 511);
    }
}
