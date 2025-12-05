//! GameFrame hook handler
//!
//! Called every server tick by SourceHook via C++ bridge.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::LazyLock;

use parking_lot::RwLock;
use slotmap::{new_key_type, SlotMap};

use crate::tasks;
use crate::timers;

new_key_type! {
    /// Key for registered GameFrame callbacks
    pub struct GameFrameKey;
}

/// Callback type for GameFrame listeners
pub type GameFrameCallback = Box<dyn Fn(bool, bool, bool) + Send + Sync>;

/// GameFrame callback registry
struct GameFrameRegistry {
    callbacks: SlotMap<GameFrameKey, GameFrameCallback>,
}

static REGISTRY: LazyLock<RwLock<GameFrameRegistry>> = LazyLock::new(|| {
    RwLock::new(GameFrameRegistry {
        callbacks: SlotMap::with_key(),
    })
});

/// Frame counter (increments every GameFrame call)
static FRAME_COUNT: AtomicU64 = AtomicU64::new(0);

/// Last tick's frame time for performance monitoring (nanoseconds)
static LAST_FRAME_TIME_NS: AtomicU64 = AtomicU64::new(0);

/// Register a callback to be called every GameFrame
///
/// # Arguments
/// * `callback` - Function called with (simulating, first_tick, last_tick)
///
/// # Returns
/// A key that can be used to unregister the callback
pub fn register_gameframe_callback<F>(callback: F) -> GameFrameKey
where
    F: Fn(bool, bool, bool) + Send + Sync + 'static,
{
    REGISTRY.write().callbacks.insert(Box::new(callback))
}

/// Unregister a GameFrame callback
///
/// # Returns
/// `true` if the callback was found and removed
pub fn unregister_gameframe_callback(key: GameFrameKey) -> bool {
    REGISTRY.write().callbacks.remove(key).is_some()
}

/// Get the current frame count
pub fn frame_count() -> u64 {
    FRAME_COUNT.load(Ordering::Relaxed)
}

/// Get the last frame processing time in nanoseconds
pub fn last_frame_time_ns() -> u64 {
    LAST_FRAME_TIME_NS.load(Ordering::Relaxed)
}

/// Called from C++ bridge every server tick
///
/// # Arguments
/// * `simulating` - True if the game is actively simulating (not paused)
/// * `first_tick` - True if this is the first tick of a frame
/// * `last_tick` - True if this is the last tick of a frame
pub fn on_game_frame(simulating: bool, first_tick: bool, last_tick: bool) {
    let start = std::time::Instant::now();

    // Increment frame counter
    FRAME_COUNT.fetch_add(1, Ordering::Relaxed);

    // Process queued tasks from other threads
    let tasks_processed = tasks::process_queued_tasks();
    if tasks_processed > 0 {
        tracing::trace!("Processed {} queued tasks", tasks_processed);
    }

    // Process timers
    timers::process();

    // Fire registered callbacks
    {
        let registry = REGISTRY.read();
        for (_, callback) in registry.callbacks.iter() {
            callback(simulating, first_tick, last_tick);
        }
    }

    // Record frame time for monitoring
    let elapsed = start.elapsed().as_nanos() as u64;
    LAST_FRAME_TIME_NS.store(elapsed, Ordering::Relaxed);

    // Warn if frame took too long (> 1ms)
    if elapsed > 1_000_000 {
        tracing::warn!(
            "GameFrame took {}ms (frame {})",
            elapsed / 1_000_000,
            FRAME_COUNT.load(Ordering::Relaxed)
        );
    }
}
