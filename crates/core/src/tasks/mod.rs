//! Task queue system for main thread execution
//!
//! Allows background threads to queue work to execute on the main game thread.
//! Tasks are processed each frame in the GameFrame hook.

pub mod queue;

pub use queue::*;
