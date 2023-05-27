use daicon_types::Id;
use stewart::Sender;
use uuid::Uuid;

use crate::protocol::ReadResult;

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
    pub on_result: Sender<ReadResult>,
}

pub struct SourceSet {
    pub id: Id,
    pub data: Vec<u8>,
    pub on_result: Sender<()>,
}
