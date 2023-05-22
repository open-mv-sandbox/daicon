use std::{
    collections::HashMap,
    io::Cursor,
    io::{Read, Write},
    mem::size_of,
};

use anyhow::Error;
use bytemuck::{bytes_of, bytes_of_mut, cast_slice_mut};
use daicon_types::{Entry, Header};
use stewart::{Actor, Context, Options, Sender, State};
use tracing::{event, instrument, Level};
use uuid::Uuid;

use crate::{
    file::{FileAction, FileMessage, ReadResult, WriteLocation},
    OpenMode,
};

pub fn start(
    ctx: &mut Context,
    file: Sender<FileMessage>,
    mode: OpenMode,
) -> Result<Sender<IndicesMessage>, Error> {
    event!(Level::DEBUG, "starting indices service");

    let (mut ctx, sender) = ctx.create(Options::default())?;

    let mut tables = Vec::new();

    // TODO: This is also the validation step, respond if we correctly validated
    match mode {
        OpenMode::ReadWrite => {
            read_table(&mut ctx, file.clone(), sender.clone())?;
        }
        OpenMode::Create => {
            // Start writing immediately at the given offset
            let mut header = Header::default();
            header.set_capacity(256);
            let table = Table {
                location: 0,
                offset: 0,
                capacity: 256,
                entries: Vec::new(),
            };

            write_table(&mut ctx, file.clone(), &table)?;

            // Track the table we just wrote
            tables.push(table);
        }
    }

    // Start the actor
    let actor = IndicesService {
        file,

        tables: Vec::new(),
        get_tasks: HashMap::new(),
        set_tasks: HashMap::new(),
    };
    ctx.start(actor)?;

    Ok(sender.map(ImplMessage::Message))
}

pub struct IndicesMessage {
    pub id: Uuid,
    pub action: IndicesAction,
}

pub enum IndicesAction {
    Get(GetIndex),
    Set(SetIndex),
}

pub struct GetIndex {
    pub id: u32,
    pub on_result: Sender<(Uuid, u64, u32)>,
}

pub struct SetIndex {
    pub id: u32,
    pub offset: u64,
    pub size: u32,
    pub on_result: Sender<()>,
}

struct IndicesService {
    file: Sender<FileMessage>,

    tables: Vec<Table>,
    get_tasks: HashMap<Uuid, GetIndex>,
    set_tasks: HashMap<Uuid, SetIndex>,
}

impl Actor for IndicesService {
    type Message = ImplMessage;

    #[instrument("indices", skip_all)]
    fn process(&mut self, ctx: &mut Context, state: &mut State<Self>) -> Result<(), Error> {
        while let Some(message) = state.next() {
            match message {
                ImplMessage::Message(message) => self.on_message(message),
                ImplMessage::ReadResult(message) => self.on_read_result(message)?,
            }
        }

        self.update_tasks(ctx);

        Ok(())
    }
}

impl IndicesService {
    fn on_message(&mut self, message: IndicesMessage) {
        match message.action {
            IndicesAction::Get(action) => {
                event!(Level::DEBUG, "received get {:#010x}", action.id);
                self.get_tasks.insert(message.id, action);
            }
            IndicesAction::Set(action) => {
                event!(Level::DEBUG, "received set {:#010x}", action.id);
                self.set_tasks.insert(message.id, action);
            }
        }
    }

    fn on_read_result(&mut self, message: ReadResult) -> Result<(), Error> {
        event!(Level::DEBUG, "received read result");

        let table = parse_table(message)?;
        self.tables.push(table);

        Ok(())
    }

    fn update_tasks(&mut self, ctx: &mut Context) {
        // Resolve gets we can resolve
        self.get_tasks
            .retain(|id, task| match find(&self.tables, task.id) {
                Some(value) => {
                    event!(Level::DEBUG, "found entry {:#010x}", task.id);
                    task.on_result.send(ctx, (*id, value.0, value.1));
                    false
                }
                None => true,
            });

        // Resolve writes we can resolve
        self.set_tasks.retain(|_, task| {
            // Find a table with an empty slot
            let table = if let Some(table) = self.tables.first_mut() {
                table
            } else {
                return true;
            };

            event!(Level::DEBUG, "setting entry {:#010x}", task.id);

            // TODO: Allocate a new table if we've read all and we can't find a slot
            if task.offset < table.offset {
                event!(Level::ERROR, "cannot insert entry, case not implemented");
                return false;
            }
            let relative = task.offset - table.offset;
            if relative > u32::MAX as u64 || table.entries.len() >= table.capacity as usize {
                event!(Level::ERROR, "cannot insert entry, case not implemented");
                return false;
            }

            // Insert the new entry
            let mut entry = Entry::default();
            entry.set_id(task.id);
            entry.set_offset(relative as u32);
            entry.set_size(task.size);
            table.entries.push(entry);

            // Flush write the table
            write_table(ctx, self.file.clone(), table).unwrap();

            // Report success
            // TODO: Wait for write to report back
            task.on_result.send(ctx, ());

            false
        });
    }
}

struct Table {
    location: u64,
    offset: u64,
    capacity: u16,
    entries: Vec<Entry>,
}

impl Table {
    fn find(&self, id: u32) -> Option<(u64, u32)> {
        self.entries
            .iter()
            .find(|entry| entry.id() == id)
            .map(|entry| {
                let offset = entry.offset() as u64 + self.offset;
                (offset, entry.size())
            })
    }
}

enum ImplMessage {
    Message(IndicesMessage),
    ReadResult(ReadResult),
}

fn find(tables: &[Table], id: u32) -> Option<(u64, u32)> {
    tables.iter().find_map(|table| table.find(id))
}

fn read_table(
    ctx: &mut Context,
    file: Sender<FileMessage>,
    sender: Sender<ImplMessage>,
) -> Result<(), Error> {
    // Start reading the first header
    let size = (size_of::<Header>() + (size_of::<Entry>() * 256)) as u64;
    let message = FileMessage {
        id: Uuid::new_v4(),
        action: FileAction::Read {
            offset: 0,
            size,
            on_result: sender.map(ImplMessage::ReadResult),
        },
    };
    file.send(ctx, message);

    Ok(())
}

fn parse_table(result: ReadResult) -> Result<Table, Error> {
    // TODO: Retry if the table's valid data is larger than what we've read.
    //  This happens if the read length heuristic is too small, we need to retry then.

    let mut data = Cursor::new(result.data);

    // Read the header
    let mut header = Header::default();
    data.read_exact(bytes_of_mut(&mut header))?;

    // Read entries
    let mut entries = vec![Entry::default(); header.valid() as usize];
    data.read_exact(cast_slice_mut(&mut entries))?;

    let table = Table {
        location: result.offset,
        offset: header.offset(),
        capacity: header.capacity(),
        entries,
    };
    Ok(table)
}

fn write_table(ctx: &mut Context, file: Sender<FileMessage>, table: &Table) -> Result<(), Error> {
    let mut data = Vec::new();

    // Write the header
    let mut header = Header::default();
    header.set_offset(table.offset);
    header.set_capacity(table.capacity);
    header.set_valid(table.entries.len() as u16);
    data.write_all(bytes_of(&header))?;

    // Write entries
    for entry in &table.entries {
        data.write_all(bytes_of(entry))?;
    }

    // Pad with empty entries
    let empty = Entry::default();
    for _ in 0..(table.capacity as usize - table.entries.len()) {
        data.write_all(bytes_of(&empty))?;
    }

    // Send to file for writing
    let action = FileAction::Write {
        location: WriteLocation::Offset(table.location),
        data,
        on_result: Sender::noop(),
    };
    let message = FileMessage {
        id: Uuid::new_v4(),
        action,
    };
    file.send(ctx, message);

    Ok(())
}
