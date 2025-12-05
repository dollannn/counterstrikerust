//! Client connection listeners
//!
//! - OnClientConnect: Called when a client initiates connection
//! - OnClientDisconnect: Called when a client disconnects
//! - OnClientPutInServer: Called when a client fully enters the game

use std::sync::LazyLock;

use parking_lot::RwLock;
use slotmap::SlotMap;

use super::{register_key, ListenerKey, ListenerType};

// Callback types
/// Callback for client connect: (slot, name, ip)
pub type ClientConnectCallback = Box<dyn Fn(i32, &str, &str) + Send + Sync>;
/// Callback for client disconnect: (slot)
pub type ClientDisconnectCallback = Box<dyn Fn(i32) + Send + Sync>;
/// Callback for client put in server: (slot)
pub type ClientPutInServerCallback = Box<dyn Fn(i32) + Send + Sync>;

// Registries
struct ClientConnectRegistry {
    callbacks: SlotMap<ListenerKey, ClientConnectCallback>,
}

struct ClientDisconnectRegistry {
    callbacks: SlotMap<ListenerKey, ClientDisconnectCallback>,
}

struct ClientPutInServerRegistry {
    callbacks: SlotMap<ListenerKey, ClientPutInServerCallback>,
}

static CLIENT_CONNECT_REGISTRY: LazyLock<RwLock<ClientConnectRegistry>> = LazyLock::new(|| {
    RwLock::new(ClientConnectRegistry {
        callbacks: SlotMap::with_key(),
    })
});

static CLIENT_DISCONNECT_REGISTRY: LazyLock<RwLock<ClientDisconnectRegistry>> =
    LazyLock::new(|| {
        RwLock::new(ClientDisconnectRegistry {
            callbacks: SlotMap::with_key(),
        })
    });

static CLIENT_PUT_IN_SERVER_REGISTRY: LazyLock<RwLock<ClientPutInServerRegistry>> =
    LazyLock::new(|| {
        RwLock::new(ClientPutInServerRegistry {
            callbacks: SlotMap::with_key(),
        })
    });

// === OnClientConnect ===

/// Register a callback to be called when a client connects
///
/// # Arguments
/// The callback receives:
/// - `slot`: Player slot index (0-63)
/// - `name`: Player name
/// - `ip`: Player IP address
///
/// # Returns
/// A key that can be used to unregister the callback via `remove_listener`.
pub fn on_client_connect<F>(callback: F) -> ListenerKey
where
    F: Fn(i32, &str, &str) + Send + Sync + 'static,
{
    let key = register_key(ListenerType::ClientConnect);
    CLIENT_CONNECT_REGISTRY
        .write()
        .callbacks
        .insert(Box::new(callback));
    key
}

pub(super) fn remove_client_connect(key: ListenerKey) -> bool {
    CLIENT_CONNECT_REGISTRY
        .write()
        .callbacks
        .remove(key)
        .is_some()
}

/// Fire all client connect callbacks
pub fn fire_client_connect(slot: i32, name: &str, ip: &str) {
    tracing::debug!(
        "Firing OnClientConnect: slot={}, name={}, ip={}",
        slot,
        name,
        ip
    );
    let registry = CLIENT_CONNECT_REGISTRY.read();
    for (_, callback) in registry.callbacks.iter() {
        callback(slot, name, ip);
    }
}

// === OnClientDisconnect ===

/// Register a callback to be called when a client disconnects
///
/// # Arguments
/// The callback receives:
/// - `slot`: Player slot index (0-63)
///
/// # Returns
/// A key that can be used to unregister the callback via `remove_listener`.
pub fn on_client_disconnect<F>(callback: F) -> ListenerKey
where
    F: Fn(i32) + Send + Sync + 'static,
{
    let key = register_key(ListenerType::ClientDisconnect);
    CLIENT_DISCONNECT_REGISTRY
        .write()
        .callbacks
        .insert(Box::new(callback));
    key
}

pub(super) fn remove_client_disconnect(key: ListenerKey) -> bool {
    CLIENT_DISCONNECT_REGISTRY
        .write()
        .callbacks
        .remove(key)
        .is_some()
}

/// Fire all client disconnect callbacks
pub fn fire_client_disconnect(slot: i32) {
    tracing::debug!("Firing OnClientDisconnect: slot={}", slot);
    let registry = CLIENT_DISCONNECT_REGISTRY.read();
    for (_, callback) in registry.callbacks.iter() {
        callback(slot);
    }
}

// === OnClientPutInServer ===

/// Register a callback to be called when a client is put in server
///
/// This is called after the client has fully connected and entered the game.
///
/// # Arguments
/// The callback receives:
/// - `slot`: Player slot index (0-63)
///
/// # Returns
/// A key that can be used to unregister the callback via `remove_listener`.
pub fn on_client_put_in_server<F>(callback: F) -> ListenerKey
where
    F: Fn(i32) + Send + Sync + 'static,
{
    let key = register_key(ListenerType::ClientPutInServer);
    CLIENT_PUT_IN_SERVER_REGISTRY
        .write()
        .callbacks
        .insert(Box::new(callback));
    key
}

pub(super) fn remove_client_put_in_server(key: ListenerKey) -> bool {
    CLIENT_PUT_IN_SERVER_REGISTRY
        .write()
        .callbacks
        .remove(key)
        .is_some()
}

/// Fire all client put in server callbacks
pub fn fire_client_put_in_server(slot: i32) {
    tracing::debug!("Firing OnClientPutInServer: slot={}", slot);
    let registry = CLIENT_PUT_IN_SERVER_REGISTRY.read();
    for (_, callback) in registry.callbacks.iter() {
        callback(slot);
    }
}
