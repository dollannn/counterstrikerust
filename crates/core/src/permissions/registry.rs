//! Permission registry - centralized storage for player permissions
//!
//! This module provides the global permission registry that stores
//! permission data for all players. Plugins mutate this registry
//! via the public API functions.

use std::collections::HashSet;
use std::sync::LazyLock;

use dashmap::DashMap;

use super::types::PermissionData;

/// Global permission registry keyed by SteamID64
static REGISTRY: LazyLock<DashMap<u64, PermissionData>> = LazyLock::new(DashMap::new);

// ============================================================================
// Mutation APIs
// ============================================================================

/// Add permission(s) to a player
///
/// If the player doesn't exist in the registry, creates a new entry.
///
/// # Arguments
/// * `steam_id` - The player's 64-bit Steam ID
/// * `permissions` - Slice of permission strings to add
///
/// # Example
/// ```ignore
/// add_permissions(76561198012345678, &["@css/ban", "@css/kick"]);
/// ```
pub fn add_permissions(steam_id: u64, permissions: &[&str]) {
    REGISTRY.entry(steam_id).or_default().add(permissions);
}

/// Remove permission(s) from a player
///
/// Does nothing if the player doesn't exist in the registry.
///
/// # Arguments
/// * `steam_id` - The player's 64-bit Steam ID
/// * `permissions` - Slice of permission strings to remove
pub fn remove_permissions(steam_id: u64, permissions: &[&str]) {
    if let Some(mut data) = REGISTRY.get_mut(&steam_id) {
        data.remove(permissions);
    }
}

/// Set all permissions for a player (replaces existing)
///
/// Creates a new entry or replaces the existing one entirely.
///
/// # Arguments
/// * `steam_id` - The player's 64-bit Steam ID
/// * `permissions` - Slice of permission strings to set
pub fn set_permissions(steam_id: u64, permissions: &[&str]) {
    let mut data = PermissionData::new();
    data.add(permissions);
    REGISTRY.insert(steam_id, data);
}

/// Clear all permissions for a player
///
/// Removes the player entirely from the registry.
///
/// # Arguments
/// * `steam_id` - The player's 64-bit Steam ID
pub fn clear_permissions(steam_id: u64) {
    REGISTRY.remove(&steam_id);
}

/// Set immunity level for a player
///
/// Creates a new entry if the player doesn't exist.
///
/// # Arguments
/// * `steam_id` - The player's 64-bit Steam ID
/// * `immunity` - Immunity level (higher = more protected)
pub fn set_immunity(steam_id: u64, immunity: u32) {
    REGISTRY.entry(steam_id).or_default().immunity = immunity;
}

// ============================================================================
// Query APIs
// ============================================================================

/// Check if a player has a specific permission
///
/// Also checks root flags: `@domain/root` grants all `@domain/*` permissions.
///
/// # Arguments
/// * `steam_id` - The player's 64-bit Steam ID
/// * `permission` - The permission string to check
///
/// # Returns
/// `true` if the player has the permission, `false` otherwise
pub fn has_permission(steam_id: u64, permission: &str) -> bool {
    REGISTRY
        .get(&steam_id)
        .map(|data| data.has(permission))
        .unwrap_or(false)
}

/// Check if a player has any of the given permissions
///
/// # Arguments
/// * `steam_id` - The player's 64-bit Steam ID
/// * `permissions` - Slice of permission strings to check
///
/// # Returns
/// `true` if the player has at least one of the permissions
pub fn has_any_permission(steam_id: u64, permissions: &[&str]) -> bool {
    REGISTRY
        .get(&steam_id)
        .map(|data| data.has_any(permissions))
        .unwrap_or(false)
}

/// Check if a player has all of the given permissions
///
/// # Arguments
/// * `steam_id` - The player's 64-bit Steam ID
/// * `permissions` - Slice of permission strings to check
///
/// # Returns
/// `true` if the player has all of the permissions
pub fn has_all_permissions(steam_id: u64, permissions: &[&str]) -> bool {
    REGISTRY
        .get(&steam_id)
        .map(|data| data.has_all(permissions))
        .unwrap_or(false)
}

/// Get all permissions for a player
///
/// # Arguments
/// * `steam_id` - The player's 64-bit Steam ID
///
/// # Returns
/// A set of all permission strings, or empty set if player not found
pub fn get_permissions(steam_id: u64) -> HashSet<String> {
    REGISTRY
        .get(&steam_id)
        .map(|data| data.all_permissions())
        .unwrap_or_default()
}

/// Get immunity level for a player
///
/// # Arguments
/// * `steam_id` - The player's 64-bit Steam ID
///
/// # Returns
/// The player's immunity level, or 0 if not found
pub fn get_immunity(steam_id: u64) -> u32 {
    REGISTRY
        .get(&steam_id)
        .map(|data| data.immunity)
        .unwrap_or(0)
}

/// Check if source player can target destination player
///
/// A player can target another if their immunity is >= the target's immunity.
///
/// # Arguments
/// * `source_id` - The attacking player's Steam ID
/// * `target_id` - The target player's Steam ID
///
/// # Returns
/// `true` if source can target destination
pub fn can_target(source_id: u64, target_id: u64) -> bool {
    let source_immunity = get_immunity(source_id);
    let target_immunity = get_immunity(target_id);
    source_immunity >= target_immunity
}

// ============================================================================
// Utility APIs
// ============================================================================

/// Check if a player has any permissions registered
///
/// # Arguments
/// * `steam_id` - The player's 64-bit Steam ID
///
/// # Returns
/// `true` if the player exists in the registry
pub fn is_registered(steam_id: u64) -> bool {
    REGISTRY.contains_key(&steam_id)
}

/// Get the number of players with permissions
pub fn player_count() -> usize {
    REGISTRY.len()
}

/// Clear all permissions for all players
///
/// Use with caution - typically only needed for tests or full resets.
pub fn clear_all() {
    REGISTRY.clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    // Generate unique steam IDs for each test to avoid parallel test interference
    static TEST_ID_COUNTER: AtomicU64 = AtomicU64::new(1_000_000);

    fn unique_steam_id() -> u64 {
        TEST_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
    }

    #[test]
    fn test_add_and_check_permissions() {
        let steam_id = unique_steam_id();

        add_permissions(steam_id, &["@css/ban", "@css/kick"]);

        assert!(has_permission(steam_id, "@css/ban"));
        assert!(has_permission(steam_id, "@css/kick"));
        assert!(!has_permission(steam_id, "@css/slay"));

        clear_permissions(steam_id);
    }

    #[test]
    fn test_remove_permissions() {
        let steam_id = unique_steam_id();

        add_permissions(steam_id, &["@css/ban", "@css/kick"]);
        remove_permissions(steam_id, &["@css/ban"]);

        assert!(!has_permission(steam_id, "@css/ban"));
        assert!(has_permission(steam_id, "@css/kick"));

        clear_permissions(steam_id);
    }

    #[test]
    fn test_set_permissions_replaces() {
        let steam_id = unique_steam_id();

        add_permissions(steam_id, &["@css/ban", "@css/kick"]);
        set_permissions(steam_id, &["@css/slay"]);

        assert!(!has_permission(steam_id, "@css/ban"));
        assert!(!has_permission(steam_id, "@css/kick"));
        assert!(has_permission(steam_id, "@css/slay"));

        clear_permissions(steam_id);
    }

    #[test]
    fn test_clear_permissions() {
        let steam_id = unique_steam_id();

        add_permissions(steam_id, &["@css/ban"]);
        assert!(is_registered(steam_id));

        clear_permissions(steam_id);
        assert!(!is_registered(steam_id));
        assert!(!has_permission(steam_id, "@css/ban"));
    }

    #[test]
    fn test_immunity() {
        let admin = unique_steam_id();
        let moderator = unique_steam_id();

        set_immunity(admin, 100);
        set_immunity(moderator, 50);

        assert_eq!(get_immunity(admin), 100);
        assert_eq!(get_immunity(moderator), 50);

        assert!(can_target(admin, moderator)); // 100 >= 50
        assert!(!can_target(moderator, admin)); // 50 < 100
        assert!(can_target(admin, admin)); // 100 >= 100

        clear_permissions(admin);
        clear_permissions(moderator);
    }

    #[test]
    fn test_has_any_all() {
        let steam_id = unique_steam_id();

        add_permissions(steam_id, &["@css/kick", "@css/ban"]);

        assert!(has_any_permission(steam_id, &["@css/kick", "@css/slay"]));
        assert!(!has_any_permission(steam_id, &["@css/slay", "@css/cvar"]));

        assert!(has_all_permissions(steam_id, &["@css/kick", "@css/ban"]));
        assert!(!has_all_permissions(steam_id, &["@css/kick", "@css/slay"]));

        clear_permissions(steam_id);
    }

    #[test]
    fn test_nonexistent_player() {
        let steam_id = unique_steam_id();

        assert!(!has_permission(steam_id, "@css/ban"));
        assert!(!has_any_permission(steam_id, &["@css/ban"]));
        assert!(!has_all_permissions(steam_id, &["@css/ban"]));
        assert_eq!(get_immunity(steam_id), 0);
        assert!(get_permissions(steam_id).is_empty());
        assert!(!is_registered(steam_id));
    }
}
