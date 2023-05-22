use std::collections::HashMap;

use anyhow::{Context as _, Error};
use stewart::{Actor, Addr, Context, Options, State, World};
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
    ctx: &mut Context,
    file: Addr<FileMessage>,
    mode: OpenMode,
) -> Result<Addr<SourceMessage>, Error> {
    event!(Level::INFO, ?mode, "opening source");

    let mut ctx = ctx.create(Options::default())?;
    let addr = ctx.addr()?;

    let source = map(&mut ctx, addr, Message::Action)?;

    let indices = indices::start(&mut ctx, file, mode)?;

    // Start the root manager actor
    let instance = FileSource {
        file,
        indices,

        get_tasks: HashMap::new(),
        set_tasks: HashMap::new(),
    };
    ctx.start(instance)?;

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
    fn process(&mut self, ctx: &mut Context, state: &mut State<Self>) -> Result<(), Error> {
        while let Some(message) = state.next() {
            match message {
                Message::Action(message) => {
                    self.on_source_message(ctx, message)?;
                }
                Message::GetIndexResult((action_id, offset, size)) => {
                    self.on_get_index_result(ctx, action_id, offset, size)?;
                }
                Message::SetWriteDataResult(result) => {
                    self.on_set_write_data_result(ctx, result)?;
                }
            }
        }

        Ok(())
    }
}

impl FileSource {
    fn on_source_message(
        &mut self,
        ctx: &mut Context,
        message: SourceMessage,
    ) -> Result<(), Error> {
        match message.action {
            SourceAction::Get { id, on_result } => {
                self.on_get(ctx, message.id, id, on_result)?;
            }
            SourceAction::Set {
                id,
                data,
                on_result,
            } => {
                self.on_set(ctx, message.id, id, data, on_result)?;
            }
        }

        Ok(())
    }

    fn on_get(
        &mut self,
        ctx: &mut Context,
        action_id: Uuid,
        id: u32,
        on_result: Addr<ReadResult>,
    ) -> Result<(), Error> {
        event!(Level::INFO, "received get {:#010x}", id);

        // Track the get task
        let task = GetTask { on_result };
        self.get_tasks.insert(action_id, task);

        // Fetch the entry
        let on_result = map_once(ctx, ctx.addr()?, Message::GetIndexResult)?;
        let message = IndicesMessage {
            id: action_id,
            action: IndicesAction::Get { id, on_result },
        };
        ctx.send(self.indices, message);

        Ok(())
    }

    fn on_get_index_result(
        &mut self,
        ctx: &mut Context,
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
        ctx.send(self.file, message);

        Ok(())
    }

    fn on_set(
        &mut self,
        ctx: &mut Context,
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
                on_result: map(ctx, ctx.addr()?, Message::SetWriteDataResult)?,
            },
        };
        ctx.send(self.file, message);

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
