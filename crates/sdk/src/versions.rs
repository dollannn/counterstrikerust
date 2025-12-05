//! Interface version strings for CreateInterface
//!
//! These strings must match exactly what the engine exports.
//! Derived from HL2SDK-CS2 interface headers.

/// Source2Server interface (IServerGameDLL equivalent)
pub const SOURCE2_SERVER: &[u8] = b"Source2Server001\0";

/// Schema system for runtime type info
pub const SCHEMA_SYSTEM: &[u8] = b"SchemaSystem_001\0";

/// Game event system (Source 2 style)
pub const GAME_EVENT_SYSTEM: &[u8] = b"GameEventSystemServerV001\0";

/// Network server service
pub const NETWORK_SERVER_SERVICE: &[u8] = b"NetworkServerService_001\0";

/// Engine service manager
pub const ENGINE_SERVICE_MGR: &[u8] = b"EngineServiceMgr001\0";

/// Source 2 game entities
pub const SOURCE2_GAME_ENTITIES: &[u8] = b"Source2GameEntities001\0";

/// Source 2 server interface
pub const SOURCE2_SERVER_CONFIG: &[u8] = b"Source2ServerConfig001\0";

/// Console variable system
pub const CVAR: &[u8] = b"VEngineCvar007\0";

/// Game event manager (legacy S1-style events)
pub const GAME_EVENT_MANAGER: &[u8] = b"GameEventManager002\0";

/// Collected interface versions for iteration
pub const INTERFACE_VERSIONS: &[(&str, &[u8])] = &[
    ("Source2Server", SOURCE2_SERVER),
    ("SchemaSystem", SCHEMA_SYSTEM),
    ("GameEventSystem", GAME_EVENT_SYSTEM),
    ("GameEventManager", GAME_EVENT_MANAGER),
    ("NetworkServerService", NETWORK_SERVER_SERVICE),
    ("EngineServiceMgr", ENGINE_SERVICE_MGR),
    ("Source2GameEntities", SOURCE2_GAME_ENTITIES),
    ("Cvar", CVAR),
];
