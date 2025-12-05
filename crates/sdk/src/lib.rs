//! CS2 Rust SDK - Source 2 Engine Type Definitions
//!
//! This crate contains opaque type definitions for Source 2 engine interfaces.
//! It has no dependencies and compiles quickly, allowing parallel compilation
//! of dependent crates.
//!
//! # Modules
//!
//! - [`interfaces`] - Opaque C++ interface types
//! - [`versions`] - Interface version strings for CreateInterface
//! - [`convar`] - ConVar system type definitions

pub mod convar;
pub mod interfaces;
pub mod versions;

pub use convar::*;
pub use interfaces::*;
pub use versions::INTERFACE_VERSIONS;
