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
    Read(ReadAction),
    /// Write a section of data.
    Write(WriteAction),
    /// Append data to the end of the file.
    Append(AppendAction),
}

/// Read a section of data.
pub struct ReadAction {
    pub offset: u64,
    pub size: u64,
    pub on_result: Sender<ReadResponse>,
}

/// Result of `ReadAction`.
pub struct ReadResponse {
    /// Identifier of originating message.
    pub id: Uuid,
    /// Result of the read action, containing the data read.
    pub result: Result<Vec<u8>, Error>,
}

/// Write a section of data.
///
/// The range of data has to be within the file, first validate that the region exists.
/// If you want to add new data, append it to the end of the file.
/// TODO: This requirement is not yet implemented in file implementations.
pub struct WriteAction {
    pub offset: u64,
    pub data: Vec<u8>,
    pub on_result: Sender<WriteResponse>,
}

/// Result of `WriteAction`.
pub struct WriteResponse {
    /// Identifier of originating message.
    pub id: Uuid,
    /// Result of the write action.
    pub result: Result<(), Error>,
}

/// Append data to the end of the file.
pub struct AppendAction {
    pub data: Vec<u8>,
    pub on_result: Sender<AppendResponse>,
}

/// Result of `AppendAction`.
pub struct AppendResponse {
    /// Identifier of originating message.
    pub id: Uuid,
    /// Result of the write action, containing the offset written to.
    pub result: Result<u64, Error>,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("action not supported by the file")]
    ActionNotSupported,
    #[error("internal error")]
    InternalError { error: String },
}
