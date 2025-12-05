//! CS2 Rust Plugin - FFI Layer
//!
//! This crate provides the FFI boundary between Metamod's C++ interface
//! and the Rust core logic. It compiles to a cdylib (.so/.dll).

pub mod ffi;

pub use cs2rust_core::shutdown;
