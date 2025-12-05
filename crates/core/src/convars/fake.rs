//! Fake ConVars for plugin settings
//!
//! FakeConVars are Rust-managed settings that integrate with the command system.
//! Unlike real ConVars, they don't register with the engine's ICvar system.
//!
//! # Example
//!
//! ```ignore
//! use std::sync::LazyLock;
//! use cs2rust_core::convars::FakeConVar;
//!
//! static MY_ENABLED: LazyLock<FakeConVar<bool>> = LazyLock::new(|| {
//!     FakeConVar::new("my_plugin_enabled", true, "Enable my plugin")
//! });
//!
//! static MY_MAX_PLAYERS: LazyLock<FakeConVar<i32>> = LazyLock::new(|| {
//!     FakeConVar::new("my_max_players", 10, "Max players in queue")
//!         .with_min(1)
//!         .with_max(64)
//!         .with_on_change(|old, new| {
//!             tracing::info!("Max players changed: {} -> {}", old, new);
//!         })
//! });
//!
//! fn some_function() {
//!     if MY_ENABLED.get() {
//!         let max = MY_MAX_PLAYERS.get();
//!         // ...
//!     }
//! }
//! ```

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::RwLock;

use crate::commands::{register_command, CommandInfo, CommandResult};
use crate::entities::PlayerController;

/// Wrapper to make a raw pointer Send+Sync
///
/// SAFETY: Only use this for pointers to static data with 'static lifetime
/// that is internally thread-safe (e.g., uses RwLock).
#[derive(Clone, Copy)]
struct SendSyncPtr<T>(*const T);

impl<T> SendSyncPtr<T> {
    /// Get the inner pointer
    fn get(&self) -> *const T {
        self.0
    }
}

// SAFETY: This is safe because:
// 1. FakeConVars are designed to be stored in static LazyLock
// 2. They live for the entire program duration
// 3. They use RwLock internally for thread-safe access
unsafe impl<T> Send for SendSyncPtr<T> {}
unsafe impl<T> Sync for SendSyncPtr<T> {}

/// Trait for types that can be used as FakeConVar values
///
/// Implement this trait for custom types that should be usable as FakeConVar values.
pub trait ConVarValue: Clone + Send + Sync + 'static {
    /// Parse from a string
    fn from_str(s: &str) -> Option<Self>;

    /// Convert to a string representation
    fn to_string_value(&self) -> String;
}

impl ConVarValue for bool {
    fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            "0" | "false" | "no" | "off" => Some(false),
            _ => None,
        }
    }

    fn to_string_value(&self) -> String {
        if *self { "1" } else { "0" }.to_string()
    }
}

impl ConVarValue for i32 {
    fn from_str(s: &str) -> Option<Self> {
        s.parse().ok()
    }

    fn to_string_value(&self) -> String {
        ToString::to_string(self)
    }
}

impl ConVarValue for i64 {
    fn from_str(s: &str) -> Option<Self> {
        s.parse().ok()
    }

    fn to_string_value(&self) -> String {
        ToString::to_string(self)
    }
}

impl ConVarValue for f32 {
    fn from_str(s: &str) -> Option<Self> {
        s.parse().ok()
    }

    fn to_string_value(&self) -> String {
        ToString::to_string(self)
    }
}

impl ConVarValue for f64 {
    fn from_str(s: &str) -> Option<Self> {
        s.parse().ok()
    }

    fn to_string_value(&self) -> String {
        ToString::to_string(self)
    }
}

impl ConVarValue for String {
    fn from_str(s: &str) -> Option<Self> {
        Some(s.to_string())
    }

    fn to_string_value(&self) -> String {
        self.clone()
    }
}

/// Change callback function type
pub type OnChangeFn<T> = Box<dyn Fn(&T, &T) + Send + Sync>;

/// A fake ConVar for plugin configuration
///
/// FakeConVars provide a convar-like interface for plugin settings
/// without registering with the engine's ICvar system.
///
/// Features:
/// - Thread-safe value storage via RwLock
/// - Optional min/max value constraints
/// - Change callbacks
/// - Auto-registration as console commands
pub struct FakeConVar<T: ConVarValue + PartialOrd> {
    /// ConVar name (used for console command)
    name: String,
    /// Current value
    value: RwLock<T>,
    /// Default value
    default: T,
    /// Description/help text
    description: String,
    /// Minimum value constraint
    min: Option<T>,
    /// Maximum value constraint
    max: Option<T>,
    /// Change callback
    on_change: Option<OnChangeFn<T>>,
    /// Whether the command has been registered
    registered: AtomicBool,
}

impl<T: ConVarValue + PartialOrd> FakeConVar<T> {
    /// Create a new FakeConVar
    ///
    /// The FakeConVar will auto-register as a console command when first accessed.
    ///
    /// # Arguments
    /// * `name` - The convar/command name
    /// * `default` - Default value
    /// * `description` - Help text
    pub fn new(name: impl Into<String>, default: T, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: RwLock::new(default.clone()),
            default,
            description: description.into(),
            min: None,
            max: None,
            on_change: None,
            registered: AtomicBool::new(false),
        }
    }

    /// Set minimum value constraint (builder pattern)
    pub fn with_min(mut self, min: T) -> Self {
        self.min = Some(min);
        self
    }

    /// Set maximum value constraint (builder pattern)
    pub fn with_max(mut self, max: T) -> Self {
        self.max = Some(max);
        self
    }

    /// Set change callback (builder pattern)
    ///
    /// The callback receives references to the old and new values.
    pub fn with_on_change<F>(mut self, f: F) -> Self
    where
        F: Fn(&T, &T) + Send + Sync + 'static,
    {
        self.on_change = Some(Box::new(f));
        self
    }

    /// Get the current value
    ///
    /// This also triggers auto-registration of the console command if not already done.
    pub fn get(&self) -> T {
        self.ensure_registered();
        self.value.read().unwrap().clone()
    }

    /// Set the value
    ///
    /// Returns true if the value was set as-is, false if it was clamped to min/max.
    pub fn set(&self, mut value: T) -> bool {
        self.ensure_registered();

        let mut clamped = false;

        // Clamp to min/max
        if let Some(ref min) = self.min {
            if value < *min {
                value = min.clone();
                clamped = true;
            }
        }
        if let Some(ref max) = self.max {
            if value > *max {
                value = max.clone();
                clamped = true;
            }
        }

        let old_value = self.get_unchecked();
        *self.value.write().unwrap() = value.clone();

        // Call change callback
        if let Some(ref callback) = self.on_change {
            callback(&old_value, &value);
        }

        !clamped
    }

    /// Get value without triggering registration (internal use)
    fn get_unchecked(&self) -> T {
        self.value.read().unwrap().clone()
    }

    /// Reset to default value
    pub fn reset(&self) {
        self.set(self.default.clone());
    }

    /// Get the convar name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the description
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Get the default value
    pub fn default_value(&self) -> &T {
        &self.default
    }

    /// Check if the current value equals the default
    pub fn is_default(&self) -> bool
    where
        T: PartialEq,
    {
        self.get() == self.default
    }

    /// Ensure the console command is registered
    fn ensure_registered(&self) {
        // Fast path: already registered
        if self.registered.load(Ordering::Acquire) {
            return;
        }

        // Try to register
        if self
            .registered
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            self.register_command_internal();
        }
    }

    /// Internal command registration
    fn register_command_internal(&self) {
        let name = self.name.clone();
        let description = self.description.clone();

        // Get a raw pointer to self for use in the callback
        // This is safe because FakeConVars are typically stored in static LazyLock
        let self_ptr = self as *const Self;

        // Wrap the pointer in a SendSync wrapper for the closure
        // SAFETY: FakeConVars are designed to be stored in static LazyLock,
        // which guarantees they live for the entire program duration and are
        // safely shared across threads (FakeConVar uses RwLock internally).
        let self_ptr = SendSyncPtr(self_ptr);

        register_command(&name, &description, move |_player, info| {
            let fake_cvar = unsafe { &*self_ptr.get() };
            fake_cvar.handle_command(_player, info)
        });

        tracing::debug!("Registered FakeConVar command: {}", self.name);
    }

    /// Handle console command execution
    fn handle_command(
        &self,
        _player: Option<&PlayerController>,
        info: &CommandInfo,
    ) -> CommandResult {
        if info.arg_count() < 2 {
            // No arguments - print current value
            let value = self.get_unchecked();
            let value_str = value.to_string_value();

            if let Some(ref min) = self.min {
                if let Some(ref max) = self.max {
                    info.reply(&format!(
                        "{} = {} (default: {}, min: {}, max: {})",
                        self.name,
                        value_str,
                        self.default.to_string_value(),
                        min.to_string_value(),
                        max.to_string_value()
                    ));
                } else {
                    info.reply(&format!(
                        "{} = {} (default: {}, min: {})",
                        self.name,
                        value_str,
                        self.default.to_string_value(),
                        min.to_string_value()
                    ));
                }
            } else if let Some(ref max) = self.max {
                info.reply(&format!(
                    "{} = {} (default: {}, max: {})",
                    self.name,
                    value_str,
                    self.default.to_string_value(),
                    max.to_string_value()
                ));
            } else {
                info.reply(&format!(
                    "{} = {} (default: {})",
                    self.name,
                    value_str,
                    self.default.to_string_value()
                ));
            }

            return CommandResult::Handled;
        }

        // Try to parse the new value
        let arg = info.arg(1);
        match T::from_str(arg) {
            Some(new_value) => {
                if self.set(new_value.clone()) {
                    info.reply(&format!("{} set to {}", self.name, new_value.to_string_value()));
                } else {
                    // Value was clamped
                    let actual = self.get_unchecked();
                    info.reply(&format!(
                        "{} clamped to {} (requested: {})",
                        self.name,
                        actual.to_string_value(),
                        new_value.to_string_value()
                    ));
                }
                CommandResult::Handled
            }
            None => {
                info.reply(&format!(
                    "Invalid value '{}' for {} (expected type: {})",
                    arg,
                    self.name,
                    std::any::type_name::<T>()
                ));
                CommandResult::Handled
            }
        }
    }
}

impl<T: ConVarValue + PartialOrd + std::fmt::Debug> std::fmt::Debug for FakeConVar<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FakeConVar")
            .field("name", &self.name)
            .field("value", &self.get_unchecked())
            .field("default", &self.default)
            .field("description", &self.description)
            .field("min", &self.min)
            .field("max", &self.max)
            .field("registered", &self.registered.load(Ordering::Relaxed))
            .finish()
    }
}

impl<T: ConVarValue + PartialOrd> std::fmt::Display for FakeConVar<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} = {}", self.name, self.get_unchecked().to_string_value())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convar_value_bool() {
        assert_eq!(bool::from_str("1"), Some(true));
        assert_eq!(bool::from_str("true"), Some(true));
        assert_eq!(bool::from_str("yes"), Some(true));
        assert_eq!(bool::from_str("on"), Some(true));
        assert_eq!(bool::from_str("0"), Some(false));
        assert_eq!(bool::from_str("false"), Some(false));
        assert_eq!(bool::from_str("no"), Some(false));
        assert_eq!(bool::from_str("off"), Some(false));
        assert_eq!(bool::from_str("invalid"), None);
    }

    #[test]
    fn test_convar_value_i32() {
        assert_eq!(i32::from_str("42"), Some(42));
        assert_eq!(i32::from_str("-10"), Some(-10));
        assert_eq!(i32::from_str("not_a_number"), None);
    }

    #[test]
    fn test_convar_value_f32() {
        assert_eq!(f32::from_str("3.14"), Some(3.14));
        assert_eq!(f32::from_str("-2.5"), Some(-2.5));
        assert_eq!(f32::from_str("not_a_number"), None);
    }

    #[test]
    fn test_fake_convar_basic() {
        let cvar = FakeConVar::new("test_cvar", 42i32, "Test convar");
        // Note: In tests without the engine, we can't call get() because it
        // tries to register the command. Use get_unchecked for testing.

        assert_eq!(cvar.name(), "test_cvar");
        assert_eq!(cvar.description(), "Test convar");
        assert_eq!(*cvar.default_value(), 42);
    }

    #[test]
    fn test_fake_convar_constraints() {
        let cvar = FakeConVar::new("test_constrained", 50i32, "Constrained convar")
            .with_min(0)
            .with_max(100);

        // Set within bounds - should work
        *cvar.value.write().unwrap() = 75;
        assert_eq!(*cvar.value.read().unwrap(), 75);
    }
}
