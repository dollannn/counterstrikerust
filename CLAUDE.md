# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

CS2Rust is a Counter-Strike 2 modding framework written in Rust. It runs as a Metamod plugin, providing type-safe access to Source 2 engine internals through Rust.

## Build Commands

```bash
just build           # Development build (native)
just steamrt         # Release build for SteamRT servers (Docker)
just steamrt-native  # Release build using local SteamRT3 (no Docker)
just test            # Run tests
just lint            # Run clippy lints
just fmt             # Format code
just package         # Build and create deployment zip (Docker)
just package-native  # Build and create deployment zip (native SteamRT)
just dev             # Build, deploy, and run local CS2 server
just deploy-local    # Deploy to local server (no build/start)
```

SteamRT builds are required for production because CS2 servers run on SteamRT with specific glibc versions.

## SteamRT Setup (Linux)

For `steamrt-native` builds without Docker, install SteamRT3 via steamcmd:

```bash
# In steamcmd
force_install_dir ~/steamrt
login anonymous
app_update 1628350 validate
```

Override the default path with `STEAMRT_PATH` environment variable:
```bash
STEAMRT_PATH=/path/to/steamrt just steamrt-native
```

## Local Development

For `just dev` workflow, install CS2 dedicated server to `./server`:

```bash
# In steamcmd
force_install_dir /path/to/CounterStrikeRust/server
login anonymous
app_update 730 validate
```

Override the default path with `CS2_SERVER_PATH` environment variable:
```bash
CS2_SERVER_PATH=/path/to/server just dev
```

## Architecture

### Crate Structure

```
crates/
├── plugin/   # FFI layer - cdylib that Metamod loads, C++ bridge exports
├── core/     # Framework logic - hooks, schema system, entities, tasks
├── engine/   # Interface loading and global storage (EngineGlobals)
├── sdk/      # Pure type definitions for Source 2 interfaces (no deps)
└── macros/   # Proc macros (#[derive(SchemaClass)])
```

Dependency flow: `plugin → core → engine → sdk`, `core → macros`

### Key Systems

**Interface Loading** (`engine/src/loader.rs`, `engine/src/globals.rs`):
- Acquires Source 2 interfaces via `CreateInterface` during plugin load
- Stores in `EngineGlobals` singleton accessed via `engine()`
- Required interfaces: `IServerGameDLL`, `CSchemaSystem`, `IGameEventSystem`

**Schema System** (`core/src/schema/`):
- Runtime introspection for Source 2 entity field offsets
- Queries `CSchemaSystem` and caches results in `DashMap`
- `SchemaField<T>` provides per-field offset caching with `OnceLock`

**Entity Wrappers** (`core/src/entities/`):
- `#[derive(SchemaClass)]` macro generates type-safe getters/setters
- Fields marked `#[schema(networked)]` auto-call `NetworkStateChanged`
- PhantomData fields carry type info; actual data lives in native memory at `ptr`

**Hook System** (`core/src/hooks/`):
- Inline hooks: function detours using iced-x86 (works on stable Rust)
- VTable hooks: virtual function pointer replacement
- Mid-function hooks: arbitrary address with register context
- GameFrame callbacks registered via `register_gameframe_callback`

### FFI Boundary

`plugin/src/ffi/exports.rs` defines `#[no_mangle] extern "C"` functions called by C++:
- `rust_plugin_load` - Plugin initialization
- `rust_plugin_unload` - Plugin shutdown
- `rust_on_game_frame` - Called every server tick via SourceHook

### Deployment

The plugin is deployed as `addons/cs2rust/bin/linuxsteamrt64/cs2rust.so` with a VDF file for Metamod registration.

## Testing

```bash
cargo test                              # All tests
cargo test --package cs2rust-core       # Single crate
cargo test test_player_pawn_constants   # Single test
```

## Reference Material

The `research/` directory contains reference implementations:
- `CounterStrikeSharp/` - C# CS2 modding framework
- `swiftly/`, `swiftlys2/` - C++ plugin frameworks
- `s2sdk/` - Source 2 SDK reference

The `third_party/` directory contains:
- `hl2sdk-cs2/` - Half-Life 2 SDK for CS2
- `metamod-source/` - Metamod:Source
