//! Protocol interface types.

mod file;
mod source;

pub use self::{
    file::{FileAction, FileMessage, FileRead, FileWrite, ReadResult, WriteLocation, WriteResult},
    source::{SourceAction, SourceGet, SourceMessage, SourceSet},
};

// We use this in the protocol, so re-export it.
pub use daicon_types::Id;
