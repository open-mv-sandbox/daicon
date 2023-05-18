//! Pterodactil file abstraction.
//!
//! A "file" is an addressable blob of binary data.

mod buffer;
mod system;

use stewart::Addr;
use uuid::Uuid;

pub use self::{buffer::open_buffer_file, system::open_system_file};

/// Message to a file actor.
pub struct FileMessage {
    pub id: Uuid,
    pub action: FileAction,
}

/// Operation to perform on a file.
pub enum FileAction {
    Read {
        offset: u64,
        size: u64,
        on_result: Addr<ReadResult>,
    },
    Write {
        location: WriteLocation,
        data: Vec<u8>,
        on_result: Addr<WriteResult>,
    },
}

/// Location for `Operation::Write`.
pub enum WriteLocation {
    Offset(u64),
    Append,
}

/// Result of `Operation::Read`.
pub struct ReadResult {
    /// Identifier of originating message.
    pub id: Uuid,
    /// Resolved stream offset read from.
    pub offset: u64,
    /// Read data.
    pub data: Vec<u8>,
}

/// Result of `Operation::Write`.
pub struct WriteResult {
    /// Identifier of originating message.
    pub id: Uuid,
    /// Resolved stream offset written to.
    pub offset: u64,
}
