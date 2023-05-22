//! File operations interface.
//!
//! A "file" is an addressable blob of binary data, not necessarily a system file.

use stewart::Sender;
use uuid::Uuid;

/// Message to a file reader/writer actor.
pub struct FileMessage {
    pub id: Uuid,
    pub action: FileAction,
}

/// Operation to perform on a file.
pub enum FileAction {
    Read(FileRead),
    Write(FileWrite),
}

pub struct FileRead {
    pub offset: u64,
    pub size: u64,
    pub on_result: Sender<ReadResult>,
}

pub struct FileWrite {
    pub location: WriteLocation,
    pub data: Vec<u8>,
    pub on_result: Sender<WriteResult>,
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
