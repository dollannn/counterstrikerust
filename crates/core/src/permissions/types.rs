//! Permission types and constants
//!
//! This module defines the core types for the permission system.

use std::collections::HashSet;

/// Permission prefix character for user flags
pub const PERMISSION_PREFIX: char = '@';

/// Permission data for a single player
#[derive(Debug, Clone, Default)]
pub struct PermissionData {
    /// Set of permission strings (e.g., "@css/ban", "@myplugin/vip")
    pub permissions: HashSet<String>,
    /// Immunity level for admin targeting (higher = more protected)
    pub immunity: u32,
}

impl PermissionData {
    /// Create empty permission data
    pub fn new() -> Self {
        Self::default()
    }

    /// Add permissions to this data
    pub fn add(&mut self, permissions: &[&str]) {
        for perm in permissions {
            self.permissions.insert((*perm).to_string());
        }
    }

    /// Remove permissions from this data
    pub fn remove(&mut self, permissions: &[&str]) {
        for perm in permissions {
            self.permissions.remove(*perm);
        }
    }

    /// Check if has a specific permission
    ///
    /// Also checks for root flags: `@domain/root` grants all `@domain/*` permissions.
    pub fn has(&self, permission: &str) -> bool {
        // Direct match
        if self.permissions.contains(permission) {
            return true;
        }

        // Check for root flag
        if let Some(domain) = extract_domain(permission) {
            let root_flag = format!("@{}/root", domain);
            let wildcard_flag = format!("@{}/*", domain);
            if self.permissions.contains(&root_flag) || self.permissions.contains(&wildcard_flag) {
                return true;
            }
        }

        false
    }

    /// Check if has any of the given permissions
    pub fn has_any(&self, permissions: &[&str]) -> bool {
        permissions.iter().any(|p| self.has(p))
    }

    /// Check if has all of the given permissions
    pub fn has_all(&self, permissions: &[&str]) -> bool {
        permissions.iter().all(|p| self.has(p))
    }

    /// Get all permissions as a cloned set
    pub fn all_permissions(&self) -> HashSet<String> {
        self.permissions.clone()
    }

    /// Clear all permissions
    pub fn clear(&mut self) {
        self.permissions.clear();
    }

    /// Check if empty (no permissions)
    pub fn is_empty(&self) -> bool {
        self.permissions.is_empty()
    }
}

/// Extract domain from permission string
///
/// `@domain/flag` -> `Some("domain")`
/// `invalid` -> `None`
pub fn extract_domain(permission: &str) -> Option<&str> {
    if permission.starts_with(PERMISSION_PREFIX) {
        permission[1..].split('/').next()
    } else {
        None
    }
}

/// Built-in permission flags (CounterStrikeSharp compatible)
pub mod flags {
    /// Root admin - grants all @css/* permissions
    pub const ROOT: &str = "@css/root";
    /// Generic admin permission
    pub const GENERIC: &str = "@css/generic";
    /// Kick players
    pub const KICK: &str = "@css/kick";
    /// Ban players
    pub const BAN: &str = "@css/ban";
    /// Slay/damage players
    pub const SLAY: &str = "@css/slay";
    /// Change map
    pub const CHANGEMAP: &str = "@css/changemap";
    /// Change convars
    pub const CVAR: &str = "@css/cvar";
    /// Execute configs
    pub const CONFIG: &str = "@css/config";
    /// Admin chat
    pub const CHAT: &str = "@css/chat";
    /// Call votes
    pub const VOTE: &str = "@css/vote";
    /// RCON access
    pub const RCON: &str = "@css/rcon";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_domain() {
        assert_eq!(extract_domain("@css/ban"), Some("css"));
        assert_eq!(extract_domain("@myplugin/vip"), Some("myplugin"));
        assert_eq!(extract_domain("invalid"), None);
        assert_eq!(extract_domain(""), None);
    }

    #[test]
    fn test_permission_data_basic() {
        let mut data = PermissionData::new();
        assert!(data.is_empty());

        data.add(&["@css/ban", "@css/kick"]);
        assert!(!data.is_empty());
        assert!(data.has("@css/ban"));
        assert!(data.has("@css/kick"));
        assert!(!data.has("@css/slay"));

        data.remove(&["@css/ban"]);
        assert!(!data.has("@css/ban"));
        assert!(data.has("@css/kick"));
    }

    #[test]
    fn test_root_flag() {
        let mut data = PermissionData::new();
        data.add(&["@css/root"]);

        // Root grants all @css/* permissions
        assert!(data.has("@css/ban"));
        assert!(data.has("@css/kick"));
        assert!(data.has("@css/anything"));

        // But not other domains
        assert!(!data.has("@other/perm"));
    }

    #[test]
    fn test_wildcard_flag() {
        let mut data = PermissionData::new();
        data.add(&["@myplugin/*"]);

        assert!(data.has("@myplugin/vip"));
        assert!(data.has("@myplugin/feature"));
        assert!(!data.has("@other/perm"));
    }

    #[test]
    fn test_has_any_all() {
        let mut data = PermissionData::new();
        data.add(&["@css/kick", "@css/ban"]);

        assert!(data.has_any(&["@css/kick", "@css/slay"]));
        assert!(!data.has_any(&["@css/slay", "@css/cvar"]));

        assert!(data.has_all(&["@css/kick", "@css/ban"]));
        assert!(!data.has_all(&["@css/kick", "@css/slay"]));
    }
}
