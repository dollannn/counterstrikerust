//! Event system types

use super::GameEventRef;

/// Result from an event handler determining how to proceed
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum HookResult {
    /// Continue processing, call other listeners and fire the event normally
    Continue = 0,

    /// Result changed (reserved for future use)
    Changed = 1,

    /// Block original event from firing, but continue calling other hooks
    Handled = 3,

    /// Block original event AND stop processing other hooks
    Stop = 4,
}

impl Default for HookResult {
    fn default() -> Self {
        Self::Continue
    }
}

/// Information passed to event handlers that can be modified
#[derive(Debug, Clone)]
pub struct EventInfo {
    /// If true, event will not be broadcast to clients
    pub dont_broadcast: bool,
}

impl EventInfo {
    /// Create new EventInfo with the given broadcast setting
    pub fn new(dont_broadcast: bool) -> Self {
        Self { dont_broadcast }
    }
}

/// Type alias for event callback functions
///
/// # Arguments
/// * `event` - Reference to the game event data
/// * `info` - Mutable event info (can modify dont_broadcast)
///
/// # Returns
/// `HookResult` indicating how to proceed
pub type EventCallback = Box<dyn Fn(&GameEventRef, &mut EventInfo) -> HookResult + Send + Sync>;
