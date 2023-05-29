//! Protocol interface types.

mod file;
mod source;

pub use self::{
    file::{
        FileAction, FileMessage, FileRead, FileReadResponse, FileWrite, FileWriteResponse,
        WriteLocation,
    },
    source::{SourceAction, SourceGet, SourceGetResponse, SourceMessage, SourceSet, SourceSetResponse},
};

// We use this in the protocol, so re-export it.
pub use daicon_types::Id;
