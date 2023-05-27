//! Protocol interface types.

mod file;
mod source;

pub use self::{
    file::{FileAction, FileMessage, FileRead, FileWrite, ReadResult, WriteLocation, WriteResult},
    source::{SourceAction, SourceGet, SourceMessage, SourceSet},
};
