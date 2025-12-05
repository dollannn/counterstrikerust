//! CS2 Rust Engine - Interface Loading and Global Storage
//!
//! This crate handles:
//! - Loading Source 2 engine interfaces via CreateInterface
//! - Storing interfaces in thread-safe global statics
//! - Providing access to engine state throughout the framework
//!
//! # Architecture
//!
//! Interfaces are acquired once during plugin load via [`loader::load_interfaces`]
//! and stored in [`globals::EngineGlobals`]. Access is provided via the
//! [`engine()`] function.
//!
//! # Thread Safety
//!
//! All interface pointers are valid for the plugin's lifetime. The main game
//! thread ID is stored for runtime checks via [`is_main_thread()`].

pub mod error;
pub mod globals;
pub mod loader;

pub use error::InterfaceError;
pub use globals::{engine, init_engine, is_engine_initialized, is_main_thread, EngineGlobals};
pub use loader::{load_interfaces, InterfaceFactory};
