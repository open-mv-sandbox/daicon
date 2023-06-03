//! Reference rust reader/writer implementation of the daicon format.
//!
//! # Sources
//!
//! Daicon lookup is abstracted as a "source", which lets you look up data by ID.
//! Higher level abstractions, such as error checking, can be implemented by implementing the
//! source protocol on top of another source.

mod file_source;
pub mod protocol;

pub use self::file_source::{open_file_source, FileSourceOptions};
