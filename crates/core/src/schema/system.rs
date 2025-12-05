//! CSchemaSystem wrapper for field offset queries
//!
//! This module provides safe access to Source 2's schema reflection system,
//! allowing runtime lookup of class field offsets. Results are cached for
//! performance.

use std::ffi::{c_char, c_void, CStr, CString};
use std::sync::LazyLock;

use dashmap::DashMap;
use tracing::{debug, trace, warn};

use super::hash::combined_hash;
use cs2rust_sdk::CSchemaSystem;

/// Error type for schema operations
#[derive(Debug, thiserror::Error)]
pub enum SchemaError {
    #[error("Schema system not initialized")]
    NotInitialized,

    #[error("Type scope not found for module: {0}")]
    TypeScopeNotFound(String),

    #[error("Class not found: {0}")]
    ClassNotFound(String),

    #[error("Field not found: {class}.{field}")]
    FieldNotFound { class: String, field: String },

    #[error("FFI error: {0}")]
    FfiError(String),
}

/// Cached schema offset entry
#[derive(Clone, Copy, Debug)]
pub struct SchemaOffset {
    /// Memory offset from class base pointer
    pub offset: i32,
    /// Whether this field triggers network replication when modified
    pub is_networked: bool,
}

/// Global offset cache: (class_hash << 32 | field_hash) -> SchemaOffset
static OFFSET_CACHE: LazyLock<DashMap<u64, SchemaOffset>> = LazyLock::new(DashMap::new);

/// Virtual function indices for CSchemaSystem
///
/// These are platform-specific vtable offsets. ISchemaSystem inherits from
/// IAppSystem, so the first ~10 indices are IAppSystem methods.
mod vfunc_indices {
    /// FindTypeScopeForModule index
    /// IAppSystem has 10 base methods, then:
    /// - 10: GlobalTypeScope()
    /// - 11: FindOrCreateTypeScopeForModule()
    /// - 12: FindTypeScopeForModule()
    #[cfg(target_os = "linux")]
    pub const FIND_TYPE_SCOPE_FOR_MODULE: usize = 12;
    #[cfg(target_os = "windows")]
    pub const FIND_TYPE_SCOPE_FOR_MODULE: usize = 12;
}

/// Virtual function indices for CSchemaSystemTypeScope
mod scope_vfunc_indices {
    /// FindDeclaredClass index
    #[cfg(target_os = "linux")]
    pub const FIND_DECLARED_CLASS: usize = 2;
    #[cfg(target_os = "windows")]
    pub const FIND_DECLARED_CLASS: usize = 2;
}

/// Get a schema field offset
///
/// This function is cached - subsequent calls with the same class/field
/// will return the cached value without querying the engine.
///
/// # Arguments
/// * `class_name` - The schema class name (e.g., "CBaseEntity")
/// * `field_name` - The field name (e.g., "m_iHealth")
///
/// # Returns
/// The field offset and network status, or an error if not found.
pub fn get_offset(class_name: &str, field_name: &str) -> Result<SchemaOffset, SchemaError> {
    // Check cache first
    let cache_key = combined_hash(class_name.as_bytes(), field_name.as_bytes());
    if let Some(entry) = OFFSET_CACHE.get(&cache_key) {
        trace!(
            "Cache hit for {}.{}: offset={}",
            class_name,
            field_name,
            entry.offset
        );
        return Ok(*entry);
    }

    // Query schema system
    let offset = query_schema_offset(class_name, field_name)?;

    debug!(
        "Resolved {}.{}: offset={}, networked={}",
        class_name, field_name, offset.offset, offset.is_networked
    );

    // Cache and return
    OFFSET_CACHE.insert(cache_key, offset);
    Ok(offset)
}

/// Query the schema system for a field offset (uncached)
fn query_schema_offset(class_name: &str, field_name: &str) -> Result<SchemaOffset, SchemaError> {
    let engine = cs2rust_engine::globals::try_engine().ok_or(SchemaError::NotInitialized)?;
    let schema_system = engine.schema_system.as_ptr();

    unsafe {
        // Get type scope for "server" module
        let type_scope = call_find_type_scope_for_module(schema_system, "server")?;

        // Find the class
        let class_info = call_find_declared_class(type_scope, class_name)?;

        // Find the field
        find_field_offset(class_info, class_name, field_name)
    }
}

/// Call CSchemaSystem::FindTypeScopeForModule
///
/// # Safety
/// `schema_system` must be a valid CSchemaSystem pointer
unsafe fn call_find_type_scope_for_module(
    schema_system: *mut CSchemaSystem,
    module_name: &str,
) -> Result<*mut c_void, SchemaError> {
    let module_cstr =
        CString::new(module_name).map_err(|e| SchemaError::FfiError(e.to_string()))?;

    // Get vtable
    let vtable = *(schema_system as *const *const usize);
    let func_ptr = *vtable.add(vfunc_indices::FIND_TYPE_SCOPE_FOR_MODULE);

    // Cast to function pointer
    // CSchemaSystemTypeScope* (*)(CSchemaSystem*, const char*)
    let func: extern "C" fn(*mut CSchemaSystem, *const c_char) -> *mut c_void =
        std::mem::transmute(func_ptr);

    let result = func(schema_system, module_cstr.as_ptr());

    if result.is_null() {
        Err(SchemaError::TypeScopeNotFound(module_name.to_string()))
    } else {
        Ok(result)
    }
}

/// Call CSchemaSystemTypeScope::FindDeclaredClass
///
/// # Safety
/// `type_scope` must be a valid CSchemaSystemTypeScope pointer
unsafe fn call_find_declared_class(
    type_scope: *mut c_void,
    class_name: &str,
) -> Result<*mut c_void, SchemaError> {
    let class_cstr = CString::new(class_name).map_err(|e| SchemaError::FfiError(e.to_string()))?;

    // Get vtable
    let vtable = *(type_scope as *const *const usize);
    let func_ptr = *vtable.add(scope_vfunc_indices::FIND_DECLARED_CLASS);

    // Cast to function pointer
    // CSchemaClassInfo* (*)(CSchemaSystemTypeScope*, const char*)
    let func: extern "C" fn(*mut c_void, *const c_char) -> *mut c_void =
        std::mem::transmute(func_ptr);

    let result = func(type_scope, class_cstr.as_ptr());

    if result.is_null() {
        Err(SchemaError::ClassNotFound(class_name.to_string()))
    } else {
        Ok(result)
    }
}

/// Find a field offset within a CSchemaClassInfo
///
/// # CSchemaClassInfo layout (from s2sdk):
/// - 0x00: m_pSchemaBinding (self-reference)
/// - 0x08: m_pszName (const char*)
/// - 0x10: m_pszProjectName (const char*)
/// - 0x18: m_nSize (i32)
/// - 0x1c: m_nFieldCount (u16)
/// - 0x1e: m_nStaticMetadataCount (u16)
/// - 0x20: m_nAlignment (u8)
/// - 0x21: m_nBaseClassCount (u8)
/// - 0x22: m_nMultipleInheritanceDepth (u16)
/// - 0x24: m_nSingleInheritanceDepth (u16)
/// - 0x28: m_pFields (SchemaClassFieldData_t*)
/// - ...
///
/// # Safety
/// `class_info` must be a valid CSchemaClassInfo pointer
unsafe fn find_field_offset(
    class_info: *mut c_void,
    class_name: &str,
    field_name: &str,
) -> Result<SchemaOffset, SchemaError> {
    // Read field count (u16 at offset 0x1c)
    let field_count = *(class_info.byte_add(0x1c) as *const u16) as usize;

    // Read fields pointer (at offset 0x28)
    let fields_ptr = *(class_info.byte_add(0x28) as *const *mut c_void);

    if fields_ptr.is_null() {
        warn!(
            "Fields pointer is null for class {} (field_count={})",
            class_name, field_count
        );
        return Err(SchemaError::FieldNotFound {
            class: class_name.to_string(),
            field: field_name.to_string(),
        });
    }

    // SchemaClassFieldData_t layout:
    // - 0x00: m_pszName (const char*)
    // - 0x08: m_pType (CSchemaType*)
    // - 0x10: m_nSingleInheritanceOffset (i32)
    // - 0x14: m_nStaticMetadataCount (i32)
    // - 0x18: m_pStaticMetadata (SchemaMetadataEntryData_t*)
    // Size: 0x20 bytes
    const FIELD_SIZE: usize = 0x20;

    for i in 0..field_count {
        let field_ptr = fields_ptr.byte_add(i * FIELD_SIZE);

        // Read field name pointer
        let name_ptr = *(field_ptr as *const *const c_char);
        if name_ptr.is_null() {
            continue;
        }

        let name = CStr::from_ptr(name_ptr).to_string_lossy();

        if name == field_name {
            // Found it! Read offset (i32 at offset 0x10)
            let offset = *(field_ptr.byte_add(0x10) as *const i32);

            // Check if networked (look for MNetworkEnable in metadata)
            let is_networked = check_field_networked(field_ptr);

            return Ok(SchemaOffset {
                offset,
                is_networked,
            });
        }
    }

    Err(SchemaError::FieldNotFound {
        class: class_name.to_string(),
        field: field_name.to_string(),
    })
}

/// Check if a field has MNetworkEnable metadata
///
/// # SchemaMetadataEntryData_t layout:
/// - 0x00: m_pszName (const char*)
/// - 0x08: m_pData (void*)
/// Size: 0x10 bytes
///
/// # Safety
/// `field_ptr` must be a valid SchemaClassFieldData_t pointer
unsafe fn check_field_networked(field_ptr: *mut c_void) -> bool {
    // Read metadata count (i32 at offset 0x14)
    let metadata_count = *(field_ptr.byte_add(0x14) as *const i32) as usize;

    // Read metadata pointer (at offset 0x18)
    let metadata_ptr = *(field_ptr.byte_add(0x18) as *const *mut c_void);

    if metadata_ptr.is_null() || metadata_count == 0 {
        return false;
    }

    const METADATA_ENTRY_SIZE: usize = 0x10;

    for i in 0..metadata_count {
        let entry_ptr = metadata_ptr.byte_add(i * METADATA_ENTRY_SIZE);
        let name_ptr = *(entry_ptr as *const *const c_char);

        if !name_ptr.is_null() {
            let name = CStr::from_ptr(name_ptr).to_string_lossy();
            if name == "MNetworkEnable" {
                return true;
            }
        }
    }

    false
}

/// Clear the offset cache
///
/// Useful for hot-reload scenarios or when schema data may have changed.
pub fn clear_cache() {
    OFFSET_CACHE.clear();
    debug!("Schema offset cache cleared");
}

/// Get the number of cached offsets
pub fn cache_size() -> usize {
    OFFSET_CACHE.len()
}

/// Prefetch offsets for a list of class/field pairs
///
/// This can be used during plugin initialization to warm up the cache
/// and detect any missing fields early.
pub fn prefetch_offsets(pairs: &[(&str, &str)]) -> Vec<Result<SchemaOffset, SchemaError>> {
    pairs
        .iter()
        .map(|(class, field)| get_offset(class, field))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_operations() {
        // Test that cache starts empty
        clear_cache();
        assert_eq!(cache_size(), 0);
    }
}
