use std::collections::HashMap;

use anyhow::{Context as _, Error};
use daicon_types::Id as FileId;
use stewart::{Actor, Context, Handler, Id, World};
use tracing::{event, instrument, Level};
use uuid::Uuid;

use crate::{
    file_source::indices::{self, Action, GetAction, SetAction},
    protocol::{file, source},
    FileSourceOptions,
};

/// Open a file as a daicon source.
///
/// If you want to start from an existing table, specify a `open_table` in options.
/// If `open_table` is not specified, the source will append a new table when required.
#[instrument("daicon::open_file_source", skip_all)]
pub fn open_file_source(
    world: &mut World,
    id: Id,
    file: Handler<file::Request>,
    options: FileSourceOptions,
) -> Result<Handler<source::Request>, Error> {
    event!(Level::INFO, "opening");

    let id = world.create(id, "daicon-file-source")?;
    let handler = Handler::to(id);

    // Handled as its own actor, as it needs to do async processing
    let indices = indices::start(world, id, file.clone(), options)?;

    // Start the root manager actor
    let actor = Service {
        handler: handler.clone(),
        file,
        indices,

        get_tasks: HashMap::new(),
        set_tasks: HashMap::new(),
    };
    world.start(id, actor)?;

    let sender = handler.map(ImplMessage::Message);
    Ok(sender)
}

struct Service {
    handler: Handler<ImplMessage>,
    file: Handler<file::Request>,
    indices: Handler<indices::Request>,

    // Ongoing tracked actions
    get_tasks: HashMap<Uuid, PendingGet>,
    set_tasks: HashMap<Uuid, PendingSet>,
}

struct PendingGet {
    on_result: Handler<source::GetResponse>,
}

struct PendingSet {
    id: FileId,
    size: u32,
    on_result: Handler<source::SetResponse>,
}

enum ImplMessage {
    Message(source::Request),
    GetIndexResult((Uuid, u64, u32)),
    SetWriteDataResult(file::WriteResponse),
}

impl Actor for Service {
    type Message = ImplMessage;

    fn process(&mut self, world: &mut World, mut cx: Context<Self>) -> Result<(), Error> {
        while let Some(message) = cx.next() {
            match message {
                ImplMessage::Message(message) => {
                    self.on_message(world, message)?;
                }
                ImplMessage::GetIndexResult((action_id, offset, size)) => {
                    self.on_get_index_result(world, action_id, offset, size)?;
                }
                ImplMessage::SetWriteDataResult(result) => {
                    self.on_set_write_data_result(world, result)?;
                }
            }
        }

        Ok(())
    }
}

impl Service {
    fn on_message(&mut self, world: &mut World, message: source::Request) -> Result<(), Error> {
        match message.action {
            source::Action::Get(action) => {
                self.on_get(world, message.id, action)?;
            }
            source::Action::Set(action) => {
                self.on_set(world, message.id, action)?;
            }
            source::Action::List(action) => {
                self.on_list(world, message.id, action)?;
            }
        }

        Ok(())
    }

    fn on_get(
        &mut self,
        world: &mut World,
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
        self.send_read_index(world, id, action.id);

        Ok(())
    }

    fn on_set(
        &mut self,
        world: &mut World,
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
        self.send_write_data(world, id, action.data);

        Ok(())
    }

    fn on_list(
        &mut self,
        world: &mut World,
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
        action.on_result.handle(world, response);
        Ok(())
    }

    fn on_get_index_result(
        &mut self,
        world: &mut World,
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
        self.send_read_data(world, id, offset, size, task.on_result);

        Ok(())
    }

    fn on_set_write_data_result(
        &mut self,
        world: &mut World,
        response: file::WriteResponse,
    ) -> Result<(), Error> {
        event!(Level::DEBUG, id = ?response.id, "received data write result");

        // Remove the task, we're done with it in this actor
        let task = self
            .set_tasks
            .remove(&response.id)
            .context("failed to get pending set task")?;

        // Write the index
        let offset = response.result?;
        self.send_write_index(world, response.id, task, offset);

        Ok(())
    }

    fn send_read_index(&self, world: &mut World, action_id: Uuid, id: FileId) {
        let on_result = self.handler.clone().map(ImplMessage::GetIndexResult);
        let action = GetAction { id, on_result };
        let message = indices::Request {
            id: action_id,
            action: Action::Get(action),
        };
        self.indices.handle(world, message);
    }

    fn send_write_index(&self, world: &mut World, id: Uuid, task: PendingSet, offset: u64) {
        let action = SetAction {
            id: task.id,
            offset,
            size: task.size,
            on_result: task
                .on_result
                .map(move |_| source::SetResponse { id, result: Ok(()) }),
        };
        let message = indices::Request {
            id,
            action: Action::Set(action),
        };
        self.indices.handle(world, message);
    }

    fn send_read_data(
        &self,
        world: &mut World,
        id: Uuid,
        offset: u64,
        size: u32,
        on_result: Handler<source::GetResponse>,
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
        let message = file::Request {
            id,
            action: file::Action::Read(action),
        };
        self.file.handle(world, message);
    }

    fn send_write_data(&self, world: &mut World, id: Uuid, data: Vec<u8>) {
        let action = file::WriteAction {
            offset: None,
            data,
            on_result: self.handler.clone().map(ImplMessage::SetWriteDataResult),
        };
        let message = file::Request {
            id,
            action: file::Action::Write(action),
        };
        self.file.handle(world, message);
    }
}
