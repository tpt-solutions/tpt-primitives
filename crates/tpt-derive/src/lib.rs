//! `#[derive(Canonical)]` for `tpt-primitives` canonical types.
//!
//! Implementing `Canonical` and `Identified` by hand is three lines of
//! boilerplate per type, and it is easy to get the schema version or the
//! identity class wrong. This derive writes them from one attribute:
//!
//! ```ignore
//! #[derive(serde::Serialize, serde::Deserialize, tpt_derive::Canonical)]
//! #[canonical(version = 2, id = ClaimTag, validate = check_claim)]
//! pub struct Claim { /* ... */ }
//! ```
//!
//! - `version` (required): the `SCHEMA_VERSION` (docs/evolution.md).
//! - `id` (optional): the identity class; generates `Identified`. Omit it for
//!   types that are content but not identities.
//! - `validate` (optional): a `fn(&Self) -> Result<(), CanonicalError>` run on
//!   decode (`Canonical::validate`).
//!
//! The type must also derive `serde::Serialize` and `serde::Deserialize`.
//! Generic types are not supported; canonical types in this workspace are
//! concrete.

use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, LitInt, Path, parse_macro_input};

/// Derive `Canonical` (and `Identified` when `id` is given).
#[proc_macro_derive(Canonical, attributes(canonical))]
pub fn derive_canonical(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand(&input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn expand(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.generics,
            "tpt-derive: generic types are not supported",
        ));
    }

    let mut version: Option<u16> = None;
    let mut id: Option<Path> = None;
    let mut validate: Option<Path> = None;
    for attr in &input.attrs {
        if !attr.path().is_ident("canonical") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("version") {
                let lit: LitInt = meta.value()?.parse()?;
                version = Some(lit.base10_parse()?);
                Ok(())
            } else if meta.path.is_ident("id") {
                id = Some(meta.value()?.parse()?);
                Ok(())
            } else if meta.path.is_ident("validate") {
                validate = Some(meta.value()?.parse()?);
                Ok(())
            } else {
                Err(meta.error("expected `version`, `id` or `validate`"))
            }
        })?;
    }

    let version = version.ok_or_else(|| {
        syn::Error::new_spanned(
            &input.ident,
            "tpt-derive: missing #[canonical(version = N)]",
        )
    })?;
    let name = &input.ident;
    let validate_body = validate.map_or_else(
        || quote! { ::core::result::Result::Ok(()) },
        |path| quote! { #path(self) },
    );
    let identified = id.map(|tag| {
        quote! {
            impl ::tpt_primitives::canonical::Identified for #name {
                type Id = #tag;
            }
        }
    });

    Ok(quote! {
        impl ::tpt_primitives::canonical::Canonical for #name {
            const SCHEMA_VERSION: u16 = #version;

            fn validate(&self) -> ::core::result::Result<(), ::tpt_primitives::CanonicalError> {
                #validate_body
            }
        }

        #identified
    })
}
