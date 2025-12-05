//! Error types for engine interface loading

/// Error type for interface loading operations
#[derive(Debug, thiserror::Error)]
pub enum InterfaceError {
    /// Interface version string was not found
    #[error("Interface not found: {0}")]
    NotFound(String),

    /// Factory function returned null for the requested interface
    #[error("Factory returned null for: {0}")]
    NullPointer(String),

    /// Failed to get factory function from Metamod
    #[error("Failed to get factory: {0}")]
    FactoryError(String),

    /// Invalid interface version string (not null-terminated)
    #[error("Invalid version string: {0}")]
    InvalidVersionString(String),

    /// Engine already initialized
    #[error("Engine already initialized")]
    AlreadyInitialized,
}
