//! # Timed Announcements Example
//!
//! Broadcasts server messages at regular intervals using the timer system.
//!
//! ## Features Demonstrated
//! - `add_timer` - Create a one-shot timer
//! - `add_repeating_timer` - Create a recurring timer
//! - `add_timer_with_flags` - Create timer with custom flags
//! - `TimerFlags` - Control timer behavior (REPEAT, STOP_ON_MAPCHANGE)
//! - `remove_timer` - Cancel a timer
//! - `on_map_start` - Initialize timers when map loads
//!
//! ## Timer Types
//! - One-shot: Fires once after delay
//! - Repeating: Fires at regular intervals
//! - Map-bound: Automatically stops when map changes
//!
//! ## Usage
//! ```ignore
//! announcements::init();
//!
//! // Create custom one-shot timer
//! announcements::schedule_announcement("Hello!", Duration::from_secs(10));
//! ```

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use cs2rust_core::{
    add_timer, add_timer_with_flags, remove_timer,
    on_map_start, TimerFlags, TimerKey,
};

/// Default announcement messages
const DEFAULT_MESSAGES: &[&str] = &[
    "Welcome to our server! Type !help for commands.",
    "Follow our rules to have a great time!",
    "Join our Discord: discord.gg/example",
    "Report hackers with !report <player>",
];

/// Tracks which message to show next
static MESSAGE_INDEX: AtomicUsize = AtomicUsize::new(0);

/// Initialize the Announcements plugin.
///
/// Sets up recurring announcement timers that reset on each map start.
pub fn init() {
    // Register map start handler to set up timers
    on_map_start(|map_name| {
        tracing::info!("Announcements: Setting up timers for map {}", map_name);

        // Reset message index on new map
        MESSAGE_INDEX.store(0, Ordering::Relaxed);

        // One-shot welcome timer (5 seconds after map start)
        let _welcome_timer: TimerKey = add_timer(Duration::from_secs(5), || {
            tracing::info!("[Announcement] Map is now ready! Have fun!");
        });

        // Repeating announcement timer with auto-cleanup on map change
        // Using add_timer_with_flags for both REPEAT and STOP_ON_MAPCHANGE
        let _announcement_timer: TimerKey = add_timer_with_flags(
            Duration::from_secs(60), // 60 seconds between announcements
            TimerFlags::REPEAT | TimerFlags::STOP_ON_MAPCHANGE,
            || {
                // Get next message in rotation
                let index = MESSAGE_INDEX.fetch_add(1, Ordering::Relaxed);
                let message = DEFAULT_MESSAGES[index % DEFAULT_MESSAGES.len()];

                tracing::info!("[Announcement] {}", message);

                // In a real plugin, you would broadcast to all players:
                // for player in get_players() {
                //     client_print(player, HudDestination::Talk, message);
                // }
            },
        );

        tracing::info!("Announcements: Timers configured");
    });

    tracing::info!("Announcements plugin initialized!");
}

/// Schedule a custom one-shot announcement after a delay.
///
/// # Arguments
/// * `message` - The message to broadcast
/// * `delay` - How long to wait before broadcasting
///
/// # Returns
/// A `TimerKey` that can be used to cancel the announcement.
pub fn schedule_announcement(message: String, delay: Duration) -> TimerKey {
    add_timer(delay, move || {
        tracing::info!("[Announcement] {}", message);
    })
}

/// Schedule a repeating announcement at regular intervals.
///
/// # Arguments
/// * `message` - The message to broadcast
/// * `interval` - Time between broadcasts
///
/// # Returns
/// A `TimerKey` that can be used to cancel the announcement.
pub fn schedule_repeating(message: String, interval: Duration) -> TimerKey {
    add_timer_with_flags(
        interval,
        TimerFlags::REPEAT | TimerFlags::STOP_ON_MAPCHANGE,
        move || {
            tracing::info!("[Announcement] {}", message);
        },
    )
}

/// Cancel a scheduled announcement.
///
/// # Arguments
/// * `timer_key` - The key returned from a schedule function
pub fn cancel_announcement(timer_key: TimerKey) {
    remove_timer(timer_key);
    tracing::debug!("Announcements: Timer cancelled");
}

/// Example of creating a countdown timer
pub fn start_countdown(seconds: u32, on_complete: impl FnOnce() + Send + 'static) {
    // Create a series of one-shot timers for countdown
    for i in (1..=seconds).rev() {
        let delay = Duration::from_secs((seconds - i) as u64);
        add_timer(delay, move || {
            tracing::info!("[Countdown] {}...", i);
        });
    }

    // Final timer calls the completion callback
    // Wrap FnOnce in Option to make it FnMut-compatible
    let mut callback = Some(on_complete);
    add_timer(Duration::from_secs(seconds as u64), move || {
        if let Some(f) = callback.take() {
            f();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_rotation() {
        MESSAGE_INDEX.store(0, Ordering::Relaxed);

        let idx1 = MESSAGE_INDEX.fetch_add(1, Ordering::Relaxed);
        let idx2 = MESSAGE_INDEX.fetch_add(1, Ordering::Relaxed);

        assert_eq!(idx1, 0);
        assert_eq!(idx2, 1);

        // Test wrap-around
        let msg1 = DEFAULT_MESSAGES[0 % DEFAULT_MESSAGES.len()];
        let msg2 = DEFAULT_MESSAGES[4 % DEFAULT_MESSAGES.len()];
        assert_eq!(msg1, msg2); // Both index to first message
    }

    #[test]
    fn test_timer_flags_combination() {
        // Verify flags can be combined
        let flags = TimerFlags::REPEAT | TimerFlags::STOP_ON_MAPCHANGE;
        assert!(flags.contains(TimerFlags::REPEAT));
        assert!(flags.contains(TimerFlags::STOP_ON_MAPCHANGE));
    }
}
