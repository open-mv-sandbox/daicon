use anyhow::Error;
use stewart::Sender;
use uuid::Uuid;

/// Message to a file reader/writer actor.
///
/// A "file" is an addressable blob of binary data, not necessarily a system file.
pub struct Message {
    pub id: Uuid,
    pub action: Action,
}

/// Action to perform on a file.
pub enum Action {
    Read(ActionRead),
    Write(ActionWrite),
}

pub struct ActionRead {
    pub offset: u64,
    pub size: u64,
    pub on_result: Sender<ActionReadResponse>,
}

/// Result of `FileRead`.
pub struct ActionReadResponse {
    /// Identifier of originating message.
    pub id: Uuid,
    /// Result of the read action, containing the data read.
    pub result: Result<Vec<u8>, Error>,
}

pub struct ActionWrite {
    pub location: Location,
    pub data: Vec<u8>,
    pub on_result: Sender<ActionWriteResponse>,
}

/// Location for `FileWrite`.
pub enum Location {
    Offset(u64),
    Append,
}

/// Result of `FileWrite`.
pub struct ActionWriteResponse {
    /// Identifier of originating message.
    pub id: Uuid,
    /// Result of the write action, containing the offset written to.
    pub result: Result<u64, Error>,
}
