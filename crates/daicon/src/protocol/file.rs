use stewart::Sender;
use thiserror::Error;
use uuid::Uuid;

/// Message to a file reader/writer actor.
///
/// A "file" is an addressable blob of binary data, not necessarily a system file.
pub struct Message {
    pub id: Uuid,
    pub action: Action,
}

/// Action to perform on a file.
///
/// Files may not support all actions, and return an error if an unsupported action is called.
pub enum Action {
    /// Read a section of data.
    Read(ActionRead),
    /// Write a section of data.
    Write(ActionWrite),
    /// Append new data to the end of the file.
    Append(ActionAppend),
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
    pub offset: u64,
    pub data: Vec<u8>,
    pub on_result: Sender<ActionWriteResponse>,
}

/// Result of `FileWrite`.
pub struct ActionWriteResponse {
    /// Identifier of originating message.
    pub id: Uuid,
    /// Result of the write action.
    pub result: Result<(), Error>,
}

pub struct ActionAppend {
    pub data: Vec<u8>,
    pub on_result: Sender<ActionAppendResponse>,
}

/// Result of `ActionAppend`.
pub struct ActionAppendResponse {
    /// Identifier of originating message.
    pub id: Uuid,
    /// Result of the write action, containing the offset written to.
    pub result: Result<u64, Error>,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("the action is not supported")]
    ActionNotSupported,
    #[error("internal error")]
    InternalError { error: String },
}
