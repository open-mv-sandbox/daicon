use std::collections::HashMap;

use anyhow::{Context, Error};
use stewart::{Actor, ActorId, Addr, Options, State, World};
use stewart_utils::{map, map_once};
use tracing::{event, instrument, Level};
use uuid::Uuid;

use crate::{
    file::{FileAction, FileMessage, ReadResult, WriteLocation, WriteResult},
    indices::{self, IndicesAction, IndicesMessage},
    OpenMode,
};

/// Open a file as a daicon source.
///
/// A "source" returns a file from UUIDs. A "file source" uses a file as a source.
#[instrument("source", skip_all)]
pub fn open_source(
    world: &mut World,
    parent: Option<ActorId>,
    file: Addr<FileMessage>,
    mode: OpenMode,
) -> Result<Addr<SourceMessage>, Error> {
    event!(Level::INFO, ?mode, "opening source");

    let id = world.create(parent, Options::default())?;
    let addr = Addr::new(id);

    let source = map(world, Some(id), addr, Message::Action)?;

    let indices = indices::start(world, Some(id), file, mode)?;

    // Start the root manager actor
    let instance = FileSource {
        id,
        file,
        indices,

        get_tasks: HashMap::new(),
        set_tasks: HashMap::new(),
    };
    world.start(id, instance)?;

    Ok(source)
}

pub struct SourceMessage {
    pub id: Uuid,
    pub action: SourceAction,
}

pub enum SourceAction {
    /// Get the data associated with a UUID.
    Get {
        id: u32,
        /// TODO: Reply with an inner file actor Addr instead.
        on_result: Addr<ReadResult>,
    },
    /// Set the data associated with a UUID.
    Set {
        id: u32,
        data: Vec<u8>,
        on_result: Addr<()>,
    },
}

struct FileSource {
    id: ActorId,
    file: Addr<FileMessage>,
    indices: Addr<IndicesMessage>,

    // Ongoing tracked requests
    get_tasks: HashMap<Uuid, GetTask>,
    set_tasks: HashMap<Uuid, SetTask>,
}

struct GetTask {
    on_result: Addr<ReadResult>,
}

struct SetTask {
    id: u32,
    size: u32,
    on_result: Addr<()>,
}

impl Actor for FileSource {
    type Message = Message;

    #[instrument("source", skip_all)]
    fn process(&mut self, world: &mut World, state: &mut State<Self>) -> Result<(), Error> {
        while let Some(message) = state.next() {
            match message {
                Message::Action(message) => {
                    self.on_source_message(world, message)?;
                }
                Message::GetIndexResult((action_id, offset, size)) => {
                    self.on_get_index_result(world, action_id, offset, size)?;
                }
                Message::SetWriteDataResult(result) => {
                    self.on_set_write_data_result(world, result)?;
                }
            }
        }

        Ok(())
    }
}

impl FileSource {
    fn on_source_message(
        &mut self,
        world: &mut World,
        message: SourceMessage,
    ) -> Result<(), Error> {
        match message.action {
            SourceAction::Get { id, on_result } => {
                self.on_get(world, message.id, id, on_result)?;
            }
            SourceAction::Set {
                id,
                data,
                on_result,
            } => {
                self.on_set(world, message.id, id, data, on_result)?;
            }
        }

        Ok(())
    }

    fn on_get(
        &mut self,
        world: &mut World,
        action_id: Uuid,
        id: u32,
        on_result: Addr<ReadResult>,
    ) -> Result<(), Error> {
        event!(Level::INFO, "received get {:#010x}", id);

        // Track the get task
        let task = GetTask { on_result };
        self.get_tasks.insert(action_id, task);

        // Fetch the entry
        let on_result = map_once(
            world,
            Some(self.id),
            Addr::new(self.id),
            Message::GetIndexResult,
        )?;
        let message = IndicesMessage {
            id: action_id,
            action: IndicesAction::Get { id, on_result },
        };
        world.send(self.indices, message);

        Ok(())
    }

    fn on_get_index_result(
        &mut self,
        world: &mut World,
        action_id: Uuid,
        offset: u64,
        size: u32,
    ) -> Result<(), Error> {
        // We've got the location of the data, so perform the read
        let task = self
            .get_tasks
            .remove(&action_id)
            .context("failed to find get task")?;

        let message = FileMessage {
            id: action_id,
            action: FileAction::Read {
                offset,
                size: size as u64,
                on_result: task.on_result,
            },
        };
        world.send(self.file, message);

        Ok(())
    }

    fn on_set(
        &mut self,
        world: &mut World,
        action_id: Uuid,
        id: u32,
        data: Vec<u8>,
        on_result: Addr<()>,
    ) -> Result<(), Error> {
        event!(
            Level::INFO,
            id = ?action_id,
            bytes = data.len(),
            "received set {:#010x}",
            id
        );

        // Append the data to the file
        let size = data.len() as u32;
        let message = FileMessage {
            id: action_id,
            action: FileAction::Write {
                location: WriteLocation::Append,
                data,
                on_result: map(
                    world,
                    Some(self.id),
                    Addr::new(self.id),
                    Message::SetWriteDataResult,
                )?,
            },
        };
        world.send(self.file, message);

        // Track the request
        let task = SetTask {
            id,
            size,
            on_result,
        };
        self.set_tasks.insert(action_id, task);

        Ok(())
    }

    fn on_set_write_data_result(
        &mut self,
        world: &mut World,
        result: WriteResult,
    ) -> Result<(), Error> {
        event!(Level::DEBUG, id = ?result.id, "received data write result");

        let task = self
            .set_tasks
            .get_mut(&result.id)
            .context("failed to get pending set task")?;

        // Write the entry
        let message = IndicesMessage {
            id: result.id,
            action: IndicesAction::Set {
                id: task.id,
                offset: result.offset,
                size: task.size,
                on_result: task.on_result,
            },
        };
        world.send(self.indices, message);

        Ok(())
    }
}

enum Message {
    Action(SourceMessage),
    GetIndexResult((Uuid, u64, u32)),
    SetWriteDataResult(WriteResult),
}
