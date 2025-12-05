//! EntityRef enum for typed entity access in listener callbacks
//!
//! Provides auto-detection of entity types from CEntityInstance pointers,
//! enabling pattern matching on common entity types like PlayerPawn and PlayerController.

use std::ffi::{c_void, CStr};
use std::fmt;

use crate::schema::SchemaObject;

use super::{BaseEntity, PlayerController, PlayerPawn};

/// Typed reference to an entity, auto-detected from CEntityInstance
///
/// This enum is passed to entity listener callbacks and provides typed access
/// to common entity types. The entity type is determined by reading the
/// classname from CEntityIdentity.
///
/// # Example
///
/// ```ignore
/// listeners::on_entity_spawned(|entity_ref| {
///     match entity_ref {
///         EntityRef::PlayerPawn(pawn) => {
///             tracing::info!("Player pawn spawned with health: {}", pawn.health());
///         }
///         EntityRef::PlayerController(controller) => {
///             tracing::info!("Player controller: {}", controller.name_string());
///         }
///         EntityRef::Unknown { classname, .. } => {
///             tracing::debug!("Unknown entity: {}", classname);
///         }
///         _ => {}
///     }
/// });
/// ```
pub enum EntityRef {
    /// CCSPlayerPawn - the physical player entity in the game world
    PlayerPawn(PlayerPawn),

    /// CCSPlayerController - manages player connection and metadata
    PlayerController(PlayerController),

    /// CBaseEntity - base entity type (used when specific type not needed)
    BaseEntity(BaseEntity),

    /// Unknown entity type - fallback for entities without specific wrappers
    Unknown {
        /// Raw pointer to CEntityInstance
        ptr: *mut c_void,
        /// Entity classname (e.g., "weapon_ak47", "prop_physics")
        classname: String,
        /// Entity index
        index: i32,
    },
}

impl fmt::Debug for EntityRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EntityRef::PlayerPawn(p) => f
                .debug_tuple("PlayerPawn")
                .field(&format_args!("{:p}", p.as_ptr()))
                .finish(),
            EntityRef::PlayerController(c) => f
                .debug_tuple("PlayerController")
                .field(&format_args!("{:p}", c.as_ptr()))
                .finish(),
            EntityRef::BaseEntity(e) => f
                .debug_tuple("BaseEntity")
                .field(&format_args!("{:p}", e.as_ptr()))
                .finish(),
            EntityRef::Unknown {
                ptr,
                classname,
                index,
            } => f
                .debug_struct("Unknown")
                .field("ptr", &format_args!("{:p}", ptr))
                .field("classname", classname)
                .field("index", index)
                .finish(),
        }
    }
}

// CEntityInstance layout:
//   +0x00: vtable
//   +0x08: m_iszPrivateVScripts (CUtlSymbolLarge)
//   +0x10: m_pEntity (CEntityIdentity*)
//
// CEntityIdentity layout:
//   +0x00: m_pInstance (CEntityInstance*)
//   +0x08: m_pClass (CEntityClass*)
//   +0x10: m_EHandle (CEntityHandle - 4 bytes)
//   +0x14: m_nameStringableIndex (int32)
//   +0x18: m_name (CUtlSymbolLarge - 8 bytes)
//   +0x20: m_designerName (CUtlSymbolLarge - 8 bytes) <- classname

/// Offset to m_pEntity in CEntityInstance
const ENTITY_IDENTITY_OFFSET: usize = 0x10;

/// Offset to m_EHandle in CEntityIdentity
const EHANDLE_OFFSET: usize = 0x10;

/// Offset to m_designerName in CEntityIdentity
const DESIGNER_NAME_OFFSET: usize = 0x20;

impl EntityRef {
    /// Create an EntityRef by detecting the entity type from CEntityInstance
    ///
    /// # Safety
    /// - `entity_ptr` must be a valid pointer to a CEntityInstance
    /// - The entity must remain valid for the lifetime of the EntityRef
    pub unsafe fn from_entity_instance(entity_ptr: *mut c_void) -> Option<Self> {
        if entity_ptr.is_null() {
            return None;
        }

        // Get classname and index from the entity
        let classname = Self::read_classname(entity_ptr)?;
        let index = Self::read_entity_index(entity_ptr);

        // Match against known entity types
        let entity_ref = match classname.as_str() {
            "CCSPlayerPawn" => PlayerPawn::from_ptr(entity_ptr)
                .map(EntityRef::PlayerPawn)
                .unwrap_or_else(|| EntityRef::Unknown {
                    ptr: entity_ptr,
                    classname,
                    index,
                }),
            "CCSPlayerController" => PlayerController::from_ptr(entity_ptr)
                .map(EntityRef::PlayerController)
                .unwrap_or_else(|| EntityRef::Unknown {
                    ptr: entity_ptr,
                    classname,
                    index,
                }),
            // Treat CBaseEntity and common base classes as BaseEntity
            "CBaseEntity" | "CBaseModelEntity" | "CBaseCombatCharacter" => {
                BaseEntity::from_ptr(entity_ptr)
                    .map(EntityRef::BaseEntity)
                    .unwrap_or_else(|| EntityRef::Unknown {
                        ptr: entity_ptr,
                        classname,
                        index,
                    })
            }
            // All other entities fall through to Unknown
            _ => EntityRef::Unknown {
                ptr: entity_ptr,
                classname,
                index,
            },
        };

        Some(entity_ref)
    }

    /// Read the classname from a CEntityInstance pointer
    ///
    /// CUtlSymbolLarge stores a pointer to an interned string.
    unsafe fn read_classname(entity_ptr: *mut c_void) -> Option<String> {
        // Read CEntityIdentity pointer
        let identity_ptr = *(entity_ptr.byte_add(ENTITY_IDENTITY_OFFSET) as *const *const c_void);
        if identity_ptr.is_null() {
            return None;
        }

        // CUtlSymbolLarge is essentially a pointer to a string
        // m_designerName.String() returns the raw string pointer
        let name_ptr = *(identity_ptr.byte_add(DESIGNER_NAME_OFFSET) as *const *const i8);
        if name_ptr.is_null() {
            return None;
        }

        // Convert to Rust string
        CStr::from_ptr(name_ptr)
            .to_str()
            .ok()
            .map(|s| s.to_string())
    }

    /// Read the entity index from a CEntityInstance pointer
    ///
    /// The entity index is stored in CEntityIdentity::m_EHandle
    unsafe fn read_entity_index(entity_ptr: *mut c_void) -> i32 {
        let identity_ptr = *(entity_ptr.byte_add(ENTITY_IDENTITY_OFFSET) as *const *const c_void);
        if identity_ptr.is_null() {
            return -1;
        }

        // CEntityHandle stores index in lower bits (14 bits for index)
        let handle = *(identity_ptr.byte_add(EHANDLE_OFFSET) as *const u32);
        (handle & 0x3FFF) as i32
    }

    /// Get the raw pointer to the underlying CEntityInstance
    pub fn as_ptr(&self) -> *mut c_void {
        match self {
            EntityRef::PlayerPawn(p) => p.as_ptr(),
            EntityRef::PlayerController(c) => c.as_ptr(),
            EntityRef::BaseEntity(e) => e.as_ptr(),
            EntityRef::Unknown { ptr, .. } => *ptr,
        }
    }

    /// Get the entity classname
    pub fn classname(&self) -> &str {
        match self {
            EntityRef::PlayerPawn(_) => PlayerPawn::CLASS_NAME,
            EntityRef::PlayerController(_) => PlayerController::CLASS_NAME,
            EntityRef::BaseEntity(_) => BaseEntity::CLASS_NAME,
            EntityRef::Unknown { classname, .. } => classname,
        }
    }

    /// Get the entity index (-1 if unavailable)
    pub fn index(&self) -> i32 {
        match self {
            EntityRef::Unknown { index, .. } => *index,
            _ => unsafe { Self::read_entity_index(self.as_ptr()) },
        }
    }

    /// Read entity index from a raw CEntityInstance pointer
    ///
    /// This is a public helper for other modules that need to get the entity index.
    /// Returns -1 if the pointer is null or the identity is null.
    pub fn read_entity_index_from_ptr(entity_ptr: *mut c_void) -> i32 {
        if entity_ptr.is_null() {
            return -1;
        }
        unsafe { Self::read_entity_index(entity_ptr) }
    }

    /// Check if this is a player pawn
    pub fn is_player_pawn(&self) -> bool {
        matches!(self, EntityRef::PlayerPawn(_))
    }

    /// Check if this is a player controller
    pub fn is_player_controller(&self) -> bool {
        matches!(self, EntityRef::PlayerController(_))
    }

    /// Check if this is any player entity (pawn or controller)
    pub fn is_player(&self) -> bool {
        self.is_player_pawn() || self.is_player_controller()
    }

    /// Try to get as PlayerPawn reference
    pub fn as_player_pawn(&self) -> Option<&PlayerPawn> {
        match self {
            EntityRef::PlayerPawn(p) => Some(p),
            _ => None,
        }
    }

    /// Try to get as mutable PlayerPawn reference
    pub fn as_player_pawn_mut(&mut self) -> Option<&mut PlayerPawn> {
        match self {
            EntityRef::PlayerPawn(p) => Some(p),
            _ => None,
        }
    }

    /// Try to get as PlayerController reference
    pub fn as_player_controller(&self) -> Option<&PlayerController> {
        match self {
            EntityRef::PlayerController(c) => Some(c),
            _ => None,
        }
    }

    /// Try to get as mutable PlayerController reference
    pub fn as_player_controller_mut(&mut self) -> Option<&mut PlayerController> {
        match self {
            EntityRef::PlayerController(c) => Some(c),
            _ => None,
        }
    }

    /// Try to get as BaseEntity reference
    pub fn as_base_entity(&self) -> Option<&BaseEntity> {
        match self {
            EntityRef::BaseEntity(e) => Some(e),
            _ => None,
        }
    }

    /// Check if classname starts with a prefix (useful for weapon detection)
    pub fn classname_starts_with(&self, prefix: &str) -> bool {
        self.classname().starts_with(prefix)
    }

    /// Check if this is a weapon entity
    pub fn is_weapon(&self) -> bool {
        let name = self.classname();
        name.starts_with("weapon_") || name.starts_with("CWeapon") || name.starts_with("CCSWeapon")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_null_ptr() {
        let result = unsafe { EntityRef::from_entity_instance(std::ptr::null_mut()) };
        assert!(result.is_none());
    }
}
