//! CS2 Rust Proc Macros
//!
//! This crate provides proc macros for the CS2Rust framework:
//!
//! - `#[derive(SchemaClass)]` - Generate type-safe schema field accessors
//! - `#[console_command]` - Register console/chat commands
//!
//! # SchemaClass Example
//!
//! ```ignore
//! use cs2rust_macros::SchemaClass;
//! use std::ffi::c_void;
//!
//! #[derive(SchemaClass)]
//! #[schema(class = "CCSPlayerPawn")]
//! pub struct PlayerPawn {
//!     ptr: *mut c_void,
//!
//!     #[schema(field = "m_iHealth", networked)]
//!     health: i32,
//!
//!     #[schema(field = "m_ArmorValue")]
//!     armor: i32,
//! }
//!
//! // Generated methods allow type-safe access:
//! // - player.health() -> i32
//! // - player.set_health(100) - auto-calls NetworkStateChanged
//! // - player.armor() -> i32
//! // - player.set_armor(50)
//! ```
//!
//! # Console Command Example
//!
//! ```ignore
//! use cs2rust_macros::console_command;
//!
//! #[console_command("csr_ping", "Respond with pong")]
//! fn cmd_ping(player: Option<&PlayerController>, info: &CommandInfo) -> CommandResult {
//!     info.reply("Pong!");
//!     CommandResult::Handled
//! }
//!
//! // With permission requirement:
//! #[console_command("css_ban", "Ban a player", permission = "@css/ban")]
//! fn cmd_ban(player: Option<&PlayerController>, info: &CommandInfo) -> CommandResult {
//!     // Only runs if player has @css/ban permission
//!     CommandResult::Handled
//! }
//!
//! // Generated:
//! // - cmd_ping() - The command handler
//! // - cmd_ping_register() - Register the command
//! // - cmd_ping_unregister() - Unregister the command
//! ```
//!
//! # Attributes
//!
//! ## Struct Attributes (SchemaClass)
//!
//! - `#[schema(class = "ClassName")]` - **Required.** The Source 2 class name.
//! - `#[schema(module = "server")]` - Optional. The module to search (default: "server").
//!
//! ## Field Attributes (SchemaClass)
//!
//! - `#[schema(field = "m_fieldName")]` - Mark as a schema field with the given name.
//! - `#[schema(networked)]` - Call NetworkStateChanged on write.
//! - `#[schema(readonly)]` - Don't generate a setter.
//! - `#[schema(entity)]` - Field is an entity handle (future use).

mod console_command;
mod parse;
mod schema_class;

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput, ItemFn};

/// Derive macro for schema class wrappers
///
/// Generates type-safe accessors for Source 2 schema fields with automatic
/// offset resolution and caching.
///
/// # Example
///
/// ```ignore
/// use cs2rust_macros::SchemaClass;
///
/// #[derive(SchemaClass)]
/// #[schema(class = "CCSPlayerPawn")]
/// pub struct PlayerPawn {
///     ptr: *mut c_void,
///
///     #[schema(field = "m_iHealth", networked)]
///     health: i32,
///
///     #[schema(field = "m_ArmorValue")]
///     armor: i32,
/// }
/// ```
///
/// # Generated Code
///
/// For each schema field, the macro generates:
///
/// - A getter method (`fn health(&self) -> i32`)
/// - A setter method (`fn set_health(&mut self, value: i32)`) unless `readonly`
/// - Constants for field names and hashes
/// - A `SchemaObject` trait implementation
///
/// For structs with a `ptr` field:
/// - `unsafe fn from_ptr(ptr: *mut c_void) -> Option<Self>`
/// - `fn as_ptr(&self) -> *mut c_void`
///
/// # Networked Fields
///
/// Fields marked with `networked` will automatically call
/// `network_state_changed()` after the setter writes the value,
/// ensuring the change is replicated to clients.
#[proc_macro_derive(SchemaClass, attributes(schema))]
pub fn derive_schema_class(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    schema_class::derive_schema_class(input).into()
}

/// Attribute macro for console command registration
///
/// Marks a function as a console/chat command and generates helper functions
/// for registration and unregistration.
///
/// # Arguments
///
/// - First argument: Command name (e.g., `"csr_ping"`)
/// - Second argument: Command description (e.g., `"Respond with pong"`)
/// - Optional: `permission = "@domain/flag"` - Required permission to run the command
///
/// # Example
///
/// ```ignore
/// use cs2rust_macros::console_command;
/// use cs2rust_core::commands::{CommandInfo, CommandResult};
/// use cs2rust_core::entities::PlayerController;
///
/// #[console_command("csr_ping", "Respond with pong")]
/// fn cmd_ping(player: Option<&PlayerController>, info: &CommandInfo) -> CommandResult {
///     info.reply("Pong!");
///     CommandResult::Handled
/// }
///
/// // With permission requirement:
/// #[console_command("css_ban", "Ban a player", permission = "@css/ban")]
/// fn cmd_ban(player: Option<&PlayerController>, info: &CommandInfo) -> CommandResult {
///     // Only runs if player has @css/ban permission
///     CommandResult::Handled
/// }
///
/// // Later, register the command:
/// cmd_ping_register();
///
/// // And unregister when done:
/// cmd_ping_unregister();
/// ```
///
/// # Generated Code
///
/// The macro generates:
///
/// - The original function with the correct signature
/// - `{name}_register()` - Register the command with the system
/// - `{name}_unregister()` - Unregister the command
/// - A static storage for the command key
#[proc_macro_attribute]
pub fn console_command(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as console_command::ConsoleCommandArgs);
    let func = parse_macro_input!(item as ItemFn);
    console_command::generate_console_command(args, func).into()
}
