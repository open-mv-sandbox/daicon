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

pub struct OpenOptions {
    allocate_capacity: u16,
}

impl OpenOptions {
    /// Sets the default capacity of new created tables.
    pub fn allocate_capacity(mut self, value: u16) -> Self {
        self.allocate_capacity = value;
        self
    }
}

impl Default for OpenOptions {
    fn default() -> Self {
        Self {
            allocate_capacity: 256,
        }
    }
}
