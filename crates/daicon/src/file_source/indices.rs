use std::{collections::HashMap, mem::size_of};

use anyhow::{Context as _, Error};
use daicon_types::{Header, Id, Index};
use stewart::{Actor, Context, Handler, State, World};
use tracing::{event, instrument, Level};
use uuid::Uuid;

use crate::{file_source::table::Table, protocol::file, FileSourceOptions};

#[instrument("daicon::start_indices", skip_all)]
pub fn start(
    world: &mut World,
    cx: &Context,
    file: Handler<file::Message>,
    options: FileSourceOptions,
) -> Result<Handler<Message>, Error> {
    event!(Level::DEBUG, "starting");

    let (_cx, id) = world.create(cx, "daicon-file-indices")?;
    let handler = Handler::to(id);

    let mut tables = Vec::new();
    let mut pending_read = None;

    // TODO: Respond with validation results, success of open or create.

    if let Some(offset) = options.open_table {
        // Start opening by reading the first table
        pending_read = Some(offset);
        read_table(world, &file, handler.clone(), offset)?;
    } else {
        // Write a table immediately at the given offset

        // TODO: Append to the end of the file, then store the offset.
        let table = Table::new(options.allocate_capacity);
        write_table(world, &file, &table, 0)?;

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

    Ok(handler.map(ImplMessage::Message))
}

pub struct Message {
    pub id: Uuid,
    pub action: Action,
}

pub enum Action {
    Get(GetAction),
    Set(SetAction),
}

pub struct GetAction {
    pub id: Id,
    pub on_result: Handler<(Uuid, u64, u32)>,
}

pub struct SetAction {
    pub id: Id,
    pub offset: u64,
    pub size: u32,
    pub on_result: Handler<Uuid>,
}

struct Service {
    sender: Handler<ImplMessage>,
    file: Handler<file::Message>,

    tables: Vec<(u64, Table)>,

    /// If set, the service is currently still reading tables at the given offset.
    pending_read: Option<u64>,

    // Ongoing tracked actions
    get_tasks: HashMap<Uuid, GetAction>,
    set_tasks: HashMap<Uuid, SetAction>,
}

impl Actor for Service {
    type Message = ImplMessage;

    fn process(
        &mut self,
        world: &mut World,
        _cx: &Context,
        state: &mut State<Self>,
    ) -> Result<(), Error> {
        while let Some(message) = state.next() {
            match message {
                ImplMessage::Message(message) => self.on_message(message),
                ImplMessage::ReadResult(message) => self.on_read_result(world, message)?,
            }
        }

        self.update_tasks(world);

        Ok(())
    }
}

impl Service {
    fn on_message(&mut self, message: Message) {
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

        // Attempt to parse the table
        // TODO: This is where validation should happen
        // TODO: Retry if the table's valid data is larger than what we've read.
        // This happens if the read length heuristic is too small, we need to retry then.
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
    file: &Handler<file::Message>,
    tables: &mut [(u64, Table)],
    id: Uuid,
    action: &SetAction,
) -> bool {
    // Find a table with an empty slot
    // TODO: We now have more than one table
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
            "cannot insert entry, allocation not implemented"
        );
        return false;
    }

    // Flush write the table
    write_table(world, file, table, *offset).unwrap();

    // Report success
    // TODO: Wait for write to report back
    action.on_result.handle(world, id);

    false
}

fn find_in(tables: &[(u64, Table)], id: Id) -> Option<(u64, u32)> {
    tables.iter().find_map(|(_, table)| table.find(id))
}

enum ImplMessage {
    Message(Message),
    ReadResult(file::ReadResponse),
}

fn read_table(
    world: &mut World,
    file: &Handler<file::Message>,
    sender: Handler<ImplMessage>,
    offset: u64,
) -> Result<(), Error> {
    // Estimate the size of the table, so we can hopefully prefetch all of it
    let size = (size_of::<Header>() + (size_of::<Index>() * 256)) as u64;

    // Start reading the first header
    let action = file::ReadAction {
        offset,
        size,
        on_result: sender.map(ImplMessage::ReadResult),
    };
    let message = file::Message {
        id: Uuid::new_v4(),
        action: file::Action::Read(action),
    };
    file.handle(world, message);

    Ok(())
}

fn write_table(
    world: &mut World,
    file: &Handler<file::Message>,
    table: &Table,
    offset: u64,
) -> Result<(), Error> {
    let data = table.serialize()?;

    // Send to file for writing
    let action = file::WriteAction {
        offset: Some(offset),
        data,
        on_result: Handler::noop(),
    };
    let message = file::Message {
        id: Uuid::new_v4(),
        action: file::Action::Write(action),
    };
    file.handle(world, message);

    Ok(())
}
