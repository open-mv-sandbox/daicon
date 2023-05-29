use anyhow::Error;
use stewart::Sender;
use uuid::Uuid;

/// Message to a file reader/writer actor.
///
/// A "file" is an addressable blob of binary data, not necessarily a system file.
pub struct FileMessage {
    pub id: Uuid,
    pub action: FileAction,
}

/// Action to perform on a file.
pub enum FileAction {
    Read(FileRead),
    Write(FileWrite),
}

pub struct FileRead {
    pub offset: u64,
    pub size: u64,
    pub on_result: Sender<FileReadResponse>,
}

/// Result of `FileRead`.
pub struct FileReadResponse {
    /// Identifier of originating message.
    pub id: Uuid,
    /// Result of the read action, containing the data read.
    pub result: Result<Vec<u8>, Error>,
}

pub struct FileWrite {
    pub location: WriteLocation,
    pub data: Vec<u8>,
    pub on_result: Sender<FileWriteResponse>,
}

/// Location for `FileWrite`.
pub enum WriteLocation {
    Offset(u64),
    Append,
}

/// Result of `FileWrite`.
pub struct FileWriteResponse {
    /// Identifier of originating message.
    pub id: Uuid,
    /// Result of the write action, containing the offset written to.
    pub result: Result<u64, Error>,
}
