//! Entity lifecycle listeners
//!
//! - OnEntityCreated: Called when an entity is allocated
//! - OnEntitySpawned: Called when an entity becomes active
//! - OnEntityDeleted: Called when an entity is being deleted

use std::ffi::c_void;
use std::sync::LazyLock;

use parking_lot::RwLock;
use slotmap::SlotMap;

use super::{register_key, ListenerKey, ListenerType};
use crate::entities::EntityRef;

// Callback types
/// Callback for entity events, receives a typed EntityRef
pub type EntityCallback = Box<dyn Fn(EntityRef) + Send + Sync>;

// Registries
struct EntityCreatedRegistry {
    callbacks: SlotMap<ListenerKey, EntityCallback>,
}

struct EntitySpawnedRegistry {
    callbacks: SlotMap<ListenerKey, EntityCallback>,
}

struct EntityDeletedRegistry {
    callbacks: SlotMap<ListenerKey, EntityCallback>,
}

static ENTITY_CREATED_REGISTRY: LazyLock<RwLock<EntityCreatedRegistry>> = LazyLock::new(|| {
    RwLock::new(EntityCreatedRegistry {
        callbacks: SlotMap::with_key(),
    })
});

static ENTITY_SPAWNED_REGISTRY: LazyLock<RwLock<EntitySpawnedRegistry>> = LazyLock::new(|| {
    RwLock::new(EntitySpawnedRegistry {
        callbacks: SlotMap::with_key(),
    })
});

static ENTITY_DELETED_REGISTRY: LazyLock<RwLock<EntityDeletedRegistry>> = LazyLock::new(|| {
    RwLock::new(EntityDeletedRegistry {
        callbacks: SlotMap::with_key(),
    })
});

// === OnEntityCreated ===

/// Register a callback to be called when an entity is created
///
/// This is called when the entity is first allocated, before it is spawned.
/// The entity may not be fully initialized at this point.
///
/// # Arguments
/// The callback receives an `EntityRef` which provides typed access to the entity.
///
/// # Returns
/// A key that can be used to unregister the callback via `remove_listener`.
pub fn on_entity_created<F>(callback: F) -> ListenerKey
where
    F: Fn(EntityRef) + Send + Sync + 'static,
{
    let key = register_key(ListenerType::EntityCreated);
    ENTITY_CREATED_REGISTRY
        .write()
        .callbacks
        .insert(Box::new(callback));
    key
}

pub(super) fn remove_entity_created(key: ListenerKey) -> bool {
    ENTITY_CREATED_REGISTRY
        .write()
        .callbacks
        .remove(key)
        .is_some()
}

/// Fire all entity created callbacks
///
/// # Safety
/// `entity_ptr` must be a valid pointer to a CEntityInstance
pub unsafe fn fire_entity_created(entity_ptr: *mut c_void) {
    if let Some(entity_ref) = EntityRef::from_entity_instance(entity_ptr) {
        tracing::trace!("Firing OnEntityCreated: {}", entity_ref.classname());
        let registry = ENTITY_CREATED_REGISTRY.read();
        for (_, callback) in registry.callbacks.iter() {
            callback(EntityRef::from_entity_instance(entity_ptr).unwrap());
        }
    }
}

// === OnEntitySpawned ===

/// Register a callback to be called when an entity is spawned
///
/// This is called when the entity becomes active in the game world.
/// The entity is fully initialized at this point.
///
/// # Arguments
/// The callback receives an `EntityRef` which provides typed access to the entity.
///
/// # Returns
/// A key that can be used to unregister the callback via `remove_listener`.
pub fn on_entity_spawned<F>(callback: F) -> ListenerKey
where
    F: Fn(EntityRef) + Send + Sync + 'static,
{
    let key = register_key(ListenerType::EntitySpawned);
    ENTITY_SPAWNED_REGISTRY
        .write()
        .callbacks
        .insert(Box::new(callback));
    key
}

pub(super) fn remove_entity_spawned(key: ListenerKey) -> bool {
    ENTITY_SPAWNED_REGISTRY
        .write()
        .callbacks
        .remove(key)
        .is_some()
}

/// Fire all entity spawned callbacks
///
/// # Safety
/// `entity_ptr` must be a valid pointer to a CEntityInstance
pub unsafe fn fire_entity_spawned(entity_ptr: *mut c_void) {
    if let Some(entity_ref) = EntityRef::from_entity_instance(entity_ptr) {
        tracing::trace!("Firing OnEntitySpawned: {}", entity_ref.classname());
        let registry = ENTITY_SPAWNED_REGISTRY.read();
        for (_, callback) in registry.callbacks.iter() {
            callback(EntityRef::from_entity_instance(entity_ptr).unwrap());
        }
    }
}

// === OnEntityDeleted ===

/// Register a callback to be called when an entity is deleted
///
/// This is called when the entity is being removed from the game.
/// The entity is still valid during this callback but will be freed afterward.
///
/// # Arguments
/// The callback receives an `EntityRef` which provides typed access to the entity.
///
/// # Returns
/// A key that can be used to unregister the callback via `remove_listener`.
pub fn on_entity_deleted<F>(callback: F) -> ListenerKey
where
    F: Fn(EntityRef) + Send + Sync + 'static,
{
    let key = register_key(ListenerType::EntityDeleted);
    ENTITY_DELETED_REGISTRY
        .write()
        .callbacks
        .insert(Box::new(callback));
    key
}

pub(super) fn remove_entity_deleted(key: ListenerKey) -> bool {
    ENTITY_DELETED_REGISTRY
        .write()
        .callbacks
        .remove(key)
        .is_some()
}

/// Fire all entity deleted callbacks
///
/// # Safety
/// `entity_ptr` must be a valid pointer to a CEntityInstance
pub unsafe fn fire_entity_deleted(entity_ptr: *mut c_void) {
    if let Some(entity_ref) = EntityRef::from_entity_instance(entity_ptr) {
        tracing::trace!("Firing OnEntityDeleted: {}", entity_ref.classname());
        let registry = ENTITY_DELETED_REGISTRY.read();
        for (_, callback) in registry.callbacks.iter() {
            callback(EntityRef::from_entity_instance(entity_ptr).unwrap());
        }
    }
}
