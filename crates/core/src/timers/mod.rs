//! Timer system for scheduling delayed and repeating callbacks
//!
//! Timers are processed every GameFrame tick and can be configured to:
//! - Fire once after a delay
//! - Repeat at a fixed interval
//! - Be automatically cleaned up on map change
//!
//! # Example
//!
//! ```ignore
//! use std::time::Duration;
//! use cs2rust_core::timers::{add_timer, add_repeating_timer, remove_timer, TimerFlags};
//!
//! // One-shot timer
//! let key = add_timer(Duration::from_secs(5), || {
//!     println!("5 seconds passed!");
//! });
//!
//! // Repeating timer
//! let key = add_repeating_timer(Duration::from_millis(100), || {
//!     println!("Tick!");
//! });
//!
//! // Cancel a timer
//! remove_timer(key);
//! ```

mod timer;

use std::sync::LazyLock;
use std::time::{Duration, Instant};

use parking_lot::RwLock;
use slotmap::SlotMap;

pub use timer::{TimerFlags, TimerKey};
use timer::Timer;

/// Timer registry
struct TimerRegistry {
    timers: SlotMap<TimerKey, Timer>,
}

static REGISTRY: LazyLock<RwLock<TimerRegistry>> = LazyLock::new(|| {
    RwLock::new(TimerRegistry {
        timers: SlotMap::with_key(),
    })
});

/// Add a one-shot timer that fires after the specified delay
///
/// # Arguments
/// * `delay` - How long to wait before firing
/// * `callback` - Function to call when the timer fires
///
/// # Returns
/// A key that can be used to cancel the timer via `remove_timer`
pub fn add_timer<F>(delay: Duration, callback: F) -> TimerKey
where
    F: FnMut() + Send + 'static,
{
    add_timer_with_flags(delay, TimerFlags::empty(), callback)
}

/// Add a repeating timer that fires at the specified interval
///
/// The timer will continue firing until cancelled via `remove_timer`.
///
/// # Arguments
/// * `interval` - Time between each execution
/// * `callback` - Function to call each time the timer fires
///
/// # Returns
/// A key that can be used to cancel the timer via `remove_timer`
pub fn add_repeating_timer<F>(interval: Duration, callback: F) -> TimerKey
where
    F: FnMut() + Send + 'static,
{
    add_timer_with_flags(interval, TimerFlags::REPEAT, callback)
}

/// Add a timer with custom flags
///
/// # Arguments
/// * `interval` - Delay (one-shot) or interval between executions (repeating)
/// * `flags` - Combination of `TimerFlags` to control behavior
/// * `callback` - Function to call when the timer fires
///
/// # Returns
/// A key that can be used to cancel the timer via `remove_timer`
///
/// # Example
///
/// ```ignore
/// use std::time::Duration;
/// use cs2rust_core::timers::{add_timer_with_flags, TimerFlags};
///
/// // Repeating timer that stops on map change
/// let key = add_timer_with_flags(
///     Duration::from_secs(1),
///     TimerFlags::REPEAT | TimerFlags::STOP_ON_MAPCHANGE,
///     || { /* ... */ }
/// );
/// ```
pub fn add_timer_with_flags<F>(interval: Duration, flags: TimerFlags, callback: F) -> TimerKey
where
    F: FnMut() + Send + 'static,
{
    let timer = Timer::new(interval, flags, callback);
    REGISTRY.write().timers.insert(timer)
}

/// Remove/cancel a timer
///
/// # Arguments
/// * `key` - The key returned from `add_timer`, `add_repeating_timer`, or `add_timer_with_flags`
///
/// # Returns
/// `true` if the timer was found and removed, `false` if not found
pub fn remove_timer(key: TimerKey) -> bool {
    REGISTRY.write().timers.remove(key).is_some()
}

/// Process all timers (called from GameFrame)
///
/// This checks all timers and fires any that are due. One-shot timers are
/// removed after firing, while repeating timers are rescheduled.
pub(crate) fn process() {
    let now = Instant::now();
    let mut to_remove = Vec::new();

    // First pass: execute callbacks and collect one-shots to remove
    {
        let registry = REGISTRY.read();
        for (key, timer) in registry.timers.iter() {
            if now >= timer.next_fire {
                // Execute the callback
                let mut callback = timer.callback.lock();
                (*callback)();

                // Mark one-shot timers for removal
                if !timer.flags.contains(TimerFlags::REPEAT) {
                    to_remove.push(key);
                }
            }
        }
    }

    // Second pass: remove one-shot timers and update next_fire for repeating
    if !to_remove.is_empty() {
        let mut registry = REGISTRY.write();
        for key in to_remove {
            registry.timers.remove(key);
        }
    }

    // Third pass: update next_fire for repeating timers that fired
    {
        let mut registry = REGISTRY.write();
        for (_, timer) in registry.timers.iter_mut() {
            if now >= timer.next_fire && timer.flags.contains(TimerFlags::REPEAT) {
                timer.next_fire = now + timer.interval;
            }
        }
    }
}

/// Remove all timers with the STOP_ON_MAPCHANGE flag
///
/// Called from OnMapEnd listener to clean up map-specific timers.
pub(crate) fn remove_mapchange_timers() {
    let mut registry = REGISTRY.write();
    let before = registry.timers.len();
    registry
        .timers
        .retain(|_, timer| !timer.flags.contains(TimerFlags::STOP_ON_MAPCHANGE));
    let removed = before - registry.timers.len();
    if removed > 0 {
        tracing::debug!("Removed {} timers on map change", removed);
    }
}
