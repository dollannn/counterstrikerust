//! Engine interface loading via CreateInterface pattern

use std::ffi::CStr;
use std::ptr::NonNull;

use cs2rust_sdk::{
    versions, CSchemaSystem, CreateInterfaceFn, ICvar, IEngineServiceMgr, IGameEventSystem,
    INetworkServerService, IServerGameDLL, ISmmAPI, ISource2GameEntities,
};

use crate::error::InterfaceError;
use crate::globals::EngineGlobals;

/// Wrapper around a CreateInterface factory function
pub struct InterfaceFactory {
    factory: CreateInterfaceFn,
    name: &'static str,
}

impl InterfaceFactory {
    /// Create a new factory wrapper
    ///
    /// # Arguments
    /// * `factory` - The CreateInterface function pointer
    /// * `name` - Human-readable name for error messages (e.g., "server", "engine")
    pub fn new(factory: CreateInterfaceFn, name: &'static str) -> Self {
        Self { factory, name }
    }

    /// Get an interface by version string
    ///
    /// # Arguments
    /// * `version` - Null-terminated version string (e.g., b"Source2Server001\0")
    ///
    /// # Safety
    /// The returned pointer is only valid if T matches the actual interface type
    pub unsafe fn get<T>(&self, version: &[u8]) -> Result<NonNull<T>, InterfaceError> {
        let version_str = CStr::from_bytes_with_nul(version).map_err(|_| {
            InterfaceError::InvalidVersionString(String::from_utf8_lossy(version).into_owned())
        })?;

        let mut ret_code: i32 = 0;
        let ptr = (self.factory)(version_str.as_ptr(), &mut ret_code);

        if ptr.is_null() {
            let version_display = version_str.to_string_lossy();
            Err(InterfaceError::NullPointer(format!(
                "{} from {}",
                version_display, self.name
            )))
        } else {
            Ok(NonNull::new_unchecked(ptr as *mut T))
        }
    }

    /// Try to get an interface, returning None on failure instead of error
    ///
    /// # Safety
    /// Same as `get`
    pub unsafe fn try_get<T>(&self, version: &[u8]) -> Option<NonNull<T>> {
        self.get(version).ok()
    }
}

/// Load all required engine interfaces
///
/// Called during plugin Load with factory functions from Metamod.
///
/// # Arguments
/// * `ismm` - Metamod API pointer
/// * `server_factory` - CreateInterface from server module
/// * `engine_factory` - CreateInterface from engine module
///
/// # Safety
/// All pointers must be valid. Factory functions must be callable.
#[tracing::instrument(skip_all)]
pub unsafe fn load_interfaces(
    ismm: *mut ISmmAPI,
    server_factory: CreateInterfaceFn,
    engine_factory: CreateInterfaceFn,
) -> Result<EngineGlobals, InterfaceError> {
    let server = InterfaceFactory::new(server_factory, "server");
    let engine = InterfaceFactory::new(engine_factory, "engine");

    // Required interfaces - fail if any are missing
    let server_dll = server.get::<IServerGameDLL>(versions::SOURCE2_SERVER)?;
    tracing::info!("IServerGameDLL: {:p}", server_dll.as_ptr());

    let schema_system = engine.get::<CSchemaSystem>(versions::SCHEMA_SYSTEM)?;
    tracing::info!("CSchemaSystem: {:p}", schema_system.as_ptr());

    let game_event_system = engine.get::<IGameEventSystem>(versions::GAME_EVENT_SYSTEM)?;
    tracing::info!("IGameEventSystem: {:p}", game_event_system.as_ptr());

    let cvar = engine.get::<ICvar>(versions::CVAR)?;
    tracing::info!("ICvar: {:p}", cvar.as_ptr());

    // Optional interfaces - log but don't fail
    let network_server_service =
        engine.try_get::<INetworkServerService>(versions::NETWORK_SERVER_SERVICE);
    if let Some(ref nss) = network_server_service {
        tracing::info!("INetworkServerService: {:p}", nss.as_ptr());
    } else {
        tracing::debug!("INetworkServerService: not available");
    }

    let engine_service_mgr = engine.try_get::<IEngineServiceMgr>(versions::ENGINE_SERVICE_MGR);
    if let Some(ref esm) = engine_service_mgr {
        tracing::info!("IEngineServiceMgr: {:p}", esm.as_ptr());
    } else {
        tracing::debug!("IEngineServiceMgr: not available");
    }

    let game_entities = server.try_get::<ISource2GameEntities>(versions::SOURCE2_GAME_ENTITIES);
    if let Some(ref ge) = game_entities {
        tracing::info!("ISource2GameEntities: {:p}", ge.as_ptr());
    } else {
        tracing::debug!("ISource2GameEntities: not available");
    }

    // Build globals struct
    let ismm_nn =
        NonNull::new(ismm).ok_or_else(|| InterfaceError::NullPointer("ISmmAPI".into()))?;

    let globals = EngineGlobals::new(ismm_nn, server_dll, schema_system, game_event_system, cvar)
        .with_network_server_service(network_server_service)
        .with_engine_service_mgr(engine_service_mgr)
        .with_game_entities(game_entities);

    Ok(globals)
}
