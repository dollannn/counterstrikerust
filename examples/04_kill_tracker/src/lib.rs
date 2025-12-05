//! # Kill Feed Tracker Example
//!
//! Tracks kills and deaths using typed game events.
//!
//! ## Features Demonstrated
//! - `register_typed_event` - Register strongly-typed event handlers
//! - `EventPlayerDeath` - Access death event details
//! - `EventPlayerHurt` - Access damage event details
//! - `HookResult` - Control event propagation
//! - `get_player_controller_by_userid` - Look up players by event user ID
//! - Static state management with `LazyLock` and `RwLock`
//!
//! ## Tracked Statistics
//! - Kills, deaths, headshots per player
//! - Damage dealt and received
//!
//! ## Usage
//! ```ignore
//! kill_tracker::init();
//!
//! // Later, query stats
//! if let Some(stats) = kill_tracker::get_stats(steam_id) {
//!     println!("K/D: {}/{}", stats.kills, stats.deaths);
//! }
//! ```

use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};

use cs2rust_core::HookResult;
use cs2rust_core::entities::get_player_controller_by_userid;
use cs2rust_core::events::typed::{
    register_typed_event, EventPlayerDeath, EventPlayerHurt,
};

/// Per-player statistics
#[derive(Debug, Clone, Default)]
pub struct PlayerStats {
    /// Number of kills
    pub kills: u32,
    /// Number of deaths
    pub deaths: u32,
    /// Number of headshot kills
    pub headshots: u32,
    /// Total damage dealt
    pub damage_dealt: u32,
    /// Total damage received
    pub damage_received: u32,
}

impl PlayerStats {
    /// Calculate kill/death ratio
    pub fn kd_ratio(&self) -> f32 {
        if self.deaths == 0 {
            self.kills as f32
        } else {
            self.kills as f32 / self.deaths as f32
        }
    }

    /// Calculate headshot percentage
    pub fn headshot_percentage(&self) -> f32 {
        if self.kills == 0 {
            0.0
        } else {
            (self.headshots as f32 / self.kills as f32) * 100.0
        }
    }
}

/// Global stats storage, keyed by SteamID64
static PLAYER_STATS: LazyLock<RwLock<HashMap<u64, PlayerStats>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// Initialize the Kill Tracker plugin.
///
/// Registers event handlers for player_death and player_hurt.
pub fn init() {
    register_death_handler();
    register_hurt_handler();

    tracing::info!("Kill Tracker plugin initialized!");
}

/// Get stats for a player by SteamID64
pub fn get_stats(steam_id: u64) -> Option<PlayerStats> {
    PLAYER_STATS
        .read()
        .ok()
        .and_then(|stats| stats.get(&steam_id).cloned())
}

/// Get all player stats
pub fn get_all_stats() -> HashMap<u64, PlayerStats> {
    PLAYER_STATS
        .read()
        .map(|stats| stats.clone())
        .unwrap_or_default()
}

/// Clear all stats (e.g., on map change)
pub fn clear_stats() {
    if let Ok(mut stats) = PLAYER_STATS.write() {
        stats.clear();
    }
    tracing::info!("Kill Tracker: Stats cleared");
}

/// Register the player_death event handler
fn register_death_handler() {
    // The `false` parameter means this is a pre-hook (fires before event propagates)
    // Use `true` for post-hooks (fires after event is processed)
    register_typed_event::<EventPlayerDeath, _>(false, |event, _info| {
        // Log the death with details
        tracing::debug!(
            "Death event: victim={}, attacker={}, weapon={}, headshot={}",
            event.userid,
            event.attacker,
            event.weapon,
            event.headshot
        );

        // Track victim's death
        if let Some(victim) = get_player_controller_by_userid(event.userid) {
            let steam_id = victim.steam_id();

            if let Ok(mut stats) = PLAYER_STATS.write() {
                let entry = stats.entry(steam_id).or_default();
                entry.deaths += 1;
            }

            tracing::info!(
                "{} died (weapon: {}, headshot: {})",
                victim.name_string(),
                event.weapon,
                event.headshot
            );
        }

        // Track attacker's kill (if not suicide)
        if event.attacker != event.userid && event.attacker >= 0 {
            if let Some(attacker) = get_player_controller_by_userid(event.attacker) {
                let steam_id = attacker.steam_id();

                if let Ok(mut stats) = PLAYER_STATS.write() {
                    let entry = stats.entry(steam_id).or_default();
                    entry.kills += 1;

                    if event.headshot {
                        entry.headshots += 1;
                    }
                }

                // Log special kills
                if event.headshot {
                    tracing::info!("{} got a HEADSHOT kill!", attacker.name_string());
                }

                if event.noscope {
                    tracing::info!("{} got a NOSCOPE kill!", attacker.name_string());
                }

                if event.thrusmoke {
                    tracing::info!("{} got a through-smoke kill!", attacker.name_string());
                }

                if event.penetrated > 0 {
                    tracing::info!(
                        "{} got a wallbang kill (penetration: {})",
                        attacker.name_string(),
                        event.penetrated
                    );
                }
            }
        }

        // Continue event processing (let other handlers run)
        HookResult::Continue
    });
}

/// Register the player_hurt event handler
fn register_hurt_handler() {
    register_typed_event::<EventPlayerHurt, _>(false, |event, _info| {
        // Track damage received by victim
        if let Some(victim) = get_player_controller_by_userid(event.userid) {
            let steam_id = victim.steam_id();

            if let Ok(mut stats) = PLAYER_STATS.write() {
                let entry = stats.entry(steam_id).or_default();
                entry.damage_received += event.dmg_health as u32;
            }
        }

        // Track damage dealt by attacker
        if event.attacker != event.userid && event.attacker >= 0 {
            if let Some(attacker) = get_player_controller_by_userid(event.attacker) {
                let steam_id = attacker.steam_id();

                if let Ok(mut stats) = PLAYER_STATS.write() {
                    let entry = stats.entry(steam_id).or_default();
                    entry.damage_dealt += event.dmg_health as u32;
                }
            }
        }

        // Log significant damage (headshots, high damage)
        if event.hitgroup == 1 {
            // Headshot
            tracing::debug!("Headshot for {} damage!", event.dmg_health);
        }

        HookResult::Continue
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_stats_defaults() {
        let stats = PlayerStats::default();
        assert_eq!(stats.kills, 0);
        assert_eq!(stats.deaths, 0);
        assert_eq!(stats.headshots, 0);
    }

    #[test]
    fn test_kd_ratio() {
        let mut stats = PlayerStats::default();
        stats.kills = 10;
        stats.deaths = 5;
        assert!((stats.kd_ratio() - 2.0).abs() < f32::EPSILON);

        // No deaths = kills as ratio
        stats.deaths = 0;
        assert!((stats.kd_ratio() - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_headshot_percentage() {
        let mut stats = PlayerStats::default();
        stats.kills = 10;
        stats.headshots = 5;
        assert!((stats.headshot_percentage() - 50.0).abs() < f32::EPSILON);

        // No kills = 0%
        stats.kills = 0;
        assert!((stats.headshot_percentage() - 0.0).abs() < f32::EPSILON);
    }
}
