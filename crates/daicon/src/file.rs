use std::collections::HashMap;

use anyhow::{Context as _, Error};
use daicon_types::Id;
use stewart::{Actor, Context, Sender, State};
use tracing::{event, instrument, Level};
use uuid::Uuid;

use crate::{
    indices::{self, IndexAction, IndexGet, IndexServiceMessage, IndexSet},
    protocol::{
        FileAction, FileMessage, FileRead, FileReadResponse, FileWrite, FileWriteResponse,
        SourceAction, SourceGet, SourceGetResponse, SourceMessage, SourceSet, SourceSetResponse,
        WriteLocation,
    },
    OpenMode, OpenOptions,
};

/// Open a file as a daicon source.
#[instrument("Source", skip_all)]
pub fn open_file_source(
    ctx: &mut Context,
    file: Sender<FileMessage>,
    mode: OpenMode,
    options: OpenOptions,
) -> Result<Sender<SourceMessage>, Error> {
    event!(Level::INFO, ?mode, "opening");

    let (mut ctx, sender) = ctx.create()?;

    let indices = indices::start(&mut ctx, file.clone(), mode, options)?;

    // Start the root manager actor
    let actor = Source {
        sender: sender.clone(),
        file,
        indices,

        get_tasks: HashMap::new(),
        set_tasks: HashMap::new(),
    };
    ctx.start(actor)?;

    let sender = sender.map(ImplMessage::Message);
    Ok(sender)
}

struct Source {
    sender: Sender<ImplMessage>,
    file: Sender<FileMessage>,
    indices: Sender<IndexServiceMessage>,

    // Ongoing tracked requests
    get_tasks: HashMap<Uuid, GetTask>,
    set_tasks: HashMap<Uuid, SetTask>,
}

struct GetTask {
    on_result: Sender<SourceGetResponse>,
}

struct SetTask {
    id: Id,
    size: u32,
    on_result: Sender<SourceSetResponse>,
}

enum ImplMessage {
    Message(SourceMessage),
    GetIndexResult((Uuid, u64, u32)),
    SetWriteDataResult(FileWriteResponse),
}

impl Actor for Source {
    type Message = ImplMessage;

    #[instrument("Source", skip_all)]
    fn process(&mut self, ctx: &mut Context, state: &mut State<Self>) -> Result<(), Error> {
        while let Some(message) = state.next() {
            match message {
                ImplMessage::Message(message) => {
                    self.on_message(ctx, message)?;
                }
                ImplMessage::GetIndexResult((action_id, offset, size)) => {
                    self.on_get_index_result(ctx, action_id, offset, size)?;
                }
                ImplMessage::SetWriteDataResult(result) => {
                    self.on_set_write_data_result(ctx, result)?;
                }
            }
        }

        Ok(())
    }
}

impl Source {
    fn on_message(&mut self, ctx: &mut Context, message: SourceMessage) -> Result<(), Error> {
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
        event!(Level::INFO, id = ?action.id, "received get");

        // Track the get task
        let task = GetTask {
            on_result: action.on_result,
        };
        self.get_tasks.insert(id, task);

        // Fetch the entry
        self.send_read_index(ctx, id, action.id);

        Ok(())
    }

    fn on_set(&mut self, ctx: &mut Context, id: Uuid, action: SourceSet) -> Result<(), Error> {
        event!(
            Level::INFO,
            id = ?action.id,
            bytes = action.data.len(),
            "received set",
        );

        // Track the request
        let task = SetTask {
            id: action.id,
            size: action.data.len() as u32,
            on_result: action.on_result,
        };
        self.set_tasks.insert(id, task);

        // Append the data to the file
        self.send_write_data(ctx, id, action.data);

        Ok(())
    }

    fn on_get_index_result(
        &mut self,
        ctx: &mut Context,
        id: Uuid,
        offset: u64,
        size: u32,
    ) -> Result<(), Error> {
        event!(Level::DEBUG, ?id, "received get index result");

        // Remove the task, we're done with it in this actor
        let task = self
            .get_tasks
            .remove(&id)
            .context("failed to find get task")?;

        // We've got the location of the data, so perform the read
        self.send_read_data(ctx, id, offset, size, task.on_result);

        Ok(())
    }

    fn on_set_write_data_result(
        &mut self,
        ctx: &mut Context,
        result: FileWriteResponse,
    ) -> Result<(), Error> {
        event!(Level::DEBUG, id = ?result.id, "received data write result");

        // Remove the task, we're done with it in this actor
        let task = self
            .set_tasks
            .remove(&result.id)
            .context("failed to get pending set task")?;

        // Write the index
        let offset = result.result?;
        self.send_write_index(ctx, result.id, task, offset);

        Ok(())
    }

    fn send_read_index(&self, ctx: &mut Context, action_id: Uuid, id: Id) {
        let on_result = self.sender.clone().map(ImplMessage::GetIndexResult);
        let action = IndexGet { id, on_result };
        let message = IndexServiceMessage {
            id: action_id,
            action: IndexAction::Get(action),
        };
        self.indices.send(ctx, message);
    }

    fn send_write_index(&self, ctx: &mut Context, id: Uuid, task: SetTask, offset: u64) {
        let action = IndexSet {
            id: task.id,
            offset,
            size: task.size,
            on_result: task
                .on_result
                .map(move |_| SourceSetResponse { id, result: Ok(()) }),
        };
        let message = IndexServiceMessage {
            id,
            action: IndexAction::Set(action),
        };
        self.indices.send(ctx, message);
    }

    fn send_read_data(
        &self,
        ctx: &mut Context,
        id: Uuid,
        offset: u64,
        size: u32,
        on_result: Sender<SourceGetResponse>,
    ) {
        let action = FileRead {
            offset,
            size: size as u64,
            on_result: on_result.map(|f: FileReadResponse| SourceGetResponse {
                id: f.id,
                result: f.result,
            }),
        };
        let message = FileMessage {
            id,
            action: FileAction::Read(action),
        };
        self.file.send(ctx, message);
    }

    fn send_write_data(&self, ctx: &mut Context, id: Uuid, data: Vec<u8>) {
        let file_action = FileWrite {
            location: WriteLocation::Append,
            data,
            on_result: self.sender.clone().map(ImplMessage::SetWriteDataResult),
        };
        let message = FileMessage {
            id,
            action: FileAction::Write(file_action),
        };
        self.file.send(ctx, message);
    }
}
