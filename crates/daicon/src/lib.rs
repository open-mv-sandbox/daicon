//! Reference rust reader/writer implementation of the daicon format.

pub mod file;
mod set;
mod source;
mod state;

use stewart::Addr;
use uuid::Uuid;

use crate::file::ReadResult;

pub use self::source::{open_file_source, OpenMode};

pub struct SourceMessage {
    pub id: Uuid,
    pub action: SourceAction,
}

pub enum SourceAction {
    /// Get the data associated with a UUID.
    Get {
        id: u32,
        /// TODO: Reply with an inner file actor Addr instead.
        on_result: Addr<ReadResult>,
    },
    /// Set the data associated with a UUID.
    Set {
        id: u32,
        data: Vec<u8>,
        on_result: Addr<()>,
    },
}
