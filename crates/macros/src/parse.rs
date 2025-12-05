//! Attribute parsing for SchemaClass derive macro

use darling::{FromDeriveInput, FromField};
use syn::{DeriveInput, Ident, Type, Visibility};

/// Parsed #[schema(...)] attributes on the struct
#[derive(Debug, FromDeriveInput)]
#[darling(attributes(schema), supports(struct_named))]
pub struct SchemaClassArgs {
    /// Struct identifier
    pub ident: Ident,

    /// Struct visibility
    pub vis: Visibility,

    /// Struct fields
    pub data: darling::ast::Data<(), SchemaFieldArgs>,

    /// Source 2 class name (e.g., "CCSPlayerPawn")
    #[darling(rename = "class")]
    pub class_name: String,

    /// Optional module name (defaults to "server")
    #[darling(default = "default_module")]
    pub module: String,
}

fn default_module() -> String {
    "server".to_string()
}

/// Parsed #[schema(...)] attributes on a field
#[derive(Debug, FromField)]
#[darling(attributes(schema))]
pub struct SchemaFieldArgs {
    /// Field identifier
    pub ident: Option<Ident>,

    /// Field type
    pub ty: Type,

    /// Field visibility
    pub vis: Visibility,

    /// Source 2 field name (e.g., "m_iHealth")
    /// If not specified, this is not a schema field (e.g., the ptr field)
    #[darling(rename = "field")]
    pub field_name: Option<String>,

    /// Whether this field is networked (requires StateChanged call on write)
    #[darling(default)]
    pub networked: bool,

    /// Whether this field is an entity handle
    #[darling(default)]
    pub entity: bool,

    /// Whether this field is read-only (no setter generated)
    #[darling(default)]
    pub readonly: bool,
}

impl SchemaFieldArgs {
    /// Check if this is a schema field (has field_name attribute)
    pub fn is_schema_field(&self) -> bool {
        self.field_name.is_some()
    }

    /// Check if this is the base pointer field
    pub fn is_ptr_field(&self) -> bool {
        self.ident.as_ref().map(|i| i == "ptr").unwrap_or(false)
    }
}

/// Parse a DeriveInput into SchemaClassArgs
pub fn parse_schema_class(input: &DeriveInput) -> darling::Result<SchemaClassArgs> {
    SchemaClassArgs::from_derive_input(input)
}
