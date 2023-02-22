//! Simple wrapper generator around bytemuck pod types.
//!
//! Sometimes, you want to expose raw byte bytemuck conversions, but not actually publicly depend
//! on bytemuck. This crate's derive macro automatically implements wrapping functions.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(Wrapmuck)]
pub fn derive_wrapmuck(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let DeriveInput { ident, .. } = input;

    let expanded = quote! {
        impl #ident {
            /// Create a new zeroed value.
            pub fn zeroed() -> Self {
                let value = Self(bytemuck::Zeroable::zeroed());
                value
            }

            /// Get the size of the value in bytes.
            pub fn bytes_len() -> usize {
                std::mem::size_of::<#ident>()
            }

            /// Convert a byte slice to a value reference.
            pub fn from_bytes(bytes: &[u8]) -> &Self {
                bytemuck::TransparentWrapper::wrap_ref(bytemuck::from_bytes(bytes))
            }

            /// Convert a mutable byte slice to a mutable value reference.
            pub fn from_bytes_mut(bytes: &mut [u8]) -> &mut Self {
                bytemuck::TransparentWrapper::wrap_mut(bytemuck::from_bytes_mut(bytes))
            }
        }

        impl std::ops::Deref for #ident {
            type Target = [u8];

            fn deref(&self) -> &Self::Target {
                bytemuck::bytes_of(&self.0)
            }
        }

        impl std::ops::DerefMut for #ident {
            fn deref_mut(&mut self) -> &mut Self::Target {
                bytemuck::bytes_of_mut(&mut self.0)
            }
        }
    };

    TokenStream::from(expanded)
}
