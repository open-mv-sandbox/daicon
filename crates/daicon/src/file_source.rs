use std::collections::HashMap;

use anyhow::{Context as _, Error};
use daicon_types::Id;
use stewart::{Actor, Context, Sender, State};
use tracing::{event, instrument, Level};
use uuid::Uuid;

use crate::{
    indices::{self, Action, GetAction, SetAction},
    protocol::{file, source},
    FileSourceOptions,
};

/// Open a file as a daicon source.
#[instrument("daicon::open_file_source", skip_all)]
pub fn open_file_source(
    ctx: &mut Context,
    file: Sender<file::Message>,
    options: FileSourceOptions,
) -> Result<Sender<source::Message>, Error> {
    event!(Level::INFO, "opening");

    let (mut ctx, sender) = ctx.create("daicon-file-source")?;

    let indices = indices::start(&mut ctx, file.clone(), options)?;

    // Start the root manager actor
    let actor = Service {
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

struct Service {
    sender: Sender<ImplMessage>,
    file: Sender<file::Message>,
    indices: Sender<indices::Message>,

    // Ongoing tracked actions
    get_tasks: HashMap<Uuid, PendingGet>,
    set_tasks: HashMap<Uuid, PendingSet>,
}

struct PendingGet {
    on_result: Sender<source::GetResponse>,
}

struct PendingSet {
    id: Id,
    size: u32,
    on_result: Sender<source::SetResponse>,
}

enum ImplMessage {
    Message(source::Message),
    GetIndexResult((Uuid, u64, u32)),
    SetWriteDataResult(file::AppendResponse),
}

impl Actor for Service {
    type Message = ImplMessage;

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

impl Service {
    fn on_message(&mut self, ctx: &mut Context, message: source::Message) -> Result<(), Error> {
        match message.action {
            source::Action::Get(action) => {
                self.on_get(ctx, message.id, action)?;
            }
            source::Action::Set(action) => {
                self.on_set(ctx, message.id, action)?;
            }
            source::Action::List(action) => {
                self.on_list(ctx, message.id, action)?;
            }
        }

        Ok(())
    }

    fn on_get(
        &mut self,
        ctx: &mut Context,
        id: Uuid,
        action: source::GetAction,
    ) -> Result<(), Error> {
        event!(Level::INFO, id = ?action.id, "received get");

        // Track the get task
        let task = PendingGet {
            on_result: action.on_result,
        };
        self.get_tasks.insert(id, task);

        // Fetch the entry
        self.send_read_index(ctx, id, action.id);

        Ok(())
    }

    fn on_set(
        &mut self,
        ctx: &mut Context,
        id: Uuid,
        action: source::SetAction,
    ) -> Result<(), Error> {
        event!(
            Level::INFO,
            id = ?action.id,
            bytes = action.data.len(),
            "received set",
        );

        // Track the request
        let task = PendingSet {
            id: action.id,
            size: action.data.len() as u32,
            on_result: action.on_result,
        };
        self.set_tasks.insert(id, task);

        // Append the data to the file
        self.send_write_data(ctx, id, action.data);

        Ok(())
    }

    fn on_list(
        &mut self,
        ctx: &mut Context,
        id: Uuid,
        action: source::ListAction,
    ) -> Result<(), Error> {
        // TODO: Actually do this
        let error = source::Error::InternalError {
            error: "not yet implemented".to_string(),
        };
        let response = source::ListResponse {
            id,
            result: Err(error),
        };
        action.on_result.send(ctx, response);
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
        result: file::AppendResponse,
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
        let action = GetAction { id, on_result };
        let message = indices::Message {
            id: action_id,
            action: Action::Get(action),
        };
        self.indices.send(ctx, message);
    }

    fn send_write_index(&self, ctx: &mut Context, id: Uuid, task: PendingSet, offset: u64) {
        let action = SetAction {
            id: task.id,
            offset,
            size: task.size,
            on_result: task
                .on_result
                .map(move |_| source::SetResponse { id, result: Ok(()) }),
        };
        let message = indices::Message {
            id,
            action: Action::Set(action),
        };
        self.indices.send(ctx, message);
    }

    fn send_read_data(
        &self,
        ctx: &mut Context,
        id: Uuid,
        offset: u64,
        size: u32,
        on_result: Sender<source::GetResponse>,
    ) {
        let action = file::ReadAction {
            offset,
            size: size as u64,
            on_result: on_result.map(|f: file::ReadResponse| source::GetResponse {
                id: f.id,
                result: f.result.map_err(|e| source::Error::InternalError {
                    error: e.to_string(),
                }),
            }),
        };
        let message = file::Message {
            id,
            action: file::Action::Read(action),
        };
        self.file.send(ctx, message);
    }

    fn send_write_data(&self, ctx: &mut Context, id: Uuid, data: Vec<u8>) {
        let action = file::AppendAction {
            data,
            on_result: self.sender.clone().map(ImplMessage::SetWriteDataResult),
        };
        let message = file::Message {
            id,
            action: file::Action::Append(action),
        };
        self.file.send(ctx, message);
    }
}
