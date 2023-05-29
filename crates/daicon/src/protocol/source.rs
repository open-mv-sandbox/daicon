use anyhow::Error;
use stewart::Sender;
use uuid::Uuid;

// We use this in the protocol, so re-export it.
pub use daicon_types::Id;

pub struct Message {
    pub id: Uuid,
    pub action: Action,
}

pub enum Action {
    /// Get the data associated with an ID.
    Get(ActionGet),
    /// Set the data associated with an ID.
    Set(ActionSet),
}

pub struct ActionGet {
    pub id: Id,
    pub on_result: Sender<ActionGetResponse>,
}

pub struct ActionGetResponse {
    pub id: Uuid,
    pub result: Result<Vec<u8>, Error>,
}

pub struct ActionSet {
    pub id: Id,
    pub data: Vec<u8>,
    pub on_result: Sender<ActionSetResponse>,
}

pub struct ActionSetResponse {
    pub id: Uuid,
    pub result: Result<(), Error>,
}
