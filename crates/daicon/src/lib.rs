//! Reference rust reader/writer implementation of the daicon format.
//!
//! # Sources
//!
//! Daicon lookup is abstracted as a "source", which lets you look up data by ID.
//! Higher level abstractions, such as error checking, can be implemented by implementing the
//! source protocol on top of another source.

mod indices;
pub mod protocol;
mod source;

pub use self::source::open_file_source;

#[derive(Debug, PartialEq, Eq)]
pub enum OpenMode {
    ReadWrite,
    Create,
}
