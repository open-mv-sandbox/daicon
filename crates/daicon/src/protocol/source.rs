use stewart::Handler;
use thiserror::Error;
use uuid::Uuid;

// We use this in the protocol, so re-export it.
pub use daicon_types::Id;

/// Source action message.
///
/// TODO: Consider if "message" could be higher level and more generic.
/// This could probably be the basis of a broader message distribution system.
/// Such a system could make use of the ID system to route messages and responses, in a way that
/// can be serialized and converted back and forth.
pub struct Message {
    pub id: Uuid,
    pub action: Action,
}

pub enum Action {
    /// Get the data associated with an ID.
    Get(GetAction),
    /// Set the data associated with an ID.
    Set(SetAction),
    /// Get a list of all indices in the source.
    List(ListAction),
}

/// Get the data associated with an ID.
pub struct GetAction {
    pub id: Id,
    pub on_result: Handler<GetResponse>,
}

pub struct GetResponse {
    pub id: Uuid,
    pub result: Result<Vec<u8>, Error>,
}

/// Set the data associated with an ID.
pub struct SetAction {
    pub id: Id,
    pub data: Vec<u8>,
    pub on_result: Handler<SetResponse>,
}

pub struct SetResponse {
    pub id: Uuid,
    pub result: Result<(), Error>,
}

/// Get a list of all indices in the source.
pub struct ListAction {
    pub on_result: Handler<ListResponse>,
}

pub struct ListResponse {
    pub id: Uuid,
    pub result: Result<(), Error>,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("internal error")]
    InternalError { error: String },
}
