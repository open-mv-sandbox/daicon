use stewart::Handler;
use thiserror::Error;
use uuid::Uuid;

/// Message to a file reader/writer actor.
///
/// A "file" is an addressable blob of binary data, not necessarily a system file.
/// This is different from a "stream" which is a read/write endpoint that *cannot* be addressed.
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
pub struct WriteAction {
    /// If given, the offset to write to.
    ///
    /// If `Some`, the region must be already a valid region of the file.
    ///
    /// If `None`, the implementation will allocate a free region of the file.
    /// This can append, or find an implementation-specific free region.
    pub offset: Option<u64>,
    pub data: Vec<u8>,
    pub on_result: Handler<WriteResponse>,
}

/// Result of `WriteAction`.
pub struct WriteResponse {
    /// Identifier of originating message.
    pub id: Uuid,
    /// Result of the write action.
    pub result: Result<u64, Error>,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("action not supported on file")]
    NotSupported,
    #[error("write allocation failed on file")]
    WriteAllocationFailed,
    #[error("internal error")]
    InternalError { error: String },
}
