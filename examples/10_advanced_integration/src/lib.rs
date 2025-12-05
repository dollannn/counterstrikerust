//! # Advanced Integration Example
//!
//! A comprehensive example demonstrating integration of multiple framework systems.
//!
//! ## Features Demonstrated
//!
//! ### Threading & Task Queue
//! - `queue_task` - Execute code on main thread from background threads
//! - Background thread work with safe callback to game thread
//!
//! ### GameFrame System
//! - `register_gameframe_callback` - Per-tick callbacks
//! - `frame_count()` - Access current frame number
//! - Frame timing and performance monitoring
//!
//! ### Entity Lifecycle
//! - `on_entity_created` - Entity memory allocated
//! - `on_entity_spawned` - Entity activated in world
//! - `on_entity_deleted` - Entity being removed
//! - `EntityRef` pattern matching
//!
//! ### Multi-System Integration
//! - Configuration via `PluginConfig`
//! - Runtime settings via `FakeConVar`
//! - Commands for control
//! - Event handling
//!
//! ## Commands
//!
//! - `!async_test` - Demonstrate async task queue
//! - `!frameinfo` - Show current frame information
//! - `!entitystats` - Show entity lifecycle stats

pub mod config;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::LazyLock;
use std::thread;
use std::time::Duration;

use cs2rust_core::{
    queue_task, register_gameframe_callback, frame_count,
    on_entity_created, on_entity_spawned, on_entity_deleted,
    register_command, CommandResult,
    FakeConVar, PluginConfig, HookResult,
    EntityRef,
};
use cs2rust_core::entities::get_player_controller_by_userid;
use cs2rust_core::events::typed::{register_typed_event, EventPlayerSpawn};
use cs2rust_core::player_has_permission;

pub use config::AdvancedConfig;

// =============================================================================
// Statistics Tracking
// =============================================================================

/// Count of entities created this session
static ENTITIES_CREATED: AtomicU64 = AtomicU64::new(0);

/// Count of entities spawned this session
static ENTITIES_SPAWNED: AtomicU64 = AtomicU64::new(0);

/// Count of entities deleted this session
static ENTITIES_DELETED: AtomicU64 = AtomicU64::new(0);

/// Count of player pawns spawned
static PLAYER_SPAWNS: AtomicU64 = AtomicU64::new(0);

/// Count of weapons spawned
static WEAPON_SPAWNS: AtomicU64 = AtomicU64::new(0);

// =============================================================================
// FakeConVars
// =============================================================================

/// Enable debug mode
pub static ADV_DEBUG: LazyLock<FakeConVar<bool>> = LazyLock::new(|| {
    FakeConVar::new("adv_debug", false, "Enable advanced debug output")
});

/// Entity logging verbosity
pub static ADV_ENTITY_LOG: LazyLock<FakeConVar<bool>> = LazyLock::new(|| {
    FakeConVar::new("adv_entity_log", false, "Log entity lifecycle events")
});

// =============================================================================
// Configuration
// =============================================================================

/// Loaded configuration
static CONFIG: LazyLock<AdvancedConfig> = LazyLock::new(|| {
    AdvancedConfig::load().unwrap_or_else(|e| {
        tracing::warn!("Failed to load config: {}, using defaults", e);
        AdvancedConfig::default()
    })
});

// =============================================================================
// Public API
// =============================================================================

/// Initialize the Advanced Integration plugin.
pub fn init() {
    let config = &*CONFIG;

    tracing::info!("Advanced Integration initializing...");
    tracing::info!("  Entity logging: {}", config.log_entity_lifecycle);
    tracing::info!("  Frame debugging: {}", config.debug_frames);

    // Initialize FakeConVars with config values
    ADV_DEBUG.set(config.debug_frames);
    ADV_ENTITY_LOG.set(config.log_entity_lifecycle);

    // Force registration
    let _ = ADV_DEBUG.get();
    let _ = ADV_ENTITY_LOG.get();

    // Register all subsystems
    register_gameframe_handler();
    register_entity_handlers();
    register_player_spawn_handler();
    register_commands();

    tracing::info!("Advanced Integration initialized!");
    tracing::info!("Commands: !async_test, !frameinfo, !entitystats");
}

/// Get entity statistics
pub fn get_entity_stats() -> EntityStats {
    EntityStats {
        created: ENTITIES_CREATED.load(Ordering::Relaxed),
        spawned: ENTITIES_SPAWNED.load(Ordering::Relaxed),
        deleted: ENTITIES_DELETED.load(Ordering::Relaxed),
        player_spawns: PLAYER_SPAWNS.load(Ordering::Relaxed),
        weapon_spawns: WEAPON_SPAWNS.load(Ordering::Relaxed),
    }
}

/// Entity lifecycle statistics
#[derive(Debug, Clone)]
pub struct EntityStats {
    pub created: u64,
    pub spawned: u64,
    pub deleted: u64,
    pub player_spawns: u64,
    pub weapon_spawns: u64,
}

// =============================================================================
// GameFrame Handler
// =============================================================================

fn register_gameframe_handler() {
    let config = &*CONFIG;
    let interval = config.frame_debug_interval;

    register_gameframe_callback(move |simulating, first_tick, last_tick| {
        // Only process when debugging is enabled
        if !ADV_DEBUG.get() {
            return;
        }

        let frame = frame_count();

        // Log at configured interval
        if frame % interval == 0 {
            tracing::debug!(
                "Frame {}: simulating={}, first={}, last={}",
                frame,
                simulating,
                first_tick,
                last_tick
            );
        }
    });
}

// =============================================================================
// Entity Lifecycle Handlers
// =============================================================================

fn register_entity_handlers() {
    // Entity created (memory allocated, may not be fully initialized)
    on_entity_created(|entity_ref| {
        ENTITIES_CREATED.fetch_add(1, Ordering::Relaxed);

        if ADV_ENTITY_LOG.get() {
            if let EntityRef::PlayerPawn(_) = entity_ref {
                tracing::debug!("Player pawn created");
            }
        }
    });

    // Entity spawned (fully initialized and active)
    on_entity_spawned(|entity_ref| {
        ENTITIES_SPAWNED.fetch_add(1, Ordering::Relaxed);

        match &entity_ref {
            EntityRef::PlayerPawn(pawn) => {
                PLAYER_SPAWNS.fetch_add(1, Ordering::Relaxed);

                if ADV_ENTITY_LOG.get() {
                    tracing::info!(
                        "Player pawn spawned: HP={}, Armor={}, Team={}",
                        pawn.health(),
                        pawn.armor(),
                        pawn.team()
                    );
                }
            }
            EntityRef::PlayerController(controller) => {
                if ADV_ENTITY_LOG.get() {
                    tracing::info!(
                        "Player controller spawned: {}",
                        controller.name_string()
                    );
                }
            }
            EntityRef::Unknown { classname, .. } => {
                if entity_ref.is_weapon() {
                    WEAPON_SPAWNS.fetch_add(1, Ordering::Relaxed);

                    if ADV_ENTITY_LOG.get() {
                        tracing::debug!("Weapon spawned: {}", classname);
                    }
                }
            }
            _ => {}
        }
    });

    // Entity deleted (being removed from game)
    on_entity_deleted(|entity_ref| {
        ENTITIES_DELETED.fetch_add(1, Ordering::Relaxed);

        if ADV_ENTITY_LOG.get() {
            tracing::debug!("Entity deleted: {}", entity_ref.classname());
        }
    });
}

// =============================================================================
// Player Spawn Handler (VIP Equipment)
// =============================================================================

fn register_player_spawn_handler() {
    let config = &*CONFIG;
    let vip_armor = config.vip_spawn_armor;

    register_typed_event::<EventPlayerSpawn, _>(true, move |event, _info| {
        // Get the player who spawned
        let Some(controller) = get_player_controller_by_userid(event.userid) else {
            return HookResult::Continue;
        };

        // Check if player has VIP permission
        if player_has_permission(&controller, "@vip/spawn") {
            if let Some(mut pawn) = controller.pawn() {
                // Give VIP players bonus armor on spawn
                pawn.set_armor(vip_armor);

                if ADV_DEBUG.get() {
                    tracing::info!(
                        "VIP {} spawned with {} armor",
                        controller.name_string(),
                        vip_armor
                    );
                }
            }
        }

        HookResult::Continue
    });
}

// =============================================================================
// Commands
// =============================================================================

fn register_commands() {
    register_async_test_command();
    register_frameinfo_command();
    register_entitystats_command();
}

/// Command to demonstrate async task queuing
fn register_async_test_command() {
    register_command(
        "csr_async_test",
        "Demonstrate async task queue",
        |_player, info| {
            info.reply("Starting background work...");
            info.reply("(Result will appear in console after 2 seconds)");

            // Spawn a background thread to do "work"
            thread::spawn(|| {
                // Simulate background work (e.g., database query, HTTP request)
                tracing::info!("Background thread starting work...");
                thread::sleep(Duration::from_secs(2));
                tracing::info!("Background thread completed work!");

                // Queue result back to main thread
                // This is CRITICAL for thread safety - game state must only
                // be modified on the main thread during GameFrame
                if queue_task(|| {
                    tracing::info!("=== ASYNC RESULT ===");
                    tracing::info!("Background work completed successfully!");
                    tracing::info!("This message was queued from a background thread");
                    tracing::info!("and executed safely on the main game thread.");
                }).is_err() {
                    tracing::error!("Failed to queue task");
                }
            });

            CommandResult::Handled
        },
    );
}

/// Command to show frame information
fn register_frameinfo_command() {
    register_command(
        "csr_frameinfo",
        "Show current frame information",
        |_player, info| {
            let frame = frame_count();

            info.reply("=== Frame Info ===");
            info.reply(&format!("Current frame: {}", frame));
            info.reply(&format!("Debug mode: {}", ADV_DEBUG.get()));

            // Calculate approximate time (assuming 64 tick)
            let seconds = frame / 64;
            let minutes = seconds / 60;
            info.reply(&format!(
                "Approx uptime: {}m {}s ({} frames)",
                minutes,
                seconds % 60,
                frame
            ));

            CommandResult::Handled
        },
    );
}

/// Command to show entity statistics
fn register_entitystats_command() {
    register_command(
        "csr_entitystats",
        "Show entity lifecycle statistics",
        |_player, info| {
            let stats = get_entity_stats();

            info.reply("=== Entity Statistics ===");
            info.reply(&format!("Created: {}", stats.created));
            info.reply(&format!("Spawned: {}", stats.spawned));
            info.reply(&format!("Deleted: {}", stats.deleted));
            info.reply("");
            info.reply("By type:");
            info.reply(&format!("  Player spawns: {}", stats.player_spawns));
            info.reply(&format!("  Weapon spawns: {}", stats.weapon_spawns));

            // Calculate churn
            let net = stats.created as i64 - stats.deleted as i64;
            info.reply("");
            info.reply(&format!("Net entities: {:+}", net));

            CommandResult::Handled
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_stats() {
        let stats = EntityStats {
            created: 100,
            spawned: 90,
            deleted: 50,
            player_spawns: 10,
            weapon_spawns: 30,
        };

        assert_eq!(stats.created, 100);
        assert_eq!(stats.spawned, 90);
    }

    #[test]
    fn test_config_defaults() {
        let config = AdvancedConfig::default();
        assert!(!config.log_entity_lifecycle);
        assert!(!config.debug_frames);
        assert_eq!(config.frame_debug_interval, 1000);
        assert_eq!(config.vip_spawn_armor, 100);
    }
}
