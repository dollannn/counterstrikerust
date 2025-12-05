//! Global listeners for server lifecycle, client, and entity events
//!
//! This module provides a callback registration system for various game events.
//! Each listener type follows the same pattern as `gameframe.rs`: callbacks are
//! stored in a thread-safe registry and invoked when the corresponding event occurs.
//!
//! # Example
//!
//! ```ignore
//! use cs2rust_core::listeners;
//!
//! // Register a map start callback
//! let key = listeners::on_map_start(|map_name| {
//!     tracing::info!("Map started: {}", map_name);
//! });
//!
//! // Later, unregister if needed
//! listeners::remove_listener(key);
//! ```

pub mod client;
pub mod entity;
pub mod server;

use std::sync::LazyLock;

use parking_lot::RwLock;
use slotmap::{new_key_type, SlotMap};

new_key_type! {
    /// Key for registered listeners, used for removal
    pub struct ListenerKey;
}

/// Internal enum to track which registry a listener belongs to
#[derive(Clone, Copy, Debug)]
enum ListenerType {
    Tick,
    MapStart,
    MapEnd,
    ClientConnect,
    ClientDisconnect,
    ClientPutInServer,
    EntityCreated,
    EntitySpawned,
    EntityDeleted,
}

/// Mapping from ListenerKey to its type for removal
struct KeyRegistry {
    keys: SlotMap<ListenerKey, ListenerType>,
}

static KEY_REGISTRY: LazyLock<RwLock<KeyRegistry>> = LazyLock::new(|| {
    RwLock::new(KeyRegistry {
        keys: SlotMap::with_key(),
    })
});

/// Register a key in the global registry
fn register_key(listener_type: ListenerType) -> ListenerKey {
    KEY_REGISTRY.write().keys.insert(listener_type)
}

/// Remove a listener by its key
///
/// Returns `true` if the listener was found and removed.
pub fn remove_listener(key: ListenerKey) -> bool {
    let listener_type = KEY_REGISTRY.write().keys.remove(key);

    match listener_type {
        Some(ListenerType::Tick) => server::remove_tick(key),
        Some(ListenerType::MapStart) => server::remove_map_start(key),
        Some(ListenerType::MapEnd) => server::remove_map_end(key),
        Some(ListenerType::ClientConnect) => client::remove_client_connect(key),
        Some(ListenerType::ClientDisconnect) => client::remove_client_disconnect(key),
        Some(ListenerType::ClientPutInServer) => client::remove_client_put_in_server(key),
        Some(ListenerType::EntityCreated) => entity::remove_entity_created(key),
        Some(ListenerType::EntitySpawned) => entity::remove_entity_spawned(key),
        Some(ListenerType::EntityDeleted) => entity::remove_entity_deleted(key),
        None => false,
    }
}

// Re-export public API
pub use client::{on_client_connect, on_client_disconnect, on_client_put_in_server};
pub use entity::{on_entity_created, on_entity_deleted, on_entity_spawned};
pub use server::{on_map_end, on_map_start, on_tick};

// Re-export fire functions for FFI layer (used by plugin crate)
pub use client::{fire_client_connect, fire_client_disconnect, fire_client_put_in_server};
pub use entity::{fire_entity_created, fire_entity_deleted, fire_entity_spawned};
pub use server::{fire_map_end, fire_map_start, fire_tick};
