//! # Round Manager Example
//!
//! Manages round events with pre/post hooks and round state tracking.
//!
//! ## Features Demonstrated
//! - `register_typed_event` with pre/post hooks
//! - `EventRoundStart`, `EventRoundEnd` - Round lifecycle events
//! - `EventBombPlanted`, `EventBombDefused` - Bomb events
//! - `HookResult` variants (Continue, Handled, Stop)
//! - `on_tick` - Per-tick game logic
//! - Atomic state management
//!
//! ## Hook Types
//!
//! - **Pre-hooks** (`post: false`): Fire before event is processed, can block
//! - **Post-hooks** (`post: true`): Fire after event is processed
//!
//! ## HookResult
//!
//! - `Continue` - Let event propagate normally
//! - `Handled` - Event handled but let others run
//! - `Stop` - Block event and stop other handlers

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use cs2rust_core::HookResult;
use cs2rust_core::on_tick;
use cs2rust_core::events::typed::{
    register_typed_event,
    EventRoundStart, EventRoundEnd, EventRoundFreezeEnd,
    EventBombPlanted, EventBombDefused, EventBombExploded,
};

// =============================================================================
// Round State
// =============================================================================

/// Current round number (1-indexed)
static ROUND_NUMBER: AtomicU32 = AtomicU32::new(0);

/// Is a round currently in progress?
static ROUND_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Is the round in freeze time (buy period)?
static FREEZE_TIME: AtomicBool = AtomicBool::new(true);

/// Is the bomb currently planted?
static BOMB_PLANTED: AtomicBool = AtomicBool::new(false);

/// Which site is bomb planted at (0=A, 1=B)
static BOMB_SITE: AtomicU32 = AtomicU32::new(0);

/// Track ticks since bomb planted (for countdown display)
static BOMB_TICKS: AtomicU32 = AtomicU32::new(0);

// =============================================================================
// Public API
// =============================================================================

/// Get the current round number
pub fn round_number() -> u32 {
    ROUND_NUMBER.load(Ordering::Relaxed)
}

/// Check if a round is currently active
pub fn is_round_active() -> bool {
    ROUND_ACTIVE.load(Ordering::Relaxed)
}

/// Check if in freeze time (buy period)
pub fn is_freeze_time() -> bool {
    FREEZE_TIME.load(Ordering::Relaxed)
}

/// Check if bomb is planted
pub fn is_bomb_planted() -> bool {
    BOMB_PLANTED.load(Ordering::Relaxed)
}

/// Get bomb site (0=A, 1=B), only valid if bomb is planted
pub fn bomb_site() -> &'static str {
    match BOMB_SITE.load(Ordering::Relaxed) {
        0 => "A",
        1 => "B",
        _ => "?",
    }
}

// =============================================================================
// Initialization
// =============================================================================

/// Initialize the Round Manager plugin.
pub fn init() {
    register_round_start();
    register_round_end();
    register_freeze_end();
    register_bomb_planted();
    register_bomb_defused();
    register_bomb_exploded();
    register_tick_handler();

    tracing::info!("Round Manager plugin initialized!");
}

// =============================================================================
// Event Handlers
// =============================================================================

/// Register round_start event (pre-hook)
fn register_round_start() {
    // Pre-hook: fires BEFORE the event is processed
    register_typed_event::<EventRoundStart, _>(false, |event, _info| {
        // Increment round counter
        let round = ROUND_NUMBER.fetch_add(1, Ordering::Relaxed) + 1;

        // Update state
        ROUND_ACTIVE.store(true, Ordering::Relaxed);
        FREEZE_TIME.store(true, Ordering::Relaxed);
        BOMB_PLANTED.store(false, Ordering::Relaxed);
        BOMB_TICKS.store(0, Ordering::Relaxed);

        tracing::info!("=== ROUND {} STARTING ===", round);
        tracing::info!("  Time limit: {}s", event.timelimit);
        tracing::info!("  Objective: {}", event.objective);

        // Return Continue to let the event propagate
        // Other plugins can also handle this event
        HookResult::Continue
    });
}

/// Register round_end event (post-hook)
fn register_round_end() {
    // Post-hook: fires AFTER the event is processed
    register_typed_event::<EventRoundEnd, _>(true, |event, _info| {
        ROUND_ACTIVE.store(false, Ordering::Relaxed);

        let winner = match event.winner {
            2 => "Terrorists",
            3 => "Counter-Terrorists",
            _ => "Unknown",
        };

        let reason = match event.reason {
            1 => "Target bombed",
            7 => "Bomb defused",
            8 => "CTs win (elimination)",
            9 => "Ts win (elimination)",
            10 => "Time ran out",
            12 => "Target saved",
            _ => "Other",
        };

        tracing::info!("=== ROUND {} ENDED ===", round_number());
        tracing::info!("  Winner: {}", winner);
        tracing::info!("  Reason: {} ({})", reason, event.reason);

        if event.match_end {
            tracing::info!("  *** MATCH END ***");
        }

        HookResult::Continue
    });
}

/// Register round_freeze_end (buy time ended)
fn register_freeze_end() {
    register_typed_event::<EventRoundFreezeEnd, _>(false, |_event, _info| {
        FREEZE_TIME.store(false, Ordering::Relaxed);

        tracing::info!("Freeze time ended - ROUND LIVE!");

        HookResult::Continue
    });
}

/// Register bomb_planted event
fn register_bomb_planted() {
    register_typed_event::<EventBombPlanted, _>(false, |event, _info| {
        BOMB_PLANTED.store(true, Ordering::Relaxed);
        BOMB_SITE.store(event.site as u32, Ordering::Relaxed);
        BOMB_TICKS.store(0, Ordering::Relaxed);

        let site = if event.site == 0 { "A" } else { "B" };
        tracing::info!("*** BOMB PLANTED AT SITE {} ***", site);
        tracing::info!("  Planted by userid: {}", event.userid);

        HookResult::Continue
    });
}

/// Register bomb_defused event
fn register_bomb_defused() {
    register_typed_event::<EventBombDefused, _>(false, |event, _info| {
        BOMB_PLANTED.store(false, Ordering::Relaxed);

        let site = if event.site == 0 { "A" } else { "B" };
        tracing::info!("*** BOMB DEFUSED AT SITE {} ***", site);
        tracing::info!("  Defused by userid: {}", event.userid);

        HookResult::Continue
    });
}

/// Register bomb_exploded event
fn register_bomb_exploded() {
    register_typed_event::<EventBombExploded, _>(false, |event, _info| {
        BOMB_PLANTED.store(false, Ordering::Relaxed);

        let site = if event.site == 0 { "A" } else { "B" };
        tracing::info!("*** BOMB EXPLODED AT SITE {} ***", site);

        HookResult::Continue
    });
}

/// Register per-tick handler for bomb countdown
fn register_tick_handler() {
    on_tick(|| {
        // Only process if bomb is planted
        if !BOMB_PLANTED.load(Ordering::Relaxed) {
            return;
        }

        let ticks = BOMB_TICKS.fetch_add(1, Ordering::Relaxed);

        // Assuming 64 tick server, announce at certain intervals
        // Bomb timer is ~40 seconds = 2560 ticks at 64 tick
        const TICK_RATE: u32 = 64;
        const ANNOUNCE_INTERVAL: u32 = TICK_RATE * 10; // Every 10 seconds

        if ticks > 0 && ticks % ANNOUNCE_INTERVAL == 0 {
            let seconds = ticks / TICK_RATE;
            tracing::debug!("Bomb planted for {}s at site {}", seconds, bomb_site());
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_state_defaults() {
        // Note: These tests may interfere with each other if run in parallel
        // because they share static state. In a real scenario, you'd reset state
        // or use more sophisticated testing strategies.

        // Test initial state accessors compile and work
        let _ = round_number();
        let _ = is_round_active();
        let _ = is_freeze_time();
        let _ = is_bomb_planted();
        let _ = bomb_site();
    }

    #[test]
    fn test_round_end_reasons() {
        // Document CS2 round end reason codes
        let reasons = [
            (1, "Target bombed"),
            (7, "Bomb defused"),
            (8, "CTs win (elimination)"),
            (9, "Ts win (elimination)"),
            (10, "Time ran out"),
            (12, "Target saved"),
        ];

        for (code, _name) in reasons {
            assert!(code > 0);
        }
    }
}
