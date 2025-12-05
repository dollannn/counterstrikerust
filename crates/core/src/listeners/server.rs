//! Server lifecycle listeners
//!
//! - OnTick: Called every server tick (simplified GameFrame)
//! - OnMapStart: Called when a map is loaded
//! - OnMapEnd: Called when a map is unloaded

use std::sync::LazyLock;

use parking_lot::RwLock;
use slotmap::SlotMap;

use super::{register_key, ListenerKey, ListenerType};

// Callback types
pub type TickCallback = Box<dyn Fn() + Send + Sync>;
pub type MapStartCallback = Box<dyn Fn(&str) + Send + Sync>;
pub type MapEndCallback = Box<dyn Fn() + Send + Sync>;

// Registries
struct TickRegistry {
    callbacks: SlotMap<ListenerKey, TickCallback>,
}

struct MapStartRegistry {
    callbacks: SlotMap<ListenerKey, MapStartCallback>,
}

struct MapEndRegistry {
    callbacks: SlotMap<ListenerKey, MapEndCallback>,
}

static TICK_REGISTRY: LazyLock<RwLock<TickRegistry>> = LazyLock::new(|| {
    RwLock::new(TickRegistry {
        callbacks: SlotMap::with_key(),
    })
});

static MAP_START_REGISTRY: LazyLock<RwLock<MapStartRegistry>> = LazyLock::new(|| {
    RwLock::new(MapStartRegistry {
        callbacks: SlotMap::with_key(),
    })
});

static MAP_END_REGISTRY: LazyLock<RwLock<MapEndRegistry>> = LazyLock::new(|| {
    RwLock::new(MapEndRegistry {
        callbacks: SlotMap::with_key(),
    })
});

// === OnTick ===

/// Register a callback to be called every server tick
///
/// This is a simplified version of GameFrame that takes no arguments.
/// For full GameFrame parameters (simulating, first_tick, last_tick),
/// use `hooks::register_gameframe_callback` instead.
///
/// # Returns
/// A key that can be used to unregister the callback via `remove_listener`.
pub fn on_tick<F>(callback: F) -> ListenerKey
where
    F: Fn() + Send + Sync + 'static,
{
    let key = register_key(ListenerType::Tick);
    TICK_REGISTRY.write().callbacks.insert(Box::new(callback));
    key
}

pub(super) fn remove_tick(key: ListenerKey) -> bool {
    TICK_REGISTRY.write().callbacks.remove(key).is_some()
}

/// Fire all tick callbacks (called from GameFrame)
pub fn fire_tick() {
    let registry = TICK_REGISTRY.read();
    for (_, callback) in registry.callbacks.iter() {
        callback();
    }
}

// === OnMapStart ===

/// Register a callback to be called when a map starts
///
/// # Arguments
/// The callback receives the map name (e.g., "de_dust2").
///
/// # Returns
/// A key that can be used to unregister the callback via `remove_listener`.
pub fn on_map_start<F>(callback: F) -> ListenerKey
where
    F: Fn(&str) + Send + Sync + 'static,
{
    let key = register_key(ListenerType::MapStart);
    MAP_START_REGISTRY
        .write()
        .callbacks
        .insert(Box::new(callback));
    key
}

pub(super) fn remove_map_start(key: ListenerKey) -> bool {
    MAP_START_REGISTRY.write().callbacks.remove(key).is_some()
}

/// Fire all map start callbacks
pub fn fire_map_start(map_name: &str) {
    tracing::info!("Firing OnMapStart: {}", map_name);
    let registry = MAP_START_REGISTRY.read();
    for (_, callback) in registry.callbacks.iter() {
        callback(map_name);
    }
}

// === OnMapEnd ===

/// Register a callback to be called when a map ends
///
/// # Returns
/// A key that can be used to unregister the callback via `remove_listener`.
pub fn on_map_end<F>(callback: F) -> ListenerKey
where
    F: Fn() + Send + Sync + 'static,
{
    let key = register_key(ListenerType::MapEnd);
    MAP_END_REGISTRY
        .write()
        .callbacks
        .insert(Box::new(callback));
    key
}

pub(super) fn remove_map_end(key: ListenerKey) -> bool {
    MAP_END_REGISTRY.write().callbacks.remove(key).is_some()
}

/// Fire all map end callbacks
pub fn fire_map_end() {
    tracing::info!("Firing OnMapEnd");

    // Clean up timers with STOP_ON_MAPCHANGE flag
    crate::timers::remove_mapchange_timers();

    let registry = MAP_END_REGISTRY.read();
    for (_, callback) in registry.callbacks.iter() {
        callback();
    }
}
