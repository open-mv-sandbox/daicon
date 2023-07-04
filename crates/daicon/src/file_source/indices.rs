use std::{collections::HashMap, mem::size_of};

use anyhow::{Context as _, Error};
use daicon_types::{Header, Id as FileId, Index};
use stewart::{Actor, Context, Handler, Id, World};
use tracing::{event, instrument, Level};
use uuid::Uuid;

use crate::{file_source::table::Table, protocol::file, FileSourceOptions};

pub struct Request {
    pub id: Uuid,
    pub action: Action,
}

pub enum Action {
    Get(GetAction),
    Set(SetAction),
}

pub struct GetAction {
    pub id: FileId,
    pub on_result: Handler<(Uuid, u64, u32)>,
}

pub struct SetAction {
    pub id: FileId,
    pub offset: u64,
    pub size: u32,
    pub on_result: Handler<Uuid>,
}

#[instrument("daicon::start_indices", skip_all)]
pub fn start(
    world: &mut World,
    id: Id,
    file: Handler<file::Request>,
    options: FileSourceOptions,
) -> Result<Handler<Request>, Error> {
    event!(Level::DEBUG, "starting");

    let id = world.create(id, "daicon-file-indices")?;
    let handler = Handler::to(id);

    let mut tables = Vec::new();
    let mut pending_read = None;

    // TODO: Respond with validation results, success of open or create.

    if let Some(offset) = options.open_table {
        // Start opening by reading the first table
        pending_read = Some(offset);
        read_table(world, &file, handler.clone(), offset)?;
    } else {
        // Write a table immediately
        let table = Table::new(options.allocate_capacity);
        write_table(world, &file, &table, None)?;

        // Track the table we just wrote
        tables.push(table);
    }

    // Start the actor
    let actor = Service {
        sender: handler.clone(),
        file,

        tables: Vec::new(),
        pending_read,

        get_tasks: HashMap::new(),
        set_tasks: HashMap::new(),
    };
    world.start(id, actor)?;

    Ok(handler.map(Message::Message))
}

struct Service {
    sender: Handler<Message>,
    file: Handler<file::Request>,

    tables: Vec<(u64, Table)>,

    /// If set, the service is currently still reading tables at the given offset.
    pending_read: Option<u64>,

    // Ongoing tracked actions
    get_tasks: HashMap<Uuid, GetAction>,
    set_tasks: HashMap<Uuid, SetAction>,
}

enum Message {
    Message(Request),
    ReadResult(file::ReadResponse),
}

impl Actor for Service {
    type Message = Message;

    fn process(&mut self, world: &mut World, mut cx: Context<Self>) -> Result<(), Error> {
        while let Some(message) = cx.next() {
            match message {
                Message::Message(message) => self.on_message(message),
                Message::ReadResult(message) => self.on_read_result(world, message)?,
            }
        }

        self.update_tasks(world);

        Ok(())
    }
}

impl Service {
    fn on_message(&mut self, message: Request) {
        match message.action {
            Action::Get(action) => {
                event!(Level::DEBUG, id = ?action.id, "received get");
                self.get_tasks.insert(message.id, action);
            }
            Action::Set(action) => {
                event!(Level::DEBUG, id = ?action.id, "received set");
                self.set_tasks.insert(message.id, action);
            }
        }
    }

    fn on_read_result(
        &mut self,
        world: &mut World,
        message: file::ReadResponse,
    ) -> Result<(), Error> {
        event!(Level::DEBUG, "received read result");

        // TODO: This is where validation should happen.

        // TODO: Retry if the table's valid data is larger than what we've read.
        // This happens if the read length heuristic is too small, we need to retry then.

        // Attempt to parse the table
        let data = message.result?;
        let (table, next) = Table::deserialize(&data)?;

        // Track the table we've at this point successfully parsed
        let offset = self.pending_read.context("no pending read")?;
        self.tables.push((offset, table));

        // If we have a next table, queue it up for the next read
        if let Some(value) = next {
            let offset = value.get();
            read_table(world, &self.file, self.sender.clone(), offset)?;
            self.pending_read = Some(offset);
        }

        // We're done reading, store so we can start doing tasks that depend on this
        self.pending_read = None;

        Ok(())
    }

    fn update_tasks(&mut self, world: &mut World) {
        // Resolve gets we can resolve
        self.get_tasks
            .retain(|id, action| update_get(world, &self.tables, *id, action));

        // Resolve sets we can resolve
        self.set_tasks
            .retain(|id, action| update_set(world, &self.file, &mut self.tables, *id, action));
    }
}

fn update_get(world: &mut World, tables: &[(u64, Table)], id: Uuid, action: &GetAction) -> bool {
    let (offset, size) = if let Some(value) = find_in(tables, action.id) {
        value
    } else {
        return true;
    };

    event!(Level::DEBUG, id = ?action.id, "found entry");
    action.on_result.handle(world, (id, offset, size));

    false
}

fn update_set(
    world: &mut World,
    file: &Handler<file::Request>,
    tables: &mut [(u64, Table)],
    id: Uuid,
    action: &SetAction,
) -> bool {
    // Find a table with an empty slot
    // TODO: We now can have more than one table when reading
    let (offset, table) = if let Some(value) = tables.first_mut() {
        value
    } else {
        return true;
    };

    event!(Level::DEBUG, id = ?action.id, "setting entry");

    // TODO: Allocate a new table if we've read all and we can't find a slot
    if !table.try_insert(action.id, action.offset, action.size) {
        event!(
            Level::ERROR,
            "cannot insert entry, allocation not yet implemented"
        );
        return false;
    }

    // Flush write the table
    // TODO: Batch writeback
    write_table(world, file, table, Some(*offset)).unwrap();

    // Report success
    // TODO: Wait for write to succeed before reporting back
    action.on_result.handle(world, id);

    false
}

fn find_in(tables: &[(u64, Table)], id: FileId) -> Option<(u64, u32)> {
    tables.iter().find_map(|(_, table)| table.find(id))
}

fn read_table(
    world: &mut World,
    file: &Handler<file::Request>,
    sender: Handler<Message>,
    offset: u64,
) -> Result<(), Error> {
    // Estimate the size of the table, so we can hopefully prefetch all of it
    let size = (size_of::<Header>() + (size_of::<Index>() * 256)) as u64;

    // Start reading the first header
    let action = file::ReadAction {
        offset,
        size,
        on_result: sender.map(Message::ReadResult),
    };
    let message = file::Request {
        id: Uuid::new_v4(),
        action: file::Action::Read(action),
    };
    file.handle(world, message);

    Ok(())
}

fn write_table(
    world: &mut World,
    file: &Handler<file::Request>,
    table: &Table,
    offset: Option<u64>,
) -> Result<(), Error> {
    let data = table.serialize()?;

    // Send to file for writing
    let action = file::WriteAction {
        offset,
        data,
        on_result: Handler::none(),
    };
    let message = file::Request {
        id: Uuid::new_v4(),
        action: file::Action::Write(action),
    };
    file.handle(world, message);

    Ok(())
}
