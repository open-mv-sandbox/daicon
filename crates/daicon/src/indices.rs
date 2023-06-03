use std::{collections::HashMap, mem::size_of};

use anyhow::Error;
use daicon_types::{Header, Id, Index};
use stewart::{Actor, Context, Sender, State};
use tracing::{event, instrument, Level};
use uuid::Uuid;

use crate::{
    protocol::file,
    table::{deserialize_table, serialize_table, Table},
    FileSourceOptions,
};

#[instrument("daicon::indices::start", skip_all)]
pub fn start(
    ctx: &mut Context,
    file: Sender<file::Message>,
    options: FileSourceOptions,
) -> Result<Sender<Message>, Error> {
    event!(Level::DEBUG, "starting");

    let (mut ctx, sender) = ctx.create("daicon-indices")?;

    let mut tables = Vec::new();
    let mut is_reading = false;

    // TODO: If given a first table, respond with validation results.

    if let Some(offset) = options.first_table {
        is_reading = true;
        read_table(&mut ctx, &file, sender.clone(), offset)?;
    } else {
        // TODO: Just let the set command trigger a new table allocation

        // Start writing immediately at the given offset
        // TODO: Append first, then store the offset.
        let table = Table::new(options.allocate_capacity);

        write_table(&mut ctx, &file, &table)?;

        // Track the table we just wrote
        tables.push(table);
    }

    // Start the actor
    let actor = Service {
        sender: sender.clone(),
        file,

        tables: Vec::new(),
        is_reading,

        get_tasks: HashMap::new(),
        set_tasks: HashMap::new(),
    };
    ctx.start(actor)?;

    Ok(sender.map(ImplMessage::Message))
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
    pub on_result: Sender<(Uuid, u64, u32)>,
}

pub struct SetAction {
    pub id: Id,
    pub offset: u64,
    pub size: u32,
    pub on_result: Sender<Uuid>,
}

struct Service {
    sender: Sender<ImplMessage>,
    file: Sender<file::Message>,

    tables: Vec<Table>,

    /// If true, the service is currently still reading tables.
    is_reading: bool,

    // Ongoing tracked actions
    get_tasks: HashMap<Uuid, GetAction>,
    set_tasks: HashMap<Uuid, SetAction>,
}

impl Actor for Service {
    type Message = ImplMessage;

    fn process(&mut self, ctx: &mut Context, state: &mut State<Self>) -> Result<(), Error> {
        while let Some(message) = state.next() {
            match message {
                ImplMessage::Message(message) => self.on_message(message),
                ImplMessage::ReadResult(message) => self.on_read_result(ctx, message)?,
            }
        }

        self.update_tasks(ctx);

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
        ctx: &mut Context,
        message: file::ReadResponse,
    ) -> Result<(), Error> {
        event!(Level::DEBUG, "received read result");

        // Attempt to parse the table
        // TODO: This is where validation should happen
        // TODO: Retry if the table's valid data is larger than what we've read.
        // This happens if the read length heuristic is too small, we need to retry then.
        let data = message.result?;
        let (table, next) = deserialize_table(data)?;

        // Track the table we've at this point successfully parsed
        self.tables.push(table);

        // If we have a next table, queue it up for the next read
        if let Some(value) = next {
            read_table(ctx, &self.file, self.sender.clone(), value.get())?;
        }

        // We're done reading, store so we can start doing tasks that depend on this
        self.is_reading = false;

        Ok(())
    }

    fn update_tasks(&mut self, ctx: &mut Context) {
        // Resolve gets we can resolve
        self.get_tasks
            .retain(|id, action| update_get(ctx, &self.tables, *id, action));

        // Resolve writes we can resolve
        self.set_tasks
            .retain(|id, action| update_set(ctx, &self.file, &mut self.tables, *id, action));
    }
}

fn update_get(ctx: &mut Context, tables: &[Table], id: Uuid, action: &GetAction) -> bool {
    let (offset, size) = if let Some(value) = find_in(tables, action.id) {
        value
    } else {
        return true;
    };

    event!(Level::DEBUG, id = ?action.id, "found entry");
    action.on_result.send(ctx, (id, offset, size));

    false
}

fn update_set(
    ctx: &mut Context,
    file: &Sender<file::Message>,
    tables: &mut [Table],
    id: Uuid,
    action: &SetAction,
) -> bool {
    // Find a table with an empty slot
    // TODO: We now have more than one table
    let table = if let Some(table) = tables.first_mut() {
        table
    } else {
        return true;
    };

    event!(Level::DEBUG, id = ?action.id, "setting entry");

    // TODO: Allocate a new table if we've read all and we can't find a slot
    if !table.can_insert(action.offset) {
        event!(
            Level::ERROR,
            "cannot insert entry, allocation not implemented"
        );
        return false;
    }

    // Insert the new entry
    table.insert(action.id, action.offset, action.size);

    // Flush write the table
    write_table(ctx, file, table).unwrap();

    // Report success
    // TODO: Wait for write to report back
    action.on_result.send(ctx, id);

    false
}

fn find_in(tables: &[Table], id: Id) -> Option<(u64, u32)> {
    tables.iter().find_map(|table| table.find(id))
}

enum ImplMessage {
    Message(Message),
    ReadResult(file::ReadResponse),
}

fn read_table(
    ctx: &mut Context,
    file: &Sender<file::Message>,
    sender: Sender<ImplMessage>,
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
    file.send(ctx, message);

    Ok(())
}

fn write_table(
    ctx: &mut Context,
    file: &Sender<file::Message>,
    table: &Table,
) -> Result<(), Error> {
    let data = serialize_table(table)?;

    // Send to file for writing
    let action = file::WriteAction {
        offset: table.location(),
        data,
        on_result: Sender::noop(),
    };
    let message = file::Message {
        id: Uuid::new_v4(),
        action: file::Action::Write(action),
    };
    file.send(ctx, message);

    Ok(())
}
