use anyhow::Error;
use daicon_types::Id;
use stewart::Sender;
use uuid::Uuid;

pub struct SourceMessage {
    pub id: Uuid,
    pub action: SourceAction,
}

pub enum SourceAction {
    /// Get the data associated with an ID.
    Get(SourceGet),
    /// Set the data associated with an ID.
    Set(SourceSet),
}

pub struct SourceGet {
    pub id: Id,
    pub on_result: Sender<SourceGetResponse>,
}

pub struct SourceGetResponse {
    pub id: Uuid,
    pub result: Result<Vec<u8>, Error>,
}

pub struct SourceSet {
    pub id: Id,
    pub data: Vec<u8>,
    pub on_result: Sender<SourceSetResponse>,
}

pub struct SourceSetResponse {
    pub id: Uuid,
    pub result: Result<(), Error>,
}
