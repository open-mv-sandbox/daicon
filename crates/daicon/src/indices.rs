use std::{
    collections::HashMap,
    io::Cursor,
    io::{Read, Write},
    mem::size_of,
};

use anyhow::Error;
use bytemuck::{bytes_of, bytes_of_mut, cast_slice_mut};
use daicon_types::{Header, Id, Index};
use stewart::{Actor, Context, Options, Sender, State};
use tracing::{event, instrument, Level};
use uuid::Uuid;

use crate::{
    protocol::{FileAction, FileMessage, FileRead, FileWrite, ReadResult, WriteLocation},
    OpenMode,
};

#[instrument("IndexService", skip_all)]
pub fn start(
    ctx: &mut Context,
    file: Sender<FileMessage>,
    mode: OpenMode,
) -> Result<Sender<IndexServiceMessage>, Error> {
    event!(Level::DEBUG, "starting");

    let (mut ctx, sender) = ctx.create(Options::default())?;

    let mut tables = Vec::new();

    // TODO: This is also the validation step, respond if we correctly validated
    match mode {
        OpenMode::ReadWrite => {
            read_table(&mut ctx, &file, sender.clone())?;
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

            write_table(&mut ctx, &file, &table)?;

            // Track the table we just wrote
            tables.push(table);
        }
    }

    // Start the actor
    let actor = IndexService {
        file,

        tables: Vec::new(),
        get_tasks: HashMap::new(),
        set_tasks: HashMap::new(),
    };
    ctx.start(actor)?;

    Ok(sender.map(ImplMessage::Message))
}

pub struct IndexServiceMessage {
    pub id: Uuid,
    pub action: IndexAction,
}

pub enum IndexAction {
    Get(IndexGet),
    Set(IndexSet),
}

pub struct IndexGet {
    pub id: Id,
    pub on_result: Sender<(Uuid, u64, u32)>,
}

pub struct IndexSet {
    pub id: Id,
    pub offset: u64,
    pub size: u32,
    pub on_result: Sender<()>,
}

struct IndexService {
    file: Sender<FileMessage>,

    tables: Vec<Table>,
    get_tasks: HashMap<Uuid, IndexGet>,
    set_tasks: HashMap<Uuid, IndexSet>,
}

impl Actor for IndexService {
    type Message = ImplMessage;

    #[instrument("IndexService", skip_all)]
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

impl IndexService {
    fn on_message(&mut self, message: IndexServiceMessage) {
        match message.action {
            IndexAction::Get(action) => {
                event!(Level::DEBUG, id = ?action.id, "received get");
                self.get_tasks.insert(message.id, action);
            }
            IndexAction::Set(action) => {
                event!(Level::DEBUG, id = ?action.id, "received set");
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
                    event!(Level::DEBUG, id = ?task.id, "found entry");
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

            event!(Level::DEBUG, id = ?task.id, "setting entry");

            // TODO: Allocate a new table if we've read all and we can't find a slot
            if !table.can_insert(task.offset) {
                event!(Level::ERROR, "cannot insert entry, case not implemented");
                return false;
            }
            let relative = task.offset - table.offset;

            // Insert the new entry
            let mut entry = Index::default();
            entry.set_id(task.id);
            entry.set_offset(relative as u32);
            entry.set_size(task.size);
            table.entries.push(entry);

            // Flush write the table
            write_table(ctx, &self.file, table).unwrap();

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
    entries: Vec<Index>,
}

impl Table {
    fn find(&self, id: Id) -> Option<(u64, u32)> {
        self.entries
            .iter()
            .find(|entry| entry.id() == id)
            .map(|entry| {
                let offset = entry.offset() as u64 + self.offset;
                (offset, entry.size())
            })
    }

    fn can_insert(&self, offset: u64) -> bool {
        // Check if we have any room at all
        if self.entries.len() >= self.capacity as usize {
            return false;
        }

        // Check if the offset is in-range
        if offset < self.offset || (offset - self.offset) > u32::MAX as u64 {
            return false;
        }

        true
    }
}

enum ImplMessage {
    Message(IndexServiceMessage),
    ReadResult(ReadResult),
}

fn find(tables: &[Table], id: Id) -> Option<(u64, u32)> {
    tables.iter().find_map(|table| table.find(id))
}

fn read_table(
    ctx: &mut Context,
    file: &Sender<FileMessage>,
    sender: Sender<ImplMessage>,
) -> Result<(), Error> {
    // Start reading the first header
    let size = (size_of::<Header>() + (size_of::<Index>() * 256)) as u64;
    let action = FileRead {
        offset: 0,
        size,
        on_result: sender.map(ImplMessage::ReadResult),
    };
    let message = FileMessage {
        id: Uuid::new_v4(),
        action: FileAction::Read(action),
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
    let mut entries = vec![Index::default(); header.valid() as usize];
    data.read_exact(cast_slice_mut(&mut entries))?;

    let table = Table {
        location: result.offset,
        offset: header.offset(),
        capacity: header.capacity(),
        entries,
    };
    Ok(table)
}

fn write_table(ctx: &mut Context, file: &Sender<FileMessage>, table: &Table) -> Result<(), Error> {
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
    let empty = Index::default();
    for _ in 0..(table.capacity as usize - table.entries.len()) {
        data.write_all(bytes_of(&empty))?;
    }

    // Send to file for writing
    let action = FileWrite {
        location: WriteLocation::Offset(table.location),
        data,
        on_result: Sender::noop(),
    };
    let message = FileMessage {
        id: Uuid::new_v4(),
        action: FileAction::Write(action),
    };
    file.send(ctx, message);

    Ok(())
}
