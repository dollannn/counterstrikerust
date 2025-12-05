//! Entity handle types for safe entity references
//!
//! In Source 2, entities are referenced via `CHandle<T>` which combines an entity index
//! with a serial number. The serial number invalidates when an entity is deleted and
//! a new one takes its slot, preventing stale references.
//!
//! # Handle Format
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                           u32 raw value                          │
//! ├─────────────────────────────┬───────────────────────────────────┤
//! │    Serial Number (17 bits)  │      Entity Index (15 bits)       │
//! │         bits 15-31          │           bits 0-14               │
//! └─────────────────────────────┴───────────────────────────────────┘
//! ```
//!
//! - Entity index: Lower 15 bits (0-32767)
//! - Serial number: Upper 17 bits
//! - Invalid handle: 0xFFFFFFFF (all bits set)

use std::ffi::c_void;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

use crate::schema::SchemaObject;

/// Maximum entity index bits (15 bits = 32768 entities)
pub const MAX_EDICT_BITS: u32 = 15;

/// Maximum number of entities
pub const MAX_EDICTS: u32 = 1 << MAX_EDICT_BITS;

/// Number of serial number bits
pub const NUM_SERIAL_NUMBER_BITS: u32 = 17;

/// Invalid handle sentinel value
pub const INVALID_EHANDLE_INDEX: u32 = 0xFFFFFFFF;

/// Entity handle mask for extracting index
const INDEX_MASK: u32 = MAX_EDICTS - 1; // 0x7FFF

/// A type-safe handle to an entity
///
/// `CHandle<T>` stores a 32-bit value containing both an entity index and a serial number.
/// When resolved, it returns `Option<T>` - `None` if the handle is invalid or the entity
/// no longer exists.
///
/// # Type Safety
///
/// The `T` parameter ensures type-safe resolution:
///
/// ```ignore
/// use cs2rust_core::entities::{CHandle, PlayerController, PlayerPawn};
///
/// let pawn_handle: CHandle<PlayerPawn> = controller.player_pawn();
/// let pawn: Option<PlayerPawn> = pawn_handle.get(); // Returns PlayerPawn
/// ```
///
/// # Example
///
/// ```ignore
/// // Get player pawn handle from controller
/// let pawn_handle: CHandle<PlayerPawn> = controller.player_pawn();
///
/// // Check validity and resolve
/// if pawn_handle.is_valid() {
///     if let Some(pawn) = pawn_handle.get() {
///         println!("Pawn health: {}", pawn.health());
///     }
/// }
/// ```
#[repr(C)]
pub struct CHandle<T> {
    value: u32,
    _marker: PhantomData<T>,
}

impl<T> CHandle<T> {
    /// Create a new handle from a raw value
    #[inline]
    pub const fn from_raw(value: u32) -> Self {
        Self {
            value,
            _marker: PhantomData,
        }
    }

    /// Create an invalid handle
    #[inline]
    pub const fn invalid() -> Self {
        Self::from_raw(INVALID_EHANDLE_INDEX)
    }

    /// Get the raw handle value
    #[inline]
    pub const fn raw(&self) -> u32 {
        self.value
    }

    /// Get the entity index (lower 15 bits)
    #[inline]
    pub const fn index(&self) -> u32 {
        self.value & INDEX_MASK
    }

    /// Get the serial number (upper 17 bits)
    #[inline]
    pub const fn serial(&self) -> u32 {
        self.value >> MAX_EDICT_BITS
    }

    /// Check if this handle is valid (not the invalid sentinel)
    ///
    /// Note: A "valid" handle may still fail to resolve if the entity
    /// was deleted or the serial number doesn't match.
    #[inline]
    pub const fn is_valid(&self) -> bool {
        self.index() != (MAX_EDICTS - 1)
    }

    /// Cast this handle to a different entity type
    ///
    /// # Safety
    ///
    /// The caller must ensure the entity is actually of type `U`.
    #[inline]
    pub const fn cast<U>(self) -> CHandle<U> {
        CHandle::from_raw(self.value)
    }
}

impl<T: SchemaObject> CHandle<T> {
    /// Resolve the handle to an entity
    ///
    /// Returns `None` if:
    /// - The handle is invalid
    /// - The entity system is not available
    /// - The entity at this index doesn't exist
    /// - The serial number doesn't match (entity was recycled)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let pawn_handle: CHandle<PlayerPawn> = controller.player_pawn();
    /// if let Some(pawn) = pawn_handle.get() {
    ///     println!("Health: {}", pawn.health());
    /// }
    /// ```
    pub fn get(&self) -> Option<T> {
        if !self.is_valid() {
            return None;
        }

        // Get entity pointer from entity system
        let ptr = super::system::get_entity_by_handle(self.value)?;

        // Safety: The entity system verified the handle is valid and returned
        // a pointer to the correct entity type
        unsafe { T::from_ptr(ptr) }
    }
}

impl<T> CHandle<T> {
    /// Get the entity pointer if valid, without type construction
    ///
    /// Useful when you need the raw pointer for FFI calls.
    pub fn get_ptr(&self) -> Option<*mut c_void> {
        if !self.is_valid() {
            return None;
        }
        super::system::get_entity_by_handle(self.value)
    }
}

impl<T> Clone for CHandle<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for CHandle<T> {}

impl<T> Default for CHandle<T> {
    fn default() -> Self {
        Self::invalid()
    }
}

impl<T> PartialEq for CHandle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T> Eq for CHandle<T> {}

impl<T> Hash for CHandle<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

impl<T> fmt::Debug for CHandle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_valid() {
            write!(
                f,
                "CHandle(index={}, serial={})",
                self.index(),
                self.serial()
            )
        } else {
            write!(f, "CHandle(invalid)")
        }
    }
}

impl<T> fmt::Display for CHandle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_valid() {
            write!(f, "{}:{}", self.index(), self.serial())
        } else {
            write!(f, "invalid")
        }
    }
}

/// Non-generic entity handle (like CEntityHandle in Source 2)
pub type CEntityHandle = CHandle<()>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_from_raw() {
        // Test typical handle value
        let handle: CHandle<()> = CHandle::from_raw(0x12345678);
        assert_eq!(handle.raw(), 0x12345678);
    }

    #[test]
    fn test_handle_index_extraction() {
        // Index is lower 15 bits
        let handle: CHandle<()> = CHandle::from_raw(0x00001234);
        assert_eq!(handle.index(), 0x1234);

        // Max index value
        let handle: CHandle<()> = CHandle::from_raw(0x7FFF);
        assert_eq!(handle.index(), 0x7FFF);
    }

    #[test]
    fn test_handle_serial_extraction() {
        // Serial is upper 17 bits
        let handle: CHandle<()> = CHandle::from_raw(0xABCD8000);
        assert_eq!(handle.serial(), 0xABCD8000 >> 15);
        assert_eq!(handle.index(), 0);
    }

    #[test]
    fn test_handle_validity() {
        // Valid handle
        let handle: CHandle<()> = CHandle::from_raw(0x00001234);
        assert!(handle.is_valid());

        // Invalid handle (sentinel value)
        let handle: CHandle<()> = CHandle::invalid();
        assert!(!handle.is_valid());

        // Handle with max index is invalid
        let handle: CHandle<()> = CHandle::from_raw(0x7FFF);
        assert!(!handle.is_valid());
    }

    #[test]
    fn test_handle_display() {
        let handle: CHandle<()> = CHandle::from_raw(0x00018001); // index=1, serial=3
        assert_eq!(format!("{}", handle), "1:3");

        let invalid: CHandle<()> = CHandle::invalid();
        assert_eq!(format!("{}", invalid), "invalid");
    }

    #[test]
    fn test_handle_debug() {
        let handle: CHandle<()> = CHandle::from_raw(0x00018001);
        let debug = format!("{:?}", handle);
        assert!(debug.contains("index=1"));
        assert!(debug.contains("serial=3"));
    }

    #[test]
    fn test_handle_cast() {
        let handle: CHandle<i32> = CHandle::from_raw(0x1234);
        let casted: CHandle<u64> = handle.cast();
        assert_eq!(casted.raw(), handle.raw());
    }

    #[test]
    fn test_handle_clone_copy() {
        let handle1: CHandle<()> = CHandle::from_raw(0x1234);
        let handle2 = handle1;
        let handle3 = handle1.clone();
        assert_eq!(handle1, handle2);
        assert_eq!(handle1, handle3);
    }
}
