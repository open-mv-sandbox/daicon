//! Rust implementation of the daicon format.

mod cache;
pub mod file;
mod file_source;
mod set;

use stewart::Addr;
use uuid::Uuid;

use crate::file::ReadResult;

pub use self::file_source::{open_file_source, OpenMode};

pub struct SourceMessage {
    pub id: Uuid,
    pub action: SourceAction,
}

pub enum SourceAction {
    /// Get the data associated with a UUID.
    Get {
        id: u64,
        /// TODO: Reply with an inner file actor Addr instead.
        on_result: Addr<ReadResult>,
    },
    /// Set the data associated with a UUID.
    Set {
        id: u64,
        data: Vec<u8>,
        on_result: Addr<()>,
    },
}
