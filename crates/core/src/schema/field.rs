//! Type-safe schema field accessor
//!
//! This module provides a generic `SchemaField<T>` type that lazily resolves
//! field offsets from the schema system and provides safe read/write access
//! to entity properties.

use std::ffi::c_void;
use std::marker::PhantomData;
use std::sync::OnceLock;

use super::system::{get_offset, SchemaError, SchemaOffset};

/// A lazily-resolved schema field accessor
///
/// The offset is queried from CSchemaSystem on first access and cached
/// in a `OnceLock` for thread-safe, lock-free subsequent access.
///
/// # Type Parameters
/// * `T` - The field type. Must be `Copy` for safe read/write through raw pointers.
///
/// # Example
///
/// ```ignore
/// // Define a field accessor (typically done once as a static)
/// static HEALTH: SchemaField<i32> = SchemaField::new("CBaseEntity", "m_iHealth");
///
/// // Use it to read/write entity properties
/// unsafe {
///     let hp = HEALTH.get(entity_ptr);
///     HEALTH.set(entity_ptr, hp + 10);
/// }
/// ```
pub struct SchemaField<T: Copy> {
    class_name: &'static str,
    field_name: &'static str,
    offset: OnceLock<SchemaOffset>,
    _marker: PhantomData<T>,
}

impl<T: Copy> SchemaField<T> {
    /// Create a new schema field accessor
    ///
    /// The offset is not resolved until first access. This allows defining
    /// fields as `const` statics without requiring the schema system to
    /// be initialized at compile time.
    ///
    /// # Arguments
    /// * `class_name` - The schema class name (e.g., "CBaseEntity")
    /// * `field_name` - The field name (e.g., "m_iHealth")
    pub const fn new(class_name: &'static str, field_name: &'static str) -> Self {
        Self {
            class_name,
            field_name,
            offset: OnceLock::new(),
            _marker: PhantomData,
        }
    }

    /// Resolve the field offset (cached after first call)
    ///
    /// This queries the schema system for the offset on first call,
    /// then returns the cached value on subsequent calls.
    pub fn resolve(&self) -> Result<&SchemaOffset, SchemaError> {
        // Check if already initialized
        if let Some(offset) = self.offset.get() {
            return Ok(offset);
        }

        // Query schema system
        let offset = get_offset(self.class_name, self.field_name)?;

        // Try to set it (may race with another thread, that's ok)
        let _ = self.offset.set(offset);

        // Return the value (either ours or the winner's)
        Ok(self.offset.get().expect("OnceLock should be set"))
    }

    /// Get the field offset (panics if resolution fails)
    ///
    /// # Panics
    /// Panics if the field cannot be resolved from the schema system.
    pub fn offset(&self) -> i32 {
        self.resolve()
            .expect("Failed to resolve schema offset")
            .offset
    }

    /// Try to get the field offset without panicking
    pub fn try_offset(&self) -> Option<i32> {
        self.resolve().ok().map(|o| o.offset)
    }

    /// Check if this field is networked
    ///
    /// Networked fields trigger replication to clients when modified
    /// and require calling `network_state_changed` after writes.
    pub fn is_networked(&self) -> bool {
        self.resolve().map(|o| o.is_networked).unwrap_or(false)
    }

    /// Read the field value from an entity pointer
    ///
    /// # Safety
    /// - `base` must be a valid pointer to an entity of the correct class
    /// - The field type `T` must match the actual schema field type
    /// - The entity must remain valid for the duration of the read
    #[inline]
    pub unsafe fn get(&self, base: *const c_void) -> T {
        debug_assert!(!base.is_null(), "Null entity pointer");
        let offset = self.offset();
        let ptr = base.byte_add(offset as usize) as *const T;
        ptr.read()
    }

    /// Write a value to the field
    ///
    /// # Safety
    /// - `base` must be a valid pointer to an entity of the correct class
    /// - The field type `T` must match the actual schema field type
    /// - The entity must remain valid for the duration of the write
    /// - For networked fields, caller must call `network_state_changed` afterwards
    ///   for the change to be replicated to clients
    #[inline]
    pub unsafe fn set(&self, base: *mut c_void, value: T) {
        debug_assert!(!base.is_null(), "Null entity pointer");
        let offset = self.offset();
        let ptr = base.byte_add(offset as usize) as *mut T;
        ptr.write(value);
    }

    /// Read the field value, returning None if resolution fails
    ///
    /// This is useful during initialization when the schema system
    /// may not be fully ready.
    ///
    /// # Safety
    /// Same requirements as `get()`
    pub unsafe fn try_get(&self, base: *const c_void) -> Option<T> {
        if base.is_null() {
            return None;
        }
        let offset = self.resolve().ok()?.offset;
        let ptr = base.byte_add(offset as usize) as *const T;
        Some(ptr.read())
    }

    /// Write a value to the field, returning success status
    ///
    /// # Safety
    /// Same requirements as `set()`
    pub unsafe fn try_set(&self, base: *mut c_void, value: T) -> bool {
        if base.is_null() {
            return false;
        }
        if let Ok(schema_offset) = self.resolve() {
            let ptr = base.byte_add(schema_offset.offset as usize) as *mut T;
            ptr.write(value);
            true
        } else {
            false
        }
    }

    /// Get a mutable reference to the field value
    ///
    /// # Safety
    /// - `base` must be a valid pointer to an entity of the correct class
    /// - The field type `T` must match the actual schema field type
    /// - The returned reference is only valid while `base` is valid
    /// - No other code may read/write this field while the reference is held
    #[inline]
    pub unsafe fn get_mut(&self, base: *mut c_void) -> &mut T {
        debug_assert!(!base.is_null(), "Null entity pointer");
        let offset = self.offset();
        let ptr = base.byte_add(offset as usize) as *mut T;
        &mut *ptr
    }

    /// Get class name
    pub const fn class_name(&self) -> &'static str {
        self.class_name
    }

    /// Get field name
    pub const fn field_name(&self) -> &'static str {
        self.field_name
    }

    /// Check if the offset has been resolved
    pub fn is_resolved(&self) -> bool {
        self.offset.get().is_some()
    }
}

// SchemaField is Send + Sync because:
// - class_name and field_name are &'static str (inherently thread-safe)
// - offset is OnceLock which is thread-safe
// - PhantomData<T> doesn't affect thread safety
unsafe impl<T: Copy> Send for SchemaField<T> {}
unsafe impl<T: Copy> Sync for SchemaField<T> {}

/// Example manual schema field definitions
///
/// These demonstrate how to manually define schema fields before
/// proc macros are available (Phase 5).
pub mod examples {
    use super::*;

    /// Manual schema field definitions for CBaseEntity
    pub mod base_entity {
        use super::*;

        /// Health points
        pub static M_I_HEALTH: SchemaField<i32> = SchemaField::new("CBaseEntity", "m_iHealth");

        /// Team number (2=T, 3=CT)
        pub static M_I_TEAM_NUM: SchemaField<i32> = SchemaField::new("CBaseEntity", "m_iTeamNum");

        /// Entity flags
        pub static M_F_FLAGS: SchemaField<u32> = SchemaField::new("CBaseEntity", "m_fFlags");
    }

    /// Manual schema field definitions for CCSPlayerPawn
    pub mod player_pawn {
        use super::*;

        /// Player health (inherited from CBaseEntity)
        pub static M_I_HEALTH: SchemaField<i32> = SchemaField::new("CCSPlayerPawn", "m_iHealth");

        /// Armor value
        pub static M_ARMOR_VALUE: SchemaField<i32> =
            SchemaField::new("CCSPlayerPawn", "m_ArmorValue");

        /// Has helmet
        pub static M_B_HAS_HELMET: SchemaField<bool> =
            SchemaField::new("CCSPlayerPawn", "m_bHasHeavyArmor");
    }

    /// Manual schema field definitions for CCSPlayerController
    pub mod player_controller {
        use super::*;

        /// Player name
        pub static M_SZ_CLAN_NAME: SchemaField<[u8; 32]> =
            SchemaField::new("CCSPlayerController", "m_szClan");

        /// Player ping
        pub static M_I_PING: SchemaField<u32> = SchemaField::new("CCSPlayerController", "m_iPing");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_field_construction() {
        let field: SchemaField<i32> = SchemaField::new("TestClass", "m_testField");
        assert_eq!(field.class_name(), "TestClass");
        assert_eq!(field.field_name(), "m_testField");
        assert!(!field.is_resolved());
    }

    #[test]
    fn test_schema_field_is_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<SchemaField<i32>>();
    }

    #[test]
    fn test_schema_field_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<SchemaField<i32>>();
    }
}
