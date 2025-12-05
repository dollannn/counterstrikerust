//! Schema System - Runtime type introspection for Source 2 entities
//!
//! This module provides access to Source 2's schema reflection system,
//! allowing runtime lookup of class field offsets for reading and writing
//! entity properties.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                   CSchemaSystem (Engine)                    │
//! │   FindTypeScopeForModule() → CSchemaSystemTypeScope         │
//! │   FindDeclaredClass() → CSchemaClassInfo                    │
//! └─────────────────────────────┬───────────────────────────────┘
//!                               │
//!                               ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                   Rust Schema Module                        │
//! │  ┌─────────────────────────────────────────────────────┐   │
//! │  │ system::get_offset(class, field) -> SchemaOffset    │   │
//! │  │   - Queries CSchemaSystem                           │   │
//! │  │   - Caches results in DashMap                       │   │
//! │  └─────────────────────────────────────────────────────┘   │
//! │                          │                                  │
//! │  ┌─────────────────────────────────────────────────────┐   │
//! │  │ SchemaField<T>                                      │   │
//! │  │   - Per-field OnceLock for offset                   │   │
//! │  │   - get(base_ptr) -> T                              │   │
//! │  │   - set(base_ptr, value)                            │   │
//! │  └─────────────────────────────────────────────────────┘   │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Usage
//!
//! ## Manual Field Access (Phase 4)
//!
//! ```ignore
//! use cs2rust_core::schema::{SchemaField, get_offset};
//!
//! // Define field accessors as statics
//! static HEALTH: SchemaField<i32> = SchemaField::new("CBaseEntity", "m_iHealth");
//!
//! // Read/write entity properties
//! unsafe {
//!     let hp = HEALTH.get(entity_ptr);
//!     HEALTH.set(entity_ptr, 100);
//! }
//!
//! // Or query offsets directly
//! let offset = get_offset("CBaseEntity", "m_iHealth")?;
//! println!("m_iHealth offset: {}", offset.offset);
//! ```
//!
//! ## Network State Changes
//!
//! When modifying networked fields (fields with `MNetworkEnable` metadata),
//! you must notify the engine to replicate changes to clients:
//!
//! ```ignore
//! if HEALTH.is_networked() {
//!     // Call network_state_changed after modifying
//!     network_state_changed(entity_ptr, HEALTH.offset());
//! }
//! ```
//!
//! # Performance
//!
//! - First access: ~1-5μs (schema system query)
//! - Subsequent access: ~10ns (cache lookup)
//! - Per-field `OnceLock` provides lock-free access after first resolution

pub mod field;
pub mod hash;
pub mod network;
pub mod system;

// Re-export primary types
pub use field::SchemaField;
pub use hash::{combined_hash, fnv1a_32, fnv1a_64};
pub use network::{clear_chain_cache, network_state_changed, network_state_changed_ex};
pub use system::{
    cache_size, clear_cache, get_offset, prefetch_offsets, SchemaError, SchemaOffset,
};

// Re-export example field definitions for testing
pub use field::examples;

/// Trait for types that wrap schema objects
///
/// This trait is implemented by the `#[derive(SchemaClass)]` macro and provides
/// a common interface for all schema object wrappers.
pub trait SchemaObject: Sized {
    /// Get the raw pointer to the native object
    fn ptr(&self) -> *mut std::ffi::c_void;

    /// Get the class name
    fn class_name(&self) -> &'static str;

    /// Check if the pointer is valid
    fn is_valid(&self) -> bool;

    /// Create an instance from a raw pointer
    ///
    /// # Safety
    /// The pointer must be valid and point to an instance of this class.
    unsafe fn from_ptr(ptr: *mut std::ffi::c_void) -> Option<Self>;
}
