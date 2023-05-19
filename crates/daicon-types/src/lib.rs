//! Daicon low-level types, for zero-copy reading and writing.
//!
//! This library version is based off the daicon 0.2.0 specification.

mod entry;
mod header;

pub use self::{entry::Entry, header::Header};

/// Magic signature of a daicon 0.x.x header, literally equivalent to 0xFF followed by ASCII "dc0".
pub const SIGNATURE: u32 = 0x306364FF;
