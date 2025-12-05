//! Hook system
//!
//! Provides multiple hook types:
//! - Inline hooks (function detours via SafetyHook)
//! - VTable hooks (virtual function pointer replacement)
//! - Mid-function hooks (arbitrary address with register context)
//!
//! Uses SafetyHook for proper hook chaining and multi-framework compatibility.
//! Also contains Rust handlers for hooks installed via SourceHook in C++.

pub mod context;
mod ffi;
pub mod gameframe;
pub mod inline;
pub mod manager;
pub mod midhook;
pub mod vtable;

// Re-export GameFrame types
pub use gameframe::{
    frame_count, last_frame_time_ns, on_game_frame, register_gameframe_callback,
    unregister_gameframe_callback, GameFrameKey,
};

// Re-export hook types
pub use context::{MidHookContext, Xmm};
pub use inline::{HookError, InlineHookKey, TypedInlineHook};
pub use manager::{hook, hook_mid, hook_vtable, hook_vtable_direct, HookKey, HookManager};
pub use midhook::MidHookKey;
pub use vtable::VTableHookKey;
