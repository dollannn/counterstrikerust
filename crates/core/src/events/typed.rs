//! Typed game event structures
//!
//! Provides strongly-typed wrappers around common game events.

use super::raw::GameEventRef;

/// Trait for typed game events
pub trait GameEvent: Sized {
    /// The event name (e.g., "player_death")
    const NAME: &'static str;

    /// Create from a raw event reference
    fn from_raw(event: &GameEventRef) -> Self;
}

/// Player death event
#[derive(Debug, Clone)]
pub struct EventPlayerDeath {
    /// User ID of the player who died
    pub userid: i32,
    /// User ID of the attacker
    pub attacker: i32,
    /// User ID of the assister (-1 if none)
    pub assister: i32,
    /// Was it a headshot?
    pub headshot: bool,
    /// Weapon used for the kill
    pub weapon: String,
    /// Whether the attacker was blinded
    pub attackerblind: bool,
    /// Distance of the kill
    pub distance: f32,
    /// Whether this was a noscope kill
    pub noscope: bool,
    /// Whether this was a through-smoke kill
    pub thrusmoke: bool,
    /// Penetration count
    pub penetrated: i32,
    /// Was it dominated?
    pub dominated: i32,
    /// Was it revenge?
    pub revenge: i32,
}

impl GameEvent for EventPlayerDeath {
    const NAME: &'static str = "player_death";

    fn from_raw(event: &GameEventRef) -> Self {
        Self {
            userid: event.get_int("userid", -1),
            attacker: event.get_int("attacker", -1),
            assister: event.get_int("assister", -1),
            headshot: event.get_bool("headshot", false),
            weapon: event.get_string("weapon", ""),
            attackerblind: event.get_bool("attackerblind", false),
            distance: event.get_float("distance", 0.0),
            noscope: event.get_bool("noscope", false),
            thrusmoke: event.get_bool("thrusmoke", false),
            penetrated: event.get_int("penetrated", 0),
            dominated: event.get_int("dominated", 0),
            revenge: event.get_int("revenge", 0),
        }
    }
}

/// Player hurt event
#[derive(Debug, Clone)]
pub struct EventPlayerHurt {
    /// User ID of the player who was hurt
    pub userid: i32,
    /// User ID of the attacker
    pub attacker: i32,
    /// Remaining health
    pub health: i32,
    /// Remaining armor
    pub armor: i32,
    /// Weapon used
    pub weapon: String,
    /// Damage to health
    pub dmg_health: i32,
    /// Damage to armor
    pub dmg_armor: i32,
    /// Hit group (0=generic, 1=head, 2=chest, etc.)
    pub hitgroup: i32,
}

impl GameEvent for EventPlayerHurt {
    const NAME: &'static str = "player_hurt";

    fn from_raw(event: &GameEventRef) -> Self {
        Self {
            userid: event.get_int("userid", -1),
            attacker: event.get_int("attacker", -1),
            health: event.get_int("health", 0),
            armor: event.get_int("armor", 0),
            weapon: event.get_string("weapon", ""),
            dmg_health: event.get_int("dmg_health", 0),
            dmg_armor: event.get_int("dmg_armor", 0),
            hitgroup: event.get_int("hitgroup", 0),
        }
    }
}

/// Player spawn event
#[derive(Debug, Clone)]
pub struct EventPlayerSpawn {
    /// User ID of the player who spawned
    pub userid: i32,
}

impl GameEvent for EventPlayerSpawn {
    const NAME: &'static str = "player_spawn";

    fn from_raw(event: &GameEventRef) -> Self {
        Self {
            userid: event.get_int("userid", -1),
        }
    }
}

/// Round start event
#[derive(Debug, Clone)]
pub struct EventRoundStart {
    /// Time limit for the round
    pub timelimit: i32,
    /// Frag limit for the round
    pub fraglimit: i32,
    /// Round objective
    pub objective: String,
}

impl GameEvent for EventRoundStart {
    const NAME: &'static str = "round_start";

    fn from_raw(event: &GameEventRef) -> Self {
        Self {
            timelimit: event.get_int("timelimit", 0),
            fraglimit: event.get_int("fraglimit", 0),
            objective: event.get_string("objective", ""),
        }
    }
}

/// Round end event
#[derive(Debug, Clone)]
pub struct EventRoundEnd {
    /// Winning team
    pub winner: i32,
    /// Reason for round end
    pub reason: i32,
    /// Legacy message (deprecated)
    pub message: String,
    /// Is match end
    pub match_end: bool,
}

impl GameEvent for EventRoundEnd {
    const NAME: &'static str = "round_end";

    fn from_raw(event: &GameEventRef) -> Self {
        Self {
            winner: event.get_int("winner", 0),
            reason: event.get_int("reason", 0),
            message: event.get_string("message", ""),
            match_end: event.get_bool("match_end", false),
        }
    }
}

/// Round freeze end event (buy time ended)
#[derive(Debug, Clone)]
pub struct EventRoundFreezeEnd;

impl GameEvent for EventRoundFreezeEnd {
    const NAME: &'static str = "round_freeze_end";

    fn from_raw(_event: &GameEventRef) -> Self {
        Self
    }
}

/// Bomb planted event
#[derive(Debug, Clone)]
pub struct EventBombPlanted {
    /// User ID of the player who planted
    pub userid: i32,
    /// Bombsite (A=0, B=1)
    pub site: i32,
}

impl GameEvent for EventBombPlanted {
    const NAME: &'static str = "bomb_planted";

    fn from_raw(event: &GameEventRef) -> Self {
        Self {
            userid: event.get_int("userid", -1),
            site: event.get_int("site", 0),
        }
    }
}

/// Bomb defused event
#[derive(Debug, Clone)]
pub struct EventBombDefused {
    /// User ID of the player who defused
    pub userid: i32,
    /// Bombsite (A=0, B=1)
    pub site: i32,
}

impl GameEvent for EventBombDefused {
    const NAME: &'static str = "bomb_defused";

    fn from_raw(event: &GameEventRef) -> Self {
        Self {
            userid: event.get_int("userid", -1),
            site: event.get_int("site", 0),
        }
    }
}

/// Bomb exploded event
#[derive(Debug, Clone)]
pub struct EventBombExploded {
    /// User ID of the player who planted
    pub userid: i32,
    /// Bombsite (A=0, B=1)
    pub site: i32,
}

impl GameEvent for EventBombExploded {
    const NAME: &'static str = "bomb_exploded";

    fn from_raw(event: &GameEventRef) -> Self {
        Self {
            userid: event.get_int("userid", -1),
            site: event.get_int("site", 0),
        }
    }
}

/// Player connect event
#[derive(Debug, Clone)]
pub struct EventPlayerConnect {
    /// Player name
    pub name: String,
    /// User ID
    pub userid: i32,
    /// Network ID (Steam ID string)
    pub networkid: String,
    /// Is it a bot?
    pub bot: bool,
}

impl GameEvent for EventPlayerConnect {
    const NAME: &'static str = "player_connect";

    fn from_raw(event: &GameEventRef) -> Self {
        Self {
            name: event.get_string("name", ""),
            userid: event.get_int("userid", -1),
            networkid: event.get_string("networkid", ""),
            bot: event.get_bool("bot", false),
        }
    }
}

/// Player disconnect event
#[derive(Debug, Clone)]
pub struct EventPlayerDisconnect {
    /// User ID
    pub userid: i32,
    /// Disconnect reason
    pub reason: i32,
    /// Player name
    pub name: String,
    /// Network ID (Steam ID string)
    pub networkid: String,
    /// Is it a bot?
    pub bot: bool,
}

impl GameEvent for EventPlayerDisconnect {
    const NAME: &'static str = "player_disconnect";

    fn from_raw(event: &GameEventRef) -> Self {
        Self {
            userid: event.get_int("userid", -1),
            reason: event.get_int("reason", 0),
            name: event.get_string("name", ""),
            networkid: event.get_string("networkid", ""),
            bot: event.get_bool("bot", false),
        }
    }
}

/// Player team change event
#[derive(Debug, Clone)]
pub struct EventPlayerTeam {
    /// User ID
    pub userid: i32,
    /// New team
    pub team: i32,
    /// Old team
    pub oldteam: i32,
    /// Is disconnect?
    pub disconnect: bool,
    /// Is silent (no message)?
    pub silent: bool,
    /// Is it a bot?
    pub isbot: bool,
}

impl GameEvent for EventPlayerTeam {
    const NAME: &'static str = "player_team";

    fn from_raw(event: &GameEventRef) -> Self {
        Self {
            userid: event.get_int("userid", -1),
            team: event.get_int("team", 0),
            oldteam: event.get_int("oldteam", 0),
            disconnect: event.get_bool("disconnect", false),
            silent: event.get_bool("silent", false),
            isbot: event.get_bool("isbot", false),
        }
    }
}

/// Weapon fire event
#[derive(Debug, Clone)]
pub struct EventWeaponFire {
    /// User ID
    pub userid: i32,
    /// Weapon name
    pub weapon: String,
    /// Is silenced?
    pub silenced: bool,
}

impl GameEvent for EventWeaponFire {
    const NAME: &'static str = "weapon_fire";

    fn from_raw(event: &GameEventRef) -> Self {
        Self {
            userid: event.get_int("userid", -1),
            weapon: event.get_string("weapon", ""),
            silenced: event.get_bool("silenced", false),
        }
    }
}

/// Helper function to register a typed event handler
pub fn register_typed_event<E, F>(post: bool, callback: F)
where
    E: GameEvent,
    F: Fn(E, &mut super::EventInfo) -> super::HookResult + Send + Sync + 'static,
{
    super::register_event(E::NAME, post, move |event, info| {
        let typed = E::from_raw(event);
        callback(typed, info)
    });
}
