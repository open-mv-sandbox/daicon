//! Daicon low-level types, for zero-copy reading and writing.
//!
//! This library does not guarantee 100% correctness in input or output, but will provide minimal
//! validation where useful. In most cases, you should not use this library directly, but instead
//! use a format-specific library that uses this library.
//!
//! Where possible, high-level wrappers are `#[repr(transparent)]` to the low-level data of those
//! types, and can be reinterpreted. However, you should avoid doing this. Safe raw binary
//! conversion can be done instead with `from_bytes`, `from_bytes_mut`, and `Deref`.
//!
//! This library version is based off the daicon 0.1.1-draft spec.

mod entry;
mod header;
pub mod data;

pub use self::{entry::ComponentEntry, header::ComponentTableHeader};

/// Signature of a daicon file, should be inserted and validated at the top of a file.
pub const SIGNATURE: &[u8] = b"\xFFdaicon0";
