use stewart::Handler;
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
    /// Insert data into a free region of the file, potentially appending.
    Insert(InsertAction),
}

/// Read a section of data.
pub struct ReadAction {
    pub offset: u64,
    pub size: u64,
    pub on_result: Handler<ReadResponse>,
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
    pub on_result: Handler<WriteResponse>,
}

/// Result of `WriteAction`.
pub struct WriteResponse {
    /// Identifier of originating message.
    pub id: Uuid,
    /// Result of the write action.
    pub result: Result<(), Error>,
}

/// Insert data into a free region of the file, potentially appending.
pub struct InsertAction {
    pub data: Vec<u8>,
    pub on_result: Handler<InsertResponse>,
}

/// Result of `InsertAction`.
pub struct InsertResponse {
    /// Identifier of originating message.
    pub id: Uuid,
    /// Result of the write action, containing the offset written to.
    pub result: Result<u64, Error>,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("action not supported on file")]
    NotSupported,
    #[error("out of space to insert data")]
    OutOfSpace,
    #[error("internal error")]
    InternalError { error: String },
}
