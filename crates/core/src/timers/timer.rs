//! Timer struct and flags

use std::time::{Duration, Instant};

use bitflags::bitflags;
use parking_lot::Mutex;
use slotmap::new_key_type;

new_key_type! {
    /// Key for registered timers
    pub struct TimerKey;
}

bitflags! {
    /// Flags that control timer behavior
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct TimerFlags: u32 {
        /// Timer repeats at the specified interval until cancelled
        const REPEAT = 0x01;
        /// Timer is automatically removed when the map changes
        const STOP_ON_MAPCHANGE = 0x02;
    }
}

/// A scheduled timer that fires a callback after a delay
pub(crate) struct Timer {
    /// Time between executions (or delay for one-shot timers)
    pub interval: Duration,
    /// The callback to execute (wrapped in Mutex for FnMut support)
    pub callback: Mutex<Box<dyn FnMut() + Send + 'static>>,
    /// Behavior flags
    pub flags: TimerFlags,
    /// When this timer should next fire
    pub next_fire: Instant,
}

impl Timer {
    /// Create a new timer
    pub fn new<F>(interval: Duration, flags: TimerFlags, callback: F) -> Self
    where
        F: FnMut() + Send + 'static,
    {
        Self {
            interval,
            callback: Mutex::new(Box::new(callback)),
            flags,
            next_fire: Instant::now() + interval,
        }
    }
}
