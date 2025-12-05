#include "metamod_bridge.h"
#include <sourcehook.h>
#include <stdio.h>

CS2RustPlugin g_CS2RustPlugin;
CEntityListenerBridge g_EntityListener;

// Interface pointers
IServerGameDLL* g_pServerGameDLL = nullptr;
IServerGameClients* g_pServerGameClients = nullptr;

// Expose plugin to Metamod
PLUGIN_EXPOSE(CS2RustPlugin, g_CS2RustPlugin);

// SourceHook declarations for IServerGameDLL
SH_DECL_HOOK3_void(IServerGameDLL, GameFrame, SH_NOATTRIB, 0, bool, bool, bool);
SH_DECL_HOOK3_void(IServerGameDLL, ServerActivate, SH_NOATTRIB, 0, void*, int, int);
SH_DECL_HOOK0_void(IServerGameDLL, GameShutdown, SH_NOATTRIB, 0);

// SourceHook declarations for IServerGameClients
SH_DECL_HOOK6(IServerGameClients, ClientConnect, SH_NOATTRIB, 0, bool, CPlayerSlot, const char*, uint64_t, const char*, bool, CBufferString*);
SH_DECL_HOOK4_void(IServerGameClients, ClientPutInServer, SH_NOATTRIB, 0, CPlayerSlot, const char*, int, uint64_t);
SH_DECL_HOOK5_void(IServerGameClients, ClientDisconnect, SH_NOATTRIB, 0, CPlayerSlot, int, const char*, uint64_t, const char*);

bool CS2RustPlugin::Load(PluginId id, ISmmAPI* ismm, char* error, size_t maxlen, bool late)
{
    PLUGIN_SAVEVARS();

    ismm->ConPrintf("[CS2RUST] C++ bridge loaded, acquiring interfaces...\n");

    // Get factory functions for Rust to use
    CreateInterfaceFn serverFactory = (CreateInterfaceFn)ismm->GetServerFactory();
    CreateInterfaceFn engineFactory = (CreateInterfaceFn)ismm->GetEngineFactory();

    if (!serverFactory) {
        snprintf(error, maxlen, "Failed to get server factory");
        return false;
    }

    if (!engineFactory) {
        snprintf(error, maxlen, "Failed to get engine factory");
        return false;
    }

    // Get IServerGameDLL for hooking
    g_pServerGameDLL = (IServerGameDLL*)serverFactory("Source2Server001", nullptr);
    if (!g_pServerGameDLL) {
        snprintf(error, maxlen, "Failed to get IServerGameDLL interface");
        return false;
    }

    // Get IServerGameClients for client hooks
    g_pServerGameClients = (IServerGameClients*)serverFactory("Source2GameClients001", nullptr);
    if (!g_pServerGameClients) {
        ismm->ConPrintf("[CS2RUST] Warning: Failed to get IServerGameClients interface, client hooks disabled\n");
    }

    ismm->ConPrintf("[CS2RUST] Server factory: %p\n", serverFactory);
    ismm->ConPrintf("[CS2RUST] Engine factory: %p\n", engineFactory);
    ismm->ConPrintf("[CS2RUST] IServerGameDLL: %p\n", g_pServerGameDLL);
    ismm->ConPrintf("[CS2RUST] IServerGameClients: %p\n", g_pServerGameClients);

    // Call into Rust with factory functions
    if (!rust_plugin_load(
        id,
        ismm,
        (void*)serverFactory,
        (void*)engineFactory,
        error,
        maxlen,
        late
    )) {
        return false;
    }

    // Install IServerGameDLL hooks
    SH_ADD_HOOK_MEMFUNC(IServerGameDLL, GameFrame, g_pServerGameDLL, &g_CS2RustPlugin, &CS2RustPlugin::Hook_GameFrame, true);
    SH_ADD_HOOK_MEMFUNC(IServerGameDLL, ServerActivate, g_pServerGameDLL, &g_CS2RustPlugin, &CS2RustPlugin::Hook_ServerActivate, true);
    SH_ADD_HOOK_MEMFUNC(IServerGameDLL, GameShutdown, g_pServerGameDLL, &g_CS2RustPlugin, &CS2RustPlugin::Hook_GameShutdown, false);

    // Install IServerGameClients hooks (if available)
    if (g_pServerGameClients) {
        SH_ADD_HOOK_MEMFUNC(IServerGameClients, ClientConnect, g_pServerGameClients, &g_CS2RustPlugin, &CS2RustPlugin::Hook_ClientConnect, true);
        SH_ADD_HOOK_MEMFUNC(IServerGameClients, ClientPutInServer, g_pServerGameClients, &g_CS2RustPlugin, &CS2RustPlugin::Hook_ClientPutInServer, true);
        SH_ADD_HOOK_MEMFUNC(IServerGameClients, ClientDisconnect, g_pServerGameClients, &g_CS2RustPlugin, &CS2RustPlugin::Hook_ClientDisconnect, false);
        ismm->ConPrintf("[CS2RUST] Client hooks installed\n");
    }

    ismm->ConPrintf("[CS2RUST] All hooks installed\n");
    ismm->ConPrintf("[CS2RUST] Plugin loaded successfully!\n");

    return true;
}

bool CS2RustPlugin::Unload(char* error, size_t maxlen)
{
    // Remove IServerGameDLL hooks
    SH_REMOVE_HOOK_MEMFUNC(IServerGameDLL, GameFrame, g_pServerGameDLL, &g_CS2RustPlugin, &CS2RustPlugin::Hook_GameFrame, true);
    SH_REMOVE_HOOK_MEMFUNC(IServerGameDLL, ServerActivate, g_pServerGameDLL, &g_CS2RustPlugin, &CS2RustPlugin::Hook_ServerActivate, true);
    SH_REMOVE_HOOK_MEMFUNC(IServerGameDLL, GameShutdown, g_pServerGameDLL, &g_CS2RustPlugin, &CS2RustPlugin::Hook_GameShutdown, false);

    // Remove IServerGameClients hooks
    if (g_pServerGameClients) {
        SH_REMOVE_HOOK_MEMFUNC(IServerGameClients, ClientConnect, g_pServerGameClients, &g_CS2RustPlugin, &CS2RustPlugin::Hook_ClientConnect, true);
        SH_REMOVE_HOOK_MEMFUNC(IServerGameClients, ClientPutInServer, g_pServerGameClients, &g_CS2RustPlugin, &CS2RustPlugin::Hook_ClientPutInServer, true);
        SH_REMOVE_HOOK_MEMFUNC(IServerGameClients, ClientDisconnect, g_pServerGameClients, &g_CS2RustPlugin, &CS2RustPlugin::Hook_ClientDisconnect, false);
    }

    return rust_plugin_unload(error, maxlen);
}

bool CS2RustPlugin::Pause(char* error, size_t maxlen)
{
    return true;
}

bool CS2RustPlugin::Unpause(char* error, size_t maxlen)
{
    return true;
}

void CS2RustPlugin::AllPluginsLoaded()
{
    // Called when all Metamod plugins have loaded
}

const char* CS2RustPlugin::GetAuthor()
{
    return rust_get_author();
}

const char* CS2RustPlugin::GetName()
{
    return rust_get_name();
}

const char* CS2RustPlugin::GetDescription()
{
    return rust_get_description();
}

const char* CS2RustPlugin::GetURL()
{
    return rust_get_url();
}

const char* CS2RustPlugin::GetLicense()
{
    return rust_get_license();
}

const char* CS2RustPlugin::GetVersion()
{
    return rust_get_version();
}

const char* CS2RustPlugin::GetDate()
{
    return rust_get_date();
}

const char* CS2RustPlugin::GetLogTag()
{
    return rust_get_log_tag();
}

// === Hook callback implementations ===

void CS2RustPlugin::Hook_GameFrame(bool simulating, bool bFirstTick, bool bLastTick)
{
    rust_on_game_frame(simulating, bFirstTick, bLastTick);
    RETURN_META(MRES_IGNORED);
}

void CS2RustPlugin::Hook_ServerActivate(void* pEdictList, int edictCount, int clientMax)
{
    // Get map name from globals or use a placeholder
    // Note: In a real implementation, you'd get this from the engine
    rust_on_map_start("unknown_map");
    RETURN_META(MRES_IGNORED);
}

void CS2RustPlugin::Hook_GameShutdown()
{
    rust_on_map_end();
    RETURN_META(MRES_IGNORED);
}

bool CS2RustPlugin::Hook_ClientConnect(CPlayerSlot slot, const char* pszName, uint64_t xuid,
                                        const char* pszNetworkID, bool unk1, CBufferString* pRejectReason)
{
    // pszNetworkID contains IP address info
    rust_on_client_connect(slot.Get(), pszName ? pszName : "", pszNetworkID ? pszNetworkID : "");
    RETURN_META_VALUE(MRES_IGNORED, true);
}

void CS2RustPlugin::Hook_ClientPutInServer(CPlayerSlot slot, const char* pszName, int type, uint64_t xuid)
{
    rust_on_client_put_in_server(slot.Get());
    RETURN_META(MRES_IGNORED);
}

void CS2RustPlugin::Hook_ClientDisconnect(CPlayerSlot slot, int reason, const char* pszName,
                                           uint64_t xuid, const char* pszNetworkID)
{
    rust_on_client_disconnect(slot.Get());
    RETURN_META(MRES_IGNORED);
}

// === Entity listener bridge implementations ===

void CEntityListenerBridge::OnEntityCreated(CEntityInstance* pEntity)
{
    rust_on_entity_created(pEntity);
}

void CEntityListenerBridge::OnEntitySpawned(CEntityInstance* pEntity)
{
    rust_on_entity_spawned(pEntity);
}

void CEntityListenerBridge::OnEntityDeleted(CEntityInstance* pEntity)
{
    rust_on_entity_deleted(pEntity);
}
