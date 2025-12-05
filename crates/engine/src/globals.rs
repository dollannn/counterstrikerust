//! Global engine interface storage
//!
//! Engine interfaces are acquired once during plugin load and stored here.
//! Access is thread-safe via OnceLock.

use std::ptr::NonNull;
use std::sync::OnceLock;
use std::thread::ThreadId;

use parking_lot::RwLock;

use cs2rust_sdk::{
    CGameEntitySystem, CSchemaSystem, ICvar, IEngineServiceMgr, IGameEventSystem,
    INetworkServerService, IServerGameDLL, ISmmAPI, ISource2GameEntities,
};

/// Global engine state containing all acquired interfaces
pub struct EngineGlobals {
    /// Metamod API pointer
    pub ismm: NonNull<ISmmAPI>,

    /// Server game DLL interface (required)
    pub server: NonNull<IServerGameDLL>,

    /// Schema system for field offset lookup (required)
    pub schema_system: NonNull<CSchemaSystem>,

    /// Game event system (required)
    pub game_event_system: NonNull<IGameEventSystem>,

    /// Console variable system (required for commands)
    pub cvar: NonNull<ICvar>,

    /// Network server service (optional)
    pub network_server_service: Option<NonNull<INetworkServerService>>,

    /// Engine service manager (optional)
    pub engine_service_mgr: Option<NonNull<IEngineServiceMgr>>,

    /// Game entities interface (optional)
    pub game_entities: Option<NonNull<ISource2GameEntities>>,

    /// Entity system - set later via StartupServer hook
    entity_system: RwLock<Option<NonNull<CGameEntitySystem>>>,

    /// Main game thread ID for thread safety checks
    pub main_thread_id: ThreadId,
}

// SAFETY: All pointers are to engine interfaces that live for the entire plugin lifetime.
// Access is synchronized via OnceLock for initialization and RwLock for entity_system.
unsafe impl Send for EngineGlobals {}
unsafe impl Sync for EngineGlobals {}

/// Global engine state storage
static ENGINE: OnceLock<EngineGlobals> = OnceLock::new();

/// Initialize engine globals
///
/// Called once during plugin load. Returns error if already initialized.
pub fn init_engine(globals: EngineGlobals) -> Result<(), &'static str> {
    ENGINE
        .set(globals)
        .map_err(|_| "Engine already initialized")
}

/// Get engine globals
///
/// # Panics
/// Panics if called before `init_engine`
pub fn engine() -> &'static EngineGlobals {
    ENGINE.get().expect("Engine not initialized")
}

/// Try to get engine globals without panicking
pub fn try_engine() -> Option<&'static EngineGlobals> {
    ENGINE.get()
}

/// Check if engine is initialized
pub fn is_engine_initialized() -> bool {
    ENGINE.get().is_some()
}

/// Check if current thread is the main game thread
pub fn is_main_thread() -> bool {
    ENGINE
        .get()
        .map(|g| std::thread::current().id() == g.main_thread_id)
        .unwrap_or(false)
}

impl EngineGlobals {
    /// Create new EngineGlobals
    ///
    /// # Arguments
    /// * `ismm` - Metamod API pointer
    /// * `server` - Server game DLL interface
    /// * `schema_system` - Schema system interface
    /// * `game_event_system` - Game event system interface
    /// * `cvar` - Console variable system interface
    pub fn new(
        ismm: NonNull<ISmmAPI>,
        server: NonNull<IServerGameDLL>,
        schema_system: NonNull<CSchemaSystem>,
        game_event_system: NonNull<IGameEventSystem>,
        cvar: NonNull<ICvar>,
    ) -> Self {
        Self {
            ismm,
            server,
            schema_system,
            game_event_system,
            cvar,
            network_server_service: None,
            engine_service_mgr: None,
            game_entities: None,
            entity_system: RwLock::new(None),
            main_thread_id: std::thread::current().id(),
        }
    }

    /// Get server interface pointer
    pub fn server_ptr(&self) -> *mut IServerGameDLL {
        self.server.as_ptr()
    }

    /// Get schema system pointer
    pub fn schema_system_ptr(&self) -> *mut CSchemaSystem {
        self.schema_system.as_ptr()
    }

    /// Get game event system pointer
    pub fn game_event_system_ptr(&self) -> *mut IGameEventSystem {
        self.game_event_system.as_ptr()
    }

    /// Get console variable system pointer
    pub fn cvar_ptr(&self) -> *mut ICvar {
        self.cvar.as_ptr()
    }

    /// Get entity system pointer (may be None before map load)
    pub fn entity_system_ptr(&self) -> Option<*mut CGameEntitySystem> {
        self.entity_system.read().map(|nn| nn.as_ptr())
    }

    /// Set entity system pointer
    ///
    /// Called from StartupServer hook when entity system becomes available
    pub fn set_entity_system(&self, ptr: *mut CGameEntitySystem) {
        if let Some(nn) = NonNull::new(ptr) {
            *self.entity_system.write() = Some(nn);
            tracing::info!("CGameEntitySystem set: {:p}", ptr);
        }
    }

    /// Clear entity system pointer
    ///
    /// Called when map unloads
    pub fn clear_entity_system(&self) {
        *self.entity_system.write() = None;
        tracing::debug!("CGameEntitySystem cleared");
    }

    /// Set optional network server service
    pub fn with_network_server_service(
        mut self,
        ptr: Option<NonNull<INetworkServerService>>,
    ) -> Self {
        self.network_server_service = ptr;
        self
    }

    /// Set optional engine service manager
    pub fn with_engine_service_mgr(mut self, ptr: Option<NonNull<IEngineServiceMgr>>) -> Self {
        self.engine_service_mgr = ptr;
        self
    }

    /// Set optional game entities interface
    pub fn with_game_entities(mut self, ptr: Option<NonNull<ISource2GameEntities>>) -> Self {
        self.game_entities = ptr;
        self
    }
}
