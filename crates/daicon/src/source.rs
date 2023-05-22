use std::collections::HashMap;

use anyhow::{Context as _, Error};
use stewart::{Actor, Context, Options, Sender, State};
use tracing::{event, instrument, Level};
use uuid::Uuid;

use crate::{
    file::{FileAction, FileMessage, ReadResult, WriteLocation, WriteResult},
    indices::{self, GetIndex, IndicesAction, IndicesMessage, SetIndex},
    OpenMode,
};

/// Open a file as a daicon source.
///
/// A "source" returns a file from UUIDs. A "file source" uses a file as a source.
#[instrument("source", skip_all)]
pub fn open_source(
    ctx: &mut Context,
    file: Sender<FileMessage>,
    mode: OpenMode,
) -> Result<Sender<SourceMessage>, Error> {
    event!(Level::INFO, ?mode, "opening source");

    let (mut ctx, sender) = ctx.create(Options::default())?;

    let indices = indices::start(&mut ctx, file.clone(), mode)?;

    // Start the root manager actor
    let instance = Source {
        sender: sender.clone(),
        file,
        indices,

        get_tasks: HashMap::new(),
        set_tasks: HashMap::new(),
    };
    ctx.start(instance)?;

    let sender = sender.map(Message::Action);
    Ok(sender)
}

pub struct SourceMessage {
    pub id: Uuid,
    pub action: SourceAction,
}

pub enum SourceAction {
    /// Get the data associated with a UUID.
    Get(SourceGet),
    /// Set the data associated with a UUID.
    Set(SourceSet),
}

pub struct SourceGet {
    pub id: u32,
    pub on_result: Sender<ReadResult>,
}

pub struct SourceSet {
    pub id: u32,
    pub data: Vec<u8>,
    pub on_result: Sender<()>,
}

struct Source {
    sender: Sender<Message>,
    file: Sender<FileMessage>,
    indices: Sender<IndicesMessage>,

    // Ongoing tracked requests
    get_tasks: HashMap<Uuid, GetTask>,
    set_tasks: HashMap<Uuid, SetTask>,
}

struct GetTask {
    on_result: Sender<ReadResult>,
}

struct SetTask {
    id: u32,
    size: u32,
    on_result: Sender<()>,
}

impl Actor for Source {
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

impl Source {
    fn on_source_message(
        &mut self,
        ctx: &mut Context,
        message: SourceMessage,
    ) -> Result<(), Error> {
        match message.action {
            SourceAction::Get(action) => {
                self.on_get(ctx, message.id, action)?;
            }
            SourceAction::Set(action) => {
                self.on_set(ctx, message.id, action)?;
            }
        }

        Ok(())
    }

    fn on_get(&mut self, ctx: &mut Context, id: Uuid, action: SourceGet) -> Result<(), Error> {
        event!(Level::INFO, "received get {:#010x}", action.id);

        // Track the get task
        let task = GetTask {
            on_result: action.on_result,
        };
        self.get_tasks.insert(id, task);

        // Fetch the entry
        let on_result = self.sender.clone().map(Message::GetIndexResult);
        let action = GetIndex {
            id: action.id,
            on_result,
        };
        let message = IndicesMessage {
            id,
            action: IndicesAction::Get(action),
        };
        self.indices.send(ctx, message);

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
        self.file.send(ctx, message);

        Ok(())
    }

    fn on_set(&mut self, ctx: &mut Context, id: Uuid, action: SourceSet) -> Result<(), Error> {
        event!(
            Level::INFO,
            id = ?id,
            bytes = action.data.len(),
            "received set {:#010x}",
            action.id
        );

        // Append the data to the file
        let size = action.data.len() as u32;
        let message = FileMessage {
            id,
            action: FileAction::Write {
                location: WriteLocation::Append,
                data: action.data,
                on_result: self.sender.clone().map(Message::SetWriteDataResult),
            },
        };
        self.file.send(ctx, message);

        // Track the request
        let task = SetTask {
            id: action.id,
            size,
            on_result: action.on_result,
        };
        self.set_tasks.insert(id, task);

        Ok(())
    }

    fn on_set_write_data_result(
        &mut self,
        ctx: &mut Context,
        result: WriteResult,
    ) -> Result<(), Error> {
        event!(Level::DEBUG, id = ?result.id, "received data write result");

        let task = self
            .set_tasks
            .get_mut(&result.id)
            .context("failed to get pending set task")?;

        // Write the entry
        let action = SetIndex {
            id: task.id,
            offset: result.offset,
            size: task.size,
            on_result: task.on_result.clone(),
        };
        let message = IndicesMessage {
            id: result.id,
            action: IndicesAction::Set(action),
        };
        self.indices.send(ctx, message);

        Ok(())
    }
}

enum Message {
    Action(SourceMessage),
    GetIndexResult((Uuid, u64, u32)),
    SetWriteDataResult(WriteResult),
}
