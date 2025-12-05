//! Console command attribute macro implementation
//!
//! Provides the `#[console_command]` attribute for ergonomic command registration.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse::Parse, parse::ParseStream, Ident, ItemFn, LitStr, Token};

/// Arguments to the console_command attribute
///
/// Usage:
/// - `#[console_command("csr_ping", "Respond with pong")]`
/// - `#[console_command("css_ban", "Ban a player", permission = "@css/ban")]`
pub struct ConsoleCommandArgs {
    /// Command name (e.g., "csr_ping")
    pub name: LitStr,
    /// Command description
    pub description: LitStr,
    /// Required permission (e.g., "@css/ban")
    pub permission: Option<LitStr>,
}

impl Parse for ConsoleCommandArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: LitStr = input.parse()?;
        input.parse::<Token![,]>()?;
        let description: LitStr = input.parse()?;

        // Check for optional permission parameter
        let permission = if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            let ident: Ident = input.parse()?;
            if ident != "permission" {
                return Err(syn::Error::new(ident.span(), "expected `permission`"));
            }
            input.parse::<Token![=]>()?;
            Some(input.parse::<LitStr>()?)
        } else {
            None
        };

        Ok(Self {
            name,
            description,
            permission,
        })
    }
}

/// Generate the console_command implementation
pub fn generate_console_command(args: ConsoleCommandArgs, func: ItemFn) -> TokenStream {
    let fn_name = &func.sig.ident;
    let fn_vis = &func.vis;
    let fn_block = &func.block;
    let fn_attrs = &func.attrs;

    let command_name = &args.name;
    let command_desc = &args.description;

    // Generate a static key holder for the command
    let key_static_name = Ident::new(
        &format!("__{}_COMMAND_KEY", fn_name.to_string().to_uppercase()),
        fn_name.span(),
    );

    // Generate the registration function name
    let register_fn_name = Ident::new(&format!("{}_register", fn_name), fn_name.span());

    // Generate the unregister function name
    let unregister_fn_name = Ident::new(&format!("{}_unregister", fn_name), fn_name.span());

    // Generate permission parameter
    let permission_arg = match &args.permission {
        Some(perm) => quote! { Some(#perm) },
        None => quote! { None },
    };

    quote! {
        // Static storage for the command key
        static #key_static_name: ::std::sync::OnceLock<::cs2rust_core::commands::CommandKey> =
            ::std::sync::OnceLock::new();

        // The original function with its attributes
        #(#fn_attrs)*
        #fn_vis fn #fn_name(
            player: Option<&::cs2rust_core::entities::PlayerController>,
            info: &::cs2rust_core::commands::CommandInfo,
        ) -> ::cs2rust_core::commands::CommandResult #fn_block

        /// Register this command with the command system
        #fn_vis fn #register_fn_name() -> Option<::cs2rust_core::commands::CommandKey> {
            let key = ::cs2rust_core::commands::register_command_ex(
                #command_name,
                #command_desc,
                #permission_arg,
                #fn_name,
            )?;
            let _ = #key_static_name.set(key);
            Some(key)
        }

        /// Unregister this command
        #fn_vis fn #unregister_fn_name() -> bool {
            if let Some(key) = #key_static_name.get() {
                ::cs2rust_core::commands::unregister_command(*key)
            } else {
                false
            }
        }
    }
}
