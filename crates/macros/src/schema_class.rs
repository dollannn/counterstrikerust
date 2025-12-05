//! SchemaClass derive macro implementation

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{DeriveInput, GenericArgument, PathArguments, Type};

use crate::parse::{parse_schema_class, SchemaFieldArgs};

/// FNV-1a 32-bit hash (compile-time computation in proc macro)
const fn fnv1a_32(data: &[u8]) -> u32 {
    const FNV_OFFSET_BASIS: u32 = 0x811c9dc5;
    const FNV_PRIME: u32 = 0x01000193;

    let mut hash = FNV_OFFSET_BASIS;
    let mut i = 0;
    while i < data.len() {
        hash ^= data[i] as u32;
        hash = hash.wrapping_mul(FNV_PRIME);
        i += 1;
    }
    hash
}

/// Extract the inner type from `PhantomData<T>` if present, otherwise return the type as-is
fn extract_inner_type(ty: &Type) -> &Type {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "PhantomData" {
                if let PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(GenericArgument::Type(inner)) = args.args.first() {
                        return inner;
                    }
                }
            }
        }
    }
    ty
}

/// Check if a type is PhantomData
fn is_phantom_data(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "PhantomData";
        }
    }
    false
}

/// Generate the SchemaClass implementation
pub fn derive_schema_class(input: DeriveInput) -> TokenStream {
    match parse_schema_class(&input) {
        Ok(args) => generate_impl(args),
        Err(e) => e.write_errors(),
    }
}

fn generate_impl(args: crate::parse::SchemaClassArgs) -> TokenStream {
    let struct_name = &args.ident;
    let class_name = &args.class_name;

    // Get fields
    let fields = match args.data {
        darling::ast::Data::Struct(fields) => fields.fields,
        _ => {
            return syn::Error::new_spanned(
                &args.ident,
                "SchemaClass can only be derived for structs",
            )
            .to_compile_error()
        }
    };

    // Compute class hash at compile time
    let class_hash = fnv1a_32(class_name.as_bytes());

    // Generate static offset storage for each schema field
    let offset_statics: Vec<_> = fields
        .iter()
        .filter(|f| f.is_schema_field())
        .map(|f| generate_offset_static(struct_name, f))
        .collect();

    // Generate getter/setter methods
    let accessors: Vec<_> = fields
        .iter()
        .filter(|f| f.is_schema_field())
        .map(|f| generate_accessors(struct_name, f))
        .collect();

    // Generate constants
    let constants = generate_constants(struct_name, class_name, class_hash, &fields);

    // Generate SchemaObject trait impl
    let schema_object_impl = generate_schema_object_impl(struct_name, class_name, &fields);

    // Generate constructor
    let constructor = generate_constructor(struct_name, &fields);

    quote! {
        // Static offset storage (one per field)
        #(#offset_statics)*

        impl #struct_name {
            #constants
            #constructor
        }

        #(#accessors)*

        #schema_object_impl
    }
}

fn generate_offset_static(struct_name: &syn::Ident, field: &SchemaFieldArgs) -> TokenStream {
    let field_ident = field.ident.as_ref().unwrap();
    let static_name = format_ident!(
        "__{}__{}_OFFSET",
        struct_name.to_string().to_uppercase(),
        field_ident.to_string().to_uppercase()
    );

    quote! {
        static #static_name: ::std::sync::OnceLock<::cs2rust_core::schema::SchemaOffset> =
            ::std::sync::OnceLock::new();
    }
}

fn generate_accessors(struct_name: &syn::Ident, field: &SchemaFieldArgs) -> TokenStream {
    let field_ident = field.ident.as_ref().unwrap();
    let field_name = field.field_name.as_ref().unwrap();
    // Extract inner type from PhantomData<T> if present
    let field_ty = extract_inner_type(&field.ty);
    let networked = field.networked;
    let readonly = field.readonly;

    let static_name = format_ident!(
        "__{}__{}_OFFSET",
        struct_name.to_string().to_uppercase(),
        field_ident.to_string().to_uppercase()
    );

    // Strip leading underscore from field name for getter/setter names
    let field_name_str = field_ident.to_string();
    let clean_name = field_name_str.strip_prefix('_').unwrap_or(&field_name_str);
    let getter_name = format_ident!("{}", clean_name);
    let setter_name = format_ident!("set_{}", clean_name);

    let const_field_name = format_ident!("{}_FIELD", clean_name.to_uppercase());

    let field_doc = format!("Get the value of `{}`", field_name);
    let setter_doc = format!("Set the value of `{}`", field_name);

    // Generate getter
    let getter = quote! {
        #[doc = #field_doc]
        #[inline]
        pub fn #getter_name(&self) -> #field_ty {
            let offset = #static_name.get_or_init(|| {
                ::cs2rust_core::schema::get_offset(
                    Self::CLASS_NAME,
                    Self::#const_field_name,
                ).expect(concat!("Failed to resolve ", stringify!(#field_ident)))
            });

            unsafe {
                let ptr = self.ptr.byte_add(offset.offset as usize) as *const #field_ty;
                ptr.read()
            }
        }
    };

    // Generate setter (unless readonly)
    let setter = if readonly {
        quote! {}
    } else {
        let state_change = if networked {
            quote! {
                // Notify engine of networked property change
                unsafe {
                    ::cs2rust_core::schema::network_state_changed(self.ptr, offset.offset);
                }
            }
        } else {
            quote! {}
        };

        quote! {
            #[doc = #setter_doc]
            #[inline]
            pub fn #setter_name(&mut self, value: #field_ty) {
                let offset = #static_name.get_or_init(|| {
                    ::cs2rust_core::schema::get_offset(
                        Self::CLASS_NAME,
                        Self::#const_field_name,
                    ).expect(concat!("Failed to resolve ", stringify!(#field_ident)))
                });

                unsafe {
                    let ptr = self.ptr.byte_add(offset.offset as usize) as *mut #field_ty;
                    ptr.write(value);
                }

                #state_change
            }
        }
    };

    quote! {
        impl #struct_name {
            #getter
            #setter
        }
    }
}

fn generate_constants(
    _struct_name: &syn::Ident,
    class_name: &str,
    class_hash: u32,
    fields: &[SchemaFieldArgs],
) -> TokenStream {
    let field_constants = fields.iter().filter(|f| f.is_schema_field()).map(|f| {
        let field_ident = f.ident.as_ref().unwrap();
        let field_name = f.field_name.as_ref().unwrap();
        let field_hash = fnv1a_32(field_name.as_bytes());

        // Strip leading underscore from field name for constant names
        let field_name_str = field_ident.to_string();
        let clean_name = field_name_str.strip_prefix('_').unwrap_or(&field_name_str);

        let const_name = format_ident!("{}_FIELD", clean_name.to_uppercase());
        let const_hash = format_ident!("{}_HASH", clean_name.to_uppercase());

        let field_doc = format!("Schema field name for `{}`", clean_name);
        let hash_doc = format!("FNV-1a hash of field name `{}`", field_name);

        quote! {
            #[doc = #field_doc]
            pub const #const_name: &'static str = #field_name;

            #[doc = #hash_doc]
            pub const #const_hash: u32 = #field_hash;
        }
    });

    quote! {
        /// Source 2 class name
        pub const CLASS_NAME: &'static str = #class_name;

        /// FNV-1a hash of class name
        pub const CLASS_HASH: u32 = #class_hash;

        #(#field_constants)*
    }
}

fn generate_schema_object_impl(
    struct_name: &syn::Ident,
    class_name: &str,
    fields: &[SchemaFieldArgs],
) -> TokenStream {
    // Check if there's a ptr field
    let has_ptr = fields.iter().any(|f| f.is_ptr_field());

    // Generate field initializers for from_ptr
    let from_ptr_impl = if has_ptr {
        let field_inits: Vec<_> = fields
            .iter()
            .filter(|f| !f.is_ptr_field())
            .filter_map(|f| {
                let ident = f.ident.as_ref()?;
                if is_phantom_data(&f.ty) {
                    Some(quote! { #ident: ::std::marker::PhantomData })
                } else {
                    Some(quote! { #ident: ::std::default::Default::default() })
                }
            })
            .collect();

        quote! {
            unsafe fn from_ptr(ptr: *mut ::std::ffi::c_void) -> Option<Self> {
                if ptr.is_null() {
                    None
                } else {
                    Some(Self {
                        ptr,
                        #(#field_inits),*
                    })
                }
            }
        }
    } else {
        quote! {
            unsafe fn from_ptr(_ptr: *mut ::std::ffi::c_void) -> Option<Self> {
                None
            }
        }
    };

    quote! {
        impl ::cs2rust_core::schema::SchemaObject for #struct_name {
            fn ptr(&self) -> *mut ::std::ffi::c_void {
                self.ptr
            }

            fn class_name(&self) -> &'static str {
                #class_name
            }

            fn is_valid(&self) -> bool {
                !self.ptr.is_null()
            }

            #from_ptr_impl
        }
    }
}

fn generate_constructor(_struct_name: &syn::Ident, fields: &[SchemaFieldArgs]) -> TokenStream {
    // Check if there's a ptr field
    let has_ptr = fields.iter().any(|f| f.is_ptr_field());

    if has_ptr {
        // from_ptr is now part of SchemaObject trait impl
        // Just generate the as_ptr helper here
        quote! {
            /// Get the raw pointer
            pub fn as_ptr(&self) -> *mut ::std::ffi::c_void {
                self.ptr
            }
        }
    } else {
        quote! {}
    }
}
