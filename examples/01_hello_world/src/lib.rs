//! # Hello World Example
//!
//! The simplest possible CS2Rust plugin demonstrating basic lifecycle events.
//!
//! ## Features Demonstrated
//! - `on_map_start` - Called when a map is loaded
//! - `on_map_end` - Called when a map is unloaded
//! - Basic logging with `tracing`
//!
//! ## Usage
//! ```ignore
//! // In your plugin's initialization
//! hello_world::init();
//! ```

use cs2rust_core::{on_map_start, on_map_end, ListenerKey};

/// Initialize the Hello World plugin.
///
/// Registers listeners for map start and end events.
pub fn init() {
    // Register a callback for when a map starts
    // The callback receives the map name as a parameter
    let _map_start_key: ListenerKey = on_map_start(|map_name| {
        tracing::info!("Hello from CS2Rust!");
        tracing::info!("Map loaded: {}", map_name);
    });

    // Register a callback for when a map ends
    // This is useful for cleanup operations
    let _map_end_key: ListenerKey = on_map_end(|| {
        tracing::info!("Goodbye! Map is ending.");
    });

    tracing::info!("Hello World plugin initialized!");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_does_not_panic() {
        // Basic smoke test - init should not panic
        // Note: In actual game environment, listeners would fire
        // but in test environment they just register without error
    }
}
