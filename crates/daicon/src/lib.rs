//! Reference rust reader/writer implementation of the daicon format.

pub mod file;
mod indices;
mod source;

pub use self::source::{open_source, SourceAction, SourceMessage};

#[derive(Debug, PartialEq, Eq)]
pub enum OpenMode {
    ReadWrite,
    Create,
}
