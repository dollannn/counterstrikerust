//! C-compatible exports called by the C++ bridge

use std::ffi::{c_char, c_int, c_void};

use tracing::instrument;

use cs2rust_core::hooks;
use cs2rust_engine::{init_engine, load_interfaces};
use cs2rust_sdk::{CreateInterfaceFn, ISmmAPI};

// Plugin metadata - static strings with null terminators for C compatibility
static AUTHOR: &[u8] = b"dollan\0";
static NAME: &[u8] = b"CounterStrikeRust\0";
static DESCRIPTION: &[u8] = b"Counter-Strike 2 Rust Modding Framework\0";
static URL: &[u8] = b"https://github.com/dollannn/CounterStrikeRust\0";
static LICENSE: &[u8] = b"MIT\0";
static VERSION: &[u8] = b"0.1.0\0";
static DATE: &[u8] = b"2024-12-01\0";
static LOG_TAG: &[u8] = b"CS2RUST\0";

/// Called when the plugin is loaded by Metamod
///
/// # Safety
/// - `ismm` must be a valid ISmmAPI pointer
/// - `server_factory` and `engine_factory` must be valid CreateInterface functions
/// - `error` must be a valid pointer to a buffer of at least `maxlen` bytes, or null
#[no_mangle]
#[instrument(skip_all)]
pub unsafe extern "C" fn rust_plugin_load(
    _plugin_id: c_int,
    ismm: *mut c_void,
    server_factory: *mut c_void,
    engine_factory: *mut c_void,
    error: *mut c_char,
    maxlen: usize,
    _late: bool,
) -> bool {
    // Initialize tracing subscriber
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init();

    tracing::info!("CS2Rust loading...");

    // Validate factory pointers
    if server_factory.is_null() {
        write_error(error, maxlen, "Server factory is null");
        return false;
    }
    if engine_factory.is_null() {
        write_error(error, maxlen, "Engine factory is null");
        return false;
    }

    // Cast factory function pointers
    let server_factory: CreateInterfaceFn = std::mem::transmute(server_factory);
    let engine_factory: CreateInterfaceFn = std::mem::transmute(engine_factory);

    // Load engine interfaces
    let globals = match load_interfaces(ismm as *mut ISmmAPI, server_factory, engine_factory) {
        Ok(g) => g,
        Err(e) => {
            tracing::error!("Failed to load interfaces: {}", e);
            write_error(error, maxlen, &format!("Interface error: {}", e));
            return false;
        }
    };

    // Store in global state
    if let Err(e) = init_engine(globals) {
        tracing::error!("Failed to init engine: {}", e);
        write_error(error, maxlen, e);
        return false;
    }

    tracing::info!("CS2 Rust Plugin loaded successfully!");
    tracing::info!("Main thread ID: {:?}", std::thread::current().id());

    true
}

/// Called when the plugin is unloaded by Metamod
///
/// # Safety
/// - `error` must be a valid pointer to a buffer of at least `maxlen` bytes, or null
#[no_mangle]
#[instrument(skip_all)]
pub unsafe extern "C" fn rust_plugin_unload(error: *mut c_char, _maxlen: usize) -> bool {
    tracing::info!("CS2Rust unloading...");

    match std::panic::catch_unwind(crate::shutdown) {
        Ok(()) => true,
        Err(_) => {
            write_error(error, 256, "Panic during shutdown");
            false
        }
    }
}

// Metadata exports - these return static strings for Metamod to display

#[no_mangle]
pub extern "C" fn rust_get_author() -> *const c_char {
    AUTHOR.as_ptr() as *const c_char
}

#[no_mangle]
pub extern "C" fn rust_get_name() -> *const c_char {
    NAME.as_ptr() as *const c_char
}

#[no_mangle]
pub extern "C" fn rust_get_description() -> *const c_char {
    DESCRIPTION.as_ptr() as *const c_char
}

#[no_mangle]
pub extern "C" fn rust_get_url() -> *const c_char {
    URL.as_ptr() as *const c_char
}

#[no_mangle]
pub extern "C" fn rust_get_license() -> *const c_char {
    LICENSE.as_ptr() as *const c_char
}

#[no_mangle]
pub extern "C" fn rust_get_version() -> *const c_char {
    VERSION.as_ptr() as *const c_char
}

#[no_mangle]
pub extern "C" fn rust_get_date() -> *const c_char {
    DATE.as_ptr() as *const c_char
}

#[no_mangle]
pub extern "C" fn rust_get_log_tag() -> *const c_char {
    LOG_TAG.as_ptr() as *const c_char
}

/// Called from C++ SourceHook every server tick
#[no_mangle]
#[instrument(skip_all)]
pub extern "C" fn rust_on_game_frame(simulating: bool, first_tick: bool, last_tick: bool) {
    hooks::on_game_frame(simulating, first_tick, last_tick);
    // Also fire OnTick listeners
    cs2rust_core::listeners::fire_tick();
}

// === Listener FFI exports ===

/// Called from C++ when a map starts (ServerActivate hook)
///
/// # Safety
/// - `map_name` must be a valid null-terminated C string
#[no_mangle]
pub unsafe extern "C" fn rust_on_map_start(map_name: *const c_char) {
    if map_name.is_null() {
        return;
    }
    let name = std::ffi::CStr::from_ptr(map_name).to_string_lossy();
    cs2rust_core::listeners::fire_map_start(&name);
}

/// Called from C++ when a map ends (GameShutdown hook)
#[no_mangle]
pub extern "C" fn rust_on_map_end() {
    cs2rust_core::listeners::fire_map_end();
}

/// Called from C++ when a client connects (ClientConnect hook)
///
/// # Safety
/// - `name` and `ip` must be valid null-terminated C strings or null
#[no_mangle]
pub unsafe extern "C" fn rust_on_client_connect(
    slot: c_int,
    name: *const c_char,
    ip: *const c_char,
) {
    let name_str = if name.is_null() {
        ""
    } else {
        std::ffi::CStr::from_ptr(name).to_str().unwrap_or("")
    };
    let ip_str = if ip.is_null() {
        ""
    } else {
        std::ffi::CStr::from_ptr(ip).to_str().unwrap_or("")
    };
    cs2rust_core::listeners::fire_client_connect(slot, name_str, ip_str);
}

/// Called from C++ when a client disconnects (ClientDisconnect hook)
#[no_mangle]
pub extern "C" fn rust_on_client_disconnect(slot: c_int) {
    cs2rust_core::listeners::fire_client_disconnect(slot);
}

/// Called from C++ when a client is put in server (ClientPutInServer hook)
#[no_mangle]
pub extern "C" fn rust_on_client_put_in_server(slot: c_int) {
    cs2rust_core::listeners::fire_client_put_in_server(slot);
}

/// Called from C++ when an entity is created (IEntityListener::OnEntityCreated)
///
/// # Safety
/// - `entity` must be a valid pointer to a CEntityInstance or null
#[no_mangle]
pub unsafe extern "C" fn rust_on_entity_created(entity: *mut c_void) {
    if !entity.is_null() {
        cs2rust_core::listeners::fire_entity_created(entity);
    }
}

/// Called from C++ when an entity is spawned (IEntityListener::OnEntitySpawned)
///
/// # Safety
/// - `entity` must be a valid pointer to a CEntityInstance or null
#[no_mangle]
pub unsafe extern "C" fn rust_on_entity_spawned(entity: *mut c_void) {
    if !entity.is_null() {
        cs2rust_core::listeners::fire_entity_spawned(entity);
    }
}

/// Called from C++ when an entity is deleted (IEntityListener::OnEntityDeleted)
///
/// # Safety
/// - `entity` must be a valid pointer to a CEntityInstance or null
#[no_mangle]
pub unsafe extern "C" fn rust_on_entity_deleted(entity: *mut c_void) {
    if !entity.is_null() {
        cs2rust_core::listeners::fire_entity_deleted(entity);
    }
}

/// Helper to write an error message to a C buffer
///
/// # Safety
/// - `error` must be a valid pointer or null
/// - `maxlen` must accurately reflect the buffer size
unsafe fn write_error(error: *mut c_char, maxlen: usize, msg: &str) {
    if !error.is_null() && maxlen > 0 {
        let bytes = msg.as_bytes();
        let len = bytes.len().min(maxlen - 1);
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), error as *mut u8, len);
        *error.add(len) = 0;
    }
}
